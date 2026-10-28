//! A monitor for simulated background jobs, built on pert-ui.
//!
//! Workers run on their own threads and report through a mailbox; every UI
//! object stays on the UI thread. The clock and the job planner are
//! replaceable so tests can drive the application deterministically.

use pert_ui::{live::*, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

/// Ten ticks per second.
pub const TICK: Duration = Duration::from_millis(100);

// ---------------------------------------------------------------------------
// Clock and randomness
// ---------------------------------------------------------------------------

/// Time as the workers and the elapsed column see it.
pub trait Clock: Send + Sync {
    /// Time since an arbitrary fixed origin.
    fn now(&self) -> Duration;
    /// Blocks the calling worker until `now()` reaches `deadline`.
    fn sleep_until(&self, deadline: Duration);
}

/// The wall clock.
pub struct SystemClock(Instant);

impl SystemClock {
    pub fn new() -> Self {
        SystemClock(Instant::now())
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep_until(&self, deadline: Duration) {
        if let Some(left) = deadline.checked_sub(self.now()) {
            std::thread::sleep(left)
        }
    }
}

/// A clock that moves only when a test advances it. Sleepers wake when the
/// time they wait for has been reached.
#[derive(Default)]
pub struct ManualClock {
    now: Mutex<Duration>,
    moved: Condvar,
}

impl ManualClock {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
        self.moved.notify_all();
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Duration {
        *self.now.lock().unwrap()
    }
    fn sleep_until(&self, deadline: Duration) {
        let mut now = self.now.lock().unwrap();
        while *now < deadline {
            now = self.moved.wait(now).unwrap();
        }
    }
}

/// A source of random numbers, used on the UI thread only.
pub trait Random {
    /// A uniform value in `low..=high`.
    fn range(&mut self, low: u64, high: u64) -> u64;
}

/// A small xorshift generator; good enough for a simulation.
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        XorShift(seed.max(1))
    }
    pub fn from_time() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e37_79b9_7f4a_7c15);
        Self::new(nanos)
    }
}

impl Random for XorShift {
    fn range(&mut self, low: u64, high: u64) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        low + x % (high - low + 1)
    }
}

/// Everything a job will do, decided on the UI thread when it starts.
#[derive(Clone, Debug, PartialEq)]
pub struct JobPlan {
    /// Total size in units.
    pub total: u64,
    /// Units processed at each tick; enough ticks to reach `total`.
    pub steps: Vec<u64>,
    /// The tick (1-based) at which the job fails, if it does.
    pub fail_at_tick: Option<usize>,
}

impl JobPlan {
    /// A random plan: 50 to 500 units, 5 to 20 per tick, and one job in five
    /// fails at a random tick before it would finish.
    pub fn random(rng: &mut dyn Random) -> JobPlan {
        let total = rng.range(50, 500);
        let mut steps = Vec::new();
        let mut sum = 0;
        while sum < total {
            let step = rng.range(5, 20);
            sum += step;
            steps.push(step);
        }
        let fail_at_tick = (rng.range(1, 5) == 1).then(|| rng.range(1, steps.len() as u64) as usize);
        JobPlan { total, steps, fail_at_tick }
    }
}

/// Where new job plans come from.
pub trait Planner {
    fn plan(&mut self, number: u32) -> JobPlan;
}

/// Plans from a random source.
pub struct RandomPlanner<R: Random>(pub R);

impl<R: Random> Planner for RandomPlanner<R> {
    fn plan(&mut self, _number: u32) -> JobPlan {
        JobPlan::random(&mut self.0)
    }
}

impl<F: FnMut(u32) -> JobPlan> Planner for F {
    fn plan(&mut self, number: u32) -> JobPlan {
        self(number)
    }
}

/// The application's environment: replaceable for tests.
pub struct Env {
    pub clock: Arc<dyn Clock>,
    pub planner: Box<dyn Planner>,
}

impl Env {
    pub fn real() -> Env {
        Env {
            clock: Arc::new(SystemClock::new()),
            planner: Box::new(RandomPlanner(XorShift::from_time())),
        }
    }
}

// ---------------------------------------------------------------------------
// Worker
// ---------------------------------------------------------------------------

