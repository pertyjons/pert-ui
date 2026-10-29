//! Literal tabs in a TextArea, through the public facade: canonical bytes,
//! caret movement, editing, clipboard and inspection keep U+0009, Tab traverses
//! focus, and the rendered tab is blank between its neighbors. In narrow
//! wrapped text a tab hanging past a line end adds no extent, so it behaves as
//! a hanging space, left to right and right to left alike: the editor gains no
//! horizontal scroll range, right-to-left positions move left by the width its
//! mark used to add, and a caret after it is drawn at the edge of the text's
//! box. `hanging_whitespace` covers that caret pin itself.
//!
//! Multiline paste keeping tabs and single-line paste mapping them to spaces
//! are `text_fields::pasted_control_characters_become_line_breaks_or_are_dropped`;
//! a typed tab in a single-line field becoming a space is
//! `text_fields::text_binding_multiline_defaults_enter_and_undo`.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::frame;

#[derive(Clone, Default)]
struct MemoryClipboard(Rc<RefCell<String>>);
impl Clipboard for MemoryClipboard {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}

fn area(text: &str) -> (Runtime, WidgetId, Signal<String>) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 160.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let id = ui
        .mount_tree(Element::new("field", TextArea::new(value.clone())))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    (ui, id, value)
}
fn key(ui: &mut Runtime, code: KeyCode) {
    support::key(ui, code);
    frame(ui);
}
fn with(ui: &mut Runtime, modifier: KeyCode, code: KeyCode) {
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
}
fn property(ui: &Runtime, key: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .properties[key]
        .clone()
}
fn selection(ui: &Runtime) -> InspectValue {
    property(ui, "selection")
}
fn text(value: &str) -> InspectValue {
    InspectValue::Text(value.into())
}

#[test]
fn a_text_area_edits_copies_and_inspects_literal_tabs_as_canonical_bytes() {
    let initial = "Name\tValue\n\tindented";
    let (mut ui, id, value) = area(initial);
    let clipboard = MemoryClipboard::default();
    ui.set_clipboard(clipboard.clone()).unwrap();
    assert_eq!(property(&ui, "text"), text(initial));
    // One caret stop on each side of a tab, as for any other grapheme.
    with(&mut ui, KeyCode::primary(), KeyCode::KeyA);
    key(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(selection(&ui), text("0..0"));
    for _ in 0..4 {
        key(&mut ui, KeyCode::ArrowRight);
    }
    assert_eq!(selection(&ui), text("4..4"));
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(selection(&ui), text("5..5"));
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::ArrowLeft);
    assert_eq!(selection(&ui), text("5..4"));
    with(&mut ui, KeyCode::primary(), KeyCode::KeyC);
    assert_eq!(*clipboard.0.borrow(), "\t");
    // Deleting the selected tab, undoing it, and Backspace after it.
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(value.get().unwrap(), "NameValue\n\tindented");
    with(&mut ui, KeyCode::primary(), KeyCode::KeyZ);
    assert_eq!(value.get().unwrap(), initial);
    with(&mut ui, KeyCode::primary(), KeyCode::KeyA);
    key(&mut ui, KeyCode::ArrowLeft);
    key(&mut ui, KeyCode::ArrowDown);
    key(&mut ui, KeyCode::Home);
    key(&mut ui, KeyCode::ArrowRight);
    assert_eq!(selection(&ui), text("12..12"));
    key(&mut ui, KeyCode::Backspace);
    assert_eq!(value.get().unwrap(), "Name\tValue\nindented");
    // Pasting tabs into the text area keeps them; copying all returns them.
    *clipboard.0.borrow_mut() = "\t\tX\t".into();
    assert_eq!(
        ui.text_command(&id, TextCommand::Paste).unwrap(),
        TextCommandOutcome::Handled
    );
    frame(&mut ui);
    let pasted = "Name\tValue\n\t\tX\tindented";
    assert_eq!(value.get().unwrap(), pasted);
    assert_eq!(property(&ui, "text"), text(pasted));
    assert_eq!(
        ui.text_command(&id, TextCommand::SelectAll).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(
        ui.text_command(&id, TextCommand::Copy).unwrap(),
        TextCommandOutcome::Handled
    );
    assert_eq!(*clipboard.0.borrow(), pasted);
    // The saved bytes are the canonical text, U+0009 included.
    let saved = value.get().unwrap().into_bytes();
    assert_eq!(saved.iter().filter(|byte| **byte == b'\t').count(), 4);
    assert_eq!(saved, pasted.as_bytes());
    let semantic = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .unwrap()
        .details
        .semantics
        .unwrap()
        .value;
    assert_eq!(semantic, Some(SemanticValue::Text(pasted.into())));
}

#[test]
fn keyboard_tab_moves_focus_between_text_areas_without_inserting_a_tab() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 320.0, 1.0).unwrap())
        .unwrap();
    let first = ui.signal("a\tb".to_owned());
    let second = ui.signal(String::new());
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new("first", TextArea::new(first.clone())),
                Element::new("second", TextArea::new(second.clone())),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    let first_id = support::widget(&ui, &root, "first");
    let second_id = support::widget(&ui, &root, "second");
    ui.focus(&first_id).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(second_id));
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::Tab);
    assert_eq!(ui.focused(), Some(first_id));
    assert_eq!(first.get().unwrap(), "a\tb");
    assert_eq!(second.get().unwrap(), "");
}

