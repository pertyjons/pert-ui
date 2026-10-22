//! The payload and drop description shared by reorderable collections.
use crate::DropOperation;

/// What a reorderable collection drags: the application's stable keys of the
/// dragged items, in the collection's order.
///
/// It is the payload type [`VirtualList::reorderable`](crate::VirtualList::reorderable)
/// starts a session with and the only one it accepts, so two lists over the same
/// kind of keys exchange items without further code. A custom drop target reads
/// it from [`DragEvent::payload`](crate::DragEvent::payload) like any other
/// payload, and a custom source may offer it to a list.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{DragPayload, ListDragItems};
/// let payload = DragPayload::new(ListDragItems::new(["kick".into(), "snare".into()]));
/// let items = payload.downcast_ref::<ListDragItems>().expect("list items");
/// assert_eq!(items.keys(), ["kick", "snare"]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ListDragItems {
    keys: Vec<String>,
}

impl ListDragItems {
    /// Wraps the dragged items' keys, in order.
    pub fn new(keys: impl IntoIterator<Item = String>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }
    /// The dragged items' keys, in the source collection's order.
    #[must_use]
    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

/// Where dropped items go, expressed against the target's current keys.
///
/// A position never names an index: the collection may change while the pointer
/// moves, and the application resolves the key against its own model when the
/// drop arrives. In a [`Tree`](crate::Tree), `Before` and `After` make the items
/// siblings of the named node under its parent, `After` following the node's
/// whole subtree, and `Into` makes them children of the node.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ListDropPosition {
    /// Immediately before the item with this key.
    Before(String),
    /// Immediately after the item with this key.
    After(String),
    /// After the last item, or into an empty collection. In a
    /// [`Tree`](crate::Tree), after the last top-level node.
    End,
    /// Into the node with this key, as its last child. Only a
    /// [`Tree`](crate::Tree) offers it, and only over a node whose
    /// [`TreeItems::can_drop_into`](crate::TreeItems::can_drop_into) answers
    /// `true`.
    Into(String),
}

/// One completed drop on a reorderable collection, handed to its callback.
#[derive(Debug)]
pub struct ListDrop<'a> {
    pub(crate) items: &'a [String],
    pub(crate) position: ListDropPosition,
    pub(crate) operation: DropOperation,
    pub(crate) internal: bool,
}

impl ListDrop<'_> {
    /// Keys of the dropped items, in their source's order. They may include the
    /// item the position names, which makes the drop a no-op the application is
    /// free to ignore.
    #[must_use]
    pub fn items(&self) -> &[String] {
        self.items
    }
    /// Where they go.
    #[must_use]
    pub fn position(&self) -> &ListDropPosition {
        &self.position
    }
    /// What the drop does with them. A move is **one** edit made by this
    /// callback: remove the items where they are and insert them here.
    #[must_use]
    pub const fn operation(&self) -> DropOperation {
        self.operation
    }
    /// Whether the items came from this same collection, so the drop reorders it.
    #[must_use]
    pub const fn is_internal(&self) -> bool {
        self.internal
    }
}
