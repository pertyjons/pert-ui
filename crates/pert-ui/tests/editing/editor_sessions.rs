//! `EditorSession`: a consumer-defined host running one editor and learning
//! how each edit ended, on both key paths.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};
use support::frame;

type Build = Box<dyn Fn(&mut UpdateContext, &EditorSession) -> Element>;
type Log = Rc<RefCell<Vec<EditorCompletion>>>;

/// Opens one editor for one edit and records how it ended. It also counts
/// Escape presses that reach it, which a session editor must keep. A new
/// `restart` generation begins another edit, starting as `next`, and
/// describes the editor again in place, focus kept.
struct Host {
    start: EditorStart,
    restart: Signal<u32>,
    next: Rc<RefCell<EditorStart>>,
    seen: u32,
    build: Build,
    session: Option<EditorSession>,
    editor: Option<WidgetId>,
    focused: bool,
    notifier: Option<Notifier>,
    log: Log,
    escapes: Rc<RefCell<u32>>,
}
impl Component for Host {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        let restart = cx.read(&self.restart)?;
        if self.session.is_none() {
            let session = EditorSession::new(notifier);
            session.begin(self.start.clone());
            let editor = (self.build)(cx, &session);
            cx.request_children(vec![editor]);
            self.session = Some(session);
            self.seen = restart;
        } else if restart != self.seen
            && let Some(session) = self.session.clone()
        {
            self.seen = restart;
            session.begin(self.next.borrow().clone());
            let editor = (self.build)(cx, &session);
            cx.request_children(vec![editor]);
        } else if self.editor.is_none()
            && let Some(child) = cx.children().first()
        {
            self.editor = Some(child.id().clone());
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(editor) = &self.editor
            && !self.focused
        {
            self.focused = true;
            cx.request_focus_within(editor)?;
        }
        while let Some(done) = self
            .session
            .as_ref()
            .and_then(EditorSession::take_completion)
        {
            self.log.borrow_mut().push(done);
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::new(0.0, 0.0)?);
        }
        cx.measure(0, space)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Escape && event.transition == KeyTransition::Pressed {
            *self.escapes.borrow_mut() += 1;
        }
        Ok(InputOutcome::Ignored)
    }
}

struct Setup {
    ui: Runtime,
    root: WidgetId,
    log: Log,
    escapes: Rc<RefCell<u32>>,
    restart: Signal<u32>,
    next: Rc<RefCell<EditorStart>>,
}
impl Setup {
    /// Mounts a host whose editor `build` makes, then lets it open and focus
    /// the editor.
    fn new(
        ui: Runtime,
        start: EditorStart,
        build: impl Fn(&EditorSession) -> Element + 'static,
    ) -> Self {
        Self::with_owner(ui, start, move |_, session| build(session))
    }
    /// As [`Self::new`], for an editor that needs a state owner.
    fn with_owner(
        ui: Runtime,
        start: EditorStart,
        build: impl Fn(&mut UpdateContext, &EditorSession) -> Element + 'static,
    ) -> Self {
        let mut setup = Self::mount(ui, start, build);
        setup.settle();
        setup
    }
    /// As [`Self::with_owner`], before the first flush.
    fn mount(
        mut ui: Runtime,
        start: EditorStart,
        build: impl Fn(&mut UpdateContext, &EditorSession) -> Element + 'static,
    ) -> Self {
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let log = Log::default();
        let escapes = Rc::new(RefCell::new(0));
        let restart = ui.signal(0);
        let next = Rc::new(RefCell::new(EditorStart::SelectAll));
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    Element::new(
                        "host",
                        Host {
                            start,
                            restart: restart.clone(),
                            next: next.clone(),
                            seen: 0,
                            build: Box::new(build),
                            session: None,
                            editor: None,
                            focused: false,
                            notifier: None,
                            log: log.clone(),
                            escapes: escapes.clone(),
                        },
                    ),
                    Element::new("other", Button::new("Other", |_| Ok(()))),
                ]),
            ))
            .unwrap();
        Self {
            ui,
            root,
            log,
            escapes,
            restart,
            next,
        }
    }
    /// Begins another edit starting as `start`, with the editor described
    /// again over the same binding.
    fn restart(&mut self, start: EditorStart) {
        *self.next.borrow_mut() = start;
        let generation = self.restart.get().unwrap();
        self.restart.set(generation + 1).unwrap();
        self.settle();
    }
    fn settle(&mut self) {
        for _ in 0..3 {
            frame(&mut self.ui);
        }
    }
    fn focused(&self) -> Option<String> {
        self.ui
            .focused()
            .map(|id| self.ui.key(&id).unwrap().to_owned())
    }
    fn log(&self) -> Vec<(EditorTrigger, EditorOutcome)> {
        self.log
            .borrow()
            .iter()
            .map(|done| (done.trigger, done.outcome))
            .collect()
    }
    /// Presses and releases `code`, with Shift held when `shift`, on the
    /// headless or the native path.
    fn key(&mut self, code: KeyCode, shift: bool, native: bool) {
        let send = |ui: &mut Runtime, code: KeyCode, action| {
            if native {
                ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                    .map(|_| ())
            } else {
                ui.key_input(code, action).map(|_| ())
            }
        };
        if shift {
            send(&mut self.ui, KeyCode::ShiftLeft, KeyAction::Press).unwrap();
        }
        send(&mut self.ui, code, KeyAction::Press).unwrap();
        send(&mut self.ui, code, KeyAction::Release).unwrap();
        if shift {
            send(&mut self.ui, KeyCode::ShiftLeft, KeyAction::Release).unwrap();
        }
        self.settle();
    }
    fn type_text(&mut self, text: &str) {
        self.ui.text_input(text).unwrap();
        self.settle();
    }
    fn text(&self) -> InspectValue {
        let editor = self
            .ui
            .find_descendant(&self.root, "editor")
            .unwrap()
            .unwrap();
        let id = self.ui.inspection_id(&editor).unwrap();
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .details
            .properties["text"]
            .clone()
    }
}

