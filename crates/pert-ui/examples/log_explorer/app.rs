//! The explorer: two tables over one application-owned store, filters, a
//! tree grouped by source, a detail pane, background loading and a live tail.
use crate::model::{self, Entry, Level, LineId, Parsed, Store};
use pert_ui::prelude::*;
use pert_ui::{
    Capacity, CollectionFilter, CommandId, CommandScope, FilterMatcher, KeyChord,
    ListRevealReceipt, ListRevealStatus, Mailbox, NormalizedValue, ProgressBar, ProgressStatus,
    SendError, SortDirection, TableColumn, TableController, TableItems, TableSort, TableSource,
    TreeController, TreeItems, TreeSource, UiSender,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    path::PathBuf,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering as Atomic},
    },
    time::Duration,
};

pub type Shared = Rc<RefCell<Store>>;

/// The rows of one table: positions in the store, in display order.
struct Rows {
    store: Shared,
    order: Rc<[u32]>,
    /// From a line's identity to its row, built by the first lookup, so a
    /// reveal or a restored selection never scans the order.
    rows: OnceCell<HashMap<LineId, usize>>,
}
impl Rows {
    fn new(store: &Shared, order: Rc<[u32]>) -> Self {
        Self {
            store: store.clone(),
            order,
            rows: OnceCell::new(),
        }
    }
    fn entry<T>(&self, index: usize, read: impl FnOnce(&Entry) -> T) -> Option<T> {
        let position = *self.order.get(index)? as usize;
        self.store.borrow().entries.get(position).map(read)
    }
}
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.order.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.entry(index, |entry| entry.id.key())
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        let id = LineId::from_key(key)?;
        let rows = self.rows.get_or_init(|| {
            let store = self.store.borrow();
            self.order
                .iter()
                .enumerate()
                .map(|(row, position)| (store.entries[*position as usize].id, row))
                .collect()
        });
        rows.get(&id).copied()
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        self.entry(index, |entry| {
            let text = match column {
                "line" => (entry.id.0 + 1).to_string(),
                "time" => entry.time.clone(),
                "level" => entry.level.name().to_owned(),
                "source" => entry.source.to_string(),
                _ => entry.preview().to_owned(),
            };
            Label::new(text)
                .overflow(TextOverflow::Ellipsis)
                .key("text")
        })
    }
    /// What the filter field matches: every column a reader sees.
    fn text(&self, index: usize, column: &str) -> Option<String> {
        self.entry(index, |entry| match column {
            "line" => (entry.id.0 + 1).to_string(),
            "time" => entry.time.clone(),
            "level" => entry.level.name().to_owned(),
            "source" => entry.source.to_string(),
            _ => entry.message.clone(),
        })
    }
}

/// Sources as roots, their lines as children.
struct Sources {
    store: Shared,
    lines: Rc<RefCell<BTreeMap<Rc<str>, Vec<LineId>>>>,
}
impl TreeItems for Sources {
    fn roots(&self) -> Vec<String> {
        self.lines
            .borrow()
            .keys()
            .map(|source| format!("s:{source}"))
            .collect()
    }
    /// Every line of a source: the tree mounts a screenful of rows, but
    /// expanding a source still lists all of its lines.
    fn children(&self, key: &str) -> Vec<String> {
        let Some(source) = key.strip_prefix("s:") else {
            return Vec::new();
        };
        self.lines
            .borrow()
            .get(source)
            .map(|lines| lines.iter().map(|id| id.key()).collect())
            .unwrap_or_default()
    }
    fn can_expand(&self, key: &str) -> bool {
        key.starts_with("s:")
    }
    fn build(&self, key: &str) -> Option<Element> {
        if let Some(source) = key.strip_prefix("s:") {
            let count = self.lines.borrow().get(source).map_or(0, Vec::len);
            return Some(Label::new(format!("{source} ({count})")).key("row"));
        }
        let store = self.store.borrow();
        let entry = store.get(LineId::from_key(key)?)?;
        Some(
            Label::new(format!("{} {}", entry.level.name(), entry.preview()))
                .overflow(TextOverflow::Ellipsis)
                .key("row"),
        )
    }
    fn text(&self, key: &str) -> Option<String> {
        if let Some(source) = key.strip_prefix("s:") {
            return Some(source.to_owned());
        }
        let store = self.store.borrow();
        store
            .get(LineId::from_key(key)?)
            .map(|entry| entry.message.clone())
    }
}

