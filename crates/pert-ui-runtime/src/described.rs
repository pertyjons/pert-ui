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
/// use pert_ui::{Dynamic, IntoElement, Label, ReadSignal, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let (name, unread) = (ui.signal("Ada".to_owned()), ui.signal(2));
/// let header =
///     ReadSignal::combine((&name, &unread), |(name, unread)| (name.clone(), *unread))?;
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
/// `key` gives each item's stable key and `build` its content: a component,
/// which `Each` describes under that key, or an [`Element`], whose own key
/// `Each` replaces with it ([`EachItem`]). The items go into a container: a
/// [`Column`] by default, or what [`Self::within`] describes, so the
/// container's own layout and builders apply. When the list changes, every item
/// is described again: an item whose
/// key stays keeps its component and state, also when it moves, a new key
/// mounts and a removed key unmounts. Keys identify items, not positions, so
/// a key must be stable for an item and unique in the list; a duplicate fails
/// the flush and keeps the previous items until the list is valid again.
///
/// `key`, `build` and the container follow the rules of [`Dynamic`]'s `build`.
/// For collections too large to describe in full, use a [`VirtualList`].
///
/// The default container is internal structure, keyed `items` and marked
/// [`Element::structural`]: it adds no segment to an item's address, so an
/// item's address is the `Each` element's address followed by the item's
/// key, in [`Runtime::find`](crate::Runtime::find), inspection paths,
/// selectors, capture targets and workspace documents. Like any structural
/// wrapper it is also transparent to naming: an item the application marks
/// [`Element::structural`] takes its name from the `Each`'s label or from a
/// naming parent above the `Each`.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Each, IntoElement, Label, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
/// let root = ui.mount_tree(
///     Each::new(&tasks, |task| task.clone(), |_, task| Label::new(task.as_str())).key("tasks"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["ship"])?.is_some());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Items of different component types are returned as [`Element`]s, which one
/// closure can return for all of them; so is an item that needs an element
/// builder, such as [`Element::with_grow`]. An element needs a key to exist,
/// and `Each` replaces it with the item's:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Button, Each, Element, IntoElement, Label, Runtime};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let entries = ui.signal(vec!["Notes".to_owned(), "+".to_owned()]);
/// let root = ui.mount_tree(
///     Each::new(&entries, Clone::clone, |_, entry| -> Element {
///         if entry == "+" {
///             Button::new("Add", |_| Ok(())).key("entry")
///         } else {
///             Label::new(entry.as_str()).key("entry")
///         }
///     })
///     .key("entries"),
/// )?;
/// ui.flush(Duration::ZERO)?;
/// assert!(ui.find(&root, &["Notes"])?.is_some());
/// assert!(ui.find(&root, &["+"])?.is_some());
/// // The elements' own key, "entry", was replaced by each item's key.
/// assert!(ui.find(&root, &["entry"])?.is_none());
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// [`Self::within`] replaces the container, and its key becomes a segment of
/// the items' addresses:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Each, IntoElement, Label, Runtime, Wrap};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// let tags = ui.signal(vec!["red".to_owned(), "green".to_owned()]);
/// let root = ui.mount_tree(
///     Each::new(&tags, |tag| tag.clone(), |_, tag| Label::new(tag.as_str()))
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
///
/// The container's addressing is decided when it first mounts; to change it,
/// give the new container another key, which mounts the items again.
/// Described again under the same key, a container keeps its component and
/// its addressing, so the items keep their identities, state and addresses;
/// a container of another type under the same key fails the flush with
/// [`RuntimeError::ChildTypeChanged`].
///
/// A test reads the items in order, each with its address segment, with
/// [`Driver::address_children`](crate::testing::Driver::address_children),
/// which looks through the default container.
#[must_use]
pub struct Each<T> {
    inner: Described<Vec<T>>,
    items: ReadSignal<Vec<T>>,
    item: BuildItem<T>,
}
description_debug!(Each<T>);

