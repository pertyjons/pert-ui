//! The workloads, described once in logical pixels and drawn by every
//! renderer from the same data. All rectangle edges lie on integer logical
//! coordinates, so they fall on physical pixel boundaries at scale 1 and 2
//! and the renderers' different rectangle rules (pert-ui covers pixel
//! centers, vello computes area coverage) agree on them.

/// The logical window every workload is drawn into.
pub const LOGICAL: (u32, u32) = (1280, 800);

/// Straight-alpha sRGB-encoded RGBA8.
pub type Rgba = [u8; 4];

pub const BACKGROUND: Rgba = [18, 22, 30, 255];

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub rect: Rect,
    pub radius: f32,
    pub fill: Rgba,
    pub border: Rgba,
}

/// One paint step of the panels workload, in drawing order.
#[derive(Clone, Copy, Debug)]
pub enum Item {
    /// A rounded rectangle filled, then stroked inward with a 1 px border.
    Panel(Panel),
    /// A plain filled rectangle.
    Plain(Rect, Rgba),
}

/// A small deterministic generator, so every run draws the same scene.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) as u32
    }
    fn range(&mut self, low: u32, high: u32) -> u32 {
        low + self.next() % (high - low + 1)
    }
}

/// `panels` rounded, bordered panels in a grid over the window, and `plain`
/// plain rectangles placed as content inside the first panels.
pub fn panels(panels: usize, plain: usize) -> Vec<Item> {
    let mut rng = Lcg(0x5eed);
    let columns = (panels as f32 * LOGICAL.0 as f32 / LOGICAL.1 as f32).sqrt().ceil() as u32;
    let rows = (panels as u32).div_ceil(columns);
    let (cell_w, cell_h) = (LOGICAL.0 / columns, LOGICAL.1 / rows);
    let fills = [[30, 36, 48, 255], [36, 44, 60, 255], [24, 52, 88, 255], [48, 40, 64, 255]];
    let borders = [[90, 104, 130, 255], [70, 140, 220, 255], [200, 160, 90, 255]];
    let contents = [[220, 226, 238, 255], [80, 200, 140, 255], [230, 90, 80, 255], [120, 160, 255, 255]];
    let mut items = Vec::new();
    let mut plain_left = plain;
    for index in 0..panels as u32 {
        let (column, row) = (index % columns, index / columns);
        let w = rng.range(cell_w / 2, cell_w - 6);
        let h = rng.range(cell_h / 2, cell_h - 6);
        let x = column * cell_w + rng.range(2, cell_w - w - 2);
        let y = row * cell_h + rng.range(2, cell_h - h - 2);
        let rect = Rect { x: x as f32, y: y as f32, w: w as f32, h: h as f32 };
        items.push(Item::Panel(Panel {
            rect,
            radius: rng.range(4, 8) as f32,
            fill: fills[rng.next() as usize % fills.len()],
            border: borders[rng.next() as usize % borders.len()],
        }));
        // Content rectangles: spread the plain budget over the panels.
        let remaining_panels = panels as u32 - index;
        let here = (plain_left as u32).div_ceil(remaining_panels).min(3);
        for k in 0..here {
            let cw = rng.range(w / 4, w * 2 / 3).max(4);
            let ch = rng.range(3, 10).min(h / 4).max(2);
            let cy = y + 8 + k * (h / 4);
            if cy + ch + 4 > y + h {
                break;
            }
            items.push(Item::Plain(
                Rect { x: (x + 8) as f32, y: cy as f32, w: cw.min(w - 16) as f32, h: ch as f32 },
                contents[rng.next() as usize % contents.len()],
            ));
            plain_left -= 1;
        }
    }
    // Any budget left (small panels) goes into a status bar strip.
    for k in 0..plain_left {
        items.push(Item::Plain(
            Rect { x: (4 + 12 * k) as f32, y: (LOGICAL.1 - 6) as f32, w: 8.0, h: 4.0 },
            contents[k % contents.len()],
        ));
    }
    items
}

/// The plotted area of the polyline workload.
pub const PLOT: Rect = Rect { x: 240.0, y: 200.0, w: 800.0, h: 400.0 };
pub const LINE: Rgba = [100, 170, 255, 255];
pub const LINE_WIDTH: f32 = 1.5;

/// The deterministic signal of `crates/pert-ui-wgpu/src/realtime_probe.rs`.
pub fn sample(frame: u64, index: usize, samples: usize) -> f32 {
    let t = index as f32 / samples as f32;
    let phase = frame as f32 * 0.07;
    0.5 + 0.3 * (t * 37.0 + phase).sin()
        + 0.12 * (t * 211.0 - 2.0 * phase).sin()
        + 0.05 * (t * 1301.0 + 3.0 * phase).sin()
}

/// The polyline's points in logical pixels, identical for every renderer.
pub fn polyline(frame: u64, samples: usize) -> Vec<(f32, f32)> {
    (0..samples)
        .map(|i| {
            (
                PLOT.x + i as f32 * PLOT.w / (samples - 1) as f32,
                PLOT.y + (1.0 - sample(frame, i, samples)) * PLOT.h,
            )
        })
        .collect()
}

/// Meter geometry: 32 bars of `BAR_W` with `BAR_GAP` between them.
pub const BARS: usize = 32;
pub const BAR_W: f32 = 24.0;
pub const BAR_GAP: f32 = 8.0;
pub const METER: Rect = Rect { x: 128.0, y: 100.0, w: 1024.0, h: 600.0 };
pub const TRACK: Rgba = [32, 36, 44, 255];
/// The 3-stop gradient from the bottom (offset 0) to the top (offset 1),
/// interpolated in sRGB-encoded components (peniko's default).
pub const STOPS: [(f32, Rgba); 3] = [
    (0.0, [20, 190, 90, 255]),
    (0.7, [235, 205, 20, 255]),
    (1.0, [235, 45, 35, 255]),
];

/// Each bar's full-height rectangle and its lit part this frame. The lit
/// height is whole logical pixels, so its top edge is pixel aligned.
pub fn bars(frame: u64) -> Vec<(Rect, Rect)> {
    (0..BARS)
        .map(|i| {
            let x = METER.x + i as f32 * (BAR_W + BAR_GAP);
            let level = 0.5 + 0.45 * (frame as f32 * 0.05 + i as f32 * 0.4).sin();
            let lit = (level * METER.h).round();
            (
                Rect { x, y: METER.y, w: BAR_W, h: METER.h },
                Rect { x, y: METER.y + METER.h - lit, w: BAR_W, h: lit },
            )
        })
        .collect()
}

/// The gradient's sRGB8 color at `t` (0 at the bottom, 1 at the top).
pub fn gradient_at(t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    let k = if t <= STOPS[1].0 { 0 } else { 1 };
    let (t0, a) = STOPS[k];
    let (t1, b) = STOPS[k + 1];
    let u = (t - t0) / (t1 - t0);
    std::array::from_fn(|c| (f32::from(a[c]) + (f32::from(b[c]) - f32::from(a[c])) * u).round() as u8)
}
