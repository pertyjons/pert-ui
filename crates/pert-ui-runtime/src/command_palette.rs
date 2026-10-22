//! A searchable overlay over the registered commands, built on [`QuickPick`].
use crate::*;
use std::{cell::RefCell, rc::Rc};

/// Scores and orders [`CommandEntry`] rows for a query: the palette's
/// replaceable matching and ranking policy.
///
/// A matcher is a pure function of the query and one row that returns `None`
/// when the row does not match and otherwise a score, where higher ranks
/// earlier. It must be deterministic, bounded and free of side effects: it
/// runs on the UI thread for every row each time the query or the listing
/// changes, and must not write bindings or perform I/O. [`Self::rank`] applies
/// it to a snapshot, so ranking is testable without a window or a runtime.
///
/// The default, [`Self::fuzzy`], matches the query against the command name
/// with [`Self::fuzzy_score`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{CommandMatcher, CommandScope, Runtime};
/// let mut ui = Runtime::new();
/// for name in ["Save", "Save As…", "Show Sidebar"] {
///     ui.register_command(name, CommandScope::Application, Vec::new(), |_, _| Ok(()))?;
/// }
/// let context = ui.command_context(pert_ui::CommandSource::Programmatic);
/// let entries = ui.commands_for(&context)?;
/// let names = |order: Vec<usize>| -> Vec<String> {
///     order.into_iter().map(|index| entries[index].name.clone()).collect()
/// };
/// // The default matcher prefers contiguous text at a word start.
/// assert_eq!(names(CommandMatcher::fuzzy().rank("sav", &entries)), ["Save", "Save As…"]);
/// // A replacement matcher, here an exact prefix match, decides alone.
/// let prefix = CommandMatcher::new(|query, entry| {
///     entry.name.to_lowercase().starts_with(&query.to_lowercase()).then_some(1)
/// });
/// assert_eq!(names(prefix.rank("sh", &entries)), ["Show Sidebar"]);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct CommandMatcher(Rc<Score>);
/// A matcher's scoring function.
type Score = dyn Fn(&str, &CommandEntry) -> Option<u32>;
impl CommandMatcher {
    /// Uses `score` as the matching and ranking policy; see the type
    /// documentation for its rules.
    pub fn new(score: impl Fn(&str, &CommandEntry) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(score))
    }
    /// The default policy: [`Self::fuzzy_score`] of the query against the
    /// command name.
    pub fn fuzzy() -> Self {
        Self::new(|query, entry| Self::fuzzy_score(query, &entry.name))
    }
    /// The score of one row, or `None` when it does not match.
    #[must_use]
    pub fn score(&self, query: &str, entry: &CommandEntry) -> Option<u32> {
        (self.0)(query, entry)
    }
    /// The positions in `entries` of the matching rows, highest score first
    /// and ties in snapshot order, which for [`Runtime::commands_for`] is by
    /// name. Rows that do not match are left out; disabled rows are ranked like
    /// any other, since invoking one reports why it did not run.
    #[must_use]
    pub fn rank(&self, query: &str, entries: &[CommandEntry]) -> Vec<usize> {
        let mut scored: Vec<(u32, usize)> = entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| self.score(query, entry).map(|score| (score, index)))
            .collect();
        // Stable: equal scores keep snapshot order.
        scored.sort_by_key(|scored| std::cmp::Reverse(scored.0));
        scored.into_iter().map(|(_, index)| index).collect()
    }
    /// A case-insensitive subsequence match of `query` against `text`,
    /// ignoring whitespace in the query.
    ///
    /// Returns `Some(0)` for a query without other characters, so an empty
    /// query keeps every row in snapshot order, and `None` when the query's
    /// characters do not all appear in `text` in order. Otherwise the score is
    /// higher for matches at the start of `text` or of a word (after a
    /// non-alphanumeric character or at a lower-to-upper case change), for
    /// consecutive matched characters, and when the query appears as one
    /// contiguous run. Characters are compared after Unicode lowercase
    /// mapping. Matching is greedy and leftmost, so the score is a
    /// deterministic heuristic rather than the best possible alignment; the
    /// work is linear in the combined length of `query` and `text`, including
    /// the search for the contiguous run. It is [`PickMatcher::fuzzy_score`],
    /// shared with every [`QuickPick`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::CommandMatcher;
    /// assert_eq!(CommandMatcher::fuzzy_score("", "Anything"), Some(0));
    /// assert_eq!(CommandMatcher::fuzzy_score("xyz", "Save"), None);
    /// let word = CommandMatcher::fuzzy_score("ss", "Show Sidebar").unwrap();
    /// let inner = CommandMatcher::fuzzy_score("ss", "Class list").unwrap();
    /// assert!(word > inner);
    /// ```
    #[must_use]
    pub fn fuzzy_score(query: &str, text: &str) -> Option<u32> {
        PickMatcher::fuzzy_score(query, text)
    }
}
impl Default for CommandMatcher {
    /// [`Self::fuzzy`].
    fn default() -> Self {
        Self::fuzzy()
    }
}
/// Shows the policy without its function.
impl std::fmt::Debug for CommandMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandMatcher").finish_non_exhaustive()
    }
}

