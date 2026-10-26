//! Filter fields for virtualized collections, driven by their sources' text.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use crate::{
    ArrangeContext, ComponentDetails, Constraints, Element, EventContext, InspectValue,
    Invalidation, LayoutError, ListItems, ListSelection, ListSource, MeasureContext, MetricRole,
    Permission, ReadSignal, Rect, SearchField, SearchKey, SemanticRole, Semantics, Signal, Size,
    SizeLimits, TableController, TextSource, TreeController, UpdateContext, WidgetId,
    runtime::{Component, RuntimeError},
};

/// Decides whether a row's text matches a filter query.
///
/// The default, [`Self::substring`], keeps rows whose text contains the query,
/// ignoring case, the same rule content search uses. [`Self::fuzzy`] keeps
/// rows whose text contains the query's characters in order, as the command
/// palette does. [`Self::new`] takes any rule. A query that is empty or only
/// whitespace never reaches the matcher: it shows every row. A filter trims
/// any other query of leading and trailing whitespace before the matcher sees
/// it, while [`Self::matches`] takes the query exactly as given. To know which
/// rows a list filter shows, read [`ListFilter::items`] instead of repeating
/// the match; code that does repeat it passes the trimmed query.
///
/// A matcher runs once per row on every query, on the UI thread, so it must be
/// deterministic, free of side effects and I/O, and cheap; its cost multiplied
/// by the rows filtered is the cost of a query.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::FilterMatcher;
/// assert!(FilterMatcher::substring().matches("ver", "Reverb"));
/// assert!(!FilterMatcher::substring().matches("rvb", "Reverb"));
/// assert!(FilterMatcher::fuzzy().matches("rvb", "Reverb"));
/// let prefix = FilterMatcher::new(|query, text| text.starts_with(query));
/// assert!(prefix.matches("Rev", "Reverb") && !prefix.matches("verb", "Reverb"));
/// ```
#[derive(Clone)]
#[must_use]
pub struct FilterMatcher(Rc<MatchFn>);
/// A matcher's rule: whether the text (second) matches the query (first).
type MatchFn = dyn Fn(&str, &str) -> bool;
impl FilterMatcher {
    /// Uses `matches(query, text)` as the rule.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    pub fn new(matches: impl Fn(&str, &str) -> bool + 'static) -> Self {
        Self(Rc::new(matches))
    }
    /// Keeps text containing the query, compared after Unicode lowercase
    /// mapping. The default. Every call on a thread returns the same rule, so
    /// describing a filter again with it does not reapply the query.
    pub fn substring() -> Self {
        thread_local! {
            static SUBSTRING: FilterMatcher = FilterMatcher::new(|query, text| {
                text.to_lowercase().contains(&query.to_lowercase())
            });
        }
        SUBSTRING.with(Clone::clone)
    }
    /// Keeps text containing the query's characters in order, as
    /// [`PickMatcher::fuzzy_score`](crate::PickMatcher::fuzzy_score) matches.
    /// Every call on a thread returns the same rule.
    pub fn fuzzy() -> Self {
        thread_local! {
            static FUZZY: FilterMatcher = FilterMatcher::new(|query, text| {
                crate::PickMatcher::fuzzy_score(query, text).is_some()
            });
        }
        FUZZY.with(Clone::clone)
    }
    /// Whether two matchers are the same rule instance, the test for
    /// reapplying a filter when it is described again.
    fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    /// Whether `text` matches `query` under this rule. The query is used as
    /// given; a filter passes it trimmed (see the [type](Self)).
    #[must_use]
    pub fn matches(&self, query: &str, text: &str) -> bool {
        (self.0)(query, text)
    }
}
impl Default for FilterMatcher {
    fn default() -> Self {
        Self::substring()
    }
}
impl std::fmt::Debug for FilterMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterMatcher").finish_non_exhaustive()
    }
}

/// How many rows a filter shows of how many it examined.
///
/// For a list or a table, `total` is the source's row count and `shown` the
/// rows that match. For a tree, `total` counts the nodes the source already
/// knows (see [`TreeItems::known_children`](crate::TreeItems::known_children))
/// and `shown` the nodes that match, without the ancestors shown around them.
/// Without a query both are the full count after filtering has been evaluated.
/// A tree reports zero until its first filter, avoiding an eager hierarchy walk.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FilterCounts {
    /// Rows that match.
    pub shown: usize,
    /// Rows examined.
    pub total: usize,
}

/// What applying a filter changed, returned by every filter's `filter` method.
///
/// Besides the counts it names where keyboard navigation should go when the
/// active item disappeared: the nearest row still shown, by the order before
/// the filter changed. Applying a filter never moves the active item itself;
/// [`CollectionFilter`] moves it while the collection has focus, and without
/// focus only as its [`HiddenActive`] policy says.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct FilterOutcome {
    /// The rows shown and examined.
    pub counts: FilterCounts,
    /// The nearest row still shown when the active item was shown before and
    /// is hidden now: the first remaining row after it, else the last one
    /// before it. `None` when the active item is still shown, was not shown,
    /// or no row remains.
    pub nearest_active: Option<String>,
}

