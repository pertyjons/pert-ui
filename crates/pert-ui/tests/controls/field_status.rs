//! The field status contract through the public facade: presentation of label,
//! help and status, form-level summary, and focus on the first field in error.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::find;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
fn role(ui: &Runtime, role: ColorRole) -> Color {
    let theme = ui.theme();
    theme.palette(theme.mode().unwrap()).unwrap().color(role)
}
fn colored(scene: &DisplayList, color: Color) -> Vec<Rect> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color: c }
            | DrawCommand::DrawMask { rect, color: c, .. }
                if *c == color =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect()
}
fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.properties.get(name).cloned())
        .unwrap()
}

#[test]
fn a_field_presents_label_help_and_status_around_any_control() {
    let mut ui = ui();
    let text = ui.signal(String::new());
    let status = ui.signal(FieldStatus::Normal);
    let root = ui
        .mount_tree(Element::new(
            "field",
            Field::new(Element::new("name", TextField::new(text)))
                .label("Name")
                .help("Shown on the title page")
                .status(status.read_only()),
        ))
        .unwrap();
    let scene = draw(&mut ui);
    let control = ui
        .bounds(&find(&ui, &root, "name").unwrap())
        .unwrap()
        .unwrap();
    let field = ui.bounds(&root).unwrap().unwrap();
    // The label sits above the control and the help line below it, in Muted.
    assert!(control.y() > 0.0);
    assert!(control.y() + control.height() < field.height());
    assert_eq!(control.width(), field.width());
    let help = colored(&scene, role(&ui, ColorRole::Muted));
    assert!(
        help.iter()
            .any(|rect| rect.y() >= control.y() + control.height())
    );
    assert!(colored(&scene, role(&ui, ColorRole::Error)).is_empty());
    assert_eq!(
        property(&ui, "field", "status"),
        InspectValue::Text("normal".into())
    );
    assert_eq!(
        property(&ui, "field", "outlined"),
        InspectValue::Bool(false)
    );

    // An error replaces the help line in the Error role and outlines the control.
    // One line replaces one line, so the field keeps its height.
    status
        .set(FieldStatus::error("A name is required"))
        .unwrap();
    let scene = draw(&mut ui);
    assert_eq!(ui.bounds(&root).unwrap().unwrap().height(), field.height());
    let error = colored(&scene, role(&ui, ColorRole::Error));
    assert!(
        error
            .iter()
            .any(|rect| rect.y() >= control.y() + control.height())
    );
    assert!(
        error
            .iter()
            .any(|rect| rect.y() < control.y() + control.height()),
        "the outline marks the control itself"
    );
    assert_eq!(
        property(&ui, "field", "status"),
        InspectValue::Text("error".into())
    );
    assert_eq!(
        property(&ui, "field", "message"),
        InspectValue::Text("A name is required".into())
    );
    assert_eq!(property(&ui, "field", "outlined"), InspectValue::Bool(true));

    // A warning uses its own role, and clearing restores the help text.
    status.set(FieldStatus::warning("Unusually long")).unwrap();
    let scene = draw(&mut ui);
    assert!(colored(&scene, role(&ui, ColorRole::Error)).is_empty());
    assert!(!colored(&scene, role(&ui, ColorRole::Warning)).is_empty());
    status.set(FieldStatus::Normal).unwrap();
    let scene = draw(&mut ui);
    assert!(colored(&scene, role(&ui, ColorRole::Warning)).is_empty());
    assert_eq!(
        property(&ui, "field", "outlined"),
        InspectValue::Bool(false)
    );
    assert_eq!(
        ui.next_deadline(),
        None,
        "status presentation schedules nothing"
    );
}

