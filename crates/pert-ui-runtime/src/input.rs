use std::collections::{BTreeMap, BTreeSet};

/// Physical key identity independent of generated text or keyboard layout.
///
/// Letter names describe conventional physical positions, not text input.
/// A platform adapter translates native codes into these owned identifiers.
/// Additional keys use an opaque platform-scoped native code; do not persist it.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum KeyCode {
    /// Physical A key.
    KeyA,
    /// Physical B key.
    KeyB,
    /// Physical C key.
    KeyC,
    /// Physical D key.
    KeyD,
    /// Physical E key.
    KeyE,
    /// Physical F key.
    KeyF,
    /// Physical G key.
    KeyG,
    /// Physical H key.
    KeyH,
    /// Physical I key.
    KeyI,
    /// Physical J key.
    KeyJ,
    /// Physical K key.
    KeyK,
    /// Physical L key.
    KeyL,
    /// Physical M key.
    KeyM,
    /// Physical N key.
    KeyN,
    /// Physical O key.
    KeyO,
    /// Physical P key.
    KeyP,
    /// Physical Q key.
    KeyQ,
    /// Physical R key.
    KeyR,
    /// Physical S key.
    KeyS,
    /// Physical T key.
    KeyT,
    /// Physical U key.
    KeyU,
    /// Physical V key.
    KeyV,
    /// Physical W key.
    KeyW,
    /// Physical X key.
    KeyX,
    /// Physical Y key.
    KeyY,
    /// Physical Z key.
    KeyZ,
    /// Physical digit 0 key.
    Digit0,
    /// Physical digit 1 key.
    Digit1,
    /// Physical digit 2 key.
    Digit2,
    /// Physical digit 3 key.
    Digit3,
    /// Physical digit 4 key.
    Digit4,
    /// Physical digit 5 key.
    Digit5,
    /// Physical digit 6 key.
    Digit6,
    /// Physical digit 7 key.
    Digit7,
    /// Physical digit 8 key.
    Digit8,
    /// Physical digit 9 key.
    Digit9,
    /// Function key F1.
    F1,
    /// Function key F2.
    F2,
    /// Function key F3.
    F3,
    /// Function key F4.
    F4,
    /// Function key F5.
    F5,
    /// Function key F6.
    F6,
    /// Function key F7.
    F7,
    /// Function key F8.
    F8,
    /// Function key F9.
    F9,
    /// Function key F10.
    F10,
    /// Function key F11.
    F11,
    /// Function key F12.
    F12,
    /// Escape physical key.
    Escape,
    /// Enter physical key.
    Enter,
    /// Tab physical key.
    Tab,
    /// Space physical key.
    Space,
    /// Backspace physical key.
    Backspace,
    /// Delete physical key.
    Delete,
    /// Insert physical key.
    Insert,
    /// Home physical key.
    Home,
    /// End physical key.
    End,
    /// PageUp physical key.
    PageUp,
    /// PageDown physical key.
    PageDown,
    /// ArrowLeft physical key.
    ArrowLeft,
    /// ArrowRight physical key.
    ArrowRight,
    /// ArrowUp physical key.
    ArrowUp,
    /// ArrowDown physical key.
    ArrowDown,
    /// ShiftLeft physical key.
    ShiftLeft,
    /// ShiftRight physical key.
    ShiftRight,
    /// ControlLeft physical key.
    ControlLeft,
    /// ControlRight physical key.
    ControlRight,
    /// AltLeft physical key.
    AltLeft,
    /// AltRight physical key.
    AltRight,
    /// SuperLeft physical key.
    SuperLeft,
    /// SuperRight physical key.
    SuperRight,
    /// CapsLock physical key.
    CapsLock,
    /// Minus physical key.
    Minus,
    /// Equal physical key.
    Equal,
    /// BracketLeft physical key.
    BracketLeft,
    /// BracketRight physical key.
    BracketRight,
    /// Backslash physical key.
    Backslash,
    /// The additional key between the left Shift and Z on ISO keyboards
    /// (`IntlBackslash`), absent from ANSI layouts.
    IntlBackslash,
    /// Semicolon physical key.
    Semicolon,
    /// Quote physical key.
    Quote,
    /// Backquote physical key.
    Backquote,
    /// Comma physical key.
    Comma,
    /// Period physical key.
    Period,
    /// Slash physical key.
    Slash,
    /// The ContextMenu (Menu/Application) key of PC keyboards. Built-in
    /// controls treat it like Shift+F10; see [`KeyEvent::requests_context_menu`].
    ContextMenu,
    /// A native key not yet named here. The code is meaningful only to its adapter.
    Native(u32),
}

