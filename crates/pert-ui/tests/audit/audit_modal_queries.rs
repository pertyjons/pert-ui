//! Current modal admission in command queries before presentation synchronization.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

struct SurfaceOwner {
    column: Column,
    kind: Rc<Cell<SurfaceKind>>,
    updates: Rc<Cell<usize>>,
    declarations: Rc<Cell<usize>>,
    fail_dismissal: Rc<Cell<bool>>,
}
impl Component for SurfaceOwner {
    fn take_children(&mut self) -> Vec<Element> {
        self.column.take_children()
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.declarations.set(self.declarations.get() + 1);
        (key == "popup").then(|| self.kind.get())
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        if self.fail_dismissal.replace(false) {
            Err(RuntimeError::EmptyCommandName)
        } else {
            Ok(())
        }
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.column.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.column.arrange(cx, size)
    }
}
struct Stage {
    ui: Runtime,
    kind: Rc<Cell<SurfaceKind>>,
    updates: Rc<Cell<usize>>,
    declarations: Rc<Cell<usize>>,
    fail_dismissal: Rc<Cell<bool>>,
    owner: WidgetId,
    popup: WidgetId,
    inside_owner: WidgetId,
    outside_owner: WidgetId,
    other_root: WidgetId,
    primary: NativeWindowId,
    other: NativeWindowId,
}
fn stage(kind: SurfaceKind) -> Stage {
    stage_with_visibility(kind, true)
}
fn stage_with_visibility(kind: SurfaceKind, shown: bool) -> Stage {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let kind = Rc::new(Cell::new(kind));
    let updates = Rc::new(Cell::new(0));
    let fail_dismissal = Rc::new(Cell::new(false));
    let declarations = Rc::new(Cell::new(0));
    let root = ui
        .mount_tree(Element::new(
            "main",
            Column::new([
                Element::new(
                    "owner",
                    SurfaceOwner {
                        column: Column::new([
                            Element::new("popup", Button::new("Modal content", |_| Ok(()))),
                            Element::new("inside-owner", Button::new("Inside owner", |_| Ok(()))),
                        ]),
                        kind: kind.clone(),
                        updates: updates.clone(),
                        declarations: declarations.clone(),
                        fail_dismissal: fail_dismissal.clone(),
                    },
                ),
                Element::new("outside-owner", Button::new("Outside owner", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    let popup = ui.find_descendant(&root, "popup").unwrap().unwrap();
    ui.set_visible(&popup, false).unwrap();
    let other_root = ui
        .mount_tree(Element::new("other", Label::new("Other window")))
        .unwrap();
    let prepared = ui
        .prepare_detach(&other_root, NativeWindowOptions::new("Other").unwrap())
        .unwrap();
    let other = ui.commit_detach(prepared).unwrap();
    let primary = ui.primary_window();
    ui.select_window(&primary).unwrap();
    ui.set_visible(&popup, shown).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let inside_owner = ui.find_descendant(&root, "inside-owner").unwrap().unwrap();
    let outside_owner = ui.find_descendant(&root, "outside-owner").unwrap().unwrap();
    Stage {
        ui,
        kind,
        updates,
        declarations,
        fail_dismissal,
        owner,
        popup,
        inside_owner,
        outside_owner,
        other_root,
        primary,
        other,
    }
}
fn change_kind(s: &Stage, kind: SurfaceKind) {
    s.kind.set(kind);
    s.ui.invalidate(&s.owner, Invalidation::Layout).unwrap();
}
fn context(ui: &mut Runtime, window: &NativeWindowId, target: Option<WidgetId>) -> CommandContext {
    ui.select_window(window).unwrap();
    CommandContext {
        window: window.clone(),
        target,
        focused: None,
        source: CommandSource::Programmatic,
        repeat: false,
    }
}
fn query(ui: &mut Runtime, id: &CommandId, context: &CommandContext, expected: bool) {
    let selected = ui.current_window();
    let deadline = ui.application_deadline();
    assert_eq!(ui.command_info(id, context).unwrap().enabled, expected);
    let rows = ui.commands_for(context).unwrap();
    assert_eq!(
        rows.iter().find(|entry| entry.id == *id).unwrap().enabled,
        Ok(expected)
    );
    assert_eq!(ui.current_window(), selected);
    assert_eq!(ui.application_deadline(), deadline);
}

#[test]
fn hidden_or_downgraded_modal_releases_targetless_and_targeted_queries_without_flush() {
    for initial in [SurfaceKind::ApplicationDialog, SurfaceKind::Dialog] {
        for hide in [false, true] {
            let mut s = stage(initial);
            let commands = [
                s.ui.register_command("Application", CommandScope::Application, vec![], |_, _| {
                    Ok(())
                })
                .unwrap(),
                s.ui.register_command("Capture", CommandScope::Capture, vec![], |_, _| Ok(()))
                    .unwrap(),
            ];
            let mut contexts = Vec::new();
            for (window, target) in [
                (s.primary.clone(), s.outside_owner.clone()),
                (s.other.clone(), s.other_root.clone()),
            ] {
                contexts.push(context(&mut s.ui, &window, None));
                contexts.push(context(&mut s.ui, &window, Some(target)));
            }
            for c in &contexts {
                for command in &commands {
                    query(
                        &mut s.ui,
                        command,
                        c,
                        initial != SurfaceKind::ApplicationDialog && c.window == s.other,
                    );
                }
            }
            if hide {
                s.ui.set_visible(&s.popup, false).unwrap();
            } else {
                change_kind(&s, SurfaceKind::ContextMenu);
            }
            let updates = s.updates.get();
            for c in &contexts {
                for command in &commands {
                    query(&mut s.ui, command, c, true);
                }
            }
            assert_eq!(
                s.updates.get(),
                updates,
                "queries must not settle presentation"
            );
        }
    }
}

#[test]
fn current_modal_kind_controls_window_scope_before_surface_sync() {
    let mut s = stage(SurfaceKind::Dialog);
    let command =
        s.ui.register_command(
            "Window",
            CommandScope::Window(s.primary.clone()),
            vec![],
            |_, _| Ok(()),
        )
        .unwrap();
    let outside = context(&mut s.ui, &s.primary, Some(s.outside_owner.clone()));
    let inside_owner = context(&mut s.ui, &s.primary, Some(s.inside_owner.clone()));
    let inside_popup = context(&mut s.ui, &s.primary, Some(s.popup.clone()));
    let targetless = context(&mut s.ui, &s.primary, None);
    query(&mut s.ui, &command, &outside, false);
    change_kind(&s, SurfaceKind::ViewDialog);
    query(&mut s.ui, &command, &outside, true);
    query(&mut s.ui, &command, &inside_owner, false);
    query(&mut s.ui, &command, &inside_popup, true);
    change_kind(&s, SurfaceKind::Dialog);
    query(&mut s.ui, &command, &outside, false);
    change_kind(&s, SurfaceKind::ContextMenu);
    query(&mut s.ui, &command, &outside, true);
    query(&mut s.ui, &command, &targetless, true);
    change_kind(&s, SurfaceKind::Dialog);
    query(&mut s.ui, &command, &outside, false);
    query(&mut s.ui, &command, &targetless, false);
    query(&mut s.ui, &command, &inside_popup, true);
}

#[test]
fn hidden_native_root_keeps_application_modal_blocking_other_window_queries() {
    let mut s = stage(SurfaceKind::ApplicationDialog);
    let command =
        s.ui.register_command("Application", CommandScope::Application, vec![], |_, _| {
            Ok(())
        })
        .unwrap();
    s.ui.select_window(&s.primary).unwrap();
    s.ui.set_window_visible(false).unwrap();
    let targetless = context(&mut s.ui, &s.other, None);
    let targeted = context(&mut s.ui, &s.other, Some(s.other_root.clone()));
    query(&mut s.ui, &command, &targetless, false);
    query(&mut s.ui, &command, &targeted, false);
}

#[test]
fn admitted_nonmodal_to_modal_blocks_queries_before_surface_sync() {
    let mut s = stage(SurfaceKind::ContextMenu);
    let command =
        s.ui.register_command(
            "Window",
            CommandScope::Window(s.primary.clone()),
            vec![],
            |_, _| Ok(()),
        )
        .unwrap();
    let outside = context(&mut s.ui, &s.primary, Some(s.outside_owner.clone()));
    let targetless = context(&mut s.ui, &s.primary, None);
    let popup = context(&mut s.ui, &s.primary, Some(s.popup.clone()));
    query(&mut s.ui, &command, &outside, true);
    query(&mut s.ui, &command, &targetless, true);
    change_kind(&s, SurfaceKind::Dialog);
    let updates = s.updates.get();
    query(&mut s.ui, &command, &outside, false);
    query(&mut s.ui, &command, &targetless, false);
    query(&mut s.ui, &command, &popup, true);
    assert_eq!(s.updates.get(), updates);
}

#[test]
fn pending_application_admission_keeps_queries_fail_closed() {
    let mut s = stage(SurfaceKind::ContextMenu);
    let command =
        s.ui.register_command("Application", CommandScope::Application, vec![], |_, _| {
            Ok(())
        })
        .unwrap();
    s.ui.select_window(&s.primary).unwrap();
    s.fail_dismissal.set(true);
    s.ui.clear_focus(); // Queue a surface error before application admission.
    change_kind(&s, SurfaceKind::ApplicationDialog);
    assert_eq!(s.ui.flush(s.ui.now()), Err(RuntimeError::EmptyCommandName));
    let targetless = context(&mut s.ui, &s.other, None);
    let targeted = context(&mut s.ui, &s.other, Some(s.other_root.clone()));
    query(&mut s.ui, &command, &targetless, false);
    query(&mut s.ui, &command, &targeted, false);
    s.ui.flush_windows(s.ui.now()).unwrap();
    query(&mut s.ui, &command, &targetless, false);
}

#[test]
fn application_upgrade_of_an_admitted_surface_blocks_other_window_queries_immediately() {
    let mut s = stage(SurfaceKind::ContextMenu);
    let command =
        s.ui.register_command("Application", CommandScope::Application, vec![], |_, _| {
            Ok(())
        })
        .unwrap();
    let targetless = context(&mut s.ui, &s.other, None);
    let targeted = context(&mut s.ui, &s.other, Some(s.other_root.clone()));
    query(&mut s.ui, &command, &targetless, true);
    change_kind(&s, SurfaceKind::ApplicationDialog);
    let updates = s.updates.get();
    query(&mut s.ui, &command, &targetless, false);
    query(&mut s.ui, &command, &targeted, false);
    assert_eq!(s.updates.get(), updates);
    change_kind(&s, SurfaceKind::ContextMenu);
    query(&mut s.ui, &command, &targetless, true);
    query(&mut s.ui, &command, &targeted, true);
}

#[test]
fn first_show_and_reshow_block_queries_before_surface_admission() {
    for kind in [SurfaceKind::ApplicationDialog, SurfaceKind::Dialog] {
        for previously_shown in [false, true] {
            let mut s = stage_with_visibility(kind, previously_shown);
            if previously_shown {
                s.ui.set_visible(&s.popup, false).unwrap();
                s.ui.flush_windows(s.ui.now()).unwrap();
            }
            let command =
                s.ui.register_command("Application", CommandScope::Application, vec![], |_, _| {
                    Ok(())
                })
                .unwrap();
            let mut contexts = Vec::new();
            for (window, target) in [
                (s.primary.clone(), s.outside_owner.clone()),
                (s.other.clone(), s.other_root.clone()),
            ] {
                contexts.push(context(&mut s.ui, &window, None));
                contexts.push(context(&mut s.ui, &window, Some(target)));
            }
            for c in &contexts {
                query(&mut s.ui, &command, c, true);
            }
            s.ui.set_visible(&s.popup, true).unwrap();
            let updates = s.updates.get();
            for c in &contexts {
                query(
                    &mut s.ui,
                    &command,
                    c,
                    kind != SurfaceKind::ApplicationDialog && c.window == s.other,
                );
            }
            assert_eq!(s.updates.get(), updates);
        }
    }
}

#[test]
fn command_listing_reuses_one_modal_declaration_pass_for_all_rows() {
    let mut s = stage(SurfaceKind::Dialog);
    let context = context(&mut s.ui, &s.primary, Some(s.popup.clone()));
    s.ui.register_command("First", CommandScope::Application, vec![], |_, _| Ok(()))
        .unwrap();
    s.declarations.set(0);
    let first = s.ui.commands_for(&context).unwrap();
    let work = s.declarations.get();
    assert!(work > 0);
    for i in 0..32 {
        s.ui.register_command(
            format!("Another {i}"),
            CommandScope::Application,
            vec![],
            |_, _| Ok(()),
        )
        .unwrap();
    }
    s.declarations.set(0);
    let more = s.ui.commands_for(&context).unwrap();
    assert!(more.len() > first.len());
    assert_eq!(s.declarations.get(), work);
}

#[test]
fn idle_key_routing_and_invocation_do_not_scan_plain_children() {
    fn work(children: usize) -> (usize, usize) {
        let mut s = stage_with_visibility(SurfaceKind::ContextMenu, false);
        s.ui.mount_tree(Element::new(
            "probe",
            SurfaceOwner {
                column: Column::new(
                    (0..children)
                        .map(|i| Element::new(format!("plain-{i}"), Label::new("Plain content"))),
                ),
                kind: s.kind.clone(),
                updates: s.updates.clone(),
                declarations: s.declarations.clone(),
                fail_dismissal: s.fail_dismissal.clone(),
            },
        ))
        .unwrap();
        s.ui.flush_windows(s.ui.now()).unwrap();
        let command =
            s.ui.register_command("Run", CommandScope::Application, vec![], |_, _| Ok(()))
                .unwrap();
        let context = context(&mut s.ui, &s.primary, Some(s.outside_owner.clone()));
        s.declarations.set(0);
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Press,
            CommandKey::Character('a'),
        ))
        .unwrap();
        let key_work = s.declarations.get();
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Release,
            CommandKey::Character('a'),
        ))
        .unwrap();
        s.declarations.set(0);
        assert_eq!(
            s.ui.invoke_command(&command, context).unwrap(),
            CommandOutcome::Invoked
        );
        (key_work, s.declarations.get())
    }
    assert_eq!(work(1), work(128));
}

#[test]
fn key_routing_admits_new_application_dialog_in_another_window_before_text() {
    let mut s = stage_with_visibility(SurfaceKind::ApplicationDialog, false);
    s.ui.select_window(&s.other).unwrap();
    s.ui.set_visible(&s.popup, true).unwrap();
    let dispatch =
        s.ui.command_key_input(CommandKeyInput::new(
            KeyCode::KeyA,
            KeyAction::Press,
            CommandKey::Character('a'),
        ))
        .unwrap();
    assert!(dispatch.handled);
    assert!(!dispatch.deliver_text);
    assert_eq!(s.ui.current_window(), s.other);
}
