//! Which input owns a menu's or dropdown's highlighted row: a pointer resting
//! where a list opens, or one the runtime's refreshed move reports again after
//! layout, never takes the keyboard's choice. Only motion to a new position, a
//! press or a wheel that scrolls hands the highlight to the pointer, and a
//! menu bar switches menus only on actual motion to another title. Driven
//! through the public facade, reading the highlight, the chosen value and the
//! actions that ran.
use crate::support;
use pert_ui::testing::Driver;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{find, press, settle};

type Log = Rc<RefCell<Vec<String>>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}

/// A host-reported pointer event at `at`, then a settle, which runs any
/// layout and the runtime's refreshed move that follows it.
#[track_caller]
fn host(ui: &mut Runtime, at: (f32, f32), kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(at.0, at.1).unwrap(), kind))
        .unwrap();
    settle(ui);
}
#[track_caller]
fn move_to(ui: &mut Runtime, at: (f32, f32)) {
    host(ui, at, PointerKind::Move);
}
/// The pointer leaving the window from `at`, as the native adapter reports it.
#[track_caller]
fn leave_window(ui: &mut Runtime, at: (f32, f32)) {
    let leave = PointerEvent::new(Point::new(at.0, at.1).unwrap(), PointerKind::Leave);
    ui.pointer_input(leave.outside()).unwrap();
    settle(ui);
}
#[track_caller]
fn wheel(ui: &mut Runtime, at: (f32, f32), dy: f32) {
    host(ui, at, PointerKind::Scroll(Point::new(0.0, dy).unwrap()));
}
/// One transition of `code`, then a settle.
#[track_caller]
fn key_edge(ui: &mut Runtime, code: KeyCode, action: KeyAction) {
    ui.key_input(code, action).unwrap();
    settle(ui);
}

#[track_caller]
fn part(ui: &Runtime, owner: &WidgetId, name: &str) -> WidgetId {
    ui.find_exposed_part(owner, name)
        .unwrap()
        .unwrap_or_else(|| panic!("no part {name:?}"))
}
#[track_caller]
fn semantics(ui: &mut Runtime, id: &WidgetId) -> Semantics {
    Driver::new(ui).semantics(id).unwrap()
}
/// The name of the highlighted row of `list`.
#[track_caller]
fn highlight(ui: &mut Runtime, list: &WidgetId) -> Option<String> {
    semantics(ui, list)
        .items
        .into_iter()
        .find(|item| item.selected)
        .and_then(|item| item.name)
}
/// The middle of the visible row of `list` named `name`, in window pixels.
#[track_caller]
fn row(ui: &mut Runtime, list: &WidgetId, name: &str) -> (f32, f32) {
    let item = semantics(ui, list)
        .items
        .into_iter()
        .find(|item| item.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no row {name:?}"));
    let [x, y, w, h] = item
        .bounds
        .unwrap_or_else(|| panic!("row {name:?} is not visible"));
    (x + w / 2.0, y + h / 2.0)
}
/// The name of the visible row of `list` under `at`.
#[track_caller]
fn row_under(ui: &mut Runtime, list: &WidgetId, at: (f32, f32)) -> Option<String> {
    semantics(ui, list)
        .items
        .into_iter()
        .find(|item| {
            item.bounds
                .is_some_and(|[x, y, w, h]| at.0 >= x && at.0 < x + w && at.1 >= y && at.1 < y + h)
        })
        .and_then(|item| item.name)
}
fn expanded(ui: &mut Runtime, list: &WidgetId, name: &str) -> Option<bool> {
    semantics(ui, list)
        .items
        .into_iter()
        .find(|item| item.name.as_deref() == Some(name))
        .and_then(|item| item.expanded)
}
fn shown(ui: &Runtime, id: &WidgetId) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == ui.key(id).unwrap() && component.visible)
        && ui.bounds(id).unwrap().is_some()
}

/// A menu row that records its name when it runs.
fn logged(log: &Log, name: &str) -> MenuItem {
    let log = log.clone();
    let ran = name.to_owned();
    MenuItem::new(name.to_lowercase(), name, move |_| {
        log.borrow_mut().push(ran.clone());
        Ok(())
    })
}
fn logged_rows(log: &Log, names: &[&str]) -> Vec<MenuItem> {
    names.iter().map(|name| logged(log, name)).collect()
}
fn long_names() -> Vec<String> {
    (0..30).map(|index| format!("R{index:02}")).collect()
}
fn long_rows(log: &Log) -> Vec<MenuItem> {
    long_names().iter().map(|name| logged(log, name)).collect()
}
fn ran(log: &Log) -> Vec<String> {
    log.borrow().clone()
}

