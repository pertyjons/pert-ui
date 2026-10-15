//! Explicit sustained-key ownership, separate from ordinary component focus.
use super::*;
use crate::{CommandContext, CommandSource, InputOutcome, KeyCode, KeyTransition, Modifiers};

/// Stable sustained-input registration, removed explicitly or with its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct HeldInputId(WidgetId, u64);
/// Terminal cancellation of an admitted sustained press.
///
/// Reasons are added as the runtime gains ways to end a press, so matches need a
/// wildcard arm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeldCancellation {
    /// The keyboard focus left the window: [`Runtime::clear_focus`](crate::Runtime::clear_focus)
    /// ran, as the platform layer does when a native window loses focus or is
    /// hidden. Every press the runtime admitted is cancelled, whichever of its
    /// windows or views owns the registration, since releases may no longer
    /// arrive.
    FocusLost,
    /// The owner became hidden or was removed.
    Unavailable,
    /// The registration was explicitly removed.
    Unregistered,
    /// A modal boundary blocked the owner.
    Modal,
    /// Explicit detach/re-dock movement.
    PresentationMoved,
    /// An input callback failed; completed host work is not rolled back.
    InputFailure,
    /// Runtime destruction or explicit shutdown.
    Shutdown,
}
type Action =
    Box<dyn FnMut(KeyCode, KeyTransition, &CommandContext) -> Result<InputOutcome, RuntimeError>>;