/// Which lines a table shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The level and source filters.
    All,
    /// Errors only.
    Errors,
}

/// One table: its controller, its own sort and the order it shows.
pub struct View {
    pub kind: Kind,
    pub controller: TableController,
    sort: RefCell<Option<TableSort>>,
    order: RefCell<Vec<u32>>,
}
impl View {
    fn new(ui: &Runtime, store: &Shared, kind: Kind, mode: SelectionMode) -> Result<Self, Error> {
        Ok(Self {
            kind,
            controller: TableController::new(
                ui,
                columns()?,
                TableSource::new(Rows::new(store, Rc::from([]))),
                ListSelection::new(ui, mode),
            )?,
            sort: RefCell::new(None),
            order: RefCell::default(),
        })
    }
    pub fn shown(&self) -> usize {
        self.order.borrow().len()
    }
    /// The key of the last row this table shows.
    pub fn last_key(&self, store: &Store) -> Option<String> {
        let last = *self.order.borrow().last()?;
        store.entries.get(last as usize).map(|entry| entry.id.key())
    }
}

fn columns() -> Result<Vec<TableColumn>, Error> {
    let width = |pixels: f32| Length::new(pixels).map_err(Error::from);
    Ok(vec![
        TableColumn::new("line", "Line")
            .width(width(70.0)?)
            .sortable(true),
        TableColumn::new("time", "Time")
            .width(width(110.0)?)
            .resizable(true)
            .sortable(true),
        TableColumn::new("level", "Level")
            .width(width(70.0)?)
            .resizable(true)
            .sortable(true),
        TableColumn::new("source", "Source")
            .width(width(100.0)?)
            .resizable(true)
            .sortable(true),
        TableColumn::new("message", "Message")
            .flex(1.0)?
            .resizable(true)
            .sortable(true),
    ])
}

/// Orders two lines by a table's sort, then by identity, so equal keys keep a
/// stable order.
fn compare(sort: Option<&TableSort>, a: &Entry, b: &Entry) -> Ordering {
    let Some(sort) = sort else {
        return a.id.cmp(&b.id);
    };
    let ordering = match sort.column.as_str() {
        "time" => a.time.cmp(&b.time),
        "level" => a.level.cmp(&b.level),
        "source" => a.source.cmp(&b.source),
        "message" => a.message.cmp(&b.message),
        _ => a.id.cmp(&b.id),
    }
    .then(a.id.cmp(&b.id));
    if sort.direction == SortDirection::Descending {
        ordering.reverse()
    } else {
        ordering
    }
}

/// Why a file could not be loaded.
#[derive(Debug)]
pub enum LoadError {
    Open(PathBuf, std::io::Error),
    Read { line: u64, error: std::io::Error },
}
impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(path, error) => write!(f, "cannot open {}: {error}", path.display()),
            Self::Read { line, error } => write!(f, "cannot read line {}: {error}", line + 1),
        }
    }
}

/// What the workers send to the UI thread.
pub enum Message {
    /// Parsed lines of load `load`, with how much of the file was read.
    Lines {
        load: u64,
        lines: Vec<Parsed>,
        skipped: u64,
        progress: f32,
    },
    /// Load `load` read `lines` lines.
    Loaded {
        load: u64,
        lines: u64,
    },
    Failed {
        load: u64,
        error: LoadError,
    },
    /// Lines from tail worker `generation`, started during load `load`.
    Tail {
        load: u64,
        generation: u64,
        lines: Vec<Parsed>,
    },
}

