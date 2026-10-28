//! Rows of cells under column headers, virtualized through the list.
use crate::{
    ArrangeContext, ColorRole, Component, ComponentDetails, Constraints, CursorIcon,
    DRAG_THRESHOLD, DragEvent, DragOutcome, DragPayload, DragPhase, DragPreview, DragRole,
    DrawError, DropOperation, DropOperations, DropResponse, Element, EventContext, Icon,
    InputOutcome, InspectValue, Invalidation, KeyCode, KeyEvent, KeyTransition, KeyboardState,
    Label, LayoutError, Length, ListItems, ListSelection, ListSource, MeasureContext, MetricRole,
    Painter, Permission, Point, PointerButton, PointerEvent, PointerKind, ReadSignal, Rect,
    RichText, RuntimeError, SemanticAction, SemanticItem, SemanticRole, SemanticValue, Semantics,
    Signal, Size, TextLayout, TextSource, TextStyle, UpdateContext, VirtualList, WidgetId,
    scrollbar,
    virtual_list::{AUTOSCROLL_INTERVAL, AUTOSCROLL_STEP, AUTOSCROLL_ZONE, ListController},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

mod editing;
pub use editing::{CellEdit, CellEditEnd, CellEditEndReason};
use editing::{CellEditorFn, EditEndFn, EditEndReporter, Editing};

/// One column of a [`Table`]: identity, title, width and what its header offers.
///
/// A column is one weighted share of the width by default, never narrower than
/// 40 logical pixels. It is neither resizable, sortable nor movable until asked.
///
/// Two columns are equal when every setting is equal and their titles come
/// from equal fixed text or the same binding (see [`TextSource::same_as`]);
/// the text last read from a binding is not compared.
#[derive(Clone, Debug)]
#[must_use]
pub struct TableColumn {
    key: String,
    /// Where the title comes from.
    source: TextSource,
    /// The title shown now: the fixed text, or the binding's last read.
    content: RichText,
    /// `content` as plain text, for names and inspection.
    title: String,
    width: ColumnWidth,
    minimum: f32,
    resizable: bool,
    sortable: bool,
    movable: bool,
    editable: bool,
}
impl PartialEq for TableColumn {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.source.same_as(&other.source)
            && self.width == other.width
            && self.minimum == other.minimum
            && self.resizable == other.resizable
            && self.sortable == other.sortable
            && self.movable == other.movable
            && self.editable == other.editable
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum ColumnWidth {
    Fixed(f32),
    Flex(f32),
}
impl TableColumn {
    /// The narrowest a column can be laid out or dragged, in logical pixels.
    ///
    /// Column positions saturate at the largest finite `f32`, so a column that
    /// starts there, after columns whose widths add up to it, has no room
    /// left and is laid out with a width of 0. Only such a saturated column
    /// is narrower than this minimum.
    pub const MIN_WIDTH: f32 = 40.0;

    /// Creates a column with one share of the width left by fixed columns. The
    /// key identifies the column to [`TableItems::cell`] and in [`TableSort`];
    /// the title is what the header shows.
    ///
    /// The title is a [`TextSource`]: fixed plain or rich text, or a binding
    /// the header follows, such as a translated title. The mounted [`Table`]
    /// reads a binding in its update and lays the header out again when it
    /// changes; until that first read a bound title is empty. Its plain text
    /// names the column's header item and its cells.
    pub fn new(key: impl Into<String>, title: impl Into<TextSource>) -> Self {
        let source = title.into();
        let content = source.fixed().cloned().unwrap_or_default();
        Self {
            key: key.into(),
            title: content.plain_text(),
            content,
            source,
            width: ColumnWidth::Flex(1.0),
            minimum: Self::MIN_WIDTH,
            resizable: false,
            sortable: false,
            movable: false,
            editable: false,
        }
    }
    /// Uses a fixed width, never below [`TableColumn::MIN_WIDTH`]: a smaller
    /// width is raised to that layout minimum rather than rejected.
    pub fn width(mut self, width: Length) -> Self {
        self.width = ColumnWidth::Fixed(width.get().max(self.minimum));
        self
    }
    /// Uses a weighted share of the width left by fixed columns.
    ///
    /// # Errors
    /// Returns [`LayoutError::InvalidWeight`] for a weight that is not finite
    /// and positive.
    pub fn flex(mut self, weight: f32) -> Result<Self, LayoutError> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(LayoutError::InvalidWeight);
        }
        self.width = ColumnWidth::Flex(weight);
        Ok(self)
    }
    /// Whether the user may drag the column's trailing edge in the header
    /// (default false). A dragged column becomes fixed at the width it was
    /// given.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
    /// Whether a press on the header requests sorting by this column (default
    /// false).
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }
    /// Whether the user may drag the column's title to another place, or move
    /// it with Alt and Left or Right while its cell is active (default false);
    /// see [`Table`].
    ///
    /// A column moves only among the movable columns next to it, so a column
    /// that is not movable keeps its place: making every column but the first
    /// movable keeps the first where it is.
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }
    /// Whether the table edits this column's cells in place with its
    /// [`Table::cell_editor`]. Default false. A cell of an editable column
    /// is edited only when [`TableItems::editable`] also allows it.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }
    /// The column's key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    /// The plain text of the header title as shown now: the fixed title, or
    /// the last text the mounted table read from a bound title (empty before
    /// the first read).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }
    /// Shows `content` as the title; returns whether it changed.
    fn set_title(&mut self, content: RichText) -> bool {
        if self.content == content {
            return false;
        }
        self.title = content.plain_text();
        self.content = content;
        true
    }
    /// The fixed width in logical pixels, never below
    /// [`TableColumn::MIN_WIDTH`], or `None` for a weighted column. This is
    /// the width the application gave with [`TableColumn::width`], or the
    /// width the user dragged a resized column to, which fixes it.
    #[must_use]
    pub const fn fixed_width(&self) -> Option<f32> {
        match self.width {
            ColumnWidth::Fixed(width) => Some(width),
            ColumnWidth::Flex(_) => None,
        }
    }
    /// The weight of the column's share of the width left by fixed columns,
    /// or `None` for a fixed column.
    #[must_use]
    pub const fn flex_weight(&self) -> Option<f32> {
        match self.width {
            ColumnWidth::Fixed(_) => None,
            ColumnWidth::Flex(weight) => Some(weight),
        }
    }
    /// Whether this column offers header resizing; see [`TableColumn::resizable`].
    /// [`Table::resize_permission`] can further restrict this capability.
    #[must_use]
    pub const fn is_resizable(&self) -> bool {
        self.resizable
    }
    /// Whether a press on the title requests sorting; see
    /// [`TableColumn::sortable`].
    #[must_use]
    pub const fn is_sortable(&self) -> bool {
        self.sortable
    }
    /// Whether this column offers movement; see [`TableColumn::movable`].
    /// [`Table::reorder_permission`] can further restrict this capability.
    #[must_use]
    pub const fn is_movable(&self) -> bool {
        self.movable
    }
    /// Whether the table edits this column's cells in place; see
    /// [`TableColumn::editable`].
    #[must_use]
    pub const fn is_editable(&self) -> bool {
        self.editable
    }
}

/// Which way a column is sorted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    /// Smallest first; shown as an upward arrow.
    Ascending,
    /// Largest first; shown as a downward arrow.
    Descending,
}

/// The column a table is sorted by, as the header shows it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSort {
    /// The sorted column's key.
    pub column: String,
    /// The direction.
    pub direction: SortDirection,
}

/// The rows of a [`Table`], addressed by position and stable key, with one cell
/// per row and column.
///
/// The rules of [`ListItems`] apply: keys are row identity, no two rows share one,
/// and the table never iterates every row. `cell` returns the content for one row
/// and column under a key of the application's choosing: the table wraps it in
/// a cell keyed by the column, so the content is at `<table>/<row key>/<column
/// key>/<content key>` ([`Table`](crate::Table#addresses-and-tests)). Content
/// may be keyed like its column, which
/// [`Runtime::find_descendant`](crate::Runtime::find_descendant) below the row
/// then refuses as ambiguous, while the full address resolves it.
/// A row exists when `key` answers for its index; its cells may be missing.
/// The order of rows is the application's: the table never sorts them.
/// An implementation is `'static`, as [`TableSource::new`] requires: it owns its
/// rows or shares them through an `Rc`, never borrowing from a local or a
/// component (see [`Component::content_rows`](crate::Component::content_rows)).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as table rows",
    label = "`{Self}` does not implement `TableItems`",
    note = "implement `TableItems` (`len`, `key`, `cell`) on a type that owns or shares the rows"
)]
pub trait TableItems {
    /// Number of rows.
    fn len(&self) -> usize;
    /// Whether there are no rows.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Stable key of the row at `index`, or `None` past the end.
    fn key(&self, index: usize) -> Option<String>;
    /// The content of row `index` in the column with key `column`, or `None`
    /// for an empty cell, which the table draws as its padding alone. It is
    /// addressed by its own key below the cell, at `<table>/<row key>/<column
    /// key>/<content key>`.
    ///
    /// Like [`ListItems::build`], it is called again for every mounted row
    /// each time the table describes its rows, on a source replacement and
    /// when a scroll or resize changes which rows are mounted, and it gets no
    /// state owner. A display control in a cell therefore takes the record's
    /// value itself, such as
    /// [`ProgressBar::new(record.status)`](crate::ProgressBar::new); a
    /// checkbox, toggle or editable label takes it through its controlled
    /// form, such as [`Checkbox::controlled`](crate::Checkbox::controlled);
    /// another editing control binds a signal per row or from a retained cell
    /// component, with the costs `build` describes. Text in an [editable](TableColumn::editable)
    /// column is edited by the table's own editor instead.
    fn cell(&self, index: usize, column: &str) -> Option<Element>;
    /// Position of the row with `key`. The default scans every position.
    fn index_of(&self, key: &str) -> Option<usize> {
        (0..self.len()).find(|&index| self.key(index).as_deref() == Some(key))
    }
    /// Searchable text of row `index` in the column with key `column`, or
    /// `None` for a cell that offers none (the default).
    ///
    /// The table joins the text of a row's cells, in column order and
    /// separated by a space, skipping cells with no or empty text, into the row's
    /// [`ListItems::text`], which content search reads for rows that are
    /// not mounted. The rules of `ListItems::text` apply: deterministic, free
    /// of side effects and I/O, and answered from data the source holds.
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let _ = (index, column);
        None
    }
    /// Whether the cell of row `index` in the [editable](TableColumn::editable)
    /// column with key `column` can be edited now; true by default. The table
    /// asks when an edit would begin and when Tab looks for the next cell to
    /// edit, never during an open edit, so answering false later does not end
    /// one. Answer from data the source holds, without side effects, as for
    /// [`Self::text`].
    fn editable(&self, index: usize, column: &str) -> bool {
        let _ = (index, column);
        true
    }
}

/// A shared, replaceable set of table rows, equal only to itself.
#[derive(Clone)]
pub struct TableSource(Rc<dyn TableItems>);
impl TableSource {
    /// Wraps a row set.
    pub fn new(items: impl TableItems + 'static) -> Self {
        Self(Rc::new(items))
    }
}
impl core::fmt::Debug for TableSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TableSource")
            .field("len", &self.0.len())
            .finish()
    }
}

/// What the header, the rows and the state share: the columns and where the last
/// arrangement put them. It holds no reference back to the state.
struct Shared {
    columns: RefCell<Vec<TableColumn>>,
    /// `(x, width)` per column from the table's last arrangement.
    resolved: RefCell<Vec<(f32, f32)>>,
    /// Bumped when a column or the horizontal offset changes, so mounted rows
    /// arrange their cells again although their own allocation did not change.
    revision: Signal<u64>,
    /// How far the columns are scrolled to the left, shared by header and rows.
    offset: Cell<f32>,
    /// The width of all columns and of the table, from the last arrangement.
    total: Cell<f32>,
    width: Cell<f32>,
    /// The column of the active cell; the active row is the selection's.
    active_column: Signal<Option<String>>,
    selection: ListSelection,
    /// The column keys in the order the application gave them, which a
    /// captured workspace records a move against.
    described: Vec<String>,
    /// Each column's width as the application gave it, which restoring a
    /// workspace entry returns a column to when the entry lists no width.
    described_widths: Vec<(String, ColumnWidth)>,
    /// The keys of the columns the user sized: by dragging an edge, or by a
    /// restored workspace entry that gave them a width. Only these widths are
    /// captured, so a width the application described with
    /// [`TableColumn::width`] stays the application's to change in a later
    /// version rather than being frozen into a saved workspace.
    user_sized: RefCell<BTreeSet<String>>,
    /// Whether a header edge is being dragged, so a controller width write
    /// cannot take the column from under the gesture.
    resizing: Cell<bool>,
    /// Bumped by controller writes that change the columns' extents, which the
    /// table answers by arranging again; the rows follow `revision`.
    relayout: Signal<u64>,
    /// The open cell edit and what the table needs to run it.
    editing: Editing,
}
impl Shared {
    // Fixed columns take their width, the rest share what is left by weight, and
    // nothing goes below its minimum; an overfull table overflows to the right.
    // Starts and the total add up in f64 and saturate at the largest finite
    // f32, so extreme valid widths keep every offset and scroll range finite.
    // A column that starts at that saturated position ends there too, with a
    // width of 0: the one case below the minimum, which `MIN_WIDTH` documents.
    fn resolve(&self, width: f32) {
        let columns = self.columns.borrow();
        let fixed: f64 = columns
            .iter()
            .map(|column| match column.width {
                ColumnWidth::Fixed(width) => f64::from(width),
                ColumnWidth::Flex(_) => 0.0,
            })
            .sum();
        let weights: f64 = columns
            .iter()
            .map(|column| match column.width {
                ColumnWidth::Fixed(_) => 0.0,
                ColumnWidth::Flex(weight) => f64::from(weight),
            })
            .sum();
        let spare = (f64::from(width) - fixed).max(0.0);
        let largest = f64::from(f32::MAX);
        let mut x = 0.0_f64;
        *self.resolved.borrow_mut() = columns
            .iter()
            .map(|column| {
                let start = x.min(largest) as f32;
                let width = match column.width {
                    ColumnWidth::Fixed(width) => width,
                    ColumnWidth::Flex(weight) => (spare * f64::from(weight) / weights) as f32,
                }
                .max(column.minimum)
                // The column ends at the largest finite position at most.
                .min(f32::MAX - start);
                x += f64::from(width);
                (start, width)
            })
            .collect();
        self.total.set(x.min(largest) as f32);
        self.width.set(width);
        self.offset
            .set(self.offset.get().clamp(0.0, self.maximum_offset()));
    }
    fn maximum_offset(&self) -> f32 {
        (self.total.get() - self.width.get()).max(0.0)
    }
    // Scrolls the columns; returns whether the offset changed.
    fn scroll_to(&self, offset: f32) -> Result<bool, RuntimeError> {
        let next = offset.clamp(0.0, self.maximum_offset());
        if self.offset.replace(next) == next {
            return Ok(false);
        }
        let revision = self.revision.get()?;
        self.revision.set(revision.wrapping_add(1))?;
        Ok(true)
    }
    // The least scroll that brings a column fully into view.
    fn reveal_column(&self, column: usize) -> Result<bool, RuntimeError> {
        let Some((start, width)) = self.resolved.borrow().get(column).copied() else {
            return Ok(false);
        };
        let (offset, view) = (self.offset.get(), self.width.get());
        let next = if start < offset || width > view {
            start
        } else if start + width > offset + view {
            start + width - view
        } else {
            offset
        };
        self.scroll_to(next)
    }
    fn column_index(&self, key: &str) -> Option<usize> {
        self.columns
            .borrow()
            .iter()
            .position(|column| column.key == key)
    }
    // The first and last index of the movable columns adjacent to `column`,
    // which are the places it can move among; `None` for a column that is not
    // movable.
    fn movable_run(&self, column: usize) -> Option<(usize, usize)> {
        let columns = self.columns.borrow();
        if !columns.get(column)?.movable {
            return None;
        }
        let first = columns[..column]
            .iter()
            .rposition(|other| !other.movable)
            .map_or(0, |fixed| fixed + 1);
        let last = columns[column..]
            .iter()
            .position(|other| !other.movable)
            .map_or(columns.len() - 1, |fixed| column + fixed - 1);
        Some((first, last))
    }
    // The column boundary within `column`'s run nearest a content position:
    // `n` is before column `n`, one past the run is after its last column.
    fn boundary_near(&self, column: usize, x: f32) -> Option<usize> {
        let (first, last) = self.movable_run(column)?;
        let resolved = self.resolved.borrow();
        let edge = |boundary: usize| match resolved.get(boundary) {
            Some((start, _)) if boundary <= last => Some(*start),
            _ => resolved.get(last).map(|(start, width)| start + width),
        };
        (first..=last + 1)
            .filter_map(|boundary| Some((boundary, (edge(boundary)? - x).abs())))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(boundary, _)| boundary)
    }
    // Moves a column to a boundary of its run and lays the columns out again at
    // the last width. Returns whether the order changed.
    fn move_to(&self, column: usize, boundary: usize) -> Result<bool, RuntimeError> {
        let Some((first, last)) = self.movable_run(column) else {
            return Ok(false);
        };
        if !(first..=last + 1).contains(&boundary) || boundary == column || boundary == column + 1 {
            return Ok(false);
        }
        {
            let mut columns = self.columns.borrow_mut();
            let moved = columns.remove(column);
            columns.insert(boundary - usize::from(column < boundary), moved);
        }
        self.resolve(self.width.get());
        let revision = self.revision.get()?;
        self.revision.set(revision.wrapping_add(1))?;
        Ok(true)
    }
}