/// Where a [`CommandPaletteController`]'s session stands, as
/// [`CommandPaletteController::status`] reports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CommandPaletteStatus {
    /// Nothing is presented. A finished invocation's receipt stays readable
    /// through [`CommandPaletteController::receipt`].
    Closed,
    /// The palette is presented, possibly showing why the last chosen command
    /// did not run.
    Open,
    /// A chosen command is queued: the palette has closed and the command runs
    /// at the next flush. It ends `Closed` when the command ran and `Open`,
    /// showing the outcome, when it did not.
    Invoking,
}
/// [`PickStatus::Pending`] is a queued command; anything a newer picker
/// version adds counts as closed.
impl From<PickStatus> for CommandPaletteStatus {
    fn from(status: PickStatus) -> Self {
        match status {
            PickStatus::Open => Self::Open,
            PickStatus::Pending => Self::Invoking,
            _ => Self::Closed,
        }
    }
}

/// How a palette describes a chosen command that did not run: the command's
/// name and its invocation result, to presented text.
/// The default reads the installed string table; an application's function
/// ignores it.
type OutcomeText =
    Rc<dyn Fn(&crate::UiStrings, &str, &Result<CommandOutcome, RuntimeError>) -> String>;

fn default_outcome_text(
    strings: &crate::UiStrings,
    command: &str,
    result: &Result<CommandOutcome, RuntimeError>,
) -> String {
    use crate::UiMessage;
    match result {
        Ok(CommandOutcome::Invoked) => strings.format(UiMessage::CommandRan { command }),
        Ok(CommandOutcome::Disabled) => strings.format(UiMessage::CommandNotAvailable { command }),
        Ok(CommandOutcome::Blocked) => strings.format(UiMessage::CommandBlocked { command }),
        Ok(CommandOutcome::RepeatSuppressed) => {
            strings.format(UiMessage::CommandNoRepeat { command })
        }
        Err(error) => strings.format(UiMessage::CommandFailed {
            command,
            reason: &error.to_string(),
        }),
    }
}

/// One opened listing: the context captured at opening and its snapshot.
struct Session {
    context: CommandContext,
    entries: Rc<[CommandEntry]>,
}
impl Session {
    /// The entry a picker item stands for, from its `command-<index>` key.
    fn entry(&self, key: &str) -> Option<&CommandEntry> {
        let index: usize = key.strip_prefix("command-")?.parse().ok()?;
        self.entries.get(index)
    }
}

