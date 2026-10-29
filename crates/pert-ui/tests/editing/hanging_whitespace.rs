//! Whitespace hanging past the end of a wrapped line, through the public
//! facade. A TextArea draws, reveals and anchors input methods at a caret
//! pinned inside the text's horizontal box, from zero to the larger of the
//! layout's width and the wrap width. Hanging spaces and tabs keep the caret
//! visible when the viewport can contain it; a narrower viewport retains
//! unavoidable clipping. Outward rounding can cause a bounded shift below one
//! physical pixel, retained for repeated whitespace and removed when the next
//! word wraps. The selection, its fill, the
//! composition underline and hit testing keep their own unpinned geometry.
//! Horizontal wheel scrolling and caret reveal share one range stored with
//! the painted layout. Single-line editors cannot hang and keep their range,
//! now shared by the wheel.
//!
//! Each test runs at scale 1 and a fractional scale. With the `cpu` feature,
//! a visible caret is also proven in pixels: the pixels that change when the
//! caret blinks lie inside the editor's text area.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;
use support::frame;

const SCALES: [f64; 2] = [1.0, 1.25];
/// The default density's control padding around an editor's text area.
const PADDING: [f32; 2] = [12.0, 6.0];
const ARABIC: &str = "\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}";

/// A runtime holding one focused editor keyed `field`.
struct Field {
    ui: Runtime,
    id: WidgetId,
    value: Signal<String>,
    scale: f64,
}
fn mount(
    width: f32,
    scale: f64,
    text: &str,
    build: impl FnOnce(Signal<String>) -> Element,
) -> Field {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, scale).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let id = ui.mount_tree(build(value.clone())).unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    Field {
        ui,
        id,
        value,
        scale,
    }
}
fn area(width: f32, scale: f64, text: &str, style: TextStyle) -> Field {
    mount(width, scale, text, |value| {
        Element::new("field", TextArea::new(value).style(style))
    })
}
impl Field {
    fn key(&mut self, code: KeyCode) {
        support::key(&mut self.ui, code);
        frame(&mut self.ui);
    }
    fn with(&mut self, modifier: KeyCode, code: KeyCode) {
        self.ui.key_input(modifier, KeyAction::Press).unwrap();
        self.key(code);
        self.ui.key_input(modifier, KeyAction::Release).unwrap();
    }
    fn type_text(&mut self, text: &str) {
        self.ui.text_input(text).unwrap();
        frame(&mut self.ui);
    }
    /// One horizontal wheel step over the editor, painted.
    fn wheel(&mut self, dx: f32) -> InputOutcome {
        let outcome = self.wheel_unpainted(dx);
        frame(&mut self.ui);
        outcome
    }
    fn wheel_unpainted(&mut self, dx: f32) -> InputOutcome {
        let [x, y, w, h] = self.ui.bounds(&self.id).unwrap().unwrap().values();
        let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        self.ui
            .pointer_input(PointerEvent::new(center, PointerKind::Move))
            .unwrap();
        self.ui
            .pointer_input(PointerEvent::new(
                center,
                PointerKind::Scroll(Point::new(dx, 0.0).unwrap()),
            ))
            .unwrap()
    }
    fn property(&self, key: &str) -> InspectValue {
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.key == "field")
            .unwrap()
            .details
            .properties[key]
            .clone()
    }
    fn scroll_x(&self) -> f32 {
        match self.property("scroll_x") {
            InspectValue::Number(value) => value as f32,
            other => panic!("scroll_x {other:?}"),
        }
    }
    fn selection(&self) -> String {
        match self.property("selection") {
            InspectValue::Text(value) => value,
            other => panic!("selection {other:?}"),
        }
    }
    /// The editor's text area in window coordinates, from its bounds and the
    /// active control padding, halved in an allocation too small for it.
    fn inner(&self) -> [f32; 4] {
        let [x, y, w, h] = self.ui.bounds(&self.id).unwrap().unwrap().values();
        let theme = self.ui.theme();
        let metrics = theme.metrics(theme.density().unwrap()).unwrap();
        let [px, py] = [
            metrics.metric(MetricRole::ControlPaddingX).min(w / 2.0),
            metrics.metric(MetricRole::ControlPaddingY).min(h / 2.0),
        ];
        [
            x + px,
            y + py,
            (w - 2.0 * px).max(0.0),
            (h - 2.0 * py).max(0.0),
        ]
    }
    /// The editor's prepared text at its current wrap width, through the
    /// public layout.
    fn layout(&mut self, text: &str, style: TextStyle) -> TextLayout {
        let wrap = self.inner()[2].max(1.0);
        self.ui
            .event(&self.id, |cx| cx.text(text, Some(wrap), style))
            .unwrap()
            .unwrap()
    }
    /// What the editor shows now.
    fn seen(&mut self) -> Seen {
        let palette = Palette::dark();
        let (caret, highlight, underline) = (
            palette.color(ColorRole::OnSurface),
            palette.color(ColorRole::TextSelection),
            palette.color(ColorRole::Primary),
        );
        let (mut carets, mut selected, mut underlines) = (Vec::new(), Vec::new(), Vec::new());
        for command in self.ui.paint().unwrap().commands() {
            match command {
                DrawCommand::FillRect { rect, color } if *color == caret => {
                    carets.push(rect.values())
                }
                DrawCommand::FillRect { rect, color } if *color == highlight => {
                    selected.push(rect.values())
                }
                DrawCommand::FillRect { rect, color }
                    if *color == underline && rect.height() == 1.0 =>
                {
                    underlines.push(rect.values())
                }
                _ => {}
            }
        }
        assert!(carets.len() <= 1, "{carets:?}");
        Seen {
            selection: self.selection(),
            scroll_x: self.scroll_x(),
            caret: carets.first().copied(),
            selected,
            underlines,
            ime: self
                .ui
                .text_input_state()
                .map(|(_, state)| state.caret.values()),
        }
    }
    /// Asserts a whole caret is drawn inside the text area, the input-method
    /// anchor is that rectangle and, with the `cpu` feature, that the pixels
    /// it changes lie inside the text area too. Returns the drawn caret.
    #[track_caller]
    fn assert_caret_visible(&mut self, context: &str) -> [f32; 4] {
        let seen = self.seen();
        let [ix, iy, iw, ih] = self.inner();
        let Some(caret @ [x, y, w, h]) = seen.caret else {
            panic!("{context}: no caret drawn: {seen:?}");
        };
        let e = 1e-3;
        assert!(
            (w - 1.0).abs() < e
                && x >= ix - e
                && x + w <= ix + iw + e
                && y >= iy - e
                && y + h <= iy + ih + e,
            "{context}: caret {caret:?} outside {:?}",
            [ix, iy, iw, ih]
        );
        assert!(
            seen.ime.is_some_and(|ime| near(ime, caret)),
            "{context}: input-method anchor {:?} for {caret:?}",
            seen.ime
        );
        #[cfg(feature = "cpu")]
        {
            let s = self.scale;
            let [x0, y0, x1, y1] = self
                .blink_pixels()
                .unwrap_or_else(|| panic!("{context}: no caret pixel visible"));
            let floor = |v: f32| (f64::from(v) * s).floor() as u32;
            let ceil = |v: f32| (f64::from(v) * s).ceil() as u32;
            assert!(
                x0 >= floor(ix) && x1 <= ceil(ix + iw) && y0 >= floor(iy) && y1 <= ceil(iy + ih),
                "{context}: caret pixels {:?} outside the text area",
                [x0, y0, x1, y1]
            );
            assert!(
                x0 >= floor(x) && x1 <= ceil(x + w),
                "{context}: caret pixels {:?} away from the caret {caret:?}",
                [x0, y0, x1, y1]
            );
        }
        caret
    }
    /// The physical bounding box `[x0, y0, x1, y1)` of the pixels one caret
    /// blink changes, then the caret blinks back.
    #[cfg(feature = "cpu")]
    fn blink_pixels(&mut self) -> Option<[u32; 4]> {
        let shown = self.render();
        self.ui
            .flush(self.ui.now() + Duration::from_millis(500))
            .unwrap();
        let hidden = self.render();
        self.ui
            .flush(self.ui.now() + Duration::from_millis(500))
            .unwrap();
        frame(&mut self.ui);
        changed(&shown, &hidden)
    }
    #[cfg(feature = "cpu")]
    fn render(&mut self) -> FrameImage {
        let scene = self.ui.paint().unwrap();
        pert_ui::cpu::CpuRenderer::new().render(&scene).unwrap()
    }
}
/// The physical bounding box of the pixels that differ between two frames.
#[cfg(feature = "cpu")]
fn changed(a: &FrameImage, b: &FrameImage) -> Option<[u32; 4]> {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    let mut bounds: Option<[u32; 4]> = None;
    for (index, (p, q)) in a.pixels().chunks(4).zip(b.pixels().chunks(4)).enumerate() {
        if p != q {
            let (x, y) = (index as u32 % a.width(), index as u32 / a.width());
            let b = bounds.get_or_insert([x, y, x + 1, y + 1]);
            *b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
        }
    }
    bounds
}
/// What an editor shows: its inspected selection and horizontal offset, the
/// caret, selection and composition fills, and the input-method anchor, all
/// in window coordinates.
#[derive(Clone, Debug, PartialEq)]
struct Seen {
    selection: String,
    scroll_x: f32,
    caret: Option<[f32; 4]>,
    selected: Vec<[f32; 4]>,
    underlines: Vec<[f32; 4]>,
    ime: Option<[f32; 4]>,
}
fn near(a: [f32; 4], b: [f32; 4]) -> bool {
    a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 1e-3)
}
/// The horizontal offset left when whitespace at the end of the text fills
/// its line to the wrap width: Parley measures that line as wide as the wrap
/// width, and the measured size rounds it up to whole physical pixels. Zero
/// unless the wrap width is a fractional number of physical pixels.
fn rounding(wrap: f32, scale: f64) -> f32 {
    let physical = f64::from(wrap) * scale;
    ((physical.ceil() - physical) / scale) as f32
}

