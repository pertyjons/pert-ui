//! Searching the application's own content: semantic names and values of
//! mounted components, and the searchable rows collections offer.
use crate::{ListRevealReceipt, ListRevealStatus, RevealOutcome, RuntimeError, SemanticRole};

/// What a [content search](crate::Runtime::content_search) looks for, with
/// its result limit and work budget.
///
/// Matching is case-insensitive substring matching over each component's
/// resolved semantic name and value text and each collection row's text.
/// [`Self::everything`] matches every searchable item instead, which is how
/// an application lists content for a picker that ranks it itself.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{ContentQuery, ContentSearchError};
/// let query = ContentQuery::new("gain")?.limit(10)?.budget(500)?;
/// assert_eq!(query.text(), Some("gain"));
/// assert_eq!(query.result_limit(), 10);
/// assert!(matches!(ContentQuery::new("  "), Err(ContentSearchError::EmptyQuery)));
/// # Ok::<(), ContentSearchError>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentQuery {
    /// Lowercased needle; `None` matches everything.
    needle: Option<String>,
    text: Option<String>,
    limit: usize,
    budget: usize,
    hidden: bool,
    rows: bool,
}

impl ContentQuery {
    /// Default number of hits one step returns before it stops: 50.
    pub const DEFAULT_LIMIT: usize = 50;
    /// Default work per step: 10 000 units, one per component examined while
    /// finding the window roots, one per component visited, one per mounted
    /// row indexed and one per collection row examined.
    pub const DEFAULT_BUDGET: usize = 10_000;

    /// Searches for `text`, trimmed.
    ///
    /// # Errors
    /// [`ContentSearchError::EmptyQuery`] for text that is blank after
    /// trimming; use [`Self::everything`] to list all content.
    pub fn new(text: impl AsRef<str>) -> Result<Self, ContentSearchError> {
        let text = text.as_ref().trim();
        if text.is_empty() {
            return Err(ContentSearchError::EmptyQuery);
        }
        Ok(Self {
            needle: Some(text.to_lowercase()),
            text: Some(text.to_owned()),
            limit: Self::DEFAULT_LIMIT,
            budget: Self::DEFAULT_BUDGET,
            hidden: true,
            rows: true,
        })
    }
    /// Matches every searchable item, for a caller that ranks the content
    /// itself, such as a [`QuickPick`](crate::QuickPick).
    #[must_use]
    pub fn everything() -> Self {
        Self {
            needle: None,
            text: None,
            limit: Self::DEFAULT_LIMIT,
            budget: Self::DEFAULT_BUDGET,
            hidden: true,
            rows: true,
        }
    }
    /// Sets the most hits one step returns.
    ///
    /// # Errors
    /// [`ContentSearchError::InvalidLimit`] for zero.
    pub fn limit(mut self, limit: usize) -> Result<Self, ContentSearchError> {
        if limit == 0 {
            return Err(ContentSearchError::InvalidLimit);
        }
        self.limit = limit;
        Ok(self)
    }
    /// Sets the work one step may do: one unit per component examined while
    /// finding the window roots, per component visited, per mounted row
    /// indexed and per collection row examined, whether it matches or not. A
    /// step that runs out returns what it found and can be continued.
    ///
    /// # Errors
    /// [`ContentSearchError::InvalidLimit`] for zero.
    pub fn budget(mut self, budget: usize) -> Result<Self, ContentSearchError> {
        if budget == 0 {
            return Err(ContentSearchError::InvalidLimit);
        }
        self.budget = budget;
        Ok(self)
    }
    /// Whether components that are not presented (an unselected tab, a
    /// collapsed section) are searched; true by default.
    #[must_use]
    pub fn hidden(mut self, include: bool) -> Self {
        self.hidden = include;
        self
    }
    /// Whether collections' rows that are not mounted are searched through
    /// their [`ContentRows`]; true by default. A caller listing
    /// [everything](Self::everything) for a picker usually turns it off, since
    /// a long collection would fill the list with its rows.
    #[must_use]
    pub fn rows(mut self, include: bool) -> Self {
        self.rows = include;
        self
    }
    /// Whether collections' unmounted rows are searched.
    #[must_use]
    pub fn includes_rows(&self) -> bool {
        self.rows
    }
    /// The trimmed search text, or `None` for [`Self::everything`].
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
    /// The most hits one step returns.
    #[must_use]
    pub fn result_limit(&self) -> usize {
        self.limit
    }
    /// The work one step may do.
    #[must_use]
    pub fn work_budget(&self) -> usize {
        self.budget
    }
    /// Whether hidden components are searched.
    #[must_use]
    pub fn includes_hidden(&self) -> bool {
        self.hidden
    }
    pub(crate) fn matches(&self, text: &str) -> bool {
        self.needle
            .as_deref()
            .is_none_or(|needle| text.to_lowercase().contains(needle))
    }
}

