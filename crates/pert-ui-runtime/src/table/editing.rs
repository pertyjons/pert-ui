//! In-place cell editing: the edit a table keeps open, the host component in
//! the edited cell and the rules that begin, end and move an edit.
use super::{Shared, TableController};
use crate::{
    ArrangeContext, Component, Constraints, EditorCompletion, EditorOutcome, EditorSession,
    EditorStart, EditorTrigger, Element, EventContext, ListItems, MeasureContext, Notifier, Rect,
    RuntimeError, Signal, Size, UpdateContext, WidgetId,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) type CellEditorFn = Box<dyn FnMut(&mut CellEdit<'_>) -> Result<Element, RuntimeError>>;

/// The key of the host component in an edited cell.
pub(super) const HOST_KEY: &str = "cell editor";

/// One cell edit as the editor callback of [`Table::cell_editor`](super::Table::cell_editor)
/// sees it: which cell, how the edit began, the session to pass to the
/// editor, and the state owner for the editor's signals.
///
/// The callback runs once per edit, from the update of the component the
/// table puts in the edited cell. Signals created through [`Self::owner`]
/// are ordinary signals of the runtime: the editor's handles to them go when
/// the edit closes and its editor unmounts, while clones the application
/// keeps elsewhere live on as any signal does.
#[non_exhaustive]
pub struct CellEdit<'a> {
    row: &'a str,
    column: &'a str,
    start: &'a EditorStart,
    session: &'a EditorSession,
    owner: &'a mut UpdateContext,
}
impl CellEdit<'_> {
    /// The edited row's key.
    #[must_use]
    pub fn row(&self) -> &str {
        self.row
    }
    /// The edited column's key.
    #[must_use]
    pub fn column(&self) -> &str {
        self.column
    }
    /// How the edit began: [`EditorStart::SelectAll`] from Enter or Tab,
    /// [`EditorStart::CaretAtEnd`] from F2 or a double-click, and
    /// [`EditorStart::Typed`] from text typed on the cell. The editor applies
    /// it through its session; a [`Select`](crate::Select) ignores it.
    #[must_use]
    pub fn start(&self) -> &EditorStart {
        self.start
    }
    /// The edit's session: pass it to the editor's `session` builder, such as
    /// [`TextField::session`](crate::TextField::session), so the table learns
    /// how the edit ended.
    #[must_use]
    pub fn session(&self) -> &EditorSession {
        self.session
    }
    /// The state owner for the editor's signals and choice controls: the
    /// update context of the component hosting the editor. See the type's
    /// documentation for how long the signals live.
    pub fn owner(&mut self) -> &mut UpdateContext {
        self.owner
    }
}

/// The open edit, which the table owns rather than the cell.
pub(super) struct OpenEdit {
    serial: u64,
    pub(super) row: String,
    pub(super) column: String,
    start: EditorStart,
    session: EditorSession,
    /// The host in the edited cell, once it has updated.
    host: Option<WidgetId>,
    /// Whether keyboard focus is in the editor.
    focused: bool,
    /// The editor callback failed: the edit closes at the table's delivery.
    failed: bool,
}

