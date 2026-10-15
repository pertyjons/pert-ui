//! Keyboard input repaints only what it changes: the runtime no longer
//! repaints the focus chain for every key, so each built-in control that
//! presents a key's effect invalidates itself. Every check compares the scene
//! after the key with the scene before it (the key must show) and with a scene
//! repainted from scratch (nothing may stay stale).
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PRESS: KeyAction = KeyAction::Press;
const RELEASE: KeyAction = KeyAction::Release;

fn runtime(width: f32, height: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui
}
fn scene(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
/// Repaints every palette reader by switching the theme mode away and back.
fn repainted(ui: &mut Runtime) -> DisplayList {
    let theme = ui.theme();
    let mode = theme.mode().unwrap();
    let other = match mode {
        ThemeMode::Dark => ThemeMode::Light,
        ThemeMode::Light => ThemeMode::Dark,
    };
    theme.set_mode(other).unwrap();
    scene(ui);
    theme.set_mode(mode).unwrap();
    scene(ui)
}
/// Sends `keys` to the focus and asserts that the scene matches one repainted
/// from scratch; returns whether the keys changed the scene.
fn keys_repaint(ui: &mut Runtime, keys: &[(KeyCode, KeyAction)], what: &str) -> bool {
    let before = scene(ui);
    for (code, action) in keys {
        ui.key_input(*code, *action).unwrap();
    }
    let after = scene(ui);
    assert!(
        after == repainted(ui),
        "{what} {keys:?}: the scene is stale"
    );
    after != before
}
/// Focuses `target`, then as [`keys_repaint`], asserting that the keys
/// changed the scene.
fn assert_repaints(ui: &mut Runtime, target: &WidgetId, keys: &[(KeyCode, KeyAction)], what: &str) {
    ui.focus(target).unwrap();
    assert!(
        keys_repaint(ui, keys, what),
        "{what} {keys:?}: the scene did not change"
    );
}
/// Mounts `element` in a column and returns its ID: a root is allocated the
/// whole window, which leaves a popup no room beside it.
fn mount(ui: &mut Runtime, element: Element) -> WidgetId {
    let root = ui
        .mount_tree(Element::new("root", Column::new([element])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.children(&root).unwrap().remove(0)
}
/// Mounts a viewport as the root, which the window's height bounds.
fn mount_root(ui: &mut Runtime, element: Element) -> WidgetId {
    let id = ui.mount_tree(element).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    id
}

#[test]
fn activation_controls_show_the_pressed_key() {
    let mut ui = runtime(300.0, 100.0);
    let id = mount(
        &mut ui,
        Element::new("button", Button::new("Apply", |_| Ok(()))),
    );
    assert_repaints(&mut ui, &id, &[(KeyCode::Space, PRESS)], "Button Space");
    assert_repaints(&mut ui, &id, &[(KeyCode::Space, RELEASE)], "Button release");
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(false);
    let id = mount(
        &mut ui,
        Element::new("check", Checkbox::new("On", value.clone())),
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::Space, PRESS), (KeyCode::Space, RELEASE)],
        "Checkbox",
    );
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(false);
    let id = mount(
        &mut ui,
        Element::new("toggle", Toggle::new("On", value.clone())),
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::Space, PRESS), (KeyCode::Space, RELEASE)],
        "Toggle",
    );
}

#[test]
fn choice_groups_show_arrow_selection() -> TestResult {
    let choices = || {
        [
            Choice::new(1, "One"),
            Choice::new(2, "Two"),
            Choice::new(3, "Three"),
        ]
    };
    let mut ui = runtime(300.0, 200.0);
    let value = ui.signal(1);
    let id = mount(
        &mut ui,
        Element::new("radio", RadioGroup::new(value, choices())?),
    );
    assert_repaints(&mut ui, &id, &[(KeyCode::ArrowDown, PRESS)], "RadioGroup");
    let mut ui = runtime(300.0, 200.0);
    let value = ui.signal(1);
    let id = mount(
        &mut ui,
        Element::new("segments", SegmentedControl::new(value, choices())?),
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::ArrowRight, PRESS)],
        "SegmentedControl",
    );
    let mut ui = runtime(300.0, 200.0);
    let value = ui.signal(1);
    let id = mount(
        &mut ui,
        Element::new("inline", InlineChoice::new(value, choices())?),
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::ArrowRight, PRESS)],
        "InlineChoice",
    );
    Ok(())
}

