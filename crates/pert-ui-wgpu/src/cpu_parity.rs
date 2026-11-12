//! CPU/GPU parity: one fixture corpus rendered by the hardware renderer and by
//! `pert_ui_cpu::CpuRenderer`, compared under the rendering contract's tiers.
//!
//! The exact tier (integer-aligned opaque fills in colors that are exact sRGB
//! bytes, nearest-sampled images and their quarter turns, the opaque clear)
//! requires equal bytes. The tolerance
//! tier (translucency, bilinear filtering, text and vector coverage, fractional
//! edges) allows a difference of at most one per 8-bit channel; it does not
//! classify which pixels may differ, and arbitrary linear colors differ by one
//! in the interior where the API's sRGB encoding rounds differently. A failing
//! fixture writes both images to the temporary directory. Fractional edges stay
//! clear of pixel centers: an edge within the GPU's subpixel precision of one
//! may be snapped across it, which the rendering contract leaves outside both
//! tiers. Evidence for the
//! adapter it ran on; run through `pert-ui-native-checks`.
//!
//! Analytic shapes have a third tier. Each pixel of a shape fixture is
//! classified from the CPU reference: it is an *edge* pixel when a shape whose
//! span holds it decides it neither by exact arithmetic nor with every
//! reference distance at least half a pixel plus
//! [`PreparedShape::PARITY_MARGIN`] from its boundary. Edge pixels may differ
//! by at most [`EDGE_TIER`]; every other pixel of a shape fixture is in the
//! tolerance tier, or in the exact tier where the fixture says so. The run
//! prints the largest difference of each tier.
//!
//! The edge tier is one step per channel and the margin 1/16 pixel: on
//! 2026-11-11 the corpus measured a largest edge difference of 1, and no
//! decided pixel outside its tier, on Intel UHD Graphics 770 with Mesa on
//! Vulkan, the only adapter available. They are measurements on that adapter.
//! A larger difference on another one is a finding to correct in the shader
//! or the preparation, up to [`EDGE_LIMIT`], beyond which the approach itself
//! is in question; `docs/rendering-platform.md` states the tiers.
use super::native_smoke::native_test_event_loop;
use super::*;
use pert_ui_runtime::{
    Column, Component, DrawError, Element, FillRule, Icon, ImageSampling, Label, Length, MaskImage,
    Painter, Path, Point, PointerEvent, PointerKind, QuarterTurn, RasterImage, ReadSignal, Rect,
    RichText, Runtime, RuntimeError, ScrollArea, Shape, TextColor, TextRenderingMode, TextStyle,
    UpdateContext, Viewport,
};
use pert_ui_runtime::{Corner, CornerPosition, FrameMapping, PreparedShape, ShapeStyle};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

/// The corpus's logical and physical size.
const SIZE: u32 = 64;

/// The largest channel difference an edge pixel of an analytic shape may
/// show: what the corpus measured on the available adapter.
const EDGE_TIER: u8 = 1;

/// The largest value [`EDGE_TIER`] may ever be raised to after a measurement
/// on another adapter.
const EDGE_LIMIT: u8 = 4;

// The tier never exceeds its limit.
const _: () = assert!(EDGE_TIER <= EDGE_LIMIT);

type Paint = Box<dyn Fn(&mut Painter) -> Result<(), DrawError>>;

/// Paints one fixture.
struct Fixture(Paint);
impl Component for Fixture {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        (self.0)(painter)
    }
}

/// One fixture: its name, whether it is in the exact tier, and its scene.
struct Case {
    name: &'static str,
    exact: bool,
    scene: DisplayList,
}

fn case(
    name: &'static str,
    exact: bool,
    paint: impl Fn(&mut Painter) -> Result<(), DrawError> + 'static,
) -> Result<Case, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(SIZE as f32, SIZE as f32, 1.0)?)?;
    ui.mount_tree(Element::new("fixture", Fixture(Box::new(paint))))?;
    ui.flush(std::time::Duration::ZERO)?;
    Ok(Case {
        name,
        exact,
        scene: ui.paint()?,
    })
}

/// A fixture laid out from an element tree at a physical extent and scale.
fn tree(
    name: &'static str,
    scale: f64,
    element: pert_ui_runtime::Element,
) -> Result<Case, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        pert_ui_runtime::PhysicalExtent::new(SIZE, SIZE),
        scale,
    )?)?;
    ui.mount_tree(element)?;
    ui.flush(std::time::Duration::ZERO)?;
    Ok(Case {
        name,
        exact: false,
        scene: ui.paint()?,
    })
}

