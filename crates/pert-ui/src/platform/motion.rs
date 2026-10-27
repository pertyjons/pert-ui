//! Bounded native motion coalescing; discrete input remains ordered.
use crate::{Point, PointerEvent, PointerKind, Runtime, RuntimeError};

pub(super) struct Motion {
    position: Point,
    outside: bool,
    pending: bool,
    scale: f64,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            position: Point::default(),
            outside: true,
            pending: false,
            scale: 1.0,
        }
    }
}
impl Motion {
    #[cfg(test)]
    pub(super) fn queue(&mut self, position: Point) {
        self.queue_at_scale(position, 1.0);
    }
    pub(super) fn queue_at_scale(&mut self, position: Point, scale: f64) {
        self.scale = scale;
        self.position = position;
        self.outside = false;
        self.pending = true;
    }
    pub(super) fn discard(&mut self) {
        self.pending = false;
    }
    pub(super) fn flush(&mut self, runtime: &mut Runtime) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.pending) {
            self.dispatch(runtime, PointerKind::Move)?;
        }
        Ok(())
    }
    pub(super) fn send(
        &mut self,
        runtime: &mut Runtime,
        kind: PointerKind,
    ) -> Result<(), RuntimeError> {
        self.flush(runtime)?;
        self.dispatch(runtime, kind)
    }
    fn dispatch(&mut self, runtime: &mut Runtime, kind: PointerKind) -> Result<(), RuntimeError> {
        if kind == PointerKind::Leave {
            self.outside = true;
        }
        let scale = runtime.viewport().scale();
        if self.scale != scale {
            self.position = Point::new(
                (f64::from(self.position.x()) * self.scale / scale) as f32,
                (f64::from(self.position.y()) * self.scale / scale) as f32,
            )?;
            self.scale = scale;
        }
        let event = PointerEvent::new(self.position, kind);
        runtime.pointer_input(if self.outside { event.outside() } else { event })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::HostIntegration;
    use crate::*;
    use std::{cell::RefCell, rc::Rc, time::Duration};

    struct Recorder(Rc<RefCell<Vec<PointerEvent>>>);
    impl Component for Recorder {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            if matches!(event.kind(), PointerKind::Press(_)) {
                cx.capture_pointer()?;
            }
            self.0.borrow_mut().push(event);
            Ok(InputOutcome::Handled)
        }
    }
    #[test]
    fn motion_bursts_are_bounded_and_release_receives_the_last_position() {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
            .unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        ui.mount_tree(Element::new("recorder", Recorder(events.clone())))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let mut motion = Motion::default();
        for index in 0..1000 {
            motion.queue(Point::new((index % 100) as f32, 20.0).unwrap());
        }
        assert!(events.borrow().is_empty());
        motion.flush(&mut ui).unwrap();
        assert_eq!(events.borrow().len(), 1);
        assert_eq!(events.borrow()[0].position().x(), 99.0);
        motion
            .send(&mut ui, PointerKind::Press(PointerButton::Primary))
            .unwrap();
        for index in 0..1000 {
            motion.queue(Point::new(100.0 + index as f32, 20.0).unwrap());
        }
        motion
            .send(&mut ui, PointerKind::Release(PointerButton::Primary))
            .unwrap();
        let captured: Vec<_> = events
            .borrow()
            .iter()
            .filter(|event| event.kind() != PointerKind::Leave)
            .copied()
            .collect();
        assert_eq!(captured.len(), 4);
        assert_eq!(captured[2].kind(), PointerKind::Move);
        assert_eq!(
            captured[3].kind(),
            PointerKind::Release(PointerButton::Primary)
        );
        assert_eq!(captured[2].position().x(), 1099.0);
        assert_eq!(captured[3].position().x(), 1099.0);
        // Coalesced motion is host motion, never the runtime's refresh.
        assert!(
            captured[..=2]
                .iter()
                .filter(|event| event.kind() == PointerKind::Move)
                .all(|event| !event.is_refresh())
        );
        let count = events.borrow().len();
        motion.flush(&mut ui).unwrap();
        assert_eq!(events.borrow().len(), count);
        motion.queue(Point::new(40.0, 20.0).unwrap());
        motion.discard();
        ui.clear_focus();
        motion.flush(&mut ui).unwrap();
        assert_eq!(events.borrow().len(), count);
    }
    #[test]
    fn queued_and_stationary_motion_follow_zoom_and_native_scale_changes() {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.5).unwrap())
            .unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        ui.mount_tree(Element::new("recorder", Recorder(events.clone())))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let mut motion = Motion::default();
        // The queued position corresponds to physical pixel (180, 90).
        motion.queue_at_scale(Point::new(120.0, 60.0).unwrap(), 1.5);
        ui.ui_settings()
            .set_zoom(UiZoom::new(1.25).unwrap())
            .unwrap();
        motion.flush(&mut ui).unwrap();
        assert_eq!(
            events.borrow().last().unwrap().position(),
            Point::new(96.0, 48.0).unwrap()
        );

        // A click without another motion still uses the new monitor scale.
        ui.set_viewport(Viewport::new(300.0, 150.0, 2.0).unwrap())
            .unwrap();
        motion
            .send(&mut ui, PointerKind::Press(PointerButton::Primary))
            .unwrap();
        let press = events.borrow().last().copied().unwrap();
        assert_eq!(press.kind(), PointerKind::Press(PointerButton::Primary));
        assert_eq!(press.position(), Point::new(72.0, 36.0).unwrap());

        // Motion arriving before release is converted exactly once at dispatch.
        motion.queue_at_scale(Point::new(80.0, 40.0).unwrap(), 2.5);
        ui.ui_settings().set_zoom(UiZoom::default()).unwrap();
        motion
            .send(&mut ui, PointerKind::Release(PointerButton::Primary))
            .unwrap();
        let release = events.borrow().last().copied().unwrap();
        assert_eq!(release.kind(), PointerKind::Release(PointerButton::Primary));
        assert_eq!(release.position(), Point::new(100.0, 50.0).unwrap());
    }
    #[test]
    fn split_release_uses_latest_motion_and_context_loss_cancels() {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 120.0, 1.0).unwrap())
            .unwrap();
        let split = ui
            .mount_tree(Element::new(
                "split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new("first", Label::new("First")),
                    Element::new("second", Label::new("Second")),
                ),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let first = ui.children(&split).unwrap()[0].clone();
        let width = |ui: &Runtime| ui.bounds(&first).unwrap().unwrap().width();
        assert_eq!(width(&ui), 156.0);
        let mut motion = Motion::default();
        motion.queue(Point::new(160.0, 40.0).unwrap());
        motion
            .send(&mut ui, PointerKind::Press(PointerButton::Primary))
            .unwrap();
        motion.queue(Point::new(180.0, 40.0).unwrap());
        motion.queue(Point::new(210.0, 40.0).unwrap());
        motion
            .send(&mut ui, PointerKind::Release(PointerButton::Primary))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(width(&ui), 206.0);
        motion
            .send(&mut ui, PointerKind::Press(PointerButton::Primary))
            .unwrap();
        motion.queue(Point::new(220.0, 40.0).unwrap());
        motion.flush(&mut ui).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(width(&ui), 216.0);
        motion.queue(Point::new(250.0, 40.0).unwrap());
        motion.discard();
        ui.clear_focus();
        motion
            .send(&mut ui, PointerKind::Release(PointerButton::Primary))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(width(&ui), 206.0);
        assert!(ui.next_deadline().is_none());
    }
}
