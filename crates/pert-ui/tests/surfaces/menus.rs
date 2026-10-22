//! Menu rows (separators, marks, shortcuts, command rows) and the menu bar,
//! driven through the public facade.
use crate::support;
use pert_ui::*;
use std::time::Duration;
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
fn click(ui: &mut Runtime, x: f32, y: f32) {
    point(ui, x, y, PointerKind::Move);
    point(ui, x, y, PointerKind::Press(PointerButton::Primary));
    point(ui, x, y, PointerKind::Release(PointerButton::Primary));
}
fn center(ui: &Runtime, id: &WidgetId) -> (f32, f32) {
    let [x, y, w, h] = ui.bounds(id).unwrap().unwrap().values();
    (x + w / 2.0, y + h / 2.0)
}
/// The semantics of the visible component with `key`.
fn semantics(ui: &Runtime, key: &str) -> Semantics {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key && component.visible)
        .and_then(|component| component.details.semantics)
        .unwrap_or_else(|| panic!("no visible semantics for {key}"))
}
fn names(semantics: &Semantics) -> Vec<&str> {
    semantics
        .items
        .iter()
        .map(|item| item.name.as_deref().unwrap_or(""))
        .collect()
}
fn focused_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}
fn ctrl(key: char) -> KeyChord {
    KeyChord::new(
        CommandKey::Character(key),
        Modifiers {
            control: true,
            ..Modifiers::default()
        },
    )
}
/// Mounts `menu` in a column and opens it from its trigger.
fn open_menu(ui: &mut Runtime, menu: Menu) -> WidgetId {
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("menu", menu)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let trigger = find(ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(ui, KeyCode::Enter);
    root
}

#[test]
fn separators_collapse_at_the_edges_and_leave_no_row_to_land_on() {
    let mut ui = ui();
    let chosen = ui.signal("");
    let (a, b) = (chosen.clone(), chosen.clone());
    let menu = Menu::new(
        &ui,
        "Actions",
        vec![
            MenuItem::separator(),
            MenuItem::new("a", "First", move |_| a.set("a")),
            MenuItem::separator(),
            MenuItem::separator(),
            MenuItem::new("b", "Second", move |_| b.set("b")),
            MenuItem::separator(),
        ],
    )
    .unwrap();
    let root = open_menu(&mut ui, menu);
    let items = semantics(&ui, "items");
    assert_eq!(names(&items), ["First", "Second"]);

    // Arrow navigation moves from one row to the next across the separator.
    press(&mut ui, KeyCode::ArrowDown);
    assert!(semantics(&ui, "items").items[1].selected);

    // Two rows and one separator band: a press in the band chooses nothing, a
    // press on the second row chooses it.
    let list = find(&ui, &root, "items").unwrap();
    let [x, y, w, h] = ui.bounds(&list).unwrap().unwrap().values();
    click(&mut ui, x + w / 2.0, y + h / 2.0);
    assert_eq!(chosen.get().unwrap(), "");
    click(&mut ui, x + w / 2.0, y + h - 2.0);
    assert_eq!(chosen.get().unwrap(), "b");
}

#[test]
fn a_menu_of_only_separators_has_a_disabled_trigger() {
    let mut ui = ui();
    let menu = Menu::new(&ui, "Empty", vec![MenuItem::separator()]).unwrap();
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("menu", menu)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(semantics(&ui, "trigger").disabled);
}

#[test]
fn toggle_and_radio_rows_write_their_setting_and_follow_it() {
    let mut ui = ui();
    let grid = ui.signal(false);
    let zoom = ui.signal(100u32);
    let menu = Menu::new(
        &ui,
        "View",
        vec![
            MenuItem::toggle("grid", grid.clone(), "Show grid"),
            MenuItem::separator(),
            MenuItem::radio("small", zoom.clone(), 100, "100%"),
            MenuItem::radio("large", zoom.clone(), 200, "200%"),
            MenuItem::new("plain", "Reset", |_| Ok(())),
        ],
    )
    .unwrap();
    open_menu(&mut ui, menu);
    let checked = |ui: &Runtime| {
        semantics(ui, "items")
            .items
            .iter()
            .map(|item| item.checked)
            .collect::<Vec<_>>()
    };
    assert_eq!(checked(&ui), [Some(false), Some(true), Some(false), None]);

    // Choosing the toggle closes the menu and flips the value.
    press(&mut ui, KeyCode::Enter);
    assert!(grid.get().unwrap());

    // Writes from elsewhere are reflected in the retained rows.
    zoom.set(200).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::Enter); // the trigger has focus again
    assert_eq!(checked(&ui), [Some(true), Some(false), Some(true), None]);

    // Choosing a radio row selects its value.
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(zoom.get().unwrap(), 100);
    ui.paint().unwrap();
}

#[test]
fn selected_rows_follow_a_selection_the_menu_does_not_own() {
    let mut ui = ui();
    // The selection lives in the application, as a theme setting would.
    let dark = ui.signal(true);
    let light = ui.signal(false);
    let rows = [("dark", "Dark", true), ("light", "Light", false)].map(|(key, label, is_dark)| {
        let (dark, light) = (dark.clone(), light.clone());
        let selected = if is_dark {
            dark.read_only()
        } else {
            light.read_only()
        };
        MenuItem::new(key, label, move |_| {
            dark.set(is_dark)?;
            light.set(!is_dark)?;
            Ok(())
        })
        .selected_signal(selected)
    });
    let menu = Menu::new(&ui, "Theme", rows.into()).unwrap();
    open_menu(&mut ui, menu);
    let checked = |ui: &Runtime| {
        semantics(ui, "items")
            .items
            .iter()
            .map(|item| item.checked)
            .collect::<Vec<_>>()
    };
    assert_eq!(checked(&ui), [Some(true), Some(false)]);

    // The row runs its own action, which moves the selection.
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert!(light.get().unwrap() && !dark.get().unwrap());
    press(&mut ui, KeyCode::Enter); // the trigger has focus again
    assert_eq!(checked(&ui), [Some(false), Some(true)]);

    // A change from elsewhere is followed without reopening.
    dark.set(true).unwrap();
    light.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(checked(&ui), [Some(true), Some(false)]);
    ui.paint().unwrap();
}

#[test]
fn shortcuts_are_presented_and_command_rows_follow_rebinding_and_enablement() {
    let mut ui = ui();
    let opened = ui.signal(0);
    let count = opened.clone();
    let open = ui
        .register_command(
            "Open…",
            CommandScope::Application,
            vec![ctrl('o')],
            move |_, _| count.set(count.get()? + 1),
        )
        .unwrap();
    let allowed = ui.signal(false);
    ui.command_enabled_signal(&open, allowed.read_only())
        .unwrap();
    let receipt = std::rc::Rc::new(std::cell::RefCell::new(None::<CommandReceipt>));
    let sink = receipt.clone();
    let menu = Menu::new(
        &ui,
        "File",
        vec![
            MenuItem::new("new", "New", |_| Ok(())).shortcut(ctrl('n')),
            MenuItem::command(&ui, "open", &open, move |_, issued| {
                *sink.borrow_mut() = Some(issued);
                Ok(())
            })
            .unwrap()
            .enabled(ui.signal(true).read_only()),
        ],
    )
    .unwrap();
    open_menu(&mut ui, menu);
    let items = semantics(&ui, "items");
    assert_eq!(names(&items), ["New", "Open…"]);
    assert_eq!(items.items[0].shortcut.as_deref(), Some("Ctrl+N"));
    assert_eq!(items.items[1].shortcut.as_deref(), Some("Ctrl+O"));
    assert!(items.items[1].disabled);

    // Rebinding and enabling the command update the open menu.
    ui.rebind_command(&open, vec![ctrl('o').shifted()]).unwrap();
    allowed.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let items = semantics(&ui, "items");
    assert_eq!(items.items[1].shortcut.as_deref(), Some("Ctrl+Shift+O"));
    assert!(!items.items[1].disabled);

    // Choosing the row runs the command once and hands over its receipt.
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(opened.get().unwrap(), 1);
    let status = receipt.borrow().as_ref().unwrap().status();
    assert_eq!(status, CommandStatus::Finished(Ok(CommandOutcome::Invoked)));
    assert_eq!(ui.command_name(&open).unwrap(), "Open…");
}