/// A fixture of analytic shapes on a `SIZE`-pixel frame at `scale`: every
/// shape method records `DrawShape`.
fn shapes(
    name: &'static str,
    exact: bool,
    scale: f64,
    paint: impl Fn(&mut Painter) -> Result<(), DrawError> + 'static,
) -> Result<Case, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        pert_ui_runtime::PhysicalExtent::new(SIZE, SIZE),
        scale,
    )?)?;
    ui.mount_tree(Element::new("fixture", Fixture(Box::new(paint))))?;
    ui.flush(std::time::Duration::ZERO)?;
    Ok(Case {
        name,
        exact,
        scene: ui.paint()?,
    })
}

/// Which pixels of `scene` are edge pixels of an analytic shape, by the CPU
/// reference; empty when the scene has no shape.
fn edge_pixels(scene: &DisplayList) -> Result<Vec<bool>, Box<dyn std::error::Error>> {
    let mut edges = Vec::new();
    let mapping = FrameMapping::new(SIZE, SIZE, scene.viewport())?;
    let clear = 0.5 + PreparedShape::PARITY_MARGIN;
    for command in scene.commands() {
        let DrawCommand::DrawShape {
            geometry,
            border,
            clip,
            ..
        } = command
        else {
            continue;
        };
        if edges.is_empty() {
            edges = vec![false; (SIZE * SIZE) as usize];
        }
        let Some(shape) = PreparedShape::new(geometry, border.as_ref(), *clip, &mapping)? else {
            continue;
        };
        let [x0, y0, x1, y1] = shape.span();
        for y in y0..y1 {
            for x in x0..x1 {
                let coverage = shape.coverage(x, y);
                let decided = coverage.exact_arithmetic()
                    || (coverage.outer_distance().abs() >= clear
                        && coverage
                            .inner_distance()
                            .is_none_or(|distance| distance.abs() >= clear));
                edges[(y * SIZE + x) as usize] |= !decided;
            }
        }
    }
    Ok(edges)
}

