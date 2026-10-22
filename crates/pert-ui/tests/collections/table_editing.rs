//! In-place cell editing in a table: how edits begin, end and move, and how
//! long they live, on both key paths.
use crate::support;
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const ROW: f32 = 24.0;

/// The `(row, column)` of each `on_activate` call.
type Activations = Rc<RefCell<Vec<(String, Option<String>)>>>;

#[derive(Clone)]
struct Item {
    key: String,
    name: Signal<String>,
    quantity: Signal<i64>,
    kind: Signal<&'static str>,
    done: Signal<bool>,
    locked: bool,
}

/// The application's rows in its order.
struct Rows(Vec<Item>);
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|item| item.key.clone())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let item = self.0.get(index)?;
        Some(match column {
            "name" => Element::new(
                // A key free for the application, even the one the table
                // gives the editor's host.
                "cell editor",
                Label::new(item.name.get().ok()?),
            ),
            "quantity" => Element::new("text", Label::new(item.quantity.get().ok()?.to_string())),
            "kind" => Element::new("text", Label::new(item.kind.get().ok()?)),
            "done" => Element::new("check", Checkbox::new(item.done.clone(), "Done")),
            _ => return None,
        })
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        (column == "name").then(|| self.0.get(index)?.name.get().ok())?
    }
    fn editable(&self, index: usize, _: &str) -> bool {
        self.0.get(index).is_some_and(|item| !item.locked)
    }
}

/// What the name editor's save does besides writing the name.
#[derive(Clone, Copy, PartialEq)]
enum OnSave {
    Nothing,
    /// Sorts the rows by name.
    Sort,
    /// Removes the saved row.
    Remove,
}

/// `count` items; the second cannot be edited.
fn make_items(ui: &Runtime, count: usize) -> Vec<Item> {
    let names = ["Cable", "Mouse", "Tape", "Lamp", "Desk", "Pen"];
    let keys = ["a", "b", "c"];
    (0..count)
        .map(|index| Item {
            key: keys
                .get(index)
                .map_or_else(|| format!("r{index}"), |key| (*key).to_owned()),
            name: ui.signal(names[index % names.len()].to_owned()),
            quantity: ui.signal(10 * (index as i64 + 1)),
            kind: ui.signal("hardware"),
            done: ui.signal(false),
            locked: index == 1,
        })
        .collect()
}
fn controller(ui: &Runtime, items: &[Item]) -> TableController {
    TableController::new(
        ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("name", "Name").editable(true),
            TableColumn::new("quantity", "Quantity").editable(true),
            TableColumn::new("done", "Done"),
            TableColumn::new("kind", "Kind").editable(true),
            TableColumn::new("note", "Note"),
        ],
        TableSource::new(Rows(items.to_vec())),
    )
    .unwrap()
}

struct Options {
    rows: usize,
    on_save: OnSave,
    cell_stops: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            rows: 3,
            on_save: OnSave::Nothing,
            cell_stops: true,
        }
    }
}