impl KeyCode {
    /// The left key of the platform's primary modifier: [`Self::SuperLeft`]
    /// (Command) on macOS and [`Self::ControlLeft`] elsewhere, the key that
    /// [`Modifiers::primary`] tests. For pressing the primary modifier from
    /// automation and tests; a shortcut is bound with
    /// [`KeyChord::primary`](crate::KeyChord::primary).
    #[must_use]
    pub const fn primary() -> Self {
        if cfg!(target_os = "macos") {
            Self::SuperLeft
        } else {
            Self::ControlLeft
        }
    }
}

/// Physical transition reported by a platform adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyAction {
    /// Down notification; an already-held key becomes a repeat.
    Press,
    /// Explicit OS repeat; never a fresh press. A repeat for a key that is not
    /// held, such as one pressed before focus was regained, is an orphan: it
    /// never establishes a press, and the runtime refuses it without delivery.
    Repeat,
    /// Up notification; an unknown up is harmless.
    Release,
}

/// Classified transition after consulting the complete pressed-key set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum KeyTransition {
    /// Newly pressed; applications can start one action here.
    Pressed,
    /// Already held; automatic OS repeat does not add another held key.
    Repeated,
    /// A held key was released.
    Released,
    /// An up arrived for a key that was not held, for example after focus loss.
    UnmatchedRelease,
}

/// Physical key event delivered to the focused component after state is updated.
///
/// This is not a text insertion. OS repetition is identified through the pressed
/// set; a component decides whether repeats are meaningful for its own action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct KeyEvent {
    /// Layout-independent physical key position.
    pub code: KeyCode,
    /// Transition relative to the previously pressed-key set.
    pub transition: KeyTransition,
}

impl KeyEvent {
    /// Whether this key is the keyboard request for a context menu: the
    /// [`KeyCode::ContextMenu`] key, or F10 while Shift is held.
    ///
    /// This is the rule the built-in context-menu owners use (`ContextMenu`,
    /// the text editors and the `Select` anchors), so a custom component that
    /// tests it opens its menu on the same keys. Only the key and Shift are
    /// consulted; the caller decides which [`KeyTransition`]s act, normally a
    /// fresh [`KeyTransition::Pressed`].
    ///
    /// ```
    /// use pert_ui_runtime::{KeyCode, KeyEvent, KeyTransition, Modifiers};
    ///
    /// let key = |code| KeyEvent { code, transition: KeyTransition::Pressed };
    /// let shift = Modifiers { shift: true, ..Modifiers::default() };
    /// assert!(key(KeyCode::ContextMenu).requests_context_menu(Modifiers::default()));
    /// assert!(key(KeyCode::F10).requests_context_menu(shift));
    /// assert!(!key(KeyCode::F10).requests_context_menu(Modifiers::default()));
    /// ```
    #[must_use]
    pub const fn requests_context_menu(self, modifiers: Modifiers) -> bool {
        match self.code {
            KeyCode::ContextMenu => true,
            KeyCode::F10 => modifiers.shift,
            _ => false,
        }
    }
}

/// Combined left/right modifier state. See [`KeyboardState::modifiers`] for
/// its source.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct Modifiers {
    /// Either Shift key is held.
    pub shift: bool,
    /// Either Control key is held.
    pub control: bool,
    /// Either Alt key is held.
    pub alt: bool,
    /// Either platform Super/Command/Windows key is held.
    pub super_key: bool,
}

impl Modifiers {
    /// Exactly the platform's primary modifier: Command ([`Self::super_key`])
    /// on macOS and [`Self::control`] elsewhere, with no other modifier.
    ///
    /// [`KeyChord::primary`](crate::KeyChord::primary) binds these modifiers,
    /// [`Self::primary`] tests for them and [`KeyCode::primary`] names the left
    /// key that produces them, so one rule decides the primary modifier.
    ///
    /// ```
    /// use pert_ui_runtime::Modifiers;
    ///
    /// assert!(Modifiers::PRIMARY.primary());
    /// assert!(!Modifiers::PRIMARY.shift && !Modifiers::PRIMARY.alt);
    /// ```
    pub const PRIMARY: Self = Self {
        shift: false,
        control: !cfg!(target_os = "macos"),
        alt: false,
        super_key: cfg!(target_os = "macos"),
    };

