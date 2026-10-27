//! Re-describing mounted editors through the public facade: a new description
//! with another binding, options or callbacks is applied, not ignored.
use crate::support;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::{frame, key};

/// The `text` inspection property of the component keyed `field`.
fn shown(ui: &Runtime) -> String {
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "field")
        .expect("the field is mounted");
    match &component.details.properties["text"] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("text is not text: {other:?}"),
    }
}

/// Mounts `first` under a column, flushes, describes it again as `second` and
/// flushes, and returns the column and the field's identity, which must
/// survive.
fn redescribe(ui: &mut Runtime, first: Element, second: Element) -> (WidgetId, WidgetId) {
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new("root", Column::new([first])))
        .unwrap();
    frame(ui);
    let before = ui.find_descendant(&root, "field").unwrap().unwrap();
    ui.reconcile_children(&root, vec![second]).unwrap();
    frame(ui);
    let after = ui.find_descendant(&root, "field").unwrap().unwrap();
    assert_eq!(before, after, "re-description must not remount the field");
    (root, after)
}

/// Asserts that the field shows `new`'s text and that typing writes `new`,
/// never `old`.
fn follows_new_binding(
    ui: &mut Runtime,
    id: &WidgetId,
    old: &Signal<String>,
    new: &Signal<String>,
) {
    assert_eq!(shown(ui), "new");
    ui.focus(id).unwrap();
    frame(ui);
    key(ui, KeyCode::End);
    ui.text_input("!").unwrap();
    frame(ui);
    assert_eq!(new.get().unwrap(), "new!");
    assert_eq!(old.get().unwrap(), "old");
}

#[test]
fn a_text_field_follows_a_new_binding() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal("old".to_owned()), ui.signal("new".to_owned()));
    let (_, id) = redescribe(
        &mut ui,
        Element::new("field", TextField::new(old.clone())),
        Element::new("field", TextField::new(new.clone())),
    );
    follows_new_binding(&mut ui, &id, &old, &new);
}

#[test]
fn a_text_area_follows_a_new_binding() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal("old".to_owned()), ui.signal("new".to_owned()));
    let (_, id) = redescribe(
        &mut ui,
        Element::new("field", TextArea::new(old.clone())),
        Element::new("field", TextArea::new(new.clone())),
    );
    follows_new_binding(&mut ui, &id, &old, &new);
}

#[test]
fn a_search_field_follows_a_new_binding() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal("old".to_owned()), ui.signal("new".to_owned()));
    let (_, id) = redescribe(
        &mut ui,
        Element::new("field", SearchField::new(old.clone())),
        Element::new("field", SearchField::new(new.clone())),
    );
    follows_new_binding(&mut ui, &id, &old, &new);
}

#[test]
fn an_inline_text_follows_a_new_binding() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal("old".to_owned()), ui.signal("new".to_owned()));
    let (_, id) = redescribe(
        &mut ui,
        Element::new("field", InlineText::new(old.clone())),
        Element::new("field", InlineText::new(new.clone())),
    );
    assert_eq!(shown(&ui), "new");
    ui.focus(&id).unwrap();
    frame(&mut ui);
    // Enter starts editing an inline row; typing then edits its draft, and a
    // second Enter saves it.
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    key(&mut ui, KeyCode::End);
    ui.text_input("!").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(new.get().unwrap(), "new!");
    assert_eq!(old.get().unwrap(), "old");
}

#[test]
fn number_inputs_follow_a_new_binding() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal(1_i64), ui.signal(5_i64));
    let (_, id) = redescribe(
        &mut ui,
        Element::new("field", IntegerInput::new(old.clone())),
        Element::new("field", IntegerInput::new(new.clone())),
    );
    assert_eq!(shown(&ui), "5");
    ui.focus(&id).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::ArrowUp);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!((old.get().unwrap(), new.get().unwrap()), (1, 6));

    let mut ui = Runtime::new();
    let (old, new) = (ui.signal(1.5), ui.signal(2.5));
    let _ = redescribe(
        &mut ui,
        Element::new("field", InlineDecimal::new(old.clone())),
        Element::new("field", InlineDecimal::new(new.clone())),
    );
    assert_eq!(shown(&ui), "2.5");
}

