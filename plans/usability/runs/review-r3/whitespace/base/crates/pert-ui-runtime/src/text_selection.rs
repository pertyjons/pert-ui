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

/// Whether a caret movement or placement keeps the selection's anchor.
///
/// [`TextSelection::moved`] and [`TextSelection::at_point`] take it: a plain
/// arrow key or click collapses the selection to the new caret, and the same
/// gesture with Shift extends it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SelectionExtension {
    /// The anchor follows the caret, leaving a collapsed selection there.
    Collapse,
    /// The anchor stays, so the selection runs from it to the new caret.
    Extend,
}
impl SelectionExtension {
    /// Shift extends; otherwise the selection collapses.
    pub(crate) fn from_shift(shift: bool) -> Self {
        if shift { Self::Extend } else { Self::Collapse }
    }
}

/// Retained selection with UTF-8 byte offsets and visual caret affinity.
///
/// Create and operate on this with the layout for the current text. Recreate it
/// after replacing text; movement refreshes affinity for width/scale changes.
/// Caret stops are the grapheme boundaries of the shaped text: a combining
/// sequence or an emoji joined by zero-width joiners is one grapheme, with no
/// stop inside it. Pointer placement ([`Self::at_point`],
/// [`Self::word_at_point`]) snaps to the nearest stop; explicit offsets given
/// to [`Self::new`] must already be stops.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[must_use]
pub struct TextSelection(Selection);
impl TextSelection {
    /// Creates a selection from UTF-8 byte offsets of `layout`'s text.
    ///
    /// The anchor is the end [`SelectionExtension::Extend`] keeps, and it may
    /// follow the focus: a backward selection, as Shift+Left makes, is valid.
    /// Both offsets must be caret stops: `0`, the text's length, or the start
    /// of a grapheme.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Label, Runtime, TextEditError, TextSelection, TextStyle};
    /// let mut ui = Runtime::new();
    /// let label = ui.mount_tree(Element::new("label", Label::new("")))?;
    /// // "e" with a combining acute accent is one grapheme of three bytes.
    /// let layout = ui.event(&label, |cx| cx.text("e\u{301}x", None, TextStyle::Body))??;
    /// // Backward, from after "x" to the start.
    /// let selection = TextSelection::new(&layout, 4, 0)?;
    /// assert_eq!(selection.byte_range(), 0..4);
    /// // Byte 1 is inside "e" with its combining accent.
    /// assert_eq!(
    ///     TextSelection::new(&layout, 1, 1),
    ///     Err(TextEditError::InvalidSelection { anchor_byte: 1, focus_byte: 1 })
    /// );
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns
    /// [`TextEditError::InvalidSelection`](crate::TextEditError::InvalidSelection)
    /// with both offsets when either lies past the end of the text or inside a
    /// grapheme; nothing is clamped or snapped.
    pub fn new(
        layout: &TextLayout,
        anchor_byte: usize,
        focus_byte: usize,
    ) -> Result<Self, crate::TextEditError> {
        let selection = Self::snapped(layout, anchor_byte, focus_byte);
        if selection.anchor_byte() != anchor_byte || selection.focus_byte() != focus_byte {
            return Err(crate::TextEditError::InvalidSelection {
                anchor_byte,
                focus_byte,
            });
        }
        Ok(selection)
    }
    /// A selection with out-of-range offsets moved to the text end and
    /// offsets inside a grapheme moved to its start, for the library's own
    /// placements (`usize::MAX` for "the end") and offsets carried across a
    /// text replacement.
    pub(crate) fn snapped(layout: &TextLayout, anchor_byte: usize, focus_byte: usize) -> Self {
        Self(Selection::new(
            Cursor::from_byte_index(&layout.shaped, anchor_byte, parley::Affinity::Downstream),
            Cursor::from_byte_index(&layout.shaped, focus_byte, parley::Affinity::Downstream),
        ))
    }
    /// UTF-8 byte offset of the fixed end of this selection.
    pub fn anchor_byte(self) -> usize {
        self.0.anchor().index()
    }
    /// UTF-8 byte offset of the moving end (caret).
    pub fn focus_byte(self) -> usize {
        self.0.focus().index()
    }
    /// Ordered UTF-8 byte range of selected text; empty for a collapsed caret.
    pub fn byte_range(self) -> std::ops::Range<usize> {
        self.0.text_range()
    }
    /// Moves the caret, keeping the anchor for [`SelectionExtension::Extend`].
    pub fn moved(
        self,
        layout: &TextLayout,
        movement: TextMovement,
        extension: SelectionExtension,
    ) -> Self {
        let extend = extension == SelectionExtension::Extend;
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
    /// Outside points snap to the nearest available caret;
    /// [`SelectionExtension::Extend`] keeps the anchor. A point outside
    /// [`TextLayout::size`] can still be over whitespace hanging past the end of
    /// a wrapped line, as [`Self::caret`] describes, and places the caret there.
    pub fn at_point(
        self,
        layout: &TextLayout,
        point: Point,
        extension: SelectionExtension,
    ) -> Self {
        let x = (point.x() - layout.origin[0]) * layout.scale;
        let y = (point.y() - layout.origin[1]) * layout.scale;
        Self(if extension == SelectionExtension::Extend {
            self.0.extend_to_point(&layout.shaped, x, y)
        } else {
            Selection::from_point(&layout.shaped, x, y)
        })
    }
    /// Selects the word at a logical point, using the shaping engine's word
    /// boundaries. Whitespace/punctuation follow the engine's Unicode segmentation.
    /// Points map to text as for [`Self::at_point`].
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
    /// After whitespace hanging past the end of a wrapped line, spaces and tabs
    /// alike, the rectangle can lie outside [`TextLayout::size`]: beyond its
    /// right edge in left-to-right text, and at negative x in right-to-left
    /// text, where the hanging whitespace is on the left. In right-to-left text
    /// a hanging space can extend the size where a tab does not, so their
    /// coordinates can differ, as [`TextLayout`] describes.
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
    /// Allocates per selected run. A selection that includes hanging whitespace
    /// can extend outside [`TextLayout::size`], as [`Self::caret`] describes.
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
            assert_eq!(
                TextSelection::new(&layout, inside, 0),
                Err(crate::TextEditError::InvalidSelection {
                    anchor_byte: inside,
                    focus_byte: 0
                }),
                "{value:?}"
            );
            let selection = TextSelection::snapped(&layout, inside, inside);
            assert_eq!(selection.byte_range(), 0..0, "{value:?}");
            assert_eq!(
                selection
                    .moved(&layout, TextMovement::Right, SelectionExtension::Collapse)
                    .focus_byte(),
                value.len() - 1,
                "{value:?}"
            );
        }
    }
}
