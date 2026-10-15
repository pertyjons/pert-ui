//! The drag session: target resolution under capture, the preview root, and the
//! single path through which a session ends on both sides.
use super::*;
use crate::{
    DragEvent, DragOutcome, DragPayload, DragPhase, DragPreview, DragRole, DropOperation,
    DropOperations, DropResponse, InputOutcome, PointerEvent, PointerKind,
};

/// Root key of the mounted preview. Structural, so it addresses no workspace state.
const PREVIEW_KEY: &str = "pert-ui:drag-preview";

/// A source's `EventContext::start_drag`, waiting for the runtime to adopt it.
pub(super) struct DragRequest {
    source: Weak<Node>,
    payload: DragPayload,
    permitted: DropOperations,
    preview: DragPreview,
}

pub(super) struct DragSession {
    source: Rc<Node>,
    payload: DragPayload,
    permitted: DropOperations,
    /// The component that claimed the drag and its latest answer.
    target: Option<(Rc<Node>, DropResponse)>,
    /// Ancestors of the target that were told about the session, nearest first.
    observers: Vec<Rc<Node>>,
    preview: Option<WidgetId>,
    hot_spot: crate::Point,
    window: u64,
    /// Last pointer position inside the window; `None` while it is outside.
    position: Option<crate::Point>,
    /// Last position inside the window: where the preview stays while the
    /// pointer is outside, and what a `Leave` reports then.
    last_position: crate::Point,
    /// Set when anything but the preview may have changed what is under a still
    /// pointer, so the target is resolved again.
    layout_changed: bool,
    /// The participant error that is ending the session, for event history.
    failure: Option<String>,
    /// The position the target was last resolved at, so a release where the
    /// pointer already was does not deliver the same `Over` twice.
    resolved_at: Option<crate::Point>,
}

/// Holds the user's preview element at its preferred size.
pub(super) struct PreviewRoot(Vec<crate::Element>);
impl Component for PreviewRoot {
    fn take_children(&mut self) -> Vec<crate::Element> {
        std::mem::take(&mut self.0)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn inspect(&self) -> crate::ComponentDetails {
        let mut details = crate::ComponentDetails::default();
        // It follows the pointer across every other view by design.
        details.validation.overlap = crate::OverlapPolicy::Allow;
        details
    }
}

impl EventContext {
    /// Starts a drag session with this component as its source.
    ///
    /// Call it from [`Component::pointer_input`] once an armed press has moved
    /// [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD) window pixels, so a press
    /// without movement stays a click. The source takes the pointer capture, as
    /// [`Self::capture_pointer`] would, and keeps it for the whole session. The
    /// runtime adopts the session when this callback returns: it mounts the
    /// preview, suspends ordinary hover, and from then on offers the drag to
    /// whatever is under the pointer through [`Component::drag_event`].
    ///
    /// Every session ends with exactly one [`Component::drag_finished`] to a
    /// source that is still mounted: a drop, a refusal, a release that nothing
    /// claimed, or a cancellation.
    /// Escape, native focus loss, [`Runtime::cancel_input`], a modal surface that
    /// blocks the source, an error from a participant, and the source being
    /// hidden, removed or input-cancelled, and runtime shutdown all cancel. A session is confined to
    /// the window it started in.
    ///
    /// ```
    /// use pert_ui_runtime::*;
    /// struct Chip { key: String, armed: Option<Point> }
    /// struct ChipKey(String);
    /// impl Component for Chip {
    ///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    ///     fn pointer_input(&mut self, cx: &mut EventContext, event: PointerEvent)
    ///         -> Result<InputOutcome, RuntimeError> {
    ///         match event.kind() {
    ///             PointerKind::Press(PointerButton::Primary) => {
    ///                 self.armed = Some(event.position());
    ///                 cx.capture_pointer()?;
    ///             }
    ///             PointerKind::Move => {
    ///                 if let Some(start) = self.armed
    ///                     && (event.position().x() - start.x())
    ///                         .hypot(event.position().y() - start.y()) >= DRAG_THRESHOLD
    ///                 {
    ///                     self.armed = None;
    ///                     cx.start_drag(
    ///                         DragPayload::new(ChipKey(self.key.clone())),
    ///                         DropOperations::MOVE,
    ///                         DragPreview::element(Element::new("chip", Label::new(self.key.as_str()))),
    ///                     )?;
    ///                 }
    ///             }
    ///             PointerKind::Release(_) => self.armed = None,
    ///             _ => return Ok(InputOutcome::Ignored),
    ///         }
    ///         Ok(InputOutcome::Handled)
    ///     }
    ///     fn drag_finished(&mut self, _: &mut EventContext, outcome: DragOutcome)
    ///         -> Result<(), RuntimeError> {
    ///         // Restore presentation only. A move was one edit on the target's side.
    ///         let _ = outcome;
    ///         Ok(())
    ///     }
    /// }
    /// # let _ = Chip { key: String::new(), armed: None };
    /// ```
    ///
    /// # Errors
    /// Returns `DragUnavailable` while another session is active or requested,
    /// or for an empty set of operations; `CaptureUnavailable` without a held
    /// button or while another component owns the capture; and the usual
    /// phase/closed errors. Returns `Closed` during shutdown terminal callbacks,
    /// which cannot admit a replacement drag.
    pub fn start_drag(
        &self,
        payload: DragPayload,
        permitted: impl Into<DropOperations>,
        preview: DragPreview,
    ) -> Result<(), RuntimeError> {
        let shared = &self.node.shared;
        shared.writable()?;
        if shared.shutting_down.get() {
            return Err(RuntimeError::Closed);
        }
        let permitted = permitted.into();
        if permitted.is_empty() || shared.drag_source.get().is_some() {
            return Err(RuntimeError::DragUnavailable);
        }
        self.capture_pointer()?;
        shared.drag_source.set(Some(self.node.serial));
        shared.drag_lost.set(false);
        *shared.drag_request.borrow_mut() = Some(DragRequest {
            source: Rc::downgrade(&self.node),
            payload,
            permitted,
            preview,
        });
        Ok(())
    }

