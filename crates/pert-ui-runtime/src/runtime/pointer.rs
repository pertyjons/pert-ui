use super::*;
use crate::{InputOutcome, PointerEvent, PointerKind};

impl Runtime {
    pub(super) fn pointer_hit(&self, point: crate::Point) -> Option<u64> {
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        self.drawing_order()
            .into_iter()
            .rev()
            .find_map(|(serial, foreground)| {
                let entry = &self.entries[&serial];
                if !entry
                    .node
                    .clip
                    .get()
                    .is_some_and(|clip| clip.contains(point))
                    || self.tooltip_node(serial)
                    || !self.modal_allows(serial)
                    || self.in_drag_preview(serial)
                {
                    return None;
                }
                let local = entry.node.window_to_local(point)?;
                let hits = if foreground {
                    entry.component.hit_test_foreground(local)
                } else {
                    entry.component.hit_test(local)
                };
                if !hits {
                    return None;
                }
                Some(serial)
            })
    }
    // Documented on `crate::host::HostIntegration::cursor_icon`.
    pub(crate) fn cursor_icon(&self) -> crate::CursorIcon {
        if let Some(icon) = self.drag_cursor() {
            return icon;
        }
        let Some(point) = self.window.shared.pointer_position.get() else {
            return crate::CursorIcon::Default;
        };
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let mut target = self
            .window
            .shared
            .capture
            .get()
            .or_else(|| self.pointer_hit(point));
        while let Some(serial) = target {
            let entry = &self.entries[&serial];
            if let Some(local) = entry.node.window_to_local(point)
                && let Some(icon) = entry.component.cursor(local)
            {
                return icon;
            }
            target = entry.node.presentation_parent_serial();
        }
        crate::CursorIcon::Default
    }

    pub(super) fn refresh_hover(&mut self) -> Result<(), RuntimeError> {
        // A drag session suspends ordinary hover and resolves its own target.
        if self.drag_in_window() {
            return Ok(());
        }
        let Some(point) = self.window.shared.pointer_position.get() else {
            return Ok(());
        };
        // A stationary pointer may enter another subregion of the same component
        // after layout (split track, tab title, or a custom widget's hit regions).
        self.pointer_input(PointerEvent::new(point, PointerKind::Move).refreshed())?;
        Ok(())
    }

