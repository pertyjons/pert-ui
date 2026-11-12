//! Where analytic shapes meet: a tab and its divider, a window's title bar
//! and body, a container's fill and its foreground frame, and a fill with
//! its border as one silhouette, in CPU pixels at scales 1, 1.25 and 2.
//! A capture under a magnified view closes the file.
#![cfg(feature = "cpu")]

use crate::support;
use pert_ui::cpu::CpuRenderer;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
use support::colored;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SCALES: [f64; 3] = [1.0, 1.25, 2.0];
const RED: [u8; 3] = [255, 0, 0];
const GREEN: [u8; 3] = [0, 255, 0];
const BLUE: [u8; 3] = [0, 0, 255];

fn color(rgb: [u8; 3]) -> Color {
    Color::from_srgb8(rgb[0], rgb[1], rgb[2], 255)
}

/// A runtime at `scale` whose `roles` are the given primaries in both modes,
/// so a pixel names the command that painted it.
fn runtime(scale: f64, roles: &[(ColorRole, [u8; 3])]) -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, scale)?)?;
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for (role, rgb) in roles {
            ui.theme().set_color(mode, *role, color(*rgb))?;
        }
    }
    Ok(ui)
}

/// Content that paints nothing.
struct Blank;
impl Component for Blank {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(120.0, 60.0)?)
    }
}

/// Content that fills its allocation.
struct Content;
impl Component for Content {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(120.0, 60.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::from_size(painter.size()), color(GREEN));
        Ok(())
    }
}

/// The frame's pixel at a physical position.
fn pixel(image: &FrameImage, x: u32, y: u32) -> [u8; 4] {
    let index = ((y * image.width() + x) * 4) as usize;
    <[u8; 4]>::try_from(&image.pixels()[index..index + 4]).unwrap()
}

fn is(actual: [u8; 4], rgb: [u8; 3]) -> bool {
    actual[3] == 255 && actual.iter().zip(rgb).all(|(a, b)| a.abs_diff(b) <= 1)
}

/// The primaries met going down column `x` from row `from` to row `to`,
/// consecutive repeats merged. Panics at a pixel that is none of `known`.
#[track_caller]
fn down(image: &FrameImage, x: u32, from: u32, to: u32, known: &[[u8; 3]]) -> Vec<[u8; 3]> {
    let mut met: Vec<[u8; 3]> = Vec::new();
    for y in from..to {
        let actual = pixel(image, x, y);
        let Some(rgb) = known.iter().find(|rgb| is(actual, **rgb)) else {
            panic!("pixel ({x}, {y}) is {actual:?}, a blend or a gap");
        };
        if met.last() != Some(rgb) {
            met.push(*rgb);
        }
    }
    met
}

fn physical(value: f32, scale: f64) -> f64 {
    f64::from(value) * scale
}

fn metric(ui: &Runtime, role: MetricRole) -> TestResult<f32> {
    Ok(ui.theme().metrics(Density::Standard)?.metric(role))
}

/// The one rectangle `scene` paints with `rgb`.
#[track_caller]
fn only(scene: &DisplayList, rgb: [u8; 3]) -> [f32; 4] {
    match colored(scene, color(rgb))[..] {
        [rect] => rect.values(),
        ref other => panic!("{} commands paint {rgb:?}", other.len()),
    }
}

/// The prepared form of a shape command, in the scene's own frame.
fn prepared(scene: &DisplayList, command: &DrawCommand) -> TestResult<PreparedShape> {
    let DrawCommand::DrawShape {
        geometry,
        border,
        clip,
        ..
    } = command
    else {
        return Err("not a shape command".into());
    };
    let viewport = scene.viewport();
    let frame = |extent: f32| physical(extent, viewport.scale()).round().max(1.0) as u32;
    let mapping = FrameMapping::new(frame(viewport.width()), frame(viewport.height()), viewport)?;
    PreparedShape::new(geometry, border.as_ref(), *clip, &mapping)?
        .ok_or_else(|| "the shape draws nothing".into())
}

