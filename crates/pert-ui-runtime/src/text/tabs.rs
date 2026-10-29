//! Literal tabs are non-printing: each keeps its shaped advance and all Unicode
//! analysis of the original text, but its coverage is skipped.
//!
//! Shaping always sees the unchanged text. Only the private brush of each exact
//! U+0009 byte range is replaced with [`TextBrush::HiddenTab`], after the
//! ordinary span properties, so fonts, sizes, line heights, bidi, grapheme and
//! word boundaries and line breaking are those of the text itself.
//!
//! A registered font may ligate a tab with printable text. Such a shaped atom
//! cannot be split faithfully, and Parley styles a glyph run by its atom's
//! first character, so hiding the tab would erase the neighbor. [`mixed_tabs`]
//! finds those tabs in one walk of the shaped atoms; the layout is then built
//! once more without their overrides and keeps the atom's original ink.

use std::ops::Range;

/// The brush of every Parley style: an index into a prepared text's colors, or
/// a literal tab whose glyphs are skipped before rasterization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum TextBrush {
    /// The resolved color at this index, which may also mark an inline icon.
    Color(usize),
    /// A literal tab outside any mixed shaped atom: shaped and measured, with
    /// no coverage.
    HiddenTab,
}
impl Default for TextBrush {
    /// The inherited foreground, which every resolved text has at index 0.
    fn default() -> Self {
        Self::Color(0)
    }
}

/// A conservative bound on the Parley style table of one admitted text.
///
/// Style runs begin and end only at resolved-span boundaries, at most one per
/// rich segment and the text's ends, and at the two ends of each tab override,
/// with at most one override per admitted byte. The root style and the
/// remaining constant are slack. Parley panics beyond `u16::MAX` styles, and
/// the admission limits keep this bound below that.
const MAX_STYLES: usize =
    2 * crate::TextEditError::MAX_TEXT_BYTES + super::RichText::MAX_SEGMENTS + 4;
const _: () = assert!(MAX_STYLES <= u16::MAX as usize);

/// The ascending byte ranges of `text`'s tabs to hide, consecutive tabs merged
/// into one range, except the tab bytes in `retained` (ascending), which split
/// a merged range. Text without tabs allocates nothing.
pub(super) fn hidden_ranges(text: &str, retained: &[usize]) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut retained = retained.iter().copied().peekable();
    for (index, _) in text.match_indices('\t') {
        while retained.next_if(|kept| *kept < index).is_some() {}
        if retained.next_if_eq(&index).is_some() {
            continue;
        }
        match ranges.last_mut() {
            Some(last) if last.end == index => last.end = index + 1,
            _ => ranges.push(index..index + 1),
        }
    }
    ranges
}

/// The ascending tab byte offsets that share a shaped atom with anything but
/// tabs, found in one logical walk of every line's runs and clusters.
///
/// Atoms never cross a run or line, even in an emergency break, and a cluster
/// that is not a ligature continuation starts one. A tab is a grapheme of its
/// own, so a tab cluster's text is exactly one tab.
pub(super) fn mixed_tabs(layout: &parley::Layout<TextBrush>, text: &str) -> Vec<usize> {
    let mut mixed = Vec::new();
    let mut tabs = Vec::new();
    let mut other = false;
    let mut close = |tabs: &mut Vec<usize>, other: &mut bool| {
        if *other {
            mixed.append(tabs);
        }
        tabs.clear();
        *other = false;
    };
    for line in layout.lines() {
        for run in line.runs() {
            for cluster in run.clusters() {
                if !cluster.is_ligature_continuation() {
                    close(&mut tabs, &mut other);
                }
                let range = cluster.text_range();
                if text.get(range.clone()) == Some("\t") {
                    tabs.push(range.start);
                } else {
                    other = true;
                }
            }
            close(&mut tabs, &mut other);
        }
    }
    // Runs are visited in visual order, which bidi text reverses.
    mixed.sort_unstable();
    mixed
}

#[cfg(test)]
mod tests {
    use super::super::{
        FontData, FontId, Icon, MaskPart, RichText, TextColor, TextLayout, TextOverflow,
        TextRequest, TextService, TextSpan, TextStyle,
    };
    use super::*;
    use crate::{Color, Length, TextMovement, TextSelection};
    use std::{
        cell::Cell,
        collections::{BTreeMap, HashMap},
        sync::Arc,
    };

    type Coverage = HashMap<(i64, i64), u8>;

    /// The synthetic font: U+0009 has a visible glyph and `rlig` ligates
    /// tab+X, X+tab and tab+tab. See `tests/assets/tab-atoms/README.md`.
    const FIXTURE: &[u8] = include_bytes!("../../tests/assets/tab-atoms/TabAtomFixture.ttf");
    /// Glyph IDs in the fixture's glyph order.
    const FIXTURE_TAB: u32 = 2;
    const FIXTURE_X: u32 = 3;
    const FIXTURE_A: u32 = 4;
    const FIXTURE_B: u32 = 5;
    const FIXTURE_TAB_X: u32 = 6;
    const FIXTURE_X_TAB: u32 = 7;
    const FIXTURE_TAB_TAB: u32 = 8;
    const FIXTURE_SPACE: u32 = 1;

    /// A service hiding tabs, and the pre-fix reference that shapes through
    /// the same boundary without the overrides.
    fn services() -> (TextService, TextService) {
        let mut reference = TextService::new().unwrap();
        reference.hide_tabs = false;
        (TextService::new().unwrap(), reference)
    }
    fn rich(
        service: &mut TextService,
        content: &RichText,
        width: Option<f32>,
        scale: f64,
    ) -> TextLayout {
        service
            .prepare_rich(
                content,
                TextRequest::new(TextStyle::Body, 14.0, &Cell::new(0))
                    .width(width)
                    .scale(scale)
                    .inline(16.0, 6.0),
            )
            .unwrap()
    }
    fn fixture(service: &mut TextService) -> FontId {
        service
            .registry
            .register(&mut service.fonts, FontData::from_static(FIXTURE))
            .unwrap()
    }