/// A text field over `name` with a session and an `on_save` counter.
fn name_field(
    ui: &Runtime,
    name: &str,
) -> (
    Signal<String>,
    Rc<RefCell<u32>>,
    impl Fn(&EditorSession) -> Element + 'static,
) {
    let value = ui.signal(name.to_owned());
    let saves = Rc::new(RefCell::new(0));
    let (bound, counter) = (value.clone(), saves.clone());
    let build = move |session: &EditorSession| {
        let counter = counter.clone();
        Element::new(
            "editor",
            TextField::new(bound.clone())
                .normalize(|text: String| {
                    if text.trim().is_empty() {
                        Err(InputRejection::new("A name is required"))
                    } else {
                        Ok(text.trim().to_owned())
                    }
                })
                .on_save(move |_, _| {
                    *counter.borrow_mut() += 1;
                    Ok(())
                })
                .session(session),
        )
    };
    (value, saves, build)
}

#[test]
fn enter_reports_saved_unchanged_and_refused() {
    for native in [false, true] {
        // Typing over the selected value, then Enter.
        let ui = Runtime::new();
        let (value, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
        s.type_text("Laptop");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Laptop");
        assert_eq!(*saves.borrow(), 1);
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);

        // Enter on an unchanged draft writes nothing and calls no on_save.
        let ui = Runtime::new();
        let (_, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(*saves.borrow(), 0);
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Unchanged)]);

        // A refused draft keeps the edit open for a retry.
        let ui = Runtime::new();
        let (value, _, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text(" ");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(s.focused().as_deref(), Some("editor"));
        s.type_text("Toner");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Toner");
        assert_eq!(
            s.log(),
            [
                (EditorTrigger::Enter, EditorOutcome::Refused),
                (EditorTrigger::Enter, EditorOutcome::Saved)
            ],
            "native {native}"
        );
    }
}

#[test]
fn escape_cancels_in_one_press_and_stays_with_the_editor() {
    for native in [false, true] {
        let ui = Runtime::new();
        let (value, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("Laptop");
        s.key(KeyCode::Escape, false, native);
        assert_eq!(value.get().unwrap(), "Cable");
        assert_eq!(s.text(), InspectValue::Text("Cable".into()));
        assert_eq!(*saves.borrow(), 0);
        assert_eq!(*s.escapes.borrow(), 0, "native {native}");
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
    }
}

#[test]
fn tab_confirms_and_keeps_focus_for_the_host_to_move() {
    for native in [false, true] {
        for reverse in [false, true] {
            let ui = Runtime::new();
            let (value, _, build) = name_field(&ui, "Cable");
            let mut s = Setup::new(ui, EditorStart::SelectAll, build);
            s.type_text("Paper");
            s.key(KeyCode::Tab, reverse, native);
            assert_eq!(value.get().unwrap(), "Paper");
            assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
            assert_eq!(
                s.log(),
                [(EditorTrigger::Tab { reverse }, EditorOutcome::Saved)],
                "native {native}, reverse {reverse}"
            );
        }
    }
}

#[test]
fn starts_select_place_the_caret_or_replace_with_typed_text() {
    let ui = Runtime::new();
    let (_, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::Typed("é".into()), build);
    assert_eq!(s.text(), InspectValue::Text("é".into()));
    s.type_text("t");
    assert_eq!(s.text(), InspectValue::Text("ét".into()));

    let ui = Runtime::new();
    let (_, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::CaretAtEnd, build);
    s.type_text("s");
    assert_eq!(s.text(), InspectValue::Text("Cables".into()));
}

#[test]
fn focus_leaving_reports_once_and_not_after_a_terminal_report() {
    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text("Monitor");
    let other = s.ui.find_descendant(&s.root, "other").unwrap().unwrap();
    s.ui.focus(&other).unwrap();
    s.settle();
    assert_eq!(value.get().unwrap(), "Monitor");
    assert_eq!(s.log(), [(EditorTrigger::FocusLeft, EditorOutcome::Saved)]);

    // After Enter ended the edit, leaving reports nothing more.
    let ui = Runtime::new();
    let (_, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.key(KeyCode::Enter, false, false);
    let other = s.ui.find_descendant(&s.root, "other").unwrap().unwrap();
    s.ui.focus(&other).unwrap();
    s.settle();
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Unchanged)]);
}