/// Renders an unfocused monospace TextArea holding `value` on the CPU.
#[cfg(feature = "cpu")]
fn rendered(value: &str) -> FrameImage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(160.0, 60.0, 2.0).unwrap())
        .unwrap();
    let signal = ui.signal(value.to_owned());
    ui.mount_tree(Element::new("field", TextArea::new(signal)))
        .unwrap();
    ui.flush(std::time::Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    pert_ui::cpu::CpuRenderer::new().render(&scene).unwrap()
}

/// In the bundled monospace font a tab, a missing private-use character and a
/// space advance equally, so the three renderings differ only in the middle
/// cell's ink.
#[cfg(feature = "cpu")]
#[test]
fn a_rendered_tab_is_blank_while_its_neighbors_stay_in_place() {
    let tab = rendered("A\tB");
    let space = rendered("A B");
    let missing = rendered("A\u{E000}B");
    assert_eq!((tab.width(), tab.height()), (space.width(), space.height()));
    // The tab draws nothing: the same pixels as a space, so A and B are
    // drawn where they were.
    assert_eq!(tab.pixels(), space.pixels());
    // Another unsupported character keeps its missing-glyph mark there.
    let differing = tab
        .pixels()
        .chunks(4)
        .zip(missing.pixels().chunks(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(differing > 20, "{differing} pixels differ");
    // The text is drawn at all: it differs from an empty field.
    assert_ne!(tab.pixels(), rendered("").pixels());
}

#[test]
fn a_full_text_area_of_tab_separated_columns_paints_and_edits() {
    use std::time::Instant;
    // One prepared layout is bounded by `TextLimit::MAX_RASTER_SIDE` before
    // the byte limit: about 400 monospace lines, whatever they hold.
    let row = "name\tvalue\tunit\t42\n";
    let rows = 300;
    let content = row.repeat(rows);
    assert!(content.len() <= TextEditError::MAX_TEXT_BYTES);
    assert_eq!(TextArea::check(&content), Ok(()));
    let start = Instant::now();
    let (mut ui, _, value) = area(&content);
    let first = start.elapsed();
    let start = Instant::now();
    ui.text_input("\t").unwrap();
    frame(&mut ui);
    let edit = start.elapsed();
    assert_eq!(value.get().unwrap(), format!("\t{content}"));
    println!(
        "TextArea with {} bytes ({rows} rows, {} tabs): mount and first frame {first:?}, \
         one edit frame {edit:?} (one observation, not a bound)",
        content.len(),
        content.matches('\t').count(),
    );
}

/// A focused monospace TextArea `width` logical pixels wide, holding `text`.
fn narrow(text: &str, width: f32) -> (Runtime, WidgetId) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let id = ui
        .mount_tree(Element::new(
            "field",
            TextArea::new(value).style(TextStyle::Monospace),
        ))
        .unwrap();
    ui.focus(&id).unwrap();
    frame(&mut ui);
    (ui, id)
}
/// The editor's inspected selection and horizontal scroll offset, its caret
/// fill if any is left after clipping, and its selection fills.
#[derive(Debug, PartialEq)]
struct Painted {
    selection: InspectValue,
    scroll_x: InspectValue,
    caret: Option<[f32; 4]>,
    selected: Vec<[f32; 4]>,
}
/// What the editor paints, with the text's clip, which is its inner area.
fn painted(ui: &mut Runtime) -> (Painted, [f32; 4]) {
    let palette = Palette::dark();
    let (caret, highlight) = (
        palette.color(ColorRole::OnSurface),
        palette.color(ColorRole::TextSelection),
    );
    let (mut carets, mut selected, mut inner) = (Vec::new(), Vec::new(), None);
    for command in ui.paint().unwrap().commands() {
        match command {
            DrawCommand::FillRect { rect, color } if *color == caret => carets.push(rect.values()),
            DrawCommand::FillRect { rect, color } if *color == highlight => {
                selected.push(rect.values())
            }
            // The text mask is clipped to the inner area; selected text is
            // clipped further to its selection.
            DrawCommand::DrawMask {
                clip: Some(clip), ..
            } if inner.is_none() && clip.width() > 8.0 => inner = Some(clip.values()),
            _ => {}
        }
    }
    assert!(carets.len() <= 1, "{carets:?}");
    (
        Painted {
            selection: selection(ui),
            scroll_x: property(ui, "scroll_x"),
            caret: carets.first().copied(),
            selected,
        },
        inner.unwrap(),
    )
}
/// Whether a painted caret is whole and inside the inner area.
fn revealed(caret: Option<[f32; 4]>, [ix, iy, iw, ih]: [f32; 4]) -> bool {
    caret.is_some_and(|[x, y, w, h]| {
        w == 1.0 && x >= ix && x + w <= ix + iw && y >= iy && y + h <= iy + ih
    })
}
/// Caret rectangles equal to a thousandth of a logical pixel, or both absent.
fn near(a: Option<[f32; 4]>, b: Option<[f32; 4]>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.iter().zip(&b).all(|(a, b)| (a - b).abs() < 1e-3),
        (a, b) => a.is_none() && b.is_none(),
    }
}
fn number(value: f64) -> InspectValue {
    InspectValue::Number(value)
}
fn scroll_sideways(ui: &mut Runtime, id: &WidgetId) {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    ui.pointer_input(PointerEvent::new(center, PointerKind::Move))
        .unwrap();
    ui.pointer_input(PointerEvent::new(
        center,
        PointerKind::Scroll(Point::new(40.0, 0.0).unwrap()),
    ))
    .unwrap();
    frame(ui);
}

