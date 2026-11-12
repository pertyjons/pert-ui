//! The analytic shape pipeline: fixed-size records for each prepared
//! [`DrawCommand::DrawShape`], read from a storage buffer by instance index.
//!
//! An arc or a triangle has one record, drawn over its span. A rectangle
//! with a [`PreparedShape::solid_box`] has one record for the box, which the
//! fragment stage paints without evaluating a distance and which is left out
//! when its color is fully transparent, and one for each strip of the span
//! around it (above, below, left and right, where they have pixels), which
//! differ only in group 0. Most of a large frame or fill is then not
//! evaluated per pixel.
//!
//! A record is nine 16-byte groups, 144 bytes, little-endian as WGSL lays
//! host-shared values out (the
//! `Shape` struct of `shapes.wgsl`). Values are those of
//! [`pert_ui_runtime::ShapeParameters`], in physical pixels relative to the
//! span's origin:
//!
//! | Group | Rectangle | Arc | Triangle |
//! |---|---|---|---|
//! | 0 | the drawn part of the span, its corners in normalized device coordinates: left, top, right, bottom | the span | the span |
//! | 1 | span origin x, y in frame pixels; 1; 0 | origin; thin-stroke factor; 0 | origin; 1; 0 |
//! | 2 | premultiplied linear fill | the arc's paint | the triangle's paint |
//! | 3 | premultiplied linear border paint | 0 | 0 |
//! | 4 | outer edges x0, y0, x1, y1 | center x, y; radius; half width | vertices 0 and 1 |
//! | 5 | outer corner sizes, clockwise from the top left | middle cos, sin; half sweep cos, sin | vertex 2; winding sign; 0 |
//! | 6 | inner edges | 0 | 0 |
//! | 7 | inner corner sizes | 0 | 0 |
//! | 8 (`u32`) | 0; inner (0 no border, 1 all border, 2 inner area); corner kinds, two bits each (1 round, 2 cut); 1 in a solid box's record | 1; 1 for a full circle; 0; 0 | 2; 0; 0; 0 |
use super::*;
use pert_ui_runtime::{
    Border, Boundary, CornerKind, FrameMapping, Inner, Paint, PreparedShape, Rect, ShapeGeometry,
    ShapeParameters,
};

/// The size of one record in the storage buffer.
pub(super) const RECORD_BYTES: usize = 144;

/// The premultiplied linear color of a paint this renderer knows.
fn solid(paint: Option<Paint>) -> Result<[f32; 4], RenderError> {
    match paint {
        None => Ok([0.0; 4]),
        Some(Paint::Solid(color)) => {
            let [red, green, blue, alpha] = color.channels();
            Ok([red * alpha, green * alpha, blue * alpha, alpha])
        }
        Some(_) => Err(RenderError::UnsupportedGraphics("unknown paint".into())),
    }
}

fn kinds(boundary: &Boundary) -> Result<u32, RenderError> {
    let mut packed = 0;
    for (index, corner) in boundary.corners().iter().enumerate() {
        let kind = match corner.kind() {
            CornerKind::Square => 0,
            CornerKind::Round => 1,
            CornerKind::Cut => 2,
            _ => {
                return Err(RenderError::UnsupportedGraphics(
                    "unknown corner kind".into(),
                ));
            }
        };
        packed |= kind << (2 * index);
    }
    Ok(packed)
}

