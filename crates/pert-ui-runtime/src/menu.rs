//! Action menus: menu rows, button-anchored menus, context menus and the
//! nested submenu levels they share with `MenuBar`. The rows are presented by
//! the option list of [`crate::select`].
use crate::choice_controls::validate;
use crate::layout::rect;
use crate::select::{OptionList, RowGeometry};
use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) type MenuAction = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;
type ReceiptSink = Box<dyn FnMut(&mut EventContext, CommandReceipt) -> Result<(), RuntimeError>>;
/// A settings row's own write: the new checked state, or `None` when it wrote
/// nothing (a radio row whose value was already selected).
type SettingWrite = Box<dyn FnMut() -> Result<Option<bool>, RuntimeError>>;
type ChangeHandler = Box<dyn FnMut(&mut EventContext, bool) -> Result<(), RuntimeError>>;
type MarkRead = Box<dyn Fn(&mut UpdateContext) -> Result<bool, RuntimeError>>;
type ChainClose = Rc<dyn Fn(&mut EventContext) -> Result<(), RuntimeError>>;
type Sideways = Rc<dyn Fn(isize) -> Result<(), RuntimeError>>;

/// How a menu row presents its two-state mark.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MarkStyle {
    /// An independent on/off setting, drawn as a check mark.
    Check,
    /// One of several exclusive settings, drawn as a dot.
    Radio,
}
pub(crate) struct Mark {
    pub(crate) style: MarkStyle,
    pub(crate) read: MarkRead,
}
/// A shortcut shown in a menu row: fixed, or following a command's aliases.
pub(crate) enum Shortcut {
    Fixed(KeyChord),
    Live(ReadSignal<Vec<KeyChord>>),
}
/// Per-row presentation a menu adds to an option: its shortcut and mark.
pub(crate) struct RowExtras {
    pub(crate) shortcut: Option<Shortcut>,
    pub(crate) shown: Option<String>,
    pub(crate) mark: Option<Mark>,
    pub(crate) checked: bool,
}
/// What a menu's list knows beyond its rows: which rows open a submenu, which
/// submenu is open, and each row's shortcut and mark, so inspection can describe
/// commands and branches.
pub(crate) struct MenuRows {
    pub(crate) keys: Vec<String>,
    pub(crate) branches: Vec<String>,
    pub(crate) level: Slot<Rc<LevelState>>,
    pub(crate) extras: Vec<RowExtras>,
    pub(crate) shortcut_text: RefCell<Vec<Option<TextLayout>>>,
}
impl MenuRows {
    pub(crate) fn marked(&self) -> bool {
        self.extras.iter().any(|extra| extra.mark.is_some())
    }
    /// Key of the submenu this list's level shows, read without subscribing.
    pub(crate) fn open_key(&self) -> Option<String> {
        self.level.get().current()
    }
    /// Takes a new description's rows, keeping the mounted level state and the
    /// prepared shortcut text, which the next arrangement refreshes. Returns
    /// whether the rows, as opposed to their callbacks, changed.
    pub(crate) fn adopt(&mut self, next: Self) -> bool {
        let Self {
            keys,
            branches,
            level: _,
            extras,
            shortcut_text: _,
        } = next;
        let changed = self.keys != keys || self.branches != branches;
        self.keys = keys;
        self.branches = branches;
        self.extras = extras;
        changed
    }
}

/// A value the callbacks of one menu description share, which reconciliation
/// points at the mounted menu's value.
///
/// Every component of a menu takes the callbacks of a new description when it
/// is reconciled, while the mounted menu keeps its open state. Callbacks
/// therefore reach state through a slot: the mounted owner points the new
/// description's slot at its own state, so callbacks built by any description
/// act on the one mounted menu.
pub(crate) struct Slot<T>(Rc<RefCell<T>>);
impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Clone> Slot<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }
    pub(crate) fn get(&self) -> T {
        self.0.borrow().clone()
    }
    pub(crate) fn set(&self, value: T) {
        *self.0.borrow_mut() = value;
    }
}

/// Which submenu one menu level shows.
///
/// Keyed rather than positional, so an open submenu survives rows being added,
/// removed or reordered around its row.
pub(crate) struct LevelState {
    open: Signal<Option<String>>,
    /// An open key whose row a description removed. Writes are not permitted
    /// during reconciliation, so the key stays in `open` but counts as closed
    /// until the next write, and a row re-added under it does not reopen.
    closed: RefCell<Option<String>>,
}
impl LevelState {
    fn new<S>(like: &Signal<S>) -> Rc<Self> {
        Rc::new(Self {
            open: like.sibling(None),
            closed: RefCell::new(None),
        })
    }
    /// The shown key among `open`, a value of the open-state signal.
    fn shown(&self, open: Option<String>) -> Option<String> {
        open.filter(|key| self.closed.borrow().as_ref() != Some(key))
    }
    /// The shown key, read without subscribing.
    fn current(&self) -> Option<String> {
        self.shown(self.open.get().ok().flatten())
    }
    /// Opens the submenu keyed `key`, or closes the open one.
    fn set_open(&self, key: Option<String>) -> Result<(), RuntimeError> {
        let stale = self.closed.borrow_mut().take();
        if stale.is_some() && stale == key {
            // The signal still holds this key; clear it first so the level is
            // notified that its submenu opens.
            self.open.set(None)?;
        }
        self.open.set(key)
    }
    /// Closes, until the next write, an open submenu whose row is not among
    /// `branches`.
    fn retain(&self, branches: &[String]) {
        if let Some(open) = self.current()
            && !branches.contains(&open)
        {
            *self.closed.borrow_mut() = Some(open);
        }
    }
}
/// What a menu row does when it is chosen.
pub(crate) enum LeafAction {
    /// Runs application code directly.
    Run(MenuAction),
    /// Queues a registered command with the context captured when the menu opened.
    Command { id: CommandId, receipt: ReceiptSink },
    /// Writes a toggle or radio row's setting, then reports a write.
    Setting {
        write: SettingWrite,
        on_change: Option<ChangeHandler>,
    },
}
impl LeafAction {
    /// Runs the action once. A command uses the context captured when its menu
    /// opened, so focus moving into the menu does not change its target.
    pub(crate) fn run(
        &mut self,
        captured: &RefCell<Option<CommandContext>>,
        cx: &mut EventContext,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Run(action) => action(cx),
            Self::Setting { write, on_change } => match (write()?, on_change) {
                (Some(checked), Some(handler)) => handler(cx, checked),
                _ => Ok(()),
            },
            Self::Command { id, receipt } => {
                let captured = captured.borrow().clone();
                let context = captured.unwrap_or_else(|| cx.command_context(CommandSource::Menu));
                let issued = cx.invoke_command(id, context)?;
                receipt(cx, issued)
            }
        }
    }
}

