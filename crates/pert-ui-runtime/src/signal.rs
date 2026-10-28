use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

use crate::runtime::{Invalidation, Node, RuntimeError, Shared};

/// A runtime-local owner that creates new state: [`crate::Runtime`],
/// [`crate::UpdateContext`], [`crate::EventContext`] and
/// [`crate::DescribeContext`], and references to them.
///
/// Constructors of built-in components that need local state (the open state
/// of a [`crate::Select`] or [`crate::ContextMenu`], a controller's signals)
/// take `&impl StateOwner`, so they can be called with the [`crate::Runtime`]
/// while an application builds its tree, and with the [`crate::UpdateContext`]
/// or [`crate::EventContext`] a [`crate::Component`] receives when it
/// describes its children from `update` or reacts to input. Each creates the
/// same [`Signal`] type in the same runtime; the handle is UI-thread local and
/// its value lives as long as any clone of it, as [`crate::Runtime::signal`]
/// documents. A component that creates state in every update creates new
/// state each time, so it keeps what must survive a description.
///
/// The trait is sealed: the library implements it, so it can gain methods
/// without breaking applications. An application's own type holds the
/// [`crate::Runtime`] or the context it was given and passes that on; it
/// cannot implement the trait, and this does not compile:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Runtime, Signal, StateOwner, UiSettings};
///
/// struct MyOwner(Runtime);
/// impl StateOwner for MyOwner {
///     fn signal<T>(&self, value: T) -> Signal<T> {
///         self.0.signal(value)
///     }
///     fn ui_settings(&self) -> UiSettings {
///         self.0.ui_settings()
///     }
/// }
/// ```
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     Choice, Component, ContextMenu, Element, Label, MenuItem, Runtime, RuntimeError, Select,
///     UpdateContext,
/// };
///
/// /// A custom component that describes a select and a context menu from its
/// /// update, with no `Runtime` at hand.
/// #[derive(Default)]
/// struct Panel {
///     described: bool,
/// }
/// impl Component for Panel {
///     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         if !self.described {
///             let mode = cx.signal(1_u8);
///             let options = [Choice::new(1, "One"), Choice::new(2, "Two")];
///             let select = Select::new(cx, mode, options)?;
///             let rows = vec![MenuItem::new("copy", "Copy", |_| Ok(()))];
///             let text = Element::new("text", Label::new("Text"));
///             let menu = ContextMenu::new(cx, text, rows)?;
///             cx.request_children(vec![
///                 Element::new("mode", select),
///                 Element::new("menu", menu),
///             ]);
///             self.described = true;
///         }
///         Ok(())
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.mount_tree(Element::new("panel", Panel::default()))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// assert!(ui.inspect().components.iter().any(|item| item.key == "mode"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot create signals",
    label = "expected a `Runtime`, `UpdateContext`, `EventContext` or `DescribeContext`",
    note = "pass the runtime or the context of the callback you are in; `StateOwner` cannot be implemented outside pert-ui"
)]
pub trait StateOwner: sealed::StateOwnerSeal {
    /// Creates a signal owned by this owner's runtime, holding `value`.
    #[must_use]
    fn signal<T>(&self, value: T) -> Signal<T>;
    /// Shared UI-thread presentation settings for this owner's runtime.
    ///
    /// Clones share the installed settings. Reading them while constructing
    /// initial child descriptions subscribes nothing; retained components use
    /// [`UpdateContext::strings`](crate::UpdateContext::strings) in update to
    /// track later table changes. Setters obey ordinary phase restrictions.
    /// The handle can outlive the owner; getters return `Closed` after shutdown.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Runtime, StateOwner, UiStrings};
    /// fn initial_strings(owner: &impl StateOwner) -> Result<UiStrings, pert_ui::RuntimeError> {
    ///     owner.ui_settings().strings()
    /// }
    /// let ui = Runtime::new();
    /// assert_eq!(initial_strings(&ui)?, UiStrings::english());
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    fn ui_settings(&self) -> crate::UiSettings;
}
pub(crate) mod sealed {
    /// Seals [`StateOwner`](super::StateOwner). The distinct name keeps rustc
    /// from pointing at a dependency's own `Sealed` trait when an application
    /// tries to implement `StateOwner`, and the message below replaces the
    /// private path in that error.
    #[diagnostic::on_unimplemented(
        message = "`StateOwner` is sealed and cannot be implemented for `{Self}`",
        label = "not one of pert-ui's state owners",
        note = "only `Runtime`, `UpdateContext`, `EventContext` and `DescribeContext` own state; hold one of them, or take `&impl StateOwner` and pass one through"
    )]
    pub trait StateOwnerSeal {}
    impl StateOwnerSeal for crate::Runtime {}
    impl StateOwnerSeal for crate::UpdateContext {}
    impl StateOwnerSeal for crate::EventContext {}
    impl<O: StateOwnerSeal + ?Sized> StateOwnerSeal for &O {}
    impl<O: StateOwnerSeal + ?Sized> StateOwnerSeal for &mut O {}

    use super::{CombineInput, CombineInputs, ReadSignal};
    use crate::RuntimeError;

    /// Proves a call of a sealed operation comes from this crate: its field
    /// is crate-private, so code outside the crate cannot construct one and cannot
    /// call [`CombineInputSeal::read_signal`] or [`CombineInputsSeal::derive`],
    /// although a bound on the public trait brings them into scope.
    #[derive(Debug, Clone, Copy)]
    pub struct Token(pub(crate) ());

    /// Seals [`CombineInput`](super::CombineInput) and carries the operation
    /// [`ReadSignal::combine`] needs from each input.
    #[diagnostic::on_unimplemented(
        message = "`CombineInput` is sealed and cannot be implemented for `{Self}`",
        label = "not one of pert-ui's bindings",
        note = "only `Signal`, `ReadSignal` and references to them implement `CombineInput`"
    )]
    pub trait CombineInputSeal {
        /// The read binding this input observes.
        fn read_signal(&self, token: Token) -> &ReadSignal<<Self as CombineInput>::Value>
        where
            Self: CombineInput;
    }

    /// Seals [`CombineInputs`](super::CombineInputs) and carries the
    /// derivation [`ReadSignal::combine`] runs for each tuple arity.
    #[diagnostic::on_unimplemented(
        message = "`CombineInputs` is sealed and cannot be implemented for `{Self}`",
        label = "not one of pert-ui's input tuples",
        note = "only pert-ui's tuples of 2 to 12 `CombineInput`s implement `CombineInputs`"
    )]
    pub trait CombineInputsSeal {
        /// Derives a binding over these inputs; see [`ReadSignal::combine`].
        fn derive<T, F>(&self, token: Token, map: F) -> Result<ReadSignal<T>, RuntimeError>
        where
            Self: CombineInputs,
            T: Clone + PartialEq + 'static,
            F: Fn(&<Self as CombineInputs>::Values) -> T + 'static;
    }
}
impl StateOwner for crate::Runtime {
    fn signal<T>(&self, value: T) -> Signal<T> {
        Self::signal(self, value)
    }
    fn ui_settings(&self) -> crate::UiSettings {
        Self::ui_settings(self)
    }
}
impl StateOwner for crate::UpdateContext {
    fn signal<T>(&self, value: T) -> Signal<T> {
        Self::signal(self, value)
    }
    fn ui_settings(&self) -> crate::UiSettings {
        Self::ui_settings(self)
    }
}
impl StateOwner for crate::EventContext {
    fn signal<T>(&self, value: T) -> Signal<T> {
        Self::signal(self, value)
    }
    fn ui_settings(&self) -> crate::UiSettings {
        Self::ui_settings(self)
    }
}
impl<O: StateOwner + ?Sized> StateOwner for &O {
    fn signal<T>(&self, value: T) -> Signal<T> {
        O::signal(self, value)
    }
    fn ui_settings(&self) -> crate::UiSettings {
        O::ui_settings(self)
    }
}
impl<O: StateOwner + ?Sized> StateOwner for &mut O {
    fn signal<T>(&self, value: T) -> Signal<T> {
        O::signal(self, value)
    }
    fn ui_settings(&self) -> crate::UiSettings {
        O::ui_settings(self)
    }
}