/// The shape fixtures: every geometry at sub-pixel phases and three scales.
fn shape_corpus() -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    let red = Color::from_srgb8(230, 40, 20, 255);
    let green = Color::from_srgb8(30, 180, 60, 255);
    let translucent = Color::from_linear(0.2, 0.4, 1.0, 0.45)?;
    let orange = Color::from_linear(1.0, 0.5, 0.0, 0.3)?;
    let length = |value: f32| Length::new(value);
    let mut cases = Vec::new();
    for (scale, name) in [
        (1.0, "shapes: rounded and cut, scale 1"),
        (1.25, "shapes: rounded and cut, scale 1.25"),
        (2.0, "shapes: rounded and cut, scale 2"),
    ] {
        cases.push(shapes(name, true, scale, move |p| {
            // Opaque exact colors at fractional positions: interiors are exact.
            p.draw_shape(
                Rect::new(1.3, 1.7, 14.4, 9.2)?,
                Shape::rounded(length(3.0)?),
                ShapeStyle::fill(red),
            )?;
            let mixed = Shape::rounded(length(2.5)?)
                .with_corner(CornerPosition::TopRight, Corner::cut(length(4.0)?))
                .with_corner(CornerPosition::BottomLeft, Corner::SQUARE);
            p.draw_shape(
                Rect::new(17.6, 2.2, 12.1, 10.9)?,
                mixed,
                ShapeStyle::fill(green).with_border(length(1.0)?, red),
            )?;
            // One corner past half, round and cut, and a radius below a pixel.
            let single =
                Shape::RECTANGLE.with_corner(CornerPosition::TopLeft, Corner::round(length(9.0)?));
            p.draw_shape(
                Rect::new(1.1, 13.4, 14.0, 9.0)?,
                single,
                ShapeStyle::fill(green),
            )?;
            p.draw_shape(
                Rect::new(17.2, 14.6, 12.0, 8.0)?,
                Shape::cut_top_right(length(8.0)?),
                ShapeStyle::fill(red),
            )?;
            p.draw_shape(
                Rect::new(1.4, 24.3, 6.0, 5.0)?,
                Shape::rounded(length(0.6)?),
                ShapeStyle::fill(red),
            )?;
            // Corners that do not fit, a border wider than half and a hairline.
            p.draw_shape(
                Rect::new(9.3, 24.1, 9.0, 5.5)?,
                Shape::rounded(length(20.0)?),
                ShapeStyle::fill(green).with_border(length(1.5)?, red),
            )?;
            p.draw_shape(
                Rect::new(20.2, 24.4, 9.0, 4.0)?,
                Shape::rounded(length(1.5)?),
                ShapeStyle::fill(green).with_border(length(3.0)?, red),
            )?;
            p.stroke_shape(
                Rect::new(1.2, 30.1, 0.4, 1.2)?,
                Shape::cut_top_right(length(0.3)?),
                length(0.2)?,
                green,
            )
        })?);
    }
    for (scale, name) in [
        (1.0, "shapes: translucent, scale 1"),
        (1.25, "shapes: translucent, scale 1.25"),
        (2.0, "shapes: translucent, scale 2"),
    ] {
        cases.push(shapes(name, false, scale, move |p| {
            // A translucent border over a translucent fill, over a fill.
            p.fill_rect(Rect::new(4.0, 4.0, 20.0, 20.0)?, red);
            p.draw_shape(
                Rect::new(2.3, 2.6, 26.2, 16.4)?,
                Shape::rounded(length(5.0)?)
                    .with_corner(CornerPosition::BottomRight, Corner::cut(length(6.0)?)),
                ShapeStyle::fill(translucent).with_border(length(2.0)?, orange),
            )?;
            // Clipped at fractional edges.
            p.with_clip(Rect::new(6.4, 20.3, 17.7, 7.1)?, |p| {
                p.draw_shape(
                    Rect::new(2.0, 18.0, 27.0, 12.0)?,
                    Shape::rounded(length(4.0)?),
                    ShapeStyle::fill(orange).with_border(length(1.0)?, translucent),
                )
            })
        })?);
    }
    for (scale, name) in [
        (1.0, "shapes: arcs and triangles, scale 1"),
        (1.25, "shapes: arcs and triangles, scale 1.25"),
        (2.0, "shapes: arcs and triangles, scale 2"),
    ] {
        cases.push(shapes(name, false, scale, move |p| {
            // A full ring, a part with caps, a thin stroke and a tiny arc.
            p.stroke_arc(
                Point::new(8.3, 8.6)?,
                length(6.0)?,
                0.0,
                360.0,
                length(2.0)?,
                red,
            )?;
            p.stroke_arc(
                Point::new(22.7, 8.2)?,
                length(6.0)?,
                33.0,
                200.0,
                length(2.5)?,
                green,
            )?;
            p.stroke_arc(
                Point::new(8.4, 23.1)?,
                length(6.0)?,
                -70.0,
                290.0,
                length(0.4)?,
                red,
            )?;
            p.stroke_arc(
                Point::new(18.0, 20.0)?,
                length(1.2)?,
                10.0,
                90.0,
                length(0.8)?,
                green,
            )?;
            p.stroke_arc(
                Point::new(25.1, 25.4)?,
                length(4.0)?,
                271.0,
                0.5,
                length(3.0)?,
                translucent,
            )?;
            // Triangles in both windings, an obtuse one and a sliver.
            p.fill_triangle(
                [
                    Point::new(14.2, 14.1)?,
                    Point::new(21.9, 15.3)?,
                    Point::new(15.4, 19.8)?,
                ],
                translucent,
            )?;
            p.fill_triangle(
                [
                    Point::new(22.3, 16.2)?,
                    Point::new(24.1, 23.7)?,
                    Point::new(30.6, 17.4)?,
                ],
                orange,
            )?;
            p.fill_triangle(
                [
                    Point::new(1.2, 30.4)?,
                    Point::new(30.7, 30.9)?,
                    Point::new(16.0, 31.3)?,
                ],
                red,
            )
        })?);
    }
    // Geometry far beyond the frame: relocated edges and a clipped view of a
    // large ring.
    cases.push(shapes("shapes: beyond the frame", false, 1.0, move |p| {
        p.draw_shape(
            Rect::new(-30_000.0, 40.3, 30_050.4, 30_000.0)?,
            Shape::rounded(length(9.0)?),
            ShapeStyle::fill(translucent).with_border(length(2.0)?, red),
        )?;
        p.stroke_arc(
            Point::new(-900.0, 20.0)?,
            length(940.0)?,
            350.0,
            20.0,
            length(3.0)?,
            green,
        )
    })?);
    cases.push(domain_edge_case()?);
    for (scale, name) in [
        (1.0, "shapes: thresholds, scale 1"),
        (1.25, "shapes: thresholds, scale 1.25"),
        (2.0, "shapes: thresholds, scale 2"),
    ] {
        cases.push(shapes(name, false, scale, move |p| {
            // Border widths on either side of the rounding threshold.
            for (index, width) in [1.49f32, 1.5, 0.49, 0.5, 2.5].into_iter().enumerate() {
                p.draw_shape(
                    Rect::new(1.3 + 6.1 * index as f32, 1.2, 5.2, 7.4)?,
                    Shape::rounded(length(1.5)?),
                    ShapeStyle::fill(translucent).with_border(length(width)?, red),
                )?;
            }
            // Arcs approaching zero width and zero sweep.
            for (index, (sweep, width)) in [
                (300.0f32, 0.05f32),
                (300.0, 0.004),
                (0.01, 2.0),
                (0.01, 0.3),
                (359.99, 1.0),
            ]
            .into_iter()
            .enumerate()
            {
                p.stroke_arc(
                    Point::new(4.2 + 6.0 * index as f32, 14.3)?,
                    length(2.4)?,
                    17.0,
                    sweep,
                    length(width)?,
                    green,
                )?;
            }
            // Triangles close to degenerate: a short edge and a thin needle.
            p.fill_triangle(
                [
                    Point::new(2.0, 20.0)?,
                    Point::new(2.01, 20.0)?,
                    Point::new(2.0, 30.0)?,
                ],
                red,
            )?;
            p.fill_triangle(
                [
                    Point::new(5.0, 20.3)?,
                    Point::new(30.0, 20.4)?,
                    Point::new(5.0, 20.5)?,
                ],
                green,
            )?;
            p.fill_triangle(
                [
                    Point::new(5.1, 23.2)?,
                    Point::new(12.3, 23.2)?,
                    Point::new(8.7, 23.9)?,
                ],
                orange,
            )
        })?);
    }
    Ok(cases)
}