    /// Asks each scrolling ancestor, innermost first, to bring `id` into view,
    /// settling layout after each one that moved. A settling flush can change
    /// the tree, so the path is rebuilt from the live target after every
    /// flush: the walk stops when the target is unmounted or hidden, or when
    /// the ancestor it just asked is no longer on the target's path.
    pub(super) fn reveal_focus(&mut self, id: &WidgetId) -> Result<(), RuntimeError> {
        let mut parent = id.0.presentation_parent_serial();
        while let Some(serial) = parent {
            if !id.0.mounted.get() || !id.0.visible.get() {
                break;
            }
            let Some(bounds) = id.0.bounds.get() else {
                break;
            };
            let surface = self.surface_kind(serial).is_some();
            let Some(entry) = self.entries.get_mut(&serial) else {
                break;
            };
            if entry.component.reveal(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                bounds,
            )? {
                self.flush(self.shared.now.get())?;
            }
            if surface || !id.0.mounted.get() {
                break;
            }
            // Continue above `serial` only while it is still on the path.
            let mut next = id.0.presentation_parent_serial();
            parent = None;
            while let Some(ancestor) = next {
                next = self
                    .entries
                    .get(&ancestor)
                    .and_then(|entry| entry.node.presentation_parent_serial());
                if ancestor == serial {
                    parent = next;
                    break;
                }
            }
        }
        Ok(())
    }
    fn revealable(&self, serial: u64) -> bool {
        let mut parent = self.entries[&serial].node.presentation_parent_serial();
        while let Some(serial) = parent {
            let entry = &self.entries[&serial];
            if entry.component.can_reveal() {
                return true;
            }
            parent = entry.node.presentation_parent_serial();
        }
        false
    }
    /// Moves focus in visible tree order in `direction`, wrapping at either end.
    /// Hidden or fully clipped components and nonfocusable containers are skipped.
    /// Uses current layout.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; forwards the errors of the
    /// [`Self::flush`] that runs first when layout is pending, of an ancestor's
    /// [`Component::reveal`](crate::Component::reveal), and of [`Self::focus`].
    pub fn focus_next(&mut self, direction: crate::FocusDirection) -> Result<(), RuntimeError> {
        let reverse = direction == crate::FocusDirection::Backward;
        self.shared.writable()?;
        if self.window.shared.layout_pending.get() {
            self.flush(self.shared.now.get())?;
        }
        let order = self.navigation_order();
        let focused = self.window.focus.as_ref().map(|id| id.0.serial);
        // The focused component keeps its place in the order even when it is
        // not a tab stop itself, so traversal continues from where it is.
        let mut current = None;
        let mut ids = Vec::new();
        for serial in order {
            let entry = &self.entries[&serial];
            let stop = !self.tooltip_node(serial)
                && self.modal_navigation_allows(serial)
                && entry.component.focusable()
                && entry.component.tab_stop()
                && (self.revealable(serial)
                    || entry.node.clip.get().is_some_and(|clip| !clip.is_empty()));
            // A scope's descendants are no stops, but the focused one keeps
            // its place, so traversal from it continues after the scope.
            let stop = stop && !self.outside_tab_scope(serial);
            if Some(serial) == focused {
                // Between the stops before it and the stops after it.
                current = Some((ids.len(), stop));
            }
            if stop {
                ids.push(serial);
            }
        }
        if ids.is_empty() {
            return Ok(());
        }
        let index = match (current, reverse) {
            (None, true) => ids.len() - 1,
            (None, false) => 0,
            // `at` stops precede the focused component; when it is a stop
            // itself it is `ids[at]`.
            (Some((at, _)), true) => (at + ids.len() - 1) % ids.len(),
            (Some((at, true)), false) => (at + 1) % ids.len(),
            (Some((at, false)), false) => at % ids.len(),
        };
        for offset in 0..ids.len() {
            let index = if reverse {
                (index + ids.len() - offset) % ids.len()
            } else {
                (index + offset) % ids.len()
            };
            // A reveal's settling flush can unmount or hide later candidates,
            // or this one; those are skipped.
            let Some(entry) = self.entries.get(&ids[index]) else {
                continue;
            };
            let id = WidgetId(entry.node.clone());
            self.reveal_focus(&id)?;
            // Settling the reveal can also change which scopes exclude it.
            if !id.0.mounted.get() || !id.0.visible.get() || self.outside_tab_scope(id.0.serial) {
                continue;
            }
            if id.0.clip.get().is_some_and(|clip| !clip.is_empty()) {
                return self.focus(&id);
            }
        }
        Ok(())
    }