#[test]
fn a_bare_field_adds_nothing_and_a_message_adds_only_its_own_line() {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::Normal);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "field",
                    Field::new(Element::new("button", Button::new("Plain", |_| Ok(()))))
                        .status(status.read_only()),
                ),
                Element::new("bare", Button::new("Plain", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    draw(&mut ui);
    let height = |ui: &Runtime, key| {
        ui.bounds(&find(ui, &root, key).unwrap())
            .unwrap()
            .unwrap()
            .height()
    };
    // No label, no help and a normal status: the field is exactly its control.
    assert_eq!(height(&ui, "field"), height(&ui, "bare"));
    status.set(FieldStatus::error("Not allowed")).unwrap();
    draw(&mut ui);
    assert!(height(&ui, "field") > height(&ui, "bare"));
    // An empty message marks the control without a line of its own.
    status.set(FieldStatus::error("")).unwrap();
    draw(&mut ui);
    assert_eq!(height(&ui, "field"), height(&ui, "bare"));
    assert_eq!(property(&ui, "field", "outlined"), InspectValue::Bool(true));
}

#[test]
fn hiding_only_the_control_collapses_its_field_without_a_layout_error() {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::error("Not allowed"));
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "field",
                    Field::new(Element::new("button", Button::new("Plain", |_| Ok(()))))
                        .label("Name")
                        .help("Help")
                        .status(status.read_only()),
                ),
                Element::new("after", Button::new("After", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    draw(&mut ui);
    let field = find(&ui, &root, "field").unwrap();
    let after = find(&ui, &root, "after").unwrap();
    let shown = ui.bounds(&after).unwrap().unwrap().y();
    assert!(shown > 0.0);
    // The field stays visible; only its control is hidden. The label, message
    // and outline go with it and the next row moves up.
    ui.set_visible(&find(&ui, &root, "button").unwrap(), false)
        .unwrap();
    let scene = draw(&mut ui);
    assert_eq!(ui.bounds(&field).unwrap().unwrap().height(), 0.0);
    assert!(ui.bounds(&after).unwrap().unwrap().y() < shown);
    assert!(colored(&scene, role(&ui, ColorRole::Error)).is_empty());
    assert_eq!(
        property(&ui, "field", "outlined"),
        InspectValue::Bool(false)
    );
    // Shown again, the field lays out as before.
    ui.set_visible(&find(&ui, &root, "button").unwrap(), true)
        .unwrap();
    draw(&mut ui);
    assert_eq!(ui.bounds(&after).unwrap().unwrap().y(), shown);
    assert_eq!(property(&ui, "field", "outlined"), InspectValue::Bool(true));
}

#[test]
fn a_form_summarizes_in_registration_order_and_rejects_bad_keys() {
    let ui = ui();
    let form = FormFields::new(&ui);
    let first = form.field("first").unwrap();
    let second = form.field("second").unwrap();
    assert_eq!(
        form.field("first").err(),
        Some(RuntimeError::DuplicateKey("first".into()))
    );
    assert_eq!(form.field("").err(), Some(RuntimeError::EmptyKey));
    assert_eq!(first.key(), "first");
    let summary = form.summary();
    assert_eq!(summary.get().unwrap(), FormSummary::default());

    assert!(second.set_changed(FieldStatus::error("bad")).unwrap());
    assert!(
        !second.set_changed(FieldStatus::error("bad")).unwrap(),
        "unchanged"
    );
    assert!(first.set_changed(FieldStatus::warning("odd")).unwrap());
    let now = summary.get().unwrap();
    assert_eq!((now.errors(), now.warnings()), (1, 1));
    assert_eq!(now.first_error(), Some("second"));

    // Registration order, not write order, decides which error is first.
    first.set(FieldStatus::error("worse")).unwrap();
    let now = summary.get().unwrap();
    assert_eq!((now.errors(), now.warnings()), (2, 0));
    assert_eq!(now.first_error(), Some("first"));
    first.clear().unwrap();
    second.clear().unwrap();
    assert_eq!(summary.get().unwrap(), FormSummary::default());
    assert_eq!(second.status().get().unwrap(), FieldStatus::Normal);
}

fn form_tree(ui: &mut Runtime) -> (WidgetId, FormFields, FieldHandle, FieldHandle) {
    let form = FormFields::new(ui);
    let (first, second) = (form.field("first").unwrap(), form.field("second").unwrap());
    let (a, b) = (ui.signal(String::new()), ui.signal(String::new()));
    let submit = form.clone();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "first-field",
                    Field::new(Element::new("first", TextField::new(a))).form(&first),
                ),
                Element::new(
                    "second-field",
                    Field::new(Element::new("second", TextField::new(b))).form(&second),
                ),
                Element::new(
                    "submit",
                    Button::new("Submit", move |cx| submit.focus_first_error(cx).map(|_| ())),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (root, form, first, second)
}
fn press(ui: &mut Runtime, id: &WidgetId) {
    ui.focus(id).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
}

#[test]
fn submitting_moves_focus_to_the_first_field_in_error() {
    let mut ui = ui();
    let (root, _form, first, second) = form_tree(&mut ui);
    let submit = find(&ui, &root, "submit").unwrap();

    // With nothing in error, the request is not made and focus stays put.
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), Some(submit.clone()));

    second.set(FieldStatus::error("bad")).unwrap();
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), find(&ui, &root, "second"));

    first.set(FieldStatus::error("worse")).unwrap();
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), find(&ui, &root, "first"));
}