    /// Every positioned glyph in visual order, hidden or not.
    fn glyphs(layout: &TextLayout) -> Vec<(u32, f32, f32)> {
        layout
            .shaped
            .lines()
            .flat_map(|line| line.items())
            .filter_map(|item| match item {
                parley::PositionedLayoutItem::GlyphRun(run) => Some(
                    run.positioned_glyphs()
                        .map(|g| (g.id, g.x, g.y))
                        .collect::<Vec<_>>(),
                ),
                parley::PositionedLayoutItem::InlineBox(_) => None,
            })
            .flatten()
            .collect()
    }
    /// The glyph IDs a layout's coverage was rasterized from.
    fn drawn(layout: &TextLayout) -> Vec<u32> {
        layout
            .signature
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.0))
            .collect()
    }
    /// The glyph IDs of every cluster that is not a literal tab, from a
    /// reference layout: what hiding must still draw.
    fn printable(layout: &TextLayout, text: &str) -> Vec<u32> {
        let mut ids = Vec::new();
        for line in layout.shaped.lines() {
            for run in line.runs() {
                for cluster in run.visual_clusters() {
                    if text.get(cluster.text_range()) != Some("\t") {
                        ids.extend(cluster.glyphs().map(|g| g.id));
                    }
                }
            }
        }
        ids.sort_unstable();
        ids
    }

    /// The test-only re-walk of a final layout: every atom of tabs alone is
    /// hidden, and no atom holding anything else carries a hidden brush.
    fn assert_final_purity(layout: &TextLayout, text: &str) {
        for line in layout.shaped.lines() {
            for run in line.runs() {
                let mut atoms: Vec<Vec<(bool, TextBrush)>> = Vec::new();
                for cluster in run.clusters() {
                    if !cluster.is_ligature_continuation() || atoms.is_empty() {
                        atoms.push(Vec::new());
                    }
                    let tab = text.get(cluster.text_range()) == Some("\t");
                    atoms.last_mut().unwrap().push((tab, cluster.style().brush));
                }
                for atom in atoms {
                    let pure = atom.iter().all(|(tab, _)| *tab);
                    for (_, brush) in &atom {
                        assert_eq!(
                            *brush == TextBrush::HiddenTab,
                            pure,
                            "{text:?}: atom {atom:?}"
                        );
                    }
                }
            }
        }
    }

    /// Nonzero coverage per brush at physical pixels of the layout's own
    /// coordinates, independent of how far ink grew the canvas.
    fn coverage(layout: &TextLayout, parts: &[MaskPart]) -> BTreeMap<usize, Coverage> {
        let [x, y] = layout
            .origin
            .map(|origin| (origin * layout.scale).round() as i64);
        let mut canvases: BTreeMap<usize, Coverage> = BTreeMap::new();
        for part in parts {
            let canvas = canvases.entry(part.brush).or_default();
            let left = (part.offset[0] * layout.scale).round() as i64 - x;
            let top = (part.offset[1] * layout.scale).round() as i64 - y;
            let w = part.image.width() as usize;
            for (index, value) in part.image.pixels().iter().enumerate() {
                if *value != 0 {
                    let pixel = canvas
                        .entry((left + (index % w) as i64, top + (index / w) as i64))
                        .or_default();
                    *pixel = (*pixel).max(*value);
                }
            }
        }
        canvases
    }
    /// Physical layout rectangles `[left, top, right, bottom]` of every hidden
    /// glyph's advance on its line box. The re-walk proves that only atoms of
    /// tabs alone are hidden.
    fn tab_regions(layout: &TextLayout) -> Vec<[i64; 4]> {
        let mut top = 0.0;
        let mut regions = Vec::new();
        for line in layout.shaped.lines() {
            let bottom = top + line.metrics().line_height;
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(run) = item else {
                    continue;
                };
                if run.style().brush == TextBrush::HiddenTab {
                    regions.extend(run.positioned_glyphs().map(|glyph| {
                        [
                            glyph.x.ceil() as i64,
                            top.ceil() as i64,
                            (glyph.x + glyph.advance).floor() as i64,
                            bottom.floor() as i64,
                        ]
                    }));
                }
            }
            top = bottom;
        }
        regions
    }
    /// The physical canvas `[left, top, right, bottom]` of a prepared layout,
    /// from its public size and origin.
    fn canvas(layout: &TextLayout) -> [i64; 4] {
        let scale = layout.scale;
        let [left, top] = layout.origin.map(|o| (-o * scale).round() as i64);
        let [width, height] = [layout.size().width(), layout.size().height()]
            .map(|extent| (extent * scale).round() as i64);
        [left, top, left + width, top + height]
    }
    /// The canvas the visible-ink policy defines for the pre-fix `reference`,
    /// computed here independently of the implementation's bounds: Parley's
    /// logical box, `width()` by `height()`, joined with the unphased raster
    /// placement of every glyph outside an atom of tabs alone whose raster
    /// has area. A zero-area raster, as a space has, places nothing and adds
    /// no bounds, not even its pen position. Icons are aligned vertically
    /// after shaping, so their glyphs give only horizontal bounds; the second
    /// value is whether every drawn glyph also gave vertical bounds. With
    /// `tab_ink`, the glyphs of tab-only atoms count too: the reference's own
    /// canvas. Without tabs, any layout is its own reference.
    fn expected_canvas(reference: &TextLayout, text: &str, tab_ink: bool) -> ([i64; 4], bool) {
        let shaped = &reference.shaped;
        let mut bounds = [
            0,
            0,
            shaped.width().ceil() as i64,
            shaped.height().ceil() as i64,
        ];
        let mut vertical = true;
        for placed in expected_placements(reference, text, tab_ink) {
            bounds[0] = bounds[0].min(placed.rect[0]);
            bounds[2] = bounds[2].max(placed.rect[2]);
            if placed.icon {
                vertical = false;
            } else {
                bounds[1] = bounds[1].min(placed.rect[1]);
                bounds[3] = bounds[3].max(placed.rect[3]);
            }
        }
        (bounds, vertical)
    }
    /// One raster [`expected_placements`] places.
    struct Placed {
        /// Physical `[left, top, right, bottom]` in Parley coordinates; an
        /// icon's vertical extent is that of its unaligned raster.
        rect: [i64; 4],
        brush: usize,
        icon: bool,
    }
    /// The unphased rasters with area that `reference` places, in the order
    /// the lines, glyph runs and glyphs are visited: every glyph outside an
    /// atom of tabs alone (with `tab_ink`, those too), skipping zero-area
    /// rasters such as a space's. Computed with Swash directly, not through
    /// the service.
    fn expected_placements(reference: &TextLayout, text: &str, tab_ink: bool) -> Vec<Placed> {
        let shaped = &reference.shaped;
        let icons: Vec<usize> = reference
            .signature
            .runs
            .iter()
            .filter(|run| run.alignment.is_some())
            .map(|run| run.brush)
            .collect();
        let mut placed = Vec::new();
        let mut context = swash::scale::ScaleContext::new();
        for line in shaped.lines() {
            // Pen positions of the glyphs of atoms of tabs alone on this line.
            let mut tabs = Vec::new();
            for run in line.runs() {
                let mut atom: Vec<parley::Cluster<'_, TextBrush>> = Vec::new();
                let mut close = |atom: &mut Vec<parley::Cluster<'_, TextBrush>>| {
                    if !tab_ink
                        && atom
                            .iter()
                            .all(|cluster| text.get(cluster.text_range()) == Some("\t"))
                    {
                        for cluster in atom.iter() {
                            let mut x = cluster.visual_offset().unwrap();
                            for glyph in cluster.glyphs() {
                                tabs.push(x + glyph.x);
                                x += glyph.advance;
                            }
                        }
                    }
                    atom.clear();
                };
                for cluster in run.clusters() {
                    if !cluster.is_ligature_continuation() {
                        close(&mut atom);
                    }
                    atom.push(cluster);
                }
                close(&mut atom);
            }
            for item in line.items() {
                let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let TextBrush::Color(brush) = glyph_run.style().brush else {
                    panic!("the reference hides nothing");
                };
                let icon = icons.contains(&brush);
                let run = glyph_run.run();
                let font = run.font();
                let font_ref =
                    swash::FontRef::from_index(font.data.as_ref(), font.index as usize).unwrap();
                let mut scaler = context
                    .builder(font_ref)
                    .size(run.font_size())
                    .hint(true)
                    .normalized_coords(run.normalized_coords().iter().map(|c| c.to_bits()))
                    .build();
                for glyph in glyph_run.positioned_glyphs() {
                    if tabs.iter().any(|x| (x - glyph.x).abs() < 0.01) {
                        continue;
                    }
                    let Some(image) = swash::scale::Render::new(&[swash::scale::Source::Outline])
                        .format(swash::zeno::Format::Alpha)
                        .offset(swash::zeno::Vector::new(
                            glyph.x - glyph.x.floor(),
                            glyph.y - glyph.y.floor(),
                        ))
                        .render(&mut scaler, u16::try_from(glyph.id).unwrap())
                    else {
                        continue;
                    };
                    if image.placement.width == 0 || image.placement.height == 0 {
                        continue;
                    }
                    let left = glyph.x.floor() as i64 + i64::from(image.placement.left);
                    let top = glyph.y.floor() as i64 - i64::from(image.placement.top);
                    placed.push(Placed {
                        rect: [
                            left,
                            top,
                            left + i64::from(image.placement.width),
                            top + i64::from(image.placement.height),
                        ],
                        brush,
                        icon,
                    });
                }
            }
        }
        placed
    }
    /// Asserts the repaired canvas is exactly the policy's, keeping the
    /// reference's vertical extent, and returns whether the old tab ink alone
    /// had defined a canvas edge.
    fn assert_policy_canvas(actual: &TextLayout, expected: &TextLayout, text: &str) -> bool {
        let (policy, vertical) = expected_canvas(expected, text, false);
        let (a, e) = (canvas(actual), canvas(expected));
        assert_eq!(
            [a[0], a[2]],
            [policy[0], policy[2]],
            "{text:?}: horizontal canvas"
        );
        if vertical {
            assert_eq!(
                [a[1], a[3]],
                [policy[1], policy[3]],
                "{text:?}: vertical canvas"
            );
        }
        // Hidden tabs never change the vertical extent, origin or baseline.
        assert_eq!([a[1], a[3]], [e[1], e[3]], "{text:?}");
        assert_eq!(actual.origin[1], expected.origin[1], "{text:?}");
        assert_eq!(actual.size().height(), expected.size().height(), "{text:?}");
        assert_eq!(
            actual.first_baseline(),
            expected.first_baseline(),
            "{text:?}"
        );
        let scale = actual.scale;
        let size =
            crate::Size::new((a[2] - a[0]) as f32 / scale, (a[3] - a[1]) as f32 / scale).unwrap();
        assert_eq!(actual.size(), size, "{text:?}");
        assert_eq!(actual.origin[0], -a[0] as f32 / scale, "{text:?}");
        let overhang = a != e;
        if !overhang {
            assert_eq!(actual.size(), expected.size(), "{text:?}");
            assert_eq!(actual.origin, expected.origin, "{text:?}");
        }
        overhang
    }
    /// The text a rich content is shaped as, icons included.
    fn shaped_text(service: &TextService, content: &RichText) -> String {
        service
            .resolve(
                content,
                TextRequest::new(TextStyle::Body, 14.0, &Cell::new(0)).inline(16.0, 6.0),
            )
            .unwrap()
            .text
    }
    /// A raster signature without the service-specific font handles.
    fn positioned(layout: &TextLayout) -> impl PartialEq + std::fmt::Debug {
        let signature = &layout.signature;
        (
            signature.canvas,
            signature
                .runs
                .iter()
                .map(|run| (run.size, run.glyphs.clone(), run.brush, run.alignment))
                .collect::<Vec<_>>(),
        )
    }
    fn inside(regions: &[[i64; 4]], x: i64, y: i64, inset: i64) -> bool {
        regions
            .iter()
            .any(|[l, t, r, b]| x >= l + inset && x < r - inset && y >= t + inset && y < b - inset)
    }
    /// The largest coverage difference per brush, outside the tab regions.
    fn difference_outside(
        regions: &[[i64; 4]],
        actual: &BTreeMap<usize, Coverage>,
        expected: &BTreeMap<usize, Coverage>,
    ) -> u8 {
        let empty = Coverage::new();
        let mut worst = 0;
        for brush in actual.keys().chain(expected.keys()) {
            let a = actual.get(brush).unwrap_or(&empty);
            let e = expected.get(brush).unwrap_or(&empty);
            for (x, y) in a.keys().chain(e.keys()) {
                if !inside(regions, *x, *y, -1) {
                    let a = a.get(&(*x, *y)).copied().unwrap_or(0);
                    let e = e.get(&(*x, *y)).copied().unwrap_or(0);
                    worst = worst.max(a.abs_diff(e));
                }
            }
        }
        worst
    }
    fn ink_inside(regions: &[[i64; 4]], canvases: &BTreeMap<usize, Coverage>) -> bool {
        canvases
            .values()
            .any(|canvas| canvas.keys().any(|(x, y)| inside(regions, *x, *y, 1)))
    }

    /// Asserts everything the pre-fix reference shows except tab ink: the
    /// policy's size and origin, lines, every glyph, caret stops, movement,
    /// selection and word geometry relative to each layout's origin, with the
    /// tabs' own glyphs the only ones not drawn.
    fn assert_matches_reference(actual: &TextLayout, expected: &TextLayout, text: &str) {
        // Size and origin follow the visible-ink policy exactly. The rest of
        // this comparison is in Parley coordinates: each layout's own origin
        // is removed. Public coordinates at a fixed paint origin are
        // `hanging_tabs_shrink_the_extent_and_translate_right_to_left_public_coordinates`.
        assert_policy_canvas(actual, expected, text);
        // To a thousandth of a logical pixel: removing the origin rounds.
        let unpainted = |layout: &TextLayout, rect: crate::Rect| {
            [
                rect.x() - layout.origin[0],
                rect.y() - layout.origin[1],
                rect.width(),
                rect.height(),
            ]
            .map(|value| (value * 1000.0).round() as i64)
        };
        let caret = |layout: &TextLayout, selection: TextSelection| {
            unpainted(layout, selection.caret(layout).unwrap())
        };
        let rectangles = |layout: &TextLayout, selection: TextSelection| {
            selection
                .rectangles(layout)
                .unwrap()
                .into_iter()
                .map(|rect| unpainted(layout, rect))
                .collect::<Vec<_>>()
        };
        assert_eq!(actual.shaped.width(), expected.shaped.width());
        assert_eq!(actual.shaped.height(), expected.shaped.height());
        let lines = |layout: &TextLayout| {
            layout
                .shaped
                .lines()
                .map(|line| {
                    let m = line.metrics();
                    let runs: Vec<_> = line
                        .runs()
                        .map(|run| (run.text_range(), run.is_rtl(), run.font_size()))
                        .collect();
                    (
                        line.text_range(),
                        m.baseline,
                        m.line_height,
                        m.advance,
                        runs,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(lines(actual), lines(expected), "{text:?}");
        let (a, e) = (glyphs(actual), glyphs(expected));
        assert_eq!(a.len(), e.len(), "{text:?}");
        for (a, e) in a.iter().zip(&e) {
            assert_eq!(a.0, e.0, "{text:?}");
            assert!(
                (a.1 - e.1).abs() < 0.01 && (a.2 - e.2).abs() < 0.01,
                "{text:?}: {a:?} vs {e:?}"
            );
        }
        let mut shown = drawn(actual);
        shown.sort_unstable();
        assert_eq!(shown, printable(expected, text), "{text:?}");
        let movements = [
            TextMovement::Left,
            TextMovement::Right,
            TextMovement::WordLeft,
            TextMovement::WordRight,
            TextMovement::Up,
            TextMovement::Down,
            TextMovement::LineStart,
            TextMovement::LineEnd,
            TextMovement::Start,
            TextMovement::End,
        ];
        for index in 0..=text.len() {
            let (a, e) = (
                TextSelection::snapped(actual, index, index),
                TextSelection::snapped(expected, index, index),
            );
            assert_eq!(a, e, "{text:?} at {index}");
            assert_eq!(caret(actual, a), caret(expected, e), "{text:?} at {index}");
            assert_eq!(
                TextSelection::new(actual, index, index).is_ok(),
                TextSelection::new(expected, index, index).is_ok()
            );
            for movement in movements {
                let extension = crate::SelectionExtension::Extend;
                let (a, e) = (
                    a.moved(actual, movement, extension),
                    e.moved(expected, movement, extension),
                );
                assert_eq!(a, e, "{text:?} {movement:?} from {index}");
                assert_eq!(rectangles(actual, a), rectangles(expected, e));
            }
        }
        assert_eq!(
            rectangles(actual, TextSelection::snapped(actual, 0, text.len())),
            rectangles(expected, TextSelection::snapped(expected, 0, text.len()))
        );
        let (width, height) = (
            actual.shaped.width() / actual.scale,
            actual.shaped.height() / actual.scale,
        );
        for step in 0..12 {
            let at = |layout: &TextLayout| {
                crate::Point::new(
                    width * step as f32 / 11.0 + layout.origin[0],
                    height * (step % 3) as f32 / 2.0 + layout.origin[1],
                )
                .unwrap()
            };
            assert_eq!(
                TextSelection::word_at_point(actual, at(actual)),
                TextSelection::word_at_point(expected, at(expected)),
                "{text:?} word at step {step}"
            );
            assert_eq!(
                TextSelection::default().at_point(
                    actual,
                    at(actual),
                    crate::SelectionExtension::Collapse
                ),
                TextSelection::default().at_point(
                    expected,
                    at(expected),
                    crate::SelectionExtension::Collapse
                ),
            );
        }
        assert_final_purity(actual, text);
    }

    /// A selection's caret offset and affinity.
    fn focus(selection: TextSelection) -> (usize, parley::Affinity) {
        cursor(selection, "focus")
    }
    /// The offset and affinity of a selection's `anchor` or `focus` end.
    /// `TextSelection` keeps its Parley cursors private, and its debug form
    /// names both.
    fn cursor(selection: TextSelection, end: &str) -> (usize, parley::Affinity) {
        let debug = format!("{selection:?}");
        let field = format!("{end}: Cursor {{ index: ");
        let rest = &debug[debug.find(&field).unwrap() + field.len()..];
        let index = rest[..rest.find(',').unwrap()].parse().unwrap();
        let affinity = if rest.starts_with(&format!("{index}, affinity: Upstream")) {
            parley::Affinity::Upstream
        } else {
            assert!(rest.starts_with(&format!("{index}, affinity: Downstream")));
            parley::Affinity::Downstream
        };
        (index, affinity)
    }
    /// The cluster a pointer at Parley `x` on `line` lands on, from that
    /// line's own visual cluster offsets: the cluster under `x`, else the
    /// nearer end cluster; and whether `x` is on its left half.
    fn cluster_at(
        layout: &TextLayout,
        line: usize,
        x: f32,
    ) -> (parley::Cluster<'_, TextBrush>, bool) {
        let line = layout.shaped.get(line).unwrap();
        let clusters: Vec<_> = line
            .runs()
            .flat_map(|run| run.visual_clusters().collect::<Vec<_>>())
            .map(|cluster| (cluster.visual_offset().unwrap(), cluster))
            .collect();
        let (first, last) = (clusters[0], clusters[clusters.len() - 1]);
        if x < first.0 {
            (first.1, true)
        } else if x >= last.0 + last.1.advance() {
            (last.1, false)
        } else {
            let (start, cluster) = clusters
                .into_iter()
                .find(|(start, cluster)| x >= *start && x < start + cluster.advance())
                .unwrap();
            (cluster, x <= start + cluster.advance() * 0.5)
        }
    }
    /// The caret a pointer at Parley `x` on `line` gets: the cluster from
    /// `cluster_at` and Parley's documented side rule. A left-side hit
    /// before a left-to-right cluster, or a right-side hit after a
    /// right-to-left one, is downstream at the cluster's start; the other
    /// sides are upstream at its end.
    fn expected_hit(layout: &TextLayout, line: usize, x: f32) -> (usize, parley::Affinity) {
        let (cluster, left) = cluster_at(layout, line, x);
        let range = cluster.text_range();
        let explicit = cluster.is_line_break() == Some(parley::BreakReason::Explicit);
        if cluster.is_rtl() != left || (!cluster.is_rtl() && explicit) {
            (range.start, parley::Affinity::Downstream)
        } else {
            (range.end, parley::Affinity::Upstream)
        }
    }
    /// A word selection's anchor and focus offsets and affinities.
    type Word = ((usize, parley::Affinity), (usize, parley::Affinity));
    /// Word starts in `text` by the word-boundary rules its test texts
    /// need (UAX #29): letters join, runs of spaces join, and any other
    /// character, a tab or a line break, is a word of its own.
    fn word_starts(text: &str) -> Vec<usize> {
        let class = |c: char| u8::from(c.is_alphanumeric()) + 2 * u8::from(c == ' ');
        let mut starts = Vec::new();
        let mut previous = None;
        for (index, c) in text.char_indices() {
            let current = class(c);
            if previous != Some(current) || current == 0 {
                starts.push(index);
            }
            previous = Some(current);
        }
        starts
    }
    /// The word a pointer at Parley `x` on `line` selects, as its anchor
    /// and focus: the word of `text` around the start of the cluster from
    /// `cluster_at`. In text of one direction both ends are upstream,
    /// entered from inside the word, except a downstream start at offset 0;
    /// Parley gives a focus before a word of the other direction downstream
    /// affinity, so mixed-direction text is refused here.
    fn expected_word(layout: &TextLayout, text: &str, line: usize, x: f32) -> Word {
        let directions: std::collections::BTreeSet<bool> = layout
            .shaped
            .lines()
            .flat_map(|line| line.runs().map(|run| run.is_rtl()).collect::<Vec<_>>())
            .collect();
        assert_eq!(directions.len(), 1, "{text:?}");
        let at = cluster_at(layout, line, x).0.text_range().start;
        let starts = word_starts(text);
        let start = starts.iter().rev().find(|s| **s <= at).copied().unwrap();
        let end = starts
            .iter()
            .find(|s| **s > at)
            .copied()
            .unwrap_or(text.len());
        let affinity = |index: usize| {
            if index == 0 {
                parley::Affinity::Downstream
            } else {
                parley::Affinity::Upstream
            }
        };
        ((start, affinity(start)), (end, parley::Affinity::Upstream))
    }
    /// Asserts a word selection's byte range and both ends.
    fn assert_word(word: TextSelection, expected: Word, context: &str) {
        assert_eq!(
            (
                word.byte_range(),
                cursor(word, "anchor"),
                cursor(word, "focus")
            ),
            (expected.0.0..expected.1.0, expected.0, expected.1),
            "{context}"
        );
    }
    /// Whether a tab cluster lies in a line's hanging whitespace, from
    /// Parley's line metrics: past the end edge, on the right in a
    /// left-to-right paragraph and on the left in a right-to-left one.
    fn tab_hangs(layout: &TextLayout, text: &str, rtl: bool) -> bool {
        layout.shaped.lines().any(|line| {
            let m = line.metrics();
            let (from, to) = if rtl {
                (m.offset, m.offset + m.hanging_advance)
            } else {
                (
                    m.offset + m.advance - m.hanging_advance,
                    m.offset + m.advance,
                )
            };
            m.hanging_advance > 0.0
                && line.runs().any(|run| {
                    run.visual_clusters().any(|cluster| {
                        let x = cluster.visual_offset().unwrap();
                        text.get(cluster.text_range()) == Some("\t")
                            && x < to - 0.01
                            && x + cluster.advance() > from + 0.01
                    })
                })
        })
    }
    /// `rect` moved from one paint origin to another.
    fn translated(rect: crate::Rect, from: [f32; 2], to: [f32; 2]) -> [f32; 4] {
        let [x, y, w, h] = rect.values();
        [x - from[0] + to[0], y - from[1] + to[1], w, h]
    }
    fn assert_near(actual: crate::Rect, expected: [f32; 4], context: &str) {
        let actual = actual.values();
        assert!(
            actual
                .iter()
                .zip(&expected)
                .all(|(a, e)| (a - e).abs() < 1e-3),
            "{context}: {actual:?} vs {expected:?}"
        );
    }

    /// Byte ranges from `(start, end)` pairs.
    fn ranges(pairs: &[(usize, usize)]) -> Vec<Range<usize>> {
        pairs.iter().map(|(start, end)| *start..*end).collect()
    }

    #[test]
    fn consecutive_tabs_merge_and_retained_tabs_split_them() {
        assert_eq!(hidden_ranges("plain", &[]), []);
        assert_eq!(hidden_ranges("plain", &[]).capacity(), 0);
        // (text, retained tab bytes, hidden ranges)
        type Case<'a> = (&'a str, &'a [usize], &'a [(usize, usize)]);
        let cases: [Case<'_>; 6] = [
            ("\ta\t\tb\t", &[], &[(0, 1), (2, 4), (5, 6)]),
            ("\t\t\tX", &[2], &[(0, 2)]),
            ("\t\t\t", &[1], &[(0, 1), (2, 3)]),
            ("\t\t\t", &[0, 1, 2], &[]),
            // Multi-byte neighbors: offsets are bytes.
            ("é\t\u{5d0}\t", &[2], &[(5, 6)]),
            ("\t\t\t\t", &[], &[(0, 4)]),
        ];
        for (text, retained, expected) in cases {
            assert_eq!(hidden_ranges(text, retained), ranges(expected), "{text:?}");
        }
    }

    #[test]
    fn tabs_keep_the_reference_geometry_and_unicode_analysis_without_their_ink() {
        let texts = [
            "\tLeading",
            "A\tB",
            "a\t\t\tb",
            "Name\tValue\tUnit and several more words\tend",
            "x\t\u{301}y",
            "\t\u{301}",
            "\t",
            "\t\t",
            "one\t\ttwo three\t\tfour",
            "e\u{301}\t\u{5d0}\u{5d1}\t123 abc",
            "\u{5d0}\u{5d1}\u{5d2}\tabc\t123",
            "abc \u{5d3}\u{5d4}\t\u{5d5} def\tghi",
            "first\tline\nsecond\t\tline",
        ];
        for style in [TextStyle::Body, TextStyle::Monospace] {
            for scale in [1.0, 1.25, 2.0] {
                let (mut fixed, mut reference) = services();
                for text in texts {
                    for width in [None, Some(220.0), Some(60.0), Some(12.0)] {
                        let shapings = fixed.shapings.get();
                        let builds = fixed.parley_builds.get();
                        let actual = fixed.prepare(text, width, scale, style).unwrap();
                        let expected = reference.prepare(text, width, scale, style).unwrap();
                        // Bundled fonts never ligate a tab: one build per shaping.
                        assert_eq!(fixed.shapings.get() - shapings, 1);
                        assert_eq!(fixed.parley_builds.get() - builds, 1);
                        assert_eq!(mixed_tabs(&expected.shaped, text), []);
                        assert_matches_reference(&actual, &expected, text);
                        let regions = tab_regions(&actual);
                        let a = coverage(&actual, &actual.parts);
                        let e = coverage(&expected, &expected.parts);
                        assert!(
                            difference_outside(&regions, &a, &e) <= 1,
                            "{text:?} {style:?} {scale} {width:?}"
                        );
                        // A combining mark after a tab is a grapheme of its
                        // own; its ink may fall in the tab's advance.
                        if !text.contains('\u{301}') {
                            assert!(!ink_inside(&regions, &a), "{text:?} {style:?} {scale}");
                            // The reference draws the missing-glyph box there.
                            let wide = regions.iter().any(|[l, _, r, _]| r - l > 4);
                            assert_eq!(ink_inside(&regions, &e), wide, "{text:?}");
                        }
                    }
                }
            }
        }
    }

    /// One prepared case of the public-coordinate test: plain text in a
    /// bundled style, or a span in the registered fixture font.
    #[derive(Clone, Copy)]
    enum Hanging {
        Plain(TextStyle),
        Fixture,
    }

    #[test]
    fn hanging_tabs_shrink_the_extent_and_translate_right_to_left_public_coordinates() {
        let arabic = "\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}";
        // (name, text, how, width, right-to-left, a tab hangs, the old tab
        // ink alone defined a canvas edge)
        type Case<'a> = (&'a str, &'a str, Hanging, Option<f32>, bool, bool, bool);
        let mono = Hanging::Plain(TextStyle::Monospace);
        let body = Hanging::Plain(TextStyle::Body);
        let cases: [Case<'_>; 8] = [
            (
                "LTR wrapped, hanging tab on a middle line",
                "aaaa\tbbbb\tcccc",
                mono,
                Some(38.0),
                false,
                true,
                true,
            ),
            (
                "LTR wrapped prose",
                "alpha beta\tgamma delta\tepsilon",
                body,
                Some(70.0),
                false,
                true,
                true,
            ),
            (
                "RTL wrapped, hanging tab on a middle line",
                arabic,
                mono,
                Some(38.0),
                true,
                true,
                true,
            ),
            (
                "RTL tab partly hanging past the text end",
                "\u{5d0}\u{5d1}\u{5d2}\u{5d3}\t",
                mono,
                Some(38.0),
                true,
                true,
                true,
            ),
            (
                "tab before an explicit newline, not overflowing",
                "ab\t\ncd",
                body,
                Some(220.0),
                false,
                false,
                false,
            ),
            (
                "unbounded LTR ending in a tab",
                "abc\t",
                body,
                None,
                false,
                false,
                false,
            ),
            (
                "unbounded RTL ending in a tab",
                "\u{5d0}\u{5d1}\u{5d2}\t",
                body,
                None,
                true,
                false,
                false,
            ),
            // Parley does not hang a tab that continues a ligature with text.
            (
                "fixture mixed atom ending a wrapped line",
                "A X\tB",
                Hanging::Fixture,
                Some(34.0),
                false,
                false,
                false,
            ),
        ];
        for scale in [1.0, 1.25] {
            let (mut fixed, mut reference) = services();
            let (font, reference_font) = (fixture(&mut fixed), fixture(&mut reference));
            let prepare = |service: &mut TextService, font: &FontId, case: &Case<'_>| {
                let (_, text, how, width, ..) = *case;
                match how {
                    Hanging::Plain(style) => service.prepare(text, width, scale, style).unwrap(),
                    Hanging::Fixture => rich(
                        service,
                        &RichText::new().span(TextSpan::new(text).font(font.clone())),
                        width,
                        scale,
                    ),
                }
            };
            let (mut ltr_shrink, mut rtl_shift) = (false, false);
            let mut categories = std::collections::BTreeSet::new();
            let (mut differing, mut literal) = (0, false);
            for case in &cases {
                let (name, text, _, width, rtl, hangs, overhang) = *case;
                let actual = prepare(&mut fixed, &font, case);
                let expected = prepare(&mut reference, &reference_font, case);
                let context = format!("{name} at scale {scale}");
                assert_eq!(tab_hangs(&expected, text, rtl), hangs, "{context}");
                assert_eq!(
                    assert_policy_canvas(&actual, &expected, text),
                    overhang,
                    "{context}"
                );
                if matches!(case.2, Hanging::Fixture) {
                    assert_eq!(
                        drawn(&actual),
                        drawn(&expected),
                        "{context}: the atom keeps its ink"
                    );
                    assert!(drawn(&actual).contains(&FIXTURE_X_TAB), "{context}");
                }
                // Phase-line coverage is placed against the repaired origin:
                // in Parley coordinates it is the reference's, minus tab ink.
                let regions = tab_regions(&actual);
                for phase in [[0, 0], [1, 0], [2, 3], [3, 1]] {
                    let a = coverage(&actual, &fixed.phase_parts(&actual, phase).unwrap());
                    let e = coverage(&expected, &reference.phase_parts(&expected, phase).unwrap());
                    assert!(
                        difference_outside(&regions, &a, &e) <= 1,
                        "{context}: phase {phase:?}"
                    );
                    assert!(!ink_inside(&regions, &a), "{context}: phase {phase:?}");
                }
                let (old, new) = (expected.origin, actual.origin);
                let shift = old[0] - new[0];
                let shrink = expected.size().width() - actual.size().width();
                if overhang && rtl {
                    assert!(shift > 0.0, "{context}: {old:?} -> {new:?}");
                    rtl_shift = true;
                } else {
                    assert_eq!(shift, 0.0, "{context}");
                }
                if overhang && !rtl {
                    assert!(shrink > 0.0, "{context}");
                    ltr_shrink = true;
                }
                println!(
                    "{context}: width {width:?}; size {:?} -> {:?}; origin {old:?} -> {new:?}; \
                     width delta {shrink}, x translation {}",
                    [expected.size().width(), expected.size().height()],
                    [actual.size().width(), actual.size().height()],
                    -shift,
                );
                for (index, line) in actual.shaped.lines().enumerate() {
                    let m = line.metrics();
                    let bounds = |origin: [f32; 2]| {
                        [
                            m.offset / scale as f32 + origin[0],
                            m.block_min_coord / scale as f32 + origin[1],
                            m.advance / scale as f32,
                            (m.block_max_coord - m.block_min_coord) / scale as f32,
                        ]
                    };
                    println!(
                        "  line {index} {:?}: hanging advance {}, public line box {:?} -> {:?}",
                        line.text_range(),
                        m.hanging_advance / scale as f32,
                        bounds(old),
                        bounds(new),
                    );
                }

                // Every caret and selection rectangle is the reference's,
                // moved from the old paint origin to the new one: none is
                // normalized away.
                let stops: Vec<usize> = (0..=text.len())
                    .filter(|index| TextSelection::new(&actual, *index, *index).is_ok())
                    .collect();
                for &from in &stops {
                    let (a, e) = (
                        TextSelection::snapped(&actual, from, from),
                        TextSelection::snapped(&expected, from, from),
                    );
                    assert_eq!(a, e, "{context}");
                    assert_near(
                        a.caret(&actual).unwrap(),
                        translated(e.caret(&expected).unwrap(), old, new),
                        &format!("{context}: caret at {from}"),
                    );
                    for &to in &stops {
                        let (a, e) = (
                            TextSelection::snapped(&actual, from, to),
                            TextSelection::snapped(&expected, from, to),
                        );
                        let (a, e) = (
                            a.rectangles(&actual).unwrap(),
                            e.rectangles(&expected).unwrap(),
                        );
                        assert_eq!(a.len(), e.len(), "{context}");
                        for (a, e) in a.into_iter().zip(e) {
                            assert_near(
                                a,
                                translated(e, old, new),
                                &format!("{context}: {from}..{to}"),
                            );
                        }
                    }
                    for movement in [
                        TextMovement::LineEnd,
                        TextMovement::LineStart,
                        TextMovement::Down,
                    ] {
                        let collapse = crate::SelectionExtension::Collapse;
                        let (a, e) = (
                            TextSelection::snapped(&actual, from, from)
                                .moved(&actual, movement, collapse),
                            TextSelection::snapped(&expected, from, from)
                                .moved(&expected, movement, collapse),
                        );
                        assert_eq!(focus(a), focus(e), "{context}");
                        assert_near(
                            a.caret(&actual).unwrap(),
                            translated(e.caret(&expected).unwrap(), old, new),
                            &format!("{context}: {movement:?} from {from}"),
                        );
                    }
                }
                // The literal values at each hanging line end.
                for (index, line) in actual.shaped.lines().enumerate() {
                    if line.metrics().hanging_advance <= 0.0 {
                        continue;
                    }
                    let range = line.text_range();
                    let end = |layout: &TextLayout| {
                        TextSelection::snapped(layout, range.start, range.start).moved(
                            layout,
                            TextMovement::LineEnd,
                            crate::SelectionExtension::Extend,
                        )
                    };
                    let (a, e) = (end(&actual), end(&expected));
                    println!(
                        "  line {index} LineEnd {:?}: caret {:?} -> {:?}; selection {:?} -> {:?}",
                        focus(a),
                        e.caret(&expected).unwrap().values(),
                        a.caret(&actual).unwrap().values(),
                        e.rectangles(&expected)
                            .unwrap()
                            .iter()
                            .map(|r| r.values())
                            .collect::<Vec<_>>(),
                        a.rectangles(&actual)
                            .unwrap()
                            .iter()
                            .map(|r| r.values())
                            .collect::<Vec<_>>(),
                    );
                }

                // Hit testing and word selection at fixed public points, in
                // four categories. Each layout's expectation comes from its
                // own clusters and its independently expected origin: the
                // repaired one from the policy canvas, the reference's from
                // the same calculation with its tab ink. At the identical
                // point the two may differ by the origin change; the
                // reference must agree with the repaired expectation at the
                // point moved by that change.
                let s = actual.scale;
                let (policy, _) = expected_canvas(&expected, text, false);
                let origin = [-policy[0] as f32 / s, -policy[1] as f32 / s];
                let (with_tab_ink, _) = expected_canvas(&expected, text, true);
                assert_eq!(
                    canvas(&expected),
                    with_tab_ink,
                    "{context}: reference canvas"
                );
                let reference_origin = [-with_tab_ink[0] as f32 / s, -with_tab_ink[1] as f32 / s];
                let public = |x: f32, y: f32| {
                    crate::Point::new(x / s + origin[0], y / s + origin[1]).unwrap()
                };
                let mut points: Vec<(String, usize, crate::Point)> = Vec::new();
                for (index, line) in actual.shaped.lines().enumerate() {
                    let m = *line.metrics();
                    let y = (m.block_min_coord + m.block_max_coord) / 2.0;
                    let clusters: Vec<_> = line
                        .runs()
                        .flat_map(|run| run.visual_clusters().collect::<Vec<_>>())
                        .collect();
                    // (a) Over a visible glyph, whose public x moved when the
                    // origin did.
                    if let Some(cluster) = clusters
                        .iter()
                        .find(|c| text.get(c.text_range()) != Some("\t") && c.advance() > 0.0)
                    {
                        let x = cluster.visual_offset().unwrap() + cluster.advance() * 0.25;
                        points.push((
                            format!("a: glyph {:?} on line {index}", cluster.text_range()),
                            index,
                            public(x, y),
                        ));
                    }
                    // (b) Between the old and the new origin.
                    if shift > 0.0 {
                        let p =
                            crate::Point::new(origin[0] + shift / 2.0, y / s + origin[1]).unwrap();
                        assert!(p.x() >= new[0] && p.x() < old[0]);
                        points.push((format!("b: origin band on line {index}"), index, p));
                    }
                    // (c) Over the hanging region: negative public x in
                    // right-to-left text, beyond the size in left-to-right.
                    for cluster in &clusters {
                        let start = cluster.visual_offset().unwrap();
                        let hanging = if rtl {
                            start < 0.0
                        } else {
                            start + cluster.advance() > actual.shaped.width() + 0.01
                        };
                        if text.get(cluster.text_range()) == Some("\t") && hanging {
                            let x = start + cluster.advance() * if rtl { 0.25 } else { 0.75 };
                            let p = public(x, y);
                            if rtl {
                                assert!(p.x() < 0.0, "{context}: {p:?}");
                            } else {
                                assert!(p.x() > actual.size().width(), "{context}: {p:?}");
                            }
                            points.push((
                                format!(
                                    "c: hanging tab {:?} on line {index}",
                                    cluster.text_range()
                                ),
                                index,
                                p,
                            ));
                        }
                    }
                    // (d) Outside points snap to the nearest caret.
                    points.push((
                        format!("d: far left of line {index}"),
                        index,
                        public(-40.0 * s, y),
                    ));
                    points.push((
                        format!("d: far right of line {index}"),
                        index,
                        public(actual.size().width() * s + 40.0 * s, y),
                    ));
                }
                // Above and below the text, 30% into a middle cluster of
                // the first and the last line, away from cluster boundaries.
                let lines = actual.shaped.len();
                for (name, index, y) in [
                    ("d: above", 0, -20.0 * s),
                    ("d: below", lines - 1, actual.shaped.height() + 20.0 * s),
                ] {
                    let line = actual.shaped.get(index).unwrap();
                    let clusters: Vec<_> = line
                        .runs()
                        .flat_map(|run| run.visual_clusters().collect::<Vec<_>>())
                        .collect();
                    let cluster = &clusters[clusters.len() / 2];
                    let x = cluster.visual_offset().unwrap() + cluster.advance() * 0.3;
                    points.push((name.into(), index, public(x, y)));
                }
                let collapse = crate::SelectionExtension::Collapse;
                for (name, line, p) in &points {
                    let x = (p.x() - origin[0]) * s;
                    let (caret, word) = (
                        expected_hit(&actual, *line, x),
                        expected_word(&actual, text, *line, x),
                    );
                    let x = (p.x() - reference_origin[0]) * s;
                    let (same_caret, same_word) = (
                        expected_hit(&expected, *line, x),
                        expected_word(&expected, text, *line, x),
                    );
                    // The reference at the identical point.
                    let at = format!("{context}: {name}, reference at the same point {p:?}");
                    let hit = TextSelection::default().at_point(&expected, *p, collapse);
                    assert_eq!(focus(hit), same_caret, "{at}");
                    assert_word(TextSelection::word_at_point(&expected, *p), same_word, &at);
                    // The repaired layout.
                    let at = format!("{context}: {name} at {p:?}");
                    let hit = TextSelection::default().at_point(&actual, *p, collapse);
                    assert_eq!(focus(hit), caret, "{at}");
                    assert_word(TextSelection::word_at_point(&actual, *p), word, &at);
                    // The reference at the point moved by the origin change.
                    let moved = crate::Point::new(p.x() - new[0] + old[0], p.y() - new[1] + old[1])
                        .unwrap();
                    let at = format!("{context}: {name}, reference at {moved:?}");
                    let hit = TextSelection::default().at_point(&expected, moved, collapse);
                    assert_eq!(focus(hit), caret, "{at}");
                    assert_word(TextSelection::word_at_point(&expected, moved), word, &at);

                    categories.insert(name[..1].to_owned());
                    if (same_caret, same_word) != (caret, word) {
                        differing += 1;
                    }
                    // The literal values of the review's discriminating point,
                    // at both scales.
                    if case.0 == "RTL wrapped, hanging tab on a middle line"
                        && name.as_str() == "a: glyph 6..8 on line 0"
                    {
                        use parley::Affinity::{Downstream, Upstream};
                        assert!((p.x() - 6.5).abs() < 1e-3 && p.y() == 10.0, "{p:?}");
                        assert_eq!(caret, (8, Upstream));
                        assert_eq!(same_caret, (8, Downstream));
                        assert_eq!(word, ((0, Downstream), (8, Upstream)));
                        assert_eq!(same_word, ((8, Upstream), (9, Upstream)));
                        literal = true;
                    }
                    println!(
                        "  hit {name} at {:?}: {caret:?}, word {word:?}; \
                         reference at the same point {same_caret:?}, word {same_word:?}; \
                         reference at {:?} agrees",
                        [p.x(), p.y()],
                        [moved.x(), moved.y()],
                    );
                }
            }
            assert!(ltr_shrink && rtl_shift, "scale {scale} keeps both deltas");
            assert_eq!(
                categories,
                ["a", "b", "c", "d"].map(String::from).into(),
                "scale {scale} covers every point category"
            );
            assert!(
                differing > 0,
                "scale {scale}: the origin change moves some hits"
            );
            assert!(literal, "scale {scale}: the literal point was tested");
            println!(
                "scale {scale}: categories {categories:?}; {differing} points hit differently \
                 in the reference at the same point"
            );
        }
    }

    /// A space and a hidden tab of the same advance both have no coverage, so
    /// their layouts agree: size, origin and every public caret and selection
    /// rectangle, wrapped or not, left to right or right to left.
    #[test]
    fn hanging_spaces_and_tabs_of_equal_advance_share_public_geometry() {
        let texts = [
            "aaaa\tbbbb\tcccc",
            "\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}",
            "\u{5d0}\u{5d1}\u{5d2}\u{5d3}\t",
            "\t\u{5d0}\u{5d1}",
            "aaaa\t",
            "\t\taaaa\t\t",
            "abc\t\ndef\t\t",
        ];
        let rects = |layout: &TextLayout, selection: TextSelection| {
            selection
                .rectangles(layout)
                .unwrap()
                .iter()
                .map(|r| r.values())
                .collect::<Vec<_>>()
        };
        for scale in [1.0, 1.25] {
            let (mut fixed, mut reference) = services();
            for text in texts {
                let twin = text.replace('\t', " ");
                for width in [None, Some(38.0), Some(12.0)] {
                    let style = TextStyle::Monospace;
                    let tab = fixed.prepare(text, width, scale, style).unwrap();
                    let space = fixed.prepare(&twin, width, scale, style).unwrap();
                    let context = format!("{text:?} at {width:?}, scale {scale}");
                    // The bundled monospace font gives both one cell.
                    assert_eq!(tab.shaped.width(), space.shaped.width(), "{context}");
                    // Each canvas is the independent one: the space's has no
                    // tabs, so it is its own reference.
                    let (policy, vertical) = expected_canvas(&space, &twin, true);
                    assert!(vertical);
                    assert_eq!(canvas(&space), policy, "{context}: space canvas");
                    let expected = reference.prepare(text, width, scale, style).unwrap();
                    assert_policy_canvas(&tab, &expected, text);
                    assert_eq!(tab.size(), space.size(), "{context}");
                    assert_eq!(tab.origin, space.origin, "{context}");
                    for index in 0..=text.len() {
                        let (a, b) = (
                            TextSelection::snapped(&tab, index, index),
                            TextSelection::snapped(&space, index, index),
                        );
                        assert_eq!(focus(a), focus(b), "{context} at {index}");
                        assert_eq!(
                            a.caret(&tab).unwrap(),
                            b.caret(&space).unwrap(),
                            "{context}: caret at {index}"
                        );
                        let collapse = crate::SelectionExtension::Collapse;
                        let end = (
                            a.moved(&tab, TextMovement::LineEnd, collapse),
                            b.moved(&space, TextMovement::LineEnd, collapse),
                        );
                        assert_eq!(end.0.caret(&tab).unwrap(), end.1.caret(&space).unwrap());
                    }
                    assert_eq!(
                        rects(&tab, TextSelection::snapped(&tab, 0, text.len())),
                        rects(&space, TextSelection::snapped(&space, 0, text.len())),
                        "{context}"
                    );
                    println!(
                        "{context}: tab and space size {:?}, origin {:?}",
                        [space.size().width(), space.size().height()],
                        space.origin
                    );
                }
            }
        }
    }

    /// The independent placements' brush chunks: one mask part each.
    fn expected_parts(placed: &[Placed]) -> usize {
        placed.chunk_by(|a, b| a.brush == b.brush).count()
    }

    /// Zero-area rasters neither place a glyph nor extend the canvas, ink or
    /// mask parts, in the default path and at phases without a vertical
    /// offset. Parley's baselines are whole pixels, so the default path renders
    /// a space 0 or 1 pixel wide and 0 high. A vertical phase offsets it by a
    /// fraction, and Swash then renders the empty outline as one pixel of zero
    /// coverage: that raster has area and stays placed, as
    /// `phase_spaces_with_a_vertical_offset_place_one_blank_pixel` records.
    #[test]
    fn empty_rasters_place_nothing_in_bounds_ink_or_mask_parts() {
        let red = Color::from_srgb8(220, 20, 20, 255);
        let blue = Color::from_srgb8(20, 20, 220, 255);
        let hebrew = "\u{5d0}\u{5d1}\u{5d2}";
        let plain = [
            " ",
            "   ",
            "\t ",
            " \t \t",
            "a ",
            " a",
            "  a  b  ",
            "\u{5d0} ",
            " \u{5d0}\u{5d1}",
            "\u{5d0}\u{5d1} \u{5d2}  ",
            "aaaa bbbb cccc",
            "aaaa  bbbb   cccc ",
            "\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}",
            "a\u{E000} b",
            "x \u{301}y ",
            "first line \nsecond  ",
        ];
        let mut contents: Vec<RichText> = plain.iter().map(|t| RichText::from(*t)).collect();
        contents.extend([
            // A space of another color between two same-colored glyphs no
            // longer splits their mask.
            RichText::new()
                .span(TextSpan::new("a").color(red))
                .span(TextSpan::new(" ").color(blue))
                .span(TextSpan::new("b").color(red)),
            RichText::new()
                .text("a ")
                .icon(Icon::Save)
                .text(" ")
                .gap()
                .text(format!(" {hebrew} ")),
            RichText::new().text(" ").icon(Icon::Close).text(" "),
        ]);
        let mut whitespace_only = 0;
        for scale in [1.0, 1.25, 2.0] {
            let (mut fixed, mut reference) = services();
            let font = fixture(&mut fixed);
            let reference_font = fixture(&mut reference);
            let fixture_text = |font: &FontId| {
                RichText::new()
                    .span(TextSpan::new("A X\tB ").font(font.clone()))
                    .text(" end ")
            };
            let mut cases: Vec<(RichText, RichText)> =
                contents.iter().map(|c| (c.clone(), c.clone())).collect();
            cases.push((fixture_text(&font), fixture_text(&reference_font)));
            for (content, reference_content) in &cases {
                let text = shaped_text(&fixed, content);
                for width in [None, Some(38.0), Some(12.0)] {
                    for style_size in [14.0, 26.0] {
                        let prepare = |service: &mut TextService, content: &RichText| {
                            service
                                .prepare_rich(
                                    content,
                                    TextRequest::new(
                                        TextStyle::Monospace,
                                        style_size,
                                        &Cell::new(0),
                                    )
                                    .width(width)
                                    .scale(scale)
                                    .inline(16.0, 6.0),
                                )
                                .unwrap()
                        };
                        let actual = prepare(&mut fixed, content);
                        let expected = prepare(&mut reference, reference_content);
                        let context = format!("{text:?} {width:?} size {style_size} scale {scale}");
                        assert_policy_canvas(&actual, &expected, &text);
                        let placed = expected_placements(&expected, &text, false);
                        // Ink is the union of the placed rasters, or None.
                        if placed.is_empty() {
                            whitespace_only += 1;
                            assert_eq!(actual.ink, None, "{context}");
                            assert!(actual.parts.is_empty(), "{context}");
                            for phase in [[1, 0], [2, 0], [3, 0]] {
                                assert!(
                                    fixed.phase_parts(&actual, phase).unwrap().is_empty(),
                                    "{context}: phase {phase:?}"
                                );
                            }
                            continue;
                        }
                        if placed.iter().all(|p| !p.icon) {
                            let [left, top, ..] = canvas(&actual);
                            let s = actual.scale;
                            let mut ink = [i64::MAX, i64::MAX, i64::MIN, i64::MIN];
                            for p in &placed {
                                ink[0] = ink[0].min(p.rect[0]);
                                ink[1] = ink[1].min(p.rect[1]);
                                ink[2] = ink[2].max(p.rect[2]);
                                ink[3] = ink[3].max(p.rect[3]);
                            }
                            let ink = [
                                (ink[0] - left) as f32 / s,
                                (ink[1] - top) as f32 / s,
                                (ink[2] - left) as f32 / s,
                                (ink[3] - top) as f32 / s,
                            ];
                            assert_eq!(actual.ink, Some(ink), "{context}");
                        } else {
                            assert!(actual.ink.is_some(), "{context}");
                        }
                        assert_eq!(actual.parts.len(), expected_parts(&placed), "{context}");
                        for phase in [[1, 0], [2, 0], [3, 0]] {
                            let parts = fixed.phase_parts(&actual, phase).unwrap();
                            assert_eq!(
                                parts.len(),
                                expected_parts(&placed),
                                "{context}: phase {phase:?}"
                            );
                            // Every phase part covers some pixel: none is a
                            // blank canvas grown by an empty raster.
                            for part in parts.iter() {
                                assert!(part.image.pixels().iter().any(|v| *v != 0));
                            }
                        }
                    }
                }
            }
        }
        assert!(whitespace_only > 0);
        // The colored case: three placements, one brush chunk, one mask.
        let (mut service, _) = services();
        let colored = rich(&mut service, &contents[plain.len()], None, 1.0);
        assert_eq!(colored.parts.len(), 1);
        assert_eq!(colored.parts[0].brush, colored.signature.runs[0].brush);
    }

    /// The phase path caches a zero-area raster like any other, at zero
    /// bytes, before skipping it: another line's miss finds it instead of
    /// rendering the space again.
    #[test]
    fn zero_area_phase_glyphs_stay_cached_and_serve_later_line_misses() {
        let (mut service, _) = services();
        let style = TextStyle::Monospace;
        let first = service.prepare("a b", None, 1.0, style).unwrap();
        // The same cells: the space has the same glyph, size and phase.
        let second = service.prepare("c d", None, 1.0, style).unwrap();
        let empty = |service: &TextService| {
            service
                .phase_glyphs
                .iter()
                .filter(|(_, glyph)| {
                    glyph.image.placement.width == 0 || glyph.image.placement.height == 0
                })
                .map(|(key, glyph)| (key.glyph, key.phase, glyph.image.data.len()))
                .collect::<std::collections::BTreeSet<_>>()
        };
        // Without a vertical phase the space's raster has no area.
        for phase in [[1, 0], [0, 0], [3, 0]] {
            let bytes = service.phase_glyph_bytes;
            let entries = service.phase_glyphs.len();
            let before = empty(&service);
            assert!(!service.phase_parts(&first, phase).unwrap().is_empty());
            let cached = empty(&service);
            let admitted: Vec<_> = cached.iter().filter(|key| !before.contains(key)).collect();
            assert_eq!(admitted.len(), 1, "phase {phase:?}: the space is admitted");
            assert_eq!(admitted[0].2, 0, "at zero bytes");
            assert_eq!(service.phase_glyphs.len(), entries + 3);
            let a_and_b = service.phase_glyph_bytes - bytes;
            // Another line misses its phase-line entry; its two letters are
            // new, its space is not.
            let lines = service.phase_lines.len();
            assert!(!service.phase_parts(&second, phase).unwrap().is_empty());
            assert_eq!(service.phase_lines.len(), lines + 1);
            assert_eq!(service.phase_glyphs.len(), entries + 5, "phase {phase:?}");
            assert_eq!(empty(&service), cached);
            // The first line again, its phase-line entry evicted: no glyph
            // is admitted again.
            service.phase_lines.clear();
            let (order, total) = (service.phase_order.len(), service.phase_glyph_bytes);
            assert!(!service.phase_parts(&first, phase).unwrap().is_empty());
            assert_eq!(service.phase_order.len(), order);
            assert_eq!(service.phase_glyph_bytes, total);
            assert!(total - bytes > a_and_b);
        }
    }

    /// The residue the zero-area test leaves: at a vertical phase a space
    /// renders as one pixel of zero coverage, which is placed like any glyph.
    /// It draws nothing and changes no public size or origin, which come from
    /// the default path, but whitespace alone then has a blank phase mask.
    #[test]
    fn phase_spaces_with_a_vertical_offset_place_one_blank_pixel() {
        let (mut service, _) = services();
        for scale in [1.0, 1.25] {
            let layout = service
                .prepare("   ", None, scale, TextStyle::Monospace)
                .unwrap();
            assert!(layout.parts.is_empty());
            assert_eq!(layout.ink, None);
            for phase in [[1, 2], [2, 3], [3, 1], [0, 2]] {
                let parts = service.phase_parts(&layout, phase).unwrap();
                assert_eq!(parts.len(), 1, "scale {scale} phase {phase:?}");
                assert!(parts[0].image.pixels().iter().all(|v| *v == 0));
                let blank = service
                    .phase_glyphs
                    .values()
                    .filter(|glyph| {
                        let p = glyph.image.placement;
                        (p.width, p.height, &glyph.image.data[..]) == (1, 1, &[0][..])
                    })
                    .count();
                assert!(blank > 0, "scale {scale} phase {phase:?}");
            }
        }
    }

    /// Bundled icons are centered through their ink, which skipping empty
    /// rasters cannot change: none of their glyphs rasterizes empty.
    #[test]
    fn bundled_icons_have_no_empty_raster_so_their_ink_is_unchanged() {
        let (mut service, _) = services();
        for icon in Icon::ALL {
            for size in [16.0, 32.0, 64.0] {
                for scale in [1.0, 1.25] {
                    let layout = service
                        .prepare_rich(
                            &RichText::new().icon(*icon),
                            TextRequest::new(TextStyle::Body, size, &Cell::new(0))
                                .scale(scale)
                                .inline(size, 0.0),
                        )
                        .unwrap();
                    let text = shaped_text(&service, &RichText::new().icon(*icon));
                    let placed = expected_placements(&layout, &text, true);
                    let positioned = glyphs(&layout).len();
                    assert!(positioned > 0);
                    assert_eq!(placed.len(), positioned, "{icon:?} {size} {scale}");
                    let [left, ..] = canvas(&layout);
                    let ink = layout.ink.unwrap();
                    let s = layout.scale;
                    let min = placed.iter().map(|p| p.rect[0]).min().unwrap();
                    let max = placed.iter().map(|p| p.rect[2]).max().unwrap();
                    assert_eq!(
                        [ink[0], ink[2]],
                        [(min - left) as f32 / s, (max - left) as f32 / s],
                        "{icon:?} {size} {scale}"
                    );
                }
            }
        }
    }

    #[test]
    fn tab_only_text_has_no_coverage_but_its_advance_and_caret_stops() {
        let (mut fixed, mut reference) = services();
        for text in ["\t", "\t\t\t"] {
            let actual = fixed.prepare(text, None, 1.0, TextStyle::Body).unwrap();
            let expected = reference.prepare(text, None, 1.0, TextStyle::Body).unwrap();
            assert!(actual.parts.is_empty());
            assert_eq!(actual.ink, None);
            assert!(expected.ink.is_some());
            assert!(actual.size().width() > 0.0);
            assert_eq!(actual.size().width(), actual.shaped.width().ceil());
            assert_eq!(actual.size(), expected.size());
            let carets: Vec<f32> = (0..=text.len())
                .map(|index| {
                    TextSelection::new(&actual, index, index)
                        .unwrap()
                        .caret(&actual)
                        .unwrap()
                        .x()
                })
                .collect();
            assert!(
                carets.windows(2).all(|pair| pair[0] < pair[1]),
                "{carets:?}"
            );
            // Phase-line rendering has nothing to draw either.
            assert!(fixed.phase_parts(&actual, [1, 2]).unwrap().is_empty());
        }
    }

    #[test]
    fn rich_spans_gaps_and_icons_keep_reference_color_size_and_alignment() {
        let red = Color::from_srgb8(220, 20, 20, 255);
        let blue = Color::from_srgb8(20, 20, 220, 255);
        let big = Length::new(22.0).unwrap();
        let small = Length::new(10.0).unwrap();
        let hebrew = "\u{5d0}\u{5d1}\u{5d2}";
        let contents = [
            RichText::new()
                .text("Name\t")
                .span(TextSpan::new("\tValue").color(red).size(big))
                .gap()
                .text("\tafter\t")
                .icon(Icon::Save)
                .text("\tend"),
            // The hidden tab is the only text an icon can align with.
            RichText::new().text("\t").icon(Icon::Save),
            RichText::new().icon(Icon::Add).text("\tX\tY"),
            RichText::new()
                .text("abc\t")
                .icon(Icon::Save)
                .text(format!("\t{hebrew}")),
            RichText::new()
                .text(format!("{hebrew}\t"))
                .icon(Icon::Close)
                .text("\tabc"),
            RichText::new()
                .span(TextSpan::new("a\t").size(big).color(blue))
                .span(TextSpan::new("\tb").size(small).color(red))
                .gap()
                .span(TextSpan::new("\t").color(red))
                .text("c"),
        ];
        for scale in [1.0, 1.25, 2.0] {
            let (mut fixed, mut reference) = services();
            for (index, content) in contents.iter().enumerate() {
                let text = shaped_text(&fixed, content);
                for width in [None, Some(70.0)] {
                    let actual = rich(&mut fixed, content, width, scale);
                    let expected = rich(&mut reference, content, width, scale);
                    assert_matches_reference(&actual, &expected, &text);
                    let alignments = |layout: &TextLayout| {
                        layout
                            .signature
                            .runs
                            .iter()
                            .filter_map(|run| run.alignment.map(|a| (run.brush, a)))
                            .collect::<Vec<_>>()
                    };
                    // All but the last content have an icon.
                    assert_eq!(
                        alignments(&expected).is_empty(),
                        index == contents.len() - 1
                    );
                    assert_eq!(alignments(&actual), alignments(&expected), "{text:?}");
                    let regions = tab_regions(&actual);
                    let a = coverage(&actual, &actual.parts);
                    let e = coverage(&expected, &expected.parts);
                    assert!(
                        difference_outside(&regions, &a, &e) <= 1,
                        "{text:?} {scale}"
                    );
                    assert!(!ink_inside(&regions, &a), "{text:?} {scale}");
                    assert_eq!(*actual.colors, *expected.colors);
                }
            }
        }
    }

    #[test]
    fn a_registered_font_ligating_a_tab_keeps_that_glyph_and_hides_pure_tab_atoms() {
        let (mut fixed, mut reference) = services();
        let font = fixture(&mut fixed);
        let reference_font = fixture(&mut reference);
        // (text, its real shaped glyphs, tabs in mixed atoms, glyphs drawn)
        type Case<'a> = (&'a str, &'a [u32], &'a [usize], &'a [u32]);
        let cases: [Case<'_>; 7] = [
            ("\t", &[FIXTURE_TAB], &[], &[]),
            ("\tX", &[FIXTURE_TAB_X], &[0], &[FIXTURE_TAB_X]),
            ("X\t", &[FIXTURE_X_TAB], &[1], &[FIXTURE_X_TAB]),
            ("\t\t", &[FIXTURE_TAB_TAB], &[], &[]),
            (
                "\t\t\tX",
                &[FIXTURE_TAB_TAB, FIXTURE_TAB_X],
                &[2],
                &[FIXTURE_TAB_X],
            ),
            (
                "A\tB",
                &[FIXTURE_A, FIXTURE_TAB, FIXTURE_B],
                &[],
                &[FIXTURE_A, FIXTURE_B],
            ),
            (
                "A\tB X\t\t\tX \t\tA\t",
                &[
                    FIXTURE_A,
                    FIXTURE_TAB,
                    FIXTURE_B,
                    FIXTURE_SPACE,
                    FIXTURE_X_TAB,
                    FIXTURE_TAB_TAB,
                    FIXTURE_X,
                    FIXTURE_SPACE,
                    FIXTURE_TAB_TAB,
                    FIXTURE_A,
                    FIXTURE_TAB,
                ],
                &[5],
                &[
                    FIXTURE_A,
                    FIXTURE_B,
                    FIXTURE_SPACE,
                    FIXTURE_X_TAB,
                    FIXTURE_X,
                    FIXTURE_SPACE,
                    FIXTURE_A,
                ],
            ),
        ];
        for (text, shaped, mixed, shown) in cases {
            let content = RichText::new().span(TextSpan::new(text).font(font.clone()));
            let reference_content =
                RichText::new().span(TextSpan::new(text).font(reference_font.clone()));
            let expected = rich(&mut reference, &reference_content, None, 1.0);
            // Real GSUB shaping of the fixture, not an assumed ligature.
            let ids: Vec<u32> = glyphs(&expected).iter().map(|g| g.0).collect();
            assert_eq!(ids, shaped, "{text:?}");
            assert_eq!(mixed_tabs(&expected.shaped, text), mixed, "{text:?}");
            let (shapings, builds) = (fixed.shapings.get(), fixed.parley_builds.get());
            let actual = rich(&mut fixed, &content, None, 1.0);
            assert_eq!(fixed.shapings.get() - shapings, 1);
            let fallback = usize::from(!mixed.is_empty());
            assert_eq!(fixed.parley_builds.get() - builds, 1 + fallback, "{text:?}");
            assert_eq!(drawn(&actual), shown, "{text:?}");
            assert_eq!(actual.ink.is_none(), shown.is_empty(), "{text:?}");
            assert_final_purity(&actual, text);
            assert_eq!(actual.size(), expected.size());
            for index in 0..=text.len() {
                assert_eq!(
                    TextSelection::snapped(&actual, index, index).caret(&actual),
                    TextSelection::snapped(&expected, index, index).caret(&expected)
                );
            }
            // A cache hit reaches neither build.
            let builds = fixed.parley_builds.get();
            let again = rich(&mut fixed, &content, None, 1.0);
            assert!(Arc::ptr_eq(&again.parts, &actual.parts));
            assert_eq!(fixed.parley_builds.get(), builds);
        }

        // A mixed atom across a color boundary keeps the color of its first
        // character, exactly as before, and every other pixel.
        let red = Color::from_srgb8(220, 20, 20, 255);
        let blue = Color::from_srgb8(20, 20, 220, 255);
        let colored = |font: &FontId| {
            RichText::new()
                .span(TextSpan::new("A\t").font(font.clone()).color(red))
                .span(TextSpan::new("X B").font(font.clone()).color(blue))
        };
        let expected = rich(&mut reference, &colored(&reference_font), None, 1.0);
        let actual = rich(&mut fixed, &colored(&font), None, 1.0);
        assert_eq!(
            drawn(&expected),
            [FIXTURE_A, FIXTURE_TAB_X, FIXTURE_SPACE, FIXTURE_B]
        );
        assert_eq!(drawn(&actual), drawn(&expected));
        let parts = |layout: &TextLayout| {
            layout
                .parts
                .iter()
                .map(|part| (part.brush, part.offset, part.image.clone()))
                .collect::<Vec<_>>()
        };
        assert_eq!(parts(&actual), parts(&expected));
        let red_index = actual
            .colors
            .iter()
            .position(|c| *c == TextColor::Literal(red))
            .unwrap();
        let mixed_run = actual
            .signature
            .runs
            .iter()
            .find(|run| run.glyphs.iter().any(|glyph| glyph.0 == FIXTURE_TAB_X));
        assert_eq!(mixed_run.unwrap().brush, red_index);
    }

    /// Which font a tab of the mixed-font test is shaped in.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TabFont {
        Body,
        Monospace,
        Fixture,
    }

    /// Body, Monospace and the registered fixture in one rich layout, with
    /// tabs at every kind of font boundary, an icon whose nearest text is a
    /// hidden tab, wrapping and phase rendering.
    fn mixed_fonts(font: &FontId) -> RichText {
        let mono = crate::FontRole::Monospace;
        RichText::new()
            .text("ab\t")
            .span(TextSpan::new("\tcd").font_role(mono))
            .text(" e")
            .span(
                TextSpan::new("\t")
                    .font(font.clone())
                    .size(Length::new(26.0).unwrap()),
            )
            .icon(Icon::Save)
            .span(TextSpan::new("h").size(Length::new(10.0).unwrap()))
            .text(" i\t")
            .span(TextSpan::new("\tAB").font(font.clone()))
            .text(" z")
            .span(TextSpan::new("\t\t").font(font.clone()))
            .text(" ")
            .span(TextSpan::new("A X\tB").font(font.clone()))
            .text(" end")
            .span(TextSpan::new("\t").font_role(mono))
            .text("next")
    }

    #[test]
    fn mixed_fonts_keep_each_tab_s_font_size_and_advance_and_icons_use_hidden_tab_metrics() {
        // (tab byte, where it is, its font, its logical size)
        type Tab = (usize, &'static str, TabFont, f32);
        let tabs: [Tab; 9] = [
            (2, "last byte of a Body span", TabFont::Body, 14.0),
            (
                3,
                "first byte of a Monospace span, after a Body tab",
                TabFont::Monospace,
                14.0,
            ),
            (
                8,
                "a fixture span's only content, the icon's nearest text",
                TabFont::Fixture,
                26.0,
            ),
            (
                15,
                "last byte of a Body span, before a fixture tab",
                TabFont::Body,
                14.0,
            ),
            (16, "first byte of a fixture span", TabFont::Fixture, 14.0),
            (
                21,
                "fixture tab-tab ligature beside Body",
                TabFont::Fixture,
                14.0,
            ),
            (
                22,
                "fixture tab-tab ligature continuation",
                TabFont::Fixture,
                14.0,
            ),
            (27, "fixture X+tab mixed atom", TabFont::Fixture, 14.0),
            (
                33,
                "a Monospace span's only content after Body",
                TabFont::Monospace,
                14.0,
            ),
        ];
        let mixed = 27;
        for scale in [1.0, 1.25] {
            let (mut fixed, mut reference) = services();
            let (font, reference_font) = (fixture(&mut fixed), fixture(&mut reference));
            let text = shaped_text(&fixed, &mixed_fonts(&font));
            let bytes: Vec<usize> = text.match_indices('\t').map(|(index, _)| index).collect();
            assert_eq!(bytes, tabs.map(|tab| tab.0));
            // The font data each service shapes Body, Monospace and the
            // fixture with; a registered font has an identity per service.
            let fonts = |service: &mut TextService, font: &FontId| {
                let first = |layout: &TextLayout| {
                    let run = layout.shaped.lines().next().unwrap().runs().next().unwrap();
                    (run.font().data.id(), run.font().index)
                };
                let body = first(&service.prepare("x", None, scale, TextStyle::Body).unwrap());
                let mono = first(
                    &service
                        .prepare("x", None, scale, TextStyle::Monospace)
                        .unwrap(),
                );
                let registered = first(&rich(
                    service,
                    &RichText::new().span(TextSpan::new("A").font(font.clone())),
                    None,
                    scale,
                ));
                move |kind: TabFont| match kind {
                    TabFont::Body => body,
                    TabFont::Monospace => mono,
                    TabFont::Fixture => registered,
                }
            };
            let (fixed_fonts, reference_fonts) = (
                fonts(&mut fixed, &font),
                fonts(&mut reference, &reference_font),
            );
            let (mut wrapped_atom, mut boundary_hang, mut icon_on_tab) = (false, false, false);
            for width in [None, Some(64.0), Some(120.0)] {
                let actual = rich(&mut fixed, &mixed_fonts(&font), width, scale);
                let expected = rich(&mut reference, &mixed_fonts(&reference_font), width, scale);
                let context = format!("scale {scale} width {width:?}");
                // Each tab's own font, size, coordinates and advance first, so
                // an override touching them fails here.
                for (byte, place, kind, size) in tabs {
                    let facts = |layout: &TextLayout| {
                        let cluster =
                            parley::Cluster::from_byte_index(&layout.shaped, byte).unwrap();
                        let run = cluster.run();
                        (
                            (run.font().data.id(), run.font().index),
                            run.font_size(),
                            run.normalized_coords()
                                .iter()
                                .map(|c| c.to_bits())
                                .collect::<Vec<_>>(),
                            cluster.advance(),
                            cluster.path().line_index(),
                            cluster.visual_offset().unwrap(),
                            cluster.style().brush,
                        )
                    };
                    let (a, e) = (facts(&actual), facts(&expected));
                    assert_eq!(a.0, fixed_fonts(kind), "{context}: {place}");
                    assert_eq!(e.0, reference_fonts(kind), "{context}: {place}");
                    assert_eq!(a.1, size * scale as f32, "{context}: {place}");
                    assert_eq!(
                        (a.1, &a.2, a.3, a.4),
                        (e.1, &e.2, e.3, e.4),
                        "{context}: {place}"
                    );
                    assert!((a.5 - e.5).abs() < 0.01, "{context}: {place}");
                    assert_eq!(
                        a.6 == TextBrush::HiddenTab,
                        byte != mixed,
                        "{context}: {place}"
                    );
                    println!(
                        "{context}: tab {byte} ({place}): {kind:?} font {:?}, size {}, coordinates {:?}, \
                         advance {}, line {}, x {}, brush {:?}",
                        a.0, a.1, a.2, a.3, a.4, a.5, a.6
                    );
                }
                assert_matches_reference(&actual, &expected, &text);
                assert!(drawn(&actual).contains(&FIXTURE_X_TAB), "{context}");
                assert!(!drawn(&actual).contains(&FIXTURE_TAB_TAB), "{context}");
                // Wrapping places the mixed atom at a line end and hangs tabs
                // at font boundaries.
                for line in actual.shaped.lines() {
                    if line.text_range().end == mixed + 1
                        && line.break_reason() == parley::BreakReason::Regular
                    {
                        wrapped_atom = true;
                    }
                }
                for byte in [8, 16, 33] {
                    let cluster = parley::Cluster::from_byte_index(&actual.shaped, byte).unwrap();
                    let x = cluster.visual_offset().unwrap();
                    if width.is_some() && x + cluster.advance() > actual.shaped.width() + 0.01 {
                        boundary_hang = true;
                    }
                }

                // The icon aligns with its nearest text run. Where that is the
                // hidden size-26 tab rather than the size-10 text after the
                // icon, the two centers differ by at least a physical pixel,
                // so skipping hidden runs would move the icon.
                let icon = actual
                    .signature
                    .runs
                    .iter()
                    .find(|run| run.alignment.is_some())
                    .unwrap();
                let reference_icon = expected
                    .signature
                    .runs
                    .iter()
                    .find(|run| run.alignment.is_some())
                    .unwrap();
                assert_eq!(icon.alignment, reference_icon.alignment, "{context}");
                for line in actual.shaped.lines() {
                    let runs: Vec<_> = line
                        .items()
                        .filter_map(|item| match item {
                            parley::PositionedLayoutItem::GlyphRun(run) => Some(run),
                            parley::PositionedLayoutItem::InlineBox(_) => None,
                        })
                        .collect();
                    let Some(at) = runs
                        .iter()
                        .position(|run| run.style().brush == TextBrush::Color(icon.brush))
                    else {
                        continue;
                    };
                    let center = |run: &parley::GlyphRun<'_, TextBrush>| {
                        let metrics = run.run().font_metrics();
                        let cap = metrics
                            .cap_height
                            .filter(|h| h.is_finite() && *h > 0.0)
                            .unwrap_or(run.run().font_size() * 0.7);
                        run.baseline() - cap * 0.5
                    };
                    let target = f32::from_bits(icon.alignment.unwrap()[0]);
                    let text_runs = |hidden: bool| {
                        runs.iter()
                            .enumerate()
                            .filter(|(index, run)| {
                                *index != at
                                    && (hidden || run.style().brush != TextBrush::HiddenTab)
                            })
                            .min_by_key(|(index, _)| index.abs_diff(at))
                            .map(|(_, run)| run)
                    };
                    let nearest = text_runs(true).unwrap();
                    if nearest.style().brush == TextBrush::HiddenTab {
                        icon_on_tab = true;
                        assert_eq!(target, center(nearest), "{context}");
                        let visible = center(text_runs(false).unwrap());
                        assert!(
                            (target - visible).abs() >= 1.0,
                            "{context}: {target} vs {visible}"
                        );
                        println!(
                            "{context}: icon aligns to the hidden tab's center {target}; \
                             the nearest visible run's is {visible}"
                        );
                    }
                }
                // The icon's own coverage is the reference's, pixel for pixel.
                let a = coverage(&actual, &actual.parts);
                let e = coverage(&expected, &expected.parts);
                assert_eq!(
                    a.get(&icon.brush),
                    e.get(&reference_icon.brush),
                    "{context}"
                );
            }
            assert!(
                wrapped_atom && boundary_hang && icon_on_tab,
                "scale {scale}"
            );

            // Phase-line rendering of the wrapped layout, in both orders
            // against a twin whose tabs are a missing character instead.
            let twin = RichText::from(text.replace('\t', "\u{E000}"));
            let expected = rich(
                &mut reference,
                &mixed_fonts(&reference_font),
                Some(64.0),
                scale,
            );
            let phases = [[1, 0], [2, 3], [3, 1], [0, 2]];
            let mut orders = Vec::new();
            for tab_first in [true, false] {
                let (mut service, _) = services();
                let font = fixture(&mut service);
                let actual = rich(&mut service, &mixed_fonts(&font), Some(64.0), scale);
                let twin = rich(&mut service, &twin, Some(64.0), scale);
                let regions = tab_regions(&actual);
                let mut seen = Vec::new();
                for phase in phases {
                    let mut render = |layout: &TextLayout| {
                        coverage(layout, &service.phase_parts(layout, phase).unwrap())
                    };
                    let (a, _) = if tab_first {
                        let a = render(&actual);
                        (a, render(&twin))
                    } else {
                        let t = render(&twin);
                        (render(&actual), t)
                    };
                    let e = coverage(&expected, &reference.phase_parts(&expected, phase).unwrap());
                    assert!(difference_outside(&regions, &a, &e) <= 1, "phase {phase:?}");
                    assert!(!ink_inside(&regions, &a), "phase {phase:?}");
                    seen.push(a);
                }
                orders.push(seen);
            }
            assert_eq!(
                orders[0], orders[1],
                "phase coverage does not depend on cache order"
            );
        }
    }

    #[test]
    fn a_tab_and_a_missing_character_never_share_phase_coverage() {
        let text_style = TextStyle::Body;
        let (_, mut reference) = services();
        let tab = reference.prepare("A\tB", None, 1.0, text_style).unwrap();
        let private = reference
            .prepare("A\u{E000}B", None, 1.0, text_style)
            .unwrap();
        // Before the fix both shape to the same glyph tuples and canvas.
        assert_eq!(tab.signature, private.signature);
        for tab_first in [true, false] {
            let (mut service, _) = services();
            let tab = service.prepare("A\tB", None, 1.0, text_style).unwrap();
            let private = service
                .prepare("A\u{E000}B", None, 1.0, text_style)
                .unwrap();
            assert_ne!(tab.signature, private.signature);
            for _ in 0..2 {
                for phase in [[0, 0], [1, 2], [3, 1]] {
                    let order = if tab_first {
                        [&tab, &private]
                    } else {
                        [&private, &tab]
                    };
                    let parts: Vec<_> = order
                        .iter()
                        .map(|layout| service.phase_parts(layout, phase).unwrap())
                        .collect();
                    let (tab_parts, private_parts) = if tab_first {
                        (&parts[0], &parts[1])
                    } else {
                        (&parts[1], &parts[0])
                    };
                    let regions = tab_regions(&tab);
                    assert!(!ink_inside(&regions, &coverage(&tab, tab_parts)));
                    assert!(ink_inside(&regions, &coverage(&private, private_parts)));
                }
            }
        }
    }

    #[test]
    fn other_unsupported_characters_keep_their_fallback_ink() {
        let (mut fixed, mut reference) = services();
        for text in ["a\u{0B}b", "a\u{E000}b", "a\u{0378}b", "\u{0B}"] {
            for style in [TextStyle::Body, TextStyle::Monospace] {
                let actual = fixed.prepare(text, None, 1.0, style).unwrap();
                let expected = reference.prepare(text, None, 1.0, style).unwrap();
                assert!(actual.ink.is_some(), "{text:?}");
                assert_eq!(positioned(&actual), positioned(&expected), "{text:?}");
                assert_eq!(
                    coverage(&actual, &actual.parts),
                    coverage(&expected, &expected.parts)
                );
                assert_eq!(drawn(&actual).len(), glyphs(&actual).len());
            }
        }
    }

    #[test]
    fn cached_measured_and_ellipsized_tabs_stay_hidden() {
        let (mut fixed, mut reference) = services();
        let reads = Cell::new(0);
        let request = TextRequest::new(TextStyle::Body, 14.0, &reads);
        // The plain cache answers before resolution: no marker in its key.
        let first = fixed.prepare_plain("A\tB", request).unwrap();
        let (resolutions, shapings, builds) = (
            fixed.resolutions.get(),
            fixed.shapings.get(),
            fixed.parley_builds.get(),
        );
        let again = fixed.prepare_plain("A\tB", request).unwrap();
        assert!(Arc::ptr_eq(&first.parts, &again.parts));
        assert_eq!(fixed.resolutions.get(), resolutions);
        assert_eq!(fixed.shapings.get(), shapings);
        assert_eq!(fixed.parley_builds.get(), builds);
        assert_eq!(
            fixed.cache.back().unwrap().key.resolved.spans[0].brush,
            0,
            "resolved spans keep ordinary color indices"
        );

        // fit_whole measures once and reuses that tagged shaping.
        let fit = |service: &mut TextService, text: &str, width: f32| {
            service
                .fit_plain(text, request, width, TextOverflow::Ellipsis)
                .unwrap()
        };
        let text = "Name\tValue";
        let (shapings, builds) = (fixed.shapings.get(), fixed.parley_builds.get());
        let whole = fit(&mut fixed, text, 400.0);
        assert!(!whole.is_truncated());
        assert_eq!(fixed.shapings.get() - shapings, 1);
        assert_eq!(fixed.parley_builds.get() - builds, 1);
        let expected = fit(&mut reference, text, 400.0);
        assert_matches_reference(whole.layout(), expected.layout(), text);
        let regions = tab_regions(whole.layout());
        assert!(!ink_inside(
            &regions,
            &coverage(whole.layout(), &whole.layout().parts)
        ));

        // Fitting compares prepared sizes. A candidate never ends at a tab
        // before the ellipsis, which drops whitespace there; the comparison
        // that meets a trailing tab is the whole text ending in one, at
        // exactly its width, which does not hang. Both choose alike.
        let long = "Name\tValue that is far too long for its column";
        let width = reference
            .prepare("Name\t\u{2026}", None, 1.0, TextStyle::Body)
            .unwrap()
            .size()
            .width();
        let whole = reference
            .prepare("Name\t", None, 1.0, TextStyle::Body)
            .unwrap();
        for (text, width, shown) in [
            (long, width, Some("Name\u{2026}")),
            ("Name\t", whole.size().width(), None),
        ] {
            let actual = fixed
                .fit_plain(text, request, width, TextOverflow::Ellipsis)
                .unwrap();
            let expected = reference
                .fit_plain(text, request, width, TextOverflow::Ellipsis)
                .unwrap();
            assert_eq!(expected.shown(), shown, "{text:?}");
            assert_eq!(actual.shown(), expected.shown(), "{text:?}");
            let fitted = shown.unwrap_or(text);
            assert_matches_reference(actual.layout(), expected.layout(), fitted);
            assert_eq!(actual.layout().size(), expected.layout().size());
            println!(
                "fit {text:?} at width {width}: shown {:?} with size {:?}; reference {:?} with {:?}",
                actual.shown(),
                actual.layout().size(),
                expected.shown(),
                expected.layout().size()
            );
        }

        // An ellipsis candidate keeping a tab, measured by the same boundary.
        let long = "Name\tValue that is far too long for its column";
        for overflow in [TextOverflow::Ellipsis, TextOverflow::MiddleEllipsis] {
            let actual = fixed.fit_plain(long, request, 110.0, overflow).unwrap();
            let expected = reference.fit_plain(long, request, 110.0, overflow).unwrap();
            let shown = actual.shown().unwrap();
            assert!(shown.contains('\t'), "{shown:?}");
            assert_eq!(actual.shown(), expected.shown());
            assert_matches_reference(actual.layout(), expected.layout(), shown);
            let layout = actual.layout();
            let regions = tab_regions(layout);
            assert!(!ink_inside(&regions, &coverage(layout, &layout.parts)));
            assert!(!ink_inside(
                &regions,
                &coverage(layout, &fixed.phase_parts(layout, [2, 1]).unwrap())
            ));
            assert!(ink_inside(
                &regions,
                &coverage(expected.layout(), &expected.layout().parts)
            ));
        }
    }

    #[test]
    fn the_worst_admitted_alternating_tabs_stay_within_the_style_bound() {
        use std::time::Instant;
        let limit = crate::TextEditError::MAX_TEXT_BYTES;
        let plain = "a\t".repeat(limit / 2);
        assert_eq!(plain.len(), limit);
        let segment = "a\t".repeat(limit / RichText::MAX_SEGMENTS / 2);
        let red = Color::from_srgb8(220, 20, 20, 255);
        let blue = Color::from_srgb8(20, 20, 220, 255);
        let spans = (0..RichText::MAX_SEGMENTS).fold(RichText::new(), |content, index| {
            content.span(TextSpan::new(segment.clone()).color(if index % 2 == 0 {
                red
            } else {
                blue
            }))
        });
        assert_eq!(spans.plain_text().len(), limit);
        for (name, content) in [("plain", RichText::from(plain.as_str())), ("spans", spans)] {
            let (mut fixed, mut reference) = services();
            let start = Instant::now();
            let expected = rich(&mut reference, &content, Some(2000.0), 1.0);
            let reference_time = start.elapsed();
            let start = Instant::now();
            let actual = rich(&mut fixed, &content, Some(2000.0), 1.0);
            let time = start.elapsed();
            let styles = actual.shaped.styles().len();
            assert!(styles <= MAX_STYLES, "{styles}");
            assert_eq!(fixed.parley_builds.get(), 1);
            assert!(actual.size().width() <= expected.size().width());
            assert_eq!(actual.shaped.width(), expected.shaped.width());
            assert_eq!(actual.shaped.height(), expected.shaped.height());
            assert_eq!(drawn(&actual).len(), limit / 2);
            println!(
                "worst admitted {name}: {} bytes, {styles} styles (bound {MAX_STYLES}), \
                 {} reference styles; prepare {time:?}, pre-fix reference {reference_time:?} \
                 (one observation, not a bound)",
                content.plain_text().len(),
                expected.shaped.styles().len(),
            );
        }
    }
}