/// The example's errors: the library's, and its own failures as text.
#[derive(Debug)]
pub enum Error {
    Ui(RuntimeError),
    Other(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ui(error) => error.fmt(f),
            Self::Other(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for Error {}
impl<E: Into<RuntimeError>> From<E> for Error {
    fn from(error: E) -> Self {
        Self::Ui(error.into())
    }
}

pub struct Explorer {
    pub store: Shared,
    pub all: View,
    pub errors: View,
    pub tree: TreeController,
    source_lines: Rc<RefCell<BTreeMap<Rc<str>, Vec<LineId>>>>,
    pub levels: Signal<Vec<Level>>,
    pub source: Signal<Option<Rc<str>>>,
    pub query: Signal<String>,
    pub progress: Signal<ProgressStatus>,
    pub status: Signal<String>,
    /// "N of M lines", after every filter including the filter field.
    pub shown: ReadSignal<String>,
    pub detail: Signal<String>,
    pub follow: Signal<bool>,
    /// The latest follow reveal, read when the next one is issued.
    pub followed: RefCell<Option<ListRevealReceipt>>,
    /// The current load; lines of an earlier one are dropped.
    load: Cell<u64>,
    /// The tail worker that may deliver; changing it stops the running one.
    tail: Arc<AtomicU64>,
    /// Allocates tailed lines' identities, past every line read; shared with
    /// every tail worker, so no identity is handed out twice, also when a
    /// worker's lines are lost.
    next_id: Arc<AtomicU64>,
    pub skipped: Cell<u64>,
    /// Tailed lines that never reached the tables: refused by a full mailbox,
    /// or delivered after their worker was stopped.
    pub lost: Arc<AtomicU64>,
    /// The loss the status last reported.
    reported_lost: Cell<u64>,
    sender: UiSender<Message>,
    _mailbox: Mailbox<Message>,
    commands: RefCell<Vec<CommandId>>,
}

impl Explorer {
    pub fn new(ui: &Runtime) -> Result<Rc<Self>, Error> {
        let store: Shared = Rc::default();
        let source_lines: Rc<RefCell<BTreeMap<Rc<str>, Vec<LineId>>>> = Rc::default();
        let tree = TreeController::new(
            ui,
            TreeSource::new(Sources {
                store: store.clone(),
                lines: source_lines.clone(),
            }),
            ListSelection::new(ui, SelectionMode::Single),
        )?;
        // The handler needs the explorer, which needs the mailbox: they meet
        // through this slot, filled below.
        let slot: Rc<RefCell<Option<std::rc::Weak<Explorer>>>> = Rc::default();
        let deliver = slot.clone();
        let mailbox = ui.mailbox(Capacity::new(8)?, move |message: Message| {
            match deliver.borrow().as_ref().and_then(std::rc::Weak::upgrade) {
                Some(explorer) => explorer.receive(message),
                None => Ok(()),
            }
        })?;
        let all = View::new(ui, &store, Kind::All, SelectionMode::Multiple)?;
        let shown = all
            .controller
            .filter_counts()
            .map(|counts| format!("{} of {} lines", counts.shown, counts.total))?;
        let explorer = Rc::new(Self {
            all,
            errors: View::new(ui, &store, Kind::Errors, SelectionMode::Multiple)?,
            tree,
            source_lines,
            levels: ui.signal(Level::ALL.to_vec()),
            source: ui.signal(None),
            query: ui.signal(String::new()),
            progress: ui.signal(ProgressStatus::Idle),
            status: ui.signal("Ready".to_owned()),
            shown,
            detail: ui.signal("Select a line to see it here.".to_owned()),
            follow: ui.signal(false),
            followed: RefCell::new(None),
            load: Cell::new(0),
            tail: Arc::new(AtomicU64::new(0)),
            next_id: Arc::new(AtomicU64::new(0)),
            skipped: Cell::new(0),
            lost: Arc::new(AtomicU64::new(0)),
            reported_lost: Cell::new(0),
            sender: mailbox.sender(),
            _mailbox: mailbox,
            commands: RefCell::default(),
            store,
        });
        *slot.borrow_mut() = Some(Rc::downgrade(&explorer));
        Ok(explorer)
    }

    fn accepts(&self, kind: Kind, entry: &Entry, filter: &Filter) -> bool {
        match kind {
            Kind::Errors => entry.level == Level::Error,
            Kind::All => {
                filter.levels.contains(&entry.level)
                    && filter
                        .source
                        .as_ref()
                        .is_none_or(|source| *source == entry.source)
            }
        }
    }
    fn filter(&self) -> Result<Filter, RuntimeError> {
        Ok(Filter {
            levels: self.levels.get()?.into_iter().collect(),
            source: self.source.get()?,
        })
    }

    fn receive(&self, message: Message) -> Result<(), RuntimeError> {
        match message {
            Message::Lines {
                load,
                lines,
                skipped,
                progress,
            } => {
                if load != self.load.get() {
                    return Ok(());
                }
                self.skipped.set(self.skipped.get() + skipped);
                self.append(lines)?;
                self.progress.set(
                    NormalizedValue::try_from(progress)
                        .map_or(ProgressStatus::Indeterminate, ProgressStatus::Determinate),
                )?;
            }
            Message::Loaded { load, lines } => {
                if load != self.load.get() {
                    return Ok(());
                }
                self.next_id.fetch_max(lines, Atomic::AcqRel);
                self.progress.set(ProgressStatus::Succeeded)?;
                self.status.set(format!(
                    "Loaded {} lines, skipped {} malformed",
                    self.store.borrow().entries.len(),
                    self.skipped.get()
                ))?;
                // Follow turned on during loading starts the tail now, after
                // the file's last line.
                if self.follow.get()? {
                    self.set_tail(true)?;
                }
            }
            Message::Failed { load, error } => {
                if load != self.load.get() {
                    return Ok(());
                }
                self.progress.set(ProgressStatus::Failed)?;
                self.status.set(format!("Loading stopped: {error}"))?;
            }
            Message::Tail {
                load,
                generation,
                lines,
            } => {
                if load != self.load.get() {
                    // The data it followed was replaced.
                    return Ok(());
                }
                if generation != self.tail.load(Atomic::Acquire) {
                    // Queued before its worker was stopped: lost, not shown.
                    self.lost.fetch_add(lines.len() as u64, Atomic::AcqRel);
                } else {
                    self.append(lines)?;
                }
                self.report_lost()?;
            }
        }
        Ok(())
    }

    /// Adds lines to the store and to each table's order where they belong,
    /// without rebuilding what is already there.
    fn append(&self, lines: Vec<Parsed>) -> Result<(), RuntimeError> {
        if lines.is_empty() {
            return Ok(());
        }
        let filter = self.filter()?;
        let first = {
            let mut store = self.store.borrow_mut();
            let mut sources = self.source_lines.borrow_mut();
            let first = store.entries.len();
            for parsed in lines {
                self.next_id.fetch_max(parsed.id.0 + 1, Atomic::AcqRel);
                let position = store.push(parsed);
                let entry = &store.entries[position];
                sources
                    .entry(entry.source.clone())
                    .or_default()
                    .push(entry.id);
            }
            first
        };
        for view in [&self.all, &self.errors] {
            let store = self.store.borrow();
            let sort = view.sort.borrow();
            let mut order = view.order.borrow_mut();
            for position in first..store.entries.len() {
                let entry = &store.entries[position];
                if !self.accepts(view.kind, entry, &filter) {
                    continue;
                }
                // Lines arrive in identity order, so an unsorted table only
                // appends; a sorted one inserts each where it belongs.
                let at = if sort.is_none() {
                    order.len()
                } else {
                    order.partition_point(|other| {
                        compare(sort.as_ref(), &store.entries[*other as usize], entry)
                            == Ordering::Less
                    })
                };
                order.insert(at, position as u32);
            }
        }
        self.republish()?;
        self.follow_last()
    }

    /// Rebuilds every table's order from the store, for a changed filter or
    /// sort.
    pub fn publish(&self) -> Result<(), RuntimeError> {
        let filter = self.filter()?;
        for view in [&self.all, &self.errors] {
            let store = self.store.borrow();
            let mut order: Vec<u32> = (0..store.entries.len() as u32)
                .filter(|position| {
                    self.accepts(view.kind, &store.entries[*position as usize], &filter)
                })
                .collect();
            let sort = view.sort.borrow();
            if sort.is_some() {
                order.sort_by(|a, b| {
                    compare(
                        sort.as_ref(),
                        &store.entries[*a as usize],
                        &store.entries[*b as usize],
                    )
                });
            }
            *view.order.borrow_mut() = order;
        }
        self.republish()
    }

    /// Hands the current orders to the tables and the tree.
    fn republish(&self) -> Result<(), RuntimeError> {
        for view in [&self.all, &self.errors] {
            let order: Rc<[u32]> = Rc::from(view.order.borrow().as_slice());
            view.controller
                .set_source(TableSource::new(Rows::new(&self.store, order)))?;
        }
        self.tree.set_source(TreeSource::new(Sources {
            store: self.store.clone(),
            lines: self.source_lines.clone(),
        }))
    }
    /// A gap in the tail: the lines are gone, and the status says how many.
    fn report_lost(&self) -> Result<(), RuntimeError> {
        let lost = self.lost.load(Atomic::Acquire);
        if lost != self.reported_lost.replace(lost) {
            self.status
                .set(format!("{lost} tailed lines lost while the view was busy"))?;
        }
        Ok(())
    }

    /// Sorts one table, leaving the other as it is.
    pub fn sort(&self, kind: Kind, sort: Option<TableSort>) -> Result<(), RuntimeError> {
        let view = self.view(kind);
        *view.sort.borrow_mut() = sort.clone();
        view.controller.set_sort(sort)?;
        self.publish()
    }
    pub fn view(&self, kind: Kind) -> &View {
        match kind {
            Kind::All => &self.all,
            Kind::Errors => &self.errors,
        }
    }

    /// While following, reveals the last row the main table shows (not the
    /// last line stored: a filter may hide it and a sort may place it
    /// elsewhere), and reports how the previous reveal ended.
    fn follow_last(&self) -> Result<(), RuntimeError> {
        if !self.follow.get()? {
            return Ok(());
        }
        let previous = self
            .followed
            .borrow()
            .as_ref()
            .map(ListRevealReceipt::status);
        if previous == Some(ListRevealStatus::NotFound) {
            self.status
                .set("Following: the last line was not found".to_owned())?;
        }
        let last = self.last_shown()?;
        if let Some(key) = last {
            *self.followed.borrow_mut() = Some(self.all.controller.reveal(key)?);
        }
        Ok(())
    }

    /// The key of the last row the main table shows after the filter field
    /// too, which matches the query as the table does: case-insensitively
    /// against the row's cells joined in column order.
    fn last_shown(&self) -> Result<Option<String>, RuntimeError> {
        let query = self.query.get()?;
        let query = query.trim();
        let store = self.store.borrow();
        let matcher = FilterMatcher::substring();
        Ok(self
            .all
            .order
            .borrow()
            .iter()
            .rev()
            .map(|position| &store.entries[*position as usize])
            .find(|entry| query.is_empty() || matcher.matches(query, &entry.searchable()))
            .map(|entry| entry.id.key()))
    }

    /// The detail pane's text for a line.
    fn describe(&self, key: &str) -> Option<String> {
        let store = self.store.borrow();
        let entry = store.get(LineId::from_key(key)?)?;
        let mut text = format!(
            "Line {} · {} · {} · {}\n\n{}",
            entry.id.0 + 1,
            entry.time,
            entry.level.name(),
            entry.source,
            entry.preview()
        );
        if entry.preview().len() < entry.message.len() {
            text.push_str(&format!(
                "…\n\n({} bytes; Copy lines copies all of it)",
                entry.message.len()
            ));
        }
        Some(text)
    }

    /// The selected lines of a table, in line order, as copied.
    pub fn selected_lines(&self, kind: Kind) -> Result<String, RuntimeError> {
        let keys = self.view(kind).controller.selection().selected().get()?;
        let mut ids: Vec<LineId> = keys
            .iter()
            .filter_map(|key| LineId::from_key(key))
            .collect();
        ids.sort();
        let store = self.store.borrow();
        Ok(ids
            .into_iter()
            .filter_map(|id| store.get(id).map(Entry::line))
            .collect::<Vec<_>>()
            .join("\n"))
    }

    /// Reads and parses `path` on a worker, in batches. A new load replaces
    /// the data and stops the tail.
    pub fn load(&self, path: PathBuf) -> Result<(), RuntimeError> {
        self.stop_tail();
        let load = self.load.get() + 1;
        self.load.set(load);
        self.store.borrow_mut().clear();
        self.source_lines.borrow_mut().clear();
        self.skipped.set(0);
        self.next_id.store(0, Atomic::Release);
        self.lost.store(0, Atomic::Release);
        self.reported_lost.set(0);
        for view in [&self.all, &self.errors] {
            view.order.borrow_mut().clear();
            view.controller.selection().clear()?;
        }
        self.publish()?;
        self.progress.set(ProgressStatus::Indeterminate)?;
        self.status.set(format!("Loading {}", path.display()))?;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            // The loader waits for room rather than dropping lines: a load
            // is complete or reports why not.
            let send = |mut message: Message| -> bool {
                loop {
                    match sender.send(message) {
                        Ok(()) => return true,
                        Err(SendError::Full(back)) => {
                            message = back;
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(SendError::Closed(_)) => return false,
                    }
                }
            };
            let file = match std::fs::File::open(&path) {
                Ok(file) => file,
                Err(error) => {
                    send(Message::Failed {
                        load,
                        error: LoadError::Open(path, error),
                    });
                    return;
                }
            };
            let total = file.metadata().map_or(1, |m| m.len()).max(1);
            let (mut read, mut index, mut skipped) = (0u64, 0u64, 0u64);
            let mut lines = Vec::new();
            use std::io::BufRead;
            for line in std::io::BufReader::new(file).lines() {
                let line = match line {
                    Ok(line) => line,
                    Err(error) => {
                        send(Message::Failed {
                            load,
                            error: LoadError::Read { line: index, error },
                        });
                        return;
                    }
                };
                read += line.len() as u64 + 1;
                match model::parse(LineId(index), &line) {
                    Some(parsed) => lines.push(parsed),
                    None => skipped += 1,
                }
                index += 1;
                if lines.len() == 5000 {
                    let message = Message::Lines {
                        load,
                        lines: std::mem::take(&mut lines),
                        skipped: std::mem::take(&mut skipped),
                        progress: read as f32 / total as f32,
                    };
                    if !send(message) {
                        return;
                    }
                }
            }
            let last = Message::Lines {
                load,
                lines,
                skipped,
                progress: 1.0,
            };
            if send(last) {
                send(Message::Loaded { load, lines: index });
            }
        });
        Ok(())
    }

    /// Starts or stops appending generated lines, as a live tail does. A
    /// stopped worker ends at its next tick, and its lines still queued are
    /// counted as lost. While a file is loading the tail waits for its end,
    /// so a tailed line never takes the identity of a line still to be read.
    pub fn set_tail(&self, on: bool) -> Result<(), RuntimeError> {
        self.stop_tail();
        self.report_lost()?;
        if !on || self.progress.get()? != ProgressStatus::Succeeded {
            return Ok(());
        }
        let generation = self.tail.load(Atomic::Acquire);
        let (sender, current) = (self.sender.clone(), self.tail.clone());
        let (next, lost, load) = (self.next_id.clone(), self.lost.clone(), self.load.get());
        std::thread::spawn(move || {
            while current.load(Atomic::Acquire) == generation {
                let first = next.fetch_add(5, Atomic::AcqRel);
                let lines: Vec<Parsed> = (first..first + 5)
                    .filter_map(|index| model::parse(LineId(index), &model::sample_line(index)))
                    .collect();
                let count = lines.len() as u64;
                match sender.send(Message::Tail {
                    load,
                    generation,
                    lines,
                }) {
                    Ok(()) => {}
                    Err(SendError::Full(_)) => {
                        lost.fetch_add(count, Atomic::AcqRel);
                    }
                    Err(SendError::Closed(_)) => return,
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        Ok(())
    }
    /// Makes any running tail worker stale.
    fn stop_tail(&self) {
        self.tail.fetch_add(1, Atomic::AcqRel);
    }
}

struct Filter {
    levels: BTreeSet<Level>,
    source: Option<Rc<str>>,
}

fn table(explorer: &Rc<Explorer>, kind: Kind) -> Result<Table, Error> {
    let sorting = explorer.clone();
    let detail = explorer.clone();
    Ok(Table::new(&explorer.view(kind).controller, 22.0)?
        .on_sort(move |_, sort| sorting.sort(kind, Some(sort.clone())))
        .on_selection_change(move |_, change| {
            // Both tables describe their active line in the one detail pane,
            // which stays in the main window when Errors is detached.
            if let Some(text) = change
                .active
                .as_deref()
                .and_then(|key| detail.describe(key))
            {
                detail.detail.set(text)?;
            }
            Ok(())
        }))
}

/// Builds the explorer and mounts it; loading starts at once.
pub fn build(ui: &mut Runtime, path: PathBuf) -> Result<(Rc<Explorer>, WidgetId), Error> {
    let explorer = Explorer::new(ui)?;
    // Every collection gets a bounded height: the tabs are fixed, the
    // regions fixed, and the splits give each pane its share.
    let all = CollectionFilter::with_query(
        explorer.query.clone(),
        explorer.all.controller.clone(),
        Element::new("table", table(&explorer, Kind::All)?),
    )
    .debounce(Duration::from_millis(150));
    let tabs = Tabs::new([
        Tab::new("all", "All lines", Element::new("all-filter", all))
            .fixed()
            .detachable(false),
        Tab::new(
            "errors",
            "Errors",
            Element::new("errors-table", table(&explorer, Kind::Errors)?),
        )
        .fixed(),
    ])
    .detachable(true);

    let picking = explorer.clone();
    let tree = Tree::new(&explorer.tree, 22.0)?.on_selection_change(move |_, change| {
        let Some(key) = change.active.as_deref() else {
            return Ok(());
        };
        match key.strip_prefix("s:") {
            // A source filters the main table.
            Some(source) => {
                if picking.source.set_changed(Some(Rc::from(source)))? {
                    picking.publish()?;
                }
            }
            // A line is shown in the detail pane.
            None => {
                if let Some(text) = picking.describe(key) {
                    picking.detail.set(text)?;
                }
            }
        }
        Ok(())
    });

    let levels = MultiSelect::new(
        ui,
        &explorer.levels,
        Level::ALL.map(|level| Choice::new(level, level.name())),
    )?;
    let level_change = explorer.clone();
    let levels = levels.on_change(move |_, _| level_change.publish());
    let all_sources = explorer.clone();
    let clear_source = Button::new("All sources", move |_| {
        if all_sources.source.set_changed(None)? {
            all_sources.publish()?;
        }
        Ok(())
    });
    let following = explorer.clone();
    let follow = Toggle::new("Follow", &explorer.follow).on_change(move |_, on| {
        following.set_tail(on)?;
        following.follow_last()
    });
    let toolbar = Row::new([
        Element::new("levels", levels),
        Element::new("all-sources", clear_source),
        Element::new("follow", follow),
        Element::new("progress", ProgressBar::new(explorer.progress.read_only())).with_grow(1),
        Element::new("shown", Label::new(explorer.shown.clone())),
    ])
    .align(Align::Center);

    let main = SplitPane::new(
        SplitAxis::LeftRight,
        Element::new("sources", tree),
        Element::new(
            "right",
            SplitPane::new(
                SplitAxis::TopBottom,
                Element::new("tabs", tabs),
                Element::new(
                    "detail",
                    ScrollArea::new(SelectableText::new(explorer.detail.read_only()).key("text")),
                ),
            ),
        ),
    );
    let root = ui.mount_tree(
        RegionLayout::new()
            .fixed()
            .top(Element::new("toolbar", toolbar))
            .center(Element::new("main", main))
            .bottom(Element::new(
                "status",
                Label::new(explorer.status.read_only()),
            ))
            .key("explorer"),
    )?;
    // Copy lines is bound once per table, scoped to its view: the chord
    // copies the table that holds focus, and follows Errors into its own
    // window when it is detached.
    for (kind, key) in [(Kind::All, "table"), (Kind::Errors, "errors-table")] {
        let view = ui
            .find_descendant(&root, key)?
            .ok_or_else(|| Error::Other(format!("no {key}")))?;
        let copying = explorer.clone();
        let command = ui.register_command(
            "Copy lines",
            CommandScope::View(view),
            vec![KeyChord::primary('c')],
            move |ui, _| {
                let text = copying.selected_lines(kind)?;
                if text.is_empty() {
                    return Ok(());
                }
                ui.set_clipboard_text(&text)?;
                let count = text.lines().count();
                copying.status.set(format!("Copied {count} lines"))
            },
        )?;
        explorer.commands.borrow_mut().push(command);
    }
    explorer.publish()?;
    explorer.load(path)?;
    Ok((explorer, root))
}