/// The left-to-right steps of the narrow-editor tests: each label, then what
/// the editor paints after it.
fn ltr_steps(text: &str) -> Vec<(&'static str, Painted, [f32; 4])> {
    let (mut ui, id) = narrow(text, 62.0);
    let mut steps = Vec::new();
    let mut step = |ui: &mut Runtime, name| {
        let (painted, inner) = painted(ui);
        steps.push((name, painted, inner));
    };
    with(&mut ui, KeyCode::primary(), KeyCode::Home);
    key(&mut ui, KeyCode::ArrowRight);
    key(&mut ui, KeyCode::ArrowRight);
    step(&mut ui, "interior");
    key(&mut ui, KeyCode::ArrowRight);
    key(&mut ui, KeyCode::ArrowRight);
    step(&mut ui, "before the hanging whitespace");
    key(&mut ui, KeyCode::End);
    step(&mut ui, "line end after the hanging whitespace");
    key(&mut ui, KeyCode::Home);
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::End);
    step(&mut ui, "selection with the hanging whitespace");
    with(&mut ui, KeyCode::primary(), KeyCode::End);
    step(&mut ui, "text end");
    scroll_sideways(&mut ui, &id);
    step(&mut ui, "after a sideways scroll");
    steps
}

#[test]
fn a_narrow_text_area_treats_a_hanging_tab_as_a_hanging_space() {
    let tab = ltr_steps("aaaa\tbbbb\tcccc");
    let space = ltr_steps("aaaa bbbb cccc");
    // Four monospace cells fit the 38-pixel inner width, the fifth hangs.
    let inner = [12.0, 6.0, 38.0, 188.0];
    let expected = [
        ("interior", "2..2", Some([28.8, 6.0, 1.0, 20.0]), vec![]),
        (
            "before the hanging whitespace",
            "4..4",
            Some([45.6, 6.0, 1.0, 20.0]),
            vec![],
        ),
        // After the hanging tab the caret is drawn at the inner area's end
        // without scrolling; the selection fill keeps its own geometry.
        (
            "line end after the hanging whitespace",
            "5..5",
            Some([49.0, 6.0, 1.0, 20.0]),
            vec![],
        ),
        (
            "selection with the hanging whitespace",
            "0..5",
            Some([49.0, 6.0, 1.0, 20.0]),
            vec![[12.0, 6.0, 38.0, 20.0]],
        ),
        ("text end", "14..14", Some([45.6, 45.0, 1.0, 20.0]), vec![]),
        (
            "after a sideways scroll",
            "14..14",
            Some([45.6, 45.0, 1.0, 20.0]),
            vec![],
        ),
    ];
    assert_eq!(tab.len(), expected.len());
    for ((name, painted, area), (label, range, caret, selected)) in tab.iter().zip(expected) {
        assert_eq!(*name, label);
        assert_eq!(*area, inner, "{name}");
        assert_eq!(painted.selection, text(range), "{name}");
        // No horizontal scroll range: the tab's mark no longer widens the
        // text past the inner width.
        assert_eq!(painted.scroll_x, number(0.0), "{name}");
        assert!(near(painted.caret, caret), "{name}: {:?}", painted.caret);
        assert_eq!(painted.selected, selected, "{name}");
        assert!(revealed(painted.caret, *area), "{name}");
    }
    // The hanging-space twin paints and scrolls the same at every step.
    assert_eq!(tab, space);
}

