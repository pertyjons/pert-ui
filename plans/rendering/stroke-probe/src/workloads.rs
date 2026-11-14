//! The timed workloads and the parity corpus, in logical pixels.
use crate::stroke::{Cap, Join, Stroke, Subpath};

/// The logical frame and the plotted area of the real-time probe.
pub const LOGICAL: [f32; 2] = [960.0, 640.0];
pub const PLOT: [f32; 4] = [80.0, 120.0, 800.0, 400.0];

/// The real-time probe's signal (`pert-ui-wgpu/src/realtime_probe.rs`).
pub fn sample(frame: u64, index: usize, samples: usize) -> f32 {
    let t = index as f32 / samples as f32;
    let phase = frame as f32 * 0.07;
    0.5 + 0.3 * (t * 37.0 + phase).sin()
        + 0.12 * (t * 211.0 - 2.0 * phase).sin()
        + 0.05 * (t * 1301.0 + 3.0 * phase).sin()
}

// A fixed linear congruential sequence in 0..1.
pub struct Lcg(u64);
impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }
    pub fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Workload {
    /// The real-time probe's polyline: width 1.5, round joins.
    Trace(usize),
    /// Random heights over the whole plot: long, crossing segments.
    Noise(usize),
    /// Points alternating between the plot's top and bottom: crowded tiles.
    Steep(usize),
    /// Two-point edges between moving nodes, as the diagram editor draws them.
    Diagram(usize),
    /// A knob's pointer: one short round-capped stroke.
    Knob,
    /// The trace under a view transform of this magnification.
    Zoomed(f32),
    /// The trace, unchanged between frames: records stay uploaded.
    Static,
}

impl Workload {
    pub fn name(self) -> String {
        match self {
            Self::Trace(n) => format!("trace, {n} points"),
            Self::Noise(n) => format!("noise, {n} points"),
            Self::Steep(n) => format!("steep alternation, {n} points"),
            Self::Diagram(n) => format!("diagram, {n} edges"),
            Self::Knob => "knob pointer".into(),
            Self::Zoomed(s) => format!("trace, 800 points, view scale {s}"),
            Self::Static => "trace, 800 points, unchanged".into(),
        }
    }

    /// Whether today's path can be measured through the public facade.
    pub fn has_baseline(self) -> bool {
        !matches!(self, Self::Zoomed(_))
    }

    pub fn strokes(self, frame: u64) -> Vec<Stroke> {
        let [x, y, w, h] = PLOT;
        let trace = |n: usize, frame: u64| {
            let points = (0..n)
                .map(|i| {
                    [
                        x + i as f32 * w / (n - 1) as f32,
                        y + (1.0 - sample(frame, i, n)) * h,
                    ]
                })
                .collect();
            Stroke::polyline(points, 1.5, Cap::Butt, Join::Round)
        };
        match self {
            Self::Trace(n) => vec![trace(n, frame)],
            Self::Static => vec![trace(800, 0)],
            Self::Noise(n) => {
                let mut random = Lcg::new(frame);
                let points = (0..n)
                    .map(|i| [x + i as f32 * w / (n - 1) as f32, y + random.next() * h])
                    .collect();
                vec![Stroke::polyline(points, 1.5, Cap::Butt, Join::Round)]
            }
            Self::Steep(n) => {
                let points = (0..n)
                    .map(|i| {
                        let top = (i + frame as usize).is_multiple_of(2);
                        [x + i as f32 * 2.0, if top { y } else { y + h }]
                    })
                    .collect();
                vec![Stroke::polyline(points, 1.5, Cap::Butt, Join::Round)]
            }
            Self::Diagram(n) => {
                let mut random = Lcg::new(7);
                let nodes: Vec<[f32; 2]> = (0..n.div_ceil(2) + 1)
                    .map(|i| {
                        let wobble = (frame as f32 * 0.05 + i as f32).sin() * 6.0;
                        [
                            40.0 + random.next() * (LOGICAL[0] - 80.0) + wobble,
                            40.0 + random.next() * (LOGICAL[1] - 80.0) - wobble,
                        ]
                    })
                    .collect();
                (0..n)
                    .map(|i| {
                        let a = nodes[i % nodes.len()];
                        let b = nodes[(i * 7 + 3) % nodes.len()];
                        Stroke::polyline(vec![a, b], 1.5, Cap::Butt, Join::Miter)
                    })
                    .collect()
            }
            Self::Knob => {
                let angle = (135.0 + (frame % 270) as f32).to_radians();
                let (cx, cy, r) = (480.0, 320.0, 24.0);
                let at = |f: f32| [cx + r * f * angle.cos(), cy + r * f * angle.sin()];
                vec![Stroke::polyline(
                    vec![at(0.35), at(1.0)],
                    3.0,
                    Cap::Round,
                    Join::Miter,
                )]
            }
            Self::Zoomed(scale) => {
                let mut stroke = trace(800, frame);
                let center = [x + w / 2.0, y + h / 2.0];
                stroke.scale = scale;
                stroke.origin = [center[0] * (1.0 - scale), center[1] * (1.0 - scale)];
                vec![stroke]
            }
        }
    }
}

/// The workloads timed in slice 0.
pub fn timed() -> Vec<Workload> {
    vec![
        Workload::Trace(800),
        Workload::Trace(4096),
        Workload::Noise(800),
        Workload::Steep(400),
        Workload::Noise(16384),
        Workload::Diagram(60),
        Workload::Knob,
        Workload::Zoomed(12.0),
        Workload::Static,
    ]
}

/// One parity case: a stroke in a small logical frame.
pub struct Case {
    pub name: String,
    pub stroke: Stroke,
    pub scale: f64,
}