/// Shapes whose prepared parameters lie far from the span, near the
/// evaluation domain's limit of 65 536 pixels, where `f32` has the fewest
/// fraction bits left.
fn domain_edge_case() -> Result<Case, Box<dyn std::error::Error>> {
    let red = Color::from_srgb8(230, 40, 20, 255);
    let green = Color::from_srgb8(30, 180, 60, 255);
    let translucent = Color::from_linear(0.2, 0.4, 1.0, 0.45)?;
    let length = |value: f32| Length::new(value);
    shapes("shapes: near the domain's edge", false, 1.0, move |p| {
        // A corner radius of about 16 000 pixels, whose center is that far
        // from the frame.
        p.draw_shape(
            Rect::new(-32_000.0, -32_000.0, 32_050.3, 32_040.6)?,
            Shape::rounded(length(30_000.0)?),
            ShapeStyle::fill(translucent).with_border(length(2.0)?, red),
        )?;
        // An arc of radius 30 020 around a center 30 000 pixels to the left.
        p.stroke_arc(
            Point::new(-30_000.0, 31.7)?,
            length(30_020.0)?,
            355.0,
            10.0,
            length(3.0)?,
            green,
        )?;
        // A triangle with a vertex 65 000 pixels away.
        p.fill_triangle(
            [
                Point::new(-65_000.0, 10.2)?,
                Point::new(40.3, 44.1)?,
                Point::new(40.3, 60.2)?,
            ],
            red,
        )
    })
}