#[test]
fn the_same_binding_keeps_a_draft_and_a_new_one_drops_it_unsaved() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal("old".to_owned()), ui.signal("new".to_owned()));
    let normalized = |signal: &Signal<String>| {
        Element::new(
            "field",
            TextField::new(signal.clone()).normalize(|text| Ok(text.trim().to_owned())),
        )
    };
    let (root, id) = redescribe(&mut ui, normalized(&old), normalized(&old));
    ui.focus(&id).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::End);
    ui.text_input(" draft").unwrap();
    frame(&mut ui);
    // Re-describing with the same binding (and a style change) keeps the draft.
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            TextField::new(old.clone())
                .style(TextStyle::Monospace)
                .normalize(|text| Ok(text.trim().to_owned())),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui), "old draft");
    // A different binding drops it: nothing is saved to either binding, even
    // when focus later leaves.
    ui.reconcile_children(&root, vec![normalized(&new)])
        .unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui), "new");
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(
        (old.get().unwrap().as_str(), new.get().unwrap().as_str()),
        ("old", "new")
    );
}

#[test]
fn a_re_description_replaces_callbacks_and_normalizer() {
    let mut ui = Runtime::new();
    let name = ui.signal(String::new());
    let saves = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let field = |tag: &'static str| {
        let saves = saves.clone();
        let upper = tag == "second";
        Element::new(
            "field",
            TextField::new(name.clone())
                .normalize(move |text| Ok(if upper { text.to_uppercase() } else { text }))
                .on_save(move |_, text| {
                    saves.borrow_mut().push(format!("{tag}:{text}"));
                    Ok(())
                }),
        )
    };
    let (root, id) = redescribe(&mut ui, field("first"), field("second"));
    ui.focus(&id).unwrap();
    frame(&mut ui);
    ui.text_input("ada").unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "ADA");
    assert_eq!(saves.borrow().as_slice(), ["second:ADA"]);
    // Removing the normalizer leaves draft mode: edits write at once again.
    ui.reconcile_children(
        &root,
        vec![Element::new("field", TextField::new(name.clone()))],
    )
    .unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::End);
    ui.text_input("!").unwrap();
    assert_eq!(name.get().unwrap(), "ADA!");
}

#[test]
fn disabling_ends_an_inline_draft_unsaved() {
    let mut ui = Runtime::new();
    let name = ui.signal("Bass".to_owned());
    let (root, id) = redescribe(
        &mut ui,
        Element::new("field", InlineText::new(name.clone())),
        Element::new("field", InlineText::new(name.clone()).placeholder("Name")),
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    ui.text_input("Lead").unwrap();
    frame(&mut ui);
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            InlineText::new(name.clone()).disabled(),
        )],
    )
    .unwrap();
    frame(&mut ui);
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Bass");
    assert_eq!(shown(&ui), "Bass");
}

#[test]
fn rebinding_a_parameter_input_cancels_its_gesture() {
    let mut ui = Runtime::new();
    let (first, host) = ui
        .parameter(
            ParameterId::new(1, 1, "first").unwrap(),
            ParameterSpec::stepped("First", 0, 10, 2).unwrap(),
        )
        .unwrap();
    let (second, _second_host) = ui
        .parameter(
            ParameterId::new(1, 2, "second").unwrap(),
            ParameterSpec::stepped("Second", 0, 10, 7).unwrap(),
        )
        .unwrap();
    let outcomes = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = outcomes.clone();
    let _monitor = host
        .on_outcome(move |event| {
            sink.borrow_mut().push(event.outcome.clone());
            Ok(())
        })
        .unwrap();
    let (root, id) = redescribe(
        &mut ui,
        Element::new("field", InlineInteger::for_parameter(first.clone())),
        Element::new("field", InlineInteger::for_parameter(first.clone())),
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Repeat).unwrap();
    assert_eq!(first.snapshot().get().unwrap().preview, Some(4));
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            InlineInteger::for_parameter(second.clone()),
        )],
    )
    .unwrap();
    frame(&mut ui);
    // Releasing the key commits nothing to either parameter: the old gesture
    // was cancelled and the new parameter had none.
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release).unwrap();
    frame(&mut ui);
    let snapshot = first.snapshot().get().unwrap();
    assert_eq!((snapshot.base, snapshot.preview), (2, None));
    assert_eq!(second.snapshot().get().unwrap().base, 7);
    assert_eq!(outcomes.borrow().as_slice(), [EditOutcome::Cancelled]);
    assert_eq!(shown(&ui), "7");
}

