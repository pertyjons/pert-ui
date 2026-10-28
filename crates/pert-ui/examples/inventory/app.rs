//! The inventory editor: a table edited in place, a toolbar of commands,
//! filters and a summary over an application-owned store and undo history.
use crate::model::{self, Category, EditError, Field, Item, ItemId, Op, Store, Value};
use pert_ui::{
    ApplicationError, CellEdit, CollectionFilter, CommandId, CommandScope, FilterMatcher, Icon,
    InlineInteger, InlineReadout, InlineSelect, InlineText, InlineToggle, InputRejection, KeyChord,
    SortDirection, TableColumn, TableController, TableItems, TableSort, TableSource, ToolGroup,
    Toolbar, Tooltip,
};
use pert_ui::{prelude::*, windowing::*};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    rc::{Rc, Weak},
};

#[derive(Debug)]
pub enum Error {
    Ui(RuntimeError),
    Missing(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ui(error) => error.fmt(f),
            Self::Missing(what) => write!(f, "{what} is missing"),
        }
    }
}
impl std::error::Error for Error {}
impl<E: Into<RuntimeError>> From<E> for Error {
    fn from(error: E) -> Self {
        Self::Ui(error.into())
    }
}

/// An application failure as the runtime reports it from a callback.
fn failure(error: impl std::error::Error + Send + Sync + 'static) -> RuntimeError {
    ApplicationError::new(error).into()
}

/// The rows the table shows, in display order. The table asks it for cells
/// of the rows it mounts only.
struct Rows {
    inventory: Weak<Inventory>,
    order: Rc<[ItemId]>,
    /// Each shown item's position, for `index_of`, which Tab and reveal ask.
    positions: Rc<HashMap<ItemId, usize>>,
}
impl Rows {
    fn item<T>(&self, index: usize, read: impl FnOnce(&Item) -> T) -> Option<T> {
        let inventory = self.inventory.upgrade()?;
        let store = inventory.store.borrow();
        store.get(*self.order.get(index)?).map(read)
    }
}
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.order.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.order.get(index).map(|id| id.key())
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.positions.get(&ItemId::from_key(key)?).copied()
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        if column == Field::Active.key() {
            // Controlled: the cell's description carries the item's value, so
            // undo or a paste that changes it shows in the next flush.
            let (id, active) = self.item(index, |item| (item.id, item.active))?;
            let inventory = self.inventory.clone();
            return Some(
                Checkbox::controlled(active, "", move |_, active| match inventory.upgrade() {
                    Some(inventory) => inventory.set(id, Field::Active, Value::Bool(active)),
                    None => Ok(()),
                })
                .key("check"),
            );
        }
        let text = self.text(index, column)?;
        Some(
            Label::new(text)
                .overflow(TextOverflow::Ellipsis)
                .key("text"),
        )
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        self.item(index, |item| cell_text(item, column))
    }
}

/// What a cell shows as text, which a search also matches.
fn cell_text(item: &Item, column: &str) -> String {
    match column {
        "name" => item.name.clone(),
        "category" => item.category.name().to_owned(),
        "quantity" => item.quantity.to_string(),
        "price" => model::money(i128::from(item.cents)),
        "total" => model::money(item.total_cents()),
        "active" => if item.active { "yes" } else { "no" }.to_owned(),
        _ => String::new(),
    }
}

pub struct Inventory {
    me: Weak<Self>,
    pub store: RefCell<Store>,
    pub table: TableController,
    sort: RefCell<Option<TableSort>>,
    order: RefCell<Rc<[ItemId]>>,
    pub title: Signal<String>,
    pub category: Signal<Option<Category>>,
    pub active_only: Signal<bool>,
    pub query: Signal<String>,
    pub vat: Signal<i64>,
    pub status: Signal<String>,
    pub totals: Signal<String>,
    pub can_undo: Signal<bool>,
    pub can_redo: Signal<bool>,
    /// Whether a selected row is shown, after every filter including the
    /// search: the rows the row commands act on.
    pub has_targets: ReadSignal<bool>,
    /// Bumped by each publication, so `has_targets` follows the rows.
    published: Signal<u64>,
}

