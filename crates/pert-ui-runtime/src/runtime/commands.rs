//! Single-pass command routing through presentation scopes.
use super::*;
use crate::{InputOutcome, KeyAction, KeyCode, KeyEvent, KeyTransition, Modifiers};

/// Layout-aware command identity, independent of committed text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandKey {
    /// Modifier-free layout character. ASCII letters compare case-insensitively;
    /// punctuation and non-ASCII characters compare exactly.
    Character(char),
    /// Named key such as Enter, Delete or an arrow; no letter-position inference.
    Named(KeyCode),
}
impl CommandKey {
    fn normalized(self) -> Self {
        match self {
            Self::Character(c) => Self::Character(c.to_ascii_lowercase()),
            key => key,
        }
    }
}
/// Key and exact modifier requirements. Primary means Command on macOS and
/// Control elsewhere. Physical fallback is explicit and only used when no logical
/// identity is available; it never runs alongside a logical match.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[must_use]
pub struct KeyChord {
    key: CommandKey,
    modifiers: Modifiers,
    fallback: Option<KeyCode>,
}
impl KeyChord {
    /// Creates an exact chord using modifier-free layout identity.
    pub fn new(key: CommandKey, modifiers: Modifiers) -> Self {
        Self {
            key: key.normalized(),
            modifiers,
            fallback: None,
        }
    }
    /// One named key without modifiers, such as Space for a transport or
    /// Delete for a selection: `KeyChord::new(CommandKey::Named(key),
    /// Modifiers::default())`.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{CommandKey, KeyChord, KeyCode, Modifiers};
    /// assert_eq!(
    ///     KeyChord::named(KeyCode::Space),
    ///     KeyChord::named(KeyCode::Space),
    /// );
    /// ```
    pub fn named(key: KeyCode) -> Self {
        Self::new(CommandKey::Named(key), Modifiers::default())
    }
    /// A conventional primary-modifier letter chord.
    pub fn primary(letter: char) -> Self {
        Self::new(CommandKey::Character(letter), Modifiers::PRIMARY)
    }
    /// Adds Shift to the exact modifiers; useful for redo and command aliases.
    pub fn shifted(mut self) -> Self {
        self.modifiers.shift = true;
        self
    }
    /// The platform's conventional chord for going back in a navigation
    /// history: Cmd+[ on macOS and Alt+Left elsewhere, as browsers use. Register
    /// it on the application's own back command; see
    /// [`HistoryNavigator`](crate::HistoryNavigator).
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{CommandKey, KeyChord, KeyCode};
    /// let back = KeyChord::navigate_back();
    /// if cfg!(target_os = "macos") {
    ///     assert_eq!(back.key(), CommandKey::Character('['));
    /// } else {
    ///     assert_eq!(back.key(), CommandKey::Named(KeyCode::ArrowLeft));
    ///     assert!(back.modifiers().alt);
    /// }
    /// ```
    pub fn navigate_back() -> Self {
        Self::navigation(true)
    }
    /// The platform's conventional chord for going forward in a navigation
    /// history: Cmd+] on macOS and Alt+Right elsewhere; see
    /// [`Self::navigate_back`].
    pub fn navigate_forward() -> Self {
        Self::navigation(false)
    }
    fn navigation(back: bool) -> Self {
        if cfg!(target_os = "macos") {
            Self::new(
                CommandKey::Character(if back { '[' } else { ']' }),
                Modifiers::PRIMARY,
            )
        } else {
            let alt = Modifiers {
                alt: true,
                ..Modifiers::default()
            };
            let key = if back {
                KeyCode::ArrowLeft
            } else {
                KeyCode::ArrowRight
            };
            Self::new(CommandKey::Named(key), alt)
        }
    }
    /// Enables a physical fallback only for native inputs lacking logical identity.
    pub fn physical_fallback(mut self, key: KeyCode) -> Self {
        self.fallback = Some(key);
        self
    }
    /// The layout identity this chord matches; ASCII letters are lowercase.
    #[must_use]
    pub fn key(self) -> CommandKey {
        self.key
    }
    /// The exact modifiers this chord requires.
    #[must_use]
    pub fn modifiers(self) -> Modifiers {
        self.modifiers
    }
    fn write_label(self, f: &mut std::fmt::Formatter<'_>, apple: bool) -> std::fmt::Result {
        let m = self.modifiers;
        // Each platform lists modifiers in its own conventional order.
        let names: [(bool, &str); 4] = if apple {
            [
                (m.control, "Ctrl"),
                (m.alt, "Option"),
                (m.shift, "Shift"),
                (m.super_key, "Cmd"),
            ]
        } else {
            [
                (m.control, "Ctrl"),
                (m.alt, "Alt"),
                (m.shift, "Shift"),
                (m.super_key, "Super"),
            ]
        };
        for (_, name) in names.iter().filter(|(held, _)| *held) {
            write!(f, "{name}+")?;
        }
        match self.key {
            CommandKey::Character(' ') => f.write_str("Space"),
            CommandKey::Character(c) => write!(f, "{}", c.to_uppercase()),
            CommandKey::Named(code) => f.write_str(&key_name(code)),
        }
    }
}
/// Presents the chord in the running platform's convention: modifier names
/// joined with `+`, then the key, such as `Ctrl+Shift+S`. macOS names Option and
/// Cmd in its own order, as `Ctrl+Option+Shift+Cmd+S`. Letters are uppercase.
/// The names are English words rather than symbols such as ⌘, because the
/// bundled fonts do not carry those glyphs. The physical fallback is not shown.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{CommandKey, KeyChord, KeyCode, Modifiers};
/// let chord = KeyChord::named(KeyCode::F5);
/// assert_eq!(chord.to_string(), "F5");
/// let shifted = KeyChord::new(CommandKey::Character('z'), Modifiers::default()).shifted();
/// assert_eq!(shifted.to_string(), "Shift+Z");
/// ```
impl std::fmt::Display for KeyChord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_label(f, cfg!(target_os = "macos"))
    }
}
/// A readable English name for a physical key in a presented chord.
fn key_name(code: KeyCode) -> String {
    let name = match code {
        KeyCode::KeyA => "A",
        KeyCode::KeyB => "B",
        KeyCode::KeyC => "C",
        KeyCode::KeyD => "D",
        KeyCode::KeyE => "E",
        KeyCode::KeyF => "F",
        KeyCode::KeyG => "G",
        KeyCode::KeyH => "H",
        KeyCode::KeyI => "I",
        KeyCode::KeyJ => "J",
        KeyCode::KeyK => "K",
        KeyCode::KeyL => "L",
        KeyCode::KeyM => "M",
        KeyCode::KeyN => "N",
        KeyCode::KeyO => "O",
        KeyCode::KeyP => "P",
        KeyCode::KeyQ => "Q",
        KeyCode::KeyR => "R",
        KeyCode::KeyS => "S",
        KeyCode::KeyT => "T",
        KeyCode::KeyU => "U",
        KeyCode::KeyV => "V",
        KeyCode::KeyW => "W",
        KeyCode::KeyX => "X",
        KeyCode::KeyY => "Y",
        KeyCode::KeyZ => "Z",
        KeyCode::Digit0 => "0",
        KeyCode::Digit1 => "1",
        KeyCode::Digit2 => "2",
        KeyCode::Digit3 => "3",
        KeyCode::Digit4 => "4",
        KeyCode::Digit5 => "5",
        KeyCode::Digit6 => "6",
        KeyCode::Digit7 => "7",
        KeyCode::Digit8 => "8",
        KeyCode::Digit9 => "9",
        KeyCode::F1 => "F1",
        KeyCode::F2 => "F2",
        KeyCode::F3 => "F3",
        KeyCode::F4 => "F4",
        KeyCode::F5 => "F5",
        KeyCode::F6 => "F6",
        KeyCode::F7 => "F7",
        KeyCode::F8 => "F8",
        KeyCode::F9 => "F9",
        KeyCode::F10 => "F10",
        KeyCode::F11 => "F11",
        KeyCode::F12 => "F12",
        KeyCode::Escape => "Esc",
        KeyCode::Enter => "Enter",
        KeyCode::Tab => "Tab",
        KeyCode::Space => "Space",
        KeyCode::Backspace => "Backspace",
        KeyCode::Delete => "Delete",
        KeyCode::Insert => "Insert",
        KeyCode::Home => "Home",
        KeyCode::End => "End",
        KeyCode::PageUp => "Page Up",
        KeyCode::PageDown => "Page Down",
        KeyCode::ArrowLeft => "Left",
        KeyCode::ArrowRight => "Right",
        KeyCode::ArrowUp => "Up",
        KeyCode::ArrowDown => "Down",
        KeyCode::ShiftLeft | KeyCode::ShiftRight => "Shift",
        KeyCode::ControlLeft | KeyCode::ControlRight => "Ctrl",
        KeyCode::AltLeft | KeyCode::AltRight => "Alt",
        KeyCode::SuperLeft | KeyCode::SuperRight => "Super",
        KeyCode::CapsLock => "Caps Lock",
        KeyCode::Minus => "-",
        KeyCode::Equal => "=",
        KeyCode::BracketLeft => "[",
        KeyCode::BracketRight => "]",
        KeyCode::Backslash => "\\",
        // Its printed legend varies with the layout (`<`, `\` or `|`).
        KeyCode::IntlBackslash => "Intl Backslash",
        KeyCode::Semicolon => ";",
        KeyCode::Quote => "'",
        KeyCode::Backquote => "`",
        KeyCode::Comma => ",",
        KeyCode::Period => ".",
        KeyCode::Slash => "/",
        KeyCode::ContextMenu => "Menu",
        KeyCode::Native(code) => return format!("Key {code:#x}"),
    };
    name.to_owned()
}
impl KeyChord {
    fn matches(self, input: &CommandKeyInput, modifiers: Modifiers) -> bool {
        self.modifiers == modifiers
            && match input.base {
                Some(key) => self.key == key.normalized(),
                None => self.fallback == Some(input.physical),
            }
    }
}
/// Owned native keyboard translation. Text content travels separately to the
/// validated text session; this input only states whether associated text exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandKeyInput {
    /// Physical position for held state and gesture ownership. Ordinary controls
    /// use [`KeyboardState::control_key`](crate::KeyboardState::control_key)
    /// for its captured named action.
    pub physical: KeyCode,
    /// OS press/repeat/release classification; synthetic transitions are excluded.
    pub action: KeyAction,
    /// Modifier-free layout identity, including Shift/Caps Lock normalization.
    pub base: Option<CommandKey>,
    /// Logical identity with native modifiers applied. For opaque native holds,
    /// runtime editor delivery preserves a press-time named control meaning
    /// through repeat/release; a key pressed as text cannot adopt a named action
    /// midway through its hold. A captured named control also suppresses new
    /// printable text when the logical meaning changes during its hold.
    /// Committed text remains a separate native payload.
    pub logical: Option<CommandKey>,
    /// Associated printable native text: the key's text holds a character that
    /// is not a control character. Routing and modifiers may suppress its
    /// delivery. A held native named control also suppresses text when its
    /// current logical meaning changes; unchanged named Space retains text.
    /// See [`KeyDispatch`].
    pub has_text: bool,
    /// AltGr or dead-key/IME composition requiring text precedence over commands.
    /// The native `App` sets it while its IME composes, for a dead key, and
    /// for printable text typed with Control and Alt held together.
    pub composing: bool,
}
impl CommandKeyInput {
    /// Constructs a headless layout-aware event with no associated text/composition.
    pub fn new(physical: KeyCode, action: KeyAction, base: CommandKey) -> Self {
        Self {
            physical,
            action,
            base: Some(base),
            logical: Some(base),
            has_text: false,
            composing: false,
        }
    }
    /// Editing-key identity for shared text controls. Layout-aware ASCII editing
    /// letters map to their conventional editing action codes, never to inserted text.
    pub fn editing_event(self, transition: KeyTransition) -> KeyEvent {
        let code = match self.base.map(CommandKey::normalized) {
            Some(CommandKey::Character('a')) => KeyCode::KeyA,
            Some(CommandKey::Character('c')) => KeyCode::KeyC,
            Some(CommandKey::Character('v')) => KeyCode::KeyV,
            Some(CommandKey::Character('x')) => KeyCode::KeyX,
            Some(CommandKey::Character('y')) => KeyCode::KeyY,
            Some(CommandKey::Character('z')) => KeyCode::KeyZ,
            Some(CommandKey::Character(_)) => KeyCode::Native(0),
            Some(CommandKey::Named(key)) => key,
            None => self.physical,
        };
        KeyEvent { code, transition }
    }
}
/// Text-editing phase result for built-in and external editable components.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextKeyOutcome {
    /// This component has no editing phase; ordinary keyboard dispatch may call it.
    NotAnEditor,
    /// Editing did not claim the chord. Ordinary bubbling starts at its parent,
    /// so an editing action cannot run twice in one routing pass.
    Ignored,
    /// An editing action or reserved editing chord consumed the key and its text.
    Handled,
    /// The editor claims associated native text; commands must not intercept it.
    InsertText,
}
/// Registration scope, resolved once for the invoking window and content target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandScope {
    /// Explicit application capture after text editing, before ordinary controls.
    Capture,
    /// Application fallback, available from every eligible window.
    Application,
    /// Bindings confined to one native window incarnation.
    Window(crate::NativeWindowId),
    /// Bindings for this retained presentation subtree. They follow detachment.
    View(WidgetId),
}
/// Source of a shared command invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandSource {
    /// Physical key routing with native repeat classification.
    KeyBinding,
    /// Menu action using the target captured when the menu opened.
    Menu,
    /// Button action with an explicit target or the active content target.
    Button,
    /// Explicit application invocation.
    Programmatic,
}
/// Owned command context, safe to retain on the UI thread for menu invocation.
/// Every invocation revalidates window, target incarnation, visibility and modality.
#[derive(Clone, Debug)]
pub struct CommandContext {
    /// Native window in which the action was invoked; default result placement.
    pub window: crate::NativeWindowId,
    /// Captured active content target, absent when only window scope is available.
    pub target: Option<WidgetId>,
    /// Focused component at capture time, if any.
    pub focused: Option<WidgetId>,
    /// Invocation source, shared by keyboard/menu/button/programmatic callers.
    pub source: CommandSource,
    /// Whether this invocation came from an OS repeat.
    pub repeat: bool,
}
/// Stable command incarnation. Unregistering never retargets an old identity.
#[derive(Clone, Debug)]
pub struct CommandId {
    serial: u64,
    identity: Rc<()>,
}
impl PartialEq for CommandId {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial && Rc::ptr_eq(&self.identity, &other.identity)
    }
}
impl Eq for CommandId {}
impl std::hash::Hash for CommandId {
    /// Hashes the serial and the registry's identity, the parts equality
    /// compares.
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.serial.hash(state);
        Rc::as_ptr(&self.identity).hash(state);
    }
}
/// Common command admission result. Disabled bindings reserve their chord.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    /// The action completed successfully once.
    Invoked,
    /// A matching disabled action reserved the input without running.
    Disabled,
    /// A modal boundary blocked the invocation.
    Blocked,
    /// OS repeats are disabled for this command (the default).
    RepeatSuppressed,
}
/// Command name and contextual enabled state for menus, buttons and inspectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandInfo {
    /// Human-readable nonblank name supplied by the host.
    pub name: String,
    /// Enabled for this exact validated context.
    pub enabled: bool,
}
/// One registered command as a captured context sees it: an owned row for a
/// command palette, a shortcut list or an inspector, from
/// [`Runtime::commands_for`].
///
/// The row is a snapshot. The context can change before it is used, so
/// invoking [`Self::id`] revalidates scope, modality and enabled state and may
/// still report `Disabled`, `Blocked` or a stale target.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CommandEntry {
    /// The command's identity, for [`Runtime::invoke_command`] or
    /// [`EventContext::invoke_command`].
    pub id: CommandId,
    /// Human-readable nonblank name supplied by the host.
    pub name: String,
    /// The scope the command was registered in.
    pub scope: CommandScope,
    /// The chord aliases at the time of the snapshot, first alias first; empty
    /// for a command without a shortcut.
    pub shortcuts: Vec<KeyChord>,
    /// Enabled for this exact validated context, as [`Runtime::command_info`]
    /// reports it: `Ok(false)` when a modal blocks it or its predicate says
    /// so, and the predicate's error when it failed, which then affects only
    /// this row.
    pub enabled: Result<bool, RuntimeError>,
}
/// Result of one physical routing pass, including whether associated native text
/// may be delivered. Native hosts must respect `deliver_text` before insertion.
///
/// The admission rule the native `App` follows, and a custom host should
/// follow, is: insert the key's text with
/// [`Runtime::text_input`] only when `deliver_text` is true, the transition
/// is a press or repeat, and the host's IME is not composing (the IME commits
/// composed text itself); remove control characters from the text first and
/// insert nothing when none remain. `deliver_text` already accounts for
/// routing, held named meaning and modifiers. A native key held with a named
/// control meaning cannot insert text when its current logical meaning changes;
/// an unchanged named Space retains its text. The flag is false while Super is
/// held or Control is held
/// without Alt, so a chord such as Control+Q or Super+Q never types its
/// letter, while Control+Alt (AltGr on Windows) text is delivered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyDispatch {
    /// Updated physical transition.
    pub event: KeyEvent,
    /// Whether any editing, component or command handler claimed the input.
    pub handled: bool,
    /// Whether associated native text can reach the current validated editor
    /// under the held modifiers and captured named meaning. See the type's
    /// admission rule.
    pub deliver_text: bool,
}
/// Whether held modifiers let a key's associated text insert: never with
/// Super, and with Control only together with Alt (AltGr on Windows).
fn text_modifiers_admit(modifiers: Modifiers) -> bool {
    !modifiers.super_key && (!modifiers.control || modifiers.alt)
}
type CommandAction =
    Rc<RefCell<dyn FnMut(&mut Runtime, &CommandContext) -> Result<(), RuntimeError>>>;