#[test]
fn a_command_row_needs_a_registered_command() {
    let mut ui = ui();
    let gone = ui
        .register_command("Gone", CommandScope::Application, vec![], |_, _| Ok(()))
        .unwrap();
    ui.unregister_command(&gone).unwrap();
    let error = MenuItem::command(&ui, "gone", &gone, |_, _| Ok(()))
        .err()
        .unwrap();
    assert_eq!(error, RuntimeError::StaleCommand);
    assert_eq!(
        ui.command_shortcuts(&gone).err(),
        Some(RuntimeError::StaleCommand)
    );
}

/// A bar over a document with one focusable control; returns the root.
struct Bar {
    root: WidgetId,
    new: Signal<u32>,
    about: Signal<u32>,
    focus: Signal<Option<Option<String>>>,
}
fn bar(ui: &mut Runtime) -> Bar {
    let new = ui.signal(0u32);
    let about = ui.signal(0u32);
    let focus = ui.signal(None);
    let (count_new, count_about) = (new.clone(), about.clone());
    let seen = focus.clone();
    let observe = ui
        .register_command(
            "Observe",
            CommandScope::Application,
            vec![],
            move |ui, context| {
                let key = context
                    .focused
                    .as_ref()
                    .map(|id| ui.key(id).map(str::to_owned))
                    .transpose()?;
                seen.set(Some(key))
            },
        )
        .unwrap();
    let menus = MenuBar::new(
        ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![
                    MenuItem::new("new", "New", move |_| count_new.set(count_new.get()? + 1)),
                    MenuItem::separator(),
                    MenuItem::command(ui, "observe", &observe, |_, _| Ok(())).unwrap(),
                ],
            ),
            MenuItem::separator(),
            MenuItem::submenu(
                "edit",
                "Edit",
                vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
            ),
            MenuItem::submenu(
                "empty",
                "Unavailable",
                vec![MenuItem::new("never", "Never", |_| Ok(())).disabled()],
            ),
            MenuItem::new("about", "About", move |_| {
                count_about.set(count_about.get()? + 1)
            }),
        ],
    )
    .unwrap()
    .content(Element::new(
        "document",
        Column::new([Element::new("control", Button::new("Control", |_| Ok(())))]),
    ));
    let root = ui.mount_tree(Element::new("bar", menus)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let control = find(ui, &root, "control").unwrap();
    ui.focus(&control).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    Bar {
        root,
        new,
        about,
        focus,
    }
}
fn expanded(ui: &Runtime, title: &str) -> Option<bool> {
    semantics(ui, &format!("title/{title}")).expanded
}

#[test]
fn a_menu_bar_describes_itself_and_its_titles() {
    let mut ui = ui();
    bar(&mut ui);
    assert_eq!(semantics(&ui, "bar").role, SemanticRole::MenuBar);
    let file = semantics(&ui, "title/file");
    assert_eq!(file.role, SemanticRole::Menu);
    assert_eq!(file.name.as_deref(), Some("File"));
    assert_eq!(file.actions, [SemanticAction::Expand]);
    assert!(semantics(&ui, "title/empty").disabled);
    let about = semantics(&ui, "title/about");
    assert_eq!(about.role, SemanticRole::Button);
    assert_eq!(about.actions, [SemanticAction::Activate]);
    ui.paint().unwrap();
}

#[test]
fn pointing_opens_switches_and_closes_bar_menus() {
    let mut ui = ui();
    let bar = bar(&mut ui);
    let file = find(&ui, &bar.root, "title/file").unwrap();
    let edit = find(&ui, &bar.root, "title/edit").unwrap();
    let empty = find(&ui, &bar.root, "title/empty").unwrap();
    let (fx, fy) = center(&ui, &file);
    let (ex, ey) = center(&ui, &edit);

    // Titles are not focusable at rest, so pressing one leaves focus in place
    // until the menu takes it.
    click(&mut ui, fx, fy);
    assert_eq!(expanded(&ui, "file"), Some(true));
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));

    // With a menu open, pointing at another title switches menus.
    point(&mut ui, ex, ey, PointerKind::Move);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(expanded(&ui, "edit"), Some(true));

    // A disabled title neither opens nor takes over.
    let (dx, dy) = center(&ui, &empty);
    point(&mut ui, dx, dy, PointerKind::Move);
    assert_eq!(expanded(&ui, "edit"), Some(true));

    // Pressing the open menu's title closes it; focus returns to the document.
    point(&mut ui, ex, ey, PointerKind::Move);
    point(&mut ui, ex, ey, PointerKind::Press(PointerButton::Primary));
    point(
        &mut ui,
        ex,
        ey,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(expanded(&ui, "edit"), Some(false));
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // A title without a menu runs its action.
    let about = find(&ui, &bar.root, "title/about").unwrap();
    let (ax, ay) = center(&ui, &about);
    click(&mut ui, ax, ay);
    assert_eq!(bar.about.get().unwrap(), 1);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
}

#[test]
fn f10_and_arrows_navigate_titles_and_leaving_returns_focus() {
    let mut ui = ui();
    let bar = bar(&mut ui);

    // Titles are not Tab stops at rest.
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/edit"));
    // The disabled title is skipped.
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/about"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/about"));
    press(&mut ui, KeyCode::Home);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::End);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/about"));

    // Escape on a title leaves the bar for the control that had focus.
    press(&mut ui, KeyCode::Escape);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // F10 toggles the bar.
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // Enter on an action title runs it and leaves the bar.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(bar.about.get().unwrap(), 1);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
}

#[test]
fn tab_in_menu_mode_leaves_the_titles_and_ends_menu_mode() {
    let mut ui = ui();
    let _bar = bar(&mut ui);
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    // Tab does not walk the titles: it goes to the next tab stop after them.
    press(&mut ui, KeyCode::Tab);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    // Menu mode ended, so the titles are not tab stops and F10 opens again.
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
}

#[test]
fn window_focus_loss_ends_menu_mode() {
    let mut ui = ui();
    let bar = bar(&mut ui);
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    ui.clear_focus();
    ui.flush(Duration::ZERO).unwrap();
    // Back in the window with nothing focused: the first F10 opens the bar
    // rather than leaving a stale menu mode.
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    let _ = bar;
}

#[test]
fn keyboard_menus_move_sideways_close_by_level_and_return_focus_after_a_command() {
    let mut ui = ui();
    let bar = bar(&mut ui);
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(expanded(&ui, "file"), Some(true));
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));

    // Right on a row without a submenu opens the next menu; Left goes back.
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(expanded(&ui, "edit"), Some(true));
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(expanded(&ui, "file"), Some(true));

    // Escape closes the menu and returns to its title; again leaves the bar.
    press(&mut ui, KeyCode::Escape);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // Choosing a row runs it once and returns focus to the document.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(bar.new.get().unwrap(), 1);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // F10 from inside an open menu leaves the bar as well.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::F10);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
}