impl Inventory {
    pub fn new(ui: &Runtime, store: Store) -> Result<Rc<Self>, Error> {
        let width = |pixels: f32| Length::new(pixels).map_err(Error::from);
        let columns = vec![
            TableColumn::new("name", "Name")
                .flex(1.0)?
                .resizable(true)
                .sortable(true)
                .editable(true),
            TableColumn::new("category", "Category")
                .width(width(120.0)?)
                .sortable(true)
                .editable(true),
            TableColumn::new("quantity", "Qty")
                .width(width(80.0)?)
                .sortable(true)
                .editable(true),
            TableColumn::new("price", "Price")
                .width(width(100.0)?)
                .sortable(true)
                .editable(true),
            TableColumn::new("total", "Total")
                .width(width(110.0)?)
                .sortable(true),
            TableColumn::new("active", "Active").width(width(70.0)?),
        ];
        let selection = ListSelection::new(ui, SelectionMode::Multiple);
        let query = ui.signal(String::new());
        let published = ui.signal(0);
        // Set once the inventory exists; until then nothing is a target.
        let handle: Rc<RefCell<Weak<Self>>> = Rc::default();
        let reach = handle.clone();
        let has_targets = selection
            .selected()
            .combine(&query.read_only(), |keys, query| {
                (keys.clone(), query.clone())
            })?
            .combine(&published.read_only(), move |(keys, query), _| {
                reach
                    .borrow()
                    .upgrade()
                    .is_some_and(|inventory| !inventory.targets(keys, query).is_empty())
            })?;
        let table = TableController::new(
            ui,
            selection,
            columns,
            TableSource::new(Rows {
                inventory: Weak::new(),
                order: Rc::from([]),
                positions: Rc::default(),
            }),
        )?;
        let inventory = Rc::new_cyclic(|me| Self {
            me: me.clone(),
            store: RefCell::new(store),
            table,
            sort: RefCell::new(None),
            order: RefCell::new(Rc::from([])),
            title: ui.signal("Office inventory".to_owned()),
            category: ui.signal(None),
            active_only: ui.signal(false),
            query,
            vat: ui.signal(25),
            status: ui.signal("Ready".to_owned()),
            totals: ui.signal(String::new()),
            can_undo: ui.signal(false),
            can_redo: ui.signal(false),
            has_targets,
            published,
        });
        *handle.borrow_mut() = Rc::downgrade(&inventory);
        inventory.publish()?;
        Ok(inventory)
    }

    /// The editor for one cell, built when the table begins its edit. Its
    /// signals belong to the edit; the store changes only when it saves.
    fn editor(&self, edit: &mut CellEdit<'_>) -> Result<Element, RuntimeError> {
        let missing = || failure(EditError::Invalid(format!("no cell {}", edit.row())));
        let id = ItemId::from_key(edit.row()).ok_or_else(missing)?;
        let field = Field::from_key(edit.column()).ok_or_else(missing)?;
        let item = self.store.borrow().get(id).cloned().ok_or_else(missing)?;
        let me = self.me.clone();
        let save = move |value: Value| match me.upgrade() {
            Some(inventory) => inventory.set(id, field, value),
            None => Ok(()),
        };
        let session = edit.session().clone();
        let owner = edit.owner();
        Ok(match field {
            Field::Name => Element::new(
                "editor",
                TextField::new(owner.signal(item.name))
                    .normalize(|text| {
                        let text = text.trim().to_owned();
                        if text.is_empty() {
                            Err(InputRejection::new("A name cannot be empty"))
                        } else {
                            Ok(text)
                        }
                    })
                    .on_save(move |_, text| save(Value::Text(text.to_owned())))
                    .session(&session),
            ),
            Field::Quantity => Element::new(
                "editor",
                IntegerInput::new(owner.signal(item.quantity))
                    .range(0, model::MAX_QUANTITY)
                    .map_err(failure)?
                    .on_save(move |_, quantity| save(Value::Int(quantity)))
                    .session(&session),
            ),
            Field::Price => Element::new(
                "editor",
                DecimalInput::new(owner.signal(item.cents as f64 / 100.0))
                    .range(0.0, model::MAX_CENTS as f64 / 100.0)
                    .map_err(failure)?
                    .on_save(move |_, price| save(Value::Cents((price * 100.0).round() as i64)))
                    .session(&session),
            ),
            Field::Category => {
                let value = owner.signal(item.category);
                Element::new(
                    "editor",
                    Select::new(
                        owner,
                        value,
                        Category::ALL.map(|category| Choice::new(category, category.name())),
                    )?
                    .on_change(move |_, category| save(Value::Category(*category)))
                    .session(&session),
                )
            }
            Field::Active => Element::new("editor", Label::new("")),
        })
    }