    /// Whether a drag session is active or requested in this runtime, for a
    /// component that presents differently while something is being dragged.
    #[must_use]
    pub fn drag_active(&self) -> bool {
        self.node.shared.drag_source.get().is_some()
    }
}

impl Runtime {
    /// Whether a drag session is active. Reading schedules nothing.
    #[must_use]
    pub fn drag_active(&self) -> bool {
        self.drag.is_some()
    }

    pub(super) fn drag_in_window(&self) -> bool {
        self.drag
            .as_ref()
            .is_some_and(|session| session.window == self.window.shared.serial)
    }

    /// Whether `serial` belongs to the mounted preview, which takes no part in
    /// hit testing, navigation or workspace addressing.
    pub(super) fn in_drag_preview(&self, mut serial: u64) -> bool {
        let Some(root) = self.drag_preview_root() else {
            return false;
        };
        loop {
            if serial == root {
                return true;
            }
            let Some(parent) = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.logical_parent_serial())
            else {
                return false;
            };
            serial = parent;
        }
    }
    pub(super) fn drag_preview_root(&self) -> Option<u64> {
        self.drag
            .as_ref()
            .and_then(|session| session.preview.as_ref())
            .map(|id| id.0.serial)
    }
    /// Where layout places the preview root: the pointer less the hot spot.
    pub(super) fn drag_preview_origin(&self) -> Option<crate::Point> {
        let session = self.drag.as_ref()?;
        crate::Point::new(
            session.last_position.x() - session.hot_spot.x(),
            session.last_position.y() - session.hot_spot.y(),
        )
        .ok()
    }
    pub(super) fn note_drag_layout_change(&mut self) {
        if let Some(session) = &mut self.drag {
            session.layout_changed = true;
        }
    }
    pub(super) fn drag_needs_service(&self) -> bool {
        self.shared.drag_request.borrow().is_some()
            || (self.drag.is_some() && self.shared.drag_lost.get())
    }

    /// A requested drag can be adopted only in its source's presentation root.
    pub(super) fn drag_needs_service_in_window(&self) -> bool {
        self.drag_needs_service()
            && self
                .shared
                .drag_request
                .borrow()
                .as_ref()
                .is_none_or(|request| {
                    request
                        .source
                        .upgrade()
                        .is_none_or(|node| Rc::ptr_eq(&node.window(), &self.window.shared))
                })
    }

