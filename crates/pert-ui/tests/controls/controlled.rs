//! Controlled controls: `Checkbox::controlled`, `Toggle::controlled` and
//! `InlineText::controlled` show the value their description carries and
//! propose each user change to their callback without writing anything.
//!
//! Each test is marked as one of two kinds. A **new-behavior** test covers a
//! transition only the controlled path has and names the assertion that
//! detects the `new` behavior substituted for it. A **contract** test checks
//! a rule the controlled path shares with `new` or gets from existing
//! machinery; it is not compared with the `new` behavior.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log<T> = Rc<RefCell<Vec<T>>>;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Checkbox,
    Toggle,
}
const KINDS: [Kind; 2] = [Kind::Checkbox, Kind::Toggle];

/// A controlled checkbox or toggle keyed `"box"` that records its proposals.
fn boolean(kind: Kind, checked: bool, log: &Log<bool>) -> Element {
    let log = log.clone();
    let on_change = move |_: &mut EventContext, value: bool| {
        log.borrow_mut().push(value);
        Ok(())
    };
    match kind {
        Kind::Checkbox => Checkbox::controlled(checked, "Box", on_change).key("box"),
        Kind::Toggle => Toggle::controlled(checked, "Box", on_change).key("box"),
    }
}

/// The same control bound to a signal, under the same key.
fn bound(kind: Kind, value: &Signal<bool>) -> Element {
    match kind {
        Kind::Checkbox => Checkbox::new(value, "Box").key("box"),
        Kind::Toggle => Toggle::new(value, "Box").key("box"),
    }
}

/// A controlled label keyed `"name"` that records its proposals.
fn label(text: &str, log: &Log<String>) -> InlineText {
    let log = log.clone();
    InlineText::controlled(text, move |_, text| {
        log.borrow_mut().push(text.to_owned());
        Ok(())
    })
}

struct Stage {
    ui: Runtime,
    root: WidgetId,
}
impl Stage {
    /// Mounts `child` in a column keyed `"root"` and settles it.
    fn new(child: Element) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let root = ui.mount_tree(Column::new([child]).key("root"))?;
        ui.flush(Duration::ZERO)?;
        Ok(Self { ui, root })
    }
    fn redescribe(&mut self, child: Element) -> Result<(), RuntimeError> {
        self.ui.reconcile_children(&self.root, vec![child])
    }
    fn flush(&mut self) -> Result<(), RuntimeError> {
        self.ui.flush(Duration::ZERO).map(|_| ())
    }
    fn id(&self, key: &str) -> Result<WidgetId, Box<dyn std::error::Error>> {
        Ok(self.ui.find(&self.root, &[key])?.ok_or("the control")?)
    }
    fn details(&self, key: &str) -> ComponentDetails {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.key == key)
            .unwrap_or_else(|| panic!("no component keyed {key}"))
            .details
    }
    fn property(&self, key: &str, name: &str) -> Option<InspectValue> {
        self.details(key).properties.get(name).cloned()
    }
    /// What the boolean control shows, in inspection and in its semantics.
    fn checked(&self) -> (Option<InspectValue>, Option<SemanticValue>) {
        let details = self.details("box");
        (
            details.properties.get("checked").cloned(),
            details.semantics.and_then(|semantics| semantics.value),
        )
    }
    fn shows(&self, checked: bool) -> bool {
        self.checked()
            == (
                Some(InspectValue::Bool(checked)),
                Some(SemanticValue::Bool(checked)),
            )
    }
    fn pointer(&mut self, key: &str, kind: PointerKind) -> TestResult {
        let id = self.id(key)?;
        let [x, y, _, height] = self.ui.bounds(&id)?.ok_or("laid out")?.values();
        self.ui.pointer_input(PointerEvent::new(
            Point::new(x + 10.0, y + height / 2.0)?,
            kind,
        ))?;
        Ok(())
    }
    /// A raw primary click on the control, without a flush.
    fn click(&mut self, key: &str) -> TestResult {
        self.pointer(key, PointerKind::Press(PointerButton::Primary))?;
        self.pointer(key, PointerKind::Release(PointerButton::Primary))
    }
    fn key(&mut self, code: KeyCode, action: KeyAction) -> Result<(), RuntimeError> {
        self.ui.key_input(code, action).map(|_| ())
    }
    fn press(&mut self, code: KeyCode) -> Result<(), RuntimeError> {
        self.key(code, KeyAction::Press)?;
        self.key(code, KeyAction::Release)
    }
    /// Begins editing the label from code and types `text` over its text.
    fn edit(&mut self, text: &str) -> TestResult {
        let id = self.id("name")?;
        assert_eq!(
            self.ui.text_command(&id, TextCommand::BeginEdit)?,
            TextCommandOutcome::Handled
        );
        self.flush()?;
        self.ui.text_input(text)?;
        Ok(())
    }
    fn editing(&self) -> bool {
        self.property("name", "editing") == Some(InspectValue::Bool(true))
    }
    fn text(&self) -> Option<InspectValue> {
        self.property("name", "text")
    }
}

