//! Slice 0 prototype of `plans/rendering/stroke-spec.md`: strokes covered
//! per pixel from their segments, on the GPU (tiled segment lists, or
//! per-segment quads into a coverage atlas) and the CPU, against today's
//! mask path. See README.md for the modes and how to run them.
mod cpu;
mod gpu;
mod stroke;
mod today;
mod workloads;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use stroke::{Frame, Prepared, Tiling};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn quick() -> bool {
    std::env::var("PROBE_QUICK").is_ok_and(|v| v == "1")
}
fn tile_size() -> u32 {
    std::env::var("PROBE_TILE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(16)
}

fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "check".into());
    match mode.as_str() {
        "check" => check(),
        "compare" => compare(),
        "time" => time(),
        other => Err(format!("unknown mode {other}; use check, compare or time").into()),
    }
}

fn case_frame(scale: f64) -> Frame {
    let side = (f64::from(workloads::CASE_FRAME) * scale).ceil() as u32;
    Frame {
        width: side,
        height: side,
        scale,
    }
}

// The smallest distance over every segment at frame pixel (x, y), which may
// lie outside the span; ignores the tile lists.
fn brute(prepared: &Prepared, x: u32, y: u32) -> f32 {
    let header = &prepared.headers[0];
    let p = [
        (i64::from(x) - i64::from(header.origin[0])) as f32 + 0.5,
        (i64::from(y) - i64::from(header.origin[1])) as f32 + 0.5,
    ];
    prepared
        .segments
        .iter()
        .map(|s| stroke::segment_distance(s, header.h, p))
        .fold(f32::INFINITY, f32::min)
}

// The reference distance and coverage per frame pixel of one prepared
// stroke; pixels in no tile are infinitely far and uncovered.
fn reference(prepared: &Prepared, frame: Frame) -> Vec<(f32, f32)> {
    let mut out = vec![(f32::INFINITY, 0.0); (frame.width * frame.height) as usize];
    for tile in &prepared.tiles {
        for y in tile.rect[1]..tile.rect[3] {
            for x in tile.rect[0]..tile.rect[2] {
                out[(y * frame.width + x) as usize] = stroke::evaluate(prepared, tile, x, y);
            }
        }
    }
    out
}

