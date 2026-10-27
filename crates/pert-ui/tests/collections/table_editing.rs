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

/// How a tracked table reports edit ends.
#[derive(Clone, Copy, PartialEq)]
enum Ends {
    /// No `on_edit_end`.
    Off,
    /// Logs each end and, when `clear`, clears its cell's modified count.
    Log { clear: bool },
}

/// What the tracked table's callbacks share.
#[derive(Clone)]
struct Tracking {
    items: Rc<RefCell<Vec<Item>>>,
    form: FormFields,
    handles: Rc<Vec<(String, FieldHandle)>>,
    order: Rc<RefCell<Vec<String>>>,
    quantity_enabled: Signal<bool>,
    fail_end: Rc<Cell<bool>>,
    /// Each cell's modified state as last set.
    states: Rc<RefCell<Vec<(String, bool)>>>,
}
impl Tracking {
    /// Sets a cell's modified state in the form and in `states`.
    fn set_modified(&self, row: &str, column: &str, modified: bool) -> Result<(), RuntimeError> {
        let key = format!("{row}/{column}");
        let mut states = self.states.borrow_mut();
        states.retain(|(name, _)| *name != key);
        states.push((key, modified));
        drop(states);
        self.handle(row, column).set_modified(modified).map(|_| ())
    }
    fn modified(&self, row: &str, column: &str) -> bool {
        let key = format!("{row}/{column}");
        self.states
            .borrow()
            .iter()
            .any(|(name, modified)| *name == key && *modified)
    }
    fn handle(&self, row: &str, column: &str) -> FieldHandle {
        let key = format!("{row}/{column}");
        self.handles
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, handle)| handle.clone())
            .unwrap()
    }
    /// A table whose name and quantity editors feed their cell's modified
    /// count, with edit ends reported as `ends` says.
    fn table(&self, state: &TableController, ends: Ends) -> Table {
        let tracking = self.clone();
        let table = Table::new(state, ROW).unwrap().cell_editor(move |edit| {
            let item = tracking
                .items
                .borrow()
                .iter()
                .find(|item| item.key == edit.row())
                .cloned()
                .ok_or(RuntimeError::Exhausted)?;
            let (cells, order, row, column) = (
                tracking.clone(),
                tracking.order.clone(),
                edit.row().to_owned(),
                edit.column().to_owned(),
            );
            let modified = move |_: &mut EventContext, modified: bool| {
                order
                    .borrow_mut()
                    .push(format!("modified {row}/{column} {modified}"));
                cells.set_modified(&row, &column, modified)
            };
            Ok(match edit.column() {
                "name" => Element::new(
                    "editor",
                    TextField::new(item.name.clone())
                        .normalize(|text: String| {
                            if text.trim().is_empty() {
                                Err(InputRejection::new("A name is required"))
                            } else {
                                Ok(text)
                            }
                        })
                        .on_modified(modified)
                        .session(edit.session()),
                ),
                _ => Element::new(
                    "editor",
                    IntegerInput::new(item.quantity.clone())
                        .range(0, 1000)
                        .unwrap()
                        .enabled(&tracking.quantity_enabled)
                        .on_modified(modified)
                        .session(edit.session()),
                ),
            })
        });
        match ends {
            Ends::Off => table,
            Ends::Log { clear } => {
                let tracking = self.clone();
                table.on_edit_end(move |_, end| {
                    tracking.order.borrow_mut().push(format!(
                        "end {}/{} {:?}",
                        end.row(),
                        end.column(),
                        end.reason()
                    ));
                    if tracking.fail_end.get() {
                        return Err(RuntimeError::Exhausted);
                    }
                    if clear {
                        tracking.set_modified(end.row(), end.column(), false)?;
                    }
                    Ok(())
                })
            }
        }
    }
}

