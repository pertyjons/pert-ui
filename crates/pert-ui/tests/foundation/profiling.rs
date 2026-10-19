//! Public elapsed-time profiling contracts, independent of desktop/GPU features.
use pert_ui::*;
use std::time::Duration;

#[test]
fn input_callbacks_layout_and_parameters_are_attributed_and_pause_is_idle() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 100.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("Initial".to_owned());
    let write = value.clone();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new(
                    "action",
                    Button::new("Action", move |_| write.set("Changed".into())),
                ),
                Element::new("label", Label::new(value.read_only())),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert!(ui.profile().stats.is_empty());
    ui.set_profiling(true).unwrap();
    let button = ui.children(&root).unwrap()[0].clone();
    for kind in [
        PointerKind::Move,
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(20.0, 15.0).unwrap(), kind))
            .unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    assert_eq!(value.get().unwrap(), "Changed");
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(20, 1, "profiled").unwrap(),
            ParameterSpec::new("Value", 0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap();
    ui.try_event(&button, |cx| {
        let edit = parameter.begin_edit(cx, parameter.snapshot().get()?.revision)?;
        edit.update(0.5)?;
        edit.commit(0.5)
    })
    .unwrap()
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let profile = ui.profile();
    for operation in [
        ProfileOperation::PointerMove,
        ProfileOperation::PointerButton,
        ProfileOperation::Focus,
        ProfileOperation::Update,
        ProfileOperation::Measure,
        ProfileOperation::Arrange,
        ProfileOperation::Paint,
        ProfileOperation::Text,
        ProfileOperation::Application,
        ProfileOperation::EditBegin,
        ProfileOperation::EditPreview,
        ProfileOperation::EditCommit,
    ] {
        assert!(
            profile.stats.iter().any(|s| s.operation == operation),
            "{operation:?}"
        );
    }
    assert!(
        profile
            .stats
            .iter()
            .filter(|s| s.component.is_some())
            .all(|s| !s.key.is_empty())
    );
    assert!(
        profile
            .stats
            .iter()
            .all(|s| s.self_time <= s.total && s.maximum <= s.total)
    );
    ui.set_profiling(false).unwrap();
    let paused = ui.profile();
    ui.focus(&button).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.profile(), paused);
    assert!(ui.next_deadline().is_none());
    fn owned<T: Send + Sync>(_: &T) {}
    owned(&profile);
}
#[test]
fn rejected_foreign_ids_are_never_attributed_to_same_numbered_local_components() {
    let mut local = Runtime::new();
    let mut foreign = Runtime::new();
    local
        .mount_tree(Element::new("local", Label::new("Local")))
        .unwrap();
    let other = foreign
        .mount_tree(Element::new("foreign", Label::new("Foreign")))
        .unwrap();
    local.set_profiling(true).unwrap();
    assert_eq!(
        local.event(&other, |_| ()),
        Err(RuntimeError::ForeignRuntime)
    );
    assert!(local.profile().stats.iter().all(|s| s.component.is_none()));
}
#[test]
fn event_controls_can_clear_and_pause_the_current_scope() {
    let mut ui = Runtime::new();
    let id = ui
        .mount_tree(Element::new("label", Label::new("Content")))
        .unwrap();
    ui.set_profiling(true).unwrap();
    ui.event(&id, |cx| {
        cx.clear_profile().unwrap();
        cx.set_profiling(false).unwrap();
    })
    .unwrap();
    let profile = ui.profile();
    assert!(!profile.enabled);
    assert!(profile.stats.is_empty() && profile.recent.is_empty());
}
