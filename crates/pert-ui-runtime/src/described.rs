//! Children described from a binding's value: [`Dynamic`], [`Show`] and
//! [`Each`].
//!
//! All three share one retained core. It reads its binding in `update` and
//! requests its children again only when the binding was written with a new
//! value or the component was described again, so an unchanged value costs
//! one comparison of write counters. Children
//! that keep their key and type across a new description keep their state, as
//! in any structural update.
use crate::*;
use std::rc::Rc;

/// The state owner a description callback of [`Dynamic`], [`Show`] or
/// [`Each`] receives, for constructors that create their own UI-local state,
/// such as [`Select::new`].
///
/// It only creates state: the callback runs during the component's update,
/// where writes are rejected, and it offers no tracked reads and cannot change
/// structure. Reading a captured signal with `get` inside the callback creates
/// no dependency, so a change of that signal alone describes nothing again:
/// combine every input the description depends on into the source binding,
/// for example with [`ReadSignal::combine`]. A signal created here is new on
/// every description, so a control bound to it starts over whenever the value
/// changes; keep state that must survive in signals created outside the
/// callback.
pub struct DescribeContext<'a>(&'a UpdateContext);

/// Shows nothing of the component being described.
impl std::fmt::Debug for DescribeContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DescribeContext").finish_non_exhaustive()
    }
}

impl crate::signal::sealed::StateOwnerSeal for DescribeContext<'_> {}
impl StateOwner for DescribeContext<'_> {
    fn signal<T>(&self, value: T) -> Signal<T> {
        self.0.signal(value)
    }
    fn ui_settings(&self) -> UiSettings {
        self.0.ui_settings()
    }
}

type Describe<T> = Rc<dyn Fn(&DescribeContext<'_>, &T) -> Vec<Element>>;

/// The shared core: a binding, how to describe children from its value, and
/// the binding's write counter when they were last described.
struct Described<T> {
    source: ReadSignal<T>,
    describe: Describe<T>,
    /// Compared instead of the value, so a value with shared interior
    /// mutability cannot hide a write; the binding itself counts only writes
    /// that changed its value.
    shown: Option<u64>,
    /// Set by a new description, whose callback may describe other children
    /// for the same value.
    stale: bool,
}

impl<T: Clone + 'static> Described<T> {
    fn new(source: ReadSignal<T>, describe: Describe<T>) -> Self {
        Self {
            source,
            describe,
            shown: None,
            stale: true,
        }
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.source = next.source;
        self.describe = next.describe;
        self.stale = true;
        Some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let value = cx.read(&self.source)?;
        let revision = self.source.revision();
        if self.stale || self.shown != Some(revision) {
            let children = (self.describe)(&DescribeContext(cx), &value);
            cx.request_children(children);
            self.shown = Some(revision);
            self.stale = false;
        }
        Ok(())
    }
}

/// Implements the shared pass-through layout and lifecycle for a wrapper of
/// [`Described`].
macro_rules! described_component {
    ($name:ident $(<$param:ident>)?) => {
        impl$(<$param: Clone + 'static>)? Component for $name$(<$param>)? {
            fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
                self.0.reconcile(next.0)
            }
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.0.update(cx)
            }
            fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
                if cx.is_empty() {
                    Ok(SizeLimits::default())
                } else {
                    cx.limits(0)
                }
            }
            fn clips_children(&self) -> bool {
                false
            }
        }
    };
}

/// One child described from a binding's value, described again whenever the
/// value changes.
///
/// The description depends only on the binding: derive one that combines
/// everything the child's structure depends on, as the second example does.
///
/// `build` returns the child for the current value; the child fills this
/// component's allocation, which measures as the child does. A new value
/// describes the child again: a child that keeps its key is reconciled and
/// keeps its state, focus and drafts, and a child under another key replaces
/// the old one, which unmounts with its state. A kept key must keep its
/// component type, as in any structural update; describe a different kind of
/// child under a different key. Describing
/// the `Dynamic` itself again, from an ancestor's new description, always
/// describes its child again with the new `build`, even for an unchanged value.
///
/// `build` runs during update: it describes and must not write state or
/// perform I/O. It receives a [`DescribeContext`] for constructors that need a
/// state owner. An invalid description, such as duplicate sibling keys or a
/// kept key with another component type, fails the flush and keeps the
/// previous child until a later value or description is valid.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Button, Dynamic, IntoElement, Label, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let signed_in = ui.signal(false);
/// let root = ui.mount_tree(
///     Dynamic::new(&signed_in, |_, signed_in| {
///         if *signed_in {
///             Label::new("Welcome back").key("greeting")
///         } else {
///             Button::new("Sign in", |_| Ok(())).key("sign-in")
///         }
///     })
///     .key("account"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["sign-in"])?.is_some());
/// signed_in.set(true)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["greeting"])?.is_some());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// A description that depends on two values combines them into one binding,
/// so a change of either describes the child again:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Dynamic, IntoElement, Label, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let (name, unread) = (ui.signal("Ada".to_owned()), ui.signal(2));
/// let header = name.combine(&unread, |name, unread| (name.clone(), *unread))?;
/// let root = ui.mount_tree(
///     Dynamic::new(header, |_, (name, unread)| {
///         if *unread == 0 {
///             Label::new(name.as_str()).key("plain")
///         } else {
///             Label::new(format!("{name} ({unread})")).key("unread")
///         }
///     })
///     .key("header"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// unread.set(0)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["plain"])?.is_some());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct Dynamic<T>(Described<T>);
description_debug!(Dynamic<T>);