/// One stable row of a menu: an action, a nested submenu, an on/off or
/// one-of-several setting, or a separator.
///
/// Actions run synchronously on the UI thread and return runtime errors to the
/// caller; the whole menu chain closes before an action runs. The key identifies
/// the row independently of its label and must be unique among its siblings.
///
/// A row can show a keyboard shortcut in a right-aligned column. The shortcut
/// is presentation only: [`Self::shortcut`] shows a chord that the application
/// binds elsewhere, and [`Self::command`] shows the live chords of a registered
/// command, which the command routing already handles. Settings rows draw a
/// check mark ([`Self::toggle`], [`Self::checked_signal`]) or a dot
/// ([`Self::radio`]) in a mark column that the whole list reserves once any row
/// has a mark, so labels stay aligned.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Column, Element, KeyChord, Menu, MenuItem, Runtime};
/// let mut ui = Runtime::new();
/// let grid = ui.signal(true);
/// let zoom = ui.signal(100u32);
/// let menu = Menu::new(&ui, "View", vec![
///     MenuItem::new("refresh", "Refresh", |_| Ok(())).shortcut(KeyChord::primary('r')),
///     MenuItem::separator(),
///     MenuItem::toggle("grid", grid.clone(), "Show grid"),
///     MenuItem::separator(),
///     MenuItem::radio("zoom-100", zoom.clone(), 100, "100%"),
///     MenuItem::radio("zoom-200", zoom.clone(), 200, "200%"),
/// ])?;
/// ui.mount_tree(Element::new("root", Column::new([Element::new("view", menu)])))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct MenuItem {
    pub(crate) key: String,
    /// The row's text and its leading icon slot.
    pub(crate) caption: Caption,
    pub(crate) enabled: bool,
    pub(crate) live: Option<crate::Permission>,
    pub(crate) action: LeafAction,
    pub(crate) submenu: Option<Vec<MenuItem>>,
    pub(crate) separator: bool,
    shortcut: Option<Shortcut>,
    mark: Option<Mark>,
}
description_debug!(MenuItem, key = |item| item.key.as_str());
impl MenuItem {
    fn with_action(
        key: impl Into<String>,
        label: impl Into<TextSource>,
        action: LeafAction,
    ) -> Self {
        Self {
            key: key.into(),
            caption: Caption::new(label),
            enabled: true,
            live: None,
            action,
            submenu: None,
            separator: false,
            shortcut: None,
            mark: None,
        }
    }
    /// Creates an enabled action with an application key independent of its label.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    ///
    /// The label is a [`TextSource`]: fixed plain or rich content, or a
    /// binding the row follows while the menu stays retained, such as a label
    /// translated when the application installs other
    /// [`UiStrings`](crate::UiStrings). The list reads a binding in its update,
    /// so a change relayouts the list without polling; a bound row is empty
    /// until that first update. Key validation never reads the label. The same
    /// applies to every constructor's label, and a menu-bar title follows a
    /// bound label too.
    pub fn new(
        key: impl Into<String>,
        label: impl Into<TextSource>,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self::with_action(key, label, LeafAction::Run(Box::new(action)))
    }
    /// Creates a nested action list. Keys are validated within each level by the
    /// menu constructor. Empty/all-disabled branches cannot be activated. Right
    /// opens a branch; Left or Escape closes one level; choosing a leaf closes
    /// the entire chain before invoking its action.
    pub fn submenu(key: impl Into<String>, label: impl Into<TextSource>, items: Vec<Self>) -> Self {
        let mut item = Self::with_action(key, label, LeafAction::Run(Box::new(|_| Ok(()))));
        item.enabled = items.iter().any(MenuItem::may_enable);
        item.submenu = Some(items);
        item
    }
    /// Creates a separator: a thin divider between groups of rows.
    ///
    /// A separator has no key, label or action and is never highlighted or
    /// activated, so the builder methods have no effect on it. Separators at the
    /// start or end of a list, and runs of adjacent separators, collapse, so a
    /// list assembled from optional groups needs no bookkeeping. Inspection
    /// reports no row for a separator.
    pub fn separator() -> Self {
        let mut item = Self::with_action("", "", LeafAction::Run(Box::new(|_| Ok(()))));
        item.enabled = false;
        item.separator = true;
        item
    }
    /// Creates an on/off setting that shows a check mark while `value` is true.
    ///
    /// Choosing it closes the menu and writes the opposite of the current value;
    /// [`Self::on_change`] reports that write. The mark follows `value` without
    /// polling, including writes from elsewhere, and inspection reports it as the
    /// row's checked state. `value` must belong to the runtime that mounts the
    /// menu.
    pub fn toggle(
        key: impl Into<String>,
        value: impl Into<Signal<bool>>,
        label: impl Into<TextSource>,
    ) -> Self {
        let value: Signal<bool> = value.into();
        let read = value.read_only();
        let mut item = Self::with_action(
            key,
            label,
            LeafAction::Setting {
                write: Box::new(move || {
                    let checked = !value.get()?;
                    value.set(checked)?;
                    Ok(Some(checked))
                }),
                on_change: None,
            },
        );
        item.mark = Some(Mark {
            style: MarkStyle::Check,
            read: Box::new(move |cx| cx.read(&read)),
        });
        item
    }
    /// Creates one of several exclusive settings that shows a dot while
    /// `selection` equals `value`.
    ///
    /// Choosing it closes the menu and writes `value`, unless `selection` already
    /// equals it, in which case nothing is written; [`Self::on_change`] reports
    /// the write. Give each alternative its own row over the same `selection`;
    /// separators around the group make it read as one. The dot follows `selection` without polling, and inspection reports
    /// it as the row's checked state. `selection` must belong to the runtime that
    /// mounts the menu.
    pub fn radio<T: Clone + PartialEq + 'static>(
        key: impl Into<String>,
        selection: impl Into<Signal<T>>,
        value: T,
        label: impl Into<TextSource>,
    ) -> Self {
        let selection: Signal<T> = selection.into();
        let read = selection.read_only();
        let chosen = value.clone();
        let mut item = Self::with_action(
            key,
            label,
            LeafAction::Setting {
                write: Box::new(move || {
                    if selection.get()? == value {
                        return Ok(None);
                    }
                    selection.set(value.clone())?;
                    Ok(Some(true))
                }),
                on_change: None,
            },
        );
        item.mark = Some(Mark {
            style: MarkStyle::Radio,
            read: Box::new(move |cx| Ok(cx.read(&read)? == chosen)),
        });
        item
    }
    /// Creates a row that invokes a registered command, presenting its name as the
    /// label and its first chord as the shortcut.
    ///
    /// Timing class of `on_receipt`: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input); the command itself runs at its invocation.
    ///
    /// The shortcut follows [`Runtime::rebind_command`], and a command whose
    /// availability is a simple binding from [`Runtime::command_enabled_signal`]
    /// grays out with it. A contextual predicate from [`Runtime::command_enabled`]
    /// depends on the target and is not known before invocation, so such a row
    /// stays enabled and a refusal arrives as [`CommandOutcome::Disabled`].
    ///
    /// Choosing the row closes the menu and queues the command through
    /// [`EventContext::invoke_command`] with the [`CommandContext`] captured when
    /// the menu opened, so the command acts on the content target and focus that
    /// were current before the menu took focus. A `MenuBar` opened by pointer or
    /// F10, or a `ContextMenu`, captures before anything in the menu is focused; a
    /// `Menu` trigger is a button that takes focus when pressed, so its captured
    /// focus is the trigger while the content target is kept. The next flush
    /// revalidates the context, scope, modality and enabled state before the
    /// command runs once.
    ///
    /// `on_receipt` receives the [`CommandReceipt`] of each invocation on the UI
    /// thread, so a failure or refusal is reported rather than lost;
    /// [`Self::label`] can replace the presented name.
    ///
    /// # Errors
    /// `StaleCommand` if the command is no longer registered.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Column, CommandScope, Element, KeyChord, Menu, MenuItem, Runtime};
    /// let mut ui = Runtime::new();
    /// let save = ui.register_command("Save", CommandScope::Application,
    ///     vec![KeyChord::primary('s')], |_, _| Ok(()))?;
    /// let last = std::rc::Rc::new(std::cell::RefCell::new(None));
    /// let receipts = last.clone();
    /// let file = Menu::new(&ui, "File", vec![MenuItem::command(&ui, "save", &save,
    ///     move |_, receipt| { *receipts.borrow_mut() = Some(receipt); Ok(()) })?])?;
    /// ui.mount_tree(Element::new("root", Column::new([Element::new("file", file)])))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn command(
        ui: &Runtime,
        key: impl Into<String>,
        command: &CommandId,
        on_receipt: impl FnMut(&mut EventContext, CommandReceipt) -> Result<(), RuntimeError> + 'static,
    ) -> Result<Self, RuntimeError> {
        let mut item = Self::with_action(
            key,
            ui.command_name(command)?,
            LeafAction::Command {
                id: command.clone(),
                receipt: Box::new(on_receipt),
            },
        );
        item.shortcut = Some(Shortcut::Live(ui.command_shortcuts(command)?));
        if let Some(enabled) = ui.command_enabled_binding(command)? {
            item = item.enabled(enabled);
        }
        Ok(item)
    }
    /// Reports each successful user write of a [`Self::toggle`] or [`Self::radio`]
    /// row with the row's new checked state: the toggle's new value, or `true`
    /// for a radio row that became selected.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    ///
    /// The handler runs synchronously after the row's own write succeeded, once
    /// the menu has requested its closure, with the event context of the list or
    /// menu-bar title that was activated. A list is hiding by then; work queued
    /// with [`EventContext::defer`](crate::EventContext::defer) runs after it
    /// has closed.
    /// Writes to the signal from elsewhere, choosing a radio row that is already
    /// selected, a refused write and deselected sibling rows report nothing.
    /// A failing handler keeps the write, propagates as the row's error and is not
    /// replayed. The handler belongs to the row's description: a redescribed menu
    /// replaces it, and a row described without one has none. Rows that write
    /// nothing themselves ([`Self::new`], [`Self::command`], submenus and
    /// separators) ignore it; for a setting the application applies itself, such
    /// as the density of a retained [`ThemeManager`](crate::ThemeManager), use
    /// [`Self::new`] with [`Self::checked_signal`] or [`Self::selected_signal`].
    /// Replaces an earlier handler.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Column, Element, KeyAction, KeyCode, Menu, MenuItem, Runtime, Viewport,
    /// };
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// let playing = ui.signal(false);
    /// let forwarded = ui.signal(None);
    /// let engine = forwarded.clone();
    /// let transport = Menu::new(&ui, "Transport", vec![
    ///     MenuItem::toggle("play", playing.clone(), "Play")
    ///         // Forward the user's choice, for example to a producer thread.
    ///         .on_change(move |_, on| engine.set(Some(on))),
    /// ])?;
    /// let root = ui.mount_tree(Element::new("root", Column::new([Element::new("transport", transport)])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let trigger = ui.find_descendant(&root, "trigger")?.ok_or("mounted")?;
    /// ui.focus(&trigger)?;
    /// for _ in 0..2 {
    ///     ui.key_input(KeyCode::Enter, KeyAction::Press)?; // open, then choose
    ///     ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    ///     ui.flush(Duration::ZERO)?;
    /// }
    /// assert_eq!(forwarded.get()?, Some(true));
    /// // The application's own write is not reported.
    /// playing.set(false)?;
    /// assert_eq!(forwarded.get()?, Some(true));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        if let LeafAction::Setting { on_change, .. } = &mut self.action {
            *on_change = Some(Box::new(handler));
        }
        self
    }
    /// Replaces the row's label with a [`TextSource`], retaining the action
    /// key and the leading icon slot; for example a rich or translated name for
    /// a [`Self::command`] row, which otherwise presents the command's name.
    /// It has no effect on a separator.
    pub fn label(mut self, label: impl Into<TextSource>) -> Self {
        if !self.separator {
            self.caption.set_source(label.into());
        }
        self
    }
    /// Replaces the leading icon slot, independently of the label. A
    /// submenu's chevron is kept.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.leading = Some(icon.into());
        self
    }
    /// Shows `chord` in the row's shortcut column, replacing a command's live
    /// shortcut. This is presentation only; the chord runs nothing unless the
    /// application also binds it, for example with [`Runtime::register_command`].
    /// It is presented in the running platform's convention; see [`KeyChord`].
    pub fn shortcut(mut self, chord: KeyChord) -> Self {
        self.shortcut = Some(Shortcut::Fixed(chord));
        self
    }
    /// Shows a check mark while `checked` is true, for an action whose on/off
    /// state lives elsewhere, such as a command that toggles a host setting.
    /// Choosing the row still runs its own action; [`Self::toggle`] is the
    /// shorthand when the action only flips a signal.
    pub fn checked_signal(mut self, checked: impl Into<ReadSignal<bool>>) -> Self {
        let checked: ReadSignal<bool> = checked.into();
        self.mark = Some(Mark {
            style: MarkStyle::Check,
            read: Box::new(move |cx| cx.read(&checked)),
        });
        self
    }
    /// Shows the radio dot while `selected` is true, for one of several exclusive
    /// actions whose selection lives elsewhere, such as a setting the application
    /// or the theme owns.
    ///
    /// Choosing the row still runs its own action, which is responsible for
    /// changing the selection; the dot follows `selected` without polling and
    /// inspection reports it as the row's checked state, as for
    /// [`Self::radio`]. Keeping exactly one row of a group selected is the
    /// caller's contract. `selected` must belong to the runtime that mounts the
    /// menu. This replaces any earlier mark.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Column, Element, Menu, MenuItem, Runtime};
    /// let mut ui = Runtime::new();
    /// // A selection held by the application, not by the menu.
    /// let fast = ui.signal(true);
    /// let slow = ui.signal(false);
    /// let row = |key: &str, label: &str, mine: bool| {
    ///     let (fast, slow) = (fast.clone(), slow.clone());
    ///     let selected = if mine { fast.read_only() } else { slow.read_only() };
    ///     MenuItem::new(key, label, move |_| {
    ///         fast.set(mine)?;
    ///         slow.set(!mine)?;
    ///         Ok(())
    ///     })
    ///     .selected_signal(selected)
    /// };
    /// let speed = Menu::new(&ui, "Speed", vec![row("fast", "Fast", true), row("slow", "Slow", false)])?;
    /// ui.mount_tree(Element::new("root", Column::new([Element::new("speed", speed)])))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn selected_signal(mut self, selected: impl Into<ReadSignal<bool>>) -> Self {
        let selected: ReadSignal<bool> = selected.into();
        self.mark = Some(Mark {
            style: MarkStyle::Radio,
            read: Box::new(move |cx| cx.read(&selected)),
        });
        self
    }
    /// Keeps the action visible but excludes it from navigation and activation.
    /// Calling this after [`Self::enabled`] preserves that fixed denial
    /// regardless of subsequent binding values.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Restricts availability. Repeated calls combine with AND, including the
    /// command's permission and fixed denial from [`Self::disabled`]. Bound
    /// changes update navigation and cancel armed activation without polling.
    /// A permission cannot enable an empty or all-disabled submenu.
    pub fn enabled(mut self, enabled: impl Into<crate::Permission>) -> Self {
        if !self.separator {
            let permission = self.live.take().unwrap_or_default().and(enabled);
            self.enabled &= permission.fixed() != Some(false);
            self.live = Some(permission);
        }
        self
    }
    pub(crate) fn presented_content(&self) -> RichText {
        self.caption.display().into_owned()
    }
    /// Whether this action can ever be activated, used to decide whether a menu or
    /// branch can open at all. An eligible bound action counts, since its value
    /// is not known before the list evaluates; fixed denial and empty/all-disabled
    /// submenu structure still exclude it.
    pub(crate) fn may_enable(&self) -> bool {
        self.enabled
            && self
                .live
                .as_ref()
                .is_none_or(|permission| permission.fixed() != Some(false))
    }
}

