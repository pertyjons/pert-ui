//! Collapsible sections and accordions: toggling, retained hidden state,
//! focus on collapse, bound expansion, reveals, keyed identity and workspace
//! persistence.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn flush(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn label(key: &str) -> Element {
    Element::new(key, Label::new(key))
}

fn button(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("{key} is not mounted"))
}

fn shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == key && component.visible)
}

fn details(ui: &Runtime, key: &str) -> ComponentDetails {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component keyed {key}"))
        .details
}

fn press(ui: &mut Runtime, code: KeyCode) {
    ui.key_input(code, KeyAction::Press).unwrap();
    ui.key_input(code, KeyAction::Release).unwrap();
    flush(ui);
}

fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) {
    ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
        .unwrap();
    flush(ui);
}

fn click(ui: &mut Runtime, x: f32, y: f32) {
    pointer(ui, x, y, PointerKind::Press(PointerButton::Primary));
    pointer(ui, x, y, PointerKind::Release(PointerButton::Primary));
}

/// The top of a component in window coordinates.
fn top(ui: &Runtime, id: &WidgetId) -> f32 {
    ui.bounds(id).unwrap().expect("laid out").y()
}

fn section(key: &str, content: Element) -> Section {
    Section::new(key, format!("Section {key}"), content)
}

fn accordion(mode: ExpandMode, open: &[&str]) -> Element {
    Element::new(
        "accordion",
        Accordion::new([
            section("a", button("a-button")),
            section("b", button("b-button")),
            section("c", button("c-button")),
        ])
        .mode(mode)
        .expanded(open.iter().copied()),
    )
}

#[test]
fn the_header_toggles_with_keys_and_a_click_on_the_header_only() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "section",
            Collapsible::new("Details", button("body")),
        ))
        .unwrap();
    flush(&mut ui);
    assert!(!shown(&ui, "body"));
    ui.focus(&root).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(shown(&ui, "body"));
    press(&mut ui, KeyCode::Space);
    assert!(!shown(&ui, "body"));
    // A held key repeats nothing and an unmatched release toggles nothing.
    ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
    flush(&mut ui);
    assert!(!shown(&ui, "body"), "Escape cancels the armed key");
    click(&mut ui, 40.0, 5.0);
    assert!(shown(&ui, "body"));
    // A press on the content is the content's, not the header's.
    let body = find(&ui, &root, "body");
    let y = top(&ui, &body) + 2.0;
    click(&mut ui, 40.0, y);
    assert!(shown(&ui, "body"));
    // A press released away from the header does not toggle.
    pointer(
        &mut ui,
        40.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    pointer(&mut ui, 40.0, y, PointerKind::Move);
    pointer(
        &mut ui,
        40.0,
        y,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(shown(&ui, "body"));
}

#[test]
fn collapsed_content_keeps_its_state_and_focus_moves_to_the_header() -> TestResult {
    let mut ui = ui();
    let open = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "outer",
        Collapsible::new(
            "Outer",
            Element::new(
                "content",
                Column::new([
                    button("inner-button"),
                    Element::new(
                        "inner",
                        Collapsible::new("Inner", label("deep")).expanded(true),
                    ),
                ]),
            ),
        )
        .bind_expanded(open.clone()),
    ))?;
    flush(&mut ui);
    let inner = find(&ui, &root, "inner");
    let focused = find(&ui, &root, "inner-button");
    ui.focus(&focused)?;
    open.set(false)?;
    flush(&mut ui);
    assert!(!shown(&ui, "inner-button"));
    assert_eq!(ui.focused(), Some(root.clone()));
    open.set(true)?;
    flush(&mut ui);
    // The nested section kept its expansion and its identity.
    assert_eq!(find(&ui, &root, "inner"), inner);
    assert!(shown(&ui, "deep"));
    Ok(())
}

#[test]
fn a_bound_section_follows_and_writes_its_signal_and_is_not_persisted() -> TestResult {
    let mut ui = ui();
    let open = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "section",
        Collapsible::new("Bound", label("body"))
            .expanded(true)
            .bind_expanded(open.clone()),
    ))?;
    flush(&mut ui);
    assert!(
        !shown(&ui, "body"),
        "the signal wins over the initial state"
    );
    ui.focus(&root)?;
    press(&mut ui, KeyCode::Enter);
    assert!(open.get()?);
    assert!(shown(&ui, "body"));
    open.set(false)?;
    flush(&mut ui);
    assert!(!shown(&ui, "body"));
    assert_eq!(ui.capture_workspace()?.entries().count(), 0);
    assert_eq!(
        details(&ui, "section").properties.get("persist"),
        Some(&InspectValue::Bool(false))
    );
    Ok(())
}

