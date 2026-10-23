//! An expandable hierarchy presented through the virtualized list.
use crate::{
    ArrangeContext, ColorRole, Component, ComponentDetails, Constraints, DrawError, DropOperations,
    Element, EventContext, Icon, InputOutcome, Invalidation, KeyCode, KeyEvent, KeyTransition,
    KeyboardState, LayoutError, ListDrop, ListDropPosition, ListItems, ListSelection, ListSource,
    MeasureContext, MetricRole, Painter, Permission, Point, PointerButton, PointerEvent,
    PointerKind, ReadSignal, Rect, RuntimeError, SemanticAction, SemanticRole, Signal, Size,
    UpdateContext, VirtualList,
    list_selection::Gesture,
    virtual_list::{ListController, ListDropMark as DropMark, ListDropPlacement, ListDropPoint},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
};

/// A hierarchy addressed by stable key, asked for only as far as it is expanded.
///
/// Keys are node identity and must be unique in the whole tree, not only among
/// siblings. `children` is called only for expanded nodes, so a source can
/// produce children lazily. `build` returns the row content for a node, an
/// element whose key equals the node's.
/// Implementations are UI-local and read-only while the tree is flattened.
/// An implementation is `'static`, as [`TreeSource::new`] requires: it owns its
/// nodes or shares them through an `Rc`, never borrowing from a local or a
/// component (see [`Component::content_rows`](crate::Component::content_rows)).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a tree hierarchy",
    label = "`{Self}` does not implement `TreeItems`",
    note = "implement `TreeItems` (`roots`, `children`, `build`) on a type that owns or shares the hierarchy"
)]
pub trait TreeItems {
    /// The top-level nodes, in order.
    fn roots(&self) -> Vec<String>;
    /// The children of `key`, in order; empty for a leaf.
    fn children(&self, key: &str) -> Vec<String>;
    /// Whether `key` can expand. The default asks for its children, so a lazy
    /// source should answer this without producing them.
    fn can_expand(&self, key: &str) -> bool {
        !self.children(key).is_empty()
    }
    /// The row content of `key`, or `None` for an unknown node. The tree keys
    /// the content with `key`; its own key is replaced.
    fn build(&self, key: &str) -> Option<Element>;
    /// Whether a [reorderable](Tree::reorderable) tree offers dropping into
    /// `key`, making the dropped nodes its children. `false` by default, so a
    /// reorderable tree moves nodes only before and after others until the
    /// application names the nodes that take children, such as folders; an
    /// empty or collapsed folder can then still receive a node. It is asked
    /// while a drag moves over the tree, so it should answer without producing
    /// children.
    fn can_drop_into(&self, key: &str) -> bool {
        let _ = key;
        false
    }
    /// Searchable text of the node `key`, or `None` when it offers none (the
    /// default).
    ///
    /// Content search reads it for shown rows that are not mounted; a node
    /// inside a collapsed parent is not a row and is not searched. The rules
    /// of [`ListItems::text`] apply: deterministic, free of side effects and
    /// I/O, and never forcing a lazy source to list children.
    fn text(&self, key: &str) -> Option<String> {
        let _ = key;
        None
    }
    /// The children of `key` the source already holds, or `None` when it would
    /// have to produce or load them first.
    ///
    /// A [filter](TreeController::filter) walks the whole known hierarchy
    /// through this, including collapsed nodes, so it must never start a
    /// listing or I/O. The default answers with [`Self::children`] for a node
    /// that [can expand](Self::can_expand) and nothing for a leaf, which suits
    /// a source holding its whole hierarchy; a lazy source, such as one whose
    /// folders a host lists on request, returns `None` for a node it has not
    /// listed, and the filter then covers only what is listed.
    fn known_children(&self, key: &str) -> Option<Vec<String>> {
        Some(if self.can_expand(key) {
            self.children(key)
        } else {
            Vec::new()
        })
    }
}

/// A shared, replaceable hierarchy for a [`TreeController`], equal only to itself.
#[derive(Clone)]
pub struct TreeSource(Rc<dyn TreeItems>);
impl TreeSource {
    /// Wraps a hierarchy.
    pub fn new(items: impl TreeItems + 'static) -> Self {
        Self(Rc::new(items))
    }
}
impl core::fmt::Debug for TreeSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TreeSource").finish_non_exhaustive()
    }
}

#[derive(Clone)]
struct FlatRow {
    key: String,
    depth: usize,
    expandable: bool,
    expanded: bool,
    parent: Option<usize>,
}
/// The expanded nodes as ordinary list rows.
struct Flat {
    rows: Vec<FlatRow>,
    index: BTreeMap<String, usize>,
    source: TreeSource,
    // Weak, because the state owns this flattening: a strong link back would be
    // a cycle that keeps both alive after the application drops the state.
    state: Weak<StateInner>,
}
impl ListItems for Flat {
    fn len(&self) -> usize {
        self.rows.len()
    }
    fn text(&self, index: usize) -> Option<String> {
        self.source.0.text(&self.rows.get(index)?.key)
    }
    fn key(&self, index: usize) -> Option<String> {
        self.rows.get(index).map(|row| row.key.clone())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let row = self.rows.get(index)?;
        let mut content = self.source.0.build(&row.key)?;
        content.key.clone_from(&row.key);
        Some(Element::new(
            row.key.clone(),
            TreeRow {
                content: Some(content),
                key: row.key.clone(),
                depth: row.depth,
                expandable: row.expandable,
                expanded: row.expanded,
                state: self.state.clone(),
                slot: Cell::new(None),
                hovered_expander: false,
            },
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.index.get(key).copied()
    }
}

struct StateInner {
    source: RefCell<TreeSource>,
    expanded: Signal<BTreeSet<String>>,
    items: Signal<ListSource>,
    flat: RefCell<Rc<Flat>>,
    selection: ListSelection,
    /// Serves [`TreeController::reveal`] through the tree's list.
    reveal: ListController,
    /// The active filter, if any; see [`TreeController::filter`].
    filter: RefCell<Option<Filtering>>,
    counts: Signal<crate::FilterCounts>,
    /// Source replacements: a revision and the nearest remaining row when the
    /// replacement hid the active node, which a filter wrapper applies.
    replaced: Signal<(u64, Option<String>)>,
}

/// A tree's filter: the query, what it found and the temporary expansion
/// layer that shows the matches.
#[derive(Clone)]
struct Filtering {
    query: String,
    matcher: crate::FilterMatcher,
    /// Matching nodes and their ancestors.
    visible: BTreeSet<String>,
    /// Ancestors of matches: the nodes the layer opened for this query.
    opened: BTreeSet<String>,
    /// The expansion shown while filtering, starting as `opened`; expanding
    /// or collapsing while filtering changes only this.
    layer: BTreeSet<String>,
}

/// What a filter found in the hierarchy a source already knows.
struct Found {
    matches: usize,
    /// Matching nodes and their ancestors.
    visible: BTreeSet<String>,
    /// Ancestors of matches.
    opened: BTreeSet<String>,
    /// Nodes examined.
    total: usize,
}

/// Walks the hierarchy `source` already knows, matching `query` when given.
fn search(source: &TreeSource, query: Option<(&str, &crate::FilterMatcher)>) -> Found {
    let mut parents: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut found = Found {
        matches: 0,
        visible: BTreeSet::new(),
        opened: BTreeSet::new(),
        total: 0,
    };
    // Depth-first without recursion; a key met twice is examined once.
    let mut pending: Vec<(String, Option<String>)> = source
        .0
        .roots()
        .into_iter()
        .rev()
        .map(|key| (key, None))
        .collect();
    while let Some((key, parent)) = pending.pop() {
        if parents.contains_key(&key) {
            continue;
        }
        parents.insert(key.clone(), parent.clone());
        if let Some((query, matcher)) = query
            && source
                .0
                .text(&key)
                .is_some_and(|text| matcher.matches(query, &text))
        {
            found.matches += 1;
            found.visible.insert(key.clone());
            let mut at = parent;
            while let Some(ancestor) = at {
                found.visible.insert(ancestor.clone());
                if !found.opened.insert(ancestor.clone()) {
                    break;
                }
                at = parents.get(&ancestor).cloned().flatten();
            }
        }
        if let Some(children) = source.0.known_children(&key) {
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, Some(key.clone()))),
            );
        }
    }
    found.total = parents.len();
    found
}