/// What a [`CollectionFilter`] does with an active item that a filter change
/// leaves hidden while focus is outside the collection, chosen with
/// [`CollectionFilter::hidden_active`].
///
/// With focus in the collection the active item always moves to the nearest
/// row still shown, whatever the policy; see the "Selection and focus"
/// paragraph of [`CollectionFilter`]. The policy acts once per delivered
/// filter change (an applied query, a table's or tree's source replacement,
/// or both), after the change, on the item the selection holds as active
/// then. It moves only the active item: in
/// [`SelectionMode::Single`](crate::SelectionMode::Single) the selection
/// follows it, and in [`SelectionMode::Multiple`](crate::SelectionMode::Multiple)
/// hidden selected keys stay selected, since the selection is never pruned.
/// Its writes go through [`ListSelection::set_active`] and are reported as
/// application writes.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HiddenActive {
    /// It stays active and hidden, and is shown again when the filter
    /// clears. The default, and the behavior before this option.
    #[default]
    Keep,
    /// It moves to the nearest row still shown, as it does while the
    /// collection has focus: the transition [`FilterOutcome::nearest_active`]
    /// describes. Only a change that hid the current active item, with a row
    /// remaining, moves it. An item that was already hidden, one that no row
    /// remains for, and one the application made active after the change
    /// hid another stay where they are. A table or tree reports only its
    /// latest source replacement, so when two replacements arrive before one
    /// delivery and the first hid the item, the second finds it already
    /// hidden and it stays.
    Nearest,
    /// No item stays active: an active item that is hidden after the change
    /// is cleared, whether this change hid it or an earlier one did, and
    /// whether or not a row remains. When a delivered filter change is
    /// evaluated with focus outside the collection, the active item is
    /// therefore shown, or there is none, afterwards. That says nothing about
    /// later application writes, nor about a change evaluated while the
    /// collection has focus, which keeps a hidden active item when no row
    /// remains.
    Clear,
}
impl HiddenActive {
    /// The name inspection reports.
    fn name(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Nearest => "nearest",
            Self::Clear => "clear",
        }
    }
}

/// The active key a change hid and the nearest row still shown, if any:
/// `Some` when `key` was a row of `before` and is not one of `after`.
pub(crate) fn hid(
    before: &dyn ListItems,
    after: &dyn ListItems,
    key: Option<String>,
) -> Option<(String, Option<String>)> {
    let key = key?;
    if before.index_of(&key).is_none() || after.index_of(&key).is_some() {
        return None;
    }
    let nearest = nearest(before, after, &key);
    Some((key, nearest))
}

/// Rows of a source restricted to the positions a filter kept, in order.
struct FilteredRows {
    inner: ListSource,
    shown: Vec<usize>,
}
impl ListItems for FilteredRows {
    fn len(&self) -> usize {
        self.shown.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.inner.key(*self.shown.get(index)?)
    }
    fn build(&self, index: usize) -> Option<Element> {
        self.inner.build(*self.shown.get(index)?)
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        let inner = self.inner.index_of(key)?;
        self.shown.binary_search(&inner).ok()
    }
    fn text(&self, index: usize) -> Option<String> {
        self.inner.text(*self.shown.get(index)?)
    }
}

/// The trimmed query, or `None` when it shows every row.
pub(crate) fn effective(query: &str) -> Option<&str> {
    let query = query.trim();
    (!query.is_empty()).then_some(query)
}

/// Restricts `source` to the rows whose text matches `query`. Rows offering no
/// text never match a query.
pub(crate) fn restrict(
    source: &ListSource,
    query: Option<&str>,
    matcher: &FilterMatcher,
) -> (ListSource, FilterCounts) {
    let total = source.len();
    let Some(query) = query else {
        return (
            source.clone(),
            FilterCounts {
                shown: total,
                total,
            },
        );
    };
    let shown: Vec<usize> = (0..total)
        .filter(|&index| {
            source
                .text(index)
                .is_some_and(|text| matcher.matches(query, &text))
        })
        .collect();
    let counts = FilterCounts {
        shown: shown.len(),
        total,
    };
    let rows = FilteredRows {
        inner: source.clone(),
        shown,
    };
    (ListSource::new(rows), counts)
}

/// The nearest row of `after` to `key`'s place in `before`, when `key` was a
/// row of `before` and is not one of `after`.
///
/// Linear in the rows: membership in `after` is collected once, so a source
/// whose `index_of` is a linear search is not searched again per row.
pub(crate) fn nearest(before: &dyn ListItems, after: &dyn ListItems, key: &str) -> Option<String> {
    if after.is_empty() || after.index_of(key).is_some() {
        return None;
    }
    let at = before.index_of(key)?;
    let kept: std::collections::HashSet<String> = (0..after.len())
        .filter_map(|index| after.key(index))
        .collect();
    let remaining = |index: usize| before.key(index).filter(|k| kept.contains(k));
    (at + 1..before.len())
        .find_map(remaining)
        .or_else(|| (0..at).rev().find_map(remaining))
}