type Cancel = Box<dyn FnMut(KeyCode, HeldCancellation)>;
pub(super) struct Registration {
    id: HeldInputId,
    keys: Vec<KeyCode>,
    modifiers: Modifiers,
    action: Action,
    cancel: Cancel,
}
#[derive(Clone)]
pub(super) struct Owner {
    registration: u64,
    context: CommandContext,
}
impl Runtime {
    /// Registers explicit physical-key capture for a visible owner in the invoking
    /// window. Text editing/composition has precedence. Keys and exact modifiers
    /// are position-based; modifier keys themselves cannot be owned.
    /// Duplicate keys in overlapping registrations are rejected in every build profile.
    ///
    /// Returning Handled for Pressed owns the key; repeats and release are routed
    /// solely here. The release record is removed before calling action. A callback
    /// error cancels any still-owned keys. Cancellation is a separate infallible
    /// cleanup callback: it must release host resources without activating commands.
    /// These UI-thread callbacks are not safe for time-critical producers and must return promptly.
    /// Lifetime follows the mounted owner, including native presentation movement;
    /// movement cancels current presses while preserving the registration.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; `ForeignRuntime` or
    /// `Unmounted` for a foreign or unmounted owner; `ShortcutConflict` for a modifier key,
    /// a key listed twice, or a key another registration of the same modifiers already
    /// owns; and `Exhausted` when registration identities run out.
    pub fn register_held_input(
        &mut self,
        owner: &WidgetId,
        keys: Vec<KeyCode>,
        modifiers: Modifiers,
        action: impl FnMut(
            KeyCode,
            KeyTransition,
            &CommandContext,
        ) -> Result<InputOutcome, RuntimeError>
        + 'static,
        cancel: impl FnMut(KeyCode, HeldCancellation) + 'static,
    ) -> Result<HeldInputId, RuntimeError> {
        self.shared.writable()?;
        self.check(owner)?;
        for (i, key) in keys.iter().enumerate() {
            if matches!(
                key,
                KeyCode::ShiftLeft
                    | KeyCode::ShiftRight
                    | KeyCode::ControlLeft
                    | KeyCode::ControlRight
                    | KeyCode::AltLeft
                    | KeyCode::AltRight
                    | KeyCode::SuperLeft
                    | KeyCode::SuperRight
            ) || keys[..i].contains(key)
                || self
                    .held_registrations
                    .values()
                    .any(|r| r.modifiers == modifiers && r.keys.contains(key))
            {
                return Err(RuntimeError::ShortcutConflict);
            }
        }
        self.held_serial = self
            .held_serial
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        let id = HeldInputId(owner.clone(), self.held_serial);
        self.held_registrations.insert(
            self.held_serial,
            Registration {
                id: id.clone(),
                keys,
                modifiers,
                action: Box::new(action),
                cancel: Box::new(cancel),
            },
        );
        Ok(id)
    }
    /// Deregisters an exact identity, canceling its admitted presses once before
    /// releasing callbacks. Late repeats/releases remain suppressed until release.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; and `StaleCommand` for an
    /// identity that is already unregistered or belongs to another registration.
    pub fn unregister_held_input(&mut self, id: &HeldInputId) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if !self
            .held_registrations
            .get(&id.1)
            .is_some_and(|r| r.id == *id)
        {
            return Err(RuntimeError::StaleCommand);
        }
        let keys: Vec<_> = self
            .held_owners
            .iter()
            .filter(|(_, owner)| owner.registration == id.1)
            .map(|(key, _)| *key)
            .collect();
        for key in keys {
            self.cancel_held_key(key, HeldCancellation::Unregistered);
        }
        self.held_registrations.remove(&id.1);
        Ok(())
    }
    fn cancel_held_key(&mut self, key: KeyCode, reason: HeldCancellation) {
        if let Some(owner) = self.held_owners.remove(&key) {
            self.suppressed_keys.insert(key);
            if let Some(registration) = self.held_registrations.get_mut(&owner.registration) {
                (registration.cancel)(key, reason);
            }
        }
    }
    pub(super) fn cancel_held(&mut self, reason: HeldCancellation) {
        let keys: Vec<_> = self.held_owners.keys().copied().collect();
        for key in keys {
            self.cancel_held_key(key, reason);
        }
    }
    pub(super) fn cancel_held_subtree(&mut self, serial: u64, reason: HeldCancellation) {
        let keys: Vec<_> = self
            .held_owners
            .iter()
            .filter(|(_, owner)| {
                self.held_registrations
                    .get(&owner.registration)
                    .is_some_and(|r| self.descendant(r.id.0.0.serial, serial))
            })
            .map(|(key, _)| *key)
            .collect();
        for key in keys {
            self.cancel_held_key(key, reason);
        }
    }
    pub(super) fn remove_held_registration(&mut self, serial: u64) {
        self.cancel_held_subtree(serial, HeldCancellation::Unavailable);
        self.held_registrations
            .retain(|_, registration| registration.id.0.0.serial != serial);
    }
    pub(super) fn cancel_blocked_held(&mut self) {
        let current = self.current_window();
        let owners: Vec<_> = self
            .held_owners
            .iter()
            .map(|(key, owner)| (*key, owner.clone()))
            .collect();
        for (key, owner) in owners {
            let eligible = self
                .held_registrations
                .get(&owner.registration)
                .map(|r| r.id.0.clone());
            let Some(view) = eligible else {
                self.cancel_held_key(key, HeldCancellation::Unavailable);
                continue;
            };
            if !view.0.mounted.get() || !view.0.visible.get() {
                self.cancel_held_key(key, HeldCancellation::Unavailable);
                continue;
            }
            if self
                .select_window(&native_windows::NativeWindowId(view.0.window()))
                .is_err()
                || !self.modal_allows(view.0.serial)
            {
                self.cancel_held_key(key, HeldCancellation::Modal);
            }
        }
        if self.contains_window(&current) {
            // Identity came from this runtime and was not exposed to callbacks.
            if let Err(error) = self.select_window(&current) {
                self.window.focus_errors.push_back(error);
            }
        }
    }
    pub(super) fn owned_key(
        &mut self,
        input: crate::CommandKeyInput,
        event: crate::KeyEvent,
    ) -> Result<bool, RuntimeError> {
        // A real non-synthetic fresh press reconciles cancellation suppression.
        if input.action == crate::KeyAction::Press && event.transition == KeyTransition::Pressed {
            self.suppressed_keys.remove(&input.physical);
        }
        if self.suppressed_keys.contains(&input.physical) {
            if input.action == crate::KeyAction::Release {
                self.suppressed_keys.remove(&input.physical);
            }
            return Ok(true);
        }
        let Some(mut owner) = self.held_owners.get(&input.physical).cloned() else {
            return Ok(false);
        };
        if input.action == crate::KeyAction::Release {
            self.held_owners.remove(&input.physical);
        }
        owner.context.repeat = event.transition == KeyTransition::Repeated;
        if let Some(registration) = self.held_registrations.get_mut(&owner.registration) {
            (registration.action)(input.physical, event.transition, &owner.context)?;
        }
        Ok(true)
    }
    pub(super) fn capture_held(
        &mut self,
        input: crate::CommandKeyInput,
        event: crate::KeyEvent,
    ) -> Result<bool, RuntimeError> {
        if event.transition != KeyTransition::Pressed {
            return Ok(false);
        }
        let context = self.command_context(CommandSource::KeyBinding);
        let candidate = self
            .held_registrations
            .iter()
            .find(|(_, r)| {
                r.keys.contains(&input.physical)
                    && r.modifiers == self.keyboard.modifiers()
                    && r.id.0.0.visible.get()
                    && r.id.0.0.mounted.get()
                    && Rc::ptr_eq(&r.id.0.0.window(), &self.window.shared)
                    && self.modal_allows(r.id.0.0.serial)
            })
            .map(|(serial, _)| *serial);
        if let Some(serial) = candidate {
            // Reserve before callback so failure has one terminal cancellation.
            self.held_owners.insert(
                input.physical,
                Owner {
                    registration: serial,
                    context: context.clone(),
                },
            );
            let accepted = (self
                .held_registrations
                .get_mut(&serial)
                .ok_or(RuntimeError::StaleCommand)?
                .action)(
                input.physical, KeyTransition::Pressed, &context
            )? == InputOutcome::Handled;
            if !accepted {
                self.held_owners.remove(&input.physical);
            }
            return Ok(accepted);
        }
        Ok(false)
    }
}
