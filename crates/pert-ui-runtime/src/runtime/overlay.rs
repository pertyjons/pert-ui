//! Development overlay drawing appended to a composed scene.
//!
//! Everything here is a pure function of validation data and the viewport, so
//! composing the overlay can invalidate nothing. The drawing is plain
//! rectangles in fixed colors: it must not depend on the theme it may be
//! debugging, and it must not enter the shape cache or a resource table.
use crate::{Color, DrawCommand, Rect, Viewport};

/// Outline color of a marker whose own color is not a valid color.
const ISSUE: Color = Color::linear_rgb(1.0, 0.0, 0.0);
/// Badge numeral color.
const BADGE_TEXT: Color = Color::linear_rgb(1.0, 1.0, 1.0);
/// Border-box outline color of the box-model overlay.
const BORDER_BOX: Color = Color::linear_rgb(0.0, 0.55, 1.0);
/// Content-box outline color of the box-model overlay.
const CONTENT_BOX: Color = Color::linear_rgb(0.0, 0.85, 0.3);

/// Marker outline width in physical pixels.
const ISSUE_STROKE: f64 = 2.0;
/// Box-model outline width in physical pixels.
const BOX_STROKE: f64 = 1.0;
/// Side of one numeral cell in physical pixels.
const BADGE_CELL: f64 = 2.0;
/// Largest count the badge writes out; anything above shows as `999+`.
const BADGE_MAX: usize = 999;

/// Rows of a 3-by-5 block numeral, most significant bit leftmost.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b001, 0b001, 0b001],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];
const PLUS: [u8; 5] = [0b000, 0b010, 0b111, 0b010, 0b000];

fn fill(out: &mut Vec<DrawCommand>, x: f64, y: f64, w: f64, h: f64, color: Color) {
    // Bounds come from validated layout; a rectangle that cannot be represented
    // is left out rather than failing the application's paint for a diagnostic.
    if w > 0.0
        && h > 0.0
        && let Ok(rect) = Rect::new(x as f32, y as f32, w as f32, h as f32)
    {
        out.push(DrawCommand::FillRect { rect, color });
    }
}

/// Appends an outline drawn inside `bounds`, at most four rectangles.
pub(super) fn outline(out: &mut Vec<DrawCommand>, bounds: [f32; 4], stroke: f64, color: Color) {
    let [x, y, w, h] = bounds.map(f64::from);
    if !(w > 0.0 && h > 0.0) {
        return;
    }
    let horizontal = stroke.min(h);
    let vertical = stroke.min(w);
    fill(out, x, y, w, horizontal, color);
    fill(out, x, y + h - horizontal, w, horizontal, color);
    let inner = h - 2.0 * horizontal;
    fill(out, x, y + horizontal, vertical, inner, color);
    fill(
        out,
        x + w - vertical,
        y + horizontal,
        vertical,
        inner,
        color,
    );
}

/// Appends marker outlines and, when `uncounted` is nonzero, the count badge.
pub(super) fn issues<'a>(
    out: &mut Vec<DrawCommand>,
    viewport: Viewport,
    markers: impl Iterator<Item = &'a crate::ValidationMarker>,
    limit: usize,
) {
    let pixel = 1.0 / viewport.scale();
    let mut drawn = 0;
    let mut uncounted = 0_usize;
    for marker in markers {
        match marker.bounds {
            Some(bounds) if drawn < limit => {
                let [r, g, b, a] = marker.color;
                let color = Color::from_linear(r, g, b, a).unwrap_or(ISSUE);
                outline(out, bounds, ISSUE_STROKE * pixel, color);
                drawn += 1;
            }
            _ => uncounted = uncounted.saturating_add(1),
        }
    }
    if uncounted > 0 {
        badge(out, viewport, uncounted);
    }
}

/// Appends box-model outlines for at most `limit` components.
pub(super) fn boxes(
    out: &mut Vec<DrawCommand>,
    viewport: Viewport,
    boxes: impl Iterator<Item = crate::validation::development::OverlayBox>,
    limit: usize,
) {
    let stroke = BOX_STROKE / viewport.scale();
    for item in boxes.take(limit) {
        outline(out, item.border, stroke, BORDER_BOX);
        if let Some(content) = item.content {
            outline(out, content, stroke, CONTENT_BOX);
        }
    }
}