    /// Whether an ancestor keeps this component out of Tab traversal through
    /// [`Component::descendant_tab_stops`]. The walk follows presentation
    /// parents up to the root and stops at a surface root, which starts a
    /// traversal scope of its own; a detached window root has no
    /// presentation parent, so it ends the walk too.
    pub(super) fn outside_tab_scope(&self, mut serial: u64) -> bool {
        loop {
            if self.surface_kind(serial).is_some() {
                return false;
            }
            let Some(parent) = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.presentation_parent_serial())
            else {
                return false;
            };
            match self.entries.get(&parent) {
                Some(entry) if !entry.component.descendant_tab_stops() => return true,
                Some(_) => serial = parent,
                None => return false,
            }
        }
    }

    /// The traversal direction the held modifiers give Tab: Shift reverses it.
    pub(super) fn tab_direction(&self) -> crate::FocusDirection {
        crate::FocusDirection::from_shift(self.keyboard.modifiers().shift)
    }

    /// Offers an unmodified Tab or Shift+Tab that reached default traversal to
    /// the focused component and the ancestors ordinary bubbling reaches,
    /// innermost first ([`Component::tab_navigation`]). Returns whether one
    /// claimed it, in which case traversal does not run.
    pub(super) fn offer_tab(
        &mut self,
        event: crate::KeyEvent,
        mut recipients: Option<&mut Vec<WidgetId>>,
    ) -> Result<bool, RuntimeError> {
        let direction = self.tab_direction();
        let mut serial = match &self.window.focus {
            Some(id) => id.0.serial,
            None => return Ok(false),
        };
        loop {
            if !self.modal_allows(serial) {
                return Ok(false);
            }
            let Some(entry) = self.entries.get_mut(&serial) else {
                return Ok(false);
            };
            entry.node.input_cancelled.set(false);
            if let Some(recipients) = recipients.as_deref_mut() {
                recipients.push(WidgetId(entry.node.clone()));
            }
            let outcome = entry.component.tab_navigation(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                direction,
                event.transition,
            )?;
            if outcome == InputOutcome::Handled {
                return Ok(true);
            }
            match entry.node.presentation_parent_serial() {
                Some(parent) => serial = parent,
                None => return Ok(false),
            }
        }
    }

    /// Applies a pointer transition and dispatches through the frontmost clipped tree.
    ///
    /// While a [drag session](EventContext::start_drag) is active in this window,
    /// transitions drive the session instead: motion resolves the drop target and
    /// moves the preview, the wheel still scrolls what is under the pointer, and
    /// releasing the last button drops or cancels. Ordinary hover, press focus
    /// and tooltips are suspended until the session ends.
    ///
    /// Press focuses the nearest focusable ancestor without scrolling the already
    /// hit target. Explicit focus and keyboard traversal still reveal their target.
    /// Ignored events bubble toward
    /// the root; they never fall through to covered siblings. Capture is released
    /// after the last physical button goes up or the owner loses context. Input
    /// state is updated even if a callback returns an error. Coordinates are logical
    /// window pixels; the platform adapter handles native DPI and wheel units.
    ///
    /// The event is delivered at once and its recipient writes what its own
    /// rules write, as [`Self::text_input`] describes; the tree, derived
    /// bindings and layout show the result after the next [`Self::flush`].
    /// A click is a press and a release at one point inside the target's
    /// [`Self::bounds`], which are known once a flush has laid it out; a
    /// component scrolled out of view or inside a closed container is revealed
    /// first with [`Self::reveal`]. [`Driver::click`](crate::testing::Driver::click)
    /// does all of this.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::testing::*;
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    /// let clicks = ui.signal(0);
    /// let counter = clicks.clone();
    /// let button = ui.mount_tree(Element::new("add",
    ///     Button::new("Add", move |_| counter.update(|n| *n += 1))))?;
    /// ui.flush(Duration::ZERO)?; // lays the button out
    /// let [x, y, width, height] = ui.bounds(&button)?.ok_or("not laid out")?.values();
    /// let middle = Point::new(x + width / 2.0, y + height / 2.0)?;
    /// for kind in [
    ///     PointerKind::Press(PointerButton::Primary),
    ///     PointerKind::Release(PointerButton::Primary),
    /// ] {
    ///     ui.pointer_input(PointerEvent::new(middle, kind))?;
    /// }
    /// assert_eq!(clicks.get()?, 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Propagates pending update/layout errors, callback errors (including those of
    /// the flush that delivers a notification the event requested) and phase
    /// restrictions.
    /// Older or external notification failures, and collective settlement errors
    /// without a uniquely attributable request, propagate without cancelling the
    /// current capture or drag. A failure attributable to this dispatch cancels it.
    /// If a preflush prevents the last held button's release from reaching its
    /// capture owner, cleanup still ends that owner's gesture.
    /// A previous recipient's focus-loss error is returned after delivering the
    /// press to its new target, without canceling that target's successful capture.
    pub fn pointer_input(&mut self, event: PointerEvent) -> Result<InputOutcome, RuntimeError> {
        let operation = match event.kind() {
            PointerKind::Move => crate::ProfileOperation::PointerMove,
            PointerKind::Press(_) | PointerKind::Release(_) => {
                crate::ProfileOperation::PointerButton
            }
            PointerKind::Leave => crate::ProfileOperation::PointerLeave,
            PointerKind::Scroll(_) | PointerKind::Pinch(_) => crate::ProfileOperation::Scroll,
        };
        let _timing = self.shared.profiler.span(operation, None, "");

        self.shared.writable()?;
        let notifications = super::notify::InputNotificationGuard::new(self.shared.clone());
        notifications.activate();
        let terminal = matches!(event.kind(), PointerKind::Release(button)
            if self.window.buttons.contains(&button) && self.window.buttons.len() == 1);
        if self.drag_in_window() {
            let outcome = self.drag_pointer(event)?;
            // A notification a participant requested fails the session as the
            // participant's own error would.
            if let Err(error) = self.deliver_input_notifications() {
                if notifications.cancel_on_error(terminal) && self.drag.is_some() {
                    self.fail_drag(&error);
                }
                return Err(error);
            }
            return Ok(outcome);
        }
        let owner = self.window.shared.capture.get();
        if let Some(serial) = owner
            && let Some(entry) = self.entries.get(&serial)
        {
            entry.node.input_cancelled.set(false);
        }
        let result = self.pointer_transition(event);
        // A source that called `start_drag` in this dispatch: adopt its session
        // now, so the very next transition is already routed as a drag.
        let result = match (result, self.service_drag()) {
            (Ok(result), Ok(())) => Ok(result),
            (Err(error), _) | (Ok(_), Err(error)) => Err(error),
        };
        // A notification the event requested fails like the event itself.
        let result =
            result.and_then(|outcome| self.deliver_input_notifications().map(|()| outcome));
        if result.is_err()
            && notifications.cancel_on_error(terminal)
            && let Some(serial) = self.window.shared.capture.get().or(owner)
            && let Some(entry) = self.entries.get_mut(&serial)
            && !entry.node.input_cancelled.replace(true)
        {
            entry.node.lose_context();
            entry.component.cancel_input(&mut EventContext {
                node: entry.node.clone(),
            });
            entry.node.invalidate(Invalidation::Paint);
        }
        // A previous focus recipient's failure must not swallow this event or
        // cancel the new recipient's capture. Deliver it after normal dispatch.
        if result.is_ok()
            && let Some(error) = self.window.focus_errors.pop_front()
        {
            return Err(error);
        }
        result
    }
    fn pointer_transition(&mut self, event: PointerEvent) -> Result<InputOutcome, RuntimeError> {
        // Preserve capture ownership until pending zoom/density cancellation,
        // including the last release of a nonfocusable captured component.
        let geometry = if self.geometry_pending() {
            self.flush(self.shared.now.get()).map(|_| ())
        } else {
            Ok(())
        };
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
        let captured = self.window.shared.capture.get();
        // Physical release bookkeeping also survives failed layout/hover callbacks.
        if self.window.buttons.is_empty() {
            self.window.shared.capture.set(None);
        }
        if let PointerKind::Release(button) = event.kind()
            && self.window.dismissed_buttons.remove(&button)
        {
            return geometry.map(|_| InputOutcome::Handled);
        }
        geometry?;
        if !self.window.shared.pending.borrow().is_empty() {
            self.flush(self.shared.now.get())?;
        }
        if self.surface_pointer(event)? {
            return Ok(InputOutcome::Handled);
        }
        if self.window.shared.layout_pending.get() {
            self.flush(self.shared.now.get())?;
        }
        self.window
            .shared
            .pointer_position
            .set(if event.is_outside() {
                None
            } else {
                Some(event.position())
            });
        let hit = if event.is_outside() {
            None
        } else {
            self.pointer_hit(event.position())
        };
        self.update_hover_path(hit);
        let old = self
            .window
            .pointer_target
            .take()
            .filter(|id| id.0.mounted.get() && id.0.visible.get());
        self.window.pointer_target = hit.map(|serial| WidgetId(self.entries[&serial].node.clone()));
        if let Some(old) = old
            && Some(old.0.serial) != hit
        {
            self.dispatch_pointer(
                old.0.serial,
                PointerEvent::new(event.position(), PointerKind::Leave),
            )?;
        }
        let target = captured.or(hit);
        let result = if let Some(serial) = target {
            if matches!(event.kind(), PointerKind::Press(_))
                && self.window.shared.capture.get().is_none()
            {
                self.select_content_target(serial);
                self.raise_ancestors(serial);
                let mut focus = Some(serial);
                let mut focus_error = None;
                let queued = self.window.focus_errors.len();
                let focused = self.window.focus.clone();
                while let Some(current) = focus {
                    let entry = &self.entries[&current];
                    if entry.component.focusable() {
                        let id = WidgetId(entry.node.clone());
                        focus_error = self.focus_without_reveal(&id).err();
                        break;
                    }
                    // A press on a pressed component that is not focusable but
                    // holds focus inside it, such as the caption of a row whose
                    // editor is typing, leaves focus where it is. Ancestors
                    // passed on the way up do not: a press on other content
                    // still moves focus as before.
                    if current == serial
                        && focused.as_ref().is_some_and(|focused| {
                            Self::focus_ancestors(&focused.0).contains(&current)
                        })
                    {
                        break;
                    }
                    focus = entry.node.presentation_parent_serial();
                }
                // A focus-loss save can change this target's binding. Refresh
                // retained content/geometry before using it for the press (e.g.
                // caret placement). Defer its callback error until dispatch.
                // Errors the transfer queued after its first wait with it, so
                // the refresh below cannot take one and skip the press.
                let transfer_errors: Vec<_> = self.window.focus_errors.drain(queued..).collect();
                let prepared = self.flush(self.shared.now.get());
                if let Some(error) = focus_error {
                    self.window.focus_errors.push_back(error);
                }
                self.window.focus_errors.extend(transfer_errors);
                prepared?;
            }
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.dispatched.set(true);
            }
            #[cfg(debug_assertions)]
            let settled = event.is_refresh()
                && self.window.shared.pending.borrow().is_empty()
                && !self.window.shared.layout_pending.get()
                && self.shared.computations.borrow().is_empty();
            #[cfg(debug_assertions)]
            self.flush_state.refresh_handler.set(None);
            let outcome = self.dispatch_pointer(serial, event);
            #[cfg(debug_assertions)]
            if settled
                && (!self.window.shared.pending.borrow().is_empty()
                    || self.window.shared.layout_pending.get()
                    || !self.shared.computations.borrow().is_empty())
            {
                // Name the component that handled it, else the target.
                let recipient = self.flush_state.refresh_handler.take().unwrap_or(serial);
                *self
                    .flush_state
                    .refresh_work
                    .borrow_mut()
                    .entry(recipient)
                    .or_default() += 1;
            }
            outcome
        } else {
            Ok(InputOutcome::Ignored)
        };
        if self.window.buttons.is_empty() {
            self.window.shared.capture.set(None);
        }
        result
    }

    pub(super) fn dispatch_pointer(
        &mut self,
        mut serial: u64,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        loop {
            if !self.modal_allows(serial) {
                return Ok(InputOutcome::Handled);
            }
            let Some(entry) = self.entries.get_mut(&serial) else {
                return Ok(InputOutcome::Ignored);
            };
            if !entry.node.visible.get() {
                return Ok(InputOutcome::Ignored);
            }
            let operation = match event.kind() {
                PointerKind::Move => crate::ProfileOperation::PointerMove,
                PointerKind::Press(_) | PointerKind::Release(_) => {
                    crate::ProfileOperation::PointerButton
                }
                PointerKind::Leave => crate::ProfileOperation::PointerLeave,
                PointerKind::Scroll(_) | PointerKind::Pinch(_) => crate::ProfileOperation::Scroll,
            };
            let _timing = self
                .shared
                .profiler
                .span(operation, Some(serial), &entry.key);
            // A new delivered event can acquire a new context after a successful
            // geometry reset. Leave only clears hover; attaching a gesture or
            // capture explicitly clears the cancellation marker even during Leave.
            if event.kind() != PointerKind::Leave {
                entry.node.input_cancelled.set(false);
            }
            // A scroll displacement is a distance, so each recipient receives it
            // in its own local pixels: what is seen moves by what was scrolled.
            let delivered = match event.kind() {
                PointerKind::Scroll(delta) if entry.node.scale.get() != 1.0 => {
                    let scale = entry.node.scale.get();
                    let local =
                        crate::Point::new(delta.x() / scale, delta.y() / scale).unwrap_or(delta);
                    let scrolled = PointerEvent::new(event.position(), PointerKind::Scroll(local));
                    if event.is_outside() {
                        scrolled.outside()
                    } else {
                        scrolled
                    }
                }
                _ => event,
            };
            let result = entry.component.pointer_input(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                delivered,
            );
            if result.is_err()
                && !entry.node.input_cancelled.get()
                && (self.window.shared.capture.get() == Some(serial)
                    || entry
                        .node
                        .gestures
                        .borrow()
                        .iter()
                        .any(|gesture| gesture.strong_count() > 0))
            {
                entry.node.input_cancelled.set(true);
                entry.node.lose_context();
                entry.component.cancel_input(&mut EventContext {
                    node: entry.node.clone(),
                });
                entry.node.invalidate(Invalidation::Paint);
            }
            let result = result?;
            if result == InputOutcome::Handled {
                #[cfg(debug_assertions)]
                if event.is_refresh() {
                    self.flush_state.refresh_handler.set(Some(serial));
                }
                return Ok(result);
            }
            match entry.node.presentation_parent_serial() {
                Some(parent) => serial = parent,
                None => return Ok(result),
            }
        }
    }
    pub(super) fn dispatch_keyboard(
        &mut self,
        serial: u64,
        event: crate::KeyEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.dispatch_keyboard_tracked(serial, event, None)
    }
    pub(super) fn dispatch_keyboard_tracked(
        &mut self,
        mut serial: u64,
        event: crate::KeyEvent,
        mut recipients: Option<&mut Vec<WidgetId>>,
    ) -> Result<InputOutcome, RuntimeError> {
        // A key typed into a component holding secret input is not recorded,
        // neither at that component nor while it bubbles.
        let concealed = self
            .entries
            .get(&serial)
            .is_some_and(|entry| entry.component.conceals_input());
        loop {
            if !self.modal_allows(serial) {
                return Ok(InputOutcome::Ignored);
            }
            let Some(entry) = self.entries.get_mut(&serial) else {
                return Ok(InputOutcome::Ignored);
            };
            let _timing = self.shared.profiler.span(
                crate::ProfileOperation::KeyInput,
                Some(serial),
                &entry.key,
            );
            // A delivered key may begin new component-owned input after preparation
            // cancelled its previous context, including unfocused recipients.
            entry.node.input_cancelled.set(false);
            if let Some(recipients) = recipients.as_deref_mut() {
                recipients.push(WidgetId(entry.node.clone()));
            }
            let result = entry.component.keyboard_event(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                event,
                &self.keyboard,
            );
            if !concealed {
                self.shared.record(
                    crate::EventKind::KeyInput(event),
                    Some(serial),
                    None,
                    || match &result {
                        Ok(_) => crate::EventStatus::Succeeded,
                        Err(error) => crate::EventStatus::Failed(error.to_string()),
                    },
                );
            }
            if result? == InputOutcome::Handled {
                return Ok(InputOutcome::Handled);
            }
            match entry.node.presentation_parent_serial() {
                Some(parent) => serial = parent,
                None => return Ok(InputOutcome::Ignored),
            }
        }
    }
}