/// Application-owned controller of a [`Tree`]: its hierarchy, which nodes are
/// expanded, and its [`ListSelection`].
///
/// Every change flattens the expanded nodes into the rows the list shows and
/// publishes them, so it must come from application or event code, never from a
/// component's `update`. Flattening costs the number of visible nodes, not the
/// size of the hierarchy. Rows that keep their key keep their component across a
/// change. Collapsing a node that hides the active item makes that node active
/// and the anchor of the next range, so keyboard navigation continues from what
/// can be seen; in [`SelectionMode::Single`](crate::SelectionMode::Single) the
/// selection follows, as it always follows the active item, and in
/// [`SelectionMode::Multiple`](crate::SelectionMode::Multiple) the selected keys
/// are left as they were. An active item that is hidden for another reason,
/// such as being moved into a collapsed node or removed by
/// [`TreeController::set_source`], stays active: only the application knows whether
/// it is gone, and navigation then starts as if nothing were active. Expansion
/// is by key and is not pruned when a source no longer has a node. Clones share
/// the same state.
#[derive(Clone)]
#[must_use]
pub struct TreeController(Rc<StateInner>);

/// Shows the selection's mode, not the nodes, expansion or filter: formatting
/// must not read a signal inside a runtime phase.
impl std::fmt::Debug for TreeController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeController")
            .field("selection", &self.0.selection)
            .finish_non_exhaustive()
    }
}
impl TreeController {
    /// Creates a controller with every node collapsed.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns `Closed` when the state owner's runtime has shut down.
    /// Creating the initial collapsed snapshot does not write existing signals
    /// or require an event phase.
    pub fn new(
        ui: &impl crate::StateOwner,
        selection: ListSelection,
        source: TreeSource,
    ) -> Result<Self, RuntimeError> {
        let expanded = ui.signal(BTreeSet::new());
        expanded.get()?;
        // Initialize the collapsed snapshot directly: construction is also
        // supported while an UpdateContext is describing its children.
        let mut rows = Vec::new();
        let mut index = BTreeMap::new();
        for key in source.0.roots() {
            if index.contains_key(&key) {
                continue;
            }
            index.insert(key.clone(), rows.len());
            rows.push(FlatRow {
                expandable: source.0.can_expand(&key),
                key,
                depth: 0,
                expanded: false,
                parent: None,
            });
        }
        Ok(Self(Rc::new_cyclic(|weak: &Weak<StateInner>| {
            let flat = Rc::new(Flat {
                rows,
                index,
                source: source.clone(),
                state: weak.clone(),
            });
            StateInner {
                source: RefCell::new(source),
                expanded,
                items: ui.signal(ListSource::from_shared(flat.clone())),
                flat: RefCell::new(flat),
                selection,
                reveal: ListController::new(ui),
                filter: RefCell::new(None),
                counts: ui.signal(crate::FilterCounts::default()),
                replaced: ui.signal((0, None)),
            }
        })))
    }
    /// The selection this tree navigates and draws.
    pub fn selection(&self) -> ListSelection {
        self.0.selection.clone()
    }
    /// The expanded node keys the user chose. A [filter](Self::filter) shows
    /// its matches through a temporary layer and leaves this unchanged.
    #[must_use]
    pub fn expanded(&self) -> ReadSignal<BTreeSet<String>> {
        self.0.expanded.read_only()
    }
    /// Whether `key` is expanded as shown: in the filter's temporary layer
    /// while a [filter](Self::filter) applies, in [`Self::expanded`] otherwise.
    ///
    /// # Errors
    /// The signal read's failures: `Closed` or `BorrowConflict`.
    pub fn is_expanded(&self, key: &str) -> Result<bool, RuntimeError> {
        if let Some(filter) = &*self.0.filter.borrow() {
            return Ok(filter.layer.contains(key));
        }
        Ok(self.0.expanded.get()?.contains(key))
    }
    /// Expands or collapses `key`. Returns whether anything changed. While a
    /// [filter](Self::filter) applies this changes only its temporary layer,
    /// so clearing the filter restores the user's expansion.
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn set_expanded(&self, key: &str, expanded: bool) -> Result<bool, RuntimeError> {
        self.0.items.shared().writable()?;
        let layered = self.0.filter.borrow_mut().as_mut().map(|filter| {
            if expanded {
                filter.layer.insert(key.to_owned())
            } else {
                filter.layer.remove(key)
            }
        });
        if let Some(changed) = layered {
            if changed && let Err(error) = self.publish_with(true) {
                // A refused write leaves the layer as it was.
                if let Some(filter) = self.0.filter.borrow_mut().as_mut() {
                    if expanded {
                        filter.layer.remove(key);
                    } else {
                        filter.layer.insert(key.to_owned());
                    }
                }
                return Err(error);
            }
            return Ok(changed);
        }
        let mut keys = self.0.expanded.get()?;
        let changed = if expanded {
            keys.insert(key.to_owned())
        } else {
            keys.remove(key)
        };
        if changed {
            self.0.expanded.set(keys)?;
            self.publish_with(true)?;
        }
        Ok(changed)
    }
    /// Flips whether `key` is expanded.
    ///
    /// # Errors
    /// Same as [`TreeController::set_expanded`].
    pub fn toggle(&self, key: &str) -> Result<(), RuntimeError> {
        let expanded = self.is_expanded(key)?;
        self.set_expanded(key, !expanded).map(|_| ())
    }
    /// Replaces the hierarchy, keeping expansion and selection by key.
    ///
    /// # Errors
    /// Same as [`TreeController::set_expanded`].
    pub fn set_source(&self, source: TreeSource) -> Result<(), RuntimeError> {
        self.0.items.shared().writable()?;
        let before = self.flat();
        let previous_source = std::mem::replace(&mut *self.0.source.borrow_mut(), source.clone());
        let previous_filter = self.0.filter.borrow().clone();
        // A filter searches the new hierarchy again. Ancestors it opened before
        // keep the user's layer choice; ancestors of new matches open.
        let counts = self.0.filter.borrow_mut().as_mut().map(|filter| {
            let found = search(&source, Some((&filter.query, &filter.matcher)));
            filter.layer = found
                .opened
                .iter()
                .filter(|key| !filter.opened.contains(*key) || filter.layer.contains(*key))
                .cloned()
                .collect();
            filter.visible = found.visible;
            filter.opened = found.opened;
            crate::FilterCounts {
                shown: found.matches,
                total: found.total,
            }
        });
        if let Err(error) = self.publish() {
            // A refused write keeps the previous hierarchy and filter, which
            // the published rows still show.
            *self.0.source.borrow_mut() = previous_source;
            *self.0.filter.borrow_mut() = previous_filter;
            return Err(error);
        }
        if let Some(counts) = counts {
            self.0.counts.set(counts)?;
        } else if self.0.counts.get()?.total != 0 {
            // Counted before by a cleared filter: keep the total current. A
            // tree never filtered pays no walk.
            let total = search(&source, None).total;
            self.0.counts.set(crate::FilterCounts {
                shown: total,
                total,
            })?;
        }
        let after = self.flat();
        let nearest = self
            .0
            .selection
            .active()
            .get()?
            .and_then(|key| crate::collection_filter::nearest(&*before, &*after, &key));
        let revision = self.0.replaced.get()?.0.wrapping_add(1);
        self.0.replaced.set((revision, nearest))
    }
    /// Source replacements with the nearest remaining row, for
    /// [`CollectionFilter`](crate::CollectionFilter).
    pub(crate) fn replaced(&self) -> ReadSignal<(u64, Option<String>)> {
        self.0.replaced.read_only()
    }
    /// Shows only the nodes whose [`TreeItems::text`] matches `query` under
    /// `matcher`, each with its ancestors, and keeps applying it when the
    /// source is replaced; an empty or whitespace query clears the filter. Any
    /// other query is trimmed of leading and trailing whitespace before
    /// `matcher` sees it, as for
    /// [`ListFilter::filter`](crate::ListFilter::filter), so code that repeats
    /// the match passes the trimmed query.
    ///
    /// The filter walks every node the source already knows, through
    /// [`TreeItems::known_children`], collapsed or not, so its cost is that
    /// hierarchy's size; it never asks a lazy source for children it has not
    /// listed, so such a source is filtered only over what it has listed. The
    /// ancestors of matches are expanded through a temporary layer, not in
    /// [`Self::expanded`]: expanding or collapsing while filtering changes only
    /// the layer, and clearing the filter shows the user's expansion again. A
    /// node shown only as an ancestor expands only to the visible nodes below
    /// it, and a match's own children are hidden unless they match. Rows that
    /// stay keep their keys and components, and the selection is never pruned.
    /// Replacing the source while filtering searches it again; ancestors the
    /// filter had opened keep the layer's choice and ancestors of new matches
    /// open. The returned outcome names the nearest remaining row when the
    /// active node disappeared; this method does not move it.
    /// [`FilterCounts::shown`](crate::FilterCounts) counts the matches, not
    /// their ancestors.
    ///
    /// # Errors
    /// Same as [`Self::set_expanded`].
    pub fn filter(
        &self,
        query: &str,
        matcher: &crate::FilterMatcher,
    ) -> Result<crate::FilterOutcome, RuntimeError> {
        self.0.items.shared().writable()?;
        let before = self.flat();
        let source = self.0.source.borrow().clone();
        let trimmed = crate::collection_filter::effective(query);
        let found = search(&source, trimmed.map(|query| (query, matcher)));
        let counts = match trimmed {
            Some(_) => crate::FilterCounts {
                shown: found.matches,
                total: found.total,
            },
            None => crate::FilterCounts {
                shown: found.total,
                total: found.total,
            },
        };
        let next = trimmed.map(|query| Filtering {
            query: query.to_owned(),
            matcher: matcher.clone(),
            visible: found.visible,
            layer: found.opened.clone(),
            opened: found.opened,
        });
        let previous = std::mem::replace(&mut *self.0.filter.borrow_mut(), next);
        if let Err(error) = self.publish() {
            *self.0.filter.borrow_mut() = previous;
            return Err(error);
        }
        self.0.counts.set(counts)?;
        let after = self.flat();
        let nearest_active = self
            .0
            .selection
            .active()
            .get()?
            .and_then(|key| crate::collection_filter::nearest(&*before, &*after, &key));
        Ok(crate::FilterOutcome {
            counts,
            nearest_active,
        })
    }
    /// The counts of the last [filter](Self::filter); zero before the first.
    #[must_use]
    pub fn filter_counts(&self) -> ReadSignal<crate::FilterCounts> {
        self.0.counts.read_only()
    }
    /// Asks the tree to scroll to and mount the row of the node `key`.
    ///
    /// A request, served like [`ListController::reveal`](crate::ListController::reveal):
    /// the tree resolves the key among the rows it shows at its next update,
    /// scrolls only as far as needed to show that row, and mounts it. The
    /// returned [`ListRevealReceipt`](crate::ListRevealReceipt) reports the
    /// row's `WidgetId` once it is mounted, and
    /// [`ListRevealStatus::NotFound`](crate::ListRevealStatus::NotFound) for a
    /// key that is unknown or inside a collapsed node. The tree does
    /// not expand ancestors, since [`TreeItems`] names children, not parents:
    /// expand them first with [`TreeController::set_expanded`], which publishes
    /// the rows at once, so a reveal made right after it finds the node. The
    /// selection is left as it is. A tree serves only requests made after it was
    /// created over this controller, so a tree rebuilt over a long-lived
    /// controller opens at its start rather than at an old target.
    ///
    /// This reveal controller supports one mounted tree at a time, including
    /// a hidden tree. Reuse it only after that owner is removed; concurrent
    /// owners do not arbitrate requests. See [`ListController`](crate::ListController).
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn reveal(&self, key: impl Into<String>) -> Result<crate::ListRevealReceipt, RuntimeError> {
        self.0.reveal.reveal(key)
    }
    fn items(&self) -> ReadSignal<ListSource> {
        self.0.items.read_only()
    }
    fn flat(&self) -> Rc<Flat> {
        self.0.flat.borrow().clone()
    }
    // Flattens the expanded nodes and publishes them as the list's rows.
    /// Publishes the flattening without moving the active row for a change
    /// that was not a collapse, such as a filter or a new source.
    fn publish(&self) -> Result<(), RuntimeError> {
        self.publish_with(false)
    }
    /// Publishes the flattening. `collapse` says the change was an expansion
    /// change, so an active row it hid moves to the node that hid it; a
    /// filter or a new source never moves the active row, as the selection is
    /// never pruned.
    fn publish_with(&self, collapse: bool) -> Result<(), RuntimeError> {
        self.0.items.shared().writable()?;
        let source = self.0.source.borrow().clone();
        let filter = self.0.filter.borrow();
        // While filtering, only matches and their ancestors are nodes, below the
        // children the source already knows, and the layer decides expansion.
        let visible = filter.as_ref().map(|filter| filter.visible.clone());
        let shown = |key: &String| visible.as_ref().is_none_or(|visible| visible.contains(key));
        let children = |key: &str| -> Vec<String> {
            match &visible {
                Some(_) => source
                    .0
                    .known_children(key)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|child| shown(child))
                    .collect(),
                None => source.0.children(key),
            }
        };
        let expanded = match filter.as_ref() {
            Some(filter) => filter.layer.clone(),
            None => self.0.expanded.get()?,
        };
        drop(filter);
        let mut rows: Vec<FlatRow> = Vec::new();
        // Depth-first without recursion, so a deep hierarchy cannot overflow the
        // stack: each entry is a node still to emit with its depth and parent row.
        let mut pending: Vec<(String, usize, Option<usize>)> = source
            .0
            .roots()
            .into_iter()
            .filter(|key| shown(key))
            .rev()
            .map(|key| (key, 0, None))
            .collect();
        let mut index = BTreeMap::new();
        while let Some((key, depth, parent)) = pending.pop() {
            // A key met twice would make two rows share an identity; the first
            // wins and the repeat is dropped rather than corrupting the list.
            if index.contains_key(&key) {
                continue;
            }
            let expandable = if visible.is_some() {
                !children(&key).is_empty()
            } else {
                source.0.can_expand(&key)
            };
            let open = expandable && expanded.contains(&key);
            let row = rows.len();
            if open {
                pending.extend(
                    children(&key)
                        .into_iter()
                        .rev()
                        .map(|child| (child, depth + 1, Some(row))),
                );
            }
            index.insert(key.clone(), row);
            rows.push(FlatRow {
                key,
                depth,
                expandable,
                expanded: open,
                parent,
            });
        }
        // An active item that a collapse hid moves to the collapsed node. The
        // previous flattening's parent links name the ancestors the item was
        // shown under; the nearest one still visible decides. Collapsed by the
        // expansion set, it hid the item. Still expanded, or no longer able to
        // expand, the item left it: it was moved or removed, which only the
        // application knows, and it stays active, like the never pruned
        // selection.
        if collapse
            && let Some(active) = self.0.selection.active().get()?
            && !index.contains_key(&active)
        {
            let previous = self.flat();
            let mut at = previous
                .index
                .get(&active)
                .and_then(|row| previous.rows.get(*row))
                .and_then(|row| row.parent);
            let visible = std::iter::from_fn(|| {
                let row = previous.rows.get(at?)?;
                at = row.parent;
                Some(row.key.clone())
            })
            .find(|key| index.contains_key(key));
            if let Some(key) = visible
                && !expanded.contains(&key)
            {
                self.0.selection.set_active(Some(key))?;
            }
        }
        let flat = Rc::new(Flat {
            rows,
            index,
            source,
            state: Rc::downgrade(&self.0),
        });
        self.0.items.set(ListSource::from_shared(flat.clone()))?;
        *self.0.flat.borrow_mut() = flat;
        Ok(())
    }
}