/// The table's editing state, shared by the table, its rows and the host.
pub(super) struct Editing {
    pub(super) editor: RefCell<Option<CellEditorFn>>,
    pub(super) open: RefCell<Option<OpenEdit>>,
    serial: Cell<u64>,
    /// The edited row's key, which the body keeps mounted.
    pub(super) row: Signal<Option<String>>,
    /// The mounted table's notifier, which sessions report through.
    pub(super) notifier: RefCell<Option<Notifier>>,
    /// An edit a row asked for (a double-click), begun at the table's delivery.
    pub(super) pending: RefCell<Option<(String, String, EditorStart)>>,
    /// The mounted body, which takes focus when an edit ends.
    pub(super) body: RefCell<Option<WidgetId>>,
    /// Whether cell content is a Tab stop ([`Table::cell_tab_stops`](super::Table::cell_tab_stops)).
    pub(super) cell_stops: Cell<bool>,
    /// The cells of the active row, by column key, for F2 into resting content.
    pub(super) active_cells: RefCell<Vec<(String, WidgetId)>>,
    /// An edit closed where the rows could not be published again (hiding);
    /// the table's next delivery publishes them.
    pub(super) stale: Cell<bool>,
}
impl Editing {
    pub(super) fn new(ui: &impl crate::StateOwner) -> Self {
        Self {
            editor: RefCell::new(None),
            open: RefCell::new(None),
            serial: Cell::new(0),
            row: ui.signal(None),
            notifier: RefCell::new(None),
            pending: RefCell::new(None),
            body: RefCell::new(None),
            cell_stops: Cell::new(true),
            active_cells: RefCell::new(Vec::new()),
            stale: Cell::new(false),
        }
    }
    /// The host for the cell of `row` and `column` while it is edited.
    pub(super) fn host(shared: &Rc<Shared>, row: &str, column: &str) -> Option<Element> {
        let open = shared.editing.open.borrow();
        let open = open
            .as_ref()
            .filter(|open| open.row == row && open.column == column)?;
        Some(
            Element::new(
                HOST_KEY,
                EditHost {
                    shared: shared.clone(),
                    serial: open.serial,
                    built: false,
                    child: None,
                    focus_requested: false,
                    notifier: None,
                },
            )
            .structural(),
        )
    }
    /// Asks the table to begin an edit at its next delivery.
    pub(super) fn request(&self, row: String, column: String, start: EditorStart) {
        *self.pending.borrow_mut() = Some((row, column, start));
        if let Some(notifier) = &*self.notifier.borrow() {
            notifier.request();
        }
    }
    /// Closes the open edit without publishing the rows, where signal writes
    /// are not allowed (hiding, unmounting): the host drops its editor and
    /// the table's next delivery restores the plain cell.
    pub(super) fn close_quietly(&self) {
        if let Some(open) = self.open.borrow_mut().take() {
            open.session.end();
            self.row.publish_trusted(None);
            self.stale.set(true);
            if let Some(notifier) = &*self.notifier.borrow() {
                notifier.request();
            }
        }
    }
    /// Whether an edit is open whose editor does not hold focus.
    pub(super) fn open_unfocused(&self) -> bool {
        self.open
            .borrow()
            .as_ref()
            .is_some_and(|open| !open.focused)
    }
    /// Whether an edit is open on a row the published rows no longer hold.
    pub(super) fn row_gone(&self, items: &dyn ListItems) -> bool {
        self.open
            .borrow()
            .as_ref()
            .is_some_and(|open| open.failed || items.index_of(&open.row).is_none())
    }
}