/// Appends the records of one shape command for a frame to `records` and
/// returns how many: none when it draws nothing there, clipped away,
/// collapsed or fully transparent.
pub(super) fn append(
    geometry: &ShapeGeometry,
    fill: Option<Paint>,
    border: Option<Border>,
    clip: Option<Rect>,
    mapping: &FrameMapping,
    records: &mut Vec<u8>,
) -> Result<u32, RenderError> {
    let fill = solid(fill)?;
    let stroke = solid(border.map(Border::paint))?;
    let Some(shape) = PreparedShape::new(geometry, border.as_ref(), clip, mapping)? else {
        return Ok(0);
    };
    if fill[3] == 0.0 && stroke[3] == 0.0 {
        return Ok(0);
    }
    let [x0, y0, x1, y1] = shape.span();
    let (width, height) = (
        f64::from(mapping.frame_width()),
        f64::from(mapping.frame_height()),
    );
    // Whole pixel edges: every pixel center is half a pixel from them, so
    // rounding these coordinates never decides a pixel.
    let ndc = |[x0, y0, x1, y1]: [u32; 4]| {
        [
            (2.0 * f64::from(x0) / width - 1.0) as f32,
            (1.0 - 2.0 * f64::from(y0) / height) as f32,
            (2.0 * f64::from(x1) / width - 1.0) as f32,
            (1.0 - 2.0 * f64::from(y1) / height) as f32,
        ]
    };
    let mut groups = [[0.0f32; 4]; 8];
    let mut meta = [0u32; 4];
    groups[1] = [x0 as f32, y0 as f32, 1.0, 0.0];
    groups[2] = fill;
    groups[3] = stroke;
    match shape.parameters() {
        ShapeParameters::Rect { outer, inner } => {
            groups[4] = outer.edges();
            groups[5] = outer.corners().map(|corner| corner.size());
            meta[2] = kinds(&outer)?;
            match inner {
                Inner::NoBorder => {}
                Inner::Filled => meta[1] = 1,
                Inner::Area(inner) => {
                    meta[1] = 2;
                    groups[6] = inner.edges();
                    groups[7] = inner.corners().map(|corner| corner.size());
                }
            }
        }
        ShapeParameters::Arc {
            center,
            radius,
            half_width,
            middle,
            half_sweep,
            full,
            thin,
        } => {
            meta[0] = 1;
            meta[1] = u32::from(full);
            groups[1][2] = thin;
            groups[4] = [center[0], center[1], radius, half_width];
            groups[5] = [middle[0], middle[1], half_sweep[0], half_sweep[1]];
        }
        ShapeParameters::Triangle { vertices, winding } => {
            meta[0] = 2;
            groups[4] = [
                vertices[0][0],
                vertices[0][1],
                vertices[1][0],
                vertices[1][1],
            ];
            groups[5] = [vertices[2][0], vertices[2][1], winding, 0.0];
        }
        _ => {
            return Err(RenderError::UnsupportedGraphics(
                "unknown shape kind".into(),
            ));
        }
    }
    let mut count = 0;
    let mut push = |part: [u32; 4], groups: &[[f32; 4]; 8], meta: [u32; 4]| {
        if part[2] <= part[0] || part[3] <= part[1] {
            return;
        }
        let mut groups = *groups;
        groups[0] = ndc(part);
        let values = groups.iter().flatten().map(|value| value.to_le_bytes());
        let flags = meta.iter().map(|value| value.to_le_bytes());
        records.extend(values.chain(flags).flatten());
        count += 1;
    };
    let Some([left, top, right, bottom]) = shape.solid_box() else {
        push([x0, y0, x1, y1], &groups, meta);
        return Ok(count);
    };
    // The strips around the box, evaluated per pixel.
    push([x0, y0, x1, top], &groups, meta);
    push([x0, bottom, x1, y1], &groups, meta);
    push([x0, top, left, bottom], &groups, meta);
    push([right, top, x1, bottom], &groups, meta);
    // The box has one coverage. The fragment stage computes its color
    // from the same paints as in the strips; here it only decides whether
    // the box draws anything: `B·c_b + F·(c_o − B.a·c_b)`.
    let coverage = shape.coverage(left, top);
    let (outer, band) = (coverage.outer(), coverage.border());
    let inside = outer - stroke[3] * band;
    if (0..4).any(|channel| stroke[channel] * band + fill[channel] * inside > 0.0) {
        meta[3] = 1;
        push([left, top, right, bottom], &groups, meta);
    }
    Ok(count)
}