    pub(super) fn drag_inspection(&self) -> Option<crate::DragInspection> {
        let session = self.drag.as_ref()?;
        Some(crate::DragInspection {
            source: session.source.serial,
            window: session.window,
            target: session.target.as_ref().map(|(node, _)| node.serial),
            accepted: session
                .target
                .as_ref()
                .and_then(|(_, response)| match response {
                    DropResponse::Accepted(operation) => Some(*operation),
                    DropResponse::Rejected | DropResponse::Forbidden => None,
                }),
            permitted: session.permitted,
            payload_type: session.payload.type_name().to_owned(),
            preview: session.preview.as_ref().map(|id| id.0.serial),
        })
    }

    /// The cursor a session shows: what a release here would do.
    pub(super) fn drag_cursor(&self) -> Option<crate::CursorIcon> {
        let session = self.drag.as_ref()?;
        if session.window != self.window.shared.serial {
            return None;
        }
        Some(match &session.target {
            None => crate::CursorIcon::Grabbing,
            Some((_, DropResponse::Rejected | DropResponse::Forbidden)) => {
                crate::CursorIcon::NoDrop
            }
            Some((_, DropResponse::Accepted(DropOperation::Move))) => crate::CursorIcon::Grabbing,
            Some((_, DropResponse::Accepted(DropOperation::Copy))) => crate::CursorIcon::Copy,
            Some((_, DropResponse::Accepted(DropOperation::Link))) => crate::CursorIcon::Alias,
        })
    }

    /// Adopts a requested session, notices a lost one, and resolves the target
    /// again after layout moved something under a still pointer.
    pub(super) fn service_drag(&mut self) -> Result<(), RuntimeError> {
        self.adopt_drag()?;
        let Some(session) = &self.drag else {
            return Ok(());
        };
        let source = session.source.clone();
        // Window-modal surfaces belong to the selected window, so modal
        // admission is judged only while the session's own window is selected.
        let here = session.window == self.window.shared.serial;
        if self.shared.drag_lost.get()
            || !source.mounted.get()
            || !source.visible.get()
            || (here && !self.modal_allows(source.serial))
        {
            return self.finish_drag(false).map(|_| ());
        }
        if self.drag.as_mut().is_some_and(|session| {
            session.window == self.window.shared.serial
                && std::mem::take(&mut session.layout_changed)
        }) {
            self.resolve_drag()?;
        }
        Ok(())
    }

    fn adopt_drag(&mut self) -> Result<(), RuntimeError> {
        let Some(request) = self.shared.drag_request.borrow_mut().take() else {
            return Ok(());
        };
        // The preview mounts into the source's window, so adoption waits for it.
        if let Some(node) = request.source.upgrade()
            && !Rc::ptr_eq(&node.window(), &self.window.shared)
        {
            *self.shared.drag_request.borrow_mut() = Some(request);
            return Ok(());
        }
        let source = request.source.upgrade().filter(|node| {
            node.mounted.get()
                && node.visible.get()
                && !self.shared.drag_lost.get()
                && node.window().capture.get() == Some(node.serial)
        });
        let Some(source) = source else {
            // The press ended or the source lost its context before adoption.
            // `start_drag` succeeded, so the source is still owed its ending.
            return self.abandon_drag_request(request.source.upgrade());
        };
        let window = source.window();
        let position = window.pointer_position.get();
        let mut session = DragSession {
            source: source.clone(),
            payload: request.payload,
            permitted: request.permitted,
            target: None,
            observers: Vec::new(),
            preview: None,
            hot_spot: request.preview.hot_spot,
            window: window.serial,
            position,
            last_position: position.unwrap_or_default(),
            layout_changed: false,
            failure: None,
            resolved_at: None,
        };
        if let Some(element) = request.preview.element {
            let root = crate::Element::new(PREVIEW_KEY, PreviewRoot(vec![element])).structural();
            match self.mount_tree(root) {
                Ok(id) => session.preview = Some(id),
                Err(error) => {
                    source.lose_context();
                    // The mount failure is the cause; a failing source callback
                    // does not replace it.
                    let _ = self.abandon_drag_request(Some(source));
                    return Err(error);
                }
            }
        }
        self.shared.record(
            crate::EventKind::DragStart,
            Some(source.serial),
            None,
            || crate::EventStatus::Succeeded,
        );
        self.drag = Some(session);
        // Nothing else reacts to the pointer during a session.
        self.update_hover_path(None);
        self.window.pointer_target = None;
        self.resolve_drag()
    }

