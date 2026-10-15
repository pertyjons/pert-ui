//! `MultiSelect` contracts: chips and rows keep their identity by value, focus
//! moves between the trigger and the popup without losing choices, the search
//! field drives the highlight, and both commit timings write exactly when
//! documented.
use pert_ui::*;
mod support;
use std::{cell::Cell, rc::Rc};
use support::{frame, key, widget};

const FRUIT: [(&str, &str); 6] = [
    ("apple", "Apple"),
    ("apricot", "Apricot"),
    ("banana", "Banana"),
    ("cherry", "Cherry"),
    ("date", "Date"),
    ("fig", "Fig"),
];

fn options() -> Vec<Choice<&'static str>> {
    FRUIT
        .iter()
        .map(|(value, label)| Choice::new(*value, *label))
        .collect()
}

struct Setup {
    ui: Runtime,
    root: WidgetId,
    value: Signal<Vec<&'static str>>,
}

fn mount(
    chosen: &[&'static str],
    build: impl FnOnce(&Runtime, Signal<Vec<&'static str>>) -> MultiSelect<&'static str>,
) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(chosen.to_vec());
    let chooser = build(&ui, value.clone());
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([
                Element::new("tags", chooser),
                Element::new("after", Button::new("After", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    Setup { ui, root, value }
}

fn plain(chosen: &[&'static str]) -> Setup {
    mount(chosen, |ui, value| {
        MultiSelect::new(ui, value, options()).unwrap()
    })
}

fn on_close(chosen: &[&'static str]) -> Setup {
    mount(chosen, |ui, value| {
        MultiSelect::new(ui, value, options())
            .unwrap()
            .commit(MultiSelectCommit::OnClose)
    })
}

fn press(ui: &mut Runtime, code: KeyCode) {
    key(ui, code);
    frame(ui);
}

/// Routes a key the way the native host does, so the focused search field
/// sees Escape before the popup does.
fn native(ui: &mut Runtime, code: KeyCode) {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
            .unwrap();
    }
    frame(ui);
}

fn click(ui: &mut Runtime, point: Point) {
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    frame(ui);
}

fn center(ui: &Runtime, id: &WidgetId) -> Point {
    let bounds = ui.bounds(id).unwrap().unwrap();
    Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap()
}

fn is_open(ui: &Runtime) -> bool {
    let tags = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "tags")
        .unwrap();
    tags.details.properties["open"] == InspectValue::Bool(true)
}

fn text(details: &ComponentDetails) -> String {
    match &details.properties["text"] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("unexpected text {other:?}"),
    }
}

/// Labels of the visible chips, in order.
fn chips(ui: &Runtime) -> Vec<String> {
    ui.inspect()
        .components
        .iter()
        .filter(|c| c.visible && c.details.properties.contains_key("removable"))
        .map(|c| text(&c.details))
        .collect()
}

/// The visible popup rows: (label, checked, highlighted).
fn rows(ui: &Runtime) -> Vec<(String, bool, bool)> {
    ui.inspect()
        .components
        .iter()
        .filter(|c| c.visible && c.details.properties.contains_key("checked"))
        .map(|c| {
            (
                text(&c.details),
                c.details.properties["checked"] == InspectValue::Bool(true),
                c.details.properties["highlighted"] == InspectValue::Bool(true),
            )
        })
        .collect()
}

fn highlighted(ui: &Runtime) -> Option<String> {
    rows(ui)
        .into_iter()
        .find(|(_, _, highlighted)| *highlighted)
        .map(|(label, ..)| label)
}

/// The widget of the component under `container` whose inspection matches.
fn keyed(ui: &Runtime, root: &WidgetId, container: &str, label: &str, marker: &str) -> WidgetId {
    let key = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.details.properties.contains_key(marker) && text(&c.details) == label)
        .map(|c| c.key)
        .unwrap_or_else(|| panic!("no {marker} component {label}"));
    let container = widget(ui, root, container);
    ui.find_descendant(&container, &key).unwrap().unwrap()
}

fn row(ui: &Runtime, root: &WidgetId, label: &str) -> WidgetId {
    keyed(ui, root, "list", label, "checked")
}

fn chip(ui: &Runtime, root: &WidgetId, label: &str) -> WidgetId {
    keyed(ui, root, "chips", label, "removable")
}

fn focused_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}

fn open_from_trigger(s: &mut Setup) {
    let trigger = widget(&s.ui, &s.root, "anchor");
    s.ui.focus(&trigger).unwrap();
    frame(&mut s.ui);
    press(&mut s.ui, KeyCode::Enter);
    assert!(is_open(&s.ui));
}

#[test]
fn the_trigger_shows_chosen_values_as_chips_in_binding_order() {
    let s = plain(&["cherry", "apple", "kiwi", "apple"]);
    // An unknown value stays in the binding but shows no chip; a repeated value
    // shows one.
    assert_eq!(chips(&s.ui), ["Cherry", "Apple"]);
    assert_eq!(s.value.get().unwrap(), ["cherry", "apple", "kiwi", "apple"]);
    let trigger =
        s.ui.inspect()
            .components
            .into_iter()
            .find(|c| c.key == "anchor")
            .unwrap();
    let semantics = trigger.details.semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::ComboBox);
    assert_eq!(
        semantics.value,
        Some(SemanticValue::Text("Cherry, Apple".into()))
    );
    assert_eq!(semantics.items.len(), FRUIT.len());
    assert!(semantics.items[0].selected && !semantics.items[1].selected);
    assert!(!is_open(&s.ui));
}

#[test]
fn duplicate_option_values_are_rejected() {
    let ui = Runtime::new();
    let value = ui.signal(Vec::<&str>::new());
    let result = MultiSelect::new(
        &ui,
        value,
        [Choice::new("a", "A"), Choice::new("a", "Again")],
    );
    assert!(matches!(result, Err(ChoiceError::DuplicateValue)));
}

#[test]
fn opening_focuses_the_search_field_and_closing_returns_to_the_trigger() {
    let mut s = plain(&["banana"]);
    open_from_trigger(&mut s);
    assert_eq!(focused_key(&s.ui).as_deref(), Some("search"));
    assert_eq!(rows(&s.ui).len(), FRUIT.len());
    // Escape with an empty query closes; the choices stay.
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    assert_eq!(focused_key(&s.ui).as_deref(), Some("anchor"));
    assert_eq!(s.value.get().unwrap(), ["banana"]);
    // Down opens the list too.
    press(&mut s.ui, KeyCode::ArrowDown);
    assert!(is_open(&s.ui));
}

#[test]
fn typing_filters_and_arrows_move_the_highlight_while_focus_stays() {
    let mut s = plain(&[]);
    open_from_trigger(&mut s);
    s.ui.text_input("ap").unwrap();
    frame(&mut s.ui);
    let labels: Vec<_> = rows(&s.ui).into_iter().map(|(label, ..)| label).collect();
    assert_eq!(labels, ["Apple", "Apricot"]);
    // The first match is highlighted before any arrow.
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Apple"));
    press(&mut s.ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Apricot"));
    press(&mut s.ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Apple"), "wraps");
    press(&mut s.ui, KeyCode::ArrowUp);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Apricot"));
    assert_eq!(focused_key(&s.ui).as_deref(), Some("search"));
    // Enter toggles the highlighted option and keeps the popup open.
    press(&mut s.ui, KeyCode::Enter);
    assert_eq!(s.value.get().unwrap(), ["apricot"]);
    assert!(is_open(&s.ui));
    press(&mut s.ui, KeyCode::Enter);
    assert!(s.value.get().unwrap().is_empty());
    // No match shows no rows.
    s.ui.text_input("zz").unwrap();
    frame(&mut s.ui);
    assert!(rows(&s.ui).is_empty());
    press(&mut s.ui, KeyCode::Enter);
    assert!(s.value.get().unwrap().is_empty());
    native(&mut s.ui, KeyCode::Escape);
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    assert!(s.ui.next_deadline().is_none(), "nothing is scheduled");
}

#[test]
fn the_first_escape_clears_the_query_and_the_second_closes() {
    let mut s = plain(&[]);
    open_from_trigger(&mut s);
    s.ui.text_input("ch").unwrap();
    frame(&mut s.ui);
    press(&mut s.ui, KeyCode::Enter);
    assert_eq!(s.value.get().unwrap(), ["cherry"]);
    native(&mut s.ui, KeyCode::Escape);
    assert!(is_open(&s.ui));
    assert_eq!(rows(&s.ui).len(), FRUIT.len());
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    assert_eq!(s.value.get().unwrap(), ["cherry"], "committed choices stay");
    // Reopening starts with an empty query.
    press(&mut s.ui, KeyCode::Enter);
    assert_eq!(rows(&s.ui).len(), FRUIT.len());
}

#[test]
fn the_headless_path_also_clears_the_query_before_closing() {
    let mut s = plain(&[]);
    open_from_trigger(&mut s);
    s.ui.text_input("ch").unwrap();
    frame(&mut s.ui);
    assert_eq!(rows(&s.ui).len(), 1);
    // `key_input`, unlike the native path, used to close the popup first.
    press(&mut s.ui, KeyCode::Escape);
    assert!(is_open(&s.ui), "the first Escape clears the query");
    assert_eq!(rows(&s.ui).len(), FRUIT.len());
    press(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui), "the second Escape closes");
}

#[test]
fn a_click_on_a_row_toggles_it_without_moving_focus() {
    let mut s = plain(&["fig"]);
    open_from_trigger(&mut s);
    let date = row(&s.ui, &s.root, "Date");
    let point = center(&s.ui, &date);
    click(&mut s.ui, point);
    assert_eq!(s.value.get().unwrap(), ["fig", "date"]);
    assert_eq!(focused_key(&s.ui).as_deref(), Some("search"));
    assert!(is_open(&s.ui));
    assert!(
        rows(&s.ui)
            .iter()
            .any(|(label, checked, _)| label == "Date" && *checked)
    );
    assert_eq!(chips(&s.ui), ["Fig", "Date"]);
}

#[test]
fn on_close_edits_a_draft_that_escape_discards() {
    let mut s = on_close(&["apple"]);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::Enter); // Apple off
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter); // Apricot on
    assert_eq!(
        s.value.get().unwrap(),
        ["apple"],
        "the draft is not written"
    );
    assert!(
        rows(&s.ui)
            .iter()
            .any(|(label, checked, _)| label == "Apricot" && *checked)
    );
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    assert_eq!(s.value.get().unwrap(), ["apple"]);
}