#[test]
fn the_domain_fixture_evaluates_far_from_its_span() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui_runtime::ShapeParameters;
    let case = domain_edge_case()?;
    let mapping = FrameMapping::new(SIZE, SIZE, case.scene.viewport())?;
    let mut farthest = Vec::new();
    for command in case.scene.commands() {
        let DrawCommand::DrawShape {
            geometry,
            border,
            clip,
            ..
        } = command
        else {
            continue;
        };
        let shape = PreparedShape::new(geometry, border.as_ref(), *clip, &mapping)?
            .ok_or("a domain fixture shape draws nothing")?;
        farthest.push(match shape.parameters() {
            ShapeParameters::Rect { outer, .. } => outer
                .corners()
                .iter()
                .map(|corner| corner.size())
                .fold(0.0f32, f32::max),
            ShapeParameters::Arc { center, radius, .. } => center[0].abs().min(radius),
            ShapeParameters::Triangle { vertices, .. } => vertices
                .iter()
                .flatten()
                .fold(0.0f32, |a, b| a.max(b.abs())),
            _ => 0.0,
        });
    }
    // The corner, the arc and the triangle, each with a parameter of at
    // least 16 000 pixels, the last close to the limit.
    assert_eq!(farthest.len(), 3, "{farthest:?}");
    assert!(
        farthest.iter().all(|value| *value >= 16_000.0),
        "{farthest:?}"
    );
    assert!(farthest[2] >= 64_000.0, "{farthest:?}");
    Ok(())
}

fn text_mode_case(
    name: &'static str,
    mode: TextRenderingMode,
    scale: f64,
) -> Result<Case, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        pert_ui_runtime::PhysicalExtent::new(SIZE, SIZE),
        scale,
    )?)?;
    ui.ui_settings().set_text_rendering(mode)?;
    ui.mount_tree(Element::new(
        "text",
        Fixture(Box::new(|p| {
            p.text("Il1 O0", None, TextStyle::Body)?
                .paint(p, Point::new(3.25, 7.25)?, Color::WHITE)
        })),
    ))?;
    ui.flush(std::time::Duration::ZERO)?;
    Ok(Case {
        name,
        exact: false,
        scene: ui.paint()?,
    })
}

fn rich_icon_case(name: &'static str, scale: f64) -> Result<Case, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        pert_ui_runtime::PhysicalExtent::new(SIZE, SIZE),
        scale,
    )?)?;
    ui.ui_settings()
        .set_text_rendering(TextRenderingMode::PhaseLine)?;
    ui.mount_tree(Element::new(
        "rich",
        Fixture(Box::new(|p| {
            let rich = RichText::new().text("H").gap().tinted_icon(
                Icon::Settings,
                TextColor::Literal(Color::from_srgb8(255, 0, 0, 255)),
            );
            p.rich_text(&rich, None, TextStyle::Body)?.paint(
                p,
                Point::new(3.25, 7.25)?,
                Color::WHITE,
            )
        })),
    ))?;
    ui.flush(std::time::Duration::ZERO)?;
    Ok(Case {
        name,
        exact: false,
        scene: ui.paint()?,
    })
}

fn checker() -> Result<RasterImage, DrawError> {
    let mut pixels = Vec::new();
    for index in 0..16u32 {
        let on = (index % 4 + index / 4) % 2 == 0;
        pixels.extend_from_slice(if on {
            &[230, 90, 20, 255]
        } else {
            &[20, 60, 200, 255]
        });
    }
    RasterImage::from_srgba8(4, 4, pixels)
}

