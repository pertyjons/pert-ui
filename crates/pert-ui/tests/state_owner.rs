//! Built-in components and controllers can be constructed by a custom
//! component from `update` and from input, through the public facade only:
//! constructors take any `StateOwner`, not just the `Runtime`.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// Describes every built-in that owns local state, from its first update.
#[derive(Default)]
struct Describer {
    controllers: Option<Controllers>,
}

/// Controllers kept so their state outlives the description.
struct Controllers {
    _selection: ListSelection,
    _list: ListController,
    _dialog: DialogController,
    _form: FormStatus,
    _toasts: ToastQueue,
    _pick: QuickPickController,
    _palette: CommandPaletteController,
    _zoom: ZoomController,
}

fn rows() -> Vec<MenuItem> {
    vec![MenuItem::new("copy", "Copy", |_| Ok(()))]
}

impl Component for Describer {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.controllers.is_some() {
            return Ok(());
        }
        let options = || [Choice::new(1_u8, "One"), Choice::new(2, "Two")];
        let select = Select::new(cx, cx.signal(1_u8), options())?;
        let inline = InlineSelect::new(cx, cx.signal(2_u8), options())?;
        let multi = MultiSelect::new(cx, cx.signal(vec![1_u8]), options())?;
        let menu = Menu::new(cx, "Actions", rows())?;
        let icon = Menu::icon(cx, Icon::MoreVertical, "More", rows())?;
        let text = Element::new("target", Label::new("Right-click"));
        let context = ContextMenu::new(cx, text, rows())?;
        let bar = MenuBar::new(cx, vec![MenuItem::submenu("file", "File", rows())])?;
        let window =
            FloatingWindow::new(cx, "tool", "Tool", Element::new("body", Label::new("Body")));
        // Visits write the history, so an update only constructs it.
        let history = NavigationHistory::<u8>::new(cx)?;
        let navigator = HistoryNavigator::new(cx, history, |_, _| Ok(()))?;
        let controllers = Controllers {
            _selection: ListSelection::new(cx, SelectionMode::Multiple),
            _list: ListController::new(cx),
            _dialog: DialogController::new(cx),
            _form: FormStatus::new(cx),
            _toasts: ToastQueue::new(cx),
            _pick: QuickPickController::new(cx),
            _palette: CommandPaletteController::new(cx),
            _zoom: ZoomController::new(cx),
        };
        cx.request_children(vec![Element::new(
            "column",
            Column::new([
                Element::new("select", select),
                Element::new("inline", inline),
                Element::new("multi", multi),
                Element::new("menu", menu),
                Element::new("icon", icon),
                Element::new("context", context),
                Element::new("bar", bar),
                Element::new("windows", WindowLayer::new([window])),
                Element::new("history", navigator),
            ]),
        )]);
        self.controllers = Some(controllers);
        Ok(())
    }
}

#[test]
fn a_custom_component_builds_the_built_ins_from_update() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("describer", Describer::default()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let keys: Vec<String> = ui.inspect().components.into_iter().map(|c| c.key).collect();
    for key in [
        "select", "inline", "multi", "menu", "icon", "context", "bar", "tool", "history",
    ] {
        assert!(keys.iter().any(|k| k == key), "{key} was not mounted");
    }
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn an_event_handler_builds_built_ins_from_its_context() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let built = Rc::new(Cell::new(false));
    let flag = built.clone();
    let button = ui
        .mount_tree(Element::new(
            "build",
            Button::new("Build", move |cx| {
                let open = cx.signal(false);
                let _ = Menu::with_state(open, "Later", rows())?;
                let _ = Select::new(cx, cx.signal(0_u8), [Choice::new(0_u8, "Zero")])?;
                let _ = ListSelection::new(cx, SelectionMode::Single);
                flag.set(true);
                Ok(())
            }),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&button).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(built.get());
}

#[test]
fn the_runtime_and_references_to_it_still_construct() {
    let ui = Runtime::new();
    let by_ref: &Runtime = &ui;
    let nested = &by_ref;
    let _ = Select::new(&ui, ui.signal(1_u8), [Choice::new(1_u8, "One")]).unwrap();
    let _ = Select::new(by_ref, ui.signal(1_u8), [Choice::new(1_u8, "One")]).unwrap();
    let _ = Select::new(nested, ui.signal(1_u8), [Choice::new(1_u8, "One")]).unwrap();
    let mut owned = Runtime::new();
    let exclusive: &mut Runtime = &mut owned;
    let _ = Select::new(
        exclusive,
        exclusive.signal(1_u8),
        [Choice::new(1_u8, "One")],
    )
    .unwrap();
    // A `&mut Runtime` stays usable after it is passed: it is reborrowed.
    let _ = exclusive.signal(0_u8);
}