/// Where a content hit is, and how a jump reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentTarget {
    /// A mounted component.
    Widget(crate::WidgetId),
    /// A collection row that is not mounted, found through the collection's
    /// [`ContentRows`]; a jump reveals it by key first.
    Row {
        /// The collection component that offered the row.
        collection: crate::WidgetId,
        /// The row's key in that collection.
        key: String,
    },
}

/// Which text of the hit matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentField {
    /// The component's resolved semantic name.
    Name,
    /// The component's semantic value.
    Value,
    /// A collection row's searchable text.
    Row,
}

/// One item content search found.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContentHit {
    /// Where the hit is.
    pub target: ContentTarget,
    /// The component's semantic role; `None` for a row that is not mounted.
    pub role: Option<SemanticRole>,
    /// The text that matched, as the component or source reported it.
    pub text: String,
    /// Which text matched.
    pub field: ContentField,
    /// The component's other text: its value when the name matched, its name
    /// when the value matched, so a picker listing
    /// [everything](ContentQuery::everything) can match either. `None` when
    /// the component has no other text, for a row, and never the value of a
    /// [protected](crate::Semantics::protected) component.
    pub other: Option<String>,
    /// The addressable key path of the component, or of the collection for a
    /// row, as inspection and the workspace use it. Internal structure, such
    /// as a table's body list, reports its nearest addressable ancestor's
    /// path; `None` only where no ancestor has one.
    pub path: Option<Vec<String>>,
    /// Whether the component is presented now: false inside an unselected
    /// tab or a collapsed section. Always false for a row that is not mounted.
    pub visible: bool,
}

/// What one [`ContentSearch::step`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ContentStep {
    /// Hits in this step, in tree order.
    pub hits: Vec<ContentHit>,
    /// Whether the whole tree and every offered collection have been
    /// searched; false when the step stopped at its limit or budget.
    pub done: bool,
    /// Work units this step used.
    pub work: usize,
}

/// A resumable content search over one runtime's mounted tree.
///
/// Created by [`Runtime::content_search`](crate::Runtime::content_search).
/// Each [`Self::step`] does at most the query's work budget and returns at
/// most its result limit, so a search with no matches over a large tree is
/// bounded per call; call it again to continue where it stopped. The search
/// walks components in tree order (window roots in mount order, children in
/// their order) and, within a collection that offers [`ContentRows`], the rows
/// that are not mounted, after the collection itself.
///
/// The search is a cursor, not a snapshot. If a component mounts or unmounts,
/// an element's [searchability](crate::Element::with_searchable) changes, or
/// the rows of any collection the search has started on change, between two
/// steps, the next step returns [`ContentSearchError::Stale`] and the caller
/// starts a new search. The cursor retains each visited collection's owned row
/// source until it is dropped, including sources later replaced by the host.
///
/// The budget counts every unit of work that grows with the tree or a
/// collection: discovering the window roots (one unit per mounted component
/// examined), visiting a component, indexing a collection's mounted children
/// (one unit each) and examining a collection row. Besides that, each step
/// reads the revision of every collection the search has already started on,
/// once, without charging it.
#[derive(Debug)]
pub struct ContentSearch {
    pub(crate) query: ContentQuery,
    /// The next entry serial to examine while window roots are still being
    /// discovered; `None` once discovery is finished.
    pub(crate) discover: Option<u64>,
    /// Window roots found so far, in mount order.
    pub(crate) roots: Vec<u64>,
    /// Components still to visit, in reverse so the next is last.
    pub(crate) pending: Vec<u64>,
    /// A collection whose unmounted rows are being searched.
    pub(crate) rows: Option<RowScan>,
    /// Every collection whose rows the search has started, with the rows it
    /// saw, so a later step notices one of them changing.
    pub(crate) visited: Vec<VisitedRows>,
    pub(crate) generation: u64,
    pub(crate) started: bool,
    pub(crate) done: bool,
    /// Identity of the runtime that created the search.
    pub(crate) owner: usize,
}

