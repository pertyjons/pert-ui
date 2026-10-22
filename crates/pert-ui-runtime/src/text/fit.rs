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
/// stays available to inspection and, for a [`Label`](crate::Label) whose text
/// is within the limit below, in a delayed tooltip. Truncation never splits a grapheme cluster, so combining
/// marks, emoji sequences and other clusters stay whole, and the whitespace
/// trimmed beside the ellipsis is removed only as whole clusters. Text with a
/// line break (a line feed, carriage return, vertical tab, form feed, next
/// line, or Unicode line or paragraph separator) keeps only its first line and
/// always shows the ellipsis. Text far wider than its width is measured
/// without being rasterized, so only the shown truncation must fit the
/// [raster limit](crate::TextLimit::MAX_RASTER_SIDE).
///
/// A first line longer than
/// [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES) is
/// cut before it is fitted, so its length never makes truncation fail or
/// costs more than a bounded amount of shaping: [`Self::Ellipsis`] keeps its
/// start and [`Self::MiddleEllipsis`] as many grapheme clusters from its start
/// as from its end, within the limit together with the ellipsis. The cut
/// line is then fitted like a line followed by a line break, so it always
/// shows the ellipsis. Finding where the line ends still reads the whole
/// text.
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
/// end). The clusters next to the ellipsis lose surrounding whitespace,
/// judged by those boundaries, so a cut line keeps its source clusters.
pub(crate) fn candidate(
    line: &str,
    bounds: &[usize],
    kept: usize,
    overflow: TextOverflow,
) -> String {
    let count = bounds.len() - 1;
    let cluster = |index: usize| &line[bounds[index]..bounds[index + 1]];
    // The end of the kept start, without the whitespace clusters before it.
    let head_end = |mut head: usize| {
        while head > 0 && blank(cluster(head - 1)) {
            head -= 1;
        }
        bounds[head]
    };
    match overflow {
        TextOverflow::MiddleEllipsis => {
            let tail = kept / 2;
            let head = kept - tail;
            let mut start = count - tail;
            while start < count && blank(cluster(start)) {
                start += 1;
            }
            format!(
                "{}{ELLIPSIS}{}",
                &line[..head_end(head)],
                &line[bounds[start]..]
            )
        }
        _ => format!("{}{ELLIPSIS}", &line[..head_end(kept)]),
    }
}

/// Whether a grapheme cluster is whitespace throughout, so trimming it keeps
/// every cluster whole: a no-break space carrying a combining mark stays.
fn blank(cluster: &str) -> bool {
    cluster.chars().all(char::is_whitespace)
}

/// The characters that force a line break when text is shaped (Unicode line
/// breaking classes BK, CR, LF and NL), so a single-line fit ends its first
/// line at any of them.
const HARD_BREAKS: [char; 7] = [
    '\n', '\r', '\u{0B}', '\u{0C}', '\u{85}', '\u{2028}', '\u{2029}',
];

/// The first line of `text` cut so that its kept clusters and the ellipsis
/// fit [`TextEditError::MAX_TEXT_BYTES`](crate::TextEditError::MAX_TEXT_BYTES),
/// with the boundaries of the kept clusters (including the end), when the
/// line is longer than that limit; `None` otherwise.
///
/// [`TextOverflow::MiddleEllipsis`] keeps as many clusters from the end as
/// from the start, joined, so a candidate keeping all of them puts the
/// ellipsis at the join. Clusters are found by iterating the line from its
/// start and from its end, never from inside it, so every boundary is one
/// of the line's own; the work stays within the limit except where the
/// segmenter must look back through a long run of regional indicators, or
/// read one cluster longer than the limit, such as a base with many
/// combining marks, before it can leave it out.
pub(crate) fn bounded(text: &str, overflow: TextOverflow) -> Option<(String, Vec<usize>)> {
    let line = &text[..text.find(HARD_BREAKS).unwrap_or(text.len())];
    if line.len() <= crate::TextEditError::MAX_TEXT_BYTES {
        return None;
    }
    let budget = crate::TextEditError::MAX_TEXT_BYTES - ELLIPSIS.len();
    let tail_share = if overflow == TextOverflow::MiddleEllipsis {
        budget / 2
    } else {
        0
    };
    let head_share = budget - tail_share;
    // Ends of the clusters kept from the start, in order.
    let mut head: Vec<usize> = line
        .grapheme_indices(true)
        .map(|(index, cluster)| index + cluster.len())
        .take_while(|end| *end <= head_share)
        .collect();
    // Starts of the clusters kept from the end, nearest the end first.
    let mut tail: Vec<usize> = if tail_share == 0 {
        Vec::new()
    } else {
        line.grapheme_indices(true)
            .rev()
            .map(|(index, _)| index)
            .take_while(|start| line.len() - start <= tail_share)
            .collect()
    };
    if overflow == TextOverflow::MiddleEllipsis {
        let kept = head.len().min(tail.len());
        head.truncate(kept);
        tail.truncate(kept);
    }
    let head_end = head.last().copied().unwrap_or(0);
    let tail_start = tail.last().copied().unwrap_or(line.len());
    let mut cut = String::with_capacity(head_end + line.len() - tail_start);
    cut.push_str(&line[..head_end]);
    cut.push_str(&line[tail_start..]);
    let shift = |index: usize| index - tail_start + head_end;
    let bounds = std::iter::once(0)
        .chain(head)
        .chain(tail.iter().rev().skip(1).map(|start| shift(*start)))
        .chain((!tail.is_empty()).then_some(cut.len()))
        .collect();
    Some((cut, bounds))
}

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
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{HighlightError, RichText};
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