/// The logical size of a parity case's frame.
pub const CASE_FRAME: f32 = 96.0;

/// Geometry for the corpus, in a 96-pixel logical frame.
fn shapes() -> Vec<(&'static str, Vec<Subpath>)> {
    let open = |points: Vec<[f32; 2]>| {
        vec![Subpath {
            points,
            closed: false,
        }]
    };
    let zigzag: Vec<[f32; 2]> = (0..9)
        .map(|i| [10.0 + i as f32 * 9.0, if i % 2 == 0 { 30.0 } else { 60.0 }])
        .collect();
    let wave: Vec<[f32; 2]> = (0..60)
        .map(|i| {
            let t = i as f32;
            [
                8.0 + t * 1.35,
                48.0 + 25.0 * (t * 0.37).sin() + 6.0 * (t * 2.9).cos(),
            ]
        })
        .collect();
    vec![
        ("zigzag", open(zigzag)),
        ("wave", open(wave)),
        (
            "sharp turns",
            open(vec![
                [10.0, 80.0],
                [48.0, 12.0],
                [52.0, 80.0],
                [86.0, 30.0],
                [20.0, 34.0],
            ]),
        ),
        (
            "closed",
            vec![Subpath {
                points: vec![[16.0, 16.0], [80.0, 22.0], [60.0, 78.0], [24.0, 64.0]],
                closed: true,
            }],
        ),
        (
            "reversal",
            open(vec![[20.0, 48.0], [70.0, 48.0], [40.0, 48.0]]),
        ),
        (
            "straight continuation",
            open(vec![[12.0, 20.0], [40.0, 40.0], [68.0, 60.0], [84.0, 72.0]]),
        ),
        (
            "short segments",
            open(vec![
                [20.0, 50.0],
                [20.3, 50.1],
                [20.5, 50.6],
                [50.0, 70.0],
                [50.2, 69.6],
                [80.0, 30.0],
            ]),
        ),
        (
            "points",
            vec![
                Subpath {
                    points: vec![[30.0, 30.0]],
                    closed: false,
                },
                Subpath {
                    points: vec![[60.3, 61.7]],
                    closed: false,
                },
            ],
        ),
        (
            "far geometry",
            open(vec![
                [-60_000.0, -20_000.0],
                [40.0, 50.0],
                [60_000.0, 30_000.0],
            ]),
        ),
    ]
}

/// The parity corpus. `quick` keeps one phase, width and scale per shape.
pub fn corpus(quick: bool) -> Vec<Case> {
    let joins = [Join::Round, Join::Bevel, Join::Miter];
    let caps = [Cap::Butt, Cap::Round, Cap::Square];
    let widths: &[f32] = if quick {
        &[3.0]
    } else {
        &[0.25, 0.75, 1.5, 3.0, 12.0]
    };
    let scales: &[f64] = if quick { &[1.25] } else { &[1.0, 1.25, 2.0] };
    let phases: &[[f32; 2]] = if quick {
        &[[0.3, 0.7]]
    } else {
        &[[0.0, 0.0], [0.3, 0.71]]
    };
    let mut cases = Vec::new();
    for (name, subpaths) in shapes() {
        for &join in &joins {
            for &cap in &caps {
                for &width in widths {
                    for &scale in scales {
                        for &phase in phases {
                            let subpaths = subpaths
                                .iter()
                                .map(|s| Subpath {
                                    points: s
                                        .points
                                        .iter()
                                        .map(|p| [p[0] + phase[0], p[1] + phase[1]])
                                        .collect(),
                                    closed: s.closed,
                                })
                                .collect();
                            cases.push(Case {
                                name: format!("{name}, {join:?}, {cap:?}, width {width}, scale {scale}, phase {phase:?}"),
                                stroke: Stroke {
                                    subpaths,
                                    width,
                                    cap,
                                    join,
                                    miter_limit: if name == "sharp turns" { 40.0 } else { 4.0 },
                                    color: [1.0; 4],
                                    origin: [0.0; 2],
                                    scale: 1.0,
                                    clip: None,
                                },
                                scale,
                            });
                        }
                    }
                }
            }
        }
    }
    // A view scale above eight, and a clip that cuts segments.
    for (scale, clip) in [(12.0, None), (1.0, Some([20.3, 15.6, 41.2, 50.9]))] {
        for join in joins {
            let mut stroke = Stroke::polyline(
                (0..30)
                    .map(|i| [10.0 + i as f32 * 2.5, 48.0 + 18.0 * (i as f32 * 0.8).sin()])
                    .collect(),
                if scale > 1.0 { 0.4 } else { 2.0 },
                Cap::Round,
                join,
            );
            stroke.scale = scale;
            stroke.origin = [48.0 * (1.0 - scale), 48.0 * (1.0 - scale)];
            stroke.clip = clip;
            cases.push(Case {
                name: format!("view scale {scale}, clip {clip:?}, {join:?}"),
                stroke,
                scale: 1.25,
            });
        }
    }
    // A round cap whose disc reaches behind its short segment's joined end,
    // and a miter spike from a near reversal far outside the frame.
    cases.push(Case {
        name: "short round-capped end".into(),
        stroke: Stroke::polyline(
            vec![[10.0, 30.0], [10.0, 10.0], [10.125, 10.0]],
            8.0,
            Cap::Round,
            Join::Bevel,
        ),
        scale: 1.0,
    });
    let mut spike = Stroke::polyline(
        vec![[-26960.0, 10.9], [-26950.0, 10.9], [-27950.0, 11.15]],
        8.0,
        Cap::Butt,
        Join::Miter,
    );
    spike.miter_limit = 10000.0;
    cases.push(Case {
        name: "miter spike from a near reversal".into(),
        stroke: spike,
        scale: 1.0,
    });
    cases
}