#[test]
fn focus_moving_into_the_content_ends_keyboard_mode() {
    let mut ui = ui();
    let bar = bar(&mut ui);
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/edit"));
    let control = find(&ui, &bar.root, "control").unwrap();
    let (cx, cy) = center(&ui, &control);
    click(&mut ui, cx, cy);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    // The titles are no longer Tab stops.
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
}

#[test]
fn a_bar_command_acts_on_the_focus_from_before_the_bar_was_used() {
    let mut ui = ui();
    let bar = bar(&mut ui);

    // Opened by pointer: the menu takes focus, the command still sees the control.
    let file = find(&ui, &bar.root, "title/file").unwrap();
    let (fx, fy) = center(&ui, &file);
    click(&mut ui, fx, fy);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bar.focus.get().unwrap(), Some(Some("control".to_owned())));

    // Opened with F10: the titles and the menu take focus in turn.
    bar.focus.set(None).unwrap();
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(bar.focus.get().unwrap(), Some(Some("control".to_owned())));
}

#[test]
fn menu_bar_keys_are_validated_at_every_level() {
    let ui = ui();
    let duplicate = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu("file", "File", vec![]),
            MenuItem::submenu("file", "Again", vec![]),
        ],
    );
    assert_eq!(duplicate.err(), Some(ChoiceError::DuplicateValue));
    let nested = MenuBar::new(
        &ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![MenuItem::new(" ", "Blank", |_| Ok(()))],
        )],
    );
    assert_eq!(nested.err(), Some(ChoiceError::EmptyValue));
}
#[test]
fn a_bar_given_no_space_keeps_its_titles_measurable() {
    // A host may mount before its first viewport; titles are clipped rather than
    // collapsed, so no title reports an empty hit area.
    let mut ui = Runtime::new();
    let bar = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            ),
            MenuItem::new("about", "About", |_| Ok(())),
        ],
    )
    .unwrap()
    .content(Element::new("document", Label::new("Document")));
    ui.mount_tree(Element::new("bar", bar)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}
#[test]
fn f10_reaches_a_bar_while_nothing_is_focused() {
    // A host that has not focused anything yet, such as right after start.
    let mut ui = ui();
    let pressed = ui.signal(0u32);
    let count = pressed.clone();
    let menus = MenuBar::new(
        &ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![MenuItem::new("new", "New", |_| Ok(()))],
        )],
    )
    .unwrap()
    .content(Element::new(
        "control",
        Button::new("Control", move |_| count.set(count.get()? + 1)),
    ));
    // The bar need not be the root: the first visible bar in tree order is used.
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("bar", menus)]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.focused(), None);

    // Only components that opt in see unfocused keys: Enter does not reach the
    // unfocused button.
    press(&mut ui, KeyCode::Enter);
    assert_eq!(pressed.get().unwrap(), 0);

    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(expanded(&ui, "file"), Some(true));
    // Without an earlier focus to return to, leaving keeps it on the titles, and
    // Tab moves on into the content.
    press(&mut ui, KeyCode::Escape);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::Tab);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
}

#[test]
fn scrolling_a_long_menu_moves_its_open_submenu_with_its_row() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 220.0, 1.0).unwrap())
        .unwrap();
    let mut items = vec![
        MenuItem::new("first", "First", |_| Ok(())),
        MenuItem::submenu(
            "more",
            "More",
            vec![MenuItem::new("inner", "Inner", |_| Ok(()))],
        ),
    ];
    items.extend(
        (0..30).map(|index| {
            MenuItem::new(format!("item-{index}"), format!("Item {index}"), |_| Ok(()))
        }),
    );
    let menus = MenuBar::new(&ui, vec![MenuItem::submenu("file", "File", items)])
        .unwrap()
        .content(Element::new("control", Button::new("Control", |_| Ok(()))));
    ui.mount_tree(Element::new("bar", menus)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowRight);
    let lists = |ui: &Runtime| -> Vec<[f32; 4]> {
        ui.inspect()
            .components
            .iter()
            .filter(|item| item.key == "items" && item.visible)
            .filter_map(|item| item.details.box_model.border_box)
            .collect()
    };
    let before = lists(&ui);
    assert_eq!(before.len(), 2, "the menu and its open submenu");
    let [x, y, w, h] = before[0];
    point(
        &mut ui,
        x + w / 2.0,
        y + h / 2.0,
        PointerKind::Scroll(Point::new(0.0, 20.0).unwrap()),
    );
    let after = lists(&ui);
    assert_eq!(after.len(), 2);
    assert!(
        after[1][1] < before[1][1],
        "the submenu follows its row up: {:?} then {:?}",
        before[1],
        after[1]
    );
}

#[test]
fn a_reveal_requested_while_a_submenu_is_open_places_it_from_the_new_offset() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 320.0, 1.0).unwrap())
        .unwrap();
    let enabled = ui.signal(true);
    let row =
        |index: usize| MenuItem::new(format!("item-{index}"), format!("Item {index}"), |_| Ok(()));
    // Only the submenu row and the rows from 21 on can be active.
    let mut items: Vec<MenuItem> = (0..3).map(|index| row(index).disabled()).collect();
    items.push(
        MenuItem::submenu(
            "more",
            "More",
            vec![MenuItem::new("inner", "Inner", |_| Ok(()))],
        )
        .enabled(enabled.read_only()),
    );
    items.extend((4..21).map(|index| row(index).disabled()));
    items.extend((21..30).map(row));
    let menus = MenuBar::new(&ui, vec![MenuItem::submenu("file", "File", items)])
        .unwrap()
        .content(Element::new("control", Button::new("Control", |_| Ok(()))));
    ui.mount_tree(Element::new("bar", menus)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowRight);
    let lists = |ui: &Runtime| -> Vec<[f32; 4]> {
        ui.inspect()
            .components
            .iter()
            .filter(|item| item.key == "items" && item.visible)
            .filter_map(|item| item.details.box_model.border_box)
            .collect()
    };
    let before = lists(&ui);
    assert_eq!(before.len(), 2, "the menu and its open submenu");

    // The open row becomes unavailable: the list makes row 21 active and
    // scrolls down to it in the same pass that places the submenu, whose row
    // moves up out of view.
    enabled.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let after = lists(&ui);
    assert_eq!(after.len(), 2, "the submenu stays open");
    assert!(
        after[1][1] < before[1][1],
        "placed from the new offset: {:?} then {:?}",
        before[1],
        after[1]
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(lists(&ui), after, "and stable in the next pass");
}

/// A bar with the named menus in the given order over a document with one
/// control. `recent` holds a plain row counting its runs and a command row.
fn described_bar(
    ui: &Runtime,
    order: &[&str],
    observe: &CommandId,
    opened: &Signal<u32>,
) -> MenuBar {
    let menus = order
        .iter()
        .map(|name| match *name {
            "file" => MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            ),
            "recent" => {
                let opened = opened.clone();
                MenuItem::submenu(
                    "recent",
                    "Recent",
                    vec![
                        MenuItem::new("song", "Song", move |_| opened.set(opened.get()? + 1)),
                        MenuItem::command(ui, "observe", observe, |_, _| Ok(())).unwrap(),
                    ],
                )
            }
            "edit" => MenuItem::submenu(
                "edit",
                "Edit",
                vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
            ),
            "help" => MenuItem::new("help", "Help", |_| Ok(())),
            other => panic!("unknown menu {other}"),
        })
        .collect();
    MenuBar::new(ui, menus).unwrap().content(Element::new(
        "document",
        Column::new([Element::new("control", Button::new("Control", |_| Ok(())))]),
    ))
}