#[test]
fn an_invalid_replacement_fails_updates_until_a_valid_description() {
    let mut ui = Runtime::new();
    let foreign = Runtime::new();
    let (old, stranger) = (ui.signal("old".to_owned()), foreign.signal("x".to_owned()));
    let (root, id) = redescribe(
        &mut ui,
        Element::new("field", TextField::new(old.clone())),
        Element::new("field", TextField::new(old.clone())),
    );
    // A re-description is applied without validation; the next update
    // validates the binding as a mount would and fails, keeping the field
    // mounted with its previous presentation.
    ui.reconcile_children(&root, vec![Element::new("field", TextField::new(stranger))])
        .unwrap();
    assert!(matches!(
        ui.flush(std::time::Duration::ZERO),
        Err(RuntimeError::ForeignRuntime)
    ));
    // The failed update is retried on every flush, not forgotten.
    assert!(matches!(
        ui.flush(std::time::Duration::ZERO),
        Err(RuntimeError::ForeignRuntime)
    ));
    assert_eq!(
        ui.find_descendant(&root, "field").unwrap(),
        Some(id.clone())
    );
    assert_eq!(shown(&ui), "old");
    // A valid description recovers the same field.
    ui.reconcile_children(
        &root,
        vec![Element::new("field", TextField::new(old.clone()))],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(ui.find_descendant(&root, "field").unwrap(), Some(id));
    assert_eq!(shown(&ui), "old");
}

#[test]
fn rebinding_a_search_field_drops_its_pending_query() {
    let mut ui = Runtime::new();
    let (old, new) = (ui.signal(String::new()), ui.signal("new".to_owned()));
    let delay = std::time::Duration::from_millis(150);
    let (root, id) = redescribe(
        &mut ui,
        Element::new("field", SearchField::new(old.clone()).debounce(delay)),
        Element::new("field", SearchField::new(old.clone()).debounce(delay)),
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    ui.text_input("draft").unwrap();
    frame(&mut ui);
    // The debounce deadline is the earliest wake, before the caret's blink.
    let debounce = ui.next_deadline().expect("the debounce deadline runs");
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            SearchField::new(new.clone()).debounce(delay),
        )],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(shown(&ui), "new");
    // The dropped query's deadline is cancelled: only the blink remains.
    assert!(ui.next_deadline().expect("the caret blinks") > debounce);
    ui.clear_focus();
    frame(&mut ui);
    assert_eq!(
        (old.get().unwrap().as_str(), new.get().unwrap().as_str()),
        ("", "new")
    );
}

#[test]
fn a_new_debounce_delay_restarts_a_pending_query_deadline() {
    let mut ui = Runtime::new();
    let query = ui.signal(String::new());
    let (long, short) = (
        std::time::Duration::from_secs(10),
        std::time::Duration::from_millis(100),
    );
    let (root, id) = redescribe(
        &mut ui,
        Element::new("field", SearchField::new(query.clone()).debounce(long)),
        Element::new("field", SearchField::new(query.clone()).debounce(long)),
    );
    ui.focus(&id).unwrap();
    frame(&mut ui);
    ui.text_input("draft").unwrap();
    frame(&mut ui);
    ui.reconcile_children(
        &root,
        vec![Element::new(
            "field",
            SearchField::new(query.clone()).debounce(short),
        )],
    )
    .unwrap();
    frame(&mut ui);
    // The pending query now waits for the new, shorter delay, not the
    // ten-second deadline armed before the re-description.
    let next = ui.next_deadline().expect("the query deadline runs");
    assert!(next <= short, "{next:?}");
    assert_eq!(query.get().unwrap(), "");
}

/// Runs [`check_redescription`] with descriptions built from two values of
/// one kind of binding, panicking with both reports when they differ.
#[track_caller]
fn conforms<T: Clone + 'static>(first: T, second: T, describe: impl Fn(&Signal<T>) -> Element) {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    let (a, b) = (ui.signal(first), ui.signal(second));
    if let Err(error) = check_redescription(&mut ui, describe(&a), describe(&b), describe(&b)) {
        panic!("{error}: {error:?}");
    }
}

