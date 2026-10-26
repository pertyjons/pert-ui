//! The component-authoring module: a custom component that describes itself
//! for inspection, keeps a task receipt, starts a drag, claims Tab and saves
//! its workspace state is written from `pert_ui::component::*` alone.
//!
//! The `widget` module must not import anything else from `pert_ui`: its
//! compiling is the evidence that the module holds every name such a component
//! writes. `every_name` imports each promised name on its own, so a removed
//! re-export fails even where the probe does not use it. The test driving it is ordinary application code and globs the
//! prelude.
use pert_ui::{ComponentInspection, OverlapPolicy, SemanticRole, prelude::*};
use std::time::Duration;

mod widget {
    use pert_ui::component::*;
    use std::{cell::RefCell, rc::Rc};

    /// What the probe recorded, shared with the test.
    #[derive(Default)]
    pub struct Record {
        pub task: Option<TaskReceipt>,
        pub drags: Vec<DragOutcome>,
        pub tabs: Vec<FocusDirection>,
    }

    /// The options a component asks for when it presents in its own window.
    pub fn own_window() -> Result<NativeWindowOptions, RuntimeError> {
        Ok(NativeWindowOptions::new("Probe")?.close_policy(DetachedClosePolicy::RemoveView))
    }

    /// Whether a kept reveal ended refused by a modal surface.
    pub fn modal_refused(outcome: &RevealOutcome) -> bool {
        matches!(outcome, RevealOutcome::Refused(RevealRefusal::ModalBlocked))
    }

    impl Record {
        pub fn task_succeeded(&self) -> bool {
            self.task
                .as_ref()
                .is_some_and(|task| matches!(task.status(), TaskStatus::Succeeded))
        }
    }

    pub struct Probe {
        pub record: Rc<RefCell<Record>>,
        pub armed: Option<Point>,
    }

    impl Component for Probe {
        fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn focusable(&self) -> bool {
            true
        }
        fn measure(
            &self,
            _cx: &MeasureContext<'_>,
            _available: Constraints,
        ) -> Result<Size, RuntimeError> {
            Ok(Size::new(80.0, 40.0)?)
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            match event.kind() {
                PointerKind::Press(PointerButton::Primary) => {
                    self.armed = Some(event.position());
                    cx.capture_pointer()?;
                }
                PointerKind::Press(PointerButton::Secondary) => {
                    let task = cx.defer_while_shown(|_| Ok(()))?;
                    self.record.borrow_mut().task = Some(task);
                }
                PointerKind::Move => {
                    if let Some(start) = self.armed
                        && (event.position().x() - start.x()) >= DRAG_THRESHOLD
                    {
                        self.armed = None;
                        cx.start_drag(
                            DragPayload::new(7_u32),
                            DropOperations::MOVE,
                            DragPreview::none(),
                        )?;
                    }
                }
                PointerKind::Release(_) => self.armed = None,
                _ => return Ok(InputOutcome::Ignored),
            }
            Ok(InputOutcome::Handled)
        }
        fn drag_finished(
            &mut self,
            _cx: &mut EventContext,
            outcome: DragOutcome,
        ) -> Result<(), RuntimeError> {
            self.record.borrow_mut().drags.push(outcome);
            Ok(())
        }
        fn tab_navigation(
            &mut self,
            _cx: &mut EventContext,
            direction: FocusDirection,
            _transition: KeyTransition,
        ) -> Result<InputOutcome, RuntimeError> {
            self.record.borrow_mut().tabs.push(direction);
            Ok(InputOutcome::Handled)
        }
        fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
            Ok(Some(
                ViewState::new("probe")?.with("mode", WorkspaceValue::text("armed")?)?,
            ))
        }
        fn restore_workspace_state(
            &mut self,
            state: &ViewState,
        ) -> Result<Invalidation, RestoreRejection> {
            state.expect_only(&["mode"])?;
            Ok(Invalidation::Paint)
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 80.0, 40.0)?, Color::ACCENT);
            Ok(())
        }
        fn inspect(&self) -> ComponentDetails {
            ComponentDetails {
                semantics: Some(
                    Semantics::new(SemanticRole::Button)
                        .name("probe")
                        .value(SemanticValue::Text("armed".to_owned()))
                        .action(SemanticAction::Activate)
                        .items([SemanticItem::new("first")]),
                ),
                validation: ValidationHints {
                    overlap: OverlapPolicy::Allow,
                    shortcuts: vec![ShortcutBinding {
                        scope: ShortcutScope::FocusedComponent,
                        ..ShortcutBinding::new(KeyCode::Enter, "activate")
                    }],
                    ..ValidationHints::default()
                },
                box_model: BoxModelInspection {
                    border: Some(BorderInspection {
                        color: [0.0, 0.0, 0.0, 1.0],
                        width: 1.0,
                    }),
                    ..BoxModelInspection::default()
                },
                properties: [("tasks".to_owned(), InspectValue::Number(1.0))].into(),
            }
        }
    }
}