fn text(value: &str) -> Option<InspectValue> {
    Some(InspectValue::Text(value.to_owned()))
}

// ---- Checkbox::controlled and Toggle::controlled ----

/// Contract: the callback runs where a signal-bound control writes, with the
/// negation of the value shown.
#[test]
fn a_click_and_a_space_release_propose_the_negated_value() -> TestResult {
    for kind in KINDS {
        for checked in [false, true] {
            let log: Log<bool> = Rc::default();
            let mut s = Stage::new(boolean(kind, checked, &log))?;
            s.click("box")?;
            s.flush()?;
            let id = s.id("box")?;
            s.ui.focus(&id)?;
            s.key(KeyCode::Space, KeyAction::Press)?;
            assert!(
                log.borrow().len() == 1,
                "{kind:?}: a press proposes nothing"
            );
            s.key(KeyCode::Space, KeyAction::Release)?;
            assert_eq!(*log.borrow(), [!checked, !checked], "{kind:?}");
        }
    }
    Ok(())
}

/// New behavior: a refused toggle leaves the described value on screen.
/// Detected by the assertion `s.shows(false)`, which fails when a toggle
/// changes the shown value as a write to a signal does.
#[test]
fn a_refused_toggle_shows_the_described_value() -> TestResult {
    for kind in KINDS {
        let log: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &log))?;
        s.click("box")?;
        s.flush()?;
        assert_eq!(*log.borrow(), [true], "{kind:?}");
        assert!(s.shows(false), "{kind:?}: {:?}", s.checked());
    }
    Ok(())
}

/// New behavior: refused activations each propose the same value. Detected
/// by the assertion on `[true, true]`, which fails when the first toggle
/// changes the shown value, so the second proposes `false`.
#[test]
fn repeated_refused_activations_propose_the_same_value() -> TestResult {
    for kind in KINDS {
        let log: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &log))?;
        s.click("box")?;
        s.flush()?;
        s.click("box")?;
        s.flush()?;
        assert_eq!(*log.borrow(), [true, true], "{kind:?}");
    }
    Ok(())
}

/// Contract and integration: inside a `Dynamic` over the model, the
/// callback's write describes the control again, and exactly one flush after
/// a raw click shows the new value.
#[test]
fn an_applied_proposal_shows_after_one_flush() -> TestResult {
    for kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let model = ui.signal(false);
        let store = model.clone();
        let root = ui.mount_tree(
            Dynamic::new(&model, move |_, &on| {
                let store = store.clone();
                let apply = move |_: &mut EventContext, on: bool| store.set(on);
                match kind {
                    Kind::Checkbox => Checkbox::controlled(on, "Box", apply).key("box"),
                    Kind::Toggle => Toggle::controlled(on, "Box", apply).key("box"),
                }
            })
            .key("root"),
        )?;
        ui.flush(Duration::ZERO)?;
        let mut s = Stage { ui, root };
        let before = s.id("box")?;
        s.click("box")?;
        assert!(model.get()?, "{kind:?}: the callback wrote the model");
        s.flush()?;
        assert!(s.shows(true), "{kind:?}: {:?}", s.checked());
        assert_eq!(s.id("box")?, before, "{kind:?}: the control was reconciled");
    }
    Ok(())
}

