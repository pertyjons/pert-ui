//! vello_cpu / vello_gpu measurement spike (phase 0, item 6 of
//! plans/rendering-capabilities.md). Throwaway evidence, not shipped.
//!
//! Draws the same workloads through pert-ui's CPU and wgpu renderers and
//! through vello_cpu (single- and multithreaded) and vello_gpu, timing each
//! frame's phases and comparing the last frames' pixels. A window opens
//! briefly because pert-ui's wgpu renderer needs a surface; every frame is
//! rendered offscreen and read back.
mod compare;
mod pertui;
mod present;
mod translate;
mod vello;
mod workloads;

use pert_ui::cpu::CpuRenderer;
use pert_ui::{DisplayList, DrawCommand, PhysicalExtent, Runtime, Viewport};
use pert_ui_wgpu::{Notice, Renderer};
use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Samples of one phase in milliseconds.
#[derive(Default, Clone)]
struct Samples(Vec<f64>);
impl Samples {
    fn push(&mut self, d: Duration) {
        self.0.push(ms(d));
    }
    fn sorted(&self) -> Vec<f64> {
        let mut v = self.0.clone();
        v.sort_by(f64::total_cmp);
        v
    }
    fn median(&self) -> f64 {
        let v = self.sorted();
        v[v.len() / 2]
    }
    fn p95(&self) -> f64 {
        let v = self.sorted();
        v[((v.len() - 1) as f64 * 0.95).round() as usize]
    }
    fn cell(&self) -> String {
        if self.0.is_empty() {
            "–".into()
        } else {
            format!("{:.3} / {:.3}", self.median(), self.p95())
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Workload {
    name: &'static str,
    kind: pertui::Kind,
}

const WORKLOADS: [Workload; 5] = [
    Workload { name: "Panels (150 rounded + 100 plain)", kind: pertui::Kind::Panels { panels: 150, plain: 100 } },
    Workload { name: "Panels light (20 rounded + 20 plain)", kind: pertui::Kind::Panels { panels: 20, plain: 20 } },
    Workload { name: "Polyline 800 points", kind: pertui::Kind::Polyline { samples: 800 } },
    Workload { name: "Polyline 4096 points", kind: pertui::Kind::Polyline { samples: 4096 } },
    Workload { name: "Gradient bars (32)", kind: pertui::Kind::Bars },
];

/// One renderer's measurement of one workload at one scale.
struct Row {
    renderer: String,
    /// Named phases, in order; `total` is their per-frame sum.
    phases: Vec<(&'static str, Samples)>,
    total: Samples,
    /// Excluded from `total`: producing the workload's data (pert-ui: signal
    /// set + flush, which runs the component's update).
    update: Option<Samples>,
    cold_compose: Option<f64>,
    last: Vec<u8>,
}

struct Spike {
    gpu: Renderer,
    vgpu: vello::Gpu,
    warm: u64,
    frames: u64,
    threads: u16,
    out: PathBuf,
}

/// The frame data the vello renderers draw for `kind` at `frame`.
struct Data {
    items: Vec<workloads::Item>,
    points: Vec<(f32, f32)>,
    bars: Vec<(workloads::Rect, workloads::Rect)>,
}
impl Data {
    fn new(kind: pertui::Kind) -> Self {
        let items = match kind {
            pertui::Kind::Panels { panels, plain } => workloads::panels(panels, plain),
            _ => Vec::new(),
        };
        Self { items, points: Vec::new(), bars: Vec::new() }
    }
    fn advance(&mut self, kind: pertui::Kind, frame: u64) -> vello::Frame<'_> {
        match kind {
            pertui::Kind::Panels { .. } => vello::Frame::Panels(&self.items),
            pertui::Kind::Polyline { samples } => {
                self.points = workloads::polyline(frame, samples);
                vello::Frame::Polyline(&self.points)
            }
            pertui::Kind::Bars => {
                self.bars = workloads::bars(frame);
                vello::Frame::Bars(&self.bars)
            }
        }
    }
}

fn mix(scene: &DisplayList) -> String {
    let (mut rects, mut masks, mut images, mut vectors) = (0, 0, 0, 0);
    for command in scene.commands() {
        match command {
            DrawCommand::FillRect { .. } => rects += 1,
            DrawCommand::DrawMask { .. } => masks += 1,
            DrawCommand::DrawImage { .. } => images += 1,
            DrawCommand::DrawVector { .. } => vectors += 1,
            _ => {}
        }
    }
    let mask_kib = scene.masks().iter().map(|m| m.pixels().len()).sum::<usize>() as f64 / 1024.0;
    format!(
        "{rects} FillRect, {masks} DrawMask ({} mask resources, {mask_kib:.0} KiB), {images} DrawImage, {vectors} DrawVector",
        scene.masks().len()
    )
}

impl Spike {
    fn pert(&mut self, w: Workload, scale: f64, gpu: bool) -> Result<(Row, String)> {
        let mut scene = pertui::Scene::new(w.kind, scale)?;
        let mut cpu = CpuRenderer::new();
        let (mut update, mut compose, mut render, mut total) =
            (Samples::default(), Samples::default(), Samples::default(), Samples::default());
        let mut cold = None;
        let mut last = Vec::new();
        let mut command_mix = String::new();
        for frame in 0..self.warm + self.frames {
            let (list, u, c) = scene.compose(frame)?;
            if frame == 0 {
                cold = Some(ms(c));
                command_mix = mix(&list);
            }
            let started = Instant::now();
            let image = if gpu { self.gpu.render_offscreen(list)? } else { cpu.render(&list)? };
            let r = started.elapsed();
            if frame >= self.warm {
                update.push(u);
                compose.push(c);
                render.push(r);
                total.push(c + r);
            }
            if frame + 1 == self.warm + self.frames {
                last = image.pixels().to_vec();
            }
        }
        let name = if gpu { "pert-ui wgpu" } else { "pert-ui CPU" };
        let render_name = if gpu { "render_offscreen (prepare + GPU + read-back)" } else { "CpuRenderer::render" };
        Ok((
            Row {
                renderer: name.into(),
                phases: vec![("compose (paint)", compose), (render_name, render)],
                total,
                update: Some(update),
                cold_compose: cold,
                last,
            },
            command_mix,
        ))
    }

    fn vcpu(&mut self, w: Workload, scale: f64, threads: u16) -> Result<Row> {
        let (width, height) = extent(scale);
        let mut renderer = vello::Cpu::new(width as u16, height as u16, threads);
        let mut data = Data::new(w.kind);
        let (mut encode, mut render, mut total) = (Samples::default(), Samples::default(), Samples::default());
        for frame in 0..self.warm + self.frames {
            let f = data.advance(w.kind, frame);
            let (e, r) = renderer.frame(&f, scale);
            if frame >= self.warm {
                encode.push(e);
                render.push(r);
                total.push(e + r);
            }
        }
        Ok(Row {
            renderer: if threads == 0 { "vello_cpu 1 thread".into() } else { format!("vello_cpu {threads} workers") },
            phases: vec![("encode (+flush)", encode), ("render", render)],
            total,
            update: None,
            cold_compose: None,
            last: renderer.pixels(),
        })
    }

    fn vgpu(&mut self, w: Workload, scale: f64) -> Result<Row> {
        let (width, height) = extent(scale);
        self.vgpu.resize(width as u16, height as u16);
        let mut data = Data::new(w.kind);
        let mut phases: [Samples; 4] = Default::default();
        let mut total = Samples::default();
        let mut last = Vec::new();
        for frame in 0..self.warm + self.frames {
            let f = data.advance(w.kind, frame);
            let (t, pixels) = self.vgpu.frame(Some(&f), scale)?;
            if frame >= self.warm {
                for (s, d) in phases.iter_mut().zip([t.encode, t.prepare, t.gpu, t.readback]) {
                    s.push(d);
                }
                total.push(t.encode + t.prepare + t.gpu + t.readback);
            }
            last = pixels;
        }
        let [encode, prepare, gpu, readback] = phases;
        Ok(Row {
            renderer: "vello_gpu".into(),
            phases: vec![
                ("encode (Scene)", encode),
                ("prepare (Renderer::render)", prepare),
                ("submit + GPU completion", gpu),
                ("read-back", readback),
            ],
            total,
            update: None,
            cold_compose: None,
            last,
        })
    }

    fn empty_readback(&mut self, scale: f64) -> Result<(f64, f64)> {
        let mut ui = Runtime::new();
        let (w, h) = workloads::LOGICAL;
        ui.set_viewport(Viewport::new(w as f32, h as f32, scale)?)?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        let mut pert = Samples::default();
        let mut velo = Samples::default();
        for i in 0..self.warm + 100 {
            let started = Instant::now();
            self.gpu.render_offscreen(scene.clone())?;
            let p = started.elapsed();
            let (t, _) = self.vgpu.frame(None, scale)?;
            if i >= self.warm {
                pert.push(p);
                velo.push(t.encode + t.prepare + t.gpu + t.readback);
            }
        }
        Ok((pert.median(), velo.median()))
    }

    fn run(&mut self) -> Result<String> {
        let mut out = String::new();
        let mut parity_out = String::from("\n## Parity of the last frame (reference: pert-ui CPU)\n\n");
        parity_out += "Channel differences over R, G, B in 1/255. *Interior* = pixels whose 3×3 neighborhood in the reference is one color; *edge* = all others (antialiased edges, the line, gradient rows). Cells: max / mean / share of pixels > 2.\n\n";
        parity_out += "| Workload | Scale | Renderer | All | Interior | Edge (pixels) |\n|---|---|---|---|---|---|\n";
        for scale in [1.0, 2.0] {
            let (width, height) = extent(scale);
            self.gpu.resize(PhysicalExtent::new(width, height))?;
            self.vgpu.resize(width as u16, height as u16);
            let (pert_empty, vello_empty) = self.empty_readback(scale)?;
            writeln!(
                out,
                "\n## Scale {scale} ({width}×{height} physical)\n\nEmpty scene, render + read-back median: pert-ui wgpu {pert_empty:.3} ms, vello_gpu {vello_empty:.3} ms.\n"
            )?;
            for w in WORKLOADS {
                eprintln!("{} at scale {scale}", w.name);
                let (cpu_row, command_mix) = self.pert(w, scale, false)?;
                let (gpu_row, _) = self.pert(w, scale, true)?;
                let rows = vec![
                    cpu_row,
                    gpu_row,
                    self.vcpu(w, scale, 0)?,
                    self.vcpu(w, scale, self.threads)?,
                    self.vgpu(w, scale)?,
                ];
                writeln!(out, "### {} — scale {scale}\n\npert-ui command mix: {command_mix}.\n", w.name)?;
                out += "| Renderer | Frame total median / p95 ms | Phases median / p95 ms | Not in total: update median ms | Cold compose ms |\n|---|---|---|---|---|\n";
                for row in &rows {
                    let phases: Vec<String> =
                        row.phases.iter().map(|(n, s)| format!("{n}: {}", s.cell())).collect();
                    writeln!(
                        out,
                        "| {} | {} | {} | {} | {} |",
                        row.renderer,
                        row.total.cell(),
                        phases.join("<br>"),
                        row.update.as_ref().map_or("–".into(), |u| format!("{:.3}", u.median())),
                        row.cold_compose.map_or("–".into(), |c| format!("{c:.3}")),
                    )?;
                }
                out += "\n";
                let reference = &rows[0].last;
                let cell = |d: compare::Diff| format!("{} / {:.3} / {:.2}%", d.max, d.mean, d.over2 * 100.0);
                for row in &rows[1..] {
                    let p = compare::parity(reference, &row.last, width as usize, height as usize);

                    writeln!(
                        parity_out,
                        "| {} | {scale} | {} | {} | {} | {} ({}) |",
                        w.name,
                        row.renderer,
                        cell(p.all),
                        cell(p.interior),
                        cell(p.edge),
                        p.edge.pixels
                    )?;
                    if row.renderer == "pert-ui wgpu" && p.all.max > 16 {
                        let (x, y) = compare::worst(reference, &row.last, width as usize);
                        let at = |img: &[u8]| {
                            let i = (y * width as usize + x) * 4;
                            format!("{:?}", &img[i..i + 3])
                        };
                        writeln!(
                            parity_out,
                            "| ↳ | | largest pert-ui wgpu difference at physical ({x}, {y}), frame {}: RGB pert-ui CPU {}, wgpu {}, vello_cpu {} | | | |",
                            self.warm + self.frames - 1,
                            at(reference),
                            at(&row.last),
                            at(&rows[2].last)
                        )?;
                    }
                }
                let p = compare::parity(&rows[2].last, &rows[4].last, width as usize, height as usize);
                writeln!(
                    parity_out,
                    "| {} | {scale} | vello_gpu vs vello_cpu 1 thread | {} | {} | {} ({}) |",
                    w.name,
                    cell(p.all),
                    cell(p.interior),
                    cell(p.edge),
                    p.edge.pixels
                )?;
                if let pertui::Kind::Polyline { samples } = w.kind {
                    // Separates coverage from blend space: vello's coverage
                    // of the same line, composited afterwards in linear light
                    // as pert-ui composites.
                    let points = workloads::polyline(self.warm + self.frames - 1, samples);
                    let mut v = vello::Cpu::new(width as u16, height as u16, 0);
                    let alpha = v.coverage(&points, scale);
                    let linear = compare::composite_linear(&alpha, workloads::BACKGROUND, workloads::LINE);
                    let p = compare::parity(reference, &linear, width as usize, height as usize);
                    writeln!(
                        parity_out,
                        "| {} | {scale} | vello_cpu coverage composited in linear light (post hoc) | {} | {} | {} ({}) |",
                        w.name,
                        cell(p.all),
                        cell(p.interior),
                        cell(p.edge),
                        p.edge.pixels
                    )?;
                }
                if scale == 1.0 && w.name.starts_with("Panels (") {
                    self.save_images(&rows, width, height)?;
                }
                if scale == 1.0 && w.name == "Polyline 800 points" {
                    // Around the largest pert-ui CPU / vello_cpu difference.
                    let (wx, wy) = compare::worst(&rows[0].last, &rows[2].last, width as usize);
                    let (x0, y0) = (wx.saturating_sub(25).min(width as usize - 50), wy.saturating_sub(25).min(height as usize - 50));
                    eprintln!("polyline crop at {x0},{y0}");
                    for row in rows.iter().filter(|r| !r.renderer.contains("workers")) {
                        let slug = row.renderer.replace([' ', '-'], "_").replace("_1_thread", "");
                        let crop = compare::crop(&row.last, width as usize, x0, y0, 50, 50, 4);
                        compare::write_png(&self.out.join(format!("polyline-x4-{slug}.png")), &crop, 200, 200)?;
                    }
                }
                if scale == 1.0 && w.name.starts_with("Gradient") {
                    self.save_bars(&rows, width)?;
                }
            }
        }
        Ok(out + &parity_out)
    }

    fn save_images(&self, rows: &[Row], width: u32, height: u32) -> Result<()> {
        let items = workloads::panels(150, 100);
        let first = items
            .iter()
            .find_map(|i| match i {
                workloads::Item::Panel(p) => Some(p.rect),
                _ => None,
            })
            .ok_or("no panel")?;
        let (cx, cy) = (first.x as usize, first.y as usize);
        for row in rows.iter().filter(|r| !r.renderer.contains("workers")) {
            let slug = row.renderer.replace([' ', '-'], "_").replace("_1_thread", "");
            compare::write_png(&self.out.join(format!("panels-{slug}.png")), &row.last, width, height)?;
            let crop = compare::crop(&row.last, width as usize, cx.saturating_sub(2), cy.saturating_sub(2), 200, 200, 1);
            compare::write_png(&self.out.join(format!("corner-{slug}.png")), &crop, 200, 200)?;
            let zoom = compare::crop(&row.last, width as usize, cx.saturating_sub(2), cy.saturating_sub(2), 25, 25, 8);
            compare::write_png(&self.out.join(format!("corner-x8-{slug}.png")), &zoom, 200, 200)?;
        }
        Ok(())
    }

    fn save_bars(&self, rows: &[Row], width: u32) -> Result<()> {
        // The first four bars, cropped, from each renderer.
        let m = workloads::METER;
        let (x, y, w, h) = (m.x as usize - 4, m.y as usize - 4, 136, m.h as usize + 8);
        for row in rows.iter().filter(|r| !r.renderer.contains("workers")) {
            let slug = row.renderer.replace([' ', '-'], "_").replace("_1_thread", "");
            let crop = compare::crop(&row.last, width as usize, x, y, w, h, 1);
            compare::write_png(&self.out.join(format!("bars-{slug}.png")), &crop, w as u32, h as u32)?;
        }
        Ok(())
    }
}

impl Spike {
    /// Part 2: the pert-ui DisplayLists of the same workloads, translated
    /// command by command into vello_cpu (src/translate.rs).
    fn translated(&mut self) -> Result<String> {
        let mut out = String::from("\n## Part 2: pert-ui DisplayList translated into vello_cpu\n\n");
        out += "Per frame: pert-ui compose (not in the totals), translation (walking the commands, issuing vello calls, converting and registering resources not seen before; with `flush`) and vello_cpu render. Parity against pert-ui CPU rendering the same DisplayList. Cells: median / p95 ms; parity max / mean / share > 2 over all, interior, edge.\n\n";
        out += "| Workload | Scale | Threads | Translate | Render | Total | First-frame translate ms (new resources, KiB) | Parity all | Interior | Edge |\n|---|---|---|---|---|---|---|---|---|---|\n";
        for scale in [1.0, 2.0] {
            let (width, height) = extent(scale);
            for w in WORKLOADS {
                for threads in [0, self.threads] {
                    let mut scene = pertui::Scene::new(w.kind, scale)?;
                    let mut translator = translate::Translator::new(threads);
                    let (mut tr, mut re, mut total) = (Samples::default(), Samples::default(), Samples::default());
                    let mut first = String::new();
                    let mut last = None;
                    for frame in 0..self.warm + self.frames {
                        let (list, _, _) = scene.compose(frame)?;
                        let (pixels, stats) = translator.render(&list);
                        if frame == 0 {
                            first = format!(
                                "{:.3} ({}, {:.0})",
                                ms(stats.translate),
                                stats.new_resources,
                                stats.new_bytes as f64 / 1024.0
                            );
                        }
                        if stats.unsupported > 0 {
                            return Err(format!("{} unsupported commands", stats.unsupported).into());
                        }
                        if frame >= self.warm {
                            tr.push(stats.translate);
                            re.push(stats.render);
                            total.push(stats.translate + stats.render);
                        }
                        last = Some((list, pixels));
                    }
                    let (list, pixels) = last.ok_or("no frames")?;
                    let reference = CpuRenderer::new().render(&list)?;
                    let glass = translate::translucent(&list);
                    let p = compare::classified(reference.pixels(), &pixels, width as usize, height as usize, Some(&glass));
                    assert_eq!(p.translucent.pixels, 0, "the workloads draw only opaque colors");
                    let cell = |d: compare::Diff| format!("{} / {:.3} / {:.2}%", d.max, d.mean, d.over2 * 100.0);
                    writeln!(
                        out,
                        "| {} | {scale} | {} | {} | {} | {} | {first} | {} | {} | {} |",
                        w.name,
                        if threads == 0 { "1".to_string() } else { format!("{threads} workers") },
                        tr.cell(),
                        re.cell(),
                        total.cell(),
                        cell(p.all),
                        cell(p.interior),
                        cell(p.edge)
                    )?;
                    if scale == 1.0 && threads == 0 && w.name.starts_with("Panels (") {
                        compare::write_png(&self.out.join("panels-translated_vello_cpu.png"), &pixels, width, height)?;
                    }
                }
            }
        }
        Ok(out)
    }
}

fn extent(scale: f64) -> (u32, u32) {
    let (w, h) = workloads::LOGICAL;
    ((f64::from(w) * scale) as u32, (f64::from(h) * scale) as u32)
}

struct Harness {
    report: Option<Result<String>>,
}

impl ApplicationHandler<Notice> for Harness {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.report.is_some() {
            return;
        }
        let result = (|| -> Result<String> {
            let window = Arc::new(event_loop.create_window(
                Window::default_attributes()
                    .with_title("vello spike")
                    .with_inner_size(winit::dpi::PhysicalSize::new(1280, 800)),
            )?);
            let size = window.inner_size();
            let gpu = Renderer::new(
                window.clone(),
                PhysicalExtent::new(size.width, size.height),
                |notice| eprintln!("renderer notice: {notice:?}"),
            )?;
            let vgpu = vello::Gpu::new()?;
            let threads = std::thread::available_parallelism()?.get().saturating_sub(1).min(8) as u16;
            let frames = std::env::var("SPIKE_FRAMES").ok().and_then(|f| f.parse().ok()).unwrap_or(200);
            let out = PathBuf::from(std::env::var("SPIKE_OUT").unwrap_or_else(|_| "results".into()));
            std::fs::create_dir_all(&out)?;
            let mut report = format!(
                "# vello spike measurements\n\nvello_gpu adapter: {:?}\n\n{frames} measured frames after 20 warm-up frames per renderer, workload and scale. vello_cpu multithreaded uses {threads} worker threads (available_parallelism − 1, capped at 8, vello's default). Times in ms, median / p95.\n",
                vgpu.info
            );
            let mut spike = Spike { gpu, vgpu, warm: 20, frames, threads, out };
            report += &spike.run()?;
            if std::env::var_os("SPIKE_SKIP_TRANSLATE").is_none() {
                report += &spike.translated()?;
            }
            drop(window);
            Ok(report)
        })();
        self.report = Some(result);
        event_loop.exit();
    }
    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
}

fn main() -> Result<()> {
    if std::env::args().nth(1).as_deref() == Some("present") {
        return present::run();
    }
    let event_loop = EventLoop::<Notice>::with_user_event().build()?;
    let mut harness = Harness { report: None };
    event_loop.run_app(&mut harness)?;
    let report = harness.report.ok_or("the event loop never resumed")??;
    print!("{report}");
    Ok(())
}
