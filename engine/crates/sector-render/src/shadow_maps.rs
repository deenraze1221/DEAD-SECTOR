//! Shadow-map textures and the uniforms that describe them.

use crate::gpu_util::{array_view, bind_group, layer_view, render_texture};
use crate::scene_pass::SHADOW_FORMAT;
use crate::settings::{RenderSettings, ShadowFilter};
use crate::shadows::{Cascade, MAX_CASCADES, MAX_SHADOWED_LIGHTS};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

/// Dynamic-offset stride of the per-view uniform (one matrix per shadow view).
pub(crate) const VIEW_STRIDE: u32 = 256;
const LOCAL_LAYERS: usize = MAX_SHADOWED_LIGHTS * 6;
/// View slots: cascades, then six faces per shadowed light, then the camera prepass.
pub(crate) const PREPASS_VIEW: u32 = (MAX_CASCADES + LOCAL_LAYERS) as u32;

pub(crate) fn cascade_view(index: usize) -> u32 {
    index as u32 * VIEW_STRIDE
}

pub(crate) fn face_view(slot: usize, face: usize) -> u32 {
    (MAX_CASCADES + slot * 6 + face) as u32 * VIEW_STRIDE
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ShadowUniform {
    cascades: [[f32; 16]; MAX_CASCADES],
    splits: [f32; 4],
    texels: [f32; 4],
    sun: [f32; 4],
    camera_forward: [f32; 4],
    local: [f32; 4],
    faces: [[f32; 16]; LOCAL_LAYERS],
}

fn filter_index(filter: ShadowFilter) -> f32 {
    match filter {
        ShadowFilter::Hard => 0.0,
        ShadowFilter::Pcf => 1.0,
        ShadowFilter::Soft => 2.0,
    }
}

impl ShadowUniform {
    pub(crate) fn new(
        settings: &RenderSettings,
        cascades: &[Cascade],
        faces: &[[Mat4; 6]],
        camera_forward: Vec3,
        occlusion: bool,
    ) -> Self {
        let mut uniform = ShadowUniform::zeroed();
        for (i, c) in cascades.iter().enumerate().take(MAX_CASCADES) {
            uniform.cascades[i] = c.view_proj.to_cols_array();
            uniform.splits[i] = c.far;
            uniform.texels[i] = c.texel;
        }
        if let Some(sun) = settings.sun_shadows.filter(|_| !cascades.is_empty()) {
            uniform.sun = [cascades.len() as f32, filter_index(sun.filter), sun.distance * 0.9, sun.distance];
        }
        for (slot, matrices) in faces.iter().enumerate().take(MAX_SHADOWED_LIGHTS) {
            for (face, m) in matrices.iter().enumerate() {
                uniform.faces[slot * 6 + face] = m.to_cols_array();
            }
        }
        let local = settings.local_shadows;
        uniform.camera_forward = camera_forward.extend(0.0).to_array();
        uniform.local =
            [filter_index(local.filter), 1.0 / local.resolution.max(1) as f32, if occlusion { 1.0 } else { 0.0 }, 0.0];
        uniform
    }
}

/// Texture sizes that require re-allocating the maps when they change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MapConfig {
    sun_resolution: u32,
    cascades: u32,
    local_resolution: u32,
    local_lights: u32,
}

impl MapConfig {
    pub(crate) fn new(settings: &RenderSettings) -> Self {
        let (sun_resolution, cascades) =
            settings.sun_shadows.map_or((1, 0), |s| (s.resolution.max(1), s.cascades.clamp(1, MAX_CASCADES as u32)));
        let local = settings.local_shadows;
        let local_lights = local.budget.min(MAX_SHADOWED_LIGHTS as u32);
        let local_resolution = if local_lights == 0 { 1 } else { local.resolution.max(1) };
        MapConfig { sun_resolution, cascades, local_resolution, local_lights }
    }
}