/// New behavior: a description with the same value keeps an armed press,
/// whose release then proposes. Detected by the assertion on `[true]`, which
/// fails when every description cancels the press.
#[test]
fn a_same_value_description_keeps_an_armed_press() -> TestResult {
    for kind in KINDS {
        let log: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &log))?;
        s.pointer("box", PointerKind::Press(PointerButton::Primary))?;
        s.redescribe(boolean(kind, false, &log))?;
        s.flush()?;
        s.pointer("box", PointerKind::Release(PointerButton::Primary))?;
        assert_eq!(*log.borrow(), [true], "{kind:?}");
    }
    Ok(())
}

/// New behavior: a description with a changed value cancels an armed press,
/// and the release proposes nothing. Detected by the assertion that the log
/// stays empty, which fails when the press survives the change, as it does
/// for a signal-bound control whose signal is written during a press.
#[test]
fn a_changed_value_cancels_an_armed_press() -> TestResult {
    for kind in KINDS {
        let log: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &log))?;
        s.pointer("box", PointerKind::Press(PointerButton::Primary))?;
        s.redescribe(boolean(kind, true, &log))?;
        s.flush()?;
        assert!(s.shows(true), "{kind:?}: the new value is shown");
        s.pointer("box", PointerKind::Release(PointerButton::Primary))?;
        s.flush()?;
        assert!(log.borrow().is_empty(), "{kind:?}: {:?}", log.borrow());
        // A keyboard press is cancelled the same way.
        let id = s.id("box")?;
        s.ui.focus(&id)?;
        s.key(KeyCode::Space, KeyAction::Press)?;
        s.redescribe(boolean(kind, false, &log))?;
        s.flush()?;
        s.key(KeyCode::Space, KeyAction::Release)?;
        assert!(log.borrow().is_empty(), "{kind:?}: {:?}", log.borrow());
    }
    Ok(())
}

/// Contract: each description replaces the callback, the constructor's
/// included.
#[test]
fn a_new_description_replaces_the_callback() -> TestResult {
    for kind in KINDS {
        let (old, new): (Log<bool>, Log<bool>) = (Rc::default(), Rc::default());
        let mut s = Stage::new(boolean(kind, false, &old))?;
        s.redescribe(boolean(kind, false, &new))?;
        s.flush()?;
        s.click("box")?;
        assert!(old.borrow().is_empty(), "{kind:?}");
        assert_eq!(*new.borrow(), [true], "{kind:?}");
    }
    Ok(())
}

/// A controlled checkbox or toggle whose constructor callback logs to
/// `constructor` and whose later `on_change` builder logs to `replacement`.
fn replaced_boolean(kind: Kind, constructor: &Log<bool>, replacement: &Log<bool>) -> Element {
    let (first, later) = (constructor.clone(), replacement.clone());
    let constructed = move |_: &mut EventContext, value: bool| {
        first.borrow_mut().push(value);
        Ok(())
    };
    let replacing = move |_: &mut EventContext, value: bool| {
        later.borrow_mut().push(value);
        Ok(())
    };
    match kind {
        Kind::Checkbox => Checkbox::controlled(false, "Box", constructed)
            .on_change(replacing)
            .key("box"),
        Kind::Toggle => Toggle::controlled(false, "Box", constructed)
            .on_change(replacing)
            .key("box"),
    }
}

/// Contract: a later `on_change` builder replaces the constructor's
/// callback, at mount and by redescription: only the replacement runs, once
/// per proposal.
#[test]
fn the_on_change_builder_replaces_the_constructor_callback() -> TestResult {
    for kind in KINDS {
        let (constructor, replacement): (Log<bool>, Log<bool>) = (Rc::default(), Rc::default());
        let mut s = Stage::new(replaced_boolean(kind, &constructor, &replacement))?;
        s.click("box")?;
        assert!(
            constructor.borrow().is_empty(),
            "{kind:?}: {:?}",
            constructor.borrow()
        );
        assert_eq!(*replacement.borrow(), [true], "{kind:?}");
        // The same through a redescription of a plain controlled control.
        let plain: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &plain))?;
        let (constructor, replacement): (Log<bool>, Log<bool>) = (Rc::default(), Rc::default());
        s.redescribe(replaced_boolean(kind, &constructor, &replacement))?;
        s.flush()?;
        s.click("box")?;
        assert!(plain.borrow().is_empty(), "{kind:?}");
        assert!(
            constructor.borrow().is_empty(),
            "{kind:?}: {:?}",
            constructor.borrow()
        );
        assert_eq!(*replacement.borrow(), [true], "{kind:?}");
    }
    Ok(())
}

