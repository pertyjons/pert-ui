//! Press-and-release activation shared by buttons and choice controls.
use crate::*;

/// Hover, focus and the armed input of one activatable control.
///
/// A primary pointer press or an unmodified Enter/Space press arms the control.
/// The matching release activates it once: a pointer release only inside the
/// control, a key release only for the key that armed it. Whichever input arms
/// first owns the gesture; the other is ignored until it ends, so no
/// interleaving of pointer and key activates twice. Escape and
/// [`Self::reset`] (context loss, hiding, unmount) cancel without activating.
#[derive(Default)]
pub(crate) struct ActivationState {
    pub(crate) hovered: bool,
    pub(crate) focused: bool,
    pointer: bool,
    key: Option<KeyCode>,
}
impl ActivationState {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
    /// Whether a pointer press or an activation key is currently armed.
    pub(crate) fn pressed(&self) -> bool {
        self.pointer || self.key.is_some()
    }
    /// Whether an Enter or Space press, rather than the pointer, armed the
    /// gesture. Its release acts on whatever the control has chosen by then,
    /// so a list that highlights rows under the pointer reads this to keep
    /// pointer input from moving that choice until the key ends the gesture.
    pub(crate) fn key_armed(&self) -> bool {
        self.key.is_some()
    }
    /// Tracks hover and the primary button; returns whether this event activates.
    pub(crate) fn pointer(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<bool, RuntimeError> {
        let hovered = event.kind() != PointerKind::Leave && cx.pointer_inside();
        if hovered != self.hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if self.key.is_none() => {
                cx.capture_pointer()?;
                self.pointer = true;
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Release(PointerButton::Primary) => {
                let activate = self.pointer && cx.pointer_inside();
                self.pointer = false;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                return Ok(activate);
            }
            _ => {}
        }
        Ok(false)
    }
    /// Tracks Enter, Space and Escape; returns whether this event activates.
    /// OS repeats of the arming key neither re-arm nor activate. Escape cancels
    /// an armed input and does nothing while the control is idle.
    pub(crate) fn keyboard(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        control: KeyCode,
    ) -> bool {
        if control == KeyCode::Escape {
            if !self.pressed() {
                return false;
            }
            self.reset();
            cx.release_pointer();
            cx.invalidate(Invalidation::Paint);
            return false;
        }
        if !matches!(control, KeyCode::Enter | KeyCode::Space) {
            return false;
        }
        if key_result(control, self.pressed(), cx.modifiers()) == InputOutcome::Ignored {
            return false;
        }
        match event.transition {
            KeyTransition::Pressed if self.key.is_none() && !self.pointer => {
                self.key = Some(event.code)
            }
            KeyTransition::Released if self.key == Some(event.code) => {
                self.key = None;
                cx.invalidate(Invalidation::Paint);
                return true;
            }
            _ => {}
        }
        cx.invalidate(Invalidation::Paint);
        false
    }
}
/// Whether a key event reaching an activatable control belongs to it.
///
/// Unmodified Enter and Space do. While armed, the control owns every Enter,
/// Space and Escape event, including after modifiers change. Escape cancels the
/// gesture; an idle control leaves Escape to its ancestors and to
/// View, Window and Application commands. Call this before
/// [`ActivationState::keyboard`], which clears the armed state.
pub(crate) fn key_result(code: KeyCode, armed: bool, modifiers: Modifiers) -> InputOutcome {
    if armed || (code != KeyCode::Escape && modifiers == Modifiers::default()) {
        InputOutcome::Handled
    } else {
        InputOutcome::Ignored
    }
}
/// Whether a navigation key with `modifiers` steps a value control: plain or
/// with Shift, the fine step. Control, Alt and Super chords belong to command
/// bindings.
pub(crate) fn steps_with(modifiers: Modifiers) -> bool {
    !modifiers.control && !modifiers.alt && !modifiers.super_key
}
/// Whether `modifiers` are Alt alone, the documented reorder chord of lists,
/// trees, tables and tabs; chords that add Shift, Control or Super belong to
/// command bindings.
pub(crate) fn alt_only(modifiers: Modifiers) -> bool {
    modifiers.alt && !modifiers.shift && !modifiers.control && !modifiers.super_key
}
/// Whether `code` is one of the keys [`ActivationState::keyboard`] handles.
pub(crate) fn activation_key(code: KeyCode) -> bool {
    matches!(code, KeyCode::Enter | KeyCode::Space | KeyCode::Escape)
}
/// Whether a pointer event reaching an enabled activatable control belongs to
/// it: primary presses and releases, motion and leaving stop propagation, and
/// every other button and wheel input is left to ancestors.
pub(crate) fn pointer_result(kind: PointerKind) -> InputOutcome {
    if matches!(
        kind,
        PointerKind::Press(PointerButton::Primary)
            | PointerKind::Release(PointerButton::Primary)
            | PointerKind::Move
            | PointerKind::Leave
    ) {
        InputOutcome::Handled
    } else {
        InputOutcome::Ignored
    }
}
