//! Single-line truncation with an ellipsis and highlighted ranges for matches.
use super::{RichText, TextLayout, TextSpan};
use crate::ColorRole;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// The character that marks removed text: U+2026 HORIZONTAL ELLIPSIS.
pub(crate) const ELLIPSIS: &str = "\u{2026}";

/// How text that does not fit its width is shown.
///
/// The default, [`Self::Wrap`], breaks text into lines at the width it is given,
/// so the whole text always shows. The ellipsis variants keep the text on one
/// line and replace removed grapheme clusters with "…" (U+2026); the full text
/// stays available to inspection and, for a [`Label`](crate::Label), in a
/// delayed tooltip. Truncation never splits a grapheme cluster, so combining
/// marks, emoji sequences and other clusters stay whole, and the whitespace
/// trimmed beside the ellipsis is removed only as whole clusters. Text with a
/// line break (a line feed, carriage return, vertical tab, form feed, next
/// line, or Unicode line or paragraph separator) keeps only its first line and
/// always shows the ellipsis. Text far wider than its width is measured
/// without being rasterized, so only the shown truncation must fit the
/// [raster limit](crate::TextLimit::MAX_RASTER_SIDE).
///
/// Truncation applies to plain text. A rich label with icons or styled spans
/// stays on one line and is clipped at its edge instead.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum TextOverflow {
    /// Wrap at the available width; nothing is removed. The default.
    #[default]
    Wrap,
    /// Keep the start, ending in "…", as in `A long file na…`.
    Ellipsis,
    /// Keep the start and the end around "…", as in `/home/…/report.txt`,
    /// which suits paths and identifiers whose ending matters.
    MiddleEllipsis,
}

/// Text prepared to fit a width, from
/// [`MeasureContext::fitted_text`](crate::MeasureContext::fitted_text).
///
/// The layout is what to measure and paint. When the text had to be
/// truncated, [`Self::shown`] holds the text the layout shows, including the
/// ellipsis; the caller keeps the full text for inspection and help.
#[derive(Clone, Debug)]
#[must_use]
pub struct FittedText {
    pub(crate) layout: TextLayout,
    pub(crate) shown: Option<String>,
}
impl FittedText {
    /// The prepared layout to measure and paint.
    pub const fn layout(&self) -> &TextLayout {
        &self.layout
    }
    /// The prepared layout, consuming the result.
    pub fn into_layout(self) -> TextLayout {
        self.layout
    }
    /// Whether text was removed to fit.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.shown.is_some()
    }
    /// The text the layout shows, including the ellipsis, when it was
    /// truncated; `None` when the whole text fits.
    #[must_use]
    pub fn shown(&self) -> Option<&str> {
        self.shown.as_deref()
    }
}

/// The text a truncation candidate shows when it keeps `kept` grapheme
/// clusters of `line`, whose cluster boundaries are `bounds` (including the
/// end). The clusters next to the ellipsis lose surrounding whitespace.
pub(crate) fn candidate(
    line: &str,
    bounds: &[usize],
    kept: usize,
    overflow: TextOverflow,
) -> String {
    let count = bounds.len() - 1;
    match overflow {
        TextOverflow::MiddleEllipsis => {
            let tail = kept / 2;
            let head = kept - tail;
            format!(
                "{}{ELLIPSIS}{}",
                trim_end(&line[..bounds[head]]),
                trim_start(&line[bounds[count - tail]..])
            )
        }
        _ => format!("{}{ELLIPSIS}", trim_end(&line[..bounds[kept]])),
    }
}

/// Whether a grapheme cluster is whitespace throughout, so trimming it keeps
/// every cluster whole: a no-break space carrying a combining mark stays.
fn blank(cluster: &str) -> bool {
    cluster.chars().all(char::is_whitespace)
}

/// `text` without its trailing whitespace clusters.
fn trim_end(text: &str) -> &str {
    let end = text
        .grapheme_indices(true)
        .rev()
        .find(|(_, cluster)| !blank(cluster))
        .map_or(0, |(index, cluster)| index + cluster.len());
    &text[..end]
}