/// A component beside the table that records its own notifications in the
/// shared order, requested by the test through its notifier.
struct Probe {
    order: Rc<RefCell<Vec<String>>>,
    notifier: Rc<RefCell<Option<Notifier>>>,
    /// Each change requests one delivery from the probe's update.
    trigger: ReadSignal<u32>,
    seen: u32,
    /// Fails the probe's next delivery once, which ends that flush before
    /// requests made during it are delivered.
    fail: Rc<Cell<bool>>,
}
impl Component for Probe {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let notifier = self
            .notifier
            .borrow_mut()
            .get_or_insert_with(|| cx.notifier())
            .clone();
        let trigger = cx.read(&self.trigger)?;
        if trigger != self.seen {
            self.seen = trigger;
            notifier.request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.order.borrow_mut().push("probe".to_owned());
        if self.fail.replace(false) {
            return Err(RuntimeError::Exhausted);
        }
        Ok(())
    }
}

/// The probe's notifier slot, failure switch and trigger.
type ProbeParts<'a> = (
    &'a Rc<RefCell<Option<Notifier>>>,
    &'a Rc<Cell<bool>>,
    &'a Signal<u32>,
);

struct Tracked {
    ui: Runtime,
    root: WidgetId,
    state: TableController,
    tracking: Tracking,
    shown: Signal<bool>,
    probe: Rc<RefCell<Option<Notifier>>>,
    fail_probe: Rc<Cell<bool>>,
    trigger: Signal<u32>,
}
impl Tracked {
    fn new(ends: Ends) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(480.0, 160.0, 1.0).unwrap())
            .unwrap();
        let items = make_items(&ui, 3);
        let state = controller(&ui, &items);
        let form = FormFields::new(&ui);
        let handles = items
            .iter()
            .flat_map(|item| ["name", "quantity"].map(|column| format!("{}/{column}", item.key)))
            .map(|key| {
                let handle = form.field(key.clone()).unwrap();
                (key, handle)
            })
            .collect();
        let tracking = Tracking {
            items: Rc::new(RefCell::new(items)),
            form,
            handles: Rc::new(handles),
            order: Rc::default(),
            quantity_enabled: ui.signal(true),
            fail_end: Rc::default(),
            states: Rc::default(),
        };
        let shown = ui.signal(true);
        let fail_probe = Rc::<Cell<bool>>::default();
        let trigger = ui.signal(0_u32);
        let probe = Rc::<RefCell<Option<Notifier>>>::default();
        let root = ui
            .mount_tree(Element::new(
                "root",
                Column::new(Self::describe(
                    &tracking,
                    &state,
                    &shown,
                    (&probe, &fail_probe, &trigger),
                    ends,
                )),
            ))
            .unwrap();
        let mut tracked = Self {
            ui,
            root,
            state,
            tracking,
            shown,
            probe,
            fail_probe,
            trigger,
        };
        tracked.settle();
        tracked
    }
    fn describe(
        tracking: &Tracking,
        state: &TableController,
        shown: &Signal<bool>,
        (probe, fail, trigger): ProbeParts<'_>,
        ends: Ends,
    ) -> Vec<Element> {
        vec![
            Element::new(
                "view",
                View::new(Element::new("table", tracking.table(state, ends)))
                    .visible_signal(shown.clone()),
            )
            .with_grow(1),
            Element::new(
                "probe",
                Probe {
                    order: tracking.order.clone(),
                    notifier: probe.clone(),
                    trigger: trigger.read_only(),
                    seen: 0,
                    fail: fail.clone(),
                },
            ),
            Element::new("other", Button::new("Other", |_| Ok(()))),
        ]
    }
    /// Describes the page again with `ends`.
    fn redescribe(&mut self, ends: Ends) {
        let children = Self::describe(
            &self.tracking,
            &self.state,
            &self.shown,
            (&self.probe, &self.fail_probe, &self.trigger),
            ends,
        );
        self.ui.reconcile_children(&self.root, children).unwrap();
    }
    fn settle(&mut self) {
        for _ in 0..3 {
            support::frame(&mut self.ui);
        }
    }
    fn body(&self) -> WidgetId {
        self.ui
            .find_descendant(&self.root, "body")
            .unwrap()
            .unwrap()
    }
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
    fn key(&mut self, code: KeyCode) {
        self.ui.key_input(code, KeyAction::Press).unwrap();
        self.ui.key_input(code, KeyAction::Release).unwrap();
        self.settle();
    }
    fn type_text(&mut self, text: &str) {
        self.ui.text_input(text).unwrap();
        self.settle();
    }
    fn count(&self) -> usize {
        self.tracking.form.summary().get().unwrap().modified()
    }
    fn editor(&self) -> Option<WidgetId> {
        self.ui.find_descendant(&self.body(), "editor").unwrap()
    }
    fn editor_text(&self) -> String {
        let editor = self.editor().expect("an editor");
        let id = self.ui.inspection_id(&editor).unwrap();
        match &self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .details
            .properties["text"]
        {
            InspectValue::Text(text) => text.clone(),
            other => panic!("not text: {other:?}"),
        }
    }
    fn ends(&self) -> Vec<String> {
        self.tracking
            .order
            .borrow()
            .iter()
            .filter(|entry| entry.starts_with("end "))
            .cloned()
            .collect()
    }
    fn quantity(&self, row: &str) -> i64 {
        self.tracking
            .items
            .borrow()
            .iter()
            .find(|item| item.key == row)
            .unwrap()
            .quantity
            .get()
            .unwrap()
    }
    /// Leaves a refused name draft open in row `a`, its editor focused.
    fn refused(&mut self) {
        self.activate("a", "name");
        self.key(KeyCode::Enter);
        self.key(KeyCode::Backspace);
        self.key(KeyCode::Enter);
        assert!(self.editor().is_some(), "the refused edit stays open");
        assert_eq!(self.count(), 1);
    }
}