#[test]
fn a_redescribed_menu_bar_takes_added_removed_and_reordered_menus() {
    let mut ui = ui();
    let opened = ui.signal(0u32);
    let focus = ui.signal(None);
    let seen = focus.clone();
    let observe = ui
        .register_command(
            "Observe",
            CommandScope::Application,
            vec![],
            move |ui, context| {
                let key = context
                    .focused
                    .as_ref()
                    .map(|id| ui.key(id).map(str::to_owned))
                    .transpose()?;
                seen.set(Some(key))
            },
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "bar",
                described_bar(&ui, &["file", "edit", "help"], &observe, &opened),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let redescribe = |ui: &mut Runtime, order: &[&str]| {
        let bar = described_bar(ui, order, &observe, &opened);
        ui.reconcile_children(&root, vec![Element::new("bar", bar)])
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    let control = find(&ui, &root, "control").unwrap();
    ui.focus(&control).unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // With the File menu open from the keyboard, a "Recent" menu is added.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(expanded(&ui, "file"), Some(true));
    let file = find(&ui, &root, "title/file").unwrap();
    redescribe(&mut ui, &["file", "recent", "edit", "help"]);
    assert_eq!(find(&ui, &root, "title/file"), Some(file));
    assert_eq!(
        expanded(&ui, "file"),
        Some(true),
        "the open menu stays open"
    );
    assert_eq!(expanded(&ui, "recent"), Some(false));
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));

    // Keyboard navigation reaches the new menu, and its command row acts on
    // the context captured before the bar was engaged.
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(expanded(&ui, "recent"), Some(true));
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(focus.get().unwrap(), Some(Some("control".to_owned())));
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/recent"));
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(opened.get().unwrap(), 1);
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));

    // Removing the open menu closes it; adding it back does not reopen it,
    // and it opens again when asked.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(expanded(&ui, "recent"), Some(true));
    redescribe(&mut ui, &["file", "edit", "help"]);
    assert!(find(&ui, &root, "title/recent").is_none());
    assert_eq!(expanded(&ui, "file"), Some(false));
    assert_eq!(expanded(&ui, "edit"), Some(false));
    redescribe(&mut ui, &["file", "recent", "edit", "help"]);
    assert_eq!(expanded(&ui, "recent"), Some(false));
    let recent = find(&ui, &root, "title/recent").unwrap();
    let (x, y) = center(&ui, &recent);
    click(&mut ui, x, y);
    assert_eq!(expanded(&ui, "recent"), Some(true));
    press(&mut ui, KeyCode::Escape);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(expanded(&ui, "recent"), Some(false));

    // Reordering changes the order of the titles on screen and for the keyboard.
    redescribe(&mut ui, &["help", "edit", "file"]);
    let left = |ui: &Runtime, key: &str| {
        ui.bounds(&find(ui, &root, key).unwrap())
            .unwrap()
            .unwrap()
            .x()
    };
    assert!(left(&ui, "title/help") < left(&ui, "title/edit"));
    assert!(left(&ui, "title/edit") < left(&ui, "title/file"));
    ui.focus(&control).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/help"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/edit"));
    press(&mut ui, KeyCode::End);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );

    // The runtime stays usable for unrelated work once the bar is gone.
    ui.unmount(&root).unwrap();
    ui.mount_tree(Element::new("other", Label::new("Other")))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

/// Rows keyed by `keys`, each writing its own key to `chosen`; a key ending in
/// `/` is a submenu of the rows after the slash, such as `recent/x,y`.
fn rows(keys: &[&str], chosen: &Signal<String>) -> Vec<MenuItem> {
    keys.iter()
        .map(|key| match key.split_once('/') {
            Some((branch, nested)) => {
                let nested: Vec<&str> = nested.split(',').collect();
                MenuItem::submenu(branch, branch.to_uppercase(), rows(&nested, chosen))
            }
            None => {
                let chosen = chosen.clone();
                let key = (*key).to_owned();
                MenuItem::new(key.clone(), key.to_uppercase(), move |_| {
                    chosen.set(key.clone())
                })
            }
        })
        .collect()
}
/// Item names of the visible list inside the shown submenu keyed `branch`.
fn submenu_names(ui: &Runtime, branch: &str) -> Option<Vec<String>> {
    let components = ui.inspect().components;
    let panel = components
        .iter()
        .find(|c| c.key == format!("submenu/{branch}") && c.visible)?
        .id;
    let inside = |mut id: Option<u64>| -> Option<bool> {
        while let Some(current) = id {
            if current == panel {
                return Some(true);
            }
            id = components.iter().find(|c| c.id == current)?.parent;
        }
        Some(false)
    };
    let items = components
        .iter()
        .find(|c| c.key == "items" && c.visible && inside(c.parent) == Some(true))?;
    let semantics = items.details.semantics.as_ref()?;
    Some(names(semantics).into_iter().map(str::to_owned).collect())
}

#[test]
fn a_redescribed_menu_takes_new_rows_and_keeps_its_open_state() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let menu = |ui: &Runtime, keys: &[&str]| Menu::new(ui, "File", rows(keys, &chosen)).unwrap();
    let first = menu(&ui, &["a"]);
    let root = open_menu(&mut ui, first);
    assert_eq!(names(&semantics(&ui, "items")), ["A"]);
    let redescribe = |ui: &mut Runtime, keys: &[&str]| {
        let next = menu(ui, keys);
        ui.reconcile_children(&root, vec![Element::new("menu", next)])
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };

    // Rows added while the menu is open appear in it, and it stays open.
    redescribe(&mut ui, &["a", "b", "c"]);
    assert_eq!(names(&semantics(&ui, "items")), ["A", "B", "C"]);
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "c");
    assert_eq!(
        focused_key(&ui).as_deref(),
        Some("trigger"),
        "the row closed the menu"
    );

    // The new description's trigger opens the mounted menu.
    press(&mut ui, KeyCode::Enter);
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));
    redescribe(&mut ui, &["c", "a"]);
    assert_eq!(names(&semantics(&ui, "items")), ["C", "A"]);
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "c");
}

#[test]
fn a_redescribed_submenu_takes_new_rows_and_closes_when_its_row_goes() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let menu = |ui: &Runtime, keys: &[&str]| Menu::new(ui, "File", rows(keys, &chosen)).unwrap();
    let first = menu(&ui, &["new", "recent/x"]);
    let root = open_menu(&mut ui, first);
    let redescribe = |ui: &mut Runtime, keys: &[&str]| {
        let next = menu(ui, keys);
        ui.reconcile_children(&root, vec![Element::new("menu", next)])
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(submenu_names(&ui, "recent"), Some(vec!["X".to_owned()]));

    // A "Recent files" list that grows while its submenu is open.
    redescribe(&mut ui, &["new", "recent/x,y"]);
    assert_eq!(
        submenu_names(&ui, "recent"),
        Some(vec!["X".to_owned(), "Y".to_owned()])
    );
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "y");

    // Removing the open submenu's row closes it; restoring the row does not
    // reopen it, and it opens again when asked.
    press(&mut ui, KeyCode::Enter);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowRight);
    assert!(submenu_names(&ui, "recent").is_some());
    redescribe(&mut ui, &["new"]);
    assert!(find(&ui, &root, "submenu/recent").is_none());
    redescribe(&mut ui, &["new", "recent/x"]);
    assert_eq!(submenu_names(&ui, "recent"), None);
    // The closed submenu keeps its own "items", so the menu's rows need a path.
    let items = ui
        .find_path(&root, &["menu", "popup", "actions", "items"])
        .unwrap()
        .unwrap();
    ui.focus(&items).unwrap();
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(submenu_names(&ui, "recent"), Some(vec!["X".to_owned()]));
}