/// A menu over a note whose text a test changes to cause layout, and so a
/// refreshed move, away from the menu.
struct MenuFixture {
    root: WidgetId,
    trigger: WidgetId,
    items: WidgetId,
    note: Signal<String>,
}
fn mount_menu(ui: &mut Runtime, rows: Vec<MenuItem>) -> MenuFixture {
    let note = ui.signal(String::from("Note"));
    let menu = Menu::new(ui, "Actions", rows).unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("menu", menu),
                Element::new("note", Label::new(&note)),
            ]),
        ))
        .unwrap();
    settle(ui);
    let menu = find(ui, &root, "menu").unwrap();
    MenuFixture {
        trigger: part(ui, &menu, "trigger"),
        items: part(ui, &menu, "items"),
        root,
        note,
    }
}
impl MenuFixture {
    /// Opens the menu from its focused trigger with Enter.
    #[track_caller]
    fn open(&self, ui: &mut Runtime) {
        ui.focus(&self.trigger).unwrap();
        press(ui, KeyCode::Enter);
        assert!(shown(ui, &self.items), "the menu opens");
    }
    /// Where row `name` appears when the menu opens, read while it is open
    /// and before any pointer input; the menu is closed again afterwards.
    #[track_caller]
    fn where_row_opens(&self, ui: &mut Runtime, name: &str) -> (f32, f32) {
        self.open(ui);
        let at = row(ui, &self.items, name);
        press(ui, KeyCode::Escape);
        assert!(!shown(ui, &self.items), "Escape closes the menu");
        at
    }
    /// Changes the note's text, a layout change elsewhere in the window that
    /// makes the runtime repeat the pointer's position as a refreshed move.
    #[track_caller]
    fn relayout(&self, ui: &mut Runtime) {
        self.note.update(|text| text.push_str(" more")).unwrap();
        settle(ui);
    }
}

#[test]
fn a_pointer_parked_where_a_menu_opens_does_not_take_its_keyboard_choice() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(
        &mut ui,
        logged_rows(&log, &["Alpha", "Beta", "Gamma", "Delta"]),
    );
    let gamma = menu.where_row_opens(&mut ui, "Gamma");
    move_to(&mut ui, gamma);
    menu.open(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));
    // A host repeating the stationary position is no motion either.
    move_to(&mut ui, gamma);
    move_to(&mut ui, gamma);
    menu.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Beta"]);
}

#[test]
fn a_content_click_beneath_a_menu_does_not_choose_its_row() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, logged_rows(&log, &["Alpha", "Beta", "Gamma"]));
    let beta = menu.where_row_opens(&mut ui, "Beta");
    // A test driver's click: press and release, no move first.
    host(&mut ui, beta, PointerKind::Press(PointerButton::Primary));
    host(&mut ui, beta, PointerKind::Release(PointerButton::Primary));
    menu.open(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Alpha"]);
}

#[test]
fn keyboard_navigation_of_a_long_menu_never_stalls_under_a_parked_pointer() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, long_rows(&log));
    let parked = menu.where_row_opens(&mut ui, "R03");
    move_to(&mut ui, parked);
    menu.open(&mut ui);
    let names = long_names();
    for (step, expected) in names.iter().enumerate().take(13).skip(1) {
        press(&mut ui, KeyCode::ArrowDown);
        assert_eq!(
            highlight(&mut ui, &menu.items).as_ref(),
            Some(expected),
            "after {step} Down"
        );
    }
    assert!(
        semantics(&mut ui, &menu.items).items[0].bounds.is_none(),
        "the list scrolled under the stationary pointer"
    );
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["R12"]);

    // End, Page Up, Home and Page Down scroll as well.
    menu.open(&mut ui);
    for (code, expected) in [
        (KeyCode::End, "R29"),
        (KeyCode::PageUp, "R21"),
        (KeyCode::Home, "R00"),
        (KeyCode::PageDown, "R08"),
    ] {
        press(&mut ui, code);
        assert_eq!(
            highlight(&mut ui, &menu.items).as_deref(),
            Some(expected),
            "after {code:?}"
        );
    }
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["R12", "R08"]);
}

#[test]
fn actual_motion_takes_the_highlight_and_the_keyboard_takes_it_back() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(
        &mut ui,
        logged_rows(&log, &["Alpha", "Beta", "Gamma", "Delta", "Epsilon"]),
    );
    let gamma = menu.where_row_opens(&mut ui, "Gamma");
    move_to(&mut ui, gamma);
    menu.open(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));

    let delta = row(&mut ui, &menu.items, "Delta");
    move_to(&mut ui, delta);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Delta"));
    // The keyboard takes over; the same position reported again is no motion.
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    move_to(&mut ui, delta);
    menu.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    // A pixel of real motion within the row is motion.
    move_to(&mut ui, (delta.0 + 1.0, delta.1));
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Delta"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Delta"]);
}