struct Inner<T> {
    shared: Rc<Shared>,
    value: RefCell<T>,
    /// Advances on every write that changed the value.
    revision: std::cell::Cell<u64>,
    subscribers: RefCell<BTreeMap<usize, (Weak<Node>, Invalidation)>>,
    computations: RefCell<BTreeMap<usize, Weak<dyn Computation>>>,
}

/// Mutable UI-local observable value created by [crate::Runtime::signal].
///
/// Clones share a value. Writes compare with `PartialEq`, notify only dependent
/// components and never invoke user callbacks synchronously. Flushes coalesce
/// notifications. Values with interior mutation must still be replaced through
/// `set`; invisible shared mutations do not notify. No worker or callback-safe endpoint.
///
/// A signal is also its own read binding: it dereferences to a [`ReadSignal`],
/// so `&signal` works wherever `&ReadSignal` is taken, such as
/// [`UpdateContext::read`](crate::UpdateContext::read);
/// [`ReadSignal::map`] derives from it directly; and it is one input of
/// [`ReadSignal::combine`]'s tuple as it is ([`CombineInput`]).
/// Parameters that take a binding by value accept `&signal` through `From`:
/// a reference converts to a clone sharing the value, and a writable signal
/// converts to a [`ReadSignal`] where only reading is needed.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::Runtime;
/// let ui = Runtime::new();
/// let count = ui.signal(2);
/// let caption = count.map(|n| format!("{n} items"))?;
/// count.update(|n| *n += 1)?;
/// assert_eq!(caption.get()?, "3 items");
/// let enabled = ui.signal(false);
/// enabled.toggle()?;
/// assert!(enabled.get()?);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// let runtime = pert_ui::Runtime::new();
/// let signal = runtime.signal(0);
/// std::thread::spawn(move || drop(signal));
/// ```
pub struct Signal<T> {
    /// The read capability; a signal is a read binding that can also write.
    read: ReadSignal<T>,
}

/// Read-only view of a signal, with the same UI-thread ownership as its writer.
///
/// Reads in an update context automatically subscribe that component. [Self::get]
/// is an untracked read for event/application code. It clones only this value.
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// let runtime = pert_ui::Runtime::new();
/// let read = runtime.signal(0).read_only();
/// read.set(1); // no write capability
/// ```
pub struct ReadSignal<T> {
    inner: Rc<Inner<T>>,
    computation: Option<Rc<dyn Computation>>,
}

/// Shows no value: formatting must not borrow a value that may be borrowed
/// already, nor run the value's own `Debug` inside a runtime phase. Read it with
/// [`Signal::get`].
impl<T> core::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Signal").finish_non_exhaustive()
    }
}

/// Shows whether the binding is derived, not the value, like [`Signal`]'s `Debug`.
impl<T> core::fmt::Debug for ReadSignal<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ReadSignal")
            .field("derived", &self.computation.is_some())
            .finish_non_exhaustive()
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            read: self.read.clone(),
        }
    }
}