#[test]
fn on_close_writes_the_draft_once_on_an_outside_press_or_tab() {
    let mut s = on_close(&["apple"]);
    let writes = Rc::new(Cell::new(0));
    let counter = writes.clone();
    let observer = s
        .value
        .read_only()
        .map(move |_| counter.set(counter.get() + 1))
        .unwrap();
    open_from_trigger(&mut s);
    let before = writes.get();
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    observer.get().unwrap();
    assert_eq!(writes.get(), before, "toggles write nothing");
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&s.ui));
    observer.get().unwrap();
    assert_eq!(s.value.get().unwrap(), ["apple", "apricot", "banana"]);
    assert_eq!(writes.get(), before + 1, "one write for the whole draft");

    // Tab out of the popup commits as well.
    let trigger = widget(&s.ui, &s.root, "anchor");
    s.ui.focus(&trigger).unwrap();
    press(&mut s.ui, KeyCode::Enter);
    press(&mut s.ui, KeyCode::Enter); // Apple off
    press(&mut s.ui, KeyCode::Tab);
    assert!(!is_open(&s.ui));
    assert_ne!(focused_key(&s.ui).as_deref(), Some("search"));
    assert_eq!(s.value.get().unwrap(), ["apricot", "banana"]);
}

#[test]
fn focus_moved_to_the_trigger_closes_the_popup_and_keeps_focus_there() {
    // Shift+Tab out of the search field and a programmatic focus of the
    // trigger both leave the popup, though the trigger lies inside the
    // chooser that owns it.
    for exit in ["shift-tab", "focus"] {
        let mut s = plain(&["apple"]);
        open_from_trigger(&mut s);
        assert!(is_open(&s.ui), "{exit}: opened");
        let trigger = widget(&s.ui, &s.root, "anchor");
        if exit == "focus" {
            s.ui.focus(&trigger).unwrap();
            frame(&mut s.ui);
        } else {
            s.ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)
                .unwrap();
            press(&mut s.ui, KeyCode::Tab);
            s.ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
                .unwrap();
            frame(&mut s.ui);
        }
        assert!(!is_open(&s.ui), "{exit}: popup stayed open");
        assert_ne!(focused_key(&s.ui).as_deref(), Some("search"), "{exit}");
        if exit == "focus" {
            assert_eq!(
                s.ui.focused(),
                Some(trigger),
                "{exit}: focus stays on the trigger"
            );
        }
        assert_eq!(
            s.value.get().unwrap(),
            ["apple"],
            "{exit}: no choice changed"
        );
    }
}