struct Fixture {
    ui: Runtime,
    root: WidgetId,
    state: TableController,
    items: Rc<RefCell<Vec<Item>>>,
    activations: Activations,
    fail: Rc<Cell<bool>>,
    shown: Signal<bool>,
    /// The name editor's editing permission.
    name_editable: Signal<bool>,
    /// The name editor's enablement.
    name_enabled: Signal<bool>,
}
impl Fixture {
    fn new(options: Options) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 160.0, 1.0).unwrap())
            .unwrap();
        let items = make_items(&ui, options.rows);
        let state = controller(&ui, &items);
        let items = Rc::new(RefCell::new(items));
        let activations = Activations::default();
        let fail = Rc::new(Cell::new(false));
        let (log, rows, publish, failing) = (
            activations.clone(),
            items.clone(),
            state.clone(),
            fail.clone(),
        );
        let on_save = options.on_save;
        let (name_editable, name_enabled) = (ui.signal(true), ui.signal(true));
        let (editable, enabled) = (name_editable.read_only(), name_enabled.read_only());
        let table = Table::new(&state, ROW)
            .unwrap()
            .cell_tab_stops(options.cell_stops)
            .on_activate(move |_, row, column| {
                log.borrow_mut()
                    .push((row.to_owned(), column.map(str::to_owned)));
                Ok(())
            })
            .cell_editor(move |edit| {
                if failing.get() {
                    return Err(RuntimeError::Exhausted);
                }
                let item = rows
                    .borrow()
                    .iter()
                    .find(|item| item.key == edit.row())
                    .cloned()
                    .ok_or(RuntimeError::Exhausted)?;
                Ok(match edit.column() {
                    "name" => {
                        let (rows, publish, key) =
                            (rows.clone(), publish.clone(), item.key.clone());
                        Element::new(
                            "editor",
                            TextField::new(item.name.clone())
                                .normalize(|text: String| {
                                    if text.trim().is_empty() {
                                        Err(InputRejection::new("A name is required"))
                                    } else {
                                        Ok(text)
                                    }
                                })
                                .on_save(move |_, _| {
                                    let mut rows = rows.borrow_mut();
                                    match on_save {
                                        OnSave::Nothing => return Ok(()),
                                        OnSave::Sort => rows.sort_by_key(|item| {
                                            item.name.get().unwrap_or_default()
                                        }),
                                        OnSave::Remove => rows.retain(|item| item.key != key),
                                    }
                                    publish.set_source(TableSource::new(Rows(rows.clone())))
                                })
                                .editable(editable.clone())
                                .enabled(enabled.clone())
                                .session(edit.session()),
                        )
                    }
                    "quantity" => Element::new(
                        "editor",
                        IntegerInput::new(item.quantity.clone())
                            .range(0, 1000)
                            .unwrap()
                            .session(edit.session()),
                    ),
                    _ => Element::new(
                        "editor",
                        Select::new(
                            edit.owner(),
                            item.kind.clone(),
                            [
                                Choice::new("hardware", "Hardware"),
                                Choice::new("software", "Software"),
                            ],
                        )
                        .unwrap()
                        .session(edit.session()),
                    ),
                })
            });
        let shown = ui.signal(true);
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new([
                    Element::new(
                        "view",
                        View::new(Element::new("table", table)).visible_signal(shown.clone()),
                    )
                    .with_grow(1),
                    Element::new("other", Button::new("Other", |_| Ok(()))),
                ]),
            ))
            .unwrap();
        let mut fixture = Self {
            ui,
            root,
            state,
            items,
            activations,
            fail,
            shown,
            name_editable,
            name_enabled,
        };
        fixture.settle();
        fixture
    }
    fn settle(&mut self) {
        for _ in 0..3 {
            support::frame(&mut self.ui);
        }
    }
    fn widget(&self, key: &str) -> Option<WidgetId> {
        self.ui.find_descendant(&self.root, key).unwrap()
    }
    fn body(&self) -> WidgetId {
        self.widget("body").unwrap()
    }
    /// Makes the cell active and focuses the body.
    fn activate(&mut self, row: &str, column: &str) {
        self.state
            .selection()
            .set_active(Some(row.to_owned()))
            .unwrap();
        self.state
            .set_active_column(Some(column.to_owned()))
            .unwrap();
        let body = self.body();
        self.ui.focus(&body).unwrap();
        self.settle();
    }
    fn active(&self) -> (Option<String>, Option<String>) {
        (
            self.state.selection().active().get().unwrap(),
            self.state.active_column().get().unwrap(),
        )
    }
    /// The host the table puts in the edited cell, with the cell's row and
    /// column. The host is the cell's own child; application content keyed
    /// like it sits below a wrapper.
    fn host(&self) -> Option<(WidgetId, String, String)> {
        for row in self.ui.children(&self.body()).unwrap() {
            for cell in self.ui.children(&row).unwrap() {
                let host = self
                    .ui
                    .children(&cell)
                    .unwrap()
                    .into_iter()
                    .find(|child| self.ui.key(child).unwrap() == "cell editor");
                if let Some(host) = host {
                    return Some((
                        host,
                        self.ui.key(&row).unwrap().to_owned(),
                        self.ui.key(&cell).unwrap().to_owned(),
                    ));
                }
            }
        }
        None
    }
    /// The edited cell as `(row, column)`.
    fn edited(&self) -> Option<(String, String)> {
        self.host().map(|(_, row, column)| (row, column))
    }
    fn focused(&self) -> Option<String> {
        self.ui
            .focused()
            .map(|id| self.ui.key(&id).unwrap().to_owned())
    }
    /// Whether focus is in the edited cell's editor.
    fn editor_focused(&self) -> bool {
        let (Some((host, _, _)), Some(focused)) = (self.host(), self.ui.focused()) else {
            return false;
        };
        let mut pending = vec![host];
        while let Some(id) = pending.pop() {
            if id == focused {
                return true;
            }
            pending.extend(self.ui.children(&id).unwrap());
        }
        false
    }
    fn text(&self) -> String {
        let focused = self.ui.focused().unwrap();
        let id = self.ui.inspection_id(&focused).unwrap();
        match &self
            .ui
            .inspect()
            .components
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .details
            .properties["text"]
        {
            InspectValue::Text(text) => text.clone(),
            other => panic!("not text: {other:?}"),
        }
    }
    /// One key transition through `key_input`, or with `native` through the
    /// command path.
    fn send(&mut self, code: KeyCode, action: KeyAction, native: bool) -> Result<(), RuntimeError> {
        if native {
            self.ui
                .command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
                .map(|_| ())
        } else {
            self.ui.key_input(code, action).map(|_| ())
        }
    }
    fn key(&mut self, code: KeyCode, shift: bool, native: bool) {
        if shift {
            self.send(KeyCode::ShiftLeft, KeyAction::Press, native)
                .unwrap();
        }
        self.send(code, KeyAction::Press, native).unwrap();
        self.send(code, KeyAction::Release, native).unwrap();
        if shift {
            self.send(KeyCode::ShiftLeft, KeyAction::Release, native)
                .unwrap();
        }
        self.settle();
    }
    /// A property of the cell's editor as inspection reports it.
    fn editor_property(&self, name: &str) -> InspectValue {
        let editor = self.widget("editor").expect("an editor is mounted");
        let id = self.ui.inspection_id(&editor).unwrap();
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .details
            .properties[name]
            .clone()
    }
    fn type_text(&mut self, text: &str) {
        self.ui.text_input(text).unwrap();
        self.settle();
    }
    fn name(&self, row: &str) -> String {
        self.item(row).name.get().unwrap()
    }
    fn item(&self, row: &str) -> Item {
        self.items
            .borrow()
            .iter()
            .find(|item| item.key == row)
            .cloned()
            .unwrap()
    }
    /// The middle of a cell, in window coordinates.
    fn cell_point(&self, row: &str, column: &str) -> Point {
        let row = self.ui.find_descendant(&self.body(), row).unwrap().unwrap();
        let cell = self.ui.find_descendant(&row, column).unwrap().unwrap();
        let [x, y, w, h] = self.ui.bounds(&cell).unwrap().unwrap().values();
        Point::new(x + w / 2.0, y + h / 2.0).unwrap()
    }
    fn click(&mut self, point: Point) {
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(point, kind))
                .unwrap();
        }
        self.settle();
    }
}