#[test]
fn a_redescribed_context_menu_takes_new_rows() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let context = |ui: &Runtime, keys: &[&str]| {
        ContextMenu::new(
            ui,
            Element::new("target", Button::new("Target", |_| Ok(()))),
            rows(keys, &chosen),
        )
        .unwrap()
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("context", context(&ui, &["a"]))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new("context", context(&ui, &["a", "b"]))],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let target = find(&ui, &root, "target").unwrap();
    let (x, y) = center(&ui, &target);
    point(&mut ui, x, y, PointerKind::Press(PointerButton::Secondary));
    point(
        &mut ui,
        x,
        y,
        PointerKind::Release(PointerButton::Secondary),
    );
    assert_eq!(names(&semantics(&ui, "items")), ["A", "B"]);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "b");
    assert!(
        ui.inspect()
            .components
            .iter()
            .all(|component| component.key != "items" || !component.visible),
        "the row closed the menu"
    );
}

#[test]
fn a_redescribed_bar_menu_takes_new_rows() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let bar = |ui: &Runtime, keys: &[&str]| {
        MenuBar::new(
            ui,
            vec![MenuItem::submenu("file", "File", rows(keys, &chosen))],
        )
        .unwrap()
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("bar", bar(&ui, &["a"]))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(names(&semantics(&ui, "items")), ["A"]);
    ui.reconcile_children(&root, vec![Element::new("bar", bar(&ui, &["a", "b", "c"]))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(names(&semantics(&ui, "items")), ["A", "B", "C"]);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "c");
}

/// A bar with a leading mark, File and Edit titles, an optional trailing
/// button and a document below.
fn slotted_bar(ui: &Runtime, trailing: bool) -> MenuBar {
    let bar = MenuBar::new(
        ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            ),
            MenuItem::submenu(
                "edit",
                "Edit",
                vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
            ),
        ],
    )
    .unwrap()
    .leading(Element::new("mark", Button::new("Mark", |_| Ok(()))))
    .content(Element::new(
        "document",
        Column::new([Element::new("control", Button::new("Control", |_| Ok(())))]),
    ));
    if trailing {
        bar.trailing(Element::new("back", Button::new("Back", |_| Ok(()))))
    } else {
        bar
    }
}
fn mount_slotted(ui: &mut Runtime, width: f32) -> WidgetId {
    ui.set_viewport(Viewport::new(width, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("bar", slotted_bar(ui, true))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    root
}
fn rect_of(ui: &Runtime, root: &WidgetId, key: &str) -> [f32; 4] {
    ui.bounds(&find(ui, root, key).unwrap())
        .unwrap()
        .unwrap()
        .values()
}

#[test]
fn slots_sit_on_the_title_row_before_the_titles_and_at_the_right_edge() {
    let mut ui = ui();
    let root = mount_slotted(&mut ui, 600.0);
    let bar = rect_of(&ui, &root, "bar");
    let mark = rect_of(&ui, &root, "mark");
    let file = rect_of(&ui, &root, "title/file");
    let edit = rect_of(&ui, &root, "title/edit");
    let back = rect_of(&ui, &root, "back");
    let control = rect_of(&ui, &root, "control");
    // The mark leads, the titles follow it, and the back button ends the row.
    assert_eq!(mark[0], bar[0]);
    assert_eq!(file[0], mark[0] + mark[2]);
    assert_eq!(edit[0], file[0] + file[2]);
    assert!((back[0] + back[2] - (bar[0] + bar[2])).abs() < 0.01);
    // Slots are centered in the row, and the content starts below it.
    let row = file[3];
    assert!((mark[1] + mark[3] / 2.0 - (file[1] + row / 2.0)).abs() < 0.01);
    assert!((back[1] + back[3] / 2.0 - (file[1] + row / 2.0)).abs() < 0.01);
    assert!(control[1] >= file[1] + row);
    ui.paint().unwrap();
}

#[test]
fn a_narrow_bar_moves_its_trailing_slot_after_the_titles_instead_of_over_them() {
    let mut ui = ui();
    let root = mount_slotted(&mut ui, 600.0);
    let edit = rect_of(&ui, &root, "title/edit");
    let back = rect_of(&ui, &root, "back");
    let narrow = edit[0] + edit[2] + back[2] / 2.0;
    let mut ui = Runtime::new();
    let root = mount_slotted(&mut ui, narrow);
    let edit = rect_of(&ui, &root, "title/edit");
    let back = rect_of(&ui, &root, "back");
    assert_eq!(back[0], edit[0] + edit[2]);
    assert!(
        back[0] + back[2] > narrow,
        "the slot is clipped at the edge"
    );
    ui.paint().unwrap();
}

#[test]
fn slots_are_tab_stops_that_f10_and_the_title_keys_skip() {
    let mut ui = ui();
    let root = mount_slotted(&mut ui, 600.0);
    // Tab order: leading, trailing, content; titles are not Tab stops.
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("mark"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("back"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("control"));
    // Arrow keys on a slot do not move into the titles.
    let back = find(&ui, &root, "back").unwrap();
    ui.focus(&back).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(focused_key(&ui).as_deref(), Some("back"));
    // F10 from a slot reaches the first title, never a slot, and leaving
    // returns to the slot.
    press(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/edit"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(focused_key(&ui).as_deref(), Some("back"));
}

#[test]
fn a_redescribed_bar_keeps_its_slots_by_key_and_can_drop_one() {
    let mut ui = ui();
    let root = mount_slotted(&mut ui, 600.0);
    let mark = find(&ui, &root, "mark").unwrap();
    ui.focus(&mark).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let bar = slotted_bar(&ui, false);
    ui.reconcile_children(&root, vec![Element::new("bar", bar)])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The mark keeps its identity and focus; the trailing slot is gone.
    assert_eq!(find(&ui, &root, "mark"), Some(mark.clone()));
    assert_eq!(ui.focused(), Some(mark));
    assert!(find(&ui, &root, "back").is_none());
    assert!(rect_of(&ui, &root, "control")[3] > 0.0);
    let bar = slotted_bar(&ui, true);
    ui.reconcile_children(&root, vec![Element::new("bar", bar)])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let back = rect_of(&ui, &root, "back");
    let bar = rect_of(&ui, &root, "bar");
    assert!((back[0] + back[2] - (bar[0] + bar[2])).abs() < 0.01);
}

#[test]
fn a_final_fixed_denial_keeps_bound_menu_actions_and_branches_disabled() {
    let mut ui = ui();
    let enabled = ui.signal(true);
    let chosen = ui.signal(false);
    let write = chosen.clone();
    let menu = Menu::new(
        &ui,
        "Actions",
        vec![
            MenuItem::submenu(
                "blocked",
                "Blocked",
                vec![MenuItem::new("nested", "Nested", |_| Ok(()))],
            )
            .enabled(enabled.read_only())
            .disabled(),
            MenuItem::new("blocked-action", "Blocked action", |_| Ok(()))
                .enabled(enabled.read_only())
                .disabled(),
            MenuItem::new("allowed", "Allowed", move |_| write.set(true)),
        ],
    )
    .unwrap();
    let root = open_menu(&mut ui, menu);
    for value in [true, false, true] {
        enabled.set(value).unwrap();
        ui.flush(ui.now()).unwrap();
        let items = semantics(&ui, "items");
        assert!(items.items[0].disabled);
        assert!(items.items[1].disabled);
        let list = ui
            .find_path(&root, &["menu", "popup", "actions", "items"])
            .unwrap()
            .unwrap();
        let bounds = ui.bounds(&list).unwrap().unwrap();
        click(&mut ui, bounds.x() + 10.0, bounds.y() + 10.0);
        assert_eq!(submenu_names(&ui, "blocked"), None);
        assert!(!chosen.get().unwrap());
    }
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::Enter);
    assert!(chosen.get().unwrap());
}

#[test]
fn live_enablement_cannot_open_empty_or_all_disabled_submenus() {
    let mut ui = ui();
    let enabled = ui.signal(true);
    let menu = Menu::new(
        &ui,
        "Actions",
        vec![
            MenuItem::submenu("empty", "Empty", vec![]).enabled(enabled.read_only()),
            MenuItem::submenu(
                "disabled",
                "Disabled",
                vec![MenuItem::new("child", "Child", |_| Ok(())).disabled()],
            )
            .enabled(enabled.read_only()),
            MenuItem::new("allowed", "Allowed", |_| Ok(())),
        ],
    )
    .unwrap();
    let root = open_menu(&mut ui, menu);
    for value in [true, false, true] {
        enabled.set(value).unwrap();
        ui.flush(ui.now()).unwrap();
        let items = semantics(&ui, "items");
        assert!(items.items[0].disabled && items.items[1].disabled);
        press(&mut ui, KeyCode::Home);
        press(&mut ui, KeyCode::ArrowRight);
        assert_eq!(submenu_names(&ui, "empty"), None);
        assert_eq!(submenu_names(&ui, "disabled"), None);
        let list = ui
            .find_path(&root, &["menu", "popup", "actions", "items"])
            .unwrap()
            .unwrap();
        let bounds = ui.bounds(&list).unwrap().unwrap();
        click(&mut ui, bounds.x() + 10.0, bounds.y() + 10.0);
        assert_eq!(submenu_names(&ui, "empty"), None);
    }
}

fn native_menu_key(ui: &mut Runtime, code: KeyCode) {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput {
            physical: code,
            action,
            base: Some(CommandKey::Named(code)),
            logical: Some(CommandKey::Named(code)),
            has_text: false,
            composing: false,
        })
        .unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
}

#[test]
fn hidden_menu_titles_compact_layout_and_native_navigation_without_retargeting_slots() {
    for (hidden, first) in [
        (vec!["file"], Some("title/edit")),
        (vec!["edit"], Some("title/file")),
        (vec!["about"], Some("title/file")),
        (vec!["file", "edit", "empty", "about"], None),
    ] {
        let mut ui = ui();
        let bar = bar(&mut ui);
        let document = find(&ui, &bar.root, "control").unwrap();
        for key in &hidden {
            let title = find(&ui, &bar.root, &format!("title/{key}")).unwrap();
            ui.set_visible(&title, false).unwrap();
        }
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
        native_menu_key(&mut ui, KeyCode::F10);
        assert_eq!(focused_key(&ui).as_deref(), first.or(Some("control")));
        if first.is_some() {
            let last = if hidden.contains(&"about") {
                "title/edit"
            } else {
                "title/about"
            };
            native_menu_key(&mut ui, KeyCode::End);
            assert_eq!(focused_key(&ui).as_deref(), Some(last));
            native_menu_key(&mut ui, KeyCode::Home);
            assert_eq!(focused_key(&ui).as_deref(), first);
            native_menu_key(&mut ui, KeyCode::ArrowLeft);
            assert_eq!(focused_key(&ui).as_deref(), Some(last));
            native_menu_key(&mut ui, KeyCode::ArrowRight);
            assert_eq!(focused_key(&ui).as_deref(), first);
            native_menu_key(&mut ui, KeyCode::Escape);
            assert_eq!(ui.focused(), Some(document.clone()));
        }
        for key in &hidden {
            let title = find(&ui, &bar.root, &format!("title/{key}")).unwrap();
            ui.set_visible(&title, true).unwrap();
        }
        ui.flush(Duration::ZERO).unwrap();
        native_menu_key(&mut ui, KeyCode::F10);
        assert_eq!(focused_key(&ui).as_deref(), Some("title/file"));
    }
}

#[test]
fn hidden_menu_titles_and_optional_slots_keep_visible_slot_geometry() {
    let mut ui = ui();
    let root = mount_slotted(&mut ui, 600.0);
    let file = find(&ui, &root, "title/file").unwrap();
    ui.set_visible(&file, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let mark = rect_of(&ui, &root, "mark");
    let edit = rect_of(&ui, &root, "title/edit");
    let back = rect_of(&ui, &root, "back");
    let content = rect_of(&ui, &root, "control");
    assert_eq!(edit[0], mark[0] + mark[2]);
    assert!(back[0] >= edit[0] + edit[2]);
    assert!(content[1] >= edit[1] + edit[3]);
    for key in ["leading", "trailing", "content"] {
        let slot = find(&ui, &root, key).unwrap();
        ui.set_visible(&slot, false).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
    }
    assert_eq!(rect_of(&ui, &root, "title/edit")[0], 0.0);
}

#[test]
fn hiding_an_open_menu_title_cancels_its_menu_and_allows_f10_on_a_visible_title() {
    let mut ui = ui();
    let bar = bar(&mut ui);
    let file = find(&ui, &bar.root, "title/file").unwrap();
    let document = find(&ui, &bar.root, "control").unwrap();
    native_menu_key(&mut ui, KeyCode::F10);
    native_menu_key(&mut ui, KeyCode::Enter);
    assert_eq!(expanded(&ui, "file"), Some(true));
    ui.set_visible(&file, false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&document).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    native_menu_key(&mut ui, KeyCode::F10);
    assert_eq!(focused_key(&ui).as_deref(), Some("title/edit"));
    native_menu_key(&mut ui, KeyCode::Enter);
    assert_eq!(expanded(&ui, "edit"), Some(true));
    native_menu_key(&mut ui, KeyCode::Escape);
    ui.set_visible(&file, true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(expanded(&ui, "file"), Some(false));
}

fn menu_is_open(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .find(|item| item.key == "menu")
        .unwrap()
        .details
        .properties
        .get("open")
        == Some(&InspectValue::Bool(true))
}

#[test]
fn replacement_external_menu_signal_controls_opening_rows_and_dismissal() {
    let mut ui = ui();
    let first = ui.signal(false);
    let second = ui.signal(false);
    let chosen = ui.signal(String::new());
    let menu = Menu::with_state(first.clone(), "Actions", rows(&["old"], &chosen)).unwrap();
    let root = open_menu(&mut ui, menu);
    assert!(first.get().unwrap());
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "menu",
            Menu::with_state(second.clone(), "Actions", rows(&["new"], &chosen)).unwrap(),
        )],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(!menu_is_open(&ui));
    assert!(first.get().unwrap());
    assert!(!second.get().unwrap());
    let trigger = find(&ui, &root, "trigger").unwrap();
    ui.focus(&trigger).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(second.get().unwrap());
    assert_eq!(names(&semantics(&ui, "items")), ["NEW"]);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "new");
    assert!(!second.get().unwrap());
    assert!(first.get().unwrap());
    first.set(false).unwrap();
    first.set(true).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(!menu_is_open(&ui));
    second.set(true).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(menu_is_open(&ui));
    press(&mut ui, KeyCode::Escape);
    assert!(!second.get().unwrap());
    assert!(first.get().unwrap());
    assert!(!menu_is_open(&ui));
}

#[test]
fn external_menu_replacement_resets_context_but_same_source_keeps_it() {
    for replace in [false, true] {
        let mut ui = ui();
        let first = ui.signal(false);
        let second = ui.signal(true);
        let observed = ui.signal(None);
        let recorded = observed.clone();
        let command = ui
            .register_command(
                "Observe menu context",
                CommandScope::Application,
                vec![],
                move |ui, context| {
                    let key = context
                        .focused
                        .as_ref()
                        .map(|id| ui.key(id).map(str::to_owned))
                        .transpose()?;
                    recorded.set(Some(key))
                },
            )
            .unwrap();
        let describe = |ui: &Runtime, open| {
            Menu::with_state(
                open,
                "Actions",
                vec![MenuItem::command(ui, "observe", &command, |_, _| Ok(())).unwrap()],
            )
            .unwrap()
        };
        let menu = describe(&ui, first.clone());
        let root = open_menu(&mut ui, menu);
        let focused = ui.focused().unwrap();
        let binding = if replace {
            second.clone()
        } else {
            first.clone()
        };
        ui.reconcile_children(
            &root,
            vec![Element::new("menu", describe(&ui, binding.clone()))],
        )
        .unwrap();
        ui.flush(ui.now()).unwrap();
        assert!(menu_is_open(&ui)); // Replacement true is read, never forced false.
        assert_eq!(ui.focused(), Some(focused));
        press(&mut ui, KeyCode::Enter);
        // The fallback is captured after the row closes the surface. Its hidden
        // focus is sanitized before invocation; only a retained engagement keeps
        // the original opener, never a replaced engagement.
        assert_eq!(
            observed.get().unwrap(),
            Some(if replace {
                None
            } else {
                Some("trigger".to_owned())
            })
        );
        assert!(!binding.get().unwrap());
        if replace {
            assert!(first.get().unwrap());
        }
    }
}

#[test]
fn replacement_external_menu_keeps_live_row_permissions() {
    let mut ui = ui();
    let first = ui.signal(false);
    let second = ui.signal(false);
    let enabled = ui.signal(true);
    let chosen = ui.signal(0);
    let menu = Menu::with_state(
        first.clone(),
        "Actions",
        rows(&["old"], &ui.signal(String::new())),
    )
    .unwrap();
    let root = open_menu(&mut ui, menu);
    let written = chosen.clone();
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "menu",
            Menu::with_state(
                second.clone(),
                "Actions",
                vec![
                    MenuItem::new("new", "New", move |_| written.set(written.get()? + 1))
                        .enabled(enabled.read_only()),
                    MenuItem::new("available", "Available", |_| Ok(())),
                ],
            )
            .unwrap(),
        )],
    )
    .unwrap();
    ui.flush(ui.now()).unwrap();
    second.set(true).unwrap();
    ui.flush(ui.now()).unwrap();
    enabled.set(false).unwrap();
    ui.flush(ui.now()).unwrap();
    assert!(semantics(&ui, "items").items[0].disabled);
    let list = find(&ui, &root, "items").unwrap();
    let bounds = ui.bounds(&list).unwrap().unwrap();
    let (x, y) = (bounds.x() + 10.0, bounds.y() + 10.0);
    click(&mut ui, x, y);
    assert_eq!(chosen.get().unwrap(), 0);
    assert!(second.get().unwrap());
    enabled.set(true).unwrap();
    ui.flush(ui.now()).unwrap();
    // The same point now runs the first row, so the refused click above hit
    // that disabled row rather than missing the rows.
    click(&mut ui, x, y);
    assert_eq!(chosen.get().unwrap(), 1);
    assert!(!second.get().unwrap());
    assert!(first.get().unwrap());
}