#[test]
fn typing_a_tab_at_a_full_line_end_matches_typing_a_space() {
    let typed = |pieces: &[&str]| {
        let (mut ui, _) = narrow("", 62.0);
        pieces
            .iter()
            .map(|piece| {
                ui.text_input(piece).unwrap();
                frame(&mut ui);
                painted(&mut ui).0
            })
            .collect::<Vec<_>>()
    };
    let tab = typed(&["aaaa", "\t", "b"]);
    assert_eq!(tab, typed(&["aaaa", " ", "b"]));
    // The typed tab hangs at the end of the text and its caret is drawn at the
    // inner area's end, as a space's is; neither it nor the next character,
    // which wraps, moves the text sideways.
    assert_eq!(tab[1].selection, text("5..5"));
    assert_eq!(tab[1].scroll_x, number(0.0));
    assert!(
        near(tab[1].caret, Some([49.0, 6.0, 1.0, 20.0])),
        "{:?}",
        tab[1]
    );
    assert_eq!(tab[2].selection, text("6..6"));
    assert_eq!(tab[2].scroll_x, number(0.0));
    assert!(
        near(tab[2].caret, Some([20.4, 26.0, 1.0, 20.0])),
        "{:?}",
        tab[2]
    );
}

/// The right-to-left steps of the narrow-editor test: each label, then what
/// the editor paints after it.
fn rtl_steps(text: &str) -> Vec<(&'static str, Painted, [f32; 4])> {
    let (mut ui, id) = narrow(text, 62.0);
    let mut steps = Vec::new();
    let mut step = |ui: &mut Runtime, name| {
        let (painted, inner) = painted(ui);
        steps.push((name, painted, inner));
    };
    with(&mut ui, KeyCode::primary(), KeyCode::Home);
    step(&mut ui, "text start at the right edge");
    // Left moves forward in right-to-left text.
    key(&mut ui, KeyCode::ArrowLeft);
    key(&mut ui, KeyCode::ArrowLeft);
    step(&mut ui, "interior");
    key(&mut ui, KeyCode::ArrowLeft);
    key(&mut ui, KeyCode::ArrowLeft);
    step(&mut ui, "before the hanging whitespace");
    key(&mut ui, KeyCode::End);
    step(&mut ui, "line end after the hanging whitespace");
    key(&mut ui, KeyCode::Home);
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::End);
    step(&mut ui, "selection with the hanging whitespace");
    with(&mut ui, KeyCode::primary(), KeyCode::End);
    step(&mut ui, "text end");
    scroll_sideways(&mut ui, &id);
    step(&mut ui, "after a sideways scroll");
    steps
}

