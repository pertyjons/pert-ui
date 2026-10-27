//! `EditorSession`: a consumer-defined host running one editor and learning
//! how each edit ended, on both key paths.
use crate::support;
use pert_ui::testing::HostIntegration;
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
                [(
                    EditorTrigger::Tab {
                        direction: direction(reverse),
                    },
                    EditorOutcome::Saved
                )],
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
                (
                    EditorTrigger::Tab {
                        direction: FocusDirection::Forward
                    },
                    EditorOutcome::Saved
                )
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
    // The plus button: the trailing icon square inside the padding.
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let [x, y, w, h] = s.ui.bounds(&editor).unwrap().unwrap().values();
    let metrics = s.ui.theme().metrics(Density::Standard).unwrap();
    let (pad, side) = (
        metrics.metric(MetricRole::ControlPaddingX),
        metrics.metric(MetricRole::IconSize),
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
                EditorTrigger::Tab {
                    direction: FocusDirection::Backward
                },
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

/// The selection of the editor or of the text editor inside it, as
/// inspection reports it.
fn drag_selection(s: &Setup) -> InspectValue {
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let root = s.ui.inspection_id(&editor).unwrap();
    let snapshot = s.ui.inspect();
    let mut subtree = vec![root];
    let mut grown = true;
    while grown {
        grown = false;
        for c in &snapshot.components {
            if !subtree.contains(&c.id) && c.parent.is_some_and(|p| subtree.contains(&p)) {
                subtree.push(c.id);
                grown = true;
            }
        }
    }
    snapshot
        .components
        .iter()
        .filter(|c| subtree.contains(&c.id))
        .find_map(|c| c.details.properties.get("selection").cloned())
        .expect("the editor reports a selection")
}
/// A collapsed selection at the start, which a drag from the left edge to
/// the middle cannot leave.
fn drag_selection_at_rest() -> InspectValue {
    InspectValue::Text("0..0".into())
}
/// Moves the held pointer back to where the drag began and checks that the
/// selection follows, so the drag `dragged` began is still active.
fn assert_drag_active(s: &mut Setup, dragged: &InspectValue) {
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    let [x, y, w, h] = s.ui.bounds(&editor).unwrap().unwrap().values();
    let _ = w;
    let back = Point::new(x + 12.0, y + h / 2.0).unwrap();
    s.ui.pointer_input(PointerEvent::new(back, PointerKind::Move))
        .unwrap();
    s.settle();
    assert_ne!(&drag_selection(s), dragged, "the drag still selects");
}

#[test]
fn a_selection_drag_keeps_enter_and_escape_from_the_session() {
    for native in [false, true] {
        let ui = Runtime::new();
        let (value, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("Laptop");
        begin_selection_drag(&mut s);
        let dragged = drag_selection(&s);
        assert_ne!(dragged, drag_selection_at_rest(), "native {native}");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Cable", "native {native}");
        assert_eq!(*saves.borrow(), 0);
        assert!(s.log().is_empty(), "native {native}");
        // Tab and Shift+Tab are kept for the drag: nothing saved or reported,
        // focus stays, and the drag is still active afterwards.
        for reverse in [false, true] {
            s.key(KeyCode::Tab, reverse, native);
            assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
            assert_eq!(value.get().unwrap(), "Cable", "native {native}");
            assert_eq!(*saves.borrow(), 0);
            assert!(s.log().is_empty(), "native {native}, reverse {reverse}");
        }
        assert_drag_active(&mut s, &dragged);
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
        let dragged = drag_selection(&s);
        s.key(KeyCode::Enter, false, native);
        for reverse in [false, true] {
            s.key(KeyCode::Tab, reverse, native);
            assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
            assert_eq!(value.get().unwrap(), 27, "native {native}");
            assert!(s.log().is_empty(), "native {native}, reverse {reverse}");
        }
        assert_drag_active(&mut s, &dragged);
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

/// The inspected property `name` of the widget keyed `key` under `root`.
fn property(ui: &Runtime, root: &WidgetId, key: &str, name: &str) -> Option<InspectValue> {
    let widget = support::widget(ui, root, key);
    let id = ui.inspection_id(&widget).unwrap();
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.id == id)
        .and_then(|c| c.details.properties.get(name).cloned())
}

/// Whether a widget keyed `key` is mounted and visible.
fn shown(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|c| c.key == key && c.visible)
}

type RowLog = Rc<RefCell<Vec<(usize, EditorTrigger, EditorOutcome)>>>;

/// A consumer-defined list host: each row rests as a button showing its
/// value, and pressing it edits that row in a session text field that trims
/// and requires a name. A Tab completion moves the edit to the next or
/// previous row, a refused one keeps the edit open, and any other terminal
/// completion closes it, focusing the row again unless focus left.
struct RowList {
    rows: Vec<Signal<String>>,
    /// The edited row; buttons and the host's delivery write it.
    editing: Signal<Option<usize>>,
    /// The row whose editor or resting buttons are built.
    built: Option<Option<usize>>,
    session: Option<EditorSession>,
    /// A child key to focus once it is mounted, then its id.
    focus_key: Option<String>,
    focus_id: Option<WidgetId>,
    notifier: Option<Notifier>,
    log: RowLog,
}
impl Component for RowList {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier()).clone();
        let session = self
            .session
            .get_or_insert_with(|| EditorSession::new(notifier.clone()))
            .clone();
        let editing = cx.read(&self.editing)?;
        if self.built != Some(editing) {
            self.built = Some(editing);
            // Begin before building the editor, which attaches to this edit.
            match editing {
                Some(_) => session.begin(EditorStart::SelectAll),
                None => session.end(),
            }
            let mut children = Vec::new();
            for (index, value) in self.rows.iter().enumerate() {
                children.push(if editing == Some(index) {
                    Element::new(
                        format!("editor-{index}"),
                        TextField::new(value.clone())
                            .normalize(|text: String| {
                                let text = text.trim();
                                if text.is_empty() {
                                    Err(InputRejection::new("A name is required"))
                                } else {
                                    Ok(text.to_owned())
                                }
                            })
                            .session(&session),
                    )
                } else {
                    let open = self.editing.clone();
                    Element::new(
                        format!("row-{index}"),
                        Button::new(value.get()?, move |_| open.set(Some(index))),
                    )
                });
            }
            cx.request_children(children);
            if let Some(index) = editing {
                self.focus_key = Some(format!("editor-{index}"));
            }
        } else if let Some(key) = self.focus_key.take() {
            match cx.children().iter().find(|child| child.key() == key) {
                Some(child) => {
                    self.focus_id = Some(child.id().clone());
                    notifier.request();
                }
                None => self.focus_key = Some(key),
            }
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(target) = self.focus_id.take() {
            cx.request_focus_within(&target)?;
        }
        let (Some(session), Some(Some(row))) = (self.session.clone(), self.built) else {
            return Ok(());
        };
        while let Some(done) = session.take_completion() {
            self.log
                .borrow_mut()
                .push((row, done.trigger, done.outcome));
            match (done.trigger, done.outcome) {
                (_, EditorOutcome::Refused) => {}
                (EditorTrigger::Tab { direction }, _) => {
                    let next = if direction == FocusDirection::Backward {
                        row.checked_sub(1)
                    } else {
                        Some(row + 1).filter(|next| *next < self.rows.len())
                    };
                    if next.is_none() {
                        self.focus_key = Some(format!("row-{row}"));
                    }
                    self.editing.set(next)?;
                }
                (EditorTrigger::FocusLeft, _) => self.editing.set(None)?,
                _ => {
                    self.focus_key = Some(format!("row-{row}"));
                    self.editing.set(None)?;
                }
            }
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        for index in 0..cx.len() {
            cx.measure(index, space)?;
        }
        Ok(Size::new(300.0, 40.0 * cx.len() as f32)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 40.0 * index as f32, 300.0, 40.0)?)?;
        }
        Ok(())
    }
}