/// How one open menu chain closes, which command context its commands use and,
/// inside a menu bar, how Left and Right move to a neighboring menu.
#[derive(Clone)]
pub(crate) struct MenuChain {
    /// Closes the whole chain, before a leaf action runs or when Left leaves the
    /// root; the context belongs to the list that closes it.
    pub(crate) close: ChainClose,
    /// Command context captured when the chain opened; empty until then.
    pub(crate) capture: Rc<RefCell<Option<CommandContext>>>,
    /// Opens the neighboring top-level menu of a bar, -1 for previous and 1 for next.
    pub(crate) sideways: Option<Sideways>,
}
impl MenuChain {
    /// A chain that one open-state signal owns, reached through `open` so a
    /// reconciled owner can point it at its mounted signal.
    pub(crate) fn closing(open: &Slot<Signal<bool>>) -> Self {
        let open = open.clone();
        Self {
            close: Rc::new(move |_| open.get().set(false)),
            capture: Rc::default(),
            sideways: None,
        }
    }
    /// Records the command context before the chain takes focus.
    pub(crate) fn capture(&self, cx: &EventContext) {
        *self.capture.borrow_mut() = Some(cx.command_context(CommandSource::Menu));
    }
    /// Takes over the chain of a new description of the same mounted owner,
    /// pointing its open state at `mounted`. An engagement already under way
    /// keeps the context it captured: the new description's rows read it.
    fn adopt(&mut self, next: Self, surface: &Slot<Signal<bool>>, mounted: &Signal<bool>) {
        surface.set(mounted.clone());
        next.capture.borrow_mut().clone_from(&self.capture.borrow());
        *self = next;
    }
}

