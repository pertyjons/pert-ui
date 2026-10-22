//! Keyed navigation and selection shared by virtualized collections.
use crate::{ChangeOrigin, ListItems, ReadSignal, RuntimeError, Signal};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

/// How many items a [`ListSelection`] holds at once.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SelectionMode {
    /// The selection is the active item and follows it.
    #[default]
    Single,
    /// Any number of items; Space toggles, Shift extends a range from the anchor
    /// and Control moves or toggles without replacing the rest.
    Multiple,
}

/// The modifier-dependent meaning of one navigation or pointer gesture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Gesture {
    /// Keyboard movement: replace, extend with Shift, or only move with Control.
    Move { extend: bool, keep: bool },
    /// A pointer press: replace, extend with Shift, or toggle with Control.
    Press { extend: bool, toggle: bool },
    /// Space on the active item.
    Toggle,
}

/// Application-owned navigation and selection state for a collection, by key.
///
/// A [`VirtualList`](crate::VirtualList), and the tree and table built on it,
/// navigate by item position, which crosses items that are not mounted, and
/// record the result here by key. The application reads and binds the two
/// signals, and may write them: [`ListSelection::active`] is the item keyboard
/// navigation is on, and [`ListSelection::selected`] the chosen keys.
///
/// The selected keys are shared behind an [`Rc`], so reading or binding them,
/// as a list does at every update, costs no copy of a large selection; a
/// change replaces the set.
///
/// Keys are identity, so the selection survives reordering and a replaced source.
/// It is never pruned: a key the source no longer has is just not drawn, because
/// only the application knows whether that item is gone or merely filtered out.
/// Clones share the same state. Nothing here schedules work.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ListSelection, Runtime, SelectionMode};
/// let ui = Runtime::new();
/// let selection = ListSelection::new(&ui, SelectionMode::Multiple);
/// selection.select(["a".to_owned(), "c".to_owned()])?;
/// selection.set_active(Some("c".to_owned()))?;
/// assert!(selection.is_selected("a")? && !selection.is_selected("b")?);
/// assert_eq!(selection.active().get()?.as_deref(), Some("c"));
/// selection.clear()?;
/// assert!(selection.selected().get()?.is_empty());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct ListSelection {
    mode: SelectionMode,
    active: Signal<Option<String>>,
    selected: Signal<Rc<BTreeSet<String>>>,
    /// Where a Shift range starts; moves with every gesture that is not a range.
    anchor: Rc<RefCell<Option<String>>>,
    /// The origin writes made now carry: `Application` unless a collection
    /// writes on behalf of its user.
    origin: Rc<Cell<ChangeOrigin>>,
    /// The origin of the latest write, which a collection reports.
    last: Rc<Cell<ChangeOrigin>>,
}

