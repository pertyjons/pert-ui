//! Re-described collections: `VirtualList`, `Tree` and `Table` keep their
//! controller, source and structure, while their callbacks follow the new
//! description, including removal.
use crate::support;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use support::{key, settle};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<String>>>;
const ROW: f32 = 24.0;

struct Rows;
impl ListItems for Rows {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("row{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        ListItems::key(self, index).map(|key| Element::new(key, Label::new("Row")))
    }
}
impl TreeItems for Rows {
    fn roots(&self) -> Vec<String> {
        (0..3).map(|index| format!("row{index}")).collect()
    }
    fn children(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
impl TableItems for Rows {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("row{index}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        TableItems::key(self, index).map(|_| Element::new("value", Label::new("Cell")))
    }
}

/// A callback that records `name` and what it received.
fn recorder(
    log: &Log,
    name: &'static str,
) -> impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static {
    let log = log.clone();
    move |_, key| {
        log.borrow_mut().push(format!("{name}:{key}"));
        Ok(())
    }
}

fn selection_recorder(
    log: &Log,
    name: &'static str,
) -> impl FnMut(&mut EventContext, &ListSelectionChange) -> Result<(), RuntimeError> + 'static {
    let log = log.clone();
    move |_, change| {
        log.borrow_mut().push(format!(
            "{name}:{}",
            change.active.as_deref().unwrap_or("none")
        ));
        Ok(())
    }
}

/// The three collections over one selection, built by `kind`.
struct Stage {
    ui: Runtime,
    selection: ListSelection,
    items: Signal<ListSource>,
    tree: TreeController,
    table: TableController,
    root: WidgetId,
}

impl Stage {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 240.0, 1.0)?)?;
        let root = ui.mount_tree(Element::new("root", Column::new([])))?;
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let items = ui.signal(ListSource::new(Rows));
        let tree = TreeController::new(&ui, selection.clone(), TreeSource::new(Rows))?;
        let table = TableController::new(
            &ui,
            selection.clone(),
            [TableColumn::new("a", "A").sortable(true)],
            TableSource::new(Rows),
        )?;
        Ok(Self {
            ui,
            selection,
            items,
            tree,
            table,
            root,
        })
    }
    fn list(&self) -> Result<VirtualList, GeometryError> {
        Ok(VirtualList::new(self.items.read_only(), ROW)?.selection(self.selection.clone()))
    }
    fn describe(&mut self, collection: Element) -> TestResult {
        self.ui.reconcile_children(&self.root, vec![collection])?;
        self.ui.flush(self.ui.now())?;
        Ok(())
    }
    /// The widget that takes the collection's focus: a table's body.
    fn focus(&mut self, table: bool) -> TestResult {
        let collection = support::widget(&self.ui, &self.root, "collection");
        let target = if table {
            self.ui.children(&collection)?[0].clone()
        } else {
            collection
        };
        self.ui.focus(&target)?;
        settle(&mut self.ui);
        Ok(())
    }
    fn enter(&mut self) {
        key(&mut self.ui, KeyCode::Enter);
        settle(&mut self.ui);
    }
}

#[test]
fn a_redescription_replaces_and_removes_the_activation_callback() -> TestResult {
    for kind in 0..3 {
        let mut f = Stage::new()?;
        let log: Log = Rc::default();
        let describe = |f: &Stage, name: Option<&'static str>| -> Result<Element, GeometryError> {
            Ok(match kind {
                0 => {
                    let list = f.list()?;
                    Element::new(
                        "collection",
                        match name {
                            Some(name) => list.on_activate(recorder(&log, name)),
                            None => list,
                        },
                    )
                }
                1 => {
                    let tree = Tree::new(&f.tree, ROW)?;
                    Element::new(
                        "collection",
                        match name {
                            Some(name) => tree.on_activate(recorder(&log, name)),
                            None => tree,
                        },
                    )
                }
                _ => {
                    let table = Table::new(&f.table, ROW)?;
                    let log = log.clone();
                    Element::new(
                        "collection",
                        match name {
                            Some(name) => table.on_activate(move |_, row, _| {
                                log.borrow_mut().push(format!("{name}:{row}"));
                                Ok(())
                            }),
                            None => table,
                        },
                    )
                }
            })
        };
        let first = describe(&f, Some("first"))?;
        f.describe(first)?;
        f.selection.set_active(Some("row1".into()))?;
        f.focus(kind == 2)?;
        f.enter();
        let second = describe(&f, Some("second"))?;
        f.describe(second)?;
        f.enter();
        let without = describe(&f, None)?;
        f.describe(without)?;
        f.enter();
        assert_eq!(*log.borrow(), ["first:row1", "second:row1"], "kind {kind}");
    }
    Ok(())
}