#[test]
fn revealing_into_a_collapsed_section_expands_it_and_can_focus() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new("first", "First", label("first-body")),
            Tab::new(
                "second",
                "Second",
                Element::new(
                    "section",
                    Collapsible::new("Buried", button("target")).persist(false),
                ),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "target");
    assert_eq!(
        ui.reveal_and_focus(&target)?,
        RevealOutcome::RevealedAndFocused { activation: None }
    );
    assert!(shown(&ui, "target"));
    assert_eq!(ui.focused(), Some(target.clone()));
    assert_eq!(
        details(&ui, "section").properties.get("expanded"),
        Some(&InspectValue::Bool(true))
    );
    Ok(())
}

#[test]
fn a_bound_section_reveal_writes_the_signal_and_a_disabled_one_refuses() -> TestResult {
    let mut ui = ui();
    let open = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "bound",
                Collapsible::new("Bound", button("bound-target")).bind_expanded(open.clone()),
            ),
            Element::new(
                "disabled",
                Collapsible::new("Disabled", button("disabled-target")).disabled(),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "bound-target");
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Revealed { activation: None }
    );
    assert!(open.get()?);
    let disabled = find(&ui, &root, "disabled");
    let target = find(&ui, &root, "disabled-target");
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Refused(RevealRefusal::AncestorRefused { ancestor: disabled })
    );
    assert!(!shown(&ui, "disabled-target"));
    Ok(())
}

#[test]
fn a_disabled_section_takes_no_focus_and_does_not_toggle() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "section",
            Collapsible::new("Off", label("body")).disabled(),
        ))
        .unwrap();
    flush(&mut ui);
    click(&mut ui, 40.0, 5.0);
    assert!(!shown(&ui, "body"));
    assert_ne!(ui.focused(), Some(root));
    let semantics = details(&ui, "section").semantics.unwrap();
    assert!(semantics.disabled);
}

#[test]
fn hiding_and_focus_loss_cancel_an_armed_header() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new("section", Collapsible::new("Header", label("body"))),
                button("elsewhere"),
            ]),
        ))
        .unwrap();
    flush(&mut ui);
    let section = find(&ui, &root, "section");
    pointer(
        &mut ui,
        40.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    ui.set_visible(&section, false).unwrap();
    flush(&mut ui);
    ui.set_visible(&section, true).unwrap();
    flush(&mut ui);
    pointer(
        &mut ui,
        40.0,
        5.0,
        PointerKind::Release(PointerButton::Primary),
    );
    assert!(!shown(&ui, "body"), "hiding cancelled the press");

    ui.focus(&section).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    let elsewhere = find(&ui, &root, "elsewhere");
    ui.focus(&elsewhere).unwrap();
    ui.focus(&section).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    flush(&mut ui);
    assert!(!shown(&ui, "body"), "focus loss cancelled the key");
}