struct ListSetup {
    ui: Runtime,
    root: WidgetId,
    rows: Vec<Signal<String>>,
    log: RowLog,
}
impl ListSetup {
    fn new(names: &[&str]) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let rows: Vec<_> = names
            .iter()
            .map(|name| ui.signal((*name).to_owned()))
            .collect();
        let log = RowLog::default();
        let root = ui
            .mount_tree(Element::new(
                "list",
                RowList {
                    rows: rows.clone(),
                    editing: ui.signal(None),
                    built: None,
                    session: None,
                    focus_key: None,
                    focus_id: None,
                    notifier: None,
                    log: log.clone(),
                },
            ))
            .unwrap();
        let mut setup = Self {
            ui,
            root,
            rows,
            log,
        };
        setup.settle();
        setup
    }
    fn settle(&mut self) {
        for _ in 0..4 {
            frame(&mut self.ui);
        }
    }
    fn focused(&self) -> Option<String> {
        self.ui
            .focused()
            .map(|id| self.ui.key(&id).unwrap().to_owned())
    }
    fn values(&self) -> Vec<String> {
        self.rows.iter().map(|row| row.get().unwrap()).collect()
    }
    fn log(&self) -> Vec<(usize, EditorTrigger, EditorOutcome)> {
        self.log.borrow().clone()
    }
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
    /// Presses the resting button of `row` with the pointer.
    fn click_row(&mut self, row: usize) {
        let button = support::widget(&self.ui, &self.root, &format!("row-{row}"));
        let [x, y, w, h] = self.ui.bounds(&button).unwrap().unwrap().values();
        let point = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(point, kind))
                .unwrap();
        }
        self.settle();
    }
}