/// A filtered view of a [`VirtualList`](crate::VirtualList)'s rows.
///
/// Bind the list to [`Self::items`] instead of the source. A filter keeps the
/// rows whose [`ListItems::text`] matches the query and hides the others; it
/// never rebuilds the rows it keeps, which keep their keys, components,
/// selection and scroll anchoring, and rows it hid come back with the same
/// keys. A row whose source offers no text never matches a query, so filter
/// only sources that implement `text`.
///
/// [`Self::filter`] applies a query, usually from a [`CollectionFilter`],
/// which calls it as its field changes and again when the source binding
/// changes. Applying writes signals, so it runs in event or application code,
/// never in a component's update. Each query reads the text of every row once;
/// a large source may answer `text` from a precomputed index.
///
/// The selection is never pruned: a selected row that is filtered out stays
/// selected and hidden, and is drawn again when the filter clears.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{FilterMatcher, ListFilter, ListItems, ListSource, VirtualList};
///
/// struct Fruit;
/// const NAMES: [&str; 4] = ["Apple", "Banana", "Cherry", "Grape"];
/// impl ListItems for Fruit {
///     fn len(&self) -> usize { NAMES.len() }
///     fn key(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_lowercase()) }
///     fn build(&self, index: usize) -> Option<Element> {
///         Some(Element::new(self.key(index)?, Label::new(NAMES[index])))
///     }
///     fn text(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_string()) }
/// }
///
/// let ui = Runtime::new();
/// let source = ui.signal(ListSource::new(Fruit));
/// let filter = ListFilter::new(&ui, source.read_only())?;
/// let _list = VirtualList::new(filter.items(), 20.0)?;
/// let outcome = filter.filter("an", &FilterMatcher::substring())?;
/// assert_eq!((outcome.counts.shown, outcome.counts.total), (1, 4));
/// assert_eq!(filter.items().get()?.key(0).as_deref(), Some("banana"));
/// filter.filter("", &FilterMatcher::substring())?;
/// assert_eq!(filter.items().get()?.len(), 4);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct ListFilter(Rc<ListFilterInner>);
struct ListFilterInner {
    source: ReadSignal<ListSource>,
    items: Signal<ListSource>,
    counts: Signal<FilterCounts>,
    /// The query and matcher last applied, reapplied by [`ListFilter::refresh`].
    applied: RefCell<(String, FilterMatcher)>,
    selection: RefCell<Option<ListSelection>>,
}

/// Shows neither the rows nor the query: formatting must not read a signal
/// inside a runtime phase. Read the rows with [`ListFilter::items`] and their
/// counts with [`ListFilter::counts`].
impl std::fmt::Debug for ListFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListFilter").finish_non_exhaustive()
    }
}
impl ListFilter {
    /// Creates a filter over `source` that shows every row.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner).
    ///
    /// # Errors
    /// The source binding's read failures: `Closed` or `BorrowConflict`.
    pub fn new(
        ui: &impl crate::StateOwner,
        source: impl Into<ReadSignal<ListSource>>,
    ) -> Result<Self, RuntimeError> {
        let source: ReadSignal<ListSource> = source.into();
        let rows = source.get()?;
        let total = rows.len();
        Ok(Self(Rc::new(ListFilterInner {
            items: ui.signal(rows),
            counts: ui.signal(FilterCounts {
                shown: total,
                total,
            }),
            source,
            applied: RefCell::new((String::new(), FilterMatcher::default())),
            selection: RefCell::new(None),
        })))
    }
    /// Names the selection the list uses, so [`FilterOutcome::nearest_active`]
    /// can report where its active item should go. Without it the outcome
    /// names no row.
    pub fn selection(self, selection: ListSelection) -> Self {
        *self.0.selection.borrow_mut() = Some(selection);
        self
    }
    /// The filtered rows, to bind a [`VirtualList`](crate::VirtualList) to.
    #[must_use]
    pub fn items(&self) -> ReadSignal<ListSource> {
        self.0.items.read_only()
    }
    /// The source binding the filter reads.
    #[must_use]
    pub fn source(&self) -> ReadSignal<ListSource> {
        self.0.source.clone()
    }
    /// The counts of the last applied query.
    #[must_use]
    pub fn counts(&self) -> ReadSignal<FilterCounts> {
        self.0.counts.read_only()
    }
    /// Applies `query` with `matcher` to the current source and publishes the
    /// rows that match. An empty or whitespace query shows every row; any other
    /// query is trimmed of leading and trailing whitespace before the matcher
    /// sees it. [`Self::items`] holds the rows shown, so the application reads
    /// them there rather than repeating the match.
    ///
    /// # Errors
    /// The source binding's read failures, and the signal writes' failures:
    /// `WriteDuringUpdate` and the other phase errors, `Closed` or
    /// `BorrowConflict`. A refused write changes nothing.
    pub fn filter(
        &self,
        query: &str,
        matcher: &FilterMatcher,
    ) -> Result<FilterOutcome, RuntimeError> {
        let source = self.0.source.get()?;
        let before = self.0.items.get()?;
        let (rows, counts) = restrict(&source, effective(query), matcher);
        let active = match &*self.0.selection.borrow() {
            Some(selection) => selection.active().get()?,
            None => None,
        };
        let nearest_active = active.and_then(|key| nearest(&*before, &*rows, &key));
        self.0.items.set(rows)?;
        self.0.counts.set(counts)?;
        *self.0.applied.borrow_mut() = (query.to_owned(), matcher.clone());
        Ok(FilterOutcome {
            counts,
            nearest_active,
        })
    }
    /// Whether the filtered rows show `key`.
    fn shows(&self, key: &str) -> Result<bool, RuntimeError> {
        Ok(self.0.items.get()?.index_of(key).is_some())
    }
    /// Applies the last query and matcher again, after the source binding
    /// changed.
    ///
    /// # Errors
    /// Same as [`Self::filter`].
    pub fn refresh(&self) -> Result<FilterOutcome, RuntimeError> {
        let (query, matcher) = self.0.applied.borrow().clone();
        self.filter(&query, &matcher)
    }
}