#[test]
fn a_press_armed_for_replaced_content_does_not_toggle_the_replacement() -> TestResult {
    let mut ui = ui();
    let mount = ui.mount_tree(Element::new(
        "host",
        Column::new([Element::new(
            "section",
            Collapsible::new("Details", label("old")),
        )]),
    ))?;
    flush(&mut ui);
    let section = find(&ui, &mount, "section");
    ui.focus(&section)?;
    flush(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    // The content is described again under a new key while Enter is held.
    ui.reconcile_children(
        &mount,
        vec![Element::new(
            "section",
            Collapsible::new("Details", label("new")),
        )],
    )?;
    flush(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    flush(&mut ui);
    assert!(
        !shown(&ui, "new"),
        "the release must not toggle the new content"
    );
    Ok(())
}

#[test]
fn collapsible_workspace_round_trips_and_rejects_foreign_fields() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("kept", Collapsible::new("Kept", label("kept-body"))),
            Element::new(
                "transient",
                Collapsible::new("Transient", label("transient-body")).persist(false),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let kept = find(&ui, &root, "kept");
    ui.focus(&kept)?;
    press(&mut ui, KeyCode::Enter);
    let saved = ui.capture_workspace()?;
    let paths: Vec<String> = saved.entries().map(|(path, _)| path.to_string()).collect();
    assert_eq!(paths, ["column/kept"]);
    let state = saved.entries().next().unwrap().1.clone();
    assert_eq!(state.kind(), "collapsible");
    assert!(state.flag("expanded")?);

    press(&mut ui, KeyCode::Enter);
    assert!(!shown(&ui, "kept-body"));
    assert!(ui.restore_workspace(&saved)?.is_complete());
    flush(&mut ui);
    assert!(shown(&ui, "kept-body"));

    let path = WorkspacePath::new(["column", "kept"])?;
    let unknown = ViewState::new("collapsible")?
        .with("expanded", WorkspaceValue::flag(false))?
        .with("height", WorkspaceValue::number(4.0)?)?;
    let report = ui.restore_workspace(&Workspace::new(
        WorkspaceSettings::default(),
        [(path.clone(), unknown)],
    )?)?;
    assert!(
        report
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::UnknownValue("height".into()))
    );
    let wrong = ViewState::new("collapsible")?.with("expanded", WorkspaceValue::integer(0))?;
    let report = ui.restore_workspace(&Workspace::new(
        WorkspaceSettings::default(),
        [(path, wrong)],
    )?)?;
    assert!(
        report
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Value("expanded".into()))
    );
    flush(&mut ui);
    assert!(shown(&ui, "kept-body"), "rejected entries changed nothing");
    Ok(())
}

#[test]
fn an_armed_section_refuses_restore() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "section",
        Collapsible::new("A", label("body")),
    ))?;
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    pointer(
        &mut ui,
        40.0,
        5.0,
        PointerKind::Press(PointerButton::Primary),
    );
    assert!(
        ui.restore_workspace(&saved)?
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    Ok(())
}

#[test]
fn accordion_keys_move_the_active_header_and_toggle_it() {
    let mut ui = ui();
    let root = ui.mount_tree(accordion(ExpandMode::Multiple, &[])).unwrap();
    flush(&mut ui);
    ui.focus(&root).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert!(shown(&ui, "b") && !shown(&ui, "a"));
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Space);
    assert!(
        shown(&ui, "b") && shown(&ui, "c"),
        "multiple sections stay open"
    );
    press(&mut ui, KeyCode::Home);
    assert_eq!(
        details(&ui, "accordion").properties.get("active"),
        Some(&InspectValue::Text("a".into()))
    );
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(
        details(&ui, "accordion").properties.get("active"),
        Some(&InspectValue::Text("a".into()))
    );
    // A modified arrow is not navigation.
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)
        .unwrap();
    flush(&mut ui);
    assert_eq!(
        details(&ui, "accordion").properties.get("active"),
        Some(&InspectValue::Text("a".into()))
    );
    let semantics = details(&ui, "accordion").semantics.unwrap();
    let rows: Vec<_> = semantics
        .items
        .iter()
        .map(|item| (item.name.clone().unwrap(), item.selected, item.expanded))
        .collect();
    assert_eq!(
        rows,
        [
            ("Section a".to_owned(), true, Some(false)),
            ("Section b".to_owned(), false, Some(true)),
            ("Section c".to_owned(), false, Some(true)),
        ]
    );
}

#[test]
fn single_mode_closes_the_open_section_and_moves_focus_out_of_it() {
    let mut ui = ui();
    let root = ui
        .mount_tree(accordion(ExpandMode::Single, &["a"]))
        .unwrap();
    flush(&mut ui);
    let a_button = find(&ui, &root, "a-button");
    ui.focus(&a_button).unwrap();
    // Click the header of section b, below section a's content.
    let b = find(&ui, &root, "b");
    let _ = b;
    let a_bottom = {
        let bounds = ui.bounds(&a_button).unwrap().unwrap();
        bounds.y() + bounds.height()
    };
    click(&mut ui, 40.0, a_bottom + 5.0);
    assert!(!shown(&ui, "a") && shown(&ui, "b"));
    assert_eq!(ui.focused(), Some(root.clone()));
    // Revealing into section c follows the same policy.
    let c_button = find(&ui, &root, "c-button");
    assert_eq!(
        ui.reveal(&c_button).unwrap(),
        RevealOutcome::Revealed { activation: None }
    );
    assert!(!shown(&ui, "b") && shown(&ui, "c"));
}