#[test]
fn enter_f2_and_typing_begin_with_their_starts() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "native {native}"
        );
        assert!(f.editor_focused());
        f.type_text("X");
        assert_eq!(f.text(), "X", "Enter selects the value");
        f.key(KeyCode::Escape, false, native);

        f.key(KeyCode::F2, false, native);
        f.type_text("s");
        assert_eq!(f.text(), "Cables", "F2 puts the caret at the end");
        f.key(KeyCode::Escape, false, native);

        f.type_text("Z");
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        assert_eq!(f.text(), "Z", "typed text replaces the value");
        f.key(KeyCode::Escape, false, native);

        f.type_text(" ");
        assert_eq!(f.edited(), None, "whitespace does not begin");
        assert_eq!(f.name("a"), "Cable");
    }
}

#[test]
fn enter_activates_cells_that_are_not_edited_in_place() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "note");
        f.key(KeyCode::Enter, false, native);
        // The locked row is not editable, so Enter activates it too.
        f.activate("b", "name");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(
            *f.activations.borrow(),
            [
                ("a".to_owned(), Some("note".to_owned())),
                ("b".to_owned(), Some("name".to_owned()))
            ],
            "native {native}"
        );
    }
}

#[test]
fn enter_and_escape_return_focus_to_the_body_on_the_edited_cell() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Laptop");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(f.name("a"), "Laptop", "native {native}");
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.active(), (Some("a".into()), Some("name".into())));

        f.key(KeyCode::Enter, false, native);
        f.type_text("Monitor");
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.name("a"), "Laptop", "Escape discards");
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert!(f.activations.borrow().is_empty());
    }
}