/// What a worker reports to the UI thread.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub id: u32,
    pub done: u64,
    pub elapsed: Duration,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Progress,
    Done,
    Failed(String),
    Cancelled,
}

fn run_worker(
    id: u32,
    plan: JobPlan,
    started: Duration,
    clock: Arc<dyn Clock>,
    cancel: Arc<AtomicBool>,
    sender: UiSender<Report>,
) {
    let mut done = 0;
    let send = |report: Report| {
        let mut report = report;
        // A full mailbox hands the value back; retry until the UI drains it
        // or the mailbox closes.
        loop {
            match sender.send(report) {
                Ok(()) => return,
                Err(SendError::Full(back)) => {
                    report = back;
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return,
            }
        }
    };
    for (index, step) in plan.steps.iter().enumerate() {
        // Tick deadlines are absolute, so a late thread start does not
        // shift the schedule.
        clock.sleep_until(started + TICK * (index as u32 + 1));
        let elapsed = clock.now() - started;
        if cancel.load(Ordering::Acquire) {
            send(Report { id, done, elapsed, outcome: Outcome::Cancelled });
            return;
        }
        done = (done + step).min(plan.total);
        let tick = index + 1;
        if plan.fail_at_tick == Some(tick) {
            let reason = format!("I/O error at unit {done}");
            send(Report { id, done, elapsed, outcome: Outcome::Failed(reason) });
            return;
        }
        let outcome = if done >= plan.total { Outcome::Done } else { Outcome::Progress };
        let finished = outcome == Outcome::Done;
        send(Report { id, done, elapsed, outcome });
        if finished {
            return;
        }
    }
}

// ---------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum JobState {
    Running,
    Done,
    Cancelled,
    Failed(String),
}