#[test]
fn a_failing_on_save_keeps_the_write_and_reports_at_the_next_flush() {
    let ui = Runtime::new();
    let value = ui.signal("Cable".to_owned());
    let bound = value.clone();
    let mut s = Setup::new(ui, EditorStart::SelectAll, move |session| {
        Element::new(
            "editor",
            TextField::new(bound.clone())
                .on_save(|_, _| Err(RuntimeError::Exhausted))
                .session(session),
        )
    });
    s.type_text("Toner");
    let pressed = s.ui.key_input(KeyCode::Enter, KeyAction::Press);
    assert_eq!(pressed.map(|_| ()), Err(RuntimeError::Exhausted));
    s.ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    s.settle();
    assert_eq!(value.get().unwrap(), "Toner");
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
}

fn quantity(
    ui: &Runtime,
    initial: i64,
) -> (Signal<i64>, impl Fn(&EditorSession) -> Element + 'static) {
    let value = ui.signal(initial);
    let bound = value.clone();
    let build = move |session: &EditorSession| {
        Element::new(
            "editor",
            IntegerInput::new(bound.clone())
                .range(0, 1000)
                .unwrap()
                .stepper(true)
                .session(session),
        )
    };
    (value, build)
}

#[test]
fn number_fields_report_through_the_session() {
    for native in [false, true] {
        // Typed text replaces the value; Enter saves it.
        let ui = Runtime::new();
        let (value, build) = quantity(&ui, 27);
        let mut s = Setup::new(ui, EditorStart::Typed("1".into()), build);
        s.type_text("2");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), 12, "native {native}");
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);

        // An out-of-range draft is refused and stays; Tab after fixing it saves.
        let ui = Runtime::new();
        let (value, build) = quantity(&ui, 27);
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("5000");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), 27);
        s.key(KeyCode::Backspace, false, native);
        s.key(KeyCode::Tab, false, native);
        assert_eq!(value.get().unwrap(), 500, "native {native}");
        assert_eq!(
            s.log(),
            [
                (EditorTrigger::Enter, EditorOutcome::Refused),
                (EditorTrigger::Tab { reverse: false }, EditorOutcome::Saved)
            ],
            "native {native}"
        );

        // Escape restores in one press and keeps the key.
        let ui = Runtime::new();
        let (value, build) = quantity(&ui, 27);
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("9");
        s.key(KeyCode::Escape, false, native);
        assert_eq!(value.get().unwrap(), 27);
        assert_eq!(s.text(), InspectValue::Text("27".into()));
        assert_eq!(*s.escapes.borrow(), 0);
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
    }
}

