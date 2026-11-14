//! A headless wgpu device with the stroke pipeline and a model of today's
//! mask path, rendering offscreen into a frame-sized target.
use crate::stroke::Prepared;
use std::ops::Range;
use std::time::{Duration, Instant};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// What a pass draws: the strokes by one of the two paths, or today's masks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    /// Each tile's pixels evaluate the tile's segment list.
    Tiles,
    /// Each segment's quads write coverage into an atlas with a maximum
    /// blend; the stroke's tiles then composite it.
    Quads,
    /// Today's masks, uploaded by `upload_masks`.
    Masks,
}
impl Draw {
    /// The stroke path `PROBE_PATH` names, tiles by default.
    pub fn from_env() -> Result<Self, String> {
        match std::env::var("PROBE_PATH").as_deref() {
            Err(_) | Ok("tiles") => Ok(Self::Tiles),
            Ok("quads") => Ok(Self::Quads),
            Ok(other) => Err(format!("unknown PROBE_PATH {other}; use tiles or quads")),
        }
    }
}

/// The coverage atlas's smallest side (`PROBE_ATLAS`, default 2048); it is
/// never smaller than the frame, so any stroke's span fits.
fn atlas_side() -> u32 {
    std::env::var("PROBE_ATLAS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2048)
}

/// One stroke placed in the atlas.
struct Place {
    at: [u32; 2],
    size: [u32; 2],
    miters: bool,
    segments: Range<u32>,
    tiles: Range<u32>,
}

/// The quads path's pipelines and atlas.
struct Quads {
    segment: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    composite_coverage: wgpu::RenderPipeline,
    _atlas: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_bind: wgpu::BindGroup,
    size: [u32; 2],
    places: Vec<Place>,
    /// Runs of strokes, in drawing order, that share the atlas.
    batches: Vec<Range<usize>>,
    /// Whether the atlas still holds the last upload's coverage: a frame
    /// whose strokes fit one batch keeps it until the next upload, as a
    /// changing vector's mask is kept until it changes.
    kept: std::cell::Cell<bool>,
}

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: String,
    width: u32,
    height: u32,
    _color: wgpu::Texture,
    color_view: wgpu::TextureView,
    coverage: wgpu::Texture,
    coverage_view: wgpu::TextureView,
    stroke_color: wgpu::RenderPipeline,
    stroke_coverage: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    frame: wgpu::Buffer,
    buffers: [Option<wgpu::Buffer>; 5],
    bind: Option<wgpu::BindGroup>,
    bytes: [Vec<u8>; 5],
    tiles: u32,
    quads: Quads,
    mask: MaskPipeline,
    // Timestamps of the last frame's passes, where the adapter has them:
    // the query set, the resolve buffer and the read-back buffer. 0 is the
    // first pass's beginning and 1 the last pass's end; on the quads path,
    // 2 is the end of the last atlas pass and 3 the beginning of the last
    // composite pass.
    timestamps: Option<(wgpu::QuerySet, wgpu::Buffer, wgpu::Buffer)>,
    // How many timestamps the last frame wrote, and the quads path's split
    // read back with them.
    written: std::cell::Cell<u32>,
    split: std::cell::Cell<Option<(Duration, Duration)>>,
    period: f32,
}

/// Today's mask drawing: an R8 coverage texture per mask, sampled linearly
/// on a quad, as `pert-ui-wgpu`'s `masks.wgsl` does.
struct MaskPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    quads: Vec<(wgpu::BindGroup, wgpu::Buffer)>,
}

