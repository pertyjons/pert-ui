//! Phase 1 before/after probe (the first and the last slice of
//! plans/rendering/phase1-plan.md). Throwaway measurement, not shipped:
//! `run.sh` assembles a scratch crate per target checkout from that
//! checkout's gallery and examples plus this harness and the two adapters,
//! and its `main` calls [`run`]. The harness uses only the public API, the
//! same source for every checkout, and counts draw commands it does not know
//! through a wildcard arm, so it compiles against the checkouts before and
//! after phase 1 unchanged.
//!
//! Scenarios, each reported as a Markdown table on stdout and as
//! `scenario<TAB>metric<TAB>value` lines in `PROBE_TSV`:
//!
//! 1. every gallery page at 1280×800 logical pixels, at scales 1, 1.25 and 2;
//! 2. the live mixer with one engine tick per frame;
//! 3. the diagram editor: a live zoom (steps 16 ms apart, inside the 150 ms
//!    window), its settle frame, a settled zoom series and a pan; and the
//!    gallery's zoom workload, sixty-four cards of four controls in a
//!    `ZoomView`, driven by the wheel through the phases of its
//!    `--zoom-benchmark` (pan, pan back, zoom in to 800%, out to 100% and
//!    further out to 12.5%), each gesture followed by its rest frame;
//! 4. a knob: the gallery's `input` page (Input & parameters) with its first knob stepped
//!    through distinct values (its readouts change text too, so its mask
//!    counts are the whole display list's), and an isolated knob without a
//!    readout, written through its host to the same values;
//! 5. the J1 change scenarios (resize and scale step) on one page.
//!
//! Definitions:
//!
//! - **Compose** is wall time for the work that produces one frame: the
//!   change itself (input, a host write or a viewport change), `flush` and
//!   `paint`. For a page, *cold* is the first paint of a fresh runtime and
//!   *warm* the median of repeated paints with nothing changed. For a frame
//!   series, *first* is its first frame and the median and p95 cover all.
//! - **CPU** is `CpuRenderer::render` of the composed scene; a page reports
//!   the median of repeated renders after one warm-up, a series one render
//!   per frame with the renderer kept across frames.
//! - **Present** is UI-thread wall time of `Renderer::render` to the probe's
//!   window: acquisition, preparation and upload of what changed, encoding,
//!   submission and presentation, without read-back. Presents are paced at
//!   least 30 ms apart so acquisition does not wait for vsync; the pause is
//!   not timed. It is not GPU execution time. A page reports its first
//!   present (*cold*) and the median of repeats; a series presents every
//!   frame once. A frame the surface did not present is left out.
//! - **GPU** is `Renderer::render_offscreen` after that present, so the
//!   scene is already prepared: one pass and the read-back, which dominates.
//!   It is not GPU execution time either. `PROBE_GPU=0` leaves both out and
//!   needs no window.
//! - **Masks** are counted in the composed display list: its mask
//!   resources and their bytes, and per frame the masks whose content the
//!   previous frame did not have. Neither the renderers' own rasterization
//!   nor their uploads are measured.
//!
//! Environment: `PROBE_TSV` (the TSV path), `PROBE_GPU=0`, `PROBE_QUICK=1`
//! (fewer repetitions, scale 1 and three pages, for a smoke run only),
//! `PROBE_J1_PAGE` (the J1 page key; by default the page with the most mask
//! bytes at scale 1, printed as `j1-page: <key>`), `PROBE_CAPTURES` (a
//! directory for CPU captures of every page at every scale) and `PROBE_TMP`
//! (a directory for the diagram editor's files).
use pert_ui::cpu::CpuRenderer;
use pert_ui::parameter::*;
use pert_ui::{
    Column, DisplayList, DrawCommand, Element, InspectValue, KeyAction, KeyCode, Modifiers, PhysicalExtent, Point,
    PointerEvent, PointerKind, ProfileOperation, Row, Runtime, Viewport, WidgetId,
};
use pert_ui_wgpu::{Frame, Notice, Renderer};
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Write as _;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const GALLERY: (f32, f32) = (1280.0, 800.0);
const MIXER: (f32, f32) = (900.0, 520.0);
const DIAGRAM: (f32, f32) = (1200.0, 800.0);
const KNOB: (f32, f32) = (320.0, 240.0);
const CARDS: (f32, f32) = (960.0, 640.0);
const FRAME: Duration = Duration::from_micros(16_667);

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn median(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn p95(v: &[f64]) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * 0.95).round() as usize]
}