#[test]
fn hanging_line_ends_draw_a_pinned_caret_inside_the_text_area() {
    for scale in SCALES {
        for text in ["aaaa bbbb cccc", "aaaa\tbbbb\tcccc"] {
            let mut f = area(62.0, scale, text, TextStyle::Monospace);
            let context = format!("{text:?} at scale {scale}");
            let [ix, _, iw, _] = f.inner();
            assert_eq!([ix, iw], [12.0, 38.0]);
            f.with(KeyCode::primary(), KeyCode::Home);
            f.key(KeyCode::End);
            // The line end after the hanging whitespace, at 54 unpinned.
            assert_eq!(f.selection(), "5..5", "{context}");
            let caret = f.assert_caret_visible(&format!("{context}: End"));
            assert!(near(caret, [49.0, 6.0, 1.0, 20.0]), "{context}: {caret:?}");
            assert_eq!(f.scroll_x(), 0.0, "{context}");
            // Extending through the whitespace: the fill is the selection's
            // own geometry, clipped at the text area.
            f.key(KeyCode::Home);
            f.with(KeyCode::ShiftLeft, KeyCode::End);
            let seen = f.seen();
            assert_eq!(seen.selection, "0..5", "{context}");
            assert_eq!(seen.selected, vec![[12.0, 6.0, 38.0, 20.0]], "{context}");
            f.assert_caret_visible(&format!("{context}: Shift+End"));
            // Hit testing keeps the canonical rule: clicking the drawn caret
            // gives the offset the public layout gives at that point, which
            // is not the pinned hanging offset.
            let point = Point::new(caret[0] + 0.5, caret[1] + 10.0).unwrap();
            for kind in [
                PointerKind::Move,
                PointerKind::Press(PointerButton::Primary),
                PointerKind::Release(PointerButton::Primary),
            ] {
                f.ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
            }
            frame(&mut f.ui);
            let layout = f.layout(text, TextStyle::Monospace);
            let local = Point::new(point.x() - ix, point.y() - 6.0).unwrap();
            let hit =
                TextSelection::default().at_point(&layout, local, SelectionExtension::Collapse);
            assert_eq!(f.selection(), format!("{0}..{0}", hit.focus_byte()));
            assert_eq!(
                f.selection(),
                "4..4",
                "{context}: the click lands before the space"
            );
            f.ui.flush(f.ui.now() + Duration::from_secs(2)).unwrap();
            // The literal bytes are untouched.
            assert_eq!(f.value.get().unwrap(), text);
        }
    }
}