/// A collection's rows as the search first saw them.
pub(crate) struct VisitedRows {
    pub(crate) collection: u64,
    pub(crate) revision: u64,
    /// Kept so the rows' identity, which a revision may derive from, cannot
    /// be reused by another source while the search runs.
    pub(crate) _rows: std::rc::Rc<dyn ContentRows>,
}
impl std::fmt::Debug for VisitedRows {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisitedRows")
            .field("collection", &self.collection)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub(crate) struct RowScan {
    pub(crate) collection: u64,
    pub(crate) revision: u64,
    /// The next of the collection's mounted children to index; rows are
    /// scanned once every child is indexed.
    pub(crate) children_next: usize,
    pub(crate) next: usize,
    pub(crate) mounted: std::collections::BTreeSet<String>,
    pub(crate) path: Option<Vec<String>>,
}

impl ContentSearch {
    pub(crate) fn new(query: ContentQuery, owner: usize) -> Self {
        Self {
            query,
            discover: None,
            roots: Vec::new(),
            pending: Vec::new(),
            rows: None,
            visited: Vec::new(),
            generation: 0,
            started: false,
            done: false,
            owner,
        }
    }
    /// The query this search answers.
    #[must_use]
    pub fn query(&self) -> &ContentQuery {
        &self.query
    }
    /// Whether the search has visited everything.
    #[must_use]
    pub fn is_done(&self) -> bool {
        self.done
    }
    /// Searches on from where the previous step stopped, doing at most the
    /// query's work budget and returning at most its result limit.
    ///
    /// It runs `Component::inspect` and [`Component::try_content_rows`] on the
    /// components it visits, under the inspection phase, so it writes no
    /// signal and schedules nothing. A step after the search is done returns
    /// no hits and `done`.
    ///
    /// # Errors
    /// [`ContentSearchError::Stale`] when the tree or a collection changed
    /// since the previous step, [`ContentSearchError::ForeignRuntime`] when
    /// `ui` is not the runtime that created the search, and
    /// [`ContentSearchError::Runtime`] for a collection's failing
    /// row read through [`Component::try_content_rows`]. After any error, discard
    /// this cursor and start a new search once the cause is resolved; a failed
    /// step may already have consumed part of its traversal without returning hits.
    ///
    /// [`Component::try_content_rows`]: crate::Component::try_content_rows
    pub fn step(&mut self, ui: &crate::Runtime) -> Result<ContentStep, ContentSearchError> {
        ui.content_step(self)
    }
}

/// Searchable rows a collection offers beyond the ones it has mounted.
///
/// A collection component returns one from
/// [`Component::try_content_rows`](crate::Component::try_content_rows), whose
/// default delegates to [`Component::content_rows`](crate::Component::content_rows); `VirtualList`
/// does, so `Table` and `Tree` rows are offered through it. Content search
/// asks for the text of each row that is not currently represented by mounted
/// children (current mounted rows are found as components) and, for a hit,
/// reveals the row by key and then focuses its
/// widget. A custom collection implements it the same way.
///
/// Implementations answer from data they already hold: deterministic, free
/// of side effects and I/O, and never forcing a lazy source to load. The
/// cost is the implementation owner's, once per row a search examines. An
/// implementation is `'static`: it owns its rows or shares them through an
/// `Rc`, never borrowing from the component (see
/// [`Component::content_rows`](crate::Component::content_rows)).
pub trait ContentRows {
    /// Number of rows.
    fn len(&self) -> usize;
    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Stable key of the row at `index`, or `None` past the end.
    fn key(&self, index: usize) -> Option<String>;
    /// Searchable text of the row at `index`; `None` when the row offers
    /// none, so it is found only while represented by current mounted content.
    fn text(&self, index: usize) -> Option<String>;
    /// Changes whenever the rows change, so a resumable search can tell that
    /// the rows it was walking are no longer the same.
    fn revision(&self) -> u64;
    /// Whether the collection's mounted row children describe these current
    /// rows. The default is `true`: search visits those children as components
    /// and skips their keys when scanning this source to avoid duplicate hits.
    ///
    /// Return `false` after replacing a source while retained children still
    /// describe an earlier one, for example in a hidden collection. Search then
    /// skips all that collection's child subtrees and reads every offered source
    /// row instead, without updating or mounting presentation. A provider using
    /// `false` must keep ancillary controls outside those child subtrees; the
    /// collection's own semantic content remains searchable. With source-row
    /// search disabled, obsolete children are still skipped. Implementations
    /// must keep the result consistent with the source/revision they return;
    /// only return `true` once mounted row content describes that source.
    fn mounted_rows_current(&self) -> bool {
        true
    }
    /// Asks the collection to scroll to and mount the row with `key`.
    ///
    /// # Errors
    /// The collection's request failures, such as a signal write refused in
    /// the current phase.
    fn reveal(&self, key: &str) -> Result<ListRevealReceipt, RuntimeError>;
}

/// The outcome of [`Runtime::jump_to_content`](crate::Runtime::jump_to_content).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ContentJump {
    /// The target was revealed, and focused if it can take focus; or the
    /// reveal was refused, as the outcome says.
    Reveal(RevealOutcome),
    /// A row that was not mounted could not be mounted: the collection no
    /// longer has it, or the request was superseded or cancelled.
    Row(ListRevealStatus),
    /// A row that was not mounted is still being mounted, for example by a
    /// collection that serves the request only once it is shown. Pass the
    /// pending jump to
    /// [`Runtime::resume_content_jump`](crate::Runtime::resume_content_jump)
    /// after the collection has mounted the row, which its
    /// [receipt](PendingJump::receipt) reports, to reveal and focus it.
    Pending(PendingJump),
}

