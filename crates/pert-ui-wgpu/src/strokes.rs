//! The stroke pipelines: a stroke is covered on the GPU from its prepared
//! segment records, its curves flattened, instead of from a CPU mask.
//!
//! Each [`DrawCommand::DrawVector`] the rasterizer leaves in the scene is
//! prepared with [`PreparedStroke`]. Its span, of whole frame pixels, gets a
//! region of the same size in a *coverage page*, a half-float texture. A first
//! render pass per page draws one quad per segment, and a second around a
//! miter's wedge, into its stroke's region under a scissor, with a maximum
//! blend; the fragment stage evaluates [`StrokeSegment::distance`]. The scene
//! pass then draws the stroke's covered tiles, merged into runs along each
//! row, as ordinary quads that read the page. The pages keep their coverage
//! while the frame's strokes do not change.
//!
//! A segment record is four 16-byte groups, 64 bytes, little-endian as WGSL
//! lays host-shared values out (the `Segment` struct of `strokes.wgsl`):
//! `a`, `b`; `d`, `n`; the end `m`, the end `k`, the start `k`; the start
//! `m`, the flags and the stroke's index in the frame, both `u32`. A stroke record is two groups: the half width, the thin-stroke
//! factor and the place of the span's origin in its page; and two over the
//! page's width and height.
use super::*;
use pert_ui_runtime::{FrameMapping, Point, PreparedStroke, Rect, StrokeSegment, VectorGeometry};

/// The size of one segment record in the storage buffer.
pub(super) const SEGMENT_BYTES: usize = 64;

/// A page's format: half floats, whose precision is relative, so that the
/// faintest coverage keeps its value. Eight bits would move it by up to
/// half of 1/255, which is several steps of a bright stroke over a dark
/// destination once the result is encoded as sRGB.
const PAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

/// The bytes one pixel of a page holds.
const PAGE_PIXEL_BYTES: u64 = 2;

/// The most bytes a frame's coverage pages may hold beyond its first page:
/// as many pixels as the mask budget holds mask pixels.
pub(super) const COVERAGE_BUDGET: u64 = 2 * ResourceKind::Mask.scene_budget_bytes() as u64;

/// The smallest side of a coverage page, in pixels. A page is never smaller
/// than the frame, so that every stroke's span fits one.
const PAGE_SIDE: u32 = 2048;

/// One prepared stroke of a frame, before it has a place in a page.
#[derive(Debug, PartialEq)]
pub(super) struct Encoded {
    // The span in frame pixels.
    span: [u32; 4],
    h: f32,
    thin: f32,
    // Straight linear color, as the scene's vertices carry it.
    color: [f32; 4],
    // Whether a segment has a miter, whose wedge is drawn apart.
    miters: bool,
    // The segment records, each with a stroke index of zero.
    segments: Vec<u8>,
    // The covered tiles as frame pixels, merged along each row.
    runs: Vec<[u32; 4]>,
}

/// Prepares one stroke command for a frame; `None` when it draws nothing
/// there.
pub(super) fn encode(
    vector: &VectorGeometry,
    origin: Point,
    scale: f32,
    color: Color,
    clip: Option<Rect>,
    mapping: &FrameMapping,
) -> Result<Option<Encoded>, RenderError> {
    // A transparent stroke is skipped before its scale is checked, as the
    // rasterizer skips one.
    if color.alpha() == 0.0 {
        return Ok(None);
    }
    let Some(stroke) = PreparedStroke::new(vector, origin, scale, clip, mapping)? else {
        return Ok(None);
    };
    let mut runs: Vec<[u32; 4]> = Vec::new();
    for tile in stroke.tiles() {
        match runs.last_mut() {
            Some(run) if run[1] == tile[1] && run[2] == tile[0] => run[2] = tile[2],
            _ => runs.push(tile),
        }
    }
    let mut miters = false;
    let mut segments = Vec::with_capacity(stroke.segments().len() * SEGMENT_BYTES);
    for segment in stroke.segments() {
        miters |= (segment.flags >> 6) & 3 == 3;
        segments.extend(record(segment).iter().flat_map(|value| value.to_le_bytes()));
    }
    Ok(Some(Encoded {
        span: stroke.span(),
        h: stroke.half_width(),
        thin: stroke.thin(),
        color: color.channels(),
        miters,
        segments,
        runs,
    }))
}