#[test]
fn on_close_lets_an_application_write_during_the_popup_win() {
    let mut s = on_close(&["apple"]);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    s.value.set(vec!["fig"]).unwrap();
    frame(&mut s.ui);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&s.ui));
    assert_eq!(s.value.get().unwrap(), ["fig"]);
}

#[test]
fn hiding_the_chooser_discards_an_open_draft() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(vec!["date"]);
    let visible = ui.signal(true);
    let chooser = MultiSelect::new(&ui, value.clone(), options())
        .unwrap()
        .commit(MultiSelectCommit::OnClose);
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([Element::new(
                "view",
                View::new(Element::new(
                    "inner",
                    Column::new([Element::new("tags", chooser)]),
                ))
                .visible_signal(visible.read_only()),
            )]),
        ))
        .unwrap();
    frame(&mut ui);
    let trigger = widget(&ui, &root, "anchor");
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter);
    visible.set(false).unwrap();
    frame(&mut ui);
    visible.set(true).unwrap();
    frame(&mut ui);
    assert!(!is_open(&ui));
    assert_eq!(value.get().unwrap(), ["date"]);
}

#[test]
fn removing_the_focused_chip_moves_focus_to_the_next_then_the_trigger() {
    let mut s = plain(&["apple", "banana", "cherry"]);
    let banana = chip(&s.ui, &s.root, "Banana");
    s.ui.focus(&banana).unwrap();
    press(&mut s.ui, KeyCode::Delete);
    assert_eq!(s.value.get().unwrap(), ["apple", "cherry"]);
    assert_eq!(chips(&s.ui), ["Apple", "Cherry"]);
    let focused = s.ui.focused().unwrap();
    assert_eq!(focused, chip(&s.ui, &s.root, "Cherry"));
    press(&mut s.ui, KeyCode::Backspace);
    assert_eq!(s.ui.focused().unwrap(), chip(&s.ui, &s.root, "Apple"));
    press(&mut s.ui, KeyCode::Delete);
    assert!(s.value.get().unwrap().is_empty());
    assert_eq!(focused_key(&s.ui).as_deref(), Some("anchor"));
}

#[test]
fn chips_and_rows_keep_their_identity_when_options_are_reordered() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(vec!["banana", "date"]);
    let source = ui.signal(options());
    let chooser = MultiSelect::bound(&ui, value.clone(), source.read_only());
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([Element::new("tags", chooser)]),
        ))
        .unwrap();
    frame(&mut ui);
    let banana = chip(&ui, &root, "Banana");
    let trigger = widget(&ui, &root, "anchor");
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&ui).as_deref(), Some("Banana"));
    let banana_row = row(&ui, &root, "Banana");
    // Reverse and relabel the options.
    let mut reordered: Vec<_> = FRUIT
        .iter()
        .rev()
        .map(|(value, label)| Choice::new(*value, label.to_uppercase()))
        .collect();
    reordered.retain(|choice| *choice.value() != "apple");
    source.set(reordered).unwrap();
    frame(&mut ui);
    assert_eq!(chips(&ui), ["BANANA", "DATE"]);
    assert_eq!(chip(&ui, &root, "BANANA"), banana, "same chip");
    assert_eq!(row(&ui, &root, "BANANA"), banana_row, "same row");
    assert_eq!(highlighted(&ui).as_deref(), Some("BANANA"));
    assert_eq!(value.get().unwrap(), ["banana", "date"]);
    // A duplicate in a dynamic source fails the update.
    source
        .set(vec![Choice::new("fig", "Fig"), Choice::new("fig", "Fig")])
        .unwrap();
    assert!(ui.flush(ui.now()).is_err());
}

