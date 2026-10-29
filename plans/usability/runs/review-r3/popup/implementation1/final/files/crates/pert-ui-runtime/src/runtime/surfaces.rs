//! Anchored presentation retains logical ownership while lifting drawing and hit order.
use super::*;
use crate::{SurfaceDismissal, SurfaceKind};

#[derive(Clone)]
pub(super) struct ActiveSurface {
    pub(super) root: u64,
    owner: u64,
    pub(super) kind: SurfaceKind,
    opener: Option<WidgetId>,
    admitted: bool,
    /// Whether focus was inside the surface's root at the last
    /// synchronization. Focus that leaves the root for another part of the
    /// owner, such as the popup's own anchor, still leaves the popup.
    focus_inside: bool,
    /// Whether focus moved from inside the surface's root to outside it since
    /// the last synchronization, even if it came back before that
    /// synchronization ran: a brief departure still leaves the popup.
    pub(super) departed: bool,
}
impl Runtime {
    /// Records, for every open surface whose root holds `from` but not `to`,
    /// that focus left it, so the next synchronization dismisses a nonmodal
    /// one even when focus returns before that synchronization runs.
    pub(super) fn note_surface_departures(&mut self, from: u64, to: u64) {
        let departed: Vec<_> = self
            .window
            .surfaces
            .iter()
            .enumerate()
            .filter(|(_, surface)| {
                surface.admitted
                    && self.descendant(from, surface.root)
                    && !self.descendant(to, surface.root)
            })
            .map(|(index, _)| index)
            .collect();
        for index in departed {
            self.window.surfaces[index].departed = true;
        }
    }
    /// Whether an application-modal root still blocks: it is visible, or only
    /// its native root is hidden (minimized or occluded), which suspends the
    /// session instead of ending it.
    pub(super) fn application_modal_active(&self, root: u64) -> bool {
        let Some(entry) = self.entries.get(&root) else {
            return false;
        };
        if self.surface_kind(root) != Some(SurfaceKind::ApplicationDialog) {
            return false;
        }
        if entry.node.visible.get() {
            return true;
        }
        if entry.node.window().visible.get() {
            return false;
        }
        let mut serial = root;
        loop {
            let Some(entry) = self.entries.get(&serial) else {
                return false;
            };
            if !entry.node.local_visible.get() {
                return false;
            }
            if entry.node.detached.get() {
                return true;
            }
            let Some(parent) = entry.node.logical_parent_serial() else {
                return true;
            };
            if !self
                .entries
                .get(&parent)
                .is_some_and(|p| p.component.child_visible(&entry.key))
            {
                return false;
            }
            serial = parent;
        }
    }
    pub(super) fn modal_allows(&self, serial: u64) -> bool {
        self.modal_allows_among(serial, &self.window.surfaces)
    }
    /// Like [`Self::modal_allows`], but against the surfaces of the window
    /// that presents `serial`, whichever window is selected; content search
    /// walks every window.
    pub(super) fn modal_allows_in_own_window(&self, serial: u64) -> bool {
        let Some(entry) = self.entries.get(&serial) else {
            return false;
        };
        let window = entry.node.window();
        if Rc::ptr_eq(&window, &self.window.shared) {
            return self.modal_allows(serial);
        }
        match self
            .parked_windows
            .values()
            .find(|data| Rc::ptr_eq(&data.shared, &window))
        {
            Some(data) => self.modal_allows_among(serial, &data.surfaces),
            None => self.modal_allows(serial),
        }
    }
    // Failed admission and a known root changing into an application dialog
    // block every window before synchronization. Keep candidates separate from
    // the admitted singleton so real admission still runs conflict, focus and
    // cancellation policy normally.
    pub(super) fn pending_application_modals(&self) -> impl Iterator<Item = u64> + '_ {
        self.window
            .surfaces
            .iter()
            .chain(
                self.parked_windows
                    .values()
                    .flat_map(|window| &window.surfaces),
            )
            .filter(|surface| !surface.admitted || surface.kind != SurfaceKind::ApplicationDialog)
            .map(|surface| surface.root)
            .filter(|root| self.application_modal_active(*root))
    }
    /// Current modal policy of a live root. Cached admission kind describes the
    /// last synchronized frame, not changes awaiting the next surface pass.
    pub(super) fn active_modal_kind(&self, root: u64) -> Option<SurfaceKind> {
        let entry = self.entries.get(&root)?;
        if !entry.node.mounted.get() || !entry.node.visible.get() {
            return None;
        }
        self.surface_kind(root).filter(|kind| kind.modal())
    }
    fn modal_allows_among(&self, serial: u64, surfaces: &[ActiveSurface]) -> bool {
        if self
            .shared
            .application_modal
            .get()
            .into_iter()
            .chain(self.pending_application_modals())
            .any(|root| self.application_modal_active(root) && !self.descendant(serial, root))
        {
            return false;
        }
        surfaces.iter().rev().all(|surface| {
            self.active_modal_kind(surface.root).is_none_or(|kind| {
                self.descendant(serial, surface.root)
                    || (kind == SurfaceKind::ViewDialog && !self.descendant(serial, surface.owner))
            })
        })
    }
    pub(super) fn modal_navigation_allows(&self, serial: u64) -> bool {
        self.modal_allows(serial)
            && self
                .window
                .surfaces
                .iter()
                .rev()
                .find(|s| {
                    s.kind.modal()
                        && self
                            .window
                            .focus
                            .as_ref()
                            .is_none_or(|id| self.descendant(id.0.serial, s.root))
                })
                .is_none_or(|s| self.descendant(serial, s.root))
    }

    /// Whether `root` is the root of an interactive surface this window
    /// admitted at its last synchronization and that its owner still
    /// declares with the admitted kind. Records awaiting admission after a
    /// failed synchronization, notifications and a kind the owner has changed
    /// since do not qualify; parked windows' records are not consulted.
    pub(super) fn admitted_interactive_surface(&self, root: u64) -> bool {
        self.window.surfaces.iter().any(|surface| {
            surface.root == root
                && surface.admitted
                && surface.kind.interactive()
                && self.surface_kind(root) == Some(surface.kind)
        })
    }
    pub(super) fn descendant(&self, mut serial: u64, ancestor: u64) -> bool {
        loop {
            if serial == ancestor {
                return true;
            }
            let Some(parent) = self
                .entries
                .get(&serial)
                .and_then(|e| e.node.presentation_parent_serial())
            else {
                return false;
            };
            serial = parent;
        }
    }
    pub(super) fn surface_kind(&self, serial: u64) -> Option<SurfaceKind> {
        let entry = self.entries.get(&serial)?;
        let parent = self
            .entries
            .get(&entry.node.presentation_parent_serial()?)?;
        parent.component.child_surface(&entry.key)
    }
    pub(super) fn tooltip_node(&self, mut serial: u64) -> bool {
        loop {
            if self.surface_kind(serial) == Some(SurfaceKind::Tooltip) {
                return true;
            }
            let Some(parent) = self
                .entries
                .get(&serial)
                .and_then(|e| e.node.presentation_parent_serial())
            else {
                return false;
            };
            serial = parent;
        }
    }
    pub(super) fn dismiss_owner(
        &mut self,
        owner: u64,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        let owns_surface = self
            .entries
            .get(&owner)
            .is_some_and(|e| e.children.iter().any(|id| self.surface_kind(*id).is_some()));
        if !owns_surface {
            return Ok(());
        }
        if let Some(entry) = self.entries.get_mut(&owner) {
            entry.component.dismiss_surface(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                reason,
            )?;
            // A surviving owner lays out again without its surface. An owner
            // that is being removed has nothing left to arrange, and marking it
            // would leave the window reporting layout work after `unmount`.
            if reason != SurfaceDismissal::OwnerRemoved {
                entry.node.invalidate(Invalidation::Layout);
            }
        }
        Ok(())
    }
    pub(super) fn defer_surface_dismissal(&mut self, owner: u64, reason: SurfaceDismissal) {
        if reason == SurfaceDismissal::OwnerRemoved {
            // Terminal cleanup cannot retry a retired incarnation. Keep each
            // cause, rather than coalescing distinct removed owners' failures.
            self.window.deferred_surface_dismissals.remove(&owner);
            if let Err(error) = self.dismiss_owner(owner, reason) {
                self.window.focus_errors.push_back(error);
            }
        } else if let Err(error) = self.dismiss_owner(owner, reason) {
            self.window
                .deferred_surface_dismissals
                .insert(owner, reason);
            self.window.surface_error.get_or_insert(error);
        }
    }
    pub(super) fn close_all_surfaces(&mut self, reason: SurfaceDismissal) {
        let owners: Vec<_> = self
            .window
            .surfaces
            .iter()
            .rev()
            .filter(|s| reason != SurfaceDismissal::WindowFocusLost || !s.kind.modal())
            .map(|s| s.owner)
            .collect();
        for owner in owners {
            self.defer_surface_dismissal(owner, reason);
        }
    }
    /// Dismisses shown help on an unmodified Escape, which it takes; a
    /// modified Escape and one with no help shown go on to editors, surfaces
    /// and components. Both key paths call this before their editing phase,
    /// so help is the first thing Escape dismisses.
    pub(super) fn tooltip_escape(&mut self) -> Result<bool, RuntimeError> {
        let unmodified = self.keyboard.modifiers() == crate::Modifiers::default();
        let tooltips: Vec<u64> = self
            .window
            .surfaces
            .iter()
            .filter(|s| unmodified && s.kind == SurfaceKind::Tooltip)
            .map(|s| s.owner)
            .collect();
        if tooltips.is_empty() {
            return Ok(false);
        }
        for owner in tooltips {
            self.dismiss_owner(owner, SurfaceDismissal::Escape)?;
        }
        self.flush(self.shared.now.get())?;
        Ok(true)
    }
    pub(super) fn surface_escape(&mut self) -> Result<bool, RuntimeError> {
        if self.tooltip_escape()? {
            return Ok(true);
        }
        if let Some(owner) = self
            .window
            .surfaces
            .iter()
            .rev()
            .find(|s| {
                s.kind.interactive()
                    && (s.kind != SurfaceKind::ViewDialog
                        || self
                            .window
                            .focus
                            .as_ref()
                            .is_some_and(|id| self.descendant(id.0.serial, s.root)))
            })
            .map(|s| s.owner)
        {
            self.dismiss_owner(owner, SurfaceDismissal::Escape)?;
            self.flush(self.shared.now.get())?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(super) fn surface_pointer(
        &mut self,
        event: crate::PointerEvent,
    ) -> Result<bool, RuntimeError> {
        use crate::PointerKind;
        if let PointerKind::Release(button) = event.kind()
            && self.window.dismissed_buttons.remove(&button)
        {
            return Ok(true);
        }
        let PointerKind::Press(button) = event.kind() else {
            return Ok(false);
        };
        let Some(top) = self.window.surfaces.iter().rev().find(|s| {
            s.kind.interactive()
                && (s.kind != SurfaceKind::ViewDialog
                    || self
                        .entries
                        .get(&s.owner)
                        .and_then(|e| e.node.clip.get())
                        .is_some_and(|r| r.contains(event.position())))
        }) else {
            return Ok(false);
        };
        let inside = |root| {
            self.entries
                .get(&root)
                .and_then(|e| e.node.clip.get())
                .is_some_and(|r| !event.is_outside() && r.contains(event.position()))
        };
        if inside(top.root) {
            return Ok(false);
        }
        if top.kind.modal() {
            self.window.dismissed_buttons.insert(button);
            return Ok(true);
        }
        let mut owners = Vec::new();
        let mut parent_press = false;
        for s in self
            .window
            .surfaces
            .iter()
            .rev()
            .filter(|s| s.kind.nonmodal())
        {
            if inside(s.root) {
                parent_press = top.kind == SurfaceKind::Submenu;
                break;
            }
            owners.push(s.owner);
        }
        if !parent_press {
            self.window.dismissed_buttons.insert(button);
        }
        for owner in owners {
            self.dismiss_owner(owner, SurfaceDismissal::OutsidePress)?;
        }
        self.flush(self.shared.now.get())?;
        Ok(!parent_press)
    }
    pub(super) fn sync_surfaces(&mut self) -> Result<(), RuntimeError> {
        let mut previous = self.window.surfaces.clone();
        let result = self.sync_surfaces_inner();
        if result.is_err() {
            // Admission can complete before a later candidate fails. Preserve
            // those visible modal blockers during recovery, including direct focus.
            for surface in &self.window.surfaces {
                if let Some(old) = previous.iter_mut().find(|old| old.root == surface.root) {
                    if surface.admitted {
                        *old = surface.clone();
                    }
                } else if surface.kind.modal() {
                    previous.push(surface.clone());
                }
            }
            // A preexisting deferred error can precede admission entirely. Keep
            // every already-laid-out modal fail-closed and project changes to
            // existing nonmodal roots. Retry real admission (safe focus and
            // cancellation) after the error is repaired.
            for root in self.paint_order() {
                if let Some(kind) = self.surface_kind(root)
                    && (kind.modal() || previous.iter().any(|s| s.root == root))
                    && !previous.iter().any(|s| s.root == root && s.kind == kind)
                    && let Some(owner) = self.entries[&root].node.presentation_parent_serial()
                    && self.entries[&owner]
                        .component
                        .child_visible(&self.entries[&root].key)
                {
                    let existing = previous.iter().position(|surface| surface.root == root);
                    let opener = existing
                        .map(|index| previous[index].opener.clone())
                        .unwrap_or_else(|| self.window.focus.clone());
                    let pending = ActiveSurface {
                        root,
                        owner,
                        kind,
                        opener,
                        admitted: false,
                        focus_inside: false,
                        departed: false,
                    };
                    if let Some(index) = existing {
                        previous[index] = pending;
                    } else {
                        previous.push(pending);
                    }
                }
            }
            self.window.surfaces = previous;
            self.window.shared.layout_pending.set(true);
        }
        result
    }
    fn sync_surfaces_inner(&mut self) -> Result<(), RuntimeError> {
        if self
            .shared
            .application_modal
            .get()
            .is_some_and(|root| !self.application_modal_active(root))
        {
            self.shared.application_modal.set(None);
        }
        if let Some(error) = self.window.surface_error.take() {
            return Err(error);
        }
        let retries = std::mem::take(&mut self.window.deferred_surface_dismissals);
        let retried = !retries.is_empty();
        for (owner, reason) in retries {
            self.defer_surface_dismissal(owner, reason);
        }
        if let Some(error) = self.window.surface_error.take() {
            return Err(error);
        }
        if retried && !self.window.shared.pending.borrow().is_empty() {
            return Ok(());
        } // Reevaluate visible child changes before admission; retire removed owners now.
        let candidates: Vec<_> = self
            .paint_order()
            .into_iter()
            .filter_map(|root| {
                let kind = self.surface_kind(root)?;
                let owner = self.entries[&root].node.presentation_parent_serial()?;
                Some((root, owner, kind))
            })
            .collect();
        let old = std::mem::take(&mut self.window.surfaces);
        // Keep surviving blockers in force during opener restoration.
        self.window.surfaces = old
            .iter()
            .filter_map(|surface| {
                let (_, owner, kind) = candidates
                    .iter()
                    .find(|(root, _, _)| *root == surface.root)?;
                let mut current = surface.clone();
                if current.kind != *kind {
                    current.owner = *owner;
                    current.kind = *kind;
                    current.admitted = false;
                    current.focus_inside = false;
                    current.departed = false;
                }
                Some(current)
            })
            .collect();
        for previous in old.iter().rev() {
            if candidates.iter().any(|(root, _, _)| *root == previous.root)
                || !previous.admitted
                || !previous.kind.interactive()
                || self
                    .window
                    .focus
                    .as_ref()
                    .is_some_and(|id| !self.descendant(id.0.serial, previous.root))
            {
                continue;
            }
            let eligible = |id: u64| {
                self.modal_allows(id)
                    && !self.tooltip_node(id)
                    && self.entries.get(&id).is_some_and(|e| {
                        e.node.visible.get()
                            && e.component.focusable()
                            && e.node.clip.get().is_some_and(|clip| !clip.is_empty())
                    })
            };
            let target = previous
                .opener
                .as_ref()
                .map(|id| id.0.serial)
                .filter(|id| eligible(*id))
                .or_else(|| {
                    self.paint_order().into_iter().find(|id| {
                        self.descendant(*id, previous.owner)
                            && eligible(*id)
                            && !self.entries[id].component.can_reveal()
                    })
                })
                .or_else(|| {
                    self.paint_order()
                        .into_iter()
                        .find(|id| self.descendant(*id, previous.owner) && eligible(*id))
                });
            if let Some(target) = target {
                let id = WidgetId(self.entries[&target].node.clone());
                self.focus_without_reveal(&id)?;
                self.window.restore_focus = false;
            }
        }
        self.window.surfaces.clear();
        for (root, owner, kind) in candidates {
            let valid_anchor = self.entries[&owner]
                .node
                .clip
                .get()
                .is_some_and(|clip| !clip.is_empty());
            let has_space = self.entries[&root]
                .node
                .clip
                .get()
                .is_some_and(|clip| !clip.is_empty());
            if kind == SurfaceKind::Notification {
                self.window.surfaces.push(ActiveSurface {
                    root,
                    owner,
                    kind,
                    opener: None,
                    admitted: true,
                    focus_inside: false,
                    departed: false,
                });
                continue;
            }
            if (!valid_anchor && !kind.modal()) || !has_space {
                if let Some(previous) = old.iter().find(|s| s.root == root) {
                    self.window.surfaces.push(previous.clone());
                }
                self.dismiss_owner(
                    owner,
                    if valid_anchor {
                        SurfaceDismissal::NoSpace
                    } else {
                        SurfaceDismissal::AnchorClipped
                    },
                )?;
                continue;
            }
            if let Some(previous) = old
                .iter()
                .find(|s| s.root == root && s.kind == kind && s.admitted)
            {
                let focus = self.window.focus.as_ref().map(|id| id.0.serial);
                let inside = focus.map(|serial| self.descendant(serial, root));
                // Focus leaves a nonmodal surface when it moves outside its
                // owner, or out of its root after having been inside it: Tab
                // from the popup to its own anchor, which lies in the owner
                // but not in the popup, leaves the popup too.
                if kind.nonmodal()
                    && (focus.is_some_and(|serial| !self.descendant(serial, owner))
                        || previous.departed
                        || (previous.focus_inside && inside == Some(false)))
                {
                    self.dismiss_owner(owner, SurfaceDismissal::FocusLeft)?;
                    continue;
                }
                let mut kept = previous.clone();
                kept.focus_inside = inside.unwrap_or(previous.focus_inside);
                kept.departed = false;
                self.window.surfaces.push(kept);
                continue;
            }
            if kind.modal()
                && self
                    .shared
                    .application_modal
                    .get()
                    .into_iter()
                    .chain(self.pending_application_modals())
                    .any(|active| !self.descendant(owner, active) && root != active)
            {
                self.dismiss_owner(owner, SurfaceDismissal::ModalConflict)?;
                return Err(RuntimeError::ModalConflict);
            }
            if kind.modal()
                && self.window.surfaces.iter().any(|s| {
                    s.kind.modal()
                        && !self.descendant(owner, s.root)
                        && (matches!(kind, SurfaceKind::Dialog | SurfaceKind::ApplicationDialog)
                            || matches!(
                                s.kind,
                                SurfaceKind::Dialog | SurfaceKind::ApplicationDialog
                            )
                            || self.descendant(owner, s.owner)
                            || self.descendant(s.owner, owner))
                })
            {
                self.dismiss_owner(owner, SurfaceDismissal::ModalConflict)?;
                return Err(RuntimeError::ModalConflict);
            }
            if kind == SurfaceKind::ApplicationDialog {
                let conflict = self
                    .parked_windows
                    .values()
                    .any(|window| window.surfaces.iter().any(|surface| surface.kind.modal()));
                if conflict || !self.window.shared.visible.get() {
                    self.dismiss_owner(owner, SurfaceDismissal::ModalConflict)?;
                    return Err(RuntimeError::ModalConflict);
                }
            }
            if !self.modal_allows(owner) {
                self.dismiss_owner(owner, SurfaceDismissal::ModalConflict)?;
                continue;
            }
            // A window permits one nonmodal surface chain. Opening an unrelated
            // owner replaces the old chain; nested owners remain independently keyed.
            if kind.nonmodal() {
                let unrelated: Vec<_> = self
                    .window
                    .surfaces
                    .iter()
                    .filter(|s| s.kind.nonmodal() && !self.descendant(owner, s.root))
                    .map(|s| s.owner)
                    .collect();
                for other in unrelated {
                    self.dismiss_owner(other, SurfaceDismissal::Replaced)?;
                }
            }
            let opener = old
                .iter()
                .find(|surface| surface.root == root)
                .map(|surface| surface.opener.clone())
                .unwrap_or_else(|| self.window.focus.clone());
            self.window.surfaces.push(ActiveSurface {
                root,
                owner,
                kind,
                opener,
                admitted: true,
                focus_inside: false,
                departed: false,
            });
            if kind == SurfaceKind::ApplicationDialog {
                self.shared.application_modal.set(Some(root));
                self.cancel_blocked_held();
                let current = self.current_window();
                for window in self
                    .native_windows()
                    .into_iter()
                    .filter(|window| *window != current)
                {
                    self.select_window(&window)?;
                    let cancelled = self.cancel_input();
                    self.select_window(&current)?;
                    cancelled?;
                }
            }
            self.cancel_blocked_held();
            if kind.interactive() {
                if kind.modal() {
                    // Cancel the old editing context without invoking focus-loss
                    // save policy a second time. Physical pressed keys remain held.
                    let recipients = self.input_recipients();
                    self.cancel_recipients(recipients);
                    // The cancelled ancestors are forgotten: a modal taking
                    // focus is a cancellation, not a departure to announce.
                    self.window.focus_within.clear();
                    if let Some(old) = self.window.focus.take() {
                        old.0.invalidate_focus_path();
                    }
                    self.window.shared.focused.set(None);
                    self.window.shared.focus_node.borrow_mut().take();
                } else if let Some(captured) = self.window.shared.capture.take()
                    && let Some(entry) = self.entries.get_mut(&captured)
                {
                    entry.node.lose_context();
                    entry.node.input_cancelled.set(true);
                    entry.component.cancel_input(&mut EventContext {
                        node: entry.node.clone(),
                    });
                }
                let eligible: Vec<_> = self
                    .paint_order()
                    .into_iter()
                    .filter(|id| {
                        self.descendant(*id, root)
                            && !self.tooltip_node(*id)
                            && self.entries[id].component.focusable()
                            && self.entries[id]
                                .node
                                .clip
                                .get()
                                .is_some_and(|clip| !clip.is_empty())
                    })
                    .collect();
                let target = eligible
                    .iter()
                    .copied()
                    .find(|id| !self.entries[id].component.can_reveal())
                    .or_else(|| eligible.first().copied());
                if let Some(target) = target.or(kind.modal().then_some(root)) {
                    let id = WidgetId(self.entries[&target].node.clone());
                    self.focus_without_reveal(&id)?;
                }
                let inside = self
                    .window
                    .focus
                    .as_ref()
                    .is_some_and(|id| self.descendant(id.0.serial, root));
                if let Some(admitted) = self.window.surfaces.iter_mut().find(|s| s.root == root) {
                    admitted.focus_inside = inside;
                }
            }
        }
        Ok(())
    }
    pub(super) fn update_hover_path(&mut self, hit: Option<u64>) {
        let mut changed = Vec::new();
        let mut path = Vec::new();
        let mut current = hit;
        while let Some(id) = current {
            path.push(id);
            current = self
                .entries
                .get(&id)
                .and_then(|e| e.node.presentation_parent_serial());
        }
        // Only old/new ancestry is touched; stationary pointer input does not
        // invalidate unrelated components or create a polling deadline.
        let mut old = self.window.pointer_target.as_ref().map(|id| id.0.serial);
        while let Some(id) = old {
            let Some(entry) = self.entries.get(&id) else {
                break;
            };
            let hover = path.contains(&id);
            if entry.node.hover_within.replace(hover) != hover {
                changed.push((id, hover));
                if entry.node.hover_reader.get() {
                    entry.node.invalidate(Invalidation::Paint);
                }
            }
            old = entry.node.presentation_parent_serial();
        }
        for id in path {
            let node = &self.entries[&id].node;
            if !node.hover_within.replace(true) {
                changed.push((id, true));
                if node.hover_reader.get() {
                    node.invalidate(Invalidation::Paint);
                }
            }
        }
        for (serial, hovered) in changed {
            if let Some(entry) = self.entries.get_mut(&serial) {
                entry.component.hover_changed(
                    &mut EventContext {
                        node: entry.node.clone(),
                    },
                    hovered,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrangeContext, Button, Component, Element, Rect, UpdateContext, Viewport};
    use std::cell::Cell;

    struct ChangingSurface {
        kind: Rc<Cell<SurfaceKind>>,
        shown: Rc<Cell<bool>>,
        children: Vec<Element>,
    }
    impl Component for ChangingSurface {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn take_children(&mut self) -> Vec<Element> {
            std::mem::take(&mut self.children)
        }
        fn child_visible(&self, key: &str) -> bool {
            key != "popup" || self.shown.get()
        }
        fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
            (key == "popup").then(|| self.kind.get())
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: crate::Size) -> Result<(), RuntimeError> {
            cx.place(0, Rect::new(0.0, 0.0, 80.0, 32.0)?)?;
            if cx.len() > 1 {
                cx.place(1, Rect::new(100.0, 50.0, 100.0, 80.0)?)?;
            }
            Ok(())
        }
        fn dismiss_surface(
            &mut self,
            cx: &mut EventContext,
            _: SurfaceDismissal,
        ) -> Result<(), RuntimeError> {
            self.shown.set(false);
            cx.invalidate(Invalidation::Layout);
            Ok(())
        }
    }
    fn fixture(
        kind: SurfaceKind,
    ) -> (
        Runtime,
        WidgetId,
        WidgetId,
        Rc<Cell<SurfaceKind>>,
        Rc<Cell<bool>>,
    ) {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let kind = Rc::new(Cell::new(kind));
        let shown = Rc::new(Cell::new(false));
        let root = ui
            .mount_tree(Element::new(
                "owner",
                ChangingSurface {
                    kind: kind.clone(),
                    shown: shown.clone(),
                    children: vec![
                        Element::new("opener", Button::new("Open", |_| Ok(()))),
                        Element::new("popup", Button::new("Inside", |_| Ok(()))),
                    ],
                },
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let opener = ui.children(&root).unwrap()[0].clone();
        ui.focus(&opener).unwrap();
        shown.set(true);
        root.0.invalidate(Invalidation::Layout);
        ui.flush(Duration::ZERO).unwrap();
        (ui, root, opener, kind, shown)
    }
    #[test]
    fn changed_modal_kind_after_early_error_blocks_and_retries_admission() {
        let (mut ui, root, opener, kind, shown) = fixture(SurfaceKind::ContextMenu);
        kind.set(SurfaceKind::Dialog);
        ui.window.surface_error = Some(crate::ApplicationError::from("deferred failure").into());
        assert!(ui.sync_surfaces().is_err());
        assert_eq!(ui.window.surfaces.len(), 1);
        assert_eq!(ui.window.surfaces[0].kind, SurfaceKind::Dialog);
        assert!(!ui.window.surfaces[0].admitted);
        assert_eq!(ui.window.surfaces[0].opener, Some(opener.clone()));
        assert_eq!(ui.focus(&opener), Err(RuntimeError::ModalBlocked));
        ui.sync_surfaces().unwrap();
        assert!(ui.window.surfaces[0].admitted);
        assert_eq!(ui.window.surfaces[0].opener, Some(opener.clone()));
        shown.set(false);
        root.0.invalidate(Invalidation::Layout);
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(ui.focused(), Some(opener));
    }
    #[test]
    fn changed_nonmodal_kind_after_early_error_releases_and_retries_admission() {
        let (mut ui, _, opener, kind, _) = fixture(SurfaceKind::Dialog);
        kind.set(SurfaceKind::ContextMenu);
        ui.window.surface_error = Some(crate::ApplicationError::from("deferred failure").into());
        assert!(ui.sync_surfaces().is_err());
        assert_eq!(ui.window.surfaces[0].kind, SurfaceKind::ContextMenu);
        assert!(!ui.window.surfaces[0].admitted);
        assert_eq!(ui.window.surfaces[0].opener, Some(opener.clone()));
        ui.focus(&opener).unwrap();
        ui.sync_surfaces().unwrap();
        assert!(ui.window.surfaces[0].admitted);
        assert_eq!(ui.window.surfaces[0].opener, Some(opener));
    }
    #[test]
    fn pending_application_dialog_blocks_other_windows_before_admission() {
        let (mut ui, _, opener, kind, _) = fixture(SurfaceKind::ContextMenu);
        let primary = ui.primary_window();
        let view = ui
            .mount_tree(Element::new("other", Button::new("Other", |_| Ok(()))))
            .unwrap();
        let prepared = ui
            .prepare_detach(&view, crate::NativeWindowOptions::new("Other").unwrap())
            .unwrap();
        let other = ui.commit_detach(prepared).unwrap();
        ui.select_window(&other).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.select_window(&primary).unwrap();
        kind.set(SurfaceKind::ApplicationDialog);
        ui.window.surface_error = Some(crate::ApplicationError::from("deferred failure").into());
        assert!(ui.sync_surfaces().is_err());
        assert!(!ui.window.surfaces[0].admitted);
        assert_eq!(ui.focus(&opener), Err(RuntimeError::ModalBlocked));
        ui.select_window(&other).unwrap();
        assert_eq!(ui.focus(&view), Err(RuntimeError::ModalBlocked));
        // Queries do not flush pending admission. A command without a content
        // target or focused recipient must still report globally blocked.
        let command = ui
            .register_command(
                "Application",
                crate::CommandScope::Application,
                vec![],
                |_, _| Ok(()),
            )
            .unwrap();
        let mut context = ui.command_context(crate::CommandSource::Programmatic);
        context.target = None;
        context.focused = None;
        assert!(!ui.command_info(&command, &context).unwrap().enabled);
        ui.select_window(&primary).unwrap();
        ui.sync_surfaces().unwrap();
        assert!(ui.window.surfaces[0].admitted);
        ui.select_window(&other).unwrap();
        assert_eq!(ui.focus(&view), Err(RuntimeError::ModalBlocked));
        ui.select_window(&primary).unwrap();
        kind.set(SurfaceKind::ContextMenu);
        ui.sync_surfaces().unwrap();
        ui.select_window(&other).unwrap();
        ui.focus(&view).unwrap();
    }
    // Public input cannot observe the first two states: a press flushes the
    // pending layout, which synchronizes again before its focus walk.
    #[test]
    fn only_a_current_interactive_admission_keeps_press_focus() {
        let (mut ui, root, _, kind, _) = fixture(SurfaceKind::ContextMenu);
        let popup = ui.children(&root).unwrap()[1].0.serial;
        assert!(ui.admitted_interactive_surface(popup));
        assert!(!ui.admitted_interactive_surface(root.0.serial));
        // The owner declares another kind than the one admitted.
        kind.set(SurfaceKind::Popover);
        assert!(!ui.admitted_interactive_surface(popup));
        // A failed synchronization records that kind as awaiting admission.
        ui.window.surface_error = Some(crate::ApplicationError::from("deferred failure").into());
        assert!(ui.sync_surfaces().is_err());
        assert_eq!(ui.window.surfaces[0].kind, SurfaceKind::Popover);
        assert!(!ui.admitted_interactive_surface(popup));
        ui.sync_surfaces().unwrap();
        assert!(ui.admitted_interactive_surface(popup));
        // A notification is admitted but not interactive.
        kind.set(SurfaceKind::Notification);
        ui.sync_surfaces().unwrap();
        assert!(ui.window.surfaces[0].admitted);
        assert!(!ui.admitted_interactive_surface(popup));
    }
    #[test]
    fn application_dialog_downgrade_releases_application_blocker() {
        let (mut ui, root, opener, kind, _) = fixture(SurfaceKind::ApplicationDialog);
        assert!(ui.shared.application_modal.get().is_some());
        kind.set(SurfaceKind::ContextMenu);
        root.0.invalidate(Invalidation::Layout);
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(ui.shared.application_modal.get(), None);
        assert_eq!(ui.window.surfaces[0].kind, SurfaceKind::ContextMenu);
        ui.focus(&opener).unwrap();
    }
}
