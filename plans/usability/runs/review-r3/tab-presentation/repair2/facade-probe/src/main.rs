//! Evidence probe through the public facade only: narrow TextArea and
//! SelectableText geometry with hanging tabs and their hanging-space twins.
//! Built unchanged against the base commit and the repaired worktree; see
//! ../README.md. It prints observations and asserts nothing.
use pert_ui::testing::HostIntegration;
use pert_ui::*;

fn settle(ui: &mut Runtime) {
    ui.flush(ui.now()).unwrap();
    ui.paint().unwrap();
}
fn key(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    settle(ui);
}
fn with(ui: &mut Runtime, modifier: KeyCode, code: KeyCode) {
    ui.key_input(modifier, KeyAction::Press).unwrap();
    key(ui, code);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    settle(ui);
}
fn property(ui: &Runtime, key: &str, name: &str) -> String {
    match ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
        .properties
        .get(name)
    {
        Some(InspectValue::Text(text)) => text.clone(),
        Some(InspectValue::Number(n)) => format!("{n}"),
        other => format!("{other:?}"),
    }
}
fn r(rect: Rect) -> String {
    let [x, y, w, h] = rect.values();
    format!("[{x:.3}, {y:.3}, {w:.3}, {h:.3}]")
}
/// Caret fills (OnSurface FillRect) and selection fills, the text mask
/// clip (the editor's inner rectangle), in window coordinates.
fn scene(ui: &mut Runtime) -> (Vec<Rect>, Vec<Rect>, Vec<Rect>) {
    let palette = Palette::dark();
    let caret = palette.color(ColorRole::OnSurface);
    let selection = palette.color(ColorRole::TextSelection);
    let scene = ui.paint().unwrap();
    let (mut carets, mut selections, mut clips) = (Vec::new(), Vec::new(), Vec::new());
    for command in scene.commands() {
        match command {
            DrawCommand::FillRect { rect, color } if *color == caret => carets.push(*rect),
            DrawCommand::FillRect { rect, color } if *color == selection => selections.push(*rect),
            DrawCommand::DrawMask { clip: Some(clip), .. } => {
                if !clips.contains(clip) {
                    clips.push(*clip)
                }
            }
            _ => {}
        }
    }
    (carets, selections, clips)
}
fn observe(ui: &mut Runtime, case: &str, step: &str) {
    let (carets, selections, clips) = scene(ui);
    let ime = ui.text_input_state().map(|(_, s)| r(s.caret));
    println!(
        "{case} | {step} | selection {} | scroll_x {} | scroll_y {} | caret fills {:?} | selection fills {:?} | mask clips {:?} | ime caret {:?}",
        property(ui, "field", "selection"),
        property(ui, "field", "scroll_x"),
        property(ui, "field", "scroll_y"),
        carets.into_iter().map(r).collect::<Vec<_>>(),
        selections.into_iter().map(r).collect::<Vec<_>>(),
        clips.into_iter().map(r).collect::<Vec<_>>(),
        ime,
    );
}
fn area(text: &str, width: f32, style: TextStyle) -> (Runtime, WidgetId) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let id = ui
        .mount_tree(Element::new("field", TextArea::new(value).style(style)))
        .unwrap();
    ui.focus(&id).unwrap();
    settle(&mut ui);
    (ui, id)
}
/// `forward` is the arrow key that moves logically forward on a line: Right
/// in left-to-right text, Left in right-to-left text.
fn wrapped(case: &str, text: &str, width: f32, forward: KeyCode) {
    let (mut ui, id) = area(text, width, TextStyle::Monospace);
    println!("{case} | text {text:?} | bounds {}", r(ui.bounds(&id).unwrap().unwrap()));
    with(&mut ui, KeyCode::primary(), KeyCode::Home);
    observe(&mut ui, case, "ctrl+home");
    key(&mut ui, forward);
    key(&mut ui, forward);
    observe(&mut ui, case, "forward x2 (interior)");
    key(&mut ui, forward);
    key(&mut ui, forward);
    observe(&mut ui, case, "forward x4 (before the hanging whitespace)");
    key(&mut ui, KeyCode::End);
    observe(&mut ui, case, "end (line end after the hanging whitespace)");
    key(&mut ui, KeyCode::Home);
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::End);
    observe(&mut ui, case, "home, shift+end (selection with the hanging whitespace)");
    with(&mut ui, KeyCode::primary(), KeyCode::End);
    observe(&mut ui, case, "ctrl+end (text end)");
    let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    ui.pointer_input(PointerEvent::new(center, PointerKind::Move)).unwrap();
    ui.pointer_input(PointerEvent::new(
        center,
        PointerKind::Scroll(Point::new(40.0, 0.0).unwrap()),
    ))
    .unwrap();
    settle(&mut ui);
    observe(&mut ui, case, "horizontal scroll by 40");
}
fn forced(case: &str, text: &str) {
    let (mut ui, id) = area(text, 220.0, TextStyle::Monospace);
    println!("{case} | text {text:?} | bounds {}", r(ui.bounds(&id).unwrap().unwrap()));
    with(&mut ui, KeyCode::primary(), KeyCode::Home);
    key(&mut ui, KeyCode::End);
    observe(&mut ui, case, "end (after the tab at the forced break)");
    with(&mut ui, KeyCode::primary(), KeyCode::End);
    observe(&mut ui, case, "ctrl+end (after the tab at the text end)");
}
/// Types into an empty field with committed text input, as a keyboard or IME
/// does, observing after each piece.
fn typed(case: &str, pieces: &[&str], width: f32) {
    let (mut ui, id) = area("", width, TextStyle::Monospace);
    println!("{case} | typed {pieces:?} | bounds {}", r(ui.bounds(&id).unwrap().unwrap()));
    for piece in pieces {
        ui.text_input(piece).unwrap();
        settle(&mut ui);
        observe(&mut ui, case, &format!("after typing {piece:?}"));
    }
}
fn selectable(case: &str, text: &str, width: f32) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(text.to_owned());
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([Element::new(
                "text",
                SelectableText::new(value.read_only()).style(TextStyle::Monospace),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let id = pert_ui::testing::Driver::new(&mut ui).find(&root, &["text"]).unwrap();
    let bounds = ui.bounds(&id).unwrap().unwrap();
    println!("{case} | text {text:?} | bounds {}", r(bounds));
    let [x, y, _, _] = bounds.values();
    // Fixed window points on the first line.
    let line = y + 10.0;
    let points = [
        ("x+1.5 (pre-fix origin band)", x + 1.5),
        ("x+10 (over the first visual glyph)", x + 10.0),
        ("x+20", x + 20.0),
        ("x+34 (last visual glyph)", x + 34.0),
        ("x+37.5 (right edge)", x + 37.5),
    ];
    for (name, px) in points {
        let start = Point::new(x + 35.5, line).unwrap();
        let end = Point::new(px, line).unwrap();
        ui.pointer_input(PointerEvent::new(start, PointerKind::Move)).unwrap();
        ui.pointer_input(PointerEvent::new(start, PointerKind::Press(PointerButton::Primary)))
            .unwrap();
        ui.pointer_input(PointerEvent::new(end, PointerKind::Move)).unwrap();
        ui.pointer_input(PointerEvent::new(end, PointerKind::Release(PointerButton::Primary)))
            .unwrap();
        settle(&mut ui);
        let (_, fills, _) = scene(&mut ui);
        println!(
            "{case} | drag from x+35.5 to {name} at y+10 | selection {} | selection fills {:?}",
            property(&ui, "text", "selection"),
            fills.into_iter().map(r).collect::<Vec<_>>()
        );
        // A plain click elsewhere collapses before the next drag.
        ui.pointer_input(PointerEvent::new(start, PointerKind::Press(PointerButton::Primary)))
            .unwrap();
        ui.pointer_input(PointerEvent::new(start, PointerKind::Release(PointerButton::Primary)))
            .unwrap();
        ui.flush(ui.now() + std::time::Duration::from_secs(2)).unwrap();
    }
}

fn main() {
    let arabic_tab = "\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}";
    let arabic_space = "\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}";
    for width in [62.0, 64.0] {
        wrapped(&format!("ltr-tab w{width}"), "aaaa\tbbbb\tcccc", width, KeyCode::ArrowRight);
        wrapped(&format!("ltr-space w{width}"), "aaaa bbbb cccc", width, KeyCode::ArrowRight);
        wrapped(&format!("rtl-tab w{width}"), arabic_tab, width, KeyCode::ArrowLeft);
        wrapped(&format!("rtl-space w{width}"), arabic_space, width, KeyCode::ArrowLeft);
    }
    forced("forced-tab", "ab\t\ncd\t");
    typed("typed-space w62", &["aaaa", " ", "b"], 62.0);
    typed("typed-tab w62", &["aaaa", "\t", "b"], 62.0);
    typed("typed-prose-space w62", &["one", " ", "two", " "], 62.0);
    selectable("selectable-rtl-tab w38", arabic_tab, 38.0);
    selectable("selectable-rtl-space w38", arabic_space, 38.0);
}