/// The parity corpus: GPU coverage against the reference, tile lists
/// against every segment, and the support against the pixels outside it. On
/// the quads path the preparation records cells, not lists: the reference is
/// then every segment inside the span, and a cell the composite misses shows
/// as a differing pixel.
fn check() -> Result<()> {
    let tile = tile_size();
    let path = gpu::Draw::from_env()?;
    let tiling = if path == gpu::Draw::Quads {
        Tiling::Cells
    } else {
        Tiling::Lists
    };
    // The quads path stores coverage in 8 bits: a decided pixel may differ by
    // the rounding, up to a step where the conversion is not exact.
    let tolerance = if path == gpu::Draw::Quads {
        1.0 / 255.0
    } else {
        0.0
    };
    let mut worst_decided = 0.0f32;
    let cases = workloads::corpus(quick());
    let mut gpus: HashMap<u32, gpu::Gpu> = HashMap::new();
    let (mut edge_pixels, mut decided_pixels) = (0u64, 0u64);
    let (mut worst, mut worst_case) = (0.0f32, String::new());
    let (mut decided_mismatch, mut list_violations, mut support_violations) = (0u64, 0u64, 0u64);
    let mut failures: Vec<String> = Vec::new();
    let mut adapter = String::new();
    for case in &cases {
        let frame = case_frame(case.scale);
        let mut prepared = Prepared::default();
        let Some(_) = stroke::prepare(&case.stroke, frame, tile, tiling, &mut prepared)? else {
            continue;
        };
        let gpu = match gpus.entry(frame.width) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(gpu::Gpu::new(frame.width, frame.height)?)
            }
        };
        adapter = gpu.adapter.clone();
        gpu.upload(&prepared);
        gpu.pass(path, true)?;
        let measured = gpu.read_coverage()?;
        let span = prepared.extents[0].span;
        let inside = |x: u32, y: u32| x >= span[0] && y >= span[1] && x < span[2] && y < span[3];
        let expected = match tiling {
            Tiling::Lists => reference(&prepared, frame),
            Tiling::Cells => {
                let thin = prepared.headers[0].thin;
                (0..frame.width * frame.height)
                    .map(|i| {
                        let (x, y) = (i % frame.width, i / frame.width);
                        if inside(x, y) {
                            let distance = brute(&prepared, x, y);
                            (distance, stroke::coverage(distance, thin))
                        } else {
                            (f32::INFINITY, 0.0)
                        }
                    })
                    .collect()
            }
        };
        let threshold = 0.5 + stroke::MARGIN as f32;
        // Pixels the clip removes, by the pixel-center rule, are uncovered
        // whatever the geometry.
        let clipped = |x: u32, y: u32| {
            case.stroke.clip.is_some_and(|[cx, cy, cw, ch]| {
                let edge = |v: f32| (f64::from(v) * frame.scale - 0.5).ceil();
                let (x, y) = (f64::from(x), f64::from(y));
                x < edge(cx) || y < edge(cy) || x >= edge(cx + cw) || y >= edge(cy + ch)
            })
        };
        for y in 0..frame.height {
            for x in 0..frame.width {
                let i = (y * frame.width + x) as usize;
                let (distance, cpu) = expected[i];
                let gpu_value = measured[i];
                if stroke::decided(distance) {
                    decided_pixels += 1;
                    worst_decided = worst_decided.max((gpu_value - cpu).abs() * 255.0);
                    if (gpu_value - cpu).abs() > tolerance {
                        decided_mismatch += 1;
                        if failures.len() < 10 {
                            failures.push(format!(
                                "{}: decided ({x}, {y}) cpu {cpu} gpu {gpu_value}",
                                case.name
                            ));
                        }
                    }
                } else {
                    edge_pixels += 1;
                    let difference = (gpu_value - cpu).abs() * 255.0;
                    if difference > worst {
                        worst = difference;
                        worst_case = format!("{} at ({x}, {y})", case.name);
                    }
                }
                let all = brute(&prepared, x, y);
                if all < threshold && !clipped(x, y) {
                    if !inside(x, y) {
                        // Coverage the span does not reach.
                        support_violations += 1;
                        if failures.len() < 10 {
                            failures.push(format!(
                                "{}: outside the span ({x}, {y}) distance {all}",
                                case.name
                            ));
                        }
                    } else if all != distance {
                        list_violations += 1;
                        if failures.len() < 10 {
                            failures.push(format!(
                                "{}: list ({x}, {y}) listed {distance} all {all}",
                                case.name
                            ));
                        }
                    }
                }
            }
        }
    }
    println!("# Stroke prototype: parity check\n");
    println!(
        "Adapter: {adapter}. Path {path:?}. Tile size {tile}. {} cases{}.\n",
        cases.len(),
        if quick() { " (quick)" } else { "" }
    );
    println!("| Measure | Value |\n|---|---|");
    println!("| Decided pixels | {decided_pixels} |");
    println!(
        "| Decided pixels that differ{} | {decided_mismatch} |",
        if tolerance > 0.0 {
            " by more than one step"
        } else {
            ""
        }
    );
    println!("| Largest decided difference (1/255 steps) | {worst_decided:.3} |");
    println!("| Edge pixels | {edge_pixels} |");
    println!("| Largest edge difference (coverage, 1/255 steps) | {worst:.3} |");
    println!("| Pixels whose list misses a covering segment | {list_violations} |");
    println!("| Covered pixels outside the span | {support_violations} |");
    println!("\nLargest edge difference: {worst_case}");
    for failure in &failures {
        println!("- {failure}");
    }
    if decided_mismatch + list_violations + support_violations > 0 {
        return Err("the check found violations".into());
    }
    Ok(())
}