/// `text` without its leading whitespace clusters.
fn trim_start(text: &str) -> &str {
    let start = text
        .grapheme_indices(true)
        .find(|(_, cluster)| !blank(cluster))
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

/// The characters that force a line break when text is shaped (Unicode line
/// breaking classes BK, CR, LF and NL), so a single-line fit ends its first
/// line at any of them.
const HARD_BREAKS: [char; 7] = [
    '\n', '\r', '\u{0B}', '\u{0C}', '\u{85}', '\u{2028}', '\u{2029}',
];

/// The first line of `text` and the boundaries of its grapheme clusters,
/// including the end, plus whether a line break removed more text.
pub(crate) fn clusters(text: &str) -> (&str, Vec<usize>, bool) {
    let end = text.find(HARD_BREAKS);
    let line = &text[..end.unwrap_or(text.len())];
    let bounds = line
        .grapheme_indices(true)
        .map(|(index, _)| index)
        .chain(std::iter::once(line.len()))
        .collect();
    (line, bounds, end.is_some())
}

/// A text range could not be highlighted.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum HighlightError {
    /// The range ends after the text or starts after it ends.
    #[error("highlight range {start}..{end} is outside the text of {len} bytes")]
    OutOfBounds {
        /// The range's start byte.
        start: usize,
        /// The range's end byte.
        end: usize,
        /// The text's length in bytes.
        len: usize,
    },
    /// A range boundary falls inside a grapheme cluster.
    #[error("highlight boundary {index} is not a grapheme cluster boundary")]
    NotGraphemeBoundary {
        /// The offending byte offset.
        index: usize,
    },
}

impl RichText {
    /// Builds text with `ranges` highlighted, for showing what a search or a
    /// filter matched.
    ///
    /// Ranges are byte offsets into `text` and must lie on grapheme cluster
    /// boundaries. They may come in any order and may overlap or touch; they
    /// are merged, and empty ranges are ignored. Highlighted text follows the
    /// [`ColorRole::Highlight`] theme role, so a theme change only repaints;
    /// the rest inherits the containing label's color. The plain text of the
    /// result is `text` unchanged, so inspection and names see the same text.
    ///
    /// # Errors
    /// Returns [`HighlightError::OutOfBounds`] for a range past the end or
    /// with its start after its end, and
    /// [`HighlightError::NotGraphemeBoundary`] for a boundary inside a
    /// grapheme cluster; nothing is built then.
    ///
    /// ```
    /// use pert_ui_runtime::{HighlightError, RichText};
    ///
    /// let text = RichText::highlighted("Reverb send", [0..3, 7..11])?;
    /// assert_eq!(text.plain_text(), "Reverb send");
    /// assert!(RichText::highlighted("e\u{301}cho", [0..1]).is_err());
    /// # Ok::<(), HighlightError>(())
    /// ```
    pub fn highlighted(
        text: &str,
        ranges: impl IntoIterator<Item = Range<usize>>,
    ) -> Result<Self, HighlightError> {
        let mut ranges: Vec<Range<usize>> = ranges.into_iter().collect();
        for range in &ranges {
            if range.start > range.end || range.end > text.len() {
                return Err(HighlightError::OutOfBounds {
                    start: range.start,
                    end: range.end,
                    len: text.len(),
                });
            }
        }
        let boundaries: Vec<usize> = text
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
            .collect();
        for range in &ranges {
            for index in [range.start, range.end] {
                if boundaries.binary_search(&index).is_err() {
                    return Err(HighlightError::NotGraphemeBoundary { index });
                }
            }
        }
        ranges.retain(|range| !range.is_empty());
        ranges.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
        for range in ranges {
            match merged.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }
        let mut content = Self::new();
        let mut at = 0;
        for range in merged {
            if at < range.start {
                content = content.text(&text[at..range.start]);
            }
            content = content.span(
                TextSpan::new(&text[range.start..range.end]).color_role(ColorRole::Highlight),
            );
            at = range.end;
        }
        if at < text.len() || text.is_empty() {
            content = content.text(&text[at..]);
        }
        Ok(content)
    }
}