impl<T> std::ops::Deref for Signal<T> {
    type Target = ReadSignal<T>;
    fn deref(&self) -> &ReadSignal<T> {
        &self.read
    }
}

/// A clone sharing the value, so a constructor taking a signal accepts `&signal`.
impl<T> From<&Signal<T>> for Signal<T> {
    fn from(signal: &Signal<T>) -> Self {
        signal.clone()
    }
}

/// The read capability of a signal, for a parameter that only reads.
impl<T> From<Signal<T>> for ReadSignal<T> {
    fn from(signal: Signal<T>) -> Self {
        signal.read
    }
}

/// The read capability of a signal, sharing its value.
impl<T> From<&Signal<T>> for ReadSignal<T> {
    fn from(signal: &Signal<T>) -> Self {
        signal.read.clone()
    }
}

/// A clone sharing the source, so a constructor taking a read binding accepts
/// `&binding`.
impl<T> From<&ReadSignal<T>> for ReadSignal<T> {
    fn from(binding: &ReadSignal<T>) -> Self {
        binding.clone()
    }
}
impl<T> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            computation: self.computation.clone(),
        }
    }
}

/// A signal the runtime publishes to without keeping it alive, so a runtime
/// setting that nobody observes costs nothing and `Shared` holds no cycle.
pub(crate) struct WeakSignal<T>(Weak<Inner<T>>);

impl<T> Default for WeakSignal<T> {
    fn default() -> Self {
        Self(Weak::new())
    }
}

impl<T> WeakSignal<T> {
    pub(crate) fn upgrade(&self) -> Option<Signal<T>> {
        self.0.upgrade().map(|inner| Signal {
            read: ReadSignal {
                inner,
                computation: None,
            },
        })
    }
}

impl<T> Signal<T> {
    pub(crate) fn downgrade(&self) -> WeakSignal<T> {
        WeakSignal(Rc::downgrade(&self.inner))
    }
    pub(crate) fn new(shared: Rc<Shared>, value: T) -> Self {
        Self {
            read: ReadSignal {
                inner: Rc::new(Inner {
                    shared,
                    value: RefCell::new(value),
                    revision: std::cell::Cell::new(0),
                    subscribers: RefCell::new(BTreeMap::new()),
                    computations: RefCell::new(BTreeMap::new()),
                }),
                computation: None,
            },
        }
    }
    /// Creates another signal on the same runtime as this one, holding
    /// `value`.
    ///
    /// A component receives bindings, not the [`Runtime`](crate::Runtime), so
    /// this is how one that owns UI-local state of its own, such as the open
    /// state of a popup it builds, creates that state from a binding it was
    /// given. The new signal is independent: it shares only the runtime and
    /// its UI thread, not the value. It never fails: a signal of a closed
    /// runtime creates a signal whose writes fail with
    /// [`RuntimeError::Closed`], like the original's.
    ///
    /// A row of a collection gets no state owner in
    /// [`ListItems::build`](crate::ListItems::build) or
    /// [`TableItems::cell`](crate::TableItems::cell). A display control in a
    /// row takes the record's value itself, such as
    /// [`ProgressBar::new`](crate::ProgressBar::new), with no binding. An
    /// editing control should be controlled where it can be:
    /// [`Checkbox::controlled`](crate::Checkbox::controlled),
    /// [`Toggle::controlled`](crate::Toggle::controlled) and
    /// [`InlineText::controlled`](crate::InlineText::controlled) take the
    /// record's value and propose each change, with no binding. For an
    /// editing control without a controlled form, a per-row binding made here
    /// is the advanced form: call it in `build` on a signal the rows hold, with the record's
    /// value, and write changes back to the records from the control's
    /// callback. Creating a signal is not a write, so any phase may. The cost
    /// is a new binding for every mounted row each time the collection
    /// describes its rows again: when the source is replaced, and when a
    /// scroll or resize changes which rows are mounted. A new binding ends an
    /// open text draft; `ListItems::build` describes the retained alternative
    /// for a control that keeps one.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::Runtime;
    /// let ui = Runtime::new();
    /// let color = ui.signal(0_u32);
    /// let open = color.sibling(false);
    /// open.set(true)?;
    /// assert!(open.get()?);
    /// assert_eq!(color.get()?, 0);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    #[must_use]
    pub fn sibling<U>(&self, value: U) -> Signal<U> {
        Signal::new(self.inner.shared.clone(), value)
    }
    pub(crate) fn shared(&self) -> &Rc<Shared> {
        &self.inner.shared
    }
    /// Counts subscribed components and how many of them are visible, pruning
    /// dead entries; visibility is what keeps a live-data source draining.
    pub(crate) fn subscriber_counts(&self) -> (usize, usize) {
        let mut subscribers = self.inner.subscribers.borrow_mut();
        subscribers.retain(|_, (node, _)| node.strong_count() > 0);
        let visible = subscribers
            .values()
            .filter(|(node, _)| node.upgrade().is_some_and(|node| node.is_presented()))
            .count();
        (subscribers.len(), visible)
    }
    pub(crate) fn presentation_interval(&self) -> Option<std::time::Duration> {
        self.inner
            .subscribers
            .borrow()
            .values()
            .filter_map(|(node, _)| node.upgrade())
            .filter(|node| node.is_presented())
            .map(|node| node.presentation_interval())
            .min()
    }
    /// A counter that advances by one, wrapping, on every write that changed
    /// the value, from any clone of this signal; a write of an equal value
    /// leaves it. Starts at zero.
    ///
    /// Comparing two readings tells whether the value was written in
    /// between even when it was changed and then changed back, which
    /// comparing values cannot. A component that holds a draft over a
    /// binding uses it to let any write since the draft began win over the
    /// draft. It is untracked: reading it subscribes nothing.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::Runtime;
    /// let ui = Runtime::new();
    /// let value = ui.signal(1);
    /// let before = value.revision();
    /// value.set(2)?;
    /// value.set(1)?;
    /// assert_eq!(value.get()?, 1);
    /// assert_ne!(value.revision(), before);
    /// value.set(1)?; // unchanged: no new revision
    /// assert_eq!(value.revision(), before.wrapping_add(2));
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.inner.revision.get()
    }
    /// Returns a read-only binding sharing this value, without cloning its contents.
    #[must_use]
    pub fn read_only(&self) -> ReadSignal<T> {
        self.read.clone()
    }
}