impl JobState {
    pub fn label(&self) -> String {
        match self {
            JobState::Running => "running".into(),
            JobState::Done => "done".into(),
            JobState::Cancelled => "cancelled".into(),
            JobState::Failed(reason) => format!("failed: {reason}"),
        }
    }
    fn rank(&self) -> u8 {
        match self {
            JobState::Running => 0,
            JobState::Done => 1,
            JobState::Cancelled => 2,
            JobState::Failed(_) => 3,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Job {
    pub id: u32,
    pub name: String,
    pub total: u64,
    pub done: u64,
    pub state: JobState,
    pub elapsed: Duration,
    /// Whether the worker has sent its last report.
    pub stopped: bool,
    started: Duration,
    cancel: Arc<AtomicBool>,
}

impl Job {
    pub fn key(&self) -> String {
        format!("job-{}", self.id)
    }
    pub fn fraction(&self) -> f64 {
        self.done as f64 / self.total as f64
    }
    pub fn percent(&self) -> String {
        format!("{:.0}%", (self.fraction() * 100.0).floor())
    }
    fn status(&self) -> ProgressStatus {
        let running = ProgressStatus::of(self.done, self.total);
        match self.state {
            JobState::Running => running,
            JobState::Done => ProgressStatus::Succeeded,
            JobState::Cancelled => ProgressStatus::Cancelled(running.fraction()),
            JobState::Failed(_) => ProgressStatus::Failed(running.fraction()),
        }
    }
}

pub fn format_elapsed(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

/// The summary line for a set of jobs.
pub fn summary(jobs: &[Job]) -> String {
    let count = |rank: u8| jobs.iter().filter(|job| job.state.rank() == rank).count();
    format!(
        "Running: {} · Done: {} · Cancelled: {} · Failed: {}",
        count(0),
        count(1),
        count(2),
        count(3)
    )
}

/// Orders jobs for a table sort; ties keep job order.
pub fn sort_jobs(jobs: &mut [Job], sort: Option<&TableSort>) {
    jobs.sort_by_key(|job| job.id);
    let Some(sort) = sort else { return };
    match sort.column.as_str() {
        "name" => {}
        "progress" => jobs.sort_by(|a, b| a.fraction().total_cmp(&b.fraction())),
        "state" => jobs.sort_by_key(|job| job.state.rank()),
        "elapsed" => jobs.sort_by_key(|job| job.elapsed),
        _ => {}
    }
    if sort.direction == SortDirection::Descending {
        jobs.reverse();
    }
}

struct Model {
    jobs: Vec<Job>,
    sort: Option<TableSort>,
    next_number: u32,
    env: Env,
    sender: UiSender<Report>,
    table: TableController,
    toasts: ToastController,
    summary: Signal<String>,
    /// Lets cell buttons call back into the model.
    this: std::rc::Weak<RefCell<Model>>,
}

type Shared = Rc<RefCell<Model>>;

impl Model {
    fn start_job(&mut self) {
        self.next_number += 1;
        let id = self.next_number;
        let plan = self.env.planner.plan(id);
        let cancel = Arc::new(AtomicBool::new(false));
        let started = self.env.clock.now();
        self.jobs.push(Job {
            id,
            name: format!("Job {id}"),
            total: plan.total,
            done: 0,
            state: JobState::Running,
            elapsed: Duration::ZERO,
            stopped: false,
            started,
            cancel: cancel.clone(),
        });
        let (clock, sender) = (self.env.clock.clone(), self.sender.clone());
        std::thread::Builder::new()
            .name(format!("job-{id}"))
            .spawn(move || run_worker(id, plan, started, clock, cancel, sender))
            .expect("spawn a job thread");
    }

    fn cancel(&mut self, id: u32) {
        let now = self.env.clock.now();
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
            if job.state == JobState::Running {
                job.cancel.store(true, Ordering::Release);
                job.state = JobState::Cancelled;
                job.elapsed = now - job.started;
            }
        }
    }

    fn apply(&mut self, report: Report) -> Result<(), RuntimeError> {
        let Some(job) = self.jobs.iter_mut().find(|job| job.id == report.id) else {
            return Ok(());
        };
        if report.outcome != Outcome::Progress {
            job.stopped = true;
        }
        // Reports after a cancel change nothing else.
        if job.state != JobState::Running {
            return Ok(());
        }
        job.done = report.done;
        job.elapsed = report.elapsed;
        match report.outcome {
            Outcome::Progress => {}
            Outcome::Done => job.state = JobState::Done,
            Outcome::Cancelled => job.state = JobState::Cancelled,
            Outcome::Failed(reason) => {
                job.state = JobState::Failed(reason.clone());
                let message = format!("{} failed: {reason}", job.name);
                // A full queue only loses the toast; the row still shows it.
                match self.toasts.push(Toast::new(message)) {
                    Ok(_) | Err(RuntimeError::ToastFull) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    /// Publishes the jobs, sorted, to the table and the summary line.
    fn publish(&mut self) -> Result<(), RuntimeError> {
        let mut rows = self.jobs.clone();
        sort_jobs(&mut rows, self.sort.as_ref());
        self.summary.set(summary(&rows))?;
        self.table.set_source(TableSource::new(Rows {
            jobs: Rc::new(rows),
            model: self.this.clone(),
        }))
    }
}

struct Rows {
    jobs: Rc<Vec<Job>>,
    model: std::rc::Weak<RefCell<Model>>,
}

impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.jobs.get(index).map(Job::key)
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.jobs.get(index)?;
        Some(match column {
            "name" => Label::new(job.name.as_str()).key("text"),
            "progress" => Row::new([
                ProgressBar::new(job.status()).key("bar").with_grow(1),
                Label::new(job.percent()).key("percent"),
            ])
            .key("cell"),
            "state" => Label::new(job.state.label()).key("text"),
            "elapsed" => Label::new(format_elapsed(job.elapsed)).key("text"),
            "action" if job.state == JobState::Running => {
                let (model, id) = (self.model.clone(), job.id);
                Button::new("Cancel", move |_| {
                    let Some(model) = model.upgrade() else { return Ok(()) };
                    let mut model = model.borrow_mut();
                    model.cancel(id);
                    model.publish()
                })
                .key("cancel")
            }
            _ => return None,
        })
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        let job = self.jobs.get(index)?;
        match column {
            "name" => Some(job.name.clone()),
            "progress" => Some(job.percent()),
            "state" => Some(job.state.label()),
            "elapsed" => Some(format_elapsed(job.elapsed)),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Building the interface
// ---------------------------------------------------------------------------

/// What the application keeps and tests read.
pub struct Monitor {
    pub root: WidgetId,
    pub start: WidgetId,
    pub table: WidgetId,
    pub summary_label: WidgetId,
    pub summary: Signal<String>,
    pub toasts: ToastController,
    model: Shared,
    _mailbox: Mailbox<Report>,
}

impl Monitor {
    /// A snapshot of the jobs in job order.
    pub fn jobs(&self) -> Vec<Job> {
        self.model.borrow().jobs.clone()
    }
}

pub const COLUMNS: [&str; 5] = ["name", "progress", "state", "elapsed", "action"];

/// Mounts the monitor on `ui`.
pub fn build(ui: &mut Runtime, env: Env) -> Result<Monitor, RuntimeError> {
    let summary_signal = ui.signal(summary(&[]));
    let toasts = ToastController::new(&*ui);
    let table = TableController::new(
        &*ui,
        ListSelection::new(&*ui, SelectionMode::Single),
        [
            TableColumn::new("name", "Name").width(Length::from(100)).sortable(true),
            TableColumn::new("progress", "Progress").width(Length::from(220)).sortable(true),
            TableColumn::new("state", "State").width(Length::from(220)).sortable(true),
            TableColumn::new("elapsed", "Elapsed").width(Length::from(90)).sortable(true),
            TableColumn::new("action", "").width(Length::from(110)),
        ],
        TableSource::new(Rows { jobs: Rc::new(Vec::new()), model: std::rc::Weak::new() }),
    )?;

    // The mailbox handler needs the model, and the model needs the
    // mailbox's sender: create the model behind a slot filled just below.
    let slot: Rc<RefCell<Option<Shared>>> = Rc::new(RefCell::new(None));
    let handler_slot = slot.clone();
    let mailbox = ui.mailbox(Capacity::new(4096)?, move |report: Report| {
        let Some(model) = handler_slot.borrow().clone() else { return Ok(()) };
        let mut model = model.borrow_mut();
        model.apply(report)?;
        model.publish()
    })?;
    let model = Rc::new_cyclic(|this| {
        RefCell::new(Model {
            jobs: Vec::new(),
            sort: None,
            next_number: 0,
            env,
            sender: mailbox.sender(),
            table: table.clone(),
            toasts: toasts.clone(),
            summary: summary_signal.clone(),
            this: this.clone(),
        })
    });
    *slot.borrow_mut() = Some(model.clone());

    let theme = ui.theme();
    let row_height = theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight);
    let sorter = model.clone();
    let table_view = Table::new(&table, row_height)?.on_sort(move |_, sort| {
        let mut model = sorter.borrow_mut();
        model.sort = Some(sort.clone());
        model.publish()
    });
    let starter = model.clone();
    let start = Button::new("Start job", move |_| {
        let mut model = starter.borrow_mut();
        model.start_job();
        model.publish()
    });
    let root = ui.mount_tree(Element::new(
        "monitor",
        ToastHost::new(
            toasts.clone(),
            Element::new(
                "content",
                Column::new([
                    Row::new([
                        start.key("start"),
                        Label::new(summary_signal.read_only()).key("summary").with_grow(1),
                    ])
                    .key("toolbar"),
                    table_view.key("jobs").with_grow(1),
                ]),
            ),
        ),
    ))?;
    model.borrow_mut().publish()?;
    let find = |path: &[&str]| -> Result<WidgetId, RuntimeError> {
        ui.find(&root, path)?
            .ok_or_else(|| RuntimeError::from(ApplicationError::new(std::io::Error::other(format!("missing {path:?}")))))
    };
    let start = find(&["content", "toolbar", "start"])?;
    let table_id = find(&["content", "jobs"])?;
    let summary_label = find(&["content", "toolbar", "summary"])?;
    Ok(Monitor {
        root,
        start,
        table: table_id,
        summary_label,
        summary: summary_signal,
        toasts,
        model,
        _mailbox: mailbox,
    })
}

/// The native application.
pub fn app(env: Env) -> App<Monitor> {
    App::new("Job monitor", move |ui| Ok(build(ui, env)?))
}
