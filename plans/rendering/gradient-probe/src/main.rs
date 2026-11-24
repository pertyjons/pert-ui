//! Offscreen phase 2 experiment. Run the release binary as described in README.
mod gpu;
mod sample;
mod seams;
mod segments;
#[allow(dead_code)]
#[rustfmt::skip]
#[path = "../../vello-spike/src/workloads.rs"]
mod workloads;
use gpu::Gpu;
use sample::{Params, Stop, encode, ramp, sample, stop};
use std::error::Error;
use std::time::Instant;
type Result<T> = std::result::Result<T, Box<dyn Error>>;

const DARK: [f64; 4] = [
    0.014443843596092545,
    0.014443843596092545,
    0.014443843596092545,
    1.,
]; // sRGB8 32
const LIGHT: [f64; 4] = [
    0.030713443732993635,
    0.030713443732993635,
    0.030713443732993635,
    1.,
]; // 49
fn base(w: u32, h: u32, space: u32) -> Params {
    Params {
        geom: [0., 0., w as f32, 0.],
        misc: [0.25, 0., 360., 0.],
        mode: [0, 0, space, 2],
        options: [0, 0, w, h],
    }
}
fn tile_error(bytes: &[u8]) -> f64 {
    let mut sum = 0.;
    for y in (0..64).step_by(8) {
        for x in (0..1024).step_by(8) {
            let mut actual = 0.;
            for row in y..y + 8 {
                for col in x..x + 8 {
                    actual += f64::from(bytes[(row * 1024 + col) * 4]);
                }
            }
            let ideal = 32. + 17. * (x as f64 + 4.) / 1024.;
            sum += (actual / 64. - ideal).powi(2);
        }
    }
    (sum / 1024.).sqrt()
}
fn gray(byte: u8) -> [f64; 4] {
    let e = f64::from(byte) / 255.;
    let v = if e < 0.04045 {
        e / 12.92
    } else {
        ((e + 0.055) / 1.055).powf(2.4)
    };
    [v, v, v, 1.]
}
struct Check {
    errors: usize,
}
impl Check {
    fn compare(
        &mut self,
        g: &Gpu,
        name: &str,
        mut p: Params,
        s: &[Stop],
        bg: f64,
        clip: u32,
    ) -> Result<Vec<u8>> {
        p.mode[3] = s.len() as u32;
        let [_, _, w, h] = p.options;
        let f = g.frame(w, h);
        g.params(&p);
        g.stops(s);
        g.draw(&f, bg, clip);
        let bytes = g.read(&f)?;
        let mut worst = [0u8; 2];
        let mut loc = [(0, 0, 0); 2];
        let mut outside = [0; 2];
        for y in 0..h {
            for x in 0..w {
                let offset = ((y * w + x) * 4) as usize;
                let value = (encode(bg.max(0.)) * 255.).round() as u8;
                let background = [value, value, value, if bg < 0. { 0 } else { 255 }];
                let reference = if x < clip {
                    sample::<f64>(
                        &Params {
                            options: [p.options[0].min(1), p.options[1], w, h],
                            ..p
                        },
                        s,
                        x,
                        y,
                        bg,
                    )
                } else {
                    background
                };
                let cpu = if x < clip {
                    sample::<f32>(&p, s, x, y, bg)
                } else {
                    background
                };
                for ch in 0..4 {
                    for (i, d) in [
                        reference[ch].abs_diff(cpu[ch]),
                        cpu[ch].abs_diff(bytes[offset + ch]),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if d > worst[i] {
                            worst[i] = d;
                            loc[i] = (x, y, ch);
                        }
                        if d > 1 {
                            outside[i] += 1;
                        }
                    }
                }
            }
        }
        println!(
            "check {name:<24} {w}x{h} bg={bg} worst(ref,cpu/gpu)={worst:?} locations={loc:?} channels>1={outside:?}"
        );
        self.errors += outside.iter().sum::<usize>();
        Ok(bytes)
    }
}