#[test]
fn right_to_left_hanging_whitespace_and_the_right_edge_start_show_the_caret() {
    for scale in SCALES {
        for text in [ARABIC.to_owned(), ARABIC.replace(' ', "\t")] {
            let mut f = area(62.0, scale, &text, TextStyle::Monospace);
            let context = format!("{text:?} at scale {scale}");
            // The start caret is at the box's right edge, pinned one caret
            // width inside it.
            f.with(KeyCode::primary(), KeyCode::Home);
            let caret = f.assert_caret_visible(&format!("{context}: start"));
            assert!(near(caret, [49.0, 6.0, 1.0, 20.0]), "{context}: {caret:?}");
            // Left moves forward; End is past the hanging whitespace on the
            // left, at negative x unpinned.
            f.key(KeyCode::End);
            assert_eq!(f.selection(), "9..9", "{context}");
            let caret = f.assert_caret_visible(&format!("{context}: End"));
            assert!(near(caret, [12.0, 6.0, 1.0, 20.0]), "{context}: {caret:?}");
            f.key(KeyCode::Home);
            f.with(KeyCode::ShiftLeft, KeyCode::End);
            let seen = f.seen();
            assert_eq!(seen.selected, vec![[12.0, 6.0, 38.0, 20.0]], "{context}");
            f.assert_caret_visible(&format!("{context}: Shift+End"));
            f.with(KeyCode::primary(), KeyCode::End);
            let caret = f.assert_caret_visible(&format!("{context}: text end"));
            // The space no longer widens the text: its twin's caret is the
            // tab's, where the old empty raster moved it to 37.2.
            assert!(
                near(caret, [33.2, 45.0, 1.0, 20.0]) || scale != 1.0,
                "{caret:?}"
            );
            assert_eq!(f.scroll_x(), 0.0, "{context}");
            assert_eq!(f.wheel(40.0), InputOutcome::Ignored, "{context}");
        }
    }
}