/// Contract: a callback error propagates from the input call; the shown value
/// stays and no later flush calls the callback again.
#[test]
fn a_callback_error_propagates_and_is_not_replayed() -> TestResult {
    for kind in KINDS {
        let calls = Rc::new(RefCell::new(0));
        let count = calls.clone();
        let failing = move |_: &mut EventContext, _: bool| {
            *count.borrow_mut() += 1;
            Err(RuntimeError::Exhausted)
        };
        let control = match kind {
            Kind::Checkbox => Checkbox::controlled(false, "Box", failing).key("box"),
            Kind::Toggle => Toggle::controlled(false, "Box", failing).key("box"),
        };
        let mut s = Stage::new(control)?;
        let id = s.id("box")?;
        s.ui.focus(&id)?;
        s.key(KeyCode::Space, KeyAction::Press)?;
        assert_eq!(
            s.ui.key_input(KeyCode::Space, KeyAction::Release).err(),
            Some(RuntimeError::Exhausted),
            "{kind:?}"
        );
        s.flush()?;
        s.flush()?;
        assert_eq!(*calls.borrow(), 1, "{kind:?}");
        assert!(s.shows(false), "{kind:?}");
    }
    Ok(())
}

/// Contract: switching the same key between `new` and `controlled` is a
/// change of binding, which cancels an armed press either way.
#[test]
fn switching_between_new_and_controlled_cancels_an_armed_press() -> TestResult {
    for kind in KINDS {
        let log: Log<bool> = Rc::default();
        let mut s = Stage::new(boolean(kind, false, &log))?;
        let value = s.ui.signal(false);
        s.pointer("box", PointerKind::Press(PointerButton::Primary))?;
        s.redescribe(bound(kind, &value))?;
        s.flush()?;
        s.pointer("box", PointerKind::Release(PointerButton::Primary))?;
        s.flush()?;
        assert!(!value.get()?, "{kind:?}: the signal was not written");
        s.pointer("box", PointerKind::Press(PointerButton::Primary))?;
        s.redescribe(boolean(kind, false, &log))?;
        s.flush()?;
        s.pointer("box", PointerKind::Release(PointerButton::Primary))?;
        s.flush()?;
        assert!(!value.get()?, "{kind:?}");
        assert!(log.borrow().is_empty(), "{kind:?}: {:?}", log.borrow());
    }
    Ok(())
}

/// Compatibility: a signal-bound control still writes and keeps an armed
/// press when the application writes its signal during the press.
#[test]
fn a_bound_control_keeps_its_press_across_a_signal_write() -> TestResult {
    for kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
        let value = ui.signal(false);
        let root = ui.mount_tree(Column::new([bound(kind, &value)]).key("root"))?;
        ui.flush(Duration::ZERO)?;
        let mut s = Stage { ui, root };
        s.pointer("box", PointerKind::Press(PointerButton::Primary))?;
        value.set(true)?;
        s.flush()?;
        s.pointer("box", PointerKind::Release(PointerButton::Primary))?;
        assert!(
            !value.get()?,
            "{kind:?}: the release toggled the written value"
        );
    }
    Ok(())
}

/// Compatibility: a parameter-bound control still edits through its host.
#[test]
fn a_parameter_control_still_commits_through_its_host() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let (parameter, _host) = ui.parameter(
        ParameterId::new(1, 1, "bypass")?,
        ParameterSpec::boolean("Bypass", false)?,
    )?;
    let root = ui.mount_tree(
        Column::new([Checkbox::for_parameter(parameter.clone()).key("box")]).key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    let mut s = Stage { ui, root };
    s.click("box")?;
    s.flush()?;
    assert!(parameter.snapshot().get()?.base);
    assert!(s.shows(true));
    Ok(())
}

// ---- InlineText::controlled ----