/// Opens, closes and observes one [`CommandPalette`].
///
/// The controller owns the palette's session: the [`CommandContext`] captured
/// when it opens, the snapshot of [`CommandEntry`] rows it lists, the query
/// and the receipt of the last chosen command. Clones address the same
/// session. One mounted palette presents a controller at a time; it claims
/// the controller when it mounts, and a second palette mounted with it stays
/// closed until the first unmounts. It then presents the controller's next
/// session, but not one the first palette was presenting when it unmounted.
/// These are the rules of the [`QuickPickController`] it is built on.
///
/// **Opening needs the runtime**, because [`Runtime::commands_for`] lists the
/// commands and their enabled state with read-only runtime access. The usual
/// way to open a palette is therefore a registered command, whose action
/// receives the runtime and the invoking context: bind the application's own
/// shortcut to it, or invoke it from a button or menu with
/// [`EventContext::invoke_command`]. The palette registers no command itself.
/// [`Self::open_with`] takes a listing the application prepared, for example
/// one that leaves out commands it does not want offered.
///
/// Reads and writes follow signal phase rules: open and close from event
/// callbacks, command actions and application code, never from `update`.
#[derive(Clone)]
#[must_use]
pub struct CommandPaletteController {
    pick: QuickPickController,
    session: Rc<RefCell<Option<Session>>>,
    last: Signal<Option<CommandReceipt>>,
}
/// Shows the status, or the error reading it after runtime shutdown.
impl std::fmt::Debug for CommandPaletteController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandPaletteController")
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}
impl CommandPaletteController {
    /// Creates a closed controller belonging to `ui`.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            pick: QuickPickController::new(ui),
            session: Rc::new(RefCell::new(None)),
            last: ui.signal(None),
        }
    }
    /// Lists the commands `context` can reach with [`Runtime::commands_for`]
    /// and opens the palette over them, as [`Self::open_with`] does.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Column, CommandPalette, CommandPaletteController, CommandPaletteStatus, CommandScope,
    ///     CommandSource, Element, KeyChord, Label, Runtime,
    /// };
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(pert_ui::Viewport::new(800.0, 600.0, 1.0)?)?;
    /// let palette = CommandPaletteController::new(&ui);
    /// let opener = palette.clone();
    /// // The application binds its own shortcut to a command that opens it.
    /// let open = ui.register_command(
    ///     "Show all commands",
    ///     CommandScope::Application,
    ///     vec![KeyChord::primary('p').shifted()],
    ///     move |ui, context| opener.open(ui, context),
    /// )?;
    /// ui.mount_tree(Element::new(
    ///     "app",
    ///     CommandPalette::new(
    ///         palette.clone(),
    ///         Element::new("content", Column::new([Element::new("text", Label::new("Document"))])),
    ///     ),
    /// ))?;
    /// let context = ui.command_context(CommandSource::Programmatic);
    /// ui.invoke_command(&open, context)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(palette.status()?, CommandPaletteStatus::Open);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns the errors of [`Runtime::commands_for`], such as
    /// `StaleCommandTarget` for a context whose window or target is gone, and
    /// then those of [`Self::open_with`].
    pub fn open(&self, ui: &mut Runtime, context: &CommandContext) -> Result<(), RuntimeError> {
        let entries = ui.commands_for(context)?;
        self.open_with(context.clone(), entries)
    }
    /// Opens the palette over `entries`, captured for `context`.
    ///
    /// The palette lists the rows in the given order for an empty query and
    /// ranks them with its [`CommandMatcher`] otherwise. Choosing a row
    /// invokes its command with `context`, whose source becomes
    /// [`CommandSource::Menu`] (a target captured when the palette opened) and
    /// which is never a repeat; invocation revalidates the context, scope,
    /// modality and enabled state, because they can change while the palette
    /// is open. Opening while a session is open replaces it: the query is
    /// cleared and the highlight returns to the first row. A command chosen
    /// earlier and still queued runs as usual, but its outcome no longer
    /// reopens the palette.
    ///
    /// # Errors
    /// Returns the signal write errors, such as `WriteDuringUpdate` or
    /// `Closed` after shutdown.
    pub fn open_with(
        &self,
        mut context: CommandContext,
        entries: Vec<CommandEntry>,
    ) -> Result<(), RuntimeError> {
        context.source = CommandSource::Menu;
        context.repeat = false;
        let strings = self.last.shared().strings.borrow().clone();
        let items = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| command_item(&strings, index, entry))
            .collect();
        self.pick.open(items)?;
        *self.session.borrow_mut() = Some(Session {
            context,
            entries: entries.into(),
        });
        Ok(())
    }
    /// Closes a presented palette, including one showing why a command did
    /// not run. Returns whether it was presented. While a chosen command is
    /// queued the palette is not presented, so this changes nothing then and
    /// the command's outcome can still reopen it; a queued command is never
    /// cancelled and its receipt always completes.
    ///
    /// # Errors
    /// Returns the signal read and write errors, such as `WriteDuringUpdate`
    /// or `Closed` after shutdown.
    pub fn close(&self) -> Result<bool, RuntimeError> {
        self.pick.close()
    }
    /// The session's current status. Reading creates no subscription.
    ///
    /// # Errors
    /// Returns the signal read errors, such as `Closed` after shutdown.
    pub fn status(&self) -> Result<CommandPaletteStatus, RuntimeError> {
        Ok(self.pick.status()?.into())
    }
    /// The receipt of the command most recently chosen in this palette, kept
    /// after the palette closes. Its binding reports the typed outcome, so an
    /// application can react to it as well. `None` before the first choice.
    ///
    /// # Errors
    /// Returns the signal read errors, such as `Closed` after shutdown.
    pub fn receipt(&self) -> Result<Option<CommandReceipt>, RuntimeError> {
        self.last.get()
    }
    /// Invokes the command `item` stands for and reports the choice to the
    /// picker: pending on the receipt, or refused when it could not be queued.
    fn choose(
        &self,
        cx: &EventContext,
        item: &PickItem,
        text: &OutcomeText,
    ) -> Result<PickChoice, RuntimeError> {
        let (id, name, context) = {
            let session = self.session.borrow();
            let Some(session) = session.as_ref() else {
                return Ok(PickChoice::Done);
            };
            let Some(row) = session.entry(&item.key) else {
                return Ok(PickChoice::Done);
            };
            (row.id.clone(), row.name.clone(), session.context.clone())
        };
        let strings = cx.strings();
        match cx.invoke_command(&id, context) {
            Ok(receipt) => {
                self.last.set(Some(receipt.clone()))?;
                let text = text.clone();
                let table = strings.clone();
                let pending =
                    PickPending::new(receipt.status_binding(), move |status| match status {
                        CommandStatus::Pending => None,
                        CommandStatus::Finished(Ok(CommandOutcome::Invoked))
                        | CommandStatus::Cancelled => Some(Ok(())),
                        CommandStatus::Finished(result) => Some(Err(text(&table, &name, result))),
                    });
                Ok(PickChoice::Pending(pending))
            }
            // A full queue is shown like any refusal, and the palette stays open.
            Err(error) => Ok(PickChoice::Refused(text(&strings, &name, &Err(error)))),
        }
    }
}

