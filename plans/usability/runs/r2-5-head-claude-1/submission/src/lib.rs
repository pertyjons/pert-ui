//! A monitor for simulated background jobs, built on pert-ui.

pub mod engine;

use std::cell::RefCell;
use std::cmp::Ordering as CmpOrdering;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use pert_ui::prelude::*;
use pert_ui::{
    Capacity, ListSelection, ProgressBar, ProgressStatus, SelectionMode, SendError, SortDirection,
    Table, TableColumn, TableController, TableItems, TableSort, TableSource, Toast,
    ToastController, ToastHost, ToastReceipt, ToastStatus, UiSender,
};

use engine::{Clock, JobPlan, Planner, Report, WorkerCount};

/// Space above and below a row's tallest control, in logical pixels.
const ROW_PADDING: f32 = 6.0;

/// The column keys, in display order.
pub const COLUMNS: [&str; 5] = ["name", "progress", "state", "elapsed", "action"];

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

/// One job as the UI knows it.
#[derive(Clone)]
struct Job {
    id: u64,
    name: String,
    size: u64,
    done: u64,
    state: JobState,
    elapsed: Duration,
    error: Option<String>,
    /// Worker reports applied so far.
    updates: u64,
    status: Signal<ProgressStatus>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    fn fraction(&self) -> f64 {
        self.done as f64 / self.size as f64
    }

    fn percent(&self) -> u64 {
        self.done * 100 / self.size
    }

    fn state_text(&self) -> String {
        match &self.error {
            Some(error) => format!("failed: {error}"),
            None => self.state.label().to_owned(),
        }
    }
}

/// Formats an elapsed time as seconds with one decimal.
pub fn format_elapsed(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

/// The application state that lives on the UI thread.
struct Monitor {
    /// Itself, for the cancel buttons the rows build.
    me: Weak<Monitor>,
    jobs: RefCell<Vec<Job>>,
    sort: RefCell<Option<TableSort>>,
    next_number: RefCell<u64>,
    planner: RefCell<Box<dyn Planner>>,
    clock: Arc<dyn Clock>,
    workers: WorkerCount,
    sender: RefCell<Option<UiSender<Report>>>,
    table: TableController,
    summary: Signal<String>,
    toasts: ToastController,
    /// Every notification posted, with its receipt.
    notices: RefCell<Vec<(String, ToastReceipt)>>,
}

impl Monitor {
    fn start_job(self: &Rc<Self>, owner: &impl StateOwner) -> Result<(), RuntimeError> {
        let number = {
            let mut next = self.next_number.borrow_mut();
            *next += 1;
            *next
        };
        let plan: JobPlan = self.planner.borrow_mut().plan();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            id: number,
            name: format!("Job {number}"),
            size: plan.size,
            done: 0,
            state: JobState::Running,
            elapsed: Duration::ZERO,
            error: None,
            updates: 0,
            status: owner.signal(ProgressStatus::of(0, plan.size)),
            cancel: cancel.clone(),
        };
        self.jobs.borrow_mut().push(job);
        if let Some(sender) = self.sender.borrow().clone() {
            engine::spawn_worker(number, plan, self.clock.clone(), cancel, &self.workers, move |report| {
                send(&sender, report)
            });
        }
        self.publish()
    }

    fn cancel_job(&self, id: u64) -> Result<(), RuntimeError> {
        let mut changed = false;
        let mut jobs = self.jobs.borrow_mut();
        if let Some(job) = jobs.iter_mut().find(|job| job.id == id && job.state == JobState::Running) {
            job.cancel.store(true, Ordering::Release);
            job.state = JobState::Cancelled;
            job.status.set(ProgressStatus::Cancelled)?;
            changed = true;
        }
        drop(jobs);
        if changed { self.publish() } else { Ok(()) }
    }

    /// Applies one worker report on the UI thread.
    fn apply(&self, report: Report) -> Result<(), RuntimeError> {
        let mut failure = None;
        {
            let mut jobs = self.jobs.borrow_mut();
            let id = match &report {
                Report::Progress { id, .. }
                | Report::Done { id, .. }
                | Report::Failed { id, .. }
                | Report::Stopped { id, .. } => *id,
            };
            let Some(job) = jobs.iter_mut().find(|job| job.id == id) else { return Ok(()) };
            if job.state != JobState::Running {
                // Reports that were on their way when the job was cancelled.
                return Ok(());
            }
            job.updates += 1;
            match report {
                Report::Progress { done, elapsed, .. } => {
                    job.done = done;
                    job.elapsed = elapsed;
                    job.status.set(ProgressStatus::of(done, job.size))?;
                }
                Report::Done { elapsed, .. } => {
                    job.done = job.size;
                    job.elapsed = elapsed;
                    job.state = JobState::Done;
                    job.status.set(ProgressStatus::Succeeded)?;
                }
                Report::Failed { done, elapsed, reason, .. } => {
                    job.done = done;
                    job.elapsed = elapsed;
                    job.state = JobState::Failed;
                    failure = Some(format!("{} failed at {}%: {reason}", job.name, job.percent()));
                    job.error = Some(reason);
                    job.status.set(ProgressStatus::Failed)?;
                }
                Report::Stopped { done, elapsed, .. } => {
                    job.done = done;
                    job.elapsed = elapsed;
                    job.state = JobState::Cancelled;
                    job.status.set(ProgressStatus::Cancelled)?;
                }
            }
        }
        if let Some(message) = failure {
            let receipt = self.toasts.push(Toast::new(message.clone()))?;
            self.notices.borrow_mut().push((message, receipt));
        }
        self.publish()
    }

    /// Sorts a copy of the jobs and shows it, with the summary line.
    fn publish(&self) -> Result<(), RuntimeError> {
        let mut rows = self.jobs.borrow().clone();
        if let Some(sort) = self.sort.borrow().as_ref() {
            sort_jobs(&mut rows, sort);
        }
        let count = |state| rows.iter().filter(|job| job.state == state).count();
        let summary = format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            count(JobState::Running),
            count(JobState::Done),
            count(JobState::Cancelled),
            count(JobState::Failed),
        );
        self.summary.set(summary)?;
        self.table.set_source(TableSource::new(Rows { jobs: rows, monitor: self.weak() }))
    }

    fn weak(&self) -> Weak<Monitor> {
        self.me.clone()
    }
}