/// The collection a [`CollectionFilter`] filters.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum FilterTarget {
    /// A [`VirtualList`](crate::VirtualList) bound to [`ListFilter::items`].
    List(ListFilter),
    /// A [`Table`](crate::Table), filtered through
    /// [`TableController::filter`].
    Table(TableController),
    /// A [`Tree`](crate::Tree), filtered through [`TreeController::filter`].
    Tree(TreeController),
}
impl From<ListFilter> for FilterTarget {
    fn from(filter: ListFilter) -> Self {
        Self::List(filter)
    }
}
impl From<TableController> for FilterTarget {
    fn from(table: TableController) -> Self {
        Self::Table(table)
    }
}
impl From<TreeController> for FilterTarget {
    fn from(tree: TreeController) -> Self {
        Self::Tree(tree)
    }
}
impl FilterTarget {
    fn apply(&self, query: &str, matcher: &FilterMatcher) -> Result<FilterOutcome, RuntimeError> {
        match self {
            Self::List(filter) => filter.filter(query, matcher),
            Self::Table(table) => table.filter(query, matcher),
            Self::Tree(tree) => tree.filter(query, matcher),
        }
    }
    /// A table's or tree's source replacements, which reapply the filter in
    /// the controller; a list's source is part of what the wrapper applies.
    fn replaced(&self) -> Option<ReadSignal<Replacement>> {
        match self {
            Self::List(_) => None,
            Self::Table(table) => Some(table.replaced()),
            Self::Tree(tree) => Some(tree.replaced()),
        }
    }
    fn counts(&self) -> Result<Option<FilterCounts>, RuntimeError> {
        match self {
            Self::List(_) => Ok(None),
            Self::Table(table) => table.filter_counts().get().map(Some),
            Self::Tree(tree) => tree.filter_counts().get().map(Some),
        }
    }
    fn selection(&self) -> Option<ListSelection> {
        match self {
            Self::List(filter) => filter.0.selection.borrow().clone(),
            Self::Table(table) => Some(table.selection()),
            Self::Tree(tree) => Some(tree.selection()),
        }
    }
    /// Whether the collection shows the row `key` now: a list's filtered
    /// rows, a table's shown rows, a tree's flattened rows with the
    /// temporary expansion.
    fn shows(&self, key: &str) -> Result<bool, RuntimeError> {
        match self {
            Self::List(filter) => filter.shows(key),
            Self::Table(table) => table.shows(key),
            Self::Tree(tree) => Ok(tree.shows(key)),
        }
    }
}

/// A table's or tree's source replacement as its controller publishes it: a
/// revision, and the active key the replacement hid with the nearest row
/// still shown, if any.
pub(crate) type Replacement = (u64, Option<(String, Option<String>)>);

/// Ids of the field and the collection, shared with the field's key callback.
#[derive(Default)]
struct Parts {
    field: Option<WidgetId>,
    collection: Option<WidgetId>,
}