#[test]
fn reentering_the_window_at_the_same_position_is_stationary_until_it_moves() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(
        &mut ui,
        logged_rows(&log, &["Alpha", "Beta", "Gamma", "Delta"]),
    );
    menu.open(&mut ui);
    let beta = row(&mut ui, &menu.items, "Beta");
    move_to(&mut ui, beta);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));
    leave_window(&mut ui, beta);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    // The native adapter reports re-entry at the identical position.
    move_to(&mut ui, beta);
    menu.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    move_to(&mut ui, (beta.0, beta.1 + 1.0));
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));

    // Leaving for other content and coming back to the exact pixel where the
    // pointer was on the list is motion: the list saw the pointer leave.
    press(&mut ui, KeyCode::ArrowDown);
    let note = ui
        .bounds(&find(&ui, &menu.root, "note").unwrap())
        .unwrap()
        .unwrap()
        .values();
    let outside = (note[0] + 300.0, note[1] + note[3] + 100.0);
    assert_eq!(row_under(&mut ui, &menu.items, outside), None);
    move_to(&mut ui, outside);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    move_to(&mut ui, (beta.0, beta.1 + 1.0));
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Beta"]);
}

#[test]
fn only_a_wheel_that_scrolls_hands_the_highlight_to_the_pointer() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, long_rows(&log));
    let parked = menu.where_row_opens(&mut ui, "R03");
    move_to(&mut ui, parked);
    menu.open(&mut ui);
    // Upward at the top: nothing scrolls, and a later refresh from unrelated
    // layout leaves the keyboard's row.
    wheel(&mut ui, parked, -64.0);
    menu.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("R00"));
    // A scroll does: the refresh after it highlights the row now under the
    // stationary pointer, and later layout keeps following it.
    wheel(&mut ui, parked, 64.0);
    let under = row_under(&mut ui, &menu.items, parked).unwrap();
    assert_ne!(under, "R03", "the rows moved under the pointer");
    assert_eq!(highlight(&mut ui, &menu.items), Some(under.clone()));
    menu.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items), Some(under.clone()));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), [under]);
}

#[test]
fn a_menu_the_pointer_drives_follows_changed_rows_and_a_keyboard_one_keeps_its_row() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(
        &mut ui,
        logged_rows(&log, &["Alpha", "Beta", "Gamma", "Delta"]),
    );
    let redescribe = |ui: &mut Runtime, names: &[&str]| {
        let next = Menu::new(ui, "Actions", logged_rows(&log, names)).unwrap();
        let note = Element::new("note", Label::new(&menu.note));
        ui.reconcile_children(&menu.root, vec![Element::new("menu", next), note])
            .unwrap();
        settle(ui);
    };
    menu.open(&mut ui);
    let gamma = row(&mut ui, &menu.items, "Gamma");
    move_to(&mut ui, gamma);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    // A row added above moves the rows under the pointer, which still owns
    // the highlight.
    redescribe(&mut ui, &["New", "Alpha", "Beta", "Gamma", "Delta"]);
    assert_eq!(
        row_under(&mut ui, &menu.items, gamma).as_deref(),
        Some("Beta")
    );
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));

    // After the keyboard takes over, the highlight stays with its row.
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    redescribe(
        &mut ui,
        &["Newer", "New", "Alpha", "Beta", "Gamma", "Delta"],
    );
    assert_eq!(
        row_under(&mut ui, &menu.items, gamma).as_deref(),
        Some("Alpha")
    );
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Gamma"]);
}

#[test]
fn reopening_a_menu_under_a_stationary_pointer_starts_from_its_first_row() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, logged_rows(&log, &["Alpha", "Beta", "Gamma"]));
    menu.open(&mut ui);
    let gamma = row(&mut ui, &menu.items, "Gamma");
    move_to(&mut ui, gamma);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    press(&mut ui, KeyCode::Escape);
    menu.open(&mut ui);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));
    move_to(&mut ui, gamma);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Alpha"]);
}

#[test]
fn a_press_highlights_its_row_without_a_move_and_a_release_elsewhere_runs_nothing() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, logged_rows(&log, &["Alpha", "Beta", "Gamma"]));
    menu.open(&mut ui);
    let (beta, gamma) = (
        row(&mut ui, &menu.items, "Beta"),
        row(&mut ui, &menu.items, "Gamma"),
    );
    host(&mut ui, beta, PointerKind::Press(PointerButton::Primary));
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Beta"));
    move_to(&mut ui, gamma);
    host(&mut ui, gamma, PointerKind::Release(PointerButton::Primary));
    assert!(ran(&log).is_empty(), "the release matches no pressed row");
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    // A matched press and release runs its row.
    host(&mut ui, beta, PointerKind::Press(PointerButton::Primary));
    host(&mut ui, beta, PointerKind::Release(PointerButton::Primary));
    assert_eq!(ran(&log), ["Beta"]);
}