#[test]
fn tab_moves_along_rows_skips_what_cannot_be_edited_and_stops_at_the_ends() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Laptop");
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.name("a"), "Laptop", "Tab saves");
        assert_eq!(
            f.edited(),
            Some(("a".into(), "quantity".into())),
            "native {native}"
        );
        assert!(f.editor_focused());
        // The done column is not editable and row b is locked.
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.edited(), Some(("a".into(), "kind".into())));
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.edited(), Some(("c".into(), "name".into())));
        f.key(KeyCode::Tab, true, native);
        assert_eq!(f.edited(), Some(("a".into(), "kind".into())));
        f.key(KeyCode::Tab, false, native);
        f.key(KeyCode::Tab, false, native);
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.edited(), Some(("c".into(), "kind".into())));
        // Past the last cell the body takes focus on the edited cell.
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.active(), (Some("c".into()), Some("kind".into())));
        // Before the first cell too.
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.key(KeyCode::Tab, true, native);
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
    }
}

#[test]
fn focus_leaving_saves_and_closes_without_moving_focus() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
        f.type_text("Laptop");
        let other = f.widget("other").unwrap();
        f.ui.focus(&other).unwrap();
        f.settle();
        assert_eq!(f.name("a"), "Laptop", "native {native}");
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("other"));
    }
}

#[test]
fn a_refused_edit_stays_open_until_focused_again_or_replaced() {
    for native in [false, true] {
        let mut f = Fixture::new(Options {
            rows: 20,
            ..Options::default()
        });
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.key(KeyCode::Backspace, false, native);
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "refused, native {native}"
        );
        assert!(f.editor_focused());
        // Leaving keeps it open, and it stays mounted while scrolled away.
        let other = f.widget("other").unwrap();
        f.ui.focus(&other).unwrap();
        f.settle();
        let _ = f.state.reveal("r19").unwrap();
        f.settle();
        assert!(
            f.ui.find_descendant(&f.body(), "r10").unwrap().is_none(),
            "scrolled away from the top"
        );
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        assert!(!f.editor_focused());
        // Beginning it again focuses the open editor, draft and all.
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "reopened, native {native}");
        assert_eq!(f.text(), "");
        // Beginning another cell discards the refused draft.
        f.activate("c", "name");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(f.edited(), Some(("c".into(), "name".into())));
        assert_eq!(f.name("a"), "Cable");
    }
}

#[test]
fn tab_moves_in_the_order_a_save_callback_left() {
    for native in [false, true] {
        // Renaming a to "Zebra" sorts it last: Shift+Tab then reaches the row
        // before it in the new order, c, not the start of the table.
        let mut f = Fixture::new(Options {
            on_save: OnSave::Sort,
            ..Options::default()
        });
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Zebra");
        f.key(KeyCode::Tab, true, native);
        assert_eq!(f.name("a"), "Zebra");
        assert_eq!(
            f.edited(),
            Some(("c".into(), "kind".into())),
            "native {native}"
        );

        // A save that removes the row closes the edit; the body takes focus.
        let mut f = Fixture::new(Options {
            on_save: OnSave::Remove,
            ..Options::default()
        });
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Gone");
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.focused().as_deref(), Some("body"));
    }
}

