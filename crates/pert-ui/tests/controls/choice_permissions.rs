//! Choice availability, parameter restrictions and retained popup cancellation.
use crate::support;
use pert_ui::*;
use std::time::Duration;
use support::{find, press};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn dropdown(
    ui: &Runtime,
    value: Signal<i32>,
    inline: bool,
    permission: Permission,
    locked: bool,
) -> Element {
    let choices = [Choice::new(1, "One"), Choice::new(2, "Two")];
    macro_rules! build {
        ($ty:ident) => {{
            let control = $ty::new(ui, value, choices)?.enabled(permission);
            Element::new(
                "dropdown",
                if locked {
                    control.disabled().enabled(true)
                } else {
                    control
                },
            )
        }};
    }
    // Fixed choices in this fixture are known unique.
    let build = || -> Result<Element, ChoiceError> {
        Ok(if inline {
            build!(InlineSelect)
        } else {
            build!(Select)
        })
    };
    build().unwrap()
}

#[test]
fn bound_dropdown_denial_cancels_open_list_press_and_recovers() -> TestResult {
    for inline in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let value = ui.signal(1);
        let allowed = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([dropdown(
                &ui,
                value.clone(),
                inline,
                allowed.read_only().into(),
                false,
            )]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let anchor = find(&ui, &root, "select").unwrap();
        ui.focus(&anchor)?;
        press(&mut ui, KeyCode::ArrowDown);
        let options = find(&ui, &root, "options").unwrap();
        let bounds = ui.bounds(&options)?.unwrap();
        let point = Point::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() - 4.0,
        )?;
        let down = PointerEvent::new(point, PointerKind::Press(PointerButton::Primary));
        let up = PointerEvent::new(point, PointerKind::Release(PointerButton::Primary));
        ui.pointer_input(down)?;
        allowed.set(false)?;
        ui.flush(Duration::ZERO)?;
        let snapshot = ui.inspect();
        let list = snapshot
            .components
            .iter()
            .find(|c| c.key == "options")
            .unwrap();
        assert!(list.visible);
        assert!(list.details.semantics.as_ref().unwrap().disabled);
        press(&mut ui, KeyCode::Enter);
        assert_eq!(value.get()?, 1);
        allowed.set(true)?;
        ui.flush(Duration::ZERO)?;
        ui.pointer_input(up)?;
        assert_eq!(value.get()?, 1, "revoked press must not resume");
        ui.pointer_input(down)?;
        ui.pointer_input(up)?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(value.get()?, 2, "the point must select on a fresh press");
        ui.focus(&anchor)?;
        press(&mut ui, KeyCode::ArrowDown);
        allowed.set(false)?;
        ui.flush(Duration::ZERO)?;
        press(&mut ui, KeyCode::Escape);
        assert!(
            !ui.inspect()
                .components
                .iter()
                .any(|c| c.key == "options" && c.visible)
        );
    }
    Ok(())
}