fn srgb_decode(value: u8) -> f32 {
    let v = f32::from(value) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

/// How far the new coverage lies from what today's CPU renderer draws.
fn compare() -> Result<()> {
    let tile = tile_size();
    let buckets = [1.0f32, 2.0, 4.0, 16.0, 64.0, 255.0];
    let mut by_shape: Vec<(String, [u64; 6], f32, u64)> = Vec::new();
    let mut todays: HashMap<u32, (today::Today, Vec<u8>)> = HashMap::new();
    let mut largest: Vec<(f32, String)> = Vec::new();
    for case in workloads::corpus(quick()) {
        if case.stroke.scale != 1.0 || case.stroke.clip.is_some() {
            continue;
        }
        let frame = case_frame(case.scale);
        let mut prepared = Prepared::default();
        if stroke::prepare(&case.stroke, frame, tile, Tiling::Lists, &mut prepared)?.is_none() {
            continue;
        }
        let new = reference(&prepared, frame);
        let (today, empty) = match todays.entry(frame.width) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                // The background as the renderer quantizes it, per pixel.
                let mut today = today::Today::new(frame)?;
                let scene = today.scene(&[])?;
                let empty = today.cpu.render_opaque(&scene)?.pixels().to_vec();
                e.insert((today, empty))
            }
        };
        let scene = today.scene(std::slice::from_ref(&case.stroke))?;
        let image = today.cpu.render_opaque(&scene)?;
        let shape = case.name.split(',').next().unwrap_or_default().to_string();
        let index = match by_shape.iter().position(|(s, ..)| *s == shape) {
            Some(i) => i,
            None => {
                by_shape.push((shape, [0; 6], 0.0, 0));
                by_shape.len() - 1
            }
        };
        let entry = &mut by_shape[index];
        for (i, &(_, c_new)) in new.iter().enumerate() {
            let (byte, background) = (image.pixels()[i * 4 + 1], empty[i * 4 + 1]);
            let background = srgb_decode(background);
            let c_old = if byte == empty[i * 4 + 1] {
                0.0
            } else {
                ((srgb_decode(byte) - background) / (1.0 - background)).clamp(0.0, 1.0)
            };
            if c_old <= 0.0 && c_new <= 0.0 {
                continue;
            }
            entry.3 += 1;
            let difference = (c_new - c_old).abs() * 255.0;
            entry.2 = entry.2.max(difference);
            if difference > 64.0 && std::env::var("PROBE_DEBUG").is_ok() {
                let (x, y) = (i as u32 % frame.width, i as u32 / frame.width);
                largest.push((
                    difference,
                    format!(
                        "{}: ({x}, {y}) today {:.3}, new {c_new:.3}, distance {:.3}",
                        case.name, c_old, new[i].0
                    ),
                ));
            }
            let bucket = buckets.iter().position(|&b| difference <= b).unwrap_or(5);
            entry.1[bucket] += 1;
        }
    }
    largest.sort_by(|a, b| b.0.total_cmp(&a.0));
    let shown: usize = std::env::var("PROBE_DEBUG")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    for (_, line) in largest.iter().take(shown) {
        eprintln!("{line}");
    }
    println!("# Stroke prototype: new coverage against today's\n");
    println!(
        "Today's CPU renderer against the reference, white strokes, coverage recovered from the green channel (8-bit sRGB, so ±1 step is quantization). Cases at command scale 1 without a clip; tile size {tile}{}.\n",
        if quick() { ", quick corpus" } else { "" }
    );
    println!("| Shape | Pixels | ≤1 | ≤2 | ≤4 | ≤16 | ≤64 | >64 | Largest |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for (shape, counts, largest, pixels) in by_shape {
        let share = |n: u64| format!("{:.2} %", 100.0 * n as f64 / pixels.max(1) as f64);
        println!(
            "| {shape} | {pixels} | {} | {} | {} | {} | {} | {} | {largest:.0} |",
            share(counts[0]),
            share(counts[1]),
            share(counts[2]),
            share(counts[3]),
            share(counts[4]),
            share(counts[5]),
        );
    }
    Ok(())
}