#[test]
fn a_row_leaving_the_rows_closes_its_edit() {
    for native in [false, true] {
        // A filter that hides the row discards the draft.
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Laptop");
        assert!(f.editor_focused(), "native {native}");
        f.state.filter("Tape", &FilterMatcher::substring()).unwrap();
        f.settle();
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.name("a"), "Cable");
        assert_eq!(f.focused().as_deref(), Some("body"));

        // So does deleting it.
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
        let rest: Vec<Item> = f.items.borrow()[1..].to_vec();
        f.state.set_source(TableSource::new(Rows(rest))).unwrap();
        f.settle();
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.focused().as_deref(), Some("body"));
    }
}

#[test]
fn sorting_keeps_the_editor_and_its_draft() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("c", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Draft");
        let mut reversed = f.items.borrow().clone();
        reversed.reverse();
        f.state
            .set_source(TableSource::new(Rows(reversed)))
            .unwrap();
        f.settle();
        assert_eq!(
            f.edited(),
            Some(("c".into(), "name".into())),
            "native {native}"
        );
        assert!(f.editor_focused());
        assert_eq!(f.text(), "Draft");
    }
}

#[test]
fn an_editor_callback_error_closes_the_edit() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.fail.set(true);
        f.activate("a", "name");
        let pressed = f.send(KeyCode::Enter, KeyAction::Press, native);
        let released = f.send(KeyCode::Enter, KeyAction::Release, native);
        assert!(
            [pressed, released].contains(&Err(RuntimeError::Exhausted)),
            "the callback's error is returned, native {native}"
        );
        f.settle();
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
        // It is not retried: the next begin builds again.
        f.fail.set(false);
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
    }
}

#[test]
fn a_dropdown_editor_completes_on_a_choice() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "kind");
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
        f.key(KeyCode::ArrowDown, false, native);
        f.key(KeyCode::ArrowDown, false, native);
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.item("a").kind.get().unwrap(),
            "software",
            "native {native}"
        );
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
    }
}

#[test]
fn a_double_click_begins_unless_modified_or_taken_by_the_cell() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        let point = f.cell_point("c", "name");
        f.click(point);
        f.click(point);
        assert_eq!(
            f.edited(),
            Some(("c".into(), "name".into())),
            "native {native}"
        );
        assert!(f.editor_focused());
        f.type_text("s");
        assert_eq!(f.text(), "Tapes", "the caret is at the end");
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.edited(), None, "Escape closed it, native {native}");

        // With a modifier held a double-click only selects.
        f.send(KeyCode::ControlLeft, KeyAction::Press, native)
            .unwrap();
        f.click(point);
        f.click(point);
        f.send(KeyCode::ControlLeft, KeyAction::Release, native)
            .unwrap();
        f.settle();
        assert_eq!(f.edited(), None, "native {native}");

        // A checkbox takes its own presses.
        let check = f.cell_point("a", "done");
        f.click(check);
        f.click(check);
        assert_eq!(f.edited(), None);
        assert!(!f.item("a").done.get().unwrap(), "toggled twice");
    }
}

#[test]
fn with_one_tab_stop_f2_enters_resting_content_and_escape_returns() {
    for native in [false, true] {
        let mut f = Fixture::new(Options {
            cell_stops: false,
            ..Options::default()
        });
        f.activate("a", "done");
        f.key(KeyCode::F2, false, native);
        assert_eq!(f.focused().as_deref(), Some("check"), "native {native}");
        f.key(KeyCode::Space, false, native);
        assert!(f.item("a").done.get().unwrap());
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.focused().as_deref(), Some("body"));
        // Tab from the body leaves the table.
        f.key(KeyCode::Tab, false, native);
        assert_eq!(f.focused().as_deref(), Some("other"));
    }
}

#[test]
fn hiding_the_table_closes_the_edit() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.key(KeyCode::Backspace, false, native);
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "refused, open, native {native}"
        );
        f.shown.set(false).unwrap();
        f.settle();
        f.shown.set(true).unwrap();
        f.settle();
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.name("a"), "Cable");
    }
}