#[test]
fn typing_and_holding_space_keeps_only_the_bounded_rounding_offset() {
    // The second width's wrap is a whole number of physical pixels at both
    // scales; the first's is not at 1.25.
    for scale in SCALES {
        for width in [62.0, 64.0] {
            for space in [" ", "\t"] {
                let mut f = area(width, scale, "", TextStyle::Monospace);
                let wrap = f.inner()[2];
                let context = format!("{space:?} width {width} scale {scale}");
                f.type_text("aaaa");
                assert_eq!(f.scroll_x(), 0.0, "{context}");
                let end = wrap + PADDING[0] - 1.0;
                let mut offsets = Vec::new();
                for count in 1..=6 {
                    f.type_text(space);
                    let caret = f.assert_caret_visible(&format!("{context}: {count} hanging"));
                    offsets.push(f.scroll_x());
                    // Pinned at the box's end, which may exceed the text area
                    // by the measured size's rounding.
                    assert!(
                        near(caret, [end, caret[1], 1.0, 20.0]),
                        "{context}: {caret:?}"
                    );
                }
                // The first whitespace can shift by outward rounding; holding
                // whitespace keeps that bounded offset, and the next word
                // wrapping restores zero below.
                let residual = rounding(wrap, scale);
                assert!(
                    offsets.iter().all(|o| (o - residual).abs() < 1e-3),
                    "{context}: {offsets:?}, rounding {residual}"
                );
                if (f64::from(wrap) * scale).fract() == 0.0 {
                    assert_eq!(residual, 0.0);
                }
                f.type_text("b");
                f.assert_caret_visible(&format!("{context}: wrapped b"));
                assert_eq!(f.scroll_x(), 0.0, "{context}");
                assert_eq!(f.selection(), "11..11");
            }
        }
    }
}