#[test]
fn a_tab_meets_the_strip_divider_without_a_gap_or_a_rounded_foot() -> TestResult {
    for scale in SCALES {
        let mut ui = runtime(
            scale,
            &[
                (ColorRole::Surface, BLUE),
                (ColorRole::Divider, RED),
                // The resting tab: the selected one's indicator covers its join.
                (ColorRole::Tab, GREEN),
            ],
        )?;
        let tab = |key: &str| Tab::new(key, format!("View {key}"), Element::new("body", Blank));
        ui.mount_tree(Element::new("tabs", Tabs::new([tab("a"), tab("b")])))?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let image = CpuRenderer::new().render(&scene)?;
        let [x, y, w, h] = only(&scene, GREEN);
        let [_, divider_y, _, divider_h] = colored(&scene, color(RED))
            .iter()
            .map(|rect| rect.values())
            .find(|[_, top, _, _]| (*top - (y + h)).abs() < 1e-3)
            .ok_or("no divider under the tab")?;
        let mid = physical(y + h / 2.0, scale).floor() as u32;
        let below = physical(divider_y + divider_h, scale).round() as u32;
        // Two logical pixels inside the tab's right edge, clear of its title.
        let inside = physical(x + w - 2.0, scale).floor() as u32;
        assert_eq!(
            down(&image, inside, mid, below, &[GREEN, RED]),
            [GREEN, RED],
            "scale {scale}: the tab's fill ends where the divider begins"
        );
        // The foot is square: the tab's first whole column is filled down to
        // the divider, and rounded at the top.
        let first = physical(x, scale).ceil() as u32;
        assert_eq!(
            down(&image, first, mid, below, &[GREEN, RED]),
            [GREEN, RED],
            "scale {scale}: the foot is square"
        );
        let top = physical(y, scale).round() as u32;
        assert!(
            !is(pixel(&image, first, top), GREEN),
            "scale {scale}: the top corner is rounded"
        );
    }
    Ok(())
}

#[test]
fn a_window_title_bar_meets_its_body_and_frame_along_its_straight_side() -> TestResult {
    for scale in SCALES {
        let mut ui = runtime(
            scale,
            &[
                (ColorRole::Surface, BLUE),
                (ColorRole::SurfaceContainer, GREEN),
                (ColorRole::PanelBorder, RED),
                (ColorRole::Primary, RED),
            ],
        )?;
        let window = FloatingWindow::new(&ui, "one", "One", Element::new("body", Blank))
            .position(Point::new(20.0, 20.0)?)?
            .initial_size(Size::from([300, 200]))?;
        ui.mount_tree(Element::new("layer", WindowLayer::new([window])))?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let image = CpuRenderer::new().render(&scene)?;
        let [x, y, w, h] = only(&scene, BLUE);
        let [title_x, title_y, title_w, _] = only(&scene, GREEN);
        assert_eq!(
            [title_x, title_y, title_w],
            [x, y, w],
            "scale {scale}: the title bar spans the body's top"
        );
        let radius = metric(&ui, MetricRole::CornerRadius)?;
        let border = metric(&ui, MetricRole::BorderWidth)?;
        let from = physical(y + radius + 1.0, scale).ceil() as u32;
        let to = physical(y + h - radius - 1.0, scale).floor() as u32;
        let edge = physical(x, scale).round() as u32;
        assert_eq!(
            down(&image, edge, from, to, &[RED]),
            [RED],
            "scale {scale}: the frame has no notch where the title bar ends"
        );
        let thickness = (physical(border, scale).round() as u32).max(1);
        assert_eq!(
            down(&image, edge + thickness, from, to, &[GREEN, BLUE]),
            [GREEN, BLUE],
            "scale {scale}: the body begins where the title bar ends"
        );
    }
    Ok(())
}