fn banding(check: &mut Check, g: &Gpu) -> Result<()> {
    let mut p = base(1024, 64, 1);
    let s = [stop(0., DARK, 1), stop(1., LIGHT, 1)];
    let off = check.compare(g, "dark-undithered", p, &s, 0., 1024)?;
    p.options[0] = 1;
    let on = check.compare(g, "dark-dithered", p, &s, 0., 1024)?;
    p.options[0] = 2;
    check.compare(g, "dark-table", p, &s, 0., 1024)?;
    p.options[0] = 1;
    let repeat = check.compare(g, "dark-repeated", p, &s, 0., 1024)?;
    let (before, after) = (tile_error(&off), tile_error(&on));
    println!(
        "banding tile RMS {before:.5} -> {after:.5}; ratio {:.4}; capture identical {}",
        after / before,
        on == repeat
    );
    if after > before * 0.5 || on != repeat {
        check.errors += 1;
    }
    if let Some(dir) = std::env::var_os("PROBE_CAPTURES") {
        for (file, bytes) in [("undithered.ppm", &off), ("dithered.ppm", &on)] {
            let mut ppm = b"P6\n1024 64\n255\n".to_vec();
            for pixel in bytes.as_chunks::<4>().0 {
                ppm.extend_from_slice(&pixel[..3]);
            }
            std::fs::write(std::path::Path::new(&dir).join(file), ppm)?;
        }
    }
    Ok(())
}
fn corpus(check: &mut Check, g: &Gpu) -> Result<()> {
    for scale in [1., 1.25, 2.] {
        let width = (320. * scale) as u32;
        let mut p = base(width, (16. * scale) as u32, 1);
        p.geom[2] = 40. * scale;
        let s = [stop(0., gray(160), 1), stop(1., gray(75), 1)];
        let full = check.compare(g, "title", p, &s, 0., width)?;
        let end = (40. * scale) as usize;
        if full
            .as_chunks::<4>()
            .0
            .iter()
            .take(width as usize)
            .skip(end)
            .any(|c| c[0].abs_diff(75) > 1)
        {
            check.errors += 1;
        }
        check.compare(g, "title-overlay", p, &s, 0., end as u32)?;
        p.options[2] = end as u32 / 2;
        let wide = Params {
            options: [0, 0, width * 2, p.options[3]],
            ..p
        };
        let resized = check.compare(g, "title-expanded", wide, &s, 0., width * 2)?;
        if resized[..width as usize * 4] != full[..width as usize * 4] {
            check.errors += 1;
        }
        let narrow = check.compare(g, "title-resized", p, &s, 0., width)?;
        if narrow[..4 * p.options[2] as usize] != full[..4 * p.options[2] as usize] {
            check.errors += 1;
        }
        for kind in 0..3 {
            let mut p = base((93. * scale) as u32, (61. * scale) as u32, 2);
            p.mode[0] = kind;
            p.geom = [32.5 * scale, 24.5 * scale, 27.5 * scale, 15.5 * scale];
            p.misc[2] = 270.;
            let s = [
                stop(0., [0.8, 0.01, 0.5, 0.12], 2),
                stop(0.37, [0.01, 1., 0.05, 0.63], 2),
                stop(1., [0.1, 0.02, 0.9, 0.9], 2),
            ];
            for (ext, bg) in [(0, 0.), (1, 1.), (2, -1.)] {
                p.mode[1] = ext;
                p.options[0] = ext % 2;
                check.compare(
                    g,
                    &format!("geom{kind}-extend{ext}"),
                    p,
                    &s,
                    bg,
                    p.options[2],
                )?;
            }
        }
    }
    for space in 0..3 {
        let s = [
            stop(0., [1., 0., 0., 0.], space),
            stop(1., [0., 0., 1., 1.], space),
        ];
        let mut p = base(257, 8, space);
        p.options[1] = 3;
        check.compare(g, "mixed-border", p, &s, -1., 257)?;
        p.options[1] = 0;
        p.options[0] = 1;
        for bg in [0., 1., -1.] {
            check.compare(g, "alpha", p, &s, bg, 257)?;
        }
    }
    let mut scrim = base(1024, 64, 0);
    let s = [stop(0., [0., 0., 0., 0.], 0), stop(1., [0., 0., 0., 1.], 0)];
    for bg in [gray(32)[0], 1.] {
        let off = check.compare(g, "alpha-scrim", scrim, &s, bg, 1024)?;
        scrim.options[0] = 1;
        let on = check.compare(g, "alpha-scrim-dither", scrim, &s, bg, 1024)?;
        let changed = on.iter().zip(&off).filter(|(a, b)| a != b).count();
        println!("black scrim bg={bg}: {changed} changed channel bytes with RGB dither");
        scrim.options[0] = 0;
    }
    // Translate and zoom the component about a large logical origin before
    // narrowing the frame-relative coefficients. The two captures coincide.
    let s = [stop(0., gray(9), 0), stop(1., gray(180), 0)];
    let mut p = base(257, 8, 0);
    let zoom = 1.75;
    let scale = 1.25;
    let pos = 3.25;
    p.geom = [
        (pos * zoom * scale) as f32,
        0.,
        (40. * zoom * scale) as f32,
        0.,
    ];
    let local = check.compare(g, "zoom-local", p, &s, 0., 257)?;
    let origin = 2f64.powi(35);
    let project = |point: f64| (point - origin) * zoom * scale;
    let from = project(origin + pos);
    let to = project(origin + pos + 40.);
    p.geom = [from as f32, 0., (to - from) as f32, 0.];
    let shifted = check.compare(g, "zoom-rebased", p, &s, 0., 257)?;
    if local != shifted {
        check.errors += 1;
    }
    for space in 0..3 {
        let tiny = [
            stop(0., [0.7, 0.02, 1., 1e-16], space),
            stop(1., [0.01, 0.8, 0.15, 0.00001], space),
        ];
        let mut p = base(65, 3, space);
        p.options[0] = 2;
        check.compare(g, "near-transparent-table", p, &tiny, -1., 65)?;
    }
    for delta in [-0.0001, 0., 0.0001] {
        let mut p = base(129, 65, 0);
        p.geom = [0.5 + delta, 0.5, 128., 0.];
        let s = [
            stop(0., gray(0), 0),
            stop(0.5, gray(0), 0),
            stop(0.5, gray(255), 0),
            stop(1., gray(255), 0),
        ];
        for kind in 0..3 {
            p.mode[0] = kind;
            p.mode[1] = 1;
            if kind > 0 {
                p.geom = [64.5 + delta, 32.5, 32., 32.];
                p.misc[0] = 0.;
            }
            check.compare(g, &format!("hard{kind}/{delta}"), p, &s, 0., 129)?;
        }
    }
    Ok(())
}