#[test]
fn typing_prose_scrolls_only_by_the_bounded_rounding_residual() {
    for scale in SCALES {
        // One proportional width where a typed space hangs and where a word
        // nearly fills its line.
        let mut f = area(79.0, scale, "", TextStyle::Body);
        let wrap = f.inner()[2];
        let mut offsets = Vec::new();
        for character in "one two three four five six seven".chars() {
            f.type_text(&character.to_string());
            f.assert_caret_visible(&format!("{character:?} at scale {scale}"));
            offsets.push(f.scroll_x());
        }
        let residual = rounding(wrap, scale);
        assert!(
            offsets
                .iter()
                .all(|o| *o == 0.0 || (o - residual).abs() < 1e-3),
            "scale {scale}: {offsets:?}"
        );
        if scale == 1.0 {
            assert!(offsets.iter().all(|o| *o == 0.0), "{offsets:?}");
        }
    }
}

#[test]
fn the_wheel_and_caret_reveal_share_one_nonnegative_range() {
    for scale in SCALES {
        // A wrapped line within a pixel of the wrap width, which nothing
        // overflows: no horizontal range, so the wheel propagates.
        let mut f = area(58.0, scale, "aaaa", TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::End);
        let caret = f.assert_caret_visible("nearly full");
        assert!(near(caret, [45.0, 6.0, 1.0, 20.0]), "{caret:?}");
        assert_eq!(f.scroll_x(), 0.0);
        assert_eq!(f.wheel(40.0), InputOutcome::Ignored);
        assert_eq!(f.wheel(-40.0), InputOutcome::Ignored);
        f.assert_caret_visible("nearly full after the wheel");

        // Hanging whitespace adds no range either.
        let mut f = area(62.0, scale, "aaaa bbbb", TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::Home);
        f.key(KeyCode::End);
        assert_eq!(f.wheel(40.0), InputOutcome::Ignored);
        assert_eq!(f.wheel(-40.0), InputOutcome::Ignored);

        // An unbreakable word overflows: reveal and the wheel stop at the same
        // end, and the start is zero.
        let word = "a".repeat(20);
        let mut f = area(62.0, scale, &word, TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::End);
        f.assert_caret_visible("overflow end");
        let end = f.scroll_x();
        assert!(end > 100.0, "{end}");
        let size = f.layout(&word, TextStyle::Monospace).size().width();
        assert_eq!(end, size - f.inner()[2], "scale {scale}");
        assert_eq!(f.wheel(40.0), InputOutcome::Ignored, "already at the end");
        assert_eq!(f.scroll_x(), end);
        assert_eq!(f.wheel(-1000.0), InputOutcome::Handled);
        assert_eq!(f.scroll_x(), 0.0);
        assert_eq!(
            f.wheel(-40.0),
            InputOutcome::Ignored,
            "already at the start"
        );
        assert_eq!(f.seen().caret, None, "scrolled away from the caret");
        assert_eq!(f.wheel(1000.0), InputOutcome::Handled);
        assert_eq!(f.scroll_x(), end);
        f.wheel(-1000.0);
        // A caret movement reveals the caret again.
        f.key(KeyCode::ArrowLeft);
        f.key(KeyCode::ArrowRight);
        assert_eq!(f.scroll_x(), end);
        f.assert_caret_visible("revealed again");
    }
}

