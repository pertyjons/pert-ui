//! The headless session: drives the editor as a user would, with the test
//! driver on the native key path, checks each result and reports what it
//! cost.
use crate::app::{self, Inventory};
use crate::model::{Category, Field, ItemId};
use pert_ui::prelude::*;
use pert_ui::testing::{Detail, Driver, InspectionQuery, Selector};
use pert_ui::{Clipboard, ClipboardError, InspectValue};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A clipboard in memory, so copy and paste need no desktop.
#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);
impl Clipboard for Memory {
    fn read_text(&mut self) -> std::result::Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> std::result::Result<(), ClipboardError> {
        text.clone_into(&mut self.0.borrow_mut());
        Ok(())
    }
}

/// Fails the session with `what` unless `ok`.
fn check(ok: bool, what: &str) -> Result {
    if ok {
        println!("ok    {what}");
        Ok(())
    } else {
        Err(format!("failed: {what}").into())
    }
}

fn report(what: &str, took: Duration) {
    println!("time  {what:<44} {took:>10.2?}");
}

struct Session {
    ui: Runtime,
    root: WidgetId,
    inventory: Rc<Inventory>,
}
impl Session {
    /// The test driver over the session's runtime: each action settles with
    /// one flush.
    fn driver(&mut self) -> Driver<'_> {
        Driver::new(&mut self.ui)
    }
    fn widget(&self, key: &str) -> Result<WidgetId> {
        Ok(self
            .ui
            .find_descendant(&self.root, key)?
            .ok_or_else(|| format!("no {key}"))?)
    }
    /// Clicks a point, such as a table cell, which is not one component of
    /// its own, and settles.
    fn click(&mut self, at: Point) -> Result {
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui.pointer_input(PointerEvent::new(at, kind))?;
        }
        self.driver().settle()?;
        Ok(())
    }
    /// The middle of a mounted cell.
    fn cell(&self, row: &str, column: &str) -> Result<Point> {
        let body = self.widget("body")?;
        let row = self
            .ui
            .find_descendant(&body, row)?
            .ok_or("the row is not mounted")?;
        let cell = self.ui.find_descendant(&row, column)?.ok_or("no cell")?;
        let [x, y, w, h] = self.ui.bounds(&cell)?.ok_or("not laid out")?.values();
        Ok(Point::new(x + w / 2.0, y + h / 2.0)?)
    }
    /// The edited cell as `(row, column)`: where the table put its host.
    fn editing(&self) -> Result<Option<(String, String)>> {
        for row in self.ui.children(&self.widget("body")?)? {
            for cell in self.ui.children(&row)? {
                for child in self.ui.children(&cell)? {
                    if self.ui.key(&child)? == "cell editor" {
                        return Ok(Some((
                            self.ui.key(&row)?.to_owned(),
                            self.ui.key(&cell)?.to_owned(),
                        )));
                    }
                }
            }
        }
        Ok(None)
    }
    fn editing_is(&self, row: &str, field: Field) -> Result<bool> {
        Ok(self.editing()? == Some((row.to_owned(), field.key().to_owned())))
    }
    /// The focused component's key and its inspected properties.
    fn focused(&self) -> Option<(String, std::collections::BTreeMap<String, InspectValue>)> {
        let id = self.ui.inspection_id(&self.ui.focused()?).ok()?;
        let component = self
            .ui
            .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
            .components
            .pop()?;
        Some((
            component.key,
            component.details.properties.into_iter().collect(),
        ))
    }
    fn focused_key(&self) -> Option<String> {
        self.focused().map(|(key, _)| key)
    }
    fn focused_text(&self) -> Option<String> {
        match self.focused()?.1.get("text")? {
            InspectValue::Text(text) => Some(text.clone()),
            _ => None,
        }
    }
    /// Whether focus is anywhere inside the table.
    fn focus_in_table(&self) -> Result<bool> {
        let Some(focused) = self.ui.focused() else {
            return Ok(false);
        };
        let table = self.ui.inspection_id(&self.widget("table")?)?;
        let snapshot = self.ui.inspect();
        let mut id = Some(self.ui.inspection_id(&focused)?);
        while let Some(current) = id {
            if current == table {
                return Ok(true);
            }
            id = snapshot
                .components
                .iter()
                .find(|c| c.id == current)
                .and_then(|c| c.parent);
        }
        Ok(false)
    }
    fn item<T>(&self, row: &str, read: impl FnOnce(&crate::model::Item) -> T) -> Result<T> {
        let id = ItemId::from_key(row).ok_or("not an item key")?;
        let store = self.inventory.store.borrow();
        Ok(read(store.get(id).ok_or("no such item")?))
    }
    fn active(&self) -> Result<Option<String>> {
        Ok(self.inventory.table.selection().active().get()?)
    }
    fn set_column(&self, field: Field) -> Result {
        Ok(self
            .inventory
            .table
            .set_active_column(Some(field.key().to_owned()))?)
    }
}