#[test]
fn sections_keep_expansion_and_state_by_key_under_reorder_and_rename() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(accordion(ExpandMode::Multiple, &["b"]))?;
    flush(&mut ui);
    let b_button = find(&ui, &root, "b-button");
    ui.reconcile_children(
        &root,
        vec![
            Section::new("c", "Renamed c", button("c-button")).into_element(),
            Section::new("b", "Renamed b", button("b-button")).into_element(),
            Section::new("a", "Renamed a", button("a-button")).into_element(),
        ],
    )?;
    flush(&mut ui);
    assert!(shown(&ui, "b") && !shown(&ui, "a") && !shown(&ui, "c"));
    assert_eq!(find(&ui, &root, "b-button"), b_button);
    let names: Vec<_> = details(&ui, "accordion")
        .semantics
        .unwrap()
        .items
        .into_iter()
        .map(|item| item.name.unwrap())
        .collect();
    assert_eq!(names, ["Renamed c", "Renamed b", "Renamed a"]);

    // A removed section forgets its expansion.
    ui.reconcile_children(
        &root,
        vec![Section::new("a", "A", button("a-button")).into_element()],
    )?;
    flush(&mut ui);
    ui.reconcile_children(
        &root,
        vec![
            Section::new("a", "A", button("a-button")).into_element(),
            Section::new("b", "B", button("b-button")).into_element(),
        ],
    )?;
    flush(&mut ui);
    assert!(!shown(&ui, "b"));
    Ok(())
}

#[test]
fn switching_to_single_mode_keeps_the_first_open_section() -> TestResult {
    let mut ui = ui();
    let mount = ui.mount_tree(Element::new(
        "host",
        Column::new([accordion(ExpandMode::Multiple, &["b", "c"])]),
    ))?;
    flush(&mut ui);
    assert!(shown(&ui, "b") && shown(&ui, "c"));
    ui.reconcile_children(&mount, vec![accordion(ExpandMode::Single, &[])])?;
    flush(&mut ui);
    assert!(shown(&ui, "b") && !shown(&ui, "c"));
    Ok(())
}

#[test]
fn a_mode_change_with_a_reorder_keeps_the_first_open_section_in_the_new_order() -> TestResult {
    let mut ui = ui();
    let mount = ui.mount_tree(Element::new(
        "host",
        Column::new([accordion(ExpandMode::Multiple, &["b", "c"])]),
    ))?;
    flush(&mut ui);
    assert!(shown(&ui, "b") && shown(&ui, "c"));
    // One description both reorders the sections to c, b, a and switches to
    // single mode: the first open section as now shown is c.
    ui.reconcile_children(
        &mount,
        vec![Element::new(
            "accordion",
            Accordion::new([
                section("c", button("c-button")),
                section("b", button("b-button")),
                section("a", button("a-button")),
            ])
            .mode(ExpandMode::Single),
        )],
    )?;
    flush(&mut ui);
    assert!(shown(&ui, "c") && !shown(&ui, "b"));
    Ok(())
}

#[test]
fn a_persisted_accordion_fails_capture_for_a_key_the_workspace_cannot_store() -> TestResult {
    let long = "k".repeat(Workspace::MAX_STRING_BYTES + 1);
    let describe = |persist: bool| {
        Element::new(
            "accordion",
            Accordion::new([section(&long, label("content"))])
                .expanded([long.clone()])
                .persist(persist),
        )
    };
    // Unpersisted, any key works and the accordion stays out of the capture.
    let mut ui = ui();
    ui.mount_tree(describe(false))?;
    flush(&mut ui);
    assert!(shown(&ui, "content"));
    assert!(ui.capture_workspace()?.is_empty());
    // Persisted, the accordion still works, but capturing names it instead
    // of reporting success without its expansion.
    let mut ui = self::ui();
    ui.mount_tree(describe(true))?;
    flush(&mut ui);
    assert!(shown(&ui, "content"));
    match ui.capture_workspace() {
        Err(WorkspaceError::Participant { path, source }) => {
            assert_eq!(path, "accordion");
            assert_eq!(*source, WorkspaceError::TextTooLong);
        }
        other => panic!("expected a participant failure, got {other:?}"),
    }
    Ok(())
}