/// A filter field above a list, table or tree that shows only the rows
/// matching what the user types.
///
/// The field is a [`SearchField`] keyed `filter` in the direct child slot
/// `filter-field`, so its address is the component's followed by
/// `filter-field/filter`, for [`Runtime::find`](crate::Runtime::find),
/// selectors and inspection paths. The field keeps no workspace state. That slot key is reserved, so the
/// collection element must use another key; `filter` is valid for the
/// collection because the inner SearchField's own key, also `filter`, is
/// scoped beneath its slot. A collection keyed `filter-field`
/// makes mounting fail with [`RuntimeError::DuplicateKey`]. The field sits above
/// the collection element given to
/// [`Self::new`], which must show the list, table or tree of the
/// [`FilterTarget`]. As the query changes, the
/// component applies it to the target in the event phase of the same flush:
///
/// - **Lists and tables** hide the rows whose text does not match and keep
///   the others, with their keys, components and scroll anchoring. Rows need
///   [`ListItems::text`] or [`TableItems::text`](crate::TableItems::text).
/// - **Trees** show each matching node with its ancestors, expanded through a
///   temporary layer that leaves [`TreeController::expanded`] untouched;
///   clearing the query restores the user's expansion. See
///   [`TreeController::filter`].
///
/// **Selection and focus.** A selected row that is filtered out stays selected
/// and hidden. When the active item disappears while the collection has
/// focus, the active item moves to the nearest remaining row (in
/// [`SelectionMode::Single`](crate::SelectionMode::Single) the selection
/// follows, since it is the active item); when no row remains and the query
/// is not empty, focus moves to the field and the hidden active item stays.
/// Without focus in the collection, as while the user types in the field,
/// [`Self::hidden_active`] decides: by default ([`HiddenActive::Keep`])
/// nothing moves and the selected row stays hidden;
/// [`HiddenActive::Nearest`] moves the active item as with focus, and
/// [`HiddenActive::Clear`] leaves no item active, so after a filter change
/// evaluated without focus the active item is shown or there is none.
///
/// Application writes and the focused case can still leave a hidden active
/// item, so an action on it checks that it is shown, for example to enable
/// it. Derive that from the shown rows and the active key, which for a list
/// are [`ListFilter::items`] and [`ListSelection::active`]:
/// `filter.items().combine(&selection.active(), |rows, active|
/// active.as_deref().is_some_and(|key| rows.index_of(key).is_some()))`
/// ([`ReadSignal::combine`], [`ListItems::index_of`]). An action on a
/// [`SelectionMode::Multiple`](crate::SelectionMode::Multiple) selection acts
/// on every selected key, so its guard checks those rather than the active
/// one, since hidden selected keys stay selected under every policy:
/// `filter.items().combine(&selection.selected(), |rows, keys|
/// keys.iter().all(|key| rows.index_of(key).is_some()))`. Down in the field
/// moves focus into the collection; Escape clears the query first, as the
/// search field documents.
///
/// **Permission.** [`Self::offered`] decides whether the field is offered at
/// all, as a constant or a binding. While it is not offered the field is not
/// shown and no query applies, so the collection shows every row.
///
/// **Cost.** Each query reads the text of every row (for a tree, every node
/// its source already knows); [`Self::debounce`] waits for a pause in typing
/// before applying it.
///
/// Inspection reports a [`SemanticRole::Group`] with the properties `query`,
/// `shown`, `total`, `offered` and `hidden_active` (`keep`, `nearest` or
/// `clear`). A new description replaces the matcher, the permission, the
/// hidden-active policy, the name and the placeholder (the same binding or
/// role keeps the text already read); the target, query binding and debounce
/// are fixed when it mounts.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{CollectionFilter, ListFilter, ListItems, ListSource, VirtualList};
/// use std::time::Duration;
///
/// struct Fruit;
/// const NAMES: [&str; 3] = ["Apple", "Banana", "Cherry"];
/// impl ListItems for Fruit {
///     fn len(&self) -> usize { NAMES.len() }
///     fn key(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_lowercase()) }
///     fn build(&self, index: usize) -> Option<Element> {
///         Some(Element::new(self.key(index)?, Label::new(NAMES[index])))
///     }
///     fn text(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_string()) }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
/// let source = ui.signal(ListSource::new(Fruit));
/// let filter = ListFilter::new(&ui, source.read_only())?;
/// let list = VirtualList::new(filter.items(), 20.0)?;
/// let query = ui.signal(String::new());
/// ui.mount_tree(Element::new(
///     "fruit",
///     CollectionFilter::with_query(query.clone(), filter.clone(), Element::new("list", list))
///         .name("Filter fruit"),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// query.set("err".to_owned())?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(filter.counts().get()?.shown, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the filter under the same key applies its matcher and permission;
/// a different matcher is applied again at the next delivery, while the same
/// matcher keeps what it applied, including a tree's temporary expansion.
#[must_use]
pub struct CollectionFilter {
    target: FilterTarget,
    query: Signal<String>,
    matcher: FilterMatcher,
    offered: Permission,
    /// The field's name, by default the string table's
    /// [`StringRole::FilterName`](crate::StringRole::FilterName), with the
    /// text last read for inspection.
    name: crate::Caption,
    /// The field's placeholder; `None` shows the name.
    placeholder: Option<TextSource>,
    debounce: Option<Duration>,
    collection: Option<Element>,
    parts: Rc<RefCell<Parts>>,
    /// The query, offer and source last applied; a change requests delivery.
    applied: Option<(String, Option<ListSource>)>,
    /// The query, offer and source the latest update read.
    wanted: (String, Option<ListSource>),
    /// A table's or tree's latest source replacement the update read, and
    /// the revision last handled.
    replacement: Option<Replacement>,
    handled_replacement: Option<u64>,
    /// What an unfocused filter change does with a hidden active item.
    hidden_active: HiddenActive,
    shown_field: bool,
    collection_focused: bool,
    counts: Cell<FilterCounts>,
    notifier: Option<crate::Notifier>,
}
description_debug!(CollectionFilter);
impl CollectionFilter {
    /// Creates a filter field over `collection`, with a query binding of its
    /// own created from `ui`, any [`StateOwner`](crate::StateOwner).
    pub fn new(
        ui: &impl crate::StateOwner,
        target: impl Into<FilterTarget>,
        collection: Element,
    ) -> Self {
        Self::with_query(ui.signal(String::new()), target, collection)
    }
    /// Creates a filter field over `collection` that edits `query`, which the
    /// application may also read and write.
    pub fn with_query(
        query: impl Into<Signal<String>>,
        target: impl Into<FilterTarget>,
        collection: Element,
    ) -> Self {
        let query: Signal<String> = query.into();
        // The installed table's name until the first read.
        let name = query
            .shared()
            .strings
            .borrow()
            .text(crate::StringRole::FilterName)
            .to_owned();
        Self {
            target: target.into(),
            query,
            matcher: FilterMatcher::default(),
            offered: Permission::allowed(),
            name: crate::Caption::new(TextSource::library(crate::StringRole::FilterName, name)),
            placeholder: None,
            debounce: None,
            collection: Some(collection),
            parts: Rc::new(RefCell::new(Parts::default())),
            applied: None,
            wanted: (String::new(), None),
            replacement: None,
            handled_replacement: None,
            hidden_active: HiddenActive::Keep,
            shown_field: true,
            collection_focused: false,
            counts: Cell::new(FilterCounts::default()),
            notifier: None,
        }
    }
    fn field(&self) -> Element {
        Element::new(
            FIELD_SLOT,
            FilterField {
                query: self.query.clone(),
                parts: self.parts.clone(),
                name: self.name.source().clone(),
                placeholder: self.placeholder.clone(),
                debounce: self.debounce,
                described: None,
                children: Vec::new(),
            },
        )
        .slot(FIELD_SLOT)
    }
    /// The rule rows must match; [`FilterMatcher::substring`] by default.
    pub fn matcher(mut self, matcher: FilterMatcher) -> Self {
        self.matcher = matcher;
        self
    }
    /// The field's accessible name and default placeholder, as a
    /// [`TextSource`]: fixed text, a binding the name follows, or a
    /// [`StringRole`](crate::StringRole) of the installed
    /// [`UiStrings`](crate::UiStrings). By default the table supplies
    /// [`StringRole::FilterName`](crate::StringRole::FilterName), "Filter" in
    /// English, and the field follows a table installed later. Only the plain
    /// text is used. Redescription applies a new name; the same binding or
    /// role keeps the name already read.
    pub fn name(mut self, name: impl Into<TextSource>) -> Self {
        self.name.set_source(name.into());
        self
    }
    /// Text the empty field shows instead of the name, as a [`TextSource`]
    /// like [`Self::name`]. Redescription applies a new placeholder, and
    /// describing the filter without one shows the name again.
    pub fn placeholder(mut self, text: impl Into<TextSource>) -> Self {
        self.placeholder = Some(text.into());
        self
    }
    /// Applies a query only once typing pauses for `delay`, through the search
    /// field's debounce; without it every edit applies at once. Fixed when the
    /// component mounts.
    pub fn debounce(mut self, delay: Duration) -> Self {
        self.debounce = Some(delay);
        self
    }
    /// Whether the field is offered, as a constant or a binding; offered by
    /// default. While it is not offered the field is hidden and the
    /// collection shows every row.
    /// Repeated calls combine restrictions; a fixed denial cannot be lifted
    /// by a later permission on the same description.
    pub fn offered(mut self, offered: impl Into<Permission>) -> Self {
        self.offered = self.offered.and(offered);
        self
    }
    /// Chooses what happens to an active item that a filter change leaves
    /// hidden while the collection lacks focus, such as while the user types
    /// in the field. Default [`HiddenActive::Keep`]; the variants describe
    /// each policy. With focus in the collection the active item moves to
    /// the nearest row whatever the policy. Redescription replaces it, and a
    /// new policy applies from the next delivered filter change; changing it
    /// moves nothing by itself.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::{HiddenActive, ListItems, ListSource};
    /// use std::time::Duration;
    ///
    /// struct Fruit;
    /// const NAMES: [&str; 3] = ["Apple", "Banana", "Cherry"];
    /// impl ListItems for Fruit {
    ///     fn len(&self) -> usize { NAMES.len() }
    ///     fn key(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_lowercase()) }
    ///     fn build(&self, index: usize) -> Option<Element> {
    ///         Some(Element::new(self.key(index)?, Label::new(NAMES[index])))
    ///     }
    ///     fn text(&self, index: usize) -> Option<String> { NAMES.get(index).map(|n| n.to_string()) }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    /// let source = ui.signal(ListSource::new(Fruit));
    /// let selection = ListSelection::new(&ui, SelectionMode::Single);
    /// let filter = ListFilter::new(&ui, source.read_only())?.selection(selection.clone());
    /// let list = VirtualList::new(filter.items(), 20.0)?.selection(selection.clone());
    /// let query = ui.signal(String::new());
    /// ui.mount_tree(Element::new(
    ///     "fruit",
    ///     CollectionFilter::with_query(query.clone(), filter, Element::new("list", list))
    ///         .hidden_active(HiddenActive::Clear),
    /// ))?;
    /// selection.set_active(Some("apple".to_owned()))?;
    /// ui.flush(Duration::ZERO)?;
    /// // The list lacks focus, as while the user types in the field.
    /// query.set("an".to_owned())?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(selection.active().get()?, None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn hidden_active(mut self, policy: HiddenActive) -> Self {
        self.hidden_active = policy;
        self
    }
    /// The query binding the field edits.
    #[must_use]
    pub fn query(&self) -> Signal<String> {
        self.query.clone()
    }
    fn gap(cx: &MeasureContext<'_>) -> f32 {
        cx.metric(MetricRole::ItemGap)
    }
    /// Applies the hidden-active policy after a filter change evaluated
    /// without focus in the collection. `hidden` is the active key the
    /// change hid, with its nearest row still shown.
    fn unfocused_hidden(
        &self,
        selection: &ListSelection,
        hidden: Option<(String, Option<String>)>,
    ) -> Result<(), RuntimeError> {
        if self.hidden_active == HiddenActive::Keep {
            return Ok(());
        }
        let Some(active) = selection.active().get()? else {
            return Ok(());
        };
        if self.target.shows(&active)? {
            return Ok(());
        }
        match self.hidden_active {
            // A state rule: whenever it became hidden.
            HiddenActive::Clear => selection.set_active(None),
            // Only the transition this change made for this active key.
            HiddenActive::Nearest => match hidden {
                Some((key, Some(nearest))) if key == active => selection.set_active(Some(nearest)),
                _ => Ok(()),
            },
            HiddenActive::Keep => Ok(()),
        }
    }
}