fn timed<T>(f: impl FnOnce() -> Result<T>) -> Result<(T, f64)> {
    let started = Instant::now();
    let value = f()?;
    Ok((value, ms(started.elapsed())))
}

/// How many repetitions and which scales the run uses.
struct Sizes {
    scales: &'static [f64],
    pages: Option<usize>,
    warm: usize,
    cpu: usize,
    ticks: usize,
    knob_values: usize,
    live_zoom: usize,
    settled_zoom: usize,
    pan: usize,
    cards: usize,
    resize: usize,
}

impl Sizes {
    fn new(quick: bool) -> Self {
        if quick {
            Self {
                scales: &[1.0],
                pages: Some(3),
                warm: 3,
                cpu: 2,
                ticks: 20,
                knob_values: 20,
                live_zoom: 6,
                settled_zoom: 3,
                pan: 10,
                cards: 6,
                resize: 5,
            }
        } else {
            Self {
                scales: &[1.0, 1.25, 2.0],
                pages: None,
                warm: 15,
                cpu: 5,
                ticks: 300,
                knob_values: 200,
                live_zoom: 30,
                settled_zoom: 8,
                pan: 60,
                cards: 60,
                resize: 30,
            }
        }
    }
}

/// Command mix and resource bytes of one scene.
#[derive(Default)]
struct Stats {
    rects: usize,
    masks_drawn: usize,
    images_drawn: usize,
    vectors_drawn: usize,
    /// Commands this harness does not know, such as phase 1's shapes.
    other: usize,
    mask_count: usize,
    mask_bytes: usize,
    image_bytes: usize,
}