#[test]
fn built_ins_present_a_new_description_like_a_fresh_mount() {
    let text = || ("old".to_owned(), "new".to_owned());
    let (a, b) = text();
    conforms(a, b, |s| {
        Element::new("c", TextField::new(s.clone()).read_only())
    });
    let (a, b) = text();
    conforms(a, b, |s| Element::new("c", TextArea::new(s.clone())));
    let (a, b) = text();
    conforms(a, b, |s| {
        Element::new("c", SearchField::new(s.clone()).placeholder("Find"))
    });
    let (a, b) = text();
    conforms(a, b, |s| {
        Element::new("c", InlineText::new(s.clone()).placeholder("Name"))
    });
    let (a, b) = text();
    conforms(a, b, |s| Element::new("c", Label::new(s.read_only())));
    conforms(1_i64, 9, |s| {
        Element::new("c", IntegerInput::new(s.clone()).range(0, 10).unwrap())
    });
    conforms(1_i64, 9, |s| {
        Element::new("c", InlineInteger::new(s.clone()).label("Voices").unit("v"))
    });
    conforms(0.5, 1.5, |s| {
        Element::new("c", DecimalInput::new(s.clone()).decimal_places(2).unwrap())
    });
    conforms(0.5, 1.5, |s| {
        Element::new("c", InlineDecimal::new(s.clone()).label("Gain"))
    });
    conforms(false, true, |s| {
        Element::new("c", Checkbox::new(s.clone(), "Loop"))
    });
    conforms(false, true, |s| {
        Element::new("c", Toggle::new(s.clone(), "Mute"))
    });
    conforms(Color::ACCENT, Color::WHITE, |s| {
        Element::new("c", ColorSwatch::bound(s.read_only()))
    });
    conforms(false, true, |s| {
        Element::new("c", Spinner::new(s.read_only()))
    });
    // Options, not only bindings, follow the new description.
    let mut ui = Runtime::new();
    let name = ui.signal("x".to_owned());
    let first = check_redescription(
        &mut ui,
        Element::new("c", TextField::new(name.clone())),
        Element::new("c", TextField::new(name.clone()).disabled()),
        Element::new("c", TextField::new(name.clone()).disabled()),
    )
    .unwrap();
    ui.unmount(&first.root).unwrap();
    let count = ui.signal(3_i64);
    check_redescription(
        &mut ui,
        Element::new("d", IntegerInput::new(count.clone())),
        Element::new("d", IntegerInput::new(count.clone()).read_only().unit("x")),
        Element::new("d", IntegerInput::new(count.clone()).read_only().unit("x")),
    )
    .unwrap();
}

#[test]
fn the_check_reports_a_component_that_keeps_its_first_binding() {
    // The color picker documents that it keeps its mounted binding; the
    // check sees the difference.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 600.0, 1.0).unwrap())
        .unwrap();
    let (a, b) = (ui.signal(Color::ACCENT), ui.signal(Color::WHITE));
    let picker = |s: &Signal<Color>| Element::new("c", ColorPicker::new(s.clone()).name("Tint"));
    let outcome = check_redescription(&mut ui, picker(&a), picker(&b), picker(&b));
    assert!(
        matches!(outcome, Err(RedescriptionError::Differs { .. })),
        "{outcome:?}"
    );
    // Mismatched keys are refused before anything mounts.
    assert!(matches!(
        pert_ui::redescribe(
            &mut ui,
            picker(&a),
            Element::new("other", ColorPicker::new(b.clone()))
        ),
        Err(RedescriptionError::KeyChanged { .. })
    ));
}

#[test]
fn a_failing_reference_flush_still_unmounts_the_reference_tree() {
    let mut ui = Runtime::new();
    let local = ui.signal("Local".to_owned());
    let foreign = Runtime::new().signal("Foreign".to_owned());
    let label = |text: &Signal<String>| Element::new("caption", Label::new(text.read_only()));
    // The reference copy binds a signal from another runtime, so its flush
    // fails after it has been mounted.
    let outcome = check_redescription(&mut ui, label(&local), label(&local), label(&foreign));
    assert!(
        matches!(outcome, Err(RedescriptionError::Runtime(_))),
        "{outcome:?}"
    );
    // Both helper-owned roots are gone: neither fixed key blocks a later check.
    ui.mount_tree(Element::new("fresh", Label::new("Free")))
        .unwrap();
    ui.mount_tree(Element::new("redescribe", Label::new("Also free")))
        .unwrap();
}

/// The `text` inspection property of the label keyed `key`.
fn label_text(ui: &Runtime, key: &str) -> String {
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("{key} is mounted"));
    match &component.details.properties["text"] {
        InspectValue::Text(text) => text.clone(),
        other => panic!("text is not text: {other:?}"),
    }
}

/// A component that records the preview of the snapshot it reads in update.
struct Watcher<T: ParameterValue> {
    snapshot: ReadSignal<ParameterSnapshot<T>>,
    seen: Rc<RefCell<String>>,
}
impl<T: ParameterValue + std::fmt::Debug> Component for Watcher<T> {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        *self.seen.borrow_mut() = format!("{:?}", cx.read(&self.snapshot)?.preview);
        Ok(())
    }
}

