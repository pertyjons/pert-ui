//! A browser-like history of the places an application has visited.

use std::{cell::Cell, num::NonZeroUsize, rc::Rc};

use crate::{ReadSignal, RuntimeError, Signal};

/// The identity of one [`NavigationHistory`] entry.
///
/// Identities are unique within one history for its lifetime and are never
/// reused, so an identity held by a list or a menu cannot retarget another
/// entry after the history drops the one it named: such an identity simply
/// stops resolving. It is distinct from the entry's position, which changes
/// as entries are added and dropped.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct HistoryEntryId(u64);

impl HistoryEntryId {
    /// The identity's number, unique within its history; for logs, keys and
    /// inspection. Numbers increase with every recorded visit.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One visited place: the application's value for it, a label to show, and
/// a stable identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryEntry<T> {
    id: HistoryEntryId,
    value: T,
    label: String,
}

impl<T> HistoryEntry<T> {
    /// The entry's identity, stable while the history keeps the entry.
    #[must_use]
    pub const fn id(&self) -> HistoryEntryId {
        self.id
    }
    /// The application's value for the place, as given to
    /// [`NavigationHistory::visit`].
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }
    /// The label given with the visit, for buttons, tooltips and lists.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// An owned view of a [`NavigationHistory`]: its entries, oldest first, and
/// which one is current.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistorySnapshot<T> {
    entries: Vec<HistoryEntry<T>>,
    /// Index of the current entry; `None` exactly when `entries` is empty.
    position: Option<usize>,
}

impl<T> HistorySnapshot<T> {
    const fn empty() -> Self {
        Self {
            entries: Vec::new(),
            position: None,
        }
    }
    /// Every entry the history keeps, oldest first. Entries after the current
    /// one are the forward stack.
    #[must_use]
    pub fn entries(&self) -> &[HistoryEntry<T>] {
        &self.entries
    }
    /// The current entry, or `None` before the first visit.
    #[must_use]
    pub fn current(&self) -> Option<&HistoryEntry<T>> {
        self.position.and_then(|index| self.entries.get(index))
    }
    /// Whether [`NavigationHistory::back`] has an entry to move to.
    #[must_use]
    pub fn can_go_back(&self) -> bool {
        self.position.is_some_and(|index| index > 0)
    }
    /// Whether [`NavigationHistory::forward`] has an entry to move to.
    #[must_use]
    pub fn can_go_forward(&self) -> bool {
        self.position
            .is_some_and(|index| index + 1 < self.entries.len())
    }
    /// The entry [`NavigationHistory::back`] would move to, for a tooltip.
    #[must_use]
    pub fn previous(&self) -> Option<&HistoryEntry<T>> {
        self.position
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.entries.get(index))
    }
    /// The entry [`NavigationHistory::forward`] would move to, for a tooltip.
    #[must_use]
    pub fn next(&self) -> Option<&HistoryEntry<T>> {
        self.position.and_then(|index| self.entries.get(index + 1))
    }
    /// The entry with `id`, if the history still keeps it.
    #[must_use]
    pub fn get(&self, id: HistoryEntryId) -> Option<&HistoryEntry<T>> {
        self.entries.iter().find(|entry| entry.id == id)
    }
    fn index_of(&self, id: HistoryEntryId) -> Option<usize> {
        self.entries.iter().position(|entry| entry.id == id)
    }
}