    /// Writes one field through the store, as one undo step.
    pub fn set(&self, id: ItemId, field: Field, value: Value) -> Result<(), RuntimeError> {
        let unchanged =
            self.store.borrow().get(id).map(|item| item.get(field)) == Some(value.clone());
        if unchanged {
            return Ok(());
        }
        self.apply(vec![Op::Set { id, field, value }], "Edited")
    }

    fn apply(&self, ops: Vec<Op>, what: &str) -> Result<(), RuntimeError> {
        let result = self.store.borrow_mut().edit(ops);
        match result {
            Ok(()) => self.status.set(what.to_owned())?,
            Err(error) => self.status.set(format!("Not applied: {error}"))?,
        }
        self.changed()
    }

    /// After any change of the store: the history state and the rows.
    fn changed(&self) -> Result<(), RuntimeError> {
        {
            let store = self.store.borrow();
            self.can_undo.set_changed(store.can_undo())?;
            self.can_redo.set_changed(store.can_redo())?;
        }
        self.publish()
    }

    /// Rebuilds the order from the filters and the sort, and publishes it.
    pub fn publish(&self) -> Result<(), RuntimeError> {
        let category = self.category.get()?;
        let active_only = self.active_only.get()?;
        let store = self.store.borrow();
        let mut items: Vec<&Item> = store
            .items
            .iter()
            .filter(|item| category.is_none_or(|c| item.category == c))
            .filter(|item| !active_only || item.active)
            .collect();
        if let Some(sort) = self.sort.borrow().as_ref() {
            items.sort_by(|a, b| {
                let ordering = match sort.column.as_str() {
                    "name" => a.name.cmp(&b.name),
                    "category" => a.category.cmp(&b.category),
                    "quantity" => a.quantity.cmp(&b.quantity),
                    "price" => a.cents.cmp(&b.cents),
                    "total" => a.total_cents().cmp(&b.total_cents()),
                    _ => std::cmp::Ordering::Equal,
                };
                if sort.direction == SortDirection::Descending {
                    ordering.reverse()
                } else {
                    ordering
                }
            });
        }
        let net: i128 = items.iter().map(|item| item.total_cents()).sum();
        let vat = net * i128::from(self.vat.get()?) / 100;
        self.totals.set(format!(
            "{} items · {} + VAT {} = {}",
            items.len(),
            model::money(net),
            model::money(vat),
            model::money(net + vat)
        ))?;
        let order: Rc<[ItemId]> = items.iter().map(|item| item.id).collect();
        drop(store);
        let positions = order
            .iter()
            .enumerate()
            .map(|(position, id)| (*id, position))
            .collect();
        *self.order.borrow_mut() = order.clone();
        self.table.set_source(TableSource::new(Rows {
            inventory: self.me.clone(),
            order,
            positions: Rc::new(positions),
        }))?;
        self.published.update(|count| *count += 1)
    }

    pub fn sort(&self, sort: Option<TableSort>) -> Result<(), RuntimeError> {
        self.sort.borrow_mut().clone_from(&sort);
        self.table.set_sort(sort)?;
        self.publish()
    }