#[test]
fn single_line_reveal_and_the_wheel_share_the_caret_width_range() {
    for scale in SCALES {
        let text = "single line text that overflows";
        let mut f = mount(120.0, scale, text, |value| {
            Element::new("field", TextField::new(value))
        });
        f.with(KeyCode::primary(), KeyCode::Home);
        f.assert_caret_visible("home");
        assert_eq!(f.scroll_x(), 0.0);
        f.key(KeyCode::End);
        let caret = f.assert_caret_visible("end");
        let end = f.scroll_x();
        let size =
            f.ui.event(&f.id, |cx| cx.text(text, None, TextStyle::Body))
                .unwrap()
                .unwrap()
                .size()
                .width();
        // The single-line range keeps its caret-width allowance: it ends one
        // caret width past the measured size, which rounds the ink up to
        // whole pixels, so End reveals the caret at or before that end.
        let max = size + 1.0 - f.inner()[2];
        assert!(end > 0.0 && end <= max + 1e-3, "{end} of {max}");
        // The wheel scrolls within that same range, never back below the
        // revealed offset as it once did; at its end it propagates.
        let outcome = f.wheel(40.0);
        assert_eq!(
            outcome == InputOutcome::Handled,
            end < max,
            "{end} of {max}"
        );
        assert!((f.scroll_x() - max).abs() < 1e-3);
        assert_eq!(f.wheel(40.0), InputOutcome::Ignored);
        assert!(f.seen().caret.is_some_and(|c| c[1] == caret[1]));
        assert_eq!(f.wheel(-1000.0), InputOutcome::Handled);
        assert_eq!(f.scroll_x(), 0.0);
        assert_eq!(f.wheel(-40.0), InputOutcome::Ignored);
        assert_eq!(f.wheel(1000.0), InputOutcome::Handled);
        assert!((f.scroll_x() - max).abs() < 1e-3);
        f.assert_caret_visible("wheeled to the end");
        // Typed trailing spaces cannot hang in a single line: they scroll.
        f.type_text("   ");
        f.assert_caret_visible("trailing spaces");
        assert!(f.scroll_x() > end);
        assert_eq!(f.value.get().unwrap(), format!("{text}   "));
    }
}

#[test]
fn a_wheel_before_the_next_paint_still_cancels_an_edit_s_pending_reveal() {
    for scale in SCALES {
        let word = "a".repeat(20);
        let mut f = area(62.0, scale, &word, TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::End);
        let end = f.scroll_x();
        // The edit's reveal waits for the next paint; the wheel comes first,
        // within the range the last paint stored.
        f.ui.text_input("b").unwrap();
        assert_eq!(f.wheel_unpainted(-40.0), InputOutcome::Handled);
        frame(&mut f.ui);
        assert_eq!(f.selection(), "21..21");
        assert_eq!(f.scroll_x(), end - 40.0);
        assert_eq!(f.seen().caret, None, "the reveal was cancelled");
        f.key(KeyCode::ArrowLeft);
        f.key(KeyCode::ArrowRight);
        f.assert_caret_visible("revealed by the next movement");
        assert!(f.scroll_x() > end);
    }
}

#[test]
fn tiny_and_zero_text_areas_keep_a_valid_range_and_the_caret_end_wins() {
    for scale in SCALES {
        // Text areas 0, 0.5, 1 and 2 pixels wide, and one whose padding is
        // halved.
        for width in [24.0, 24.5, 25.0, 26.0, 20.0] {
            let mut f = area(width, scale, "aa bb", TextStyle::Monospace);
            let [ix, _, iw, _] = f.inner();
            let layout = f.layout("aa bb", TextStyle::Monospace);
            let box_width = layout.size().width().max(iw.max(1.0));
            let collapse = SelectionExtension::Collapse;
            let start = TextSelection::new(&layout, 0, 0).unwrap();
            let line_end = start.moved(&layout, TextMovement::LineEnd, collapse);
            let text_end = start.moved(&layout, TextMovement::End, collapse);
            for (step, offset, selection) in [
                ("start", 0, start),
                ("line end", 3, line_end),
                ("text end", 5, text_end),
            ] {
                match step {
                    "start" => f.with(KeyCode::primary(), KeyCode::Home),
                    "line end" => f.key(KeyCode::End),
                    _ => f.with(KeyCode::primary(), KeyCode::End),
                }
                let context = format!("width {width} scale {scale}: {step}");
                assert_eq!(f.selection(), format!("{offset}..{offset}"), "{context}");
                assert_eq!(selection.focus_byte(), offset);
                let canonical = selection.caret(&layout).unwrap();
                let pinned = canonical.x().clamp(0.0, box_width - 1.0);
                let scroll = f.scroll_x();
                assert!(scroll.is_finite() && scroll >= 0.0, "{context}: {scroll}");
                if iw < 1.0 {
                    // The caret's end stays in view and its start is clipped.
                    assert!(
                        (scroll - (pinned + 1.0 - iw).max(0.0)).abs() < 1e-3,
                        "{context}: {scroll} for a caret at {pinned}"
                    );
                }
                let seen = f.seen();
                if iw == 0.0 {
                    assert_eq!(seen.caret, None, "{context}: nothing to show");
                } else if iw < 1.0 {
                    // The visible part is the caret's end, filling the area.
                    let caret = seen.caret.unwrap_or_else(|| panic!("{context}"));
                    assert!(
                        near(caret, [ix, caret[1], iw, caret[3]]),
                        "{context}: {caret:?}"
                    );
                } else {
                    f.assert_caret_visible(&context);
                }
            }
        }
    }
}