#[test]
fn a_container_fill_and_its_foreground_frame_have_one_geometry_around_the_content() -> TestResult {
    for scale in SCALES {
        let mut ui = runtime(scale, &[(ColorRole::PanelBorder, RED)])?;
        // Odd insets put the panel's edges off the pixel grid at 1.25.
        ui.mount_tree(Element::new(
            "outer",
            Container::new(Element::new(
                "panel",
                Container::new(Element::new("body", Content))
                    .background(color(BLUE))
                    .border_role(ColorRole::PanelBorder)
                    .padding(Length::new(4.0)?),
            ))
            .padding(Length::new(7.3)?),
        ))?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let image = CpuRenderer::new().render(&scene)?;
        let position = |rgb| {
            scene
                .commands()
                .iter()
                .position(|command| support::paints_with(command, color(rgb)))
                .ok_or("the panel painted no such command")
        };
        let (fill, content, frame) = (position(BLUE)?, position(GREEN)?, position(RED)?);
        assert!(
            fill < content && content < frame,
            "the fill lies under the content and the frame over it"
        );
        let (fill, frame) = (&scene.commands()[fill], &scene.commands()[frame]);
        assert_eq!(support::extent(fill), support::extent(frame));
        // Every pixel has the same outer coverage in both commands: the two
        // follow one outline. They are still composited one after the other,
        // unlike the single command of the next test.
        let (filled, framed) = (prepared(&scene, fill)?, prepared(&scene, frame)?);
        let mut partial = 0;
        for y in 0..image.height() {
            for x in 0..image.width() {
                let outer = filled.coverage(x, y).outer();
                assert_eq!(outer, framed.coverage(x, y).outer(), "({x}, {y})");
                if outer > 0.0 && outer < 1.0 {
                    partial += 1;
                }
            }
        }
        assert!(partial > 0, "scale {scale}: the corners are rounded");
        let [x, y, _, h] = support::extent(fill).ok_or("no extent")?.values();
        let border = metric(&ui, MetricRole::BorderWidth)?;
        let thickness = (physical(border, scale).round() as u32).max(1);
        let edge = physical(x, scale).round() as u32;
        let row = physical(y + h / 2.0, scale).floor() as u32;
        assert!(
            is(pixel(&image, edge, row), RED),
            "scale {scale}: frame edge"
        );
        assert!(
            is(pixel(&image, edge + thickness, row), BLUE),
            "scale {scale}: the fill follows the frame directly"
        );
    }
    Ok(())
}

#[test]
fn a_frame_drawn_as_one_shape_shows_no_fill_outside_its_border() -> TestResult {
    for scale in SCALES {
        // A disabled button is a `Surface` fill with an `Outline` border.
        let mut ui = runtime(
            scale,
            &[
                (ColorRole::Background, BLUE),
                (ColorRole::Surface, GREEN),
                (ColorRole::Outline, RED),
            ],
        )?;
        ui.mount_tree(Element::new(
            "outer",
            Container::new(Element::new(
                "button",
                Button::new("", |_| Ok(())).disabled(),
            ))
            .padding(Length::new(7.3)?),
        ))?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let frames: Vec<_> = scene
            .commands()
            .iter()
            .filter(|command| support::paints_with(command, color(RED)))
            .collect();
        let [frame] = frames[..] else {
            return Err(format!("{} commands paint the outline", frames.len()).into());
        };
        assert_eq!(
            support::paints(frame),
            [color(GREEN), color(RED)],
            "scale {scale}: fill and border are one command"
        );
        let shape = prepared(&scene, frame)?;
        let image = CpuRenderer::new().render(&scene)?;
        let mut ramp = 0;
        for y in 0..image.height() {
            for x in 0..image.width() {
                let outer = shape.coverage(x, y).outer();
                if outer > 0.0 && outer < 1.0 {
                    ramp += 1;
                    // Two commands would leave up to a quarter of the fill
                    // here, under the border's partial coverage.
                    assert!(
                        pixel(&image, x, y)[1] <= 1,
                        "scale {scale}: fill shows at ({x}, {y}): {:?}",
                        pixel(&image, x, y)
                    );
                }
            }
        }
        assert!(ramp > 0, "scale {scale}: the corners are rounded");
        let [x, y, w, h] = support::extent(frame).ok_or("no extent")?.values();
        let center = pixel(
            &image,
            physical(x + w / 2.0, scale) as u32,
            physical(y + h / 2.0, scale) as u32,
        );
        assert!(is(center, GREEN), "scale {scale}: the fill is drawn");
    }
    Ok(())
}

