use pert_ui::{
    Capacity, ListSelection, Mailbox, ProgressBar, ProgressStatus, SelectionMode, SortDirection,
    TableColumn, TableController, TableItems, TableSort, TableSource, UiSender, prelude::*,
};
use std::{
    cell::RefCell,
    cmp::Ordering,
    rc::{Rc, Weak},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

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
    units: u64,
    elapsed: Duration,
    state: State,
    command: Sender<Command>,
}
impl Job {
    fn name(&self) -> String {
        format!("Job {}", self.id)
    }
    fn fraction(&self) -> f64 {
        self.units as f64 / self.total as f64
    }
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, a: u64, b: u64) -> u64 {
        a + self.next() % (b - a + 1)
    }
}
#[derive(Clone, Copy)]
struct Plan {
    total: u64,
    fail_at: Option<u64>,
    manual: bool,
    seed: u64,
}
#[derive(Clone, Copy)]
enum Command {
    Cancel,
    Tick(u64),
    Stop,
}
#[derive(Clone)]
struct Report {
    id: usize,
    units: u64,
    elapsed: Duration,
    state: State,
}
fn worker(id: usize, plan: Plan, rx: Receiver<Command>, out: UiSender<Report>) {
    let mut rng = Rng(plan.seed.max(1));
    let mut units = 0;
    let mut ticks = 0;
    let start = Instant::now();
    loop {
        let command = if plan.manual {
            match rx.recv() {
                Ok(c) => c,
                Err(_) => break,
            }
        } else {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(c) => c,
                Err(RecvTimeoutError::Timeout) => Command::Tick(rng.range(5, 20)),
                Err(_) => break,
            }
        };
        let state = match command {
            Command::Stop => break,
            Command::Cancel => State::Cancelled,
            Command::Tick(n) => {
                ticks += 1;
                units = (units + n.clamp(5, 20)).min(plan.total);
                if plan.fail_at.is_some_and(|at| units >= at) {
                    State::Failed
                } else if units == plan.total {
                    State::Done
                } else {
                    State::Running
                }
            }
        };
        let mut report = Report {
            id,
            units,
            elapsed: if plan.manual {
                Duration::from_millis(ticks * 100)
            } else {
                start.elapsed()
            },
            state,
        };
        // Never lose a terminal update when the UI is briefly busy. Retry with
        // bounded waits, checking cancellation/shutdown even under backpressure.
        loop {
            let terminal = report.state != State::Running;
            match out.send(report) {
                Ok(()) => {
                    if terminal {
                        return;
                    }
                    break;
                }
                Err(pert_ui::SendError::Closed(_)) => return,
                Err(pert_ui::SendError::Full(value)) => {
                    report = value;
                    match rx.recv_timeout(Duration::from_millis(5)) {
                        Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                        Ok(Command::Cancel) => {
                            report.state = State::Cancelled;
                        }
                        _ => {}
                    }
                }
            }
        }
        if state != State::Running {
            break;
        }
    }
}
struct Store {
    jobs: RefCell<Vec<Job>>,
    table: TableController,
    summary: Signal<String>,
    notification: Signal<String>,
}
fn compare(a: &Job, b: &Job, column: &str) -> Ordering {
    let order = match column {
        "name" => a.name().cmp(&b.name()),
        "progress" => a.fraction().total_cmp(&b.fraction()),
        "state" => a.state.text().cmp(b.state.text()),
        "elapsed" => a.elapsed.cmp(&b.elapsed),
        _ => Ordering::Equal,
    };
    order.then(a.id.cmp(&b.id))
}
impl Store {
    fn publish(self: &Rc<Self>) -> Result<(), RuntimeError> {
        let mut rows = self.jobs.borrow().clone();
        if let Some(sort) = self.table.sort().get()? {
            rows.sort_by(|a, b| {
                let c = compare(a, b, &sort.column);
                if sort.direction == SortDirection::Descending {
                    c.reverse()
                } else {
                    c
                }
            });
        }
        let count = |s| rows.iter().filter(|j| j.state == s).count();
        self.summary.set(format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            count(State::Running),
            count(State::Done),
            count(State::Cancelled),
            count(State::Failed)
        ))?;
        self.table.set_source(TableSource::new(Rows {
            jobs: rows,
            store: Rc::downgrade(self),
        }))
    }
    fn cancel(self: &Rc<Self>, id: usize) -> Result<(), RuntimeError> {
        if let Some(j) = self
            .jobs
            .borrow_mut()
            .iter_mut()
            .find(|j| j.id == id && j.state == State::Running)
        {
            let _ = j.command.send(Command::Cancel);
            j.state = State::Cancelled;
        }
        self.publish()
    }
    fn report(self: &Rc<Self>, report: Report) -> Result<(), RuntimeError> {
        let failed = {
            let mut jobs = self.jobs.borrow_mut();
            let Some(j) = jobs.iter_mut().find(|j| j.id == report.id) else {
                return Ok(());
            };
            if j.state != State::Running {
                return Ok(());
            }
            j.units = report.units;
            j.elapsed = report.elapsed;
            j.state = report.state;
            j.state == State::Failed
        };
        if failed {
            self.notification.set(format!(
                "Notification: Job {} failed — simulated processing error",
                report.id
            ))?;
        }
        self.publish()
    }
}
struct Rows {
    jobs: Vec<Job>,
    store: Weak<Store>,
}
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.jobs.get(i).map(|j| format!("job-{}", j.id))
    }
    fn cell(&self, i: usize, col: &str) -> Option<Element> {
        let j = self.jobs.get(i)?;
        let store = self.store.upgrade()?;
        Some(match col {
            "name" => Label::new(j.name()).key("name"),
            "progress" => Row::new([
                ProgressBar::new(store.summary.sibling(ProgressStatus::of(j.units, j.total)))
                    .key("bar")
                    .with_grow(1),
                Label::new(format!(
                    "{:.0}% ({}/{})",
                    j.fraction() * 100.,
                    j.units,
                    j.total
                ))
                .key("percentage"),
            ])
            .key("progress"),
            "state" => Label::new(if j.state == State::Failed {
                "failed: simulated error"
            } else {
                j.state.text()
            })
            .key("state"),
            "elapsed" => Label::new(format!("{:.1}s", j.elapsed.as_secs_f64())).key("elapsed"),
            "action" if j.state == State::Running => {
                let weak = self.store.clone();
                let id = j.id;
                Button::new("Cancel", move |_| {
                    if let Some(s) = weak.upgrade() {
                        s.cancel(id)?;
                    }
                    Ok(())
                })
                .key("cancel")
            }
            _ => Label::new("").key("empty"),
        })
    }
}
struct Monitor {
    store: Rc<Store>,
    mailbox: Mailbox<Report>,
    threads: RefCell<Vec<(Sender<Command>, JoinHandle<()>)>>,
    rng: RefCell<Rng>,
    plans: RefCell<std::collections::VecDeque<Plan>>,
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.mailbox.close();
        for (tx, _) in self.threads.get_mut().iter() {
            let _ = tx.send(Command::Stop);
        }
        for (_, h) in self.threads.get_mut().drain(..) {
            let _ = h.join();
        }
    }
}
impl Monitor {
    fn start(&self) -> Result<(), RuntimeError> {
        let id = self.store.jobs.borrow().len() + 1;
        let plan = self.plans.borrow_mut().pop_front().unwrap_or_else(|| {
            let mut r = self.rng.borrow_mut();
            let total = r.range(50, 500);
            Plan {
                total,
                fail_at: if id % 5 == 0 {
                    Some(r.range(1, total - 1))
                } else {
                    None
                },
                manual: false,
                seed: r.next(),
            }
        });
        let (tx, rx) = mpsc::channel();
        self.store.jobs.borrow_mut().push(Job {
            id,
            total: plan.total,
            units: 0,
            elapsed: Duration::ZERO,
            state: State::Running,
            command: tx.clone(),
        });
        let out = self.mailbox.sender();
        let handle = thread::spawn(move || worker(id, plan, rx, out));
        self.threads.borrow_mut().push((tx, handle));
        self.store.publish()
    }
}
fn mount(ui: &mut Runtime) -> Result<(Rc<Monitor>, WidgetId), RuntimeError> {
    let summary = ui.signal(String::from(
        "Running: 0   Done: 0   Cancelled: 0   Failed: 0",
    ));
    let table = TableController::new(
        ui,
        [
            TableColumn::new("name", "Name")
                .width(110u16.into())
                .sortable(true),
            TableColumn::new("progress", "Progress")
                .width(330u16.into())
                .sortable(true),
            TableColumn::new("state", "State")
                .width(210u16.into())
                .sortable(true),
            TableColumn::new("elapsed", "Elapsed")
                .width(100u16.into())
                .sortable(true),
            TableColumn::new("action", "Action").width(100u16.into()),
        ],
        TableSource::new(Rows {
            jobs: vec![],
            store: Weak::new(),
        }),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let store = Rc::new(Store {
        jobs: RefCell::new(vec![]),
        table,
        summary,
        notification: ui.signal("Notifications: no failures".to_owned()),
    });
    let target = store.clone();
    let mailbox = ui.mailbox(Capacity::new(1024)?, move |r| target.report(r))?;
    let monitor = Rc::new(Monitor {
        store: store.clone(),
        mailbox,
        threads: RefCell::new(vec![]),
        rng: RefCell::new(Rng(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            | 1)),
        plans: RefCell::default(),
    });
    let start = monitor.clone();
    let sorting = store.clone();
    let root = ui.mount_tree(
        Column::new([
            Label::new("Background jobs").key("title"),
            Button::new("Start job", move |_| start.start()).key("start"),
            Label::new(&store.summary).key("summary"),
            Table::new(&store.table, 42.)?
                .on_sort(move |_, _| sorting.publish())
                .key("jobs")
                .with_grow(1),
            Label::new(&store.notification).key("notification"),
        ])
        .key("monitor"),
    )?;
    Ok((monitor, root))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--headless") {
        headless()?;
    } else {
        App::new("Background job monitor", |ui| {
            mount(ui)?;
            Ok(())
        })
        .run()?;
    }
    Ok(())
}
type CheckResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn press(ui: &mut Runtime, id: &WidgetId, key: pert_ui::KeyCode) -> CheckResult {
    ui.focus(id)?;
    ui.key_input(key, pert_ui::KeyAction::Press)?;
    ui.key_input(key, pert_ui::KeyAction::Release)?;
    Ok(())
}
fn wait_for(ui: &mut Runtime, condition: impl Fn() -> bool) -> CheckResult {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        ui.flush(Duration::ZERO)?;
        if condition() {
            return Ok(());
        }
        assert!(Instant::now() < deadline, "worker response timed out");
        thread::sleep(Duration::from_millis(2));
    }
}
fn visible_text(ui: &Runtime) -> String {
    ui.inspect()
        .components
        .iter()
        .filter_map(|c| c.details.semantics.as_ref())
        .map(|s| format!("{:?} {:?} {:?}", s.name, s.value, s.description))
        .collect::<Vec<_>>()
        .join("\n")
}
fn headless() -> CheckResult {
    use pert_ui::{CaptureRequest, CaptureTarget, KeyAction, KeyCode, Modifiers};
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000., 520., 1.)?)?;
    let (monitor, root) = mount(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    monitor.plans.borrow_mut().extend([
        Plan {
            total: 50,
            fail_at: None,
            manual: true,
            seed: 1,
        },
        Plan {
            total: 500,
            fail_at: None,
            manual: true,
            seed: 2,
        },
        Plan {
            total: 200,
            fail_at: None,
            manual: true,
            seed: 3,
        },
        Plan {
            total: 100,
            fail_at: None,
            manual: true,
            seed: 4,
        },
        Plan {
            total: 80,
            fail_at: Some(20),
            manual: true,
            seed: 5,
        },
    ]);
    let start = ui
        .find_descendant(&root, "start")?
        .ok_or("start button missing")?;
    for _ in 0..5 {
        press(&mut ui, &start, KeyCode::Space)?;
        ui.flush(Duration::ZERO)?;
    }
    assert_eq!(monitor.store.jobs.borrow().len(), 5);
    assert!(visible_text(&ui).contains("Running: 5"));
    let tick = |id: usize, n: u64| {
        monitor.store.jobs.borrow()[id - 1]
            .command
            .send(Command::Tick(n))
            .unwrap();
    };
    tick(1, 20);
    tick(2, 10);
    tick(3, 15);
    tick(4, 5);
    tick(5, 20);
    wait_for(&mut ui, || {
        monitor.store.jobs.borrow().iter().all(|j| j.units > 0)
    })?;
    let text = visible_text(&ui);
    assert!(
        text.contains("40% (20/50)"),
        "progress label missing: {text}"
    );
    assert!(text.contains("failed: simulated error"));
    assert!(text.contains("Notification: Job 5 failed"));
    assert!(text.contains("Running: 4   Done: 0   Cancelled: 0   Failed: 1"));
    let row = ui.find_descendant(&root, "job-2")?.ok_or("row 2 missing")?;
    let cancel = ui
        .find_descendant(&row, "cancel")?
        .ok_or("cancel missing")?;
    press(&mut ui, &cancel, KeyCode::Space)?;
    ui.flush(Duration::ZERO)?;
    let stopped = monitor.store.jobs.borrow()[1].units;
    let cancel_deadline = Instant::now() + Duration::from_millis(250);
    while !monitor.threads.borrow()[1].1.is_finished() {
        assert!(
            Instant::now() < cancel_deadline,
            "cancelled worker did not stop promptly"
        );
        thread::sleep(Duration::from_millis(2));
    }
    ui.flush(Duration::ZERO)?;
    assert_eq!(monitor.store.jobs.borrow()[1].state, State::Cancelled);
    assert_eq!(monitor.store.jobs.borrow()[1].units, stopped);
    tick(1, 20);
    wait_for(&mut ui, || monitor.store.jobs.borrow()[0].units == 40)?;
    tick(1, 20);
    wait_for(&mut ui, || {
        monitor.store.jobs.borrow()[0].state == State::Done
    })?;
    assert!(visible_text(&ui).contains("Running: 2   Done: 1   Cancelled: 1   Failed: 1"));
    let table = ui.find_descendant(&root, "jobs")?.ok_or("table missing")?;
    for column in ["name", "progress", "state", "elapsed"] {
        for (direction, key) in [
            (SortDirection::Ascending, KeyCode::ArrowUp),
            (SortDirection::Descending, KeyCode::ArrowDown),
        ] {
            monitor
                .store
                .table
                .set_active_column(Some(column.to_owned()))?;
            ui.focus(&table)?;
            ui.native_modifiers(Modifiers {
                alt: true,
                ..Modifiers::default()
            })?;
            ui.key_input(key, KeyAction::Press)?;
            ui.key_input(key, KeyAction::Release)?;
            ui.native_modifiers(Modifiers::default())?;
            ui.flush(Duration::ZERO)?;
            assert_eq!(
                monitor.store.table.sort().get()?,
                Some(TableSort {
                    column: column.to_owned(),
                    direction
                })
            );
            // Independent fixture expectations, including stable ties.
            let mut expected = match column {
                "name" => vec![1, 2, 3, 4, 5],
                "progress" => vec![2, 4, 3, 5, 1],
                "state" => vec![2, 1, 5, 3, 4],
                "elapsed" => vec![2, 3, 4, 5, 1],
                _ => unreachable!(),
            };
            if direction == SortDirection::Descending {
                expected.reverse();
            }
            let body = ui.children(&table)?[0].clone();
            let actual: Vec<_> = ui
                .children(&body)?
                .iter()
                .map(|id| ui.key(id).unwrap().to_owned())
                .collect();
            let wanted: Vec<_> = expected.iter().map(|id| format!("job-{id}")).collect();
            assert_eq!(actual, wanted, "sort {column} {direction:?}");
        }
    }
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "monitor".into(),
    ])));
    ui.flush(Duration::ZERO)?;
    pert_ui::cpu::CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    std::fs::write("jobs.png", capture.image.encode_png())?;
    // Stop every remaining job, then prove arbitrary future flushes do no work.
    monitor.store.cancel(3)?;
    monitor.store.cancel(4)?;
    for (_, handle) in monitor.threads.borrow_mut().drain(..) {
        handle.join().expect("worker panicked");
    }
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(ui.next_deadline(), None);
    assert!(!ui.needs_paint());
    for now in [Duration::from_secs(1), Duration::from_secs(60)] {
        let report = ui.flush(now)?;
        assert_eq!(report.components_updated, 0);
        assert!(!ui.needs_paint());
        assert_eq!(ui.next_deadline(), None);
    }
    assert!(visible_text(&ui).contains("Running: 0   Done: 1   Cancelled: 3   Failed: 1"));
    println!(
        "PASS: UI start/progress/completion, cancellation, failure notification, all four sorts in both directions, idle sleep; CPU capture: jobs.png"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn acceptance() {
        super::headless().unwrap();
    }
}
