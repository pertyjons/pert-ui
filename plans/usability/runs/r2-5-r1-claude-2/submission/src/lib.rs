//! Background job monitor built on pert-ui.
//!
//! Workers run on ordinary threads and only send owned [`JobEvent`]s through a
//! [`UiSender`]; every UI object (signals, the table controller, the toast queue)
//! lives on the UI thread and is changed in the mailbox handler.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use pert_ui::prelude::*;
use pert_ui::{
    ApplicationError, Capacity, Length, ListSelection, ProgressStatus, SelectionMode, SendError, SortDirection, Table,
    TableColumn, TableController, TableItems, TableSort, TableSource, Toast, ToastHost, ToastQueue,
    ToastReceipt, UiSender,
};

/// Ticks per second of a simulated job.
pub const TICK: Duration = Duration::from_millis(100);

// ---------------------------------------------------------------------------
// Randomness and job plans

/// The random source the application draws job plans from.
pub trait Random {
    /// A value in `lo..=hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32;
}

/// A small xorshift generator; good enough for a simulation.
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        XorShift(seed | 1)
    }
    pub fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        XorShift::new(nanos)
    }
}

impl Random for XorShift {
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        lo + (x % u64::from(hi - lo + 1)) as u32
    }
}

/// Everything random about one job, decided when it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct JobPlan {
    /// Total size in units.
    pub total: u32,
    /// Units processed per tick, used in turn (cycled if the job outlasts them).
    pub steps: Vec<u32>,
    /// The processed-unit count at which the job fails, if it does.
    pub fail_at: Option<u32>,
}

/// A plan with the task's distribution: 50..=500 units, 5..=20 units per tick,
/// and one job in five failing at a random point.
pub fn random_plan(rng: &mut dyn Random) -> JobPlan {
    let total = rng.range(50, 500);
    let steps = (0..total.div_ceil(5)).map(|_| rng.range(5, 20)).collect();
    let fail_at = (rng.range(1, 5) == 1).then(|| rng.range(1, total - 1));
    JobPlan { total, steps, fail_at }
}

// ---------------------------------------------------------------------------
// Cancellation and pacing (the worker's clock)

/// Shared cancellation flag a worker can also wait on.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<(Mutex<bool>, Condvar)>);

impl CancelToken {
    pub fn cancel(&self) {
        let (flag, wake) = &*self.0;
        *flag.lock().unwrap() = true;
        wake.notify_all();
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.0.lock().unwrap()
    }
    /// Waits up to `timeout`; returns whether the token was cancelled.
    pub fn wait(&self, timeout: Duration) -> bool {
        let (flag, wake) = &*self.0;
        let guard = flag.lock().unwrap();
        let (guard, _) = wake.wait_timeout_while(guard, timeout, |c| !*c).unwrap();
        *guard
    }
}

/// What a pacer reports when the worker asks for the next tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pace {
    /// A tick happened; the job's elapsed time.
    Tick(Duration),
    /// The job was cancelled while waiting; the job's elapsed time.
    Cancelled(Duration),
}

/// The worker's clock: blocks until the next tick.
pub trait Pacer: Send {
    fn next_tick(&mut self, cancel: &CancelToken) -> Pace;
}

/// Real time at [`TICK`] intervals; wakes at once on cancellation.
pub struct RealPacer {
    start: Instant,
    ticks: u32,
}

impl RealPacer {
    pub fn new() -> Self {
        RealPacer { start: Instant::now(), ticks: 0 }
    }
}

impl Default for RealPacer {
    fn default() -> Self {
        Self::new()
    }
}

impl Pacer for RealPacer {
    fn next_tick(&mut self, cancel: &CancelToken) -> Pace {
        self.ticks += 1;
        let due = TICK * self.ticks;
        let wait = due.saturating_sub(self.start.elapsed());
        if cancel.wait(wait) {
            Pace::Cancelled(self.start.elapsed())
        } else {
            Pace::Tick(self.start.elapsed())
        }
    }
}

// ---------------------------------------------------------------------------
// Worker

/// What a worker reports to the UI thread.
#[derive(Clone, Debug, PartialEq)]
pub enum JobEvent {
    Progress { id: u32, done: u32, elapsed: Duration },
    Done { id: u32, elapsed: Duration },
    Cancelled { id: u32, elapsed: Duration },
    Failed { id: u32, done: u32, elapsed: Duration, reason: String },
}