/// Contract: Enter and focus loss with a changed draft propose the
/// normalized text; an unchanged draft and Escape propose nothing.
#[test]
fn saves_propose_the_normalized_draft() -> TestResult {
    let log: Log<String> = Rc::default();
    let field = |log: &Log<String>| {
        label("Old", log)
            .normalize(|text| Ok(text.trim().to_owned()))
            .key("name")
    };
    let mut s = Stage::new(
        Column::new([field(&log), Button::new("Other", |_| Ok(())).key("other")]).key("form"),
    )?;
    let form = s.ui.find(&s.root, &["form"])?.ok_or("the form")?;
    s.root = form;
    s.edit(" New ")?;
    s.press(KeyCode::Enter)?;
    assert_eq!(*log.borrow(), ["New"]);
    // Focus leaving the label saves too.
    s.edit("Moved")?;
    let other = s.id("other")?;
    s.ui.focus(&other)?;
    s.flush()?;
    assert_eq!(*log.borrow(), ["New", "Moved"]);
    // An unchanged draft and Escape propose nothing.
    s.edit("Old")?;
    s.press(KeyCode::Enter)?;
    s.edit("Discarded")?;
    s.press(KeyCode::Escape)?;
    s.flush()?;
    assert_eq!(*log.borrow(), ["New", "Moved"]);
    assert_eq!(s.text(), text("Old"));
    Ok(())
}

/// New behavior: a save the application does not apply leaves the label
/// showing the described text. Detected by the assertion `s.text() ==
/// text("Old")`, which fails when the save writes the shown text as a save
/// to a signal does.
#[test]
fn an_unapplied_save_shows_the_described_text() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    s.edit("New")?;
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert_eq!(*log.borrow(), ["New"]);
    assert!(!s.editing());
    assert_eq!(s.text(), text("Old"));
    let semantics = s.details("name").semantics.ok_or("semantics")?;
    assert_eq!(semantics.value, Some(SemanticValue::Text("Old".to_owned())));
    Ok(())
}

/// Contract: the normalizer refuses as with `new`; the draft stays open with
/// the refusal, and nothing is proposed.
#[test]
fn a_normalizer_refusal_keeps_the_draft_and_proposes_nothing() -> TestResult {
    let log: Log<String> = Rc::default();
    let refusals: Log<bool> = Rc::default();
    let report = refusals.clone();
    let mut s = Stage::new(
        label("Old", &log)
            .normalize(|text| {
                if text.is_empty() {
                    Err(InputRejection::new("Enter a name"))
                } else {
                    Ok(text)
                }
            })
            .on_rejection(move |_, rejection| {
                report.borrow_mut().push(rejection.is_some());
                Ok(())
            })
            .key("name"),
    )?;
    s.edit("")?;
    s.press(KeyCode::Delete)?;
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert!(s.editing());
    assert!(s.property("name", "validation_error").is_some());
    assert!(log.borrow().is_empty());
    assert_eq!(*refusals.borrow(), [true]);
    Ok(())
}

/// Contract: the widened normalizer contract. A normalizer that reads the
/// application's model refuses a name another row has; after that row is
/// renamed, the next Enter checks the same draft again and saves it.
#[test]
fn a_model_dependent_normalizer_accepts_after_the_model_changes() -> TestResult {
    let log: Log<String> = Rc::default();
    let others = Rc::new(RefCell::new(vec!["Drums".to_owned()]));
    let taken = others.clone();
    let mut s = Stage::new(
        label("Bass", &log)
            .normalize(move |text| {
                if taken.borrow().contains(&text) {
                    Err(InputRejection::new("Another track has this name"))
                } else {
                    Ok(text)
                }
            })
            .key("name"),
    )?;
    s.edit("Drums")?;
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert!(s.editing(), "refused while another row has the name");
    assert!(log.borrow().is_empty());
    // Changing the model alone checks nothing again.
    others.borrow_mut()[0] = "Kit".to_owned();
    s.flush()?;
    assert!(s.editing());
    s.press(KeyCode::Enter)?;
    assert_eq!(*log.borrow(), ["Drums"]);
    Ok(())
}