    /// Ends a request that `start_drag` accepted but that never became a
    /// session: the source, if it is still mounted, is told it was cancelled.
    fn abandon_drag_request(&mut self, source: Option<Rc<Node>>) -> Result<(), RuntimeError> {
        self.shared.drag_source.set(None);
        self.shared.drag_lost.set(false);
        let Some(source) = source.filter(|node| node.mounted.get()) else {
            return Ok(());
        };
        if source.window().capture.get() == Some(source.serial) {
            source.window().capture.set(None);
        }
        let Some(entry) = self.entries.get_mut(&source.serial) else {
            return Ok(());
        };
        entry.component.drag_finished(
            &mut EventContext {
                node: entry.node.clone(),
            },
            DragOutcome::Cancelled,
        )
    }

    /// One pointer transition while a session owns this window's pointer.
    pub(super) fn drag_pointer(
        &mut self,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(button) => {
                self.window.buttons.insert(button);
            }
            PointerKind::Release(button) => {
                self.window.buttons.remove(&button);
            }
            _ => {}
        }
        self.window
            .shared
            .held_buttons
            .set(self.window.buttons.len());
        let released = self.window.buttons.is_empty();
        // A pending zoom or density change cancels the capture owner, and with
        // it the session, before any position is interpreted.
        let prepared = if self.geometry_pending()
            || !self.window.shared.pending.borrow().is_empty()
            || self.window.shared.layout_pending.get()
        {
            self.flush(self.shared.now.get()).map(|_| ())
        } else {
            Ok(())
        };
        if self.drag.is_none() {
            if released {
                self.window.shared.capture.set(None);
            }
            return prepared.map(|()| InputOutcome::Handled);
        }
        if let Err(error) = prepared {
            if self.shared.notification_error_cancels_input(released) {
                let _ = self.finish_drag(false);
            }
            return Err(error);
        }
        let position = (!event.is_outside()).then(|| event.position());
        self.window.shared.pointer_position.set(position);
        if let Some(session) = &mut self.drag {
            session.position = position;
            if let Some(position) = position
                && std::mem::replace(&mut session.last_position, position) != position
                && session.preview.is_some()
            {
                // Only the preview root is arranged again: its retained drawing
                // is composed at a new origin and nothing is measured or painted.
                self.window.shared.layout_pending.set(true);
                self.window.shared.scene_dirty.set(true);
            }
        }
        if released {
            // The drop goes to whatever is under the release, which need not be
            // what the last motion was over, and is asked there first. A release
            // outside the window resolves to no target at all.
            if self
                .drag
                .as_ref()
                .is_some_and(|session| session.resolved_at != position)
            {
                self.resolve_drag()?;
            }
            return self.finish_drag(true);
        }
        if let PointerKind::Scroll(_) = event.kind()
            && let Some(hit) = position.and_then(|point| self.pointer_hit(point))
        {
            // The wheel still scrolls what is under the pointer, so a long list
            // can be travelled with the button held.
            let scrolled = self
                .dispatch_pointer(hit, event)
                .and_then(|_| self.flush(self.shared.now.get()));
            if let Err(error) = scrolled {
                if self.shared.notification_error_cancels_input(false) {
                    self.fail_drag(&error);
                }
                return Err(error);
            }
            if self.drag.is_none() {
                return Ok(InputOutcome::Handled);
            }
        }
        self.resolve_drag()?;
        Ok(InputOutcome::Handled)
    }

    /// Ends the session because a participant failed; every other participant is
    /// still told, and history records the failure rather than a cancellation.
    pub(super) fn fail_drag(&mut self, error: &RuntimeError) {
        if let Some(session) = &mut self.drag {
            session.failure.get_or_insert_with(|| error.to_string());
        }
        let _ = self.finish_drag(false);
    }

    /// Offers the drag to whatever is under the pointer and tells the target's
    /// ancestors, ending the parts of components the session moved away from.
    pub(super) fn resolve_drag(&mut self) -> Result<(), RuntimeError> {
        let result = self.resolve_drag_inner();
        if let Err(error) = &result {
            // A participant failed: both sides are told, then the error surfaces.
            self.fail_drag(error);
        }
        result
    }
    fn resolve_drag_inner(&mut self) -> Result<(), RuntimeError> {
        let Some(session) = &self.drag else {
            return Ok(());
        };
        let current = session.target.as_ref().map(|(node, _)| node.serial);
        let position = session.position;
        let hit = position.and_then(|point| self.pointer_hit(point));
        if let Some(session) = &mut self.drag {
            session.resolved_at = position;
        }
        let mut claimed = None;
        let mut candidate = hit;
        while let Some(serial) = candidate {
            if !self.modal_allows(serial) {
                break;
            }
            let phase = if current == Some(serial) {
                DragPhase::Over
            } else {
                DragPhase::Enter
            };
            let response = self.deliver_drag(serial, phase, DragRole::Candidate)?;
            // Accepting or forbidding claims the drag; rejecting passes it on,
            // which is also how the current target gives it up.
            if response != DropResponse::Rejected {
                claimed = Some((serial, response));
                break;
            }
            candidate = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.presentation_parent_serial());
        }
        let mut path = Vec::new();
        let mut ancestor = claimed.and_then(|(serial, _)| {
            self.entries
                .get(&serial)
                .and_then(|entry| entry.node.presentation_parent_serial())
        });
        while let Some(serial) = ancestor {
            let Some(entry) = self.entries.get(&serial) else {
                break;
            };
            path.push(entry.node.clone());
            ancestor = entry.node.presentation_parent_serial();
        }
        // The session records who now holds the drag before anyone is told to
        // let go, so a failure below cannot leave a component that was already
        // told `Leave` as the target, or a new target without its ending.
        let previous = self
            .drag
            .as_mut()
            .map(|session| {
                session.target = claimed.and_then(|(serial, response)| {
                    self.entries
                        .get(&serial)
                        .map(|entry| (entry.node.clone(), response))
                });
                std::mem::replace(&mut session.observers, path.clone())
            })
            .unwrap_or_default();
        let target = claimed.map(|(serial, _)| serial);
        let mut failure = None;
        if let Some(old) = current
            && current != target
            && let Err(error) = self.deliver_drag(old, DragPhase::Leave, DragRole::Candidate)
        {
            failure = Some(error);
        }
        for old in previous {
            // An observer that has just become the target was told so by its
            // `Enter`; a `Leave` now would end the part it has only begun.
            if old.mounted.get()
                && Some(old.serial) != target
                && !path.iter().any(|node| Rc::ptr_eq(node, &old))
                && let Err(error) =
                    self.deliver_drag(old.serial, DragPhase::Leave, DragRole::Observer)
            {
                failure.get_or_insert(error);
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        for observer in path {
            self.deliver_drag(observer.serial, DragPhase::Over, DragRole::Observer)?;
        }
        Ok(())
    }

    fn deliver_drag(
        &mut self,
        serial: u64,
        phase: DragPhase,
        role: DragRole,
    ) -> Result<DropResponse, RuntimeError> {
        let Some(session) = &self.drag else {
            return Ok(DropResponse::Rejected);
        };
        let payload = session.payload.clone();
        let permitted = session.permitted;
        let source = WidgetId(session.source.clone());
        let position = session.position.unwrap_or(session.last_position);
        let Some(preferred) = permitted.preferred(self.shared.modifiers.get()) else {
            return Ok(DropResponse::Rejected);
        };
        let Some(entry) = self.entries.get_mut(&serial) else {
            return Ok(DropResponse::Rejected);
        };
        if !entry.node.visible.get() {
            return Ok(DropResponse::Rejected);
        }
        let event = DragEvent {
            phase,
            role,
            position,
            payload: &payload,
            permitted,
            preferred,
            source: &source,
        };
        let response = entry.component.drag_event(
            &mut EventContext {
                node: entry.node.clone(),
            },
            &event,
        )?;
        if let DropResponse::Accepted(operation) = response
            && role == DragRole::Candidate
            && !permitted.contains(operation)
        {
            return Err(RuntimeError::DropNotPermitted);
        }
        Ok(response)
    }

    /// Ends the session and tells both sides, target first. `released` is the
    /// last button's release, inside or outside the window, after the target was
    /// resolved there; anything else cancels.
    ///
    /// Every participant is told even when one of them fails; the first error is
    /// returned afterwards.
    pub(super) fn finish_drag(&mut self, released: bool) -> Result<InputOutcome, RuntimeError> {
        if self.drag.is_none() {
            return Ok(InputOutcome::Ignored);
        }
        let mut failure = None;
        let target = self
            .drag
            .as_ref()
            .and_then(|session| session.target.clone())
            .filter(|(node, _)| node.mounted.get());
        let outcome = match target {
            Some((node, DropResponse::Accepted(_))) if released => {
                match self.deliver_drag(node.serial, DragPhase::Drop, DragRole::Candidate) {
                    Ok(DropResponse::Accepted(operation)) => DragOutcome::Dropped(operation),
                    Ok(DropResponse::Rejected | DropResponse::Forbidden) => DragOutcome::Refused,
                    Err(error) => {
                        failure = Some(error);
                        DragOutcome::Cancelled
                    }
                }
            }
            Some((node, _)) => {
                if let Err(error) =
                    self.deliver_drag(node.serial, DragPhase::Leave, DragRole::Candidate)
                {
                    failure = Some(error);
                }
                DragOutcome::Cancelled
            }
            None if released => DragOutcome::Unclaimed,
            None => DragOutcome::Cancelled,
        };
        let observers = self
            .drag
            .as_mut()
            .map(|session| std::mem::take(&mut session.observers))
            .unwrap_or_default();
        for observer in observers.into_iter().filter(|node| node.mounted.get()) {
            if let Err(error) =
                self.deliver_drag(observer.serial, DragPhase::Leave, DragRole::Observer)
            {
                failure.get_or_insert(error);
            }
        }
        let Some(session) = self.drag.take() else {
            return Ok(InputOutcome::Handled);
        };
        self.shared.drag_source.set(None);
        self.shared.drag_lost.set(false);
        if let Some(preview) = session.preview
            && preview.0.mounted.get()
            && let Err(error) = self.unmount(&preview)
        {
            failure.get_or_insert(error);
        }
        let initiating = session.failure;
        let source = session.source;
        if source.window().capture.get() == Some(source.serial) {
            source.window().capture.set(None);
        }
        if source.mounted.get()
            && let Some(entry) = self.entries.get_mut(&source.serial)
            && let Err(error) = entry.component.drag_finished(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                outcome,
            )
        {
            failure.get_or_insert(error);
        }
        self.shared.record(
            crate::EventKind::DragEnd,
            Some(source.serial),
            None,
            || match (&failure, outcome) {
                (Some(error), _) => crate::EventStatus::Failed(error.to_string()),
                // The error that ended the session, when cleanup itself succeeded.
                (None, _) if initiating.is_some() => {
                    crate::EventStatus::Failed(initiating.clone().unwrap_or_default())
                }
                (None, DragOutcome::Dropped(_)) => crate::EventStatus::Succeeded,
                (None, DragOutcome::Refused) => {
                    crate::EventStatus::Failed("drop refused by its target".into())
                }
                // Nothing was dropped, whatever the source makes of it.
                (None, DragOutcome::Unclaimed | DragOutcome::Cancelled) => {
                    crate::EventStatus::Cancelled
                }
            },
        );
        // Ordinary hover resumes under the pointer at the next flush.
        self.window.shared.stack_hover_pending.set(true);
        self.window.shared.scene_dirty.set(true);
        failure.map_or(Ok(InputOutcome::Handled), Err)
    }

    /// Cancels from a path that cannot return an error; a participant's failure
    /// is queued for the next flush instead of lost.
    pub(super) fn cancel_drag_infallibly(&mut self) {
        let request = self.shared.drag_request.borrow_mut().take();
        if let Some(request) = request
            && let Err(error) = self.abandon_drag_request(request.source.upgrade())
        {
            self.window.focus_errors.push_back(error);
        }
        if self.drag.is_none() {
            self.shared.drag_source.set(None);
            self.shared.drag_lost.set(false);
            return;
        }
        if let Err(error) = self.finish_drag(false) {
            self.window.focus_errors.push_back(error);
        }
    }
}