/// New-behavior test 4. Named assertion: activating the cell again focuses
/// the editor, whose text is the draft typed before it was disabled.
/// Mutation checked: discarding a number field's draft on revocation shows
/// the quantity again.
#[test]
fn a_disabled_number_cell_keeps_its_draft() {
    let mut t = Tracked::new(Ends::Off);
    t.activate("a", "quantity");
    t.key(KeyCode::Enter);
    t.type_text("42");
    t.tracking.quantity_enabled.set(false).unwrap();
    t.settle();
    assert!(t.editor().is_some(), "the edit stays open");
    assert_eq!(t.quantity("a"), 10);
    t.tracking.quantity_enabled.set(true).unwrap();
    t.settle();
    t.activate("a", "quantity");
    t.key(KeyCode::Enter);
    assert_eq!(t.editor_text(), "42");
    t.key(KeyCode::Enter);
    assert_eq!(t.quantity("a"), 42);
    assert!(t.editor().is_none());
}

/// New-behavior test 14. Named assertion: after Escape the editor's own
/// `false` has reached the application, so the count is zero, although the
/// table removed the editor. Mutation checked: reporting Escape's
/// completion before requesting the modified report leaves the count at
/// one.
#[test]
fn escape_reaches_the_application_before_the_table_removes_the_editor() {
    for column in ["name", "quantity"] {
        let mut t = Tracked::new(Ends::Log { clear: false });
        t.activate("a", column);
        t.key(KeyCode::Enter);
        t.type_text(if column == "name" { "Lamp" } else { "42" });
        assert_eq!(t.count(), 1, "{column}");
        t.key(KeyCode::Escape);
        assert_eq!(t.count(), 0, "{column}");
        assert!(t.editor().is_none(), "{column}: the edit is closed");
        assert_eq!(
            t.ends(),
            [format!(
                "end a/{column} Completed(EditorCompletion {{ trigger: Escape, outcome: Cancelled }})"
            )]
        );
        let order = t.tracking.order.borrow();
        let modified = order
            .iter()
            .position(|entry| *entry == format!("modified a/{column} false"))
            .unwrap();
        let ended = order
            .iter()
            .position(|entry| entry.starts_with("end "))
            .unwrap();
        assert!(modified < ended, "{column}: {order:?}");
    }
}