/// Refuses records beyond what one storage buffer binding may hold.
pub(super) fn check_records(bytes: usize, max: u64) -> Result<(), LimitError> {
    let bytes = bytes as u64;
    if bytes > max {
        return Err(LimitError::ShapeRecords { bytes, max });
    }
    Ok(())
}

/// The shape pipeline of one surface format.
#[derive(Clone)]
pub(super) struct Pipeline {
    pub(super) pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

impl Pipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pert-ui shapes"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shapes.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pert-ui shape records"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pert-ui shapes"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pert-ui shapes"),
            layout: Some(&pipeline_layout),
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
                // The fragment's color is premultiplied by its coverage.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, layout }
    }

    /// Binds `buffer`, which holds the frame's records.
    pub(super) fn bind(&self, device: &wgpu::Device, buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pert-ui shape records"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::retained::RetainedGeometry;
    use pert_ui_runtime::{
        Component, Corner, CornerPosition, DrawError, Element, Length, Painter, Point, Runtime,
        RuntimeError, Shape, ShapePrepareError, ShapeStyle, UpdateContext, Viewport,
    };

    type Outcome = Result<(), Box<dyn std::error::Error>>;
    type Paints = Box<dyn Fn(&mut Painter) -> Result<(), DrawError>>;

    struct Drawn(Paints);
    impl Component for Drawn {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            (self.0)(painter)
        }
    }

    fn scene(
        width: f32,
        height: f32,
        scale: f64,
        paint: impl Fn(&mut Painter) -> Result<(), DrawError> + 'static,
    ) -> Result<DisplayList, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, height, scale)?)?;
        ui.mount_tree(Element::new("scene", Drawn(Box::new(paint))))?;
        ui.flush(std::time::Duration::ZERO)?;
        Ok(ui.paint()?)
    }

    fn floats(record: &[u8], group: usize) -> [f32; 4] {
        std::array::from_fn(|index| {
            let start = group * 16 + index * 4;
            f32::from_le_bytes(record[start..start + 4].try_into().expect("four bytes"))
        })
    }

    fn flags(record: &[u8]) -> [u32; 4] {
        std::array::from_fn(|index| {
            let start = 128 + index * 4;
            u32::from_le_bytes(record[start..start + 4].try_into().expect("four bytes"))
        })
    }

    fn records(scene: &DisplayList, extent: PhysicalExtent) -> Result<Vec<u8>, RenderError> {
        Ok(RetainedGeometry::default()
            .prepare_for(scene, frame_mapping(scene, extent), u64::MAX)?
            .records)
    }

    #[test]
    fn a_rectangle_record_holds_its_span_paints_and_boundaries() -> Outcome {
        let fill = Color::from_linear(0.0, 0.0, 1.0, 0.5)?;
        let line = Color::from_linear(1.0, 0.0, 0.0, 1.0)?;
        let scene = scene(100.0, 50.0, 1.0, move |p| {
            let shape = Shape::rounded(Length::new(4.0)?)
                .with_corner(CornerPosition::TopRight, Corner::cut(Length::new(6.0)?))
                .with_corner(CornerPosition::BottomLeft, Corner::SQUARE);
            p.draw_shape(
                Rect::new(10.0, 5.0, 40.0, 20.0)?,
                shape,
                ShapeStyle::fill(fill).with_border(Length::new(2.0)?, line),
            )
        })?;
        let records = records(&scene, PhysicalExtent::new(100, 50))?;
        // The four strips around the solid box, then the box.
        assert_eq!(records.len(), 5 * RECORD_BYTES);
        // The span is the rectangle grown by one pixel: 9..51 × 4..26. The
        // box is the inner area below the rows its cut corner reaches and
        // above those of its round one: 12..48 × 13..21.
        let parts: Vec<_> = records
            .chunks(RECORD_BYTES)
            .map(|record| floats(record, 0))
            .collect();
        assert_eq!(
            parts,
            [
                [-0.82, 0.84, 0.02, 0.48],
                [-0.82, 0.16, 0.02, -0.04],
                [-0.82, 0.48, -0.76, 0.16],
                [-0.04, 0.48, 0.02, 0.16],
                [-0.76, 0.48, -0.04, 0.16],
            ]
        );
        // The box is marked and not evaluated; the strips are. All have
        // the same parameters.
        let solid = &records[4 * RECORD_BYTES..];
        assert_eq!(flags(solid), [0, 2, 0b00_01_10_01, 1]);
        assert_eq!(solid[16..128], records[16..128]);
        for strip in records[..4 * RECORD_BYTES].chunks(RECORD_BYTES) {
            assert_eq!(strip[16..], records[16..RECORD_BYTES]);
        }
        assert_eq!(floats(&records, 1), [9.0, 4.0, 1.0, 0.0]);
        // Premultiplied paints.
        assert_eq!(floats(&records, 2), [0.0, 0.0, 0.5, 0.5]);
        assert_eq!(floats(&records, 3), [1.0, 0.0, 0.0, 1.0]);
        // Edges relative to the span origin, and the corner sizes.
        assert_eq!(floats(&records, 4), [1.0, 1.0, 41.0, 21.0]);
        assert_eq!(floats(&records, 5), [4.0, 6.0, 4.0, 0.0]);
        // The inner boundary: inset by the border, corners by their offsets.
        assert_eq!(floats(&records, 6), [3.0, 3.0, 39.0, 19.0]);
        let inner = floats(&records, 7);
        assert_eq!((inner[0], inner[2], inner[3]), (2.0, 2.0, 0.0));
        assert!((inner[1] - (6.0 - (2.0 - std::f32::consts::SQRT_2) * 2.0)).abs() < 1e-5);
        // A rectangle with an inner area; round, cut, round, square.
        assert_eq!(flags(&records), [0, 2, 0b00_01_10_01, 0]);
        Ok(())
    }

    #[test]
    fn arc_and_triangle_records_hold_their_parameters() -> Outcome {
        let scene = scene(64.0, 64.0, 1.0, |p| {
            p.stroke_arc(
                Point::new(20.5, 20.5)?,
                Length::new(8.0)?,
                0.0,
                360.0,
                Length::new(0.5)?,
                Color::WHITE,
            )?;
            p.fill_triangle(
                [
                    Point::new(40.0, 40.0)?,
                    Point::new(50.0, 40.0)?,
                    Point::new(40.0, 50.0)?,
                ],
                Color::WHITE,
            )
        })?;
        let records = records(&scene, PhysicalExtent::new(64, 64))?;
        assert_eq!(records.len(), 2 * RECORD_BYTES);
        let (arc, triangle) = records.split_at(RECORD_BYTES);
        // A full circle, thinner than a pixel: evaluated one pixel wide with
        // its coverage scaled by the width.
        assert_eq!(flags(arc), [1, 1, 0, 0]);
        // The span starts one pixel beyond the stroke's box: 20.5 − 8.5 − 1.
        assert_eq!(floats(arc, 1), [11.0, 11.0, 0.5, 0.0]);
        assert_eq!(floats(arc, 4), [9.5, 9.5, 8.0, 0.5]);
        assert_eq!(floats(arc, 3), [0.0; 4]);
        assert_eq!(flags(triangle), [2, 0, 0, 0]);
        assert_eq!(floats(triangle, 4), [1.0, 1.0, 11.0, 1.0]);
        assert_eq!(floats(triangle, 5), [1.0, 11.0, 1.0, 0.0]);
        Ok(())
    }

    #[test]
    fn shapes_batch_as_record_ranges_in_draw_order() -> Outcome {
        let scene = scene(64.0, 64.0, 1.0, |p| {
            let shape = Shape::rounded(Length::new(3.0)?);
            p.draw_shape(
                Rect::new(1.0, 1.0, 10.0, 10.0)?,
                shape,
                ShapeStyle::fill(Color::WHITE),
            )?;
            p.draw_shape(
                Rect::new(12.0, 1.0, 10.0, 10.0)?,
                shape,
                ShapeStyle::fill(Color::WHITE),
            )?;
            p.fill_rect(Rect::new(30.0, 1.0, 5.0, 5.0)?, Color::WHITE);
            p.draw_shape(
                Rect::new(40.0, 1.0, 10.0, 10.0)?,
                shape,
                ShapeStyle::fill(Color::WHITE),
            )?;
            // Clipped away and fully transparent shapes leave no record.
            p.draw_shape(
                Rect::new(100.0, 1.0, 10.0, 10.0)?,
                shape,
                ShapeStyle::fill(Color::WHITE),
            )?;
            p.draw_shape(
                Rect::new(1.0, 20.0, 10.0, 10.0)?,
                shape,
                ShapeStyle::fill(Color::from_linear(1.0, 1.0, 1.0, 0.0)?),
            )
        })?;
        let encoded = RetainedGeometry::default().prepare(&scene)?;
        // Each drawn shape has its four strips and its box.
        assert_eq!(encoded.records.len(), 15 * RECORD_BYTES);
        let batches: Vec<_> = encoded
            .batches
            .iter()
            .map(|batch| (batch.source, batch.range.clone()))
            .collect();
        assert_eq!(
            batches,
            [
                (Source::Shape, 0..10),
                (Source::Fill, 0..6),
                (Source::Shape, 10..15)
            ]
        );
        Ok(())
    }

    #[test]
    fn an_unchanged_shape_is_not_encoded_again() -> Outcome {
        let paint = |x: f32| {
            move |p: &mut Painter| -> Result<(), DrawError> {
                let shape = Shape::rounded(Length::new(3.0)?);
                p.draw_shape(
                    Rect::new(1.0, 1.0, 10.0, 10.0)?,
                    shape,
                    ShapeStyle::fill(Color::WHITE),
                )?;
                p.draw_shape(
                    Rect::new(x, 20.0, 10.0, 10.0)?,
                    shape,
                    ShapeStyle::fill(Color::WHITE),
                )
            }
        };
        let extent = PhysicalExtent::new(64, 64);
        let first = scene(64.0, 64.0, 1.0, paint(5.0))?;
        let cache = RetainedGeometry::default().prepare_for(
            &first,
            frame_mapping(&first, extent),
            u64::MAX,
        )?;
        assert_eq!(cache.encoded_commands(), 2);
        // An equal scene at the same extent encodes nothing.
        let same = cache.prepare_for(&first, frame_mapping(&first, extent), u64::MAX)?;
        assert_eq!(same.encoded_commands(), 0);
        assert_eq!(same.records, cache.records);
        // A moved shape encodes only its own record.
        let moved = scene(64.0, 64.0, 1.0, paint(6.0))?;
        let next = same.prepare_for(&moved, frame_mapping(&moved, extent), u64::MAX)?;
        assert_eq!(next.encoded_commands(), 1);
        let first = 5 * RECORD_BYTES;
        assert_eq!(next.records[..first], cache.records[..first]);
        assert_ne!(next.records[first..], cache.records[first..]);
        Ok(())
    }

    #[test]
    fn a_command_whose_record_count_changes_keeps_later_commands_in_place() -> Outcome {
        // The first command is a filled frame (strips and a box), a frame
        // without fill (strips only, its transparent box left out), a pill
        // (one record) or absent; the commands after it are unchanged.
        let paint = |first: u8| {
            move |p: &mut Painter| -> Result<(), DrawError> {
                let frame = Rect::new(2.0, 2.0, 30.0, 20.0)?;
                let line = Length::new(1.0)?;
                match first {
                    0 => p.draw_shape(
                        frame,
                        Shape::rounded(Length::new(3.0)?),
                        ShapeStyle::fill(Color::WHITE)
                            .with_border(line, Color::from_linear(0.0, 0.0, 0.0, 1.0)?),
                    )?,
                    1 => p.draw_shape(
                        frame,
                        Shape::rounded(Length::new(3.0)?),
                        ShapeStyle::border(line, Color::from_linear(0.0, 0.0, 0.0, 1.0)?),
                    )?,
                    2 => p.draw_shape(
                        Rect::new(2.0, 2.0, 30.0, 8.0)?,
                        Shape::rounded(Length::new(4.0)?),
                        ShapeStyle::fill(Color::WHITE),
                    )?,
                    _ => {}
                }
                p.fill_rect(Rect::new(40.0, 2.0, 5.0, 5.0)?, Color::WHITE);
                p.draw_shape(
                    Rect::new(2.0, 30.0, 30.0, 20.0)?,
                    Shape::rounded(Length::new(3.0)?),
                    ShapeStyle::fill(Color::WHITE),
                )?;
                p.fill_triangle(
                    [
                        Point::new(40.0, 40.0)?,
                        Point::new(50.0, 40.0)?,
                        Point::new(40.0, 50.0)?,
                    ],
                    Color::WHITE,
                )
            }
        };
        let extent = PhysicalExtent::new(64, 64);
        let scenes = [0, 1, 2, 3, 0, 2, 1, 3, 1, 0]
            .into_iter()
            .map(|first| scene(64.0, 64.0, 1.0, paint(first)))
            .collect::<Result<Vec<_>, _>>()?;
        let mut counts = Vec::new();
        let mut cache = RetainedGeometry::default();
        for scene in &scenes {
            let mapping = frame_mapping(scene, extent);
            let fresh = RetainedGeometry::default().prepare_for(scene, mapping, u64::MAX)?;
            cache = cache.prepare_for(scene, mapping, u64::MAX)?;
            assert_eq!(cache.records, fresh.records);
            assert_eq!(cache.bytes, fresh.bytes);
            let ranges = |geometry: &RetainedGeometry| -> Vec<_> {
                geometry
                    .batches
                    .iter()
                    .map(|batch| (batch.source, batch.range.clone()))
                    .collect()
            };
            assert_eq!(ranges(&cache), ranges(&fresh));
            counts.push(fresh.records.len() / RECORD_BYTES);
        }
        // The later shape's five records and the triangle's one follow five,
        // four, one or no record of the first command.
        assert_eq!(counts, [11, 10, 7, 6, 11, 7, 10, 6, 10, 11]);
        Ok(())
    }

    #[test]
    fn an_equal_scene_is_encoded_again_for_another_extent() -> Outcome {
        // A viewport 33.4 logical pixels wide: frames of 33 and of 34 pixels
        // map it with different factors, so the same command gets different
        // records, and a cache from one extent is not reused for the other.
        let scene = scene(33.4, 20.0, 1.0, |p| {
            p.draw_shape(
                Rect::new(3.3, 2.0, 25.2, 12.0)?,
                Shape::rounded(Length::new(3.0)?),
                ShapeStyle::fill(Color::WHITE),
            )
        })?;
        let narrow = PhysicalExtent::new(33, 20);
        let wide = PhysicalExtent::new(34, 20);
        let cache = RetainedGeometry::default().prepare_for(
            &scene,
            frame_mapping(&scene, narrow),
            u64::MAX,
        )?;
        let again = cache.prepare_for(&scene, frame_mapping(&scene, narrow), u64::MAX)?;
        assert_eq!(again.encoded_commands(), 0);
        let resized = again.prepare_for(&scene, frame_mapping(&scene, wide), u64::MAX)?;
        assert_eq!(resized.encoded_commands(), 1);
        assert_ne!(resized.records, cache.records);
        assert_eq!(resized.records, records(&scene, wide)?);
        // And back: nothing of the other extent's encoding is kept.
        let back = resized.prepare_for(&scene, frame_mapping(&scene, narrow), u64::MAX)?;
        assert_eq!(back.encoded_commands(), 1);
        assert_eq!(back.records, cache.records);
        // Without an extent nothing is drawn.
        assert!(
            RetainedGeometry::default()
                .prepare_for(&scene, None, u64::MAX)?
                .records
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn preparation_refuses_records_beyond_the_budget_before_storing_them() -> Outcome {
        let scene = scene(64.0, 64.0, 1.0, |p| {
            let shape = Shape::rounded(Length::new(3.0)?);
            for x in [1.0, 12.0, 23.0] {
                p.draw_shape(
                    Rect::new(x, 1.0, 10.0, 10.0)?,
                    shape,
                    ShapeStyle::fill(Color::WHITE),
                )?;
            }
            Ok(())
        })?;
        let mapping = frame_mapping(&scene, PhysicalExtent::new(64, 64));
        let exact = 15 * RECORD_BYTES as u64;
        assert!(
            RetainedGeometry::default()
                .prepare_for(&scene, mapping, exact)
                .is_ok()
        );
        let error = RetainedGeometry::default()
            .prepare_for(&scene, mapping, exact - 1)
            .err()
            .ok_or("the records exceed the budget")?;
        assert_eq!(
            error,
            RenderError::Limit(LimitError::ShapeRecords {
                bytes: exact,
                max: exact - 1
            })
        );
        Ok(())
    }

    #[test]
    fn an_equal_scene_counts_as_prepared_only_for_the_same_extent() -> Outcome {
        let scene = scene(33.4, 20.0, 1.0, |p| {
            p.draw_shape(
                Rect::new(3.3, 2.0, 25.2, 12.0)?,
                Shape::rounded(Length::new(3.0)?),
                ShapeStyle::fill(Color::WHITE),
            )
        })?;
        let narrow = frame_mapping(&scene, PhysicalExtent::new(33, 20));
        let wide = frame_mapping(&scene, PhysicalExtent::new(34, 20));
        assert!(narrow.is_some() && wide.is_some() && narrow != wide);
        assert!(prepared_for(Some(&scene), narrow, &scene, narrow));
        assert!(!prepared_for(Some(&scene), narrow, &scene, wide));
        assert!(!prepared_for(Some(&scene), None, &scene, narrow));
        assert!(!prepared_for(None, narrow, &scene, narrow));
        let other = self::scene(33.4, 20.0, 1.0, |_| Ok(()))?;
        assert!(!prepared_for(Some(&other), narrow, &scene, narrow));
        Ok(())
    }

    #[test]
    fn records_beyond_the_binding_limit_are_refused_before_any_upload() {
        assert_eq!(check_records(0, 0), Ok(()));
        assert_eq!(check_records(RECORD_BYTES, RECORD_BYTES as u64), Ok(()));
        assert_eq!(
            check_records(2 * RECORD_BYTES, RECORD_BYTES as u64),
            Err(LimitError::ShapeRecords {
                bytes: 2 * RECORD_BYTES as u64,
                max: RECORD_BYTES as u64
            })
        );
    }

    #[test]
    fn a_shape_that_cannot_be_prepared_is_a_typed_error() -> Outcome {
        let scene = scene(64.0, 64.0, 1.0, |p| {
            p.draw_shape(
                Rect::new(-1.0e6, -1.0e6, 2.0e6, 2.0e6)?,
                Shape::rounded(Length::new(5.0e5)?),
                ShapeStyle::fill(Color::WHITE),
            )
        })?;
        let error = RetainedGeometry::default()
            .prepare(&scene)
            .err()
            .ok_or("the corner is far larger than the range")?;
        assert_eq!(error, RenderError::Shape(ShapePrepareError::OutOfRange));
        Ok(())
    }
}
