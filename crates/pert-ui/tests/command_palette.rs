//! `CommandPalette` contracts: pure ranking, the context captured at opening,
//! highlight keys that leave focus in the field, invocation that closes the
//! palette first and revalidates, outcomes that are shown instead of skipped,
//! dismissal, and the controller's lifetime rules.
use pert_ui::*;
mod support;
use std::{cell::Cell, cell::RefCell, rc::Rc};
use support::{find, key, settle, widget};

/// Registers a command that counts its invocations and records the context's
/// source.
fn counted(ui: &mut Runtime, name: &str) -> (CommandId, Rc<Cell<u32>>) {
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let id = ui
        .register_command(name, CommandScope::Application, Vec::new(), move |_, _| {
            seen.set(seen.get() + 1);
            Ok(())
        })
        .unwrap();
    (id, count)
}

struct Setup {
    ui: Runtime,
    root: WidgetId,
    palette: CommandPaletteController,
    opener: WidgetId,
}

/// An application with a focused button beside a palette whose controller
/// the returned setup opens.
fn setup(build: impl FnOnce(CommandPalette) -> CommandPalette) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let palette = CommandPaletteController::new(&ui);
    let content = Column::new([
        Element::new("edit", Button::new("Edit", |_| Ok(()))),
        Element::new("other", Button::new("Other", |_| Ok(()))),
    ]);
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([Element::new(
                "palette",
                build(CommandPalette::new(
                    palette.clone(),
                    Element::new("content", content),
                )),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let opener = widget(&ui, &root, "edit");
    ui.focus(&opener).unwrap();
    settle(&mut ui);
    Setup {
        ui,
        root,
        palette,
        opener,
    }
}

impl Setup {
    fn open(&mut self) {
        let context = self.ui.command_context(CommandSource::KeyBinding);
        self.palette.open(&mut self.ui, &context).unwrap();
        settle(&mut self.ui);
    }
    fn field(&self) -> WidgetId {
        widget(&self.ui, &self.root, "search")
    }
    /// The palette's listed rows, highlighted ones marked with `>`.
    fn rows(&self) -> Vec<String> {
        let snapshot = self.ui.inspect();
        let list = snapshot
            .components
            .iter()
            .find(|c| c.key == "content" && c.details.properties.contains_key("results"))
            .and_then(|c| c.details.semantics.clone())
            .expect("palette content");
        list.items
            .iter()
            .map(|item| {
                let name = item.name.clone().unwrap_or_default();
                if item.selected {
                    format!(">{name}")
                } else {
                    name
                }
            })
            .collect()
    }
    fn message(&self) -> Option<String> {
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.key == "message" && c.visible)
            .and_then(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
    }
}

#[test]
fn ranking_is_pure_deterministic_and_replaceable() {
    let mut ui = Runtime::new();
    for name in ["Toggle Sidebar", "Insert Text", "Save", "Save All", "Close"] {
        let _ = ui
            .register_command(name, CommandScope::Application, Vec::new(), |_, _| Ok(()))
            .unwrap();
    }
    let context = ui.command_context(CommandSource::Programmatic);
    let entries = ui.commands_for(&context).unwrap();
    let names = |order: Vec<usize>| -> Vec<&str> {
        order
            .into_iter()
            .map(|index| entries[index].name.as_str())
            .collect()
    };
    let fuzzy = CommandMatcher::default();
    // An empty or blank query keeps every row in snapshot (name) order.
    assert_eq!(
        names(fuzzy.rank("  ", &entries)),
        ["Close", "Insert Text", "Save", "Save All", "Toggle Sidebar"]
    );
    // Word starts outrank inner matches; rows that do not match are left out.
    assert_eq!(names(fuzzy.rank("ts", &entries)), ["Toggle Sidebar"]);
    assert_eq!(names(fuzzy.rank("t", &entries))[0], "Toggle Sidebar");
    assert_eq!(names(fuzzy.rank("sv", &entries)), ["Save", "Save All"]);
    assert!(fuzzy.rank("qq", &entries).is_empty());
    // Case and whitespace in the query do not matter.
    assert_eq!(
        fuzzy.rank("SAVE all", &entries),
        fuzzy.rank("saveall", &entries)
    );
    // Ranking twice gives the same order.
    assert_eq!(fuzzy.rank("s", &entries), fuzzy.rank("s", &entries));
    // A replacement matcher decides alone; equal scores keep snapshot order.
    let everything = CommandMatcher::new(|_, _| Some(7));
    assert_eq!(everything.rank("zzz", &entries), [0, 1, 2, 3, 4]);
}

#[test]
fn opening_focuses_the_field_and_lists_the_captured_context() {
    let mut s = setup(|p| p);
    let _ = counted(&mut s.ui, "Save");
    let _ = counted(&mut s.ui, "Close");
    // The palette registers no command of its own.
    let context = s.ui.command_context(CommandSource::Programmatic);
    assert_eq!(s.ui.commands_for(&context).unwrap().len(), 2);
    s.open();
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    assert_eq!(s.ui.focused(), Some(s.field()));
    assert_eq!(s.rows(), [">Close", "Save"]);
    // A command registered after opening is not in this session's snapshot.
    let _ = counted(&mut s.ui, "Zoom");
    settle(&mut s.ui);
    assert_eq!(s.rows(), [">Close", "Save"]);
}

#[test]
fn arrow_keys_move_the_highlight_and_a_new_query_starts_at_the_best_match() {
    let mut s = setup(|p| p);
    for name in ["Alpha", "Beta", "Gamma"] {
        let _ = counted(&mut s.ui, name);
    }
    s.open();
    key(&mut s.ui, KeyCode::ArrowDown);
    settle(&mut s.ui);
    assert_eq!(s.rows(), ["Alpha", ">Beta", "Gamma"]);
    // Focus and the query stay where they were.
    assert_eq!(s.ui.focused(), Some(s.field()));
    key(&mut s.ui, KeyCode::ArrowUp);
    key(&mut s.ui, KeyCode::ArrowUp);
    settle(&mut s.ui);
    assert_eq!(s.rows(), ["Alpha", "Beta", ">Gamma"], "Up wraps at the top");
    key(&mut s.ui, KeyCode::PageUp);
    settle(&mut s.ui);
    assert_eq!(
        s.rows(),
        [">Alpha", "Beta", "Gamma"],
        "a page stops at the end"
    );
    key(&mut s.ui, KeyCode::ArrowDown);
    settle(&mut s.ui);
    s.ui.text_input("a").unwrap();
    settle(&mut s.ui);
    // Every name contains an `a`; the new query highlights its best match.
    assert_eq!(s.rows()[0], ">Alpha");
}

#[test]
fn enter_invokes_the_highlighted_command_after_the_modal_palette_closes() {
    let mut s = setup(|p| p);
    let (_alpha_id, alpha) = counted(&mut s.ui, "Alpha");
    let target = Rc::new(RefCell::new(None));
    let seen = target.clone();
    let _ =
        s.ui.register_command(
            "Beta",
            CommandScope::Application,
            Vec::new(),
            move |ui, context| {
                // The modal palette is gone and the opener has focus again.
                *seen.borrow_mut() = Some((context.focused.clone(), context.source, ui.focused()));
                Ok(())
            },
        )
        .unwrap();
    s.open();
    key(&mut s.ui, KeyCode::ArrowDown);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(alpha.get(), 0);
    let (focused, source, now) = target.borrow().clone().expect("Beta ran");
    assert_eq!(focused, Some(s.opener.clone()));
    assert_eq!(source, CommandSource::Menu);
    assert_eq!(now, Some(s.opener.clone()));
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    assert_eq!(
        s.palette.receipt().unwrap().unwrap().status(),
        CommandStatus::Finished(Ok(CommandOutcome::Invoked))
    );
    assert_eq!(s.ui.focused(), Some(s.opener.clone()));
}

#[test]
fn a_command_disabled_since_opening_reopens_the_palette_with_the_outcome() {
    let mut s = setup(|p| p);
    let enabled = s.ui.signal(true);
    let (id, count) = counted(&mut s.ui, "Export");
    s.ui.command_enabled_signal(&id, enabled.read_only())
        .unwrap();
    s.open();
    enabled.set(false).unwrap();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(count.get(), 0);
    assert_eq!(
        s.palette.receipt().unwrap().unwrap().status(),
        CommandStatus::Finished(Ok(CommandOutcome::Disabled))
    );
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    assert_eq!(
        s.message().as_deref(),
        Some("“Export” is not available right now.")
    );
    assert_eq!(s.ui.focused(), Some(s.field()));
    // Escape closes the reopened palette, which then stays closed.
    key(&mut s.ui, KeyCode::Escape);
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    assert_eq!(s.ui.focused(), Some(s.opener.clone()));
}

#[test]
fn stale_contexts_and_failing_actions_are_shown_not_skipped() {
    let mut s = setup(|p| p.outcome_text(|name, result| format!("{name}: {result:?}")));
    let _ =
        s.ui.register_command("Fail", CommandScope::Application, Vec::new(), |_, _| {
            Err(ApplicationError::from("the engine is offline").into())
        })
        .unwrap();
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    let message = s.message().expect("an outcome message");
    assert!(message.starts_with("Fail: Err("), "{message}");
    assert!(message.contains("engine is offline"), "{message}");

    // A view-scoped command whose view is removed while the palette is open.
    let mut s = setup(|p| p);
    let view = widget(&s.ui, &s.root, "other");
    let _ =
        s.ui.register_command(
            "Scoped",
            CommandScope::View(view.clone()),
            Vec::new(),
            |_, _| Ok(()),
        )
        .unwrap();
    s.ui.set_command_target(Some(&view)).unwrap();
    let context = s.ui.command_context(CommandSource::KeyBinding);
    s.palette.open(&mut s.ui, &context).unwrap();
    settle(&mut s.ui);
    assert!(s.rows().iter().any(|row| row.contains("Scoped")));
    s.ui.set_visible(&view, false).unwrap();
    settle(&mut s.ui);
    // The palette is still open; choose the scoped command.
    let scoped = s
        .rows()
        .iter()
        .position(|row| row.contains("Scoped"))
        .unwrap();
    for _ in 0..scoped {
        key(&mut s.ui, KeyCode::ArrowDown);
    }
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    let status = s.palette.receipt().unwrap().unwrap().status();
    assert!(
        matches!(
            status,
            CommandStatus::Finished(Err(RuntimeError::StaleCommandTarget))
        ),
        "{status:?}"
    );
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    assert!(s.message().unwrap().contains("Scoped"));
}

#[test]
fn a_destination_command_reveals_outside_the_closed_modal_palette() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let palette = CommandPaletteController::new(&ui);
    let tabs = Tabs::new([
        Tab::new(
            "home",
            "Home",
            Element::new("home-button", Button::new("Home", |_| Ok(()))),
        ),
        Tab::new(
            "mixer",
            "Mixer",
            Element::new("fader", Button::new("Fader", |_| Ok(()))),
        ),
    ]);
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([Element::new(
                "palette",
                CommandPalette::new(palette.clone(), Element::new("tabs", tabs)),
            )]),
        ))
        .unwrap();
    settle(&mut ui);
    let fader = widget(&ui, &root, "fader");
    let target = fader.clone();
    let _ = ui
        .register_command(
            "Go to: Mixer",
            CommandScope::Application,
            Vec::new(),
            move |ui, _| {
                let outcome = ui.reveal_and_focus(&target)?;
                if outcome.is_revealed() {
                    Ok(())
                } else {
                    Err(ApplicationError::from(format!("{outcome:?}")).into())
                }
            },
        )
        .unwrap();
    let context = ui.command_context(CommandSource::KeyBinding);
    palette.open(&mut ui, &context).unwrap();
    settle(&mut ui);
    ui.text_input("mixer").unwrap();
    key(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(
        palette.receipt().unwrap().unwrap().status(),
        CommandStatus::Finished(Ok(CommandOutcome::Invoked))
    );
    assert_eq!(palette.status().unwrap(), CommandPaletteStatus::Closed);
    assert_eq!(ui.focused(), Some(fader));
}

