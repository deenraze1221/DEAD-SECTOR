//! GPU pass timing with timestamp queries (when the adapter supports them).

use anyhow::{Context, Result};

const MAX_QUERIES: u32 = 256;

pub(crate) struct GpuProfiler {
    set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    labels: Vec<&'static str>,
    period_ns: f32,
}

impl GpuProfiler {
    /// `None` when the device was created without timestamp queries.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let size = MAX_QUERIES as u64 * 8;
        Some(GpuProfiler {
            set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("pass-timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count: MAX_QUERIES,
            }),
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamp-resolve"),
                size,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("timestamp-readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            labels: Vec::new(),
            period_ns: queue.get_timestamp_period(),
        })
    }

    /// Timestamp writes for the next render pass, recorded under `label`.
    pub(crate) fn pass(&mut self, label: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let index = self.labels.len() as u32 * 2;
        if index + 2 > MAX_QUERIES {
            return None;
        }
        self.labels.push(label);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.set,
            beginning_of_pass_write_index: Some(index),
            end_of_pass_write_index: Some(index + 1),
        })
    }

    /// Copies this frame's timestamps for [`Self::read`].
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let count = self.labels.len() as u32 * 2;
        if count > 0 {
            encoder.resolve_query_set(&self.set, 0..count, &self.resolve, 0);
            encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, count as u64 * 8);
        }
    }

    /// Milliseconds per label for the submitted frame; call after the GPU finished.
    pub(crate) fn read(&mut self, device: &wgpu::Device) -> Result<Vec<(&'static str, f64)>> {
        let count = self.labels.len() * 2;
        let mut totals: Vec<(&'static str, f64)> = Vec::new();
        if count > 0 {
            let slice = self.readback.slice(..count as u64 * 8);
            let (sender, receiver) = std::sync::mpsc::channel();
            slice.map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
            device.poll(wgpu::PollType::wait_indefinitely())?;
            receiver.recv()?.context("mapping timestamps")?;
            {
                let data = slice.get_mapped_range()?;
                let stamps: &[u64] = bytemuck::cast_slice(&data);
                for (i, label) in self.labels.iter().enumerate() {
                    let ms = stamps[i * 2 + 1].saturating_sub(stamps[i * 2]) as f64 * self.period_ns as f64 / 1e6;
                    match totals.iter_mut().find(|(l, _)| l == label) {
                        Some((_, total)) => *total += ms,
                        None => totals.push((label, ms)),
                    }
                }
            }
            self.readback.unmap();
        }
        self.labels.clear();
        Ok(totals)
    }
}
