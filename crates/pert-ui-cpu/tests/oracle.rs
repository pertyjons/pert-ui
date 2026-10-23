//! The CPU renderer against hand-computed pixels: a mathematical oracle for
//! the rendering model, through scenes built only with public APIs.
use pert_ui_cpu::{CpuRenderError, CpuRenderer};
use pert_ui_runtime::{
    CaptureRequest, CaptureTarget, Color, Component, DisplayList, DrawError, Element, FillRule,
    FrameImage, ImageSampling, Length, LimitError, MaskImage, Painter, Path, Point, QuarterTurn,
    RasterImage, Rect, Runtime, RuntimeError, Shape, UpdateContext, Viewport,
};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What a test scene draws.
type Paint = Box<dyn Fn(&mut Painter) -> Result<(), DrawError>>;

/// A component that paints whatever its closure draws.
struct Scene(Paint);
impl Component for Scene {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &pert_ui_runtime::MeasureContext<'_>,
        available: pert_ui_runtime::Constraints,
    ) -> Result<pert_ui_runtime::Size, RuntimeError> {
        Ok(pert_ui_runtime::Size::new(
            available.width().unwrap_or(0.0),
            available.height().unwrap_or(0.0),
        )?)
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
    ui.mount("scene", Scene(Box::new(paint)))?;
    ui.flush(Duration::ZERO)?;
    Ok(ui.paint()?)
}

fn render(scene: &DisplayList) -> Result<FrameImage, CpuRenderError> {
    CpuRenderer::new().render(scene)
}

fn pixel(frame: &FrameImage, x: u32, y: u32) -> [u8; 4] {
    let start = ((y * frame.width() + x) * 4) as usize;
    frame.pixels()[start..start + 4].try_into().unwrap()
}

#[test]
fn thin_rounded_outline_keeps_all_sides_at_75_percent_zoom() -> TestResult {
    let frame = render(&scene(60.0, 40.0, 0.75, |p| {
        p.fill_rect(
            Rect::new(0.0, 0.0, 60.0, 40.0)?,
            Color::from_srgb8(0, 0, 0, 255),
        );
        p.stroke_shape(
            Rect::new(10.25, 10.25, 40.0, 20.0)?,
            Shape::Rounded(Length::new(4.0)?),
            Length::new(1.0)?,
            Color::WHITE,
        )
    })?)?;
    for (x, y) in [(20, 8), (20, 22), (8, 15), (37, 15)] {
        assert_eq!(
            pixel(&frame, x, y),
            [255, 255, 255, 255],
            "side at ({x}, {y})"
        );
    }
    Ok(())
}