/// One tree row: a depth indent, an expander slot and the node's content.
///
/// The expander is drawn and hit-tested by the row rather than being a button, so
/// a tree stays one Tab stop however many rows it has. A press outside the
/// expander is left unhandled, which lets the list select the row.
/// The expander slot uses SurfaceHover while hovered.
struct TreeRow {
    content: Option<Element>,
    key: String,
    depth: usize,
    expandable: bool,
    expanded: bool,
    state: Weak<StateInner>,
    slot: Cell<Option<Rect>>,
    hovered_expander: bool,
}
impl TreeRow {
    fn indent(&self, cx: &MeasureContext<'_>) -> (f32, f32) {
        let step = cx.metric(MetricRole::IconSize);
        (self.depth as f32 * step, step)
    }
}
impl Component for TreeRow {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let layout = self.depth != next.depth;
        let paint = self.expandable != next.expandable || self.expanded != next.expanded;
        self.depth = next.depth;
        self.expandable = next.expandable;
        self.expanded = next.expanded;
        self.state = next.state;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.content.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.len() != 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        let (indent, step) = self.indent(cx);
        let lead = indent + step;
        let content = cx.measure(
            0,
            Constraints::new(
                available.width().map(|width| (width - lead).max(0.0)),
                available.height(),
            )?,
        )?;
        Ok(Size::new(
            lead + content.width(),
            content.height().max(step),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() != 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        let (indent, step) = self.indent(cx.measurement());
        let lead = (indent + step).min(size.width());
        let content = cx.measure(
            0,
            Constraints::new(Some(size.width() - lead), Some(size.height()))?,
        )?;
        let top = ((size.height() - step) / 2.0).max(0.0);
        self.slot.set(
            Rect::new(
                indent.min(size.width()),
                top,
                step.min(size.width() - indent.min(size.width())),
                step.min(size.height()),
            )
            .ok(),
        );
        let content_height = content.height().min(size.height());
        cx.place(
            0,
            Rect::new(
                lead,
                (size.height() - content_height) / 2.0,
                size.width() - lead,
                content_height,
            )?,
        )
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if self.expandable
            && let Some(slot) = self.slot.get()
        {
            if self.hovered_expander {
                painter.fill_shape(
                    slot,
                    crate::choice_controls::rounded(painter.metric(MetricRole::CornerRadius))?,
                    painter.color(ColorRole::SurfaceHover),
                )?;
            }
            painter.icon(
                if self.expanded {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                slot,
                painter.color(ColorRole::Muted),
            )?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if matches!(event.kind(), PointerKind::Move | PointerKind::Leave) {
            let hovered = self.expandable
                && event.kind() == PointerKind::Move
                && !event.is_outside()
                && cx.pointer_inside()
                && cx
                    .to_local(event.position())
                    .is_some_and(|point| self.slot.get().is_some_and(|slot| slot.contains(point)));
            if self.hovered_expander != hovered {
                self.hovered_expander = hovered;
                cx.invalidate(Invalidation::Paint);
            }
        }
        if self.expandable
            && event.kind() == PointerKind::Press(PointerButton::Primary)
            && let Some(point) = cx.to_local(event.position())
            && self.slot.get().is_some_and(|slot| slot.contains(point))
            && let Some(state) = self.state.upgrade()
        {
            // A collapse can move the active node to this one: the user's choice.
            let controller = TreeController(state);
            let selection = controller.selection();
            selection.with_origin(crate::ChangeOrigin::User, || controller.toggle(&self.key))?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hovered_expander {
            self.hovered_expander = false;
            cx.invalidate(Invalidation::Paint);
        }
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

/// An expandable hierarchy: one Tab stop, keyboard navigation and selection over
/// every visible node, virtualized through [`VirtualList`].
///
/// The tree adds no virtualization of its own. A [`TreeController`] flattens the
/// expanded nodes into list rows, so mounting, scrolling, pinning and reveal are
/// the list's, and navigation and selection are the shared [`ListSelection`]
/// contract: Up/Down, Home/End and Page keys move over visible nodes, mounted or
/// not. The tree gives the unmodified cross arrows their outline meaning: Right
/// expands the active node, or moves to its first child when it is already
/// expanded; Left collapses it, or moves to its parent. Modified cross arrows,
/// other than reordering with Alt, are left to command bindings. Each row is indented by one IconSize per
/// level and vertically centers its content and expander. The expander is drawn
/// by the row, not a button, so rows add no Tab
/// stops; pressing it toggles the node, and pressing the rest of the row selects
/// it. Enter runs [`Tree::on_activate`]. The default expander uses
/// [`Icon::ChevronRight`] and [`Icon::ChevronDown`]; override those roles through
/// [`Runtime::icons`](crate::Runtime::icons) to change their glyphs throughout the UI.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Label, ListSelection, Runtime, SelectionMode, Tree,
///     TreeItems, TreeSource, TreeController, Viewport};
/// struct Files;
/// impl TreeItems for Files {
///     fn roots(&self) -> Vec<String> {
///         vec!["src".into(), "README.md".into()]
///     }
///     fn children(&self, key: &str) -> Vec<String> {
///         if key == "src" { vec!["src/lib.rs".into()] } else { Vec::new() }
///     }
///     fn build(&self, key: &str) -> Option<Element> {
///         Some(Element::new(key, Label::new(key.rsplit('/').next()?)))
///     }
/// }
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
/// let selection = ListSelection::new(&ui, SelectionMode::Single);
/// let state = TreeController::new(&ui, selection, TreeSource::new(Files))?;
/// let tree = ui.mount_tree(Element::new("files", Tree::new(&state, 24.0)?))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// assert_eq!(ui.children(&tree)?.len(), 2, "only the roots are visible");
/// state.set_expanded("src", true)?;
/// ui.flush(std::time::Duration::ZERO)?;
/// assert_eq!(ui.children(&tree)?.len(), 3);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Describing a mounted tree again keeps everything the first description
/// gave it: the controller, row height and options, and also the
/// [`on_activate`](Self::on_activate) and [`reorderable`](Self::reorderable)
/// callbacks, the [`reorder_permission`](Self::reorder_permission) binding and
/// the [`drag_operations`](Self::drag_operations). A later description's values
/// are ignored; the tree's data, expansion and selection belong to the
/// controller. Change them through it, make a callback read application state
/// that changes, or give the tree a new key to replace it.
///
/// Like a [`VirtualList`](crate::VirtualList), the tree mounts the rows inside
/// the height it is allocated and needs a bounded one; see [the list's
/// bounded extent](crate::VirtualList#a-bounded-extent). Expanding a node asks
/// [`TreeItems::children`] for all its children and indexes them, so a node
/// with very many children costs in proportion to them although only a
/// screenful mounts, and a replaced source flattens its expanded nodes again.
#[must_use]
pub struct Tree {
    list: VirtualList,
    state: TreeController,
    /// One level of indent from the last arrangement, shared with the
    /// placement that turns a pointer into a drop position.
    step: Rc<Cell<f32>>,
}
description_debug!(Tree);
impl Tree {
    /// Creates a tree over a controller with a fixed logical row height.
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for a row height that is not finite,
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) for a
    /// negative one and [`GeometryError::Empty`](crate::GeometryError::Empty)
    /// for zero.
    pub fn new(controller: &TreeController, row_height: f32) -> Result<Self, crate::GeometryError> {
        Ok(Self {
            list: VirtualList::new(controller.items(), row_height)?
                .selection(controller.selection())
                .controller(controller.0.reveal.clone()),
            state: controller.clone(),
            step: Rc::new(Cell::new(0.0)),
        })
    }
    /// Makes nodes draggable and the tree a drop target for
    /// [`ListDragItems`](crate::ListDragItems), which is how an outline is
    /// rearranged. A tree is not reorderable unless this is called.
    ///
    /// Dragging is the list's: a primary press on a row that its content did
    /// not handle arms a drag of the node's key, or of every selected key in
    /// display order when the node is part of a multiple selection; see
    /// [`VirtualList::reorderable`]. The keys may include a node together with
    /// its descendants, and the application decides what moving both means,
    /// typically moving the topmost.
    ///
    /// Over a row, the upper half drops before the node and the lower half
    /// after it, as siblings under its parent; the insertion line starts at the
    /// indent of the level the nodes would join. The lower half of an expanded
    /// node's row drops before its first child. Where the row is the last one
    /// shown at one or more levels, the pointer's horizontal position chooses
    /// the level, so a node can be put after its parent. A row whose node
    /// [`TreeItems::can_drop_into`] divides into quarters: the outer ones drop
    /// before and after, and the middle half drops into the node as its last
    /// child, outlining the row. A position inside one of the dragged nodes'
    /// own subtrees is forbidden and shows the no-drop cursor.
    ///
    /// `on_drop` receives a [`ListDrop`] whose [`ListDropPosition`] names the
    /// application's keys: `Before` and `After` a sibling, `Into` a node, or
    /// `End` of an empty tree. It performs the move as one edit in the
    /// application's model and publishes the hierarchy with
    /// [`TreeController::set_source`], expanding the receiving node with
    /// [`TreeController::set_expanded`] if it should show the result, and returns
    /// whether it did. Keys, and with them expansion and selection, survive
    /// the move.
    ///
    /// The keyboard equivalent is Alt with an arrow on the active node, through
    /// the same callback: Up and Down move it before its previous or after its
    /// next sibling, Left after its parent, and Right into its previous sibling
    /// when that sibling [can take it](TreeItems::can_drop_into).
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::*;
    /// use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};
    /// // Children per parent; "" holds the top level.
    /// type Model = Rc<RefCell<BTreeMap<String, Vec<String>>>>;
    /// #[derive(Clone)]
    /// struct Outline(BTreeMap<String, Vec<String>>);
    /// impl TreeItems for Outline {
    ///     fn roots(&self) -> Vec<String> {
    ///         self.children("")
    ///     }
    ///     fn children(&self, key: &str) -> Vec<String> {
    ///         self.0.get(key).cloned().unwrap_or_default()
    ///     }
    ///     fn can_drop_into(&self, key: &str) -> bool {
    ///         key.ends_with('/')
    ///     }
    ///     fn build(&self, key: &str) -> Option<Element> {
    ///         Some(Element::new(key, Label::new(key)))
    ///     }
    /// }
    /// let model: Model = Rc::new(RefCell::new(BTreeMap::from([
    ///     ("".to_owned(), vec!["docs/".to_owned(), "notes.txt".to_owned()]),
    ///     ("docs/".to_owned(), Vec::new()),
    /// ])));
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let selection = ListSelection::new(&ui, SelectionMode::Single);
    /// let state = TreeController::new(&ui, selection, TreeSource::new(Outline(model.borrow().clone())))?;
    /// let (edited, published) = (model.clone(), state.clone());
    /// let tree = Tree::new(&state, 24.0)?.reorderable(move |_, drop| {
    ///     let mut model = edited.borrow_mut();
    ///     let ListDropPosition::Into(parent) = drop.position() else {
    ///         return Ok(false);
    ///     };
    ///     for siblings in model.values_mut() {
    ///         siblings.retain(|key| !drop.items().contains(key));
    ///     }
    ///     model.entry(parent.clone()).or_default().extend(drop.items().iter().cloned());
    ///     published.set_source(TreeSource::new(Outline(model.clone())))?;
    ///     published.set_expanded(parent, true)?;
    ///     Ok(true)
    /// });
    /// ui.mount_tree(Element::new("files", tree))?;
    /// ui.flush(Duration::ZERO)?;
    /// // Drag "notes.txt" onto the middle of the "docs/" row.
    /// let at = |y: f32, kind| PointerEvent::new(Point::new(60.0, y).unwrap(), kind);
    /// ui.pointer_input(at(36.0, PointerKind::Press(PointerButton::Primary)))?;
    /// ui.pointer_input(at(30.0, PointerKind::Move))?;
    /// ui.pointer_input(at(12.0, PointerKind::Move))?;
    /// ui.flush(ui.now())?;
    /// ui.pointer_input(at(12.0, PointerKind::Release(PointerButton::Primary)))?;
    /// ui.flush(ui.now())?;
    /// assert_eq!(model.borrow()["docs/"], ["notes.txt"]);
    /// assert_eq!(model.borrow()[""], ["docs/"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn reorderable(
        mut self,
        on_drop: impl FnMut(&mut EventContext, &ListDrop<'_>) -> Result<bool, RuntimeError> + 'static,
    ) -> Self {
        let (state, step) = (self.state.clone(), self.step.clone());
        self.list = self
            .list
            .reorderable(on_drop)
            .placement(move |keys, at| place(&state, step.get(), keys, at));
        self
    }
    /// Restricts pointer and keyboard reordering through the retained list.
    /// Default allowed; [`Self::reorderable`] is still required. Repeated
    /// restrictions combine through [`Permission::and`]. Revocation cancels an
    /// active drag or armed press and removes its drop indication, without
    /// changing selection or expansion. The same restriction applies to accepting
    /// drops from another list or tree. Denied Alt-arrow reordering falls through
    /// to ordinary tree navigation. Bound changes are observed on update.
    /// Compatible redescription retains this descriptor and its bindings; change
    /// its bound signal or use a new tree key to replace the restriction.
    pub fn reorder_permission(mut self, permission: impl Into<Permission>) -> Self {
        self.list = self.list.reorder_permission(permission);
        self
    }
    /// The operations a drag started by this tree permits; a move by default.
    /// Has no effect without [`Self::reorderable`].
    pub fn drag_operations(mut self, operations: DropOperations) -> Self {
        self.list = self.list.drag_operations(operations);
        self
    }
    /// Reports every change of the tree's [`ListSelection`] after it took
    /// effect, as [`VirtualList::on_selection_change`] does for a list: a click,
    /// a key or a collapse that moves the active node to its parent is
    /// [`ChangeOrigin::User`](crate::ChangeOrigin::User), and a write through
    /// the selection or the [`TreeController`] is
    /// [`ChangeOrigin::Application`](crate::ChangeOrigin::Application). The
    /// callback runs on the tree, and like the other callbacks it belongs to
    /// the first description of a mounted tree.
    pub fn on_selection_change(
        mut self,
        on_change: impl FnMut(
            &mut EventContext,
            &crate::ListSelectionChange,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.list = self.list.on_selection_change(on_change);
        self
    }
    /// Runs when Enter is pressed on the active node; receives its key.
    pub fn on_activate(
        mut self,
        activate: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.list = self.list.on_activate(activate);
        self
    }
    // Left and Right on the active node, as `true` when the key had an effect.
    fn cross(&self, right: bool) -> Result<bool, RuntimeError> {
        let flat = self.state.flat();
        let Some(index) = self
            .state
            .0
            .selection
            .active()
            .get()?
            .and_then(|key| flat.index_of(&key))
        else {
            return Ok(false);
        };
        let row = &flat.rows[index];
        let target = match (right, row.expandable, row.expanded) {
            (true, true, false) => return self.state.set_expanded(&row.key, true),
            (false, true, true) => return self.state.set_expanded(&row.key, false),
            // An expanded node's first child is the next row.
            (true, true, true) => Some(index + 1).filter(|next| *next < flat.rows.len()),
            (false, ..) => row.parent,
            (true, false, _) => None,
        };
        let Some(target) = target else {
            return Ok(false);
        };
        self.state.0.selection.apply(
            &*flat,
            target,
            Gesture::Move {
                extend: false,
                keep: false,
            },
        )?;
        Ok(true)
    }
    // Alt with an arrow: the keyboard equivalent of dragging the active node
    // one place in the outline. `false` when there is nowhere to go.
    fn move_by_key(&mut self, cx: &mut EventContext, code: KeyCode) -> Result<bool, RuntimeError> {
        let flat = self.state.flat();
        let Some(index) = self.list.active_index() else {
            return Ok(false);
        };
        let Some(row) = flat.rows.get(index) else {
            return Ok(false);
        };
        let parent_key = row.parent.map(|parent| flat.rows[parent].key.clone());
        // The shown siblings, as a pointer drag sees them: every child of an
        // expanded node unfiltered, only matches and their ancestors while a
        // filter applies, so a move never lands beside a hidden node.
        let siblings: Vec<String> = flat
            .rows
            .iter()
            .filter(|sibling| sibling.parent == row.parent)
            .map(|sibling| sibling.key.clone())
            .collect();
        let Some(at) = siblings.iter().position(|key| *key == row.key) else {
            return Ok(false);
        };
        let previous = at.checked_sub(1).map(|previous| siblings[previous].clone());
        // Each position with the row whose subtree would receive the nodes.
        let (position, parent) = match code {
            KeyCode::ArrowUp => match previous {
                Some(key) => (ListDropPosition::Before(key), row.parent),
                None => return Ok(false),
            },
            KeyCode::ArrowDown => match siblings.get(at + 1) {
                Some(key) => (ListDropPosition::After(key.clone()), row.parent),
                None => return Ok(false),
            },
            KeyCode::ArrowLeft => match (row.parent, parent_key) {
                (Some(parent), Some(key)) => {
                    (ListDropPosition::After(key), flat.rows[parent].parent)
                }
                _ => return Ok(false),
            },
            KeyCode::ArrowRight => match previous {
                Some(key) if flat.source.0.can_drop_into(&key) => {
                    let parent = flat.index_of(&key);
                    (ListDropPosition::Into(key), parent)
                }
                _ => return Ok(false),
            },
            _ => return Ok(false),
        };
        let keys = self.list.dragged_keys(index);
        if encloses(&flat, parent, &keys) {
            return Ok(false);
        }
        self.list.reorder_keys(cx, &keys, position)
    }
}

// Whether the row `parent` or one of its ancestors is among `keys`: nodes
// dropped there would become descendants of themselves.
fn encloses(flat: &Flat, mut parent: Option<usize>, keys: &[String]) -> bool {
    while let Some(index) = parent {
        let Some(row) = flat.rows.get(index) else {
            return false;
        };
        if keys.contains(&row.key) {
            return true;
        }
        parent = row.parent;
    }
    false
}

// Where a drop over the tree goes: see `Tree::reorderable`. `None` inside a
// dragged node's own subtree, and where the row under the pointer is no longer
// in the hierarchy: a publication may land between the list's layout and a
// drop, and no other node is substituted for the one the user saw.
fn place(
    state: &TreeController,
    step: f32,
    keys: &[String],
    at: &ListDropPoint,
) -> Option<ListDropPlacement> {
    let flat = state.flat();
    let (Some(shown), Some(key)) = (at.index(), at.key()) else {
        return Some(ListDropPlacement::new(
            ListDropPosition::End,
            DropMark::Line {
                boundary: 0,
                indent: 0.0,
            },
        ));
    };
    // The row's place in the current flattening decides the position; the
    // mark is drawn where the list shows the row.
    let index = flat.index_of(key)?;
    let row = &flat.rows[index];
    let into = flat.source.0.can_drop_into(&row.key);
    let (before, after) = if into { (0.25, 0.75) } else { (0.5, 0.5) };
    let indent = |depth: usize| depth as f32 * step;
    let (placement, parent) = if at.fraction() < before {
        (
            ListDropPlacement::new(
                ListDropPosition::Before(row.key.clone()),
                DropMark::Line {
                    boundary: shown,
                    indent: indent(row.depth),
                },
            ),
            row.parent,
        )
    } else if at.fraction() < after {
        (
            ListDropPlacement::new(
                ListDropPosition::Into(row.key.clone()),
                DropMark::Into(shown),
            ),
            Some(index),
        )
    } else if let Some(child) = flat
        .rows
        .get(index + 1)
        .filter(|next| next.parent == Some(index))
    {
        // Below an expanded node its first child follows, so the boundary is
        // the start of its children rather than the end of its subtree.
        (
            ListDropPlacement::new(
                ListDropPosition::Before(child.key.clone()),
                DropMark::Line {
                    boundary: shown + 1,
                    indent: indent(child.depth),
                },
            ),
            Some(index),
        )
    } else {
        // The last row shown at every level between its own and the next
        // row's: the pointer's horizontal position picks the level.
        let lowest = flat.rows.get(index + 1).map_or(0, |next| next.depth);
        let level = if step > 0.0 {
            // Truncation is the intended floor of a nonnegative quotient.
            ((at.across().max(0.0) / step) as usize).clamp(lowest, row.depth)
        } else {
            row.depth
        };
        let mut anchor = index;
        while flat.rows[anchor].depth > level {
            match flat.rows[anchor].parent {
                Some(parent) => anchor = parent,
                None => break,
            }
        }
        let anchor = &flat.rows[anchor];
        (
            ListDropPlacement::new(
                ListDropPosition::After(anchor.key.clone()),
                DropMark::Line {
                    boundary: shown + 1,
                    indent: indent(anchor.depth),
                },
            ),
            anchor.parent,
        )
    };
    (!encloses(&flat, parent, keys)).then_some(placement)
}
// The tree is its list with two keys reinterpreted, so it forwards every
// `Component` method the list overrides. `forwarding::every_list_override_is_forwarded`
// fails when the list gains an override this block does not pass on.
impl Component for Tree {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.list.update(cx)
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.list.deliver_notifications(cx)
    }
    fn can_reveal(&self) -> bool {
        self.list.can_reveal()
    }
    fn content_rows(&self) -> Option<Rc<dyn crate::ContentRows>> {
        self.list.content_rows()
    }
    fn try_content_rows(&self) -> Result<Option<Rc<dyn crate::ContentRows>>, RuntimeError> {
        self.list.try_content_rows()
    }
    fn focusable(&self) -> bool {
        self.list.focusable()
    }
    fn descendant_tab_stops(&self) -> bool {
        self.list.descendant_tab_stops()
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.list.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.step.set(cx.measurement().metric(MetricRole::IconSize));
        self.list.arrange(cx, size)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.list.pointer_input(cx, event)
    }
    fn text_input(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        self.list.text_input(cx, text)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if self.list.is_reorderable()
            && crate::activation::alt_only(keyboard.chord_modifiers(event))
            && matches!(
                control,
                KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
            )
            && matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
        {
            self.move_by_key(cx, control)?;
            return Ok(InputOutcome::Handled);
        }
        let cross = match control {
            KeyCode::ArrowRight => Some(true),
            KeyCode::ArrowLeft => Some(false),
            _ => None,
        };
        if let Some(right) = cross
            && matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
        {
            // Modified cross arrows are left to command bindings.
            if keyboard.chord_modifiers(event) != crate::Modifiers::default() {
                return Ok(InputOutcome::Ignored);
            }
            // Moving or collapsing over the active node is the user's choice.
            let selection = self.state.0.selection.clone();
            let crossed = selection.with_origin(crate::ChangeOrigin::User, || self.cross(right))?;
            return Ok(if crossed {
                InputOutcome::Handled
            } else {
                InputOutcome::Ignored
            });
        }
        self.list.keyboard_event(cx, event, keyboard)
    }
    fn reveal(&mut self, cx: &mut EventContext, rect: Rect) -> Result<bool, RuntimeError> {
        self.list.reveal(cx, rect)
    }
    fn hit_test_foreground(&self, point: Point) -> bool {
        self.list.hit_test_foreground(point)
    }
    fn input_reset(&mut self) {
        self.list.input_reset();
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.list.hover_changed(cx, hovered);
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &crate::DragEvent<'_>,
    ) -> Result<crate::DropResponse, RuntimeError> {
        self.list.drag_event(cx, event)
    }
    fn drag_finished(
        &mut self,
        cx: &mut EventContext,
        outcome: crate::DragOutcome,
    ) -> Result<(), RuntimeError> {
        self.list.drag_finished(cx, outcome)
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.list.timer_elapsed(cx)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.list.paint(painter)
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.list.paint_foreground(painter)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = self.list.inspect();
        let flat = self.state.flat();
        let active = self
            .state
            .0
            .selection
            .active()
            .get()
            .ok()
            .flatten()
            .and_then(|key| flat.index_of(&key))
            .map(|index| &flat.rows[index]);
        if let Some(semantics) = details.semantics.take() {
            // The expanded state and the expand or collapse action describe the
            // active node, which is the one the cross arrows act on.
            let semantics =
                match active {
                    Some(row) if row.expandable => semantics
                        .expanded(row.expanded)
                        .enabled_actions([if row.expanded {
                            SemanticAction::Collapse
                        } else {
                            SemanticAction::Expand
                        }]),
                    _ => semantics,
                };
            // A reorderable tree's `Move` comes from its list.
            details.semantics = Some(semantics.role(SemanticRole::Tree));
        }
        details
    }
}

#[cfg(test)]
mod forwarding {
    /// The names of the methods in the first `impl Component for <name>` block.
    fn overrides(source: &str, name: &str) -> Vec<String> {
        let header = format!("impl Component for {name} {{");
        let start = source
            .find(&header)
            .unwrap_or_else(|| panic!("no `{header}` block"));
        // Methods of the block are indented once; its closing brace is not.
        let body = &source[start + header.len()..];
        let end = body.find("\n}").expect("the block closes");
        body[..end]
            .lines()
            .filter_map(|line| line.strip_prefix("    fn "))
            .filter_map(|rest| rest.split(['(', '<']).next())
            .map(str::to_owned)
            .collect()
    }

    // `Tree` embeds a `VirtualList` and forwards by hand. A method the list
    // overrides and the tree does not forward silently loses that behavior,
    // which is how drag and drop first went missing here.
    #[test]
    fn every_list_override_is_forwarded() {
        let list = overrides(include_str!("virtual_list.rs"), "VirtualList");
        assert!(
            list.iter().any(|name| name == "drag_event"),
            "the list's overrides were not found: {list:?}"
        );
        let tree = include_str!("tree.rs");
        let header = "impl Component for Tree {";
        let block = &tree[tree.find(header).expect("the tree's block")..];
        let block = &block[..block.find("\n}").expect("the block closes")];
        let missing: Vec<_> = list
            .iter()
            .filter(|name| !block.contains(&format!("self.list.{name}(")))
            .collect();
        assert!(
            missing.is_empty(),
            "`Tree` does not forward these `VirtualList` overrides: {missing:?}"
        );
    }
}