    /// Whether the platform's primary modifier is held: Command
    /// ([`Self::super_key`]) on macOS and [`Self::control`] elsewhere.
    ///
    /// This is the modifier [`KeyChord::primary`](crate::KeyChord::primary)
    /// binds and that built-in controls such as [`ZoomView`](crate::ZoomView)
    /// test for their modified gestures. It says nothing about the other
    /// modifiers; check them separately when a gesture needs an exact
    /// combination.
    ///
    /// ```
    /// use pert_ui_runtime::Modifiers;
    ///
    /// let control = Modifiers { control: true, ..Modifiers::default() };
    /// assert_eq!(control.primary(), !cfg!(target_os = "macos"));
    /// assert!(!Modifiers::default().primary());
    /// ```
    #[must_use]
    pub const fn primary(self) -> bool {
        if cfg!(target_os = "macos") {
            self.super_key
        } else {
            self.control
        }
    }
}

/// Physical pressed-key state and captured native control meanings for a runtime.
///
/// Supports simultaneous keys with no framework-imposed rollover limit. Actual
/// delivered combinations depend on keyboard hardware and the operating system.
/// This is UI presentation/input state, not a cross-thread keyboard transport.
/// Focus loss clears it; the focused component receives a reset notification.
/// Equality includes the latest event's captured meaning as well as held keys
/// and modifiers; it does not compare only the physical pressed set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KeyboardState {
    native_modifiers: Option<Modifiers>,
    pressed: BTreeSet<KeyCode>,
    control_keys: BTreeMap<KeyCode, KeyCode>,
    /// The modifiers held when each held key was pressed.
    press_modifiers: BTreeMap<KeyCode, Modifiers>,
    last_control_key: Option<(KeyEvent, KeyCode)>,
}

impl KeyboardState {
    /// Returns the named control action associated with a physical key event.
    ///
    /// Native adapters retain distinct opaque identities for numpad keys. Their
    /// named meaning (such as Enter or ArrowLeft with NumLock off) is captured
    /// at the initial press and remains fixed through repeats and the matching
    /// release, even if the native layout changes during the hold. Unnamed keys
    /// and headless presses without a captured native meaning retain their physical code.
    ///
    /// Use this for deciding a control's action, but use [`KeyEvent::code`] for
    /// arming and matching a release and [`Self::is_pressed`] for held ownership.
    /// Interpret the event during its callback: this is current input state,
    /// retaining the latest event's meaning and held mappings, not event history.
    /// Copied historical events cannot recover earlier meanings after another
    /// press/release cycle. Unmapped events fall back to their physical code.
    ///
    /// ```
    /// use pert_ui_runtime::{CommandKey, CommandKeyInput, KeyAction, KeyCode, Runtime};
    /// let mut ui = Runtime::new();
    /// // A synthetic opaque key for this example; real adapters own native codes.
    /// let physical = KeyCode::Native(42);
    /// let dispatch = ui.command_key_input(CommandKeyInput {
    ///     physical,
    ///     action: KeyAction::Press,
    ///     base: Some(CommandKey::Named(KeyCode::ArrowLeft)),
    ///     logical: Some(CommandKey::Named(KeyCode::ArrowLeft)),
    ///     has_text: false,
    ///     composing: false,
    /// })?;
    /// assert_eq!(dispatch.event.code, physical);
    /// assert!(ui.keyboard().is_pressed(physical));
    /// assert!(!ui.keyboard().is_pressed(KeyCode::ArrowLeft));
    /// assert_eq!(ui.keyboard().control_key(dispatch.event), KeyCode::ArrowLeft);
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    #[must_use]
    pub fn control_key(&self, event: KeyEvent) -> KeyCode {
        self.last_control_key
            .filter(|(last, _)| *last == event)
            .map(|(_, key)| key)
            .or_else(|| self.control_keys.get(&event.code).copied())
            .unwrap_or(event.code)
    }