/// The picker row for the command at `index`: its name, its first shortcut
/// and its state at the end, disabled unless it was enabled when listed.
fn command_item(strings: &crate::UiStrings, index: usize, entry: &CommandEntry) -> PickItem {
    let shortcut = entry.shortcuts.first().map(ToString::to_string);
    let mut detail = shortcut.clone().unwrap_or_default();
    let state = match &entry.enabled {
        Ok(true) => None,
        Ok(false) => Some(strings.text(crate::StringRole::CommandDisabled)),
        Err(_) => Some(strings.text(crate::StringRole::CommandUnavailable)),
    };
    if let Some(state) = state {
        if !detail.is_empty() {
            detail.push_str(" · ");
        }
        detail.push_str(state);
    }
    let mut item = PickItem::new(format!("command-{index}"), entry.name.clone())
        .disabled(entry.enabled != Ok(true));
    if !detail.is_empty() {
        item = item.detail(detail);
    }
    if let Some(shortcut) = shortcut {
        item = item.shortcut(shortcut);
    }
    item
}

/// A searchable overlay that lists the commands an application registered and
/// runs the one the user chooses.
///
/// The palette is a [`QuickPick`] over the session's [`CommandEntry`] rows:
/// it wraps the application's content (`anchor`), which it lays out as its
/// own allocation, and presents over it, when its
/// [`CommandPaletteController`] opens, a panel with a [`SearchField`] and a
/// virtualized list of the session's rows. Each row shows the command's name
/// and its first shortcut, and marks a command that was disabled or whose
/// enabled check failed when the palette opened.
///
/// - **Surface.** Window-modal by default: the panel sits near the top of the
///   window (one control height below its top edge) over a dimmed backdrop,
///   Tab stays inside it, and a press on the backdrop closes it.
///   [`Self::nonmodal`] presents it instead as a nonmodal surface at the top
///   of the anchor, which closes on an outside press and when focus leaves it.
///   Escape, hiding or removing the palette, and window focus loss close it
///   either way. Opening focuses the search field, and closing restores the
///   previously focused component.
/// - **Filtering and ranking** are the [`CommandMatcher`]'s, which is pure;
///   [`Self::matcher`] replaces it. An empty query lists every row in snapshot
///   order.
/// - **Keyboard.** Typing edits the query; Up and Down move the highlight,
///   wrapping at the ends, and Page Up and Page Down move it by
///   [`Self::VISIBLE_ROWS`], all without leaving the field. A new query
///   highlights its best match. Enter invokes the highlighted command. On the
///   native key path Escape first clears a non-empty query and then closes the
///   palette, as [`SearchField`] documents; a pointer press followed by its
///   matching release inside the row invokes it.
/// - **Invocation.** Choosing a command closes the palette and queues the
///   command with the context captured when it opened, through
///   [`EventContext::invoke_command`], which revalidates the context, scope,
///   modality and enabled state when it runs at the next flush. Closing first
///   means a modal palette never blocks its own command, and a destination
///   the command reveals outside the palette is not hidden behind it. A
///   command that ran ends the session. A command that did not, because it
///   became disabled, was blocked, its context went stale or its action
///   failed, reopens the palette with the outcome shown above the list, so no
///   choice is skipped silently; the application reads the same typed result
///   from [`CommandPaletteController::receipt`]. A command action reports a
///   result other than success, such as a destination its reveal could not
///   show, by returning an error, for example an
///   [`ApplicationError`] with the
///   [`RevealOutcome`] described.
///
/// Configuration ([`Self::matcher`], [`Self::placeholder`],
/// [`Self::empty_text`], [`Self::width`],
/// [`Self::nonmodal`]) applies from the next opening when the palette is
/// described again. A failed Pending result reopens the same configuration
/// snapshot. [`Self::outcome_text`] replaces the formatter for subsequent choices,
/// including in an already open session. An already Pending choice keeps the
/// formatter captured when chosen. A reentrant redescription during the choose
/// callback keeps that callback and its formatter, as for QuickPick. The controller
/// is fixed when the palette mounts; a
/// description with a different controller is ignored, so give a palette with
/// another controller a different key.
///
/// Inspection reports a `Dialog` (modal) or `Popover` named "Command palette"
/// with `open` and `status` properties, and the results as a `ListBox` named
/// "Commands" whose items carry each row's name, shortcut, highlight and
/// disabled state.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     CommandPalette, CommandPaletteController, CommandPaletteStatus, CommandReceipt,
///     CommandScope, CommandSource, CommandStatus, CommandOutcome, Element, KeyAction,
///     KeyCode, Label, Runtime,
/// };
/// use std::{cell::Cell, rc::Rc, time::Duration};
/// let mut ui = Runtime::new();
/// let saved = Rc::new(Cell::new(false));
/// let flag = saved.clone();
/// ui.register_command("Save", CommandScope::Application, Vec::new(), move |_, _| {
///     flag.set(true);
///     Ok(())
/// })?;
/// ui.register_command("Show sidebar", CommandScope::Application, Vec::new(), |_, _| Ok(()))?;
/// ui.set_viewport(pert_ui::Viewport::new(800.0, 600.0, 1.0)?)?;
/// let palette = CommandPaletteController::new(&ui);
/// ui.mount_tree(Element::new(
///     "app",
///     CommandPalette::new(palette.clone(), Element::new("doc", Label::new("Document"))),
/// ))?;
/// let context = ui.command_context(CommandSource::Programmatic);
/// palette.open(&mut ui, &context)?;
/// ui.flush(Duration::ZERO)?;
/// // Focus is in the search field: type, then choose the best match.
/// ui.text_input("sav")?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(saved.get());
/// assert_eq!(palette.status()?, CommandPaletteStatus::Closed);
/// let receipt = palette.receipt()?.expect("a command was chosen");
/// assert_eq!(receipt.status(), CommandStatus::Finished(Ok(CommandOutcome::Invoked)));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct CommandPalette {
    inner: QuickPick,
    controller: CommandPaletteController,
    outcome: Rc<RefCell<OutcomeText>>,
}
impl CommandPalette {
    /// The preferred width of the panel, 520 logical pixels like
    /// [`QuickPick::DEFAULT_WIDTH`], narrowed to the window; [`Self::width`]
    /// changes it.
    pub const DEFAULT_WIDTH: Length = QuickPick::DEFAULT_WIDTH;
    /// The number of rows the list shows before it scrolls, and the distance
    /// Page Up and Page Down move the highlight.
    pub const VISIBLE_ROWS: usize = QuickPick::VISIBLE_ROWS;