/// A value whose every instance hashes alike, to force key collisions.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Colliding(&'static str);
impl std::hash::Hash for Colliding {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        0_u8.hash(state);
    }
}

#[test]
fn colliding_values_keep_their_widgets_when_options_are_reordered() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(vec![Colliding("a"), Colliding("b")]);
    let choices = |order: [&'static str; 2]| {
        order
            .map(|name| Choice::new(Colliding(name), name.to_uppercase()))
            .to_vec()
    };
    let source = ui.signal(choices(["a", "b"]));
    let chooser = MultiSelect::bound(&ui, value.clone(), source.read_only());
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([Element::new("tags", chooser)]),
        ))
        .unwrap();
    frame(&mut ui);
    let a = chip(&ui, &root, "A");
    let b = chip(&ui, &root, "B");
    // Reordering must not swap which value each retained chip denotes.
    source.set(choices(["b", "a"])).unwrap();
    frame(&mut ui);
    assert_eq!(chip(&ui, &root, "A"), a, "A keeps its chip");
    assert_eq!(chip(&ui, &root, "B"), b, "B keeps its chip");
}

#[test]
fn on_close_drops_the_draft_after_a_write_that_restored_the_value() {
    let mut s = on_close(&["apple"]);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter); // Apricot on in the draft
    // The application writes another value and then the original again.
    s.value.set(vec!["fig"]).unwrap();
    frame(&mut s.ui);
    s.value.set(vec!["apple"]).unwrap();
    frame(&mut s.ui);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&s.ui));
    assert_eq!(
        s.value.get().unwrap(),
        ["apple"],
        "the application's write wins"
    );
}

#[test]
fn disabled_options_and_a_disabled_chooser_refuse_input() {
    let mut s = mount(&["banana"], |ui, value| {
        let mut options = options();
        options[1] = Choice::new("apricot", "Apricot").disabled();
        options[2] = Choice::new("banana", "Banana").disabled();
        MultiSelect::new(ui, value, options).unwrap()
    });
    // The chip of a disabled option cannot be removed.
    let banana =
        s.ui.inspect()
            .components
            .into_iter()
            .find(|c| c.details.properties.contains_key("removable"))
            .unwrap();
    assert!(banana.details.semantics.unwrap().disabled);
    open_from_trigger(&mut s);
    // The highlight skips disabled rows.
    press(&mut s.ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Cherry"));
    let apricot = row(&s.ui, &s.root, "Apricot");
    let point = center(&s.ui, &apricot);
    click(&mut s.ui, point);
    assert_eq!(s.value.get().unwrap(), ["banana"]);

    let disabled = mount(&["fig"], |ui, value| {
        MultiSelect::new(ui, value, options()).unwrap().disabled()
    });
    // Neither the trigger nor the muted chip is a Tab stop.
    let mut ui = disabled.ui;
    ui.focus_next(false).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("after"));
}

#[test]
fn a_custom_filter_decides_which_options_are_listed() {
    let mut s = mount(&[], |ui, value| {
        MultiSelect::new(ui, value, options())
            .unwrap()
            .filter(|query, choice| choice.value().starts_with(query))
    });
    open_from_trigger(&mut s);
    s.ui.text_input("a").unwrap();
    frame(&mut s.ui);
    let labels: Vec<_> = rows(&s.ui).into_iter().map(|(label, ..)| label).collect();
    assert_eq!(labels, ["Apple", "Apricot"]);
}

#[test]
fn a_replaced_binding_never_receives_an_old_draft() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let first = ui.signal(vec!["apple"]);
    let second = ui.signal(vec!["cherry"]);
    let describe = |ui: &Runtime, value: Signal<Vec<&'static str>>| {
        Element::new(
            "tags",
            MultiSelect::new(ui, value, options())
                .unwrap()
                .commit(MultiSelectCommit::OnClose),
        )
    };
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([describe(&ui, first.clone())]),
        ))
        .unwrap();
    frame(&mut ui);
    let trigger = widget(&ui, &root, "anchor");
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter); // Apricot into the draft
    ui.reconcile_children(&root, vec![describe(&ui, second.clone())])
        .unwrap();
    frame(&mut ui);
    assert!(is_open(&ui), "describing again keeps the popup");
    click(&mut ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&ui));
    assert_eq!(first.get().unwrap(), ["apple"]);
    assert_eq!(second.get().unwrap(), ["cherry"]);
    assert_eq!(chips(&ui), ["Cherry"]);
}

#[test]
fn pressing_the_trigger_while_open_closes_without_reopening() {
    let mut s = on_close(&[]);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::Enter); // Apple into the draft
    let trigger = widget(&s.ui, &s.root, "anchor");
    let point = center(&s.ui, &trigger);
    click(&mut s.ui, point);
    assert!(!is_open(&s.ui), "the outside press closes and is consumed");
    assert_eq!(s.value.get().unwrap(), ["apple"]);
}

#[test]
fn removing_the_chooser_with_an_open_draft_writes_nothing() {
    let mut s = on_close(&["fig"]);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::Enter);
    s.ui.reconcile_children(&s.root, Vec::new()).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.value.get().unwrap(), ["fig"]);
}

