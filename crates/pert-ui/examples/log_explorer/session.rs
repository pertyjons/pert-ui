//! The headless session: loads a generated log, then scrolls, sorts,
//! filters, searches, copies, follows a live tail and asks for an inspection
//! snapshot from another thread, reporting mounted counts and flush times and
//! checking that the counts stay bounded.
use crate::app::{self, Explorer, Kind};
use crate::model::{self, Level, LineId, MALFORMED_EVERY};
use pert_ui::prelude::*;
use pert_ui::{
    Capacity, Clipboard, ClipboardError, CommandKey, CommandKeyInput, ContentQuery,
    ListRevealStatus, ProgressStatus, SortDirection, TableSort, TextCommand, ValidationCode,
};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// More than a screenful of every collection, far fewer than the lines.
const BOUNDED: usize = 3_000;

#[derive(Clone, Default)]
struct Memory(Rc<RefCell<String>>);
impl Clipboard for Memory {
    fn read_text(&mut self) -> std::result::Result<String, ClipboardError> {
        Ok(self.0.borrow().clone())
    }
    fn write_text(&mut self, text: &str) -> std::result::Result<(), ClipboardError> {
        *self.0.borrow_mut() = text.to_owned();
        Ok(())
    }
}

struct Session {
    ui: Runtime,
    root: WidgetId,
    explorer: Rc<Explorer>,
    now: Duration,
}
impl Session {
    /// One flush at a later time, and how long it took.
    fn flush(&mut self) -> Result<Duration> {
        self.now += Duration::from_millis(16);
        let start = Instant::now();
        self.ui.flush(self.now)?;
        Ok(start.elapsed())
    }
    fn settle(&mut self) -> Result<Duration> {
        let mut took = Duration::ZERO;
        for _ in 0..3 {
            took += self.flush()?;
        }
        Ok(took)
    }
    fn mounted(&self) -> usize {
        self.ui.inspect().components.len()
    }
    fn bounded(&self, when: &str) -> Result {
        let mounted = self.mounted();
        println!("{when:<34} {mounted:>6} components mounted");
        if mounted > BOUNDED {
            return Err(format!("{when}: {mounted} components mounted").into());
        }
        Ok(())
    }
    fn widget(&self, key: &str) -> Result<WidgetId> {
        Ok(self
            .ui
            .find_descendant(&self.root, key)?
            .ok_or_else(|| format!("no {key}"))?)
    }
    /// The primary modifier held around `code`, as the native host sends it.
    fn chord(&mut self, code: KeyCode, key: char) -> Result {
        let modifier = KeyCode::primary();
        let named = |action| CommandKeyInput::new(modifier, action, CommandKey::Named(modifier));
        self.ui.command_key_input(named(KeyAction::Press))?;
        for action in [KeyAction::Press, KeyAction::Release] {
            self.ui.command_key_input(CommandKeyInput::new(
                code,
                action,
                CommandKey::Character(key),
            ))?;
        }
        self.ui.command_key_input(named(KeyAction::Release))?;
        self.flush()?;
        Ok(())
    }
}

fn report(what: &str, took: Duration) {
    println!("{what:<34} {took:>10.2?}");
}

/// Every stored line has its own identity.
fn unique(explorer: &Explorer) -> Result {
    let store = explorer.store.borrow();
    let ids: std::collections::HashSet<_> = store.entries.iter().map(|entry| entry.id).collect();
    if ids.len() != store.entries.len() {
        return Err("two stored lines share an identity".into());
    }
    Ok(())
}

