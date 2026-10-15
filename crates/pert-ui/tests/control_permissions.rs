//! Reactive permissions of the parameter controls, color controls and
//! collapsible headers: revoking a bound permission mid-gesture cancels the
//! gesture without committing and releases focus, granting it again replays
//! nothing, a fixed denial is never lifted by a binding, same-key
//! redescription replaces the permission, and a foreign binding fails the
//! update.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 600.0, 1.0)?)?;
    Ok(ui)
}

fn settle(ui: &mut Runtime) {
    ui.flush(ui.now()).unwrap();
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key).unwrap().unwrap()
}

fn disabled(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.semantics)
        .unwrap_or_else(|| panic!("no semantics for {key}"))
        .disabled
}

fn center(ui: &Runtime, id: &WidgetId, x: f32, y: f32) -> Point {
    let bounds = ui.bounds(id).unwrap().unwrap();
    Point::new(
        bounds.x() + bounds.width() * x,
        bounds.y() + bounds.height() * y,
    )
    .unwrap()
}

fn gain(
    ui: &mut Runtime,
) -> Result<(Parameter<f32>, ParameterHost<f32>), Box<dyn std::error::Error>> {
    Ok(ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 100.0, 50.0)?,
    )?)
}

/// Mounts `control` above a button and returns the root and the control.
fn mount(
    ui: &mut Runtime,
    control: Element,
) -> Result<(WidgetId, WidgetId), Box<dyn std::error::Error>> {
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            control,
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ]),
    ))?;
    settle(ui);
    let control = ui.children(&root)?[0].clone();
    Ok((root, control))
}

#[test]
fn revoking_a_slider_mid_gesture_cancels_without_commit_and_releases_focus() -> TestResult {
    for kind in ["slider", "knob", "inline"] {
        let mut ui = runtime()?;
        let (parameter, _host) = gain(&mut ui)?;
        let allowed = ui.signal(true);
        let control = match kind {
            "slider" => Element::new(
                "control",
                Slider::new(parameter.clone()).enabled(allowed.read_only()),
            ),
            "knob" => Element::new(
                "control",
                Knob::new(parameter.clone()).enabled(allowed.read_only()),
            ),
            _ => Element::new(
                "control",
                InlineSlider::new(parameter.clone()).enabled(allowed.read_only()),
            ),
        };
        let (_, control) = mount(&mut ui, control)?;
        ui.focus(&control)?;
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
        assert!(
            parameter.snapshot().get()?.preview.is_some(),
            "{kind}: previewing"
        );
        allowed.set(false)?;
        settle(&mut ui);
        ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
        settle(&mut ui);
        let snapshot = parameter.snapshot().get()?;
        assert_eq!(snapshot.base, 50.0, "{kind}: nothing committed");
        assert_eq!(snapshot.preview, None, "{kind}: the preview ended");
        assert_ne!(
            ui.focused(),
            Some(control.clone()),
            "{kind}: focus released"
        );
        assert!(disabled(&ui, "control"), "{kind}");

        // Granting it again replays nothing; a new gesture commits.
        allowed.set(true)?;
        settle(&mut ui);
        assert_eq!(parameter.snapshot().get()?.base, 50.0, "{kind}: no replay");
        ui.focus(&control)?;
        ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
        ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
        settle(&mut ui);
        assert!(
            parameter.snapshot().get()?.base > 50.0,
            "{kind}: editable again"
        );
    }
    Ok(())
}

#[test]
fn a_fixed_denial_is_not_lifted_and_redescription_replaces_the_permission() -> TestResult {
    let mut ui = runtime()?;
    let (parameter, _host) = gain(&mut ui)?;
    let allowed = ui.signal(true);
    let (root, _) = mount(
        &mut ui,
        Element::new(
            "control",
            Slider::new(parameter.clone())
                .disabled()
                .enabled(allowed.read_only()),
        ),
    )?;
    assert!(disabled(&ui, "control"), "the fixed denial stays");
    let describe = |slider: Slider| {
        vec![
            Element::new("control", slider),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ]
    };
    ui.reconcile_children(&root, describe(Slider::new(parameter.clone())))?;
    settle(&mut ui);
    assert!(
        !disabled(&ui, "control"),
        "a new description replaces the denial"
    );
    ui.reconcile_children(&root, describe(Slider::new(parameter.clone()).disabled()))?;
    settle(&mut ui);
    assert!(disabled(&ui, "control"));
    Ok(())
}

#[test]
fn a_foreign_permission_fails_the_update() -> TestResult {
    let other = Runtime::new();
    let foreign = other.signal(true);
    for build in [
        |p: Parameter<f32>, f: ReadSignal<bool>| Element::new("c", Slider::new(p).enabled(f)),
        |_: Parameter<f32>, f: ReadSignal<bool>| {
            Element::new(
                "c",
                ColorSwatch::new(Color::WHITE)
                    .on_activate(|_| Ok(()))
                    .enabled(f),
            )
        },
        |_: Parameter<f32>, f: ReadSignal<bool>| {
            Element::new(
                "c",
                Collapsible::new("Title", Element::new("body", Label::new("x"))).enabled(f),
            )
        },
    ] {
        let mut ui = runtime()?;
        let (parameter, _host) = gain(&mut ui)?;
        ui.mount_tree(build(parameter, foreign.read_only()))?;
        assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime));
    }
    let mut ui = runtime()?;
    let color = ui.signal(Color::WHITE);
    ui.mount_tree(Element::new(
        "picker",
        ColorPicker::new(color).enabled(foreign.read_only()),
    ))?;
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::ForeignRuntime));
    Ok(())
}

