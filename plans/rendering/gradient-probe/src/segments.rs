// Compare a candidate segmented ramp to direct interpolation on a finite
// grid. This can disqualify a representation; sampling cannot prove a bound.
use super::*;
fn linear(t: f64, source: &[Stop], space: u32) -> Stop {
    let c = ramp(t, source, space);
    stop(t as f32, c, 0)
}
fn error(t: f64, source: &[Stop], space: u32, candidate: &[Stop]) -> f64 {
    let a = ramp(t, source, space);
    let b = ramp(t, candidate, 0);
    let mut worst = (a[3] - b[3]).abs();
    for bg in [0., 1.] {
        for ch in 0..3 {
            let composite = |v: [f64; 4]| encode(v[ch] * v[3] + bg * (1. - v[3]));
            worst = worst.max((composite(a) - composite(b)).abs());
        }
    }
    worst * 255.
}
pub(super) fn experiment(g: &Gpu, checks: &mut Check) -> Result<()> {
    for (label, source) in [
        (
            "two-stop",
            vec![stop(0., gray(0), 2), stop(1., gray(255), 2)],
        ),
        (
            "alternating",
            (0..16)
                .map(|i| stop(i as f32 / 15., gray(if i % 2 == 0 { 0 } else { 255 }), 2))
                .collect(),
        ),
    ] {
        let mut nodes: Vec<_> = source
            .iter()
            .map(|s| linear(s.data[0].into(), &source, 2))
            .collect();
        loop {
            let mut worst = (0., 0usize, 0.);
            for (i, pair) in nodes.windows(2).enumerate() {
                let a = f64::from(pair[0].data[0]);
                let b = f64::from(pair[1].data[0]);
                for j in 1..16 {
                    let t = a + (b - a) * f64::from(j) / 16.;
                    let err = error(t, &source, 2, pair);
                    if err > worst.0 {
                        worst = (err, i, t);
                    }
                }
            }
            if worst.0 <= 0.49 || nodes.len() >= 65 {
                break;
            }
            nodes.insert(worst.1 + 1, linear(worst.2, &source, 2));
        }
        let worst = (0..=65536)
            .map(|i| error(f64::from(i) / 65536., &source, 2, &nodes))
            .fold(0., f64::max);
        println!(
            "segments {label} count={} max sampled pre-dither error={worst:.4} encoded bytes",
            nodes.len() - 1
        );
        let p = base(1024, 64, 0);
        checks.compare(g, label, p, &nodes, 0., 1024)?;
        let mut direct = p;
        direct.mode[2] = 2;
        checks.compare(g, "direct-OKLab", direct, &source, 0., 1024)?;
        if label == "two-stop" {
            bench(g, "segmented", p, &nodes, 0)?;
            bench(g, "direct-Oklab", direct, &source, 0)?;
        }
    }
    Ok(())
}