#[test]
fn a_narrow_right_to_left_text_area_keeps_its_hanging_tab_in_the_extent_of_a_space() {
    let tab =
        rtl_steps("\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}");
    let space =
        rtl_steps("\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}");
    let inner = [12.0, 6.0, 38.0, 188.0];
    let expected = [
        // The start caret, on the right edge, is drawn at the inner end.
        (
            "text start at the right edge",
            "0..0",
            Some([49.0, 6.0, 1.0, 20.0]),
            vec![],
        ),
        ("interior", "4..4", Some([33.2, 6.0, 1.0, 20.0]), vec![]),
        (
            "before the hanging whitespace",
            "8..8",
            Some([16.4, 6.0, 1.0, 20.0]),
            vec![],
        ),
        // The line end after the hanging tab, left of the text's box, is
        // drawn at the inner area's start.
        (
            "line end after the hanging whitespace",
            "9..9",
            Some([12.0, 6.0, 1.0, 20.0]),
            vec![],
        ),
        (
            "selection with the hanging whitespace",
            "0..9",
            Some([12.0, 6.0, 1.0, 20.0]),
            vec![[12.0, 6.0, 38.0, 20.0]],
        ),
        // The last line's caret: the pre-fix tab origin placed it 3 pixels
        // further right, at 36.2.
        ("text end", "22..22", Some([33.2, 45.0, 1.0, 20.0]), vec![]),
        (
            "after a sideways scroll",
            "22..22",
            Some([33.2, 45.0, 1.0, 20.0]),
            vec![],
        ),
    ];
    assert_eq!(tab.len(), expected.len());
    for ((name, painted, area), (label, range, caret, selected)) in tab.iter().zip(expected) {
        assert_eq!(*name, label);
        assert_eq!(*area, inner, "{name}");
        assert_eq!(painted.selection, text(range), "{name}");
        assert_eq!(painted.scroll_x, number(0.0), "{name}");
        assert!(near(painted.caret, caret), "{name}: {:?}", painted.caret);
        assert_eq!(painted.selected, selected, "{name}");
        assert!(revealed(painted.caret, *area), "{name}");
    }
    // The hanging-space twin paints and scrolls the same at every step: an
    // empty space raster no longer widens the text where the tab does not.
    assert_eq!(tab, space);
}

#[test]
fn right_to_left_selectable_text_selects_at_fixed_points_after_the_origin_moved() {
    let value = "\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}";
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(38.0, 200.0, 1.0).unwrap())
        .unwrap();
    let signal = ui.signal(value.to_owned());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([Element::new(
                "text",
                SelectableText::new(signal.read_only()).style(TextStyle::Monospace),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    let id = support::widget(&ui, &root, "text");
    // The hanging tab adds no width: 37 pixels of ink in a 38-pixel
    // allocation, where its mark measured 40. A hanging space, whose empty
    // raster once measured 42, now measures the same.
    let bounds = ui.bounds(&id).unwrap().unwrap();
    assert_eq!(bounds.values(), [0.0, 0.0, 37.0, 59.0]);
    let space = value.replace('\t', " ");
    let twin = ui
        .event(&id, |cx| cx.text(&space, Some(38.0), TextStyle::Monospace))
        .unwrap()
        .unwrap();
    assert_eq!(twin.size(), Size::new(37.0, 59.0).unwrap());
    let layout = ui
        .event(&id, |cx| cx.text(value, Some(38.0), TextStyle::Monospace))
        .unwrap()
        .unwrap();
    let selection = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "text")
            .unwrap()
            .details
            .properties["selection"]
            .clone()
    };
    // Drags on the first line from near its right edge to fixed points:
    // (end x, selection). The second point is over the first visual glyph,
    // the first is where the pre-fix origin placed nothing.
    for (x, expected) in [
        (1.5, "0..8"),
        (10.0, "0..6"),
        (20.0, "0..4"),
        (34.0, "0..0"),
    ] {
        let start = Point::new(35.5, 10.0).unwrap();
        let end = Point::new(x, 10.0).unwrap();
        for (point, kind) in [
            (start, PointerKind::Move),
            (start, PointerKind::Press(PointerButton::Primary)),
            (end, PointerKind::Move),
            (end, PointerKind::Release(PointerButton::Primary)),
        ] {
            ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
        }
        frame(&mut ui);
        assert_eq!(selection(&ui), text(expected), "drag to {x}");
        // The same points through the public layout give the same range.
        let placed = TextSelection::default()
            .at_point(&layout, start, SelectionExtension::Collapse)
            .at_point(&layout, end, SelectionExtension::Extend);
        assert_eq!(
            text(&format!(
                "{}..{}",
                placed.anchor_byte(),
                placed.focus_byte()
            )),
            text(expected)
        );
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(start, kind)).unwrap();
        }
        ui.flush(ui.now() + std::time::Duration::from_secs(2))
            .unwrap();
    }
}