fn storage(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stroke probe target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

impl Gpu {
    pub fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
        adapters.retain(|a| {
            matches!(
                a.get_info().device_type,
                wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
            )
        });
        adapters.sort_by_key(|a| a.get_info().device_type != wgpu::DeviceType::IntegratedGpu);
        let adapter = adapters
            .into_iter()
            .next()
            .ok_or("no hardware Vulkan adapter")?;
        let info = adapter.get_info();
        let features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("stroke probe"),
                required_features: features,
                required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                ..Default::default()
            }))?;
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("strokes"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stroke.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("strokes"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage(1),
                storage(2),
                storage(3),
                storage(4),
                storage(5),
            ],
        });
        let atlas_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("atlas"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("strokes"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("composite"),
            bind_group_layouts: &[Some(&layout), Some(&atlas_layout)],
            immediate_size: 0,
        });
        let staged = |vertex: &str, entry: &str, layout: &wgpu::PipelineLayout, format, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline =
            |entry: &str, format, blend| staged("vertex", entry, &pipeline_layout, format, blend);
        let stroke_color = pipeline(
            "color",
            FORMAT,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let stroke_coverage = pipeline("coverage", wgpu::TextureFormat::R32Float, None);
        let maximum = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Max,
        };
        let segment = staged(
            "segment_vertex",
            "segment_coverage",
            &pipeline_layout,
            wgpu::TextureFormat::R8Unorm,
            Some(wgpu::BlendState {
                color: maximum,
                alpha: maximum,
            }),
        );
        let composite = staged(
            "composite_vertex",
            "composite",
            &composite_layout,
            FORMAT,
            Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
        );
        let composite_coverage = staged(
            "composite_vertex",
            "composite_coverage",
            &composite_layout,
            wgpu::TextureFormat::R32Float,
            None,
        );
        let side = atlas_side();
        let atlas_size = [width.max(side), height.max(side)];
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("coverage atlas"),
            size: wgpu::Extent3d {
                width: atlas_size[0],
                height: atlas_size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&Default::default());
        let atlas_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas"),
            layout: &atlas_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            }],
        });
        let quads = Quads {
            segment,
            composite,
            composite_coverage,
            _atlas: atlas,
            atlas_view,
            atlas_bind,
            size: atlas_size,
            places: Vec::new(),
            batches: Vec::new(),
            kept: std::cell::Cell::new(false),
        };
        let mask = MaskPipeline::new(&device);
        if let Some(error) = pollster::block_on(scope.pop()) {
            return Err(error.to_string().into());
        }
        let frame = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut values = Vec::new();
        for v in [
            width as f32,
            height as f32,
            atlas_size[0] as f32,
            atlas_size[1] as f32,
        ] {
            values.extend_from_slice(&v.to_le_bytes());
        }
        queue.write_buffer(&frame, 0, &values);
        let color = target(&device, width, height, FORMAT);
        let coverage = target(&device, width, height, wgpu::TextureFormat::R32Float);
        Ok(Self {
            adapter: format!("{} ({:?}, {})", info.name, info.backend, info.driver_info),
            color_view: color.create_view(&Default::default()),
            coverage_view: coverage.create_view(&Default::default()),
            _color: color,
            coverage,
            width,
            height,
            stroke_color,
            stroke_coverage,
            layout,
            frame,
            buffers: Default::default(),
            bind: None,
            bytes: Default::default(),
            tiles: 0,
            quads,
            mask,
            timestamps: features.contains(wgpu::Features::TIMESTAMP_QUERY).then(|| {
                let buffer = |usage| {
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("timestamps"),
                        size: 32,
                        usage,
                        mapped_at_creation: false,
                    })
                };
                (
                    device.create_query_set(&wgpu::QuerySetDescriptor {
                        label: Some("pass"),
                        ty: wgpu::QueryType::Timestamp,
                        count: 4,
                    }),
                    buffer(wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC),
                    buffer(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ),
                )
            }),
            written: std::cell::Cell::new(2),
            split: std::cell::Cell::new(None),
            period: queue.get_timestamp_period(),
            device,
            queue,
        })
    }

    /// Encodes and uploads a frame's strokes, growing a buffer only when it
    /// is too small. This and the preparation are the "prepare" interval.
    pub fn upload(&mut self, prepared: &Prepared) {
        let [records @ .., regions] = &mut self.bytes;
        prepared.encode(records);
        regions.clear();
        self.quads.place(prepared, regions);
        self.quads.kept.set(false);
        let mut replaced = self.bind.is_none();
        for (slot, bytes) in self.buffers.iter_mut().zip(&self.bytes) {
            let required = (bytes.len() as u64).max(64);
            if slot.as_ref().is_none_or(|b| b.size() < required) {
                *slot = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("stroke records"),
                    size: required.next_power_of_two(),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                replaced = true;
            }
            if let Some(buffer) = slot
                && !bytes.is_empty()
            {
                self.queue.write_buffer(buffer, 0, bytes);
            }
        }
        if replaced {
            let [Some(a), Some(b), Some(c), Some(d), Some(e)] = &self.buffers else {
                unreachable!("every buffer exists");
            };
            self.bind = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("strokes"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.frame.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: b.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: c.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: d.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: e.as_entire_binding(),
                    },
                ],
            }));
        }
        self.tiles = prepared.tiles.len() as u32;
    }

    /// The quads path's atlas batches for the last `upload`.
    pub fn batches(&self) -> usize {
        self.quads.batches()
    }

    /// Bytes uploaded by the last `upload`.
    pub fn uploaded_bytes(&self) -> usize {
        self.bytes.iter().map(Vec::len).sum()
    }

    /// One offscreen pass: encode, submit and wait for completion. The
    /// "GPU pass" interval, as the real-time probe's.
    pub fn pass(&self, draw: Draw, coverage: bool) -> Result<Duration, Box<dyn std::error::Error>> {
        let start = Instant::now();
        self.written.set(2);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        if draw == Draw::Quads {
            self.encode_quads(&mut encoder, coverage);
        } else {
            let (view, clear) = self.target(coverage);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("stroke probe"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                timestamp_writes: self.timestamps.as_ref().map(|(set, ..)| {
                    wgpu::RenderPassTimestampWrites {
                        query_set: set,
                        beginning_of_pass_write_index: Some(0),
                        end_of_pass_write_index: Some(1),
                    }
                }),
                ..Default::default()
            });
            if draw == Draw::Masks {
                pass.set_pipeline(&self.mask.pipeline);
                for (bind, quad) in &self.mask.quads {
                    pass.set_bind_group(0, bind, &[]);
                    pass.set_vertex_buffer(0, quad.slice(..));
                    pass.draw(0..6, 0..1);
                }
            } else if self.tiles > 0
                && let Some(bind) = &self.bind
            {
                pass.set_pipeline(if coverage {
                    &self.stroke_coverage
                } else {
                    &self.stroke_color
                });
                pass.set_bind_group(0, bind, &[]);
                pass.draw(0..4, 0..self.tiles);
            }
        }
        self.finish(encoder, start)
    }

    /// The target and clear color of a pass.
    fn target(&self, coverage: bool) -> (&wgpu::TextureView, wgpu::Color) {
        if coverage {
            (&self.coverage_view, wgpu::Color::TRANSPARENT)
        } else {
            (
                &self.color_view,
                wgpu::Color {
                    r: 0.02,
                    g: 0.02,
                    b: 0.03,
                    a: 1.0,
                },
            )
        }
    }

    /// The quads path: per batch, a coverage pass into the atlas and a
    /// composite pass into the target; only the composite while the atlas
    /// keeps the last upload's coverage. The timestamps span all of them.
    fn encode_quads(&self, encoder: &mut wgpu::CommandEncoder, coverage: bool) {
        let (view, clear) = self.target(coverage);
        let quads = &self.quads;
        let batches = quads.batches.len();
        let stamps = |begin: Option<u32>, end: Option<u32>| {
            self.timestamps
                .as_ref()
                .filter(|_| begin.is_some() || end.is_some())
                .map(|(set, ..)| wgpu::RenderPassTimestampWrites {
                    query_set: set,
                    beginning_of_pass_write_index: begin,
                    end_of_pass_write_index: end,
                })
        };
        if batches == 0 || self.bind.is_none() {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                timestamp_writes: stamps(Some(0), Some(1)),
                ..Default::default()
            });
            return;
        }
        let Some(bind) = &self.bind else { return };
        let kept = batches == 1 && quads.kept.get();
        self.written.set(if kept { 2 } else { 4 });
        for (n, batch) in quads.batches.iter().enumerate() {
            if !kept {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("coverage atlas"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &quads.atlas_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    timestamp_writes: stamps(
                        (n == 0).then_some(0),
                        (n + 1 == batches).then_some(2),
                    ),
                    ..Default::default()
                });
                pass.set_pipeline(&quads.segment);
                pass.set_bind_group(0, bind, &[]);
                for place in &quads.places[batch.clone()] {
                    pass.set_scissor_rect(place.at[0], place.at[1], place.size[0], place.size[1]);
                    pass.draw(0..4, place.segments.clone());
                    if place.miters {
                        pass.draw(4..8, place.segments.clone());
                    }
                }
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: if n == 0 {
                            wgpu::LoadOp::Clear(clear)
                        } else {
                            wgpu::LoadOp::Load
                        },
                        store: wgpu::StoreOp::Store,
                    },
                })],
                timestamp_writes: stamps(
                    (n + 1 == batches).then_some(if kept { 0 } else { 3 }),
                    (n + 1 == batches).then_some(1),
                ),
                ..Default::default()
            });
            pass.set_pipeline(if coverage {
                &quads.composite_coverage
            } else {
                &quads.composite
            });
            pass.set_bind_group(0, bind, &[]);
            pass.set_bind_group(1, &quads.atlas_bind, &[]);
            let tiles =
                quads.places[batch.start].tiles.start..quads.places[batch.end - 1].tiles.end;
            if !tiles.is_empty() {
                pass.draw(0..4, tiles);
            }
        }
        quads.kept.set(batches == 1);
    }

    /// Resolves the timestamps, submits and waits.
    fn finish(
        &self,
        mut encoder: wgpu::CommandEncoder,
        start: Instant,
    ) -> Result<Duration, Box<dyn std::error::Error>> {
        if let Some((set, resolve, read)) = &self.timestamps {
            let written = self.written.get();
            encoder.resolve_query_set(set, 0..written, resolve, 0);
            encoder.copy_buffer_to_buffer(resolve, 0, read, 0, u64::from(written) * 8);
        }
        let submission = self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(5)),
        })?;
        Ok(start.elapsed())
    }

    /// The GPU's own time between the beginning of the last frame's first
    /// pass and the end of its last, read back outside the timed interval;
    /// `None` without timestamp support. On the quads path it also reads
    /// the split `last_split` returns.
    pub fn last_execution(&self) -> Result<Option<Duration>, Box<dyn std::error::Error>> {
        let Some((_, _, read)) = &self.timestamps else {
            return Ok(None);
        };
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(5)),
        })?;
        receive.recv_timeout(Duration::from_secs(5))??;
        let ticks = {
            let mapped = read.slice(..).get_mapped_range()?;
            let value = |i: usize| {
                u64::from_le_bytes(mapped[i * 8..i * 8 + 8].try_into().unwrap_or([0; 8]))
            };
            let split = (self.written.get() == 4).then(|| {
                (
                    value(2).saturating_sub(value(0)),
                    value(1).saturating_sub(value(3)),
                )
            });
            self.split
                .set(split.map(|(a, b)| (self.duration(a), self.duration(b))));
            value(1).saturating_sub(value(0))
        };
        read.unmap();
        Ok(Some(self.duration(ticks)))
    }

    fn duration(&self, ticks: u64) -> Duration {
        Duration::from_nanos((ticks as f64 * f64::from(self.period)) as u64)
    }

    /// On the quads path, the last frame's time from its first pass's
    /// beginning to its last atlas pass's end, and its last composite
    /// pass's own time; the two are the whole frame when it has one batch.
    /// `None` when the frame drew no atlas pass.
    pub fn last_split(&self) -> Option<(Duration, Duration)> {
        self.split.get()
    }

    /// The GPU's current clock in MHz where the kernel reports it (Linux
    /// i915), read outside the timed intervals.
    pub fn clock() -> Option<u32> {
        let entries = std::fs::read_dir("/sys/class/drm").ok()?;
        entries.flatten().find_map(|entry| {
            std::fs::read_to_string(entry.path().join("gt_cur_freq_mhz"))
                .ok()?
                .trim()
                .parse()
                .ok()
        })
    }

    /// Uploads today's masks: a texture per mask, created and written for
    /// every frame, as a changing vector's new coverage is.
    pub fn upload_masks(&mut self, masks: &[(pert_ui::MaskImage, [f32; 4], [f32; 4])]) {
        self.mask.quads.clear();
        for (mask, rect, color) in masks {
            let size = wgpu::Extent3d {
                width: mask.width(),
                height: mask.height(),
                depth_or_array_layers: 1,
            };
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("mask"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                mask.pixels(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(mask.width()),
                    rows_per_image: Some(mask.height()),
                },
                size,
            );
            let view = texture.create_view(&Default::default());
            let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mask"),
                layout: &self.mask.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.mask.sampler),
                    },
                ],
            });
            // Two triangles in normalized device coordinates with UVs.
            let [x, y, w, h] = *rect;
            let (l, r) = (
                2.0 * x / self.width as f32 - 1.0,
                2.0 * (x + w) / self.width as f32 - 1.0,
            );
            let (t, b) = (
                1.0 - 2.0 * y / self.height as f32,
                1.0 - 2.0 * (y + h) / self.height as f32,
            );
            let mut bytes = Vec::with_capacity(6 * 32);
            for [px, py, u, v] in [
                [l, t, 0.0, 0.0],
                [l, b, 0.0, 1.0],
                [r, t, 1.0, 0.0],
                [r, t, 1.0, 0.0],
                [l, b, 0.0, 1.0],
                [r, b, 1.0, 1.0],
            ] {
                for value in [px, py].into_iter().chain(*color).chain([u, v]) {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
            let quad = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("mask quad"),
                size: bytes.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&quad, 0, &bytes);
            self.mask.quads.push((bind, quad));
        }
    }

    /// Reads the coverage target back as f32 per pixel.
    pub fn read_coverage(&self) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let bytes = self.read(&self.coverage)?;
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect())
    }

    fn read(&self, texture: &wgpu::Texture) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let (width, height) = (self.width, self.height);
        let unpadded = width as usize * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read-back"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(5)),
        })?;
        receive.recv_timeout(Duration::from_secs(5))??;
        let mapped = buffer.slice(..).get_mapped_range()?;
        let mut out = Vec::with_capacity(unpadded * height as usize);
        for row in mapped.chunks(padded) {
            out.extend_from_slice(&row[..unpadded]);
        }
        Ok(out)
    }
}

