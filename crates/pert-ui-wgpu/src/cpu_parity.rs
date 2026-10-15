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
use super::native_smoke::native_test_event_loop;
use super::*;
use pert_ui_runtime::{
    Column, Component, DrawError, Element, FillRule, Icon, ImageSampling, Label, Length, MaskImage,
    Painter, Path, Point, PointerEvent, PointerKind, QuarterTurn, RasterImage, ReadSignal, Rect,
    RichText, Runtime, RuntimeError, ScrollArea, Shape, TextColor, TextRenderingMode, TextStyle,
    UpdateContext, Viewport,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

/// The corpus's logical and physical size.
const SIZE: u32 = 64;

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
    ui.mount("fixture", Fixture(Box::new(paint)))?;
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
    ui.mount(
        "text",
        Fixture(Box::new(|p| {
            p.text("Il1 O0", None, TextStyle::Body)?
                .paint(p, 3.25, 7.25, Color::WHITE)
        })),
    )?;
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
    ui.mount(
        "rich",
        Fixture(Box::new(|p| {
            let rich = RichText::new().text("H").gap().tinted_icon(
                Icon::Settings,
                TextColor::Literal(Color::from_srgb8(255, 0, 0, 255)),
            );
            p.rich_text(&rich, None, TextStyle::Body)?
                .paint(p, 3.25, 7.25, Color::WHITE)
        })),
    )?;
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
    let linear = Color::rgba(0.9, 0.1, 0.05, 1.0)?;
    let translucent = Color::rgba(0.2, 0.4, 1.0, 0.45)?;
    let orange = Color::rgba(1.0, 0.5, 0.0, 0.3)?;
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
                        Shape::Rounded(Length::new(4.0)?),
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

/// The largest channel difference and how many channels differ at all.
fn difference(gpu: &pert_ui_runtime::FrameImage, cpu: &pert_ui_runtime::FrameImage) -> (u8, usize) {
    gpu.pixels()
        .iter()
        .zip(cpu.pixels())
        .fold((0, 0), |(largest, count), (a, b)| {
            let difference = a.abs_diff(*b);
            (largest.max(difference), count + usize::from(difference > 0))
        })
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
            let mut renderer = Renderer::new(window.clone(), SIZE, SIZE, |_| {})?;
            let info = renderer.adapter.get_info();
            println!("Parity adapter: {} ({:?})", info.name, info.backend);
            let mut cpu = pert_ui_cpu::CpuRenderer::new();
            let mut failures = Vec::new();
            for case in corpus()? {
                let gpu = renderer.render_offscreen(case.scene.clone())?;
                let software = cpu.render(&case.scene)?;
                let (largest, differing) = difference(&gpu, &software);
                let allowed = if case.exact { 0 } else { 1 };
                println!(
                    "Parity {}: {} tier, largest channel difference {largest}, {differing} of {} channels differ",
                    case.name,
                    if case.exact { "exact" } else { "tolerance" },
                    gpu.pixels().len()
                );
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
                    12.0 + self.current,
                    12.0 + row as f32 * 22.0,
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
            let mut gpu = Renderer::new(window, 800, 500, |_| {})?;
            let mut cpu = pert_ui_cpu::CpuRenderer::new();
            println!(
                "Text mode adapter: {} ({:?})",
                gpu.adapter.get_info().name,
                gpu.adapter.get_info().backend
            );
            for mode in [TextRenderingMode::LineMask, TextRenderingMode::PhaseLine] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(800.0, 500.0, 1.0)?)?;
                ui.ui_settings().set_text_rendering(mode)?;
                let offset = ui.signal(0.0f32);
                ui.mount(
                    "moving-text",
                    MovingText {
                        offset: offset.read_only(),
                        current: 0.0,
                    },
                )?;
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