/// Builds the framed first level of a menu chain, the panel every menu surface uses.
pub(crate) fn menu_panel<S>(
    like: &Signal<S>,
    items: Vec<MenuItem>,
    chain: MenuChain,
) -> Result<Container, ChoiceError> {
    validate_menu(&items)?;
    let branch = MenuBranch::new(like, items, chain, None)?;
    Ok(
        crate::popover::transient_panel(Element::new("actions", branch).structural())
            .padding(MetricRole::ControlPaddingX),
    )
}

/// Builds the popup element of a menu owned by a widget rather than by a consumer.
///
/// `open` is the widget's own open-state signal, which also supplies the runtime
/// for the menu's internal state. The result is the same rounded Surface panel
/// with a PanelBorder outline that every other transient surface uses, so a
/// built-in menu is reproducible through `ContextMenu` and the public `MenuItem`.
pub(crate) fn owned_menu(
    open: &Signal<bool>,
    items: Vec<MenuItem>,
) -> Result<Element, ChoiceError> {
    let chain = MenuChain::closing(&Slot::new(open.clone()));
    Ok(Element::new("menu", menu_panel(open, items, chain)?)
        .structural()
        .expose_part("clipboard"))
}

type Opener = Rc<dyn Fn(&EventContext) -> Result<(), RuntimeError>>;

