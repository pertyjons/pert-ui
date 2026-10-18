//! Workspace capture uses current identities without presenting pending changes.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{frame, widget};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn section(key: &str) -> Section {
    Section::new(
        key,
        key,
        Element::new(format!("body-{key}"), Label::new(key)),
    )
}
fn tab(key: &str) -> Tab {
    Tab::new(
        key,
        key,
        Element::new(format!("body-{key}"), Label::new(key)),
    )
}
fn state(workspace: &Workspace, kind: &str) -> ViewState {
    workspace
        .entries()
        .find(|(_, state)| state.kind() == kind)
        .unwrap()
        .1
        .clone()
}
#[test]
fn accordion_capture_excludes_removed_keys_and_keeps_hidden_mounted_keys() {
    for initially_flushed in [false, true] {
        let mut ui = ui();
        let root = ui
            .mount_tree(Element::new(
                "accordion",
                Accordion::new([section("a"), section("b")])
                    .mode(ExpandMode::Multiple)
                    .expanded(["a", "b"]),
            ))
            .unwrap();
        if initially_flushed {
            frame(&mut ui);
        }
        ui.reconcile_children(&root, vec![section("a").into_element()])
            .unwrap();
        let a = widget(&ui, &root, "a");
        ui.set_visible(&a, false).unwrap();
        let saved = ui.capture_workspace().unwrap();
        let open = state(&saved, "accordion");
        assert_eq!(open.map("open").unwrap().len(), 1);
        assert_eq!(
            open.map("open").unwrap().get("a"),
            Some(&WorkspaceValue::flag(true))
        );
        assert!(ui.restore_workspace(&saved).unwrap().is_complete());
        ui.set_visible(&a, true).unwrap();
        frame(&mut ui);
        assert!(
            ui.inspect()
                .components
                .iter()
                .find(|c| c.key == "body-a")
                .unwrap()
                .visible
        );
    }
}
#[test]
fn tabs_capture_resolves_removed_selection_without_updates_or_notifications() {
    let mut ui = ui();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let sink = changes.clone();
    let root = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([tab("a"), tab("b"), tab("c")]).on_select(move |_, change| {
                sink.borrow_mut().push(change.selected.clone());
                Ok(())
            }),
        ))
        .unwrap();
    frame(&mut ui);
    let pick_b = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["tabs"]).unwrap(),
            ViewState::new("tabs")
                .unwrap()
                .with("selected", WorkspaceValue::text("b").unwrap())
                .unwrap(),
        )],
    )
    .unwrap();
    assert!(ui.restore_workspace(&pick_b).unwrap().is_complete());
    frame(&mut ui);
    changes.borrow_mut().clear();
    ui.reconcile_children(
        &root,
        [tab("x"), tab("c"), tab("a")]
            .into_iter()
            .map(Tab::into_element)
            .collect(),
    )
    .unwrap();
    let saved = ui.capture_workspace().unwrap();
    assert_eq!(state(&saved, "tabs").text("selected").unwrap(), "c");
    assert!(changes.borrow().is_empty());
    let snapshot = ui.inspect();
    let tabs = snapshot
        .components
        .iter()
        .find(|c| c.key == "tabs")
        .unwrap();
    assert_eq!(
        tabs.details.properties.get("selected"),
        Some(&InspectValue::Text("b".into()))
    );
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    frame(&mut ui);
    assert_eq!(*changes.borrow(), [Some("c".to_owned())]);
    ui.reconcile_children(&root, Vec::new()).unwrap();
    let empty = ui.capture_workspace().unwrap();
    assert!(state(&empty, "tabs").get("selected").is_none());
    assert!(ui.restore_workspace(&empty).unwrap().is_complete());
}
#[test]
fn all_detached_tabs_capture_without_an_inline_selection_and_roundtrip() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new("tabs", Tabs::new([tab("a"), tab("b")])))
        .unwrap();
    frame(&mut ui);
    let primary = ui.primary_window();
    let a = widget(&ui, &root, "a");
    let b = widget(&ui, &root, "b");
    let prepared = ui
        .prepare_detach(&a, NativeWindowOptions::new("A").unwrap())
        .unwrap();
    let first = ui.commit_detach(prepared).unwrap();
    ui.select_window(&primary).unwrap();
    frame(&mut ui);
    let prepared = ui
        .prepare_detach(&b, NativeWindowOptions::new("B").unwrap())
        .unwrap();
    let second = ui.commit_detach(prepared).unwrap();
    ui.select_window(&primary).unwrap();
    let saved = ui.capture_workspace().unwrap();
    assert!(state(&saved, "tabs").get("selected").is_none());
    assert!(ui.restore_workspace(&saved).unwrap().is_complete());
    ui.redock(&first).unwrap();
    ui.select_window(&primary).unwrap();
    frame(&mut ui);
    let saved = ui.capture_workspace().unwrap();
    assert_eq!(state(&saved, "tabs").text("selected").unwrap(), "a");
    ui.redock(&second).unwrap();
}