/// The stored bytes of a linear premultiplied color, as the oracle computes them.
fn stored(linear: [f32; 4]) -> [u8; 4] {
    let encode = |value: f32| {
        Color::from_linear(value.clamp(0.0, 1.0), 0.0, 0.0, 1.0)
            .unwrap()
            .to_srgb8()[0]
    };
    [
        encode(linear[0]),
        encode(linear[1]),
        encode(linear[2]),
        (linear[3].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// The linear premultiplied value of stored bytes.
fn loaded(bytes: [u8; 4]) -> [f32; 4] {
    let decode = |byte: u8| Color::from_srgb8(byte, 0, 0, 255).red();
    [
        decode(bytes[0]),
        decode(bytes[1]),
        decode(bytes[2]),
        f32::from(bytes[3]) / 255.0,
    ]
}

/// Source-over of a straight color with `coverage` onto stored bytes.
fn over(destination: [u8; 4], color: Color, coverage: f32) -> [u8; 4] {
    let [r, g, b, a] = color.channels();
    let a = a * coverage;
    let d = loaded(destination);
    let keep = 1.0 - a;
    stored([
        r * a + d[0] * keep,
        g * a + d[1] * keep,
        b * a + d[2] * keep,
        a + d[3] * keep,
    ])
}

fn background() -> [u8; 4] {
    let [r, g, b, a] = Color::BACKGROUND.channels();
    stored([r * a, g * a, b * a, a])
}

#[test]
fn the_frame_starts_as_the_premultiplied_background() -> TestResult {
    let frame = render(&scene(3.0, 2.0, 1.0, |_| Ok(()))?)?;
    assert_eq!((frame.width(), frame.height()), (3, 2));
    for y in 0..2 {
        for x in 0..3 {
            assert_eq!(pixel(&frame, x, y), background());
        }
    }
    Ok(())
}

#[test]
fn opaque_integer_fills_are_exact_and_cover_pixel_centers() -> TestResult {
    let red = Color::from_linear(1.0, 0.0, 0.0, 1.0)?;
    let frame = render(&scene(4.0, 1.0, 1.0, move |p| {
        p.fill_rect(Rect::new(1.0, 0.0, 2.0, 1.0)?, red);
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 0, 0), background());
    assert_eq!(pixel(&frame, 1, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 2, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 3, 0), background());
    // A center on the leading edge is covered, one on the trailing edge not.
    let frame = render(&scene(3.0, 1.0, 1.0, move |p| {
        p.fill_rect(Rect::new(0.5, 0.0, 1.0, 1.0)?, red);
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 0, 0), [255, 0, 0, 255]);
    assert_eq!(pixel(&frame, 1, 0), background());
    // A rectangle that misses every center draws nothing.
    let frame = render(&scene(2.0, 1.0, 1.0, move |p| {
        p.fill_rect(Rect::new(0.6, 0.0, 0.3, 1.0)?, red);
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 0, 0), background());
    Ok(())
}

#[test]
fn logical_geometry_follows_the_physical_scale() -> TestResult {
    let white = Color::WHITE;
    let frame = render(&scene(2.0, 2.0, 2.0, move |p| {
        p.fill_rect(Rect::new(0.0, 0.0, 1.0, 1.0)?, white);
        Ok(())
    })?)?;
    assert_eq!((frame.width(), frame.height()), (4, 4));
    for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(pixel(&frame, x, y), [255; 4]);
    }
    assert_eq!(pixel(&frame, 2, 0), background());
    assert_eq!(pixel(&frame, 0, 2), background());
    Ok(())
}

#[test]
fn translucent_overlap_quantizes_after_each_command() -> TestResult {
    let blue = Color::from_linear(0.0, 0.2, 1.0, 0.5)?;
    let orange = Color::from_linear(1.0, 0.4, 0.0, 0.3)?;
    let frame = render(&scene(1.0, 1.0, 1.0, move |p| {
        let all = Rect::new(0.0, 0.0, 1.0, 1.0)?;
        p.fill_rect(all, blue);
        p.fill_rect(all, orange);
        Ok(())
    })?)?;
    let expected = over(over(background(), blue, 1.0), orange, 1.0);
    assert_eq!(pixel(&frame, 0, 0), expected);
    Ok(())
}

#[test]
fn clips_crop_fills_masks_and_images() -> TestResult {
    let white = Color::WHITE;
    let frame = render(&scene(4.0, 1.0, 1.0, move |p| {
        p.with_clip(Rect::new(1.0, 0.0, 2.0, 1.0).unwrap(), |p| {
            p.fill_rect(Rect::new(0.0, 0.0, 4.0, 1.0).unwrap(), white);
        });
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 0, 0), background());
    assert_eq!(pixel(&frame, 1, 0), [255; 4]);
    assert_eq!(pixel(&frame, 2, 0), [255; 4]);
    assert_eq!(pixel(&frame, 3, 0), background());
    Ok(())
}

#[test]
fn masks_filter_coverage_bilinearly_with_clamped_edges() -> TestResult {
    let white = Color::WHITE;
    let frame = render(&scene(4.0, 1.0, 1.0, move |p| {
        let mask = MaskImage::new(2, 1, vec![255, 0])?;
        p.draw_mask(&mask, Rect::new(0.0, 0.0, 4.0, 1.0)?, white);
        Ok(())
    })?)?;
    // Centers at u = 1/8, 3/8, 5/8, 7/8 sample texel positions -0.25, 0.25,
    // 0.75 and 1.25: clamped full coverage, then 3/4, 1/4 and clamped none.
    for (x, coverage) in [(0, 1.0), (1, 0.75), (2, 0.25), (3, 0.0)] {
        assert_eq!(
            pixel(&frame, x, 0),
            over(background(), white, coverage),
            "{x}"
        );
    }
    Ok(())
}

#[test]
fn a_quarter_turn_maps_the_source_through_its_corners() -> TestResult {
    let white = Color::WHITE;
    let frame = render(&scene(1.0, 2.0, 1.0, move |p| {
        let mask = MaskImage::new(2, 1, vec![255, 0])?;
        p.draw_mask_turned(
            &mask,
            Rect::new(0.0, 0.0, 1.0, 2.0)?,
            white,
            QuarterTurn::Right,
        );
        Ok(())
    })?)?;
    // Turned right, the upright source's left column runs down the right
    // side, so the covered texel reaches the top or the bottom.
    let top = pixel(&frame, 0, 0);
    let bottom = pixel(&frame, 0, 1);
    let full = over(background(), white, 1.0);
    let none = over(background(), white, 0.0);
    assert!(
        (top == full && bottom == none) || (top == none && bottom == full),
        "{top:?} {bottom:?}"
    );
    let source = QuarterTurn::Right.source(0.5, 0.25);
    let expected_top = if source[0] < 0.5 { full } else { none };
    assert_eq!(top, expected_top);
    Ok(())
}

#[test]
fn images_are_decoded_to_linear_light_before_filtering() -> TestResult {
    let image = RasterImage::from_srgba8(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255])?;
    let linear = image.clone();
    let frame = render(&scene(4.0, 1.0, 1.0, move |p| {
        p.draw_image(
            &linear,
            Rect::new(0.0, 0.0, 4.0, 1.0)?,
            ImageSampling::Linear,
        );
        Ok(())
    })?)?;
    // Halfway between black and white in linear light is 0.5, which encodes
    // as 188, not the 128 that filtering encoded bytes would give.
    let mix = |weight: f32| stored([weight, weight, weight, 1.0]);
    assert_eq!(pixel(&frame, 0, 0), mix(0.0));
    assert_eq!(pixel(&frame, 1, 0), mix(0.25));
    assert_eq!(pixel(&frame, 2, 0), mix(0.75));
    assert_eq!(pixel(&frame, 3, 0), mix(1.0));
    let frame = render(&scene(4.0, 1.0, 1.0, move |p| {
        p.draw_image(
            &image,
            Rect::new(0.0, 0.0, 4.0, 1.0)?,
            ImageSampling::Nearest,
        );
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 1, 0), [0, 0, 0, 255]);
    assert_eq!(pixel(&frame, 2, 0), [255; 4]);
    Ok(())
}

#[test]
fn a_tint_fades_and_colors_an_image() -> TestResult {
    let image = RasterImage::from_srgba8(1, 1, vec![255, 255, 255, 255])?;
    let tint = Color::from_linear(1.0, 0.0, 0.0, 0.5)?;
    let frame = render(&scene(1.0, 1.0, 1.0, move |p| {
        p.draw_image_tinted(
            &image,
            Rect::new(0.0, 0.0, 1.0, 1.0)?,
            ImageSampling::Nearest,
            tint,
        );
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 0, 0), over(background(), tint, 1.0));
    Ok(())
}

#[test]
fn vectors_are_prepared_into_coverage_and_drawn() -> TestResult {
    let white = Color::WHITE;
    let frame = render(&scene(20.0, 20.0, 1.0, move |p| {
        let circle = Path::circle(Point::new(10.0, 10.0)?, Length::new(6.0)?)?;
        p.fill_path(&circle, FillRule::NonZero, white)?;
        Ok(())
    })?)?;
    assert_eq!(pixel(&frame, 10, 10), [255; 4], "the center is inside");
    assert_eq!(pixel(&frame, 1, 1), background(), "a corner is outside");
    // Edge pixels carry partial coverage.
    let edge = pixel(&frame, 4, 10);
    assert!(edge != background() && edge != [255; 4], "{edge:?}");
    Ok(())
}

#[test]
fn refusals_happen_before_any_work() -> TestResult {
    let mut renderer = CpuRenderer::new();
    let empty = scene(0.0, 10.0, 1.0, |_| Ok(()))?;
    assert_eq!(renderer.render(&empty), Err(CpuRenderError::EmptyViewport));
    let huge = scene(5000.0, 5000.0, 1.0, |_| Ok(()))?;
    assert_eq!(
        renderer.render(&huge),
        Err(CpuRenderError::Limit(LimitError::FrameSize {
            width: 5000,
            height: 5000
        }))
    );
    // The renderer stays usable.
    assert!(renderer.render(&scene(1.0, 1.0, 1.0, |_| Ok(()))?).is_ok());
    Ok(())
}

/// A mounted label in a runtime the CPU renderer serves captures for.
fn captured_ui() -> Result<(Runtime, CpuRenderer), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(120.0, 60.0, 2.0)?)?;
    ui.mount_tree(Element::new(
        "scene",
        Scene(Box::new(|p| {
            p.fill_rect(Rect::new(10.0, 10.0, 40.0, 20.0)?, Color::WHITE);
            Ok(())
        })),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.set_capture_support(true);
    Ok((ui, CpuRenderer::new()))
}

#[test]
fn window_component_path_and_region_captures_are_crops_of_one_frame() -> TestResult {
    let (mut ui, mut renderer) = captured_ui()?;
    let full = renderer.render(&ui.paint()?)?;
    let window = ui.primary_window().id();
    let targets = [
        CaptureTarget::Window(window),
        CaptureTarget::Path(vec!["scene".into()]),
        CaptureTarget::Region {
            window,
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 10.0,
        },
    ];
    for target in targets {
        let pending = ui.request_capture(CaptureRequest::new(target.clone()));
        ui.flush(Duration::ZERO)?;
        assert_eq!(renderer.serve_captures(&mut ui), 1, "{target:?}");
        let capture = pending.wait()?.value?;
        let [x, y, _, _] = capture.region;
        let (left, top) = (
            (f64::from(x) * capture.scale) as u32,
            (f64::from(y) * capture.scale) as u32,
        );
        for row in 0..capture.image.height() {
            for column in 0..capture.image.width() {
                assert_eq!(
                    pixel(&capture.image, column, row),
                    pixel(&full, left + column, top + row),
                    "{target:?} at {column},{row}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_region_reaching_past_the_window_is_cropped_and_reported() -> TestResult {
    let (mut ui, mut renderer) = captured_ui()?;
    let window = ui.primary_window().id();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
        window,
        x: 100.0,
        y: 40.0,
        width: 50.0,
        height: 50.0,
    }));
    ui.flush(Duration::ZERO)?;
    renderer.serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    assert!(capture.clipped);
    assert_eq!(capture.region, [100.0, 40.0, 20.0, 20.0]);
    assert_eq!((capture.image.width(), capture.image.height()), (40, 40));
    let invalid = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
        window,
        x: 0.0,
        y: 0.0,
        width: -1.0,
        height: 5.0,
    }));
    assert_eq!(
        invalid.wait()?.value.err(),
        Some(pert_ui_runtime::CaptureError::InvalidRegion)
    );
    let outside = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
        window,
        x: 500.0,
        y: 0.0,
        width: 5.0,
        height: 5.0,
    }));
    assert_eq!(
        outside.wait()?.value.err(),
        Some(pert_ui_runtime::CaptureError::Hidden)
    );
    Ok(())
}

#[test]
fn a_reused_renderer_draws_each_scene_as_a_fresh_one_would() -> TestResult {
    // The prepared-vector cache must neither keep a replaced path nor confuse
    // two paths drawn at the same place in successive scenes.
    let circle = |radius: f32| {
        scene(40.0, 40.0, 1.0, move |p| {
            let path = Path::circle(Point::new(20.0, 20.0)?, Length::new(radius)?)?;
            p.fill_path(&path, FillRule::NonZero, Color::WHITE)?;
            Ok(())
        })
    };
    let (small, large) = (circle(8.0)?, circle(16.0)?);
    let mut reused = CpuRenderer::new();
    let first = reused.render(&small)?;
    let second = reused.render(&large)?;
    assert_ne!(first.pixels(), second.pixels());
    assert_eq!(second.pixels(), render(&large)?.pixels());
    // A refused scene in between leaves the renderer as it was.
    let huge = scene(5000.0, 5000.0, 1.0, |_| Ok(()))?;
    assert!(reused.render(&huge).is_err());
    assert_eq!(reused.render(&small)?.pixels(), first.pixels());
    Ok(())
}

#[test]
fn a_capture_shows_the_flush_it_reports() -> TestResult {
    let (mut ui, mut renderer) = captured_ui()?;
    let window = ui.primary_window().id();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Window(window)));
    ui.flush(Duration::from_millis(250))?;
    renderer.serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    assert_eq!(capture.at, Duration::from_millis(250));
    assert_eq!(capture.window, window);
    assert_eq!(capture.scale, 2.0);
    assert_eq!(
        capture.image.pixels(),
        renderer.render(&ui.paint()?)?.pixels(),
        "the capture is the frame of that flush"
    );
    // Nothing is left to serve once every request has its reply.
    assert_eq!(renderer.serve_captures(&mut ui), 0);
    Ok(())
}