#[test]
fn an_open_edit_is_reached_again_although_its_cell_became_ineligible() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.key(KeyCode::Backspace, false, native);
        f.key(KeyCode::Enter, false, native);
        let other = f.widget("other").unwrap();
        f.ui.focus(&other).unwrap();
        f.settle();
        // The same row, now locked.
        let mut rows = f.items.borrow().clone();
        rows[0].locked = true;
        f.state.set_source(TableSource::new(Rows(rows))).unwrap();
        f.settle();
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "still open, native {native}"
        );
        assert!(!f.editor_focused());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
        assert!(f.activations.borrow().is_empty());
    }
}

#[test]
fn a_later_description_does_not_change_the_mounted_tab_policy() {
    for native in [false, true] {
        let mut f = Fixture::new(Options {
            cell_stops: false,
            ..Options::default()
        });
        // Built and dropped, then applied: the first description's choice stays.
        drop(Table::new(&f.state, ROW).unwrap().cell_tab_stops(true));
        let view = f.widget("view").unwrap();
        f.ui.reconcile_children(
            &view,
            vec![Element::new(
                "table",
                Table::new(&f.state, ROW).unwrap().cell_tab_stops(true),
            )],
        )
        .unwrap();
        f.settle();
        f.activate("a", "done");
        f.key(KeyCode::F2, false, native);
        assert_eq!(f.focused().as_deref(), Some("check"), "native {native}");
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.focused().as_deref(), Some("body"), "native {native}");
    }
}

#[test]
fn unmounting_drops_the_editor_and_a_later_table_brings_its_own() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 160.0, 1.0).unwrap())
        .unwrap();
    let items = make_items(&ui, 3);
    let state = controller(&ui, &items);
    let token = Rc::new(());
    let (held, captured) = (token.clone(), state.clone());
    let editor = Table::new(&state, ROW).unwrap().cell_editor(move |edit| {
        // The callback holds the controller, as an application's often does.
        let _ = (&held, &captured);
        Ok(Element::new(
            "editor",
            TextField::new(edit.owner().signal(String::new())).session(edit.session()),
        ))
    });
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("table", editor).with_grow(1)]),
        ))
        .unwrap();
    support::frame(&mut ui);
    assert_eq!(Rc::strong_count(&token), 2);
    ui.reconcile_children(&root, Vec::new()).unwrap();
    support::frame(&mut ui);
    assert_eq!(Rc::strong_count(&token), 1, "the callback was dropped");

    // A table mounted later without an editor activates instead.
    let activations = Activations::default();
    let log = activations.clone();
    ui.reconcile_children(
        &root,
        vec![
            Element::new(
                "table",
                Table::new(&state, ROW)
                    .unwrap()
                    .on_activate(move |_, row, column| {
                        log.borrow_mut()
                            .push((row.to_owned(), column.map(str::to_owned)));
                        Ok(())
                    }),
            )
            .with_grow(1),
        ],
    )
    .unwrap();
    support::frame(&mut ui);
    let body = ui.find_descendant(&root, "body").unwrap().unwrap();
    state.selection().set_active(Some("a".into())).unwrap();
    state.set_active_column(Some("name".into())).unwrap();
    ui.focus(&body).unwrap();
    support::frame(&mut ui);
    support::press(&mut ui, KeyCode::Enter);
    support::frame(&mut ui);
    assert_eq!(
        *activations.borrow(),
        [("a".to_owned(), Some("name".to_owned()))]
    );
    let rows = ui.children(&body).unwrap();
    let cells = ui.children(&rows[0]).unwrap();
    let name = ui.children(&cells[0]).unwrap();
    assert_eq!(
        ui.key(&name[0]).unwrap(),
        "content",
        "no host, only content"
    );
}

#[test]
fn an_editor_that_cannot_take_focus_leaves_the_body_focused_and_escape_cancels() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.name_enabled.set(false).unwrap();
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "the edit opened, native {native}"
        );
        assert_eq!(f.editor_property("enabled"), InspectValue::Bool(false));
        assert!(!f.editor_focused());
        assert_eq!(f.focused().as_deref(), Some("body"));
        // Beginning it again finds nothing to focus.
        f.key(KeyCode::F2, false, native);
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        assert_eq!(f.focused().as_deref(), Some("body"));

        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.active(), (Some("a".into()), Some("name".into())));
        assert_eq!(f.name("a"), "Cable");
        assert!(f.activations.borrow().is_empty());
    }
}

