//! Measures what strokes cost the CPU renderer.
//!
//! Run in release: `cargo run --release -p pert-ui-cpu --example stroke_cost`.
//! Each workload strokes polylines over an 800×400 plot at scales 1 and 2,
//! with new points on every frame, and reports the median render time of
//! the frame less that of the same frame without its strokes. Two further
//! rows render one scene that does not change, a trace and a grid of
//! ellipse outlines, again and again with one renderer.
use pert_ui_cpu::CpuRenderer;
use pert_ui_runtime::{
    Color, Component, DisplayList, DrawError, Element, Length, LineJoin, Painter, Point, Rect,
    Runtime, RuntimeError, StrokeStyle, UpdateContext, Viewport,
};
use std::time::{Duration, Instant};

const WIDTH: f32 = 800.0;
const HEIGHT: f32 = 400.0;

/// The polylines one frame strokes over the plot's background.
struct Plot(Vec<Vec<Point>>);
impl Component for Plot {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT)?,
            Color::from_linear(0.02, 0.02, 0.03, 1.0)?,
        );
        let style = StrokeStyle::new(Length::new(1.5)?).with_join(LineJoin::Round);
        let color = Color::from_linear(0.3, 0.6, 1.0, 1.0)?;
        for line in &self.0 {
            p.stroke_polyline(line, style, color)?;
        }
        Ok(())
    }
}

/// Sixty ellipse outlines over the plot's background.
struct Rings;
impl Component for Rings {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.fill_rect(
            Rect::new(0.0, 0.0, WIDTH, HEIGHT)?,
            Color::from_linear(0.02, 0.02, 0.03, 1.0)?,
        );
        let style = StrokeStyle::new(Length::new(1.5)?);
        let color = Color::from_linear(0.3, 0.6, 1.0, 1.0)?;
        for index in 0..60 {
            let (column, row) = ((index % 10) as f32, (index / 10) as f32);
            p.stroke_ellipse(
                Rect::new(12.3 + 78.0 * column, 9.6 + 64.0 * row, 60.0, 44.0)?,
                style,
                color,
            )?;
        }
        Ok(())
    }
}

fn rings(scale: f64) -> Result<DisplayList, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(WIDTH, HEIGHT, scale)?)?;
    ui.mount_tree(Element::new("rings", Rings))?;
    ui.flush(Duration::ZERO)?;
    Ok(ui.paint()?)
}

/// A value in `0..1` that differs per frame and index.
fn noise(frame: u64, index: usize) -> f32 {
    let mut state = (frame << 32 | index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    state ^= state >> 29;
    state = state.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    ((state >> 40) as f32) / (1u64 << 24) as f32
}

/// A smooth trace with a little noise, as a meter or a scope draws.
fn trace(frame: u64, points: usize) -> Result<Vec<Vec<Point>>, RuntimeError> {
    let line = (0..points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            let wave = (t * 40.0 + frame as f32 * 0.1).sin();
            Point::new(
                t * WIDTH,
                HEIGHT / 2.0 + 120.0 * wave + 12.0 * noise(frame, i),
            )
        })
        .collect::<Result<_, _>>()?;
    Ok(vec![line])
}

/// Noise over the plot's whole height.
fn full_noise(frame: u64, points: usize) -> Result<Vec<Vec<Point>>, RuntimeError> {
    let line = (0..points)
        .map(|i| {
            let t = i as f32 / (points - 1) as f32;
            Point::new(t * WIDTH, 8.0 + (HEIGHT - 16.0) * noise(frame, i))
        })
        .collect::<Result<_, _>>()?;
    Ok(vec![line])
}

/// Sixty long edges, as a diagram draws.
fn edges(frame: u64) -> Result<Vec<Vec<Point>>, RuntimeError> {
    (0..60)
        .map(|i| {
            Ok(vec![
                Point::new(
                    WIDTH * noise(frame, 2 * i),
                    HEIGHT * noise(frame, 2 * i + 1),
                )?,
                Point::new(
                    WIDTH * noise(frame + 1000, 2 * i),
                    HEIGHT * noise(frame + 1000, 2 * i + 1),
                )?,
            ])
        })
        .collect()
}

fn scene(lines: Vec<Vec<Point>>, scale: f64) -> Result<DisplayList, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(WIDTH, HEIGHT, scale)?)?;
    ui.mount_tree(Element::new("plot", Plot(lines)))?;
    ui.flush(Duration::ZERO)?;
    Ok(ui.paint()?)
}

fn median(mut times: Vec<Duration>) -> f64 {
    times.sort();
    times[times.len() / 2].as_secs_f64() * 1000.0
}

type Workload = (
    &'static str,
    Box<dyn Fn(u64) -> Result<Vec<Vec<Point>>, RuntimeError>>,
);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workloads: Vec<Workload> = vec![
        ("trace, 800 points", Box::new(|frame| trace(frame, 800))),
        ("trace, 4096 points", Box::new(|frame| trace(frame, 4096))),
        (
            "noise, 800 points",
            Box::new(|frame| full_noise(frame, 800)),
        ),
        (
            "noise, 16384 points",
            Box::new(|frame| full_noise(frame, 16384)),
        ),
        ("diagram, 60 edges", Box::new(edges)),
    ];
    for scale in [1.0, 2.0] {
        let mut renderer = CpuRenderer::new();
        let empty = scene(Vec::new(), scale)?;
        let without = median(
            (0..15)
                .map(|_| {
                    let start = Instant::now();
                    std::hint::black_box(renderer.render_opaque(&empty)).map(|_| start.elapsed())
                })
                .collect::<Result<_, _>>()?,
        );
        for (name, lines) in &workloads {
            let mut times = Vec::new();
            let mut refused = None;
            for frame in 0..15 {
                let scene = scene(lines(frame)?, scale)?;
                let start = Instant::now();
                match renderer.render_opaque(&scene) {
                    Ok(image) => {
                        times.push(start.elapsed());
                        std::hint::black_box(image);
                    }
                    Err(error) => {
                        refused = Some(error);
                        break;
                    }
                }
            }
            match refused {
                Some(error) => println!("scale {scale}: {name}: refused: {error}"),
                None => println!("scale {scale}: {name}: {:.2} ms", median(times) - without),
            }
        }
        // Scenes that do not change: the renderer keeps what it retains.
        for (name, scene) in [
            (
                "trace, 800 points, unchanged",
                scene(trace(0, 800)?, scale)?,
            ),
            ("ellipses, 60, unchanged", rings(scale)?),
        ] {
            renderer.render_opaque(&scene)?;
            let times = (0..15)
                .map(|_| {
                    let start = Instant::now();
                    std::hint::black_box(renderer.render_opaque(&scene)).map(|_| start.elapsed())
                })
                .collect::<Result<_, _>>()?;
            println!("scale {scale}: {name}: {:.2} ms", median(times) - without);
        }
    }
    Ok(())
}