/// Every kind of pointer input between an activation key's press and release
/// only hovers: the release runs the row the keyboard chose.
#[test]
fn pointer_input_cannot_move_a_choice_armed_by_enter_or_space() {
    for code in [KeyCode::Enter, KeyCode::Space] {
        let mut ui = ui();
        let log = Log::default();
        let menu = mount_menu(&mut ui, long_rows(&log));
        menu.open(&mut ui);
        press(&mut ui, KeyCode::ArrowDown);
        key_edge(&mut ui, code, KeyAction::Press);
        let r04 = row(&mut ui, &menu.items, "R04");
        let r05 = row(&mut ui, &menu.items, "R05");
        move_to(&mut ui, r04);
        assert_eq!(
            highlight(&mut ui, &menu.items).as_deref(),
            Some("R01"),
            "{code:?} move"
        );
        host(&mut ui, r05, PointerKind::Press(PointerButton::Primary));
        assert_eq!(
            highlight(&mut ui, &menu.items).as_deref(),
            Some("R01"),
            "{code:?} press"
        );
        host(&mut ui, r05, PointerKind::Release(PointerButton::Primary));
        wheel(&mut ui, r05, 64.0);
        assert!(
            semantics(&mut ui, &menu.items).items[0].bounds.is_none(),
            "the wheel still scrolls"
        );
        menu.relayout(&mut ui);
        move_to(&mut ui, r04);
        assert_eq!(
            highlight(&mut ui, &menu.items).as_deref(),
            Some("R01"),
            "{code:?} wheel"
        );
        assert!(ran(&log).is_empty());
        key_edge(&mut ui, code, KeyAction::Release);
        assert_eq!(ran(&log), ["R01"], "{code:?} release");
    }
}

#[test]
fn escape_and_release_end_an_armed_key_and_motion_takes_the_highlight_again() {
    let mut ui = ui();
    let log = Log::default();
    let mut rows = logged_rows(&log, &["Alpha", "Beta", "Gamma", "Delta"]);
    rows.insert(
        1,
        MenuItem::submenu("more", "More", logged_rows(&log, &["Inner"])),
    );
    let menu = mount_menu(&mut ui, rows);
    menu.open(&mut ui);
    let (gamma, delta) = (
        row(&mut ui, &menu.items, "Gamma"),
        row(&mut ui, &menu.items, "Delta"),
    );

    // Escape dismisses the menu, which cancels the armed Enter: its release
    // runs nothing. The reopened menu starts from its first row and the next
    // motion takes the highlight.
    press(&mut ui, KeyCode::ArrowDown);
    key_edge(&mut ui, KeyCode::Enter, KeyAction::Press);
    move_to(&mut ui, delta);
    press(&mut ui, KeyCode::Escape);
    assert!(!shown(&ui, &menu.items), "Escape closes the menu");
    key_edge(&mut ui, KeyCode::Enter, KeyAction::Release);
    assert_eq!(ran(&log), Vec::<String>::new());
    menu.open(&mut ui);
    move_to(&mut ui, delta);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Alpha"));
    move_to(&mut ui, gamma);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));

    // The release of an armed Enter opens the keyboard's branch, after which
    // motion over the parent list takes its highlight again.
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("More"));
    key_edge(&mut ui, KeyCode::Enter, KeyAction::Press);
    move_to(&mut ui, delta);
    key_edge(&mut ui, KeyCode::Enter, KeyAction::Release);
    assert_eq!(expanded(&mut ui, &menu.items, "More"), Some(true));
    move_to(&mut ui, gamma);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
}

/// Two submenus under the parent's rows; the pointer rests over the second.
fn branches(log: &Log) -> Vec<MenuItem> {
    vec![
        logged(log, "Alpha"),
        MenuItem::submenu("first", "First", logged_rows(log, &["One", "Two"])),
        logged(log, "Gamma"),
        MenuItem::submenu("second", "Second", logged_rows(log, &["Three", "Four"])),
    ]
}

#[test]
fn right_and_left_keep_the_keyboard_branch_under_a_parked_parent_pointer() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, branches(&log));
    let second = menu.where_row_opens(&mut ui, "Second");
    move_to(&mut ui, second);
    menu.open(&mut ui);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("First"));
    for _ in 0..2 {
        press(&mut ui, KeyCode::ArrowRight);
        assert_eq!(expanded(&mut ui, &menu.items, "First"), Some(true));
        assert_eq!(expanded(&mut ui, &menu.items, "Second"), Some(false));
        move_to(&mut ui, second);
        press(&mut ui, KeyCode::ArrowLeft);
        assert_eq!(expanded(&mut ui, &menu.items, "First"), Some(false));
        assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("First"));
    }
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Two"]);
}