#[test]
fn built_in_frames_with_a_fill_and_a_border_are_one_command() -> TestResult {
    let mut ui = runtime(
        1.0,
        &[
            (ColorRole::Surface, GREEN),
            (ColorRole::Outline, RED),
            (ColorRole::FocusRing, BLUE),
        ],
    )?;
    let field = ui.mount_tree(Element::new(
        "field",
        TextField::new(ui.signal(String::new())),
    ))?;
    ui.flush(Duration::ZERO)?;
    let framed = |ui: &mut Runtime, border: [u8; 3]| -> TestResult<Vec<Vec<Color>>> {
        Ok(ui
            .paint()?
            .commands()
            .iter()
            .filter(|command| support::paints_with(command, color(border)))
            .map(support::paints)
            .collect())
    };
    assert_eq!(framed(&mut ui, RED)?, [[color(GREEN), color(RED)]]);
    // Focused, the border is the focus ring, still with the fill.
    ui.focus(&field)?;
    ui.flush(ui.now())?;
    assert_eq!(framed(&mut ui, BLUE)?, [[color(GREEN), color(BLUE)]]);
    assert!(framed(&mut ui, RED)?.is_empty());

    // A focused button: its fill and the ring. Disabled, the outline stays
    // with the fill.
    let mut ui = runtime(
        1.0,
        &[
            (ColorRole::Button, GREEN),
            (ColorRole::Surface, GREEN),
            (ColorRole::Outline, RED),
            (ColorRole::FocusRing, BLUE),
        ],
    )?;
    let enabled = ui.signal(true);
    let button = ui.mount_tree(Element::new(
        "button",
        Button::new("", |_| Ok(())).enabled(&enabled),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&button)?;
    ui.flush(ui.now())?;
    assert_eq!(framed(&mut ui, BLUE)?, [[color(GREEN), color(BLUE)]]);
    assert_eq!(
        ui.paint()?
            .commands()
            .iter()
            .filter(|command| support::paints_with(command, color(GREEN)))
            .count(),
        1
    );
    enabled.set(false)?;
    ui.flush(ui.now())?;
    assert_eq!(framed(&mut ui, RED)?, [[color(GREEN), color(RED)]]);
    Ok(())
}

/// A rounded tile with a border, as one shape, counting its paints.
struct Tile(Rc<Cell<u32>>);
impl Component for Tile {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(20.0, 10.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.0.set(self.0.get() + 1);
        painter.draw_shape(
            Rect::from_size(painter.size()),
            Shape::rounded(Length::new(4.0)?),
            ShapeStyle::fill(color(GREEN)).with_border(Length::new(1.0)?, color(RED)),
        )
    }
}

/// The tile drawn where and as large as a view displays it, without a view.
struct Displayed {
    rect: Rect,
    factor: f32,
}
impl Component for Displayed {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.draw_shape(
            self.rect,
            Shape::rounded(Length::new(4.0 * self.factor)?),
            ShapeStyle::fill(color(GREEN)).with_border(Length::new(self.factor)?, color(RED)),
        )
    }
}