fn send_final(sender: &UiSender<JobEvent>, mut event: JobEvent) {
    // A terminal event must arrive; wait for room while the mailbox is full.
    loop {
        match sender.send(event) {
            Ok(()) | Err(SendError::Closed(_)) => return,
            Err(SendError::Full(back)) => {
                event = back;
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

/// The body of one job's thread. Touches no UI object: it only sends events.
pub fn run_job(
    id: u32,
    plan: JobPlan,
    mut pacer: Box<dyn Pacer>,
    cancel: CancelToken,
    sender: UiSender<JobEvent>,
) {
    let mut done = 0u32;
    let mut step = 0usize;
    loop {
        let elapsed = match pacer.next_tick(&cancel) {
            Pace::Cancelled(elapsed) => return send_final(&sender, JobEvent::Cancelled { id, elapsed }),
            Pace::Tick(elapsed) => elapsed,
        };
        if cancel.is_cancelled() {
            return send_final(&sender, JobEvent::Cancelled { id, elapsed });
        }
        let units = plan.steps.get(step % plan.steps.len().max(1)).copied().unwrap_or(5);
        step += 1;
        done = (done + units).min(plan.total);
        if let Some(fail_at) = plan.fail_at.filter(|&at| done >= at) {
            let reason = format!("simulated error at unit {fail_at}");
            return send_final(&sender, JobEvent::Failed { id, done: fail_at, elapsed, reason });
        }
        if done >= plan.total {
            return send_final(&sender, JobEvent::Done { id, elapsed });
        }
        // Progress is lossy: a later one carries the newer count anyway.
        let _ = sender.send(JobEvent::Progress { id, done, elapsed });
    }
}

// ---------------------------------------------------------------------------
// UI-thread model

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobState {
    Running,
    Done,
    Cancelled,
    Failed,
}

impl JobState {
    pub fn label(self) -> &'static str {
        match self {
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Cancelled => "cancelled",
            JobState::Failed => "failed",
        }
    }
}

/// One job as the UI thread knows it, with the bindings its row shows.
#[derive(Clone)]
struct Job {
    id: u32,
    name: String,
    total: u32,
    done: u32,
    state: JobState,
    elapsed: Duration,
    cancel: CancelToken,
    progress: Signal<ProgressStatus>,
    percent: Signal<String>,
    state_text: Signal<String>,
    elapsed_text: Signal<String>,
    running: Signal<bool>,
}

impl Job {
    fn fraction(&self) -> f64 {
        f64::from(self.done) / f64::from(self.total.max(1))
    }
}

/// Snapshot of one row, for tests and diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct JobView {
    pub id: u32,
    pub name: String,
    pub total: u32,
    pub done: u32,
    pub state: JobState,
    pub elapsed: Duration,
}

/// Counts shown on the summary line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub running: usize,
    pub done: usize,
    pub cancelled: usize,
    pub failed: usize,
}

impl Summary {
    pub fn text(&self) -> String {
        format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            self.running, self.done, self.cancelled, self.failed
        )
    }
}

/// Supplies a plan for job number `n`.
pub type Planner = Box<dyn FnMut(u32) -> JobPlan>;
/// Supplies the clock of job number `n`.
pub type PacerFactory = Box<dyn FnMut(u32) -> Box<dyn Pacer>>;

/// The random source and the clock, replaceable by tests.
pub struct Env {
    pub planner: Planner,
    pub pacers: PacerFactory,
}

impl Env {
    /// Random plans and real time.
    pub fn real() -> Self {
        let mut rng = XorShift::from_time();
        Env {
            planner: Box::new(move |_| random_plan(&mut rng)),
            pacers: Box::new(|_| Box::new(RealPacer::new())),
        }
    }
}

struct Monitor {
    jobs: Vec<Job>,
    next_id: u32,
    sort: Option<TableSort>,
    env: Env,
    sender: UiSender<JobEvent>,
    table: TableController,
    toasts: ToastQueue,
    toast_receipts: Vec<(String, ToastReceipt)>,
    summary: Signal<String>,
    counts: Summary,
    ui_signal_owner: Signal<()>,
}

impl Monitor {
    fn start_job(&mut self) -> Result<(), RuntimeError> {
        let id = self.next_id;
        self.next_id += 1;
        let plan = (self.env.planner)(id);
        let pacer = (self.env.pacers)(id);
        let owner = &self.ui_signal_owner;
        let job = Job {
            id,
            name: format!("Job {id}"),
            total: plan.total,
            done: 0,
            state: JobState::Running,
            elapsed: Duration::ZERO,
            cancel: CancelToken::default(),
            progress: owner.sibling(ProgressStatus::of(0, u64::from(plan.total))),
            percent: owner.sibling("0%".to_owned()),
            state_text: owner.sibling(JobState::Running.label().to_owned()),
            elapsed_text: owner.sibling(format_elapsed(Duration::ZERO)),
            running: owner.sibling(true),
        };
        let (cancel, sender) = (job.cancel.clone(), self.sender.clone());
        self.jobs.push(job);
        std::thread::Builder::new()
            .name(format!("job-{id}"))
            .spawn(move || run_job(id, plan, pacer, cancel, sender))
            .map_err(ApplicationError::new)?;
        self.refresh()
    }