#[test]
fn accordion_workspace_round_trips_and_rejects_invalid_documents() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(accordion(ExpandMode::Single, &["b"]))?;
    flush(&mut ui);
    let saved = ui.capture_workspace()?;
    let state = saved
        .entries()
        .find(|(path, _)| path.to_string() == "accordion")
        .unwrap()
        .1
        .clone();
    assert_eq!(state.kind(), "accordion");
    assert_eq!(
        state.map("open")?.get("b"),
        Some(&WorkspaceValue::flag(true))
    );
    assert_eq!(state.map("open")?.len(), 1);

    let section_a = find(&ui, &root, "a-button");
    ui.reveal(&section_a)?;
    flush(&mut ui);
    assert!(ui.restore_workspace(&saved)?.is_complete());
    flush(&mut ui);
    assert!(shown(&ui, "b") && !shown(&ui, "a"));

    let path = WorkspacePath::new(["accordion"])?;
    let restore = |ui: &mut Runtime, state: ViewState| -> Vec<RestoreRejection> {
        ui.restore_workspace(
            &Workspace::new(WorkspaceSettings::default(), [(path.clone(), state)]).unwrap(),
        )
        .unwrap()
        .rejected()
        .map(|(_, reason)| reason.clone())
        .collect()
    };
    let two = WorkspaceValue::map([
        ("a".to_owned(), WorkspaceValue::flag(true)),
        ("c".to_owned(), WorkspaceValue::flag(true)),
    ])?;
    assert_eq!(
        restore(&mut ui, ViewState::new("accordion")?.with("open", two)?),
        [RestoreRejection::Value("open".into())]
    );
    let missing = WorkspaceValue::map([("gone".to_owned(), WorkspaceValue::flag(true))])?;
    assert_eq!(
        restore(&mut ui, ViewState::new("accordion")?.with("open", missing)?),
        [RestoreRejection::Unresolved("gone".into())]
    );
    let number = WorkspaceValue::map([("a".to_owned(), WorkspaceValue::integer(1))])?;
    assert_eq!(
        restore(&mut ui, ViewState::new("accordion")?.with("open", number)?),
        [RestoreRejection::Value("open".into())]
    );
    let empty = WorkspaceValue::map(Vec::new())?;
    assert_eq!(
        restore(
            &mut ui,
            ViewState::new("accordion")?
                .with("open", empty)?
                .with("extra", WorkspaceValue::flag(true))?
        ),
        [RestoreRejection::UnknownValue("extra".into())]
    );
    flush(&mut ui);
    assert!(shown(&ui, "b"), "rejected documents changed nothing");
    Ok(())
}

#[test]
fn a_restore_before_mount_opens_sections_before_the_first_update() -> TestResult {
    let mut first = ui();
    first.mount_tree(accordion(ExpandMode::Multiple, &["a", "c"]))?;
    flush(&mut first);
    let saved = first.capture_workspace()?;

    let mut second = ui();
    let _report = second.restore_workspace(&saved)?;
    second.mount_tree(accordion(ExpandMode::Multiple, &[]))?;
    flush(&mut second);
    assert!(shown(&second, "a") && shown(&second, "c") && !shown(&second, "b"));
    Ok(())
}

#[test]
fn a_section_content_keeps_its_own_workspace_address() -> TestResult {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "accordion",
        Accordion::new([Section::new(
            "tabs-section",
            "Tabs",
            Element::new(
                "inner",
                Tabs::new([
                    Tab::new("x", "X", label("x-body")),
                    Tab::new("y", "Y", label("y-body")),
                ]),
            ),
        )])
        .persist(false),
    ))?;
    flush(&mut ui);
    let paths: Vec<String> = ui
        .capture_workspace()?
        .entries()
        .map(|(path, _)| path.to_string())
        .collect();
    assert!(
        paths.contains(&"accordion/tabs-section/inner".to_owned()),
        "{paths:?}"
    );
    assert!(!paths.contains(&"accordion".to_owned()));
    Ok(())
}