impl<T: Clone> Signal<T> {
    /// Clones the current value without registering a component dependency.
    ///
    /// # Errors
    /// Returns `Closed` after runtime drop, or `BorrowConflict` for a reentrant read.
    pub fn get(&self) -> Result<T, RuntimeError> {
        self.read_only().get()
    }
}

impl<T: PartialEq> Signal<T> {
    pub(crate) fn publish_copy(&self, value: T)
    where
        T: Copy,
    {
        self.publish_trusted(value);
    }
    /// Live revision publication may defer presentation-only readers behind clips.
    pub(crate) fn publish_presentation_copy(&self, value: T)
    where
        T: Copy,
    {
        let mut current = self.inner.value.borrow_mut();
        if *current == value {
            return;
        }
        *current = value;
        drop(current);
        self.advance_revision();
        self.invalidate_computations();
        self.inner
            .subscribers
            .borrow_mut()
            .retain(|_, (node, kind)| {
                if let Some(node) = node.upgrade() {
                    node.invalidate_presentation(*kind);
                    true
                } else {
                    false
                }
            });
    }
    /// Publishes a library-owned value whose equality and drop run no user code,
    /// so no reentrant borrow can occur and publication cannot fail.
    pub(crate) fn publish_trusted(&self, value: T) {
        let mut current = self.inner.value.borrow_mut();
        if *current == value {
            return;
        }
        *current = value;
        drop(current);
        self.advance_revision();
        self.invalidate_computations();
        self.inner
            .subscribers
            .borrow_mut()
            .retain(|_, (node, kind)| {
                if let Some(node) = node.upgrade() {
                    node.invalidate(*kind);
                    true
                } else {
                    false
                }
            });
    }
    /// Replaces a changed value and coalesces dependent component invalidations.
    ///
    /// An equal value changes nothing; use [`Self::set_changed`] to learn
    /// whether the value changed. No user callback runs as a notification.
    /// Equality follows `T`; use validated values if NaN equality is undesirable.
    ///
    /// # Errors
    /// Every error leaves the value unchanged and notifies nobody:
    /// `Closed` after the runtime shut down; `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint` or `WriteDuringInspection` from
    /// inside that phase; `WriteDuringDelivery` from inside a parameter delivery
    /// callback, such as a
    /// [`ParameterHost`](crate::ParameterHost) sink, which must defer the write
    /// itself until the callback has returned; and `BorrowConflict` for a
    /// reentrant write from the value's own `PartialEq`.
    pub fn set(&self, value: T) -> Result<(), RuntimeError> {
        self.set_changed(value).map(|_| ())
    }
    /// Writes like [`Self::set`] and returns whether the value changed.
    ///
    /// # Errors
    /// The errors of [`Self::set`], with the same guarantee that a failed write
    /// changes nothing.
    pub fn set_changed(&self, value: T) -> Result<bool, RuntimeError> {
        self.inner.shared.writable()?;
        self.publish(value)
    }
    pub(crate) fn publish(&self, value: T) -> Result<bool, RuntimeError> {
        let mut current = self
            .inner
            .value
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        if *current == value {
            return Ok(false);
        }
        let old = std::mem::replace(&mut *current, value);
        drop(current);
        self.advance_revision();
        self.invalidate_computations();
        self.inner
            .subscribers
            .borrow_mut()
            .retain(|_, (node, kind)| {
                if let Some(node) = node.upgrade() {
                    node.invalidate(*kind);
                    true
                } else {
                    false
                }
            });
        drop(old);
        Ok(true)
    }
    /// Counts a write that changed the value, as [`Self::revision`]
    /// documents for every publication path.
    fn advance_revision(&self) {
        self.inner
            .revision
            .set(self.inner.revision.get().wrapping_add(1));
    }
    fn invalidate_computations(&self) {
        self.inner.computations.borrow_mut().retain(|key, weak| {
            if let Some(computation) = weak.upgrade() {
                computation.invalidate();
                self.inner
                    .shared
                    .computations
                    .borrow_mut()
                    .insert(*key, weak.clone());
                true
            } else {
                false
            }
        });
    }
}