#[test]
fn an_open_popup_refuses_toggles_after_the_dropdown_is_disabled() {
    for commit in [MultiSelectCommit::EachToggle, MultiSelectCommit::OnClose] {
        let mut s = mount(&["apple"], |ui, value| {
            MultiSelect::new(ui, value, options())
                .unwrap()
                .commit(commit)
        });
        open_from_trigger(&mut s);
        // An OnClose draft holds a toggle made before disabling.
        let banana = row(&s.ui, &s.root, "Banana");
        let at = center(&s.ui, &banana);
        click(&mut s.ui, at);
        let value = s.value.clone();
        let disabled = MultiSelect::new(&s.ui, value, options())
            .unwrap()
            .commit(commit)
            .disabled();
        s.ui.reconcile_children(
            &s.root,
            vec![
                Element::new("tags", disabled),
                Element::new("after", Button::new("After", |_| Ok(()))),
            ],
        )
        .unwrap();
        frame(&mut s.ui);
        let before = s.value.get().unwrap();
        assert!(is_open(&s.ui), "{commit:?}: the popup stays open");
        let disabled_rows =
            s.ui.inspect()
                .components
                .iter()
                .filter(|c| c.visible && c.details.properties.contains_key("checked"))
                .all(|c| c.details.semantics.as_ref().is_some_and(|s| s.disabled));
        assert!(disabled_rows, "{commit:?}: open rows show as disabled");
        let cherry = row(&s.ui, &s.root, "Cherry");
        let at = center(&s.ui, &cherry);
        click(&mut s.ui, at);
        assert_eq!(s.value.get().unwrap(), before, "{commit:?}: no toggle");
        native(&mut s.ui, KeyCode::Escape);
        native(&mut s.ui, KeyCode::Escape);
        assert!(!is_open(&s.ui), "{commit:?}: Escape closes");
        assert_eq!(
            s.value.get().unwrap(),
            before,
            "{commit:?}: no draft is written"
        );
    }
}

#[test]
fn reactive_permission_cancels_row_capture_and_checks_drafts_at_close() {
    for reenable_before_close in [false, true] {
        let mut permission = None;
        let mut s = mount(&["apple"], |ui, value| {
            let allowed = ui.signal(true);
            permission = Some(allowed.clone());
            MultiSelect::new(ui, value, options())
                .unwrap()
                .commit(MultiSelectCommit::OnClose)
                .enabled(allowed.read_only())
        });
        let allowed = permission.unwrap();
        open_from_trigger(&mut s);
        let banana = row(&s.ui, &s.root, "Banana");
        let at = center(&s.ui, &banana);
        let down = PointerEvent::new(at, PointerKind::Press(PointerButton::Primary));
        let up = PointerEvent::new(at, PointerKind::Release(PointerButton::Primary));
        s.ui.pointer_input(down).unwrap();
        allowed.set(false).unwrap();
        frame(&mut s.ui);
        assert!(is_open(&s.ui));
        assert!(
            s.ui.inspect()
                .components
                .iter()
                .filter(|c| c.visible && c.details.properties.contains_key("checked"))
                .all(|c| c.details.semantics.as_ref().unwrap().disabled)
        );
        allowed.set(true).unwrap();
        frame(&mut s.ui);
        s.ui.pointer_input(up).unwrap();
        frame(&mut s.ui);
        let checked = |ui: &Runtime| {
            ui.inspect()
                .components
                .iter()
                .find(|c| {
                    c.details
                        .semantics
                        .as_ref()
                        .is_some_and(|s| s.name.as_deref() == Some("Banana"))
                })
                .unwrap()
                .details
                .properties
                .get("checked")
                .cloned()
        };
        assert_eq!(
            checked(&s.ui),
            Some(InspectValue::Bool(false)),
            "revoked row press cannot resume"
        );
        click(&mut s.ui, at);
        assert_eq!(
            checked(&s.ui),
            Some(InspectValue::Bool(true)),
            "fresh press toggles draft"
        );
        allowed.set(false).unwrap();
        frame(&mut s.ui);
        if reenable_before_close {
            allowed.set(true).unwrap();
            frame(&mut s.ui);
        }
        // Outside dismissal normally commits an OnClose draft.
        click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
        assert!(!is_open(&s.ui));
        assert_eq!(
            s.value.get().unwrap(),
            if reenable_before_close {
                vec!["apple", "banana"]
            } else {
                vec!["apple"]
            }
        );
    }
}