#[test]
fn a_redescription_replaces_removes_and_reinstalls_the_selection_callback() -> TestResult {
    for kind in 0..3 {
        let mut f = Stage::new()?;
        let log: Log = Rc::default();
        let describe = |f: &Stage, name: Option<&'static str>| -> Result<Element, GeometryError> {
            let element = match (kind, name) {
                (0, Some(name)) => Element::new(
                    "collection",
                    f.list()?
                        .on_selection_change(selection_recorder(&log, name)),
                ),
                (0, None) => Element::new("collection", f.list()?),
                (1, Some(name)) => Element::new(
                    "collection",
                    Tree::new(&f.tree, ROW)?.on_selection_change(selection_recorder(&log, name)),
                ),
                (1, None) => Element::new("collection", Tree::new(&f.tree, ROW)?),
                (_, Some(name)) => Element::new(
                    "collection",
                    Table::new(&f.table, ROW)?.on_selection_change(selection_recorder(&log, name)),
                ),
                (_, None) => Element::new("collection", Table::new(&f.table, ROW)?),
            };
            Ok(element)
        };
        let first = describe(&f, Some("first"))?;
        f.describe(first)?;
        f.selection.set_active(Some("row0".into()))?;
        settle(&mut f.ui);
        let second = describe(&f, Some("second"))?;
        f.describe(second)?;
        f.selection.set_active(Some("row1".into()))?;
        settle(&mut f.ui);
        let without = describe(&f, None)?;
        f.describe(without)?;
        f.selection.set_active(Some("row2".into()))?;
        settle(&mut f.ui);
        // A callback installed again starts from the selection it first sees,
        // so the change made without one is not reported to it.
        let third = describe(&f, Some("third"))?;
        f.describe(third)?;
        settle(&mut f.ui);
        f.selection.set_active(Some("row0".into()))?;
        settle(&mut f.ui);
        assert_eq!(
            *log.borrow(),
            ["first:row0", "second:row1", "third:row0"],
            "kind {kind}"
        );
    }
    Ok(())
}

#[test]
fn a_callback_removed_and_reinstalled_before_delivery_discards_the_undelivered_change() -> TestResult
{
    for table in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(600.0, 240.0, 1.0)?)?;
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let items = ui.signal(ListSource::new(Rows));
        let state = TableController::new(
            &ui,
            selection.clone(),
            [TableColumn::new("a", "A")],
            TableSource::new(Rows),
        )?;
        // 0: the old callback; 1: none; 2: a new callback.
        let phase = ui.signal(0_u8);
        let log: Log = Rc::default();
        // Two lists sharing the selection report first: the first removes the
        // last collection's callback and the second installs a new one, both
        // while the last collection's change waits for delivery.
        let sharing = |next: u8| -> Result<VirtualList, GeometryError> {
            let phase = phase.clone();
            Ok(VirtualList::new(items.read_only(), ROW)?
                .selection(selection.clone())
                // Only the first change moves the phase on.
                .on_selection_change(move |_, _| {
                    if phase.get()? + 1 == next {
                        phase.set(next)?;
                    }
                    Ok(())
                }))
        };
        let (removing, reinstalling) = (sharing(1)?, sharing(2)?);
        let (record, rows, shared) = (log.clone(), items.clone(), selection.clone());
        let last = Dynamic::new(&phase, move |_, phase| {
            let name = match phase {
                0 => Some("old"),
                1 => None,
                _ => Some("new"),
            };
            let callback = name.map(|name| {
                let log = record.clone();
                move |_: &mut EventContext, change: &ListSelectionChange| {
                    log.borrow_mut().push(format!(
                        "{name}:{}",
                        change.active.as_deref().unwrap_or("none")
                    ));
                    Ok(())
                }
            });
            if table {
                let table = Table::new(&state, ROW).unwrap();
                Element::new(
                    "last",
                    match callback {
                        Some(callback) => table.on_selection_change(callback),
                        None => table,
                    },
                )
            } else {
                let list = VirtualList::new(rows.read_only(), ROW)
                    .unwrap()
                    .selection(shared.clone());
                Element::new(
                    "last",
                    match callback {
                        Some(callback) => list.on_selection_change(callback),
                        None => list,
                    },
                )
            }
        });
        ui.mount_tree(Element::new(
            "root",
            Row::new([
                Element::new("removing", removing),
                Element::new("reinstalling", reinstalling),
                Element::new("wiring", last),
            ]),
        ))?;
        ui.flush(Duration::ZERO)?;
        selection.set_active(Some("row1".into()))?;
        settle(&mut ui);
        assert_eq!(phase.get()?, 2, "both sharing callbacks ran");
        // The new callback starts from the selection it first sees.
        selection.set_active(Some("row2".into()))?;
        settle(&mut ui);
        assert_eq!(*log.borrow(), ["new:row2"], "table {table}");
    }
    Ok(())
}