#[test]
fn sliders_show_an_arrow_step() -> TestResult {
    let mut ui = runtime(300.0, 100.0);
    let (parameter, _host) = ui.parameter(
        ParameterId::new(1, 1, "value")?,
        ParameterSpec::new("Value", 0.0, 100.0, 50.0)?,
    )?;
    let id = mount(
        &mut ui,
        Element::new("slider", Slider::new(parameter.clone())),
    );
    assert_repaints(&mut ui, &id, &[(KeyCode::ArrowRight, PRESS)], "Slider");
    keys_repaint(&mut ui, &[(KeyCode::ArrowRight, RELEASE)], "Slider commit");
    let mut ui = runtime(300.0, 100.0);
    let (parameter, _host) = ui.parameter(
        ParameterId::new(1, 1, "value")?,
        ParameterSpec::new("Value", 0.0, 100.0, 50.0)?,
    )?;
    let id = mount(
        &mut ui,
        Element::new("inline", InlineSlider::new(parameter)),
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::ArrowRight, PRESS)],
        "InlineSlider",
    );
    Ok(())
}

#[test]
fn text_editors_show_caret_selection_and_deletion() {
    for multi in [false, true] {
        let mut ui = runtime(300.0, 100.0);
        let value = ui.signal(String::from("Hello world"));
        let field = if multi {
            Element::new("field", TextArea::new(value))
        } else {
            Element::new("field", TextField::new(value))
        };
        let id = mount(&mut ui, field);
        let what = if multi { "TextArea" } else { "TextField" };
        assert_repaints(&mut ui, &id, &[(KeyCode::End, PRESS)], what);
        assert_repaints(&mut ui, &id, &[(KeyCode::Home, PRESS)], what);
        assert_repaints(&mut ui, &id, &[(KeyCode::End, PRESS)], what);
        assert_repaints(
            &mut ui,
            &id,
            &[(KeyCode::ShiftLeft, PRESS), (KeyCode::ArrowLeft, PRESS)],
            what,
        );
        ui.key_input(KeyCode::ShiftLeft, RELEASE).unwrap();
        assert_repaints(&mut ui, &id, &[(KeyCode::Backspace, PRESS)], what);
    }
}

#[test]
fn number_inputs_show_steps_and_edits() -> TestResult {
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(5_i64);
    let id = mount(&mut ui, Element::new("integer", IntegerInput::new(value)));
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::ArrowUp, PRESS)],
        "IntegerInput step",
    );
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::Backspace, PRESS)],
        "IntegerInput edit",
    );
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(0.5_f64);
    let id = mount(&mut ui, Element::new("decimal", DecimalInput::new(value)));
    assert_repaints(
        &mut ui,
        &id,
        &[(KeyCode::ArrowUp, PRESS)],
        "DecimalInput step",
    );
    Ok(())
}