/// Contract: described text the editor does not accept is refused at
/// adoption, as a refused bound value is, without failing the runtime: the
/// label keeps its text, reports `binding_error`, marks its semantics invalid
/// and cannot begin editing; during a session it ends the session unsaved;
/// the next valid text ends the refusal.
#[test]
fn described_text_the_editor_refuses_is_refused_at_adoption() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    s.redescribe(label("two\nlines", &log).key("name"))?;
    s.flush()?;
    assert_eq!(s.text(), text("Old"));
    assert!(s.property("name", "binding_error").is_some());
    let semantics = s.details("name").semantics.ok_or("semantics")?;
    assert!(semantics.invalid);
    let id = s.id("name")?;
    assert_eq!(
        s.ui.text_command(&id, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    // The next acceptable text ends the refusal.
    s.redescribe(label("Fine", &log).key("name"))?;
    s.flush()?;
    assert_eq!(s.text(), text("Fine"));
    assert_eq!(s.property("name", "binding_error"), None);
    assert!(!s.details("name").semantics.ok_or("semantics")?.invalid);
    // Described during a session, it ends the session unsaved.
    s.edit("Typed")?;
    s.redescribe(label(&"x".repeat(TextEditError::MAX_TEXT_BYTES + 1), &log).key("name"))?;
    s.flush()?;
    assert!(!s.editing());
    assert_eq!(s.text(), text("Fine"));
    assert!(s.property("name", "binding_error").is_some());
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert!(log.borrow().is_empty());
    Ok(())
}

/// Contract: a label mounted with refused text shows nothing until a valid
/// description arrives.
#[test]
fn a_label_mounted_with_refused_text_shows_no_text() -> TestResult {
    let log: Log<String> = Rc::default();
    let s = Stage::new(label("two\nlines", &log).key("name"))?;
    assert_eq!(s.text(), text(""));
    assert!(s.property("name", "binding_error").is_some());
    Ok(())
}

/// New behavior: a description with the same text keeps an open draft.
/// Detected by the assertion `s.editing()` after the redescription, which
/// fails when every description ends the draft, as a new binding does.
#[test]
fn a_same_text_description_keeps_the_draft() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    s.edit("Typed")?;
    s.redescribe(label("Old", &log).key("name"))?;
    s.flush()?;
    assert!(s.editing());
    assert_eq!(s.property("name", "draft"), text("Typed"));
    s.press(KeyCode::Enter)?;
    assert_eq!(*log.borrow(), ["Typed"]);
    Ok(())
}

/// New behavior: changed text ends the draft when it is described, even
/// when another description returns to the shown text before the next
/// flush. Detected by the assertion `!s.editing()`, which fails when the
/// draft ends only at update, by comparing the described text with the
/// shown text.
#[test]
fn a_changed_text_ends_the_draft_even_when_changed_back_before_a_flush() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    s.edit("Typed")?;
    s.redescribe(label("New", &log).key("name"))?;
    s.redescribe(label("Old", &log).key("name"))?;
    s.flush()?;
    assert!(!s.editing());
    assert_eq!(s.text(), text("Old"));
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert!(log.borrow().is_empty(), "{:?}", log.borrow());
    Ok(())
}

/// Contract: changed described text wins over the draft and ends it
/// unsaved, as an application write does with `new`.
#[test]
fn a_changed_text_ends_the_draft_unsaved() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    s.edit("Typed")?;
    s.redescribe(label("Renamed", &log).key("name"))?;
    s.flush()?;
    assert!(!s.editing());
    assert_eq!(s.text(), text("Renamed"));
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert!(log.borrow().is_empty());
    Ok(())
}

/// Contract: switching the same key between `new` and `controlled` while
/// editing ends the draft unsaved, either way.
#[test]
fn switching_between_new_and_controlled_ends_the_draft() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).key("name"))?;
    let value = s.ui.signal("Old".to_owned());
    s.edit("Typed")?;
    s.redescribe(InlineText::new(&value).key("name"))?;
    s.flush()?;
    assert!(!s.editing());
    s.edit("Again")?;
    s.redescribe(label("Old", &log).key("name"))?;
    s.flush()?;
    assert!(!s.editing());
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert_eq!(value.get()?, "Old");
    assert!(log.borrow().is_empty());
    Ok(())
}