    /// Returns whether this physical key is currently held.
    #[must_use]
    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    /// Iterates all held keys in deterministic key-code order, not press order.
    pub fn pressed(&self) -> impl Iterator<Item = KeyCode> + '_ {
        self.pressed.iter().copied()
    }

    /// Returns left/right-combined modifiers.
    ///
    /// Once the host has reported the operating system's modifier snapshot
    /// through `Runtime::native_modifiers`, that snapshot is returned: it also
    /// covers modifiers held before the window gained focus, which the pressed
    /// set never sees. Physical key events do not change it until the host
    /// reports the next snapshot. Before any report, and again after focus loss
    /// clears the state, the value is derived from the held modifier keys of
    /// [`Self::pressed`].
    #[must_use]
    pub fn modifiers(&self) -> Modifiers {
        self.current_modifiers()
    }

    /// The modifiers that decide whether `event` is a chord a component
    /// admits: for a repeat of a held key, those held when it was pressed, so a
    /// press a component admitted keeps repeating when modifiers change and a
    /// press it left to command bindings stays theirs; otherwise the current
    /// [`Self::modifiers`].
    ///
    /// Use it for admission only. A held step that changes with Shift, for
    /// example, still reads the current [`Self::modifiers`].
    ///
    /// ```
    /// use pert_ui_runtime::{KeyAction, KeyCode, Modifiers, Runtime};
    /// let mut ui = Runtime::new();
    /// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
    /// ui.native_modifiers(Modifiers::PRIMARY)?;
    /// let repeat = ui.key_input(KeyCode::ArrowDown, KeyAction::Repeat)?;
    /// assert_eq!(ui.keyboard().chord_modifiers(repeat), Modifiers::default());
    /// assert_eq!(ui.keyboard().modifiers(), Modifiers::PRIMARY);
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    #[must_use]
    pub fn chord_modifiers(&self, event: KeyEvent) -> Modifiers {
        match event.transition {
            KeyTransition::Repeated => self
                .press_modifiers
                .get(&event.code)
                .copied()
                .unwrap_or_else(|| self.modifiers()),
            _ => self.modifiers(),
        }
    }

    fn current_modifiers(&self) -> Modifiers {
        if let Some(modifiers) = self.native_modifiers {
            return modifiers;
        }
        let held = |left, right| self.is_pressed(left) || self.is_pressed(right);
        Modifiers {
            shift: held(KeyCode::ShiftLeft, KeyCode::ShiftRight),
            control: held(KeyCode::ControlLeft, KeyCode::ControlRight),
            alt: held(KeyCode::AltLeft, KeyCode::AltRight),
            super_key: held(KeyCode::SuperLeft, KeyCode::SuperRight),
        }
    }

    pub(crate) fn apply(&mut self, code: KeyCode, action: KeyAction) -> KeyEvent {
        self.apply_control_key(code, action, code)
    }

    pub(crate) fn apply_control_key(
        &mut self,
        code: KeyCode,
        action: KeyAction,
        named: KeyCode,
    ) -> KeyEvent {
        let meaning = self.control_keys.get(&code).copied().unwrap_or_else(|| {
            if action == KeyAction::Press {
                named
            } else {
                code
            }
        });
        let transition = match action {
            // An orphan repeat is reported but never establishes a press.
            KeyAction::Repeat => KeyTransition::Repeated,
            KeyAction::Press if self.pressed.insert(code) => {
                if matches!(code, KeyCode::Native(_)) || named != code {
                    self.control_keys.insert(code, named);
                }
                self.press_modifiers.insert(code, self.modifiers());
                KeyTransition::Pressed
            }
            KeyAction::Press => KeyTransition::Repeated,
            KeyAction::Release if self.pressed.remove(&code) => {
                self.control_keys.remove(&code);
                self.press_modifiers.remove(&code);
                KeyTransition::Released
            }
            KeyAction::Release => KeyTransition::UnmatchedRelease,
        };
        let event = KeyEvent { code, transition };
        self.last_control_key = Some((event, meaning));
        event
    }

    pub(crate) fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.native_modifiers = Some(modifiers);
    }
    pub(crate) fn clear(&mut self) {
        self.native_modifiers = None;
        self.pressed.clear();
        self.control_keys.clear();
        self.press_modifiers.clear();
        self.last_control_key = None;
    }
}