#[test]
fn steps_edit_the_draft_while_a_session_is_open() {
    let ui = Runtime::new();
    let (value, build) = quantity(&ui, 27);
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.key(KeyCode::ArrowUp, false, false);
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let _ = editor;
    // The plus button: the trailing icon square inside the padding.
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let [x, y, w, h] = s.ui.bounds(&editor).unwrap().unwrap().values();
    let metrics = s.ui.theme().metrics(Density::Standard).unwrap();
    let (pad, side) = (
        metrics.get(MetricRole::ControlPaddingX),
        metrics.get(MetricRole::IconSize),
    );
    let point = Point::new(x + w - pad - side / 2.0, y + h / 2.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    s.settle();
    // Two steps are in the draft, none in the binding.
    assert_eq!(s.text(), InspectValue::Text("29".into()));
    assert_eq!(value.get().unwrap(), 27, "steps did not save");
    s.key(KeyCode::Escape, false, false);
    assert_eq!(value.get().unwrap(), 27);
    assert_eq!(s.text(), InspectValue::Text("27".into()));
}

/// A session dropdown over `value` inside a host.
fn dropdown(value: &Signal<&'static str>, ui: Runtime) -> Setup {
    let bound = value.clone();
    Setup::with_owner(ui, EditorStart::SelectAll, move |cx, session| {
        Element::new(
            "editor",
            Select::new(
                cx,
                bound.clone(),
                [
                    Choice::new("hardware", "Hardware"),
                    Choice::new("software", "Software"),
                ],
            )
            .unwrap()
            .session(session),
        )
    })
}

#[test]
fn a_dropdown_reports_choices_cancellation_and_tab() {
    for native in [false, true] {
        // ArrowDown opens the list, ArrowDown moves, Enter chooses.
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let mut s = dropdown(&value, ui);
        s.key(KeyCode::ArrowDown, false, native);
        s.key(KeyCode::ArrowDown, false, native);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "software", "native {native}");
        assert_eq!(s.log(), [(EditorTrigger::Choice, EditorOutcome::Saved)]);

        // Choosing the current option is a completion too.
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let mut s = dropdown(&value, ui);
        s.key(KeyCode::ArrowDown, false, native);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(s.log(), [(EditorTrigger::Choice, EditorOutcome::Unchanged)]);

        // Escape in the open list closes it only; on the trigger it cancels.
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let mut s = dropdown(&value, ui);
        s.key(KeyCode::ArrowDown, false, native);
        s.key(KeyCode::Escape, false, native);
        assert!(s.log().is_empty(), "native {native}");
        s.key(KeyCode::Escape, false, native);
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
        assert_eq!(*s.escapes.borrow(), 0);

        // Tab on the closed trigger confirms unchanged and keeps focus.
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let mut s = dropdown(&value, ui);
        s.key(KeyCode::Tab, true, native);
        assert_eq!(
            s.log(),
            [(
                EditorTrigger::Tab { reverse: true },
                EditorOutcome::Unchanged
            )],
            "native {native}"
        );
    }
}

#[test]
fn shown_help_takes_the_first_escape_before_the_editor() {
    for native in [false, true] {
        let ui = Runtime::new();
        let value = ui.signal("Cable".to_owned());
        let bound = value.clone();
        let mut s = Setup::new(ui, EditorStart::SelectAll, move |session| {
            Element::new(
                "tip",
                Tooltip::new(
                    Element::new("editor", TextField::new(bound.clone()).session(session)),
                    "Name of the item",
                ),
            )
        });
        assert_eq!(s.focused().as_deref(), Some("editor"));
        s.type_text("Laptop");
        let later = s.ui.now() + std::time::Duration::from_millis(600);
        s.ui.flush(later).unwrap();
        s.key(KeyCode::Escape, false, native);
        assert!(s.log().is_empty(), "native {native}: help went first");
        s.key(KeyCode::Escape, false, native);
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
        assert_eq!(value.get().unwrap(), "Cable");
    }
}