#[test]
fn a_redescribed_context_menu_follows_whether_any_row_can_run() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let context = |ui: &Runtime, enabled: bool| {
        let items = rows(&["a"], &chosen)
            .into_iter()
            .map(|item| if enabled { item } else { item.disabled() })
            .collect();
        ContextMenu::new(
            ui,
            Element::new("target", Button::new("Target", |_| Ok(()))),
            items,
        )
        .unwrap()
    };
    let open = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .any(|component| component.key == "items" && component.visible)
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("context", context(&ui, false))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let target = find(&ui, &root, "target").unwrap();
    let (x, y) = center(&ui, &target);
    let secondary = |ui: &mut Runtime| {
        point(ui, x, y, PointerKind::Press(PointerButton::Secondary));
        point(ui, x, y, PointerKind::Release(PointerButton::Secondary));
    };
    secondary(&mut ui);
    assert!(!open(&ui), "an all-disabled menu does not open");
    ui.reconcile_children(&root, vec![Element::new("context", context(&ui, true))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    secondary(&mut ui);
    assert!(open(&ui), "the enabled description opens");
    press(&mut ui, KeyCode::Escape);
    ui.reconcile_children(&root, vec![Element::new("context", context(&ui, false))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    secondary(&mut ui);
    assert!(!open(&ui), "the disabled description no longer opens");
}

#[test]
fn a_revoked_menu_title_ends_its_open_menu_and_armed_press() {
    let mut ui = ui();
    let file_on = ui.signal(true);
    let about_on = ui.signal(true);
    let about = ui.signal(0);
    let count = about.clone();
    let bar = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            )
            .enabled(file_on.read_only()),
            MenuItem::new("about", "About", move |_| count.set(count.get()? + 1))
                .enabled(about_on.read_only()),
        ],
    )
    .unwrap();
    let root = ui.mount_tree(Element::new("bar", bar)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let file = find(&ui, &root, "title/file").unwrap();
    let (x, y) = center(&ui, &file);
    click(&mut ui, x, y);
    assert_eq!(expanded(&ui, "file"), Some(true));
    for enabled in [false, true] {
        file_on.set(enabled).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
    assert_eq!(
        expanded(&ui, "file"),
        Some(false),
        "re-enabling does not reopen the revoked menu"
    );
    let title = find(&ui, &root, "title/about").unwrap();
    let (x, y) = center(&ui, &title);
    point(&mut ui, x, y, PointerKind::Move);
    point(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    for enabled in [false, true] {
        about_on.set(enabled).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    }
    point(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
    assert_eq!(
        about.get().unwrap(),
        0,
        "the revoked press does not activate"
    );
    click(&mut ui, x, y);
    assert_eq!(about.get().unwrap(), 1, "a new press still activates");
}

#[test]
fn an_open_menu_title_redescribed_as_an_action_is_no_longer_open() {
    let mut ui = ui();
    let describe = |ui: &Runtime, menu: bool| {
        let file = if menu {
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            )
        } else {
            MenuItem::new("file", "File", |_| Ok(()))
        };
        Element::new(
            "bar",
            MenuBar::new(
                ui,
                vec![
                    file,
                    MenuItem::submenu(
                        "edit",
                        "Edit",
                        vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
                    ),
                ],
            )
            .unwrap(),
        )
    };
    let root = ui
        .mount_tree(Element::new("root", Column::new([describe(&ui, true)])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let file = find(&ui, &root, "title/file").unwrap();
    let (x, y) = center(&ui, &file);
    click(&mut ui, x, y);
    assert_eq!(expanded(&ui, "file"), Some(true));
    ui.reconcile_children(&root, vec![describe(&ui, false)])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // No menu is open any more, so pointing at another title opens nothing.
    let edit = find(&ui, &root, "title/edit").unwrap();
    let (x, y) = center(&ui, &edit);
    point(&mut ui, x, y, PointerKind::Move);
    assert_eq!(expanded(&ui, "edit"), Some(false));
}

#[test]
fn revoked_menu_row_does_not_retarget_an_armed_key_or_resume_a_pointer_press() {
    for mouse in [false, true] {
        let mut ui = ui();
        let enabled = ui.signal(true);
        let calls = ui.signal(0);
        let a = calls.clone();
        let b = calls.clone();
        let menu = Menu::new(
            &ui,
            "Menu",
            vec![
                MenuItem::new("a", "A", move |_| a.set(a.get()? + 1)).enabled(enabled.read_only()),
                MenuItem::new("b", "B", move |_| b.set(b.get()? + 1)),
            ],
        )
        .unwrap();
        let root = open_menu(&mut ui, menu);
        let list = ui
            .find_path(&root, &["menu", "popup", "actions", "items"])
            .unwrap()
            .unwrap();
        let bounds = ui.bounds(&list).unwrap().unwrap();
        if mouse {
            point(
                &mut ui,
                bounds.x() + 10.0,
                bounds.y() + 10.0,
                PointerKind::Press(PointerButton::Primary),
            );
        } else {
            ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
        }
        enabled.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        if mouse {
            enabled.set(true).unwrap();
            ui.flush(ui.now()).unwrap();
            point(
                &mut ui,
                bounds.x() + 10.0,
                bounds.y() + 10.0,
                PointerKind::Release(PointerButton::Primary),
            );
        } else {
            ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
        }
        assert_eq!(calls.get().unwrap(), 0);
        press(&mut ui, KeyCode::Enter);
        assert_eq!(calls.get().unwrap(), 1);
    }
}

#[test]
fn menu_permissions_combine_in_both_builder_orders() {
    let mut ui = ui();
    let yes = ui.signal(true);
    let no = ui.signal(false);
    let menu = Menu::new(
        &ui,
        "Menu",
        vec![
            MenuItem::new("a", "A", |_| panic!("denied"))
                .disabled()
                .enabled(yes.read_only()),
            MenuItem::new("b", "B", |_| panic!("denied"))
                .enabled(yes.read_only())
                .disabled(),
            MenuItem::new("c", "C", |_| panic!("denied"))
                .enabled(no.read_only())
                .enabled(yes.read_only()),
            MenuItem::new("d", "D", |_| Ok(())),
        ],
    )
    .unwrap();
    open_menu(&mut ui, menu);
    let items = semantics(&ui, "items");
    assert!(items.items[..3].iter().all(|item| item.disabled));
    no.set(true).unwrap();
    ui.flush(ui.now()).unwrap();
    let items = semantics(&ui, "items");
    assert!(items.items[..2].iter().all(|item| item.disabled));
    assert!(!items.items[2].disabled);
}

#[test]
fn fixed_menu_redescription_cancels_a_held_key() {
    let mut ui = ui();
    let calls = ui.signal(0);
    let describe = |disabled| {
        let a = calls.clone();
        let b = calls.clone();
        let item = MenuItem::new("a", "A", move |_| a.set(a.get()? + 1));
        Menu::new(
            &ui,
            "Menu",
            vec![
                if disabled { item.disabled() } else { item },
                MenuItem::new("b", "B", move |_| b.set(b.get()? + 1)),
            ],
        )
        .unwrap()
    };
    let menu = describe(false);
    let replacement = describe(true);
    let root = open_menu(&mut ui, menu);
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.reconcile_children(&root, vec![Element::new("menu", replacement)])
        .unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    assert_eq!(calls.get().unwrap(), 0);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(calls.get().unwrap(), 1);
}

#[test]
fn title_fixed_denial_then_reenable_before_flush_cancels_the_pointer_press() {
    let mut ui = ui();
    let calls = ui.signal(0);
    let describe = |allowed| {
        let calls = calls.clone();
        Element::new(
            "bar",
            MenuBar::new(
                &ui,
                vec![
                    MenuItem::new("action", "Action", move |_| calls.set(calls.get()? + 1))
                        .enabled(allowed),
                ],
            )
            .unwrap(),
        )
    };
    let initial = describe(true);
    let denied = describe(false);
    let allowed = describe(true);
    let root = ui
        .mount_tree(Element::new("root", Column::new([initial])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let title = find(&ui, &root, "title/action").unwrap();
    let (x, y) = center(&ui, &title);
    point(&mut ui, x, y, PointerKind::Press(PointerButton::Primary));
    ui.reconcile_children(&root, vec![denied]).unwrap();
    ui.reconcile_children(&root, vec![allowed]).unwrap();
    point(&mut ui, x, y, PointerKind::Release(PointerButton::Primary));
    assert_eq!(calls.get().unwrap(), 0);
    click(&mut ui, x, y);
    assert_eq!(calls.get().unwrap(), 1);
}

#[test]
fn an_opener_opens_a_context_menu_from_application_code() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let menu = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        rows(&["a", "b"], &chosen),
    )
    .unwrap();
    let opener = menu.controller();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("context", menu)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let context = ui.command_context(CommandSource::Menu);
    assert!(
        opener
            .open(Some(Point::new(40.0, 12.0).unwrap()), &context)
            .unwrap()
    );
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(names(&semantics(&ui, "items")), ["A", "B"]);
    let popup = find(&ui, &root, "popup").unwrap();
    let [x, y, _, _] = ui.bounds(&popup).unwrap().unwrap().values();
    assert_eq!(
        (x, y),
        (40.0, 12.0),
        "opened at the requested local position"
    );
    assert_eq!(focused_key(&ui).as_deref(), Some("items"));

    // Opening again moves the open menu, and the rows still run.
    assert!(opener.open(None, &context).unwrap());
    ui.flush(Duration::ZERO).unwrap();
    let [_, y, _, _] = ui.bounds(&popup).unwrap().unwrap().values();
    assert!(y > 12.0, "below the content now: {y}");
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(chosen.get().unwrap(), "b");
    assert!(
        ui.inspect()
            .components
            .iter()
            .all(|component| component.key != "items" || !component.visible),
        "a chosen row closes the menu"
    );

    // An opener from a later description reaches the mounted menu.
    let later = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        rows(&["c"], &chosen),
    )
    .unwrap();
    let later_opener = later.controller();
    ui.reconcile_children(&root, vec![Element::new("context", later)])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(later_opener.open(None, &context).unwrap());
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(names(&semantics(&ui, "items")), ["C"]);
    press(&mut ui, KeyCode::Escape);

    // A menu whose rows cannot run does not open, as the gestures do nothing.
    let disabled = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        vec![MenuItem::new("off", "Off", |_| Ok(())).disabled()],
    )
    .unwrap();
    ui.reconcile_children(&root, vec![Element::new("context", disabled)])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!opener.open(None, &context).unwrap());
    assert!(!later_opener.open(None, &context).unwrap());
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .all(|component| component.key != "items" || !component.visible)
    );
}

#[test]
fn an_opener_refuses_a_hidden_or_removed_owner() {
    let mut ui = ui();
    let chosen = ui.signal(String::new());
    let visible = ui.signal(true);
    let menu = ContextMenu::new(
        &ui,
        Element::new("target", Button::new("Target", |_| Ok(()))),
        rows(&["a"], &chosen),
    )
    .unwrap();
    let opener = menu.controller();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "view",
                View::new(Element::new("context", menu)).visible_signal(visible.read_only()),
            )]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    visible.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let context = ui.command_context(CommandSource::Menu);
    assert!(
        !opener.open(None, &context).unwrap(),
        "a hidden owner cannot present its menu"
    );
    visible.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        ui.inspect()
            .components
            .iter()
            .all(|component| component.key != "items" || !component.visible),
        "the refused request does not open the menu on reveal"
    );
    assert!(opener.open(None, &context).unwrap());
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(names(&semantics(&ui, "items")), ["A"]);
    press(&mut ui, KeyCode::Escape);

    // A removed menu refuses as well, instead of writing state nothing reads.
    let view = find(&ui, &root, "view").unwrap();
    ui.reconcile_children(&view, Vec::new()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(!opener.open(None, &context).unwrap());
}