#[test]
fn a_branch_clicked_without_a_move_is_the_row_right_and_left_return_to() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, branches(&log));
    menu.open(&mut ui);
    Driver::new(&mut ui).click_item(&menu.items, 3).unwrap();
    assert_eq!(expanded(&mut ui, &menu.items, "Second"), Some(true));
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Second"));
    // Right enters the open submenu, Left returns to its row, and Right opens
    // the same branch again.
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Second"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(expanded(&mut ui, &menu.items, "Second"), Some(true));
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Four"]);
}

#[test]
fn a_context_menu_opened_by_key_ignores_a_parked_pointer() {
    let mut ui = ui();
    let log = Log::default();
    let context = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        logged_rows(&log, &["Cut", "Copy", "Paste", "Delete"]),
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("context", context)]),
        ))
        .unwrap();
    settle(&mut ui);
    let context = find(&ui, &root, "context").unwrap();
    let target = find(&ui, &root, "target").unwrap();
    let items = part(&ui, &context, "items");
    ui.focus(&target).unwrap();
    press(&mut ui, KeyCode::ContextMenu);
    let paste = row(&mut ui, &items, "Paste");
    press(&mut ui, KeyCode::Escape);
    move_to(&mut ui, paste);
    ui.focus(&target).unwrap();
    press(&mut ui, KeyCode::ContextMenu);
    assert_eq!(highlight(&mut ui, &items).as_deref(), Some("Cut"));
    press(&mut ui, KeyCode::ArrowDown);
    move_to(&mut ui, paste);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Copy"]);
}

#[test]
fn a_context_menu_opened_at_the_pointer_scrolls_by_key_beneath_it() {
    let mut ui = ui();
    let log = Log::default();
    let context = ContextMenu::new(
        &ui,
        Element::new("target", Label::new("Target")),
        long_rows(&log),
    )
    .unwrap();
    let root = ui.mount_tree(Element::new("context", context)).unwrap();
    settle(&mut ui);
    let items = part(&ui, &root, "items");
    let at = (40.0, 40.0);
    host(&mut ui, at, PointerKind::Press(PointerButton::Secondary));
    host(&mut ui, at, PointerKind::Release(PointerButton::Secondary));
    assert_eq!(highlight(&mut ui, &items).as_deref(), Some("R00"));
    // The menu opens beneath the pointer; move onto its third row.
    let r02 = row(&mut ui, &items, "R02");
    move_to(&mut ui, r02);
    assert_eq!(highlight(&mut ui, &items).as_deref(), Some("R02"));
    for _ in 0..12 {
        press(&mut ui, KeyCode::ArrowDown);
    }
    assert_eq!(highlight(&mut ui, &items).as_deref(), Some("R14"));
    move_to(&mut ui, r02);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["R14"]);
}

/// A dropdown, with a note below whose text a test changes to cause layout.
struct SelectFixture {
    root: WidgetId,
    control: WidgetId,
    options: WidgetId,
    note: Signal<String>,
}
fn options(count: u8) -> Vec<Choice<u8>> {
    (0..count)
        .map(|value| Choice::new(value, format!("O{value:02}")))
        .collect()
}
fn mount_select(ui: &mut Runtime, select: Element) -> SelectFixture {
    let note = ui.signal(String::from("Note"));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([select, Element::new("note", Label::new(&note))]),
        ))
        .unwrap();
    settle(ui);
    let select = find(ui, &root, "mode").unwrap();
    SelectFixture {
        control: part(ui, &select, "control"),
        options: part(ui, &select, "options"),
        root,
        note,
    }
}
impl SelectFixture {
    #[track_caller]
    fn open(&self, ui: &mut Runtime) {
        ui.focus(&self.control).unwrap();
        press(ui, KeyCode::ArrowDown);
        assert!(shown(ui, &self.options), "the list opens");
    }
    #[track_caller]
    fn where_row_opens(&self, ui: &mut Runtime, name: &str) -> (f32, f32) {
        self.open(ui);
        let at = row(ui, &self.options, name);
        press(ui, KeyCode::Escape);
        assert!(!shown(ui, &self.options), "Escape closes the list");
        at
    }
    #[track_caller]
    fn relayout(&self, ui: &mut Runtime) {
        self.note.update(|text| text.push_str(" more")).unwrap();
        settle(ui);
    }
}

#[test]
fn select_and_inline_select_lists_ignore_a_parked_pointer() {
    for inline in [false, true] {
        let mut ui = ui();
        let value = ui.signal(1_u8);
        let element = if inline {
            Element::new(
                "mode",
                InlineSelect::new(&ui, value.clone(), options(6))
                    .unwrap()
                    .label("Mode"),
            )
        } else {
            Element::new("mode", Select::new(&ui, value.clone(), options(6)).unwrap())
        };
        let select = mount_select(&mut ui, element);
        let parked = select.where_row_opens(&mut ui, "O04");
        move_to(&mut ui, parked);
        select.open(&mut ui);
        assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O01"));
        press(&mut ui, KeyCode::ArrowDown);
        move_to(&mut ui, parked);
        select.relayout(&mut ui);
        assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O02"));
        press(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), 2, "inline: {inline}");
    }
}