/// A change of a [`ListSelection`] that a collection reports through
/// [`VirtualList::on_selection_change`](crate::VirtualList::on_selection_change),
/// [`Tree::on_selection_change`](crate::Tree::on_selection_change) or
/// [`Table::on_selection_change`](crate::Table::on_selection_change).
///
/// It carries both halves of the selection, before and after: the active item
/// keyboard navigation is on, and the selected keys. In
/// [`SelectionMode::Single`] they agree; in [`SelectionMode::Multiple`] either
/// can change alone.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListSelectionChange {
    /// The active item before the change.
    pub previous_active: Option<String>,
    /// The active item after the change.
    pub active: Option<String>,
    /// The selected keys before the change.
    pub previous_selected: Rc<BTreeSet<String>>,
    /// The selected keys after the change.
    pub selected: Rc<BTreeSet<String>>,
    /// What caused the change: [`ChangeOrigin::User`] for a click or key on a
    /// collection, [`ChangeOrigin::Application`] for a write through the
    /// [`ListSelection`] methods or a controller.
    pub origin: ChangeOrigin,
}
impl ListSelection {
    /// Creates an empty selection with no active item.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner, mode: SelectionMode) -> Self {
        Self {
            mode,
            active: ui.signal(None),
            selected: ui.signal(Rc::new(BTreeSet::new())),
            anchor: Rc::new(RefCell::new(None)),
            origin: Rc::new(Cell::new(ChangeOrigin::Application)),
            last: Rc::new(Cell::new(ChangeOrigin::Application)),
        }
    }
    /// Runs `write` with its selection writes carrying `origin`, for a
    /// collection that writes on behalf of its user.
    pub(crate) fn with_origin<R>(&self, origin: ChangeOrigin, write: impl FnOnce() -> R) -> R {
        let previous = self.origin.replace(origin);
        let result = write();
        self.origin.set(previous);
        result
    }
    /// The origin of the latest write.
    pub(crate) fn last_origin(&self) -> ChangeOrigin {
        self.last.get()
    }
    fn written(&self) {
        self.last.set(self.origin.get());
    }
    /// Whether one or several items can be selected.
    #[must_use]
    pub const fn mode(&self) -> SelectionMode {
        self.mode
    }
    /// The item keyboard navigation is on, if any.
    #[must_use]
    pub fn active(&self) -> ReadSignal<Option<String>> {
        self.active.read_only()
    }
    /// The selected keys, shared rather than copied on every read.
    #[must_use]
    pub fn selected(&self) -> ReadSignal<Rc<BTreeSet<String>>> {
        self.selected.read_only()
    }
    /// Whether `key` is selected.
    ///
    /// # Errors
    /// The signal read's failures: `Closed` or `BorrowConflict`.
    pub fn is_selected(&self, key: &str) -> Result<bool, RuntimeError> {
        Ok(self.selected.get()?.contains(key))
    }
    /// Moves the active item and makes it the anchor of the next range. `None`
    /// leaves no item active.
    ///
    /// In [`SelectionMode::Multiple`] the selected keys do not change. In
    /// [`SelectionMode::Single`] the selection is the active item, so it
    /// becomes exactly `key`, or empty for `None`.
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`. A rejected call changes
    /// neither the signals nor the anchor.
    pub fn set_active(&self, key: Option<String>) -> Result<(), RuntimeError> {
        if self.mode == SelectionMode::Single {
            self.selected.set(Rc::new(key.iter().cloned().collect()))?;
        }
        self.active.set(key.clone())?;
        *self.anchor.borrow_mut() = key;
        self.written();
        Ok(())
    }
    /// Replaces the selected keys. In [`SelectionMode::Single`] only the first
    /// key is kept, and it becomes the active item and the anchor, as
    /// [`ListSelection::set_active`] does; selecting no key there leaves no
    /// item active. In [`SelectionMode::Multiple`] the active item and the
    /// anchor do not change.
    ///
    /// # Errors
    /// Same as [`ListSelection::set_active`].
    pub fn select(&self, keys: impl IntoIterator<Item = String>) -> Result<(), RuntimeError> {
        let mut keys = keys.into_iter();
        match self.mode {
            SelectionMode::Single => self.set_active(keys.next()),
            SelectionMode::Multiple => {
                self.selected.set(Rc::new(keys.collect()))?;
                self.written();
                Ok(())
            }
        }
    }
    /// Selects nothing and leaves no item active.
    ///
    /// # Errors
    /// Same as [`ListSelection::set_active`].
    pub fn clear(&self) -> Result<(), RuntimeError> {
        self.set_active(None)?;
        self.selected.set(Rc::new(BTreeSet::new()))?;
        self.written();
        Ok(())
    }
    /// Applies one gesture that landed on `index` of `items`.
    pub(crate) fn apply(
        &self,
        items: &dyn ListItems,
        index: usize,
        gesture: Gesture,
    ) -> Result<(), RuntimeError> {
        let Some(key) = items.key(index) else {
            return Ok(());
        };
        let single = self.mode == SelectionMode::Single;
        match gesture {
            Gesture::Toggle if single => self.replace(key)?,
            Gesture::Toggle | Gesture::Press { toggle: true, .. } if !single => {
                let mut selected = Rc::unwrap_or_clone(self.selected.get()?);
                if !selected.remove(&key) {
                    selected.insert(key.clone());
                }
                self.selected.set(Rc::new(selected))?;
                self.set_active(Some(key))?;
            }
            Gesture::Move { extend: true, .. } | Gesture::Press { extend: true, .. } if !single => {
                // The range runs from the anchor, which a range never moves, to
                // the item the gesture landed on, whichever comes first.
                let anchor = self
                    .anchor
                    .borrow()
                    .as_deref()
                    .and_then(|anchor| items.index_of(anchor))
                    .unwrap_or(index);
                let range = anchor.min(index)..=anchor.max(index);
                self.selected.set(Rc::new(
                    range.filter_map(|index| items.key(index)).collect(),
                ))?;
                self.active.set(Some(key))?;
            }
            Gesture::Move { keep: true, .. } if !single => self.set_active(Some(key))?,
            _ => self.replace(key)?,
        }
        self.written();
        Ok(())
    }
    fn replace(&self, key: String) -> Result<(), RuntimeError> {
        if self.mode == SelectionMode::Multiple {
            self.selected.set(Rc::new(BTreeSet::from([key.clone()])))?;
        }
        self.set_active(Some(key))
    }
}