#[test]
fn a_consumer_list_moves_its_edit_between_rows_and_keeps_a_refused_one_open() {
    use EditorOutcome::{Refused, Saved, Unchanged};
    for native in [false, true] {
        let mut s = ListSetup::new(&["Cable", "Mouse", "Toner"]);
        assert_eq!(s.focused(), None);
        s.click_row(0);
        assert_eq!(s.focused().as_deref(), Some("editor-0"), "native {native}");
        // The edit started with the value selected: typing replaces it.
        s.type_text("Laptop");
        s.key(KeyCode::Tab, false, native);
        assert_eq!(s.focused().as_deref(), Some("editor-1"), "native {native}");
        assert_eq!(s.values(), ["Laptop", "Mouse", "Toner"]);
        // A blank name is refused; the edit stays open on its row.
        s.type_text(" ");
        s.key(KeyCode::Tab, false, native);
        assert_eq!(s.focused().as_deref(), Some("editor-1"), "native {native}");
        assert_eq!(
            property(&s.ui, &s.root, "editor-1", "text"),
            Some(InspectValue::Text(" ".into()))
        );
        s.type_text("Monitor ");
        s.key(KeyCode::Tab, true, native);
        assert_eq!(s.focused().as_deref(), Some("editor-0"), "native {native}");
        assert_eq!(s.values(), ["Laptop", "Monitor", "Toner"]);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(s.focused().as_deref(), Some("row-0"), "native {native}");
        assert!(support::find(&s.ui, &s.root, "editor-0").is_none());
        // Tab from the last row closes the edit on it.
        s.click_row(2);
        assert_eq!(s.focused().as_deref(), Some("editor-2"));
        s.key(KeyCode::Tab, false, native);
        assert_eq!(s.focused().as_deref(), Some("row-2"), "native {native}");
        assert_eq!(
            s.log(),
            [
                (
                    0,
                    EditorTrigger::Tab {
                        direction: FocusDirection::Forward
                    },
                    Saved
                ),
                (
                    1,
                    EditorTrigger::Tab {
                        direction: FocusDirection::Forward
                    },
                    Refused
                ),
                (
                    1,
                    EditorTrigger::Tab {
                        direction: FocusDirection::Backward
                    },
                    Saved
                ),
                (0, EditorTrigger::Enter, Unchanged),
                (
                    2,
                    EditorTrigger::Tab {
                        direction: FocusDirection::Forward
                    },
                    Unchanged
                ),
            ],
            "native {native}"
        );
    }
}

impl Setup {
    /// The inspected property `name` of the editor.
    fn property(&self, name: &str) -> Option<InspectValue> {
        property(&self.ui, &self.root, "editor", name)
    }
}

