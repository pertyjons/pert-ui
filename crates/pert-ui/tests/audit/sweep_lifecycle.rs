//! Lifecycle and redescription regressions from the post-repair review sweep.
use pert_ui::*;
use std::time::Duration;

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    ui.flush(ui.now()).unwrap();
}

#[test]
fn a_new_zoom_controller_ends_a_pan_in_progress() {
    let mut ui = runtime();
    let first = ZoomController::new(&ui);
    let second = ZoomController::new(&ui);
    let view = |controller: &ZoomController| {
        ZoomView::new([(
            Point::new(300.0, 250.0).unwrap(),
            Element::new("item", Label::new("Item")),
        )])
        .controller(controller)
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("canvas", view(&first))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 50.0, 50.0, PointerKind::Move);
    pointer(
        &mut ui,
        50.0,
        50.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 70.0, 60.0, PointerKind::Move);
    assert_ne!(
        first.view().get().unwrap(),
        ViewTransform::IDENTITY,
        "the pan moved"
    );
    ui.reconcile_children(&root, vec![Element::new("canvas", view(&second))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 150.0, 120.0, PointerKind::Move);
    pointer(
        &mut ui,
        150.0,
        120.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(
        second.view().get().unwrap(),
        ViewTransform::IDENTITY,
        "the old gesture never writes into the new controller"
    );
}

fn title_expanded(ui: &Runtime, key: &str) -> Option<bool> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key && component.visible)
        .and_then(|component| component.details.semantics)
        .and_then(|semantics| semantics.expanded)
}

#[test]
fn a_menu_title_that_changes_kind_ends_its_armed_press() {
    let mut ui = runtime();
    let bar = |ui: &Runtime, submenu: bool| {
        let item = if submenu {
            MenuItem::submenu(
                "about",
                "About",
                vec![MenuItem::new("info", "Info", |_| Ok(()))],
            )
        } else {
            MenuItem::new("about", "About", |_| Ok(()))
        };
        MenuBar::new(ui, vec![item]).unwrap()
    };
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("bar", bar(&ui, false))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let title = ui.find_descendant(&root, "title/about").unwrap().unwrap();
    let [x, y, w, h] = ui.bounds(&title).unwrap().unwrap().values();
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    pointer(&mut ui, cx, cy, PointerKind::Move);
    pointer(&mut ui, cx, cy, PointerKind::Press(PointerButton::Primary));
    ui.reconcile_children(&root, vec![Element::new("bar", bar(&ui, true))])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(
        &mut ui,
        cx,
        cy,
        PointerKind::Release(PointerButton::Primary),
    );
    assert_eq!(
        title_expanded(&ui, "title/about"),
        Some(false),
        "a press armed on the action does not open the new menu"
    );
}

fn tooltip_open(ui: &Runtime) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == "tip")
        .map(|component| component.details.properties["open"] == InspectValue::Bool(true))
        .unwrap()
}

#[test]
fn escape_dismisses_a_shown_tooltip_and_passes_when_none_is_shown() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new(
            "tip",
            Tooltip::new(
                Element::new("button", Button::new("Save", |_| Ok(()))),
                "Saves the file",
            ),
        )]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 10.0, 10.0, PointerKind::Move);
    ui.flush(Duration::from_millis(500)).unwrap();
    assert!(tooltip_open(&ui), "hover shows the help after the delay");
    let hits = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = hits.clone();
    ui.register_command(
        "Back",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::Escape),
            Modifiers::default(),
        )],
        move |_, _| {
            counted.set(counted.get() + 1);
            Ok(())
        },
    )
    .unwrap();
    let escape = |ui: &mut Runtime| {
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Escape,
                action,
                CommandKey::Named(KeyCode::Escape),
            ))
            .unwrap();
        }
        ui.flush(ui.now()).unwrap();
    };
    escape(&mut ui);
    assert!(!tooltip_open(&ui), "Escape dismisses the help");
    assert_eq!(hits.get(), 0, "and takes that Escape");
    escape(&mut ui);
    assert_eq!(hits.get(), 1, "with no help shown, Escape reaches commands");
    assert!(!tooltip_open(&ui), "dismissed help stays suppressed");
}

fn role_count(ui: &mut Runtime, role: ColorRole) -> usize {
    let color = ui.theme().palette(ThemeMode::Dark).unwrap().color(role);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .filter(|command| {
            matches!(command,
                DrawCommand::FillRect { color: fill, .. }
                | DrawCommand::DrawMask { color: fill, .. }
                if *fill == color)
        })
        .count()
}