fn summary(samples: &mut [Duration]) -> String {
    if samples.is_empty() {
        return "–".into();
    }
    samples.sort_unstable();
    let median = samples[samples.len() / 2];
    let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
    format!(
        "{:.3} / {:.3}",
        median.as_secs_f64() * 1e3,
        p95.as_secs_f64() * 1e3
    )
}

/// Per-frame cost of each workload, new and today's, in the real-time
/// probe's intervals. Milliseconds, median / 95th percentile.
fn time() -> Result<()> {
    let tile = tile_size();
    let path = gpu::Draw::from_env()?;
    let tiling = if path == gpu::Draw::Quads {
        Tiling::Cells
    } else {
        Tiling::Lists
    };
    let (warmup, frames) = if quick() { (3, 20) } else { (12, 300) };
    println!("# Stroke prototype: timing\n");
    let mut header = false;
    for scale in [1.0f64, 2.0] {
        let frame = Frame {
            width: (f64::from(workloads::LOGICAL[0]) * scale) as u32,
            height: (f64::from(workloads::LOGICAL[1]) * scale) as u32,
            scale,
        };
        let mut gpu = gpu::Gpu::new(frame.width, frame.height)?;
        if !header {
            println!(
                "Adapter: {}. Path {path:?}. Tile size {tile}. {warmup} warm-up and {frames} measured frames per workload. Prepare = preparation ({tiling:?}), encoding and upload; GPU = encode, submit and wait for the frame's offscreen passes, measured on the host; GPU execution = the passes' own begin-to-end timestamps; atlas and composite = the quads path's share of it, its atlas passes and its last composite pass (the whole frame with one batch); today's path runs in the same frames, so both share the GPU's clock; MHz = the GPU clock after the new passes, median; CPU = preparation with lists, per-pixel evaluation and blending into a linear frame; today's CPU = the CPU renderer's frame with the strokes minus without them. Presentation is not measured.\n",
                gpu.adapter
            );
            println!(
                "| Workload | Scale | Segments | References | Tiles | Longest list | Batches | Bytes | Prepare | GPU | GPU execution | Atlas | Composite | CPU | Today: rasterize | Today: prepare | Today: GPU | Today: GPU execution | Today: CPU | MHz |"
            );
            println!("|{}", "---|".repeat(20));
            header = true;
        }
        let mut canvas = cpu::Canvas::new(frame.width, frame.height);
        let mut today = today::Today::new(frame)?;
        let empty = today.scene(&[])?;
        let mut empty_times = Vec::new();
        for _ in 0..frames.min(60) {
            let start = Instant::now();
            std::hint::black_box(today.cpu.render_opaque(&empty)?);
            empty_times.push(start.elapsed());
        }
        empty_times.sort_unstable();
        let empty_cpu = empty_times[empty_times.len() / 2];
        for workload in workloads::timed() {
            let mut prepared = Prepared::default();
            // The CPU's own preparation, with lists.
            let mut own = Prepared::default();
            let mut phases: [Vec<Duration>; 11] = Default::default();
            let mut clocks: Vec<u32> = Vec::new();
            let mut stats = stroke::Stats::default();
            let unchanged = matches!(workload, workloads::Workload::Static);
            if unchanged {
                for s in workload.strokes(0) {
                    stroke::prepare(&s, frame, tile, tiling, &mut prepared)?;
                    stroke::prepare(&s, frame, tile, Tiling::Lists, &mut own)?;
                }
                gpu.upload(&prepared);
            }
            let mut baseline_masks = false;
            // Today's path may refuse a workload; the row then says so.
            let mut refused: Option<String> = None;
            let no_cpu = std::env::var("PROBE_NO_CPU").is_ok_and(|v| v == "1");
            for n in 0..warmup + frames {
                let strokes = workload.strokes(n);
                // New: prepare and upload, unless unchanged.
                let start = Instant::now();
                if !unchanged {
                    prepared.clear();
                    stats = stroke::Stats::default();
                    for s in &strokes {
                        if let Some(s) = stroke::prepare(s, frame, tile, tiling, &mut prepared)? {
                            stats.segments += s.segments;
                            stats.references += s.references;
                            stats.tiles += s.tiles;
                            stats.longest = stats.longest.max(s.longest);
                        }
                    }
                    gpu.upload(&prepared);
                }
                let prepare = start.elapsed();
                let gpu_time = gpu.pass(path, false)?;
                let execution = gpu.last_execution()?.unwrap_or_default();
                let (atlas, composite) = gpu.last_split().unwrap_or((Duration::ZERO, execution));
                let clock = gpu::Gpu::clock();
                canvas.clear([0.0, 0.0, 0.0, 1.0]);
                let start = Instant::now();
                if !unchanged && !no_cpu {
                    own.clear();
                    for s in &strokes {
                        stroke::prepare(s, frame, tile, Tiling::Lists, &mut own)?;
                    }
                }
                if !no_cpu {
                    canvas.draw(&own);
                }
                let cpu_time = start.elapsed();
                let mut measured = vec![prepare, gpu_time, execution, atlas, composite, cpu_time];
                let skip_today = std::env::var("PROBE_NO_TODAY").is_ok_and(|v| v == "1");
                if workload.has_baseline() && refused.is_none() && !skip_today {
                    let scene = today.scene(&strokes)?;
                    let start = Instant::now();
                    let rasterized = match today.rasterizer.prepare(&scene) {
                        Ok(rasterized) => rasterized,
                        Err(error) => {
                            refused = Some(format!("refused: {error:?}"));
                            continue;
                        }
                    };
                    let rasterize = start.elapsed();
                    let start = Instant::now();
                    if !unchanged || !baseline_masks {
                        gpu.upload_masks(&today.masks(&rasterized));
                        baseline_masks = true;
                    }
                    let upload = start.elapsed();
                    let today_gpu = gpu.pass(gpu::Draw::Masks, false)?;
                    let today_execution = gpu.last_execution()?.unwrap_or_default();
                    let today_cpu = if std::env::var("PROBE_NO_TODAY_CPU").is_ok_and(|v| v == "1") {
                        Duration::ZERO
                    } else {
                        let start = Instant::now();
                        std::hint::black_box(today.cpu.render_opaque(&scene)?);
                        start.elapsed().saturating_sub(empty_cpu)
                    };
                    measured.extend([
                        if unchanged { Duration::ZERO } else { rasterize },
                        if unchanged { Duration::ZERO } else { upload },
                        today_gpu,
                        today_execution,
                        today_cpu,
                    ]);
                }
                if n >= warmup {
                    for (phase, value) in phases.iter_mut().zip(measured) {
                        phase.push(value);
                    }
                    clocks.extend(clock);
                }
            }
            if unchanged {
                stats.segments = prepared.segments.len();
                stats.references = prepared.lists.len();
                stats.tiles = prepared.tiles.len();
                stats.longest = prepared
                    .tiles
                    .iter()
                    .map(|t| t.count as usize)
                    .max()
                    .unwrap_or(0);
            }
            let [p, g, ge, qa, qc, c, r, u, tg, tge, tc] = &mut phases;
            clocks.sort_unstable();
            let clock = clocks
                .get(clocks.len() / 2)
                .map_or_else(|| "–".to_string(), u32::to_string);
            let today_columns = match &refused {
                Some(reason) => format!("{reason} | | | | | {clock}"),
                None => format!(
                    "{} | {} | {} | {} | {} | {clock}",
                    summary(r),
                    summary(u),
                    summary(tg),
                    summary(tge),
                    summary(tc)
                ),
            };
            let (atlas, composite) = if path == gpu::Draw::Quads {
                (summary(qa), summary(qc))
            } else {
                ("–".to_string(), "–".to_string())
            };
            println!(
                "| {} | {scale} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {atlas} | {composite} | {} | {today_columns} |",
                workload.name(),
                stats.segments,
                stats.references,
                stats.tiles,
                stats.longest,
                gpu.batches(),
                gpu.uploaded_bytes(),
                summary(p),
                summary(g),
                summary(ge),
                summary(c),
            );
        }
    }
    Ok(())
}
