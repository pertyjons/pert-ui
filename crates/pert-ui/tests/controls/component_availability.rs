//! Reactive availability through the public facade, independent of control presentation.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Button,
    Link,
    Checkbox,
    Toggle,
    InlineToggle,
    Chip,
    RadioGroup,
    SegmentedControl,
    InlineChoice,
}

const KINDS: &[Kind] = &[
    Kind::Button,
    Kind::Link,
    Kind::Checkbox,
    Kind::Toggle,
    Kind::InlineToggle,
    Kind::Chip,
    Kind::RadioGroup,
    Kind::SegmentedControl,
    Kind::InlineChoice,
];

fn control(kind: Kind, value: Signal<bool>, permission: Permission, locked: bool) -> Element {
    macro_rules! configure {
        ($control:expr) => {{
            let control = $control;
            let control = if locked { control.disabled() } else { control };
            Element::new("control", control.enabled(permission))
        }};
    }
    match kind {
        Kind::RadioGroup => configure!(RadioGroup::new(value, [Choice::new(true, "On")]).unwrap()),
        Kind::SegmentedControl => {
            configure!(SegmentedControl::new(value, [Choice::new(true, "On")]).unwrap())
        }
        Kind::InlineChoice => {
            configure!(InlineChoice::new(value, [Choice::new(true, "On")]).unwrap())
        }
        Kind::Button => configure!(Button::new("Action", move |_| value.set(true))),
        Kind::Link => configure!(Link::new("Action", move |_| value.set(true))),
        Kind::Checkbox => configure!(Checkbox::new(value, "Value")),
        Kind::Toggle => configure!(Toggle::new(value, "Value")),
        Kind::InlineToggle => configure!(InlineToggle::new(value, "Value")),
        Kind::Chip => configure!(Chip::removable("Item", move |_| value
            .set(true)
            .map(|_| ()))),
    }
}

fn activate(ui: &mut Runtime, kind: Kind) -> Result<(), RuntimeError> {
    let code = if matches!(kind, Kind::Chip) {
        KeyCode::Delete
    } else {
        KeyCode::Space
    };
    ui.command_key_input(CommandKeyInput::new(
        code,
        KeyAction::Press,
        CommandKey::Named(code),
    ))?;
    ui.command_key_input(CommandKeyInput::new(
        code,
        KeyAction::Release,
        CommandKey::Named(code),
    ))?;
    Ok(())
}

#[test]
fn fixed_denial_survives_bound_permission_and_redescription_can_replace_it() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let allowed = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([control(
                kind,
                value.clone(),
                allowed.read_only().into(),
                true,
            )]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let target = ui.children(&root)?[0].clone();
        ui.focus_next(FocusDirection::Forward)?;
        assert_ne!(ui.focused(), Some(target.clone()), "{kind:?}");
        // Explicit focus can target non-tab-stops; prove input is refused there too.
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(!value.get()?, "{kind:?}");
        ui.reconcile_children(
            &root,
            vec![control(
                kind,
                value.clone(),
                allowed.read_only().into(),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(ui.children(&root)?[0], target);
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(value.get()?, "{kind:?}");
    }
    Ok(())
}

#[test]
fn revocation_cancels_armed_activation_and_a_new_press_recovers() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let allowed = ui.signal(true);
        let target = ui.mount_tree(control(
            kind,
            value.clone(),
            allowed.read_only().into(),
            false,
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&target)?;
        // Chip removes on fresh Delete, so exercise its armed pointer instead.
        let pointer = if matches!(kind, Kind::Chip) {
            let bounds = ui.bounds(&target)?.unwrap();
            Some(Point::new(bounds.width() - 8.0, bounds.height() / 2.0)?)
        } else {
            None
        };
        if let Some(point) = pointer {
            ui.pointer_input(PointerEvent::new(
                point,
                PointerKind::Press(PointerButton::Primary),
            ))?;
        } else {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Space,
                KeyAction::Press,
                CommandKey::Named(KeyCode::Space),
            ))?;
        }
        allowed.set(false)?;
        ui.flush(Duration::ZERO)?;
        assert_ne!(ui.focused(), Some(target.clone()), "{kind:?}");
        allowed.set(true)?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&target)?;
        if let Some(point) = pointer {
            ui.pointer_input(PointerEvent::new(
                point,
                PointerKind::Release(PointerButton::Primary),
            ))?;
        } else {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::Space,
                KeyAction::Release,
                CommandKey::Named(KeyCode::Space),
            ))?;
        }
        assert!(!value.get()?, "{kind:?}");
        activate(&mut ui, kind)?;
        assert!(value.get()?, "{kind:?}");
    }
    Ok(())
}

#[test]
fn availability_rejects_a_foreign_runtime_binding() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let foreign = Runtime::new();
        let allowed = foreign.signal(true);
        let value = ui.signal(false);
        ui.mount_tree(control(kind, value, allowed.read_only().into(), false))?;
        assert!(
            matches!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime)),
            "{kind:?}"
        );
    }
    Ok(())
}

