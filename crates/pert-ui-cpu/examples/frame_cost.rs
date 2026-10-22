//! Measures CPU rendering time and frame memory at a range of window sizes.
//!
//! Run in release: `cargo run --release -p pert-ui-cpu --example frame_cost`.
//! Each size renders a scene of overlapping translucent panels, text-like
//! masks and a filtered image, and reports the median of several renders.
use pert_ui_cpu::CpuRenderer;
use pert_ui_runtime::{
    Color, Component, DrawError, ImageSampling, MaskImage, Painter, PhysicalExtent, RasterImage,
    Rect, Runtime, RuntimeError, UpdateContext, Viewport,
};
use std::time::{Duration, Instant};

/// A dense scene that scales with the window: a panel grid with masks and images.
struct Load {
    width: f32,
    height: f32,
}
impl Component for Load {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let panel = Color::from_linear(0.2, 0.25, 0.3, 0.85)?;
        let accent = Color::from_linear(0.3, 0.6, 1.0, 1.0)?;
        let mask = MaskImage::new(8, 8, (0..64).map(|i| (i * 4) as u8).collect())?;
        let image = RasterImage::from_srgba8(
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )?;
        let mut y = 0.0;
        while y < self.height {
            let mut x = 0.0;
            while x < self.width {
                p.fill_rect(Rect::new(x + 4.0, y + 4.0, 152.0, 92.0)?, panel);
                for line in 0..4 {
                    p.draw_mask(
                        &mask,
                        Rect::new(x + 12.0, y + 12.0 + line as f32 * 16.0, 120.0, 12.0)?,
                        accent,
                    );
                }
                p.draw_image(
                    &image,
                    Rect::new(x + 120.0, y + 60.0, 28.0, 28.0)?,
                    ImageSampling::Linear,
                );
                x += 160.0;
            }
            y += 100.0;
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut renderer = CpuRenderer::new();
    for (width, height) in [
        (640u32, 480u32),
        (1280, 800),
        (1920, 1080),
        (2560, 1440),
        (3840, 2160),
    ] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::from_physical(
            PhysicalExtent::new(width, height),
            1.0,
        )?)?;
        ui.mount(
            "load",
            Load {
                width: width as f32,
                height: height as f32,
            },
        )?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let mut times = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            let frame = renderer.render(&scene)?;
            times.push(start.elapsed());
            std::hint::black_box(frame);
        }
        times.sort();
        println!(
            "{width}×{height}: {} commands, median {:.1} ms, frame {:.1} MiB",
            scene.commands().len(),
            times[times.len() / 2].as_secs_f64() * 1000.0,
            f64::from(width) * f64::from(height) * 4.0 / 1048576.0
        );
    }
    Ok(())
}