#[test]
fn accordion_restore_uses_the_current_child_keys_before_update() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "accordion",
        Accordion::new([section("a", button("a-body"))]),
    ))?;
    flush(&mut ui);
    let open_b = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["accordion"])?,
            ViewState::new("accordion")?.with(
                "open",
                WorkspaceValue::map([("b".to_owned(), WorkspaceValue::flag(true))])?,
            )?,
        )],
    )?;
    ui.reconcile_children(
        &root,
        [
            section("b", button("b-body")),
            section("c", button("c-body")),
        ]
        .into_iter()
        .map(Section::into_element)
        .collect(),
    )?;
    assert!(ui.restore_workspace(&open_b)?.is_complete());
    assert_eq!(
        ui.capture_workspace()?
            .entries()
            .find(|(path, _)| path.to_string() == "accordion")
            .unwrap()
            .1,
        open_b.entries().next().unwrap().1
    );
    flush(&mut ui);
    assert!(shown(&ui, "b-body"));
    assert!(!shown(&ui, "c-body"));
    ui.reconcile_children(&root, vec![section("c", button("c-body")).into_element()])?;
    let before = ui.capture_workspace()?;
    let mixed = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["accordion"])?,
            ViewState::new("accordion")?.with(
                "open",
                WorkspaceValue::map([
                    ("b".into(), WorkspaceValue::flag(true)),
                    ("c".into(), WorkspaceValue::flag(true)),
                ])?,
            )?,
        )],
    )?;
    assert!(
        ui.restore_workspace(&mixed)?
            .rejected()
            .any(|(_, rejection)| *rejection == RestoreRejection::Unresolved("b".into()))
    );
    assert_eq!(ui.capture_workspace()?, before);
    flush(&mut ui);
    assert!(!shown(&ui, "c-body"));
    Ok(())
}

#[test]
fn single_accordion_rejects_multiple_fresh_keys_without_writing() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "accordion",
        Accordion::new([section("a", button("a-body"))]).mode(ExpandMode::Single),
    ))?;
    flush(&mut ui);
    ui.reconcile_children(
        &root,
        [
            section("b", button("b-body")),
            section("c", button("c-body")),
        ]
        .into_iter()
        .map(Section::into_element)
        .collect(),
    )?;
    let before = ui.capture_workspace()?;
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["accordion"])?,
            ViewState::new("accordion")?.with(
                "open",
                WorkspaceValue::map([
                    ("b".into(), WorkspaceValue::flag(true)),
                    ("c".into(), WorkspaceValue::flag(true)),
                ])?,
            )?,
        )],
    )?;
    assert!(
        ui.restore_workspace(&saved)?
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Value("open".into()))
    );
    assert_eq!(ui.capture_workspace()?, before);
    Ok(())
}

#[test]
fn armed_accordion_rejects_restore_to_a_fresh_section() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "accordion",
        Accordion::new([section("a", button("a-body"))]),
    ))?;
    flush(&mut ui);
    ui.focus(&root)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.reconcile_children(
        &root,
        vec![
            section("a", button("a-body")).into_element(),
            section("b", button("b-body")).into_element(),
        ],
    )?;
    let before = ui.capture_workspace()?;
    let saved = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["accordion"])?,
            ViewState::new("accordion")?.with(
                "open",
                WorkspaceValue::map([("b".into(), WorkspaceValue::flag(true))])?,
            )?,
        )],
    )?;
    assert!(
        ui.restore_workspace(&saved)?
            .rejected()
            .any(|(_, reason)| *reason == RestoreRejection::Busy)
    );
    assert_eq!(ui.capture_workspace()?, before);
    Ok(())
}

#[test]
fn expansion_binding_handoff_cancels_keyboard_and_pointer_activation() {
    for mouse in [false, true] {
        let mut ui = ui();
        let old = ui.signal(false);
        let new = ui.signal(false);
        let root = ui
            .mount_tree(Element::new("root", Column::default()))
            .unwrap();
        let describe = |value| {
            vec![Element::new(
                "section",
                Collapsible::new("Title", label("body")).bind_expanded(value),
            )]
        };
        ui.reconcile_children(&root, describe(old.clone())).unwrap();
        flush(&mut ui);
        let section = find(&ui, &root, "section");
        ui.focus(&section).unwrap();
        if mouse {
            pointer(
                &mut ui,
                10.0,
                10.0,
                PointerKind::Press(PointerButton::Primary),
            );
        } else {
            ui.key_input(KeyCode::Space, KeyAction::Press).unwrap();
        }
        ui.reconcile_children(&root, describe(new.clone())).unwrap();
        if mouse {
            pointer(
                &mut ui,
                10.0,
                10.0,
                PointerKind::Release(PointerButton::Primary),
            );
        } else {
            ui.key_input(KeyCode::Space, KeyAction::Release).unwrap();
        }
        flush(&mut ui);
        assert!(!old.get().unwrap());
        assert!(!new.get().unwrap());
        ui.focus(&section).unwrap();
        press(&mut ui, KeyCode::Space);
        assert!(new.get().unwrap());
    }
}

