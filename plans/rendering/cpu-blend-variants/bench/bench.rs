//! Renders the same DisplayLists with every blend-storage variant of the
//! copied CPU renderer and with vello_cpu (through the vello spike's
//! translator), interleaved per scene. `bench.sh` builds this file into a
//! scratch crate beside the gallery's modules and the spike's sources.
use crate::{compare, pertui, translate};
use pert_ui::{DisplayList, Runtime, Viewport, WidgetId};
use pert_ui_cpu_variants::{CpuRenderer, Variant};
use std::error::Error;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const WINDOW: (f32, f32) = (1280.0, 800.0);
const VARIANTS: [(Variant, &str); 4] = [
    (Variant::Current, "(0) current"),
    (Variant::Encoded8, "(a) encoded 8-bit"),
    (Variant::Linear16, "(b) linear 16-bit"),
    (Variant::Linear16Fast, "(c) (b) + opaque fill fast path"),
];

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
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

fn page_scene(key: &str, scale: f64) -> Result<Option<DisplayList>> {
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

/// One scene's medians: variants (0, a, b, c), vello_cpu, and the diffs of
/// a, b, c against (0), plus (0) against the real pert-ui-cpu.
struct Measured {
    times: [f64; 4],
    /// Variant (0) split: clear, vector preparation, fills, masks, images.
    split: [f64; 5],
    vello: f64,
    diffs: [compare::Parity; 3],
    original: compare::Diff,
}

fn measure(scene: &DisplayList, frames: usize) -> Result<Measured> {
    let extent = scene.viewport().frame_extent();
    let (w, h) = (extent.width() as usize, extent.height() as usize);
    let mut renderers: Vec<CpuRenderer> = VARIANTS.iter().map(|(v, _)| CpuRenderer::with_variant(*v)).collect();
    let mut translator = translate::Translator::new(0);
    let mut images = Vec::new();
    for r in &mut renderers {
        images.push(r.render(scene)?.pixels().to_vec());
    }
    let _ = translator.render(scene);
    let mut samples: [Vec<f64>; 5] = Default::default();
    for _ in 0..frames {
        for (i, r) in renderers.iter_mut().enumerate() {
            let t = Instant::now();
            r.render(scene)?;
            samples[i].push(ms(t.elapsed()));
        }
        let t = Instant::now();
        let _ = translator.render(scene);
        samples[4].push(ms(t.elapsed()));
    }
    let glass = translate::translucent(scene);
    let diffs = std::array::from_fn(|i| compare::classified(&images[0], &images[i + 1], w, h, Some(&glass)));
    let original = pert_ui::cpu::CpuRenderer::new().render(scene)?;
    let original = compare::parity(original.pixels(), &images[0], w, h).all;
    let mut split: [Vec<f64>; 5] = Default::default();
    let mut profiler = CpuRenderer::with_variant(Variant::Current);
    let _ = profiler.profile_current(scene)?;
    for _ in 0..frames.min(10) {
        for (s, d) in split.iter_mut().zip(profiler.profile_current(scene)?) {
            s.push(ms(d));
        }
    }
    let [a, b, c, d, e] = samples.map(median);
    Ok(Measured { times: [a, b, c, d], vello: e, split: split.map(median), diffs, original })
}

fn cell(d: compare::Diff) -> String {
    format!("{} / {:.2} / {:.2}%", d.max, d.mean, d.over2 * 100.0)
}

pub fn run() -> Result<()> {
    let frames: usize = std::env::var("BENCH_FRAMES").ok().and_then(|f| f.parse().ok()).unwrap_or(30);
    let mut out = format!(
        "# CPU blend-storage variants\n\nEach scene rendered {frames} times after one warm-up by every variant and by vello_cpu 1 thread (translation + render, src/translate.rs of the vello spike), interleaved. Times are medians in ms. Diffs against (0): max / mean / share > 2 in 1/255 over opaque interior / translucent interior / edge pixels. *(0) = pert-ui-cpu* is the largest difference between variant (0) and the real `pert_ui::cpu::CpuRenderer` (must be 0).\n\n"
    );
    let header = "| Scene | Scale | (0) | (a) | (b) | (c) | vello_cpu | (0) split: clear / vectors / fills / masks / images | (0) = pert-ui-cpu | (a) vs (0) | (b) vs (0) | (c) vs (0) |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n";
    let diffs = |m: &Measured| -> String {
        m.diffs
            .iter()
            .map(|p| format!("{}<br>{}<br>{}", cell(p.interior), cell(p.translucent), cell(p.edge)))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let row = |name: &str, scale: f64, m: &Measured| -> String {
        format!(
            "| {name} | {scale} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {} | {} |\n",
            m.times[0], m.times[1], m.times[2], m.times[3], m.vello,
            m.split.iter().map(|v| format!("{v:.2}")).collect::<Vec<_>>().join(" / "),
            m.original.max, diffs(m)
        )
    };
    out += "## Synthetic workloads (frame 219)\n\n";
    out += header;
    let workloads = [
        ("Panels (150 + 100)", pertui::Kind::Panels { panels: 150, plain: 100 }),
        ("Panels light (20 + 20)", pertui::Kind::Panels { panels: 20, plain: 20 }),
        ("Polyline 800", pertui::Kind::Polyline { samples: 800 }),
        ("Polyline 4096", pertui::Kind::Polyline { samples: 4096 }),
        ("Gradient bars", pertui::Kind::Bars),
    ];
    for scale in [1.0, 2.0] {
        // The background alone: each variant's fixed cost per frame.
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
        ui.flush(Duration::ZERO)?;
        out += &row("Empty scene (background only)", scale, &measure(&ui.paint()?, frames)?);
        for (name, kind) in workloads {
            let mut scene = pertui::Scene::new(kind, scale)?;
            let (list, _, _) = scene.compose(219)?;
            let m = measure(&list, frames)?;
            out += &row(name, scale, &m);
            eprintln!("{name} {scale}");
        }
    }
    let keys: Vec<String> = {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, 1.0)?)?;
        let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
        ui.flush(Duration::ZERO)?;
        pages(&ui, &root)?.into_iter().map(|p| p.0).collect()
    };
    let mut summary = String::from("\n## Gallery summary\n\nPer scale, over the pages: min / median / max of each renderer's median time (ms), and the share of the gap between (0) and vello_cpu that a variant closes, `((0) − variant) / ((0) − vello_cpu)` per page (min / median / max).\n\n| Scale | (0) | (a) | (b) | (c) | vello_cpu | Gap closed by (a) | by (b) | by (c) |\n|---|---|---|---|---|---|---|---|---|\n");
    let mut splits = String::from("\nShare of variant (0)'s time per part over the pages (min / median / max), from `profile_current`:\n\n| Scale | Clear | Vector preparation | FillRect | DrawMask | DrawImage |\n|---|---|---|---|---|---|\n");
    out += "\n## Gallery pages\n\n";
    out += header;
    for scale in [1.0, 2.0] {
        let mut all: Vec<Measured> = Vec::new();
        for key in &keys {
            let Some(scene) = page_scene(key, scale)? else { continue };
            let m = measure(&scene, frames)?;
            out += &row(key, scale, &m);
            eprintln!("{key} {scale}");
            all.push(m);
        }
        let stat = |v: Vec<f64>| {
            let mut s = v.clone();
            s.sort_by(f64::total_cmp);
            format!("{:.2} / {:.2} / {:.2}", s[0], s[s.len() / 2], s[s.len() - 1])
        };
        let col = |i: usize| all.iter().map(|m| m.times[i]).collect::<Vec<_>>();
        let gap = |i: usize| {
            all.iter()
                .map(|m| (m.times[0] - m.times[i]) / (m.times[0] - m.vello))
                .collect::<Vec<_>>()
        };
        let share = |i: usize| all.iter().map(|m| m.split[i] / m.split.iter().sum::<f64>()).collect::<Vec<_>>();
        writeln!(
            splits,
            "| {scale} | {} | {} | {} | {} | {} |",
            stat(share(0)),
            stat(share(1)),
            stat(share(2)),
            stat(share(3)),
            stat(share(4)),
        )?;
        writeln!(
            summary,
            "| {scale} | {} | {} | {} | {} | {} | {} | {} | {} |",
            stat(col(0)),
            stat(col(1)),
            stat(col(2)),
            stat(col(3)),
            stat(all.iter().map(|m| m.vello).collect()),
            stat(gap(1)),
            stat(gap(2)),
            stat(gap(3)),
        )?;
    }
    print!("{out}{summary}{splits}");
    Ok(())
}