/// A browser-like history of the places an application has visited, such as
/// pages, documents or selections, with back and forward movement.
///
/// The application decides what a place is (`T`, compared with `Eq`) and
/// when one is visited. The history only records and moves: [`Self::back`],
/// [`Self::forward`] and [`Self::go_to`] return the entry to restore and
/// never restore anything themselves.
///
/// - [`Self::visit`] records a new current place and drops every entry after
///   the current one (the forward stack), as a browser does. A visit equal to
///   the current entry records nothing.
/// - Moving back, forward or to an entry chosen from a list only changes
///   which entry is current; it never adds or reorders entries.
/// - The history keeps at most its capacity, [`Self::DEFAULT_CAPACITY`]
///   unless created with [`Self::with_capacity`]; a visit beyond it drops the
///   oldest entry.
/// - Entries are identified by [`HistoryEntryId`], never by position.
///
/// **Not recording the history's own navigation.** Restoring an entry makes
/// the application navigate, and that navigation must not be recorded as a
/// new visit. Two rules handle it without a separate guard:
///
/// - A visit equal to the current entry records nothing, so after
///   [`Self::back`] made an entry current, reporting that same place again
///   is harmless.
/// - Components that report changes with a [`ChangeOrigin`](crate::ChangeOrigin),
///   such as [`Tabs::on_select`](crate::Tabs::on_select), report the
///   application's own navigation (a reveal) as
///   [`ChangeOrigin::Reveal`](crate::ChangeOrigin::Reveal). Record only
///   [`ChangeOrigin::User`](crate::ChangeOrigin::User) changes, so the
///   intermediate places a restoration passes through are not recorded
///   either.
///
/// The state is published through signals, so controls such as back and
/// forward buttons and history lists update without polling:
/// [`Self::state`] for the whole snapshot and [`Self::can_go_back`],
/// [`Self::can_go_forward`] and [`Self::current`] for the common parts. The
/// history is UI-thread only (`Rc` based, neither `Send` nor `Sync`), and it
/// is not persisted. Cloning it shares the same history.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{KeyAction, Viewport};
/// use std::time::Duration;
///
/// fn tab(key: &str) -> Tab {
///     Tab::new(key, key, Element::new("body", Label::new(key)))
/// }
/// let mut ui = Runtime::new();
/// let history = NavigationHistory::new(&ui)?;
/// history.visit("mix".to_owned(), "Mix")?;
/// let recorder = history.clone();
/// // Record what the user selects, not what the application reveals.
/// let strip = Tabs::new([tab("mix"), tab("edit")]).on_select(move |_, change| {
///     if change.origin == ChangeOrigin::User
///         && let Some(key) = &change.selected
///     {
///         recorder.visit(key.clone(), key.clone())?;
///     }
///     Ok(())
/// });
/// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
/// let tabs = ui.mount_tree(Element::new("tabs", strip))?;
/// ui.flush(Duration::ZERO)?;
///
/// // The user moves to the next tab.
/// ui.focus(&tabs)?;
/// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
/// ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
/// assert_eq!(history.state().get()?.entries().len(), 2);
/// assert!(history.can_go_back().get()?);
///
/// // Back returns the place; restoring it is a reveal and records nothing.
/// let entry = history.back()?.expect("an earlier entry");
/// ui.reveal_child(&tabs, entry.value())?;
/// let state = history.state().get()?;
/// assert_eq!(state.entries().len(), 2);
/// assert_eq!(state.current().map(HistoryEntry::label), Some("Mix"));
/// assert!(history.can_go_forward().get()?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct NavigationHistory<T> {
    state: Signal<Rc<HistorySnapshot<T>>>,
    can_go_back: ReadSignal<bool>,
    can_go_forward: ReadSignal<bool>,
    current: ReadSignal<Option<HistoryEntry<T>>>,
    capacity: NonZeroUsize,
    next_id: Rc<Cell<u64>>,
}
impl<T> NavigationHistory<T> {
    /// Whether two handles are clones of one history.
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        self.current.same_source(&other.current)
    }
}

impl<T> Clone for NavigationHistory<T> {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            can_go_back: self.can_go_back.clone(),
            can_go_forward: self.can_go_forward.clone(),
            current: self.current.clone(),
            capacity: self.capacity,
            next_id: self.next_id.clone(),
        }
    }
}

/// Shows the capacity and the next identity, not the entries, which may not
/// implement `Debug`.
impl<T> std::fmt::Debug for NavigationHistory<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationHistory")
            .field("capacity", &self.capacity)
            .field("next_id", &self.next_id.get())
            .finish_non_exhaustive()
    }
}