#[test]
fn a_matching_row_press_and_release_invokes_it_and_the_backdrop_closes() {
    let mut s = setup(|p| p);
    let (_, alpha) = counted(&mut s.ui, "Alpha");
    let (_, beta) = counted(&mut s.ui, "Beta");
    s.open();
    let row = widget(&s.ui, &s.root, "command-1");
    let bounds = s.ui.bounds(&row).unwrap().unwrap();
    let at = Point::new(bounds.x() + 4.0, bounds.y() + 4.0).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        s.ui.pointer_input(PointerEvent::new(at, kind)).unwrap();
        if matches!(kind, PointerKind::Press(_)) {
            assert_eq!((alpha.get(), beta.get()), (0, 0));
            assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
        }
    }
    settle(&mut s.ui);
    assert_eq!((alpha.get(), beta.get()), (0, 1));

    s.open();
    // Far below the panel, on the backdrop.
    let at = Point::new(400.0, 590.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    assert_eq!((alpha.get(), beta.get()), (0, 1));
}

#[test]
fn a_nonmodal_palette_closes_on_an_outside_press_and_when_focus_leaves() {
    let mut s = setup(CommandPalette::nonmodal);
    let _ = counted(&mut s.ui, "Alpha");
    s.open();
    assert_eq!(s.ui.focused(), Some(s.field()));
    let snapshot = s.ui.inspect();
    let owner = snapshot
        .components
        .iter()
        .find(|c| c.key == "palette")
        .unwrap();
    assert_eq!(
        owner.details.semantics.as_ref().unwrap().role,
        SemanticRole::Popover
    );
    // Focus moved to the application's content closes it.
    s.ui.focus(&s.opener).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);

    s.open();
    // Below the panel and outside every surface.
    let at = Point::new(400.0, 590.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
}

