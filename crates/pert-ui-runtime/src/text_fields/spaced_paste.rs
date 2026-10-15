//! Placement and spacing for `TextCommand::PasteBefore`/`PasteAfter`.
//!
//! Offsets are logical UTF-8 byte offsets, never visual positions, so the result
//! is the same for left-to-right and right-to-left runs.

use std::ops::Range;

/// A maximal run of non-whitespace characters, in logical byte offsets.
type Word = Range<usize>;

/// Returns the word containing `caret`, treating a caret touching either side of
/// a run as inside it. Returns `None` when both neighbors are whitespace or edges.
fn word_at(text: &str, caret: usize) -> Option<Word> {
    let before = text[..caret].chars().next_back();
    let after = text[caret..].chars().next();
    if !before.is_some_and(|c| !c.is_whitespace()) && !after.is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    let start = text[..caret]
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let end = text[caret..]
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map_or(text.len(), |(i, _)| caret + i);
    Some(start..end)
}

/// Returns the last word ending at or before `caret`.
fn word_before(text: &str, caret: usize) -> Option<Word> {
    let end = text[..caret]
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, c)| i + c.len_utf8())?;
    word_at(text, end)
}

/// Returns the first word starting at or after `caret`.
fn word_after(text: &str, caret: usize) -> Option<Word> {
    let start = text[caret..]
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| caret + i)?;
    word_at(text, start)
}

/// Returns the insertion offset for a paste placed beside the word or selection
/// at `selection`, and the span that stays selected afterwards.
///
/// A nonempty selection is the target span. Otherwise the word under the caret is,
/// and when the caret touches no word the nearest word in the chosen direction is
/// used, which may be on an adjacent line. Without any such word the caret itself
/// is the insertion point.
fn target(text: &str, selection: Range<usize>, after: bool) -> (usize, Range<usize>) {
    if !selection.is_empty() {
        let at = if after {
            selection.end
        } else {
            selection.start
        };
        return (at, selection);
    }
    let caret = selection.start;
    let word = word_at(text, caret).or_else(|| {
        if after {
            word_after(text, caret)
        } else {
            word_before(text, caret)
        }
    });
    word.map_or((caret, caret..caret), |word| {
        (if after { word.end } else { word.start }, word)
    })
}

/// The text to splice in at `at` so the pasted content stays a separate word.
///
/// A single space is added on a side whose neighbor is neither whitespace nor the
/// end of the text, so a paste at a line start or beside existing spacing does not
/// double them. Line breaks count as whitespace.
fn spaced(text: &str, at: usize, insert: &str) -> String {
    let lead = !(at == 0
        || text[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace));
    let trail = !(at == text.len() || text[at..].chars().next().is_some_and(char::is_whitespace));
    let mut out = String::with_capacity(insert.len() + 2);
    if lead {
        out.push(' ');
    }
    out.push_str(insert);
    if trail {
        out.push(' ');
    }
    out
}

/// Resolves a spaced paste into the replacement to apply and the span to reselect.
///
/// `clipboard` is trimmed, because the surrounding spacing is supplied here rather
/// than taken from whatever produced the clipboard text. Returns `None` when the
/// trimmed clipboard is empty, which is not an edit.
pub(super) fn resolve(
    text: &str,
    selection: Range<usize>,
    clipboard: &str,
    after: bool,
) -> Option<(usize, String, Range<usize>)> {
    let trimmed = clipboard.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (at, keep) = target(text, selection, after);
    let insert = spaced(text, at, trimmed);
    let shift = if at <= keep.start { insert.len() } else { 0 };
    Some((at, insert, keep.start + shift..keep.end + shift))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies `resolve` so the tests read as before/after text rather than offsets.
    fn paste(text: &str, caret: Range<usize>, clip: &str, after: bool) -> (String, String) {
        let Some((at, insert, keep)) = resolve(text, caret, clip, after) else {
            return (text.to_string(), String::new());
        };
        let mut out = text.to_string();
        out.insert_str(at, &insert);
        let selected = out[keep].to_string();
        (out, selected)
    }

    #[test]
    fn caret_in_a_word_pastes_beside_that_word() {
        let text = "det snabba räven";
        let caret = 6..6; // inside "snabba"
        assert_eq!(
            paste(text, caret.clone(), "bruna", false).0,
            "det bruna snabba räven"
        );
        assert_eq!(
            paste(text, caret, "bruna", true).0,
            "det snabba bruna räven"
        );
    }

    #[test]
    fn a_selection_is_the_target_and_stays_selected() {
        let text = "det snabba räven";
        let selection = 4..10; // "snabba"
        let (before, kept) = paste(text, selection.clone(), "bruna", false);
        assert_eq!(before, "det bruna snabba räven");
        assert_eq!(kept, "snabba");
        let (after, kept) = paste(text, selection, "bruna", true);
        assert_eq!(after, "det snabba bruna räven");
        assert_eq!(kept, "snabba");
    }

    #[test]
    fn a_caret_outside_a_word_uses_the_nearest_word_in_that_direction() {
        let text = "det snabba  räven";
        let caret = 11..11; // between the two spaces
        assert_eq!(
            paste(text, caret.clone(), "bruna", false).0,
            "det bruna snabba  räven"
        );
        assert_eq!(
            paste(text, caret, "bruna", true).0,
            "det snabba  räven bruna"
        );
    }

    #[test]
    fn existing_spacing_and_edges_are_not_doubled() {
        assert_eq!(paste("räven", 0..0, "den", false).0, "den räven");
        assert_eq!(paste("räven", 5..5, "sprang", true).0, "räven sprang");
        assert_eq!(paste("", 0..0, "ensam", false).0, "ensam");
        // A line start takes no leading space, and a line end no trailing one.
        assert_eq!(paste("ett\ntvå", 4..4, "x", false).0, "ett\nx två");
        assert_eq!(paste("ett\ntvå", 3..3, "x", true).0, "ett x\ntvå");
    }

    #[test]
    fn the_clipboard_is_trimmed_and_empty_clipboards_are_not_edits() {
        assert_eq!(paste("räven", 0..0, "  den\n ", false).0, "den räven");
        assert_eq!(paste("räven", 0..0, "   ", false).0, "räven");
        assert!(resolve("räven", 0..0, " \n\t ", true).is_none());
    }

    #[test]
    fn multibyte_words_keep_char_boundaries() {
        let text = "höger vänster";
        let caret = 8..8; // inside "vänster", after a multibyte char
        assert_eq!(
            paste(text, caret, "mitten", false).0,
            "höger mitten vänster"
        );
    }
}