/// New-behavior test 15 (rule 11). Named assertion for each end the table
/// performs itself: `on_edit_end` ran once with its reason and the count is
/// zero. Mutations checked: not recording `cancel_unfocused`'s end leaves
/// the body-Escape count at one; dropping the hidden end in
/// `close_quietly` leaves the hiding count at one.
#[test]
fn the_table_reports_the_edits_it_ends() {
    // A terminal completion is reported after the editor's own `false`.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.activate("a", "name");
    t.key(KeyCode::Enter);
    t.type_text("Lamp");
    t.key(KeyCode::Enter);
    assert_eq!(
        *t.tracking.order.borrow(),
        [
            "modified a/name true",
            "modified a/name false",
            "end a/name Completed(EditorCompletion { trigger: Enter, outcome: Saved })",
        ]
    );

    // Escape on the body ends an edit whose editor does not hold focus.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.refused();
    let body = t.body();
    t.ui.focus(&body).unwrap();
    t.settle();
    assert!(
        t.editor().is_some(),
        "a refused departure keeps the edit open"
    );
    t.key(KeyCode::Escape);
    assert_eq!(t.ends(), ["end a/name Cancelled"]);
    assert_eq!(t.count(), 0);

    // A filter that hides the edited row.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.refused();
    t.state.filter("Tape", &FilterMatcher::substring()).unwrap();
    t.settle();
    assert_eq!(t.ends(), ["end a/name RowLeft"]);
    assert_eq!(t.count(), 0);

    // Deleting the row.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.refused();
    let rest: Vec<Item> = t
        .tracking
        .items
        .borrow()
        .iter()
        .filter(|item| item.key != "a")
        .cloned()
        .collect();
    t.state.set_source(TableSource::new(Rows(rest))).unwrap();
    t.settle();
    assert_eq!(t.ends(), ["end a/name RowLeft"]);
    assert_eq!(t.count(), 0);

    // Hiding the table.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.refused();
    t.shown.set(false).unwrap();
    t.settle();
    assert_eq!(t.ends(), ["end a/name Hidden"]);
    assert_eq!(t.count(), 0);
}

/// New-behavior test 16. Named assertion: exactly one `Replaced` for the
/// first cell whichever way the second edit begins, and the counts follow
/// each cell. Mutation checked: recording the replacement twice fails the
/// "exactly one" assertion.
#[test]
fn replacement_is_reported_once_after_settling() {
    for double_click in [false, true] {
        let mut t = Tracked::new(Ends::Log { clear: true });
        t.refused();
        if double_click {
            let row = t.ui.find_descendant(&t.body(), "c").unwrap().unwrap();
            let cell = t.ui.find_descendant(&row, "name").unwrap().unwrap();
            let [x, y, w, h] = t.ui.bounds(&cell).unwrap().unwrap().values();
            let point = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
            for _ in 0..2 {
                for kind in [
                    PointerKind::Press(PointerButton::Primary),
                    PointerKind::Release(PointerButton::Primary),
                ] {
                    t.ui.pointer_input(PointerEvent::new(point, kind)).unwrap();
                }
            }
            t.settle();
        } else {
            t.activate("c", "name");
            t.key(KeyCode::F2);
        }
        let context = format!("double click {double_click}");
        assert_eq!(t.ends(), ["end a/name Replaced"], "{context}");
        assert!(!t.tracking.modified("a", "name"), "{context}");
        assert_eq!(t.count(), 0, "{context}");
        t.type_text("x");
        assert_eq!(t.count(), 1, "{context}");
        assert!(t.tracking.modified("c", "name"), "{context}");
    }
}

/// New-behavior test 17. Named assertion: in one flush with a refused
/// completion, another component's request and the table hiding, the order
/// is the probe's callback, then `Hidden`. Mutation checked: delivering
/// ends from the table's own delivery puts `Hidden` before the probe.
#[test]
fn edit_ends_keep_the_global_notification_order() {
    let prepare = |t: &mut Tracked| {
        t.refused();
        t.tracking.order.borrow_mut().clear();
        // In one flush: the editor's refused departure, the probe's request
        // and the table hiding.
        t.ui.clear_focus();
        t.probe.borrow().as_ref().unwrap().request();
        t.shown.set(false).unwrap();
    };
    let mut t = Tracked::new(Ends::Log { clear: true });
    prepare(&mut t);
    t.ui.flush(t.ui.now()).unwrap();
    t.settle();
    assert_eq!(
        t.tracking.order.borrow()[..2],
        ["probe", "end a/name Hidden"],
        "nothing about the edit before the probe"
    );

    // A failing edit-end callback: the flush returns the error once, the
    // probe still ran, and nothing is delivered again.
    let mut t = Tracked::new(Ends::Log { clear: true });
    t.tracking.fail_end.set(true);
    prepare(&mut t);
    let flushed = t.ui.flush(t.ui.now()).map(|_| ());
    assert_eq!(flushed, Err(RuntimeError::Exhausted));
    t.settle();
    assert_eq!(
        t.tracking.order.borrow()[..2],
        ["probe", "end a/name Hidden"]
    );
    assert_eq!(t.ends(), ["end a/name Hidden"], "not delivered again");

    // In a hidden window nothing is delivered until it is shown.
    use pert_ui::testing::HostIntegration;
    let mut t = Tracked::new(Ends::Log { clear: true });
    prepare(&mut t);
    t.ui.set_window_visible(false).unwrap();
    t.ui.flush(t.ui.now()).unwrap();
    assert!(t.tracking.order.borrow().is_empty(), "held while hidden");
    t.ui.set_window_visible(true).unwrap();
    t.settle();
    assert_eq!(
        t.tracking.order.borrow()[..2],
        ["probe", "end a/name Hidden"]
    );
}