#[test]
fn escape_clears_the_query_first_on_the_native_path_and_then_closes() {
    let mut s = setup(|p| p);
    let _ = counted(&mut s.ui, "Alpha");
    s.open();
    s.ui.text_input("al").unwrap();
    settle(&mut s.ui);
    let escape = |ui: &mut Runtime| {
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Escape,
                action,
                CommandKey::Named(KeyCode::Escape),
            ))
            .unwrap();
        }
        settle(ui);
    };
    escape(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    assert_eq!(s.rows(), [">Alpha"]);
    escape(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
}

#[test]
fn hiding_or_removing_the_palette_ends_its_session() {
    let mut s = setup(|p| p);
    let _ = counted(&mut s.ui, "Alpha");
    s.open();
    let owner = widget(&s.ui, &s.root, "palette");
    s.ui.set_visible(&owner, false).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    s.ui.set_visible(&owner, true).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);

    s.open();
    let content = Element::new("content", Label::new("Gone"));
    s.ui.reconcile_children(&s.root, vec![content]).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    // A palette mounted again with the controller does not reopen that session.
    let again = Element::new(
        "palette",
        CommandPalette::new(
            s.palette.clone(),
            Element::new("content", Label::new("Back")),
        ),
    );
    s.ui.reconcile_children(&s.root, vec![again]).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    assert!(find(&s.ui, &s.root, "search").is_none_or(|field| s.ui.focused() != Some(field)));
}