/// Mounts the control `phase` describes, a component reading `watched`'s
/// snapshot and a label over a `Computed` derived from it, then starts a
/// gesture with `begin`, moves to phase 1 and flushes once. Reconciliation
/// cancels the gesture inside that flush, and both dependents must show the
/// cancelled snapshot when it returns, with no update left for a later flush.
fn dependents_settle_in_the_reconciling_flush<T>(
    ui: &mut Runtime,
    watched: &Parameter<T>,
    phase: &Signal<u8>,
    control: impl Fn(u8) -> Element + 'static,
    begin: impl FnOnce(&mut Runtime, &WidgetId),
) where
    T: ParameterValue + std::fmt::Debug,
{
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let seen = Rc::new(RefCell::new(String::new()));
    let computed = watched
        .snapshot()
        .map(|snapshot| format!("{:?}", snapshot.preview))
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("host", Dynamic::new(phase, move |_, phase| control(*phase))),
                Element::new(
                    "watcher",
                    Watcher {
                        snapshot: watched.snapshot(),
                        seen: seen.clone(),
                    },
                ),
                Element::new("computed", Label::new(computed.clone())),
            ]),
        ))
        .unwrap();
    frame(ui);
    let field = ui.find_descendant(&root, "field").unwrap().unwrap();
    begin(ui, &field);
    frame(ui);
    let previewing = seen.borrow().clone();
    assert_ne!(previewing, "None", "the gesture previews");
    assert_eq!(label_text(ui, "computed"), previewing);
    phase.set(1).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(watched.snapshot().get().unwrap().preview, None);
    assert_eq!(*seen.borrow(), "None");
    assert_eq!(label_text(ui, "computed"), "None");
    // Nothing is left for another flush: painting finds no pending update.
    ui.paint().unwrap();
    assert_eq!(ui.flush(ui.now()).unwrap().components_updated, 0);
}

#[test]
fn a_reconciled_number_input_settles_the_cancelled_snapshot_in_the_same_flush() {
    for deny in [false, true] {
        let mut ui = Runtime::new();
        let (first, _first_host) = ui
            .parameter(
                ParameterId::new(1, 1, "first").unwrap(),
                ParameterSpec::stepped("First", 0, 10, 2).unwrap(),
            )
            .unwrap();
        let (second, _second_host) = ui
            .parameter(
                ParameterId::new(1, 2, "second").unwrap(),
                ParameterSpec::stepped("Second", 0, 10, 7).unwrap(),
            )
            .unwrap();
        let phase = ui.signal(0_u8);
        let watched = first.clone();
        dependents_settle_in_the_reconciling_flush(
            &mut ui,
            &watched,
            &phase,
            move |phase| {
                let input = match (phase, deny) {
                    (0, _) | (_, true) => InlineInteger::for_parameter(first.clone()),
                    _ => InlineInteger::for_parameter(second.clone()),
                };
                let input = if phase == 1 && deny {
                    input.disabled()
                } else {
                    input
                };
                Element::new("field", input)
            },
            |ui, field| {
                ui.focus(field).unwrap();
                frame(ui);
                ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
                ui.key_input(KeyCode::ArrowUp, KeyAction::Repeat).unwrap();
            },
        );
    }
}

#[test]
fn a_reconciled_slider_settles_the_cancelled_snapshot_in_the_same_flush() {
    for deny in [false, true] {
        let mut ui = Runtime::new();
        let (first, _first_host) = ui
            .parameter(
                ParameterId::new(1, 1, "first").unwrap(),
                ParameterSpec::new("First", 0.0, 100.0, 50.0).unwrap(),
            )
            .unwrap();
        let (second, _second_host) = ui
            .parameter(
                ParameterId::new(1, 2, "second").unwrap(),
                ParameterSpec::new("Second", 0.0, 100.0, 50.0).unwrap(),
            )
            .unwrap();
        let phase = ui.signal(0_u8);
        let watched = first.clone();
        dependents_settle_in_the_reconciling_flush(
            &mut ui,
            &watched,
            &phase,
            move |phase| {
                let slider = match (phase, deny) {
                    (0, _) | (_, true) => Slider::new(first.clone()),
                    _ => Slider::new(second.clone()),
                };
                let slider = if phase == 1 && deny {
                    slider.enabled(false)
                } else {
                    slider
                };
                Element::new("field", slider)
            },
            |ui, field| {
                let [x, y, width, height] = ui.bounds(field).unwrap().unwrap().values();
                ui.pointer_input(PointerEvent::new(
                    Point::new(x + width * 0.8, y + height / 2.0).unwrap(),
                    PointerKind::Press(PointerButton::Primary),
                ))
                .unwrap();
            },
        );
    }
}