    /// The selected rows the table shows, in display order: what the row
    /// commands act on. A selected row a filter hides stays selected but is
    /// not a target.
    pub fn selected(&self) -> Result<Vec<ItemId>, RuntimeError> {
        let keys = self.table.selection().selected().get()?;
        Ok(self.targets(&keys, &self.query.get()?))
    }
    /// The rows of `keys` shown after the category and Active filters, in
    /// the published order, and after the search `query`, matched as the
    /// table's filter field matches: the row's cell texts joined in column
    /// order, compared by substring.
    fn targets(&self, keys: &std::collections::BTreeSet<String>, query: &str) -> Vec<ItemId> {
        if keys.is_empty() {
            return Vec::new();
        }
        let query = query.trim();
        let matcher = FilterMatcher::substring();
        let columns = self.table.columns();
        let store = self.store.borrow();
        let shown = |item: &Item| {
            let text: Vec<String> = columns
                .iter()
                .map(|column| cell_text(item, column.key()))
                .filter(|text| !text.is_empty())
                .collect();
            query.is_empty() || matcher.matches(query, &text.join(" "))
        };
        self.order
            .borrow()
            .iter()
            .copied()
            .filter(|id| keys.contains(&id.key()))
            .filter(|id| store.get(*id).is_some_and(shown))
            .collect()
    }
    /// Where new rows go: after the active row, or at the end.
    fn insertion_point(&self) -> Result<usize, RuntimeError> {
        let store = self.store.borrow();
        let active = self.table.selection().active().get()?;
        Ok(active
            .and_then(|key| ItemId::from_key(&key))
            .and_then(|id| store.position(id))
            .map_or(store.items.len(), |at| at + 1))
    }
    fn insert(&self, items: Vec<Item>, what: &str) -> Result<(), RuntimeError> {
        let at = self.insertion_point()?;
        let keys: Vec<String> = items.iter().map(|item| item.id.key()).collect();
        let ops = vec![Op::Insert(
            items
                .into_iter()
                .enumerate()
                .map(|(offset, item)| (at + offset, item))
                .collect(),
        )];
        self.store.borrow_mut().edit(ops).map_err(failure)?;
        self.status.set(what.to_owned())?;
        self.changed()?;
        self.table.selection().select(keys.clone())?;
        if let Some(last) = keys.last() {
            self.table.selection().set_active(Some(last.clone()))?;
            self.table
                .set_active_column(Some(Field::Name.key().to_owned()))?;
            let _ = self.table.reveal(last.clone())?;
        }
        Ok(())
    }
    pub fn add(&self) -> Result<(), RuntimeError> {
        let id = self.store.borrow_mut().allocate();
        let item = Item {
            id,
            name: "New item".into(),
            category: self.category.get()?.unwrap_or(Category::Supplies),
            quantity: 1,
            cents: 0,
            active: true,
        };
        self.insert(vec![item], "Added an item")
    }
    pub fn duplicate(&self) -> Result<(), RuntimeError> {
        let selected = self.selected()?;
        let copies: Vec<Item> = {
            let mut store = self.store.borrow_mut();
            let originals: Vec<Item> = selected
                .iter()
                .filter_map(|id| store.get(*id).cloned())
                .collect();
            originals
                .into_iter()
                .map(|mut item| {
                    item.id = store.allocate();
                    item.name = format!("{} (copy)", item.name);
                    item
                })
                .collect()
        };
        if copies.is_empty() {
            return Ok(());
        }
        let count = copies.len();
        self.insert(copies, &format!("Duplicated {count} items"))
    }
    pub fn delete(&self) -> Result<(), RuntimeError> {
        let selected = self.selected()?;
        if selected.is_empty() {
            return Ok(());
        }
        let count = selected.len();
        self.apply(
            vec![Op::Remove(selected)],
            &format!("Deleted {count} items"),
        )?;
        self.table.selection().clear()
    }
    pub fn undo(&self, redo: bool) -> Result<(), RuntimeError> {
        let result = self.store.borrow_mut().step(redo);
        match result {
            Ok(true) => self
                .status
                .set(if redo { "Redone" } else { "Undone" }.to_owned())?,
            Ok(false) => {}
            Err(error) => self.status.set(format!("History failed: {error}"))?,
        }
        self.changed()
    }
    pub fn copy_text(&self) -> Result<String, RuntimeError> {
        let store = self.store.borrow();
        Ok(self
            .selected()?
            .iter()
            .filter_map(|id| store.get(*id).map(Item::tsv))
            .collect::<Vec<_>>()
            .join("\n"))
    }
    pub fn paste_text(&self, text: &str) -> Result<(), RuntimeError> {
        let mut items = Vec::new();
        for (number, line) in text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
        {
            let id = self.store.borrow_mut().allocate();
            match Item::parse_tsv(id, line) {
                Ok(item) => items.push(item),
                Err(why) => {
                    return self
                        .status
                        .set(format!("Paste refused: line {}: {why}", number + 1));
                }
            }
        }
        if items.is_empty() {
            return self.status.set("Nothing to paste".to_owned());
        }
        let count = items.len();
        self.insert(items, &format!("Pasted {count} items"))
    }
}