impl<T: Clone + 'static> Each<T> {
    /// Describes one child per item of `items`, keyed by `key`, in a
    /// [`Column`]. `build` returns the item's content, any component or an
    /// [`Element`] ([`EachItem`]): a component is described under the item's
    /// key, and an element keeps everything but its key, which the item's
    /// replaces. The column is internal structure, transparent to addresses
    /// and naming: an item's address is this element's address followed by
    /// the item's key.
    ///
    /// Timing class of `key` and `build`: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// let mut ui = Runtime::new();
    /// let tasks = ui.signal(vec!["write".to_owned()]);
    /// let list = ui.mount_tree(
    ///     Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
    /// )?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// let row = ui.find(&list, &["write"])?.ok_or("not mounted")?;
    /// assert_eq!(ui.key(&row)?, "write");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new<R: EachItem>(
        items: impl Into<ReadSignal<Vec<T>>>,
        key: impl Fn(&T) -> String + 'static,
        build: impl Fn(&DescribeContext<'_>, &T) -> R + 'static,
    ) -> Self {
        let items: ReadSignal<Vec<T>> = items.into();
        let item: BuildItem<T> =
            Rc::new(move |cx, value| build(cx, value).into_item(key(value), sealed::Token::new()));
        let within: Within = Rc::new(|children| Column::new(children).key("items").structural());
        Self {
            inner: Described::new(items.clone(), Self::describe(item.clone(), within)),
            items,
            item,
        }
    }
    /// Places the items' elements in the container `within` describes, such as
    /// a [`Row`] or [`Wrap`], instead of a [`Column`]. The container's key is
    /// part of the items' path; keep it stable so they are retained. The
    /// element is used as it is, whatever its key, `items` included; for the
    /// default addressing with another layout, mark it
    /// [`Element::structural`].
    ///
    /// The container's addressing is decided when it first mounts; to change
    /// it, give the new container another key, which mounts the items again.
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

/// What an [`Each`]'s `build` returns for one item: any component, which
/// `Each` describes under the item's key, or an [`Element`], whose own key
/// `Each` replaces with the item's.
///
/// A component becomes `Element::new(item_key, component)`, exactly what
/// `component.key(item_key)` describes, so return the component itself, with
/// no `.key(..)`. An `Element` keeps everything but its key: its label, its
/// grow factor and its addressing marking. Return one for items of different
/// component types, which one closure can only return as `Element`, or for an
/// item that needs an element builder, such as [`Element::with_grow`] or
/// [`Element::structural`]; such an element still needs some key to exist,
/// and `Each` replaces it.
///
/// The trait is sealed and its conversion is the library's: code never
/// names or implements it or calls its conversion, it only returns a
/// component or an element from `build`, where the type is inferred.
/// Nothing else can be an item:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::EachItem;
/// struct Row;
/// impl EachItem for Row {}
/// ```
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// let ui = Runtime::new();
/// let tasks = ui.signal(vec!["write".to_owned()]);
/// // A `String` is not a component.
/// let _ = Each::new(&tasks, Clone::clone, |_, task| task.clone());
/// ```
///
/// A generic function cannot call the conversion either, without or with an
/// argument for the private token it requires:
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{EachItem, Element};
/// fn convert<R: EachItem>(value: R) -> Element {
///     value.into_item("chosen".to_owned())
/// }
/// ```
///
/// ```compile_fail
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{EachItem, Element};
/// fn convert<R: EachItem>(value: R) -> Element {
///     value.into_item("chosen".to_owned(), Default::default())
/// }
/// ```
///
/// Returning the component compiles and addresses the item by its key:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// let mut ui = Runtime::new();
/// let tags = ui.signal(vec!["red".to_owned()]);
/// let list = ui.mount_tree(
///     Each::new(&tags, Clone::clone, |_, tag| Label::new(tag.as_str())).key("tags"),
/// )?;
/// ui.flush(std::time::Duration::ZERO)?;
/// assert!(ui.find(&list, &["red"])?.is_some());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be an `Each` item",
    label = "an `Each` item is a component or an `Element`",
    note = "return the component itself, such as `Label::new(text)`, or an `Element` for items of different component types"
)]
pub trait EachItem: sealed::EachItemSeal {}
/// Every component is an item, described under the item's key.
#[diagnostic::do_not_recommend]
impl<C: Component + 'static> EachItem for C {}
/// An element is an item, with its key replaced by the item's.
impl EachItem for Element {}

mod sealed {
    use crate::{Component, Element};

    /// Proof that the conversion is called from this crate: the field is
    /// private, only [`Self::new`] constructs one and nothing outside the
    /// crate can name, construct, default or duplicate it, so no code
    /// outside it can call [`EachItemSeal::into_item`].
    #[derive(Debug)]
    pub struct Token(());
    impl Token {
        pub(crate) fn new() -> Self {
            Self(())
        }
    }

    /// Seals [`EachItem`](super::EachItem) and carries its conversion. The
    /// distinct name keeps rustc from pointing at a dependency's own
    /// `Sealed` trait when code tries to implement `EachItem`.
    pub trait EachItemSeal {
        /// The item's element under `key`.
        fn into_item(self, key: String, token: Token) -> Element;
    }
    #[diagnostic::do_not_recommend]
    impl<C: Component + 'static> EachItemSeal for C {
        fn into_item(self, key: String, _: Token) -> Element {
            Element::new(key, self)
        }
    }
    impl EachItemSeal for Element {
        fn into_item(mut self, key: String, _: Token) -> Element {
            self.key = key;
            self
        }
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