#[test]
fn a_normalization_equivalent_draft_completes_unchanged() {
    for native in [false, true] {
        for code in [KeyCode::Enter, KeyCode::Tab] {
            let ui = Runtime::new();
            let (value, saves, build) = name_field(&ui, "Cable");
            let mut s = Setup::new(ui, EditorStart::SelectAll, build);
            s.type_text("  Cable ");
            // The draft differs from the value until it is normalized.
            assert_eq!(s.text(), InspectValue::Text("  Cable ".into()));
            assert_eq!(s.property("dirty"), Some(InspectValue::Bool(true)));
            s.key(code, false, native);
            assert_eq!(value.get().unwrap(), "Cable");
            assert_eq!(*saves.borrow(), 0, "native {native}, {code:?}");
            let trigger = if code == KeyCode::Enter {
                EditorTrigger::Enter
            } else {
                EditorTrigger::Tab {
                    direction: FocusDirection::Forward,
                }
            };
            assert_eq!(
                s.log(),
                [(trigger, EditorOutcome::Unchanged)],
                "native {native}"
            );
        }
    }
    // Focus leaving with an equivalent draft is unchanged too.
    let ui = Runtime::new();
    let (_, saves, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text("Cable  ");
    let other = s.ui.find_descendant(&s.root, "other").unwrap().unwrap();
    s.ui.focus(&other).unwrap();
    s.settle();
    assert_eq!(*saves.borrow(), 0);
    assert_eq!(
        s.log(),
        [(EditorTrigger::FocusLeft, EditorOutcome::Unchanged)]
    );
}

#[test]
fn a_refused_tab_keeps_the_draft_and_focus_for_a_retry() {
    for native in [false, true] {
        for reverse in [false, true] {
            let ui = Runtime::new();
            let (value, saves, build) = name_field(&ui, "Cable");
            let mut s = Setup::new(ui, EditorStart::SelectAll, build);
            s.type_text("   ");
            s.key(KeyCode::Tab, reverse, native);
            assert_eq!(value.get().unwrap(), "Cable", "native {native}");
            assert_eq!(*saves.borrow(), 0);
            assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
            assert_eq!(s.text(), InspectValue::Text("   ".into()));
            assert_eq!(
                s.log(),
                [(
                    EditorTrigger::Tab {
                        direction: direction(reverse),
                    },
                    EditorOutcome::Refused
                )],
                "native {native}, reverse {reverse}"
            );
            // The edit stayed open: a corrected draft completes it.
            s.type_text("Toner");
            s.key(KeyCode::Tab, reverse, native);
            assert_eq!(value.get().unwrap(), "Toner", "native {native}");
            assert_eq!(*saves.borrow(), 1);
            assert_eq!(
                s.log()[1..],
                [(
                    EditorTrigger::Tab {
                        direction: direction(reverse),
                    },
                    EditorOutcome::Saved
                )]
            );
        }
    }
}

#[test]
fn a_composition_keeps_enter_tab_and_escape_from_the_session() {
    for native in [false, true] {
        let ui = Runtime::new();
        let (value, saves, build) = name_field(&ui, "Cable");
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("Lap");
        s.ui.text_preedit(&TextPreedit::new("top", None).unwrap())
            .unwrap();
        s.settle();
        assert_eq!(s.property("composing"), Some(InspectValue::Bool(true)));
        s.key(KeyCode::Enter, false, native);
        s.key(KeyCode::Tab, false, native);
        assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
        assert_eq!(value.get().unwrap(), "Cable", "native {native}");
        assert!(s.log().is_empty(), "native {native}");
        // Escape ends the composition only; the draft and the edit stay.
        s.key(KeyCode::Escape, false, native);
        assert_eq!(s.property("composing"), Some(InspectValue::Bool(false)));
        assert_eq!(
            s.text(),
            InspectValue::Text("Lap".into()),
            "native {native}"
        );
        assert!(s.log().is_empty(), "native {native}");
        assert_eq!(*s.escapes.borrow(), 0);
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Lap");
        assert_eq!(*saves.borrow(), 1);
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
    }
}

#[test]
fn a_unicode_typed_start_replaces_an_active_composition() {
    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.ui.text_preedit(&TextPreedit::new("かな", Some(0..6)).unwrap())
        .unwrap();
    s.settle();
    assert_eq!(s.property("composing"), Some(InspectValue::Bool(true)));
    // A decomposed accent: two scalar values, one grapheme.
    s.restart(EditorStart::Typed("日本e\u{301}".into()));
    assert_eq!(s.property("composing"), Some(InspectValue::Bool(false)));
    assert_eq!(s.text(), InspectValue::Text("日本e\u{301}".into()));
    s.type_text("語");
    assert_eq!(s.text(), InspectValue::Text("日本e\u{301}語".into()));
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), "日本e\u{301}語");
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
}

/// A session text field over `value` whose editing is restricted by
/// `editable` and whose enablement by `enabled`.
fn permitted_field(
    value: &Signal<String>,
    editable: &Signal<bool>,
    enabled: &Signal<bool>,
) -> impl Fn(&EditorSession) -> Element + 'static {
    let (value, editable, enabled) = (value.clone(), editable.clone(), enabled.clone());
    move |session: &EditorSession| {
        Element::new(
            "editor",
            TextField::new(value.clone())
                .editable(&editable)
                .enabled(&enabled)
                .session(session),
        )
    }
}