/// A content jump waiting for its row to mount; see [`ContentJump::Pending`].
///
/// It is a continuation, not a receipt: it holds the collection's reveal
/// receipt, which [`Self::receipt`] lends, so the row's outcome is never
/// lost: it reaches `Mounted`, `NotFound`, `Superseded` or `Cancelled`, and
/// [`Runtime::resume_content_jump`](crate::Runtime::resume_content_jump)
/// turns that into the jump's final outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingJump {
    pub(crate) receipt: ListRevealReceipt,
}
impl PendingJump {
    /// The collection's receipt for the row's reveal. Its
    /// [`status`](ListRevealReceipt::status) is `Pending` until the collection
    /// settles it, and its
    /// [`status_binding`](ListRevealReceipt::status_binding) lets a component
    /// observe the row mounting without polling.
    #[must_use]
    pub fn receipt(&self) -> &ListRevealReceipt {
        &self.receipt
    }
}

/// Why a content query or search step failed.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ContentSearchError {
    /// The search text is blank.
    #[error("the content search text is empty")]
    EmptyQuery,
    /// A result limit or work budget of zero.
    #[error("a content search limit or budget must be at least one")]
    InvalidLimit,
    /// The tree or a collection changed since the previous step; start a new
    /// search.
    #[error("the content changed since the search started")]
    Stale,
    /// The search belongs to another runtime.
    #[error("the content search belongs to another runtime")]
    ForeignRuntime,
    /// A runtime operation failed.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}
