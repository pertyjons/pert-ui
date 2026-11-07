//! J1 throwaway headless probe: resize / scale cost breakdown.
use pert_ui::{ProfileOperation, Runtime, Viewport, WidgetId};
use std::collections::BTreeMap;
use std::error::Error;
use std::time::{Duration, Instant};

const WINDOW: (f32, f32) = (1280.0, 800.0);
type Result<T> = std::result::Result<T, Box<dyn Error>>;

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
fn count_nodes(ui: &Runtime, from: &WidgetId) -> Result<usize> {
    let mut stack = vec![from.clone()];
    let mut n = 0;
    while let Some(id) = stack.pop() {
        n += 1;
        stack.extend(ui.children(&id)?);
    }
    Ok(n)
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

pub(crate) fn run() -> Result<()> {
    let page_key = std::env::var("PROBE_CHANGE_PAGE").unwrap_or_else(|_| "navigation".into());
    let scenario = std::env::var("PROBE_SCENARIO").unwrap_or_else(|_| "resize".into());
    let frames: usize = std::env::var("PROBE_FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
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
    let _ = ui.paint()?;
    let nodes = count_nodes(&ui, &root)?;
    println!("page {page_key} scenario {scenario}: nodes reachable from root = {nodes}, page subtree = {}", count_nodes(&ui, &page)?);
    ui.set_profiling(true)?;
    ui.clear_profile()?;
    pert_ui_runtime::j1::take();
    pert_ui_runtime::j1::enable(std::env::var("PROBE_J1").as_deref() != Ok("0"));
    let steps: Vec<(f32, f64)> = match scenario.as_str() {
        "resize" => (1..=frames).map(|i| (WINDOW.0 + 7.0 * i as f32, 1.0)).collect(),
        "scale" => [1.25, 1.5, 1.75, 2.0].into_iter().map(|s| (WINDOW.0, s)).collect(),
        // Same viewport each frame, for the no-change baseline.
        "idle" => (1..=frames).map(|_| (WINDOW.0, 1.0)).collect(),
        // Resize back and forth between two widths.
        "toggle" => (1..=frames).map(|i| (WINDOW.0 + if i % 2 == 0 { 0.0 } else { 7.0 }, 1.0)).collect(),
        _ => return Err("bad scenario".into()),
    };
    let (mut vp, mut fl, mut pa, mut tot) = (vec![], vec![], vec![], vec![]);
    for (width, scale) in &steps {
        let t0 = Instant::now();
        ui.set_viewport(Viewport::new(*width, WINDOW.1, *scale)?)?;
        let t1 = Instant::now();
        now += Duration::from_micros(16_667);
        {
            let _j = pert_ui_runtime::j1::time("P.flush");
            ui.flush(now)?;
        }
        let t2 = Instant::now();
        {
            let _j = pert_ui_runtime::j1::time("P.paint");
            let _ = ui.paint()?;
        }
        let t3 = Instant::now();
        let d = pert_ui_runtime::j1::take_distinct();
        if vp.is_empty() || vp.len() + 1 == steps.len() {
            println!("frame {} distinct: {:?}", vp.len() + 1, d);
        }
        vp.push(ms(t1 - t0));
        fl.push(ms(t2 - t1));
        pa.push(ms(t3 - t2));
        tot.push(ms(t3 - t0));
    }
    pert_ui_runtime::j1::enable(false);
    let n = steps.len() as f64;
    let load = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
    println!("loadavg at end: {}", load.trim());
    println!(
        "median ms: set_viewport {:.3}, flush {:.2}, paint {:.2}, total {:.2}",
        median(vp),
        median(fl),
        median(pa),
        median(tot)
    );
    let profile = ui.profile();
    let mut by_op: BTreeMap<String, (f64, f64, u64)> = Default::default();
    for row in &profile.stats {
        let e = by_op.entry(format!("{:?}", row.operation)).or_default();
        e.0 += ms(row.self_time);
        e.1 += ms(row.total);
        e.2 += row.calls;
    }
    let mut ranked: Vec<_> = by_op.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.total_cmp(&a.1.0));
    println!("profiler per frame: op self_ms total_ms count");
    for (name, (s, t, c)) in ranked.iter().take(12) {
        println!("  {name:14} {:8.3} {:8.3} {:8.1}", s / n, t / n, *c as f64 / n);
    }
    let mut measure: BTreeMap<String, (u64, f64)> = Default::default();
    for row in &profile.stats {
        if format!("{:?}", row.operation) == "Measure" {
            let e = measure.entry(row.key.clone()).or_default();
            e.0 += row.calls;
            e.1 += ms(row.self_time);
        }
    }
    let mut m: Vec<_> = measure.into_iter().collect();
    m.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    println!("Measure calls per frame by key (top 15):");
    for (k, (c, t)) in m.iter().take(15) {
        println!("  {k:40} {:8.1} {:8.3}", *c as f64 / n, t / n);
    }
    println!("j1 per frame: name total_ms count");
    for (name, d, c) in pert_ui_runtime::j1::take() {
        println!("  {name:28} {:8.3} {:10.1}", ms(d) / n, c as f64 / n);
    }
    Ok(())
}