/// New-behavior test 18. Named assertion: describing the table without
/// `on_edit_end` and again with a new callback before delivery reports
/// nothing for the recorded end, while the table closed the edit. Mutation
/// checked: keeping undelivered payloads across removal hands the old end to
/// the reinstalled callback.
#[test]
fn removing_the_callback_discards_pending_ends() {
    // Records the end of an edit whose row a filter hides, and ends the
    // flush before delivering it: the table's delivery records the end, and
    // the probe's delivery after it in the same round fails, so the flush
    // returns before the next round would deliver the end.
    let pending = || {
        let mut t = Tracked::new(Ends::Log { clear: false });
        t.refused();
        t.tracking.order.borrow_mut().clear();
        t.state.filter("Tape", &FilterMatcher::substring()).unwrap();
        t.fail_probe.set(true);
        let next = t.trigger.get().unwrap() + 1;
        t.trigger.set(next).unwrap();
        let flushed = t.ui.flush(t.ui.now()).map(|_| ());
        assert_eq!(flushed, Err(RuntimeError::Exhausted));
        assert_eq!(
            *t.tracking.order.borrow(),
            ["probe"],
            "the end is held back"
        );
        t
    };
    let mut t = pending();
    t.redescribe(Ends::Off);
    t.redescribe(Ends::Log { clear: false });
    t.settle();
    assert!(t.ends().is_empty(), "{:?}", t.tracking.order.borrow());
    assert!(t.editor().is_none(), "the edit is closed all the same");
    // A later end reaches the new callback.
    t.state.filter("", &FilterMatcher::substring()).unwrap();
    t.settle();
    t.activate("c", "name");
    t.key(KeyCode::Enter);
    t.key(KeyCode::Escape);
    assert_eq!(
        t.ends(),
        ["end c/name Completed(EditorCompletion { trigger: Escape, outcome: Cancelled })"]
    );

    // Replacing the callback instead hands it the pending end.
    let mut t = pending();
    t.redescribe(Ends::Log { clear: true });
    t.settle();
    assert_eq!(t.ends(), ["end a/name RowLeft"]);
    assert_eq!(t.count(), 0, "the new callback cleared it");
}

/// New-behavior test 12 in a table: `DiscardDraft` aimed at a session
/// editor discards its draft, reports nothing to the session and keeps the
/// edit open.
#[test]
fn discard_draft_keeps_a_cells_edit_open() {
    let mut t = Tracked::new(Ends::Log { clear: false });
    t.activate("a", "quantity");
    t.key(KeyCode::Enter);
    t.type_text("42");
    assert_eq!(t.count(), 1);
    let editor = t.editor().unwrap();
    assert_eq!(
        t.ui.text_command(&editor, TextCommand::DiscardDraft)
            .unwrap(),
        TextCommandOutcome::Handled
    );
    t.settle();
    assert_eq!(t.count(), 0);
    assert!(t.editor().is_some(), "the edit stays open");
    assert!(t.ends().is_empty());
    t.key(KeyCode::Enter);
    assert_eq!(
        t.ends(),
        ["end a/quantity Completed(EditorCompletion { trigger: Enter, outcome: Unchanged })"]
    );
    assert_eq!(t.quantity("a"), 10);
}