#[test]
fn a_second_palette_with_the_same_controller_stays_closed() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let _ = counted(&mut ui, "Alpha");
    let palette = CommandPaletteController::new(&ui);
    let first = CommandPalette::new(palette.clone(), Element::new("a", Label::new("A")));
    let second = CommandPalette::new(palette.clone(), Element::new("b", Label::new("B")));
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([
                Element::new("slot", Column::new([Element::new("first", first)])),
                Element::new("second", second),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let context = ui.command_context(CommandSource::Programmatic);
    palette.open(&mut ui, &context).unwrap();
    settle(&mut ui);
    let open = |key: &str| {
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == key)
            .and_then(|c| c.details.properties.get("open").cloned())
    };
    assert_eq!(open("first"), Some(InspectValue::Bool(true)));
    assert_eq!(open("second"), Some(InspectValue::Bool(false)));
    palette.close().unwrap();
    settle(&mut ui);

    // Once the first palette is removed, the waiting one presents the next
    // session of the shared controller, although nothing re-describes it.
    let slot = widget(&ui, &root, "slot");
    ui.reconcile_children(&slot, Vec::new()).unwrap();
    settle(&mut ui);
    let context = ui.command_context(CommandSource::Programmatic);
    palette.open(&mut ui, &context).unwrap();
    settle(&mut ui);
    assert_eq!(palette.status().unwrap(), CommandPaletteStatus::Open);
    let open = |key: &str| {
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == key)
            .and_then(|c| c.details.properties.get("open").cloned())
    };
    assert_eq!(open("second"), Some(InspectValue::Bool(true)));
    assert!(find(&ui, &root, "search").is_some_and(|field| ui.focused() == Some(field)));
}

#[test]
fn a_nonmodal_palette_showing_an_outcome_still_closes_when_focus_leaves() {
    let mut s = setup(CommandPalette::nonmodal);
    let enabled = s.ui.signal(true);
    let (id, _) = counted(&mut s.ui, "Export");
    s.ui.command_enabled_signal(&id, enabled.read_only())
        .unwrap();
    s.open();
    enabled.set(false).unwrap();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
    assert_eq!(s.ui.focused(), Some(s.field()));
    s.ui.focus(&s.opener).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
}

#[test]
fn native_focus_loss_closes_modal_and_nonmodal_palette() {
    for nonmodal in [false, true] {
        let mut s = setup(|palette| {
            if nonmodal {
                palette.nonmodal()
            } else {
                palette
            }
        });
        let _ = counted(&mut s.ui, "Alpha");
        s.open();
        s.ui.cancel_input().unwrap();
        settle(&mut s.ui);
        assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Open);
        s.ui.clear_focus();
        assert_eq!(s.ui.focused(), None);
        settle(&mut s.ui);
        assert_eq!(s.palette.status().unwrap(), CommandPaletteStatus::Closed);
    }
}

#[test]
fn equivalent_inner_description_still_replaces_palette_outcome_formatter() {
    let mut s = setup(|p| p.outcome_text(|_, _| "Old outcome".into()));
    let _command =
        s.ui.register_command("Fail", CommandScope::Application, Vec::new(), |_, _| {
            Err(ApplicationError::from("offline").into())
        })
        .unwrap();
    s.open();
    let next = CommandPalette::new(
        s.palette.clone(),
        Element::new(
            "content",
            Column::new([
                Element::new("edit", Button::new("Edit", |_| Ok(()))),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ),
    )
    .outcome_text(|_, _| "New outcome".into());
    s.ui.reconcile_children(&s.root, vec![Element::new("palette", next)])
        .unwrap();
    settle(&mut s.ui);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.message().as_deref(), Some("New outcome"));
}