#[test]
fn revoking_editing_refuses_a_dirty_draft_and_keeps_the_edit_open() {
    for native in [false, true] {
        let ui = Runtime::new();
        let value = ui.signal("Cable".to_owned());
        let (editable, enabled) = (ui.signal(true), ui.signal(true));
        let mut s = Setup::new(
            ui,
            EditorStart::SelectAll,
            permitted_field(&value, &editable, &enabled),
        );
        s.type_text("Laptop");
        editable.set(false).unwrap();
        s.settle();
        assert_eq!(s.property("read_only"), Some(InspectValue::Bool(true)));
        s.key(KeyCode::Enter, false, native);
        s.key(KeyCode::Tab, false, native);
        assert_eq!(value.get().unwrap(), "Cable", "native {native}");
        assert_eq!(s.text(), InspectValue::Text("Laptop".into()));
        assert_eq!(s.focused().as_deref(), Some("editor"), "native {native}");
        // Allowing editing again replays nothing; the next Enter saves.
        editable.set(true).unwrap();
        s.settle();
        assert_eq!(s.log().len(), 2, "native {native}");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), "Laptop", "native {native}");
        assert_eq!(
            s.log(),
            [
                (EditorTrigger::Enter, EditorOutcome::Refused),
                (
                    EditorTrigger::Tab {
                        direction: FocusDirection::Forward
                    },
                    EditorOutcome::Refused
                ),
                (EditorTrigger::Enter, EditorOutcome::Saved),
            ],
            "native {native}"
        );

        // A clean field made read-only completes unchanged, and Escape is
        // still the session's.
        let ui = Runtime::new();
        let value = ui.signal("Cable".to_owned());
        let (editable, enabled) = (ui.signal(true), ui.signal(true));
        let mut s = Setup::new(
            ui,
            EditorStart::SelectAll,
            permitted_field(&value, &editable, &enabled),
        );
        editable.set(false).unwrap();
        s.settle();
        assert_eq!(s.property("read_only"), Some(InspectValue::Bool(true)));
        s.key(KeyCode::Escape, false, native);
        assert_eq!(*s.escapes.borrow(), 0, "native {native}");
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
    }
}

#[test]
fn disabling_the_editor_keeps_its_draft_unsaved() {
    let ui = Runtime::new();
    let value = ui.signal("Cable".to_owned());
    let (editable, enabled) = (ui.signal(true), ui.signal(true));
    let mut s = Setup::new(
        ui,
        EditorStart::SelectAll,
        permitted_field(&value, &editable, &enabled),
    );
    s.type_text("Laptop");
    assert_eq!(s.focused().as_deref(), Some("editor"));
    enabled.set(false).unwrap();
    s.settle();
    assert_eq!(s.property("enabled"), Some(InspectValue::Bool(false)));
    assert_ne!(s.focused().as_deref(), Some("editor"));
    assert_eq!(value.get().unwrap(), "Cable");
    assert_eq!(s.text(), InspectValue::Text("Laptop".into()));
    // The editor lost focus without a focus loss: nothing is reported and
    // the edit stays open for the host to close or resume.
    assert!(s.log().is_empty());
    enabled.set(true).unwrap();
    s.settle();
    assert!(s.log().is_empty(), "enabling replays nothing");
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    s.ui.focus(&editor).unwrap();
    s.settle();
    assert_eq!(s.text(), InspectValue::Text("Laptop".into()));
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), "Laptop");
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);
}

