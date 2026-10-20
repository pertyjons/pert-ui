//! The application-owned inventory: items, validated edits and an undo
//! history. Nothing here knows about the user interface.
use std::{collections::HashSet, fmt};

/// The largest quantity of one item.
pub const MAX_QUANTITY: i64 = 1_000_000;
/// The largest price of one item, in cents.
pub const MAX_CENTS: i64 = 100_000_000;

/// An item's identity, independent of where it is stored or shown.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct ItemId(u64);
impl ItemId {
    /// The table row key.
    pub fn key(self) -> String {
        format!("i{}", self.0)
    }
    pub fn from_key(key: &str) -> Option<Self> {
        key.strip_prefix('i')?.parse().ok().map(Self)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Category {
    Hardware,
    Software,
    Service,
    Supplies,
}
impl Category {
    pub const ALL: [Self; 4] = [
        Self::Hardware,
        Self::Software,
        Self::Service,
        Self::Supplies,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Hardware => "Hardware",
            Self::Software => "Software",
            Self::Service => "Service",
            Self::Supplies => "Supplies",
        }
    }
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.name().eq_ignore_ascii_case(text.trim()))
    }
}

/// An edited property of an item; its key is the table column's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Field {
    Name,
    Category,
    Quantity,
    Price,
    Active,
}
impl Field {
    pub const ALL: [Self; 5] = [
        Self::Name,
        Self::Category,
        Self::Quantity,
        Self::Price,
        Self::Active,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Category => "category",
            Self::Quantity => "quantity",
            Self::Price => "price",
            Self::Active => "active",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|field| field.key() == key)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Text(String),
    Category(Category),
    Int(i64),
    /// Cents, so sums are exact.
    Cents(i64),
    Bool(bool),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub category: Category,
    pub quantity: i64,
    pub cents: i64,
    pub active: bool,
}
impl Item {
    /// The item's total. Validated quantities and prices keep it far from
    /// overflowing, and sums over items use `i128`.
    pub fn total_cents(&self) -> i128 {
        i128::from(self.quantity) * i128::from(self.cents)
    }
    pub fn get(&self, field: Field) -> Value {
        match field {
            Field::Name => Value::Text(self.name.clone()),
            Field::Category => Value::Category(self.category),
            Field::Quantity => Value::Int(self.quantity),
            Field::Price => Value::Cents(self.cents),
            Field::Active => Value::Bool(self.active),
        }
    }
    fn set(&mut self, value: Value) {
        match value {
            Value::Text(name) => self.name = name,
            Value::Category(category) => self.category = category,
            Value::Int(quantity) => self.quantity = quantity,
            Value::Cents(cents) => self.cents = cents,
            Value::Bool(active) => self.active = active,
        }
    }
    /// One tab-separated line, as copied to the clipboard.
    pub fn tsv(&self) -> String {
        format!(
            "{}\t{}\t{}\t{}\t{}",
            self.name,
            self.category.name(),
            self.quantity,
            money(i128::from(self.cents)),
            if self.active { "yes" } else { "no" }
        )
    }
    /// Parses a copied line into a new item with identity `id`.
    pub fn parse_tsv(id: ItemId, line: &str) -> Result<Self, EditError> {
        let invalid = |why: String| EditError::Invalid(why);
        let parts: Vec<&str> = line.split('\t').collect();
        let [name, category, quantity, price, active] = parts[..] else {
            return Err(invalid(format!("expected 5 columns, got {}", parts.len())));
        };
        let item = Self {
            id,
            name: name.trim().to_owned(),
            category: Category::parse(category)
                .ok_or_else(|| invalid(format!("unknown category {category:?}")))?,
            quantity: quantity
                .trim()
                .parse()
                .map_err(|_| invalid(format!("bad quantity {quantity:?}")))?,
            cents: parse_money(price).ok_or_else(|| invalid(format!("bad price {price:?}")))?,
            active: match active.trim().to_ascii_lowercase().as_str() {
                "yes" | "true" | "1" => true,
                "no" | "false" | "0" => false,
                _ => return Err(invalid(format!("bad active flag {active:?}"))),
            },
        };
        for field in Field::ALL {
            validate(field, &item.get(field))?;
        }
        Ok(item)
    }
}

pub fn money(cents: i128) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    format!("{sign}{}.{:02}", cents / 100, cents % 100)
}
pub fn parse_money(text: &str) -> Option<i64> {
    let value: f64 = text.trim().parse().ok()?;
    (value.is_finite() && value.abs() < 1e12).then(|| (value * 100.0).round() as i64)
}

/// One undoable change.
#[derive(Clone, Debug)]
pub enum Op {
    Set {
        id: ItemId,
        field: Field,
        value: Value,
    },
    /// Inserts items at their positions, in ascending order.
    Insert(Vec<(usize, Item)>),
    Remove(Vec<ItemId>),
}

#[derive(Debug, PartialEq)]
pub enum EditError {
    UnknownItem(ItemId),
    Invalid(String),
}
impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownItem(id) => write!(f, "item {} no longer exists", id.0),
            Self::Invalid(why) => f.write_str(why),
        }
    }
}
impl std::error::Error for EditError {}

#[derive(Default)]
pub struct Store {
    pub items: Vec<Item>,
    next_id: u64,
    undo: Vec<Vec<Op>>,
    redo: Vec<Vec<Op>>,
}

