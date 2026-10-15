//! The shared bottom-right resize mark for floating frames.

use crate::{ColorRole, DrawError, FillRule, Painter, Path, Point, Rect};

/// Draws a small triangle inside the existing corner hit area.
pub(crate) fn paint_corner_triangle(painter: &mut Painter, grip: Rect) -> Result<(), DrawError> {
    let [x, y, width, height] = grip.values();
    let side = width.min(height);
    if side == 0.0 {
        return Ok(());
    }
    let margin = 2.0_f32.min(side / 4.0);
    let leg = (side - 2.0 * margin).min(8.0);
    let right = x + width - margin;
    let bottom = y + height - margin;
    let triangle = Path::polygon([
        Point::new(right - leg, bottom)?,
        Point::new(right, bottom - leg)?,
        Point::new(right, bottom)?,
    ])?;
    painter.fill_path(
        &triangle,
        FillRule::NonZero,
        painter.color(ColorRole::Muted),
    )
}