#[test]
fn long_select_and_inline_select_lists_scroll_by_key_under_a_parked_pointer() {
    for inline in [false, true] {
        let mut ui = ui();
        let value = ui.signal(0_u8);
        let element = if inline {
            Element::new(
                "mode",
                InlineSelect::new(&ui, value.clone(), options(30)).unwrap(),
            )
        } else {
            Element::new(
                "mode",
                Select::new(&ui, value.clone(), options(30)).unwrap(),
            )
        };
        let select = mount_select(&mut ui, element);
        let parked = select.where_row_opens(&mut ui, "O03");
        move_to(&mut ui, parked);
        select.open(&mut ui);
        for _ in 0..12 {
            press(&mut ui, KeyCode::ArrowDown);
        }
        assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O12"));
        press(&mut ui, KeyCode::Enter);
        assert_eq!(value.get().unwrap(), 12, "inline: {inline}");
    }
}

#[test]
fn a_redescribed_select_returns_its_highlight_to_the_selection() {
    let mut ui = ui();
    let value = ui.signal(1_u8);
    let element = Element::new("mode", Select::new(&ui, value.clone(), options(5)).unwrap());
    let select = mount_select(&mut ui, element);
    select.open(&mut ui);
    let o03 = row(&mut ui, &select.options, "O03");
    move_to(&mut ui, o03);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
    // New options are a new description: the highlight starts again from the
    // selected value, and the stationary pointer does not take it back.
    let next = Select::new(&ui, value.clone(), options(6)).unwrap();
    let note = Element::new("note", Label::new(&select.note));
    ui.reconcile_children(&select.root, vec![Element::new("mode", next), note])
        .unwrap();
    settle(&mut ui);
    assert!(shown(&ui, &select.options));
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O01"));
    move_to(&mut ui, o03);
    select.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O01"));
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 2);
}

#[test]
fn a_bound_select_the_pointer_drives_follows_changed_options() {
    let mut ui = ui();
    let value = ui.signal(1_u8);
    let source = ui.signal(options(5));
    let element = Element::new(
        "mode",
        Select::bound(&ui, value.clone(), source.read_only()),
    );
    let select = mount_select(&mut ui, element);
    select.open(&mut ui);
    let o03 = row(&mut ui, &select.options, "O03");
    move_to(&mut ui, o03);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
    source
        .update(|options| options.insert(0, Choice::new(9, "O09")))
        .unwrap();
    settle(&mut ui);
    assert_eq!(
        row_under(&mut ui, &select.options, o03).as_deref(),
        Some("O02")
    );
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O02"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 2);
}

#[test]
fn a_revoked_and_restored_select_compares_motion_with_positions_seen_while_revoked() {
    let mut ui = ui();
    let value = ui.signal(1_u8);
    let allowed = ui.signal(true);
    let element = Element::new(
        "mode",
        Select::new(&ui, value.clone(), options(7))
            .unwrap()
            .enabled(&allowed),
    );
    let select = mount_select(&mut ui, element);
    select.open(&mut ui);
    let (o03, o05) = (
        row(&mut ui, &select.options, "O03"),
        row(&mut ui, &select.options, "O05"),
    );
    move_to(&mut ui, o03);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
    allowed.set(false).unwrap();
    settle(&mut ui);
    assert!(shown(&ui, &select.options), "a revoked list stays open");
    move_to(&mut ui, o05);
    allowed.set(true).unwrap();
    settle(&mut ui);
    // The native adapter repeats the position the list saw while revoked.
    move_to(&mut ui, o05);
    select.relayout(&mut ui);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
    ui.focus(&select.options).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(value.get().unwrap(), 4);
}

/// A native-like sequence: motion bursts coalesced to one position, repeated
/// reports of it, a window leave and re-entry and fractional positions.
#[test]
fn a_native_like_coordinate_sequence_keeps_the_keyboard_choice_until_real_motion() {
    let mut ui = ui();
    let value = ui.signal(0_u8);
    let element = Element::new(
        "mode",
        Select::new(&ui, value.clone(), options(30)).unwrap(),
    );
    let select = mount_select(&mut ui, element);
    let parked = select.where_row_opens(&mut ui, "O02");
    let parked = (parked.0 + 0.25, parked.1 + 0.5);
    move_to(&mut ui, (parked.0 - 7.0, parked.1 - 3.0));
    move_to(&mut ui, parked);
    select.open(&mut ui);
    move_to(&mut ui, parked);
    for _ in 0..10 {
        press(&mut ui, KeyCode::ArrowDown);
        move_to(&mut ui, parked);
    }
    leave_window(&mut ui, parked);
    move_to(&mut ui, parked);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O11"));
    let moved = (parked.0, parked.1 + 0.5);
    move_to(&mut ui, moved);
    let under = row_under(&mut ui, &select.options, moved);
    assert!(under.is_some());
    assert_eq!(highlight(&mut ui, &select.options), under);
    press(&mut ui, KeyCode::Enter);
    let chosen = format!("O{:02}", value.get().unwrap());
    assert_eq!(Some(chosen), under);
}