#[test]
fn window_focus_loss_and_hiding_report_focus_left_but_hiding_the_editor_does_not() {
    // The window losing focus saves the draft, leaving the composition out.
    let ui = Runtime::new();
    let (value, saves, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text("Monitor");
    s.ui.text_preedit(&TextPreedit::new("s", None).unwrap())
        .unwrap();
    s.settle();
    assert_eq!(s.property("composing"), Some(InspectValue::Bool(true)));
    s.ui.clear_focus();
    s.settle();
    assert_eq!(s.focused(), None);
    assert_eq!(value.get().unwrap(), "Monitor");
    assert_eq!(*saves.borrow(), 1);
    assert_eq!(s.log(), [(EditorTrigger::FocusLeft, EditorOutcome::Saved)]);

    // Hiding the window clears focus the same way.
    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text("Monitor");
    assert_eq!(s.focused().as_deref(), Some("editor"));
    s.ui.set_window_visible(false).unwrap();
    s.settle();
    assert_eq!(s.focused(), None);
    assert_eq!(value.get().unwrap(), "Monitor");
    // A hidden window retains the host's notification until it is shown.
    assert!(s.log().is_empty());
    s.ui.set_window_visible(true).unwrap();
    s.settle();
    assert_eq!(s.log(), [(EditorTrigger::FocusLeft, EditorOutcome::Saved)]);

    // A refused draft is refused on the window's focus loss too.
    let ui = Runtime::new();
    let (value, _, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text(" ");
    s.ui.clear_focus();
    s.settle();
    assert_eq!(value.get().unwrap(), "Cable");
    assert_eq!(
        s.log(),
        [(EditorTrigger::FocusLeft, EditorOutcome::Refused)]
    );

    // Hiding the editor's view discards the draft and reports nothing: the
    // host caused it.
    let ui = Runtime::new();
    let (value, saves, build) = name_field(&ui, "Cable");
    let mut s = Setup::new(ui, EditorStart::SelectAll, build);
    s.type_text("Monitor");
    assert_eq!(s.focused().as_deref(), Some("editor"));
    let host = s.ui.find_descendant(&s.root, "host").unwrap().unwrap();
    s.ui.set_visible(&host, false).unwrap();
    s.settle();
    assert_ne!(s.focused().as_deref(), Some("editor"));
    assert_eq!(value.get().unwrap(), "Cable");
    assert_eq!(*saves.borrow(), 0);
    assert!(s.log().is_empty());
    // Shown and focused again, the editor holds the saved value, not the
    // draft, and completing it saves nothing.
    s.ui.set_visible(&host, true).unwrap();
    s.settle();
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    s.ui.focus(&editor).unwrap();
    s.settle();
    assert_eq!(s.focused().as_deref(), Some("editor"));
    assert_eq!(s.text(), InspectValue::Text("Cable".into()));
    assert_eq!(s.property("dirty"), Some(InspectValue::Bool(false)));
    s.key(KeyCode::Enter, false, false);
    assert_eq!(value.get().unwrap(), "Cable");
    assert_eq!(*saves.borrow(), 0);
    assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Unchanged)]);
}

/// Clicks the option at `row` of two in the open list of `s`'s dropdown.
fn click_option(s: &mut Setup, row: usize) {
    let list = support::widget(&s.ui, &s.root, "options");
    let [x, y, w, h] = s.ui.bounds(&list).unwrap().unwrap().values();
    let point = Point::new(x + w / 2.0, y + h * (row as f32 * 2.0 + 1.0) / 4.0).unwrap();
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        s.ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
    }
    s.settle();
}

#[test]
fn a_click_in_the_open_list_completes_the_dropdown_edit() {
    for (row, chosen, outcome) in [
        (1, "software", EditorOutcome::Saved),
        (0, "hardware", EditorOutcome::Unchanged),
    ] {
        let ui = Runtime::new();
        let value = ui.signal("hardware");
        let changes = Rc::new(RefCell::new(0));
        let (bound, counter) = (value.clone(), changes.clone());
        let mut s = Setup::with_owner(ui, EditorStart::SelectAll, move |cx, session| {
            let counter = counter.clone();
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
                .on_change(move |_, _| {
                    *counter.borrow_mut() += 1;
                    Ok(())
                })
                .session(session),
            )
        });
        s.key(KeyCode::ArrowDown, false, false);
        assert!(shown(&s.ui, "options"), "the list is open");
        // Moving focus into the list is no departure.
        assert!(s.log().is_empty());
        click_option(&mut s, row);
        assert!(!shown(&s.ui, "options"), "the click chose and closed");
        assert_eq!(value.get().unwrap(), chosen);
        assert_eq!(*changes.borrow(), usize::from(row == 1));
        assert_eq!(s.log(), [(EditorTrigger::Choice, outcome)]);
    }
}

fn price(
    ui: &Runtime,
    initial: f64,
) -> (Signal<f64>, impl Fn(&EditorSession) -> Element + 'static) {
    let value = ui.signal(initial);
    let bound = value.clone();
    let build = move |session: &EditorSession| {
        Element::new(
            "editor",
            DecimalInput::new(bound.clone())
                .range(0.0, 100.0)
                .unwrap()
                .session(session),
        )
    };
    (value, build)
}