pub fn run(items: usize) -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1200.0, 800.0, 1.0)?)?;
    let clipboard = Memory::default();
    ui.set_clipboard(clipboard.clone())?;
    let start = Instant::now();
    let (inventory, _commands, root) = app::build(&mut ui, items)?;
    let mut s = Session {
        ui,
        root,
        inventory: inventory.clone(),
    };
    s.driver().settle()?;
    report(
        &format!("build and first layout, {items} items"),
        start.elapsed(),
    );
    let mounted = s.ui.inspect().components.len();
    check(mounted < 3_000, &format!("{mounted} components mounted"))?;

    // A click makes a cell active; Enter edits it with the value selected,
    // and Enter saves and returns to the table.
    let body = s.widget("body")?;
    let rows = s.ui.children(&body)?;
    let row = s.ui.key(rows.get(1).ok_or("a second row")?)?.to_owned();
    s.click(s.cell(&row, "name")?)?;
    check(
        s.active()? == Some(row.clone()),
        "a click makes the row active",
    )?;
    let start = Instant::now();
    s.driver().key(KeyCode::Enter)?;
    report("Enter opens the editor", start.elapsed());
    check(s.editing_is(&row, Field::Name)?, "Enter edits the name")?;
    s.driver().type_text("Renamed item")?;
    let start = Instant::now();
    s.driver().key(KeyCode::Enter)?;
    report("Enter saves", start.elapsed());
    check(
        s.item(&row, |item| item.name.clone())? == "Renamed item",
        "typing replaced the selected name and Enter saved it",
    )?;
    check(
        s.editing()?.is_none() && s.focused_key().as_deref() == Some("body"),
        "the table has focus again",
    )?;

    // Tab moves along the row; typed digits replace a number's value.
    s.driver().key(KeyCode::Enter)?;
    s.driver().key(KeyCode::Tab)?;
    check(
        s.editing_is(&row, Field::Category)?,
        "Tab moves to the category",
    )?;
    s.driver().key(KeyCode::Tab)?;
    check(
        s.editing_is(&row, Field::Quantity)?,
        "Tab moves to the quantity",
    )?;
    s.driver().type_text("12")?;
    s.driver().key(KeyCode::Tab)?;
    check(
        s.item(&row, |item| item.quantity)? == 12,
        "the quantity is 12, not appended to the old value",
    )?;
    check(s.editing_is(&row, Field::Price)?, "Tab moves to the price")?;
    let price = s.item(&row, |item| item.cents)?;
    s.driver().type_text("1")?;
    s.driver().key(KeyCode::Escape)?;
    check(
        s.editing()?.is_none() && s.item(&row, |item| item.cents)? == price,
        "Escape cancels in one press",
    )?;

    // Typing on the active cell begins an edit with that text.
    s.set_column(Field::Quantity)?;
    s.driver().type_text("7")?;
    check(
        s.editing_is(&row, Field::Quantity)? && s.focused_text().as_deref() == Some("7"),
        "typing begins an edit",
    )?;
    s.driver().key(KeyCode::Enter)?;
    check(
        s.item(&row, |item| item.quantity)? == 7,
        "and Enter saves it",
    )?;

    // An empty name is refused and stays open for correction.
    s.set_column(Field::Name)?;
    s.driver().key(KeyCode::Enter)?;
    s.driver().key(KeyCode::Backspace)?;
    s.driver().key(KeyCode::Enter)?;
    check(
        s.editing_is(&row, Field::Name)?,
        "an empty name is refused and stays open",
    )?;
    s.driver().key(KeyCode::Escape)?;
    check(
        s.item(&row, |item| item.name.clone())? == "Renamed item",
        "Escape keeps the saved name",
    )?;

    // A double-click edits with the caret at the end.
    let other = s.ui.key(rows.get(2).ok_or("a third row")?)?.to_owned();
    let point = s.cell(&other, "name")?;
    s.click(point)?;
    s.click(point)?;
    check(s.editing_is(&other, Field::Name)?, "a double-click edits")?;
    let name = s.item(&other, |item| item.name.clone())?;
    s.driver().type_text("!")?;
    check(
        s.focused_text() == Some(format!("{name}!")),
        "with the caret at the end",
    )?;
    s.driver().key(KeyCode::Escape)?;

    // The table is one Tab stop: F2 reaches the checkbox, Escape returns.
    s.set_column(Field::Active)?;
    let active = s.item(&other, |item| item.active)?;
    s.driver().key(KeyCode::F2)?;
    check(
        s.focused_key().as_deref() == Some("check"),
        "F2 reaches the checkbox",
    )?;
    s.driver().key(KeyCode::Space)?;
    check(
        s.item(&other, |item| item.active)? != active,
        "Space toggles it in the store",
    )?;
    s.driver().key(KeyCode::Escape)?;
    check(
        s.focused_key().as_deref() == Some("body"),
        "Escape returns to the table",
    )?;
    s.driver().key(KeyCode::Tab)?;
    check(!s.focus_in_table()?, "Tab leaves the table in one step")?;

    // Undo restores the store, and the mounted checkbox follows it.
    let start = Instant::now();
    s.driver().chord(KeyChord::primary('z'), KeyCode::KeyZ)?;
    report("undo", start.elapsed());
    check(
        s.item(&other, |item| item.active)? == active,
        "undo restores the store",
    )?;
    let check_box = {
        let row =
            s.ui.find_descendant(&body, &other)?
                .ok_or("the row is mounted")?;
        s.ui.find_descendant(&row, "check")?.ok_or("a checkbox")?
    };
    let id = s.ui.inspection_id(&check_box)?;
    let shown =
        s.ui.query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
            .components
            .pop()
            .and_then(|c| c.details.properties.get("checked").cloned());
    check(
        shown == Some(InspectValue::Bool(active)),
        "the checkbox shows the restored value",
    )?;

    // Copy and paste rows as text, then delete them.
    let count = inventory.store.borrow().items.len();
    inventory
        .table
        .selection()
        .select([row.clone(), other.clone()])?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    s.driver().chord(KeyChord::primary('c'), KeyCode::KeyC)?;
    check(clipboard.0.borrow().lines().count() == 2, "two rows copied")?;
    let start = Instant::now();
    s.driver().chord(KeyChord::primary('v'), KeyCode::KeyV)?;
    report("paste two rows", start.elapsed());
    check(
        inventory.store.borrow().items.len() == count + 2,
        "two rows pasted",
    )?;
    s.driver().key(KeyCode::Delete)?;
    check(
        inventory.store.borrow().items.len() == count,
        "Delete removes the pasted rows",
    )?;

    // A rename that a sort moves keeps the edit on its row.
    s.inventory.sort(Some(pert_ui::TableSort {
        column: "name".into(),
        direction: pert_ui::SortDirection::Ascending,
    }))?;
    s.driver().settle()?;
    inventory.table.selection().set_active(Some(row.clone()))?;
    s.set_column(Field::Name)?;
    let _ = inventory.table.reveal(row.clone())?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    s.driver().key(KeyCode::Enter)?;
    s.driver().type_text("Aaa first")?;
    let start = Instant::now();
    s.driver().key(KeyCode::Tab)?;
    report("save a sorted rename and move", start.elapsed());
    check(
        s.editing_is(&row, Field::Category)?,
        "Tab stays on the renamed row after the sort moved it",
    )?;
    s.driver().key(KeyCode::Escape)?;
    // The editor gives focus back on Escape, on the cell it edited; a
    // dropdown cell chooses with the keyboard.
    s.ui.focus(&body)?;
    s.set_column(Field::Category)?;
    let id = ItemId::from_key(&row).ok_or("an item key")?;
    inventory.set(
        id,
        Field::Category,
        crate::model::Value::Category(Category::Hardware),
    )?;
    s.driver().settle()?;
    let expected = Category::Software;
    s.driver().key(KeyCode::Enter)?;
    s.driver().key(KeyCode::ArrowDown)?;
    s.driver().key(KeyCode::ArrowDown)?;
    s.driver().key(KeyCode::Enter)?;
    check(
        s.item(&row, |item| item.category)? == expected
            && s.editing()?.is_none()
            && s.focused_key().as_deref() == Some("body"),
        "a dropdown cell saves the chosen category and returns",
    )?;

    // A price saves in cents; Shift+Tab moves back along the row.
    s.set_column(Field::Price)?;
    s.driver().key(KeyCode::Enter)?;
    s.driver().type_text("12.5")?;
    s.driver().key(KeyCode::Enter)?;
    check(
        s.item(&row, |item| item.cents)? == 1250,
        "a price saves in cents",
    )?;
    s.driver().key(KeyCode::Enter)?;
    let back = KeyChord::new(
        CommandKey::Named(KeyCode::Tab),
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    s.driver().chord(back, KeyCode::Tab)?;
    check(
        s.editing_is(&row, Field::Quantity)?,
        "Shift+Tab moves back to the quantity",
    )?;
    s.driver().key(KeyCode::Escape)?;

    // Undo and redo the price.
    s.driver().chord(KeyChord::primary('z'), KeyCode::KeyZ)?;
    let undone = s.item(&row, |item| item.cents)?;
    s.driver().chord(KeyChord::primary('y'), KeyCode::KeyY)?;
    check(
        undone != 1250 && s.item(&row, |item| item.cents)? == 1250,
        "undo and redo the price",
    )?;

    // A refused edit stays open when focus leaves, until it is reached again.
    s.set_column(Field::Name)?;
    s.driver().key(KeyCode::Enter)?;
    s.driver().key(KeyCode::Backspace)?;
    s.driver().key(KeyCode::Enter)?;
    let title = s.widget("title")?;
    s.ui.focus(&title)?;
    s.driver().settle()?;
    check(
        s.editing_is(&row, Field::Name)?,
        "a refused edit stays open when focus leaves",
    )?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    s.driver().key(KeyCode::Enter)?;
    check(
        s.focused_text().as_deref() == Some(""),
        "Enter reaches the refused draft again",
    )?;
    s.driver().key(KeyCode::Escape)?;
    check(
        s.editing()?.is_none() && s.item(&row, |item| item.name.clone())? == "Aaa first",
        "Escape discards it",
    )?;

    // The toolbar's Add button and the Duplicate shortcut.
    let count = inventory.store.borrow().items.len();
    let add = s.widget("add")?;
    s.driver().click(&add)?;
    check(
        inventory.store.borrow().items.len() == count + 1,
        "the Add button adds an item",
    )?;
    s.driver().chord(KeyChord::primary('d'), KeyCode::KeyD)?;
    check(
        inventory.store.borrow().items.len() == count + 2,
        "the Duplicate shortcut copies the selected item",
    )?;

    // Copied and pasted values, and a paste that is refused whole.
    inventory.table.selection().select([row.clone()])?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    s.driver().chord(KeyChord::primary('c'), KeyCode::KeyC)?;
    check(
        *clipboard.0.borrow() == s.item(&row, crate::model::Item::tsv)?,
        "a copied row is its tab-separated fields",
    )?;
    "Widget\tService\t3\t2.50\tno".clone_into(&mut clipboard.0.borrow_mut());
    s.driver().chord(KeyChord::primary('v'), KeyCode::KeyV)?;
    let pasted = inventory
        .store
        .borrow()
        .items
        .iter()
        .find(|item| item.name == "Widget")
        .map(|item| (item.category, item.quantity, item.cents, item.active));
    check(
        pasted == Some((Category::Service, 3, 250, false)),
        "a pasted row has the copied values",
    )?;
    let count = inventory.store.borrow().items.len();
    "Gadget\tHardware\t1\t1.00\tmaybe".clone_into(&mut clipboard.0.borrow_mut());
    s.driver().chord(KeyChord::primary('v'), KeyCode::KeyV)?;
    check(
        inventory.store.borrow().items.len() == count
            && inventory.status.get()?.starts_with("Paste refused"),
        "a paste with an invalid field is refused whole",
    )?;

    // Row commands act on the selected rows the table shows: a row hidden by
    // the category filter or by the search is not deleted.
    inventory.table.selection().select([row.clone()])?;
    let category = s.item(&row, |item| item.category)?;
    let other_category = Category::ALL
        .into_iter()
        .find(|c| *c != category)
        .ok_or("another category")?;
    inventory.category.set(Some(other_category))?;
    inventory.publish()?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    let count = inventory.store.borrow().items.len();
    s.driver().key(KeyCode::Delete)?;
    check(
        !inventory.has_targets.get()? && inventory.store.borrow().items.len() == count,
        "a row the category filter hides is not a target",
    )?;
    inventory.category.set(None)?;
    inventory.publish()?;
    inventory.query.set("no item is called this".to_owned())?;
    s.driver().settle()?;
    s.ui.focus(&body)?;
    s.driver().settle()?;
    s.driver().key(KeyCode::Delete)?;
    check(
        !inventory.has_targets.get()? && inventory.store.borrow().items.len() == count,
        "a row the search hides is not a target",
    )?;
    inventory.query.set(String::new())?;
    s.driver().settle()?;
    check(inventory.has_targets.get()?, "shown again, it is a target")?;

    // The store checks a group in order before applying any of it.
    let id = ItemId::from_key(&row).ok_or("an item key")?;
    let count = inventory.store.borrow().items.len();
    {
        use crate::model::{EditError, Op, Value};
        let mut store = inventory.store.borrow_mut();
        let mismatched = store.edit(vec![Op::Set {
            id,
            field: Field::Name,
            value: Value::Bool(true),
        }]);
        let removed_first = store.edit(vec![
            Op::Remove(vec![id]),
            Op::Set {
                id,
                field: Field::Quantity,
                value: Value::Int(1),
            },
        ]);
        check(
            matches!(mismatched, Err(EditError::Invalid(_)))
                && removed_first == Err(EditError::UnknownItem(id))
                && store.items.len() == count,
            "the store refuses a mismatched value and a group that removes first",
        )?;
    }
    "Huge\tHardware\t9223372036854775807\t1.00\tyes".clone_into(&mut clipboard.0.borrow_mut());
    s.driver().chord(KeyChord::primary('v'), KeyCode::KeyV)?;
    check(
        inventory.store.borrow().items.len() == count,
        "a pasted quantity out of range is refused",
    )?;

    println!("status: {}", inventory.status.get()?);
    println!("totals: {}", inventory.totals.get()?);
    Ok(())
}