/// A consumer-defined editor: a focusable box that takes its start and
/// reports Enter through the public attachment.
struct Custom {
    attachment: EditorAttachment,
    started: Rc<RefCell<Option<EditorStart>>>,
}
impl Component for Custom {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn focus_gained(&mut self, _: &mut EventContext) {
        if let Some(start) = self.attachment.take_start() {
            *self.started.borrow_mut() = Some(start);
        }
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(80.0, 24.0)?)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Enter && event.transition == KeyTransition::Pressed {
            self.attachment
                .report(EditorTrigger::Enter, EditorOutcome::Saved);
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}

#[test]
fn a_custom_editor_follows_the_protocol_through_the_public_api() {
    let ui = Runtime::new();
    let started = Rc::new(RefCell::new(None));
    let seen = started.clone();
    let mut s = Setup::new(ui, EditorStart::Typed("x".into()), move |session| {
        Element::new(
            "editor",
            Custom {
                attachment: session.attach(),
                started: seen.clone(),
            },
        )
    });
    assert_eq!(*started.borrow(), Some(EditorStart::Typed("x".into())));
    s.key(KeyCode::Enter, false, false);
    s.key(KeyCode::Enter, false, false);
    // The first report ended the edit; the second is dropped.
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
}

/// Presses the primary button inside `key`'s left edge and drags to its
/// middle, leaving the button down.
fn begin_selection_drag(s: &mut Setup) {
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let [x, y, w, h] = s.ui.bounds(&editor).unwrap().unwrap().values();
    let from = Point::new(x + 12.0, y + h / 2.0).unwrap();
    let to = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        from,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    s.ui.pointer_input(PointerEvent::new(to, PointerKind::Move))
        .unwrap();
    s.settle();
}
fn end_selection_drag(s: &mut Setup) {
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let [x, y, w, h] = s.ui.bounds(&editor).unwrap().unwrap().values();
    let to = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(
        to,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    s.settle();
}

#[test]
fn a_selection_drag_keeps_enter_and_escape_from_the_session() {
    for native in [false, true] {
        let ui = Runtime::new();
        let (value, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("Laptop");
        begin_selection_drag(&mut s);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Cable", "native {native}");
        assert_eq!(*saves.borrow(), 0);
        assert!(s.log().is_empty(), "native {native}");
        end_selection_drag(&mut s);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Laptop");
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);

        // Escape during a number field's drag ends the drag, not the draft.
        let ui = Runtime::new();
        let (value, build) = quantity(&ui, 27);
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("9");
        begin_selection_drag(&mut s);
        s.key(KeyCode::Enter, false, native);
        s.key(KeyCode::Escape, false, native);
        assert_eq!(value.get().unwrap(), 27, "native {native}");
        assert_eq!(s.text(), InspectValue::Text("9".into()), "native {native}");
        assert!(s.log().is_empty(), "native {native}");
        end_selection_drag(&mut s);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), 9);
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
    }
}

#[test]
fn a_new_edit_starts_in_an_editor_that_keeps_focus() {
    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.restart(EditorStart::Typed("Z".into()));
    assert_eq!(s.text(), InspectValue::Text("Z".into()));
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), "Z");
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);

    let ui = Runtime::new();
    let (_, build) = quantity(&ui, 27);
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.restart(EditorStart::Typed("5".into()));
    assert_eq!(s.text(), InspectValue::Text("5".into()));
}

#[test]
fn typed_starts_may_be_empty_and_refuse_line_breaks() {
    let ui = Runtime::new();
    let (_, _, build) = name_field(&ui, "Cable");
    let s = Setup::new(ui, EditorStart::Typed(String::new()), build);
    assert_eq!(s.text(), InspectValue::Text(String::new()));

    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::mount(ui, EditorStart::Typed("a\nb".into()), move |_, session| {
        build(session)
    });
    let failures: Vec<_> = (0..4)
        .filter_map(|_| s.ui.flush(s.ui.now()).err())
        .collect();
    assert_eq!(
        failures,
        [RuntimeError::TextEdit(TextEditError::InvalidText)]
    );
    // The value stays, selected, and the edit stays open.
    assert_eq!(s.text(), InspectValue::Text("Cable".into()));
    s.type_text("Mouse");
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), "Mouse");
}

#[test]
fn an_open_list_completes_only_the_edit_it_opened_in() {
    for native in [false, true] {
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let mut s = dropdown(&value, ui);
        s.key(KeyCode::ArrowDown, false, native);
        s.restart(EditorStart::SelectAll);
        // The list opened in the first edit; its choice cannot end the second.
        s.key(KeyCode::Enter, false, native);
        assert!(s.log().is_empty(), "native {native}");
        s.key(KeyCode::Escape, false, native);
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
    }
}

#[test]
fn a_parameter_dropdown_ignores_the_session() {
    let mut ui = Runtime::new();
    let id = |text| ChoiceId::new(text).unwrap();
    let spec = ParameterSpec::choices(
        "Waveform",
        [
            ChoiceOption::new(id("sine"), "Sine"),
            ChoiceOption::new(id("saw"), "Saw"),
        ],
        id("sine"),
    )
    .unwrap();
    let (waveform, _host) = ui
        .parameter(ParameterId::new(1, 1, "waveform").unwrap(), spec)
        .unwrap();
    let mut s = Setup::with_owner(ui, EditorStart::SelectAll, move |cx, session| {
        Element::new(
            "editor",
            Select::for_parameter(cx, waveform.clone()).session(session),
        )
    });
    // Escape bubbles and Tab traverses as without a session.
    s.key(KeyCode::Escape, false, false);
    assert_eq!(*s.escapes.borrow(), 1);
    s.key(KeyCode::Tab, false, false);
    assert_eq!(s.focused().as_deref(), Some("other"));
    assert!(s.log().is_empty());
}

#[test]
fn a_number_field_refused_since_mount_completes_unchanged() {
    let ui = Runtime::new();
    let (value, build) = quantity(&ui, 2000);
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), 2000);
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Unchanged)]);
}