impl TableController {
    /// Whether the cell of `row` and `column` can be edited now: the table has
    /// an editor, the column is editable, and the source says the cell is.
    pub(super) fn editable_cell(&self, row: &str, column: &str) -> bool {
        let shared = &self.0.shared;
        if shared.editing.editor.borrow().is_none()
            || !shared
                .columns
                .borrow()
                .iter()
                .any(|candidate| candidate.key == column && candidate.editable)
        {
            return false;
        }
        let source = self.0.source.borrow();
        source
            .0
            .index_of(row)
            .is_some_and(|index| source.0.editable(index, column))
    }
    /// Begins an edit of an eligible cell, making it active; returns whether
    /// the cell was eligible. The cell whose edit is open focuses its editor
    /// again; any other open edit closes first, its draft discarded.
    pub(super) fn begin_edit(
        &self,
        cx: &EventContext,
        row: &str,
        column: &str,
        start: EditorStart,
    ) -> Result<bool, RuntimeError> {
        // The open edit's cell focuses its editor again whatever the source
        // now says: eligibility governs new edits, not the open one.
        let editing = &self.0.shared.editing;
        if let Some(open) = editing
            .open
            .borrow()
            .as_ref()
            .filter(|open| open.row == row && open.column == column && !open.failed)
        {
            if let Some(host) = &open.host {
                cx.request_focus_within(host)?;
            }
            return Ok(true);
        }
        if !self.editable_cell(row, column) {
            return Ok(false);
        }
        let Some(notifier) = editing.notifier.borrow().clone() else {
            return Ok(false);
        };
        if let Some(old) = editing.open.borrow_mut().take() {
            old.session.end();
        }
        let session = EditorSession::new(notifier);
        session.begin(start.clone());
        let serial = editing.serial.get().wrapping_add(1);
        editing.serial.set(serial);
        *editing.open.borrow_mut() = Some(OpenEdit {
            serial,
            row: row.to_owned(),
            column: column.to_owned(),
            start,
            session,
            host: None,
            focused: false,
            failed: false,
        });
        editing.row.set(Some(row.to_owned()))?;
        self.make_active(row, column)?;
        self.republish()?;
        Ok(true)
    }
    /// Closes the open edit and restores the plain cell; returns it.
    fn close_edit(&self) -> Result<Option<OpenEdit>, RuntimeError> {
        let editing = &self.0.shared.editing;
        let Some(open) = editing.open.borrow_mut().take() else {
            return Ok(None);
        };
        open.session.end();
        editing.row.set(None)?;
        self.republish()?;
        Ok(Some(open))
    }
    fn make_active(&self, row: &str, column: &str) -> Result<(), RuntimeError> {
        if self.0.selection.active().get()?.as_deref() != Some(row) {
            self.0.selection.set_active(Some(row.to_owned()))?;
        }
        if self.0.shared.active_column.get()?.as_deref() != Some(column) {
            self.0.shared.active_column.set(Some(column.to_owned()))?;
        }
        Ok(())
    }
    /// Cancels the open edit from Escape on the body, when its editor does
    /// not hold focus.
    pub(super) fn cancel_unfocused(&self) -> Result<bool, RuntimeError> {
        if !self.0.shared.editing.open_unfocused() {
            return Ok(false);
        }
        self.close_edit()?;
        Ok(true)
    }
    /// The table's delivery: a begin a row asked for, an edit whose row left
    /// or whose editor failed, and the session's completions.
    pub(super) fn deliver_edits(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        let editing = &self.0.shared.editing;
        if editing.stale.replace(false) {
            editing.row.set(None)?;
            self.republish()?;
        }
        if let Some((row, column, start)) = editing.pending.borrow_mut().take() {
            self.begin_edit(cx, &row, &column, start)?;
        }
        let items = self.0.items.get()?;
        if editing.row_gone(&*items) {
            let focused = editing
                .open
                .borrow()
                .as_ref()
                .is_some_and(|open| open.focused);
            self.close_edit()?;
            if focused {
                self.focus_body(cx)?;
            }
        }
        loop {
            let next = {
                let open = editing.open.borrow();
                let Some(open) = open.as_ref() else {
                    break;
                };
                let Some(done) = open.session.take_completion() else {
                    break;
                };
                (done, open.row.clone(), open.column.clone(), open.focused)
            };
            self.complete(cx, next)?;
        }
        Ok(())
    }
    fn complete(
        &self,
        cx: &EventContext,
        (done, row, column, focused): (EditorCompletion, String, String, bool),
    ) -> Result<(), RuntimeError> {
        if !done.outcome.is_terminal() {
            return Ok(());
        }
        self.close_edit()?;
        let present = self.0.items.get()?.index_of(&row).is_some();
        // A report delivered after focus left the editor never moves focus.
        if !focused || !present {
            if focused {
                self.focus_body(cx)?;
            }
            return Ok(());
        }
        match (done.trigger, done.outcome) {
            (EditorTrigger::FocusLeft, _) => {}
            (EditorTrigger::Tab { reverse }, EditorOutcome::Saved | EditorOutcome::Unchanged) => {
                match self.next_cell(&row, &column, reverse)? {
                    Some((row, column)) => {
                        self.begin_edit(cx, &row, &column, EditorStart::SelectAll)?;
                    }
                    None => {
                        self.make_active(&row, &column)?;
                        self.focus_body(cx)?;
                    }
                }
            }
            _ => {
                self.make_active(&row, &column)?;
                self.focus_body(cx)?;
            }
        }
        Ok(())
    }
    fn focus_body(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        match &*self.0.shared.editing.body.borrow() {
            Some(body) => cx.request_focus_for(body),
            None => Ok(()),
        }
    }
    /// The next or previous eligible cell after `row` and `column`, in the
    /// current column and row order: along the row, then from the first or
    /// last column of each following or preceding row.
    fn next_cell(
        &self,
        row: &str,
        column: &str,
        reverse: bool,
    ) -> Result<Option<(String, String)>, RuntimeError> {
        let columns: Vec<String> = self
            .0
            .shared
            .columns
            .borrow()
            .iter()
            .filter(|column| column.editable)
            .map(|column| column.key.clone())
            .collect();
        let items = self.0.items.get()?;
        let (Some(mut index), Some(position)) =
            (items.index_of(row), self.0.shared.column_index(column))
        else {
            return Ok(None);
        };
        // Editable columns after (or before) the edited one, in column order.
        let order = self.0.shared.columns.borrow();
        let mut candidates: Vec<&String> = if reverse {
            columns
                .iter()
                .rev()
                .filter(|key| order.iter().position(|c| &c.key == *key) < Some(position))
                .collect()
        } else {
            columns
                .iter()
                .filter(|key| order.iter().position(|c| &c.key == *key) > Some(position))
                .collect()
        };
        drop(order);
        let mut key = row.to_owned();
        loop {
            for candidate in &candidates {
                if self.editable_cell(&key, candidate) {
                    return Ok(Some((key, (*candidate).clone())));
                }
            }
            let next = if reverse {
                index.checked_sub(1)
            } else {
                index.checked_add(1).filter(|next| *next < items.len())
            };
            let Some(next) = next else {
                return Ok(None);
            };
            index = next;
            let Some(next_key) = items.key(index) else {
                return Ok(None);
            };
            key = next_key;
            candidates = if reverse {
                columns.iter().rev().collect()
            } else {
                columns.iter().collect()
            };
        }
    }
}