/// A button-anchored action menu with arrow navigation and shared popup dismissal.
///
/// Menus retain keyed actions, skip disabled entries, scroll long lists and
/// close on activation, Escape, outside press or owner context loss. Every level,
/// including each submenu, is a rounded Surface panel with a PanelBorder outline. Empty/all-disabled
/// menus have a disabled trigger. A framed trigger takes [`Button`]'s neutral
/// inactive frame; an icon trigger remains frameless with muted content.
/// Nested lists use `MenuItem::submenu`, with Right
/// to open and Left/Escape to return. Rows may carry separators, shortcuts and
/// check or radio marks; see [`MenuItem`]. No native menus or implicit I/O are
/// used. For an application's File, Edit and Help menus, see `MenuBar`.
///
/// The highlighted row is the one Enter runs, and the keyboard and the pointer
/// take turns owning it at every level. The keyboard takes it with each
/// handled press of Up, Down, Home, End or a Page key, including each OS
/// repeat of one, and with a Right or Left press, before it acts. Key releases,
/// navigation keys with modifiers and keys the menu ignores leave it with
/// whichever input owns it, so actual pointer motion takes it back even while
/// a navigation key is still held, until that key's next repeat. The pointer
/// takes it by actual motion over the rows, a primary press on a row or a
/// wheel that scrolls them; motion over a separator or a disabled row takes
/// it without highlighting that row. From then on the highlight follows the
/// row under the pointer as rows change or scroll.
///
/// An Enter or Space press that arms activation takes the highlight for the
/// keyboard as well. Until that key's release or a cancellation, such as
/// Escape dismissing the menu, pointer motion, presses and wheels only hover:
/// only the keyboard moves the highlight, and the release runs the row
/// highlighted then. While a primary pointer press is armed, Enter and Space
/// neither arm nor take the highlight; the pointer keeps it, and only its own
/// release on the pressed row runs that row.
///
/// A list shown beneath a stationary pointer, as a menu opened by key over
/// where the pointer rests, keeps its first row until the pointer moves or
/// acts, and rows that keyboard navigation scrolls beneath it do not take the
/// highlight either. Motion means the host reporting a window position other
/// than the last one the list saw, compared exactly in logical coordinates: a
/// repeated report of the same position, including the pointer re-entering
/// the window exactly where it left, is stationary until the next distinct
/// motion. After the window's scale factor changes, the first native report
/// of an unmoved pointer can carry new logical coordinates and then counts as
/// motion.
///
/// Reconciling a new description under the same key, for example one whose
/// "Recent files" submenu gained rows, takes its rows, their actions and the
/// trigger. Rows are matched by item key at every level: a surviving row keeps
/// its highlight and an open submenu stays open, a new row appears and a
/// removed row disappears, closing its submenu if that was open; a highlight
/// the pointer owns moves to the row now under it. Whether the
/// menu is open and the command context captured when it opened are kept, except
/// when an explicitly supplied open binding is replaced. That replacement follows
/// its current value and starts without the previous engagement's command context.
///
/// # Inspection and tests
///
/// Use [`Driver::exposed_part`](crate::testing::Driver::exposed_part) to find
/// `trigger`, the actual Button even for an icon menu, and `items`, the own
/// level's Menu list. Click the trigger to open it, then
/// [`Driver::click_item`](crate::testing::Driver::click_item) on a visible row.
/// Item order excludes separators; disabled rows run no command. Rows report
/// visible pointer rectangles, with offscreen rows absent. Wheel or keyboard
/// navigation scrolls them. Querying or revealing a closed list does not open it.
/// A submenu row expands its panel: query `submenu/<row-key>` in this scope,
/// then `items` in that panel's scope for the nested list. The slash is literal;
/// stable keys, rather than captions, select these parts. Application/workspace
/// addresses and semantic naming are independent of these exposed names.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
/// let count = ui.signal(0_u32);
/// let chosen = count.clone();
/// let root = ui.mount_tree(Column::new([
///     Menu::new(&ui, "Actions", vec![
///         MenuItem::new("run", "Run", move |_| chosen.update(|n| *n += 1)),
///     ])?.key("actions"),
/// ]).key("root"))?;
/// let mut driver = Driver::new(&mut ui);
/// driver.settle()?;
/// let menu = driver.find(&root, &["actions"])?;
/// let trigger = driver.exposed_part(&menu, "trigger")?;
/// let items = driver.exposed_part(&menu, "items")?;
/// driver.click(&trigger)?;
/// driver.click_item(&items, 0)?;
/// assert_eq!(count.get()?, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Menu {
    popover: Popover,
    /// The open state the trigger and the rows of this description reach.
    surface: Slot<Signal<bool>>,
    /// The mounted open state, which the popover reads.
    open: Signal<bool>,
    /// The description supplies an explicit binding rather than a disposable
    /// convenience signal. Only an explicit replacement changes mounted state.
    external_open: bool,
    chain: MenuChain,
}
description_debug!(Menu);
impl Menu {
    /// Creates a closed action menu opened by a button showing `label`, a
    /// [`TextSource`]: fixed plain or rich content, or a binding the trigger
    /// follows. `ui` owns the internal open-state signal and all callbacks run
    /// on its thread.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns [`ChoiceError::EmptyValue`] for an empty or whitespace-only action key and
    /// [`ChoiceError::DuplicateValue`] for two sibling items with the same key, at every
    /// submenu level.
    pub fn new(
        ui: &impl crate::StateOwner,
        label: impl Into<TextSource>,
        items: Vec<MenuItem>,
    ) -> Result<Self, ChoiceError> {
        Self::with_state(ui.signal(false), label, items).map(Self::own_open)
    }
    /// [`Self::new`] with its open state supplied as a signal, for a component
    /// that keeps or shares the open state itself, for example to close the
    /// menu from elsewhere or to keep it open across descriptions. A component
    /// that only builds the menu from [`crate::Component::update`] can call
    /// [`Self::new`] with its [`crate::UpdateContext`] instead; to own the
    /// state, create the signal with [`crate::UpdateContext::signal`].
    ///
    /// The menu reads and writes `open` as its own open state: `true` while
    /// its list is shown. Create it closed (`false`), give each menu its own
    /// signal, and keep the signal for as long as the menu should stay open
    /// across descriptions. A replacement supplied signal becomes the mounted
    /// open state from the next update; initialize it to `false` to start closed.
    /// Its existing value is never overwritten merely to initialize the menu.
    /// The old signal is no longer read or written, and its captured command
    /// context is discarded. Reusing the same signal keeps the current engagement.
    /// Writing `false` from the application closes an open menu.
    /// Keys, errors and lifecycle match [`Self::new`].
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     Component, Element, Menu, MenuItem, Runtime, RuntimeError, Signal, UpdateContext,
    /// };
    ///
    /// /// A custom component that describes an actions menu from its update.
    /// #[derive(Default)]
    /// struct Actions {
    ///     open: Option<Signal<bool>>,
    /// }
    /// impl Component for Actions {
    ///     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
    ///         if self.open.is_none() {
    ///             let open = cx.signal(false);
    ///             let rows = vec![MenuItem::new("rename", "Rename", |_| Ok(()))];
    ///             let menu = Menu::with_state(open.clone(), "Actions", rows)?;
    ///             self.open = Some(open);
    ///             cx.request_children(vec![Element::new("menu", menu)]);
    ///         }
    ///         Ok(())
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new("actions", Actions::default()))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// assert!(ui.inspect().components.iter().any(|item| item.key == "menu"));
    /// # Ok::<(), RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`ChoiceError::EmptyValue`] for an empty or whitespace-only action key and
    /// [`ChoiceError::DuplicateValue`] for two sibling items with the same key, at every
    /// submenu level.
    pub fn with_state(
        open: impl Into<Signal<bool>>,
        label: impl Into<TextSource>,
        items: Vec<MenuItem>,
    ) -> Result<Self, ChoiceError> {
        let open: Signal<bool> = open.into();
        let label = label.into();
        Self::with_trigger(open, items, move |open, enabled| {
            let button = Button::new(label, move |cx| open(cx));
            Element::new("trigger", if enabled { button } else { button.disabled() })
                .structural()
                .expose_part("trigger")
        })
    }
    /// Creates a menu opened by a frameless icon button with a readable action
    /// name and tooltip. It uses ordinary Button padding, hover and focus behavior,
    /// without a resting fill, for placement in title bars/toolbars. Disabled-only
    /// menus cannot open. Keys and lifecycle match [`Self::new`].
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns [`ChoiceError::EmptyValue`] for an empty or whitespace-only action key and
    /// [`ChoiceError::DuplicateValue`] for two sibling items with the same key, at every
    /// submenu level.
    pub fn icon(
        ui: &impl crate::StateOwner,
        icon: impl Into<IconSource>,
        name: impl Into<TextSource>,
        items: Vec<MenuItem>,
    ) -> Result<Self, ChoiceError> {
        Self::icon_with_state(ui.signal(false), icon, name, items).map(Self::own_open)
    }
    /// [`Self::icon`] with its open state supplied as a signal; see
    /// [`Self::with_state`] for the signal's rules.
    ///
    /// # Errors
    /// Returns [`ChoiceError::EmptyValue`] for an empty or whitespace-only action key and
    /// [`ChoiceError::DuplicateValue`] for two sibling items with the same key, at every
    /// submenu level.
    pub fn icon_with_state(
        open: impl Into<Signal<bool>>,
        icon: impl Into<IconSource>,
        name: impl Into<TextSource>,
        items: Vec<MenuItem>,
    ) -> Result<Self, ChoiceError> {
        let open: Signal<bool> = open.into();
        let icon = icon.into();
        let name: TextSource = name.into();
        Self::with_trigger(open, items, move |open, enabled| {
            let button = Button::icon(icon, name.clone(), move |cx| open(cx)).frameless();
            Element::new(
                "trigger",
                Tooltip::new(
                    Element::new("button", if enabled { button } else { button.disabled() })
                        .structural()
                        .expose_part("trigger"),
                    name,
                ),
            )
            .structural()
        })
    }
    fn own_open(mut self) -> Self {
        self.external_open = false;
        self
    }
    fn with_trigger(
        open: Signal<bool>,
        items: Vec<MenuItem>,
        trigger: impl FnOnce(Opener, bool) -> Element,
    ) -> Result<Self, ChoiceError> {
        validate_menu(&items)?;
        let surface = Slot::new(open.clone());
        let chain = MenuChain::closing(&surface);
        let opener: Opener = {
            let (chain, surface) = (chain.clone(), surface.clone());
            Rc::new(move |cx| {
                chain.capture(cx);
                surface.get().set(true)
            })
        };
        let anchor = trigger(opener, items.iter().any(MenuItem::may_enable));
        let branch = MenuBranch::new(&open, items, chain.clone(), None)?;
        let mut popover = Popover::empty(open.clone());
        popover.set_list_content(anchor, Element::new("actions", branch).structural());
        Ok(Self {
            popover,
            surface,
            open,
            external_open: true,
            chain,
        })
    }
}
impl Component for Menu {
    /// Points the new description's trigger and rows at the retained or explicitly
    /// replaced open state. Only replacement discards the captured engagement.
    /// The trigger and every level are reconciled children.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            popover,
            surface,
            open,
            external_open,
            chain,
        } = next;
        let replaced = external_open && !self.open.read_only().same_source(&open.read_only());
        let invalidation = if replaced {
            self.open = open;
            // An explicitly replaced binding starts a different engagement.
            // New rows must not inherit the old opener's command context.
            self.chain.capture.borrow_mut().take();
            self.popover.reconcile(popover)
        } else {
            None
        };
        self.chain.adopt(chain, &surface, &self.open);
        self.surface = surface;
        self.external_open = external_open;
        invalidation
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.popover.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.popover.update(cx)
    }
    fn child_visible(&self, key: &str) -> bool {
        self.popover.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.popover.child_surface(key)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        r: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.popover.dismiss_surface(cx, r)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.popover.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.popover.arrange(cx, s)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        menu_details(self.popover.inspect())
    }
}