pub(crate) struct ShadowMaps {
    pub config: MapConfig,
    pub sun_layers: Vec<wgpu::TextureView>,
    pub local_layers: Vec<wgpu::TextureView>,
    sun_array: wgpu::TextureView,
    local_array: wgpu::TextureView,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    views: wgpu::Buffer,
    pub view_group: wgpu::BindGroup,
    /// Bind group for the scene shader, rebuilt when the occlusion texture changes.
    group: Option<(u64, wgpu::BindGroup)>,
}

impl ShadowMaps {
    pub(crate) fn new(device: &wgpu::Device, view_layout: &wgpu::BindGroupLayout, config: MapConfig) -> Self {
        let sun_size = (config.sun_resolution, config.sun_resolution);
        let sun = render_texture(device, "sun-shadows", sun_size, config.cascades, SHADOW_FORMAT);
        let local_size = (config.local_resolution, config.local_resolution);
        let local = render_texture(device, "local-shadows", local_size, config.local_lights * 6, SHADOW_FORMAT);
        let views = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow-views"),
            size: (PREPASS_VIEW + 1) as u64 * VIEW_STRIDE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_group = bind_group(
            device,
            "shadow-views",
            view_layout,
            &[wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &views,
                offset: 0,
                size: std::num::NonZeroU64::new(64),
            })],
        );
        ShadowMaps {
            config,
            sun_layers: (0..config.cascades).map(|l| layer_view(&sun, l)).collect(),
            local_layers: (0..config.local_lights * 6).map(|l| layer_view(&local, l)).collect(),
            sun_array: array_view(&sun),
            local_array: array_view(&local),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("shadow-compare"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            }),
            uniform: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shadows"),
                size: std::mem::size_of::<ShadowUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            views,
            view_group,
            group: None,
        }
    }

    /// Uploads one view-projection per view slot.
    pub(crate) fn write_view(&self, queue: &wgpu::Queue, offset: u32, view_proj: Mat4) {
        queue.write_buffer(&self.views, offset as u64, bytemuck::bytes_of(&view_proj.to_cols_array()));
    }

    pub(crate) fn write_uniform(&self, queue: &wgpu::Queue, uniform: &ShadowUniform) {
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(uniform));
    }

    /// The scene shader's shadow/occlusion bind group. `occlusion` is the
    /// screen-space occlusion texture (or a 1×1 stand-in) identified by `key`.
    pub(crate) fn scene_group(
        &mut self,
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        key: u64,
        occlusion: &wgpu::TextureView,
    ) -> &wgpu::BindGroup {
        if self.group.as_ref().is_none_or(|(k, _)| *k != key) {
            let group = bind_group(
                device,
                "shadows",
                layout,
                &[
                    self.uniform.as_entire_binding(),
                    wgpu::BindingResource::TextureView(&self.sun_array),
                    wgpu::BindingResource::TextureView(&self.local_array),
                    wgpu::BindingResource::Sampler(&self.sampler),
                    wgpu::BindingResource::TextureView(occlusion),
                ],
            );
            self.group = Some((key, group));
        }
        &self.group.as_ref().expect("group just built").1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{LocalShadows, SunShadows};

    #[test]
    fn view_slots_do_not_overlap() {
        assert_eq!(cascade_view(3) + VIEW_STRIDE, face_view(0, 0));
        assert_eq!(face_view(MAX_SHADOWED_LIGHTS - 1, 5) + VIEW_STRIDE, PREPASS_VIEW * VIEW_STRIDE);
    }

    #[test]
    fn config_reflects_settings() {
        let settings = RenderSettings {
            sun_shadows: Some(SunShadows { resolution: 1024, cascades: 9, ..Default::default() }),
            local_shadows: LocalShadows { budget: 7, resolution: 512, ..Default::default() },
            ..Default::default()
        };
        let config = MapConfig::new(&settings);
        assert_eq!((config.sun_resolution, config.cascades), (1024, 4));
        assert_eq!((config.local_lights, config.local_resolution), (4, 512));
        let off = MapConfig::new(&RenderSettings {
            sun_shadows: None,
            local_shadows: LocalShadows { budget: 0, ..Default::default() },
            ..Default::default()
        });
        assert_eq!((off.cascades, off.local_lights, off.sun_resolution, off.local_resolution), (0, 0, 1, 1));
    }
}