impl Store {
    pub fn generated(count: usize) -> Self {
        let mut store = Self::default();
        let words = [
            "Cable", "Laptop", "License", "Support", "Paper", "Monitor", "Toner", "Hosting",
        ];
        for index in 0..count {
            let id = store.allocate();
            store.items.push(Item {
                id,
                name: format!("{} {}", words[index % words.len()], index + 1),
                category: Category::ALL[index * 7 % 4],
                quantity: (index * 13 % 40) as i64 + 1,
                cents: ((index * 7919) % 50_000) as i64 + 99,
                active: index % 5 != 0,
            });
        }
        store
    }
    pub fn allocate(&mut self) -> ItemId {
        let id = ItemId(self.next_id);
        self.next_id += 1;
        id
    }
    pub fn position(&self, id: ItemId) -> Option<usize> {
        self.items.iter().position(|item| item.id == id)
    }
    pub fn get(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Checks a group in order, each operation against the store as the ones
    /// before it leave it, so a group is checked before any of it applies
    /// and never half applies.
    fn check(&self, ops: &[Op]) -> Result<(), EditError> {
        let mut removed = HashSet::new();
        let mut inserted = HashSet::new();
        for op in ops {
            match op {
                Op::Set { id, field, value } => {
                    validate(*field, value)?;
                    let present =
                        inserted.contains(id) || (!removed.contains(id) && self.get(*id).is_some());
                    if !present {
                        return Err(EditError::UnknownItem(*id));
                    }
                }
                Op::Insert(items) => {
                    for (_, item) in items {
                        for field in Field::ALL {
                            validate(field, &item.get(field))?;
                        }
                        removed.remove(&item.id);
                        inserted.insert(item.id);
                    }
                }
                Op::Remove(ids) => {
                    for id in ids {
                        inserted.remove(id);
                        removed.insert(*id);
                    }
                }
            }
        }
        Ok(())
    }
    /// Applies one operation of a checked group and returns its inverse.
    fn apply(&mut self, op: Op) -> Result<Op, EditError> {
        Ok(match op {
            Op::Set { id, field, value } => {
                let item = self
                    .items
                    .iter_mut()
                    .find(|item| item.id == id)
                    .ok_or(EditError::UnknownItem(id))?;
                let old = item.get(field);
                item.set(value);
                Op::Set {
                    id,
                    field,
                    value: old,
                }
            }
            Op::Insert(items) => {
                let ids = items.iter().map(|(_, item)| item.id).collect();
                for (at, item) in items {
                    let at = at.min(self.items.len());
                    self.items.insert(at, item);
                }
                Op::Remove(ids)
            }
            Op::Remove(ids) => {
                let removed = self
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| ids.contains(&item.id))
                    .map(|(position, item)| (position, item.clone()))
                    .collect();
                self.items.retain(|item| !ids.contains(&item.id));
                Op::Insert(removed)
            }
        })
    }
    /// Applies a checked group and returns its inverse, last operation first.
    fn apply_all(&mut self, ops: Vec<Op>) -> Result<Vec<Op>, EditError> {
        let mut inverse = ops
            .into_iter()
            .map(|op| self.apply(op))
            .collect::<Result<Vec<_>, _>>()?;
        inverse.reverse();
        Ok(inverse)
    }
    /// Checks and applies a group of operations as one undo step.
    pub fn edit(&mut self, ops: Vec<Op>) -> Result<(), EditError> {
        self.check(&ops)?;
        let inverse = self.apply_all(ops)?;
        if !inverse.is_empty() {
            self.undo.push(inverse);
            self.redo.clear();
        }
        Ok(())
    }
    /// Undoes (or redoes) the last step; `false` when there is none.
    pub fn step(&mut self, redo: bool) -> Result<bool, EditError> {
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        let Some(ops) = stack.pop() else {
            return Ok(false);
        };
        if let Err(error) = self.check(&ops) {
            // Keep the step for a later attempt rather than losing it.
            let stack = if redo { &mut self.redo } else { &mut self.undo };
            stack.push(ops);
            return Err(error);
        }
        let inverse = self.apply_all(ops)?;
        if redo {
            self.undo.push(inverse);
        } else {
            self.redo.push(inverse);
        }
        Ok(true)
    }
}

/// Whether `value` is a valid value of `field`, of its kind and in range.
pub fn validate(field: Field, value: &Value) -> Result<(), EditError> {
    let invalid = |why: &str| Err(EditError::Invalid(why.to_owned()));
    match (field, value) {
        (Field::Name, Value::Text(name)) if name.trim().is_empty() => {
            invalid("a name cannot be empty")
        }
        (Field::Quantity, Value::Int(quantity)) if !(0..=MAX_QUANTITY).contains(quantity) => {
            invalid("quantity is out of range")
        }
        (Field::Price, Value::Cents(cents)) if !(0..=MAX_CENTS).contains(cents) => {
            invalid("price is out of range")
        }
        (Field::Name, Value::Text(_))
        | (Field::Category, Value::Category(_))
        | (Field::Quantity, Value::Int(_))
        | (Field::Price, Value::Cents(_))
        | (Field::Active, Value::Bool(_)) => Ok(()),
        _ => invalid("the value does not fit the field"),
    }
}