#[test]
fn crops_follow_the_frame_scale_of_a_native_extent() -> TestResult {
    // At 1000 physical pixels and scale 1.5 the logical width is just above
    // 666.67, so a logical x of 500 lands just before physical column 750.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        pert_ui_runtime::PhysicalExtent::new(1000, 100),
        1.5,
    )?)?;
    ui.mount_tree(Element::new("scene", Scene(Box::new(|_| Ok(())))))?;
    ui.flush(Duration::ZERO)?;
    ui.set_capture_support(true);
    let mut renderer = CpuRenderer::new();
    let window = ui.primary_window().id();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
        window,
        x: 500.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    }));
    ui.flush(Duration::ZERO)?;
    renderer.serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    let logical = f64::from(ui.inspect().windows[0].viewport.width());
    let left = (500.0 * 1000.0 / logical).floor() as u32;
    let right = (510.0 * 1000.0 / logical).ceil() as u32;
    assert_eq!(
        capture.image.width(),
        right - left,
        "outward to the frame's own grid"
    );
    Ok(())
}

#[test]
fn a_malformed_region_is_invalid_even_in_a_hidden_window() -> TestResult {
    let (mut ui, _) = captured_ui()?;
    let window = ui.primary_window().id();
    ui.set_window_visible(false)?;
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
        window,
        x: f32::NAN,
        y: 0.0,
        width: 5.0,
        height: 5.0,
    }));
    assert_eq!(
        pending.wait()?.value.err(),
        Some(pert_ui_runtime::CaptureError::InvalidRegion)
    );
    Ok(())
}