#[test]
fn accordion_reports_origins_and_keeps_accepted_state_after_callback_failure() -> TestResult {
    use std::{cell::RefCell, rc::Rc};
    let mut ui = ui();
    let log = Rc::new(RefCell::new(Vec::new()));
    let fail = ui.signal(false);
    let mirror = ui.signal(Vec::<String>::new());
    let describe = |single| {
        let log = log.clone();
        let fail = fail.clone();
        let mirror = mirror.clone();
        vec![Element::new(
            "accordion",
            Accordion::new([section("a", label("a-body")), section("b", label("b-body"))])
                .mode(if single {
                    ExpandMode::Single
                } else {
                    ExpandMode::Multiple
                })
                .on_change(move |_, change| {
                    log.borrow_mut()
                        .push((change.expanded.clone(), change.origin));
                    mirror.set(change.expanded.clone())?;
                    if fail.get()? {
                        Err(RuntimeError::EmptyWindowTitle)
                    } else {
                        Ok(())
                    }
                }),
        )]
    };
    let root = ui.mount_tree(Element::new("root", Column::default()))?;
    ui.reconcile_children(&root, describe(false))?;
    flush(&mut ui);
    let accordion = find(&ui, &root, "accordion");
    assert!(log.borrow().is_empty());
    let closed = ui.capture_workspace()?;
    ui.reveal_child(&accordion, "a")?;
    ui.focus(&accordion)?;
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Space);
    assert_eq!(*mirror.get()?, ["a", "b"]);
    ui.reconcile_children(&root, describe(true))?;
    flush(&mut ui);
    assert!(ui.restore_workspace(&closed)?.is_complete());
    flush(&mut ui);
    assert_eq!(
        log.borrow()
            .iter()
            .map(|(_, origin)| *origin)
            .collect::<Vec<_>>(),
        [
            ChangeOrigin::Reveal,
            ChangeOrigin::User,
            ChangeOrigin::Redescription,
            ChangeOrigin::Restore
        ]
    );
    fail.set(true)?;
    assert!(ui.reveal_child(&accordion, "b").is_err());
    assert!(shown(&ui, "b-body"));
    let count = log.borrow().len();
    fail.set(false)?;
    flush(&mut ui);
    assert_eq!(log.borrow().len(), count, "failure is not replayed");
    Ok(())
}

#[test]
fn accordion_initial_restore_is_silent_and_callback_removal_discards_notifications() -> TestResult {
    let mut ui = ui();
    let mut accordion = Accordion::new([section("a", label("body"))])
        .on_change(|_, _| panic!("initial restore must be silent"));
    let state = ViewState::new("accordion")?.with(
        "open",
        WorkspaceValue::map([("a".to_owned(), WorkspaceValue::flag(true))])?,
    )?;
    accordion.restore_workspace_state(&state)?;
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("accordion", accordion)]),
    ))?;
    flush(&mut ui);
    assert!(shown(&ui, "body"));
    let closed = Workspace::new(
        WorkspaceSettings::default(),
        [(
            WorkspacePath::new(["root", "accordion"])?,
            ViewState::new("accordion")?.with("open", WorkspaceValue::map([])?)?,
        )],
    )?;
    assert!(ui.restore_workspace(&closed)?.is_complete());
    // Restore has queued a change but has not delivered it: remove the handler.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "accordion",
            Accordion::new([section("a", label("body"))]),
        )],
    )?;
    flush(&mut ui);
    assert!(!shown(&ui, "body"));
    let control = find(&ui, &root, "accordion");
    ui.focus(&control)?;
    press(&mut ui, KeyCode::Space);
    assert!(shown(&ui, "body"));
    Ok(())
}