impl<T: Clone + PartialEq> Signal<T> {
    /// Changes a copy of the current value in `change` and writes it like
    /// [`Self::set`], so an unchanged copy notifies nobody.
    ///
    /// The write capability is checked before `change` runs, so a forbidden
    /// phase never runs it. `change` receives an owned copy and holds no
    /// borrow of the signal, so it may read this signal or others. A write it
    /// makes itself, to this signal or another, is an ordinary write that takes
    /// effect at once; the changed copy is then written over it.
    ///
    /// `change` cannot fail. An edit that can fail reads the value with
    /// [`Self::get`], decides, and writes with [`Self::set`] only on success.
    /// If `change` panics, the panic propagates and this method's own write
    /// is not made, so the signal keeps its value and nobody is notified;
    /// writes `change` made itself before panicking stay made.
    ///
    /// # Errors
    /// The errors of [`Self::get`] and [`Self::set`]. They concern this
    /// method's own write, which an error leaves unmade, notifying nobody;
    /// writes `change` made itself stay made.
    pub fn update(&self, change: impl FnOnce(&mut T)) -> Result<(), RuntimeError> {
        self.inner.shared.writable()?;
        let mut value = self.get()?;
        change(&mut value);
        self.set(value)
    }
}

impl Signal<bool> {
    /// Inverts the value, as [`Self::update`] with `!`.
    ///
    /// # Errors
    /// The errors of [`Self::update`].
    pub fn toggle(&self) -> Result<(), RuntimeError> {
        self.update(|value| *value = !*value)
    }
}

impl<T: Clone> ReadSignal<T> {
    /// Clones the current value without tracking; use the update context to bind it.
    ///
    /// # Errors
    /// Returns `Closed` after runtime drop or `BorrowConflict` on a reentrant borrow.
    /// A dirty derived binding cannot evaluate during paint, layout, inspection
    /// or delivery and returns that phase's write error; read it during update
    /// first. Reading an already settled derived value is safe in those phases.
    pub fn get(&self) -> Result<T, RuntimeError> {
        self.inner.shared.readable()?;
        if let Some(computation) = &self.computation {
            computation.refresh()?;
        }
        Ok(self
            .inner
            .value
            .try_borrow()
            .map_err(|_| RuntimeError::BorrowConflict)?
            .clone())
    }
}

impl<T: 'static> ReadSignal<T> {
    pub(crate) fn shared(&self) -> &Rc<Shared> {
        &self.inner.shared
    }
    /// The stored value's write counter, as [`Signal::revision`]; for a derived
    /// binding, its published result's. Read it after reading the value, which
    /// settles a derived binding.
    pub(crate) fn revision(&self) -> u64 {
        self.inner.revision.get()
    }
    pub(crate) fn subscribe(&self, node: &Rc<Node>, kind: Invalidation) -> Subscription {
        let key = Rc::as_ptr(node) as usize;
        self.inner
            .subscribers
            .borrow_mut()
            .entry(key)
            .and_modify(|(_, old)| *old = (*old).max(kind))
            .or_insert((Rc::downgrade(node), kind));
        let weak = Rc::downgrade(&self.inner);
        Subscription {
            remove: Some(Box::new(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.subscribers.borrow_mut().remove(&key);
                }
            })),
        }
    }
}

pub(crate) struct Subscription {
    remove: Option<Box<dyn FnOnce()>>,
}

impl Subscription {
    /// Keeps `value` alive until the subscription is released, for a weakly
    /// held setting signal that a component reads.
    pub(crate) fn keep_alive<T: 'static>(value: T) -> Self {
        Self {
            remove: Some(Box::new(move || drop(value))),
        }
    }
}

pub(crate) trait Computation {
    fn invalidate(&self);
    fn refresh(&self) -> Result<(), RuntimeError>;
}

struct Derived {
    shared: Rc<Shared>,
    dirty: Cell<bool>,
    compute: RefCell<Box<dyn FnMut() -> Result<(), RuntimeError>>>,
    invalidate_dependents: Box<dyn Fn()>,
    _inputs: Vec<Subscription>,
}
impl Computation for Derived {
    fn invalidate(&self) {
        if !self.dirty.replace(true) {
            (self.invalidate_dependents)();
        }
    }
    fn refresh(&self) -> Result<(), RuntimeError> {
        if !self.dirty.get() {
            return Ok(());
        }
        self.shared.check_evaluation()?;
        let mut compute = self
            .compute
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)?;
        let _guard = self.shared.evaluating();
        compute()?;
        self.dirty.set(false);
        Ok(())
    }
}
impl Drop for Derived {
    fn drop(&mut self) {
        self.shared
            .computations
            .borrow_mut()
            .remove(&(self as *const Self as usize));
    }
}

/// Builds a derived binding: checks the phase, evaluates once under the
/// evaluation guard, and stores the result with one node that owns `sources`
/// and `evaluate` and holds the subscriptions `subscribe` makes. [`ReadSignal::map`]
/// and [`ReadSignal::combine`] share it, so their rules live here.
fn derived<S: 'static, T: Clone + PartialEq + 'static>(
    shared: Rc<Shared>,
    sources: S,
    evaluate: impl Fn(&S) -> Result<T, RuntimeError> + 'static,
    subscribe: impl FnOnce(&S, &Weak<dyn Computation>) -> Vec<Subscription>,
) -> Result<ReadSignal<T>, RuntimeError> {
    shared.check_evaluation()?;
    let initial = {
        let _guard = shared.evaluating();
        evaluate(&sources)?
    };
    let output = Signal::new(shared.clone(), initial);
    let target = output.clone();
    let downstream = output.clone();
    let derived: Rc<Derived> = Rc::new_cyclic(|weak: &Weak<Derived>| {
        let weak: Weak<dyn Computation> = weak.clone();
        let inputs = subscribe(&sources, &weak);
        Derived {
            shared,
            dirty: Cell::new(false),
            compute: RefCell::new(Box::new(move || {
                target.publish(evaluate(&sources)?).map(|_| ())
            })),
            invalidate_dependents: Box::new(move || downstream.invalidate_computations()),
            _inputs: inputs,
        }
    });
    Ok(ReadSignal {
        inner: output.read.inner,
        computation: Some(derived),
    })
}