#[test]
fn a_redescription_replaces_and_removes_the_text_input_callback() -> TestResult {
    let mut f = Stage::new()?;
    let log: Log = Rc::default();
    let first = f.list()?.on_text_input(recorder(&log, "first"));
    f.describe(Element::new("collection", first))?;
    f.selection.set_active(Some("row0".into()))?;
    f.focus(false)?;
    f.ui.text_input("a")?;
    let second = f.list()?.on_text_input(recorder(&log, "second"));
    f.describe(Element::new("collection", second))?;
    f.ui.text_input("b")?;
    let without = f.list()?;
    f.describe(Element::new("collection", without))?;
    f.ui.text_input("c")?;
    assert_eq!(*log.borrow(), ["first:a", "second:b"]);
    Ok(())
}

#[test]
fn a_redescribed_table_replaces_and_removes_its_sort_callback() -> TestResult {
    let mut f = Stage::new()?;
    let log: Log = Rc::default();
    let sorting = |name: &'static str| {
        let log = log.clone();
        move |_: &mut EventContext, sort: &TableSort| {
            log.borrow_mut().push(format!("{name}:{}", sort.column));
            Ok(())
        }
    };
    let click = |f: &mut Stage| -> TestResult {
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            f.ui.pointer_input(PointerEvent::new(Point::new(40.0, 10.0)?, kind))?;
        }
        settle(&mut f.ui);
        Ok(())
    };
    let first = Table::new(&f.table, ROW)?.on_sort(sorting("first"));
    f.describe(Element::new("collection", first))?;
    click(&mut f)?;
    let second = Table::new(&f.table, ROW)?.on_sort(sorting("second"));
    f.describe(Element::new("collection", second))?;
    click(&mut f)?;
    let without = Table::new(&f.table, ROW)?;
    f.describe(Element::new("collection", without))?;
    click(&mut f)?;
    assert_eq!(*log.borrow(), ["first:a", "second:a"]);
    Ok(())
}

#[test]
fn a_redescribed_table_builds_later_edits_with_its_new_cell_editor() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 240.0, 1.0)?)?;
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TableController::new(
        &ui,
        selection.clone(),
        [TableColumn::new("a", "A").editable(true)],
        TableSource::new(Rows),
    )?;
    let log: Log = Rc::default();
    let value = ui.signal(String::from("Cell"));
    let describe = |name: Option<&'static str>| -> Result<Element, GeometryError> {
        let activated = log.clone();
        let table = Table::new(&state, ROW)?.on_activate(move |_, row, _| {
            activated.borrow_mut().push(format!("activate:{row}"));
            Ok(())
        });
        let Some(name) = name else {
            return Ok(Element::new("table", table));
        };
        let (built, value) = (log.clone(), value.clone());
        Ok(Element::new(
            "table",
            table.cell_editor(move |edit| {
                built.borrow_mut().push(format!("{name}:{}", edit.row()));
                Ok(Element::new(
                    "field",
                    TextField::new(value.clone()).session(edit.session()),
                ))
            }),
        ))
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(Some("first"))?]),
    ))?;
    settle(&mut ui);
    let table = support::widget(&ui, &root, "table");
    let body = ui.children(&table)?[0].clone();
    let edit = |ui: &mut Runtime, row: &str| -> TestResult {
        ui.focus(&body)?;
        selection.set_active(Some(row.into()))?;
        state.set_active_column(Some("a".into()))?;
        settle(ui);
        key(ui, KeyCode::Enter);
        settle(ui);
        // Escape closes the edit, returning focus to the body.
        key(ui, KeyCode::Escape);
        settle(ui);
        Ok(())
    };
    edit(&mut ui, "row0")?;
    ui.reconcile_children(&root, vec![describe(Some("second"))?])?;
    settle(&mut ui);
    edit(&mut ui, "row1")?;
    ui.reconcile_children(&root, vec![describe(None)?])?;
    settle(&mut ui);
    // Without an editor, Enter activates the cell instead of editing it.
    edit(&mut ui, "row2")?;
    assert_eq!(
        *log.borrow(),
        ["first:row0", "second:row1", "activate:row2"]
    );
    Ok(())
}
