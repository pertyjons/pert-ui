//! Frame-cost baseline on the gallery's real pages (phase 0, item 4 of
//! plans/rendering-capabilities.md). Throwaway measurement, not shipped:
//! `run.sh` copies the gallery's sources next to this file into a scratch
//! crate whose `main` calls [`run`].
//!
//! For every gallery page at 1280×800 logical pixels, at scale 1 and 2, it
//! records the composed scene's command mix and resource bytes, the cold and
//! warm composition time, the CPU renderer's time and the wgpu renderer's
//! offscreen time (preparation, one pass and read-back; an empty scene's
//! read-back is reported for comparison). Two change scenarios follow on the
//! heaviest page: an interactive window resize (+7 logical pixels per frame)
//! and a scale change, counting the coverage masks each frame newly needs.
use pert_ui::cpu::CpuRenderer;
use pert_ui::{DisplayList, DrawCommand, PhysicalExtent, ProfileOperation, Runtime, Viewport, WidgetId};
use pert_ui_wgpu::{Notice, Renderer};
use std::collections::HashSet;
use std::error::Error;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const WINDOW: (f32, f32) = (1280.0, 800.0);
const WARM: usize = 15;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn p95(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * 0.95).round() as usize]
}

/// Command mix and resource bytes of one scene.
#[derive(Default)]
struct Stats {
    rects: usize,
    masks_drawn: usize,
    images_drawn: usize,
    vectors_drawn: usize,
    clipped: usize,
    mask_count: usize,
    mask_bytes: usize,
    image_bytes: usize,
}

fn stats(scene: &DisplayList) -> Stats {
    let mut s = Stats::default();
    for command in scene.commands() {
        match command {
            DrawCommand::FillRect { .. } => s.rects += 1,
            DrawCommand::DrawMask { clip, .. } => {
                s.masks_drawn += 1;
                s.clipped += usize::from(clip.is_some());
            }
            DrawCommand::DrawImage { clip, .. } => {
                s.images_drawn += 1;
                s.clipped += usize::from(clip.is_some());
            }
            DrawCommand::DrawVector { clip, .. } => {
                s.vectors_drawn += 1;
                s.clipped += usize::from(clip.is_some());
            }
            _ => {}
        }
    }
    s.mask_count = scene.masks().len();
    s.mask_bytes = scene.masks().iter().map(|m| m.pixels().len()).sum();
    s.image_bytes = scene
        .images()
        .iter()
        .map(|i| i.width() as usize * i.height() as usize * 4)
        .sum();
    s
}

/// Content identities of a scene's masks, through `MaskImage`'s `Hash`.
fn mask_keys(scene: &DisplayList) -> HashSet<u64> {
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    scene.masks().iter().map(|m| hasher.hash_one(m)).collect()
}

fn find(ui: &Runtime, from: &WidgetId, key: &str) -> Result<WidgetId> {
    let mut stack = vec![from.clone()];
    while let Some(id) = stack.pop() {
        if ui.key(&id)? == key {
            return Ok(id);
        }
        stack.extend(ui.children(&id)?);
    }
    Err(format!("no component keyed {key}").into())
}

fn pages(ui: &Runtime, root: &WidgetId) -> Result<Vec<(String, WidgetId)>> {
    let top = find(ui, root, "gallery-tabs")?;
    let mut out = Vec::new();
    for view in ui.children(&top)? {
        let strip = find(ui, &view, "category-pages")?;
        for page in ui.children(&strip)? {
            out.push((ui.key(&page)?.to_string(), page));
        }
    }
    Ok(out)
}

fn settle(ui: &mut Runtime, now: &mut Duration) -> Result<()> {
    for _ in 0..4 {
        *now += Duration::from_millis(100);
        ui.flush(*now)?;
    }
    Ok(())
}

fn profile_ms(ui: &Runtime, operation: ProfileOperation) -> f64 {
    ui.profile()
        .stats
        .iter()
        .filter(|row| row.operation == operation)
        .map(|row| ms(row.self_time))
        .sum()
}

struct Probe {
    renderer: Renderer,
}

impl Probe {
    fn gpu(&mut self, scene: &DisplayList) -> Result<f64> {
        let started = Instant::now();
        self.renderer.render_offscreen(scene.clone())?;
        Ok(ms(started.elapsed()))
    }

