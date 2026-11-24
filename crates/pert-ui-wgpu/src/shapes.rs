//! The analytic shape pipeline: fixed-size records for each prepared
//! [`DrawCommand::DrawShape`], read from a storage buffer by instance index.
//!
//! An arc or a triangle has one record, drawn over its span. A rectangle
//! with a [`PreparedShape::solid_box`] of at least [`MIN_SOLID_BOX_PIXELS`]
//! has one record for each strip of the span around the box (above, below,
//! left and right, where they have pixels), which differ only in group 0, and
//! its box is a [`SolidBox`]: one color, drawn by the fill pipeline and left
//! out when fully transparent. Most of a large frame or fill is then not
//! evaluated per pixel, and its pixels do not run the shape shader at all.
//! Any other rectangle has one record over its span, so a run of small
//! shapes stays one draw.
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
//! | 8 (`u32`) | 0; inner (0 no border, 1 all border, 2 inner area); corner kinds, two bits each (1 round, 2 cut); 0 | 1; 1 for a full circle; 0; 0 | 2; 0; 0; 0 |
use super::*;
use pert_ui_runtime::{
    Border, Boundary, CornerKind, FrameMapping, Inner, Paint, PreparedShape, Rect, ShapeGeometry,
    ShapeParameters,
};

/// The size of one record in the storage buffer.
pub(super) const RECORD_BYTES: usize = 144;

/// The fewest pixels a solid box has to be drawn as a fill quad of its own.
/// A smaller box would cost more as its own draw, which splits the run of
/// consecutive shapes into separate draws and adds vertices to encode, than
/// its pixels cost evaluated: a rectangle with a smaller box is one record
/// over its whole span. On Intel UHD Graphics 770 the gallery's cards zoomed
/// out, with every box split out, took up to 6 % longer on the GPU and 11 %
/// longer to present than with this threshold, while large panels' boxes
/// are what made pages faster (`plans/rendering/render05-probe`). Tests use 256 pixels,
/// so the parity check's 64-pixel frames reach both paths; the paths are the
/// same code whatever the threshold.
pub(super) const MIN_SOLID_BOX_PIXELS: u64 = if cfg!(test) { 256 } else { 4096 };