impl Component for CollectionFilter {
    fn take_children(&mut self) -> Vec<Element> {
        let mut children = vec![self.field()];
        children.extend(self.collection.take());
        children
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // A different matcher applies at the next delivery. The same one, as
        // when an owner describes its parts again for another string table,
        // keeps what was applied, so a tree's temporary layer survives.
        if !self.matcher.same(&next.matcher) {
            self.applied = None;
        }
        self.matcher = next.matcher;
        self.offered = next.offered;
        self.hidden_active = next.hidden_active;
        // The field slot adopts the new name and placeholder itself.
        self.name.reconcile(next.name);
        self.placeholder = next.placeholder;
        Some(Invalidation::Layout)
    }
    fn child_visible(&self, key: &str) -> bool {
        key != FIELD_SLOT || self.shown_field
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // The name is only reported here; the field slot draws it.
        self.name.update_paint(cx)?;
        let offered = self.offered.read(cx)?;
        if offered != self.shown_field {
            self.shown_field = offered;
            cx.request_layout();
        }
        let query = if offered {
            cx.read(&self.query)?
        } else {
            String::new()
        };
        let source = match &self.target {
            FilterTarget::List(filter) => Some(cx.read(&filter.0.source)?),
            _ => None,
        };
        {
            let mut parts = self.parts.borrow_mut();
            for child in cx.children() {
                if child.key() == FIELD_SLOT {
                    parts.field = Some(child.id().clone());
                } else {
                    parts.collection = Some(child.id().clone());
                    self.collection_focused = child.focus_within();
                }
            }
        }
        self.wanted = (query, source);
        self.replacement = match self.target.replaced() {
            Some(replaced) => Some(cx.read(&replaced)?),
            None => None,
        };
        let replaced = self.replacement.as_ref().map(|(revision, _)| *revision);
        if self.handled_replacement.is_none() {
            // Replacements before mounting are not this wrapper's to follow.
            self.handled_replacement = replaced;
        }
        let notifier = self.notifier.get_or_insert_with(|| cx.notifier());
        if self.applied.as_ref() != Some(&self.wanted) || replaced != self.handled_replacement {
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let replacement = self
            .replacement
            .clone()
            .filter(|(revision, _)| Some(*revision) != self.handled_replacement);
        if let Some((revision, _)) = &replacement {
            self.handled_replacement = Some(*revision);
        }
        // The change and the active key it hid with its nearest row: a
        // query applied over the current source supplies its own transition,
        // which takes precedence over a replacement in the same delivery.
        let (outcome, hidden) = if self.applied.as_ref() != Some(&self.wanted) {
            let active = match self.target.selection() {
                Some(selection) => selection.active().get()?,
                None => None,
            };
            let outcome = self.target.apply(&self.wanted.0, &self.matcher)?;
            self.applied = Some(self.wanted.clone());
            let hidden = active
                .zip(outcome.nearest_active.clone())
                .map(|(key, nearest)| (key, Some(nearest)));
            (outcome, hidden)
        } else if let Some((_, hid)) = replacement {
            // The controller reapplied the filter to its new source; follow
            // its counts and the active row it hid, as for a query change.
            let outcome = FilterOutcome {
                counts: self.target.counts()?.unwrap_or_default(),
                nearest_active: hid.as_ref().and_then(|(_, nearest)| nearest.clone()),
            };
            (outcome, hid)
        } else {
            return Ok(());
        };
        self.counts.set(outcome.counts);
        cx.invalidate(Invalidation::Paint);
        if self.collection_focused {
            if let (Some(key), Some(selection)) = (&outcome.nearest_active, self.target.selection())
            {
                // The collection has focus, so the query or source changed
                // elsewhere: an application write, reported as such.
                selection.set_active(Some(key.clone()))?;
            }
            let field = self.parts.borrow().field.clone();
            if outcome.counts.shown == 0
                && effective(&self.wanted.0).is_some()
                && let Some(field) = field
            {
                cx.request_focus_for(&field)?;
            }
        } else if let Some(selection) = self.target.selection() {
            self.unfocused_hidden(&selection, hidden)?;
        }
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let mut width: f32 = 0.0;
        let mut height: f32 = 0.0;
        for index in 0..cx.len() {
            let minimum = cx.limits(index)?.minimum();
            width = width.max(minimum.width());
            height += minimum.height();
        }
        if cx.len() > 1 {
            height += Self::gap(cx);
        }
        Ok(SizeLimits::at_least(Size::new(width, height)?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        match cx.len() {
            0 => Ok(Size::ZERO),
            1 => cx.measure(0, available),
            2 => {
                let field = cx.measure(0, Constraints::new(available.width(), None)?)?;
                let collection = cx.measure(1, available)?;
                Ok(Size::new(
                    field.width().max(collection.width()),
                    field.height() + Self::gap(cx) + collection.height(),
                )?)
            }
            _ => Err(LayoutError::InvalidChild.into()),
        }
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        match cx.len() {
            0 => Ok(()),
            1 => cx.place(0, Rect::from_size(size)),
            2 => {
                let field = cx
                    .measure(0, Constraints::new(Some(size.width()), None)?)?
                    .height()
                    .min(size.height());
                cx.place(0, Rect::new(0.0, 0.0, size.width(), field)?)?;
                let top = (field + Self::gap(cx.measurement())).min(size.height());
                cx.place(
                    1,
                    Rect::new(0.0, top, size.width(), (size.height() - top).max(0.0))?,
                )
            }
            _ => Err(LayoutError::InvalidChild.into()),
        }
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let counts = self.counts.get();
        let query = self
            .applied
            .as_ref()
            .map(|(q, _)| q.clone())
            .unwrap_or_default();
        details
            .properties
            .insert("query".into(), InspectValue::Text(query));
        details
            .properties
            .insert("shown".into(), InspectValue::Number(counts.shown as f64));
        details
            .properties
            .insert("total".into(), InspectValue::Number(counts.total as f64));
        details
            .properties
            .insert("offered".into(), InspectValue::Bool(self.shown_field));
        details.properties.insert(
            "hidden_active".into(),
            InspectValue::Text(self.hidden_active.name().into()),
        );
        details.semantics = Some(Semantics::new(SemanticRole::Group).name(self.name.text()));
        details
    }
}

/// The key of the slot that holds the filter's search field.
const FIELD_SLOT: &str = "filter-field";

/// The `filter-field` slot holding a SearchField keyed `filter`, described again when its
/// name or placeholder reads differently, so a binding or a replaced string
/// table relabels it.
struct FilterField {
    query: Signal<String>,
    parts: Rc<RefCell<Parts>>,
    name: TextSource,
    placeholder: Option<TextSource>,
    debounce: Option<Duration>,
    /// The name and placeholder the field was last described with.
    described: Option<(String, String)>,
    children: Vec<Element>,
}
impl Component for FilterField {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let same_placeholder = match (&self.placeholder, &next.placeholder) {
            (Some(mine), Some(theirs)) => mine.same_as(theirs),
            (None, None) => true,
            _ => false,
        };
        if self.name.same_as(&next.name) && same_placeholder {
            return None;
        }
        self.name = next.name;
        self.placeholder = next.placeholder;
        Some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let name = self.name.read(cx)?.plain_text();
        let placeholder = match &self.placeholder {
            Some(placeholder) => placeholder.read(cx)?.plain_text(),
            None => name.clone(),
        };
        let texts = (name.clone(), placeholder.clone());
        if self.described.as_ref() == Some(&texts) {
            return Ok(());
        }
        let parts = self.parts.clone();
        let mut field = SearchField::new(self.query.clone()).on_key(move |cx, key| {
            if key == SearchKey::Down
                && let Some(collection) = parts.borrow().collection.clone()
            {
                cx.request_focus_within(&collection)?;
            }
            Ok(())
        });
        field = field.placeholder(placeholder);
        if let Some(delay) = self.debounce {
            field = field.debounce(delay);
        }
        cx.request_children(vec![Element::new("filter", field).with_label(name.clone())]);
        cx.request_layout();
        self.described = Some(texts);
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(0, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::new(0.0, 0.0, size.width(), size.height())?)?;
        }
        Ok(())
    }
}