#[test]
fn an_unmounted_field_is_skipped_and_never_retargets_a_later_control() {
    let mut ui = ui();
    let (root, _form, first, second) = form_tree(&mut ui);
    let submit = find(&ui, &root, "submit").unwrap();
    first.set(FieldStatus::error("worse")).unwrap();
    second.set(FieldStatus::error("bad")).unwrap();
    ui.unmount(&find(&ui, &root, "first-field").unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // The first field still counts in the summary, which is the application's
    // state, but focus goes to the first field that can actually receive it.
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), find(&ui, &root, "second"));
}

#[test]
fn request_focus_for_rejects_a_foreign_runtime_and_ignores_a_stale_target() {
    let mut ui = ui();
    let mut foreign = self::ui();
    let outsider = foreign
        .mount_tree(Element::new("outsider", Button::new("Other", |_| Ok(()))))
        .unwrap();
    let outcome = Rc::new(RefCell::new(None));
    let target = Rc::new(RefCell::new(None::<WidgetId>));
    let (seen, aimed) = (outcome.clone(), target.clone());
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new("gone", Button::new("Gone", |_| Ok(()))),
                Element::new(
                    "act",
                    Button::new("Act", move |cx| {
                        let id = aimed.borrow().clone().unwrap();
                        *seen.borrow_mut() = Some(cx.request_focus_for(&id));
                        Ok(())
                    }),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let act = find(&ui, &root, "act").unwrap();

    *target.borrow_mut() = Some(outsider);
    press(&mut ui, &act);
    assert_eq!(*outcome.borrow(), Some(Err(RuntimeError::ForeignRuntime)));

    let gone = find(&ui, &root, "gone").unwrap();
    ui.unmount(&gone).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    *target.borrow_mut() = Some(gone);
    press(&mut ui, &act);
    assert_eq!(*outcome.borrow(), Some(Ok(())));
    assert_eq!(ui.focused(), Some(act), "a stale target is ignored");
}

#[test]
fn the_control_keeps_its_size_whatever_text_surrounds_it() {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::Normal);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "field",
                    Field::new(Element::new("input", TextField::new(ui.signal(String::new()))))
                        .label("A label that is considerably wider than any short message")
                        .help("A help line long enough to be wider than the text field it explains, by some margin.")
                        .status(status.read_only()),
                ),
                Element::new("bare", TextField::new(ui.signal(String::new()))),
            ]),
        ))
        .unwrap();
    draw(&mut ui);
    let bounds = |ui: &Runtime, key| ui.bounds(&find(ui, &root, key).unwrap()).unwrap().unwrap();
    let bare = bounds(&ui, "bare");
    let size = |rect: Rect| (rect.width(), rect.height());
    // Long surrounding text wraps to the control; it never widens the control.
    assert_eq!(size(bounds(&ui, "input")), size(bare));
    for next in [
        FieldStatus::error("Short"),
        FieldStatus::warning(
            "A warning that is long enough to need more than one line under a text field of the default width.",
        ),
        FieldStatus::error(""),
        FieldStatus::Normal,
    ] {
        status.set(next.clone()).unwrap();
        draw(&mut ui);
        assert_eq!(size(bounds(&ui, "input")), size(bare), "{next:?}");
        assert_eq!(bounds(&ui, "field").width(), bare.width(), "{next:?}");
    }
}