/// Sends a report, retrying while the mailbox is full. Progress reports are
/// lossy: a full mailbox drops one, since the next one supersedes it.
fn send(sender: &UiSender<Report>, mut report: Report) -> bool {
    loop {
        match sender.send(report) {
            Ok(()) => return true,
            Err(SendError::Closed(_)) => return false,
            Err(SendError::Full(Report::Progress { .. })) => return true,
            Err(SendError::Full(back)) => {
                report = back;
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
}

fn sort_jobs(jobs: &mut [Job], sort: &TableSort) {
    jobs.sort_by(|a, b| {
        let order = match sort.column.as_str() {
            "progress" => a.fraction().partial_cmp(&b.fraction()).unwrap_or(CmpOrdering::Equal),
            "state" => a.state.cmp(&b.state),
            "elapsed" => a.elapsed.cmp(&b.elapsed),
            // Names are "Job n": by number, so "Job 10" follows "Job 9".
            _ => a.id.cmp(&b.id),
        };
        let order = order.then(a.id.cmp(&b.id));
        match sort.direction {
            SortDirection::Descending => order.reverse(),
            _ => order,
        }
    });
}

/// The rows the table shows: a sorted copy of the jobs.
struct Rows {
    jobs: Vec<Job>,
    monitor: Weak<Monitor>,
}

impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }

    fn key(&self, index: usize) -> Option<String> {
        Some(format!("job-{}", self.jobs.get(index)?.id))
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        let id: u64 = key.strip_prefix("job-")?.parse().ok()?;
        self.jobs.iter().position(|job| job.id == id)
    }

    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.jobs.get(index)?;
        Some(match column {
            "name" => Label::new(job.name.clone()).key("name"),
            "progress" => Row::new([
                ProgressBar::new(job.status.read_only()).key("bar").with_grow(1),
                Label::new(format!("{}%", job.percent())).key("percent"),
            ])
            .key("progress"),
            "state" => {
                let role = match job.state {
                    JobState::Failed => ColorRole::Error,
                    JobState::Done => ColorRole::Success,
                    JobState::Cancelled => ColorRole::Muted,
                    JobState::Running => ColorRole::OnSurface,
                };
                Label::new(job.state_text()).color_role(role).key("state")
            }
            "elapsed" => Label::new(format_elapsed(job.elapsed)).key("elapsed"),
            "action" if job.state == JobState::Running => {
                let (monitor, id) = (self.monitor.clone(), job.id);
                Button::new("Cancel", move |_| match monitor.upgrade() {
                    Some(monitor) => monitor.cancel_job(id),
                    None => Ok(()),
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
            "state" => Some(job.state_text()),
            _ => None,
        }
    }
}

/// What the application needs from outside: its clock and its random source.
pub struct Environment {
    pub clock: Arc<dyn Clock>,
    pub planner: Box<dyn Planner>,
}

impl Environment {
    /// The wall clock and random plans.
    pub fn real() -> Self {
        Environment {
            clock: Arc::new(engine::SystemClock::default()),
            planner: Box::new(engine::RandomPlanner::from_time()),
        }
    }
}

/// Handles to the mounted application, for its host and its tests.
#[derive(Clone)]
pub struct MonitorHandles {
    pub root: WidgetId,
    pub table: TableController,
    pub summary: ReadSignal<String>,
    pub toasts: ToastController,
    pub workers: WorkerCount,
    monitor: Rc<Monitor>,
}

/// What the application knows about one job, for tests.
#[derive(Clone, Debug, PartialEq)]
pub struct JobView {
    pub name: String,
    pub state: JobState,
    pub done: u64,
    pub size: u64,
    pub elapsed: Duration,
    /// Worker reports applied so far.
    pub updates: u64,
}

impl MonitorHandles {
    /// Every job, in start order.
    pub fn jobs(&self) -> Vec<JobView> {
        self.monitor
            .jobs
            .borrow()
            .iter()
            .map(|job| JobView {
                name: job.name.clone(),
                state: job.state,
                done: job.done,
                size: job.size,
                elapsed: job.elapsed,
                updates: job.updates,
            })
            .collect()
    }

    /// Every notification posted so far, with its current status.
    pub fn notifications(&self) -> Vec<(String, ToastStatus)> {
        self.monitor.notices.borrow().iter().map(|(text, receipt)| (text.clone(), receipt.status())).collect()
    }
}

/// Builds the monitor on `ui` and mounts it.
pub fn build(ui: &mut Runtime, env: Environment) -> Result<MonitorHandles, RuntimeError> {
    let columns = [
        TableColumn::new("name", "Name").sortable(true).resizable(true),
        TableColumn::new("progress", "Progress").sortable(true).resizable(true).flex(2.0)?,
        TableColumn::new("state", "State").sortable(true).resizable(true).flex(1.5)?,
        TableColumn::new("elapsed", "Elapsed").sortable(true).resizable(true),
        TableColumn::new("action", "").width(Length::new(96.0)?),
    ];
    let table = TableController::new(
        &*ui,
        ListSelection::new(&*ui, SelectionMode::Single),
        columns,
        TableSource::new(Rows { jobs: Vec::new(), monitor: Weak::new() }),
    )?;
    let summary = ui.signal(String::new());
    let toasts = ToastController::new(&*ui);
    let workers = WorkerCount::default();
    let monitor = Rc::new_cyclic(|me| Monitor {
        me: me.clone(),
        jobs: RefCell::new(Vec::new()),
        sort: RefCell::new(None),
        next_number: RefCell::new(0),
        planner: RefCell::new(env.planner),
        clock: env.clock,
        workers: workers.clone(),
        sender: RefCell::new(None),
        table: table.clone(),
        summary: summary.clone(),
        toasts: toasts.clone(),
        notices: RefCell::new(Vec::new()),
    });

    let receiver = monitor.clone();
    let mailbox = ui.mailbox(Capacity::new(1024).expect("a valid capacity"), move |report| {
        receiver.apply(report)
    })?;
    *monitor.sender.borrow_mut() = Some(mailbox.sender());
    ui.keep_alive(mailbox);
    monitor.publish()?;

    // Rows hold a Cancel button, so they are as tall as a control. The table
    // takes a fixed height, read here from the current density.
    let theme = ui.theme();
    let row_height = theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight) + ROW_PADDING;
    let starter = monitor.clone();
    let sorter = monitor.clone();
    let view = Column::new([
        Row::new([
            Button::new("Start job", move |cx| starter.start_job(cx)).key("start"),
            Label::new(summary.read_only()).key("summary").with_grow(1),
        ])
        .key("toolbar"),
        Table::new(&table, row_height)?
            .on_sort(move |_, sort| {
                *sorter.sort.borrow_mut() = Some(sort.clone());
                sorter.publish()
            })
            .key("jobs")
            .with_grow(1),
    ]);
    let root = ui.mount_tree(ToastHost::new(toasts.clone(), view.key("content")).key("monitor"))?;
    Ok(MonitorHandles { root, table, summary: summary.read_only(), toasts, workers, monitor })
}

/// The native application: the monitor in a window. `built` receives the
/// handles once the content is mounted; the application keeps them alive.
pub fn app(env: Environment, built: impl FnOnce(MonitorHandles) + 'static) -> App {
    App::new("Job monitor", move |ui| {
        let handles = build(ui, env)?;
        ui.keep_alive(handles.clone());
        built(handles);
        Ok(())
    })
}