impl<T: Clone + 'static> ReadSignal<T> {
    /// Derives a read-only value, retaining this input for the result's lifetime.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    ///
    /// The pure UI-thread function runs once here, then at most once per batch of
    /// input writes (or an earlier explicit read). Equal results do not invalidate
    /// downstream widgets. No callback runs from `Signal::set`; the runtime settles
    /// derived values before component updates. Do not mutate state or perform I/O
    /// in the function. Dropping the last result releases its subscriptions.
    /// [`Self::combine`] derives from several bindings at once.
    ///
    /// The function runs in the evaluation phase, so a write it attempts is
    /// refused: the write returns `WriteDuringUpdate` to the function and
    /// changes nothing, here and at every later evaluation. A later
    /// evaluation's error, such as a failed read of the input, is returned by
    /// the read that settles the result: an explicit [`Self::get`], or the flush
    /// that settles derived values before component updates.
    ///
    /// # Errors
    /// `Closed` after the runtime shut down; `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// when called inside that phase (a component's update may derive); then
    /// the input's read errors, such as `BorrowConflict` for a cyclic
    /// reentrant read, without calling the function.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::Runtime;
    /// let ui = Runtime::new();
    /// let count = ui.signal(2);
    /// let caption = count.map(|n| format!("{n} items"))?;
    /// count.set(3)?;
    /// assert_eq!(caption.get()?, "3 items");
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn map<U: Clone + PartialEq + 'static>(
        &self,
        map: impl Fn(&T) -> U + 'static,
    ) -> Result<ReadSignal<U>, RuntimeError> {
        derived(
            self.inner.shared.clone(),
            self.clone(),
            move |source| Ok(map(&source.get()?)),
            |source, weak| vec![source.subscribe_computation(weak.clone())],
        )
    }

    fn subscribe_computation(&self, computation: Weak<dyn Computation>) -> Subscription {
        let key = computation.as_ptr() as *const () as usize;
        self.inner
            .computations
            .borrow_mut()
            .insert(key, computation);
        let weak = Rc::downgrade(&self.inner);
        Subscription {
            remove: Some(Box::new(move || {
                if let Some(inner) = weak.upgrade() {
                    inner.computations.borrow_mut().remove(&key);
                }
            })),
        }
    }
}

/// One input of [`ReadSignal::combine`]: a [`Signal`] or a [`ReadSignal`], or a
/// reference to one.
///
/// An application never names this trait: it writes its inputs in a tuple,
/// and each element implements it. `&signal` and `&binding` work, and so does
/// a binding a call returns, such as `count.map(..)?` or `filter.items()`, or an
/// owned clone of a signal; every element is cloned into the derived binding,
/// so a reference is not kept. Only generic code over inputs names it, as a
/// bound.
///
/// The trait is sealed: only pert-ui's bindings implement it, so a value that
/// is not a binding is refused with a message saying so. A fixed value the
/// derivation needs is captured by the function instead.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ReadSignal, Runtime};
/// let ui = Runtime::new();
/// let count = ui.signal(2_u32);
/// let label = ui.signal(String::from("items"));
/// let doubled = count.map(|n| n * 2)?;
/// let caption = ReadSignal::combine(
///     (&count, &doubled, count.map(|n| n + 1)?, label.clone()),
///     |(count, doubled, next, label)| format!("{count} {label}, {doubled} doubled, {next} next"),
/// )?;
/// assert_eq!(caption.get()?, "2 items, 4 doubled, 3 next");
/// count.set(3)?;
/// assert_eq!(caption.get()?, "3 items, 6 doubled, 4 next");
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a signal or read binding",
    label = "not an input `ReadSignal::combine` can observe",
    note = "each element of the tuple is a `Signal` or `ReadSignal`, or a reference to one; capture a fixed value in the function instead"
)]
pub trait CombineInput: sealed::CombineInputSeal {
    /// The type of the value the input holds, which the derivation reads.
    type Value: Clone + 'static;
}
impl<T: Clone + 'static> CombineInput for Signal<T> {
    type Value = T;
}
impl<T: Clone + 'static> sealed::CombineInputSeal for Signal<T> {
    fn read_signal(
        &self,
        _token: sealed::Token,
    ) -> &ReadSignal<<Signal<T> as CombineInput>::Value> {
        &self.read
    }
}
impl<T: Clone + 'static> CombineInput for ReadSignal<T> {
    type Value = T;
}
impl<T: Clone + 'static> sealed::CombineInputSeal for ReadSignal<T> {
    fn read_signal(
        &self,
        _token: sealed::Token,
    ) -> &ReadSignal<<ReadSignal<T> as CombineInput>::Value> {
        self
    }
}
impl<B: CombineInput> CombineInput for &B {
    type Value = B::Value;
}
impl<'a, B: CombineInput> sealed::CombineInputSeal for &'a B {
    fn read_signal(&self, token: sealed::Token) -> &ReadSignal<<&'a B as CombineInput>::Value> {
        (**self).read_signal(token)
    }
}