#[test]
fn revoking_editing_keeps_focus_and_refuses_a_changed_draft() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Laptop");
        f.name_editable.set(false).unwrap();
        f.settle();
        assert_eq!(f.editor_property("read_only"), InspectValue::Bool(true));
        assert!(f.editor_focused(), "read-only keeps focus, native {native}");
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        f.type_text("s");
        assert_eq!(f.text(), "Laptop", "edits are refused");

        // A changed draft is refused: the edit stays open in the editor.
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "native {native}"
        );
        assert!(f.editor_focused());
        assert_eq!(f.name("a"), "Cable");
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.edited(), None);
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.name("a"), "Cable");

        // An unchanged draft closes as unchanged.
        f.key(KeyCode::Enter, false, native);
        assert!(f.editor_focused(), "native {native}");
        assert_eq!(f.editor_property("read_only"), InspectValue::Bool(true));
        f.key(KeyCode::Enter, false, native);
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.name("a"), "Cable");
        assert!(f.activations.borrow().is_empty());
    }
}

#[test]
fn a_disabled_editor_stays_open_until_escape_on_the_body() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.type_text("Laptop");
        assert!(f.editor_focused(), "native {native}");
        f.name_enabled.set(false).unwrap();
        f.settle();
        assert_eq!(f.editor_property("enabled"), InspectValue::Bool(false));
        assert!(!f.editor_focused(), "a disabled editor loses focus");
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "the edit stays open, native {native}"
        );
        assert_eq!(f.name("a"), "Cable", "the draft is not saved");

        let body = f.body();
        f.ui.focus(&body).unwrap();
        f.settle();
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        f.key(KeyCode::Escape, false, native);
        assert_eq!(f.edited(), None, "native {native}");
        assert_eq!(f.focused().as_deref(), Some("body"));
        assert_eq!(f.name("a"), "Cable");
    }
}

#[test]
fn unmounting_with_an_open_edit_leaves_no_session() {
    for native in [false, true] {
        let mut f = Fixture::new(Options::default());
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        f.key(KeyCode::Backspace, false, native);
        f.key(KeyCode::Enter, false, native);
        assert_eq!(
            f.edited(),
            Some(("a".into(), "name".into())),
            "refused, open, native {native}"
        );
        assert!(f.editor_focused());
        let view = f.widget("view").unwrap();
        f.ui.reconcile_children(&view, Vec::new()).unwrap();
        f.settle();
        assert!(f.widget("table").is_none(), "unmounted");
        assert_eq!(f.name("a"), "Cable");

        // A table mounted later on the same controller opens no edit by itself.
        let built = Rc::new(Cell::new(0));
        let (count, rows) = (built.clone(), f.items.clone());
        let table = Table::new(&f.state, ROW).unwrap().cell_editor(move |edit| {
            count.set(count.get() + 1);
            let name = rows
                .borrow()
                .iter()
                .find(|item| item.key == edit.row())
                .map(|item| item.name.clone())
                .ok_or(RuntimeError::Exhausted)?;
            Ok(Element::new(
                "editor",
                TextField::new(name).session(edit.session()),
            ))
        });
        f.ui.reconcile_children(&view, vec![Element::new("table", table)])
            .unwrap();
        f.settle();
        assert_eq!(f.edited(), None, "no edit survived, native {native}");
        assert_eq!(built.get(), 0, "no editor was built for an old edit");
        assert_eq!(f.name("a"), "Cable");
        // The same cell begins a new edit with the value, not the old draft.
        f.activate("a", "name");
        f.key(KeyCode::Enter, false, native);
        assert_eq!(f.edited(), Some(("a".into(), "name".into())));
        assert!(f.editor_focused());
        assert_eq!(built.get(), 1);
        assert_eq!(f.text(), "Cable");
    }
}
