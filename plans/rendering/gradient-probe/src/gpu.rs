// Offscreen sRGB render pass on a hardware adapter. Timings synchronize the
// queue and exclude readback; they include command encoding and submission.
use super::sample::{Params, Stop};
use std::time::Duration;
use wgpu::*;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Gpu {
    device: Device,
    queue: Queue,
    pipeline: RenderPipeline,
    bind: BindGroup,
    params: Buffer,
    stops: Buffer,
    image: Texture,
}
pub struct Frame {
    target: Texture,
    view: TextureView,
    width: u32,
    height: u32,
}
impl Gpu {
    pub fn new() -> Result<Self> {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.enumerate_adapters(Backends::VULKAN))
            .into_iter()
            .find(|a| {
                matches!(
                    a.get_info().device_type,
                    DeviceType::IntegratedGpu | DeviceType::DiscreteGpu
                )
            })
            .ok_or("no hardware Vulkan adapter")?;
        println!("adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default()))?;
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(include_str!("paint.wgsl").into()),
        });
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let buffer = |size, usage| {
            device.create_buffer(&BufferDescriptor {
                label: None,
                size,
                usage: usage | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let params = buffer(64, BufferUsages::UNIFORM);
        let stops = buffer(32 * 65, BufferUsages::STORAGE);
        let lut = buffer(4097 * 8, BufferUsages::STORAGE);
        queue.write_buffer(&lut, 0, bytemuck::cast_slice(super::sample::transfers()));
        let image = device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: 256,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = image.create_view(&Default::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..Default::default()
        });
        let bind = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: params.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: stops.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: lut.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        Ok(Self {
            device,
            queue,
            pipeline,
            bind,
            params,
            stops,
            image,
        })
    }
    pub fn frame(&self, width: u32, height: u32) -> Frame {
        let target = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = target.create_view(&Default::default());
        Frame {
            target,
            view,
            width,
            height,
        }
    }
    pub fn params(&self, p: &Params) {
        self.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(p));
    }
    pub fn stops(&self, s: &[Stop]) {
        self.queue
            .write_buffer(&self.stops, 0, bytemuck::cast_slice(s));
    }
    pub fn image(&self, bytes: &[u8]) {
        self.queue.write_texture(
            self.image.as_image_copy(),
            bytes,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024),
                rows_per_image: None,
            },
            self.image.size(),
        );
    }
    pub fn draw(&self, f: &Frame, bg: f64, clip: u32) {
        self.draw_rects(f, bg, &[[0, 0, clip.min(f.width), f.height]]);
    }
    pub fn draw_rects(&self, f: &Frame, bg: f64, rects: &[[u32; 4]]) {
        let mut enc = self.device.create_command_encoder(&Default::default());
        let color = bg.max(0.);
        {
            let mut pass = enc.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &f.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: color,
                            g: color,
                            b: color,
                            a: if bg < 0. { 0. } else { 1. },
                        }),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            for &[x, y, w, h] in rects {
                pass.set_scissor_rect(x, y, w, h);
                pass.draw(0..3, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
    }
    pub fn wait(&self) -> Result<()> {
        self.device.poll(PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        Ok(())
    }
    pub fn read(&self, f: &Frame) -> Result<Vec<u8>> {
        let row = (f.width * 4).div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&BufferDescriptor {
            label: None,
            size: u64::from(row) * u64::from(f.height),
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            f.target.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: None,
                },
            },
            f.target.size(),
        );
        self.queue.submit([enc.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.wait()?;
        rx.recv()??;
        let data = buffer.slice(..).get_mapped_range()?;
        Ok(data
            .chunks_exact(row as usize)
            .flat_map(|r| r[..f.width as usize * 4].iter().copied())
            .collect())
    }
}