#[test]
fn invalid_sections_are_rejected_before_the_accordion_mounts_or_changes() -> TestResult {
    let cases: [(Vec<Section>, LayoutError); 3] = [
        (
            vec![Section::new("a", " ", button("a-button"))],
            LayoutError::BlankLabel,
        ),
        (
            vec![Section::new(" ", "A", button("a-button"))],
            LayoutError::BlankKey,
        ),
        (
            vec![
                Section::new("a", "A", button("a-button")),
                Section::new("a", "B", button("b-button")),
            ],
            LayoutError::DuplicateKey,
        ),
    ];
    for (sections, expected) in cases {
        let mut ui = ui();
        assert_eq!(
            ui.mount_tree(Element::new("accordion", Accordion::new(sections)))
                .err(),
            Some(RuntimeError::Layout(expected))
        );
        assert!(ui.inspect().components.is_empty());
    }
    // A raw child without a label is only seen by the update that reads it.
    let mut raw = ui();
    raw.mount_tree(Element::new("accordion", Accordion::new([])).children([button("raw")]))?;
    assert_eq!(
        raw.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );

    let mut runtime = ui();
    let root = runtime.mount_tree(Element::new(
        "root",
        Column::new([accordion(ExpandMode::Multiple, &["b"])]),
    ))?;
    flush(&mut runtime);
    let accordion_id = find(&runtime, &root, "accordion");
    let children = runtime.children(&accordion_id)?;
    assert!(shown(&runtime, "b-button"));
    let invalid = Element::new(
        "accordion",
        Accordion::new([
            section("a", button("a-button")),
            Section::new("b", "  ", button("b-button")),
        ]),
    );
    assert_eq!(
        runtime.reconcile_children(&root, vec![invalid]),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    flush(&mut runtime);
    assert_eq!(runtime.children(&accordion_id)?, children);
    assert!(shown(&runtime, "b-button"));
    assert!(!shown(&runtime, "a-button"));
    Ok(())
}

type TestResult2 = Result<Vec<DrawCommand>, Box<dyn std::error::Error>>;

#[test]
fn the_collapsible_icon_slot_replaces_and_keeps_the_plain_name() -> TestResult {
    let paint = |section: Collapsible| -> TestResult2 {
        let mut ui = ui();
        ui.mount_tree(Element::new("section", section))?;
        flush(&mut ui);
        let commands = ui.paint()?.commands().to_vec();
        assert_eq!(
            details(&ui, "section")
                .semantics
                .and_then(|semantics| semantics.name),
            Some("Details".to_owned())
        );
        Ok(commands)
    };
    let replaced = paint(
        Collapsible::new("Details", button("body"))
            .leading_icon(Icon::Settings)
            .leading_icon(Icon::Folder),
    )?;
    let composed = paint(Collapsible::new(
        RichText::from("Details").leading_icon(Icon::Folder),
        button("body"),
    ))?;
    let accumulated = paint(Collapsible::new(
        RichText::from("Details")
            .leading_icon(Icon::Settings)
            .leading_icon(Icon::Folder),
        button("body"),
    ))?;
    assert_eq!(replaced, composed);
    assert_ne!(replaced, accumulated);
    Ok(())
}

#[test]
fn a_blank_title_is_rejected_before_the_section_mounts_or_changes() -> TestResult {
    for title in ["", " ", "\t\n"] {
        let mut ui = ui();
        assert_eq!(
            ui.mount_tree(Element::new(
                "section",
                Collapsible::new(title, button("body"))
            ))
            .err(),
            Some(RuntimeError::Layout(LayoutError::BlankLabel))
        );
        assert!(ui.inspect().components.is_empty());
    }
    let mut icon_only = ui();
    assert_eq!(
        icon_only
            .mount_tree(Element::new(
                "section",
                Collapsible::new(RichText::new().icon(Icon::Folder), button("body"))
            ))
            .err(),
        Some(RuntimeError::Layout(LayoutError::BlankLabel))
    );

    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "section",
            Collapsible::new("Details", button("body")).expanded(true),
        )]),
    ))?;
    flush(&mut ui);
    let section = find(&ui, &root, "section");
    let children = ui.children(&section)?;
    assert!(shown(&ui, "body"));
    assert_eq!(
        ui.reconcile_children(
            &root,
            vec![Element::new(
                "section",
                Collapsible::new("  ", button("other"))
            )],
        ),
        Err(RuntimeError::Layout(LayoutError::BlankLabel))
    );
    flush(&mut ui);
    assert_eq!(ui.children(&section)?, children);
    assert!(shown(&ui, "body"));
    let name = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "section")
        .and_then(|component| component.details.semantics)
        .and_then(|semantics| semantics.name);
    assert_eq!(name.as_deref(), Some("Details"));
    Ok(())
}
