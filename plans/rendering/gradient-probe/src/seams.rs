// Stress a full sweep with a black/white discontinuity at half a turn.
// Measure misclassified samples by geometric distance to the separating ray.
use super::*;
pub(super) fn experiment(g: &Gpu, checks: &mut Check) -> Result<()> {
    let s = [
        stop(0., gray(0), 0),
        stop(0.5, gray(0), 0),
        stop(0.5, gray(255), 0),
        stop(1., gray(255), 0),
    ];
    let (w, h) = (257, 129);
    let frame = g.frame(w, h);
    g.stops(&s);
    for signed in [false, true] {
        let mut flips = 0;
        let mut max_width = 0f64;
        let mut max_phase = 0f64;
        let mut ties = 0;
        for origin in [
            [128.5, 64.5],
            [128.501, 64.499],
            [128.49, 64.51],
            [-32768.5, -32768.5],
            // Start a 30-degree ray within a fraction of a pixel of (128,64).
            // Vary distance to the origin and the sign of the separation.
            [128.5 - 32768., 64.5 - (32768. / 3f64.sqrt()) as f32 + 0.002],
            [128.5 - 32768., 64.5 - (32768. / 3f64.sqrt()) as f32 - 0.002],
            [128.5 - 512., 64.5 - (512. / 3f64.sqrt()) as f32],
            [128.5 - 8., 64.5 - (8. / 3f64.sqrt()) as f32],
        ] {
            for degrees in [0f32, 30., 45., 89.999, 135., 180., 270.] {
                let theta = f64::from(degrees).to_radians();
                let (mut sin, mut cos) = theta.sin_cos();
                // Force exact cardinal and diagonal ray symmetries in the oracle.
                match degrees {
                    0. => {
                        sin = 0.;
                        cos = 1.;
                    }
                    45. => sin = cos,
                    135. => cos = -sin,
                    180. => {
                        sin = 0.;
                        cos = -1.;
                    }
                    270. => {
                        sin = -1.;
                        cos = 0.;
                    }
                    _ => {}
                }
                let mut p = base(w, h, 0);
                p.geom = [origin[0], origin[1], 100., 100.];
                p.mode = [2, 0, 0, 4];
                p.misc = [0., degrees, 360., 0.];
                if signed {
                    p.options[1] = 4;
                    // Scale by the dominant component. The 45-degree products
                    // then multiply by exactly one even under GPU contraction.
                    let norm = cos.abs().max(sin.abs());
                    p.misc[2] = (cos / norm) as f32;
                    p.misc[3] = (sin / norm) as f32;
                }
                g.params(&p);
                g.draw(&frame, 0., w);
                let image = g.read(&frame)?;
                for y in 0..h {
                    for x in 0..w {
                        let dx = f64::from(x) + 0.5 - f64::from(origin[0]);
                        let dy = f64::from(y) + 0.5 - f64::from(origin[1]);
                        let side = cos * dy - sin * dx;
                        let end = side < 0. || (side == 0. && dx * cos + dy * sin < 0.);
                        if image[((y * w + x) * 4) as usize] != if end { 255 } else { 0 } {
                            flips += 1;
                            max_width = max_width.max(side.abs());
                            if side == 0. {
                                ties += 1;
                            }
                            if dx != 0. || dy != 0. {
                                max_phase = max_phase.max(
                                    (side.abs() / dx.hypot(dy)).min(1.).asin()
                                        / std::f64::consts::TAU,
                                );
                            }
                        }
                    }
                }
            }
        }
        println!(
            "sweep signed={signed} opposite-side pixels {flips} including {ties} exact ties; maximum distance {max_width:.9} physical px / {max_phase:.10} turns"
        );
        if signed && (max_width > 0.5 || ties != 0) {
            checks.errors += 1;
        }
    }
    Ok(())
}