/// A segment's sixteen record values, its stroke index zero.
fn record(segment: &StrokeSegment) -> [f32; 16] {
    let StrokeSegment {
        a,
        b,
        d,
        n,
        end_m,
        end_k,
        start_m,
        start_k,
        flags,
        ..
    } = *segment;
    [
        a[0],
        a[1],
        b[0],
        b[1],
        d[0],
        d[1],
        n[0],
        n[1],
        end_m[0],
        end_m[1],
        end_k,
        start_k,
        start_m[0],
        start_m[1],
        f32::from_bits(flags),
        f32::from_bits(0),
    ]
}

/// Where one stroke of the frame is drawn in the coverage pages.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Place {
    page: u32,
    // The region: its corner in the page and its size, in pixels.
    at: [u32; 2],
    size: [u32; 2],
    // The stroke's records in the frame's segment buffer.
    segments: std::ops::Range<u32>,
    miters: bool,
}

/// The strokes of one frame, placed in coverage pages in drawing order.
#[derive(Debug, Default, PartialEq)]
pub(super) struct Frame {
    // The size of every page.
    side: [u32; 2],
    // The segment records of all strokes, `SEGMENT_BYTES` each.
    pub(super) segments: Vec<u8>,
    // The stroke records, 32 bytes each.
    pub(super) strokes: Vec<u8>,
    places: Vec<Place>,
    // Shelf packing: the page being filled, the next free corner on its
    // current shelf and that shelf's height.
    page: u32,
    cursor: [u32; 2],
    shelf: u32,
}

impl Frame {
    /// An empty frame whose pages hold any span of `mapping`'s frame.
    pub(super) fn new(mapping: Option<&FrameMapping>) -> Self {
        Self {
            side: mapping.map_or([PAGE_SIDE; 2], |mapping| {
                [
                    mapping.frame_width().max(PAGE_SIDE),
                    mapping.frame_height().max(PAGE_SIDE),
                ]
            }),
            ..Self::default()
        }
    }

    /// How many pages the frame's strokes use.
    pub(super) fn pages(&self) -> u32 {
        if self.places.is_empty() {
            0
        } else {
            self.page + 1
        }
    }

