//! The prelude contract: a custom component, and a headless run of it, need no
//! import other than `pert_ui::prelude::*`.
//!
//! This file must not import anything else from `pert_ui`: its compiling is the
//! evidence that the prelude names every type the `Component` trait's method
//! signatures name.
use pert_ui::prelude::*;
use std::{cell::Cell, rc::Rc, time::Duration};

/// A focusable swatch that counts presses and Space keys, overriding the
/// methods a custom widget commonly implements.
struct Swatch {
    presses: Rc<Cell<u32>>,
    color: ReadSignal<Color>,
    shown: Color,
}

impl Component for Swatch {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.color = next.color;
        Some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.shown = cx.read(&self.color)?;
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn workspace_state_with_children(
        &self,
        _children: &[ChildInfo],
    ) -> Result<Option<ViewState>, WorkspaceError> {
        self.workspace_state()
    }
    fn content_rows(&self) -> Option<Rc<dyn ContentRows>> {
        None
    }
    fn cursor(&self, _point: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Move)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.presses.set(self.presses.get() + 1);
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn keyboard_event(
        &mut self,
        _cx: &mut EventContext,
        event: KeyEvent,
        _keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Space && event.transition == KeyTransition::Pressed {
            self.presses.set(self.presses.get() + 1);
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn drag_event(
        &mut self,
        _cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        let _: (DragPhase, DragRole, &DragPayload) = (event.phase(), event.role(), event.payload());
        let _: (DropOperations, DropOperation) = (event.permitted(), event.preferred());
        Ok(DropResponse::Rejected)
    }
    fn drag_finished(
        &mut self,
        _cx: &mut EventContext,
        _outcome: DragOutcome,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn text_key_event(
        &mut self,
        _cx: &mut EventContext,
        _input: CommandKeyInput,
        _transition: KeyTransition,
        _keyboard: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        Ok(TextKeyOutcome::NotAnEditor)
    }
    fn text_command(
        &mut self,
        _cx: &mut EventContext,
        _command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        Ok(TextCommandOutcome::NotAnEditor)
    }
    fn text_input_state(&self) -> Option<TextInputState> {
        None
    }
    fn text_preedit(
        &mut self,
        _cx: &mut EventContext,
        _preedit: &TextPreedit,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_redocked(&mut self, _cx: &mut EventContext, _child: &WidgetId) {}
    fn child_transform(&self) -> ViewTransform {
        ViewTransform::IDENTITY
    }
    fn child_surface(&self, _key: &str) -> Option<SurfaceKind> {
        None
    }
    fn dismiss_surface(
        &mut self,
        _cx: &mut EventContext,
        _reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn reveal(&mut self, _cx: &mut EventContext, _rect: Rect) -> Result<bool, RuntimeError> {
        Ok(false)
    }
    fn size_limits(&self) -> SizeLimits {
        SizeLimits::default()
    }
    fn measure_limits(&self, _cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        Ok(self.size_limits())
    }
    fn take_children(&mut self) -> Vec<Element> {
        Vec::new()
    }
    fn measure(
        &self,
        _cx: &MeasureContext<'_>,
        _available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn arrange(&self, _cx: &mut ArrangeContext<'_>, _size: Size) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        Ok(None)
    }
    fn restore_workspace_state(
        &mut self,
        _state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        Err(RestoreRejection::Unsupported)
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails::default()
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 40.0, 20.0)?, self.shown);
        Ok(())
    }
}

#[test]
fn a_custom_component_is_written_and_driven_from_the_prelude_alone()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 50.0, 1.0)?)?;
    let presses = Rc::new(Cell::new(0));
    let color = ui.signal(Color::ACCENT);
    let swatch = Swatch {
        presses: Rc::clone(&presses),
        color: color.read_only(),
        shown: Color::SURFACE,
    };
    let root = ui.mount_tree(Element::new("swatch", swatch))?;
    ui.flush(Duration::ZERO)?;

    let press = PointerEvent::new(
        Point::new(5.0, 5.0)?,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(ui.pointer_input(press)?, InputOutcome::Handled);
    ui.focus(&root)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    assert_eq!(presses.get(), 2);

    ui.flush(Duration::ZERO)?;
    let list: DisplayList = ui.paint()?;
    assert!(
        list.commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::FillRect { .. }))
    );
    Ok(())
}