struct Rows {
    source: TableSource,
    shared: Rc<Shared>,
}
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.source.0.len()
    }
    fn text(&self, index: usize) -> Option<String> {
        let texts: Vec<String> = self
            .shared
            .columns
            .borrow()
            .iter()
            .filter_map(|column| self.source.0.text(index, &column.key))
            .filter(|text| !text.is_empty())
            .collect();
        (!texts.is_empty()).then(|| texts.join(" "))
    }
    fn key(&self, index: usize) -> Option<String> {
        self.source.0.key(index)
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.source.0.key(index)?;
        // Cells follow the current column order, which a move changes.
        let cells = self
            .shared
            .columns
            .borrow()
            .iter()
            .map(|column| {
                // The edited cell holds the editor's host instead. Content
                // keyed like the host sits in a wrapper of the table's, so
                // the two never meet under one key.
                let host = Editing::host(&self.shared, &key, &column.key);
                let wrapped = host.is_some();
                let mut content_wrapped = false;
                let content = host.or_else(|| {
                    self.source.0.cell(index, &column.key).map(|content| {
                        if content.key() == editing::HOST_KEY {
                            content_wrapped = true;
                            Element::new(CONTENT_KEY, CellContent(Some(content))).structural()
                        } else {
                            content
                        }
                    })
                });
                Element::new(
                    column.key.clone(),
                    TableCell {
                        content,
                        wrapped: wrapped || content_wrapped,
                        column: column.key.clone(),
                        shared: self.shared.clone(),
                    },
                )
            })
            .collect();
        Some(Element::new(
            key,
            TableRow {
                cells,
                key: self.source.0.key(index)?,
                shared: self.shared.clone(),
                active: None,
                clicks: crate::ClickCounter::new(),
                clicked: None,
            },
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.source.0.index_of(key)
    }
}

struct StateInner {
    shared: Rc<Shared>,
    /// The application's rows, kept to rebuild them after a column move.
    source: RefCell<TableSource>,
    items: Signal<ListSource>,
    sort: Signal<Option<TableSort>>,
    selection: ListSelection,
    /// Serves [`TableController::reveal`] through the body's list.
    reveal: ListController,
    /// The query and matcher of [`TableController::filter`], applied again
    /// whenever the rows are published.
    filter: RefCell<(String, crate::FilterMatcher)>,
    counts: Signal<crate::FilterCounts>,
    /// Source replacements: a revision and, when the replacement hid the
    /// active row, its key and the nearest remaining row, which a filter
    /// wrapper applies.
    replaced: Signal<crate::collection_filter::Replacement>,
}

/// Application-owned controller of a [`Table`]: its columns, rows, sort indication and
/// [`ListSelection`].
///
/// Rows are replaced with [`TableController::set_source`], which is how sorted or
/// filtered data reaches the table; rows that keep their key keep their
/// components. The sort is an indication the header draws. The table never
/// reorders rows: the application sorts its own data in
/// [`Table::on_sort`] and publishes the result. Changes come from application or
/// event code, never from a component's `update`. Clones share the same state.
#[derive(Clone)]
#[must_use]
pub struct TableController(Rc<StateInner>);

/// Shows the selection's mode, not the rows, sort or filter: formatting must
/// not read a signal inside a runtime phase.
impl std::fmt::Debug for TableController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableController")
            .field("selection", &self.0.selection)
            .finish_non_exhaustive()
    }
}
impl TableController {
    /// Creates a controller with no sort indication.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns `EmptyKey` for a column without a key, `DuplicateKey` for two
    /// columns sharing one, and [`LayoutError::NoColumns`] for no columns at all.
    pub fn new(
        ui: &impl crate::StateOwner,
        selection: ListSelection,
        columns: impl IntoIterator<Item = TableColumn>,
        source: TableSource,
    ) -> Result<Self, RuntimeError> {
        let columns: Vec<TableColumn> = columns.into_iter().collect();
        if columns.is_empty() {
            return Err(LayoutError::NoColumns.into());
        }
        for (index, column) in columns.iter().enumerate() {
            if column.key.is_empty() {
                return Err(RuntimeError::EmptyKey);
            }
            if columns[..index].iter().any(|other| other.key == column.key) {
                return Err(RuntimeError::DuplicateKey(column.key.clone()));
            }
        }
        let described = columns.iter().map(|column| column.key.clone()).collect();
        let described_widths = columns
            .iter()
            .map(|column| (column.key.clone(), column.width))
            .collect();
        let shared = Rc::new(Shared {
            columns: RefCell::new(columns),
            resolved: RefCell::new(Vec::new()),
            revision: ui.signal(0),
            offset: Cell::new(0.0),
            total: Cell::new(0.0),
            width: Cell::new(0.0),
            active_column: ui.signal(None),
            selection: selection.clone(),
            described,
            described_widths,
            user_sized: RefCell::new(BTreeSet::new()),
            resizing: Cell::new(false),
            relayout: ui.signal(0),
            editing: Editing::new(ui),
        });
        let total = source.0.len();
        Ok(Self(Rc::new(StateInner {
            items: ui.signal(ListSource::new(Rows {
                source: source.clone(),
                shared: shared.clone(),
            })),
            filter: RefCell::new((String::new(), crate::FilterMatcher::default())),
            counts: ui.signal(crate::FilterCounts {
                shown: total,
                total,
            }),
            replaced: ui.signal((0, None)),
            source: RefCell::new(source),
            shared,
            sort: ui.signal(None),
            selection,
            reveal: ListController::new(ui),
        })))
    }
    /// The selection the table's rows navigate and draw.
    pub fn selection(&self) -> ListSelection {
        self.0.selection.clone()
    }
    /// Replaces the rows, keeping selection by key.
    ///
    /// May be called from an input callback, including a button in this table's
    /// own cell. Rows adopt the source during the next update; surviving keys
    /// keep their row identities. A cell that reads its own signal can follow
    /// changing data without replacing the source. Replace it when membership,
    /// order or fixed cell descriptions change.
    ///
    /// # Errors
    /// The errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn set_source(&self, source: TableSource) -> Result<(), RuntimeError> {
        let before = self.0.items.get()?;
        self.publish(source.clone())?;
        // Kept only once published, so a rejected write is not applied later
        // by a column move that publishes the rows again.
        *self.0.source.borrow_mut() = source;
        let after = self.0.items.get()?;
        let active = self.0.selection.active().get()?;
        let hidden = crate::collection_filter::hid(&*before, &*after, active);
        let revision = self.0.replaced.get()?.0.wrapping_add(1);
        self.0.replaced.set((revision, hidden))
    }
    /// Source replacements with the active key they hid and its nearest
    /// remaining row, for [`CollectionFilter`](crate::CollectionFilter).
    pub(crate) fn replaced(&self) -> ReadSignal<crate::collection_filter::Replacement> {
        self.0.replaced.read_only()
    }
    /// Whether the table shows the row `key` now, for
    /// [`CollectionFilter`](crate::CollectionFilter).
    pub(crate) fn shows(&self, key: &str) -> Result<bool, RuntimeError> {
        Ok(self.0.items.get()?.index_of(key).is_some())
    }
    // Publishes the rows again, so mounted rows rebuild their cells in the
    // current column order.
    fn republish(&self) -> Result<(), RuntimeError> {
        #[cfg(test)]
        if tests::FAIL_REPUBLISH.with(|fail| fail.replace(false)) {
            return Err(RuntimeError::BorrowConflict);
        }
        let source = self.0.source.borrow().clone();
        self.publish(source)
    }
    fn publish(&self, source: TableSource) -> Result<(), RuntimeError> {
        let (query, matcher) = self.0.filter.borrow().clone();
        self.publish_filtered(source, &query, &matcher).map(|_| ())
    }
    // Publishes the rows of `source` that match `query`, and their counts.
    fn publish_filtered(
        &self,
        source: TableSource,
        query: &str,
        matcher: &crate::FilterMatcher,
    ) -> Result<crate::FilterCounts, RuntimeError> {
        let rows = ListSource::new(Rows {
            source,
            shared: self.0.shared.clone(),
        });
        let (rows, counts) = crate::collection_filter::restrict(
            &rows,
            crate::collection_filter::effective(query),
            matcher,
        );
        self.0.items.set(rows)?;
        self.0.counts.set(counts)?;
        Ok(counts)
    }
    /// Shows only the rows whose text matches `query` under `matcher`, and
    /// keeps applying it when the rows are replaced or a column moves; an empty
    /// or whitespace query shows every row. Any other query is trimmed of
    /// leading and trailing whitespace before `matcher` sees it, as for
    /// [`ListFilter::filter`](crate::ListFilter::filter), so code that repeats
    /// the match passes the trimmed query.
    ///
    /// A row's text is its cells' [`TableItems::text`] joined in column order
    /// with a space between them,
    /// so a source that offers no text has no row matching a query. Rows that
    /// stay keep their keys and components; the selection is never pruned, so
    /// a selected row filtered out stays selected and hidden. The returned
    /// outcome names the nearest remaining row when the active row disappeared
    /// ([`FilterOutcome::nearest_active`](crate::FilterOutcome)); this method
    /// does not move it. A [`CollectionFilter`](crate::CollectionFilter) calls
    /// it as its field changes. Each query reads every row's text once.
    ///
    /// # Errors
    /// Same as [`TableController::set_source`].
    pub fn filter(
        &self,
        query: &str,
        matcher: &crate::FilterMatcher,
    ) -> Result<crate::FilterOutcome, RuntimeError> {
        let before = self.0.items.get()?;
        let source = self.0.source.borrow().clone();
        let counts = self.publish_filtered(source, query, matcher)?;
        *self.0.filter.borrow_mut() = (query.to_owned(), matcher.clone());
        let after = self.0.items.get()?;
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
    /// The counts of the current filter, or the full row count without one.
    #[must_use]
    pub fn filter_counts(&self) -> ReadSignal<crate::FilterCounts> {
        self.0.counts.read_only()
    }
    /// The sort indication the header draws, if any.
    #[must_use]
    pub fn sort(&self) -> ReadSignal<Option<TableSort>> {
        self.0.sort.read_only()
    }
    /// Sets the sort indication. This draws an arrow; it sorts nothing.
    ///
    /// # Errors
    /// Same as [`TableController::set_source`].
    pub fn set_sort(&self, sort: Option<TableSort>) -> Result<(), RuntimeError> {
        self.0.sort.set(sort)
    }
    /// The column of the active cell, if any. The active row is the
    /// [`ListSelection::active`] item, so the two together name the cell.
    #[must_use]
    pub fn active_column(&self) -> ReadSignal<Option<String>> {
        self.0.shared.active_column.read_only()
    }
    /// Moves the active cell to a column, or to none. The table scrolls the
    /// column into view the next time the user moves it; this call does not.
    ///
    /// # Errors
    /// Returns [`LayoutError::UnknownKey`] for a key no column has, and the
    /// errors of [`Signal::set`](crate::Signal::set): `Closed` after the
    /// runtime shut down; `WriteDuringUpdate`, `WriteDuringLayout`,
    /// `WriteDuringPaint`, `WriteDuringInspection` or `WriteDuringDelivery`
    /// from inside that phase; or `BorrowConflict`.
    pub fn set_active_column(&self, column: Option<String>) -> Result<(), RuntimeError> {
        if let Some(key) = &column
            && self.0.shared.column_index(key).is_none()
        {
            return Err(LayoutError::UnknownKey(key.clone()).into());
        }
        self.0.shared.active_column.set(column)
    }
    /// Fixes a column's width from application code, as dragging its edge
    /// does: the column becomes fixed at `width`, counts as sized by the
    /// user, so a captured workspace keeps the width, and the mounted table
    /// and its rows arrange again at the next flush. A width below
    /// [`TableColumn::MIN_WIDTH`] is raised to that layout minimum rather
    /// than rejected, as a drag stops there. The column need not be
    /// [`TableColumn::resizable`], which only governs the header edge. A
    /// restored workspace entry replaces the width like any other.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Length, ListSelection, Runtime, SelectionMode,
    ///     Table, TableColumn, TableItems, TableSource, TableController, Viewport};
    /// struct Empty;
    /// impl TableItems for Empty {
    ///     fn len(&self) -> usize { 0 }
    ///     fn key(&self, _: usize) -> Option<String> { None }
    ///     fn cell(&self, _: usize, _: &str) -> Option<Element> { None }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
    /// let state = TableController::new(
    ///     &ui,
    ///     ListSelection::new(&ui, SelectionMode::Single),
    ///     [TableColumn::new("name", "Name"), TableColumn::new("size", "Size")],
    ///     TableSource::new(Empty),
    /// )?;
    /// ui.mount_tree(Element::new("table", Table::new(&state, 24.0)?))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// state.set_column_width("size", Length::new(80.0)?)?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// assert_eq!(state.columns()[1].fixed_width(), Some(80.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`LayoutError::UnknownKey`] for a key no column has,
    /// [`RuntimeError::GestureInProgress`] while the user is dragging a column
    /// edge, since the drag owns the widths until it ends, and the signal
    /// write's failures, such as `WriteDuringUpdate`, with the columns
    /// unchanged in every case.
    pub fn set_column_width(&self, key: &str, width: Length) -> Result<(), RuntimeError> {
        let shared = &self.0.shared;
        let index = shared
            .column_index(key)
            .ok_or_else(|| LayoutError::UnknownKey(key.to_owned()))?;
        if shared.resizing.get() {
            return Err(RuntimeError::GestureInProgress);
        }
        shared.revision.shared().writable()?;
        let revision = shared.revision.get()?;
        let relayout = shared.relayout.get()?;
        {
            let mut columns = shared.columns.borrow_mut();
            let column = &mut columns[index];
            column.width = ColumnWidth::Fixed(width.get().max(column.minimum));
            shared.user_sized.borrow_mut().insert(column.key.clone());
        }
        shared.revision.set(revision.wrapping_add(1))?;
        shared.relayout.set(relayout.wrapping_add(1))?;
        Ok(())
    }
    /// The columns as they are now, in the order the user arranged them and
    /// with their current widths: a column the user resized, directly or
    /// through a restored workspace, is fixed at that width; every other
    /// column has the width the application described.
    #[must_use]
    pub fn columns(&self) -> Vec<TableColumn> {
        self.0.shared.columns.borrow().clone()
    }
    /// Asks the table to scroll vertically to and mount the row with `key`.
    ///
    /// A request, served like [`ListController::reveal`](crate::ListController::reveal):
    /// the table resolves the key through [`TableItems::index_of`] at its next
    /// update, scrolls its rows only as far as needed to show that row below
    /// the header, and mounts it. The returned
    /// [`ListRevealReceipt`](crate::ListRevealReceipt) reports the row's
    /// `WidgetId` once it is mounted, and
    /// [`ListRevealStatus::NotFound`](crate::ListRevealStatus::NotFound) for an
    /// unknown key. The horizontal
    /// offset, the selection and the active cell are left as they are. A table
    /// serves only requests made after it was created over this controller, so
    /// a table rebuilt over a long-lived controller opens at its start rather
    /// than at an old target.
    ///
    /// This reveal controller supports one mounted table at a time, including
    /// a hidden table. Reuse it only after that owner is removed; concurrent
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
}