/// Presses one key on the focus and asserts that the scene shows it.
fn press(ui: &mut Runtime, code: KeyCode, what: &str) {
    assert!(
        keys_repaint(ui, &[(code, PRESS)], what),
        "{what} {code:?}: the scene did not change"
    );
    ui.key_input(code, RELEASE).unwrap();
}
/// Mounts `element` and focuses its first Tab stop.
fn mount_focused(ui: &mut Runtime, element: Element) -> WidgetId {
    let id = mount(ui, element);
    ui.focus_next(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    id
}

#[test]
fn popups_show_opening_and_the_highlighted_row() -> TestResult {
    let choices = || {
        [
            Choice::new(1, "One"),
            Choice::new(2, "Two"),
            Choice::new(3, "Three"),
        ]
    };
    let mut ui = runtime(300.0, 300.0);
    let value = ui.signal(1);
    let select = Select::new(&ui, value, choices())?;
    mount_focused(&mut ui, Element::new("select", select));
    press(&mut ui, KeyCode::ArrowDown, "Select open");
    press(&mut ui, KeyCode::ArrowDown, "Select highlight");
    press(&mut ui, KeyCode::Escape, "Select close");
    let mut ui = runtime(300.0, 300.0);
    let value = ui.signal(1);
    let select = InlineSelect::new(&ui, value, choices())?.label("Snap");
    mount_focused(&mut ui, Element::new("inline", select));
    press(&mut ui, KeyCode::ArrowDown, "InlineSelect open");
    press(&mut ui, KeyCode::ArrowDown, "InlineSelect highlight");
    let mut ui = runtime(300.0, 300.0);
    let items = || {
        vec![
            MenuItem::new("a", "Alpha", |_| Ok(())),
            MenuItem::new("b", "Beta", |_| Ok(())),
        ]
    };
    let menu = Menu::new(&ui, "Actions", items())?;
    mount_focused(&mut ui, Element::new("menu", menu));
    press(&mut ui, KeyCode::Enter, "Menu open");
    press(&mut ui, KeyCode::ArrowDown, "Menu highlight");
    let mut ui = runtime(300.0, 300.0);
    let content = Element::new("owner", Button::new("Owner", |_| Ok(())));
    let menu = ContextMenu::new(&ui, content, items())?;
    mount_focused(&mut ui, Element::new("context", menu));
    ui.key_input(KeyCode::ShiftLeft, PRESS)?;
    press(&mut ui, KeyCode::F10, "ContextMenu open");
    ui.key_input(KeyCode::ShiftLeft, RELEASE)?;
    press(&mut ui, KeyCode::ArrowDown, "ContextMenu highlight");
    let mut ui = runtime(300.0, 300.0);
    let bar = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu("file", "File", items()),
            MenuItem::submenu("edit", "Edit", items()),
        ],
    )?;
    mount(&mut ui, Element::new("bar", bar));
    press(&mut ui, KeyCode::F10, "MenuBar enter");
    press(&mut ui, KeyCode::ArrowRight, "MenuBar next title");
    press(&mut ui, KeyCode::ArrowDown, "MenuBar open");
    press(&mut ui, KeyCode::ArrowDown, "MenuBar highlight");
    press(&mut ui, KeyCode::ArrowRight, "MenuBar next menu");
    Ok(())
}

struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(
            self.key(index)?,
            Label::new(format!("Row {index}")),
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("row-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}
struct Outline;
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        vec!["a".into(), "b".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key == "a" {
            vec!["a/0".into(), "a/1".into()]
        } else {
            Vec::new()
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        key == "a"
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
struct Numbers;
impl TableItems for Numbers {
    fn len(&self) -> usize {
        20
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 20).then(|| format!("n{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        Some(Element::new(
            "text",
            Label::new(format!("{column} {index}")),
        ))
    }
}

#[test]
fn collections_show_the_active_row_and_scrolling() -> TestResult {
    let mut ui = runtime(300.0, 120.0);
    let items = ui.signal(ListSource::new(Rows(30)));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let list = VirtualList::new(items.read_only(), 24.0)?.selection(selection);
    mount_focused(&mut ui, Element::new("list", list));
    press(&mut ui, KeyCode::ArrowDown, "VirtualList active row");
    press(&mut ui, KeyCode::ArrowDown, "VirtualList next row");
    press(&mut ui, KeyCode::End, "VirtualList end");
    let mut ui = runtime(300.0, 120.0);
    let items = ui.signal(ListSource::new(Rows(30)));
    let list = VirtualList::new(items.read_only(), 24.0)?;
    let id = mount_root(&mut ui, Element::new("list", list));
    ui.focus(&id)?;
    press(&mut ui, KeyCode::PageDown, "VirtualList page");
    let mut ui = runtime(300.0, 200.0);
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(&ui, TreeSource::new(Outline), selection)?;
    mount_focused(&mut ui, Element::new("tree", Tree::new(&state, 24.0)?));
    press(&mut ui, KeyCode::ArrowDown, "Tree active row");
    press(&mut ui, KeyCode::ArrowDown, "Tree next row");
    press(&mut ui, KeyCode::ArrowUp, "Tree previous row");
    press(&mut ui, KeyCode::ArrowRight, "Tree expand");
    press(&mut ui, KeyCode::ArrowRight, "Tree first child");
    press(&mut ui, KeyCode::ArrowLeft, "Tree parent");
    press(&mut ui, KeyCode::ArrowLeft, "Tree collapse");
    let mut ui = runtime(400.0, 200.0);
    let state = TableController::new(
        &ui,
        [
            TableColumn::new("number", "Number").width(Length::new(100.0)?),
            TableColumn::new("square", "Square"),
        ],
        TableSource::new(Numbers),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    mount_focused(&mut ui, Element::new("table", Table::new(&state, 24.0)?));
    press(&mut ui, KeyCode::ArrowDown, "Table active row");
    press(&mut ui, KeyCode::ArrowRight, "Table active cell");
    press(&mut ui, KeyCode::PageDown, "Table page");
    Ok(())
}

#[test]
fn containers_show_tab_selection_resizing_scrolling_and_zoom() {
    let tab = |key: &str| {
        Tab::new(
            key,
            format!("View {key}"),
            Element::new("body", Label::new(key.to_owned())),
        )
    };
    let mut ui = runtime(400.0, 200.0);
    mount_focused(
        &mut ui,
        Element::new("tabs", Tabs::new([tab("a"), tab("b"), tab("c")])),
    );
    press(&mut ui, KeyCode::ArrowRight, "Tabs arrow");
    ui.key_input(KeyCode::ControlLeft, PRESS).unwrap();
    press(&mut ui, KeyCode::Tab, "Tabs Ctrl+Tab");
    ui.key_input(KeyCode::ControlLeft, RELEASE).unwrap();
    let leaf = |key: &str| {
        Element::new(
            key,
            Container::default().preferred_size(Size::from([40, 40])),
        )
    };
    let mut ui = runtime(208.0, 80.0);
    mount_focused(
        &mut ui,
        Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("a"), leaf("b")),
        ),
    );
    press(&mut ui, KeyCode::ArrowRight, "SplitPane resize");
    let mut ui = runtime(200.0, 100.0);
    let rows = (0..20).map(|index| {
        Element::new(
            format!("row-{index}"),
            Button::new(format!("Row {index}"), |_| Ok(())),
        )
    });
    mount_root(
        &mut ui,
        Element::new(
            "scroll",
            ScrollArea::new(Element::new("rows", Column::new(rows))),
        ),
    );
    ui.focus_next(false).unwrap();
    // The focused row leaves pages to its viewport.
    press(&mut ui, KeyCode::PageDown, "ScrollArea page");
    let label = |key: &str| Element::new(key, Label::new(key.to_owned()));
    let mut ui = runtime(300.0, 200.0);
    let id = mount(
        &mut ui,
        Element::new(
            "canvas",
            ZoomView::new([
                (Point::new(20.0, 20.0).unwrap(), label("a")),
                (Point::new(120.0, 80.0).unwrap(), label("b")),
            ]),
        ),
    );
    ui.focus(&id).unwrap();
    press(&mut ui, KeyCode::Equal, "ZoomView zoom");
    press(&mut ui, KeyCode::ArrowLeft, "ZoomView pan");
}

#[test]
fn inline_rows_show_text_mode() {
    let mut ui = runtime(300.0, 100.0);
    let name = ui.signal(String::from("Lead"));
    mount_focused(&mut ui, Element::new("name", InlineText::new(name)));
    press(&mut ui, KeyCode::F2, "InlineText edit");
    press(&mut ui, KeyCode::End, "InlineText caret");
    press(&mut ui, KeyCode::Escape, "InlineText cancel");
    press(&mut ui, KeyCode::Enter, "InlineText edit again");
    press(&mut ui, KeyCode::Enter, "InlineText save");
    let mut ui = runtime(300.0, 100.0);
    let text = ui.signal(String::from("120 bpm"));
    mount_focused(
        &mut ui,
        Element::new("readout", InlineReadout::new(text.read_only())),
    );
    press(&mut ui, KeyCode::Enter, "InlineReadout text mode");
    press(&mut ui, KeyCode::Escape, "InlineReadout leave");
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(5_i64);
    mount_focused(&mut ui, Element::new("octave", InlineInteger::new(value)));
    press(&mut ui, KeyCode::ArrowUp, "InlineInteger step");
    press(&mut ui, KeyCode::F2, "InlineInteger text mode");
    press(&mut ui, KeyCode::Backspace, "InlineInteger edit");
    press(&mut ui, KeyCode::Escape, "InlineInteger cancel");
    let mut ui = runtime(300.0, 100.0);
    let value = ui.signal(false);
    mount_focused(
        &mut ui,
        Element::new("solo", InlineToggle::new(value, "Solo")),
    );
    press(&mut ui, KeyCode::Space, "InlineToggle");
    let mut ui = runtime(300.0, 100.0);
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 50.0).unwrap(),
        )
        .unwrap();
    mount_focused(
        &mut ui,
        Element::new("amount", InlineSlider::new(parameter)),
    );
    press(&mut ui, KeyCode::Enter, "InlineSlider text mode");
    press(&mut ui, KeyCode::ArrowUp, "InlineSlider text step");
    press(&mut ui, KeyCode::Escape, "InlineSlider cancel");
}