pub fn run(path: PathBuf, lines: u64) -> Result {
    // Malformed timestamps are skipped, not stored.
    for line in [
        "abcdefghijT99:99:99.999 INFO db message",
        "2026-02-30T12:00:00.000 INFO db message",
        "2026-10-19T24:00:00.000 INFO db message",
        "2026-10-19T12:60:00.000 INFO db message",
        "2026-10-19T12:00:00.00x INFO db message",
    ] {
        if model::parse(LineId(0), line).is_some() {
            return Err(format!("{line:?} was accepted").into());
        }
    }
    if model::parse(LineId(0), "2024-02-29T23:59:59.999 INFO db leap day").is_none() {
        return Err("a valid timestamp was refused".into());
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1400.0, 900.0, 1.0)?)?;
    let clipboard = Memory::default();
    ui.set_clipboard(clipboard.clone())?;
    let (explorer, root) = app::build(&mut ui, path.clone())?;
    let mut s = Session {
        ui,
        root,
        explorer: explorer.clone(),
        now: Duration::ZERO,
    };

    // Loading: flush until the worker reports the end.
    let start = Instant::now();
    let (mut flushes, mut worst) = (0, Duration::ZERO);
    while s.explorer.progress.get()? != ProgressStatus::Succeeded {
        if s.explorer.progress.get()? == ProgressStatus::Failed {
            return Err(s.explorer.status.get()?.into());
        }
        worst = worst.max(s.flush()?);
        flushes += 1;
        if start.elapsed() > Duration::from_secs(300) {
            return Err("loading timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let stored = explorer.store.borrow().entries.len() as u64;
    println!(
        "loaded {stored} lines ({} malformed skipped) in {:.2?}, {flushes} flushes, worst {worst:.2?}",
        explorer.skipped.get(),
        start.elapsed()
    );
    if stored + explorer.skipped.get() != lines || explorer.skipped.get() != lines / MALFORMED_EVERY
    {
        return Err("every line is either stored or counted as skipped".into());
    }
    s.settle()?;
    s.bounded("after loading")?;

    // Scrolling the main table with the wheel.
    let table = s.widget("table")?;
    let [x, y, w, h] = s.ui.bounds(&table)?.ok_or("table laid out")?.values();
    let over = Point::new(x + w / 2.0, y + h / 2.0)?;
    let start = Instant::now();
    for _ in 0..100 {
        s.ui.pointer_input(PointerEvent::new(
            over,
            PointerKind::Scroll(Point::new(0.0, 120.0)?),
        ))?;
        s.flush()?;
    }
    report("100 wheel steps, each", start.elapsed() / 100);
    s.bounded("while scrolled")?;

    // Sorting one table leaves the other alone.
    let errors_first = explorer.errors.last_key(&explorer.store.borrow());
    let start = Instant::now();
    explorer.sort(
        Kind::All,
        Some(TableSort {
            column: "message".into(),
            direction: SortDirection::Ascending,
        }),
    )?;
    s.flush()?;
    report("sort by message", start.elapsed());
    if explorer.errors.last_key(&explorer.store.borrow()) != errors_first {
        return Err("sorting the main table reordered Errors".into());
    }

    // The filter field, debounced.
    explorer.query.set("timeout retry".to_owned())?;
    let start = Instant::now();
    for _ in 0..20 {
        s.flush()?;
    }
    let counts = explorer.all.controller.filter_counts().get()?;
    report("filter 'timeout retry' (20 flushes)", start.elapsed());
    println!("  {counts:?}");
    explorer.query.set(String::new())?;
    s.settle()?;
    for _ in 0..20 {
        s.flush()?;
    }

    // The level filter.
    let start = Instant::now();
    explorer.levels.set(vec![Level::Error])?;
    explorer.publish()?;
    s.flush()?;
    report("errors-only level filter", start.elapsed());
    if explorer.all.shown() != explorer.errors.shown() {
        return Err("errors only shows the Errors view's lines".into());
    }
    explorer.levels.set(Level::ALL.to_vec())?;
    explorer.publish()?;
    s.settle()?;

    // A click on a row selects it and shows it in the detail pane; Copy
    // lines copies the focused table's selection.
    let row = Point::new(x + 200.0, y + 22.0 + 2.5 * 22.0)?;
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        s.ui.pointer_input(PointerEvent::new(row, kind))?;
    }
    s.flush()?;
    s.chord(KeyCode::KeyC, 'c')?;
    let copied = clipboard.0.borrow().clone();
    if copied.lines().count() != 1 || !explorer.status.get()?.starts_with("Copied 1") {
        return Err(format!("copy lines copied {copied:?}").into());
    }
    println!("copy lines: {copied:?}");

    // The detail pane shows the clicked line, and its text can be copied.
    let detail = s.widget("detail")?;
    let detail =
        s.ui.find_descendant(&detail, "text")?
            .ok_or("no detail text")?;
    s.ui.text_command(&detail, TextCommand::SelectAll)?;
    s.ui.text_command(&detail, TextCommand::Copy)?;
    let described = clipboard.0.borrow().clone();
    if !described.starts_with("Line ") || !copied.ends_with(described.lines().last().unwrap_or("-"))
    {
        return Err(format!("the detail pane shows the clicked line: {described:?}").into());
    }

    // The tree: select a source, which filters the main table.
    let sources = s.widget("sources")?;
    s.ui.focus(&sources)?;
    s.flush()?;
    let start = Instant::now();
    for code in [KeyCode::ArrowDown, KeyCode::ArrowRight] {
        s.ui.key_input(code, KeyAction::Press)?;
        s.ui.key_input(code, KeyAction::Release)?;
        s.flush()?;
    }
    report("tree: select and expand a source", start.elapsed());
    let source = explorer.source.get()?.ok_or("a source is selected")?;
    println!("  source {source}: {}", explorer.shown.get()?);
    s.bounded("with an expanded source")?;

    // Content search, stepped within its budget.
    let start = Instant::now();
    let mut search = s.ui.content_search(ContentQuery::new("timeout")?);
    let (mut steps, mut hits) = (0, 0);
    loop {
        let step = search.step(&s.ui)?;
        steps += 1;
        hits += step.hits.len();
        if step.done {
            break;
        }
    }
    report("content search 'timeout'", start.elapsed());
    println!("  {steps} steps, {hits} hits");
    if hits == 0 {
        return Err("content search finds lines".into());
    }

    // Following a live tail reveals the last row the filter field leaves.
    explorer.query.set("timeout".to_owned())?;
    for _ in 0..20 {
        s.flush()?;
    }
    explorer.follow.set(true)?;
    explorer.set_tail(true)?;
    let before = explorer.store.borrow().entries.len();
    let start = Instant::now();
    let mut worst = Duration::ZERO;
    while start.elapsed() < Duration::from_secs(1) {
        worst = worst.max(s.flush()?);
        std::thread::sleep(Duration::from_millis(10));
    }
    explorer.set_tail(false)?;
    s.settle()?;
    let after = explorer.store.borrow().entries.len();
    let followed = explorer
        .followed
        .borrow()
        .as_ref()
        .map(|receipt| receipt.status());
    println!(
        "tail: {} lines in 1 s, worst flush {worst:.2?}, follow {followed:?}",
        after - before,
    );
    if after == before || !matches!(followed, Some(ListRevealStatus::Mounted(_))) {
        return Err("following reveals the last matching row".into());
    }
    explorer.query.set(String::new())?;
    for _ in 0..20 {
        s.flush()?;
    }
    s.bounded("after following")?;

    // A tail the UI does not drain fills the mailbox: the lines it could not
    // deliver, and those still queued when it stops, are reported as lost.
    explorer.set_tail(true)?;
    std::thread::sleep(Duration::from_millis(1500));
    explorer.set_tail(false)?;
    s.settle()?;
    let lost = explorer.lost.load(std::sync::atomic::Ordering::Acquire);
    println!(
        "undrained tail: {lost} lines lost; {}",
        explorer.status.get()?
    );
    if lost == 0 || !explorer.status.get()?.contains("lost") {
        return Err("a full mailbox loses lines and says so".into());
    }
    unique(&explorer)?;

    // Follow turned on while a file loads starts the tail only after it.
    explorer.load(path.clone())?;
    explorer.follow.set(true)?;
    explorer.set_tail(true)?;
    let start = Instant::now();
    while s.explorer.progress.get()? != ProgressStatus::Succeeded {
        s.flush()?;
        if start.elapsed() > Duration::from_secs(300) {
            return Err("reloading timed out".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let loaded = explorer.store.borrow().entries.len();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        s.flush()?;
        std::thread::sleep(Duration::from_millis(10));
    }
    explorer.set_tail(false)?;
    s.settle()?;
    if explorer.store.borrow().entries.len() <= loaded {
        return Err("the tail starts once loading ends".into());
    }
    unique(&explorer)?;
    explorer.follow.set(false)?;

    // An inspection snapshot asked for from another thread.
    let service = s.ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let asked = std::thread::spawn(move || -> std::result::Result<usize, String> {
        let pending = client.snapshot().map_err(|error| error.to_string())?;
        let snapshot = pending
            .wait_timeout(Duration::from_secs(10))
            .map_err(|error| error.to_string())?;
        Ok(snapshot.value.components.len())
    });
    let start = Instant::now();
    while !asked.is_finished() && start.elapsed() < Duration::from_secs(10) {
        s.flush()?;
        std::thread::sleep(Duration::from_millis(1));
    }
    // Joined only once it has returned, so a stalled thread fails the session
    // instead of hanging it.
    if !asked.is_finished() {
        return Err("the inspection snapshot timed out".into());
    }
    let components = asked
        .join()
        .map_err(|_| "the inspection thread panicked")??;
    report("inspection snapshot from a thread", start.elapsed());
    println!("  {components} components");

    let unbounded =
        s.ui.validation_report()
            .issues
            .into_iter()
            .filter(|issue| issue.code == ValidationCode::UnboundedCollection)
            .count();
    if unbounded > 0 {
        return Err(format!("{unbounded} unbounded collections reported").into());
    }
    s.ui.shutdown();
    println!("log explorer: headless session passed");
    Ok(())
}