/// The P12 menu bar: File (Save, Beta) and View (Zoom) above content.
struct BarFixture {
    bar: WidgetId,
    file: WidgetId,
    view: WidgetId,
    content: (f32, f32),
}
fn mount_bar(ui: &mut Runtime, log: &Log) -> BarFixture {
    let bar = MenuBar::new(
        ui,
        vec![
            MenuItem::submenu("file", "File", logged_rows(log, &["Save", "Beta"])),
            MenuItem::submenu("view", "View", logged_rows(log, &["Zoom"])),
        ],
    )
    .unwrap()
    .content(Element::new(
        "content",
        Column::new([Element::new("button", Button::new("Content", |_| Ok(())))]),
    ));
    let bar = ui.mount_tree(Element::new("bar", bar)).unwrap();
    settle(ui);
    let file = part(ui, &bar, "title/file");
    let view = part(ui, &bar, "title/view");
    let [x, y, w, h] = ui.bounds(&bar).unwrap().unwrap().values();
    BarFixture {
        content: (x + w - 20.0, y + h - 20.0),
        bar,
        file,
        view,
    }
}
impl BarFixture {
    fn middle(&self, ui: &Runtime, title: &WidgetId) -> (f32, f32) {
        let [x, y, w, h] = ui.bounds(title).unwrap().unwrap().values();
        (x + w / 2.0, y + h / 2.0)
    }
    /// Which titles show their menus: (File, View).
    fn open(&self, ui: &mut Runtime) -> (bool, bool) {
        let open = |ui: &mut Runtime, title: &WidgetId| semantics(ui, title).expanded == Some(true);
        (open(ui, &self.file), open(ui, &self.view))
    }
    /// F10, then Down: the keyboard opens File.
    #[track_caller]
    fn open_file_by_key(&self, ui: &mut Runtime) {
        press(ui, KeyCode::F10);
        press(ui, KeyCode::ArrowDown);
    }
}

#[test]
fn the_p12_pointer_positions_leave_a_keyboard_opened_file_menu_open() {
    type Setup = fn(&mut Runtime, &BarFixture);
    let cases: [(&str, Setup); 5] = [
        ("never reported", |_, _| {}),
        ("parked over View", |ui, bar| {
            let view = bar.middle(ui, &bar.view);
            move_to(ui, view);
        }),
        ("the same move twice over View", |ui, bar| {
            let view = bar.middle(ui, &bar.view);
            move_to(ui, view);
            move_to(ui, view);
        }),
        ("a move over View, then leaving the window", |ui, bar| {
            let view = bar.middle(ui, &bar.view);
            move_to(ui, view);
            leave_window(ui, view);
        }),
        ("parked over File", |ui, bar| {
            let file = bar.middle(ui, &bar.file);
            move_to(ui, file);
        }),
    ];
    for (case, setup) in cases {
        let mut ui = ui();
        let log = Log::default();
        let bar = mount_bar(&mut ui, &log);
        setup(&mut ui, &bar);
        bar.open_file_by_key(&mut ui);
        assert_eq!(bar.open(&mut ui), (true, false), "{case}");
        press(&mut ui, KeyCode::Enter);
        assert_eq!(ran(&log), ["Save"], "{case}");
    }
}

#[test]
fn a_repeated_or_reentering_title_position_after_keyboard_opening_switches_nothing() {
    let mut ui = ui();
    let log = Log::default();
    let bar = mount_bar(&mut ui, &log);
    let view = bar.middle(&ui, &bar.view);
    move_to(&mut ui, view);
    bar.open_file_by_key(&mut ui);
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (true, false));
    // Re-entering the window at the identical position is stationary.
    leave_window(&mut ui, view);
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (true, false));
    // The next distinct motion over View switches to its menu.
    move_to(&mut ui, (view.0 + 1.0, view.1));
    assert_eq!(bar.open(&mut ui), (false, true));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Zoom"]);
}

#[test]
fn actual_motion_to_another_title_switches_menus_and_a_repeat_does_not() {
    let mut ui = ui();
    let log = Log::default();
    let bar = mount_bar(&mut ui, &log);
    let (file, view) = (bar.middle(&ui, &bar.file), bar.middle(&ui, &bar.view));
    bar.open_file_by_key(&mut ui);
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (false, true));
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (false, true));
    // Title to title and back: each title saw the pointer leave it for the
    // other, so returning to the exact pixel is motion.
    move_to(&mut ui, file);
    assert_eq!(bar.open(&mut ui), (true, false));
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (false, true));
    move_to(&mut ui, file);
    assert_eq!(bar.open(&mut ui), (true, false));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Save"]);
}