/// What a command does.
type Action = Box<dyn FnMut(&mut Runtime) -> Result<(), RuntimeError>>;

/// The commands, registered once; shortcuts and toolbar buttons share them.
pub struct Commands {
    pub add: CommandId,
    pub duplicate: CommandId,
    pub delete: CommandId,
    pub undo: CommandId,
    pub redo: CommandId,
    pub copy: CommandId,
    pub paste: CommandId,
}
impl Commands {
    fn register(ui: &mut Runtime, inventory: &Rc<Inventory>) -> Result<Self, RuntimeError> {
        let mut command = |name: &str,
                           chords: Vec<KeyChord>,
                           enabled: Option<ReadSignal<bool>>,
                           mut action: Action|
         -> Result<CommandId, RuntimeError> {
            let id =
                ui.register_command(name, CommandScope::Application, chords, move |ui, _| {
                    action(ui)
                })?;
            if let Some(enabled) = enabled {
                ui.command_enabled_signal(&id, enabled)?;
            }
            Ok(id)
        };
        let selected = Some(inventory.has_targets.clone());
        let (i, d, x, u, r, c, p) = (
            inventory.clone(),
            inventory.clone(),
            inventory.clone(),
            inventory.clone(),
            inventory.clone(),
            inventory.clone(),
            inventory.clone(),
        );
        Ok(Self {
            add: command(
                "Add item",
                vec![KeyChord::primary('n')],
                None,
                Box::new(move |_| i.add()),
            )?,
            duplicate: command(
                "Duplicate",
                vec![KeyChord::primary('d')],
                selected.clone(),
                Box::new(move |_| d.duplicate()),
            )?,
            delete: command(
                "Delete",
                vec![KeyChord::named(KeyCode::Delete)],
                selected.clone(),
                Box::new(move |_| x.delete()),
            )?,
            undo: command(
                "Undo",
                vec![KeyChord::primary('z')],
                Some(inventory.can_undo.read_only()),
                Box::new(move |_| u.undo(false)),
            )?,
            redo: command(
                "Redo",
                vec![KeyChord::primary('z').shifted(), KeyChord::primary('y')],
                Some(inventory.can_redo.read_only()),
                Box::new(move |_| r.undo(true)),
            )?,
            copy: command(
                "Copy rows",
                vec![KeyChord::primary('c')],
                selected,
                Box::new(move |ui| {
                    let text = c.copy_text()?;
                    ui.set_clipboard_text(&text)?;
                    c.status
                        .set(format!("Copied {} rows", text.lines().count()))
                }),
            )?,
            paste: command(
                "Paste rows",
                vec![KeyChord::primary('v')],
                None,
                Box::new(move |ui| {
                    let text = ui.clipboard_text()?;
                    p.paste_text(&text)
                }),
            )?,
        })
    }
}

/// A toolbar button showing `icon` for a command, with its name and
/// shortcut as help.
fn command_button(
    ui: &Runtime,
    key: &str,
    command: &CommandId,
    icon: Icon,
) -> Result<Element, RuntimeError> {
    let button = Button::command_icon(ui, command, icon, |_, _| Ok(()))?.frameless();
    Ok(Element::new(
        format!("{key}-help"),
        Tooltip::command(ui, Element::new(key.to_owned(), button), command)?,
    ))
}