fn corpus() -> Result<Vec<Case>, Box<dyn std::error::Error>> {
    // Exact sRGB bytes, so encoding them back is exact on every backend.
    let red = Color::from_srgb8(230, 40, 20, 255);
    let green = Color::from_srgb8(30, 180, 60, 255);
    // An arbitrary linear color, whose encoding graphics APIs may round
    // differently by one step.
    let linear = Color::from_linear(0.9, 0.1, 0.05, 1.0)?;
    let translucent = Color::from_linear(0.2, 0.4, 1.0, 0.45)?;
    let orange = Color::from_linear(1.0, 0.5, 0.0, 0.3)?;
    Ok(vec![
        case("opaque clear", true, |_| Ok(()))?,
        case("opaque integer fills", true, move |p| {
            p.fill_rect(Rect::new(4.0, 4.0, 20.0, 12.0)?, red);
            p.fill_rect(Rect::new(16.0, 10.0, 30.0, 40.0)?, green);
            Ok(())
        })?,
        case("nearest image", true, |p| {
            p.draw_image(
                &checker()?,
                Rect::new(8.0, 8.0, 32.0, 32.0)?,
                ImageSampling::Nearest,
            );
            Ok(())
        })?,
        case("nearest image turned", true, |p| {
            p.draw_image_turned(
                &checker()?,
                Rect::new(8.0, 8.0, 32.0, 48.0)?,
                ImageSampling::Nearest,
                Color::WHITE,
                QuarterTurn::Right,
            );
            Ok(())
        })?,
        case("arbitrary linear color", false, move |p| {
            p.fill_rect(Rect::new(4.0, 4.0, 40.0, 20.0)?, linear);
            Ok(())
        })?,
        case("translucent overlap", false, move |p| {
            p.fill_rect(Rect::new(0.0, 0.0, 40.0, 40.0)?, translucent);
            p.fill_rect(Rect::new(20.0, 20.0, 40.0, 40.0)?, orange);
            Ok(())
        })?,
        case("fractional edges", false, move |p| {
            p.fill_rect(Rect::new(3.2, 5.7, 20.3, 9.4)?, red);
            Ok(())
        })?,
        case("linear image", false, |p| {
            p.draw_image(
                &checker()?,
                Rect::new(4.0, 4.0, 56.0, 56.0)?,
                ImageSampling::Linear,
            );
            Ok(())
        })?,
        case("tinted mask", false, move |p| {
            let mask = MaskImage::new(3, 2, vec![0, 128, 255, 255, 64, 0])?;
            p.draw_mask(&mask, Rect::new(2.0, 2.0, 60.0, 40.0)?, translucent);
            Ok(())
        })?,
        tree(
            "text",
            1.0,
            pert_ui_runtime::Element::new("label", pert_ui_runtime::Label::new("Pert ÅÄÖ fi 42")),
        )?,
        text_mode_case("text phase line", TextRenderingMode::PhaseLine, 1.0)?,
        text_mode_case("text phase line 75%", TextRenderingMode::PhaseLine, 0.75)?,
        text_mode_case("text phase line 90%", TextRenderingMode::PhaseLine, 0.9)?,
        text_mode_case("text phase line 125%", TextRenderingMode::PhaseLine, 1.25)?,
        rich_icon_case("phase inline icon 75%", 0.75)?,
        rich_icon_case("phase inline icon 90%", 0.9)?,
        rich_icon_case("phase inline icon 125%", 1.25)?,
        tree(
            "fractional scale",
            1.5,
            pert_ui_runtime::Element::new("button", pert_ui_runtime::Button::new("Go", |_| Ok(()))),
        )?,
        tree(
            "thin rounded outline at 75%",
            0.75,
            pert_ui_runtime::Element::new(
                "outline",
                Fixture(Box::new(|p| {
                    p.stroke_shape(
                        Rect::new(10.25, 10.25, 40.0, 20.0)?,
                        Shape::rounded(Length::new(4.0)?),
                        Length::new(1.0)?,
                        Color::WHITE,
                    )
                })),
            ),
        )?,
        case("vector circle", false, move |p| {
            let circle = Path::circle(Point::new(32.0, 32.0)?, Length::new(20.0)?)?;
            p.fill_path(&circle, FillRule::NonZero, green)?;
            Ok(())
        })?,
    ])
}

struct Parity {
    error: Option<Box<dyn std::error::Error>>,
}
impl ApplicationHandler<Notice> for Parity {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("pert-ui CPU/GPU parity")
                        .with_inner_size(winit::dpi::PhysicalSize::new(SIZE, SIZE)),
                )?,
            );
            let mut renderer =
                Renderer::new(window.clone(), PhysicalExtent::new(SIZE, SIZE), |_| {})?;
            let info = renderer.adapter.get_info();
            println!("Parity adapter: {} ({:?})", info.name, info.backend);
            let mut cpu = pert_ui_cpu::CpuRenderer::new();
            let mut failures = Vec::new();
            let mut edge_tier = 0;
            for case in corpus()?.into_iter().chain(shape_corpus()?) {
                let gpu = renderer.render_offscreen(case.scene.clone())?;
                let software = cpu.render(&case.scene)?;
                let edges = edge_pixels(&case.scene)?;
                // An edge pixel of a shape is judged by the edge tier, every
                // other pixel by the fixture's own tier.
                let (mut largest, mut differing, mut edge_largest, mut edge_count) = (0, 0, 0, 0);
                for (index, (a, b)) in gpu
                    .pixels()
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(software.pixels().as_chunks::<4>().0.iter())
                    .enumerate()
                {
                    let difference = a
                        .iter()
                        .zip(b)
                        .map(|(a, b)| a.abs_diff(*b))
                        .max()
                        .unwrap_or(0);
                    if edges.get(index).copied().unwrap_or(false) {
                        edge_count += 1;
                        edge_largest = edge_largest.max(difference);
                    } else {
                        largest = largest.max(difference);
                        differing += usize::from(difference > 0);
                    }
                }
                edge_tier = edge_tier.max(edge_largest);
                let allowed = if case.exact { 0 } else { 1 };
                println!(
                    "Parity {}: {} tier, largest channel difference {largest}, {differing} pixels differ; {edge_count} edge pixels, largest difference {edge_largest}",
                    case.name,
                    if case.exact { "exact" } else { "tolerance" },
                );
                if edge_largest > EDGE_TIER {
                    largest = largest.max(edge_largest);
                }
                if largest > allowed {
                    // Both images go beside each other in the temporary
                    // directory, so the difference can be inspected.
                    let stem = std::env::temp_dir()
                        .join(format!("pert-ui-parity-{}", case.name.replace(' ', "-")));
                    let gpu_path = stem.with_extension("gpu.png");
                    let cpu_path = stem.with_extension("cpu.png");
                    std::fs::write(&gpu_path, gpu.encode_png())?;
                    std::fs::write(&cpu_path, software.encode_png())?;
                    failures.push(format!(
                        "{}: difference {largest} > {allowed}; see {} and {}",
                        case.name,
                        gpu_path.display(),
                        cpu_path.display()
                    ));
                }
            }
            println!(
                "Parity edge tier: largest difference {edge_tier} over the shape fixtures (bound {EDGE_TIER})"
            );
            if failures.is_empty() {
                Ok(())
            } else {
                Err(failures.join("; ").into())
            }
        })();
        self.error = result.err();
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

