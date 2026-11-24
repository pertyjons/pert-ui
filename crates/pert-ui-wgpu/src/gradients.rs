//! The side buffer bound only for shapes with spatial paint.
use super::*;
use pert_ui_runtime::{FrameMapping, Gradient, GradientExtend, GradientSpace};

#[derive(Default)]
struct Storage {
    buffer: Option<wgpu::Buffer>,
    previous: Vec<u8>,
}

impl Storage {
    fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: Vec<u8>,
        max: u64,
    ) -> bool {
        let size = bytes.len() as u64;
        let allocate = self
            .buffer
            .as_ref()
            .is_none_or(|buffer| buffer.size() < size);
        if allocate {
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gradient storage"),
                size: size.next_power_of_two().min(max),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let upload = if allocate {
            0..bytes.len()
        } else {
            changed_bytes(&self.previous, &bytes)
        };
        if let Some(buffer) = &self.buffer
            && !upload.is_empty()
        {
            queue.write_buffer(buffer, upload.start as u64, &bytes[upload.clone()]);
        }
        self.previous = bytes;
        allocate
    }
}
#[derive(Default)]
pub(super) struct Table {
    binding: Option<wgpu::BindGroup>,
    paints: Storage,
    ramps: Storage,
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
            *self = Self::default();
            return Ok(());
        };
        let mut paints = Vec::new();
        let mut ramps = Vec::new();
        let mut indices = std::collections::HashMap::<u64, Vec<(&Gradient, u32)>>::new();
        for gradient in gradients {
            let mapping = gradient
                .prepare(mapping)
                .map_err(pert_ui_runtime::DrawError::from)?;
            let entries = indices.entry(gradient.ramp_key()).or_default();
            let first = if let Some((_, first)) = entries
                .iter()
                .find(|(g, _)| g.space() == gradient.space() && g.stops() == gradient.stops())
            {
                *first
            } else {
                let first = u32::try_from(ramps.len() / 48).map_err(|_| LimitError::VertexCount)?;
                for ((offset, color), components) in
                    gradient.stops().iter().zip(gradient.components())
                {
                    for value in [*components, color.channels(), [*offset, 0., 0., 0.]]
                        .into_iter()
                        .flatten()
                    {
                        ramps.extend(value.to_le_bytes());
                    }
                }
                entries.push((gradient, first));
                first
            };
            let [x, y] = mapping.axis();
            paints.extend(
                mapping
                    .coefficients()
                    .into_iter()
                    .chain([x, y, gradient.inner().unwrap_or(-1.), 0.])
                    .flat_map(f32::to_le_bytes),
            );
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
            paints.extend(
                [gradient.stops().len() as u32, space, extend, first]
                    .into_iter()
                    .flat_map(u32::to_le_bytes),
            );
        }
        for bytes in [&paints, &ramps] {
            if bytes.len() as u64 > max {
                return Err(LimitError::GradientRecords {
                    bytes: bytes.len() as u64,
                    max,
                }
                .into());
            }
        }
        let paints_changed = self.paints.upload(device, queue, paints, max);
        let ramps_changed = self.ramps.upload(device, queue, ramps, max);
        if (paints_changed || ramps_changed)
            && let (Some(paints), Some(ramps)) = (&self.paints.buffer, &self.ramps.buffer)
        {
            self.binding = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("gradient paints"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: paints.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ramps.as_entire_binding(),
                    },
                ],
            }));
        }
        Ok(())
    }
    pub(super) fn binding(&self) -> Result<&wgpu::BindGroup, RenderError> {
        self.binding
            .as_ref()
            .ok_or_else(|| RenderError::Graphics("gradient records are missing".into()))
    }
}