#[test]
fn revoking_a_swatch_cancels_its_armed_activation_and_releases_focus() -> TestResult {
    let mut ui = runtime()?;
    let allowed = ui.signal(true);
    let activations = Rc::new(Cell::new(0));
    let count = activations.clone();
    let swatch = ColorSwatch::new(Color::WHITE)
        .on_activate(move |_| {
            count.set(count.get() + 1);
            Ok(())
        })
        .enabled(allowed.read_only());
    let (_, swatch) = mount(&mut ui, Element::new("control", swatch))?;
    ui.focus(&swatch)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    allowed.set(false)?;
    settle(&mut ui);
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    settle(&mut ui);
    assert_eq!(activations.get(), 0, "the armed activation was cancelled");
    assert_ne!(ui.focused(), Some(swatch.clone()));
    assert!(disabled(&ui, "control"));
    allowed.set(true)?;
    settle(&mut ui);
    assert_eq!(activations.get(), 0, "nothing replayed");
    ui.focus(&swatch)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    assert_eq!(activations.get(), 1);
    Ok(())
}

#[test]
fn revoking_a_picker_mid_drag_writes_nothing_and_redescription_replaces_it() -> TestResult {
    let mut ui = runtime()?;
    let color = ui.signal(Color::from_hex("#ff0000")?);
    let allowed = ui.signal(true);
    let describe = |permission: Permission| {
        vec![
            Element::new(
                "picker",
                ColorPicker::new(color.clone()).enabled(permission),
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ]
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new(describe(allowed.read_only().into())),
    ))?;
    settle(&mut ui);
    let area = find(&ui, &root, "area");
    let press = center(&ui, &area, 0.5, 0.5);
    ui.pointer_input(PointerEvent::new(
        press,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    let moved = center(&ui, &area, 0.2, 0.8);
    ui.pointer_input(PointerEvent::new(moved, PointerKind::Move))?;
    settle(&mut ui);
    allowed.set(false)?;
    settle(&mut ui);
    ui.pointer_input(PointerEvent::new(
        moved,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    settle(&mut ui);
    assert_eq!(color.get()?.to_hex(), "#ff0000", "the drag wrote nothing");
    assert_ne!(ui.focused(), Some(area.clone()), "focus left the area");
    assert!(disabled(&ui, "area"));
    assert!(disabled(&ui, "picker"));

    // The retained picker takes a new description's permission.
    ui.reconcile_children(&root, describe(Permission::allowed()))?;
    settle(&mut ui);
    assert!(!disabled(&ui, "picker"));
    assert!(!disabled(&ui, "area"));
    ui.focus(&find(&ui, &root, "hue"))?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    settle(&mut ui);
    assert_ne!(color.get()?.to_hex(), "#ff0000", "editable again");
    Ok(())
}

#[test]
fn revoking_a_header_cancels_its_armed_press_and_keeps_expansion() -> TestResult {
    for kind in ["collapsible", "accordion"] {
        let mut ui = runtime()?;
        let allowed = ui.signal(true);
        let element = if kind == "collapsible" {
            Element::new(
                "control",
                Collapsible::new("Inputs", Element::new("body", Label::new("Body")))
                    .enabled(allowed.read_only()),
            )
        } else {
            Element::new(
                "control",
                Accordion::new([Section::new(
                    "inputs",
                    "Inputs",
                    Element::new("body", Label::new("Body")),
                )])
                .enabled(allowed.read_only()),
            )
        };
        let (_, control) = mount(&mut ui, element)?;
        let visible = |ui: &Runtime| {
            ui.inspect()
                .components
                .into_iter()
                .find(|c| c.key == "body")
                .is_some_and(|c| c.visible)
        };
        assert!(!visible(&ui), "{kind}: starts collapsed");
        ui.focus(&control)?;
        ui.key_input(KeyCode::Enter, KeyAction::Press)?;
        allowed.set(false)?;
        settle(&mut ui);
        ui.key_input(KeyCode::Enter, KeyAction::Release)?;
        settle(&mut ui);
        assert!(!visible(&ui), "{kind}: the armed press toggled nothing");
        assert_ne!(
            ui.focused(),
            Some(control.clone()),
            "{kind}: focus released"
        );
        assert!(disabled(&ui, "control"), "{kind}");
        // A pointer press on the header is refused while denied.
        let header = center(&ui, &control, 0.5, 0.05);
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            ui.pointer_input(PointerEvent::new(header, kind))?;
        }
        settle(&mut ui);
        assert!(!visible(&ui), "{kind}: pointer refused");

        allowed.set(true)?;
        settle(&mut ui);
        assert!(!visible(&ui), "{kind}: nothing replayed");
        ui.focus(&control)?;
        ui.key_input(KeyCode::Enter, KeyAction::Press)?;
        ui.key_input(KeyCode::Enter, KeyAction::Release)?;
        settle(&mut ui);
        assert!(visible(&ui), "{kind}: toggles again");
    }
    Ok(())
}