/// The key of the wrapper around application content keyed like the
/// editor's host.
const CONTENT_KEY: &str = "content";

/// A cell's application content keyed like the editor's host, under a key
/// of the table's own, so the content's key stays free.
struct CellContent(Option<Element>);
impl Component for CellContent {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, space)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.refresh_forwarded(0)?;
        cx.place(0, Rect::from_size(size))
    }
    fn inspect(&self) -> ComponentDetails {
        // The innermost table component around the content gives the row's
        // fixed height, so development validation names the content once.
        fixed_height()
    }
}

/// Inspection details with only [`ValidationHints::fixed_height`](crate::ValidationHints::fixed_height):
/// a cell's content must fit the row's height.
pub(super) fn fixed_height() -> ComponentDetails {
    let mut details = ComponentDetails::default();
    details.validation.fixed_height = true;
    details
}

/// One cell: the application's content inset by ControlPaddingX and clipped to
/// its column, centered vertically in the row.
struct TableCell {
    content: Option<Element>,
    /// Whether the content is wrapped by the table, in the editor's host or
    /// in [`CellContent`], which then reports the row's fixed height in this
    /// cell's place.
    wrapped: bool,
    /// The column's key, whose current title names the cell.
    column: String,
    shared: Rc<Shared>,
}
impl Component for TableCell {
    fn take_children(&mut self) -> Vec<Element> {
        self.content.take().into_iter().collect()
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // An edit beginning or ending changes the cell's child; the update
        // this schedules makes development validation read the hint again.
        (std::mem::replace(&mut self.wrapped, next.wrapped) != next.wrapped)
            .then_some(Invalidation::Paint)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let padding = 2.0 * cx.metric(MetricRole::ControlPaddingX);
        // An empty cell is its padding alone.
        if cx.is_empty() {
            return Ok(Size::new(padding, 0.0)?);
        }
        let content = cx.measure(
            0,
            Constraints::new(
                available.width().map(|width| (width - padding).max(0.0)),
                available.height(),
            )?,
        )?;
        Ok(Size::new(content.width() + padding, content.height())?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            return Ok(());
        }
        let padding = cx
            .measurement()
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let width = size.width() - 2.0 * padding;
        let content = cx.measure(0, Constraints::new(Some(width), Some(size.height()))?)?;
        let height = content.height().min(size.height());
        cx.place(
            0,
            Rect::new(padding, (size.height() - height) / 2.0, width, height)?,
        )
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        // With the body as the one Tab stop, Escape that resting content F2
        // entered leaves unhandled returns to the body.
        let editing = &self.shared.editing;
        if !editing.cell_stops.get()
            && keys.control_key(event) == KeyCode::Escape
            && event.transition == KeyTransition::Pressed
            && keys.chord_modifiers(event) == crate::Modifiers::default()
            && let Some(body) = &*editing.body.borrow()
        {
            cx.request_focus_for(body)?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        // A cell is named by its column, as a reader names it; what it holds is
        // its content's to describe. The title is looked up, so a bound title
        // renames mounted cells too.
        let columns = self.shared.columns.borrow();
        let title = columns
            .iter()
            .find(|column| column.key == self.column)
            .map_or("", |column| column.title.as_str());
        let mut details = ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Cell).name(title)),
            ..ComponentDetails::default()
        };
        details.validation.fixed_height = !self.wrapped;
        details
    }
}

