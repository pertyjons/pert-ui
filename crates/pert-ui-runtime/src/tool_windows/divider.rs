//! The divider between an edge's expanded panels and the center, which
//! resizes those panels.
use super::ToolEdge;
use super::model::{self, Link};
use crate::*;
use std::{cell::Cell, collections::BTreeSet, rc::Rc};

/// The extents the edge's expanded tool windows had before a resize began,
/// restored when it is cancelled.
struct OriginalExtent {
    key: String,
    extent: Option<f32>,
    restored: Option<(f32, f64)>,
}
type Original = Vec<OriginalExtent>;

struct Drag {
    position: f32,
    extent: f32,
    original: Original,
}

pub(super) struct EdgeDivider {
    link: Rc<Link>,
    edge: ToolEdge,
    enabled: bool,
    focused: bool,
    hovered: bool,
    drag: Option<Drag>,
    keys: BTreeSet<KeyCode>,
    key_origin: Option<Original>,
    size: Cell<Size>,
    /// The divider's name from the installed string table.
    name: String,
}
impl EdgeDivider {
    pub(super) fn new(link: Rc<Link>, edge: ToolEdge) -> Self {
        Self {
            link,
            edge,
            enabled: false,
            focused: false,
            hovered: false,
            drag: None,
            keys: BTreeSet::new(),
            key_origin: None,
            size: Cell::new(Size::ZERO),
            name: UiStrings::english().get(edge.resize_role()).to_owned(),
        }
    }
    fn original(&self) -> Original {
        let model = self.link.model();
        let model = model.borrow();
        model
            .open_on(self.edge)
            .into_iter()
            .map(|window| OriginalExtent {
                key: window.key.clone(),
                extent: window.extent,
                restored: window.restored,
            })
            .collect()
    }
    /// The along-axis coordinate of a pointer position in the layout's
    /// space, signed so that larger values grow the panel. The layout's space
    /// rather than the divider's own: the divider moves with the resize, so
    /// its local coordinates would chase the pointer.
    fn growth(&self, cx: &EventContext, position: Point) -> Option<f32> {
        let local = cx.to_local(position)?;
        let origin = cx.position_in_parent()?;
        let (x, y) = (local.x() + origin.x(), local.y() + origin.y());
        Some(match self.edge {
            ToolEdge::Left => x,
            ToolEdge::Right => -x,
            ToolEdge::Top => y,
            ToolEdge::Bottom => -y,
        })
    }
    fn resizing(&self) -> bool {
        self.drag.is_some() || self.key_origin.is_some()
    }
    fn set_resizing(&self) {
        let resizing = self.resizing();
        *self.link.model().borrow_mut().resizing.get_mut(self.edge) = resizing;
    }
    /// Moves the edge's panels to `extent`, kept within the arranged range.
    fn resize_to(&self, cx: &mut EventContext, extent: f32) {
        let model = self.link.model();
        let mut model = model.borrow_mut();
        let Some(geometry) = *model.geometry.get(self.edge) else {
            return;
        };
        let extent = extent.clamp(geometry.minimum, geometry.maximum.max(geometry.minimum));
        if extent != geometry.extent {
            model.set_extent(self.edge, extent);
            cx.invalidate(Invalidation::Layout);
        }
    }
    fn current(&self) -> Option<f32> {
        let model = self.link.model();
        let model = model.borrow();
        model
            .geometry
            .get(self.edge)
            .map(|geometry| geometry.extent)
    }
    fn restore(&mut self, original: Original) {
        let model = self.link.model();
        let mut model = model.borrow_mut();
        for original in original {
            if let Some(window) = model
                .windows
                .iter_mut()
                .find(|window| window.key == original.key)
            {
                window.extent = original.extent;
                window.restored = original.restored;
            }
        }
    }
    /// Reports the committed extent of each tool window a resize that began
    /// at `original` changed.
    fn report(&self, original: &Original) {
        let model = self.link.model();
        let mut model = model.borrow_mut();
        for original in original {
            if model
                .window(&original.key)
                .is_some_and(|window| window.extent != original.extent)
            {
                model.record(
                    &original.key,
                    crate::ToolWindowChangeKind::Geometry,
                    ChangeOrigin::User,
                );
            }
        }
    }
    fn cancel(&mut self) {
        if let Some(drag) = self.drag.take() {
            self.restore(drag.original);
        }
        if let Some(original) = self.key_origin.take() {
            self.restore(original);
        }
        self.keys.clear();
        self.set_resizing();
    }
}
impl Component for EdgeDivider {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if self.edge != next.edge {
            self.cancel();
            self.edge = next.edge;
        }
        None
    }
    fn focusable(&self) -> bool {
        self.enabled
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.enabled.then_some(if self.edge.vertical() {
            CursorIcon::ResizeHorizontal
        } else {
            CursorIcon::ResizeVertical
        })
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn unmounted(&mut self) {
        *self.link.model().borrow_mut().resizing.get_mut(self.edge) = false;
        self.drag = None;
        self.key_origin = None;
        self.keys.clear();
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.name = cx.strings()?.get(self.edge.resize_role()).to_owned();
        let model = self.link.model();
        let permission = model.borrow().resize_permission(self.edge);
        let revision = model::revision(&model, cx);
        cx.read(&revision.read_only())?;
        self.enabled = permission.read(cx)?;
        if !self.enabled && self.resizing() {
            self.cancel();
            cx.cancel_input();
            cx.request_layout();
        }
        self.focused = cx.focused() && self.enabled;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let width = cx.metric(MetricRole::ResizeWidth);
        Ok(Size::new(width, width)?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.size.set(size);
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled {
            cx.release_pointer();
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                if self.key_origin.is_some() {
                    return Ok(InputOutcome::Handled);
                }
                let (Some(position), Some(extent)) =
                    (self.growth(cx, event.position()), self.current())
                else {
                    return Ok(InputOutcome::Ignored);
                };
                self.cancel();
                self.drag = Some(Drag {
                    position,
                    extent,
                    original: self.original(),
                });
                self.set_resizing();
                cx.capture_pointer()?;
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Move if self.drag.is_some() => {
                if let (Some(drag), Some(position)) =
                    (&self.drag, self.growth(cx, event.position()))
                {
                    let extent = drag.extent + position - drag.position;
                    self.resize_to(cx, extent);
                }
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                if let Some(drag) = self.drag.take() {
                    self.report(&drag.original);
                }
                self.set_resizing();
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if control == KeyCode::Escape {
            if !self.resizing() {
                return Ok(InputOutcome::Ignored);
            }
            self.cancel();
            cx.release_pointer();
            cx.invalidate(Invalidation::Layout);
            return Ok(InputOutcome::Handled);
        }
        if !self.enabled || !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        if self.drag.is_some() {
            return Ok(InputOutcome::Handled);
        }
        let direction = match (self.edge, control) {
            (ToolEdge::Left, KeyCode::ArrowRight)
            | (ToolEdge::Right, KeyCode::ArrowLeft)
            | (ToolEdge::Top, KeyCode::ArrowDown)
            | (ToolEdge::Bottom, KeyCode::ArrowUp) => 1.0,
            (ToolEdge::Left, KeyCode::ArrowLeft)
            | (ToolEdge::Right, KeyCode::ArrowRight)
            | (ToolEdge::Top, KeyCode::ArrowUp)
            | (ToolEdge::Bottom, KeyCode::ArrowDown) => -1.0,
            _ => return Ok(InputOutcome::Ignored),
        };
        // A new Control, Alt or Super chord is left to command bindings and to
        // a floating frame's Alt+arrow move; held keys keep resizing.
        if event.transition == KeyTransition::Pressed
            && !self.keys.contains(&event.code)
            && !crate::activation::steps_with(keyboard.modifiers())
        {
            return Ok(InputOutcome::Ignored);
        }
        match event.transition {
            KeyTransition::Pressed => {
                if self.key_origin.is_none() {
                    self.key_origin = Some(self.original());
                    self.set_resizing();
                }
                self.keys.insert(event.code);
            }
            KeyTransition::Repeated if !self.keys.contains(&event.code) => {
                return Ok(InputOutcome::Handled);
            }
            KeyTransition::Released | KeyTransition::UnmatchedRelease => {
                self.keys.remove(&event.code);
                if self.keys.is_empty() {
                    if let Some(original) = self.key_origin.take() {
                        self.report(&original);
                    }
                    self.set_resizing();
                }
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        if let Some(extent) = self.current() {
            let step = if keyboard.modifiers().shift {
                32.0
            } else {
                8.0
            };
            self.resize_to(cx, extent + direction * step);
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        let resizing = self.resizing();
        self.cancel();
        self.focused = false;
        self.hovered = false;
        cx.release_pointer();
        if resizing {
            cx.invalidate(Invalidation::Layout);
        } else {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = self.size.get();
        let (w, h) = (size.width(), size.height());
        let thickness = p.metric(MetricRole::BorderWidth).min(w.min(h));
        let color = p.color(if self.focused {
            ColorRole::FocusRing
        } else if self.resizing() {
            ColorRole::Primary
        } else if self.enabled && self.hovered {
            ColorRole::ResizeHandleHover
        } else if self.enabled {
            ColorRole::ResizeHandle
        } else {
            ColorRole::Divider
        });
        let line = if self.edge.vertical() {
            Rect::new((w - thickness) / 2.0, 0.0, thickness, h)?
        } else {
            Rect::new(0.0, (h - thickness) / 2.0, w, thickness)?
        };
        p.fill_rect(line, color);
        if self.enabled && w > 0.0 && h > 0.0 {
            let along = if self.edge.vertical() { h } else { w };
            let length = (p.metric(MetricRole::IconSize) * 1.5).min(along);
            let grip_thickness = (thickness * 3.0).min(w.min(h));
            let grip = if self.edge.vertical() {
                Rect::new(
                    (w - grip_thickness) / 2.0,
                    (h - length) / 2.0,
                    grip_thickness,
                    length,
                )?
            } else {
                Rect::new(
                    (w - length) / 2.0,
                    (h - grip_thickness) / 2.0,
                    length,
                    grip_thickness,
                )?
            };
            p.fill_shape(
                grip,
                Shape::Rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                color,
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("resizable".into(), InspectValue::Bool(self.enabled));
        if let Some(extent) = self.current() {
            details
                .properties
                .insert("extent".into(), InspectValue::Number(f64::from(extent)));
        }
        details.semantics = Some(
            Semantics::new(SemanticRole::Splitter)
                .name(self.name.as_str())
                .disabled(!self.enabled)
                .enabled_actions([SemanticAction::Resize]),
        );
        details
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_remains_busy_until_every_edge_resize_ends() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut layout = crate::ToolWindowLayout::new(Element::new("body", Label::new("Body")));
        let mut left = EdgeDivider::new(layout.link.clone(), ToolEdge::Left);
        let mut bottom = EdgeDivider::new(layout.link.clone(), ToolEdge::Bottom);
        left.key_origin = Some(Vec::new());
        bottom.drag = Some(Drag {
            position: 0.0,
            extent: 0.0,
            original: Vec::new(),
        });
        left.set_resizing();
        bottom.set_resizing();
        let state = ViewState::new("tool-windows")?
            .with(
                "expanded",
                WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
            )?
            .with(
                "extents",
                WorkspaceValue::map(Vec::<(String, WorkspaceValue)>::new())?,
            )?;
        left.cancel();
        assert_eq!(
            layout.restore_workspace_state(&state),
            Err(RestoreRejection::Busy)
        );
        bottom.cancel();
        assert_eq!(
            layout.restore_workspace_state(&state),
            Ok(Invalidation::Layout)
        );
        Ok(())
    }
}
