//! Part 2 on real scenes: every headless gallery page's DisplayList,
//! translated into vello_cpu by ../src/translate.rs, against pert-ui's CPU
//! renderer. `gallery.sh` builds this file into a scratch crate beside the
//! gallery's modules; nothing here opens a window or needs a GPU.
use crate::{compare, translate};
use pert_ui::cpu::CpuRenderer;
use pert_ui::{DisplayList, DrawCommand, Runtime, Viewport, WidgetId};
use std::error::Error;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const WINDOW: (f32, f32) = (1280.0, 800.0);
const FRAMES: usize = 30;

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

fn scene_of(key: &str, scale: f64) -> Result<Option<DisplayList>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
    let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
    let mut now = Duration::ZERO;
    ui.flush(now)?;
    let page = pages(&ui, &root)?.into_iter().find(|p| p.0 == key).ok_or("page vanished")?.1;
    if !ui.reveal(&page)?.is_revealed() {
        return Ok(None);
    }
    for _ in 0..4 {
        now += Duration::from_millis(100);
        ui.flush(now)?;
    }
    Ok(Some(ui.paint()?))
}

pub fn run() -> Result<()> {
    let threads = std::thread::available_parallelism()?.get().saturating_sub(1).min(8) as u16;
    let out_dir = std::path::PathBuf::from(std::env::var("SPIKE_OUT").unwrap_or_else(|_| "results".into()));
    let keys: Vec<String> = {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, 1.0)?)?;
        let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
        ui.flush(Duration::ZERO)?;
        pages(&ui, &root)?.into_iter().map(|p| p.0).collect()
    };
    let mut out = String::from("\n## Part 2 on the headless gallery: DisplayList → vello_cpu\n\n");
    writeln!(out, "Every gallery page at 1280×800 logical, painted once; the same DisplayList rendered {FRAMES} times after one warm-up by pert-ui's CpuRenderer and by the translator (vello_cpu 1 thread and {threads} workers). Cold = the first translation, which converts and registers every mask and image (KiB = RGBA8 pixmap bytes registered). Times median / p95 ms. Parity of the translated frame against pert-ui CPU: max / mean / share > 2 over opaque interior pixels (3×3 uniform in the reference, under no translucent command), translucent interior pixels (3×3 uniform, under a fill/mask color with alpha < 1 or an image with translucent tint or pixels) and edge pixels (the rest).\n")?;
    out += "| Page | Scale | Rects / masks / images / vectors | pert-ui CPU render | Translate cold ms (KiB) | Translate warm | vello_cpu render 1 thread | Translate warm / render, workers | Opaque interior | Translucent interior | Edge |\n|---|---|---|---|---|---|---|---|---|---|---|\n";
    let mut saved = false;
    for scale in [1.0, 2.0] {
        for key in &keys {
            let Some(scene) = scene_of(key, scale)? else {
                writeln!(out, "| {key} | {scale} | not revealed |")?;
                continue;
            };
            let (mut rects, mut masks, mut images, mut vectors) = (0, 0, 0, 0);
            for c in scene.commands() {
                match c {
                    DrawCommand::FillRect { .. } => rects += 1,
                    DrawCommand::DrawMask { .. } => masks += 1,
                    DrawCommand::DrawImage { .. } => images += 1,
                    DrawCommand::DrawVector { .. } => vectors += 1,
                    _ => {}
                }
            }
            let mut cpu = CpuRenderer::new();
            let reference = cpu.render(&scene)?;
            let cpu_times: Vec<f64> = (0..FRAMES)
                .map(|_| -> Result<f64> {
                    let s = Instant::now();
                    cpu.render(&scene)?;
                    Ok(ms(s.elapsed()))
                })
                .collect::<Result<_>>()?;
            let mut st = translate::Translator::new(0);
            let (pixels, cold) = st.render(&scene);
            if cold.unsupported > 0 {
                return Err(format!("{key}: {} unsupported commands", cold.unsupported).into());
            }
            let (mut tr, mut re) = (Vec::new(), Vec::new());
            for _ in 0..FRAMES {
                let (_, s) = st.render(&scene);
                tr.push(ms(s.translate));
                re.push(ms(s.render));
            }
            let mut mt = translate::Translator::new(threads);
            let _ = mt.render(&scene);
            let (mut mtr, mut mre) = (Vec::new(), Vec::new());
            for _ in 0..FRAMES {
                let (_, s) = mt.render(&scene);
                mtr.push(ms(s.translate));
                mre.push(ms(s.render));
            }
            let extent = scene.viewport().frame_extent();
            let glass = translate::translucent(&scene);
            let p = compare::classified(reference.pixels(), &pixels, extent.width() as usize, extent.height() as usize, Some(&glass));
            let cell = |d: compare::Diff| format!("{} / {:.2} / {:.2}%", d.max, d.mean, d.over2 * 100.0);
            writeln!(
                out,
                "| {key} | {scale} | {rects} / {masks} / {images} / {vectors} | {:.2} / {:.2} | {:.2} ({:.0}) | {:.3} / {:.3} | {:.2} / {:.2} | {:.3} / {:.2} | {} | {} | {} |",
                median(cpu_times.clone()),
                p95(cpu_times),
                ms(cold.translate),
                cold.new_bytes as f64 / 1024.0,
                median(tr.clone()),
                p95(tr),
                median(re.clone()),
                p95(re),
                median(mtr),
                median(mre),
                cell(p.interior),
                cell(p.translucent),
                cell(p.edge),
            )?;
            eprintln!("{key} at {scale} done");
            if scale == 1.0 && !saved && std::env::var("SPIKE_SAVE_PAGE").is_ok_and(|k| &k == key) {
                std::fs::create_dir_all(&out_dir)?;
                compare::write_png(&out_dir.join(format!("gallery-{key}-pert_ui_CPU.png")), reference.pixels(), extent.width(), extent.height())?;
                compare::write_png(&out_dir.join(format!("gallery-{key}-translated_vello_cpu.png")), &pixels, extent.width(), extent.height())?;
                saved = true;
            }
        }
    }
    print!("{out}");
    Ok(())
}