/// The inputs of [`ReadSignal::combine`]: a tuple of 2 to 12
/// [`CombineInput`]s.
///
/// An application writes the tuple, `(&name, &port)`, and never names this
/// trait; generic code over a set of inputs names it as a bound, with
/// [`Self::Values`] the tuple of their values. The limit 12 is the arity to
/// which the standard library implements its tuple traits (`Debug`,
/// `PartialEq`, `Hash`, `Default`). There is no 1-tuple: a derivation from
/// one binding is [`ReadSignal::map`]. More than 12 inputs combine in steps,
/// a `combine` over a tuple of derived bindings, or related fields live in
/// one signal that `map` projects.
///
/// The trait is sealed: only pert-ui's tuples implement it, so a tuple outside
/// the limit is refused with a message naming the limit.
///
/// Twelve inputs, the most one call takes:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ReadSignal, Runtime};
/// let ui = Runtime::new();
/// let [a, b, c, d, e, f, g, h, i, j, k, l] = std::array::from_fn(|n| ui.signal(n));
/// let sum = ReadSignal::combine(
///     (&a, &b, &c, &d, &e, &f, &g, &h, &i, &j, &k, &l),
///     |(a, b, c, d, e, f, g, h, i, j, k, l)| a + b + c + d + e + f + g + h + i + j + k + l,
/// )?;
/// assert_eq!(sum.get()?, 66);
/// l.set(100)?;
/// assert_eq!(sum.get()?, 155);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Thirteen inputs do not compile, because the tuple is longer than 12:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ReadSignal, Runtime};
/// let ui = Runtime::new();
/// let a = ui.signal(1_u32);
/// let sum = ReadSignal::combine(
///     (&a, &a, &a, &a, &a, &a, &a, &a, &a, &a, &a, &a, &a),
///     |values| values.0,
/// )?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Nor does one input, because a 1-tuple is shorter than 2; write
/// `count.map(|n| *n)` instead:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ReadSignal, Runtime};
/// let ui = Runtime::new();
/// let count = ui.signal(1_u32);
/// let same = ReadSignal::combine((&count,), |(n,)| *n)?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Generic code over any inputs, here whether their values differ from the
/// applied ones:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{CombineInputs, ReadSignal, Runtime, RuntimeError};
///
/// fn changed<I>(inputs: I, applied: I::Values) -> Result<ReadSignal<bool>, RuntimeError>
/// where
///     I: CombineInputs,
///     I::Values: PartialEq,
/// {
///     ReadSignal::combine(inputs, move |values| *values != applied)
/// }
///
/// let ui = Runtime::new();
/// let name = ui.signal(String::from("server"));
/// let port = ui.signal(8080_u16);
/// let dirty = changed((&name, &port), (String::from("server"), 8080))?;
/// assert!(!dirty.get()?);
/// port.set(8443)?;
/// assert!(dirty.get()?);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a tuple of 2 to 12 bindings",
    label = "expected a tuple of signals or read bindings, such as `(&a, &b)`",
    note = "`ReadSignal::combine((&a, &b), |(a, b)| ...)` derives from 2 to 12 bindings; `ReadSignal::map` derives from one, and more than 12 combine in steps"
)]
pub trait CombineInputs: sealed::CombineInputsSeal {
    /// The inputs' values as a tuple in input order, `(A::Value, B::Value, …)`;
    /// the derivation's function receives a reference to it.
    type Values: 'static;
}

/// Implements [`CombineInputs`] for one tuple arity: the first input names the
/// runtime every other must share.
macro_rules! combine_inputs {
    ($first_input:ident $first:ident $(, $input:ident $value:ident)+) => {
        impl<$first_input: CombineInput $(, $input: CombineInput)+> CombineInputs
            for ($first_input, $($input,)+)
        {
            type Values = ($first_input::Value, $($input::Value,)+);
        }
        impl<$first_input: CombineInput $(, $input: CombineInput)+> sealed::CombineInputsSeal
            for ($first_input, $($input,)+)
        {
            fn derive<T, Map>(
                &self,
                token: sealed::Token,
                map: Map,
            ) -> Result<ReadSignal<T>, RuntimeError>
            where
                T: Clone + PartialEq + 'static,
                Map: Fn(&<($first_input, $($input,)+) as CombineInputs>::Values) -> T + 'static,
            {
                let ($first, $($value,)+) = self;
                let ($first, $($value,)+) = (
                    $first.read_signal(token).clone(),
                    $($value.read_signal(token).clone(),)+
                );
                let shared = $first.inner.shared.clone();
                $(
                    if !Rc::ptr_eq(&shared, &$value.inner.shared) {
                        return Err(RuntimeError::ForeignRuntime);
                    }
                )+
                derived(
                    shared,
                    ($first, $($value,)+),
                    move |($first, $($value,)+)| {
                        Ok(map(&($first.get()?, $($value.get()?,)+)))
                    },
                    |($first, $($value,)+), weak| {
                        vec![
                            $first.subscribe_computation(weak.clone()),
                            $($value.subscribe_computation(weak.clone()),)+
                        ]
                    },
                )
            }
        }
    };
}
combine_inputs!(A a, B b);
combine_inputs!(A a, B b, C c);
combine_inputs!(A a, B b, C c, D d);
combine_inputs!(A a, B b, C c, D d, E e);
combine_inputs!(A a, B b, C c, D d, E e, F f);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g, H h);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g, H h, I i);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k);
combine_inputs!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k, L l);