/// The premultiplied linear color of a paint this renderer knows.
fn solid(paint: Option<pert_ui_runtime::RecordedPaint>) -> Result<[f32; 4], RenderError> {
    match paint {
        None => Ok([0.0; 4]),
        Some(Paint::Solid(color)) => {
            let [red, green, blue, alpha] = color.channels();
            Ok([red * alpha, green * alpha, blue * alpha, alpha])
        }
        Some(Paint::Gradient(id)) => Ok([id.index() as f32, 0., 0., -1.]),
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

/// A rectangle's solid box: whole frame pixels of one coverage, painted in
/// one color by the fill pipeline instead of the shape shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SolidBox {
    /// Its corners in normalized device coordinates: left, top, right, bottom.
    pub(super) ndc: [f32; 4],
    /// Its premultiplied linear color, `B·c_b + F·(c_o − B.a·c_b)` at its
    /// coverage, with a nonzero alpha.
    pub(super) color: [f32; 4],
}

impl SolidBox {
    /// The color as straight alpha, as the fill pipeline blends it. Each
    /// premultiplied channel is a sum of products of channels no larger than
    /// their paint's alpha with the same coverages as the alpha's terms, and
    /// rounding is monotonic, so it never exceeds the alpha and its quotient
    /// lies in `0..=1`, also for a subnormal alpha.
    pub(super) fn straight(&self) -> [f32; 4] {
        let [red, green, blue, alpha] = self.color;
        [red / alpha, green / alpha, blue / alpha, alpha]
    }
}

/// Appends the records of one shape command for a frame to `records` and
/// returns how many, none when it draws nothing there (clipped away,
/// collapsed or fully transparent), and its solid box, if it has one that
/// draws.
pub(super) fn append(
    geometry: &ShapeGeometry,
    fill: Option<pert_ui_runtime::RecordedPaint>,
    border: Option<Border>,
    clip: Option<Rect>,
    mapping: &FrameMapping,
    records: &mut Vec<u8>,
) -> Result<(u32, Option<SolidBox>), RenderError> {
    let fill = solid(fill)?;
    let stroke = solid(border.map(Border::paint))?;
    let Some(shape) = PreparedShape::new(geometry, border.as_ref(), clip, mapping)? else {
        return Ok((0, None));
    };
    if fill[3] == 0.0 && stroke[3] == 0.0 {
        return Ok((0, None));
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
    let mut push = |part: [u32; 4]| {
        if part[2] <= part[0] || part[3] <= part[1] {
            return;
        }
        let mut groups = groups;
        groups[0] = ndc(part);
        let values = groups.iter().flatten().map(|value| value.to_le_bytes());
        let flags = meta.iter().map(|value| value.to_le_bytes());
        records.extend(values.chain(flags).flatten());
        count += 1;
    };
    let large = |[left, top, right, bottom]: &[u32; 4]| {
        u64::from(right - left) * u64::from(bottom - top) >= MIN_SOLID_BOX_PIXELS
    };
    let Some([left, top, right, bottom]) = shape
        .solid_box()
        .filter(|r| fill[3] >= 0. && stroke[3] >= 0. && large(r))
    else {
        push([x0, y0, x1, y1]);
        return Ok((count, None));
    };
    // The strips around the box, evaluated per pixel.
    push([x0, y0, x1, top]);
    push([x0, bottom, x1, y1]);
    push([x0, top, left, bottom]);
    push([right, top, x1, bottom]);
    // The box has one coverage, so one color from the strips' paints, as
    // the shape shader composes them: `B·c_b + F·(c_o − B.a·c_b)`.
    let coverage = shape.coverage(left, top);
    let (outer, band) = (coverage.outer(), coverage.border());
    let inside = outer - stroke[3] * band;
    let color: [f32; 4] =
        std::array::from_fn(|channel| stroke[channel] * band + fill[channel] * inside);
    let solid_box = (color[3] > 0.0).then(|| SolidBox {
        ndc: ndc([left, top, right, bottom]),
        color,
    });
    Ok((count, solid_box))
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
    pub(super) gradient_pipeline: wgpu::RenderPipeline,
    pub(super) gradient_layout: wgpu::BindGroupLayout,
}

impl Pipeline {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pert-ui shapes"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("shapes.wgsl"),
                    "\n",
                    include_str!("gradients.wgsl")
                )
                .into(),
            ),
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
        let make_pipeline = |layouts: &[Option<&wgpu::BindGroupLayout>], fragment: &str| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pert-ui shapes"),
                bind_group_layouts: layouts,
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    entry_point: Some(fragment),
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
            })
        };
        let gradient_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gradient paints"),
            entries: &[0, 1].map(|binding| wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }),
        });
        let pipeline = make_pipeline(&[Some(&layout)], "fragment");
        let gradient_pipeline = make_pipeline(
            &[Some(&layout), Some(&gradient_layout)],
            "gradient_fragment",
        );
        Self {
            pipeline,
            layout,
            gradient_layout,
            gradient_pipeline,
        }
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
        let extent = PhysicalExtent::new(100, 50);
        let encoded = RetainedGeometry::default().prepare_for(
            &scene,
            frame_mapping(&scene, extent),
            u64::MAX,
        )?;
        let records = encoded.records;
        // The span is the rectangle grown by one pixel: 9..51 × 4..26. Its
        // solid box, the inner area below the rows its cut corner reaches
        // and above those of its round one, is 12..48 × 13..21: 288 pixels,
        // at least the tests' `MIN_SOLID_BOX_PIXELS`. So four strips around
        // it are records, and the box is a fill.
        assert_eq!(records.len(), 4 * RECORD_BYTES);
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
            ]
        );
        assert_eq!(encoded.bytes.len(), 6 * 32);
        // The strips have the same parameters.
        for strip in records.chunks(RECORD_BYTES) {
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
    fn a_large_solid_box_is_a_fill_quad_after_the_strips_around_it() -> Outcome {
        let fill = Color::from_linear(0.0, 0.0, 1.0, 0.5)?;
        let line = Color::from_linear(1.0, 0.0, 0.0, 1.0)?;
        let style = ShapeStyle::fill(fill).with_border(Length::new(2.0)?, line);
        let shape = Shape::rounded(Length::new(6.0)?);
        let rect = Rect::new(10.3, 5.0, 150.0, 80.0)?;
        let scene = scene(200.0, 100.0, 1.0, move |p| {
            p.draw_shape(rect, shape, style.clone())
        })?;
        let extent = PhysicalExtent::new(200, 100);
        let mapping = frame_mapping(&scene, extent).ok_or("no frame mapping")?;
        let encoded = RetainedGeometry::default().prepare_for(&scene, Some(mapping), u64::MAX)?;
        let DrawCommand::DrawShape {
            geometry,
            border,
            clip,
            ..
        } = scene.commands()[0]
        else {
            return Err("not a shape".into());
        };
        let prepared = PreparedShape::new(&geometry, border.as_ref(), clip, &mapping)?
            .ok_or("the shape draws nothing")?;
        let [left, top, right, bottom] = prepared.solid_box().ok_or("no solid box")?;
        assert!(u64::from(right - left) * u64::from(bottom - top) >= MIN_SOLID_BOX_PIXELS);
        // Four strips that, with the box, tile the span exactly.
        assert_eq!(encoded.records.len(), 4 * RECORD_BYTES);
        let pixels = |[l, t, r, b]: [f32; 4]| {
            let x = |ndc: f32| f64::from(ndc + 1.0) * 100.0;
            let y = |ndc: f32| f64::from(1.0 - ndc) * 50.0;
            ((x(r) - x(l)) * (y(b) - y(t))).round() as u64
        };
        let strips: u64 = encoded
            .records
            .chunks(RECORD_BYTES)
            .map(|record| pixels(floats(record, 0)))
            .sum();
        let [x0, y0, x1, y1] = prepared.span();
        let solid = u64::from(right - left) * u64::from(bottom - top);
        assert_eq!(strips + solid, u64::from(x1 - x0) * u64::from(y1 - y0));
        // The box after the strips: one quad of the fill pipeline, in the
        // fill's color as straight alpha, since no border reaches it.
        let batches: Vec<_> = encoded
            .batches
            .iter()
            .map(|batch| (batch.source, batch.range.clone()))
            .collect();
        assert_eq!(batches, [(Source::Shape, 0..4), (Source::Fill, 0..6)]);
        let vertex = |index: usize| -> [f32; 6] {
            std::array::from_fn(|value| {
                let start = index * 32 + value * 4;
                f32::from_ne_bytes(
                    encoded.bytes[start..start + 4]
                        .try_into()
                        .expect("four bytes"),
                )
            })
        };
        let ndc_x = |x: u32| (2.0 * f64::from(x) / 200.0 - 1.0) as f32;
        let ndc_y = |y: u32| (1.0 - 2.0 * f64::from(y) / 100.0) as f32;
        assert_eq!(vertex(0), [ndc_x(left), ndc_y(top), 0.0, 0.0, 1.0, 0.5]);
        assert_eq!(vertex(5), [ndc_x(right), ndc_y(bottom), 0.0, 0.0, 1.0, 0.5]);
        Ok(())
    }

    #[test]
    fn a_solid_box_splits_out_from_the_threshold() -> Outcome {
        // Rounded rectangles whose solid boxes have exactly the threshold's
        // pixels, and one fewer.
        let extent = PhysicalExtent::new(100, 100);
        let encode = |width: f32, height: f32| -> Result<_, Box<dyn std::error::Error>> {
            let scene = scene(100.0, 100.0, 1.0, move |p| {
                p.draw_shape(
                    Rect::new(2.0, 2.0, width, height)?,
                    Shape::rounded(Length::new(2.0)?),
                    ShapeStyle::fill(Color::WHITE),
                )
            })?;
            let mapping = frame_mapping(&scene, extent).ok_or("no frame mapping")?;
            let DrawCommand::DrawShape {
                geometry,
                border,
                clip,
                ..
            } = scene.commands()[0]
            else {
                return Err("not a shape".into());
            };
            let area = PreparedShape::new(&geometry, border.as_ref(), clip, &mapping)?
                .and_then(|shape| shape.solid_box())
                .map_or(0, |[l, t, r, b]| u64::from(r - l) * u64::from(b - t));
            let encoded =
                RetainedGeometry::default().prepare_for(&scene, Some(mapping), u64::MAX)?;
            Ok((
                area,
                encoded.records.len() / RECORD_BYTES,
                encoded.bytes.len() / 32,
            ))
        };
        let mut found = [false; 2];
        for width in 1..60u8 {
            for height in 1..60u8 {
                let (area, records, vertices) = encode(f32::from(width), f32::from(height))?;
                if area == MIN_SOLID_BOX_PIXELS {
                    assert_eq!((records, vertices), (4, 6), "{width}×{height}");
                    found[0] = true;
                } else if area == MIN_SOLID_BOX_PIXELS - 1 {
                    assert_eq!((records, vertices), (1, 0), "{width}×{height}");
                    found[1] = true;
                }
            }
        }
        assert_eq!(found, [true, true]);
        Ok(())
    }

    #[test]
    fn a_solid_box_color_is_straight_and_takes_a_border_that_covers_it() -> Outcome {
        // Subnormal, tiny and ordinary alphas with channels up to the alpha.
        for alpha in [f32::from_bits(1), 1.0e-38, 1.0e-20, 0.25, 1.0] {
            let solid = SolidBox {
                ndc: [0.0; 4],
                color: [alpha, alpha * 0.5, 0.0, alpha],
            };
            let straight = solid.straight();
            assert!(straight.iter().all(|value| (0.0..=1.0).contains(value)));
            assert_eq!((straight[0], straight[3]), (1.0, alpha));
        }
        // A border wider than half the rectangle covers all of it, its box
        // included, which takes the border's color over the fill.
        let border = Color::from_linear(1.0, 0.0, 0.0, 0.5)?;
        let scene = scene(100.0, 100.0, 1.0, move |p| {
            p.draw_shape(
                Rect::new(2.0, 2.0, 60.0, 60.0)?,
                Shape::rounded(Length::new(2.0)?),
                ShapeStyle::fill(Color::WHITE).with_border(Length::new(40.0)?, border),
            )
        })?;
        let encoded = RetainedGeometry::default().prepare_for(
            &scene,
            frame_mapping(&scene, PhysicalExtent::new(100, 100)),
            u64::MAX,
        )?;
        assert_eq!(encoded.bytes.len(), 6 * 32);
        let channel = |index: usize| {
            let start = 8 + index * 4;
            f32::from_ne_bytes(
                encoded.bytes[start..start + 4]
                    .try_into()
                    .expect("four bytes"),
            )
        };
        // Premultiplied `B·1 + F·(1 − B.a·1)` is (0.5 + 0.5, 0.5, 0.5, 1).
        assert_eq!([0, 1, 2, 3].map(channel), [1.0, 0.5, 0.5, 1.0]);
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
        // Each drawn shape, too small for a solid box of its own, has one
        // record, and consecutive shapes are one batch.
        assert_eq!(encoded.records.len(), 3 * RECORD_BYTES);
        let batches: Vec<_> = encoded
            .batches
            .iter()
            .map(|batch| (batch.source, batch.range.clone()))
            .collect();
        assert_eq!(
            batches,
            [
                (Source::Shape, 0..2),
                (Source::Fill, 0..6),
                (Source::Shape, 2..3)
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
        // A moved shape encodes only its own records.
        let moved = scene(64.0, 64.0, 1.0, paint(6.0))?;
        let next = same.prepare_for(&moved, frame_mapping(&moved, extent), u64::MAX)?;
        assert_eq!(next.encoded_commands(), 1);
        let first = RECORD_BYTES;
        assert_eq!(next.records[..first], cache.records[..first]);
        assert_ne!(next.records[first..], cache.records[first..]);
        Ok(())
    }

    #[test]
    fn a_command_whose_record_count_changes_keeps_later_commands_in_place() -> Outcome {
        // A fill and a small shape come first, so the varying command starts
        // at nonzero vertex and record offsets. It is a filled frame (strips,
        // and a box drawn as a fill quad), a frame without fill (strips only,
        // its transparent box left out), a pill (one record) or absent. The
        // commands after it are unchanged, among them two shapes clipped to
        // their boxes, which leave no strip and are only fill quads; their
        // records, quads and batches stay in place.
        let paint = |first: u8| {
            move |p: &mut Painter| -> Result<(), DrawError> {
                let frame = Rect::new(2.0, 2.0, 100.0, 80.0)?;
                let line = Length::new(1.0)?;
                p.fill_rect(Rect::new(150.0, 2.0, 5.0, 5.0)?, Color::WHITE);
                p.draw_shape(
                    Rect::new(150.0, 20.0, 12.0, 12.0)?,
                    Shape::rounded(Length::new(3.0)?),
                    ShapeStyle::fill(Color::WHITE),
                )?;
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
                p.fill_rect(Rect::new(110.0, 2.0, 5.0, 5.0)?, Color::WHITE);
                p.draw_shape(
                    Rect::new(2.0, 100.0, 12.0, 12.0)?,
                    Shape::rounded(Length::new(3.0)?),
                    ShapeStyle::fill(Color::WHITE),
                )?;
                p.fill_triangle(
                    [
                        Point::new(140.0, 140.0)?,
                        Point::new(150.0, 140.0)?,
                        Point::new(140.0, 150.0)?,
                    ],
                    Color::WHITE,
                )?;
                for y in [125.0, 160.0] {
                    p.with_clip(Rect::new(30.0, y, 40.0, 30.0)?, |p| {
                        p.draw_shape(
                            Rect::new(10.0, 120.0, 100.0, 75.0)?,
                            Shape::rounded(Length::new(3.0)?),
                            ShapeStyle::fill(Color::WHITE),
                        )
                    })?;
                }
                Ok(())
            }
        };
        let extent = PhysicalExtent::new(200, 200);
        let scenes = [0, 1, 2, 3, 0, 2, 1, 3, 1, 0]
            .into_iter()
            .map(|first| scene(200.0, 200.0, 1.0, paint(first)))
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
            // Both match encoding the whole scene at once, which places
            // every batch without rebasing a retained slot.
            let mut records = Vec::new();
            let (bytes, batches) = geometry_commands(
                scene,
                scene.commands(),
                mapping.as_ref(),
                &mut records,
                &mut None,
            )?;
            let whole: Vec<_> = batches
                .iter()
                .map(|batch| (batch.source, batch.range.clone()))
                .collect();
            assert_eq!(ranges(&cache), whole);
            assert_eq!(cache.bytes, bytes);
            assert_eq!(cache.records, records);
            if std::ptr::eq(scene, &scenes[0]) {
                // The frame's strips join the small shape's record, its box
                // the fill before it and the one after, and the clipped
                // shapes' boxes are one fill batch.
                assert_eq!(
                    whole,
                    [
                        (Source::Fill, 0..6),
                        (Source::Shape, 0..5),
                        (Source::Fill, 6..18),
                        (Source::Shape, 5..7),
                        (Source::Fill, 18..30),
                    ]
                );
            }
            counts.push(fresh.records.len() / RECORD_BYTES);
        }
        // The small shapes' records and the triangle's surround four, one or
        // no record of the varying command; the clipped shapes have none.
        assert_eq!(counts, [7, 7, 4, 3, 7, 4, 7, 3, 7, 7]);
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
        let exact = 3 * RECORD_BYTES as u64;
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