// A menu is a popover whose content is commands, so only the role differs.
fn menu_details(mut details: ComponentDetails) -> ComponentDetails {
    if let Some(semantics) = details.semantics.as_mut() {
        semantics.role = SemanticRole::Menu;
    }
    details
}

/// Rejects empty or duplicate keys among the rows of every level; separators
/// have no key and are skipped.
pub(crate) fn validate_menu(items: &[MenuItem]) -> Result<(), ChoiceError> {
    let rows = || items.iter().filter(|item| !item.separator);
    let choices: Vec<_> = rows()
        .map(|i| Choice::new(i.key.clone(), i.caption.text()))
        .collect();
    validate(&choices)?;
    for item in rows() {
        if item.key.trim().is_empty() {
            return Err(ChoiceError::EmptyValue);
        }
        if let Some(children) = &item.submenu {
            validate_menu(children)?;
        }
    }
    Ok(())
}
/// One level of a menu chain: its list of rows and a submenu panel per row that
/// opens one, of which at most one is shown.
struct MenuBranch {
    children: Vec<Element>,
    /// Which submenu this level shows, shared with its list's callbacks and
    /// with the levels nested below it.
    level: Slot<Rc<LevelState>>,
    shown: Option<String>,
    keys: Vec<String>,
    branches: Vec<String>,
    geometry: Rc<RowGeometry>,
}
impl MenuBranch {
    /// Takes a signal rather than a `Runtime`, so a built-in widget that only
    /// receives a binding can own a menu with the same machinery.
    fn new<S>(
        like: &Signal<S>,
        items: Vec<MenuItem>,
        chain: MenuChain,
        parent: Option<Slot<Rc<LevelState>>>,
    ) -> Result<Self, ChoiceError> {
        let level = Slot::new(LevelState::new(like));
        // Separators become bands before rows; leading, trailing and repeated
        // separators collapse.
        let mut rows = Vec::new();
        let mut separators = Vec::new();
        let mut pending = false;
        for item in items {
            if item.separator {
                pending = !rows.is_empty();
                continue;
            }
            if std::mem::take(&mut pending) {
                separators.push(rows.len());
            }
            rows.push(item);
        }
        let mut children = Vec::new();
        let mut options = Vec::new();
        let mut extras = Vec::new();
        let mut leaves = Vec::new();
        let mut branches = Vec::new();
        let keys: Vec<String> = rows.iter().map(|i| i.key.clone()).collect();
        let mut enablement = Vec::new();
        let mut labels = Vec::new();
        for mut item in rows {
            let content = if item.submenu.is_some() {
                item.presented_content().trailing_icon(Icon::ChevronRight)
            } else {
                item.presented_content()
            };
            let choice = Choice::from_rich(item.key.clone(), content);
            if item.enabled
                && let Some(live) = item.live.clone()
            {
                enablement.push((options.len(), live));
            }
            if item.caption.source().is_bound() {
                labels.push(crate::select::LiveLabel {
                    index: options.len(),
                    text: item.caption.source().clone(),
                    leading: item.caption.leading.clone(),
                    submenu: item.submenu.is_some(),
                    read: None,
                });
            }
            options.push(if item.enabled {
                choice
            } else {
                choice.disabled()
            });
            extras.push(RowExtras {
                shortcut: item.shortcut.take(),
                shown: None,
                mark: item.mark.take(),
                checked: false,
            });
            if let Some(nested) = item.submenu.take() {
                branches.push(item.key.clone());
                children.push(
                    Element::new(
                        format!("submenu/{}", item.key),
                        crate::popover::transient_panel(
                            Element::new(
                                "branch",
                                Self::new(like, nested, chain.clone(), Some(level.clone()))?,
                            )
                            .structural(),
                        )
                        .padding(MetricRole::ControlPaddingX),
                    )
                    .structural()
                    .expose_part(format!("submenu/{}", item.key)),
                );
            } else {
                leaves.push(item);
            }
        }
        let choose_level = level.clone();
        let right_level = level.clone();
        let choose_branches = branches.clone();
        let right_branches = branches.clone();
        let MenuChain {
            close,
            capture,
            sideways,
        } = chain;
        let close_left = close.clone();
        let sideways_left = sideways.clone();
        let mut list = OptionList::with_separators(
            options,
            None,
            Box::new(move |key: &String, cx| {
                if choose_branches.contains(key) {
                    choose_level.get().set_open(Some(key.clone()))?;
                } else if let Some(item) = leaves
                    .iter_mut()
                    .find(|i| &i.key == key && i.enabled)
                    // A bound action is activated only while its binding allows it,
                    // so a stale list cannot run a grayed-out action.
                    .filter(|i| i.live.as_ref().is_none_or(|l| l.get() == Ok(true)))
                {
                    close(cx)?;
                    item.action.run(&capture, cx)?;
                }
                Ok(())
            }),
            separators,
        );
        list.on_right = Some(Box::new(move |key, _| {
            if right_branches.contains(key) {
                right_level.get().set_open(Some(key.clone()))?;
            } else if let Some(sideways) = &sideways {
                sideways(1)?;
            }
            Ok(())
        }));
        list.on_left = Some(Box::new(move |cx| {
            if let Some(parent) = &parent {
                parent.get().set_open(None)?;
            } else if let Some(sideways) = &sideways_left {
                sideways(-1)?;
            } else {
                close_left(cx)?;
            }
            Ok(())
        }));
        list.enablement = enablement;
        list.labels = labels;
        list.menu = Some(MenuRows {
            keys: keys.clone(),
            branches: branches.clone(),
            level: level.clone(),
            extras,
            shortcut_text: RefCell::new(Vec::new()),
        });
        let geometry = list.geometry.clone();
        children.insert(
            0,
            Element::new("items", list)
                .structural()
                .expose_part("items"),
        );
        Ok(Self {
            children,
            level,
            shown: None,
            keys,
            branches,
            geometry,
        })
    }
}
impl Component for MenuBranch {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    /// Points the new description's callbacks at this level's open submenu and
    /// takes its rows. The list and the submenu panels are reconciled children,
    /// matched by row key; a removed row's open submenu closes.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            level,
            shown: _,
            keys,
            branches,
            geometry: _,
        } = next;
        let state = self.level.get();
        state.retain(&branches);
        level.set(state);
        let changed = self.keys != keys || self.branches != branches;
        self.keys = keys;
        self.branches = branches;
        // The shown submenu is read again in the update this schedules.
        changed.then_some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let state = self.level.get();
        let open = cx.read(&state.open)?;
        self.shown = state.shown(open);
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key == "items"
            || self
                .shown
                .as_ref()
                .is_some_and(|s| key.strip_prefix("submenu/") == Some(s))
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key != "items").then_some(SurfaceKind::Submenu)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.level.get().set_open(None)?;
        self.shown = None;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(size))?;
        // The rows are shown at this size, so a reveal the list asked for is
        // applied here, before an open submenu is placed from the offset.
        self.geometry.settle(size.height());
        if cx.len() > 1 {
            let index = self
                .shown
                .as_ref()
                .and_then(|key| self.keys.iter().position(|k| k == key))
                .unwrap_or(0);
            let v = cx.measurement().viewport();
            let s = cx.measure(1, Constraints::new(Some(v.width()), Some(v.height()))?)?;
            cx.place(
                1,
                rect(
                    size.width(),
                    (self.geometry.top(index) - self.geometry.offset.get()).max(0.0),
                    s.width(),
                    s.height(),
                )?,
            )?;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
}