#[test]
fn an_opaque_frame_clears_with_the_background_at_full_alpha() -> TestResult {
    // A translucent theme background: a capture keeps it premultiplied, a
    // presented frame shows its RGB opaque, as an opaque GPU surface does.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(4.0, 2.0, 1.0)?)?;
    let translucent = Color::from_linear(1.0, 1.0, 1.0, 0.5)?;
    ui.theme().set_color(
        pert_ui_runtime::ThemeMode::Dark,
        pert_ui_runtime::ColorRole::Background,
        translucent,
    )?;
    ui.theme().set_mode(pert_ui_runtime::ThemeMode::Dark)?;
    let fill = Color::from_linear(0.0, 0.0, 1.0, 0.5)?;
    ui.mount(
        "scene",
        Scene(Box::new(move |p| {
            p.fill_rect(Rect::new(0.0, 0.0, 2.0, 2.0)?, fill);
            Ok(())
        })),
    )?;
    ui.flush(Duration::ZERO)?;
    let scene = ui.paint()?;
    let mut renderer = CpuRenderer::new();
    let opaque = renderer.render_opaque(&scene)?;
    assert_eq!(pixel(&opaque, 3, 0), stored([1.0, 1.0, 1.0, 1.0]));
    assert_eq!(
        pixel(&opaque, 0, 0),
        over(stored([1.0, 1.0, 1.0, 1.0]), fill, 1.0)
    );
    let kept = renderer.render(&scene)?;
    assert_eq!(pixel(&kept, 3, 0), stored([0.5, 0.5, 0.5, 0.5]));
    Ok(())
}