    /// Places `stroke` in a page, appends its records, and appends to
    /// `vertices` the quads of its runs for a frame of `mapping`. Returns the
    /// page they read.
    pub(super) fn push(
        &mut self,
        stroke: &Encoded,
        mapping: &FrameMapping,
        record_budget: u64,
        vertices: &mut Vec<u8>,
    ) -> Result<u32, RenderError> {
        let [x0, y0, x1, y1] = stroke.span;
        let size = [x1 - x0, y1 - y0];
        // A span lies within the frame, and a page is at least that large.
        if self.cursor[0] + size[0] > self.side[0] {
            self.cursor = [0, self.cursor[1] + self.shelf];
            self.shelf = 0;
        }
        if self.cursor[1] + size[1] > self.side[1] {
            self.page += 1;
            self.cursor = [0, 0];
            self.shelf = 0;
        }
        // The first page is always allowed, so that a frame larger than
        // the budget can still draw strokes.
        let pages = u64::from(self.page) + 1;
        let bytes = pages * u64::from(self.side[0]) * u64::from(self.side[1]) * PAGE_PIXEL_BYTES;
        if self.page > 0 && bytes > COVERAGE_BUDGET {
            return Err(LimitError::StrokeCoverage {
                bytes,
                max: COVERAGE_BUDGET,
            }
            .into());
        }
        let at = self.cursor;
        self.cursor[0] += size[0];
        self.shelf = self.shelf.max(size[1]);

        let index = u32::try_from(self.places.len()).map_err(|_| LimitError::VertexCount)?;
        let first = self.segments.len() / SEGMENT_BYTES;
        let bytes = (self.segments.len() + stroke.segments.len()) as u64;
        if bytes > record_budget {
            return Err(LimitError::StrokeRecords {
                bytes,
                max: record_budget,
            }
            .into());
        }
        self.segments.extend_from_slice(&stroke.segments);
        // Each record's last value is its stroke's index, as bits.
        for record in self.segments[first * SEGMENT_BYTES..]
            .as_chunks_mut::<SEGMENT_BYTES>()
            .0
        {
            record[SEGMENT_BYTES - 4..].copy_from_slice(&index.to_le_bytes());
        }
        let last = self.segments.len() / SEGMENT_BYTES;
        let range = u32::try_from(first).map_err(|_| LimitError::VertexCount)?
            ..u32::try_from(last).map_err(|_| LimitError::VertexCount)?;
        for value in [
            stroke.h,
            stroke.thin,
            at[0] as f32,
            at[1] as f32,
            2.0 / self.side[0] as f32,
            2.0 / self.side[1] as f32,
            0.0,
            0.0,
        ] {
            self.strokes.extend_from_slice(&value.to_le_bytes());
        }
        self.places.push(Place {
            page: self.page,
            at,
            size,
            segments: range,
            miters: stroke.miters,
        });

        // Whole pixel edges, as a shape's: rounding them never decides a
        // pixel. The offset from a frame pixel to its texel is whole too.
        let (width, height) = (
            f64::from(mapping.frame_width()),
            f64::from(mapping.frame_height()),
        );
        let offset = [
            (f64::from(at[0]) - f64::from(x0)) as f32,
            (f64::from(at[1]) - f64::from(y0)) as f32,
        ];
        for run in &stroke.runs {
            let l = (2.0 * f64::from(run[0]) / width - 1.0) as f32;
            let t = (1.0 - 2.0 * f64::from(run[1]) / height) as f32;
            let r = (2.0 * f64::from(run[2]) / width - 1.0) as f32;
            let b = (1.0 - 2.0 * f64::from(run[3]) / height) as f32;
            for [x, y] in [[l, t], [l, b], [r, t], [r, t], [l, b], [r, b]] {
                for value in [x, y].into_iter().chain(stroke.color).chain(offset) {
                    vertices.extend_from_slice(&value.to_ne_bytes());
                }
            }
        }
        Ok(self.page)
    }
}

/// The stroke pipelines of one surface format.
#[derive(Clone)]
pub(super) struct Pipelines {
    coverage: wgpu::RenderPipeline,
    pub(super) composite: wgpu::RenderPipeline,
    records: wgpu::BindGroupLayout,
    page: wgpu::BindGroupLayout,
}

impl Pipelines {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let storage = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let records = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pert-ui stroke records"),
            entries: &[storage(0), storage(1)],
        });
        let page = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pert-ui stroke coverage page"),
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pert-ui stroke coverage"),
            source: wgpu::ShaderSource::Wgsl(include_str!("strokes.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pert-ui stroke coverage"),
            bind_group_layouts: &[Some(&records)],
            immediate_size: 0,
        });
        // The largest coverage wins, which is the smallest distance's.
        let maximum = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Max,
        };
        let coverage = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pert-ui stroke coverage"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: PAGE_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: maximum,
                        alpha: maximum,
                    }),
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
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pert-ui stroke composite"),
            source: wgpu::ShaderSource::Wgsl(include_str!("stroke_composite.wgsl").into()),
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pert-ui stroke composite"),
            bind_group_layouts: &[Some(&page)],
            immediate_size: 0,
        });
        let composite = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pert-ui stroke composite"),
            layout: Some(&layout),
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
                // Straight color and the coverage as alpha, as for a mask.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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
        Self {
            coverage,
            composite,
            records,
            page,
        }
    }
}