#[test]
fn decimal_fields_report_through_the_session() {
    for native in [false, true] {
        // A typed start, completed by Enter.
        let ui = Runtime::new();
        let (value, build) = price(&ui, 2.25);
        let mut s = Setup::new(ui, EditorStart::Typed("7".into()), build);
        assert_eq!(s.text(), InspectValue::Text("7".into()));
        s.type_text(".5");
        s.key(KeyCode::Enter, false, native);
        assert_eq!(value.get().unwrap(), 7.5, "native {native}");
        assert_eq!(s.log(), [(EditorTrigger::Enter, EditorOutcome::Saved)]);

        // A draft that parses to the value is unchanged.
        let ui = Runtime::new();
        let (value, build) = price(&ui, 2.25);
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.type_text("2.250");
        assert_eq!(s.text(), InspectValue::Text("2.250".into()));
        s.key(KeyCode::Tab, false, native);
        assert_eq!(value.get().unwrap(), 2.25);
        assert_eq!(
            s.log(),
            [(
                EditorTrigger::Tab {
                    direction: FocusDirection::Forward
                },
                EditorOutcome::Unchanged
            )],
            "native {native}"
        );

        // Out of range is refused and kept; Escape then restores in one press.
        let ui = Runtime::new();
        let (value, build) = price(&ui, 2.25);
        let mut s = Setup::new(ui, EditorStart::CaretAtEnd, build);
        s.type_text("00");
        assert_eq!(s.text(), InspectValue::Text("2.2500".into()));
        s.text_replace("500.5");
        s.key(KeyCode::Tab, true, native);
        assert_eq!(value.get().unwrap(), 2.25, "native {native}");
        assert_eq!(s.text(), InspectValue::Text("500.5".into()));
        assert_eq!(s.focused().as_deref(), Some("editor"));
        s.key(KeyCode::Escape, false, native);
        assert_eq!(value.get().unwrap(), 2.25);
        assert_eq!(
            s.text(),
            InspectValue::Text("2.25".into()),
            "native {native}"
        );
        assert_eq!(*s.escapes.borrow(), 0);
        assert_eq!(
            s.log(),
            [
                (
                    EditorTrigger::Tab {
                        direction: FocusDirection::Backward
                    },
                    EditorOutcome::Refused
                ),
                (EditorTrigger::Escape, EditorOutcome::Cancelled)
            ],
            "native {native}"
        );

        // A step changes the draft only.
        let ui = Runtime::new();
        let (value, build) = price(&ui, 2.25);
        let mut s = Setup::new(ui, EditorStart::SelectAll, build);
        s.key(KeyCode::ArrowUp, false, native);
        assert_ne!(
            s.text(),
            InspectValue::Text("2.25".into()),
            "native {native}"
        );
        assert_eq!(value.get().unwrap(), 2.25, "native {native}");
        assert!(s.log().is_empty());
        s.key(KeyCode::Escape, false, native);
        assert_eq!(s.text(), InspectValue::Text("2.25".into()));
        assert_eq!(s.log(), [(EditorTrigger::Escape, EditorOutcome::Cancelled)]);
    }
}

impl Setup {
    /// Selects the editor's whole text and types `text` over it.
    fn text_replace(&mut self, text: &str) {
        let editor = self
            .ui
            .find_descendant(&self.root, "editor")
            .unwrap()
            .unwrap();
        self.ui
            .text_command(&editor, TextCommand::SelectAll)
            .unwrap();
        self.settle();
        self.type_text(text);
    }
}