impl<T: Clone + Eq + 'static> NavigationHistory<T> {
    /// The capacity of a history created with [`Self::new`]: 100 entries.
    pub const DEFAULT_CAPACITY: NonZeroUsize = match NonZeroUsize::new(100) {
        Some(capacity) => capacity,
        None => unreachable!(),
    };

    /// Creates an empty history in `ui` that keeps at most
    /// [`Self::DEFAULT_CAPACITY`] entries. `ui` is any
    /// [`StateOwner`](crate::StateOwner): the runtime, or an update or event
    /// context, so a component can create its history from its own update.
    ///
    /// # Errors
    /// Returns `Closed` when `ui` has shut down, or `BorrowConflict` on a
    /// reentrant borrow while its derived signals are created.
    pub fn new(ui: &impl crate::StateOwner) -> Result<Self, RuntimeError> {
        Self::with_capacity(ui, Self::DEFAULT_CAPACITY)
    }

    /// Creates an empty history in `ui` that keeps at most `capacity` entries;
    /// a visit beyond it drops the oldest entry.
    ///
    /// # Errors
    /// As [`Self::new`].
    pub fn with_capacity(
        ui: &impl crate::StateOwner,
        capacity: NonZeroUsize,
    ) -> Result<Self, RuntimeError> {
        let state = ui.signal(Rc::new(HistorySnapshot::empty()));
        let read = state.read_only();
        Ok(Self {
            can_go_back: read.map(|state| state.can_go_back())?,
            can_go_forward: read.map(|state| state.can_go_forward())?,
            current: read.map(|state| state.current().cloned())?,
            state,
            capacity,
            next_id: Rc::new(Cell::new(1)),
        })
    }

    /// The most entries this history keeps.
    #[must_use]
    pub const fn capacity(&self) -> NonZeroUsize {
        self.capacity
    }

    /// The whole history: entries oldest first and the current one.
    #[must_use]
    pub fn state(&self) -> ReadSignal<Rc<HistorySnapshot<T>>> {
        self.state.read_only()
    }

    /// Whether [`Self::back`] has an entry to move to.
    #[must_use]
    pub fn can_go_back(&self) -> ReadSignal<bool> {
        self.can_go_back.clone()
    }

    /// Whether [`Self::forward`] has an entry to move to.
    #[must_use]
    pub fn can_go_forward(&self) -> ReadSignal<bool> {
        self.can_go_forward.clone()
    }

    /// The current entry, `None` before the first visit.
    #[must_use]
    pub fn current(&self) -> ReadSignal<Option<HistoryEntry<T>>> {
        self.current.clone()
    }

    /// Records `value` as the new current place, labelled `label`, and returns
    /// whether anything was recorded.
    ///
    /// A visit equal to the current entry records nothing and keeps that
    /// entry's label. Otherwise every entry after the current one is dropped,
    /// the visit becomes the newest and current entry with a new identity,
    /// and the oldest entry is dropped when the history is over capacity. An
    /// equal value earlier in the history is not merged: it is a separate
    /// visit, as in a browser.
    ///
    /// # Errors
    /// Every error leaves the history unchanged: the signal write errors of
    /// [`Signal::set`] (`Closed`, the `WriteDuring*` phase errors when called
    /// from update, layout, paint, inspection or a delivery callback, and
    /// `BorrowConflict`), and `Exhausted` if the history has handed out
    /// `u64::MAX` identities.
    pub fn visit(&self, value: T, label: impl Into<String>) -> Result<bool, RuntimeError> {
        let state = self.state.get()?;
        if state.current().is_some_and(|entry| entry.value == value) {
            return Ok(false);
        }
        let id = self.next_id.get();
        let next = id.checked_add(1).ok_or(RuntimeError::Exhausted)?;
        let mut entries = state.entries.clone();
        entries.truncate(state.position.map_or(0, |index| index + 1));
        entries.push(HistoryEntry {
            id: HistoryEntryId(id),
            value,
            label: label.into(),
        });
        let excess = entries.len().saturating_sub(self.capacity.get());
        entries.drain(..excess);
        let position = Some(entries.len() - 1);
        self.state
            .set(Rc::new(HistorySnapshot { entries, position }))?;
        self.next_id.set(next);
        Ok(true)
    }

    /// Makes the previous entry current and returns it for the application to
    /// restore; `None`, changing nothing, when there is no earlier entry.
    ///
    /// # Errors
    /// The signal write errors of [`Self::visit`]; the history is unchanged.
    pub fn back(&self) -> Result<Option<HistoryEntry<T>>, RuntimeError> {
        let state = self.state.get()?;
        match state.position.and_then(|index| index.checked_sub(1)) {
            Some(index) => self.move_to(&state, index).map(Some),
            None => Ok(None),
        }
    }

    /// Makes the next entry current and returns it for the application to
    /// restore; `None`, changing nothing, when there is no later entry.
    ///
    /// # Errors
    /// The signal write errors of [`Self::visit`]; the history is unchanged.
    pub fn forward(&self) -> Result<Option<HistoryEntry<T>>, RuntimeError> {
        let state = self.state.get()?;
        if !state.can_go_forward() {
            return Ok(None);
        }
        let index = state.position.map_or(0, |index| index + 1);
        self.move_to(&state, index).map(Some)
    }

    /// Makes the entry `id` current, as choosing it from a history list does,
    /// and returns it for the application to restore. The entries keep their
    /// order: nothing is dropped and no new entry is added.
    ///
    /// Returns `None`, changing nothing, when the history no longer keeps the
    /// entry (it was dropped by a later visit or by the capacity), so a stale
    /// identity never moves to another entry. Choosing the current entry
    /// returns it without a change.
    ///
    /// # Errors
    /// The signal write errors of [`Self::visit`]; the history is unchanged.
    pub fn go_to(&self, id: HistoryEntryId) -> Result<Option<HistoryEntry<T>>, RuntimeError> {
        let state = self.state.get()?;
        match state.index_of(id) {
            Some(index) => self.move_to(&state, index).map(Some),
            None => Ok(None),
        }
    }

    /// Removes every entry. Identities already handed out are not reused.
    ///
    /// # Errors
    /// The signal write errors of [`Self::visit`]; the history is unchanged.
    pub fn clear(&self) -> Result<(), RuntimeError> {
        self.state.set(Rc::new(HistorySnapshot::empty()))
    }

    fn move_to(
        &self,
        state: &HistorySnapshot<T>,
        index: usize,
    ) -> Result<HistoryEntry<T>, RuntimeError> {
        let entry = state.entries[index].clone();
        if state.position != Some(index) {
            self.state.set(Rc::new(HistorySnapshot {
                entries: state.entries.clone(),
                position: Some(index),
            }))?;
        }
        Ok(entry)
    }
}
