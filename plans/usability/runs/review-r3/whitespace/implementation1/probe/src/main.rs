//! Evidence probe through the public facade only, built unchanged against the
//! approved-tab baseline tree and the whitespace implementation worktree. It
//! prints observations and asserts nothing.
//!
//! Every observation records the inspected selection and scroll offsets, the
//! display list's caret and selection fills and the text clip, the IME caret
//! anchor and the CPU pixels of the caret: the bounding box of pixels that
//! differ between the frame and the same frame after one caret blink (empty
//! when no caret pixel is visible).
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;

const SCALES: [f64; 2] = [1.0, 1.25];

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
struct Scene {
    carets: Vec<Rect>,
    selections: Vec<Rect>,
    underlines: Vec<Rect>,
    clips: Vec<Rect>,
    frame: FrameImage,
}
fn scene(ui: &mut Runtime) -> Scene {
    let palette = Palette::dark();
    let caret = palette.color(ColorRole::OnSurface);
    let selection = palette.color(ColorRole::TextSelection);
    let underline = palette.color(ColorRole::Primary);
    let list = ui.paint().unwrap();
    let (mut carets, mut selections, mut underlines, mut clips) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for command in list.commands() {
        match command {
            DrawCommand::FillRect { rect, color } if *color == caret => carets.push(*rect),
            DrawCommand::FillRect { rect, color } if *color == selection => selections.push(*rect),
            DrawCommand::FillRect { rect, color } if *color == underline && rect.height() == 1.0 => {
                underlines.push(*rect)
            }
            DrawCommand::DrawMask { clip: Some(clip), .. } => {
                if !clips.contains(clip) {
                    clips.push(*clip)
                }
            }
            _ => {}
        }
    }
    let frame = pert_ui::cpu::CpuRenderer::new().render(&list).unwrap();
    Scene { carets, selections, underlines, clips, frame }
}
/// Physical bounding box `[x0, y0, x1, y1)` of pixels differing between frames.
fn diff_box(a: &FrameImage, b: &FrameImage) -> Option<[u32; 4]> {
    if (a.width(), a.height()) != (b.width(), b.height()) {
        return None;
    }
    let mut bounds: Option<[u32; 4]> = None;
    for (index, (p, q)) in a.pixels().chunks(4).zip(b.pixels().chunks(4)).enumerate() {
        if p != q {
            let (x, y) = (index as u32 % a.width(), index as u32 / a.width());
            let b = bounds.get_or_insert([x, y, x + 1, y + 1]);
            b[0] = b[0].min(x);
            b[1] = b[1].min(y);
            b[2] = b[2].max(x + 1);
            b[3] = b[3].max(y + 1);
        }
    }
    bounds
}
fn observe(ui: &mut Runtime, case: &str, step: &str) {
    observe_field(ui, "field", case, step)
}
fn observe_field(ui: &mut Runtime, field: &str, case: &str, step: &str) {
    let shown = scene(ui);
    // One blink later the caret toggles; the next blink restores it.
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    let blinked = scene(ui);
    ui.flush(ui.now() + Duration::from_millis(500)).unwrap();
    settle(ui);
    let ime = ui.text_input_state().map(|(_, s)| r(s.caret));
    println!(
        "{case} | {step} | selection {} | scroll_x {} | scroll_y {} | caret fills {:?} | selection fills {:?} | underlines {:?} | mask clips {:?} | ime caret {:?} | caret pixels {:?}",
        property(ui, field, "selection"),
        property(ui, field, "scroll_x"),
        property(ui, field, "scroll_y"),
        shown.carets.into_iter().map(r).collect::<Vec<_>>(),
        shown.selections.into_iter().map(r).collect::<Vec<_>>(),
        shown.underlines.into_iter().map(r).collect::<Vec<_>>(),
        shown.clips.into_iter().map(r).collect::<Vec<_>>(),
        ime,
        diff_box(&shown.frame, &blinked.frame),
    );
}
fn runtime(width: f32, scale: f64) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, scale).unwrap())
        .unwrap();
    ui
}
fn area_with(
    text: &str,
    width: f32,
    scale: f64,
    style: TextStyle,
    edit: impl FnOnce(TextArea) -> TextArea,
) -> (Runtime, WidgetId, Signal<String>) {
    let mut ui = runtime(width, scale);
    let value = ui.signal(text.to_owned());
    let id = ui
        .mount_tree(Element::new(
            "field",
            edit(TextArea::new(value.clone()).style(style)),
        ))
        .unwrap();
    ui.focus(&id).unwrap();
    settle(&mut ui);
    (ui, id, value)
}
fn area(text: &str, width: f32, scale: f64, style: TextStyle) -> (Runtime, WidgetId) {
    let (ui, id, _) = area_with(text, width, scale, style, |a| a);
    (ui, id)
}
fn wheel(ui: &mut Runtime, id: &WidgetId, dx: f32) -> InputOutcome {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    ui.pointer_input(PointerEvent::new(center, PointerKind::Move))
        .unwrap();
    let outcome = ui
        .pointer_input(PointerEvent::new(
            center,
            PointerKind::Scroll(Point::new(dx, 0.0).unwrap()),
        ))
        .unwrap();
    settle(ui);
    outcome
}
/// `forward` is the arrow key that moves logically forward on a line.
fn wrapped(case: &str, text: &str, width: f32, scale: f64, forward: KeyCode) {
    let (mut ui, id) = area(text, width, scale, TextStyle::Monospace);
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
    let outcome = wheel(&mut ui, &id, 40.0);
    observe(&mut ui, case, &format!("wheel +40 after end: {outcome:?}"));
    let outcome = wheel(&mut ui, &id, -40.0);
    observe(&mut ui, case, &format!("wheel -40: {outcome:?}"));
    key(&mut ui, KeyCode::Home);
    with(&mut ui, KeyCode::ShiftLeft, KeyCode::End);
    observe(&mut ui, case, "home, shift+end (selection with the hanging whitespace)");
    with(&mut ui, KeyCode::primary(), KeyCode::End);
    observe(&mut ui, case, "ctrl+end (text end)");
    let outcome = wheel(&mut ui, &id, 40.0);
    observe(&mut ui, case, &format!("wheel +40: {outcome:?}"));
}
fn typed(case: &str, pieces: &[&str], width: f32, scale: f64, style: TextStyle) {
    let (mut ui, id) = area("", width, scale, style);
    println!("{case} | typed {pieces:?} | bounds {}", r(ui.bounds(&id).unwrap().unwrap()));
    for piece in pieces {
        ui.text_input(piece).unwrap();
        settle(&mut ui);
        observe(&mut ui, case, &format!("after typing {piece:?}"));
    }
}
fn selectable(case: &str, text: &str, width: f32, scale: f64) {
    let mut ui = runtime(width, scale);
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
    let id = pert_ui::testing::Driver::new(&mut ui)
        .find(&root, &["text"])
        .unwrap();
    println!("{case} | text {text:?} | bounds {}", r(ui.bounds(&id).unwrap().unwrap()));
    let layout = ui
        .event(&id, |cx| cx.text(text, Some(width), TextStyle::Monospace))
        .unwrap()
        .unwrap();
    let carets = (0..=text.len())
        .filter_map(|i| TextSelection::new(&layout, i, i).ok())
        .map(|s| format!("{}:{}", s.focus_byte(), r(s.caret(&layout).unwrap())))
        .collect::<Vec<_>>();
    println!(
        "{case} | public layout size {:?} | carets {carets:?}",
        [layout.size().width(), layout.size().height()]
    );
}
fn single_line(case: &str, scale: f64) {
    let mut ui = runtime(120.0, scale);
    let value = ui.signal("single line text that overflows".to_owned());
    let id = ui
        .mount_tree(Element::new("field", TextField::new(value)))
        .unwrap();
    ui.focus(&id).unwrap();
    settle(&mut ui);
    with(&mut ui, KeyCode::primary(), KeyCode::Home);
    observe(&mut ui, case, "home");
    key(&mut ui, KeyCode::End);
    observe(&mut ui, case, "end");
    let outcome = wheel(&mut ui, &id, 40.0);
    observe(&mut ui, case, &format!("wheel +40 after end: {outcome:?}"));
    let outcome = wheel(&mut ui, &id, -1000.0);
    observe(&mut ui, case, &format!("wheel -1000: {outcome:?}"));
    let outcome = wheel(&mut ui, &id, 1000.0);
    observe(&mut ui, case, &format!("wheel +1000: {outcome:?}"));
    key(&mut ui, KeyCode::ArrowLeft);
    observe(&mut ui, case, "left after wheel");
}
/// Raster mode: each case's public bounds, mask commands and frame bytes,
/// saved under `dir` for a pixel comparison between builds.
fn raster(dir: &str) {
    let hebrew = "\u{5d0}\u{5d1}\u{5d2}";
    let arabic = "\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}";
    let red = Color::from_srgb8(220, 20, 20, 255);
    let blue = Color::from_srgb8(20, 20, 220, 255);
    std::fs::create_dir_all(dir).unwrap();
    // (name, element, viewport width)
    type Build = Box<dyn Fn(&mut Runtime) -> Element>;
    let text = |t: &'static str| -> Build {
        Box::new(move |ui: &mut Runtime| {
            let v = ui.signal(t.to_owned());
            Element::new("text", SelectableText::new(v.read_only()).style(TextStyle::Monospace))
        })
    };
    let owned = |t: String| -> Build {
        Box::new(move |ui: &mut Runtime| {
            let v = ui.signal(t.clone());
            Element::new("text", SelectableText::new(v.read_only()).style(TextStyle::Monospace))
        })
    };
    let label = |r: RichText| -> Build {
        Box::new(move |_: &mut Runtime| Element::new("text", Label::new(r.clone())))
    };
    let cases: Vec<(&str, Build, f32)> = vec![
        ("ltr-plain", text("aaaa"), 200.0),
        ("ltr-trailing-spaces", text("aaaa   "), 200.0),
        ("ltr-leading-spaces", text("   aaaa"), 200.0),
        ("ltr-inner-spaces", text("aa  bb"), 200.0),
        ("rtl-trailing-space", owned(format!("{hebrew} ")), 200.0),
        ("rtl-leading-space", owned(format!(" {hebrew}")), 200.0),
        ("ltr-wrapped-hanging-space", text("aaaa bbbb cccc"), 38.0),
        ("ltr-wrapped-hanging-spaces", text("aaaa   bbbb cccc"), 38.0),
        ("rtl-wrapped-hanging-space", text(arabic), 38.0),
        ("rtl-wrapped-hanging-tab", owned(arabic.replace(' ', "\t")), 38.0),
        ("whitespace-only", text("   "), 200.0),
        ("tab-only", text("\t\t"), 200.0),
        ("tab-and-space", text("a\tb c"), 200.0),
        ("unsupported", text("a\u{E000} b"), 200.0),
        ("colored-space-split", label(RichText::new()
            .span(TextSpan::new("a").color(red))
            .span(TextSpan::new(" ").color(blue))
            .span(TextSpan::new("b").color(red))), 200.0),
        ("inline-icons", label(RichText::new().text("a ").icon(Icon::Save).text(" b ").icon(Icon::Close)), 200.0),
        ("icon-only", label(RichText::new().icon(Icon::Add)), 200.0),
        ("leading-icon-label", Box::new(|_: &mut Runtime| Element::new("text", Label::new("Save").leading_icon(Icon::Save))), 200.0),
    ];
    for scale in SCALES {
        for (name, build, width) in &cases {
            // Two paint origins: a whole-pixel one and one a quarter pixel off,
            // which takes the phase path.
            for (offset, origin) in [("whole", 0.0f32), ("quarter", 0.25)] {
                let mut ui = runtime(*width + 8.0, scale);
                let element = build(&mut ui);
                let root = ui
                    .mount_tree(Element::new(
                        "page",
                        Container::new(element).padding(Length::new(4.0 + origin).unwrap()),
                    ))
                    .unwrap();
                settle(&mut ui);
                let id = pert_ui::testing::Driver::new(&mut ui).find(&root, &["text"]).unwrap();
                let bounds = ui.bounds(&id).unwrap().unwrap();
                let list = ui.paint().unwrap();
                let masks: Vec<String> = list
                    .commands()
                    .iter()
                    .filter_map(|c| match c {
                        DrawCommand::DrawMask { rect, mask, clip, .. } => {
                            let image = list.mask(*mask).unwrap();
                            Some(format!(
                                "{} image {}x{} clip {}",
                                r(*rect),
                                image.width(),
                                image.height(),
                                clip.map_or("none".into(), r)
                            ))
                        }
                        _ => None,
                    })
                    .collect();
                let frame = pert_ui::cpu::CpuRenderer::new().render(&list).unwrap();
                let file = format!("{dir}/s{scale}-{name}-{offset}.rgba");
                std::fs::write(&file, frame.pixels()).unwrap();
                println!(
                    "raster s{scale} {name} {offset} | bounds {} | masks {} {masks:?} | frame {}x{}",
                    r(bounds),
                    masks.len(),
                    frame.width(),
                    frame.height()
                );
            }
        }
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "raster" {
        return raster(&args[2]);
    }
    let arabic_tab = "\u{627}\u{628}\u{62c}\u{62f}\t\u{647}\u{648}\u{632}\u{62d}\t\u{637}\u{64a}";
    let arabic_space = "\u{627}\u{628}\u{62c}\u{62f} \u{647}\u{648}\u{632}\u{62d} \u{637}\u{64a}";
    for scale in SCALES {
        let s = format!("s{scale}");
        wrapped(&format!("ltr-tab w62 {s}"), "aaaa\tbbbb\tcccc", 62.0, scale, KeyCode::ArrowRight);
        wrapped(&format!("ltr-space w62 {s}"), "aaaa bbbb cccc", 62.0, scale, KeyCode::ArrowRight);
        wrapped(&format!("rtl-tab w62 {s}"), arabic_tab, 62.0, scale, KeyCode::ArrowLeft);
        wrapped(&format!("rtl-space w62 {s}"), arabic_space, 62.0, scale, KeyCode::ArrowLeft);
        typed(&format!("typed-space w62 {s}"), &["aaaa", " ", "b"], 62.0, scale, TextStyle::Monospace);
        typed(&format!("typed-tab w62 {s}"), &["aaaa", "\t", "b"], 62.0, scale, TextStyle::Monospace);
        let prose: Vec<String> = "one two three four five".chars().map(String::from).collect();
        let prose: Vec<&str> = prose.iter().map(String::as_str).collect();
        typed(&format!("prose-body w90 {s}"), &prose, 90.0, scale, TextStyle::Body);
        typed(&format!("prose-body w79 {s}"), &prose, 79.0, scale, TextStyle::Body);
        typed(&format!("held-space w62 {s}"), &["aaaa", " ", " ", " ", " ", " ", " "], 62.0, scale, TextStyle::Monospace);
        // Inner width 34: four 8.4-pixel cells (33.6) nearly fill it.
        let (mut ui, id) = area("aaaa", 58.0, scale, TextStyle::Monospace);
        let case = format!("nearly-full w58 {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::End);
        observe(&mut ui, &case, "ctrl+end");
        let outcome = wheel(&mut ui, &id, 40.0);
        observe(&mut ui, &case, &format!("wheel +40: {outcome:?}"));
        let outcome = wheel(&mut ui, &id, -40.0);
        observe(&mut ui, &case, &format!("wheel -40: {outcome:?}"));
        for width in [20.0, 24.0, 24.5, 25.0, 26.0] {
            let (mut ui, _) = area("aa bb", width, scale, TextStyle::Monospace);
            let case = format!("tiny w{width} {s}");
            with(&mut ui, KeyCode::primary(), KeyCode::Home);
            observe(&mut ui, &case, "ctrl+home");
            key(&mut ui, KeyCode::End);
            observe(&mut ui, &case, "end");
            with(&mut ui, KeyCode::primary(), KeyCode::End);
            observe(&mut ui, &case, "ctrl+end");
        }
        // Read-only: no caret, selection fill only.
        let (mut ui, _, _) =
            area_with("aaaa bbbb cccc", 62.0, scale, TextStyle::Monospace, |a| a.read_only());
        let case = format!("read-only w62 {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::KeyA);
        observe(&mut ui, &case, "select all");
        with(&mut ui, KeyCode::primary(), KeyCode::Home);
        with(&mut ui, KeyCode::ShiftLeft, KeyCode::End);
        observe(&mut ui, &case, "shift+end on line 0");
        // Reflow: widen, then narrow again at the hanging line end.
        let (mut ui, _) = area("aaaa bbbb cccc", 62.0, scale, TextStyle::Monospace);
        let case = format!("resize {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::Home);
        key(&mut ui, KeyCode::End);
        observe(&mut ui, &case, "end at w62");
        ui.set_viewport(Viewport::new(200.0, 200.0, scale).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "resized to w200");
        ui.set_viewport(Viewport::new(62.0, 200.0, scale).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "resized back to w62");
        let other = if scale == 1.0 { 1.25 } else { 1.0 };
        ui.set_viewport(Viewport::new(62.0, 200.0, other).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, &format!("scale changed to {other}"));
        // Focus loss and an unfocused relayout keep the forced reveal.
        let (mut ui, id) = area("aaaa bbbb cccc", 62.0, scale, TextStyle::Monospace);
        let case = format!("focus {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::Home);
        key(&mut ui, KeyCode::End);
        pert_ui::testing::Driver::new(&mut ui).clear_focus().unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "after focus loss");
        ui.set_viewport(Viewport::new(63.0, 200.0, scale).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "unfocused resize to w63");
        ui.focus(&id).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "focus regained");
        // Composition at a hanging line end.
        let (mut ui, _) = area("aaaa", 62.0, scale, TextStyle::Monospace);
        let case = format!("preedit {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::End);
        ui.text_preedit(&TextPreedit::new(" ", Some(1..1)).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "preedit space, cursor after it");
        ui.text_preedit(&TextPreedit::new("  x", Some(2..2)).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "preedit two spaces and x, cursor before x");
        ui.text_preedit(&TextPreedit::new("", None).unwrap()).unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, "preedit cancelled");
        // Pending edit reveal cancelled by a wheel before the next paint.
        let (mut ui, id) = area("aaaa", 62.0, scale, TextStyle::Monospace);
        let case = format!("pending-wheel {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::End);
        observe(&mut ui, &case, "ctrl+end");
        // The edit's reveal is pending until the next paint; a wheel first.
        ui.text_input(" ").unwrap();
        let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
        let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        ui.pointer_input(PointerEvent::new(center, PointerKind::Move)).unwrap();
        let outcome = ui
            .pointer_input(PointerEvent::new(
                center,
                PointerKind::Scroll(Point::new(-40.0, 0.0).unwrap()),
            ))
            .unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, &format!("typed space, wheel -40 before paint: {outcome:?}"));
        key(&mut ui, KeyCode::ArrowLeft);
        key(&mut ui, KeyCode::ArrowRight);
        observe(&mut ui, &case, "left, right (reveal requested again)");
        // The same with an unbreakable word that overflows, so the wheel moves.
        let (mut ui, id) = area("aaaaaaaaaaaaaaaaaaaa", 62.0, scale, TextStyle::Monospace);
        let case = format!("pending-wheel-overflow {s}");
        with(&mut ui, KeyCode::primary(), KeyCode::End);
        observe(&mut ui, &case, "ctrl+end");
        ui.text_input("b").unwrap();
        let [x, y, w, h] = ui.bounds(&id).unwrap().unwrap().values();
        let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        ui.pointer_input(PointerEvent::new(center, PointerKind::Move)).unwrap();
        let outcome = ui
            .pointer_input(PointerEvent::new(
                center,
                PointerKind::Scroll(Point::new(-40.0, 0.0).unwrap()),
            ))
            .unwrap();
        settle(&mut ui);
        observe(&mut ui, &case, &format!("typed b, wheel -40 before paint: {outcome:?}"));
        key(&mut ui, KeyCode::ArrowLeft);
        key(&mut ui, KeyCode::ArrowRight);
        observe(&mut ui, &case, "left, right (reveal requested again)");
        let outcome = wheel(&mut ui, &id, 1000.0);
        observe(&mut ui, &case, &format!("wheel +1000: {outcome:?}"));
        single_line(&format!("single-line {s}"), scale);
        selectable(&format!("selectable-rtl-tab w38 {s}"), arabic_tab, 38.0, scale);
        selectable(&format!("selectable-rtl-space w38 {s}"), arabic_space, 38.0, scale);
        selectable(&format!("selectable-ltr-space w38 {s}"), "aaaa bbbb cccc", 38.0, scale);
    }
}