#[test]
fn replacement_unsubscribes_old_permissions_and_hidden_controls_refresh_on_reveal() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let old = ui.signal(true);
        let next = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([control(kind, value.clone(), old.read_only().into(), false)]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let target = ui.children(&root)?[0].clone();
        ui.reconcile_children(
            &root,
            vec![control(kind, value.clone(), next.read_only().into(), false)],
        )?;
        ui.flush(Duration::ZERO)?;
        old.set(false)?;
        assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0, "{kind:?}");
        ui.set_visible(&root, false)?;
        ui.flush(Duration::ZERO)?;
        next.set(false)?;
        assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0, "{kind:?}");
        ui.set_visible(&root, true)?;
        ui.flush(Duration::ZERO)?;
        ui.focus_next(FocusDirection::Forward)?;
        assert_ne!(ui.focused(), Some(target.clone()), "{kind:?}");
        next.set(true)?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(value.get()?, "{kind:?}");
    }
    Ok(())
}

#[test]
fn application_permission_cannot_override_parameter_read_only() -> TestResult {
    for kind in [Kind::Checkbox, Kind::Toggle, Kind::InlineToggle] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let (value, host) = ui.parameter(
            ParameterId::new(1, 1, "value")?,
            ParameterSpec::boolean("Value", false)?.read_only(),
        )?;
        let element = match kind {
            Kind::Checkbox => Element::new(
                "control",
                Checkbox::for_parameter(value.clone()).enabled(true),
            ),
            Kind::Toggle => Element::new(
                "control",
                Toggle::for_parameter(value.clone()).enabled(true),
            ),
            Kind::InlineToggle => Element::new(
                "control",
                InlineToggle::for_parameter(value.clone()).enabled(true),
            ),
            _ => unreachable!(),
        };
        let target = ui.mount_tree(element)?;
        ui.flush(Duration::ZERO)?;
        ui.focus_next(FocusDirection::Forward)?;
        assert_ne!(ui.focused(), Some(target.clone()));
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(!value.snapshot().get()?.base);
        host.replace_spec(ParameterSpec::boolean("Value", false)?)?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(value.snapshot().get()?.base);
    }
    Ok(())
}

#[test]
fn fixed_disabled_descriptions_are_inert_before_the_first_flush() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        let value = ui.signal(false);
        ui.mount_tree(control(kind, value, Permission::allowed(), true))?;
        let entry = ui
            .inspect()
            .components
            .into_iter()
            .find(|entry| entry.key == "control")
            .unwrap();
        assert!(!entry.details.validation.interactive, "{kind:?}");
        assert!(
            entry.details.semantics.unwrap().actions.is_empty(),
            "{kind:?}"
        );
    }
    Ok(())
}

#[test]
fn redescribed_fixed_denials_skip_controls_without_an_explicit_flush() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([control(kind, value.clone(), Permission::allowed(), false)]),
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.reconcile_children(
            &root,
            vec![control(kind, value, Permission::allowed(), true)],
        )?;
        let entry = ui
            .inspect()
            .components
            .into_iter()
            .find(|entry| entry.key == "control")
            .unwrap();
        assert!(!entry.details.validation.interactive, "{kind:?}");
        ui.focus_next(FocusDirection::Forward)?;
        assert!(ui.focused().is_none(), "{kind:?}");
    }
    Ok(())
}

#[test]
fn pointer_revocation_cancels_a_proven_hit_for_every_control() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let allowed = ui.signal(true);
        let target = ui.mount_tree(control(
            kind,
            value.clone(),
            allowed.read_only().into(),
            false,
        ))?;
        ui.flush(Duration::ZERO)?;
        let bounds = ui.bounds(&target)?.unwrap();
        let point = if matches!(
            kind,
            Kind::RadioGroup | Kind::SegmentedControl | Kind::InlineChoice
        ) {
            Point::new(8.0, 8.0)?
        } else {
            Point::new(bounds.width() - 8.0, bounds.height() / 2.0)?
        };
        let press = PointerEvent::new(point, PointerKind::Press(PointerButton::Primary));
        let release = PointerEvent::new(point, PointerKind::Release(PointerButton::Primary));
        ui.pointer_input(press)?;
        ui.pointer_input(release)?;
        assert!(value.get()?, "the point must actually activate {kind:?}");
        value.set(false)?;
        ui.pointer_input(press)?;
        allowed.set(false)?;
        ui.flush(Duration::ZERO)?;
        allowed.set(true)?;
        ui.flush(Duration::ZERO)?;
        ui.pointer_input(release)?;
        assert!(!value.get()?, "revoked {kind:?}");
    }
    Ok(())
}

#[test]
fn fixed_redescription_cancels_a_held_press_even_if_reenabled_before_flush() -> TestResult {
    for &kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([control(kind, value.clone(), Permission::allowed(), false)]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let target = ui.children(&root)?[0].clone();
        ui.focus(&target)?;
        let bounds = ui.bounds(&target)?.unwrap();
        let [x, y, width, height] = bounds.values();
        let point = if matches!(
            kind,
            Kind::RadioGroup | Kind::SegmentedControl | Kind::InlineChoice
        ) {
            Point::new(x + 8.0, y + 8.0)?
        } else {
            Point::new(x + width - 8.0, y + height / 2.0)?
        };
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Press(PointerButton::Primary),
        ))?;
        ui.reconcile_children(
            &root,
            vec![control(kind, value.clone(), Permission::allowed(), true)],
        )?;
        ui.reconcile_children(
            &root,
            vec![control(kind, value.clone(), Permission::allowed(), false)],
        )?;
        ui.flush(Duration::ZERO)?;
        ui.pointer_input(PointerEvent::new(
            point,
            PointerKind::Release(PointerButton::Primary),
        ))?;
        assert!(!value.get()?, "{kind:?}");
        ui.focus(&target)?;
        activate(&mut ui, kind)?;
        assert!(value.get()?, "{kind:?}");
    }
    Ok(())
}