#[test]
fn returning_from_content_to_a_title_position_seen_before_switches_menus() {
    let mut ui = ui();
    let log = Log::default();
    let bar = mount_bar(&mut ui, &log);
    let view = bar.middle(&ui, &bar.view);
    // Hover View with the bar closed, then go down into the content.
    move_to(&mut ui, view);
    move_to(&mut ui, bar.content);
    bar.open_file_by_key(&mut ui);
    assert_eq!(bar.open(&mut ui), (true, false));
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (false, true));
    // A pointer-opened File, then content, then the exact View pixel again.
    let file = bar.middle(&ui, &bar.file);
    move_to(&mut ui, file);
    assert_eq!(bar.open(&mut ui), (true, false));
    move_to(&mut ui, bar.content);
    move_to(&mut ui, view);
    assert_eq!(bar.open(&mut ui), (false, true));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Zoom"]);
    assert!(find(&ui, &bar.bar, "button").is_some());
}

#[test]
fn keyboard_left_and_right_between_menus_survive_a_repeated_title_position() {
    let mut ui = ui();
    let log = Log::default();
    let bar = mount_bar(&mut ui, &log);
    let file = bar.middle(&ui, &bar.file);
    move_to(&mut ui, file);
    bar.open_file_by_key(&mut ui);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(bar.open(&mut ui), (false, true));
    move_to(&mut ui, file);
    assert_eq!(bar.open(&mut ui), (false, true));
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(bar.open(&mut ui), (true, false));
    press(&mut ui, KeyCode::ArrowRight);
    move_to(&mut ui, file);
    assert_eq!(bar.open(&mut ui), (false, true));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Zoom"]);
    // A pointer press on a title still opens its menu.
    host(&mut ui, file, PointerKind::Press(PointerButton::Primary));
    host(&mut ui, file, PointerKind::Release(PointerButton::Primary));
    assert_eq!(bar.open(&mut ui), (true, false));
}

#[test]
fn right_on_a_row_without_a_submenu_takes_the_highlight_from_the_pointer() {
    let mut ui = ui();
    let log = Log::default();
    let menu = mount_menu(&mut ui, logged_rows(&log, &["Alpha", "Beta", "Gamma"]));
    menu.open(&mut ui);
    let gamma = row(&mut ui, &menu.items, "Gamma");
    move_to(&mut ui, gamma);
    // Right acts on the highlight even where it opens nothing, so the row
    // stays the keyboard's when the rows then move under the pointer.
    press(&mut ui, KeyCode::ArrowRight);
    assert!(shown(&ui, &menu.items));
    let next = Menu::new(
        &ui,
        "Actions",
        logged_rows(&log, &["New", "Alpha", "Beta", "Gamma"]),
    )
    .unwrap();
    let note = Element::new("note", Label::new(&menu.note));
    ui.reconcile_children(&menu.root, vec![Element::new("menu", next), note])
        .unwrap();
    settle(&mut ui);
    assert_eq!(
        row_under(&mut ui, &menu.items, gamma).as_deref(),
        Some("Beta")
    );
    assert_eq!(highlight(&mut ui, &menu.items).as_deref(), Some("Gamma"));
    press(&mut ui, KeyCode::Enter);
    assert_eq!(ran(&log), ["Gamma"]);
}

#[test]
fn a_select_disabled_and_enabled_again_by_redescription_keeps_its_row() {
    let mut ui = ui();
    let value = ui.signal(1_u8);
    let source = ui.signal(options(6));
    let describe = |ui: &Runtime, enabled: bool| {
        let select = Select::bound(ui, value.clone(), source.read_only()).enabled(enabled);
        Element::new("mode", select)
    };
    let element = describe(&ui, true);
    let select = mount_select(&mut ui, element);
    let redescribe = |ui: &mut Runtime, enabled: bool| {
        let note = Element::new("note", Label::new(&select.note));
        let next = describe(ui, enabled);
        ui.reconcile_children(&select.root, vec![next, note])
            .unwrap();
        settle(ui);
    };
    select.open(&mut ui);
    let o03 = row(&mut ui, &select.options, "O03");
    move_to(&mut ui, o03);
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
    // Disabled while open, its rows move under the stationary pointer.
    redescribe(&mut ui, false);
    assert!(shown(&ui, &select.options), "a disabled list stays open");
    source
        .update(|options| options.insert(0, Choice::new(9, "O09")))
        .unwrap();
    settle(&mut ui);
    redescribe(&mut ui, true);
    select.relayout(&mut ui);
    assert_eq!(
        row_under(&mut ui, &select.options, o03).as_deref(),
        Some("O02")
    );
    assert_eq!(highlight(&mut ui, &select.options).as_deref(), Some("O03"));
}