    fn page_table(&mut self, scale: f64) -> Result<(String, String)> {
        let mut out = String::new();
        let mut heaviest = (0usize, String::new());
        let extent = PhysicalExtent::new(
            (f64::from(WINDOW.0) * scale) as u32,
            (f64::from(WINDOW.1) * scale) as u32,
        );
        self.renderer.resize(extent)?;
        let empty = {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            let _ = self.gpu(&scene)?;
            median((0..WARM).map(|_| self.gpu(&scene)).collect::<Result<_>>()?)
        };
        out += &format!(
            "\n## Pages at scale {scale} ({}×{} physical)\n\nEmpty scene offscreen render + read-back: median {empty:.2} ms.\n\n",
            extent.width(),
            extent.height()
        );
        out += "| Page | Rects | Masks drawn | Images | Vectors | Clipped | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | GPU cold ms | GPU warm ms (p95) |\n";
        out += "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n";
        let mut probe_ui = Runtime::new();
        probe_ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
        let root = crate::gallery::Gallery::new()?.mount(&mut probe_ui)?;
        probe_ui.flush(Duration::ZERO)?;
        let keys: Vec<String> = pages(&probe_ui, &root)?.into_iter().map(|p| p.0).collect();
        drop(probe_ui);
        for key in keys {
            // A fresh runtime per page, so that the cold paint includes the
            // page's first coverage rasterization and text preparation.
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
            let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
            let mut now = Duration::ZERO;
            ui.flush(now)?;
            let page = pages(&ui, &root)?
                .into_iter()
                .find(|p| p.0 == key)
                .ok_or("page vanished")?
                .1;
            if !ui.reveal(&page)?.is_revealed() {
                out += &format!("| {key} | not revealed |\n");
                continue;
            }
            settle(&mut ui, &mut now)?;
            ui.set_profiling(true)?;
            ui.clear_profile()?;
            let started = Instant::now();
            let scene = ui.paint()?;
            let cold = ms(started.elapsed());
            let (paint, text, geometry) = (
                profile_ms(&ui, ProfileOperation::Paint),
                profile_ms(&ui, ProfileOperation::Text),
                profile_ms(&ui, ProfileOperation::Geometry),
            );
            ui.set_profiling(false)?;
            let warm = median(
                (0..WARM)
                    .map(|_| -> Result<f64> {
                        let started = Instant::now();
                        ui.paint()?;
                        Ok(ms(started.elapsed()))
                    })
                    .collect::<Result<_>>()?,
            );
            let mut cpu = CpuRenderer::new();
            let _ = cpu.render(&scene)?;
            let cpu_ms = median(
                (0..5)
                    .map(|_| -> Result<f64> {
                        let started = Instant::now();
                        cpu.render(&scene)?;
                        Ok(ms(started.elapsed()))
                    })
                    .collect::<Result<_>>()?,
            );
            // A new renderer per page would be fairer for the cold time but
            // costs a device; instead a different scene in between evicts
            // nothing, so "cold" is the first submission of this scene's
            // resources to the shared renderer.
            let gpu_cold = self.gpu(&scene)?;
            let warm_gpu: Vec<f64> = (0..WARM).map(|_| self.gpu(&scene)).collect::<Result<_>>()?;
            let s = stats(&scene);
            if s.mask_bytes > heaviest.0 {
                heaviest = (s.mask_bytes, key.clone());
            }
            out += &format!(
                "| {key} | {} | {} | {} | {} | {} | {} | {:.0} | {:.0} | {cold:.2} | {paint:.2}/{text:.2}/{geometry:.2} | {warm:.3} | {cpu_ms:.1} | {gpu_cold:.2} | {:.2} ({:.2}) |\n",
                s.rects,
                s.masks_drawn,
                s.images_drawn,
                s.vectors_drawn,
                s.clipped,
                s.mask_count,
                s.mask_bytes as f64 / 1024.0,
                s.image_bytes as f64 / 1024.0,
                median(warm_gpu.clone()),
                p95(warm_gpu),
            );
        }
        Ok((out, heaviest.1))
    }