/// One renderer's coverage pages and record buffers.
#[derive(Default)]
pub(super) struct Atlas {
    // The size the pages were created with.
    side: [u32; 2],
    // Each page's view, to draw into, and its binding, to read.
    pages: Vec<(wgpu::TextureView, wgpu::BindGroup)>,
    segments: Option<wgpu::Buffer>,
    strokes: Option<wgpu::Buffer>,
    records: Option<wgpu::BindGroup>,
    // The strokes the pages hold coverage of, when they hold any.
    drawn: std::cell::RefCell<Option<Vec<Place>>>,
}

impl Atlas {
    /// Makes the pages and buffers hold `frame`: uploads the records that
    /// differ from `before`'s and creates missing pages. Nothing here fails.
    pub(super) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipelines: &Pipelines,
        before: &Frame,
        frame: &Frame,
    ) {
        let pages = frame.pages() as usize;
        if self.side != frame.side {
            self.pages.clear();
            self.side = frame.side;
        }
        // Obsolete coverage is released; the buffers keep their capacity.
        self.pages.truncate(pages);
        while self.pages.len() < pages {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("pert-ui stroke coverage page"),
                size: wgpu::Extent3d {
                    width: self.side[0],
                    height: self.side[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: PAGE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pert-ui stroke coverage page"),
                layout: &pipelines.page,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                }],
            });
            self.pages.push((view, bind));
        }
        if pages == 0 {
            *self.drawn.get_mut() = None;
            return;
        }
        let limit = device.limits().max_buffer_size;
        let replaced = [
            (
                &mut self.segments,
                &before.segments,
                &frame.segments,
                "pert-ui stroke segments",
            ),
            (
                &mut self.strokes,
                &before.strokes,
                &frame.strokes,
                "pert-ui stroke records",
            ),
        ]
        .map(|(slot, before, after, label)| {
            let mut upload = changed_bytes(before, after);
            let required = after.len() as u64;
            let replace = slot.as_ref().is_none_or(|buffer| buffer.size() < required);
            if replace {
                let capacity = required
                    .checked_next_power_of_two()
                    .unwrap_or(required)
                    .min(limit)
                    .max(required);
                upload = 0..after.len();
                *slot = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: capacity,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            if let Some(buffer) = slot.as_ref()
                && !upload.is_empty()
            {
                queue.write_buffer(buffer, upload.start as u64, &after[upload]);
            }
            replace
        });
        if (replaced[0] || replaced[1] || self.records.is_none())
            && let (Some(segments), Some(strokes)) = (&self.segments, &self.strokes)
        {
            self.records = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pert-ui stroke records"),
                layout: &pipelines.records,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: segments.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: strokes.as_entire_binding(),
                    },
                ],
            }));
        }
        // The pages keep their coverage only while every record and place
        // is what they were drawn from.
        if before.segments != frame.segments
            || before.strokes != frame.strokes
            || self.drawn.get_mut().as_ref() != Some(&frame.places)
        {
            *self.drawn.get_mut() = None;
        }
    }

    /// Encodes the passes that draw `frame`'s coverage into the pages,
    /// unless the pages already hold it.
    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipelines: &Pipelines,
        frame: &Frame,
    ) -> Result<(), RenderError> {
        if frame.places.is_empty() || self.drawn.borrow().as_ref() == Some(&frame.places) {
            return Ok(());
        }
        let missing = || RenderError::Graphics("stroke coverage pages are missing".into());
        let records = self.records.as_ref().ok_or_else(missing)?;
        for (index, (view, _)) in self.pages.iter().enumerate() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pert-ui stroke coverage"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&pipelines.coverage);
            pass.set_bind_group(0, records, &[]);
            for place in frame
                .places
                .iter()
                .filter(|place| place.page as usize == index)
            {
                pass.set_scissor_rect(place.at[0], place.at[1], place.size[0], place.size[1]);
                pass.draw(0..4, place.segments.clone());
                if place.miters {
                    pass.draw(4..8, place.segments.clone());
                }
            }
        }
        if frame.pages() as usize > self.pages.len() {
            return Err(missing());
        }
        *self.drawn.borrow_mut() = Some(frame.places.clone());
        Ok(())
    }

    /// The binding of page `index`, for the composite.
    pub(super) fn page(&self, index: usize) -> Result<&wgpu::BindGroup, RenderError> {
        self.pages
            .get(index)
            .map(|(_, bind)| bind)
            .ok_or_else(|| RenderError::Graphics("a stroke coverage page is missing".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui_runtime::{
        Component, DrawError, Element, Length, LineCap, LineJoin, Painter, Runtime, RuntimeError,
        StrokeStyle, UpdateContext, Viewport,
    };

    struct Lines(LineCap, LineJoin);
    impl Component for Lines {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            let style = StrokeStyle::new(Length::new(4.0)?)
                .with_cap(self.0)
                .with_join(self.1);
            p.stroke_polyline(
                &[
                    Point::new(8.0, 8.0)?,
                    Point::new(40.0, 12.0)?,
                    Point::new(20.0, 40.0)?,
                ],
                style,
                Color::WHITE,
            )
        }
    }

    /// Strokes each polyline, two pixels wide, in a square viewport.
    struct Polylines(Vec<Vec<(f32, f32)>>);
    impl Component for Polylines {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            for line in &self.0 {
                let points = line
                    .iter()
                    .map(|&(x, y)| Point::new(x, y))
                    .collect::<Result<Vec<_>, _>>()?;
                p.stroke_polyline(&points, StrokeStyle::new(Length::new(2.0)?), Color::WHITE)?;
            }
            Ok(())
        }
    }

    fn polylines(
        side: f32,
        lines: &[&[(f32, f32)]],
    ) -> Result<DisplayList, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(side, side, 1.0)?)?;
        let lines = lines.iter().map(|line| line.to_vec()).collect();
        ui.mount_tree(Element::new("lines", Polylines(lines)))?;
        ui.flush(std::time::Duration::ZERO)?;
        // As the renderer prepares it: strokes stay vectors.
        Ok(pert_ui_runtime::VectorRasterizer::default().prepare_fills(&ui.paint()?)?)
    }

    #[test]
    fn a_stroke_is_encoded_again_when_its_geometry_changes()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::retained::RetainedGeometry;
        let first = polylines(
            64.0,
            &[&[(4.0, 8.0), (60.0, 8.0)], &[(4.0, 40.0), (60.0, 44.0)]],
        )?;
        let cache = RetainedGeometry::default().prepare(&first)?;
        assert_eq!(cache.encoded_commands(), 2);
        assert_eq!(cache.strokes.pages(), 1);
        // The same scene reuses both strokes and places them alike.
        let again = cache.prepare(&first)?;
        assert_eq!(again.encoded_commands(), 0);
        assert_eq!(again.strokes, cache.strokes);
        assert_eq!(
            (&again.bytes, &again.batches),
            (&cache.bytes, &cache.batches)
        );
        // Other points in the second stroke: its command is equal, since a
        // vector's ID is its index in its own scene, but its geometry is not.
        let moved = polylines(
            64.0,
            &[&[(4.0, 8.0), (60.0, 8.0)], &[(4.0, 40.0), (60.0, 50.0)]],
        )?;
        assert_eq!(moved.commands(), first.commands());
        let next = cache.prepare(&moved)?;
        assert_eq!(next.encoded_commands(), 1);
        assert_ne!(next.strokes.segments, cache.strokes.segments);
        assert_eq!(
            next.strokes,
            RetainedGeometry::default().prepare(&moved)?.strokes
        );
        Ok(())
    }

    #[test]
    fn runs_merge_tiles_along_a_row_and_read_the_strokes_region()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::retained::RetainedGeometry;
        // Two horizontal lines: each covers one row of tiles across its span.
        let scene = polylines(
            64.0,
            &[&[(4.0, 8.0), (60.0, 8.0)], &[(4.0, 40.0), (60.0, 40.0)]],
        )?;
        let frame = RetainedGeometry::default().prepare(&scene)?;
        assert_eq!(frame.strokes.places.len(), 2);
        // One run of six vertices per stroke, in one batch on page 0.
        assert_eq!(frame.bytes.len(), 2 * 6 * 32);
        assert_eq!(
            frame.batches,
            [Batch {
                range: 0..12,
                source: Source::Stroke(0)
            }]
        );
        // The second stroke lies beside the first on the page's first shelf,
        // and its vertices carry the step from its span to its region.
        let [first, second] = [&frame.strokes.places[0], &frame.strokes.places[1]];
        assert_eq!(first.at, [0, 0]);
        assert_eq!(second.at, [first.size[0], 0]);
        let value = |vertex: usize, at: usize| {
            let start = vertex * 32 + at * 4;
            f32::from_ne_bytes([
                frame.bytes[start],
                frame.bytes[start + 1],
                frame.bytes[start + 2],
                frame.bytes[start + 3],
            ])
        };
        // Both spans start at x = 1 (4 less the support of 3), rows 5 and 37.
        assert_eq!([value(0, 6), value(0, 7)], [-1.0, -5.0]);
        assert_eq!(
            [value(6, 6), value(6, 7)],
            [second.at[0] as f32 - 1.0, -37.0]
        );
        // The stroke records: half width, thin factor, place, page scale.
        let words = frame.strokes.strokes.as_chunks::<4>().0;
        let record: Vec<f32> = words[8..16]
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        assert_eq!(
            record,
            [
                1.0,
                1.0,
                second.at[0] as f32,
                0.0,
                2.0 / 2048.0,
                2.0 / 2048.0,
                0.0,
                0.0
            ]
        );
        Ok(())
    }

    #[test]
    fn strokes_fill_shelves_and_pages_until_the_coverage_budget()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::retained::RetainedGeometry;
        // Diagonals across a frame as large as a page: one page each.
        let diagonal: &[(f32, f32)] = &[(0.0, 0.0), (2048.0, 2048.0)];
        let pages = (COVERAGE_BUDGET / (2048 * 2048 * 2)) as usize;
        let full = polylines(2048.0, &vec![diagonal; pages])?;
        let frame = RetainedGeometry::default().prepare(&full)?;
        assert_eq!(frame.strokes.pages() as usize, pages);
        assert_eq!(frame.batches.len(), pages);
        let over = polylines(2048.0, &vec![diagonal; pages + 1])?;
        assert!(matches!(
            RetainedGeometry::default().prepare(&over),
            Err(RenderError::Limit(LimitError::StrokeCoverage {
                bytes,
                max: COVERAGE_BUDGET
            })) if bytes > COVERAGE_BUDGET
        ));
        // A frame whose single page exceeds the budget still has that page.
        let wide = Viewport::new(8192.0, 8192.0, 1.0)?;
        let mapping = FrameMapping::new(8192, 8192, wide)?;
        let (small, _) = scene(LineCap::Butt, LineJoin::Round)?;
        let DrawCommand::DrawVector {
            vector,
            origin,
            scale,
            color,
            clip,
        } = small.commands()[0]
        else {
            return Err("the stroke is recorded as a vector".into());
        };
        let vector = small.vector(vector).ok_or("the scene holds its vector")?;
        let encoded =
            encode(vector, origin, scale, color, clip, &mapping)?.ok_or("the stroke is visible")?;
        let mut frame = Frame::new(Some(&mapping));
        assert_eq!(
            frame.push(&encoded, &mapping, u64::MAX, &mut Vec::new())?,
            0
        );
        assert_eq!(frame.side, [8192, 8192]);
        // Short strokes share a page: a second shelf begins where the first
        // is full.
        let short: Vec<Vec<(f32, f32)>> = (0..3)
            .map(|i| {
                vec![
                    (10.0, 100.0 + 300.0 * i as f32),
                    (900.0, 300.0 + 300.0 * i as f32),
                ]
            })
            .collect();
        let short: Vec<&[(f32, f32)]> = short.iter().map(Vec::as_slice).collect();
        let frame = RetainedGeometry::default().prepare(&polylines(2048.0, &short)?)?;
        assert_eq!(frame.strokes.pages(), 1);
        let places = &frame.strokes.places;
        assert_eq!(places[1].at, [places[0].size[0], 0]);
        assert_eq!(places[2].at, [0, places[0].size[1].max(places[1].size[1])]);
        Ok(())
    }

    #[test]
    fn segment_records_beyond_the_binding_limit_are_refused()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::retained::RetainedGeometry;
        let scene = polylines(64.0, &[&[(4.0, 8.0), (30.0, 20.0), (60.0, 8.0)]])?;
        let mapping = frame_mapping(&scene, scene.viewport().frame_extent());
        let refused = RetainedGeometry::default().prepare_for(&scene, mapping, 100);
        assert!(matches!(
            refused,
            Err(RenderError::Limit(LimitError::StrokeRecords {
                bytes: 128,
                max: 100
            }))
        ));
        assert!(
            RetainedGeometry::default()
                .prepare_for(&scene, mapping, 128)
                .is_ok()
        );
        Ok(())
    }

    // The scene of one stroke and its frame mapping.
    fn scene(
        cap: LineCap,
        join: LineJoin,
    ) -> Result<(DisplayList, FrameMapping), Box<dyn std::error::Error>> {
        let viewport = Viewport::new(64.0, 64.0, 1.0)?;
        let mut ui = Runtime::new();
        ui.set_viewport(viewport)?;
        ui.mount_tree(Element::new("lines", Lines(cap, join)))?;
        ui.flush(std::time::Duration::ZERO)?;
        Ok((ui.paint()?, FrameMapping::new(64, 64, viewport)?))
    }

    #[test]
    fn a_record_holds_its_segment_and_its_strokes_index() -> Result<(), Box<dyn std::error::Error>>
    {
        for cap in [LineCap::Butt, LineCap::Round, LineCap::Square] {
            for join in [LineJoin::Round, LineJoin::Bevel, LineJoin::Miter] {
                let (scene, mapping) = scene(cap, join)?;
                let DrawCommand::DrawVector {
                    vector,
                    origin,
                    scale,
                    color,
                    clip,
                } = scene.commands()[0]
                else {
                    return Err("the stroke is recorded as a vector".into());
                };
                let vector = scene.vector(vector).ok_or("the scene holds its vector")?;
                let encoded = encode(vector, origin, scale, color, clip, &mapping)?
                    .ok_or("the stroke is visible")?;
                let stroke = PreparedStroke::new(vector, origin, scale, clip, &mapping)?
                    .ok_or("the stroke is visible")?;
                assert_eq!(encoded.miters, join == LineJoin::Miter);
                let mut frame = Frame::new(Some(&mapping));
                let mut vertices = Vec::new();
                // A second stroke of the frame has index one.
                frame.push(&encoded, &mapping, u64::MAX, &mut vertices)?;
                frame.push(&encoded, &mapping, u64::MAX, &mut vertices)?;
                let records = frame.segments.as_chunks::<SEGMENT_BYTES>().0;
                assert_eq!(records.len(), 2 * stroke.segments().len());
                for (index, record) in records.iter().enumerate() {
                    let segment = &stroke.segments()[index % stroke.segments().len()];
                    let word = |at: usize| {
                        u32::from_le_bytes([
                            record[at * 4],
                            record[at * 4 + 1],
                            record[at * 4 + 2],
                            record[at * 4 + 3],
                        ])
                    };
                    assert_eq!(word(0), segment.a[0].to_bits());
                    assert_eq!(word(3), segment.b[1].to_bits());
                    assert_eq!(word(10), segment.end_k.to_bits());
                    assert_eq!(word(11), segment.start_k.to_bits());
                    assert_eq!(word(12), segment.start_m[0].to_bits());
                    assert_eq!(word(14), segment.flags, "{cap:?} {join:?}");
                    assert_eq!(word(15), (index / stroke.segments().len()) as u32);
                }
            }
        }
        Ok(())
    }
}
