//! GPU connection with an ordered backend fallback.

use anyhow::{Context, Result};

/// Backends tried in order when `WGPU_BACKEND` is not set: Vulkan first, then DX12
/// on Windows for GPUs without a working Vulkan driver (Metal on macOS).
pub fn backend_order() -> Vec<wgpu::Backends> {
    let mut order = vec![wgpu::Backends::VULKAN];
    if cfg!(windows) {
        order.push(wgpu::Backends::DX12);
    }
    if cfg!(target_os = "macos") {
        order.push(wgpu::Backends::METAL);
    }
    order
}

/// A working GPU device, plus the window surface when one was requested.
pub struct Connection {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface<'static>>,
}

/// Connects to the first backend in [`backend_order`] that yields a working device
/// (presenting to `window`, if given). `WGPU_BACKEND` replaces the order with a
/// single explicit choice.
pub fn connect(window: Option<std::sync::Arc<dyn wgpu::DisplayAndWindowHandle>>) -> Result<Connection> {
    let candidates = if std::env::var_os("WGPU_BACKEND").is_some() {
        vec![wgpu::Backends::all().with_env()]
    } else {
        backend_order()
    };
    let mut failures = Vec::new();
    for backends in candidates {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle().with_env();
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(descriptor);
        let surface = match window.clone().map(|w| instance.create_surface(w)).transpose() {
            Ok(surface) => surface,
            Err(error) => {
                failures.push(format!("{backends:?}: {error}"));
                continue;
            }
        };
        match pollster::block_on(request_device(&instance, surface.as_ref())) {
            Ok((adapter, device, queue)) => return Ok(Connection { instance, adapter, device, queue, surface }),
            Err(error) => {
                log::warn!("{backends:?} unavailable: {error:#}");
                failures.push(format!("{backends:?}: {error:#}"));
            }
        }
    }
    anyhow::bail!("no usable GPU backend:\n  {}", failures.join("\n  "))
}

/// Requests an adapter and device. `surface` makes the adapter compatible with a window.
pub async fn request_device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: surface,
            apply_limit_buckets: false,
        })
        .await
        .context("no compatible GPU adapter found")?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("dead-sector"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        })
        .await
        .context("creating GPU device")?;
    Ok((adapter, device, queue))
}
