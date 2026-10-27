use pert_ui::{prelude::*, *};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Running,
    Done,
    Cancelled,
    Failed,
}
impl State {
    fn text(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}
#[derive(Clone)]
struct Job {
    id: usize,
    total: u64,
    processed: u64,
    state: State,
    elapsed: Duration,
    bar: Signal<ProgressStatus>,
    command: mpsc::Sender<Command>,
}
impl Job {
    fn name(&self) -> String {
        format!("Job {}", self.id)
    }
}
#[derive(Clone, Copy)]
enum Command {
    Tick,
    Cancel,
}
struct Update {
    id: usize,
    processed: u64,
    elapsed: Duration,
    state: State,
}
#[derive(Clone)]
struct Rows(Vec<Job>);
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        Some(self.0.get(i)?.id.to_string())
    }
    fn cell(&self, i: usize, col: &str) -> Option<Element> {
        let j = self.0.get(i)?;
        Some(match col {
            "progress" => Column::new([
                Label::new(format!(
                    "{:.0}%  ({} / {} units)",
                    100.0 * j.processed as f64 / j.total as f64,
                    j.processed,
                    j.total
                ))
                .key("percent"),
                ProgressBar::new(j.bar.read_only()).key("bar"),
            ])
            .key("progress"),
            "action" if j.state == State::Running => {
                let tx = j.command.clone();
                Button::new("Cancel", move |_| {
                    let _ = tx.send(Command::Cancel);
                    Ok(())
                })
                .key("cancel")
            }
            _ => Label::new(self.text(i, col).unwrap_or_default()).key("value"),
        })
    }
    fn text(&self, i: usize, col: &str) -> Option<String> {
        let j = self.0.get(i)?;
        Some(match col {
            "name" => j.name(),
            "state" => {
                if j.state == State::Failed {
                    "failed: simulated processing error".into()
                } else {
                    j.state.text().into()
                }
            }
            "elapsed" => format!("{:.1} s", j.elapsed.as_secs_f64()),
            _ => String::new(),
        })
    }
}
struct Model {
    jobs: Vec<Job>,
    table: TableController,
    summary: Signal<String>,
    notification: Signal<String>,
    failed: Signal<bool>,
}
impl Model {
    fn ordered(&self) -> std::result::Result<Vec<Job>, RuntimeError> {
        let mut jobs = self.jobs.clone();
        if let Some(sort) = self.table.sort().get()? {
            jobs.sort_by(|a, b| {
                let order = match sort.column.as_str() {
                    "name" => a.name().cmp(&b.name()),
                    "progress" => (a.processed * b.total).cmp(&(b.processed * a.total)),
                    "state" => a.state.text().cmp(b.state.text()),
                    "elapsed" => a.elapsed.cmp(&b.elapsed),
                    _ => a.id.cmp(&b.id),
                };
                let order = if sort.direction == SortDirection::Descending {
                    order.reverse()
                } else {
                    order
                };
                order.then(a.id.cmp(&b.id))
            });
        }
        Ok(jobs)
    }
    fn publish(&self) -> std::result::Result<(), RuntimeError> {
        self.table
            .set_source(TableSource::new(Rows(self.ordered()?)))?;
        let count = |state| self.jobs.iter().filter(|j| j.state == state).count();
        self.summary.set(format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            count(State::Running),
            count(State::Done),
            count(State::Cancelled),
            count(State::Failed)
        ))
    }
    fn apply(&mut self, u: Update) -> std::result::Result<(), RuntimeError> {
        let j = self
            .jobs
            .iter_mut()
            .find(|j| j.id == u.id)
            .expect("known worker");
        j.processed = u.processed;
        j.elapsed = u.elapsed;
        j.state = u.state;
        j.bar.set(if u.state == State::Done {
            ProgressStatus::Succeeded
        } else {
            ProgressStatus::of(j.processed, j.total)
        })?;
        if u.state == State::Failed {
            self.notification.set(format!(
                "{} failed: simulated processing error at {} / {} units",
                j.name(),
                j.processed,
                j.total
            ))?;
            self.failed.set(true)?;
        }
        self.publish()
    }
}
struct Monitor {
    model: Rc<RefCell<Model>>,
    mailbox: Mailbox<Update>,
    workers: RefCell<Vec<thread::JoinHandle<()>>>,
    manual: bool,
    rng: RefCell<u64>,
}
fn random(seed: &mut u64, lo: u64, hi: u64) -> u64 {
    *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    lo + (*seed >> 32) % (hi - lo + 1)
}
fn deliver(tx: &UiSender<Update>, mut u: Update) {
    loop {
        match tx.send(u) {
            Ok(()) => return,
            Err(SendError::Full(value)) => {
                u = value;
                thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return,
        }
    }
}
impl Monitor {
    fn start(&self, ui: &impl StateOwner) -> std::result::Result<(), RuntimeError> {
        let id = self.model.borrow().jobs.len() + 1;
        let (total, fail_at) = if self.manual {
            match id {
                1 => (50, None),
                4 => (80, None),
                5 => (150, Some(30)),
                _ => (500, None),
            }
        } else {
            let mut seed = self.rng.borrow_mut();
            let total = random(&mut seed, 50, 500);
            let fail = if id % 5 == 0 {
                Some(random(&mut seed, 1, total - 1))
            } else {
                None
            };
            (total, fail)
        };
        let (command, rx) = mpsc::channel();
        self.model.borrow_mut().jobs.push(Job {
            id,
            total,
            processed: 0,
            state: State::Running,
            elapsed: Duration::ZERO,
            bar: ui.signal(ProgressStatus::of(0, total)),
            command,
        });
        self.model.borrow().publish()?;
        let tx = self.mailbox.sender();
        let manual = self.manual;
        let mut seed = *self.rng.borrow() ^ id as u64;
        self.workers.borrow_mut().push(thread::spawn(move || {
            let began = Instant::now();
            let mut processed = 0;
            let mut ticks = 0;
            loop {
                let command = if manual {
                    rx.recv().ok()
                } else {
                    match rx.recv_timeout(Duration::from_millis(100)) {
                        Ok(c) => Some(c),
                        Err(mpsc::RecvTimeoutError::Timeout) => Some(Command::Tick),
                        Err(_) => None,
                    }
                };
                let Some(command) = command else { break };
                let state = match command {
                    Command::Cancel => State::Cancelled,
                    Command::Tick => {
                        ticks += 1;
                        processed = (processed
                            + if manual { 15 } else { random(&mut seed, 5, 20) })
                        .min(total);
                        if let Some(point) = fail_at {
                            if processed >= point {
                                processed = point;
                                State::Failed
                            } else {
                                State::Running
                            }
                        } else if processed == total {
                            State::Done
                        } else {
                            State::Running
                        }
                    }
                };
                deliver(
                    &tx,
                    Update {
                        id,
                        processed,
                        elapsed: if manual {
                            Duration::from_millis(ticks * 100)
                        } else {
                            began.elapsed()
                        },
                        state,
                    },
                );
                if state != State::Running {
                    break;
                }
            }
        }));
        Ok(())
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.mailbox.close();
        for j in &self.model.borrow().jobs {
            let _ = j.command.send(Command::Cancel);
        }
        for worker in self.workers.get_mut().drain(..) {
            let _ = worker.join();
        }
    }
}
fn mount(
    ui: &mut Runtime,
    manual: bool,
) -> std::result::Result<(Rc<Monitor>, WidgetId), RuntimeError> {
    let table = TableController::new(
        ui,
        [
            TableColumn::new("name", "Name").sortable(true),
            TableColumn::new("progress", "Progress").sortable(true),
            TableColumn::new("state", "State").sortable(true),
            TableColumn::new("elapsed", "Elapsed").sortable(true),
            TableColumn::new("action", "Action"),
        ],
        TableSource::new(Rows(vec![])),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let model = Rc::new(RefCell::new(Model {
        jobs: vec![],
        table: table.clone(),
        summary: ui.signal(String::from(
            "Running: 0   Done: 0   Cancelled: 0   Failed: 0",
        )),
        notification: ui.signal(String::new()),
        failed: ui.signal(false),
    }));
    let target = model.clone();
    let mailbox = ui.mailbox(Capacity::new(256)?, move |u| target.borrow_mut().apply(u))?;
    let monitor = Rc::new(Monitor {
        model: model.clone(),
        mailbox,
        workers: RefCell::new(vec![]),
        manual,
        rng: RefCell::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
        ),
    });
    let start = monitor.clone();
    let sorter = model.clone();
    let content = Column::new([
        Label::new("Background job monitor").key("title"),
        Button::new("Start job", move |cx| start.start(cx)).key("start"),
        Label::new(model.borrow().summary.read_only()).key("summary"),
        Banner::new(
            model.borrow().notification.read_only(),
            Vec::<Element>::new(),
        )
        .visible_signal(model.borrow().failed.read_only())
        .key("notification"),
        Table::new(&table, 64.0)?
            .on_sort(move |_, _| sorter.borrow().publish())
            .key("table")
            .with_grow(1),
    ])
    .key("monitor");
    let root = ui.mount_tree(content)?;
    Ok((monitor, root))
}
fn main() -> Result {
    if std::env::args().any(|arg| arg == "--headless") {
        headless()?;
    } else {
        App::new("Background job monitor", |ui| {
            mount(ui, false)?;
            Ok(())
        })
        .run()?;
    }
    Ok(())
}

fn press(ui: &mut Runtime, id: &WidgetId) -> Result {
    ui.focus(id)?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    ui.flush(ui.now())?;
    Ok(())
}
fn wait_for(ui: &mut Runtime, monitor: &Monitor, condition: impl Fn(&[Job]) -> bool) -> Result {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        ui.flush(ui.now())?;
        if condition(&monitor.model.borrow().jobs) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err("worker delivery timed out".into());
        }
        thread::sleep(Duration::from_millis(1));
    }
}
fn visible_text(ui: &Runtime, text: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .filter(|c| c.visible)
        .any(|c| c.details.semantics.as_ref().and_then(|s| s.name.as_deref()) == Some(text))
}
fn tick(ui: &mut Runtime, m: &Monitor, id: usize) -> Result {
    let old = m.model.borrow().jobs[id - 1].processed;
    m.model.borrow().jobs[id - 1].command.send(Command::Tick)?;
    wait_for(ui, m, |jobs| jobs[id - 1].processed > old)
}
fn row_names(ui: &Runtime, table: &WidgetId) -> Result<Vec<String>> {
    fn visit(ui: &Runtime, id: &WidgetId, out: &mut Vec<(f32, String)>) -> Result {
        let inspected = ui.inspection_id(id)?;
        if let Some(c) = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == inspected)
        {
            if let Some(name) = c.details.semantics.and_then(|s| s.name) {
                if name.starts_with("Job ") && name[4..].parse::<usize>().is_ok() {
                    out.push((ui.bounds(id)?.ok_or("row has no bounds")?.y(), name));
                }
            }
        }
        for child in ui.children(id)? {
            visit(ui, &child, out)?;
        }
        Ok(())
    }
    let mut rows = vec![];
    visit(ui, table, &mut rows)?;
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    Ok(rows.into_iter().map(|(_, n)| n).collect())
}
fn headless() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1200.0, 700.0, 1.0)?)?;
    let (monitor, root) = mount(&mut ui, true)?;
    ui.flush(Duration::ZERO)?;
    let start = ui
        .find_descendant(&root, "start")?
        .ok_or("start button missing")?;
    for _ in 0..5 {
        press(&mut ui, &start)?;
    }
    assert!(visible_text(
        &ui,
        "Running: 5   Done: 0   Cancelled: 0   Failed: 0"
    ));
    tick(&mut ui, &monitor, 1)?;
    assert!(
        visible_text(&ui, "30%  (15 / 50 units)"),
        "progress label updates"
    );
    assert!(
        ui.inspect().components.iter().any(|c| c
            .details
            .semantics
            .as_ref()
            .is_some_and(|s| s.role == SemanticRole::ProgressIndicator && s.value.is_some())),
        "bar has inspected progress"
    );
    tick(&mut ui, &monitor, 2)?;
    let table = ui.find_descendant(&root, "table")?.ok_or("table missing")?;
    let row = monitor.model.borrow().table.reveal("2")?;
    ui.flush(ui.now())?;
    let row_id = match row.status() {
        ListRevealStatus::Mounted(id) => id,
        _ => return Err("cancel row was not revealed".into()),
    };
    let cancel = ui
        .find_descendant(&row_id, "cancel")?
        .ok_or("cancel button missing")?;
    let cancelled_at = monitor.model.borrow().jobs[1].processed;
    press(&mut ui, &cancel)?;
    wait_for(&mut ui, &monitor, |j| j[1].state == State::Cancelled)?;
    assert!(visible_text(&ui, "cancelled"));
    for _ in 0..3 {
        tick(&mut ui, &monitor, 1)?;
    }
    for _ in 0..6 {
        tick(&mut ui, &monitor, 4)?;
    }
    for _ in 0..2 {
        tick(&mut ui, &monitor, 5)?;
    }
    tick(&mut ui, &monitor, 3)?;
    assert!(visible_text(&ui, "done"));
    assert!(visible_text(&ui, "failed: simulated processing error"));
    assert!(visible_text(
        &ui,
        "Job 5 failed: simulated processing error at 30 / 150 units"
    ));
    assert!(visible_text(
        &ui,
        "Running: 1   Done: 2   Cancelled: 1   Failed: 1"
    ));
    let controller = monitor.model.borrow().table.clone();
    let body = ui.children(&table)?[0].clone();
    controller.selection().set_active(Some("1".into()))?;
    for column in ["name", "progress", "state", "elapsed"] {
        for (direction, key) in [
            (SortDirection::Ascending, KeyCode::ArrowUp),
            (SortDirection::Descending, KeyCode::ArrowDown),
        ] {
            controller.set_active_column(Some(column.into()))?;
            ui.focus(&body)?;
            ui.native_modifiers(Modifiers {
                alt: true,
                ..Modifiers::default()
            })?;
            ui.key_input(key, KeyAction::Press)?;
            ui.key_input(key, KeyAction::Release)?;
            ui.native_modifiers(Modifiers::default())?;
            ui.flush(ui.now())?;
            assert_eq!(
                controller.sort().get()?,
                Some(TableSort {
                    column: column.into(),
                    direction
                })
            );
            let jobs = monitor.model.borrow().ordered()?;
            assert_eq!(
                row_names(&ui, &table)?,
                jobs.iter().map(Job::name).collect::<Vec<_>>(),
                "rendered row order for {column} {direction:?}"
            );
            // Independently check the requested values, not just the model's chosen order.
            let values: Vec<_> = jobs
                .iter()
                .map(|j| match column {
                    "name" => j.name(),
                    "progress" => format!("{:010}", j.processed * 1_000_000 / j.total),
                    "state" => j.state.text().into(),
                    _ => format!("{:010}", j.elapsed.as_millis()),
                })
                .collect();
            assert!(
                values
                    .windows(2)
                    .all(|v| if direction == SortDirection::Ascending {
                        v[0] <= v[1]
                    } else {
                        v[0] >= v[1]
                    })
            );
        }
    }
    controller.set_sort(Some(TableSort {
        column: "name".into(),
        direction: SortDirection::Ascending,
    }))?;
    monitor.model.borrow().publish()?;
    ui.flush(ui.now())?;
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::window(
        &ui.primary_window(),
    )));
    ui.flush(ui.now())?;
    pert_ui::cpu::CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    std::fs::write("jobs.png", capture.image.encode_png())?;
    let row = controller.reveal("3")?;
    ui.flush(ui.now())?;
    let row_id = match row.status() {
        ListRevealStatus::Mounted(id) => id,
        _ => return Err("last row missing".into()),
    };
    let cancel = ui
        .find_descendant(&row_id, "cancel")?
        .ok_or("last cancel missing")?;
    press(&mut ui, &cancel)?;
    wait_for(&mut ui, &monitor, |j| j[2].state == State::Cancelled)?;
    assert_eq!(
        monitor.model.borrow().jobs[1].processed,
        cancelled_at,
        "cancelled worker never advanced"
    );
    assert!(visible_text(
        &ui,
        "Running: 0   Done: 2   Cancelled: 2   Failed: 1"
    ));
    ui.clear_focus();
    ui.flush(Duration::from_secs(10))?;
    ui.paint()?;
    assert_eq!(ui.next_deadline(), None, "idle runtime has no wakeups");
    ui.flush(Duration::from_secs(20))?;
    assert!(!ui.needs_paint(), "idle runtime has no repaint");
    println!(
        "PASS: start, progress, completion, cancellation, failure row and notification, summary, all sorts, idle scheduling; CPU capture: jobs.png"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn headless_acceptance() {
        super::headless().unwrap();
    }
}