#[test]
fn escape_ends_a_scrollbar_drag_visibly() {
    // Pressed on the thumb at the right edge without hovering it first, and
    // dragged away, so only the drag emphasizes the thumb when Escape ends it.
    fn drag_then_escape(ui: &mut Runtime, bar: Point, what: &str) {
        let kind = PointerKind::Press(PointerButton::Primary);
        ui.pointer_input(PointerEvent::new(bar, kind)).unwrap();
        let away = Point::new(20.0, 30.0).unwrap();
        ui.pointer_input(PointerEvent::new(away, PointerKind::Move))
            .unwrap();
        press(ui, KeyCode::Escape, what);
    }
    let mut ui = runtime(200.0, 100.0);
    let content = Container::new(Element::new("button", Button::new("Action", |_| Ok(()))))
        .size_limits(SizeLimits::at_least(Size::from([180, 400])));
    mount_root(
        &mut ui,
        Element::new("scroll", ScrollArea::new(Element::new("content", content))),
    );
    ui.focus_next(false).unwrap();
    drag_then_escape(&mut ui, Point::new(196.0, 5.0).unwrap(), "ScrollArea");
    let mut ui = runtime(200.0, 100.0);
    let items = ui.signal(ListSource::new(Rows(30)));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let list = VirtualList::new(items.read_only(), 24.0)
        .unwrap()
        .selection(selection);
    mount_root(&mut ui, Element::new("list", list));
    ui.focus_next(false).unwrap();
    drag_then_escape(&mut ui, Point::new(196.0, 5.0).unwrap(), "VirtualList");
    let mut ui = runtime(200.0, 100.0);
    let text = (0..30)
        .map(|line| format!("Line {line}\n"))
        .collect::<String>();
    let value = ui.signal(text);
    mount_root(&mut ui, Element::new("area", TextArea::new(value)));
    ui.focus_next(false).unwrap();
    // The editor's bar runs inside its padding.
    let metrics = ThemeMetrics::default();
    let lane = Point::new(
        200.0 - metrics.get(MetricRole::ControlPaddingX) - 2.0,
        metrics.get(MetricRole::ControlPaddingY) + 4.0,
    );
    drag_then_escape(&mut ui, lane.unwrap(), "TextArea");
}

#[test]
fn a_refused_choice_still_moves_the_active_option() -> TestResult {
    // Another local gesture owns the parameter. A refused choice moves only
    // the focused option, which the group must repaint. Remote pending controls
    // are deliberately noneditable and covered by parameter_controls instead.
    let mut ui = runtime(300.0, 200.0);
    let id = |text: &str| ChoiceId::new(text);
    let spec = ParameterSpec::choices(
        "Waveform",
        [
            ChoiceOption::new(id("sine")?, "Sine"),
            ChoiceOption::new(id("saw")?, "Saw"),
            ChoiceOption::new(id("square")?, "Square"),
        ],
        id("sine")?,
    )?;
    let (waveform, _host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, spec)?;
    let group = mount(
        &mut ui,
        Element::new("group", RadioGroup::for_parameter(waveform.clone())),
    );
    let _edit = ui.event(&group, |cx| {
        waveform.begin_edit(cx, waveform.snapshot().get()?.revision)
    })??;
    assert_repaints(
        &mut ui,
        &group,
        &[(KeyCode::ArrowDown, PRESS)],
        "RadioGroup",
    );
    assert_repaints(
        &mut ui,
        &group,
        &[(KeyCode::ArrowDown, PRESS)],
        "RadioGroup again",
    );
    assert_eq!(waveform.snapshot().get()?.base, id("sine")?);
    Ok(())
}
