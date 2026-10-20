//! `MenuItem::on_change` on toggle and radio rows reports successful user
//! writes only, under the user-write callback contract.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{find, press};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn point(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
fn click(ui: &mut Runtime, x: f32, y: f32, button: PointerButton) {
    point(ui, x, y, PointerKind::Move);
    point(ui, x, y, PointerKind::Press(button));
    point(ui, x, y, PointerKind::Release(button));
}
fn center(ui: &Runtime, id: &WidgetId) -> (f32, f32) {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    (x + w / 2.0, y + h / 2.0)
}

type Log = Rc<RefCell<Vec<(&'static str, bool)>>>;

/// Whether a menu list is shown.
fn list_open(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == "items" && component.visible)
}

fn logging(
    log: &Log,
    name: &'static str,
) -> impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static {
    let log = log.clone();
    move |_, checked| {
        log.borrow_mut().push((name, checked));
        Ok(())
    }
}

fn mount_menu(ui: &mut Runtime, menu: Menu) -> WidgetId {
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("menu", menu)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    root
}
/// Opens the menu from its trigger and chooses the row `steps` below the first.
fn choose(ui: &mut Runtime, root: &WidgetId, steps: usize) {
    let trigger = find(ui, root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(ui, KeyCode::Enter);
    for _ in 0..steps {
        press(ui, KeyCode::ArrowDown);
    }
    press(ui, KeyCode::Enter);
}

#[test]
fn user_choices_report_the_new_checked_state_once() {
    let mut ui = ui();
    let grid = ui.signal(false);
    let zoom = ui.signal(100u32);
    let log = Log::default();
    let menu = Menu::new(
        &ui,
        "View",
        vec![
            MenuItem::toggle("grid", "Show grid", grid.clone()).on_change(logging(&log, "grid")),
            MenuItem::radio("small", "100%", zoom.clone(), 100).on_change(logging(&log, "small")),
            MenuItem::radio("large", "200%", zoom.clone(), 200).on_change(logging(&log, "large")),
        ],
    )
    .unwrap();
    let root = mount_menu(&mut ui, menu);
    choose(&mut ui, &root, 0);
    assert!(grid.get().unwrap());
    assert_eq!(*log.borrow(), [("grid", true)]);
    choose(&mut ui, &root, 0);
    assert_eq!(log.borrow().last(), Some(&("grid", false)));
    // A radio row reports `true` when it becomes selected; the deselected
    // sibling reports nothing.
    choose(&mut ui, &root, 2);
    assert_eq!(zoom.get().unwrap(), 200);
    assert_eq!(log.borrow().last(), Some(&("large", true)));
    let before = log.borrow().len();
    // Choosing the selected radio row again writes and reports nothing.
    choose(&mut ui, &root, 2);
    assert_eq!(log.borrow().len(), before);
    // Application writes report nothing.
    grid.set(true).unwrap();
    zoom.set(100).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(log.borrow().len(), before);
}

#[test]
fn a_failing_handler_keeps_the_write_and_is_not_replayed() {
    let mut ui = ui();
    let grid = ui.signal(false);
    let calls = Rc::new(RefCell::new(0));
    let count = calls.clone();
    let menu = Menu::new(
        &ui,
        "View",
        vec![
            MenuItem::toggle("grid", "Show grid", grid.clone()).on_change(move |_, _| {
                *count.borrow_mut() += 1;
                Err(ApplicationError::from("the setting could not be applied").into())
            }),
        ],
    )
    .unwrap();
    let root = mount_menu(&mut ui, menu);
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    // A list chooses its row when the key is released.
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    let error = ui
        .key_input(KeyCode::Enter, KeyAction::Release)
        .unwrap_err();
    assert_eq!(error.to_string(), "the setting could not be applied");
    ui.flush(Duration::ZERO).unwrap();
    // The closure requested before the handler ran stands.
    assert!(!list_open(&ui));
    assert!(grid.get().unwrap(), "the write stands");
    assert_eq!(*calls.borrow(), 1);
    ui.flush(Duration::from_millis(1)).unwrap();
    assert_eq!(*calls.borrow(), 1, "not replayed");
}

#[test]
fn a_redescription_replaces_and_removes_the_handler() {
    let mut ui = ui();
    let grid = ui.signal(false);
    let log = Log::default();
    let menu = |ui: &Runtime, name: Option<&'static str>| {
        let mut row = MenuItem::toggle("grid", "Show grid", grid.clone());
        if let Some(name) = name {
            row = row.on_change(logging(&log, name));
        }
        Element::new("menu", Menu::new(ui, "View", vec![row]).unwrap())
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([menu(&ui, Some("first"))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.reconcile_children(&root, vec![menu(&ui, Some("second"))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    choose(&mut ui, &root, 0);
    assert_eq!(*log.borrow(), [("second", true)]);
    ui.reconcile_children(&root, vec![menu(&ui, None)]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    choose(&mut ui, &root, 0);
    assert!(!grid.get().unwrap());
    assert_eq!(log.borrow().len(), 1);
    // A redescription while the menu is open applies to the open list.
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(list_open(&ui));
    ui.reconcile_children(&root, vec![menu(&ui, Some("third"))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(log.borrow().last(), Some(&("third", true)));
}

#[test]
fn rows_without_a_write_of_their_own_ignore_it_and_disabled_rows_write_nothing() {
    let mut ui = ui();
    let ran = ui.signal(0u32);
    let grid = ui.signal(false);
    let log = Log::default();
    let count = ran.clone();
    let menu = Menu::new(
        &ui,
        "View",
        vec![
            MenuItem::new("plain", "Plain", move |_| count.set(count.get()? + 1))
                .on_change(logging(&log, "plain")),
            MenuItem::toggle("grid", "Show grid", grid.clone())
                .on_change(logging(&log, "grid"))
                .disabled(),
        ],
    )
    .unwrap();
    let root = mount_menu(&mut ui, menu);
    choose(&mut ui, &root, 0);
    assert_eq!(ran.get().unwrap(), 1);
    // The disabled row cannot be reached or chosen.
    choose(&mut ui, &root, 1);
    assert!(!grid.get().unwrap());
    assert!(log.borrow().is_empty());
}

#[test]
fn context_menus_and_menu_bar_titles_report_too_and_detached_work_survives() {
    let mut ui = ui();
    let grid = ui.signal(false);
    let wrap = ui.signal(false);
    let ruler = ui.signal(false);
    let deferred = ui.signal(false);
    let log = Log::default();
    let done = deferred.clone();
    let context = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        vec![
            MenuItem::toggle("wrap", "Wrap", wrap.clone()).on_change(move |cx, _| {
                let done = done.clone();
                let _receipt = cx.defer_detached(move |_| done.set(true))?;
                Ok(())
            }),
        ],
    )
    .unwrap();
    let bar = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu(
                "view",
                "View",
                vec![
                    MenuItem::toggle("grid", "Show grid", grid.clone())
                        .on_change(logging(&log, "grid")),
                ],
            ),
            MenuItem::toggle("ruler", "Ruler", ruler.clone()).on_change(logging(&log, "ruler")),
        ],
    )
    .unwrap()
    .content(Element::new("context", context));
    let root = ui.mount_tree(Element::new("bar", bar)).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // A direct title runs its row.
    let title = ui.find_descendant(&root, "title/ruler").unwrap().unwrap();
    let (x, y) = center(&ui, &title);
    click(&mut ui, x, y, PointerButton::Primary);
    assert!(ruler.get().unwrap());
    assert_eq!(*log.borrow(), [("ruler", true)]);

    // A row of a title's menu, opened by pointer.
    let view = ui.find_descendant(&root, "title/view").unwrap().unwrap();
    let (x, y) = center(&ui, &view);
    click(&mut ui, x, y, PointerButton::Primary);
    // The opened menu highlights its first row; Enter chooses it.
    press(&mut ui, KeyCode::Enter);
    assert!(grid.get().unwrap());
    assert_eq!(log.borrow().last(), Some(&("grid", true)));

    // A context menu row; work deferred from the handler outlives the list.
    let target = ui.find_descendant(&root, "target").unwrap().unwrap();
    let (x, y) = center(&ui, &target);
    click(&mut ui, x, y, PointerButton::Secondary);
    press(&mut ui, KeyCode::Enter);
    assert!(wrap.get().unwrap());
    ui.flush(Duration::ZERO).unwrap();
    assert!(deferred.get().unwrap());
}

/// Clicks the middle of the open list furthest right, a submenu's list once
/// one opens; the lists here hold one row, so that chooses it.
fn click_open_row(ui: &mut Runtime) {
    let list = ui
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.key == "items" && c.visible)
        .filter_map(|c| c.details.box_model.border_box)
        .max_by(|a, b| a[0].total_cmp(&b[0]))
        .expect("an open list");
    let [x, y, w, h] = list;
    click(ui, x + w / 2.0, y + h / 2.0, PointerButton::Primary);
}

/// Clicks the shown component keyed `key` below `root`.
fn click_key(ui: &mut Runtime, root: &WidgetId, key: &str, button: PointerButton) {
    let id = ui
        .find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("no {key}"));
    let (x, y) = center(ui, &id);
    click(ui, x, y, button);
}

#[test]
fn pointer_choices_in_nested_and_context_menus_report() {
    let mut ui = ui();
    let nested = ui.signal(false);
    let wrap = ui.signal(false);
    let log = Log::default();
    let context = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        vec![MenuItem::toggle("wrap", "Wrap", wrap.clone()).on_change(logging(&log, "wrap"))],
    )
    .unwrap();
    let menu = Menu::new(
        &ui,
        "View",
        vec![MenuItem::submenu(
            "more",
            "More",
            vec![
                MenuItem::toggle("nested", "Nested", nested.clone())
                    .on_change(logging(&log, "nested")),
            ],
        )],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("menu", menu), Element::new("context", context)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    click_key(&mut ui, &root, "trigger", PointerButton::Primary);
    // The first click opens the submenu, the second chooses its row.
    click_open_row(&mut ui);
    click_open_row(&mut ui);
    assert!(nested.get().unwrap());
    assert_eq!(*log.borrow(), [("nested", true)]);
    click_key(&mut ui, &root, "target", PointerButton::Secondary);
    click_open_row(&mut ui);
    assert!(wrap.get().unwrap());
    assert_eq!(log.borrow().last(), Some(&("wrap", true)));
}

#[test]
fn a_direct_menu_bar_title_reports_when_chosen_by_keyboard() {
    let mut ui = ui();
    let ruler = ui.signal(false);
    let log = Log::default();
    let bar = MenuBar::new(
        &ui,
        vec![MenuItem::toggle("ruler", "Ruler", ruler.clone()).on_change(logging(&log, "ruler"))],
    )
    .unwrap()
    .content(Element::new("body", Label::new("Body")));
    let root = ui.mount_tree(Element::new("bar", bar)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let title = ui.find_descendant(&root, "title/ruler").unwrap().unwrap();
    ui.focus(&title).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(ruler.get().unwrap());
    assert_eq!(*log.borrow(), [("ruler", true)]);
}