fn stats(scene: &DisplayList) -> Stats {
    let mut s = Stats::default();
    for command in scene.commands() {
        #[allow(unreachable_patterns)]
        match command {
            DrawCommand::FillRect { .. } => s.rects += 1,
            DrawCommand::DrawMask { .. } => s.masks_drawn += 1,
            DrawCommand::DrawImage { .. } => s.images_drawn += 1,
            DrawCommand::DrawVector { .. } => s.vectors_drawn += 1,
            _ => s.other += 1,
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

/// Masks and KiB in `scene` that `previous` did not have.
fn fresh(scene: &DisplayList, previous: &HashSet<u64>) -> (usize, f64) {
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    let mut seen = HashSet::new();
    let mut bytes = 0;
    for mask in scene.masks() {
        let key = hasher.hash_one(mask);
        if !previous.contains(&key) && seen.insert(key) {
            bytes += mask.pixels().len();
        }
    }
    (seen.len(), bytes as f64 / 1024.0)
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
    // An empty f64 sum is -0.0; adding 0.0 prints it as 0.00.
    ui.profile()
        .stats
        .iter()
        .filter(|row| row.operation == operation)
        .map(|row| ms(row.self_time))
        .sum::<f64>()
        + 0.0
}

fn extent(size: (f32, f32), scale: f64) -> PhysicalExtent {
    PhysicalExtent::new(
        (f64::from(size.0) * scale) as u32,
        (f64::from(size.1) * scale) as u32,
    )
}

/// A gallery runtime showing `key`, settled, with profiling off.
fn gallery_page(scale: f64, key: &str) -> Result<(Runtime, WidgetId, Duration)> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(GALLERY.0, GALLERY.1, scale)?)?;
    let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
    let mut now = Duration::ZERO;
    ui.flush(now)?;
    let page = pages(&ui, &root)?
        .into_iter()
        .find(|p| p.0 == key)
        .ok_or_else(|| format!("no page {key}"))?
        .1;
    if !ui.reveal(&page)?.is_revealed() {
        return Err(format!("page {key} not revealed").into());
    }
    settle(&mut ui, &mut now)?;
    ui.set_profiling(false)?;
    Ok((ui, page, now))
}

/// Per-frame measurements of one scenario.
struct Series {
    name: String,
    compose: Vec<f64>,
    cpu: Vec<f64>,
    present: Vec<f64>,
    gpu: Vec<f64>,
    masks: Vec<f64>,
    mask_kib: Vec<f64>,
    new_masks: Vec<f64>,
    new_kib: Vec<f64>,
    other: usize,
    renderer: CpuRenderer,
    previous: HashSet<u64>,
}

impl Series {
    /// Starts a series whose first frame is compared with `initial`.
    fn new(name: impl Into<String>, initial: &DisplayList) -> Self {
        Self {
            name: name.into(),
            compose: vec![],
            cpu: vec![],
            present: vec![],
            gpu: vec![],
            masks: vec![],
            mask_kib: vec![],
            new_masks: vec![],
            new_kib: vec![],
            other: 0,
            renderer: CpuRenderer::new(),
            previous: mask_keys(initial),
        }
    }

    fn frame(&mut self, gpu: &mut Gpu, compose: f64, scene: &DisplayList) -> Result<()> {
        self.compose.push(compose);
        let s = stats(scene);
        self.masks.push(s.mask_count as f64);
        self.mask_kib.push(s.mask_bytes as f64 / 1024.0);
        self.other = self.other.max(s.other);
        let (count, kib) = fresh(scene, &self.previous);
        self.new_masks.push(count as f64);
        self.new_kib.push(kib);
        self.previous = mask_keys(scene);
        let ((), cpu) = timed(|| {
            self.renderer.render(scene)?;
            Ok(())
        })?;
        self.cpu.push(cpu);
        self.present.extend(gpu.present(scene)?);
        if let Some(time) = gpu.time(scene)? {
            self.gpu.push(time);
        }
        Ok(())
    }

    fn new_total(&self) -> f64 {
        self.new_masks.iter().sum()
    }

    fn report(&self, out: &mut Report) {
        let first = self.compose.first().copied().unwrap_or(f64::NAN);
        let _ = writeln!(
            out.md,
            "| {} | {} | {first:.2} | {:.2} ({:.2}) | {:.2} | {} | {} | {:.0} | {:.0} | {:.0} ({:.0}) | {:.0} | {} |",
            self.name,
            self.compose.len(),
            median(&self.compose),
            p95(&self.compose),
            median(&self.cpu),
            if self.present.is_empty() {
                "–".to_owned()
            } else {
                format!("{:.2} ({:.2})", median(&self.present), p95(&self.present))
            },
            if self.gpu.is_empty() {
                "–".to_owned()
            } else {
                format!("{:.2} ({:.2})", median(&self.gpu), p95(&self.gpu))
            },
            median(&self.masks),
            median(&self.mask_kib),
            median(&self.new_masks),
            self.new_total(),
            self.new_kib.iter().sum::<f64>(),
            self.other,
        );
        let n = &self.name;
        out.row(n, "frames", self.compose.len() as f64);
        out.row(n, "compose_first_ms", first);
        out.row(n, "compose_median_ms", median(&self.compose));
        out.row(n, "compose_p95_ms", p95(&self.compose));
        out.row(n, "cpu_median_ms", median(&self.cpu));
        if !self.present.is_empty() {
            out.row(n, "present_frames", self.present.len() as f64);
            out.row(n, "present_median_ms", median(&self.present));
            out.row(n, "present_p95_ms", p95(&self.present));
        }
        if !self.gpu.is_empty() {
            out.row(n, "gpu_median_ms", median(&self.gpu));
            out.row(n, "gpu_p95_ms", p95(&self.gpu));
        }
        out.row(n, "mask_resources_median", median(&self.masks));
        out.row(n, "mask_kib_median", median(&self.mask_kib));
        out.row(n, "new_masks_median", median(&self.new_masks));
        out.row(n, "new_masks_total", self.new_total());
        out.row(n, "new_mask_kib_total", self.new_kib.iter().sum());
        out.row(n, "other_commands_max", self.other as f64);
    }
}

const SERIES_HEADER: &str = "| Scenario | Frames | Compose first ms | Compose median ms (p95) | CPU median ms | Present median ms (p95) | GPU median ms (p95) | Mask resources (median) | Mask KiB (median) | New masks per frame, median (total) | New mask KiB total | Other commands (max) |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n";

/// The Markdown report and the TSV rows.
#[derive(Default)]
struct Report {
    md: String,
    tsv: String,
}

impl Report {
    fn row(&mut self, scenario: &str, metric: &str, value: f64) {
        let _ = writeln!(self.tsv, "{scenario}\t{metric}\t{value}");
    }
}

/// The offscreen wgpu renderer, absent with `PROBE_GPU=0`.
struct Gpu(Option<Renderer>, Option<Instant>);

impl Gpu {
    fn resize(&mut self, extent: PhysicalExtent) -> Result<()> {
        if let Some(renderer) = &mut self.0 {
            renderer.resize(extent)?;
        }
        Ok(())
    }

    fn time(&mut self, scene: &DisplayList) -> Result<Option<f64>> {
        let Some(renderer) = &mut self.0 else {
            return Ok(None);
        };
        let scene = scene.clone();
        let started = Instant::now();
        renderer.render_offscreen(scene)?;
        Ok(Some(ms(started.elapsed())))
    }

    /// Presents `scene` to the window, at least 30 ms after the last
    /// present, and returns the `render` call's time when it presented.
    fn present(&mut self, scene: &DisplayList) -> Result<Option<f64>> {
        let Some(renderer) = &mut self.0 else {
            return Ok(None);
        };
        const PACE: Duration = Duration::from_millis(30);
        if let Some(last) = self.1 {
            std::thread::sleep(PACE.saturating_sub(last.elapsed()));
        }
        let scene = scene.clone();
        let started = Instant::now();
        let frame = renderer.render(scene, || {})?;
        let time = ms(started.elapsed());
        self.1 = Some(Instant::now());
        Ok(matches!(frame, Frame::Presented).then_some(time))
    }
}

struct Probe {
    gpu: Gpu,
    sizes: Sizes,
    captures: Option<PathBuf>,
    tmp: PathBuf,
    out: Report,
}

impl Probe {
    /// Scenario 1: every gallery page at `scale`. Returns the page with the
    /// most mask bytes.
    fn page_table(&mut self, scale: f64) -> Result<String> {
        let extent = extent(GALLERY, scale);
        self.gpu.resize(extent)?;
        let empty = {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(GALLERY.0, GALLERY.1, scale)?)?;
            ui.flush(Duration::ZERO)?;
            let scene = ui.paint()?;
            let _ = self.gpu.time(&scene)?;
            let mut times = vec![];
            for _ in 0..self.sizes.warm {
                times.extend(self.gpu.time(&scene)?);
            }
            median(&times)
        };
        let _ = write!(
            self.out.md,
            "\n## Gallery pages at scale {scale} ({}×{} physical)\n\nEmpty scene offscreen render + read-back: median {empty:.2} ms.\n\n",
            extent.width(),
            extent.height()
        );
        self.out.md += "| Page | Rects | Masks drawn | Images | Vectors | Other | Mask resources | Mask KiB | Image KiB | Compose cold ms | (paint/text/geometry self ms) | Compose warm ms | CPU render ms | Present cold ms | Present warm ms | GPU first ms | GPU warm ms (p95) |\n";
        self.out.md += "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n";
        let keys: Vec<String> = {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(GALLERY.0, GALLERY.1, scale)?)?;
            let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
            ui.flush(Duration::ZERO)?;
            let mut keys: Vec<String> = pages(&ui, &root)?.into_iter().map(|p| p.0).collect();
            if let Some(limit) = self.sizes.pages {
                keys.truncate(limit);
            }
            keys
        };
        let mut heaviest = (0usize, String::new());
        for key in keys {
            // A fresh runtime per page, so that the cold paint includes the
            // page's first coverage rasterization and text preparation.
            let (mut ui, _page, _now) = match gallery_page(scale, &key) {
                Ok(page) => page,
                Err(error) => {
                    let _ = writeln!(self.out.md, "| {key} | {error} |");
                    continue;
                }
            };
            ui.set_profiling(true)?;
            ui.clear_profile()?;
            let (scene, cold) = timed(|| Ok(ui.paint()?))?;
            let (paint, text, geometry) = (
                profile_ms(&ui, ProfileOperation::Paint),
                profile_ms(&ui, ProfileOperation::Text),
                profile_ms(&ui, ProfileOperation::Geometry),
            );
            ui.set_profiling(false)?;
            let mut warm = vec![];
            for _ in 0..self.sizes.warm {
                warm.push(timed(|| Ok(ui.paint()?))?.1);
            }
            let mut cpu = CpuRenderer::new();
            let image = cpu.render(&scene)?;
            let mut cpu_times = vec![];
            for _ in 0..self.sizes.cpu {
                cpu_times.push(timed(|| Ok(cpu.render(&scene)?))?.1);
            }
            let present_cold = self.gpu.present(&scene)?;
            let mut present_warm = vec![];
            for _ in 0..self.sizes.cpu {
                present_warm.extend(self.gpu.present(&scene)?);
            }
            let gpu_cold = self.gpu.time(&scene)?;
            let mut gpu_warm = vec![];
            for _ in 0..self.sizes.warm {
                gpu_warm.extend(self.gpu.time(&scene)?);
            }
            if let Some(dir) = &self.captures {
                std::fs::create_dir_all(dir)?;
                std::fs::write(dir.join(format!("{key}@{scale}.png")), image.encode_png())?;
            }
            let s = stats(&scene);
            if s.mask_bytes > heaviest.0 {
                heaviest = (s.mask_bytes, key.clone());
            }
            let (warm_median, cpu_median) = (median(&warm), median(&cpu_times));
            let gpu_cell = match gpu_cold {
                Some(cold) => format!(
                    "{cold:.2} | {:.2} ({:.2})",
                    median(&gpu_warm),
                    p95(&gpu_warm)
                ),
                None => "– | –".to_owned(),
            };
            let present_cell = match present_cold {
                Some(cold) => format!("{cold:.2} | {:.2}", median(&present_warm)),
                None => "– | –".to_owned(),
            };
            let _ = writeln!(
                self.out.md,
                "| {key} | {} | {} | {} | {} | {} | {} | {:.0} | {:.0} | {cold:.2} | {paint:.2}/{text:.2}/{geometry:.2} | {warm_median:.3} | {cpu_median:.1} | {present_cell} | {gpu_cell} |",
                s.rects,
                s.masks_drawn,
                s.images_drawn,
                s.vectors_drawn,
                s.other,
                s.mask_count,
                s.mask_bytes as f64 / 1024.0,
                s.image_bytes as f64 / 1024.0,
            );
            let n = format!("page/{key}/{scale}");
            self.out.row(&n, "rects", s.rects as f64);
            self.out.row(&n, "masks_drawn", s.masks_drawn as f64);
            self.out.row(&n, "other_commands", s.other as f64);
            self.out.row(&n, "mask_resources", s.mask_count as f64);
            self.out.row(&n, "mask_kib", s.mask_bytes as f64 / 1024.0);
            self.out.row(&n, "compose_cold_ms", cold);
            self.out.row(&n, "compose_warm_ms", warm_median);
            self.out.row(&n, "cpu_median_ms", cpu_median);
            if let Some(cold) = present_cold {
                self.out.row(&n, "present_cold_ms", cold);
                self.out.row(&n, "present_warm_median_ms", median(&present_warm));
            }
            if let Some(cold) = gpu_cold {
                self.out.row(&n, "gpu_first_ms", cold);
                self.out.row(&n, "gpu_warm_median_ms", median(&gpu_warm));
                self.out.row(&n, "gpu_warm_p95_ms", p95(&gpu_warm));
            }
        }
        Ok(heaviest.1)
    }

    /// Scenario 2: the mixer, one engine tick per frame.
    fn mixer(&mut self) -> Result<Series> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(MIXER.0, MIXER.1, 1.0)?)?;
        self.gpu.resize(extent(MIXER, 1.0))?;
        let mixer = crate::mixer::probe_build(&mut ui)?;
        let _ = &mixer.root;
        let mut now = Duration::ZERO;
        settle(&mut ui, &mut now)?;
        let initial = ui.paint()?;
        let mut series = Series::new("mixer: one tick per frame", &initial);
        let mut extra_flushes = 0;
        for _ in 0..self.sizes.ticks {
            let before = crate::mixer::ProbeMixer::levels(&ui);
            mixer.tick()?;
            // Let the engine publish every channel before the first flush,
            // so one flush usually delivers the whole tick.
            std::thread::sleep(Duration::from_millis(2));
            now += FRAME;
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut compose = 0.0;
            let mut flushes = 0;
            loop {
                compose += timed(|| Ok(ui.flush(now)?))?.1;
                flushes += 1;
                let levels = crate::mixer::ProbeMixer::levels(&ui);
                if levels.iter().zip(&before).all(|(a, b)| a != b) {
                    break;
                }
                if Instant::now() > deadline {
                    return Err("the mixer's meters did not show the tick in time".into());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            extra_flushes += flushes - 1;
            let (scene, paint) = timed(|| Ok(ui.paint()?))?;
            series.frame(&mut self.gpu, compose + paint, &scene)?;
        }
        let _ = writeln!(
            self.out.md,
            "\nMixer: {extra_flushes} extra flushes over {} ticks while waiting for the engine; their time is in compose.",
            self.sizes.ticks
        );
        Ok(series)
    }

    /// Scenario 3: the diagram editor's zoom and pan, each on a fresh editor.
    fn diagram(&mut self) -> Result<Vec<Series>> {
        let mut out = vec![];
        for scenario in ["live zoom", "settled zoom", "pan"] {
            let dir = self.tmp.join(format!(
                "diagram-{}-{}",
                std::process::id(),
                scenario.replace(' ', "-")
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)?;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(DIAGRAM.0, DIAGRAM.1, 1.0)?)?;
            self.gpu.resize(extent(DIAGRAM, 1.0))?;
            let diagram = crate::diagram::probe_build(&mut ui, dir.clone())?;
            let mut now = Duration::ZERO;
            settle(&mut ui, &mut now)?;
            let [_, _, w, h] = ui
                .bounds(&diagram.canvas())?
                .ok_or("the canvas is not laid out")?
                .values();
            let center = Point::new(w / 2.0, h / 2.0)?;
            let initial = ui.paint()?;
            match scenario {
                "live zoom" => {
                    let mut steps = Series::new("diagram: live zoom steps", &initial);
                    for _ in 0..self.sizes.live_zoom {
                        now += FRAME;
                        let (scene, t) = timed(|| {
                            diagram.zoom_by(1.06, center)?;
                            ui.flush(now)?;
                            Ok(ui.paint()?)
                        })?;
                        steps.frame(&mut self.gpu, t, &scene)?;
                    }
                    let last = ui.paint()?;
                    let mut rest = Series::new("diagram: live zoom settle frame", &last);
                    // The scale has rested past the 150 ms window.
                    now += Duration::from_millis(160);
                    let (scene, t) = timed(|| {
                        ui.flush(now)?;
                        Ok(ui.paint()?)
                    })?;
                    rest.frame(&mut self.gpu, t, &scene)?;
                    let _ = writeln!(
                        self.out.md,
                        "\nDiagram live zoom ends at scale {:.3}.",
                        diagram.scale()?
                    );
                    out.push(steps);
                    out.push(rest);
                }
                "settled zoom" => {
                    let mut series = Series::new("diagram: settled zoom in and out", &initial);
                    let n = self.sizes.settled_zoom;
                    for factor in std::iter::repeat_n(1.25f32, n).chain(std::iter::repeat_n(0.8, n)) {
                        // Each step comes after the previous one has settled.
                        now += Duration::from_millis(200);
                        let (scene, t) = timed(|| {
                            diagram.zoom_by(factor, center)?;
                            ui.flush(now)?;
                            Ok(ui.paint()?)
                        })?;
                        series.frame(&mut self.gpu, t, &scene)?;
                    }
                    out.push(series);
                }
                _ => {
                    let mut series = Series::new("diagram: pan 7 px per frame", &initial);
                    for _ in 0..self.sizes.pan {
                        now += FRAME;
                        let (scene, t) = timed(|| {
                            diagram.pan_by(7.0, 3.0)?;
                            ui.flush(now)?;
                            Ok(ui.paint()?)
                        })?;
                        series.frame(&mut self.gpu, t, &scene)?;
                    }
                    out.push(series);
                }
            }
            drop(diagram);
            drop(ui);
            let _ = std::fs::remove_dir_all(&dir);
        }
        Ok(out)
    }

    /// Scenario 3, continued: the gallery's zoom workload, driven as its
    /// `--zoom-benchmark` drives it: one wheel step of ten logical pixels per
    /// frame at the window's center, with the primary modifier held to zoom,
    /// and after each gesture the frame that prepares the content at rest.
    fn cards(&mut self) -> Result<Vec<Series>> {
        const STEP: f32 = 10.0;
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(CARDS.0, CARDS.1, 1.0)?)?;
        self.gpu.resize(extent(CARDS, 1.0))?;
        let canvas = crate::pages::layout::probe_zoom_canvas(&ui)?;
        ui.mount_tree(canvas)?;
        let mut now = Duration::ZERO;
        settle(&mut ui, &mut now)?;
        let primary = KeyCode::primary();
        let center = Point::new(CARDS.0 / 2.0, CARDS.1 / 2.0)?;
        let mut out = vec![];
        for (name, zoom, delta) in [
            ("pan", false, Point::new(STEP, STEP)?),
            ("pan back", false, Point::new(-STEP, -STEP)?),
            ("zoom in", true, Point::new(0.0, -STEP)?),
            ("zoom out", true, Point::new(0.0, STEP)?),
            ("zoom out further", true, Point::new(0.0, STEP)?),
        ] {
            let initial = ui.paint()?;
            let mut steps = Series::new(format!("cards: {name} steps"), &initial);
            if zoom {
                ui.key_input(primary, KeyAction::Press)?;
            }
            for _ in 0..self.sizes.cards {
                now += FRAME;
                let (scene, t) = timed(|| {
                    ui.flush(now)?;
                    ui.pointer_input(PointerEvent::new(center, PointerKind::Scroll(delta)))?;
                    ui.flush(now)?;
                    Ok(ui.paint()?)
                })?;
                steps.frame(&mut self.gpu, t, &scene)?;
            }
            if zoom {
                ui.key_input(primary, KeyAction::Release)?;
            }
            let last = ui.paint()?;
            let mut rest = Series::new(format!("cards: {name} rest frame"), &last);
            // The gesture rests; this flush prepares whatever it stretched.
            now += Duration::from_millis(500);
            let (scene, t) = timed(|| {
                ui.flush(now)?;
                Ok(ui.paint()?)
            })?;
            rest.frame(&mut self.gpu, t, &scene)?;
            let scale = ui
                .inspect()
                .components
                .iter()
                .find(|item| item.key == "canvas")
                .and_then(|item| match item.details.properties.get("view_scale") {
                    Some(InspectValue::Number(value)) => Some(*value),
                    _ => None,
                })
                .ok_or("the canvas reports no view scale")?;
            let _ = writeln!(self.out.md, "\nCards, {name}: ends at {:.1}%.", scale * 100.0);
            self.out.row(&format!("cards: {name} steps"), "end_scale", scale);
            out.push(steps);
            out.push(rest);
        }
        Ok(out)
    }

    /// Scenario 4: the gallery's first knob, stepped with Shift+Up from the
    /// minimum, and an isolated knob written to the same values.
    fn knobs(&mut self) -> Result<Vec<Series>> {
        let values = self.sizes.knob_values;
        let gallery = {
            let (mut ui, page, mut now) = gallery_page(1.0, "input")?;
            self.gpu.resize(extent(GALLERY, 1.0))?;
            let knob = find(&ui, &page, "knob")?;
            ui.focus(&knob)?;
            ui.key_input(KeyCode::Home, KeyAction::Press)?;
            ui.key_input(KeyCode::Home, KeyAction::Release)?;
            settle(&mut ui, &mut now)?;
            let initial = ui.paint()?;
            let mut series = Series::new(
                "knob: gallery input page, Shift+Up (whole display list)",
                &initial,
            );
            ui.native_modifiers(Modifiers {
                shift: true,
                ..Modifiers::default()
            })?;
            for _ in 0..values {
                now += FRAME;
                let (scene, t) = timed(|| {
                    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
                    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
                    ui.flush(now)?;
                    Ok(ui.paint()?)
                })?;
                series.frame(&mut self.gpu, t, &scene)?;
            }
            ui.native_modifiers(Modifiers::default())?;
            if series.new_total() == 0.0 {
                return Err("the gallery knob's steps changed no mask: did it turn?".into());
            }
            series
        };
        let isolated = {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(KNOB.0, KNOB.1, 1.0)?)?;
            self.gpu.resize(extent(KNOB, 1.0))?;
            let (parameter, host) = ui.parameter(
                ParameterId::new(9, 1, "phase1-probe-knob")?,
                ParameterSpec::new("Probe", -100.0, 100.0, 0.0)?,
            )?;
            // In a column and a row, as a page places it, so the knob keeps
            // its natural size instead of filling the viewport.
            ui.mount_tree(Element::new(
                "column",
                Column::new([Element::new(
                    "row",
                    Row::new([Element::new("knob", Knob::new(parameter).show_value(false))]),
                )]),
            ))?;
            let mut now = Duration::ZERO;
            host.write_base(-100.0)?;
            settle(&mut ui, &mut now)?;
            let initial = ui.paint()?;
            let mut series = Series::new("knob: isolated, no readout, host writes", &initial);
            for step in 1..=values {
                now += FRAME;
                // The values Shift+Up visits: 0.001 of the range per step.
                let value = -100.0 + 0.2 * step as f32;
                let (scene, t) = timed(|| {
                    host.write_base(value)?;
                    ui.flush(now)?;
                    Ok(ui.paint()?)
                })?;
                series.frame(&mut self.gpu, t, &scene)?;
            }
            series
        };
        Ok(vec![gallery, isolated])
    }

    /// Scenario 5: the J1 resize and scale step on `page_key`.
    fn change(&mut self, page_key: &str) -> Result<Vec<Series>> {
        let mut out = vec![];
        for scenario in ["resize +7 px per frame", "scale 1 → 1.25 → 1.5 → 1.75 → 2"] {
            let (mut ui, _page, mut now) = gallery_page(1.0, page_key)?;
            let initial = ui.paint()?;
            let mut series = Series::new(format!("J1 `{page_key}`: {scenario}"), &initial);
            let steps: Vec<(f32, f64)> = if scenario.starts_with("resize") {
                (1..=self.sizes.resize)
                    .map(|i| (GALLERY.0 + 7.0 * i as f32, 1.0))
                    .collect()
            } else {
                [1.25, 1.5, 1.75, 2.0].into_iter().map(|s| (GALLERY.0, s)).collect()
            };
            for (width, scale) in steps {
                now += FRAME;
                let (scene, t) = timed(|| {
                    ui.set_viewport(Viewport::new(width, GALLERY.1, scale)?)?;
                    ui.flush(now)?;
                    Ok(ui.paint()?)
                })?;
                self.gpu.resize(extent((width, GALLERY.1), scale))?;
                series.frame(&mut self.gpu, t, &scene)?;
            }
            out.push(series);
        }
        Ok(out)
    }

    fn series_table(&mut self, title: &str, series: &[Series]) {
        let _ = write!(self.out.md, "\n## {title}\n\n{SERIES_HEADER}");
        for s in series {
            s.report(&mut self.out);
        }
    }

    fn run(&mut self) -> Result<()> {
        self.out.md += "# Phase 1 probe\n\nDefinitions are in the probe's module documentation (`plans/rendering/phase1-probe/probe.rs`).\n";
        let mut heaviest = String::new();
        for &scale in self.sizes.scales {
            let page = self.page_table(scale)?;
            if scale == 1.0 {
                heaviest = page;
            }
        }
        let mixer = self.mixer()?;
        self.series_table("Mixer with live data", &[mixer]);
        let diagram = self.diagram()?;
        self.series_table("Diagram editor zoom and pan", &diagram);
        let cards = self.cards()?;
        self.series_table("Gallery zoom workload (64 cards)", &cards);
        let knobs = self.knobs()?;
        self.series_table("Knob", &knobs);
        let j1_page = std::env::var("PROBE_J1_PAGE").unwrap_or_else(|_| heaviest.clone());
        let _ = writeln!(
            self.out.md,
            "\nj1-page: {j1_page}\nheaviest-page: {heaviest}"
        );
        let mut change = self.change(&j1_page)?;
        if !heaviest.is_empty() && heaviest != j1_page {
            // A page that became the heaviest is reported separately.
            change.extend(self.change(&heaviest)?);
        }
        self.series_table("J1 change scenarios", &change);
        Ok(())
    }
}

