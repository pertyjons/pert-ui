//! Owned selection operations over the same shaped text that is painted.
use crate::{Point, Rect, TextLayout};
use parley::editing::{Cursor, Selection};

/// A movement in shaped text. Left/right follow visual order, including bidi runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TextMovement {
    /// Previous visual grapheme.
    Left,
    /// Next visual grapheme.
    Right,
    /// Previous visual word boundary.
    WordLeft,
    /// Next visual word boundary.
    WordRight,
    /// Previous displayed line, retaining the preferred horizontal position.
    Up,
    /// Next displayed line, retaining the preferred horizontal position.
    Down,
    /// Start of the displayed line.
    LineStart,
    /// End of the displayed line.
    LineEnd,
    /// Start of the complete text.
    Start,
    /// End of the complete text.
    End,
}

/// Retained selection with UTF-8 byte offsets and visual caret affinity.
///
/// Create and operate on this with the layout for the current text. Recreate it
/// after replacing text; movement refreshes affinity for width/scale changes.
/// Offsets are snapped to grapheme boundaries, including inside ligatures.
/// Combining sequences and emoji joined by zero-width joiners form one stop.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[must_use]
pub struct TextSelection(Selection);
impl TextSelection {
    /// Creates a selection, clamping out-of-range offsets to text end and snapping
    /// offsets inside clusters to their start. Anchor is retained when extending.
    pub fn new(layout: &TextLayout, anchor: usize, focus: usize) -> Self {
        Self(Selection::new(
            Cursor::from_byte_index(&layout.shaped, anchor, parley::Affinity::Downstream),
            Cursor::from_byte_index(&layout.shaped, focus, parley::Affinity::Downstream),
        ))
    }
    /// Byte offset of the fixed end of this selection.
    pub fn anchor(self) -> usize {
        self.0.anchor().index()
    }
    /// Byte offset of the moving end (caret).
    pub fn focus(self) -> usize {
        self.0.focus().index()
    }
    /// Ordered byte range of selected text; empty for a collapsed caret.
    pub fn range(self) -> std::ops::Range<usize> {
        self.0.text_range()
    }
    /// Moves the caret, retaining the anchor when `extend` is true.
    pub fn moved(self, layout: &TextLayout, movement: TextMovement, extend: bool) -> Self {
        let s = self.0.refresh(&layout.shaped);
        let l = &layout.shaped;
        Self(match movement {
            TextMovement::Left => s.previous_visual(l, extend),
            TextMovement::Right => s.next_visual(l, extend),
            TextMovement::WordLeft => s.previous_visual_word(l, extend),
            TextMovement::WordRight => s.next_visual_word(l, extend),
            TextMovement::Up => s.previous_line(l, extend),
            TextMovement::Down => s.next_line(l, extend),
            TextMovement::LineStart => s.line_start(l, extend),
            TextMovement::LineEnd => s.line_end(l, extend),
            TextMovement::Start | TextMovement::End => {
                let index = if movement == TextMovement::Start {
                    0
                } else {
                    usize::MAX
                };
                let c = Cursor::from_byte_index(l, index, parley::Affinity::Downstream);
                if extend { s.extend(c) } else { c.into() }
            }
        })
    }
    /// Places the caret at a logical point relative to the painted text origin.
    /// Outside points snap to the nearest available caret; `extend` keeps anchor.
    pub fn at_point(self, layout: &TextLayout, point: Point, extend: bool) -> Self {
        let x = (point.x() - layout.origin[0]) * layout.scale;
        let y = (point.y() - layout.origin[1]) * layout.scale;
        Self(if extend {
            self.0.extend_to_point(&layout.shaped, x, y)
        } else {
            Selection::from_point(&layout.shaped, x, y)
        })
    }
    /// Selects the word at a logical point, using the shaping engine's word
    /// boundaries. Whitespace/punctuation follow the engine's Unicode segmentation.
    pub fn word_at_point(layout: &TextLayout, point: Point) -> Self {
        Self(Selection::word_from_point(
            &layout.shaped,
            (point.x() - layout.origin[0]) * layout.scale,
            (point.y() - layout.origin[1]) * layout.scale,
        ))
    }
    /// Caret rectangle in logical coordinates relative to painted text origin.
    /// Width is one logical pixel.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) or
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) when the layout's
    /// prepared geometry does not convert to a valid rectangle.
    pub fn caret(self, layout: &TextLayout) -> Result<Rect, crate::GeometryError> {
        let r = self.0.focus().geometry(&layout.shaped, layout.scale);
        convert(layout, r.x0, r.y0, r.x1, r.y1)
    }
    /// Selection rectangles in logical coordinates relative to painted text origin.
    /// Allocates per selected run.
    ///
    /// # Errors
    /// As [`Self::caret`], for any selected run.
    pub fn rectangles(self, layout: &TextLayout) -> Result<Vec<Rect>, crate::GeometryError> {
        self.0
            .geometry(&layout.shaped)
            .into_iter()
            .map(|(r, _)| convert(layout, r.x0, r.y0, r.x1, r.y1))
            .collect()
    }
}
fn convert(
    l: &TextLayout,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Result<Rect, crate::GeometryError> {
    Rect::new(
        x0 as f32 / l.scale + l.origin[0],
        y0 as f32 / l.scale + l.origin[1],
        (x1 - x0) as f32 / l.scale,
        (y1 - y0) as f32 / l.scale,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TextStyle, text::TextService};

    #[test]
    fn shaped_carets_snap_to_whole_graphemes() {
        let mut text = TextService::new().unwrap();
        for (value, inside) in [("e\u{301}x", 1), ("👩‍💻x", 4)] {
            let layout = text.prepare(value, None, 1.0, TextStyle::Body).unwrap();
            let selection = TextSelection::new(&layout, inside, inside);
            assert_eq!(selection.range(), 0..0, "{value:?}");
            assert_eq!(
                selection.moved(&layout, TextMovement::Right, false).focus(),
                value.len() - 1,
                "{value:?}"
            );
        }
    }
}