    fn apply(&mut self, event: JobEvent) -> Result<(), RuntimeError> {
        let id = match &event {
            JobEvent::Progress { id, .. }
            | JobEvent::Done { id, .. }
            | JobEvent::Cancelled { id, .. }
            | JobEvent::Failed { id, .. } => *id,
        };
        let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) else { return Ok(()) };
        if job.state != JobState::Running {
            return Ok(());
        }
        let mut failure = None;
        match event {
            JobEvent::Progress { done, elapsed, .. } => {
                job.done = done;
                job.elapsed = elapsed;
            }
            JobEvent::Done { elapsed, .. } => {
                job.done = job.total;
                job.elapsed = elapsed;
                job.state = JobState::Done;
            }
            JobEvent::Cancelled { elapsed, .. } => {
                job.elapsed = elapsed;
                job.state = JobState::Cancelled;
            }
            JobEvent::Failed { done, elapsed, reason, .. } => {
                job.done = done;
                job.elapsed = elapsed;
                job.state = JobState::Failed;
                failure = Some(reason);
            }
        }
        job.progress.set(match job.state {
            JobState::Running => ProgressStatus::of(u64::from(job.done), u64::from(job.total)),
            JobState::Done => ProgressStatus::Succeeded,
            JobState::Cancelled => ProgressStatus::Cancelled,
            JobState::Failed => ProgressStatus::Failed,
        })?;
        job.percent.set(format!("{:.0}%", job.fraction() * 100.0))?;
        job.elapsed_text.set(format_elapsed(job.elapsed))?;
        job.running.set(job.state == JobState::Running)?;
        job.state_text.set(match &failure {
            Some(reason) => format!("failed: {reason}"),
            None => job.state.label().to_owned(),
        })?;
        if let Some(reason) = failure {
            let message = format!("{} failed: {reason}", job.name);
            match self.toasts.push(Toast::new(message.clone())) {
                Ok(receipt) => self.toast_receipts.push((message, receipt)),
                // A full queue only loses the notification; the row still shows the failure.
                Err(RuntimeError::ToastFull) => {}
                Err(error) => return Err(error),
            }
        }
        self.refresh()
    }

    fn compare(&self, a: &Job, b: &Job) -> Ordering {
        let Some(sort) = &self.sort else { return a.id.cmp(&b.id) };
        let by = match sort.column.as_str() {
            "progress" => a.fraction().total_cmp(&b.fraction()),
            "state" => a.state.cmp(&b.state),
            "elapsed" => a.elapsed.cmp(&b.elapsed),
            _ => Ordering::Equal,
        }
        .then(a.id.cmp(&b.id));
        match sort.direction {
            SortDirection::Ascending => by,
            SortDirection::Descending => by.reverse(),
        }
    }

    /// Recounts the summary and republishes the rows in the current order.
    fn refresh(&mut self) -> Result<(), RuntimeError> {
        let mut counts = Summary::default();
        for job in &self.jobs {
            match job.state {
                JobState::Running => counts.running += 1,
                JobState::Done => counts.done += 1,
                JobState::Cancelled => counts.cancelled += 1,
                JobState::Failed => counts.failed += 1,
            }
        }
        self.counts = counts;
        self.summary.set(counts.text())?;
        let mut rows = self.jobs.clone();
        rows.sort_by(|a, b| self.compare(a, b));
        self.table.set_source(TableSource::new(Rows(Rc::new(rows))))
    }
}

fn format_elapsed(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

fn row_key(id: u32) -> String {
    format!("job-{id}")
}

/// The published, ordered rows.
struct Rows(Rc<Vec<Job>>);

impl TableItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|job| row_key(job.id))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.0.iter().position(|job| row_key(job.id) == key)
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let job = self.0.get(index)?;
        match column {
            "name" => Some(job.name.clone()),
            "state" => job.state_text.get().ok(),
            _ => None,
        }
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.0.get(index)?;
        Some(match column {
            "name" => Element::new("name-label", Label::new(job.name.clone())),
            "progress" => Element::new(
                "progress-cell",
                Row::new([
                    Element::new("bar", ProgressBar::new(job.progress.read_only())).with_grow(1),
                    Element::new("percent", Label::new(job.percent.read_only())),
                ]),
            ),
            "state" => Element::new(
                "state-label",
                Label::new(job.state_text.read_only()).overflow(pert_ui::TextOverflow::Ellipsis),
            ),
            "elapsed" => Element::new("elapsed-label", Label::new(job.elapsed_text.read_only())),
            "action" if job.state == JobState::Running => {
                let cancel = job.cancel.clone();
                Element::new(
                    "cancel",
                    Button::new("Cancel", move |_| {
                        cancel.cancel();
                        Ok(())
                    })
                    .enabled(job.running.read_only()),
                )
            }
            _ => return None,
        })
    }
}