/// One row: its cells placed at the table's resolved columns.
struct TableRow {
    cells: Vec<Element>,
    key: String,
    shared: Rc<Shared>,
    /// The active cell's column index while this is the active row.
    active: Option<usize>,
    /// Primary presses on this row's cells, for a double-click that begins
    /// an edit, and the column of the last one.
    clicks: crate::ClickCounter,
    clicked: Option<String>,
}
impl Component for TableRow {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.cells)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // A column change leaves the row's own allocation alone, so this read is
        // what makes the row arrange its cells again.
        cx.read(&self.shared.revision)?;
        let row = cx.read_paint(&self.shared.selection.active())?;
        let column = cx.read_paint(&self.shared.active_column)?;
        let is_active = row.as_deref() == Some(self.key.as_str());
        self.active = is_active
            .then_some(column)
            .flatten()
            .and_then(|key| self.shared.column_index(&key));
        if is_active {
            // F2 moves focus into the active cell's resting content.
            *self.shared.editing.active_cells.borrow_mut() = cx
                .children()
                .iter()
                .map(|child| (child.key().to_owned(), child.id().clone()))
                .collect();
        }
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(0.0),
            available.height().unwrap_or(0.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let resolved = self.shared.resolved.borrow();
        let offset = self.shared.offset.get();
        for child in 0..cx.len() {
            // By key rather than position: the cells are in the order the row
            // was built in, which a column move changes before the row is
            // built again.
            let column = self.shared.column_index(cx.measurement().key(child)?);
            let (x, width) = column
                .and_then(|column| resolved.get(column).copied())
                .unwrap_or((0.0, 0.0));
            let cell = Rect::new(x - offset, 0.0, width, size.height())?;
            cx.place_clipped(child, cell, cell)?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        // A press names the cell's column and is then left unhandled, so the list
        // still makes this row active and selects it. The second press of an
        // unmodified double-click on one cell also asks the table to edit it.
        if event.kind() == PointerKind::Press(PointerButton::Primary)
            && let Some(point) = cx.to_local(event.position())
        {
            let x = point.x() + self.shared.offset.get();
            let column = self
                .shared
                .resolved
                .borrow()
                .iter()
                .position(|(start, width)| x >= *start && x < start + width);
            if let Some(column) = column {
                let key = self.shared.columns.borrow()[column].key.clone();
                self.shared.active_column.set(Some(key.clone()))?;
                let unmodified = cx.modifiers() == crate::Modifiers::default();
                if !unmodified || self.clicked.as_deref() != Some(key.as_str()) {
                    self.clicks.reset();
                }
                let count = self.clicks.press_event(cx, event);
                self.clicked = unmodified.then_some(key.clone());
                if unmodified && count == Some(2) {
                    self.shared.editing.request(
                        self.key.clone(),
                        key,
                        crate::EditorStart::CaretAtEnd,
                    );
                }
            }
        }
        Ok(InputOutcome::Ignored)
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (Some(column), size) = (self.active, painter.size()) else {
            return Ok(());
        };
        let Some((start, width)) = self.shared.resolved.borrow().get(column).copied() else {
            return Ok(());
        };
        let cell = Rect::new(start - self.shared.offset.get(), 0.0, width, size.height())?
            .intersection(Rect::from_size(size));
        if !cell.is_empty() {
            painter.focus_ring(cell, crate::Shape::Rectangle)?;
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

type SortHandler = Box<dyn FnMut(&mut EventContext, &TableSort) -> Result<(), RuntimeError>>;
type CellHandler =
    Box<dyn FnMut(&mut EventContext, &str, Option<&str>) -> Result<(), RuntimeError>>;
type SelectionHandler =
    Box<dyn FnMut(&mut EventContext, &crate::ListSelectionChange) -> Result<(), RuntimeError>>;

/// A header gesture in progress.
enum HeaderGesture {
    /// Pressed on a sortable or movable title, at this window position: sorts
    /// on release over the same column, and a movable one becomes a drag once
    /// it has moved [`DRAG_THRESHOLD`].
    ///
    /// Columns are named by key: a keyboard move during the press changes
    /// every index after it.
    Title(String, Point),
    /// Dragging a column's trailing edge: the column, its width at the press and
    /// where the press was. The width follows the pointer's movement since the
    /// press, not an absolute position: narrowing the last column of a table
    /// scrolled to its far end shortens the content and clamps the offset, which
    /// would otherwise move the column under the pointer and collapse it.
    Resize(String, f32, f32),
    /// Dragging the horizontal scrollbar: where on the thumb it was grabbed.
    Scroll(f32),
}

/// Rows of cells under a header that does not scroll.
///
/// The header is ControlHeight tall and drawn by the table: each column has a
/// title and separator, with an arrow beside the sorted title. A title is
/// clipped before the reserved arrow, even at the column's minimum width. The
/// sortable or movable title uses SurfaceHover under the pointer; a resizable
/// edge uses ResizeHandleHover. Neither hover state changes column widths. The
/// body below it is a [`VirtualList`], so only rows near the viewport are mounted
/// and scrolling, reveal and pinned rows are the list's. Navigation and selection
/// over rows are the shared [`ListSelection`] contract, and the body is the
/// table's one Tab stop. Columns are not virtualized. When they need more than
/// the table's width, header and rows scroll together by one horizontal offset:
/// by a horizontal wheel, by an overlay scrollbar along the bottom edge that
/// follows the theme's scrollbar metrics, and by moving the active cell, which
/// scrolls its column into view.
///
/// The title of a [`TableColumn::movable`] column can be dragged: once the press
/// has moved [`DRAG_THRESHOLD`](crate::DRAG_THRESHOLD) it becomes a
/// [drag session](EventContext::start_drag) with the title as its preview, and
/// while it is over the table a Primary line across header and rows marks the
/// column boundary nearest the pointer where it would go. A column moves only
/// among the movable columns next to it. Resting within 24 logical pixels of the
/// left or right edge of a table whose columns overflow scrolls them. A release
/// over the table moves the column; anywhere else, or Escape, leaves it where it
/// was. Alt with Left or Right moves the active cell's column one place. The
/// table owns the order, as it owns dragged widths: cells follow their column,
/// [`TableController::columns`] reports the order and a captured workspace keeps it.
/// Restoration obeys the current column capabilities: an immovable column stays
/// in its slot, and movable columns remain inside their contiguous movable run.
/// The saved positions must be `0..n` for the `n` columns the entry lists, as
/// capture writes them. Other positions, an incompatible order, or a saved
/// width that is not finite, is below [`TableColumn::MIN_WIDTH`] or is beyond
/// the largest `f32`, which capture never writes, report
/// [`crate::RestoreRejection::OutOfRange`] with the column's key and reject the
/// entire table entry without changing widths or order. A saved key
/// missing from the current table reports [`crate::RestoreRejection::Unresolved`].
/// Columns the entry does not list, such as ones added since, keep their slots.
/// Restoration requires a writable runtime. A closed runtime or forbidden write
/// phase returns [`crate::RestoreRejection::Runtime`] without changing the
/// column widths or order, including when calling the component hook directly.
///
/// A press and release on a [`TableColumn::sortable`] title sets the sort
/// indication, ascending first and flipping on the same column, and runs
/// [`Table::on_sort`]; the application sorts its own data there and publishes it
/// with [`TableController::set_source`]. From the keyboard, Alt with Up or Down
/// asks for the active cell's column ascending or descending through the same
/// indication and callback, so a sort needs no pointer; the chord is left to
/// command bindings while no column is active or the active one is not
/// sortable. Dragging the trailing edge of a
/// [`TableColumn::resizable`] column resizes it, not below its minimum, and fixes
/// its width; the grip is the theme's ResizeWidth around the edge; Escape or
/// losing the pointer ends the drag where it is.
/// [`TableController::set_column_width`] fixes a width from application code
/// the same way, except during such a drag.
///
/// A header press that starts a gesture requests the keyboard focus for the
/// body, as a press on a cell does, wherever the focus was before. The request
/// is applied at the next flush, which a native host runs before it delivers a
/// key, so Escape reaches the header gesture. It ends a column resize at its
/// current width, ends a scrollbar thumb drag with the thumb emphasized only if
/// the pointer is still over the bar, and cancels a title press, which then
/// neither sorts nor becomes a drag.
///
/// The active cell is the [`ListSelection`]'s active row and the
/// [`TableController::active_column`]. Unmodified Left and Right move it between
/// columns, from no column to the first or the last, and stop at the ends (other
/// modified chords than Alt's column move are left to command bindings); a press on a cell
/// sets its column as well as its row. The cell gets the
/// [`ColorRole::FocusRing`] ring in place of the list's row ring, shown whenever a column is
/// active. Enter runs [`Table::on_activate`] with the row and the column,
/// except on a cell the table edits in place ([cell editing](#cell-editing)).
/// Cells themselves are not Tab stops; interactive content inside a cell keeps
/// its own input, including its own Tab stop unless
/// [`Table::cell_tab_stops`] turns those off.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Label, ListSelection, Runtime, SelectionMode, Table,
///     TableColumn, TableItems, TableSource, TableController, Viewport};
/// struct Squares(usize);
/// impl TableItems for Squares {
///     fn len(&self) -> usize { self.0 }
///     fn key(&self, index: usize) -> Option<String> {
///         (index < self.0).then(|| format!("n{index}"))
///     }
///     fn cell(&self, index: usize, column: &str) -> Option<Element> {
///         let value = if column == "n" { index } else { index * index };
///         Some(Element::new("value", Label::new(value.to_string())))
///     }
/// }
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
/// let state = TableController::new(
///     &ui,
///     ListSelection::new(&ui, SelectionMode::Single),
///     [TableColumn::new("n", "n"), TableColumn::new("square", "n²").sortable(true)],
///     TableSource::new(Squares(100_000)),
/// )?;
/// let table = ui.mount_tree(Element::new("squares", Table::new(&state, 24.0)?))?;
/// ui.flush(std::time::Duration::ZERO)?;
/// let body = ui.children(&table)?[0].clone();
/// assert!(ui.children(&body)?.len() < 40, "rows are virtualized");
/// // Row "n3", column "square", the content keyed "value".
/// let nine = ui.find(&table, &["n3", "square", "value"])?.ok_or("not mounted")?;
/// assert_eq!(ui.key(&nine)?, "value");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Describing a mounted table again keeps its controller, row height and
/// [`cell_tab_stops`](Self::cell_tab_stops): the table's data, columns and
/// state belong to its controller, so change them through it, or give the
/// table a new key to replace them. Its callbacks and permissions follow the
/// new description, including their removal: [`on_sort`](Self::on_sort),
/// [`on_activate`](Self::on_activate),
/// [`on_selection_change`](Self::on_selection_change),
/// [`cell_editor`](Self::cell_editor),
/// [`resize_permission`](Self::resize_permission) and
/// [`reorder_permission`](Self::reorder_permission). A fixed denial in the new
/// description cancels a resize or column move at the update the
/// re-description schedules, as revoking the permission does.
///
/// Like a [`VirtualList`], the table mounts the rows inside the height it is
/// allocated and needs a bounded one; see [the list's bounded
/// extent](VirtualList#a-bounded-extent) for the containers that give one. It
/// builds every column of each mounted row, including columns scrolled out of
/// view, and an active [filter](TableController::filter) reads every row's text
/// again whenever the source is replaced.
///
/// The row height is fixed: a cell gives its content at most that height,
/// centered, and cuts off the rest. Choose it for the tallest content, as
/// [`Table::new`] describes. Development validation reports content that
/// measured or is laid out taller than its row as
/// [`ValidationCode::ExceedsFixedHeight`](crate::ValidationCode::ExceedsFixedHeight),
/// once per mounted cell, naming the content and the component that gives it
/// the row's height: the cell, or, for content keyed like the editor's host
/// and for an open edit's editor, the table's wrapper around it, keyed
/// `content` or `cell editor`.
///
/// # Addresses and tests
///
/// The body is internal structure, and so is the component that reports edit
/// ends, which has no children: addresses skip both. A row is addressed by its
/// key below the table, a cell by its column's key below the row, and the
/// content [`TableItems::cell`] returned by its own key below the cell:
/// `<table>/<row key>/<column key>/<content key>`, as
/// `ui.find(&table, &["n3", "square", "value"])` in the first example. The
/// wrappers a cell may hold around its content or an editor add no segment,
/// so an edited cell's editor is at `<table>/<row key>/<column key>/<editor
/// key>`. Content may be keyed like its column, since keys are unique only
/// among siblings; [`Runtime::find_descendant`](crate::Runtime::find_descendant)
/// below the row then sees the cell and its content and fails with
/// [`RuntimeError::AmbiguousKey`], while the full address resolves the
/// content.
///
/// [`Driver::address_children(&table)`](crate::testing::Driver::address_children)
/// lists the mounted rows in source order with their keys, including the
/// offscreen ones the body retains; [`TableController::reveal`] mounts
/// another. [`Runtime::children`](crate::Runtime::children) of the table
/// lists the body and the reporter instead.
///
/// The column titles are the table's [semantic items](crate::Semantics::items),
/// in the current column order, each named by its title, selected while the
/// table is sorted by it and offering the actions its column allows. Each
/// title's [`bounds`](crate::SemanticItem::bounds) are its visible rectangle
/// in the header, in window coordinates, and absent for a title scrolled
/// entirely out of view.
/// [`Driver::click_item(&table, index)`](crate::testing::Driver::click_item)
/// presses and releases at the middle of that rectangle as a pointer would,
/// so it sorts a sortable column; a middle within the resize grip of a
/// column's trailing edge starts a resize instead. From the keyboard, Alt
/// with Up or Down sorts the active column.
///
/// # Cell editing
///
/// With a [`Table::cell_editor`], cells of [editable](TableColumn::editable)
/// columns that [`TableItems::editable`] allows are edited in place. The table
/// owns the open edit: it puts a host in the edited cell, which builds the
/// editor once with an [`EditorSession`](crate::EditorSession) and focuses
/// it, keeps the edited row mounted for the whole edit (scrolled away or not;
/// it is not kept in view), and follows the session's reports.
///
/// - **Begin.** Enter on the active cell begins with the value selected, F2
///   and a double-click with the caret at the end, and committed text typed
///   while the body is focused with that text in place of the value
///   ([`EditorStart`](crate::EditorStart)); text that is only whitespace
///   does not begin, since Space selects. The body has no text input state,
///   so typing never starts an IME composition there: an IME user begins
///   with Enter or F2. A begin makes the cell active. On the cell whose edit
///   is open it focuses the editor again; on another cell it closes the open
///   edit, discarding a refused draft. Content inside a cell that handles a
///   press, such as a checkbox, takes a double-click first, and a press with
///   a modifier never begins an edit.
/// - **End.** The table acts only on reports, in its own delivery after the
///   editor's save callbacks have run, so a sort or filter they caused
///   applies. Escape, and Enter or a choice that saved or changed nothing,
///   close the edit and return focus to the body on the edited cell. Tab and
///   Shift+Tab that saved or changed nothing move the edit to the next or
///   previous eligible cell, along the row in the current column order and
///   then on the following or preceding rows, with the value selected; past
///   either end of the table the body takes focus on the edited cell. Focus
///   leaving the editor otherwise closes the edit without moving focus, and
///   a refused confirmation keeps it open with its draft. A report delivered
///   after focus left the editor never moves focus. Help, a composition, a
///   selection drag and an open dropdown list take Escape first, as their
///   editors define, and a press that leaves focus in the editor leaves the
///   edit open.
/// - **Rows and lifetime.** Rows are keyed, so sorting and same-key
///   replacement keep the editor and its draft. When the edited row leaves
///   the rows, by deletion or a [filter](TableController::filter), the edit
///   closes with its draft discarded and the body takes focus if the editor
///   had it; the selection is not pruned. Hiding or unmounting the table
///   closes the edit. Escape on the body cancels an open edit whose editor
///   does not hold focus, such as a disabled one, which keeps its draft
///   unsaved until then (the
///   [draft policy](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#draft-policy)).
/// - **Reports.** [`Table::on_edit_end`] reports each edit's end with a
///   [`CellEditEnd`]: the editor's own terminal completion, after the
///   editor's `on_modified` report, and each end the table performs itself
///   (body Escape, another edit beginning, the row leaving, a failed
///   editor callback, hiding), where the removed editor reports nothing.
///   Removing the table reports nothing.
///
/// Tab looks for the next eligible cell row by row, asking
/// [`TableItems::index_of`] and [`TableItems::editable`] for each, so a table
/// with few editable rows pays for the rows it passes.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{KeyAction, Viewport};
/// use std::rc::Rc;
/// struct Items(Rc<Vec<Signal<String>>>);
/// impl TableItems for Items {
///     fn len(&self) -> usize { self.0.len() }
///     fn key(&self, index: usize) -> Option<String> {
///         (index < self.0.len()).then(|| format!("item{index}"))
///     }
///     fn cell(&self, index: usize, _: &str) -> Option<Element> {
///         let name = self.0.get(index)?.get().ok()?;
///         Some(Element::new("name", Label::new(name)))
///     }
/// }
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
/// let names = Rc::new(vec![ui.signal("Cable".to_owned()), ui.signal("Mouse".to_owned())]);
/// let state = TableController::new(
///     &ui,
///     ListSelection::new(&ui, SelectionMode::Single),
///     [TableColumn::new("name", "Name").editable(true)],
///     TableSource::new(Items(names.clone())),
/// )?;
/// let (rows, republish) = (names.clone(), state.clone());
/// // The row holds the editor, a text field of ControlHeight.
/// let theme = ui.theme();
/// let row = theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight);
/// let table = Table::new(&state, row)?.cell_editor(move |edit| {
///     let index: usize = edit.row().trim_start_matches("item").parse().unwrap_or(0);
///     let (name, rows, republish) = (rows[index].clone(), rows.clone(), republish.clone());
///     Ok(Element::new(
///         "field",
///         TextField::new(name)
///             .session(edit.session())
///             // Show the saved name in the plain cell.
///             .on_save(move |_, _| republish.set_source(TableSource::new(Items(rows.clone())))),
///     ))
/// });
/// let root = ui.mount_tree(Element::new("table", table))?;
/// ui.flush(ui.now())?;
/// let body = ui.children(&root)?[0].clone();
/// ui.focus(&body)?;
/// state.selection().set_active(Some("item0".into()))?;
/// state.set_active_column(Some("name".into()))?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// ui.text_input("Laptop")?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// ui.flush(ui.now())?;
/// assert_eq!(names[0].get()?, "Laptop");
/// assert_eq!(ui.focused(), Some(body), "the body has focus again");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Table {
    resize_permission: Permission,
    reorder_permission: Permission,
    resize_allowed: bool,
    reorder_allowed: bool,
    resize_original: Option<(String, ColumnWidth, bool)>,
    /// The body until it becomes the table's child, so builders can still
    /// configure it.
    body: Option<VirtualList>,
    /// The editor callback until the mounted table installs it.
    cell_editor: Option<CellEditorFn>,
    /// The edit-end callback until the mounted table installs it.
    on_edit_end: Option<EditEndFn>,
    /// Whether cell content is a Tab stop, installed by the mounted table.
    cell_stops: bool,
    /// The mounted body, which a header press focuses so Escape reaches the
    /// header gesture; retained from the last update.
    body_id: Option<WidgetId>,
    state: TableController,
    on_sort: Option<SortHandler>,
    sort: Option<TableSort>,
    /// The controller's relayout count from the last update.
    relayout: u64,
    header: std::cell::Cell<f32>,
    titles: RefCell<Vec<Option<TextLayout>>>,
    gesture: Option<HeaderGesture>,
    rows: std::cell::Cell<usize>,
    /// The themed ResizeWidth from the last arrangement; hit testing and the
    /// cursor have no metric access, so it is retained beside the geometry.
    grip: std::cell::Cell<f32>,
    bar_extents: Cell<scrollbar::Extents>,
    size: Cell<Size>,
    row_height: f32,
    activate: Rc<RefCell<Option<CellHandler>>>,
    /// The body list forwards its selection changes here, observing them only
    /// once a callback is installed.
    on_selection: Rc<RefCell<Option<SelectionHandler>>>,
    observe_selection: Rc<crate::virtual_list::SelectionGate>,
    /// The pointer rests over the scrollbar, presented as draggable.
    bar_hover: bool,
    /// Local pointer position; horizontal scrolling can move another header
    /// under a stationary pointer.
    header_hover: Option<Point>,
    /// The key of the column this table's drag session carries.
    dragging: Option<String>,
    /// Local x of the drag while it is over the table. The boundary is derived
    /// from it at paint and drop, so it follows the columns when they scroll
    /// under a still pointer.
    drop_x: Option<f32>,
    /// Logical pixels scrolled on each tick while a column drag rests near an
    /// edge: negative towards the first column.
    autoscroll: f32,
}
description_debug!(Table);

/// The payload of a column drag: the column's key. Private, so only a table
/// claims it, and only the table that started it.
struct DraggedColumn(String);
impl Table {
    /// Restricts header resizing, which still requires a resizable column.
    /// Default allowed; repeated restrictions combine through [`Permission::and`].
    /// Revocation cancels an active resize and restores its original fixed width
    /// or flex weight, including whether the width was a user preference. Ordinary
    /// Escape retains the dragged width as before. Bound changes are observed
    /// during update without polling; restoration invalidates layout. Describing
    /// the table again replaces the restriction with the new description's; a
    /// fixed denial there cancels like a revocation.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Label, ListSelection, Runtime, SelectionMode,
    ///     Table, TableColumn, TableController, TableItems, TableSource, Viewport};
    /// struct Rows;
    /// impl TableItems for Rows {
    ///     fn len(&self) -> usize { 1 }
    ///     fn key(&self, index: usize) -> Option<String> {
    ///         (index == 0).then(|| "row".into())
    ///     }
    ///     fn cell(&self, _: usize, _: &str) -> Option<Element> {
    ///         Some(Element::new("value", Label::new("Editable width")))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(320.0, 160.0, 1.0)?)?;
    /// let can_resize = ui.signal(true);
    /// let state = TableController::new(&ui,
    ///     ListSelection::new(&ui, SelectionMode::Single),
    ///     [TableColumn::new("value", "Value").resizable(true)],
    ///     TableSource::new(Rows))?;
    /// ui.mount_tree(Element::new("table", Table::new(&state, 24.0)?
    ///     .resize_permission(can_resize.read_only())))?;
    /// ui.flush(ui.now())?;
    /// // Revocation also cancels an in-progress resize and restores its preference.
    /// can_resize.set(false)?;
    /// ui.flush(ui.now())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn resize_permission(mut self, permission: impl Into<Permission>) -> Self {
        self.resize_permission = self.resize_permission.and(permission);
        self
    }
    /// Restricts pointer and keyboard column movement, without making an
    /// immovable column movable. Default allowed; repeated restrictions combine
    /// through [`Permission::and`]. Revocation cancels an armed or active move
    /// before the order changes. Denied Alt-arrow column movement is ignored.
    /// Sorting and scrolling retain their own policy. Describing the table
    /// again replaces the restriction with the new description's; a fixed
    /// denial there cancels like a revocation.
    pub fn reorder_permission(mut self, permission: impl Into<Permission>) -> Self {
        self.reorder_permission = self.reorder_permission.and(permission);
        self
    }
    /// Creates a table over a controller with a fixed logical row height.
    ///
    /// The row is fixed, and a cell gives its content at most that height, so
    /// choose it for the tallest content. Every single-line control, such as
    /// a button, a text field or a checkbox, measures at least the theme's
    /// ControlHeight, so a row with controls, or with an
    /// [editable](TableColumn::editable) column, is at least ControlHeight
    /// tall; text-only rows fit one line of body text with
    /// [`TextOverflow::Ellipsis`](crate::TextOverflow::Ellipsis); a larger
    /// control, such as a [`Knob`](crate::Knob), needs its own height.
    /// Development validation reports content taller than the row as
    /// [`ValidationCode::ExceedsFixedHeight`](crate::ValidationCode::ExceedsFixedHeight).
    ///
    /// A mounted table keeps the height it was created with, also across a
    /// density change and when described again. An application that lets the
    /// user switch density gives the table the height of the largest preset
    /// it offers: the largest ControlHeight over those
    /// [`Density`](crate::Density) values, 40 logical pixels for
    /// `Comfortable` with the default metrics.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::Density;
    ///
    /// struct Tasks;
    /// impl TableItems for Tasks {
    ///     fn len(&self) -> usize { 1 }
    ///     fn key(&self, index: usize) -> Option<String> {
    ///         (index == 0).then(|| "task0".to_owned())
    ///     }
    ///     fn cell(&self, _: usize, _: &str) -> Option<Element> {
    ///         Some(Element::new("done", Checkbox::controlled(false, "Done", |_, _| Ok(()))))
    ///     }
    /// }
    /// let mut ui = Runtime::new();
    /// let controller = TableController::new(
    ///     &ui,
    ///     ListSelection::new(&ui, SelectionMode::Single),
    ///     [TableColumn::new("done", "Done")],
    ///     TableSource::new(Tasks),
    /// )?;
    /// // The row holds the checkbox at the density in effect.
    /// let theme = ui.theme();
    /// let row = theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight);
    /// // At every density the application offers, the largest preset's.
    /// let mut largest = 0.0_f32;
    /// for density in [Density::Compact, Density::Standard, Density::Comfortable] {
    ///     largest = largest.max(theme.metrics(density)?.metric(MetricRole::ControlHeight));
    /// }
    /// assert!(largest >= row);
    /// ui.mount_tree(Element::new("tasks", Table::new(&controller, row)?))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Returns [`GeometryError::NonFinite`](crate::GeometryError::NonFinite)
    /// for a row height that is not finite,
    /// [`GeometryError::Negative`](crate::GeometryError::Negative) for a
    /// negative one and [`GeometryError::Empty`](crate::GeometryError::Empty)
    /// for zero.
    pub fn new(
        controller: &TableController,
        row_height: f32,
    ) -> Result<Self, crate::GeometryError> {
        let activate: Rc<RefCell<Option<CellHandler>>> = Rc::new(RefCell::new(None));
        let (handler, column) = (activate.clone(), controller.active_column());
        let on_selection: Rc<RefCell<Option<SelectionHandler>>> = Rc::new(RefCell::new(None));
        let selection_handler = on_selection.clone();
        let observe_selection = Rc::new(crate::virtual_list::SelectionGate::default());
        let (editor, typed) = (controller.clone(), controller.clone());
        Ok(Self {
            resize_permission: Permission::allowed(),
            reorder_permission: Permission::allowed(),
            resize_allowed: true,
            reorder_allowed: true,
            resize_original: None,
            body_id: None,
            cell_editor: None,
            on_edit_end: None,
            cell_stops: true,
            body: Some(
                VirtualList::new(controller.0.items.read_only(), row_height)?
                    .selection(controller.selection())
                    .controller(controller.0.reveal.clone())
                    // The active cell is outlined by its row instead.
                    .active_outline(false)
                    .keep_mounted(controller.0.shared.editing.row.read_only())
                    // The callbacks forward to the table's, which follow a
                    // re-description of the table.
                    .owned_callbacks()
                    .on_selection_change_gated(observe_selection.clone(), move |cx, change| {
                        match selection_handler.borrow_mut().as_mut() {
                            Some(handler) => handler(cx, change),
                            None => Ok(()),
                        }
                    })
                    .on_activate(move |cx, row| {
                        let column = column.get()?;
                        // Enter on an editable cell edits it with its value selected.
                        if let Some(column) = &column
                            && editor.begin_edit(cx, row, column, crate::EditorStart::SelectAll)?
                        {
                            return Ok(());
                        }
                        match handler.borrow_mut().as_mut() {
                            Some(handler) => handler(cx, row, column.as_deref()),
                            None => Ok(()),
                        }
                    })
                    .on_text_input(move |cx, text| {
                        // Typed text begins an edit of the active cell; Space selects.
                        if text.trim().is_empty() {
                            return Ok(());
                        }
                        let row = typed.0.selection.active().get()?;
                        let column = typed.0.shared.active_column.get()?;
                        if let (Some(row), Some(column)) = (row, column) {
                            typed.begin_edit(
                                cx,
                                &row,
                                &column,
                                crate::EditorStart::Typed(text.to_owned()),
                            )?;
                        }
                        Ok(())
                    }),
            ),
            state: controller.clone(),
            on_sort: None,
            sort: None,
            relayout: 0,
            header: std::cell::Cell::new(0.0),
            titles: RefCell::new(Vec::new()),
            gesture: None,
            bar_hover: false,
            header_hover: None,
            rows: std::cell::Cell::new(0),
            grip: std::cell::Cell::new(0.0),
            bar_extents: Cell::new(scrollbar::Extents::STANDARD),
            size: Cell::new(Size::ZERO),
            row_height,
            activate,
            on_selection,
            observe_selection,
            dragging: None,
            drop_x: None,
            autoscroll: 0.0,
        })
    }
    /// Runs after a sortable header set the sort indication. Sort the data and
    /// publish it with [`TableController::set_source`]; failures propagate through
    /// input routing.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    pub fn on_sort(
        mut self,
        sort: impl FnMut(&mut EventContext, &TableSort) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_sort = Some(Box::new(sort));
        self
    }
    /// Runs when Enter is pressed on the active row: receives the row's key and
    /// the active column's key, or `None` while no column is active. This is
    /// where an application opens an editor for the cell. Failures propagate
    /// through input routing.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    pub fn on_activate(
        self,
        activate: impl FnMut(&mut EventContext, &str, Option<&str>) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        *self.activate.borrow_mut() = Some(Box::new(activate));
        self
    }
    /// Reports every change of the table's [`ListSelection`] after it took
    /// effect, as [`VirtualList::on_selection_change`] does for a list: a click
    /// or a key on a row is [`ChangeOrigin::User`](crate::ChangeOrigin::User),
    /// and a write through the selection or the [`TableController`] is
    /// [`ChangeOrigin::Application`](crate::ChangeOrigin::Application). The
    /// callback runs on the table's body list, so [`EventContext::id`] names
    /// the body. Describing the table again replaces or removes it, as for a
    /// list.
    ///
    /// Timing class: [after settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).
    pub fn on_selection_change(
        self,
        on_change: impl FnMut(
            &mut EventContext,
            &crate::ListSelectionChange,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        *self.on_selection.borrow_mut() = Some(Box::new(on_change));
        self.observe_selection.set(true);
        self
    }
    /// Edits cells of [editable](TableColumn::editable) columns in place:
    /// `editor` builds the editor for one cell when an edit of it begins,
    /// and the table hosts it in the cell and follows how the edit ends.
    /// See [cell editing](Table#cell-editing) for when edits begin, end and
    /// move, and [`CellEdit`] for what the callback receives. The editor
    /// must be a session editor that takes focus, built with
    /// [`CellEdit::session`]; parameter-bound editors ignore sessions and
    /// are not cell editors.
    ///
    /// Timing class: [evaluation](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#evaluation).
    ///
    /// The callback runs once per edit, in the update of the cell's host:
    /// its error fails that flush, is not retried, and closes the edit with
    /// the body keeping focus. A later call replaces an earlier one on the
    /// same description. Describing the table again replaces or removes the
    /// callback for the edits that begin afterwards; an open edit keeps the
    /// editor it built.
    pub fn cell_editor(
        mut self,
        editor: impl FnMut(&mut CellEdit<'_>) -> Result<Element, RuntimeError> + 'static,
    ) -> Self {
        self.cell_editor = Some(Box::new(editor));
        self
    }
    /// Calls `ended` once for each cell edit that ends, however it ended,
    /// after the editor's own reports: a terminal completion the editor
    /// reported ([`CellEditEndReason::Completed`]), and each edit the table
    /// ends itself, which removes the editor without a report of its own:
    /// Escape on the body while the editor does not hold focus, another edit
    /// beginning, the edited row leaving the rows by deletion or a filter, a
    /// failed [cell editor](Self::cell_editor) and the table hiding.
    /// [`CellEditEnd`] names the cell and the [`CellEditEndReason`].
    ///
    /// Timing class: [after settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).
    ///
    /// This is where an application clears what it tracks per edit, such as
    /// the unsaved count a cell editor's `on_modified` feeds: the callback
    /// runs in an event phase and may write signals, including
    /// [`FieldHandle::set_modified`](crate::FieldHandle::set_modified),
    /// which the cell editor callback cannot, since it runs in an update.
    /// Each end is one notification request of its own, so ends keep their
    /// place in the order of every component's notifications; an end can
    /// therefore arrive after a later edit's cell editor callback ran, as
    /// for a replacement begun from F2. A hidden table in a visible window is
    /// still delivered to; a hidden native window holds the reports until it
    /// is shown. The callback's error is returned from the flush that
    /// delivered it; that end is not reported again, and the others still
    /// are.
    ///
    /// The callback's [`EventContext`] is that of an invisible child of the
    /// table that delivers the reports, so [`EventContext::id`] names that
    /// child and [`EventContext::request_focus`] does not focus the table's
    /// body. Removing the table reports nothing, as for any removed
    /// component, and drops ends not yet delivered. Describing the table
    /// again with another callback replaces it, and ends not yet delivered
    /// go to the new one; describing it without one removes it and discards
    /// them. Ends recorded while no callback is installed are never reported.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    ///
    /// # fn example(table: Table, drafts: FieldHandle) -> Table {
    /// table.on_edit_end(move |_, end| {
    ///     match end.reason() {
    ///         CellEditEndReason::Completed(EditorCompletion {
    ///             trigger: EditorTrigger::Tab { direction: FocusDirection::Backward },
    ///             outcome: EditorOutcome::Saved,
    ///             ..
    ///         }) => { /* saved, moving back */ }
    ///         CellEditEndReason::Hidden | CellEditEndReason::RowLeft => {}
    ///         _ => {}
    ///     }
    ///     drafts.set_modified(false).map(|_| ())
    /// })
    /// # }
    /// ```
    pub fn on_edit_end(
        mut self,
        ended: impl FnMut(&mut EventContext, &CellEditEnd) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_edit_end = Some(Box::new(ended));
        self
    }
    /// Whether interactive content inside cells, such as a checkbox, is a
    /// stop of Tab traversal; true by default. With false the body is the
    /// table's one Tab stop ([`VirtualList::item_tab_stops`]): F2 on an
    /// active cell that is not edited in place moves focus into its content,
    /// and Escape that the content leaves unhandled returns focus to the
    /// body. Pointer presses still reach the content. Set before the table
    /// mounts; a later description keeps the first choice.
    pub fn cell_tab_stops(mut self, stops: bool) -> Self {
        self.cell_stops = stops;
        self.body = self.body.take().map(|body| body.item_tab_stops(stops));
        self
    }
    /// F2 and Escape for cell editing, after a header gesture's Escape: F2
    /// edits the active cell with the caret at the end, or enters its resting
    /// content when cells are not Tab stops; Escape cancels an open edit
    /// whose editor does not hold focus.
    fn edit_key(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        control: KeyCode,
        keys: &KeyboardState,
    ) -> Result<Option<InputOutcome>, RuntimeError> {
        if event.transition != KeyTransition::Pressed
            || keys.chord_modifiers(event) != crate::Modifiers::default()
        {
            return Ok(None);
        }
        let editing = &self.state.0.shared.editing;
        match control {
            KeyCode::F2 => {
                let row = self.state.0.selection.active().get()?;
                let column = self.state.0.shared.active_column.get()?;
                let (Some(row), Some(column)) = (row, column) else {
                    return Ok(None);
                };
                if self
                    .state
                    .begin_edit(cx, &row, &column, crate::EditorStart::CaretAtEnd)?
                {
                    return Ok(Some(InputOutcome::Handled));
                }
                if editing.cell_stops.get() {
                    return Ok(None);
                }
                let cell = editing
                    .active_cells
                    .borrow()
                    .iter()
                    .find(|(key, _)| *key == column)
                    .map(|(_, id)| id.clone());
                match cell {
                    Some(cell) => {
                        cx.request_focus_within(&cell)?;
                        Ok(Some(InputOutcome::Handled))
                    }
                    None => Ok(None),
                }
            }
            KeyCode::Escape if self.gesture.is_none() => Ok(self
                .state
                .cancel_unfocused()?
                .then_some(InputOutcome::Handled)),
            _ => Ok(None),
        }
    }
    /// Moves the keyboard focus to the body, where a press on a cell puts it, so
    /// the Escape that ends a header gesture reaches the header by bubbling
    /// from the body wherever the focus was before.
    fn focus_body(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        match &self.body_id {
            Some(body) => cx.request_focus_for(body),
            None => Ok(()),
        }
    }
    // The horizontal bar: track, thumb and maximum offset. It reserves the far
    // end when the rows overflow too, so it never crosses the body's own bar.
    fn set_bar_hover(&mut self, cx: &mut EventContext, hover: bool) {
        if self.bar_hover != hover {
            self.bar_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn set_header_hover(&mut self, cx: &mut EventContext, hover: Option<Point>) {
        if self.header_hover != hover {
            self.header_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn header_target(&self, point: Point) -> Option<(usize, bool)> {
        (point.y() >= 0.0 && point.y() < self.header.get()).then_some(())?;
        self.edge_at(point.x(), self.grip.get())
            .map(|index| (index, true))
            .or_else(|| {
                let index = self.column_at(point.x())?;
                (self.state.0.shared.columns.borrow()[index].sortable
                    || self.reorder_allowed && self.state.0.shared.columns.borrow()[index].movable)
                    .then_some((index, false))
            })
    }
    fn bar(&self) -> Result<scrollbar::ScrollbarGeometry, crate::GeometryError> {
        let shared = &self.state.0.shared;
        let size = self.size.get();
        let extents = self.bar_extents.get();
        let body = (size.height() - self.header.get()).max(0.0);
        let crossing = self.rows.get() as f32 * self.row_height > body;
        scrollbar::geometry(
            extents,
            Rect::from_size(size),
            false,
            shared.total.get(),
            shared.offset.get(),
            if crossing { extents.lane } else { 0.0 },
        )
    }
    fn overflows(&self) -> bool {
        self.state.0.shared.maximum_offset() > 0.0
    }
    fn scroll(&self, cx: &mut EventContext, offset: f32) -> Result<bool, RuntimeError> {
        let changed = self.state.0.shared.scroll_to(offset)?;
        if changed {
            cx.invalidate(Invalidation::Paint);
        }
        Ok(changed)
    }
    fn drag_bar(&self, cx: &mut EventContext, point: Point, grab: f32) -> Result<(), RuntimeError> {
        let bar = self.bar()?;
        if let Some(next) = bar.offset_for_pointer(point, grab) {
            self.scroll(cx, next)?;
        }
        Ok(())
    }
    // Left and Right: from no column to the first or last, then one at a time.
    fn move_column(&self, cx: &mut EventContext, right: bool) -> Result<bool, RuntimeError> {
        let shared = &self.state.0.shared;
        let count = shared.columns.borrow().len();
        let current = shared
            .active_column
            .get()?
            .and_then(|key| shared.column_index(&key));
        let next = match (current, right) {
            (None, true) => 0,
            (None, false) => count - 1,
            (Some(column), true) => (column + 1).min(count - 1),
            (Some(column), false) => column.saturating_sub(1),
        };
        let key = shared.columns.borrow()[next].key.clone();
        let moved = shared.active_column.set_changed(Some(key))?;
        if shared.reveal_column(next)? {
            cx.invalidate(Invalidation::Paint);
        }
        Ok(moved)
    }
    // The column whose trailing edge is within the resize grip of `x`, a
    // position in the table's own coordinates.
    fn edge_at(&self, x: f32, grip: f32) -> Option<usize> {
        let x = x + self.state.0.shared.offset.get();
        let columns = self.state.0.shared.columns.borrow();
        self.state
            .0
            .shared
            .resolved
            .borrow()
            .iter()
            .enumerate()
            .find(|(index, (start, width))| {
                self.resize_allowed
                    && columns[*index].resizable
                    && (x - (start + width)).abs() <= grip / 2.0
            })
            .map(|(index, _)| index)
    }
    fn column_at(&self, x: f32) -> Option<usize> {
        let x = x + self.state.0.shared.offset.get();
        self.state
            .0
            .shared
            .resolved
            .borrow()
            .iter()
            .position(|(start, width)| x >= *start && x < start + width)
    }
    // Moves a column to a boundary and rebuilds the rows in the new order.
    fn move_column_to(
        &self,
        cx: &mut EventContext,
        column: usize,
        boundary: usize,
    ) -> Result<bool, RuntimeError> {
        if !self.state.0.shared.move_to(column, boundary)? {
            return Ok(false);
        }
        self.state.republish()?;
        cx.invalidate(Invalidation::Layout);
        Ok(true)
    }
    // Alt with Left or Right: the active cell's column one place, within its run.
    fn move_active_column(&self, cx: &mut EventContext, right: bool) -> Result<(), RuntimeError> {
        let shared = &self.state.0.shared;
        let Some(column) = shared
            .active_column
            .get()?
            .and_then(|key| shared.column_index(&key))
        else {
            return Ok(());
        };
        let boundary = if right {
            column + 2
        } else {
            match column.checked_sub(1) {
                Some(boundary) => boundary,
                None => return Ok(()),
            }
        };
        if self.move_column_to(cx, column, boundary)? {
            let moved = if right { column + 1 } else { column - 1 };
            if shared.reveal_column(moved)? {
                cx.invalidate(Invalidation::Paint);
            }
        }
        Ok(())
    }
    // Where the dragged column would go for the drag's current position.
    fn drop_boundary(&self) -> Option<(usize, usize)> {
        let shared = &self.state.0.shared;
        let column = shared.column_index(self.dragging.as_deref()?)?;
        let x = self.drop_x? + shared.offset.get();
        Some((column, shared.boundary_near(column, x)?))
    }
    // How fast a column drag at local `x` scrolls: negative towards the start.
    fn autoscroll_at(&self, x: f32) -> f32 {
        let width = self.size.get().width();
        let zone = AUTOSCROLL_ZONE.min(width / 3.0);
        if !self.overflows() || zone <= 0.0 {
            return 0.0;
        }
        let depth = if x < zone {
            -(zone - x.max(0.0)) / zone
        } else if x > width - zone {
            (x.min(width) - (width - zone)) / zone
        } else {
            0.0
        };
        depth * AUTOSCROLL_STEP
    }
    fn end_drop_indication(&mut self, cx: &mut EventContext) {
        if self.drop_x.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
        if std::mem::take(&mut self.autoscroll) != 0.0 {
            cx.cancel_timer();
        }
    }
    // The column order a workspace entry asks for, resolved against the current
    // columns: the listed ones take the slots the listed ones hold now, in the
    // saved order, and columns added since keep theirs. Without an order, the
    // order the application described.
    fn restored_order(
        &self,
        state: &crate::ViewState,
    ) -> Result<Vec<String>, crate::RestoreRejection> {
        use crate::RestoreRejection;
        let shared = &self.state.0.shared;
        let current: Vec<String> = shared
            .columns
            .borrow()
            .iter()
            .map(|column| column.key.clone())
            .collect();
        let validate = |candidate: Vec<String>| {
            for (index, key) in current.iter().enumerate() {
                let next = candidate
                    .iter()
                    .position(|other| other == key)
                    .ok_or_else(|| RestoreRejection::OutOfRange(key.clone()))?;
                let allowed = shared
                    .movable_run(index)
                    .map_or(next == index, |(first, last)| {
                        (first..=last).contains(&next)
                    });
                if !allowed {
                    return Err(RestoreRejection::OutOfRange(key.clone()));
                }
            }
            Ok(candidate)
        };
        let Some(order) = state.get("order") else {
            let mut described: Vec<String> = shared
                .described
                .iter()
                .filter(|key| current.contains(key))
                .cloned()
                .collect();
            described.extend(
                current
                    .iter()
                    .filter(|key| !described.contains(key))
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            return validate(described);
        };
        let order = order
            .as_map()
            .ok_or_else(|| RestoreRejection::Value("order".to_owned()))?;
        let mut listed = Vec::with_capacity(order.len());
        for (key, position) in order {
            if !current.contains(key) {
                return Err(RestoreRejection::Unresolved(key.clone()));
            }
            let position = position
                .as_integer()
                .ok_or_else(|| RestoreRejection::Value("order".to_owned()))?;
            // Capture writes the positions 0..n of every column it lists, so
            // the listed positions must be exactly such a permutation; any
            // other numbers would be renumbered on the next capture.
            let listed_positions = order.len() as i64;
            if !(0..listed_positions).contains(&position)
                || listed.iter().any(|(other, _)| *other == position)
            {
                return Err(RestoreRejection::OutOfRange(key.clone()));
            }
            listed.push((position, key.clone()));
        }
        listed.sort();
        let mut saved = listed.into_iter().map(|(_, key)| key);
        validate(
            current
                .iter()
                .map(|key| {
                    if order.contains_key(key) {
                        saved.next().unwrap_or_else(|| key.clone())
                    } else {
                        key.clone()
                    }
                })
                .collect(),
        )
    }
    // Sets the sort indication and asks the application to sort, as a header
    // press and the keyboard both do.
    fn request_sort(
        &mut self,
        cx: &mut EventContext,
        column: String,
        direction: SortDirection,
    ) -> Result<(), RuntimeError> {
        let sort = TableSort { column, direction };
        self.state.set_sort(Some(sort.clone()))?;
        if let Some(handler) = &mut self.on_sort {
            handler(cx, &sort)?;
        }
        Ok(())
    }
    fn resize(&self, cx: &mut EventContext, column: usize, width: f32) -> Result<(), RuntimeError> {
        let shared = &self.state.0.shared;
        {
            let mut columns = shared.columns.borrow_mut();
            let next = ColumnWidth::Fixed(width.max(columns[column].minimum));
            // A repeated position changes nothing. The runtime may deliver the
            // same pointer position again after a layout change, and answering
            // that with another layout change would never settle.
            if columns[column].width == next {
                return Ok(());
            }
            columns[column].width = next;
            shared
                .user_sized
                .borrow_mut()
                .insert(columns[column].key.clone());
        }
        let revision = shared.revision.get()?;
        shared.revision.set(revision.wrapping_add(1))?;
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }
}
impl Component for Table {
    fn workspace_state(&self) -> Result<Option<crate::ViewState>, crate::WorkspaceError> {
        let columns = self.state.0.shared.columns.borrow();
        let user_sized = self.state.0.shared.user_sized.borrow();
        // Only a column the user sized, by dragging or through a restored
        // entry, carries a width. A flexible column is a share of whatever
        // width the table has, so persisting its measured extent would freeze
        // one window size into the document, and a width the application
        // described stays the application's to change in a later version.
        let widths: Vec<_> = columns
            .iter()
            .filter(|column| user_sized.contains(&column.key))
            .filter_map(|column| match column.width {
                ColumnWidth::Fixed(width) => Some((
                    column.key().to_owned(),
                    crate::WorkspaceValue::finite(width),
                )),
                ColumnWidth::Flex(_) => None,
            })
            .collect();
        let state =
            crate::ViewState::new("table")?.with("widths", crate::WorkspaceValue::map(widths)?)?;
        // The order is recorded only once the user moved a column, so a table
        // as the application described it captures as it always did.
        let order: Vec<_> = columns.iter().map(|column| column.key.clone()).collect();
        if order == self.state.0.shared.described {
            return Ok(Some(state));
        }
        let positions = order
            .into_iter()
            .enumerate()
            .map(|(index, key)| (key, crate::WorkspaceValue::integer(index as i64)));
        Ok(Some(
            state.with("order", crate::WorkspaceValue::map(positions)?)?,
        ))
    }
    fn restore_workspace_state(
        &mut self,
        state: &crate::ViewState,
    ) -> Result<Invalidation, crate::RestoreRejection> {
        use crate::RestoreRejection;
        state.expect_only(&["widths", "order"])?;
        if self.gesture.is_some() || self.dragging.is_some() {
            return Err(RestoreRejection::Busy);
        }
        let widths = state.map("widths")?;
        let shared = &self.state.0.shared;
        let order = self.restored_order(state)?;
        // Resolve everything before writing anything: a rejected entry must
        // leave the table exactly as it was, not half resized.
        let mut resolved = Vec::with_capacity(widths.len());
        for (key, value) in widths {
            let index = shared
                .columns
                .borrow()
                .iter()
                .position(|column| column.key() == key)
                .ok_or_else(|| RestoreRejection::Unresolved(key.clone()))?;
            let stored = value
                .as_number()
                .ok_or_else(|| RestoreRejection::Value("widths".to_owned()))?;
            // Capture never writes a width below the column's minimum, so a
            // smaller one is not this table's data: rejected, never raised.
            // The stored number is checked before narrowing, which could
            // round a width just below the minimum up to it, and one beyond
            // the largest `f32` is out of range too.
            let minimum = f64::from(shared.columns.borrow()[index].minimum);
            if !stored.is_finite() || stored < minimum || stored > f64::from(f32::MAX) {
                return Err(RestoreRejection::OutOfRange(key.clone()));
            }
            let width = stored as f32;
            resolved.push((index, width));
        }
        // This public trait hook can also be called outside Runtime's restore
        // entry point. Reject closed runtimes and forbidden write phases before
        // changing controller state, including when no order change is needed.
        shared
            .revision
            .shared()
            .writable()
            .map_err(RestoreRejection::Runtime)?;
        let revision = shared.revision.get().map_err(RestoreRejection::Runtime)?;
        let previous_columns = shared.columns.borrow().clone();
        let previous_user_sized = shared.user_sized.borrow().clone();
        // Keep the old row source alive through both publication writes. Its
        // destructor must not run between publishing rows and their counts.
        let previous_rows = self
            .state
            .0
            .items
            .get()
            .map_err(RestoreRejection::Runtime)?;
        shared
            .revision
            .set(revision.wrapping_add(1))
            .map_err(RestoreRejection::Runtime)?;
        let reordered = {
            let mut columns = shared.columns.borrow_mut();
            // A column the entry gives no width was not sized by the user when
            // it was captured, so it returns to the width the application gave
            // it, and capturing again reproduces the entry. A column the entry
            // does give a width counts as user sized, including one an older
            // capture recorded only because the application had fixed it.
            for column in columns.iter_mut() {
                if let Some((_, width)) = shared
                    .described_widths
                    .iter()
                    .find(|(key, _)| *key == column.key)
                {
                    column.width = *width;
                }
            }
            let mut user_sized = shared.user_sized.borrow_mut();
            user_sized.clear();
            for (index, width) in resolved {
                columns[index].width = ColumnWidth::Fixed(width);
                user_sized.insert(columns[index].key.clone());
            }
            drop(user_sized);
            let current: Vec<_> = columns.iter().map(|column| column.key.clone()).collect();
            let reordered = order != current;
            if reordered {
                let mut previous = std::mem::take(&mut *columns);
                for key in &order {
                    if let Some(at) = previous.iter().position(|column| column.key == *key) {
                        columns.push(previous.remove(at));
                    }
                }
            }
            reordered
        };
        // Row text and filter matchers are required to be side-effect free;
        // publishing ListSource and FilterCounts compares no application values.
        // Preserve the columns if publication is nevertheless rejected. The
        // revision above only invalidates rows; it carries no persisted state.
        if reordered && let Err(error) = self.state.republish() {
            *shared.columns.borrow_mut() = previous_columns;
            *shared.user_sized.borrow_mut() = previous_user_sized;
            return Err(RestoreRejection::Runtime(error));
        }
        // Only now that the restore cannot fail: the resolved columns follow
        // the new order at once, as after a move, so nothing pairs `columns`
        // and `resolved` by index across the reorder, while a rejected restore
        // above keeps the resolved columns and the offset `resolve` clamps.
        shared.resolve(shared.width.get());
        drop(previous_rows);
        Ok(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        // The body stays the first child; the reporter that delivers edit
        // ends follows it.
        let Some(body) = self.body.take() else {
            return Vec::new();
        };
        vec![
            Element::new("body", body).structural(),
            Element::new(
                editing::REPORTER_KEY,
                EditEndReporter {
                    shared: self.state.0.shared.clone(),
                },
            )
            .structural(),
        ]
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The controller, row height and body stay. The callbacks follow the
        // description: the body's forward to the cells replaced here, and a
        // removed selection callback discards the body's undelivered changes.
        let on_selection = next.on_selection.borrow_mut().take();
        self.observe_selection.set(on_selection.is_some());
        *self.on_selection.borrow_mut() = on_selection;
        *self.activate.borrow_mut() = next.activate.borrow_mut().take();
        self.on_sort = next.on_sort;
        // Edits that begin from now on build their editor with the new
        // callback; an open edit keeps the editor it built. A first
        // description's callback not yet installed is replaced too.
        self.cell_editor = None;
        *self.state.0.shared.editing.editor.borrow_mut() = next.cell_editor;
        // A new edit-end callback receives the ends not yet delivered; none
        // removes the callback and discards them.
        self.on_edit_end = None;
        self.state.0.shared.editing.ends.describe(next.on_edit_end);
        // The update this schedules reads the new restrictions and cancels a
        // gesture they deny.
        let mut update = false;
        if !self.resize_permission.same_as(&next.resize_permission) {
            self.resize_permission = next.resize_permission;
            update = true;
        }
        if !self.reorder_permission.same_as(&next.reorder_permission) {
            self.reorder_permission = next.reorder_permission;
            update = true;
        }
        update.then_some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let editing = &self.state.0.shared.editing;
        // The mounted description's editing options; a redescription
        // installs its editor callback itself.
        if let Some(editor) = self.cell_editor.take() {
            *editing.editor.borrow_mut() = Some(editor);
        }
        if let Some(ended) = self.on_edit_end.take() {
            editing.ends.describe(Some(ended));
        }
        editing.cell_stops.set(self.cell_stops);
        if editing.notifier.borrow().is_none() {
            *editing.notifier.borrow_mut() = Some(cx.notifier());
        }
        *editing.body.borrow_mut() = cx.children().first().map(|child| child.id().clone());
        // An edit whose row left the rows, or whose editor failed, closes at
        // the next delivery, where the rows can be published again.
        if editing.row_gone(&*cx.read_paint(&self.state.0.items)?)
            && let Some(notifier) = &*editing.notifier.borrow()
        {
            notifier.request();
        }
        // Bound titles are read here, which subscribes the table to them.
        let bound: Vec<(String, TextSource)> = self
            .state
            .0
            .shared
            .columns
            .borrow()
            .iter()
            .filter(|column| column.source.is_bound())
            .map(|column| (column.key.clone(), column.source.clone()))
            .collect();
        let mut retitled = false;
        for (key, source) in bound {
            let content = source.read(cx)?;
            if let Some(column) = self
                .state
                .0
                .shared
                .columns
                .borrow_mut()
                .iter_mut()
                .find(|column| column.key == key)
            {
                retitled |= column.set_title(content);
            }
        }
        if retitled {
            cx.request_layout();
        }
        let resize_allowed = self.resize_permission.read_paint(cx)?;
        let reorder_allowed = self.reorder_permission.read_paint(cx)?;
        if self.resize_allowed
            && !resize_allowed
            && matches!(self.gesture, Some(HeaderGesture::Resize(..)))
        {
            let revision = self.state.0.shared.revision.get()?;
            if let Some((key, width, user_sized)) = self.resize_original.take() {
                let shared = &self.state.0.shared;
                let mut changed = false;
                if let Some(column) = shared
                    .columns
                    .borrow_mut()
                    .iter_mut()
                    .find(|c| c.key == key)
                {
                    changed = column.width != width;
                    column.width = width;
                }
                let mut preferences = shared.user_sized.borrow_mut();
                changed |= if user_sized {
                    preferences.insert(key)
                } else {
                    preferences.remove(&key)
                };
                drop(preferences);
                if changed {
                    // The revision is library-owned; notify dependent cells
                    // without invoking application code during update.
                    shared.revision.publish_trusted(revision.wrapping_add(1));
                    cx.request_layout();
                }
            }
            self.gesture = None;
            self.state.0.shared.resizing.set(false);
            cx.cancel_input();
        }
        // A title press may still sort after movement is denied. Only an
        // exclusively movable title loses its armed press; a started drag is
        // always cancelled, including one from a sortable column.
        let movement_only_press = self.reorder_allowed
            && !reorder_allowed
            && match &self.gesture {
                Some(HeaderGesture::Title(key, _)) => self
                    .state
                    .0
                    .shared
                    .columns
                    .borrow()
                    .iter()
                    .find(|column| column.key == *key)
                    .is_none_or(|column| !column.sortable),
                _ => false,
            };
        if self.reorder_allowed
            && !reorder_allowed
            && (self.dragging.is_some() || movement_only_press)
        {
            self.gesture = None;
            self.dragging = None;
            self.drop_x = None;
            self.autoscroll = 0.0;
            cx.cancel_input();
        }
        self.resize_allowed = resize_allowed;
        self.reorder_allowed = reorder_allowed;
        // A controller width write changes the columns' extents, which only
        // an arrangement applies to the header and the rows.
        let relayout = cx.read(&self.state.0.shared.relayout)?;
        if relayout != self.relayout {
            self.relayout = relayout;
            cx.request_layout();
        }
        self.body_id = cx.children().first().map(|child| child.id().clone());
        self.sort = cx.read_paint(&self.state.sort())?;
        self.rows.set(cx.read_paint(&self.state.0.items)?.len());
        Ok(())
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (point.y() < self.header.get()).then_some(())?;
        if self.edge_at(point.x(), self.grip.get()).is_some() {
            return Some(CursorIcon::ResizeHorizontal);
        }
        // Only a title that can be dragged offers to be.
        let column = self.column_at(point.x())?;
        (self.reorder_allowed && self.state.0.shared.columns.borrow()[column].movable)
            .then_some(CursorIcon::Move)
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let header = cx.metric(MetricRole::ControlHeight);
        let body = cx.measure(
            0,
            Constraints::new(
                available.width(),
                available.height().map(|height| (height - header).max(0.0)),
            )?,
        )?;
        Ok(Size::new(body.width(), body.height() + header)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        // The body, then the edit-end reporter, which has no extent.
        if cx.len() != 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        cx.place(1, Rect::new(0.0, 0.0, 0.0, 0.0)?)?;
        let header = cx
            .measurement()
            .metric(MetricRole::ControlHeight)
            .min(size.height());
        self.header.set(header);
        self.size.set(size);
        self.bar_extents
            .set(scrollbar::Extents::read(cx.measurement()));
        let shared = &self.state.0.shared;
        shared.resolve(size.width());
        self.grip
            .set(cx.measurement().metric(MetricRole::ResizeWidth));
        *self.titles.borrow_mut() = shared
            .columns
            .borrow()
            .iter()
            .map(|column| {
                (!column.title.is_empty())
                    .then(|| {
                        cx.measurement()
                            .rich_text(&column.content, None, TextStyle::Body)
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        cx.place(
            0,
            Rect::new(0.0, header, size.width(), size.height() - header)?,
        )
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        let header = self.header.get();
        painter.fill_rect(
            Rect::new(0.0, 0.0, size.width(), header)?,
            painter.color(ColorRole::Surface),
        );
        let padding = painter.metric(MetricRole::ControlPaddingX);
        let icon = painter.metric(MetricRole::IconSize);
        let border = painter.metric(MetricRole::BorderWidth);
        let columns = self.state.0.shared.columns.borrow();
        let resolved = self.state.0.shared.resolved.borrow();
        let titles = self.titles.borrow();
        let offset = self.state.0.shared.offset.get();
        let header_hover = self
            .header_hover
            .and_then(|point| self.header_target(point));
        for (index, (start, width)) in resolved.iter().copied().enumerate() {
            let start = start - offset;
            let Some(column) = Rect::new(start, 0.0, width, header)
                .ok()
                .map(|rect| rect.intersection(Rect::from_size(size)))
                .filter(|rect| !rect.is_empty())
            else {
                continue;
            };
            if header_hover == Some((index, false)) {
                painter.fill_rect(column, painter.color(ColorRole::SurfaceHover));
            }
            if header_hover == Some((index, true)) {
                let grip = self.grip.get().min(width);
                painter.fill_rect(
                    Rect::new(start + width - grip / 2.0, 0.0, grip, header)?
                        .intersection(Rect::from_size(size)),
                    painter.color(ColorRole::ResizeHandleHover),
                );
            }
            let sorted = self
                .sort
                .as_ref()
                .filter(|sort| sort.column == columns[index].key);
            painter.with_clip(column, |painter| -> Result<(), DrawError> {
                if let Some(Some(title)) = titles.get(index) {
                    let left = start + padding;
                    let right = if sorted.is_some() {
                        start + (width - icon - padding / 2.0).max(0.0)
                            - painter.metric(MetricRole::IconGap)
                    } else {
                        start + width - padding
                    };
                    painter.with_clip(
                        Rect::new(left, 0.0, (right - left).max(0.0), header)?,
                        |painter| {
                            title.paint(
                                painter,
                                crate::Point::new(
                                    left,
                                    ((header - title.size().height()) / 2.0).max(0.0),
                                )?,
                                painter.color(ColorRole::OnSurface),
                            )
                        },
                    )?;
                }
                if let Some(sort) = sorted {
                    painter.icon(
                        match sort.direction {
                            SortDirection::Ascending => Icon::ChevronUp,
                            SortDirection::Descending => Icon::ChevronDown,
                        },
                        Rect::new(
                            start + (width - icon - padding / 2.0).max(0.0),
                            ((header - icon) / 2.0).max(0.0),
                            icon.min(width),
                            icon.min(header),
                        )?,
                        painter.color(ColorRole::Primary),
                    )?;
                }
                Ok(())
            })?;
            painter.fill_rect(
                Rect::new(start + width - border, 0.0, border, header)?
                    .intersection(Rect::from_size(size)),
                painter.color(ColorRole::Divider),
            );
        }
        painter.fill_rect(
            Rect::new(
                0.0,
                (header - border).max(0.0),
                size.width(),
                border.min(header),
            )?,
            painter.color(ColorRole::Divider),
        );
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        // The bar under the pointer shows that it can be dragged. Moving onto a
        // cell that takes the pointer delivers `Leave` here, which clears it.
        if !matches!(self.gesture, Some(HeaderGesture::Scroll(_)))
            && matches!(event.kind(), PointerKind::Move | PointerKind::Leave)
        {
            let over = event.kind() == PointerKind::Move
                && !event.is_outside()
                && cx.pointer_inside()
                && self.hit_test_foreground(point);
            self.set_bar_hover(cx, over);
            let header_hover = if event.kind() == PointerKind::Move
                && !event.is_outside()
                && cx.pointer_inside()
            {
                self.header_target(point).map(|_| point)
            } else {
                None
            };
            self.set_header_hover(cx, header_hover);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary)
                if !event.is_outside()
                    && self.overflows()
                    && self.bar().is_ok_and(|bar| bar.track.contains(point)) =>
            {
                let grab = self.bar()?.grab_offset(point);
                cx.capture_pointer()?;
                self.focus_body(cx)?;
                self.gesture = Some(HeaderGesture::Scroll(grab));
                self.drag_bar(cx, point, grab)?;
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Press(PointerButton::Primary)
                if point.y() >= 0.0 && point.y() < self.header.get() && !event.is_outside() =>
            {
                if let Some(column) = self.edge_at(point.x(), self.grip.get()) {
                    let width = self.state.0.shared.resolved.borrow()[column].1;
                    let key = self.state.0.shared.columns.borrow()[column].key.clone();
                    cx.capture_pointer()?;
                    self.focus_body(cx)?;
                    let original = self.state.0.shared.columns.borrow()[column].width;
                    let user_sized = self.state.0.shared.user_sized.borrow().contains(&key);
                    self.resize_original = Some((key.clone(), original, user_sized));
                    self.gesture = Some(HeaderGesture::Resize(key, width, point.x()));
                    self.state.0.shared.resizing.set(true);
                    return Ok(InputOutcome::Handled);
                }
                if let Some(column) = self.column_at(point.x())
                    && (self.state.0.shared.columns.borrow()[column].sortable
                        || self.reorder_allowed
                            && self.state.0.shared.columns.borrow()[column].movable)
                {
                    let key = self.state.0.shared.columns.borrow()[column].key.clone();
                    cx.capture_pointer()?;
                    self.focus_body(cx)?;
                    self.gesture = Some(HeaderGesture::Title(key, event.position()));
                    return Ok(InputOutcome::Handled);
                }
            }
            // A movable title becomes a drag once the press has moved far
            // enough; the distance is in window pixels, since it describes the
            // hand.
            PointerKind::Move
                if !cx.drag_active()
                    && let Some(HeaderGesture::Title(key, origin)) = &self.gesture
                    && (event.position().x() - origin.x())
                        .hypot(event.position().y() - origin.y())
                        >= DRAG_THRESHOLD
                    && let Some(column) = self.state.0.shared.column_index(key)
                    && self.reorder_allowed
                    && self.state.0.shared.movable_run(column).is_some() =>
            {
                let column = self.state.0.shared.columns.borrow()[column].clone();
                self.gesture = None;
                cx.start_drag(
                    DragPayload::new(DraggedColumn(column.key.clone())),
                    DropOperations::MOVE,
                    DragPreview::element(Element::new("column", Label::new(column.content))),
                )?;
                self.dragging = Some(column.key);
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Move => match &self.gesture {
                Some(HeaderGesture::Resize(key, width, pressed)) => {
                    if let Some(column) = self.state.0.shared.column_index(key) {
                        self.resize(cx, column, *width + point.x() - *pressed)?;
                    }
                    return Ok(InputOutcome::Handled);
                }
                Some(HeaderGesture::Scroll(grab)) => {
                    self.drag_bar(cx, point, *grab)?;
                    return Ok(InputOutcome::Handled);
                }
                _ => {}
            },
            PointerKind::Scroll(delta)
                if delta.x() != 0.0
                    && self.scroll(cx, self.state.0.shared.offset.get() + delta.x())? =>
            {
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Release(PointerButton::Primary) => {
                self.resize_original = None;
                self.state.0.shared.resizing.set(false);
                match self.gesture.take() {
                    Some(HeaderGesture::Resize(key, width, pressed)) => {
                        if let Some(column) = self.state.0.shared.column_index(&key) {
                            self.resize(cx, column, width + point.x() - pressed)?;
                        }
                        cx.release_pointer();
                        return Ok(InputOutcome::Handled);
                    }
                    Some(HeaderGesture::Scroll(grab)) => {
                        self.drag_bar(cx, point, grab)?;
                        cx.release_pointer();
                        let over = !event.is_outside()
                            && cx.pointer_inside()
                            && self.hit_test_foreground(point);
                        self.set_bar_hover(cx, over);
                        cx.invalidate(Invalidation::Paint);
                        return Ok(InputOutcome::Handled);
                    }
                    Some(HeaderGesture::Title(key, _)) => {
                        cx.release_pointer();
                        let Some(column) = self.state.0.shared.column_index(&key) else {
                            return Ok(InputOutcome::Handled);
                        };
                        if self.state.0.shared.columns.borrow()[column].sortable
                            && point.y() >= 0.0
                            && point.y() < self.header.get()
                            && self.column_at(point.x()) == Some(column)
                        {
                            let key = self.state.0.shared.columns.borrow()[column].key.clone();
                            let direction = match &self.sort {
                                Some(sort)
                                    if sort.column == key
                                        && sort.direction == SortDirection::Ascending =>
                                {
                                    SortDirection::Descending
                                }
                                _ => SortDirection::Ascending,
                            };
                            self.request_sort(cx, key, direction)?;
                        }
                        return Ok(InputOutcome::Handled);
                    }
                    None => {}
                }
            }
            _ => {}
        }
        Ok(InputOutcome::Ignored)
    }
    fn input_reset(&mut self) {
        self.gesture = None;
        self.state.0.shared.resizing.set(false);
        self.resize_original = None;
        self.bar_hover = false;
        self.header_hover = None;
        self.drop_x = None;
        // A pending autoscroll tick finds nothing to do and stops.
        self.autoscroll = 0.0;
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        // Only this table's own column drag concerns it.
        if !self.reorder_allowed
            || event.role() != DragRole::Candidate
            || event.source() != &cx.id()
            || event
                .payload()
                .downcast_ref::<DraggedColumn>()
                .is_none_or(|column| self.dragging.as_ref() != Some(&column.0))
        {
            return Ok(DropResponse::Rejected);
        }
        let Some(point) = cx
            .to_local(event.position())
            .filter(|_| event.phase() != DragPhase::Leave)
        else {
            self.end_drop_indication(cx);
            return Ok(DropResponse::Rejected);
        };
        if self.drop_x.replace(point.x()) != Some(point.x()) {
            cx.invalidate(Invalidation::Paint);
        }
        if event.phase() == DragPhase::Drop {
            let boundary = self.drop_boundary();
            self.end_drop_indication(cx);
            // A column that is gone, or no longer movable, is refused rather
            // than reported as moved. A drop beside itself moves nothing and
            // is still a completed drop.
            let Some((column, boundary)) = boundary else {
                return Ok(DropResponse::Rejected);
            };
            self.move_column_to(cx, column, boundary)?;
            return Ok(DropResponse::Accepted(DropOperation::Move));
        }
        let speed = self.autoscroll_at(point.x());
        if speed != 0.0 && self.autoscroll == 0.0 {
            cx.set_timer(AUTOSCROLL_INTERVAL)?;
        } else if speed == 0.0 && self.autoscroll != 0.0 {
            cx.cancel_timer();
        }
        self.autoscroll = speed;
        Ok(DropResponse::Accepted(DropOperation::Move))
    }
    fn drag_finished(&mut self, cx: &mut EventContext, _: DragOutcome) -> Result<(), RuntimeError> {
        self.dragging = None;
        self.end_drop_indication(cx);
        Ok(())
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.autoscroll == 0.0 || !cx.drag_active() {
            self.autoscroll = 0.0;
            return Ok(());
        }
        // The columns move under a still pointer; the line is derived from
        // them at paint, so it follows.
        if self.scroll(cx, self.state.0.shared.offset.get() + self.autoscroll)? {
            cx.set_timer(AUTOSCROLL_INTERVAL)?;
        } else {
            self.autoscroll = 0.0;
        }
        Ok(())
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered {
            self.set_bar_hover(cx, false);
            self.set_header_hover(cx, None);
        }
    }
    fn hit_test_foreground(&self, point: Point) -> bool {
        self.overflows() && self.bar().is_ok_and(|bar| bar.track.contains(point))
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        if let Some((_, boundary)) = self.drop_boundary() {
            let shared = &self.state.0.shared;
            let resolved = shared.resolved.borrow();
            let edge = match resolved.get(boundary) {
                Some((start, _)) => Some(*start),
                None => resolved.last().map(|(start, width)| start + width),
            };
            if let Some(edge) = edge {
                let thickness = painter.metric(MetricRole::FocusWidth);
                // Kept inside the table so the first and last boundaries show.
                let x = (edge - shared.offset.get() - thickness / 2.0)
                    .clamp(0.0, (size.width() - thickness).max(0.0));
                painter.fill_rect(
                    Rect::new(x, 0.0, thickness.min(size.width()), size.height())?,
                    painter.color(ColorRole::Primary),
                );
            }
        }
        if self.overflows() {
            let thumb = self.bar()?.thumb;
            let active = self.bar_hover || matches!(self.gesture, Some(HeaderGesture::Scroll(_)));
            painter.fill_rect(thumb, scrollbar::thumb_color(painter, active));
        }
        Ok(())
    }
    // The body is the Tab stop. With one lane it leaves the cross arrows
    // unhandled, and they arrive here to move the active cell.
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.state.deliver_edits(cx)
    }
    fn visibility_changed(&mut self, visible: bool) {
        // No edit survives hiding; the plain cell returns at the next delivery.
        if !visible {
            self.state.0.shared.editing.close_quietly(true);
        }
    }
    fn unmounted(&mut self) {
        // The controller may outlive the table and serve another one, which
        // brings its own options; the callback goes, so a capture of the
        // controller in it cannot keep the controller alive.
        let editing = &self.state.0.shared.editing;
        editing.close_quietly(false);
        editing.ends.clear();
        editing.stale.set(false);
        let editor = editing.editor.borrow_mut().take();
        drop(editor);
        editing.cell_stops.set(true);
        editing.pending.borrow_mut().take();
        editing.active_cells.borrow_mut().clear();
        *editing.notifier.borrow_mut() = None;
        *editing.body.borrow_mut() = None;
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if let Some(outcome) = self.edit_key(cx, event, control, keys)? {
            return Ok(outcome);
        }
        // Escape ends a header gesture, which the runtime's own Escape handling
        // does not reach: the pointer capture is this component's while the
        // focus, and so the context Escape cancels, is the body's.
        if control == KeyCode::Escape
            && event.transition == KeyTransition::Pressed
            && let Some(gesture) = self.gesture.take()
        {
            // A resized column keeps the width it has; a pressed title neither
            // sorts nor becomes a drag.
            self.resize_original = None;
            self.state.0.shared.resizing.set(false);
            cx.release_pointer();
            if matches!(gesture, HeaderGesture::Scroll(_)) {
                // The drag's emphasis ends; as on release, the bar stays
                // emphasized only if the stationary pointer is still over it.
                let over = cx
                    .pointer_position()
                    .filter(|_| cx.pointer_inside())
                    .and_then(|point| cx.to_local(point))
                    .is_some_and(|point| self.hit_test_foreground(point));
                self.set_bar_hover(cx, over);
                cx.invalidate(Invalidation::Paint);
            }
            return Ok(InputOutcome::Handled);
        }
        // The keyboard equivalent of pressing the active cell's title: Alt
        // with Up or Down asks for that direction. A repeat asks nothing again.
        let direction = match control {
            KeyCode::ArrowUp => Some(SortDirection::Ascending),
            KeyCode::ArrowDown => Some(SortDirection::Descending),
            _ => None,
        };
        if let Some(direction) = direction {
            if !crate::activation::alt_only(keys.chord_modifiers(event)) {
                return Ok(InputOutcome::Ignored);
            }
            let Some(key) = self.state.0.shared.active_column.get()? else {
                return Ok(InputOutcome::Ignored);
            };
            let sortable = self
                .state
                .0
                .shared
                .column_index(&key)
                .is_some_and(|column| self.state.0.shared.columns.borrow()[column].sortable);
            if !sortable {
                return Ok(InputOutcome::Ignored);
            }
            if event.transition == KeyTransition::Pressed {
                self.request_sort(cx, key, direction)?;
            }
            return Ok(InputOutcome::Handled);
        }
        let right = match control {
            KeyCode::ArrowRight => true,
            KeyCode::ArrowLeft => false,
            _ => return Ok(InputOutcome::Ignored),
        };
        if !matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        // The keyboard equivalent of dragging the active cell's column.
        if crate::activation::alt_only(keys.chord_modifiers(event)) {
            if !self.reorder_allowed {
                return Ok(InputOutcome::Ignored);
            }
            self.move_active_column(cx, right)?;
            return Ok(InputOutcome::Handled);
        }
        // Other modified chords are left to command bindings.
        if keys.chord_modifiers(event) != crate::Modifiers::default() {
            return Ok(InputOutcome::Ignored);
        }
        self.move_column(cx, right)?;
        Ok(InputOutcome::Handled)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let columns = self.state.0.shared.columns.borrow();
        details
            .properties
            .insert("columns".into(), InspectValue::Number(columns.len() as f64));
        details.properties.insert(
            "resize_allowed".into(),
            InspectValue::Bool(self.resize_allowed),
        );
        details.properties.insert(
            "reorder_allowed".into(),
            InspectValue::Bool(self.reorder_allowed),
        );
        details.properties.insert(
            "scroll_x".into(),
            InspectValue::Number(f64::from(self.state.0.shared.offset.get())),
        );
        if let Some((_, boundary)) = self.drop_boundary() {
            details.properties.insert(
                "drop_boundary".into(),
                InspectValue::Number(boundary as f64),
            );
        }
        if let Ok(Some(column)) = self.state.0.shared.active_column.get() {
            details
                .properties
                .insert("active_column".into(), InspectValue::Text(column));
        }
        details.properties.insert(
            "column_widths".into(),
            InspectValue::List(
                self.state
                    .0
                    .shared
                    .resolved
                    .borrow()
                    .iter()
                    .map(|(_, width)| InspectValue::Number(f64::from(*width)))
                    .collect(),
            ),
        );
        // The headers are rows of this one component, so they are items; the
        // selected one is the sorted column. The rows are the body's to describe,
        // and the body, which has no label of its own, takes the table's name.
        // Each title's rectangle is the one painting fills and `column_at`
        // finds it in.
        let resolved = self.state.0.shared.resolved.borrow();
        let offset = self.state.0.shared.offset.get();
        let header = self.header.get();
        details.semantics = Some(
            Semantics::new(SemanticRole::Table)
                .names_children()
                .value(SemanticValue::Number {
                    value: self.rows.get() as f64,
                    minimum: Some(0.0),
                    maximum: None,
                })
                .items(columns.iter().enumerate().map(|(index, column)| {
                    let item = SemanticItem::new(column.title.as_str()).selected(
                        self.sort
                            .as_ref()
                            .is_some_and(|sort| sort.column == column.key),
                    );
                    let item = if column.sortable {
                        item.action(SemanticAction::Select)
                    } else {
                        item
                    };
                    let item = if self.reorder_allowed && column.movable {
                        item.action(SemanticAction::Move)
                    } else {
                        item
                    };
                    let item = if self.resize_allowed && column.resizable {
                        item.action(SemanticAction::Resize)
                    } else {
                        item
                    };
                    match resolved.get(index).and_then(|(start, width)| {
                        Rect::new(start - offset, 0.0, *width, header).ok()
                    }) {
                        Some(rect) => item.local_bounds(rect),
                        None => item,
                    }
                }))
                .enabled_actions(self.overflows().then_some(SemanticAction::Scroll)),
        );
        details
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Detail, InspectValue, InspectionQuery, Length, ListSelection, RestoreRejection, Runtime,
        SelectionMode, Selector, Viewport, WidgetId,
    };

    thread_local! {
        /// Makes the next republication fail, as a rejected signal write
        /// would; a test's signal borrow cannot reach the controller's items.
        pub(super) static FAIL_REPUBLISH: Cell<bool> = const { Cell::new(false) };
    }

    struct Rows;
    impl TableItems for Rows {
        fn len(&self) -> usize {
            3
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < 3).then(|| format!("r{index}"))
        }
        fn cell(&self, _: usize, column: &str) -> Option<Element> {
            Some(Element::new("text", crate::Label::new(column)))
        }
    }

    /// Three movable 200 px columns in a 300 px table.
    fn mounted() -> Result<(Runtime, TableController, WidgetId), Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
        let controller = TableController::new(
            &ui,
            ListSelection::new(&ui, SelectionMode::Single),
            ["a", "b", "c"].map(|key| {
                TableColumn::new(key, key.to_uppercase())
                    .width(Length::new(200.0).unwrap())
                    .movable(true)
            }),
            TableSource::new(Rows),
        )?;
        let table = ui.mount_tree(Element::new("table", Table::new(&controller, 24.0)?))?;
        ui.flush(ui.now())?;
        Ok((ui, controller, table))
    }

    /// The scroll offset, the column widths and the title bounds inspection
    /// reports.
    fn presented(
        ui: &Runtime,
        table: &WidgetId,
    ) -> (InspectValue, InspectValue, Vec<Option<[f32; 4]>>) {
        let id = ui.inspection_id(table).unwrap();
        let mut record = ui
            .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
            .components
            .remove(0);
        let property = |name: &str| record.details.properties[name].clone();
        let (offset, widths) = (property("scroll_x"), property("column_widths"));
        let bounds = record
            .details
            .semantics
            .take()
            .map(|semantics| {
                semantics
                    .items
                    .into_iter()
                    .map(|item| item.bounds)
                    .collect()
            })
            .unwrap_or_default();
        (offset, widths, bounds)
    }

    #[test]
    fn a_rejected_restore_keeps_the_resolved_columns_and_the_offset()
    -> Result<(), Box<dyn std::error::Error>> {
        // An entry that reorders the columns and narrows two of them, so the
        // scroll range it would give is shorter than the offset below.
        let (mut source, controller, _) = mounted()?;
        controller.0.shared.move_to(0, 3)?;
        controller.set_column_width("b", Length::new(100.0)?)?;
        controller.set_column_width("c", Length::new(100.0)?)?;
        source.flush(source.now())?;
        let entry = source.capture_workspace()?;

        // Scrolled to a nonzero offset, with a width change waiting for
        // layout; the twin gets the same and no restore.
        let prepare = |ui: &mut Runtime, controller: &TableController| {
            controller.0.shared.scroll_to(250.0)?;
            ui.flush(ui.now())?;
            controller.set_column_width("a", Length::new(300.0)?)
        };
        let (mut ui, controller, table) = mounted()?;
        prepare(&mut ui, &controller)?;
        let (mut twin, twin_controller, twin_table) = mounted()?;
        prepare(&mut twin, &twin_controller)?;

        FAIL_REPUBLISH.with(|fail| fail.set(true));
        let report = ui.restore_workspace(&entry);
        FAIL_REPUBLISH.with(|fail| fail.set(false));
        let report = report?;
        assert!(
            report
                .rejected()
                .any(|(_, reason)| matches!(reason, RestoreRejection::Runtime(_))),
            "{report:?}"
        );
        assert_eq!(presented(&ui, &table), presented(&twin, &twin_table));
        ui.flush(ui.now())?;
        twin.flush(twin.now())?;
        assert_eq!(presented(&ui, &table), presented(&twin, &twin_table));
        assert_eq!(
            presented(&ui, &table).0,
            InspectValue::Number(250.0),
            "the offset was kept"
        );
        Ok(())
    }
}