struct Harness {
    probe: Option<Probe>,
    result: Option<Result<()>>,
}

impl ApplicationHandler<Notice> for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let Some(mut probe) = self.probe.take() else {
            return;
        };
        let result = (|| -> Result<()> {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes()
                    .with_title("pert-ui phase 1 probe")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800)),
            )?);
            let size = window.inner_size();
            probe.gpu = Gpu(
                Some(Renderer::new(
                    window.clone(),
                    PhysicalExtent::new(size.width, size.height),
                    |notice| eprintln!("renderer notice: {notice:?}"),
                )?),
                None,
            );
            let result = probe.run();
            print!("{}", probe.out.md);
            write_tsv(&probe.out)?;
            drop(probe);
            drop(window);
            result
        })();
        self.result = Some(result);
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn write_tsv(report: &Report) -> Result<()> {
    if let Ok(path) = std::env::var("PROBE_TSV") {
        std::fs::write(path, &report.tsv)?;
    }
    Ok(())
}

pub(crate) fn run() -> Result<()> {
    let quick = std::env::var("PROBE_QUICK").as_deref() == Ok("1");
    let probe = Probe {
        gpu: Gpu(None, None),
        sizes: Sizes::new(quick),
        captures: std::env::var_os("PROBE_CAPTURES").map(PathBuf::from),
        tmp: std::env::var_os("PROBE_TMP").map_or_else(std::env::temp_dir, PathBuf::from),
        out: Report::default(),
    };
    if std::env::var("PROBE_GPU").as_deref() == Ok("0") {
        let mut probe = probe;
        let result = probe.run();
        print!("{}", probe.out.md);
        write_tsv(&probe.out)?;
        return result;
    }
    let event_loop = EventLoop::<Notice>::with_user_event().build()?;
    let mut harness = Harness {
        probe: Some(probe),
        result: None,
    };
    event_loop.run_app(&mut harness)?;
    harness.result.ok_or("the event loop never resumed")?
}