#[test]
fn a_read_only_text_area_shows_its_selection_fill_over_hanging_whitespace() {
    for scale in SCALES {
        let mut f = mount(62.0, scale, "aaaa bbbb cccc", |value| {
            Element::new(
                "field",
                TextArea::new(value).style(TextStyle::Monospace).read_only(),
            )
        });
        #[cfg(feature = "cpu")]
        let plain = f.render();
        f.with(KeyCode::primary(), KeyCode::KeyA);
        let seen = f.seen();
        assert_eq!(seen.selection, "0..14");
        assert_eq!(seen.caret, None, "a read-only field draws no caret");
        assert_eq!(seen.scroll_x, 0.0);
        assert_eq!(seen.selected[0], [12.0, 6.0, 38.0, 20.0]);
        assert_eq!(seen.selected.len(), 3);
        #[cfg(feature = "cpu")]
        {
            // The fill's pixels: inside the text area, from its first line.
            let [x0, y0, x1, y1] = changed(&plain, &f.render()).expect("a visible fill");
            let s = scale;
            assert_eq!(x0, (12.0 * s).floor() as u32);
            assert_eq!(y0, (6.0 * s).floor() as u32);
            assert!(x1 <= (50.0 * s).ceil() as u32 && y1 <= (194.0 * s).ceil() as u32);
        }
    }
}

#[test]
fn resizing_restyling_and_rescaling_keep_the_pinned_caret_visible() {
    for scale in SCALES {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(62.0, 200.0, scale).unwrap())
            .unwrap();
        let value = ui.signal("aaaa bbbb cccc".to_owned());
        let style = ui.signal(TextStyle::Monospace);
        let text = value.clone();
        let root = ui
            .mount_tree(Element::new(
                "dynamic",
                Dynamic::new(style.read_only(), move |_, style| {
                    Element::new("field", TextArea::new(text.clone()).style(*style))
                }),
            ))
            .unwrap();
        frame(&mut ui);
        let id = support::widget(&ui, &root, "field");
        ui.focus(&id).unwrap();
        frame(&mut ui);
        let mut f = Field {
            ui,
            id,
            value,
            scale,
        };
        f.with(KeyCode::primary(), KeyCode::Home);
        f.key(KeyCode::End);
        f.assert_caret_visible("hanging line end");
        let other = if scale == 1.0 { 1.25 } else { 1.0 };
        for name in [
            "widened, nothing hangs",
            "narrowed again",
            "compact density",
            "other scale",
        ] {
            match name {
                "widened, nothing hangs" => {
                    f.ui.set_viewport(Viewport::new(200.0, 200.0, scale).unwrap())
                        .unwrap()
                }
                "narrowed again" => {
                    f.ui.set_viewport(Viewport::new(62.0, 200.0, scale).unwrap())
                        .unwrap()
                }
                "compact density" => {
                    f.ui.theme().set_density(Density::Compact).unwrap();
                }
                _ => {
                    f.ui.set_viewport(Viewport::new(62.0, 200.0, other).unwrap())
                        .unwrap();
                    f.scale = other;
                }
            }
            frame(&mut f.ui);
            f.assert_caret_visible(&format!("scale {scale}: {name}"));
            assert_eq!(f.selection(), "5..5", "{name}");
            assert!(
                f.scroll_x() <= rounding(f.inner()[2], f.scale) + 1e-3,
                "{name}"
            );
        }
        style.set(TextStyle::Body).unwrap();
        frame(&mut f.ui);
        f.ui.focus(&f.id).unwrap();
        frame(&mut f.ui);
        f.assert_caret_visible(&format!("scale {scale}: restyled"));
        assert_eq!(f.value.get().unwrap(), "aaaa bbbb cccc");
    }
}