/// A right-click, Menu key or Shift+F10 action menu around arbitrary owner
/// content.
///
/// Secondary press opens at the pointer; the ContextMenu (Menu) key or
/// Shift+F10 ([`KeyEvent::requests_context_menu`]) opens below the owner
/// content. Placement is constrained to the window, with scrollable lists and
/// nested `MenuItem::submenu` navigation. Input must reach this wrapper through
/// normal bubbling; children can intentionally consume their own context gesture.
/// The wrapper itself takes no focus and is no Tab stop, so these keys reach it
/// from focused content inside it; content without a focusable component offers
/// the menu to the pointer only. Application code opens it through
/// [`Self::controller`], for a toolbar button or a command that offers the same
/// actions. Focus, dismissal, disabled actions, errors and which input owns
/// the highlighted row follow [`Menu`], and so does reconciling a new
/// description; a menu opened at the pointer keeps its first row until the
/// pointer moves or acts on it. All callbacks
/// run on the owning UI thread. No native OS menu is created.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     ContextMenu, Element, KeyAction, KeyCode, Label, MenuItem, Point, PointerButton,
///     PointerEvent, PointerKind, Runtime, Viewport,
/// };
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
/// let copied = ui.signal(false);
/// let copy = copied.clone();
/// let menu = ContextMenu::new(
///     &ui,
///     Element::new("text", Label::new("Right-click me")),
///     vec![MenuItem::new("copy", "Copy", move |_| copy.set(true))],
/// )?;
/// ui.mount_tree(Element::new("area", menu))?;
/// ui.flush(Duration::ZERO)?;
/// let at = Point::new(10.0, 10.0)?;
/// for kind in [
///     PointerKind::Press(PointerButton::Secondary),
///     PointerKind::Release(PointerButton::Secondary),
/// ] {
///     ui.pointer_input(PointerEvent::new(at, kind))?;
/// }
/// // The open menu takes focus; Enter runs the highlighted first row.
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// assert!(copied.get()?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// # Inspection and tests
///
/// This composite exposes `items` and `submenu/<row-key>` with the same scopes
/// and pointer geometry as [`Menu`]. The wrapped application content retains
/// its own key path. Open with a context-menu request or the controller before
/// clicking rows; finding a hidden part does not open the menu. Editors that
/// own clipboard menus expose a `clipboard` scope, then that scope's `items`.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::testing::*;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
/// let count = ui.signal(0_u32);
/// let chosen = count.clone();
/// let root = ui.mount_tree(Column::new([
///     ContextMenu::new(&ui, Button::new("Target", |_| Ok(())).key("target"),
///         vec![MenuItem::new("run", "Run", move |_| chosen.update(|n| *n += 1))])?
///         .key("context"),
/// ]).key("root"))?;
/// let mut driver = Driver::new(&mut ui);
/// driver.settle()?;
/// let context = driver.find(&root, &["context"])?;
/// let target = driver.find(&context, &["target"])?;
/// driver.runtime_mut().focus(&target)?;
/// driver.key(KeyCode::ContextMenu)?;
/// let items = driver.exposed_part(&context, "items")?;
/// driver.click_item(&items, 0)?;
/// assert_eq!(count.get()?, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct ContextMenu {
    popover: Popover,
    open: Signal<bool>,
    /// The open state the rows of this description reach.
    surface: Slot<Signal<bool>>,
    chain: MenuChain,
    position: Option<Point>,
    enabled: bool,
    /// Whether any row can run, for controllers. Plain shared state, not a
    /// signal: reconciliation updates it, and only an opener's event-time read
    /// consults it, so nothing subscribes to it.
    runnable: Rc<std::cell::Cell<bool>>,
    /// The last request a controller made; the menu applies each once.
    request: Signal<Option<OpenRequest>>,
    seen_request: u64,
    /// Whether the menu is shown, for controllers: true until a mount reports
    /// hiding, false once removed, published from the lifecycle hooks as
    /// library-owned state.
    visible_signal: Signal<bool>,
    /// The mounted menu's signals, which the controllers of this description
    /// reach; a reconciled description points its slot at the mounted one.
    opener: Slot<OpenerLink>,
}
description_debug!(ContextMenu);

/// What a controller asked for: where to open, and the context the rows run in.
#[derive(Clone)]
struct OpenRequest {
    /// Distinguishes requests, since a context compares by nothing else.
    serial: u64,
    position: Option<Point>,
    context: CommandContext,
}
impl PartialEq for OpenRequest {
    fn eq(&self, other: &Self) -> bool {
        self.serial == other.serial
    }
}
/// The mounted menu's state a controller writes.
#[derive(Clone)]
struct OpenerLink {
    open: Signal<bool>,
    request: Signal<Option<OpenRequest>>,
    runnable: Rc<std::cell::Cell<bool>>,
    visible: Signal<bool>,
}

/// Opens a mounted [`ContextMenu`] from application code.
///
/// Obtained with [`ContextMenu::controller`] before the menu is mounted, from
/// any description of it: every controller of a mounted menu reaches the
/// mounted menu, including controllers taken from later descriptions of the
/// same key. Clones share the menu.
#[derive(Clone)]
#[must_use]
pub struct ContextMenuController {
    link: Slot<OpenerLink>,
}