#[test]
fn a_hidden_field_in_error_is_skipped_for_the_first_visible_one() {
    let mut ui = ui();
    let (root, _form, first, second) = form_tree(&mut ui);
    let submit = find(&ui, &root, "submit").unwrap();
    first.set(FieldStatus::error("worse")).unwrap();
    second.set(FieldStatus::error("bad")).unwrap();
    // Hidden with its field, as on an inactive tab.
    ui.set_visible(&find(&ui, &root, "first-field").unwrap(), false)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), find(&ui, &root, "second"));
    // Shown again, it is the first field in error once more.
    ui.set_visible(&find(&ui, &root, "first-field").unwrap(), true)
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), find(&ui, &root, "first"));
}

#[test]
fn a_removed_field_stops_counting_and_frees_its_key() {
    let mut ui = ui();
    let (root, form, first, second) = form_tree(&mut ui);
    let submit = find(&ui, &root, "submit").unwrap();
    first.set(FieldStatus::error("worse")).unwrap();
    second.set(FieldStatus::warning("hmm")).unwrap();
    assert_eq!(form.summary().get().unwrap().errors(), 1);
    // The row goes away with its registration.
    assert_eq!(first.remove(), Ok(true));
    assert_eq!(first.clone().remove(), Ok(false), "already removed");
    let summary = form.summary().get().unwrap();
    assert_eq!((summary.errors(), summary.warnings()), (0, 1));
    assert_eq!(summary.first_error(), None);
    // A removed field is no longer reached, even though its Field is mounted.
    press(&mut ui, &submit);
    assert_eq!(ui.focused(), Some(submit.clone()));
    // A stale handle's writes stay out of the summary.
    first.set(FieldStatus::error("stale")).unwrap();
    assert_eq!(form.summary().get().unwrap().errors(), 0);
    // The key can be registered again, as a new field at the end.
    let again = form.field("first").unwrap();
    again.set(FieldStatus::error("new")).unwrap();
    assert_eq!(form.summary().get().unwrap().first_error(), Some("first"));
    assert_eq!(first.remove(), Ok(false), "a stale handle never reaches it");
    assert_eq!(form.summary().get().unwrap().errors(), 1);
    assert_eq!(again.remove(), Ok(true));
    assert_eq!(form.summary().get().unwrap().errors(), 0);
}

#[test]
fn a_hidden_label_names_the_control_without_taking_space() {
    let mut ui = ui();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "field",
                    Field::new(Element::new("button", Button::new("Plain", |_| Ok(()))))
                        .label("Volume")
                        .hide_label(),
                ),
                Element::new("bare", Button::new("Plain", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    draw(&mut ui);
    let height = |ui: &Runtime, key| {
        ui.bounds(&find(ui, &root, key).unwrap())
            .unwrap()
            .unwrap()
            .height()
    };
    assert_eq!(height(&ui, "field"), height(&ui, "bare"));
    let field = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "field")
        .unwrap();
    assert_eq!(
        field.details.semantics.unwrap().name.as_deref(),
        Some("Volume")
    );
}