#[test]
fn composition_pins_the_caret_and_anchor_but_not_the_underline() {
    for scale in SCALES {
        let mut f = area(62.0, scale, "aaaa", TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::End);
        f.ui.text_preedit(&TextPreedit::new(" ", Some(1..1)).unwrap())
            .unwrap();
        frame(&mut f.ui);
        let caret = f.assert_caret_visible(&format!("scale {scale}: composing a space"));
        assert!(near(caret, [49.0, 6.0, 1.0, 20.0]), "{caret:?}");
        // The underline is the composed range's own geometry, unpinned.
        let layout = f.layout("aaaa ", TextStyle::Monospace);
        let [x, y, w, h] = TextSelection::new(&layout, 4, 5)
            .unwrap()
            .rectangles(&layout)
            .unwrap()[0]
            .values();
        let seen = f.seen();
        let origin = [PADDING[0] - seen.scroll_x, PADDING[1]];
        // It runs past the box's end and the pinned caret, where the text
        // area clips it, as it clips a selection fill.
        let [ix, _, iw, _] = f.inner();
        assert!(origin[0] + x + w > ix + iw + 1.0, "{:?}", [x, w]);
        let left = origin[0] + x;
        assert_eq!(seen.underlines.len(), 1);
        assert!(
            near(
                seen.underlines[0],
                [left, origin[1] + y + h - 1.0, ix + iw - left, 1.0]
            ),
            "{:?}",
            seen.underlines
        );
        assert!((seen.scroll_x - rounding(f.inner()[2], scale)).abs() < 1e-3);
        f.ui.text_preedit(&TextPreedit::new("", None).unwrap())
            .unwrap();
        frame(&mut f.ui);
        let caret = f.assert_caret_visible("composition cancelled");
        assert!(near(caret, [45.6, 6.0, 1.0, 20.0]), "{caret:?}");
        assert_eq!(f.value.get().unwrap(), "aaaa");
    }
}

#[test]
fn focus_loss_hides_the_caret_and_relayout_keeps_the_text_still() {
    for scale in SCALES {
        let mut f = area(62.0, scale, "aaaa bbbb cccc", TextStyle::Monospace);
        f.with(KeyCode::primary(), KeyCode::Home);
        f.key(KeyCode::End);
        f.assert_caret_visible("focused");
        pert_ui::testing::Driver::new(&mut f.ui)
            .clear_focus()
            .unwrap();
        frame(&mut f.ui);
        let seen = f.seen();
        assert_eq!((seen.caret, seen.ime), (None, None));
        assert_eq!(seen.scroll_x, 0.0);
        // Arrangement reveals even an unfocused caret; pinned, it moves
        // nothing.
        f.ui.set_viewport(Viewport::new(63.0, 200.0, scale).unwrap())
            .unwrap();
        frame(&mut f.ui);
        assert_eq!(f.scroll_x(), 0.0);
        f.ui.focus(&f.id).unwrap();
        frame(&mut f.ui);
        let caret = f.assert_caret_visible("focus regained");
        assert!(near(caret, [50.0, 6.0, 1.0, 20.0]), "{caret:?}");
        assert_eq!(f.selection(), "5..5");
    }
}