/// The component in the edited cell: builds the editor once, focuses it,
/// and tells the table whether focus is in it.
struct EditHost {
    shared: Rc<Shared>,
    serial: u64,
    built: bool,
    child: Option<WidgetId>,
    focus_requested: bool,
    notifier: Option<Notifier>,
}
impl EditHost {
    fn current(&self) -> bool {
        self.shared
            .editing
            .open
            .borrow()
            .as_ref()
            .is_some_and(|open| open.serial == self.serial)
    }
}
impl Component for EditHost {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let editing = &self.shared.editing;
        if !self.current() {
            // The edit closed where the rows were not published again.
            if !cx.children().is_empty() {
                cx.request_children(Vec::new());
            }
            self.child = None;
            return Ok(());
        }
        if !self.built {
            // One attempt per edit: a failure is not retried.
            self.built = true;
            let (row, column, start, session) = {
                let mut open = editing.open.borrow_mut();
                let Some(open) = open.as_mut() else {
                    return Ok(());
                };
                open.host = Some(cx.id());
                (
                    open.row.clone(),
                    open.column.clone(),
                    open.start.clone(),
                    open.session.clone(),
                )
            };
            // Taken out while application code runs, so it cannot reenter.
            let editor = editing.editor.borrow_mut().take();
            let Some(mut editor) = editor else {
                return Ok(());
            };
            let built = editor(&mut CellEdit {
                row: &row,
                column: &column,
                start: &start,
                session: &session,
                owner: cx,
            });
            editing.editor.borrow_mut().get_or_insert(editor);
            match built {
                Ok(element) => cx.request_children(vec![element]),
                Err(error) => {
                    if let Some(open) = editing.open.borrow_mut().as_mut() {
                        open.failed = true;
                    }
                    if let Some(notifier) = &*editing.notifier.borrow() {
                        notifier.request();
                    }
                    return Err(error);
                }
            }
        } else if !self.focus_requested
            && let Some(child) = cx.children().first()
        {
            self.focus_requested = true;
            self.child = Some(child.id().clone());
            self.notifier.get_or_insert_with(|| cx.notifier()).request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        match &self.child {
            Some(child) if self.current() => cx.request_focus_within(child),
            _ => Ok(()),
        }
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if let Some(open) = self
            .shared
            .editing
            .open
            .borrow_mut()
            .as_mut()
            .filter(|open| open.serial == self.serial)
        {
            open.focused = within;
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::new(0.0, 0.0)?);
        }
        cx.measure(0, space)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        Ok(())
    }
}