/// Renders the corpus on both renderers and checks the tiers.
pub(super) fn cpu_gpu_parity() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = native_test_event_loop()?;
    let mut check = Parity { error: None };
    event_loop.run_app(&mut check)?;
    match check.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct MovingText {
    offset: ReadSignal<f32>,
    current: f32,
}

impl Component for MovingText {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read_paint(&self.offset)?;
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        for row in 0..20 {
            painter
                .text(
                    &format!("Hamburgefonts 0123456789 — Il1 O0 row {row}"),
                    None,
                    TextStyle::Body,
                )?
                .paint(
                    painter,
                    Point::new(12.0 + self.current, 12.0 + row as f32 * 22.0)?,
                    Color::WHITE,
                )?;
        }
        Ok(())
    }
}

fn percentile(samples: &mut [std::time::Duration], percentile: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percentile / 100].as_micros()
}

struct TextModeProbe {
    error: Option<Box<dyn std::error::Error>>,
}

impl ApplicationHandler<Notice> for TextModeProbe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let window = Arc::new(
                event_loop.create_window(
                    Window::default_attributes()
                        .with_title("pert-ui text modes")
                        .with_inner_size(winit::dpi::PhysicalSize::new(800, 500)),
                )?,
            );
            let mut gpu = Renderer::new(window, PhysicalExtent::new(800, 500), |_| {})?;
            let mut cpu = pert_ui_cpu::CpuRenderer::new();
            // `pert-ui-native-checks` finds the adapter by this `Hardware: `
            // prefix and the `;` after it; its tests pin the text.
            println!("Hardware: {:?}; text modes", gpu.adapter.get_info());
            for mode in [TextRenderingMode::LineMask, TextRenderingMode::PhaseLine] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
                ui.ui_settings().set_text_rendering(mode)?;
                let offset = ui.signal(0.0f32);
                ui.mount_tree(Element::new(
                    "moving-text",
                    MovingText {
                        offset: offset.read_only(),
                        current: 0.0,
                    },
                ))?;
                ui.flush(std::time::Duration::ZERO)?;
                let mut preparation = Vec::new();
                let mut cpu_render = Vec::new();
                let mut gpu_render = Vec::new();
                let mut commands = 0usize;
                let mut mask_bytes = 0usize;
                for frame in 0..32 {
                    offset.set((frame % 4) as f32 * 0.25)?;
                    let start = std::time::Instant::now();
                    ui.flush(std::time::Duration::ZERO)?;
                    let scene = ui.paint()?;
                    preparation.push(start.elapsed());
                    commands = scene.len();
                    mask_bytes = scene.masks().iter().map(|mask| mask.pixels().len()).sum();
                    let start = std::time::Instant::now();
                    let cpu_frame = std::hint::black_box(cpu.render(&scene)?);
                    cpu_render.push(start.elapsed());
                    let start = std::time::Instant::now();
                    let gpu_frame = std::hint::black_box(gpu.render_offscreen(scene)?);
                    gpu_render.push(start.elapsed());
                    if frame == 1
                        && let Some(directory) = std::env::var_os("PERT_UI_TEXT_MODE_REPORT_DIR")
                    {
                        let directory = std::path::Path::new(&directory);
                        std::fs::create_dir_all(directory)?;
                        std::fs::write(
                            directory.join(format!("{mode:?}-cpu.png")),
                            cpu_frame.encode_png(),
                        )?;
                        std::fs::write(
                            directory.join(format!("{mode:?}-gpu.png")),
                            gpu_frame.encode_png(),
                        )?;
                    }
                }
                println!(
                    "Text mode {mode:?}: commands {commands}, scene mask bytes {mask_bytes}; prepare median/p95 {} / {} us, CPU render {} / {} us, GPU offscreen+readback {} / {} us",
                    percentile(&mut preparation.clone(), 50),
                    percentile(&mut preparation, 95),
                    percentile(&mut cpu_render.clone(), 50),
                    percentile(&mut cpu_render, 95),
                    percentile(&mut gpu_render.clone(), 50),
                    percentile(&mut gpu_render, 95),
                );

                let mut scrolled = Runtime::new();
                scrolled.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
                scrolled.ui_settings().set_text_rendering(mode)?;
                scrolled.mount_tree(Element::new(
                    "scroll",
                    ScrollArea::new(Element::new(
                        "rows",
                        Column::new((0..50).map(|row| {
                            Element::new(
                                format!("row-{row}"),
                                Label::new(format!("Hamburgefonts 0123456789 — Il1 O0 row {row}")),
                            )
                        })),
                    )),
                ))?;
                scrolled.flush(std::time::Duration::ZERO)?;
                let mut preparation = Vec::new();
                let mut cpu_render = Vec::new();
                let mut gpu_render = Vec::new();
                let mut commands = 0usize;
                let mut mask_bytes = 0usize;
                for frame in 0..32 {
                    let start = std::time::Instant::now();
                    scrolled.pointer_input(PointerEvent::new(
                        Point::new(20.0, 20.0)?,
                        PointerKind::Scroll(Point::new(0.0, 0.25)?),
                    ))?;
                    scrolled.flush(std::time::Duration::ZERO)?;
                    let scene = scrolled.paint()?;
                    preparation.push(start.elapsed());
                    commands = scene.len();
                    mask_bytes = scene.masks().iter().map(|mask| mask.pixels().len()).sum();
                    let start = std::time::Instant::now();
                    let cpu_frame = std::hint::black_box(cpu.render(&scene)?);
                    cpu_render.push(start.elapsed());
                    let start = std::time::Instant::now();
                    let gpu_frame = std::hint::black_box(gpu.render_offscreen(scene)?);
                    gpu_render.push(start.elapsed());
                    if frame == 1
                        && let Some(directory) = std::env::var_os("PERT_UI_TEXT_MODE_REPORT_DIR")
                    {
                        let directory = std::path::Path::new(&directory);
                        std::fs::create_dir_all(directory)?;
                        std::fs::write(
                            directory.join(format!("{mode:?}-scroll-cpu.png")),
                            cpu_frame.encode_png(),
                        )?;
                        std::fs::write(
                            directory.join(format!("{mode:?}-scroll-gpu.png")),
                            gpu_frame.encode_png(),
                        )?;
                    }
                }
                println!(
                    "Text mode {mode:?} scrolling: commands {commands}, scene mask bytes {mask_bytes}; input+prepare median/p95 {} / {} us, CPU render {} / {} us, GPU offscreen+readback {} / {} us",
                    percentile(&mut preparation.clone(), 50),
                    percentile(&mut preparation, 95),
                    percentile(&mut cpu_render.clone(), 50),
                    percentile(&mut cpu_render, 95),
                    percentile(&mut gpu_render.clone(), 50),
                    percentile(&mut gpu_render, 95),
                );
            }
            Ok(())
        })();
        self.error = result.err();
        event_loop.exit();
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

/// Native diagnostic for moving text. GPU timings include offscreen readback.
pub(super) fn text_rendering_probe() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = native_test_event_loop()?;
    let mut check = TextModeProbe { error: None };
    event_loop.run_app(&mut check)?;
    match check.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
