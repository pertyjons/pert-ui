//! The side buffer bound only for shapes with spatial paint.
use super::*;
use pert_ui_runtime::{FrameMapping, Gradient, GradientExtend, GradientSpace};

const RECORD: usize = 32 + 48 * Gradient::MAX_STOPS;
#[derive(Default)]
pub(super) struct Table {
    binding: Option<wgpu::BindGroup>,
    // Retain the storage buffer through its bind group.
    bytes: Vec<u8>,
}
impl Table {
    pub(super) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        gradients: &[Gradient],
        mapping: Option<&FrameMapping>,
        max: u64,
    ) -> Result<(), RenderError> {
        let Some(mapping) = mapping.filter(|_| !gradients.is_empty()) else {
            self.binding = None;
            self.bytes.clear();
            return Ok(());
        };
        let size = gradients
            .len()
            .checked_mul(RECORD)
            .ok_or(LimitError::VertexCount)?;
        if size as u64 > max {
            return Err(LimitError::GradientRecords {
                bytes: size as u64,
                max,
            }
            .into());
        }
        let mut bytes = Vec::with_capacity(size);
        for gradient in gradients {
            let start = bytes.len();
            let c = gradient
                .coefficients(mapping)
                .map_err(pert_ui_runtime::DrawError::from)?;
            bytes.extend(c.into_iter().flat_map(f32::to_le_bytes));
            let space = match gradient.space() {
                GradientSpace::Oklab => 0,
                GradientSpace::Linear => 1,
                GradientSpace::Srgb => 2,
            };
            let extend = match gradient.extend() {
                GradientExtend::Pad => 0,
                GradientExtend::Repeat => 1,
                GradientExtend::Reflect => 2,
            };
            bytes.extend(
                [
                    gradient.stops().len() as u32,
                    space,
                    extend,
                    gradient.inner().unwrap_or(-1.).to_bits(),
                ]
                .into_iter()
                .flat_map(u32::to_le_bytes),
            );
            for ((offset, color), components) in gradient.stops().iter().zip(gradient.components())
            {
                for v in [*components, color.channels(), [*offset, 0., 0., 0.]]
                    .into_iter()
                    .flatten()
                {
                    bytes.extend(v.to_le_bytes());
                }
            }
            bytes.resize(start + RECORD, 0);
        }
        if self.bytes != bytes {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gradient records"),
                size: size as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&buffer, 0, &bytes);
            self.binding = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("gradient paints"),
                layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            }));
            self.bytes = bytes;
        }
        Ok(())
    }
    pub(super) fn binding(&self) -> Result<&wgpu::BindGroup, RenderError> {
        self.binding
            .as_ref()
            .ok_or_else(|| RenderError::Graphics("gradient records are missing".into()))
    }
}