#[test]
fn a_replaced_toast_queue_forgets_the_hovered_control() {
    let mut ui = runtime();
    ui.theme().set_mode(ThemeMode::Dark).unwrap();
    let host = |queue: &ToastController| {
        ToastHost::new(
            queue.clone(),
            Element::new("content", Label::new("Content")),
        )
    };
    let first = ToastController::new(&ui);
    first
        .push(Toast::new("Saved").action("Apply", |_| Ok(())))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("host", host(&first))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let stack = ui.find_descendant(&root, "toasts").unwrap().unwrap();
    let [x, y, w, h] = ui.bounds(&stack).unwrap().unwrap().values();
    let base = role_count(&mut ui, ColorRole::ButtonHover);
    // Find a point over one of the first toast's controls.
    let mut over = None;
    'scan: for row in 0..40 {
        for column in 0..80 {
            let point = (x + w * column as f32 / 80.0, y + h * row as f32 / 40.0);
            pointer(&mut ui, point.0, point.1, PointerKind::Move);
            if role_count(&mut ui, ColorRole::ButtonHover) > base {
                over = Some(point);
                break 'scan;
            }
        }
    }
    let (px, py) = over.expect("a toast control takes hover");
    // The replacement's toast keeps local ID 1 but has a different layout.
    let second = ToastController::new(&ui);
    second
        .push(
            Toast::new("A much longer message that wraps onto several lines in the stack")
                .action("Apply", |_| Ok(())),
        )
        .unwrap();
    ui.reconcile_children(&root, vec![Element::new("host", host(&second))])
        .unwrap();
    ui.flush(ui.now()).unwrap();
    let after = role_count(&mut ui, ColorRole::ButtonHover);
    pointer(&mut ui, px, py, PointerKind::Move);
    let fresh = role_count(&mut ui, ColorRole::ButtonHover);
    assert_eq!(after, fresh, "hover is what the pointer is over now");
}

#[test]
fn a_narrow_floating_window_ends_its_title_in_an_ellipsis() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let title = "A very long window title that cannot fit a narrow frame";
    let window = FloatingWindow::new(&ui, "one", title, Element::new("body", Label::new("Body")))
        .position(Point::new(20.0, 20.0).unwrap())
        .unwrap()
        .initial_size(Size::from([160, 120]))
        .unwrap();
    ui.mount_tree(Element::new("layer", WindowLayer::new([window])))
        .unwrap();
    ui.flush(std::time::Duration::ZERO).unwrap();
    let snapshot = ui.inspect();
    let label = snapshot
        .components
        .iter()
        .find(|c| c.key == "label" && c.details.properties.contains_key("truncated"))
        .expect("the title reports its truncation");
    assert_eq!(
        label.details.properties["truncated"],
        InspectValue::Bool(true)
    );
    let InspectValue::Text(shown) = &label.details.properties["shown"] else {
        panic!("a truncated title reports its shown text");
    };
    assert!(shown.ends_with('…'), "{shown}");
    let window = snapshot.components.iter().find(|c| c.key == "one").unwrap();
    assert_eq!(
        window.details.semantics.as_ref().unwrap().name.as_deref(),
        Some(title),
        "the window keeps its full name"
    );
}

#[test]
fn a_modified_escape_leaves_shown_help_to_its_command() {
    let mut ui = runtime();
    ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new(
            "tip",
            Tooltip::new(
                Element::new("button", Button::new("Save", |_| Ok(()))),
                "Saves the file",
            ),
        )]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    pointer(&mut ui, 10.0, 10.0, PointerKind::Move);
    ui.flush(Duration::from_millis(500)).unwrap();
    assert!(tooltip_open(&ui));
    let hits = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = hits.clone();
    ui.register_command(
        "Chord",
        CommandScope::Application,
        vec![KeyChord::new(
            CommandKey::Named(KeyCode::Escape),
            Modifiers::PRIMARY,
        )],
        move |_, _| {
            counted.set(counted.get() + 1);
            Ok(())
        },
    )
    .unwrap();
    ui.native_modifiers(Modifiers::PRIMARY).unwrap();
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(
            KeyCode::Escape,
            action,
            CommandKey::Named(KeyCode::Escape),
        ))
        .unwrap();
    }
    ui.native_modifiers(Modifiers::default()).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(hits.get(), 1, "the modified Escape reaches its command");
    assert!(tooltip_open(&ui), "and leaves the help shown");
}

#[test]
fn a_narrow_tool_window_header_fits_its_title_beside_its_controls() {
    let panel = |width: f32, title: &str| {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, 300.0, 1.0).unwrap())
            .unwrap();
        let controller = ToolWindowController::new();
        let layout = ToolWindowLayout::new(Element::new("main", Label::new("Main")))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .controller(&controller)
            .window(ToolWindow::new(
                "files",
                title,
                Element::new("body", Label::new("Body")),
            ));
        ui.mount_tree(Element::new("tools", layout)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        controller.toggle("files").unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.inspect()
            .components
            .into_iter()
            .find(|c| {
                c.details.properties.get("tool-window") == Some(&InspectValue::Text("files".into()))
            })
            .expect("the panel is mounted")
            .details
    };
    let narrow = panel(200.0, "A long tool window title");
    assert_eq!(narrow.properties["truncated"], InspectValue::Bool(true));
    let InspectValue::Text(shown) = &narrow.properties["shown"] else {
        panic!("a truncated title reports what it shows");
    };
    assert!(shown.is_empty() || shown.ends_with('…'), "{shown:?}");
    assert_eq!(
        narrow.semantics.unwrap().name.as_deref(),
        Some("A long tool window title"),
        "the full title stays the panel's name"
    );
    let wide = panel(1600.0, "A long tool window title");
    assert_eq!(wide.properties["truncated"], InspectValue::Bool(false));
    // With no room for any title, an empty one has still lost nothing.
    let squeezed = panel(40.0, "Files");
    assert_eq!(squeezed.properties["truncated"], InspectValue::Bool(true));
    assert_eq!(
        squeezed.properties["shown"],
        InspectValue::Text(String::new())
    );
    let untitled = panel(40.0, "");
    assert_eq!(untitled.properties["truncated"], InspectValue::Bool(false));
}