/// Column keys, titles and fixed widths, in display order.
pub const COLUMNS: [(&str, &str, f32); 5] = [
    ("name", "Name", 90.0),
    ("progress", "Progress", 300.0),
    ("state", "State", 260.0),
    ("elapsed", "Elapsed", 100.0),
    ("action", "", 100.0),
];

/// Row height of the job table.
pub const ROW_HEIGHT: f32 = 36.0;

/// A handle on a mounted monitor, for tests and headless sessions.
#[derive(Clone)]
pub struct MonitorHandle {
    inner: Rc<RefCell<Monitor>>,
    pub root: WidgetId,
    pub table: TableController,
}

impl MonitorHandle {
    /// The jobs in the order the table shows them.
    pub fn rows(&self) -> Vec<JobView> {
        let monitor = self.inner.borrow();
        let mut jobs: Vec<&Job> = monitor.jobs.iter().collect();
        jobs.sort_by(|a, b| monitor.compare(a, b));
        jobs.into_iter()
            .map(|j| JobView {
                id: j.id,
                name: j.name.clone(),
                total: j.total,
                done: j.done,
                state: j.state,
                elapsed: j.elapsed,
            })
            .collect()
    }
    pub fn summary(&self) -> Summary {
        self.inner.borrow().counts
    }
    pub fn summary_text(&self) -> Result<String, RuntimeError> {
        self.inner.borrow().summary.get()
    }
    /// Every notification posted, with its receipt.
    pub fn toasts(&self) -> Vec<(String, ToastReceipt)> {
        self.inner.borrow().toast_receipts.clone()
    }
}

/// Mounts the monitor into `ui`.
pub fn build(ui: &mut Runtime, env: Env) -> Result<MonitorHandle, RuntimeError> {
    let columns = COLUMNS.iter().map(|&(key, title, width)| {
        let column = TableColumn::new(key, title).width(Length::new(width).expect("valid width"));
        if key == "action" { column } else { column.sortable(true) }
    });
    let table = TableController::new(
        ui,
        columns,
        TableSource::new(Rows(Rc::default())),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let toasts = ToastQueue::new(ui);
    let summary = ui.signal(Summary::default().text());

    // The handler needs the monitor, and the monitor needs the mailbox's sender:
    // fill a slot after both exist.
    let slot: Rc<RefCell<Option<Rc<RefCell<Monitor>>>>> = Rc::default();
    let handler_slot = slot.clone();
    let mailbox = ui.mailbox(Capacity::new(1024)?, move |event: JobEvent| {
        match handler_slot.borrow().as_ref() {
            Some(monitor) => monitor.borrow_mut().apply(event),
            None => Ok(()),
        }
    })?;
    let monitor = Rc::new(RefCell::new(Monitor {
        jobs: Vec::new(),
        next_id: 1,
        sort: None,
        env,
        sender: mailbox.sender(),
        table: table.clone(),
        toasts: toasts.clone(),
        toast_receipts: Vec::new(),
        summary: summary.clone(),
        counts: Summary::default(),
        ui_signal_owner: ui.signal(()),
    }));
    *slot.borrow_mut() = Some(monitor.clone());
    ui.keep_alive(mailbox);

    let starter = monitor.clone();
    let sorter = monitor.clone();
    let view = Column::new([
        Element::new(
            "toolbar",
            Row::new([
                Element::new("start", Button::new("Start job", move |_| starter.borrow_mut().start_job())),
                Element::new("summary", Label::new(summary.read_only())),
            ]),
        ),
        Element::new(
            "jobs",
            Table::new(&table, ROW_HEIGHT)?.on_sort(move |_, sort| {
                let mut monitor = sorter.borrow_mut();
                monitor.sort = Some(sort.clone());
                monitor.refresh()
            }),
        )
        .with_grow(1),
    ]);
    let root = ui.mount_tree(Element::new("monitor", ToastHost::new(toasts, Element::new("content", view))))?;
    Ok(MonitorHandle { inner: monitor, root, table })
}