pub fn build(ui: &mut Runtime, items: usize) -> Result<(Rc<Inventory>, Commands, WidgetId), Error> {
    let inventory = Inventory::new(ui, Store::generated(items))?;
    let commands = Commands::register(ui, &inventory)?;

    let (sorting, editing) = (inventory.clone(), Rc::downgrade(&inventory));
    // The rows hold the Active checkbox and the editors, which measure the
    // theme's ControlHeight; the example offers no density switch.
    let theme = ui.theme();
    let row_height = theme
        .metrics(theme.density()?)?
        .metric(MetricRole::ControlHeight);
    let table = Table::new(&inventory.table, row_height)?
        // One Tab stop: F2 reaches the Active checkbox, Tab leaves the table.
        .cell_tab_stops(false)
        .on_sort(move |_, sort| sorting.sort(Some(sort.clone())))
        .cell_editor(move |edit| match editing.upgrade() {
            Some(inventory) => inventory.editor(edit),
            None => Err(failure(EditError::Invalid("the inventory closed".into()))),
        });
    let filtered = CollectionFilter::with_query(
        inventory.query.clone(),
        inventory.table.clone(),
        Element::new("table", table),
    );

    let category_filter = InlineSelect::new(
        ui,
        inventory.category.clone(),
        std::iter::once(Choice::new(None, "All"))
            .chain(Category::ALL.map(|category| Choice::new(Some(category), category.name()))),
    )?
    .label("Category");
    let refilter = inventory.clone();
    let category_filter = category_filter.on_change(move |_, _| refilter.publish());
    let refilter = inventory.clone();
    let active_only = InlineToggle::new(inventory.active_only.clone(), "Active only")
        .icon(Icon::Check)
        .on_change(move |_, _| refilter.publish());
    let revat = inventory.clone();
    let vat = InlineInteger::new(inventory.vat.clone())
        .label("VAT")
        .unit("%")
        .range(0, 100)
        .map_err(failure)?
        .on_save(move |_, _| revat.publish());

    let toolbar = Toolbar::new(
        ui,
        [
            ToolGroup::new(
                "document",
                [Element::new(
                    "title",
                    InlineText::new(inventory.title.clone()).placeholder("Untitled inventory"),
                )],
            )
            .essential(),
            ToolGroup::separator("s1"),
            ToolGroup::new(
                "rows",
                [
                    command_button(ui, "add", &commands.add, Icon::Add)?,
                    command_button(ui, "duplicate", &commands.duplicate, Icon::Duplicate)?,
                    command_button(ui, "delete", &commands.delete, Icon::Delete)?,
                ],
            ),
            ToolGroup::new(
                "history",
                [
                    command_button(ui, "undo", &commands.undo, Icon::Undo)?,
                    command_button(ui, "redo", &commands.redo, Icon::Redo)?,
                ],
            ),
            ToolGroup::new(
                "clipboard",
                [
                    command_button(ui, "copy", &commands.copy, Icon::Copy)?,
                    command_button(ui, "paste", &commands.paste, Icon::Paste)?,
                ],
            )
            .collapse_first(),
            ToolGroup::separator("s2"),
            ToolGroup::new(
                "filters",
                [
                    Element::new("category-filter", category_filter),
                    Element::new("active-only", active_only),
                ],
            ),
            ToolGroup::spacer("spring"),
            ToolGroup::new("vat-group", [Element::new("vat", vat)]),
        ],
    )?;

    let root = ui.mount_tree(
        RegionLayout::new()
            .fixed()
            .top(Element::new("toolbar", toolbar))
            .center(Element::new("filtered", filtered))
            .bottom(Element::new(
                "status",
                Row::new([
                    Element::new("message", Label::new(inventory.status.read_only())).with_grow(1),
                    Element::new("totals", InlineReadout::new(inventory.totals.read_only())),
                ]),
            ))
            .key("inventory"),
    )?;
    ui.find_descendant(&root, "table")?
        .ok_or(Error::Missing("the table"))?;
    Ok((inventory, commands, root))
}