#[test]
fn a_capture_under_a_magnified_and_a_live_view_is_the_frame_cropped_to_the_shape() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.25)?)?;
    let zoom = ZoomController::new(&ui);
    let paints = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "canvas",
        ZoomView::new([(
            Point::new(3.0, 2.0)?,
            Element::new("tile", Tile(paints.clone())),
        )])
        .controller(&zoom),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.set_capture_support(true);
    // Sixteen-fold, beyond the eight-fold cap of prepared coverage, and then
    // a change inside the live-zoom interval, which reuses retained drawing.
    for (at, factor, live) in [(1000, 16.0, false), (1010, 12.0, true)] {
        let at = Duration::from_millis(at);
        let painted = paints.get();
        zoom.set(ViewTransform::new(
            ViewScale::new(factor)?,
            Point::new(5.5, 7.25)?,
        ))?;
        ui.flush(at)?;
        let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
            "canvas".into(),
            "tile".into(),
        ])));
        let whole = CpuRenderer::new().render(&ui.paint()?)?;
        assert_eq!(CpuRenderer::new().serve_captures(&mut ui), 1);
        let capture = pending.wait()?.value?;
        if live {
            assert_eq!(
                paints.get(),
                painted,
                "{factor}×: the frame and the capture reuse the retained drawing"
            );
        }
        // The region is the tile as displayed: its place and size through
        // the view, at the state of the flush the frame was painted from.
        let expected = [
            5.5 + factor * 3.0,
            7.25 + factor * 2.0,
            factor * 20.0,
            factor * 10.0,
        ];
        for (actual, expected) in capture.region.iter().zip(expected) {
            assert!(
                (actual - expected).abs() < 1e-3,
                "{factor}×: {:?}",
                capture.region
            );
        }
        assert_eq!((capture.scale, capture.clipped), (1.25, false));
        assert_eq!(capture.at, at, "{factor}×: the frame is this flush's");
        // The crop is the frame's own pixels over that region.
        let [x, y, w, h] = capture.region;
        let left = physical(x, 1.25).floor() as u32;
        let top = physical(y, 1.25).floor() as u32;
        let right = physical(x + w, 1.25).ceil() as u32;
        let bottom = physical(y + h, 1.25).ceil() as u32;
        assert_eq!(
            (capture.image.width(), capture.image.height()),
            (right - left, bottom - top),
            "{factor}×"
        );
        for row in 0..capture.image.height() {
            for column in 0..capture.image.width() {
                assert_eq!(
                    pixel(&capture.image, column, row),
                    pixel(&whole, left + column, top + row),
                    "{factor}×: ({column}, {row})"
                );
            }
        }
        // Retained or not, the shape is evaluated at its displayed size,
        // curves included: the frame equals one that draws the tile there
        // directly, which a stretched image of the smaller tile would not.
        let mut direct = Runtime::new();
        direct.set_viewport(Viewport::new(400.0, 300.0, 1.25)?)?;
        direct.mount_tree(Element::new(
            "displayed",
            Displayed {
                rect: Rect::new(expected[0], expected[1], expected[2], expected[3])?,
                factor,
            },
        ))?;
        direct.flush(Duration::ZERO)?;
        let direct = CpuRenderer::new().render(&direct.paint()?)?;
        for row in top..bottom {
            for column in left..right {
                assert_eq!(
                    pixel(&whole, column, row),
                    pixel(&direct, column, row),
                    "{factor}×: ({column}, {row})"
                );
            }
        }
        // A border of whole
        // physical pixels at the left edge, the fill inside it.
        let middle = capture.image.height() / 2;
        let border = physical(factor, 1.25).round() as u32;
        assert!(is(pixel(&capture.image, 1, middle), RED), "{factor}×");
        assert!(
            is(pixel(&capture.image, border - 1, middle), RED),
            "{factor}×"
        );
        assert!(
            is(pixel(&capture.image, border + 1, middle), GREEN),
            "{factor}×"
        );
    }
    Ok(())
}