/// Shows nothing of the menu it reaches, which may not be mounted yet.
impl std::fmt::Debug for ContextMenuController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContextMenuController")
            .finish_non_exhaustive()
    }
}
impl ContextMenuController {
    /// Opens the menu at `position`, in the owner content's local coordinates
    /// as a secondary press is, or below the content for `None`, as the Menu
    /// key does. The rows run in `context`, captured by the caller with
    /// [`Runtime::command_context`] or [`EventContext::command_context`], so
    /// a command the menu offers acts on the content that was current when it
    /// was opened. The menu opens at the next flush through the surface
    /// protocol the gestures use: an anchor with no space closes it again
    /// with [`SurfaceDismissal::NoSpace`], and Escape, outside presses, focus
    /// loss and hiding dismiss it as usual. Opening an open menu moves it to
    /// the new position and replaces its context.
    ///
    /// Returns `false` without opening when no row can run, as the gestures
    /// then do nothing, and while the mounted menu is hidden or once it has
    /// been removed, since a request then has no owner to present it and must
    /// not open the menu later when the owner is shown again; `true` once the
    /// request is recorded. A request made before the menu is mounted opens it
    /// when it mounts.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     CommandSource, ContextMenu, Element, KeyAction, KeyCode, Label, MenuItem, Point,
    ///     Runtime, Viewport,
    /// };
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let copied = ui.signal(false);
    /// let copy = copied.clone();
    /// let menu = ContextMenu::new(
    ///     &ui,
    ///     Element::new("text", Label::new("Content")),
    ///     vec![MenuItem::new("copy", "Copy", move |_| copy.set(true))],
    /// )?;
    /// let controller = menu.controller();
    /// ui.mount_tree(Element::new("area", menu))?;
    /// ui.flush(Duration::ZERO)?;
    /// let context = ui.command_context(CommandSource::Menu);
    /// assert!(controller.open(Some(Point::new(10.0, 10.0)?), &context)?);
    /// ui.flush(Duration::ZERO)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    /// assert!(copied.get()?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// The errors of the menu's signal reads and writes, those of
    /// [`Signal::set`](crate::Signal::set): `Closed` after the runtime shut
    /// down; `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` from inside that phase;
    /// or `BorrowConflict`. A refused call opens nothing.
    pub fn open(
        &self,
        position: Option<Point>,
        context: &CommandContext,
    ) -> Result<bool, RuntimeError> {
        let link = self.link.get();
        if !link.runnable.get() || !link.visible.get()? {
            return Ok(false);
        }
        let serial = link
            .request
            .get()?
            .map_or(0, |request| request.serial)
            .wrapping_add(1);
        link.request.set(Some(OpenRequest {
            serial,
            position,
            context: context.clone(),
        }))?;
        link.open.set(true)?;
        Ok(true)
    }
}
impl ContextMenu {
    /// Clones this description's controller, which opens the menu from
    /// application code; see [`ContextMenuController::open`].
    pub fn controller(&self) -> ContextMenuController {
        ContextMenuController {
            link: self.opener.clone(),
        }
    }
    /// Wraps content in an initially closed menu.
    /// An empty/all-disabled menu does not open.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns [`ChoiceError::EmptyValue`] for an empty or whitespace-only action key and
    /// [`ChoiceError::DuplicateValue`] for two sibling items with the same key, at every
    /// submenu level.
    pub fn new(
        ui: &impl crate::StateOwner,
        content: Element,
        items: Vec<MenuItem>,
    ) -> Result<Self, ChoiceError> {
        validate_menu(&items)?;
        let enabled = items.iter().any(MenuItem::may_enable);
        let open = ui.signal(false);
        let surface = Slot::new(open.clone());
        let chain = MenuChain::closing(&surface);
        let branch = MenuBranch::new(&open, items, chain.clone(), None)?;
        let mut popover = Popover::empty(open.clone());
        popover.set_list_content(content, Element::new("actions", branch).structural());
        let runnable = Rc::new(std::cell::Cell::new(enabled));
        let request = ui.signal(None);
        // Openers refuse while the mounted menu is hidden or once it is
        // removed; the lifecycle hooks publish both.
        let visible_signal = ui.signal(true);
        let opener = Slot::new(OpenerLink {
            open: open.clone(),
            request: request.clone(),
            runnable: runnable.clone(),
            visible: visible_signal.clone(),
        });
        Ok(Self {
            popover,
            open,
            surface,
            chain,
            position: None,
            enabled,
            runnable,
            request,
            seen_request: 0,
            visible_signal,
            opener,
        })
    }
    fn open_at(
        &mut self,
        cx: &mut EventContext,
        position: Option<Point>,
    ) -> Result<(), RuntimeError> {
        self.position = position;
        self.chain.capture(cx);
        self.open.set(true)?;
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }
}
impl Component for ContextMenu {
    /// Points the new description's rows at this menu's open state and captured
    /// context and takes whether any row can run. The content and every level
    /// are reconciled children; the open position is kept.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            popover: _,
            open: _,
            surface,
            chain,
            position: _,
            enabled,
            runnable: _,
            request: _,
            seen_request: _,
            visible_signal: _,
            opener,
        } = next;
        self.chain.adopt(chain, &surface, &self.open);
        self.surface = surface;
        // Whether any row can run only gates opening; the wrapper draws
        // nothing of its own, so a change needs no invalidation. Openers read
        // the shared value when they run, so nothing is published.
        self.enabled = enabled;
        self.runnable.set(enabled);
        // Openers taken from the new description reach this mounted menu.
        opener.set(self.opener.get());
        None
    }
    fn inspect(&self) -> ComponentDetails {
        menu_details(self.popover.inspect())
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.popover.take_children()
    }
    fn visibility_changed(&mut self, visible: bool) {
        self.visible_signal.publish_trusted(visible);
    }
    fn unmounted(&mut self) {
        self.visible_signal.publish_trusted(false);
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.popover.update(cx)?;
        // An opener's request places the menu and gives the rows their
        // context, as a gesture does in its event; the open state itself is
        // the signal the popover already read.
        if let Some(request) = cx.read(&self.request)?
            && request.serial != self.seen_request
        {
            self.seen_request = request.serial;
            self.position = request.position;
            *self.chain.capture.borrow_mut() = Some(request.context);
            cx.request_layout();
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        self.popover.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::ContextMenu)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        r: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.popover.dismiss_surface(cx, r)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.popover.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::from_size(s))?;
        if cx.len() == 2 {
            let v = cx.measurement().viewport();
            let p = cx.measure(1, Constraints::new(Some(v.width()), Some(v.height()))?)?;
            let (x, y) = self.position.map_or((0.0, s.height()), |p| (p.x(), p.y()));
            cx.place(
                1,
                rect(x, y, p.width().min(v.width()), p.height().min(v.height()))?,
            )?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.enabled && e.kind() == PointerKind::Press(PointerButton::Secondary) {
            let position = cx.to_local(e.position()).or(self.position);
            self.open_at(cx, position)?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.enabled && e.requests_context_menu(keys.modifiers()) {
            if e.transition == KeyTransition::Pressed {
                self.open_at(cx, None)?;
            }
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
}