type Enabled = Rc<dyn Fn(&Runtime, &CommandContext) -> Result<bool, RuntimeError>>;
#[derive(Clone)]
pub(super) struct CommandPress {
    id: CommandId,
    context: CommandContext,
    admitted: bool,
}
pub(super) struct RegisteredCommand {
    id: CommandId,
    name: String,
    scope: CommandScope,
    chords: Vec<KeyChord>,
    /// Published copy of `chords` for presentation, updated by rebinding.
    shortcuts: Signal<Vec<KeyChord>>,
    repeat: bool,
    in_modals: bool,
    enabled: Enabled,
    /// The shared binding behind `enabled`, when it is a simple one.
    enabled_binding: Option<ReadSignal<bool>>,
    action: CommandAction,
}
/// Read-only admission for one exact command context. A listing computes this
/// once instead of walking the mounted tree for every registered command.
#[derive(Clone, Copy)]
struct CommandModalPolicy {
    allows_target: bool,
    local_modal: bool,
}
impl CommandModalPolicy {
    fn admits(self, command: &RegisteredCommand) -> bool {
        self.allows_target
            && (!matches!(
                command.scope,
                CommandScope::Application | CommandScope::Capture
            ) || command.in_modals
                || !self.local_modal)
    }
}

impl Runtime {
    /// Registers a named host action and its chord aliases in one scope.
    ///
    /// Lifetime is explicit: unregister it or remove its view/window scope to
    /// release callbacks and bindings. Dropping the returned [`CommandId`] keeps
    /// the command registered. Duplicate chords in the same scope, including
    /// alias/fallback collisions, return `ShortcutConflict` in debug and release.
    /// An empty chord list supports menu/button-only commands. Repeats default off.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Runtime, CommandScope, CommandSource, KeyChord, CommandOutcome};
    /// let mut ui = Runtime::new();
    /// let playing = ui.signal(false);
    /// let state = playing.clone();
    /// let play = ui.register_command("Play", CommandScope::Application,
    ///     vec![KeyChord::primary('p')], move |_, _| {
    ///         state.set(!state.get()?)?;
    ///         Ok(())
    ///     })?;
    /// let context = ui.command_context(CommandSource::Programmatic);
    /// assert_eq!(ui.invoke_command(&play, context)?, CommandOutcome::Invoked);
    /// assert!(playing.get()?);
    /// ui.unregister_command(&play)?;
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `EmptyCommandName` for a blank name; `ForeignRuntime` or `Unmounted` for a view
    /// scope whose view is foreign or unmounted, and `Unmounted` for a window scope that is
    /// no longer a presentation root; `ShortcutConflict` when two chords overlap each other
    /// or a chord of another command in the same scope; and `Exhausted` when command
    /// identities run out.
    pub fn register_command(
        &mut self,
        name: impl Into<String>,
        scope: CommandScope,
        chords: Vec<KeyChord>,
        action: impl FnMut(&mut Runtime, &CommandContext) -> Result<(), RuntimeError> + 'static,
    ) -> Result<CommandId, RuntimeError> {
        self.shared.writable()?;
        let name = name.into();
        if name.trim().is_empty() {
            return Err(RuntimeError::EmptyCommandName);
        }
        self.validate_command_scope(&scope)?;
        self.validate_chords(&scope, &chords, None)?;
        self.command_serial = self
            .command_serial
            .checked_add(1)
            .ok_or(RuntimeError::Exhausted)?;
        let id = CommandId {
            serial: self.command_serial,
            identity: Rc::new(()),
        };
        self.commands.insert(
            id.serial,
            RegisteredCommand {
                id: id.clone(),
                name,
                scope,
                shortcuts: self.signal(chords.clone()),
                chords,
                repeat: false,
                in_modals: false,
                enabled: Rc::new(|_, _| Ok(true)),
                enabled_binding: None,
                action: Rc::new(RefCell::new(action)),
            },
        );
        Ok(id)
    }
    fn validate_command_scope(&self, scope: &CommandScope) -> Result<(), RuntimeError> {
        match scope {
            CommandScope::View(view) => self.check(view),
            CommandScope::Window(window) if !self.contains_window(window) => {
                Err(RuntimeError::Unmounted)
            }
            _ => Ok(()),
        }
    }
    fn validate_chords(
        &self,
        scope: &CommandScope,
        chords: &[KeyChord],
        excluding: Option<u64>,
    ) -> Result<(), RuntimeError> {
        let overlaps = |a: &KeyChord, b: &KeyChord| {
            a.modifiers == b.modifiers
                && (a.key == b.key || a.fallback.is_some() && a.fallback == b.fallback)
        };
        for (i, chord) in chords.iter().enumerate() {
            if chords[..i].iter().any(|other| overlaps(chord, other))
                || self
                    .commands
                    .values()
                    .filter(|command| {
                        Some(command.id.serial) != excluding && command.scope == *scope
                    })
                    .any(|command| command.chords.iter().any(|other| overlaps(chord, other)))
            {
                return Err(RuntimeError::ShortcutConflict(*chord));
            }
        }
        Ok(())
    }
    fn command(&self, id: &CommandId) -> Result<&RegisteredCommand, RuntimeError> {
        self.commands
            .get(&id.serial)
            .filter(|entry| entry.id == *id)
            .ok_or(RuntimeError::StaleCommand)
    }
    /// Removes one exact registration, releasing its action and chord aliases.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and `StaleCommand` for a command that is unregistered or belongs to another runtime.
    pub fn unregister_command(&mut self, id: &CommandId) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.command(id)?;
        self.commands.remove(&id.serial);
        Ok(())
    }
    /// Atomically replaces aliases after conflict validation, retaining the command.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `StaleCommand` for a command that is unregistered or belongs to another runtime; and
    /// `ShortcutConflict` when two chords overlap each other or a chord of another command
    /// in the same scope. The old aliases stay on error.
    pub fn rebind_command(
        &mut self,
        id: &CommandId,
        chords: Vec<KeyChord>,
    ) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.validate_chords(&self.command(id)?.scope, &chords, Some(id.serial))?;
        let command = self
            .commands
            .get_mut(&id.serial)
            .ok_or(RuntimeError::StaleCommand)?;
        command.shortcuts.set(chords.clone())?;
        command.chords = chords;
        Ok(())
    }
    /// The host-supplied name of a registered command, for presenting it in a
    /// menu, a button or a shortcut list.
    ///
    /// # Errors
    /// `StaleCommand` if the command was unregistered.
    pub fn command_name(&self, id: &CommandId) -> Result<&str, RuntimeError> {
        Ok(&self.command(id)?.name)
    }
    /// The command's current chord aliases as a binding, first alias first.
    ///
    /// The binding follows [`Self::rebind_command`], so a menu row or a shortcut
    /// list that reads it shows a changed shortcut without being rebuilt. After
    /// the command is unregistered the binding keeps its last value.
    ///
    /// # Errors
    /// `StaleCommand` if the command was unregistered.
    pub fn command_shortcuts(
        &self,
        id: &CommandId,
    ) -> Result<ReadSignal<Vec<KeyChord>>, RuntimeError> {
        Ok(self.command(id)?.shortcuts.read_only())
    }
    /// The simple enabled binding installed by [`Self::command_enabled_signal`],
    /// so a control presenting the command can gray out with it.
    ///
    /// `None` means the command is always enabled or uses a contextual predicate
    /// from [`Self::command_enabled`], whose answer depends on the target and is
    /// known only per invocation or through [`Self::command_info`].
    ///
    /// # Errors
    /// `StaleCommand` if the command was unregistered.
    pub fn command_enabled_binding(
        &self,
        id: &CommandId,
    ) -> Result<Option<ReadSignal<bool>>, RuntimeError> {
        Ok(self.command(id)?.enabled_binding.clone())
    }
    /// Installs a pure UI-thread contextual predicate shared by every source.
    /// Signal writes during evaluation are rejected. Errors stop invocation.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and `StaleCommand` for a command that is unregistered or belongs to another runtime.
    /// Errors the predicate itself returns surface later, from [`Self::command_info`] and
    /// [`Self::invoke_command`].
    pub fn command_enabled(
        &mut self,
        id: &CommandId,
        enabled: impl Fn(&Runtime, &CommandContext) -> Result<bool, RuntimeError> + 'static,
    ) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.command(id)?;
        let command = self
            .commands
            .get_mut(&id.serial)
            .ok_or(RuntimeError::StaleCommand)?;
        command.enabled = Rc::new(enabled);
        command.enabled_binding = None;
        Ok(())
    }
    /// Allows an application/capture command from inside an admitted modal chain.
    /// Default false prevents host fallback actions from escaping dialog editing.
    /// This never allows blocked targets outside the modal or bypasses enabled state.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and `StaleCommand` for a command that is unregistered or belongs to another runtime.
    pub fn command_in_modals(&mut self, id: &CommandId, allowed: bool) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.command(id)?;
        self.commands
            .get_mut(&id.serial)
            .ok_or(RuntimeError::StaleCommand)?
            .in_modals = allowed;
        Ok(())
    }
    /// Opts a command into OS repeat invocation. Only repeats of a physical
    /// press admitted by this command invoke it; a repeat cannot start a new
    /// command after focus or modifiers change. An unowned repeat is left to
    /// component input and associated text insertion rather than reserved by a
    /// matching command chord. Releases never invoke commands.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// and `StaleCommand` for a command that is unregistered or belongs to another runtime.
    pub fn command_repeats(&mut self, id: &CommandId, repeat: bool) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.command(id)?;
        self.commands
            .get_mut(&id.serial)
            .ok_or(RuntimeError::StaleCommand)?
            .repeat = repeat;
        Ok(())
    }
    /// Selects an explicit content target independently of focus. None uses only
    /// window/application scopes.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `ForeignRuntime` or `Unmounted` for a foreign or unmounted target; and
    /// `StaleCommandTarget` for a hidden target or one in another window than the selected
    /// one.
    pub fn set_command_target(&mut self, target: Option<&WidgetId>) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        if let Some(target) = target {
            self.check(target)?;
            if !target.0.visible.get() || self.window_of(target)? != self.current_window() {
                return Err(RuntimeError::StaleCommandTarget);
            }
        }
        *self.window.shared.command_target.borrow_mut() = target.map(|id| Rc::downgrade(&id.0));
        Ok(())
    }
    /// Captures this window's current content target for a menu/button/host action.
    /// Toolbar/menu focus does not replace a content target with an unrelated view.
    pub fn command_context(&self, source: CommandSource) -> CommandContext {
        let target = self
            .window
            .shared
            .command_target
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .map(WidgetId)
            .filter(|id| {
                id.0.mounted.get()
                    && id.0.visible.get()
                    && Rc::ptr_eq(&id.0.window(), &self.window.shared)
            });
        CommandContext {
            window: self.current_window(),
            target,
            focused: self.window.focus.clone(),
            source,
            repeat: false,
        }
    }
    fn validate_command_context(&self, context: &CommandContext) -> Result<(), RuntimeError> {
        if !self.contains_window(&context.window) || !context.window.0.visible.get() {
            return Err(RuntimeError::StaleCommandTarget);
        }
        if let Some(target) = &context.target {
            self.check(target)
                .map_err(|_| RuntimeError::StaleCommandTarget)?;
            if !target.0.visible.get() || self.window_of(target)? != context.window {
                return Err(RuntimeError::StaleCommandTarget);
            }
        }
        Ok(())
    }
    /// Reads a command's label/enabled state for the exact captured target.
    /// Temporarily selects its window and discards stale optional focus, matching
    /// invocation context. The caller's selected root is restored even on failure.
    /// A modal-blocked command reports disabled. The predicate is read-only; this
    /// query does not flush pending UI changes or execute queued actions. Modal
    /// admission uses current root visibility and declared kind even before
    /// surface synchronization. A minimized application-modal owner still blocks
    /// other windows, and incomplete application-modal admission stays blocked.
    /// The read-only query makes one pass over all mounted nodes to inspect surface
    /// declarations; pending
    /// component updates remain unevaluated until a flush.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `StaleCommandTarget` when the context's window is gone or hidden, its target is
    /// foreign, unmounted, hidden or in another window, or the command's scope does not
    /// cover the target; `Unmounted` when the context's window cannot be selected;
    /// `StaleCommand` for a command that is unregistered or belongs to another runtime; and
    /// the error of the command's enabled predicate.
    pub fn command_info(
        &mut self,
        id: &CommandId,
        context: &CommandContext,
    ) -> Result<CommandInfo, RuntimeError> {
        self.shared.writable()?;
        self.validate_command_context(context)?;
        let mut context = context.clone();
        self.sanitize_command_focus(&mut context);
        let previous = self.current_window();
        self.select_window(&context.window)?;
        let result = (|| {
            let command = self.command(id)?;
            let eligible = self.command_scope_allows(&command.scope, &context);
            if !eligible {
                return Err(RuntimeError::StaleCommandTarget);
            }
            let previous = self.shared.phase.replace(Phase::Inspect);
            let _guard = InspectionGuard {
                shared: self.shared.clone(),
                previous,
            };
            Ok(CommandInfo {
                name: command.name.clone(),
                enabled: self.command_admitted(command, &context)
                    && (command.enabled)(self, &context)?,
            })
        })();
        self.select_window(&previous)?;
        result
    }
    /// Lists every registered command whose scope covers the captured context,
    /// with its name, scope, shortcuts and enabled state, so an application
    /// can present commands without keeping a second registry.
    ///
    /// This is [`Self::command_info`] for all eligible commands at once, with
    /// its rules: the context's window is selected for the query and the
    /// caller's selection restored, stale optional focus is discarded, a
    /// modal-blocked command is listed as disabled, and predicates run
    /// read-only. A predicate's error is reported in its own row's
    /// [`CommandEntry::enabled`], so one failing predicate cannot hide the
    /// other commands. Application and capture commands are always eligible, window
    /// commands for their window, and view commands when the context's target
    /// lies in their view. Rows are sorted by name in code-point order, ties in
    /// registration order; ranking for a search is the caller's.
    ///
    /// The result is an owned snapshot. A command palette usually opens from a
    /// command's action, which receives the runtime and the invoking context:
    /// list there, publish the rows to the palette, and invoke the chosen
    /// row's id, which revalidates it.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{CommandScope, CommandSource, KeyChord, Runtime};
    /// let mut ui = Runtime::new();
    /// let save = ui.register_command("Save", CommandScope::Application,
    ///     vec![KeyChord::primary('s')], |_, _| Ok(()))?;
    /// let export = ui.register_command("Export", CommandScope::Application,
    ///     Vec::new(), |_, _| Ok(()))?;
    /// let saving = ui.signal(false);
    /// ui.command_enabled_signal(&export, saving.read_only())?;
    /// let context = ui.command_context(CommandSource::Programmatic);
    /// let rows = ui.commands_for(&context)?;
    /// let names: Vec<_> = rows.iter().map(|row| (row.name.as_str(), row.enabled.clone())).collect();
    /// assert_eq!(names, [("Export", Ok(false)), ("Save", Ok(true))]);
    /// assert_eq!(rows[1].id, save);
    /// assert_eq!(rows[1].shortcuts, [KeyChord::primary('s')]);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// Mounted surface declarations are inspected once for the entire listing,
    /// independently of the number of eligible commands.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// `StaleCommandTarget` when the context's window is gone or hidden or its target is
    /// foreign, unmounted, hidden or in another window; and `Unmounted` when the context's
    /// window cannot be selected. Predicate errors are per row, not errors of the listing.
    pub fn commands_for(
        &mut self,
        context: &CommandContext,
    ) -> Result<Vec<CommandEntry>, RuntimeError> {
        self.shared.writable()?;
        self.validate_command_context(context)?;
        let mut context = context.clone();
        self.sanitize_command_focus(&mut context);
        let previous = self.current_window();
        self.select_window(&context.window)?;
        let rows = {
            let previous = self.shared.phase.replace(Phase::Inspect);
            let _guard = InspectionGuard {
                shared: self.shared.clone(),
                previous,
            };
            let mut rows = Vec::new();
            let modal = self.command_modal_policy(&context);
            // Registration order: the map is keyed by increasing serial.
            for command in self.commands.values() {
                if !self.command_scope_allows(&command.scope, &context) {
                    continue;
                }
                rows.push(CommandEntry {
                    id: command.id.clone(),
                    name: command.name.clone(),
                    scope: command.scope.clone(),
                    shortcuts: command.chords.clone(),
                    enabled: if modal.admits(command) {
                        (command.enabled)(self, &context)
                    } else {
                        Ok(false)
                    },
                });
            }
            // Stable, so equal names keep registration order.
            rows.sort_by(|a, b| a.name.cmp(&b.name));
            rows
        };
        self.select_window(&previous)?;
        Ok(rows)
    }
    fn sanitize_command_focus(&self, context: &mut CommandContext) {
        context.focused = context.focused.take().filter(|focused| {
            self.window_of(focused).ok().as_ref() == Some(&context.window)
                && focused.0.visible.get()
        });
    }
    fn command_scope_allows(&self, scope: &CommandScope, context: &CommandContext) -> bool {
        match scope {
            CommandScope::Application | CommandScope::Capture => true,
            CommandScope::Window(window) => *window == context.window,
            CommandScope::View(view) => context
                .target
                .as_ref()
                .is_some_and(|target| self.descendant(target.0.serial, view.0.serial)),
        }
    }
    fn command_admitted(&self, command: &RegisteredCommand, context: &CommandContext) -> bool {
        self.command_modal_policy(context).admits(command)
    }
    /// Invokes the shared action path for keyboard, menus, buttons and host calls.
    ///
    /// Revalidates context, scope, modality and enabled state each time. A stale
    /// menu never retargets another editor. Results default to the invoking window;
    /// the action may explicitly target another validated window. Callback errors
    /// propagate without replaying an action or rolling back completed host work.
    ///
    /// Two side effects precede and follow the action:
    ///
    /// - Before admission, every presentation root is flushed as by
    ///   [`HostIntegration::flush_windows`](crate::host::HostIntegration::flush_windows), so a
    ///   modal opened through a signal is admitted
    ///   before the captured target is validated. A flush error, even from a
    ///   component unrelated to the command, is returned before the command is
    ///   admitted, and the action does not run.
    /// - An action that returns an error is followed by
    ///   [`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input):
    ///   held keys, pointer buttons, capture and component gestures in the
    ///   selected window end, as after any failed input callback, and the error
    ///   is returned.
    ///
    /// Admission uses the synchronized surface frontier, without walking all
    /// mounted nodes again.
    ///
    /// The window selected before the call is selected again afterwards, unless
    /// the action retired it.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// the flush errors of
    /// [`HostIntegration::flush_windows`](crate::host::HostIntegration::flush_windows), before
    /// admission; the context and
    /// command errors of [`Self::command_info`]; `BorrowConflict` when the action invokes
    /// itself; and the action's own error, after
    /// [`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input). An error
    /// returned by the action is an explicit command failure, including a nested
    /// runtime error that the action chooses to propagate. Catch and handle an
    /// unrelated nested delivery error inside the action to keep the command
    /// successful. A refused
    /// admission is not an error: it is a [`CommandOutcome`] such as `Blocked` or
    /// `Disabled`.
    pub fn invoke_command(
        &mut self,
        id: &CommandId,
        context: CommandContext,
    ) -> Result<CommandOutcome, RuntimeError> {
        self.shared.writable()?;
        let previous = self.current_window();
        let _guard = CommandDispatchGuard::new(self.shared.clone());
        // Admit signal-driven modal changes in every root before validating a
        // captured target. This also separates successive queued invocations.
        self.flush_windows(self.now())?;
        let result = self.invoke_command_inner(id, context);
        if self.contains_window(&previous) {
            self.select_window(&previous)?;
        }
        result
    }
    fn invoke_command_inner(
        &mut self,
        id: &CommandId,
        mut context: CommandContext,
    ) -> Result<CommandOutcome, RuntimeError> {
        self.shared.writable()?;
        self.validate_command_context(&context)?;
        self.sanitize_command_focus(&mut context);
        self.select_window(&context.window)?;
        let command = self.command(id)?;
        if !self.command_scope_allows(&command.scope, &context) {
            return Err(RuntimeError::StaleCommandTarget);
        }
        if !self
            .synchronized_command_modal_policy(&context)
            .admits(command)
        {
            return Ok(CommandOutcome::Blocked);
        }
        let repeats = command.repeat;
        let action = command.action.clone();
        let enabled = command.enabled.clone();
        let is_enabled = {
            let previous = self.shared.phase.replace(Phase::Inspect);
            let _guard = InspectionGuard {
                shared: self.shared.clone(),
                previous,
            };
            enabled(self, &context)?
        };
        if !is_enabled {
            return Ok(CommandOutcome::Disabled);
        }
        if context.repeat && !repeats {
            return Ok(CommandOutcome::RepeatSuppressed);
        }
        let mut action = action
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        let result = {
            let _errors = super::notify::InputErrorGuard::new(&self.shared);
            action(self, &context)
        };
        if result.is_err() {
            match self.cancel_input() {
                // An action that shut the runtime down has already ended all
                // input; its own error is the one to report.
                Ok(()) | Err(RuntimeError::Closed) => {}
                Err(error) => return Err(error),
            }
            if let Some(input) = self.shared.notification_input.borrow().as_ref() {
                input.command_cleanup.set(true);
            }
        }
        result?;
        Ok(CommandOutcome::Invoked)
    }
    fn command_modal_allows(&self, context: &CommandContext) -> bool {
        self.synchronized_command_modal_policy(context)
            .allows_target
    }
    // Input and invocation have synchronized every window before admission.
    // Their policy visits surface roots rather than every mounted component.
    fn synchronized_command_modal_policy(&self, context: &CommandContext) -> CommandModalPolicy {
        let local_modal = self
            .window
            .surfaces
            .iter()
            .any(|surface| self.active_modal_kind(surface.root).is_some());
        let allows_target = context
            .target
            .as_ref()
            .or(context.focused.as_ref())
            .map_or_else(
                || {
                    !local_modal
                        && !self
                            .shared
                            .application_modal
                            .get()
                            .into_iter()
                            .chain(self.pending_application_modals())
                            .any(|root| self.application_modal_active(root))
                },
                |target| self.modal_allows(target.0.serial),
            );
        CommandModalPolicy {
            allows_target,
            local_modal,
        }
    }
    fn command_modal_policy(&self, context: &CommandContext) -> CommandModalPolicy {
        let target = context.target.as_ref().or(context.focused.as_ref());
        let mut policy = CommandModalPolicy {
            allows_target: true,
            local_modal: false,
        };
        // A newly shown root may not be in the last surface frontier. Queries
        // inspect current mounted declarations, without layout or admission work.
        for (&root, entry) in &self.entries {
            let window = entry.node.window();
            if !entry.node.mounted.get() || (!entry.node.visible.get() && window.visible.get()) {
                continue;
            }
            let Some(kind) = self.surface_kind(root).filter(|kind| kind.modal()) else {
                continue;
            };
            if kind == crate::SurfaceKind::ApplicationDialog
                && self.application_modal_active(root)
                && target.is_none_or(|target| !self.descendant(target.0.serial, root))
            {
                policy.allows_target = false;
            }
            if !entry.node.visible.get() || !Rc::ptr_eq(&entry.node.window(), &context.window.0) {
                continue;
            }
            policy.local_modal = true;
            if kind != crate::SurfaceKind::ApplicationDialog {
                let allowed = target.is_some_and(|target| {
                    self.descendant(target.0.serial, root)
                        || (kind == crate::SurfaceKind::ViewDialog
                            && entry
                                .node
                                .presentation_parent_serial()
                                .is_some_and(|owner| !self.descendant(target.0.serial, owner)))
                });
                policy.allows_target &= allowed;
            }
        }
        policy
    }
    pub(super) fn select_content_target(&mut self, serial: u64) {
        let mut target = Some(serial);
        while let Some(serial) = target {
            if self.surface_kind(serial).is_some_and(|kind| kind.modal()) || self.entries.get(&serial).is_some_and(|entry| entry.component.command_target()) || self.commands.values().any(|command| matches!(&command.scope, CommandScope::View(view) if view.0.serial == serial)) {
                *self.window.shared.command_target.borrow_mut() = self.entries.get(&serial).map(|entry| Rc::downgrade(&entry.node)); return;
            }
            target = self
                .entries
                .get(&serial)
                .and_then(|e| e.node.presentation_parent_serial());
        }
    }
    fn route_bindings(
        &mut self,
        scope: &CommandScope,
        input: &CommandKeyInput,
        context: &CommandContext,
    ) -> Result<bool, RuntimeError> {
        // Owned repeats are delivered by command_key_input_inner before routing.
        // A repeat cannot establish a new command after a component owned its press.
        if context.repeat {
            return Ok(false);
        }
        let matching: Vec<_> = self
            .commands
            .values()
            .filter(|entry| entry.scope == *scope)
            .filter_map(|entry| {
                entry
                    .chords
                    .iter()
                    .find(|chord| chord.matches(input, self.shared.modifiers.get()))
                    .map(|chord| (entry.id.clone(), *chord))
            })
            .collect();
        if let [(_, chord), _, ..] = matching.as_slice() {
            return Err(RuntimeError::ShortcutConflict(*chord));
        }
        if let Some((id, _)) = matching.first() {
            let outcome = self.invoke_command(id, context.clone())?;
            if outcome == CommandOutcome::Blocked {
                return Ok(false);
            }
            self.command_presses.insert(
                input.physical,
                CommandPress {
                    id: id.clone(),
                    context: context.clone(),
                    admitted: outcome == CommandOutcome::Invoked,
                },
            );
            return Ok(true);
        }
        Ok(false)
    }
    /// Routes one translated native key exactly once. Text editing precedes
    /// explicit capture, ordinary focus bubbling, target/window bindings and app
    /// fallback. The returned flag controls associated native text insertion.
    /// Native hosts must discard synthetic events before calling this method.
    ///
    /// The complete order, after a flush of every window; the first stage that
    /// consumes the key ends routing. A fresh press is the first press of a
    /// key, not a repeat.
    ///
    /// 1. A repeat of a key that is not held (an orphan) is consumed and
    ///    changes nothing.
    /// 2. During a drag session, Escape's fresh press cancels the drag.
    /// 3. Keys already owned are consumed by their owner. First a held-input
    ///    owner that is no longer available, or that a modal surface now
    ///    blocks, is cancelled. Then:
    ///    - a held key whose input was cancelled stays suppressed until its
    ///      release or a new fresh press;
    ///    - a key whose fresh press a held-input registration took sends its
    ///      repeats and release to that registration;
    ///    - a key whose fresh press matched a command binding keeps it: each
    ///      repeat runs the command again when its press was admitted (a
    ///      disabled command's press was consumed without running), and the
    ///      release ends it.
    /// 4. A key that a modal surface forbids to commands is consumed.
    /// 5. Shown delayed help takes an unmodified fresh Escape press.
    /// 6. The text phase: from the focused component up, each
    ///    [`Component::text_key_event`](crate::Component::text_key_event)
    ///    may handle the key or insert its text; an editor that leaves it
    ///    passes it to its parent, and the first component that is not an
    ///    editor ends the phase. An active composition then consumes the key.
    /// 7. On a fresh press, command bindings of [`CommandScope::Capture`],
    ///    then [`Runtime::register_held_input`] registrations, each taking
    ///    the key as in stage 3. A repeat never starts a command.
    /// 8. Escape's fresh press dismisses the frontmost dismissible surface,
    ///    and otherwise ends the focused component's timers, capture and
    ///    gesture contexts before it is delivered.
    /// 9. An unmodified Tab or Shift+Tab, pressed or repeated, is offered to
    ///    the focused component and its ancestors through
    ///    [`Component::tab_navigation`](crate::Component::tab_navigation); when
    ///    none claims it, focus moves to the next or previous Tab stop. Either
    ///    way the key is consumed.
    /// 10. Ordinary bubbling
    ///     ([`Component::keyboard_event`](crate::Component::keyboard_event)),
    ///     from the component that ended the text phase upward.
    /// 11. On a fresh press, command bindings of the command target's views,
    ///     outward, then of the window, then of the application.
    ///
    /// This is the entry point the native `App` uses, and the only one that
    /// routes command bindings and [`Runtime::register_held_input`]
    /// registrations. A test or custom host that should behave as an
    /// application does for its user uses this method rather than
    /// [`Runtime::key_input`]. [`Driver::key`](crate::testing::Driver::key)
    /// and [`Driver::chord`](crate::testing::Driver::chord) send synthetic
    /// named keys and chords through it; a test of a layout, of text a key
    /// carries or of a composition builds the [`CommandKeyInput`] itself.
    ///
    /// The key is delivered at once and its recipient writes what its own
    /// rules write, as [`Runtime::text_input`] describes; the tree, derived
    /// bindings and layout show the result after the next
    /// [`Runtime::flush`]. During a drag session, Escape's press cancels the drag
    /// before anything else sees it, and a modifier change re-resolves the
    /// offered operation, exactly as through [`Runtime::key_input`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase;
    /// forwards the errors of the flush that runs first, of the focused component's key
    /// callbacks ([`Component::text_key_event`](crate::Component::text_key_event),
    /// [`Component::tab_navigation`](crate::Component::tab_navigation),
    /// [`Component::keyboard_event`](crate::Component::keyboard_event)), of dismissing
    /// shown help or a surface, of held-input
    /// actions, of any command invoked (see [`Self::invoke_command`]) and of the flush
    /// that delivers a notification the key requested; and returns
    /// `ShortcutConflict` when bindings of more than one command in one scope match the
    /// key. Direct dispatch errors and delivery failures attributable to this key
    /// cancel input as by
    /// [`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input); an error from
    /// that cancellation
    /// is returned instead. Older/external notification failures, including
    /// notifications generated by the initial preflush, propagate without cancelling
    /// held input. Collective settlement errors without a uniquely attributable
    /// request also preserve the current gesture. An invoked command action's
    /// returned error remains an explicit command failure and cancels input,
    /// as documented by [`Self::invoke_command`]. If the initial preflush prevents
    /// a held key's release from dispatching, cleanup still releases its physical
    /// state and held actions. Ordinary preflush update failures, including
    /// failures in another visible window, also cancel input in the selected
    /// window before returning the error.
    ///
    /// All presentation roots are flushed before routing, so a newly shown
    /// application dialog in another window also blocks text delivery. Modal
    /// routing then checks synchronized surfaces rather than all mounted nodes.
    pub fn command_key_input(
        &mut self,
        input: CommandKeyInput,
    ) -> Result<KeyDispatch, RuntimeError> {
        let notifications = super::notify::InputNotificationGuard::new(self.shared.clone());
        let terminal =
            input.action == KeyAction::Release && self.keyboard.is_pressed(input.physical);
        // Only a notification this dispatch requested fails like the key itself.
        let result = self
            .command_key_input_inner(input)
            .and_then(|dispatch| self.deliver_input_notifications().map(|()| dispatch));
        if result.is_err() && notifications.cancel_on_error(terminal) {
            match self.cancel_input() {
                // A key whose command shut the runtime down keeps its error.
                Ok(()) | Err(RuntimeError::Closed) => {}
                Err(error) => return Err(error),
            }
        }
        result
    }
    fn command_key_input_inner(
        &mut self,
        input: CommandKeyInput,
    ) -> Result<KeyDispatch, RuntimeError> {
        self.shared.writable()?;
        self.flush_windows(self.now())?;
        if let Some(input) = self.shared.notification_input.borrow().as_ref() {
            self.shared
                .notification_origin
                .replace(Some(Rc::downgrade(input)));
        }
        let was_pressed = self.keyboard.is_pressed(input.physical);
        let event = if input.action == KeyAction::Repeat && !was_pressed {
            // An orphan repeat must never establish a new physical press. Keep
            // rejecting every late repeat until a real press arrives.
            KeyEvent {
                code: input.physical,
                transition: KeyTransition::Repeated,
            }
        } else {
            let named = match (input.physical, input.logical) {
                (KeyCode::Native(_), Some(CommandKey::Named(named))) => named,
                _ => input.physical,
            };
            self.keyboard
                .apply_control_key(input.physical, input.action, named)
        };
        if let Some(input) = self.shared.notification_input.borrow().as_ref() {
            input.dispatched.set(true);
        }
        let modifiers_changed =
            self.shared.modifiers.replace(self.keyboard.modifiers()) != self.keyboard.modifiers();
        let mut result = KeyDispatch {
            event,
            handled: false,
            deliver_text: false,
        };
        if input.action == KeyAction::Repeat && !was_pressed {
            result.handled = true;
            return Ok(result);
        }
        let mut editing_input = input;
        if matches!(input.physical, KeyCode::Native(_)) {
            let control = self.keyboard.control_key(event);
            if control != input.physical {
                editing_input.base = Some(CommandKey::Named(control));
                editing_input.logical = Some(CommandKey::Named(control));
                // A named control held across a NumLock/layout change cannot
                // start inserting the new printable payload. Preserve text
                // for an unchanged named key (for example a native Space).
                editing_input.has_text &= input.logical == Some(CommandKey::Named(control));
            } else {
                // A key pressed as text cannot adopt a named control action
                // when NumLock/layout changes before its matching release.
                if matches!(editing_input.base, Some(CommandKey::Named(_))) {
                    editing_input.base = None;
                }
                if matches!(editing_input.logical, Some(CommandKey::Named(_))) {
                    editing_input.logical = None;
                }
            }
        }
        if self.drag_in_window() {
            // As in `key_input`: Escape cancels the drag before any other
            // routing, and a modifier may change the offered operation.
            if input.physical == KeyCode::Escape && event.transition == KeyTransition::Pressed {
                self.finish_drag(false)?;
                result.handled = true;
                return Ok(result);
            }
            if modifiers_changed {
                self.resolve_drag()?;
            }
        }
        self.cancel_blocked_held();
        if self.owned_key(input, event)? {
            result.handled = true;
            return Ok(result);
        }
        if let Some(mut owner) = self.command_presses.get(&input.physical).cloned() {
            if matches!(
                event.transition,
                KeyTransition::Released | KeyTransition::UnmatchedRelease
            ) {
                self.command_presses.remove(&input.physical);
            } else if event.transition == KeyTransition::Repeated && owner.admitted {
                owner.context.repeat = true;
                if self.command(&owner.id).is_ok()
                    && self.validate_command_context(&owner.context).is_ok()
                {
                    self.invoke_command(&owner.id, owner.context)?;
                }
            }
            result.handled = true;
            return Ok(result);
        }
        let mut context = self.command_context(CommandSource::KeyBinding);
        context.repeat = event.transition == KeyTransition::Repeated;
        if !self.command_modal_allows(&context) {
            result.handled = true;
            return Ok(result);
        }
        // Shown help takes an unmodified Escape before any editor does.
        if input.physical == KeyCode::Escape
            && event.transition == KeyTransition::Pressed
            && self.tooltip_escape()?
        {
            result.handled = true;
            return Ok(result);
        }
        // The text phase starts at the focused component. An editor that
        // leaves the key (`Ignored`) passes the text phase on to its parent, so
        // a control that owns an embedded editor answers the keys the editor
        // leaves to it before capture bindings see them. The first component
        // that is not an editor ends the phase; ordinary bubbling starts there.
        let mut ordinary = self.window.focus.as_ref().map(|id| id.0.serial);
        while let Some(serial) = ordinary {
            let entry = self
                .entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?;
            entry.node.input_cancelled.set(false);
            let text_result = entry.component.text_key_event(
                &mut EventContext {
                    node: entry.node.clone(),
                },
                editing_input,
                event.transition,
                &self.keyboard,
            )?;
            match text_result {
                TextKeyOutcome::Handled => {
                    result.handled = true;
                    return Ok(result);
                }
                TextKeyOutcome::InsertText => {
                    result.handled = true;
                    result.deliver_text =
                        editing_input.has_text && text_modifiers_admit(self.keyboard.modifiers());
                    return Ok(result);
                }
                TextKeyOutcome::Ignored => ordinary = entry.node.presentation_parent_serial(),
                TextKeyOutcome::NotAnEditor => break,
            }
        }
        if input.composing {
            result.handled = true;
            result.deliver_text =
                editing_input.has_text && text_modifiers_admit(self.keyboard.modifiers());
            return Ok(result);
        }
        let press = matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        );
        if press && self.route_bindings(&CommandScope::Capture, &input, &context)? {
            result.handled = true;
            return Ok(result);
        }
        if press && self.capture_held(input, event)? {
            result.handled = true;
            return Ok(result);
        }
        if input.physical == KeyCode::Escape && event.transition == KeyTransition::Pressed {
            if self.surface_escape()? {
                result.handled = true;
                return Ok(result);
            }
            // As in `key_input`: Escape ends the focused component's timers,
            // capture and gesture contexts before it is delivered.
            if let Some(id) = &self.window.focus {
                id.0.lose_context();
            }
        }
        if input.physical == KeyCode::Tab
            && !self.keyboard.modifiers().control
            && !self.keyboard.modifiers().alt
            && !self.keyboard.modifiers().super_key
            && press
        {
            if !self.offer_tab(event, None)? {
                self.focus_next(self.tab_direction())?;
            }
            result.handled = true;
            return Ok(result);
        }
        if let Some(serial) = ordinary
            && self.dispatch_keyboard(serial, event)? == InputOutcome::Handled
        {
            result.handled = true;
            return Ok(result);
        }
        if press {
            let mut target = context.target.as_ref().map(|id| id.0.serial);
            while let Some(serial) = target {
                let node = self
                    .entries
                    .get(&serial)
                    .ok_or(RuntimeError::StaleCommandTarget)?
                    .node
                    .clone();
                if self.route_bindings(
                    &CommandScope::View(WidgetId(node.clone())),
                    &input,
                    &context,
                )? {
                    result.handled = true;
                    return Ok(result);
                }
                target = node.presentation_parent_serial();
            }
            if self.route_bindings(
                &CommandScope::Window(context.window.clone()),
                &input,
                &context,
            )? || self.route_bindings(&CommandScope::Application, &input, &context)?
            {
                result.handled = true;
                return Ok(result);
            }
        }
        result.deliver_text =
            editing_input.has_text && press && text_modifiers_admit(self.keyboard.modifiers());
        Ok(result)
    }
    pub(super) fn remove_window_commands(&mut self, window: &crate::NativeWindowId) {
        self.commands.retain(
            |_, command| !matches!(&command.scope, CommandScope::Window(scope) if scope == window),
        );
    }
    pub(super) fn remove_scoped_commands(&mut self, serial: u64) {
        self.commands.retain(|_, command| !matches!(&command.scope, CommandScope::View(view) if view.0.serial == serial));
    }
}