    /// Composition and render cost while the window is resized or rescaled.
    fn change_scenario(&mut self, page_key: &str) -> Result<String> {
        let mut out = format!("\n## Change scenarios on page `{page_key}`\n\n");
        out += "| Scenario | Frames | Compose median ms (p95) | New masks per frame (median) | New mask KiB per frame (median) | GPU offscreen median ms (p95) |\n|---|---|---|---|---|---|\n";
        let mut breakdown = String::from("\nProfiler self time by operation (largest first):\n\n");
        for scenario in ["resize +7 px per frame", "scale 1 → 1.25 → 1.5 → 1.75 → 2"] {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, 1.0)?)?;
            let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
            let mut now = Duration::ZERO;
            ui.flush(now)?;
            let page = pages(&ui, &root)?
                .into_iter()
                .find(|p| p.0 == page_key)
                .ok_or("no such page")?
                .1;
            let _ = ui.reveal(&page)?;
            settle(&mut ui, &mut now)?;
            let mut previous = mask_keys(&ui.paint()?);
            ui.set_profiling(true)?;
            ui.clear_profile()?;
            let steps: Vec<(f32, f64)> = if scenario.starts_with("resize") {
                (1..=30).map(|i| (WINDOW.0 + 7.0 * i as f32, 1.0)).collect()
            } else {
                [1.25, 1.5, 1.75, 2.0].into_iter().map(|s| (WINDOW.0, s)).collect()
            };
            let (mut compose, mut fresh, mut fresh_bytes, mut gpu) = (vec![], vec![], vec![], vec![]);
            for (width, scale) in steps {
                let started = Instant::now();
                ui.set_viewport(Viewport::new(width, WINDOW.1, scale)?)?;
                now += Duration::from_micros(16_667);
                ui.flush(now)?;
                let scene = ui.paint()?;
                compose.push(ms(started.elapsed()));
                let keys = mask_keys(&scene);
                let hasher = BuildHasherDefault::<DefaultHasher>::default();
                let new_bytes: usize = scene
                    .masks()
                    .iter()
                    .filter(|m| !previous.contains(&hasher.hash_one(*m)))
                    .map(|m| m.pixels().len())
                    .sum();
                fresh.push(keys.difference(&previous).count() as f64);
                fresh_bytes.push(new_bytes as f64 / 1024.0);
                previous = keys;
                let extent = PhysicalExtent::new(
                    (f64::from(width) * scale) as u32,
                    (f64::from(WINDOW.1) * scale) as u32,
                );
                self.renderer.resize(extent)?;
                gpu.push(self.gpu(&scene)?);
            }
            let profile = ui.profile();
            ui.set_profiling(false)?;
            let mut by_operation: std::collections::BTreeMap<String, f64> = Default::default();
            for row in &profile.stats {
                *by_operation.entry(format!("{:?}", row.operation)).or_default() += ms(row.self_time);
            }
            let mut ranked: Vec<_> = by_operation.into_iter().collect();
            ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
            let frames = compose.len() as f64;
            let top: Vec<String> = ranked
                .iter()
                .take(6)
                .map(|(name, total)| format!("{name} {:.2}", total / frames))
                .collect();
            breakdown += &format!("- {scenario}: self ms per frame: {}\n", top.join(", "));
            out += &format!(
                "| {scenario} | {} | {:.2} ({:.2}) | {:.0} | {:.0} | {:.2} ({:.2}) |\n",
                compose.len(),
                median(compose.clone()),
                p95(compose),
                median(fresh),
                median(fresh_bytes),
                median(gpu.clone()),
                p95(gpu)
            );
        }
        Ok(out + &breakdown)
    }
}

struct Harness {
    report: Option<Result<String>>,
}

impl ApplicationHandler<Notice> for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let result = (|| -> Result<String> {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes()
                    .with_title("pert-ui frame-cost baseline")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800)),
            )?);
            let size = window.inner_size();
            let renderer = Renderer::new(
                window.clone(),
                PhysicalExtent::new(size.width, size.height),
                |notice| eprintln!("renderer notice: {notice:?}"),
            )?;
            let mut probe = Probe { renderer };
            println!("# Frame-cost baseline on gallery pages");
            let page = match std::env::var("PROBE_CHANGE_PAGE") {
                // A named page runs only the change scenarios, for profiling.
                Ok(page) => page,
                Err(_) => {
                    let (table, heaviest) = probe.page_table(1.0)?;
                    println!("{table}");
                    let (table, _) = probe.page_table(2.0)?;
                    println!("{table}");
                    heaviest
                }
            };
            println!("{}", probe.change_scenario(&page)?);
            drop(window);
            Ok(String::new())
        })();
        self.report = Some(result);
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

pub(crate) fn run() -> Result<()> {
    let event_loop = EventLoop::<Notice>::with_user_event().build()?;
    let mut harness = Harness { report: None };
    event_loop.run_app(&mut harness)?;
    let report = harness.report.ok_or("the event loop never resumed")??;
    print!("{report}");
    Ok(())
}