/// The glyph rows the badge shows for `count`.
fn glyphs(count: usize) -> Vec<[u8; 5]> {
    let shown = count.min(BADGE_MAX);
    let mut glyphs: Vec<_> = shown
        .to_string()
        .bytes()
        .map(|digit| DIGITS[usize::from(digit - b'0')])
        .collect();
    if count > BADGE_MAX {
        glyphs.push(PLUS);
    }
    glyphs
}

fn badge(out: &mut Vec<DrawCommand>, viewport: Viewport, count: usize) {
    let cell = BADGE_CELL / viewport.scale();
    let glyphs = glyphs(count);
    // One cell of padding around the numerals and one between them.
    let columns = glyphs.len() * 4 + 1;
    let width = columns as f64 * cell;
    let height = 7.0 * cell;
    let left = (f64::from(viewport.width()) - width).max(0.0);
    fill(out, left, 0.0, width, height, ISSUE);
    for (index, glyph) in glyphs.iter().enumerate() {
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..3 {
                if bits & (0b100 >> column) != 0 {
                    fill(
                        out,
                        left + (1 + index * 4 + column) as f64 * cell,
                        (1 + row) as f64 * cell,
                        cell,
                        cell,
                        BADGE_TEXT,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(bounds: Option<[f32; 4]>) -> crate::ValidationMarker {
        crate::ValidationMarker {
            component: 1,
            bounds,
            color: [1.0, 0.0, 0.0, 1.0],
            codes: Vec::new(),
        }
    }

    fn lit(glyph: &[u8; 5]) -> usize {
        glyph.iter().map(|row| row.count_ones() as usize).sum()
    }

    #[test]
    fn outline_stays_inside_bounds_and_degenerates_without_overdraw() {
        let mut out = Vec::new();
        outline(&mut out, [10.0, 20.0, 30.0, 40.0], 2.0, ISSUE);
        assert_eq!(out.len(), 4);
        for command in &out {
            let DrawCommand::FillRect { rect, .. } = command else {
                panic!("outline draws rectangles only");
            };
            assert!(rect.x() >= 10.0 && rect.x() + rect.width() <= 40.0);
            assert!(rect.y() >= 20.0 && rect.y() + rect.height() <= 60.0);
        }
        // Thinner than two strokes: the horizontal edges cover it, no sides.
        let mut thin = Vec::new();
        outline(&mut thin, [0.0, 0.0, 30.0, 3.0], 2.0, ISSUE);
        assert_eq!(thin.len(), 2);
    }

    #[test]
    fn badge_counts_unplaced_and_excess_markers() {
        let viewport = Viewport::new(200.0, 100.0, 1.0).unwrap();
        let placed = marker(Some([0.0, 0.0, 10.0, 10.0]));
        let unplaced = marker(None);
        let mut out = Vec::new();
        issues(&mut out, viewport, [&placed, &placed].into_iter(), 64);
        assert_eq!(out.len(), 8, "no badge without uncounted markers");

        // One outline fits the limit; one placed and one unplaced are counted.
        let mut out = Vec::new();
        issues(
            &mut out,
            viewport,
            [&placed, &placed, &unplaced].into_iter(),
            1,
        );
        assert_eq!(out.len(), 4 + 1 + lit(&DIGITS[2]));
        let DrawCommand::FillRect { rect, .. } = out[4] else {
            panic!("badge background");
        };
        assert_eq!(rect.x() + rect.width(), 200.0);
        assert_eq!(rect.y(), 0.0);
    }

    #[test]
    fn badge_numerals_saturate() {
        assert_eq!(glyphs(7), [DIGITS[7]]);
        assert_eq!(glyphs(64), [DIGITS[6], DIGITS[4]]);
        assert_eq!(glyphs(999), [DIGITS[9]; 3]);
        assert_eq!(glyphs(1000), [DIGITS[9], DIGITS[9], DIGITS[9], PLUS]);
    }
}