#[test]
fn describing_a_field_again_applies_its_label_help_and_status_binding() {
    let mut ui = ui();
    let first = ui.signal(FieldStatus::error("First"));
    let second = ui.signal(FieldStatus::warning("Second"));
    let describe = |label: &str, help: &str, status: &Signal<FieldStatus>| {
        Element::new(
            "field",
            Field::new(Element::new("button", Button::new("Plain", |_| Ok(()))))
                .label(label)
                .help(help)
                .status(status.read_only()),
        )
    };
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([describe("Old", "Old help", &first)]),
        ))
        .unwrap();
    draw(&mut ui);
    let button = find(&ui, &root, "button").unwrap();
    assert_eq!(
        property(&ui, "field", "message"),
        InspectValue::Text("First".into())
    );
    ui.reconcile_children(&root, vec![describe("New", "New help", &second)])
        .unwrap();
    draw(&mut ui);
    assert_eq!(find(&ui, &root, "button").unwrap(), button);
    assert_eq!(
        property(&ui, "field", "label"),
        InspectValue::Text("New".into())
    );
    assert_eq!(
        property(&ui, "field", "help"),
        InspectValue::Text("New help".into())
    );
    assert_eq!(
        property(&ui, "field", "message"),
        InspectValue::Text("Second".into())
    );
    // The old binding no longer drives the field.
    first.set(FieldStatus::Normal).unwrap();
    draw(&mut ui);
    assert_eq!(
        property(&ui, "field", "status"),
        InspectValue::Text("warning".into())
    );
}

/// The color a scene shows behind `rect`: the last fill drawn before command
/// `before` that covers it, or the scene background.
fn backdrop(scene: &DisplayList, before: usize, rect: Rect) -> Color {
    let covers = |outer: &Rect| {
        outer.x() <= rect.x()
            && outer.y() <= rect.y()
            && outer.x() + outer.width() >= rect.x() + rect.width()
            && outer.y() + outer.height() >= rect.y() + rect.height()
    };
    scene.commands()[..before]
        .iter()
        .rev()
        .find_map(|command| match command {
            DrawCommand::FillRect { rect: outer, color } if covers(outer) => Some(*color),
            _ => None,
        })
        .unwrap_or_else(|| scene.background())
}

#[test]
fn an_error_message_is_readable_on_every_neutral_background() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for background in [
            None,
            Some(ColorRole::Surface),
            Some(ColorRole::SurfaceContainer),
        ] {
            let mut ui = ui();
            ui.theme().set_mode(mode).unwrap();
            let status = ui.signal(FieldStatus::error("A name is required"));
            let field = Element::new(
                "field",
                Field::new(Element::new(
                    "name",
                    TextField::new(ui.signal(String::new())),
                ))
                .status(status.read_only()),
            );
            let tree = match background {
                None => field,
                Some(role) => Element::new("panel", Container::new(field).background_role(role)),
            };
            let root = ui.mount_tree(tree).unwrap();
            let scene = draw(&mut ui);
            let control = ui
                .bounds(&find(&ui, &root, "name").unwrap())
                .unwrap()
                .unwrap();
            let error = role(&ui, ColorRole::Error);
            let expected = background.map_or(role(&ui, ColorRole::Background), |r| role(&ui, r));
            // The message's glyphs, below the outlined control.
            let glyphs: Vec<_> = scene
                .commands()
                .iter()
                .enumerate()
                .filter_map(|(index, command)| match command {
                    DrawCommand::DrawMask { rect, color, .. }
                        if *color == error && rect.y() >= control.y() + control.height() =>
                    {
                        Some((index, *rect))
                    }
                    _ => None,
                })
                .collect();
            assert!(!glyphs.is_empty(), "{mode:?} {background:?}: no message");
            for (index, rect) in glyphs {
                let behind = backdrop(&scene, index, rect);
                assert_eq!(behind, expected, "{mode:?} {background:?}");
                let ratio = OpaqueColor::try_from(error)
                    .unwrap()
                    .contrast_ratio(OpaqueColor::try_from(behind).unwrap());
                assert!(
                    ratio >= 4.5,
                    "{mode:?} error text on {background:?}: {ratio:.2}"
                );
            }
        }
    }
}