fn bench(g: &Gpu, name: &str, mut p: Params, s: &[Stop], updates: u32) -> Result<()> {
    p.mode[3] = s.len() as u32;
    let [_, work, w, h] = p.options;
    let f = g.frame(w, h);
    g.params(&p);
    g.stops(s);
    let image: Vec<u8> = (0..256)
        .flat_map(|i| {
            let c = ramp::<f64>(f64::from(i) / 255., s, p.mode[2]);
            [0, 1, 2, 3].map(|j| ((if j == 3 { c[j] } else { encode(c[j]) }) * 255.).round() as u8)
        })
        .collect();
    let clips = |frame: u64| -> Vec<[u32; 4]> {
        if work == 1 || work == 2 {
            workloads::bars(frame)
                .into_iter()
                .map(|(_, r)| [r.x as u32, r.y as u32, r.w as u32, r.h as u32])
                .collect()
        } else {
            vec![[0, 0, w, h]]
        }
    };
    let retained = clips(0);
    g.image(&image);
    g.draw(&f, 0., w);
    g.wait()?;
    let mut gpu = Vec::new();
    let mut cpu = Vec::new();
    let mut changed = s.to_vec();
    let mut output = vec![[0u8; 4]; (w * h) as usize];
    for _ in 0..11 {
        let now = Instant::now();
        for i in 0..8 {
            if updates != 0 {
                p.geom[0] = (i % 2) as f32;
                if updates == 2 {
                    changed[0].color[0] = s[0].color[0] * (0.99 + 0.01 * (i % 2) as f32);
                }
                g.params(&p);
                if updates == 2 {
                    g.stops(&changed);
                }
            }
            if updates != 0 {
                g.draw_rects(&f, 0., &clips(i));
            } else {
                g.draw_rects(&f, 0., &retained);
            }
        }
        g.wait()?;
        gpu.push(now.elapsed().as_secs_f64() * 1000. / 8.);
        let now = Instant::now();
        for &[left, top, width, height] in &retained {
            for y in top..top + height {
                for x in left..left + width {
                    let pixel = if work == 2 {
                        let t =
                            (1. - (y as f32 + 0.5 - p.geom[1]) / p.geom[3]).clamp(0., 1.) * 255.;
                        let lo = t.floor() as usize;
                        let hi = (lo + 1).min(255);
                        let mut c = [255; 4];
                        for ch in 0..3 {
                            let a = sample::decode(f32::from(image[4 * lo + ch]) / 255.);
                            let b = sample::decode(f32::from(image[4 * hi + ch]) / 255.);
                            c[ch] = (encode(a + (b - a) * t.fract()) * 255.).round() as u8;
                        }
                        c
                    } else {
                        sample::<f32>(&p, &changed, x, y, 0.)
                    };
                    output[(y * w + x) as usize] = pixel;
                }
            }
        }
        std::hint::black_box(&output);
        cpu.push(now.elapsed().as_secs_f64() * 1000.);
    }
    if work != 2 && updates == 0 {
        // Reuse the horizontal row every eight rows with ordered dither;
        // a vertical meter repeats its samples every eight columns.
        let period = if p.options[0] == 0 { 1 } else { 8 };
        let mut cached = vec![[0u8; 4]; output.len()];
        let mut times = Vec::new();
        for _ in 0..11 {
            let now = Instant::now();
            if work == 1 {
                for y in 0..h {
                    let row: [_; 8] =
                        std::array::from_fn(|i| sample::<f32>(&p, s, i as u32 % period, y, 0.));
                    for &[left, top, width, height] in &retained {
                        if (top..top + height).contains(&y) {
                            for x in left..left + width {
                                cached[(y * w + x) as usize] = row[(x % period) as usize];
                            }
                        }
                    }
                }
            } else {
                for y in 0..h {
                    let start = (y * w) as usize;
                    if y < period {
                        for x in 0..w {
                            cached[start + x as usize] = sample::<f32>(&p, s, x, y, 0.);
                        }
                    } else {
                        let source = ((y % period) * w) as usize;
                        cached.copy_within(source..source + w as usize, start);
                    }
                }
            }
            std::hint::black_box(&cached);
            times.push(now.elapsed().as_secs_f64() * 1000.);
        }
        if cached != output {
            return Err(format!("row reuse changed {name}").into());
        }
        times.sort_by(f64::total_cmp);
        println!(
            "row reuse {name} CPU ms {:.4} ({:.4}..{:.4})",
            times[5], times[0], times[10]
        );
    }
    gpu.sort_by(f64::total_cmp);
    cpu.sort_by(f64::total_cmp);
    println!(
        "bench {name:<20} GPU ms median {:.4} range {:.4}..{:.4} CPU ms median {:?} range {:?}..{:?} recurrent upload {}/frame",
        gpu[5],
        gpu[0],
        gpu[10],
        cpu.get(5),
        cpu.first(),
        cpu.last(),
        match updates {
            1 => 64,
            2 => 64 + 32 * s.len(),
            _ => 0,
        }
    );
    Ok(())
}
fn main() -> Result<()> {
    let g = Gpu::new()?;
    let mut check = Check { errors: 0 };
    banding(&mut check, &g)?;
    corpus(&mut check, &g)?;
    seams::experiment(&g, &mut check)?;
    segments::experiment(&g, &mut check)?;
    for (name, mut p, colors) in [
        ("panel", base(1024, 64, 1), vec![DARK, LIGHT]),
        (
            "alpha-strip",
            base(320, 24, 0),
            vec![[0.4, 0.2, 0.6, 0.], [0.4, 0.2, 0.6, 1.]],
        ),
        (
            "bars",
            base(workloads::LOGICAL.0, workloads::LOGICAL.1, 1),
            workloads::STOPS
                .iter()
                .map(|(_, c)| [gray(c[0])[0], gray(c[1])[0], gray(c[2])[0], 1.])
                .collect(),
        ),
    ] {
        let s: Vec<_> = colors
            .iter()
            .enumerate()
            .map(|(i, c)| {
                stop(
                    if name == "bars" {
                        workloads::STOPS[i].0
                    } else {
                        i as f32 / (colors.len() - 1) as f32
                    },
                    *c,
                    p.mode[2],
                )
            })
            .collect();
        if name == "bars" {
            p.options[1] = 1;
            p.geom = [128., 100., 0., 600.];
        }
        bench(&g, name, p, &s, 0)?;
        p.options[0] = 1;
        bench(&g, &format!("{name}-dither"), p, &s, 0)?;
        p.options[0] = 2;
        bench(&g, &format!("{name}-table"), p, &s, 0)?;
        if name == "bars" {
            p.options[0] = 0;
            bench(&g, "bars-geometry", p, &s, 1)?;
            bench(&g, "bars-changing", p, &s, 2)?;
            p.options[1] = 2;
            bench(&g, "bars-texture", p, &s, 0)?;
        }
    }
    println!("total failing comparisons: {}", check.errors);
    if check.errors > 0 {
        return Err("parity or banding gate failed".into());
    }
    Ok(())
}