/// Drives a parameter-bound number field, with or without a session, through
/// an invalid and changed drafts, completion keys and a focus departure, and
/// returns what each step left: the editor's text, focus, the parameter's
/// base and the outcomes its host saw, plus the session's reports.
fn parameter_script(decimal: bool, native: bool, with_session: bool) -> (Vec<String>, Setup) {
    let mut ui = Runtime::new();
    let (voices, voices_host) = ui
        .parameter(
            ParameterId::new(1, 1, "voices").unwrap(),
            ParameterSpec::stepped("Voices", 0, 16, 4).unwrap(),
        )
        .unwrap();
    let (gain, gain_host) = ui
        .parameter(
            ParameterId::new(1, 2, "gain").unwrap(),
            ParameterSpec::new("Gain", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    let proposals = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = proposals.clone();
    let _voices_monitor = voices_host
        .on_outcome(move |event| {
            sink.borrow_mut().push(format!("{:?}", event.outcome));
            Ok(())
        })
        .unwrap();
    let sink = proposals.clone();
    let _gain_monitor = gain_host
        .on_outcome(move |event| {
            sink.borrow_mut().push(format!("{:?}", event.outcome));
            Ok(())
        })
        .unwrap();
    let (built_voices, built_gain) = (voices.clone(), gain.clone());
    let mut s = Setup::new(ui, EditorStart::Typed("1".into()), move |session| {
        if decimal {
            let field = DecimalInput::for_parameter(built_gain.clone());
            let field = if with_session {
                field.session(session)
            } else {
                field
            };
            Element::new("editor", field)
        } else {
            let field = IntegerInput::for_parameter(built_voices.clone());
            let field = if with_session {
                field.session(session)
            } else {
                field
            };
            Element::new("editor", field)
        }
    });
    let base = |decimal: bool| {
        if decimal {
            format!("{:?}", gain.snapshot().get().unwrap().base)
        } else {
            format!("{:?}", voices.snapshot().get().unwrap().base)
        }
    };
    let [invalid, first, second, third] = if decimal {
        ["1.5", "0.25", "0.75", "1"]
    } else {
        ["41", "8", "12", "2"]
    };
    let mut trace = Vec::new();
    let mut observe = |s: &Setup, step: &str| {
        trace.push(format!(
            "{step}: text {:?}, focus {:?}, base {}, proposals {:?}, escapes {}",
            s.text(),
            s.focused(),
            base(decimal),
            proposals.borrow(),
            s.escapes.borrow()
        ));
    };
    observe(&s, "mounted");
    s.key(KeyCode::Escape, false, native);
    observe(&s, "escape at rest");
    s.text_replace(invalid);
    s.key(KeyCode::Enter, false, native);
    observe(&s, "enter on an invalid draft");
    s.key(KeyCode::Tab, false, native);
    observe(&s, "tab on an invalid draft");
    let editor = s.ui.find_descendant(&s.root, "editor").unwrap().unwrap();
    s.ui.focus(&editor).unwrap();
    s.text_replace(first);
    s.key(KeyCode::Enter, false, native);
    observe(&s, "enter on a changed draft");
    s.text_replace(second);
    s.key(KeyCode::Escape, false, native);
    observe(&s, "escape on a changed draft");
    s.text_replace(second);
    s.key(KeyCode::Tab, true, native);
    observe(&s, "shift-tab on a changed draft");
    s.ui.focus(&editor).unwrap();
    s.text_replace(third);
    let other = s.ui.find_descendant(&s.root, "other").unwrap().unwrap();
    s.ui.focus(&other).unwrap();
    s.settle();
    observe(&s, "focus departure with a changed draft");
    drop(observe);
    (trace, s)
}

#[test]
fn parameter_number_fields_ignore_the_session() {
    for (decimal, native) in [(false, false), (false, true), (true, false), (true, true)] {
        let case = format!("decimal {decimal}, native {native}");
        let (with, s) = parameter_script(decimal, native, true);
        let (without, plain) = parameter_script(decimal, native, false);
        // The session changed nothing and heard nothing.
        assert_eq!(with, without, "{case}");
        assert!(s.log().is_empty(), "{case}");
        assert!(plain.log().is_empty(), "{case}");
        // The script did what it claims: the start was not applied, Escape
        // at rest bubbled, the invalid draft was not proposed, and the
        // changed draft committed through the parameter.
        let (initial, committed) = if decimal { ("0.5", "0.25") } else { ("4", "8") };
        assert!(
            with[0].starts_with(&format!(
                "mounted: text Text({initial:?}), focus Some(\"editor\")"
            )),
            "{case}: {}",
            with[0]
        );
        assert!(with[1].ends_with("escapes 1"), "{case}: {}", with[1]);
        assert!(with[2].contains("proposals []"), "{case}: {}", with[2]);
        assert!(
            with[4].contains(&format!("base {committed}")) && with[4].contains("Committed"),
            "{case}: {}",
            with[4]
        );
        // Escape restored the changed draft, and Shift+Tab and the focus
        // departure cancelled theirs instead of proposing them.
        assert!(
            with[5].starts_with(&format!(
                "escape on a changed draft: text Text({committed:?})"
            )),
            "{case}: {}",
            with[5]
        );
        for step in &with[6..] {
            assert!(
                step.contains(&format!("base {committed}, proposals [\"Committed"))
                    && step.matches("Committed").count() == 1,
                "{case}: {step}"
            );
        }
    }
}

/// The traversal direction of a Tab press with Shift held (`reverse`) or not.
fn direction(reverse: bool) -> FocusDirection {
    if reverse {
        FocusDirection::Backward
    } else {
        FocusDirection::Forward
    }
}