impl Quads {
    /// Places each stroke's span in the atlas, shelf by shelf in drawing
    /// order, starting a new batch when the atlas is full; writes the
    /// regions, one `vec4<u32>` per stroke.
    fn place(&mut self, prepared: &Prepared, regions: &mut Vec<u8>) {
        self.places.clear();
        self.batches.clear();
        let [width, height] = self.size;
        let (mut x, mut y, mut shelf, mut first) = (0, 0, 0, 0);
        for (i, extent) in prepared.extents.iter().enumerate() {
            let size = [
                extent.span[2] - extent.span[0],
                extent.span[3] - extent.span[1],
            ];
            if x + size[0] > width {
                (x, y, shelf) = (0, y + shelf, 0);
            }
            if y + size[1] > height {
                self.batches.push(first..i);
                (x, y, shelf, first) = (0, 0, 0, i);
            }
            self.places.push(Place {
                at: [x, y],
                size,
                miters: extent.miters,
                segments: extent.segments.clone(),
                tiles: extent.tiles.clone(),
            });
            for value in [x, y, 0, 0] {
                regions.extend_from_slice(&value.to_le_bytes());
            }
            x += size[0];
            shelf = shelf.max(size[1]);
        }
        if first < self.places.len() {
            self.batches.push(first..self.places.len());
        }
    }

    pub fn batches(&self) -> usize {
        self.batches.len()
    }
}

impl MaskPipeline {
    fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("masks"),
            source: wgpu::ShaderSource::Wgsl(MASKS_WGSL.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mask"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("masks"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("masks"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: 32,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("mask"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            pipeline,
            layout,
            sampler,
            quads: Vec::new(),
        }
    }
}

// pert-ui-wgpu's masks.wgsl, unchanged.
const MASKS_WGSL: &str = r"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};
@group(0) @binding(0) var coverage: texture_2d<f32>;
@group(0) @binding(1) var coverage_sampler: sampler;
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) color: vec4<f32>, @location(2) uv: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(position, 0.0, 1.0);
    output.color = color;
    output.uv = uv;
    return output;
}
@fragment fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(coverage, coverage_sampler, input.uv).r;
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
";