/// Contract: each description replaces the callback; a callback error
/// propagates from the input call and is not replayed.
#[test]
fn the_save_callback_follows_the_description_and_errors_are_not_replayed() -> TestResult {
    let (old, new): (Log<String>, Log<String>) = (Rc::default(), Rc::default());
    let mut s = Stage::new(label("Old", &old).key("name"))?;
    s.redescribe(label("Old", &new).key("name"))?;
    s.flush()?;
    s.edit("Typed")?;
    s.press(KeyCode::Enter)?;
    assert!(old.borrow().is_empty());
    assert_eq!(*new.borrow(), ["Typed"]);
    let calls = Rc::new(RefCell::new(0));
    let count = calls.clone();
    s.redescribe(
        InlineText::controlled("Old", move |_, _| {
            *count.borrow_mut() += 1;
            Err(RuntimeError::Exhausted)
        })
        .key("name"),
    )?;
    s.flush()?;
    s.edit("Failing")?;
    assert_eq!(
        s.ui.key_input(KeyCode::Enter, KeyAction::Press).err(),
        Some(RuntimeError::Exhausted)
    );
    s.key(KeyCode::Enter, KeyAction::Release)?;
    s.flush()?;
    s.flush()?;
    assert_eq!(*calls.borrow(), 1);
    assert!(!s.editing());
    assert_eq!(s.text(), text("Old"));
    Ok(())
}

/// Contract: a later `on_save` builder replaces the constructor's callback,
/// at mount and by redescription: only the replacement runs, once per save.
#[test]
fn the_on_save_builder_replaces_the_constructor_callback() -> TestResult {
    let replaced = |constructor: &Log<String>, replacement: &Log<String>| {
        let later = replacement.clone();
        label("Old", constructor)
            .on_save(move |_, text| {
                later.borrow_mut().push(text.to_owned());
                Ok(())
            })
            .key("name")
    };
    let (constructor, replacement): (Log<String>, Log<String>) = (Rc::default(), Rc::default());
    let mut s = Stage::new(replaced(&constructor, &replacement))?;
    s.edit("Typed")?;
    s.press(KeyCode::Enter)?;
    assert!(
        constructor.borrow().is_empty(),
        "{:?}",
        constructor.borrow()
    );
    assert_eq!(*replacement.borrow(), ["Typed"]);
    // The same through a redescription of a plain controlled label.
    let plain: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &plain).key("name"))?;
    let (constructor, replacement): (Log<String>, Log<String>) = (Rc::default(), Rc::default());
    s.redescribe(replaced(&constructor, &replacement))?;
    s.flush()?;
    s.edit("Again")?;
    s.press(KeyCode::Enter)?;
    assert!(plain.borrow().is_empty());
    assert!(
        constructor.borrow().is_empty(),
        "{:?}",
        constructor.borrow()
    );
    assert_eq!(*replacement.borrow(), ["Again"]);
    Ok(())
}

/// Compatibility: `BeginEdit` and the row mode work as with `new`.
#[test]
fn begin_edit_and_row_mode_work_as_with_new() -> TestResult {
    let log: Log<String> = Rc::default();
    let mut s = Stage::new(label("Old", &log).in_row(true).key("name"))?;
    assert_eq!(s.property("name", "in_row"), Some(InspectValue::Bool(true)));
    let id = s.id("name")?;
    assert_eq!(
        s.ui.text_command(&id, TextCommand::BeginEdit)?,
        TextCommandOutcome::Handled
    );
    s.flush()?;
    assert!(s.editing());
    assert_eq!(
        s.ui.text_command(&id, TextCommand::BeginEdit)?,
        TextCommandOutcome::Unavailable
    );
    Ok(())
}

/// Compatibility: a bound label still writes its signal on save.
#[test]
fn a_bound_label_still_writes_its_signal() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal("Old".to_owned());
    let root = ui.mount_tree(Column::new([InlineText::new(&value).key("name")]).key("root"))?;
    ui.flush(Duration::ZERO)?;
    let mut s = Stage { ui, root };
    s.edit("New")?;
    s.press(KeyCode::Enter)?;
    s.flush()?;
    assert_eq!(value.get()?, "New");
    assert_eq!(s.text(), text("New"));
    Ok(())
}