    /// Wraps `anchor` and presents the palette `controller` opens over it.
    /// The controller is claimed when the palette mounts, not here.
    pub fn new(controller: CommandPaletteController, anchor: Element) -> Self {
        let outcome: Rc<RefCell<OutcomeText>> =
            Rc::new(RefCell::new(Rc::new(default_outcome_text)));
        let (chooser, text) = (controller.clone(), outcome.clone());
        let inner = QuickPick::new(controller.pick.clone(), anchor, move |cx, item| {
            let text = text.borrow().clone();
            chooser.choose(cx, item, &text)
        })
        .name(crate::StringRole::PaletteName)
        .list_name(crate::StringRole::PaletteCommands)
        .placeholder(crate::StringRole::PalettePlaceholder)
        .empty_text(crate::StringRole::PaletteNoMatches);
        Self {
            inner,
            controller,
            outcome,
        }
    }
    /// Presents the palette as a nonmodal surface at the top of the anchor
    /// instead of a window-modal one; see the type documentation.
    pub fn nonmodal(mut self) -> Self {
        self.inner = self.inner.nonmodal();
        self
    }
    /// Replaces the matching and ranking policy, [`CommandMatcher::fuzzy`] by
    /// default.
    pub fn matcher(mut self, matcher: CommandMatcher) -> Self {
        let session = self.controller.session.clone();
        self.inner = self.inner.matcher(PickMatcher::new(move |query, item| {
            let session = session.borrow();
            let entry = session.as_ref()?.entry(&item.key)?;
            matcher.score(query, entry)
        }));
        self
    }
    /// Sets the search field's hint text as a [`TextSource`]: fixed text, a
    /// binding, or a [`StringRole`](crate::StringRole), as
    /// [`QuickPick::placeholder`](crate::QuickPick::placeholder). By default the installed
    /// [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::PalettePlaceholder`](crate::StringRole::PalettePlaceholder),
    /// "Type a command" in English.
    pub fn placeholder(mut self, text: impl Into<TextSource>) -> Self {
        self.inner = self.inner.placeholder(text);
        self
    }
    /// Sets the text shown when no row matches the query, a [`TextSource`] as
    /// for [`Self::placeholder`]. By default the string table supplies
    /// [`StringRole::PaletteNoMatches`](crate::StringRole::PaletteNoMatches),
    /// "No matching commands" in English.
    pub fn empty_text(mut self, text: impl Into<TextSource>) -> Self {
        self.inner = self.inner.empty_text(text);
        self
    }
    /// Replaces how a chosen command that did not run is described, from its
    /// name and invocation result. The default takes its words from the
    /// installed [`UiStrings`](crate::UiStrings) (the `Command…` roles of
    /// [`StringRole`](crate::StringRole)): it names the command and says that
    /// it is not available right now, is blocked by an open dialog or failed
    /// with the error's text. The function must be pure and bounded.
    /// Redescription applies it to subsequent choices; an already Pending choice
    /// retains its captured formatter. Reentrant redescription during the choose
    /// callback retains the active callback's formatter.
    pub fn outcome_text(
        self,
        text: impl Fn(&str, &Result<CommandOutcome, RuntimeError>) -> String + 'static,
    ) -> Self {
        *self.outcome.borrow_mut() = Rc::new(move |_, name, result| text(name, result));
        self
    }
    /// Sets the panel's preferred width, [`Self::DEFAULT_WIDTH`] by default;
    /// it is narrowed to the window. Any width greater than zero is valid;
    /// [`Length`] already excludes non-finite and negative values.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     CommandPalette, CommandPaletteController, Element, GeometryError, Label, Length,
    ///     Runtime,
    /// };
    /// let ui = Runtime::new();
    /// let palette = || {
    ///     let content = Element::new("content", Label::new("Document"));
    ///     CommandPalette::new(CommandPaletteController::new(&ui), content)
    /// };
    /// assert!(palette().width(Length::new(640.0)?).is_ok());
    /// assert!(matches!(palette().width(Length::new(0.0)?), Err(GeometryError::Empty)));
    /// assert!(matches!(Length::new(f32::NAN), Err(GeometryError::NonFinite)));
    /// # Ok::<(), GeometryError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`GeometryError::Empty`] for zero.
    pub fn width(mut self, width: Length) -> Result<Self, GeometryError> {
        self.inner = self.inner.width(width)?;
        Ok(self)
    }
}
/// Every hook forwards to the [`QuickPick`] the palette is built on; only
/// inspection names the status in the palette's own terms.
impl Component for CommandPalette {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if !self.inner.same_controller(&next.inner) {
            return None;
        }
        let invalidation = self.inner.reconcile(next.inner);
        // Accepted descriptions can leave current presentation unchanged.
        *self.outcome.borrow_mut() = next.outcome.borrow().clone();
        invalidation
    }
    fn mounted(&mut self) {
        self.inner.mounted();
    }
    fn unmounted(&mut self) {
        self.inner.unmounted();
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.inner.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.inner.update(cx)
    }
    fn child_visible(&self, key: &str) -> bool {
        self.inner.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.inner.child_surface(key)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.inner.dismiss_surface(cx, reason)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.inner.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.inner.arrange(cx, size)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.inner.keyboard_event(cx, event, keyboard)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = self.inner.inspect();
        let status = CommandPaletteStatus::from(self.inner.presented_status());
        d.properties
            .insert("status".into(), InspectValue::Text(format!("{status:?}")));
        d
    }
}