impl Runtime {
    /// Reconciles OS-reported modifiers, including keys held before native focus
    /// arrived. This does not fabricate physical press/release events. Native hosts
    /// call it for ModifiersChanged; native focus loss clears the state.
    ///
    /// During a drag session in this window, a change re-resolves the offered
    /// operation, so the target sees an `Over` with the new modifiers before a
    /// release performs it.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`](crate::RuntimeError::Closed) after shutdown, and
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` when called from inside that phase.
    /// During a drag session it forwards the errors of re-resolving the offer: the target's
    /// [`Component::drag_event`](crate::Component::drag_event) error and
    /// `DropNotPermitted`.
    pub fn native_modifiers(&mut self, modifiers: Modifiers) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.keyboard.set_modifiers(modifiers);
        let changed = self.shared.modifiers.replace(modifiers) != modifiers;
        if changed && self.drag_in_window() {
            self.resolve_drag()?;
        }
        Ok(())
    }
}

/// Reactive completion of a queued menu/button invocation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    /// The action will run at the next runtime flush, outside the component borrow.
    Pending,
    /// One common invocation path completed or rejected admission.
    Finished(Result<CommandOutcome, RuntimeError>),
    /// Runtime shutdown discarded the request before invocation.
    Cancelled,
}
/// UI-local completion of one queued menu/button invocation.
///
/// The status is stored outside the runtime's signal graph, like an
/// [`TaskReceipt`](crate::TaskReceipt)'s, so it stays readable after
/// runtime shutdown, where [`CommandStatus::Cancelled`] reports an invocation
/// the shutdown discarded. [`Self::status_binding`] mirrors it as a binding, so
/// a component can report a typed failure without polling while the runtime
/// lives. Retaining a receipt retains neither the command nor the runtime.
///
/// Receipts compare equal when they report the same invocation: clones of one
/// receipt are equal, and two invocations are different even when they end in
/// the same status.
#[derive(Clone)]
#[must_use = "the receipt is the only report of the invocation's result, including the \
              action's own error"]
pub struct CommandReceipt(ReceiptStatus<CommandStatus>);
impl CommandReceipt {
    /// Pending or the retained terminal result, readable after shutdown.
    /// Reading creates no subscription or timer.
    #[must_use]
    pub fn status(&self) -> CommandStatus {
        self.0.get()
    }
    /// The same status as a binding, so a component that reads it during
    /// update is invalidated when the invocation finishes. Like every binding
    /// it reads `Err(RuntimeError::Closed)` after runtime shutdown; use
    /// [`Self::status`] then.
    pub fn status_binding(&self) -> ReadSignal<CommandStatus> {
        self.0.binding()
    }
}
impl PartialEq for CommandReceipt {
    fn eq(&self, other: &Self) -> bool {
        self.0.same(&other.0)
    }
}
impl Eq for CommandReceipt {}
/// Shows the current status, as [`CommandReceipt::status`] reads it.
impl std::fmt::Debug for CommandReceipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandReceipt")
            .field("status", &self.status())
            .finish()
    }
}
pub(super) struct QueuedCommand {
    id: CommandId,
    context: CommandContext,
    receipt: CommandReceipt,
}
impl Runtime {
    // Documented on `crate::host::HostIntegration::flush_commands`.
    pub(crate) fn flush_commands(&mut self) -> Result<(), RuntimeError> {
        self.shared.writable()?;
        self.dispatch_queued_commands()
    }
    // Documented on `crate::host::HostIntegration::has_command_requests`.
    pub(crate) fn has_command_requests(&self) -> bool {
        !self.shared.command_requests.borrow().is_empty()
    }
    pub(super) fn dispatch_queued_commands(&mut self) -> Result<(), RuntimeError> {
        if self.shared.command_dispatching.get() {
            return Ok(());
        }
        let _guard = CommandDispatchGuard::new(self.shared.clone());
        let requests = std::mem::take(&mut *self.shared.command_requests.borrow_mut());
        for request in requests {
            // An earlier action may have shut the runtime down; the rest of the
            // batch was discarded with it rather than failing admission.
            let status = if self.shared.closed.get() {
                CommandStatus::Cancelled
            } else {
                // This boundary captures failures in a receipt instead of
                // propagating them to the input whose flush services the queue.
                let _errors = super::notify::InputErrorGuard::new(&self.shared);
                CommandStatus::Finished(self.invoke_command(&request.id, request.context.clone()))
            };
            request.finish(status);
        }
        Ok(())
    }
    /// Uses one reactive enabled value for this command and UI controls bound to
    /// the same signal. For contextual predicates,
    /// use command_enabled and query command_info for each captured target.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for a signal of another runtime and the signal's read
    /// errors (`Closed`, `BorrowConflict`), then the errors of [`Self::command_enabled`].
    pub fn command_enabled_signal(
        &mut self,
        id: &CommandId,
        enabled: impl Into<ReadSignal<bool>>,
    ) -> Result<(), RuntimeError> {
        let enabled: ReadSignal<bool> = enabled.into();
        self.read_signal(&enabled)?;
        let binding = enabled.clone();
        self.command_enabled(id, move |_, _| enabled.get())?;
        self.commands
            .get_mut(&id.serial)
            .ok_or(RuntimeError::StaleCommand)?
            .enabled_binding = Some(binding);
        Ok(())
    }
}
impl EventContext {
    /// Captures the active content target independently of toolbar/menu focus.
    /// Retain this context when opening a menu. A removed/moved target will fail
    /// validation at invocation; a stale optional focused identity is discarded.
    pub fn command_context(&self, source: CommandSource) -> CommandContext {
        let window = self.node.window();
        let target = window
            .command_target
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .filter(|node| {
                node.mounted.get() && node.visible.get() && Rc::ptr_eq(&node.window(), &window)
            })
            .map(WidgetId);
        let focused = window
            .focus_node
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .map(WidgetId);
        CommandContext {
            window: self.native_window(),
            target,
            focused,
            source,
            repeat: false,
        }
    }
    /// Queues the exact shared command from a button or captured menu target.
    /// The next flush revalidates target, scope, enabled state and modality
    /// before calling host code once, outside this component borrow, through
    /// [`Runtime::invoke_command`] with its side effects. Observe the receipt:
    /// action failures are retained there, never silently lost or replayed.
    /// Associated keyboard text must already be suppressed by routing.
    ///
    /// # Errors
    /// Returns `CommandQueueFull` when [`Runtime::MAX_QUEUED_COMMANDS`]
    /// invocations are waiting, or the runtime phase/shutdown error, without
    /// queueing the invocation.
    pub fn invoke_command(
        &self,
        id: &CommandId,
        context: CommandContext,
    ) -> Result<CommandReceipt, RuntimeError> {
        self.node.shared.writable()?;
        let mut requests = self.node.shared.command_requests.borrow_mut();
        if requests.len() >= Runtime::MAX_QUEUED_COMMANDS {
            return Err(RuntimeError::CommandQueueFull);
        }
        let receipt = CommandReceipt(ReceiptStatus::new(
            self.node.shared.clone(),
            CommandStatus::Pending,
        ));
        requests.push(QueuedCommand {
            id: id.clone(),
            context,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }
}
impl Runtime {
    /// Maximum number of queued command invocations, from
    /// [`EventContext::invoke_command`], waiting for the next flush. An
    /// invocation beyond it is refused with [`RuntimeError::CommandQueueFull`];
    /// the queue is emptied by every flush and by
    /// [`HostIntegration::flush_commands`](crate::host::HostIntegration::flush_commands).
    pub const MAX_QUEUED_COMMANDS: usize = 64;
}

impl QueuedCommand {
    /// Stores a terminal status and publishes it to the receipt's binding,
    /// which cannot fail, in any phase and after shutdown alike.
    fn finish(&self, status: CommandStatus) {
        self.receipt.0.set(status);
    }
    /// Discards an invocation that runtime shutdown ended before it ran.
    pub(super) fn cancel(self) {
        self.finish(CommandStatus::Cancelled);
    }
}
impl Drop for QueuedCommand {
    fn drop(&mut self) {
        // A request dropped without a terminal status, for example by an
        // unwinding action, reports cancellation rather than staying pending.
        if self.receipt.status() == CommandStatus::Pending {
            self.finish(CommandStatus::Cancelled);
        }
    }
}

struct CommandDispatchGuard {
    shared: Rc<Shared>,
    previous: bool,
}
impl CommandDispatchGuard {
    fn new(shared: Rc<Shared>) -> Self {
        let previous = shared.command_dispatching.replace(true);
        Self { shared, previous }
    }
}
impl Drop for CommandDispatchGuard {
    fn drop(&mut self) {
        self.shared.command_dispatching.set(self.previous);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Presents a chord in an explicit platform convention.
    struct Label(KeyChord, bool);
    impl std::fmt::Display for Label {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.0.write_label(f, self.1)
        }
    }

    #[test]
    fn chords_are_presented_in_each_platform_convention() {
        let all = Modifiers {
            shift: true,
            control: true,
            alt: true,
            super_key: true,
        };
        let chord = KeyChord::new(CommandKey::Character('s'), all);
        assert_eq!(Label(chord, false).to_string(), "Ctrl+Alt+Shift+Super+S");
        assert_eq!(Label(chord, true).to_string(), "Ctrl+Option+Shift+Cmd+S");
        let named = |code| KeyChord::named(code);
        assert_eq!(
            Label(named(KeyCode::PageDown), false).to_string(),
            "Page Down"
        );
        assert_eq!(Label(named(KeyCode::ArrowLeft), false).to_string(), "Left");
        assert_eq!(
            Label(named(KeyCode::Native(0x2a)), false).to_string(),
            "Key 0x2a"
        );
        let space = KeyChord::new(CommandKey::Character(' '), Modifiers::default());
        assert_eq!(Label(space, false).to_string(), "Space");
        let punctuation = KeyChord::new(CommandKey::Character('+'), Modifiers::default());
        assert_eq!(Label(punctuation, false).to_string(), "+");
        // The physical fallback never appears in the presented chord.
        let fallback = KeyChord::primary('z').physical_fallback(KeyCode::KeyY);
        assert!(fallback.to_string().ends_with('Z'));
    }
}