#[test]
fn replaced_multi_select_permission_refreshes_retained_rows_and_unsubscribes() {
    let mut permissions = None;
    let mut s = mount(&["apple"], |ui, value| {
        let old = ui.signal(true);
        let next = ui.signal(true);
        permissions = Some((old.clone(), next));
        MultiSelect::new(ui, value, options())
            .unwrap()
            .enabled(old.read_only())
    });
    let (old, next) = permissions.unwrap();
    open_from_trigger(&mut s);
    let banana = row(&s.ui, &s.root, "Banana");
    let held_at = center(&s.ui, &banana);
    s.ui.pointer_input(PointerEvent::new(
        held_at,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    let replacement = MultiSelect::new(&s.ui, s.value.clone(), options())
        .unwrap()
        .enabled(next.read_only());
    s.ui.reconcile_children(
        &s.root,
        vec![
            Element::new("tags", replacement),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut s.ui);
    assert_eq!(row(&s.ui, &s.root, "Banana"), banana);
    let cherry = row(&s.ui, &s.root, "Cherry");
    let moved_at = center(&s.ui, &cherry);
    s.ui.pointer_input(PointerEvent::new(moved_at, PointerKind::Move))
        .unwrap();
    frame(&mut s.ui);
    assert_eq!(
        highlighted(&s.ui).as_deref(),
        Some("Cherry"),
        "capture must be released before physical release"
    );
    s.ui.pointer_input(PointerEvent::new(
        held_at,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    frame(&mut s.ui);
    assert_eq!(s.value.get().unwrap(), ["apple"]);
    old.set(false).unwrap();
    assert_eq!(s.ui.flush(s.ui.now()).unwrap().components_updated, 0);
    next.set(false).unwrap();
    frame(&mut s.ui);
    let at = center(&s.ui, &banana);
    click(&mut s.ui, at);
    assert_eq!(s.value.get().unwrap(), ["apple"]);
    next.set(true).unwrap();
    frame(&mut s.ui);
    click(&mut s.ui, at);
    assert_eq!(s.value.get().unwrap(), ["apple", "banana"]);
}

#[test]
fn reactive_permission_cancels_chip_removal_and_hidden_controls_refresh() {
    let mut permission = None;
    let mut s = mount(&["apple"], |ui, value| {
        let allowed = ui.signal(true);
        permission = Some(allowed.clone());
        MultiSelect::new(ui, value, options())
            .unwrap()
            .enabled(allowed.read_only())
    });
    let allowed = permission.unwrap();
    let apple = chip(&s.ui, &s.root, "Apple");
    let bounds = s.ui.bounds(&apple).unwrap().unwrap();
    let at = Point::new(
        bounds.x() + bounds.width() - 8.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap();
    s.ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    allowed.set(false).unwrap();
    frame(&mut s.ui);
    allowed.set(true).unwrap();
    frame(&mut s.ui);
    s.ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    assert_eq!(s.value.get().unwrap(), ["apple"]);
    click(&mut s.ui, at);
    assert!(
        s.value.get().unwrap().is_empty(),
        "fresh pointer removes the chip"
    );
    s.ui.set_visible(&s.root, false).unwrap();
    frame(&mut s.ui);
    allowed.set(false).unwrap();
    assert_eq!(s.ui.flush(s.ui.now()).unwrap().components_updated, 0);
    s.ui.set_visible(&s.root, true).unwrap();
    frame(&mut s.ui);
    let trigger = widget(&s.ui, &s.root, "anchor");
    s.ui.focus(&trigger).unwrap();
    press(&mut s.ui, KeyCode::Enter);
    assert!(!is_open(&s.ui));
}

#[test]
fn replacing_sources_keeps_draft_row_identity_and_updates_highlight_subscriptions() {
    let mut sources = None;
    let mut s = mount(&["apple"], |ui, value| {
        let old = ui.signal(options());
        let replacement = ui.signal(
            options()
                .into_iter()
                .map(|choice| {
                    if *choice.value() == "apple" {
                        Choice::new("apple", "Renamed apple")
                    } else {
                        choice
                    }
                })
                .collect::<Vec<_>>(),
        );
        sources = Some((old.clone(), replacement));
        MultiSelect::bound(ui, value, old.read_only()).commit(MultiSelectCommit::OnClose)
    });
    let (old, replacement) = sources.unwrap();
    open_from_trigger(&mut s);
    let banana = row(&s.ui, &s.root, "Banana");
    let at = center(&s.ui, &banana);
    click(&mut s.ui, at);
    assert_eq!(s.value.get().unwrap(), ["apple"]);
    let next = MultiSelect::bound(&s.ui, s.value.clone(), replacement.read_only())
        .commit(MultiSelectCommit::OnClose);
    s.ui.reconcile_children(
        &s.root,
        vec![
            Element::new("tags", next),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut s.ui);
    assert_eq!(row(&s.ui, &s.root, "Banana"), banana);
    assert!(
        rows(&s.ui)
            .iter()
            .any(|(label, _, _)| label == "Renamed apple")
    );
    assert!(
        rows(&s.ui)
            .iter()
            .any(|(label, checked, _)| label == "Banana" && *checked)
    );
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Banana"));
    s.ui.pointer_input(PointerEvent::new(
        Point::new(0.0, 0.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    native(&mut s.ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Cherry"));
    old.set(vec![Choice::new("old", "Unused")]).unwrap();
    assert_eq!(s.ui.flush(s.ui.now()).unwrap().components_updated, 0);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&s.ui));
    assert_eq!(s.value.get().unwrap(), ["apple", "banana"]);
}

#[test]
fn replacing_filter_updates_open_rows_without_restarting_the_session() {
    let mut s = plain(&["apple"]);
    open_from_trigger(&mut s);
    let banana = row(&s.ui, &s.root, "Banana");
    let next = MultiSelect::new(&s.ui, s.value.clone(), options())
        .unwrap()
        .filter(|_, choice| *choice.value() == "banana");
    s.ui.reconcile_children(
        &s.root,
        vec![
            Element::new("tags", next),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut s.ui);
    assert!(is_open(&s.ui));
    assert_eq!(
        rows(&s.ui)
            .iter()
            .map(|(label, _, _)| label.as_str())
            .collect::<Vec<_>>(),
        ["Banana"]
    );
    assert_eq!(row(&s.ui, &s.root, "Banana"), banana);
    native(&mut s.ui, KeyCode::Enter);
    assert_eq!(s.value.get().unwrap(), ["apple", "banana"]);
    let next = MultiSelect::new(&s.ui, s.value.clone(), options()).unwrap();
    s.ui.reconcile_children(
        &s.root,
        vec![
            Element::new("tags", next),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )
    .unwrap();
    frame(&mut s.ui);
    assert_eq!(rows(&s.ui).len(), FRUIT.len());
    assert_eq!(row(&s.ui, &s.root, "Banana"), banana);
    native(&mut s.ui, KeyCode::ArrowDown);
    assert_eq!(highlighted(&s.ui).as_deref(), Some("Apricot"));
}

#[test]
fn invalid_replacement_keeps_valid_rows_then_recovers_and_refreshes_after_reopen() {
    let mut sources = None;
    let mut s = mount(&[], |ui, value| {
        let old = ui.signal(options());
        let next = ui.signal(vec![
            Choice::new("new", "New"),
            Choice::new("new", "Duplicate"),
        ]);
        sources = Some((old.clone(), next));
        MultiSelect::bound(ui, value, old.read_only())
    });
    let (_old, next) = sources.unwrap();
    open_from_trigger(&mut s);
    let before = rows(&s.ui);
    let replacement = MultiSelect::bound(&s.ui, s.value.clone(), next.read_only());
    s.ui.reconcile_children(
        &s.root,
        vec![
            Element::new("tags", replacement),
            Element::new("after", Button::new("After", |_| Ok(()))),
        ],
    )
    .unwrap();
    assert_eq!(
        s.ui.flush(s.ui.now()),
        Err(RuntimeError::Choice(ChoiceError::DuplicateValue))
    );
    assert_eq!(rows(&s.ui), before);
    next.set(vec![Choice::new("new", "New")]).unwrap();
    frame(&mut s.ui);
    assert_eq!(
        rows(&s.ui)
            .iter()
            .map(|(label, _, _)| label.as_str())
            .collect::<Vec<_>>(),
        ["New"]
    );
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    next.set(vec![Choice::new("final", "Final")]).unwrap();
    frame(&mut s.ui);
    open_from_trigger(&mut s);
    assert_eq!(
        rows(&s.ui)
            .iter()
            .map(|(label, _, _)| label.as_str())
            .collect::<Vec<_>>(),
        ["Final"]
    );
}

type Reports = Rc<std::cell::RefCell<Vec<Vec<&'static str>>>>;

/// A handler that records each reported value.
fn recorder(
    reports: &Reports,
) -> impl FnMut(&mut EventContext, &[&'static str]) -> Result<(), RuntimeError> + 'static {
    let reports = reports.clone();
    move |_, values| {
        reports.borrow_mut().push(values.to_vec());
        Ok(())
    }
}

fn reporting(chosen: &[&'static str], commit: MultiSelectCommit) -> (Setup, Reports) {
    let reports = Reports::default();
    let handler = recorder(&reports);
    let s = mount(chosen, move |ui, value| {
        MultiSelect::new(ui, value, options())
            .unwrap()
            .commit(commit)
            .on_change(handler)
    });
    (s, reports)
}

#[test]
fn each_toggle_reports_user_toggles_and_chip_removals_but_not_application_writes() {
    let (mut s, reports) = reporting(&["apple"], MultiSelectCommit::EachToggle);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter); // Apricot on, from the search field
    let banana = row(&s.ui, &s.root, "Banana");
    let at = center(&s.ui, &banana);
    click(&mut s.ui, at); // Banana on, from a row
    assert_eq!(
        *reports.borrow(),
        [vec!["apple", "apricot"], vec!["apple", "apricot", "banana"]]
    );
    native(&mut s.ui, KeyCode::Escape);
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    s.value.set(vec!["apple", "banana"]).unwrap();
    frame(&mut s.ui);
    assert_eq!(
        reports.borrow().len(),
        2,
        "an application write is not a user write"
    );
    let apple = chip(&s.ui, &s.root, "Apple");
    s.ui.focus(&apple).unwrap();
    press(&mut s.ui, KeyCode::Delete);
    assert_eq!(s.value.get().unwrap(), ["banana"]);
    assert_eq!(reports.borrow().last().unwrap(), &["banana"]);
    assert_eq!(reports.borrow().len(), 3);
}

#[test]
fn on_close_reports_only_a_draft_that_was_written() {
    let (mut s, reports) = reporting(&["apple"], MultiSelectCommit::OnClose);
    // Draft toggles and Escape report nothing.
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    assert!(reports.borrow().is_empty(), "a draft toggle is not a write");
    native(&mut s.ui, KeyCode::Escape);
    native(&mut s.ui, KeyCode::Escape);
    assert!(!is_open(&s.ui));
    assert!(reports.borrow().is_empty(), "Escape discards");

    // An unchanged draft writes and reports nothing.
    open_from_trigger(&mut s);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert!(!is_open(&s.ui));
    assert!(
        reports.borrow().is_empty(),
        "an unchanged draft writes nothing"
    );

    // An outside press writes the draft and reports it once.
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert_eq!(*reports.borrow(), [vec!["apple", "apricot"]]);

    // Tab out of the popup is a committing close as well.
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::Enter); // Apple off
    press(&mut s.ui, KeyCode::Tab);
    assert!(!is_open(&s.ui));
    assert_eq!(reports.borrow().last().unwrap(), &["apricot"]);
    assert_eq!(reports.borrow().len(), 2);

    // A draft that loses to an application write reports nothing.
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    s.value.set(vec!["fig"]).unwrap();
    frame(&mut s.ui);
    click(&mut s.ui, Point::new(470.0, 410.0).unwrap());
    assert_eq!(s.value.get().unwrap(), ["fig"]);
    assert_eq!(
        reports.borrow().len(),
        2,
        "the losing draft is not reported"
    );
}

#[test]
fn on_close_focus_on_a_chip_commits_the_draft_and_its_removal_reports_again() {
    let (mut s, reports) = reporting(&["apple", "fig"], MultiSelectCommit::OnClose);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter); // Apricot into the draft
    // Focus leaving the popup for a chip is a committing close.
    let fig = chip(&s.ui, &s.root, "Fig");
    s.ui.focus(&fig).unwrap();
    frame(&mut s.ui);
    assert!(!is_open(&s.ui));
    assert_eq!(*reports.borrow(), [vec!["apple", "fig", "apricot"]]);
    // A chip removal writes at once under OnClose too.
    let fig = chip(&s.ui, &s.root, "Fig");
    s.ui.focus(&fig).unwrap();
    press(&mut s.ui, KeyCode::Delete);
    assert_eq!(s.value.get().unwrap(), ["apple", "apricot"]);
    assert_eq!(reports.borrow().last().unwrap(), &["apple", "apricot"]);
    assert_eq!(reports.borrow().len(), 2);
}

#[test]
fn hiding_or_removing_the_chooser_reports_no_discarded_draft() {
    let (mut s, reports) = reporting(&["fig"], MultiSelectCommit::OnClose);
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::Enter);
    s.ui.reconcile_children(&s.root, Vec::new()).unwrap();
    frame(&mut s.ui);
    assert_eq!(s.value.get().unwrap(), ["fig"]);
    assert!(reports.borrow().is_empty());
}

#[test]
fn a_disabled_chooser_reports_no_refused_removal() {
    let reports = Reports::default();
    let handler = recorder(&reports);
    let mut s = mount(&["apple", "fig"], move |ui, value| {
        MultiSelect::new(ui, value, options())
            .unwrap()
            .disabled()
            .on_change(handler)
    });
    let apple = chip(&s.ui, &s.root, "Apple");
    let at = center(&s.ui, &apple);
    click(&mut s.ui, at);
    let _ = s.ui.focus(&apple);
    press(&mut s.ui, KeyCode::Delete);
    assert_eq!(s.value.get().unwrap(), ["apple", "fig"]);
    assert!(reports.borrow().is_empty());
}

#[test]
fn a_failing_handler_keeps_the_write_closes_the_popup_and_is_not_replayed() {
    let calls = Rc::new(Cell::new(0));
    let counter = calls.clone();
    let mut s = mount(&["apple"], move |ui, value| {
        MultiSelect::new(ui, value, options())
            .unwrap()
            .commit(MultiSelectCommit::OnClose)
            .on_change(move |_, _| {
                counter.set(counter.get() + 1);
                Err(RuntimeError::BorrowConflict)
            })
    });
    open_from_trigger(&mut s);
    press(&mut s.ui, KeyCode::ArrowDown);
    press(&mut s.ui, KeyCode::Enter);
    let outside = Point::new(470.0, 410.0).unwrap();
    let mut failed = false;
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        failed |=
            s.ui.pointer_input(PointerEvent::new(outside, kind))
                .is_err();
        failed |= s.ui.flush(s.ui.now()).is_err();
    }
    assert!(failed, "the handler's error propagates");
    for _ in 0..3 {
        let _ = s.ui.flush(s.ui.now());
    }
    assert_eq!(
        s.value.get().unwrap(),
        ["apple", "apricot"],
        "the write stays"
    );
    assert!(!is_open(&s.ui), "the popup closed");
    assert_eq!(calls.get(), 1, "the failed report is not replayed");
}

#[test]
fn redescription_replaces_or_removes_the_handler_and_keeps_the_open_draft() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 420.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(vec!["apple"]);
    let first = Reports::default();
    let second = Reports::default();
    let describe = |ui: &Runtime, handler: Option<&Reports>| {
        let chooser = MultiSelect::new(ui, value.clone(), options())
            .unwrap()
            .commit(MultiSelectCommit::OnClose);
        let chooser = match handler {
            Some(reports) => chooser.on_change(recorder(reports)),
            None => chooser,
        };
        Element::new("tags", chooser)
    };
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([describe(&ui, Some(&first))]),
        ))
        .unwrap();
    frame(&mut ui);
    let trigger = widget(&ui, &root, "anchor");
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter); // Apricot into the draft
    ui.reconcile_children(&root, vec![describe(&ui, Some(&second))])
        .unwrap();
    frame(&mut ui);
    assert!(is_open(&ui), "the popup stays open");
    click(&mut ui, Point::new(470.0, 410.0).unwrap());
    assert_eq!(
        value.get().unwrap(),
        ["apple", "apricot"],
        "the draft survived"
    );
    assert!(first.borrow().is_empty());
    assert_eq!(*second.borrow(), [vec!["apple", "apricot"]]);

    ui.reconcile_children(&root, vec![describe(&ui, None)])
        .unwrap();
    frame(&mut ui);
    ui.focus(&widget(&ui, &root, "anchor")).unwrap();
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter); // Apple off
    click(&mut ui, Point::new(470.0, 410.0).unwrap());
    assert_eq!(value.get().unwrap(), ["apricot"]);
    assert_eq!(second.borrow().len(), 1, "a removed handler is not called");
}