#[test]
fn dropdown_permissions_replace_unsubscribe_and_keep_fixed_denial() -> TestResult {
    for inline in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let value = ui.signal(1);
        let old = ui.signal(true);
        let next = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([dropdown(
                &ui,
                value.clone(),
                inline,
                old.read_only().into(),
                true,
            )]),
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.focus_next(FocusDirection::Forward)?;
        assert!(ui.focused().is_none());
        ui.reconcile_children(
            &root,
            vec![dropdown(
                &ui,
                value.clone(),
                inline,
                old.read_only().into(),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        let anchor = find(&ui, &root, "select").unwrap();
        ui.focus(&anchor)?;
        press(&mut ui, KeyCode::ArrowDown);
        ui.reconcile_children(
            &root,
            vec![dropdown(
                &ui,
                value.clone(),
                inline,
                next.read_only().into(),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        old.set(false)?;
        assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
        next.set(false)?;
        ui.flush(Duration::ZERO)?;
        assert!(
            ui.inspect()
                .components
                .iter()
                .filter(|c| c.key == "options" || c.key == "select")
                .all(|c| c.details.semantics.as_ref().unwrap().disabled)
        );
        press(&mut ui, KeyCode::Escape);
        let foreign = Runtime::new();
        ui.reconcile_children(
            &root,
            vec![dropdown(
                &ui,
                value,
                inline,
                foreign.signal(true).read_only().into(),
                false,
            )],
        )?;
        assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime));
    }
    Ok(())
}

#[test]
fn application_permission_cannot_override_read_only_choice_parameters() -> TestResult {
    for kind in 0..5 {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let one = ChoiceId::new("one")?;
        let two = ChoiceId::new("two")?;
        let spec = || {
            ParameterSpec::choices(
                "Choice",
                [
                    ChoiceOption::new(one.clone(), "One"),
                    ChoiceOption::new(two.clone(), "Two"),
                ],
                one.clone(),
            )
        };
        let (value, host) = ui.parameter(ParameterId::new(1, 1, "choice")?, spec()?.read_only())?;
        let element = match kind {
            0 => Element::new(
                "control",
                RadioGroup::for_parameter(value.clone()).enabled(true),
            ),
            1 => Element::new(
                "control",
                SegmentedControl::for_parameter(value.clone()).enabled(true),
            ),
            2 => Element::new(
                "control",
                InlineChoice::for_parameter(value.clone()).enabled(true),
            ),
            3 => Element::new(
                "control",
                Select::for_parameter(&ui, value.clone()).enabled(true),
            ),
            _ => Element::new(
                "control",
                InlineSelect::for_parameter(&ui, value.clone()).enabled(true),
            ),
        };
        let root = ui.mount_tree(Element::new("root", Column::new([element])))?;
        ui.flush(Duration::ZERO)?;
        let target = if kind < 3 {
            find(&ui, &root, "control").unwrap()
        } else {
            find(&ui, &root, "select").unwrap()
        };
        ui.focus_next(FocusDirection::Forward)?;
        assert!(ui.focused().is_none());
        ui.focus(&target)?;
        press(&mut ui, KeyCode::ArrowDown);
        press(&mut ui, KeyCode::Enter);
        assert_eq!(value.snapshot().get()?.base, one);
        host.replace_spec(spec()?)?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&target)?;
        press(&mut ui, KeyCode::ArrowDown);
        if kind >= 3 {
            press(&mut ui, KeyCode::ArrowDown);
            press(&mut ui, KeyCode::Enter);
        }
        assert_eq!(value.snapshot().get()?.base, two, "kind {kind}");
    }
    Ok(())
}

#[test]
fn dropdown_anchor_revocation_cancels_held_keys_and_fixed_redescription() -> TestResult {
    for inline in [false, true] {
        for fixed in [false, true] {
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
            let value = ui.signal(1);
            let allowed = ui.signal(true);
            let root = ui.mount_tree(Element::new(
                "root",
                Column::new([dropdown(
                    &ui,
                    value.clone(),
                    inline,
                    allowed.read_only().into(),
                    false,
                )]),
            ))?;
            ui.flush(Duration::ZERO)?;
            let anchor = find(&ui, &root, "select").unwrap();
            ui.focus(&anchor)?;
            ui.key_input(KeyCode::Space, KeyAction::Press)?;
            if fixed {
                ui.reconcile_children(
                    &root,
                    vec![dropdown(
                        &ui,
                        value.clone(),
                        inline,
                        Permission::allowed(),
                        true,
                    )],
                )?;
                ui.reconcile_children(
                    &root,
                    vec![dropdown(&ui, value, inline, Permission::allowed(), false)],
                )?;
            } else {
                allowed.set(false)?;
                ui.flush(Duration::ZERO)?;
                allowed.set(true)?;
            }
            ui.flush(Duration::ZERO)?;
            ui.focus(&anchor)?;
            ui.key_input(KeyCode::Space, KeyAction::Release)?;
            ui.flush(Duration::ZERO)?;
            assert!(
                !ui.inspect()
                    .components
                    .iter()
                    .any(|c| c.key == "options" && c.visible)
            );
            press(&mut ui, KeyCode::Space);
            assert!(
                ui.inspect()
                    .components
                    .iter()
                    .any(|c| c.key == "options" && c.visible)
            );
        }
    }
    Ok(())
}

#[test]
fn multi_select_rejects_foreign_permission_but_fixed_denial_is_final() -> TestResult {
    for locked in [false, true] {
        let mut ui = Runtime::new();
        let foreign = Runtime::new();
        let value = ui.signal(vec![1]);
        let control = MultiSelect::new(&ui, value, [Choice::new(1, "One")])?;
        let control = if locked { control.disabled() } else { control };
        ui.mount_tree(Element::new(
            "choice",
            control
                .enabled(foreign.signal(true).read_only())
                .enabled(true),
        ))?;
        let result = ui.flush(Duration::ZERO);
        if locked {
            result?;
            ui.focus_next(FocusDirection::Forward)?;
            assert!(ui.focused().is_none());
        } else {
            assert_eq!(result, Err(RuntimeError::ForeignRuntime));
        }
    }
    Ok(())
}

#[test]
fn permission_source_replacement_preserves_dropdown_highlight() -> TestResult {
    for inline in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let value = ui.signal(1);
        let old = ui.signal(true);
        let next = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([dropdown(
                &ui,
                value.clone(),
                inline,
                old.read_only().into(),
                false,
            )]),
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.focus(&find(&ui, &root, "select").unwrap())?;
        press(&mut ui, KeyCode::ArrowDown);
        press(&mut ui, KeyCode::End);
        ui.reconcile_children(
            &root,
            vec![dropdown(
                &ui,
                value.clone(),
                inline,
                next.read_only().into(),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        press(&mut ui, KeyCode::Enter);
        assert_eq!(
            value.get()?,
            2,
            "permission replacement must preserve highlighted choice"
        );
    }
    Ok(())
}