impl<T: Clone + 'static> Dynamic<T> {
    /// Describes one child from `source`'s value with `build`.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    pub fn new(
        source: impl Into<ReadSignal<T>>,
        build: impl Fn(&DescribeContext<'_>, &T) -> Element + 'static,
    ) -> Self {
        Self(Described::new(
            source.into(),
            Rc::new(move |cx, value| vec![build(cx, value)]),
        ))
    }
}
described_component!(Dynamic<T>);

/// A child that exists only while a binding reads true.
///
/// While `when` reads true, `build` describes the child, which fills this
/// component's allocation; while it reads false there is no child, the
/// component measures zero and the child is unmounted with its state, focus
/// and subscriptions. Showing it again builds a fresh child. To hide content
/// while keeping its state, use [`View::visible_signal`] instead.
///
/// Describing the `Show` itself again, from an ancestor's new description,
/// describes a shown child again with the new `build`. `build` follows the
/// rules of [`Dynamic`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{IntoElement, Label, Runtime, Show};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let details = ui.signal(false);
/// let root = ui.mount_tree(
///     Show::new(&details, |_| Label::new("Advanced settings").key("advanced"))
///         .key("details"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.children(&root)?.is_empty());
/// details.toggle()?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(ui.children(&root)?.len(), 1);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct Show(Described<bool>);
description_debug!(Show);

impl Show {
    /// Describes a child with `build` while `when` reads true.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    pub fn new(
        when: impl Into<ReadSignal<bool>>,
        build: impl Fn(&DescribeContext<'_>) -> Element + 'static,
    ) -> Self {
        Self(Described::new(
            when.into(),
            Rc::new(move |cx, shown| if *shown { vec![build(cx)] } else { Vec::new() }),
        ))
    }
}
described_component!(Show);

type Within = Rc<dyn Fn(Vec<Element>) -> Element>;
type BuildItem<T> = Rc<dyn Fn(&DescribeContext<'_>, &T) -> Element>;

/// One child per item of a binding's list, each under the key the item gives,
/// inside a container.
///
/// `key` gives each item's stable key and `build` its element, which `Each`
/// keys with that key, replacing the element's own. The items' elements go
/// into a container: a [`Column`] keyed `items` by default, or what
/// [`Self::within`] describes, so the container's own layout and builders
/// apply. When the list changes, every item is described again: an item whose
/// key stays keeps its component and state, also when it moves, a new key
/// mounts and a removed key unmounts. Keys identify items, not positions, so
/// a key must be stable for an item and unique in the list; a duplicate fails
/// the flush and keeps the previous items until the list is valid again.
///
/// `key`, `build` and the container follow the rules of [`Dynamic`]'s `build`.
/// For collections too large to describe in full, use a [`VirtualList`].
///
/// The default container adds its key to an item's path, so a lookup from the
/// `Each` element goes through `items`:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Each, IntoElement, Label, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
/// let root = ui.mount_tree(
///     Each::new(&tasks, |task| task.clone(), |_, task| Label::new(task.as_str()).key("task"))
///         .key("tasks"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["items", "ship"])?.is_some());
/// // The element's own key, "task", was replaced by the item's key.
/// assert!(ui.find(&root, &["items", "task"])?.is_none());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// [`Self::within`] replaces the container, and its key replaces `items`:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Each, IntoElement, Label, Runtime, Wrap};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let tags = ui.signal(vec!["red".to_owned(), "green".to_owned()]);
/// let root = ui.mount_tree(
///     Each::new(&tags, |tag| tag.clone(), |_, tag| Label::new(tag.as_str()).key("tag"))
///         .within(|tags| Wrap::new(tags).key("tags"))
///         .key("tag-list"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// let red = ui.find(&root, &["tags", "red"])?;
/// tags.update(|tags| tags.push("blue".to_owned()))?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(ui.find(&root, &["tags", "red"])?, red);
/// assert!(ui.find(&root, &["tags", "blue"])?.is_some());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct Each<T> {
    inner: Described<Vec<T>>,
    items: ReadSignal<Vec<T>>,
    item: BuildItem<T>,
}
description_debug!(Each<T>);

impl<T: Clone + 'static> Each<T> {
    /// Describes one element per item of `items`, keyed by `key`, in a
    /// [`Column`] keyed `items`.
    ///
    /// Timing class of `key` and `build`: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    pub fn new(
        items: impl Into<ReadSignal<Vec<T>>>,
        key: impl Fn(&T) -> String + 'static,
        build: impl Fn(&DescribeContext<'_>, &T) -> Element + 'static,
    ) -> Self {
        let items: ReadSignal<Vec<T>> = items.into();
        let item: BuildItem<T> = Rc::new(move |cx, value| {
            let mut element = build(cx, value);
            element.key = key(value);
            element
        });
        let within: Within = Rc::new(|children| Column::new(children).key("items"));
        Self {
            inner: Described::new(items.clone(), Self::describe(item.clone(), within)),
            items,
            item,
        }
    }
    /// Places the items' elements in the container `within` describes, such as
    /// a [`Row`] or [`Wrap`], instead of a [`Column`]. The container's key is
    /// part of the items' path; keep it stable so they are retained.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    pub fn within(mut self, within: impl Fn(Vec<Element>) -> Element + 'static) -> Self {
        self.inner = Described::new(
            self.items.clone(),
            Self::describe(self.item.clone(), Rc::new(within)),
        );
        self
    }
    fn describe(item: BuildItem<T>, within: Within) -> Describe<Vec<T>> {
        Rc::new(move |cx, values| {
            vec![within(values.iter().map(|value| item(cx, value)).collect())]
        })
    }
}

impl<T: Clone + 'static> Component for Each<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.items = next.items;
        self.item = next.item;
        self.inner.reconcile(next.inner)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.inner.update(cx)
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            Ok(SizeLimits::default())
        } else {
            cx.limits(0)
        }
    }
    fn clips_children(&self) -> bool {
        false
    }
}