impl<T: Clone + PartialEq + 'static> ReadSignal<T> {
    /// Derives a read-only value from 2 to 12 bindings of one runtime, with
    /// the batching, equality and ownership rules of [`Self::map`].
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    ///
    /// `inputs` is a tuple of [`Signal`]s and [`ReadSignal`]s, or references
    /// to them, in any mix: `(&name, &port)`, `(filter.items(), &query)`. Each
    /// element is cloned into the result, so a reference is not kept and a
    /// signal needs no `read_only()`. Every input must belong to the runtime of
    /// the first. The limit is 12, the arity of the standard library's tuple
    /// traits; a derivation from one binding is [`Self::map`], and more than
    /// 12 inputs combine in steps, a `combine` over derived bindings, or live
    /// as related fields of one signal that `map` projects
    /// ([`CombineInputs`]).
    ///
    /// `map` receives a reference to the tuple of the inputs' values, in input
    /// order. The pattern `|(a, b)|` binds `a` and `b` as references to the
    /// values, as two parameters would; an annotated closure names the tuple,
    /// `|values: &(u32, String)|`, and a named function takes `&(A, B)`. A
    /// closure with two parameters, `|a, b|`, does not compile ("expected
    /// closure that takes 1 argument").
    ///
    /// The pure UI-thread function runs once here, reading the inputs in tuple
    /// order, and then at most once per batch of input writes however many
    /// inputs changed: the result is one derived binding with one
    /// subscription per input, so writes mark it stale and run nothing, and
    /// the runtime evaluates it before component updates, or at an earlier
    /// explicit read. Do not mutate state or perform I/O in the function. An
    /// equal result advances no revision and updates no component that reads
    /// it; a binding derived from the result was marked stale with it and
    /// still evaluates again, its own equal result stopping there.
    ///
    /// The result retains every input, a derived input with its own
    /// computation so it stays current, and the function with whatever it
    /// captured; clones of the result share them, and dropping the last clone
    /// releases the subscriptions, the inputs and the function. An input may
    /// repeat, `(&a, &a)`: both positions read the same value.
    ///
    /// The function runs in the evaluation phase, so a write it attempts is
    /// refused: the write returns `WriteDuringUpdate` to the function and
    /// changes nothing, here and at every later evaluation. A later
    /// evaluation's error, such as a failed read of an input, is returned by
    /// the read that settles the result: an explicit [`Self::get`], or the
    /// flush that settles derived values before component updates.
    ///
    /// # Errors
    /// In this order: `ForeignRuntime` when an input belongs to another
    /// runtime than the first, before any input is read or the function runs;
    /// `Closed` after the runtime shut down; `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// when called inside that phase (a component's update may derive); then
    /// the first input read error in tuple order, such as `BorrowConflict`
    /// for a reentrant read, without calling the function.
    ///
    /// A settings form's dirty flag, over three field signals and the applied
    /// record:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{ReadSignal, Runtime};
    /// let ui = Runtime::new();
    /// let name = ui.signal(String::from("server"));
    /// let port = ui.signal(8080_u16);
    /// let secure = ui.signal(true);
    /// let applied = ui.signal((String::from("server"), 8080_u16, true));
    /// let dirty = ReadSignal::combine(
    ///     (&name, &port, &secure, &applied),
    ///     |(name, port, secure, applied)| {
    ///         (name, port, secure) != (&applied.0, &applied.1, &applied.2)
    ///     },
    /// )?;
    /// assert!(!dirty.get()?);
    /// port.set(8443)?;
    /// assert!(dirty.get()?);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn combine<I: CombineInputs>(
        inputs: I,
        map: impl Fn(&I::Values) -> T + 'static,
    ) -> Result<Self, RuntimeError> {
        sealed::CombineInputsSeal::derive(&inputs, sealed::Token(()), map)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(remove) = self.remove.take() {
            remove();
        }
    }
}

impl<T> ReadSignal<T> {
    /// Whether two handles observe the same stored signal or derived computation.
    ///
    /// Compares identity without reading or evaluating values, including after
    /// shutdown. Equal values in distinct signals are not the same source. Use
    /// during `Component::reconcile` to retain caches/subscriptions when a binding
    /// has not changed; clones of one handle always share a source.
    #[must_use]
    pub fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
    pub(crate) fn same_optional(a: &Option<Self>, b: &Option<Self>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same_source(b),
            _ => false,
        }
    }
}

/// The one receipt contract's storage: an owned status that stays readable
/// after runtime shutdown, mirrored by a binding for components.
///
/// Every receipt keeps its status here and exposes `status()` from
/// [`Self::get`] and `status_binding()` from [`Self::binding`]. Only
/// library-owned statuses are stored, whose equality and drop run no user code,
/// so publication cannot fail and may happen in any phase, including a
/// component's update or teardown; after shutdown the binding reads `Closed`
/// while the owned status keeps its last value.
pub(crate) struct ReceiptStatus<S> {
    status: Rc<RefCell<S>>,
    binding: Signal<S>,
}
impl<S> Clone for ReceiptStatus<S> {
    fn clone(&self) -> Self {
        Self {
            status: self.status.clone(),
            binding: self.binding.clone(),
        }
    }
}
impl<S: Clone + PartialEq> ReceiptStatus<S> {
    pub(crate) fn new(shared: Rc<Shared>, status: S) -> Self {
        Self {
            status: Rc::new(RefCell::new(status.clone())),
            binding: Signal::new(shared, status),
        }
    }
    /// The owned status, readable after shutdown.
    pub(crate) fn get(&self) -> S {
        self.status.borrow().clone()
    }
    /// The status as a binding, which reads `Closed` after shutdown.
    pub(crate) fn binding(&self) -> ReadSignal<S> {
        self.binding.read_only()
    }
    /// Records a new status and notifies the binding's readers.
    pub(crate) fn set(&self, status: S) {
        self.status.replace(status.clone());
        self.binding.publish_trusted(status);
    }
    /// Whether two handles report the same request.
    pub(crate) fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.status, &other.status)
    }
}