/// Each name the module promises, imported one by one, so removing any
/// re-export fails to compile.
#[allow(unused_imports)]
mod every_name {
    use pert_ui::component::{
        ArrangeContext, BorderInspection, BoxModelInspection, ChildInfo, ChildReveal, Color,
        ColorRole, CommandContext, CommandId, CommandKey, CommandKeyInput, CommandOutcome,
        CommandReceipt, CommandSource, CommandStatus, Component, ComponentDetails, Constraints,
        ContentRows, CursorIcon, DRAG_THRESHOLD, DetachedClosePolicy, DisplayList, DragEvent,
        DragOutcome, DragPayload, DragPhase, DragPreview, DragRole, DrawCommand, DrawError,
        DropOperation, DropOperations, DropResponse, EditorAttachment, EditorOutcome,
        EditorSession, EditorStart, EditorTrigger, Element, EventContext, FillRule, FocusDirection,
        Icon, IconSource, ImageId, ImageSampling, InputOutcome, InspectValue, Invalidation,
        KeyAction, KeyCode, KeyEvent, KeyTransition, KeyboardState, Length, LineCap, LineJoin,
        Magnification, MaskId, MaskImage, MeasureContext, MetricRole, Modifiers, MotionPreference,
        NativeWindowId, NativeWindowOptions, Notifier, OverlapPolicy, Painter, Path, PathBuilder,
        PathElement, Point, PointerButton, PointerEvent, PointerKind, PresentationReceipt,
        PresentationStatus, QuarterTurn, RasterImage, ReadSignal, Rect, RestoreRejection,
        RevealOutcome, RevealReceipt, RevealRefusal, RevealStatus, RichText, RuntimeError,
        SemanticAction, SemanticItem, SemanticRole, SemanticValue, Semantics, Shape,
        ShortcutBinding, ShortcutScope, Signal, Size, SizeLimits, StrokeStyle, SurfaceDismissal,
        SurfaceKind, TaskReceipt, TaskStatus, TextCommand, TextCommandOutcome, TextInputState,
        TextKeyOutcome, TextLayout, TextOverflow, TextPreedit, TextStyle, Transform2D,
        UpdateContext, ValidationHints, VectorGeometry, VectorId, ViewScale, ViewState,
        ViewTransform, Viewport, WidgetId, WorkspaceError, WorkspaceValue,
    };
}

fn probe(ui: &Runtime) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == "probe")
        .expect("the probe is mounted")
}

#[test]
fn a_component_written_from_the_component_module_alone_runs_its_contracts()
-> Result<(), Box<dyn std::error::Error>> {
    use std::{cell::RefCell, rc::Rc};

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let record = Rc::new(RefCell::new(widget::Record::default()));
    let root = ui.mount_tree(Element::new(
        "probe",
        widget::Probe {
            record: Rc::clone(&record),
            armed: None,
        },
    ))?;
    ui.flush(Duration::ZERO)?;

    // A kept task receipt reports the deferred work's success.
    let at = |x: f32| Point::new(x, 10.0);
    ui.pointer_input(PointerEvent::new(
        at(10.0)?,
        PointerKind::Press(PointerButton::Secondary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        at(10.0)?,
        PointerKind::Release(PointerButton::Secondary),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(record.borrow().task_succeeded());

    // A press and a move past the threshold start a drag; releasing over the
    // source, which accepts nothing, finishes it.
    ui.pointer_input(PointerEvent::new(
        at(10.0)?,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(at(40.0)?, PointerKind::Move))?;
    ui.pointer_input(PointerEvent::new(
        at(40.0)?,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(record.borrow().drags.len(), 1);

    // The focused component claims Tab.
    ui.focus(&root)?;
    ui.key_input(KeyCode::Tab, KeyAction::Press)?;
    assert_eq!(record.borrow().tabs, [FocusDirection::Forward]);

    // The description reaches inspection.
    let details = probe(&ui).details;
    let semantics = details.semantics.expect("the probe describes itself");
    assert_eq!(
        (semantics.role, semantics.name.as_deref()),
        (SemanticRole::Button, Some("probe"))
    );
    assert_eq!(details.validation.overlap, OverlapPolicy::Allow);
    assert_eq!(
        details.box_model.border.map(|border| border.width),
        Some(1.0)
    );

    // Detachment options and a reveal refusal are written from the module.
    assert_eq!(
        widget::own_window()?.closing_policy(),
        pert_ui::DetachedClosePolicy::RemoveView
    );
    assert!(widget::modal_refused(&pert_ui::RevealOutcome::Refused(
        pert_ui::RevealRefusal::ModalBlocked
    )));

    // The state saves and restores through the workspace document.
    let saved = ui.capture_workspace()?;
    assert!(ui.restore_workspace(&saved)?.is_complete());
    Ok(())
}
