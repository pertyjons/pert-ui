//! A monitor for simulated background jobs.
//!
//! [`build`] mounts the whole interface on a [`Runtime`]; the native program
//! calls it from `App::new` and the tests call it on a headless runtime with
//! a [`ManualClock`] and a scripted [`Planner`].

pub mod sim;

use pert_ui::prelude::*;
use pert_ui::{Capacity, Mailbox, SortDirection, TableSort, UiSender};
use sim::{Clock, End, JobPlan, Planner, Report};
use std::cell::{OnceCell, RefCell};
use std::cmp::Ordering as CmpOrdering;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub use sim::{ManualClock, RandomPlanner, SystemClock, XorShift};

/// Column keys, in display order.
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

/// One job as the UI thread knows it.
#[derive(Clone)]
pub struct Job {
    pub id: u64,
    pub name: String,
    pub size: u64,
    pub done: u64,
    pub state: JobState,
    pub elapsed: Duration,
    pub error: Option<String>,
    /// Whether the worker thread has ended (it sent its final report).
    pub worker_ended: bool,
    status: Signal<ProgressStatus>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    pub fn percent(&self) -> u64 {
        self.done * 100 / self.size.max(1)
    }
    fn fraction(&self) -> f64 {
        self.done as f64 / self.size.max(1) as f64
    }
    pub fn state_text(&self) -> String {
        match &self.error {
            Some(error) => format!("failed: {error}"),
            None => self.state.label().to_owned(),
        }
    }
    pub fn elapsed_text(&self) -> String {
        format_elapsed(self.elapsed)
    }
    fn progress_status(&self) -> ProgressStatus {
        match self.state {
            JobState::Running => ProgressStatus::of(self.done, self.size),
            JobState::Done => ProgressStatus::Succeeded,
            JobState::Cancelled => ProgressStatus::Cancelled,
            JobState::Failed => ProgressStatus::Failed,
        }
    }
}

pub fn format_elapsed(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub running: usize,
    pub done: usize,
    pub cancelled: usize,
    pub failed: usize,
}

impl Counts {
    pub fn text(&self) -> String {
        format!(
            "Running: {} · Done: {} · Cancelled: {} · Failed: {}",
            self.running, self.done, self.cancelled, self.failed
        )
    }
}

/// Orders two jobs by a table column, ties broken by job number.
pub fn compare(a: &Job, b: &Job, column: &str) -> CmpOrdering {
    let primary = match column {
        "progress" => a.fraction().total_cmp(&b.fraction()),
        "state" => a.state.cmp(&b.state),
        "elapsed" => a.elapsed.cmp(&b.elapsed),
        // Names are "Job N": their natural order is the job number.
        _ => CmpOrdering::Equal,
    };
    primary.then(a.id.cmp(&b.id))
}

/// What the native program and the tests inject.
pub struct Deps {
    pub clock: Arc<dyn Clock>,
    pub planner: Box<dyn Planner>,
}

impl Deps {
    pub fn real() -> Self {
        Deps {
            clock: Arc::new(SystemClock::new()),
            planner: Box::new(RandomPlanner(XorShift::from_time())),
        }
    }
}

/// The application state, shared by the callbacks on the UI thread.
pub struct Monitor {
    jobs: RefCell<Vec<Job>>,
    sort: RefCell<Option<TableSort>>,
    table: TableController,
    summary: Signal<String>,
    toasts: ToastController,
    clock: Arc<dyn Clock>,
    planner: RefCell<Box<dyn Planner>>,
    mailbox: OnceCell<Mailbox<Report>>,
    sender: OnceCell<UiSender<Report>>,
    me: OnceCell<Weak<Monitor>>,
    /// Every notification posted, with its receipt: inspection reports how
    /// many toasts are visible but not their text.
    notices: RefCell<Vec<(String, ToastReceipt)>>,
}

impl Monitor {
    /// The jobs in the order the table shows them.
    pub fn jobs(&self) -> Vec<Job> {
        self.jobs.borrow().clone()
    }

    pub fn job(&self, name: &str) -> Option<Job> {
        self.jobs.borrow().iter().find(|job| job.name == name).cloned()
    }

    pub fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in self.jobs.borrow().iter() {
            match job.state {
                JobState::Running => counts.running += 1,
                JobState::Done => counts.done += 1,
                JobState::Cancelled => counts.cancelled += 1,
                JobState::Failed => counts.failed += 1,
            }
        }
        counts
    }

    /// The notifications posted so far and their current status.
    pub fn notifications(&self) -> Vec<(String, ToastStatus)> {
        self.notices.borrow().iter().map(|(text, receipt)| (text.clone(), receipt.status())).collect()
    }

    pub fn summary(&self) -> ReadSignal<String> {
        self.summary.read_only()
    }

    pub fn table(&self) -> &TableController {
        &self.table
    }

    /// Starts the next job on its own thread.
    pub fn start_job(&self, owner: &impl StateOwner) -> Result<(), RuntimeError> {
        let id = self.jobs.borrow().len() as u64 + 1;
        let plan: JobPlan = self.planner.borrow_mut().plan(id);
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Job {
            id,
            name: format!("Job {id}"),
            size: plan.size,
            done: 0,
            state: JobState::Running,
            elapsed: Duration::ZERO,
            error: None,
            worker_ended: false,
            status: owner.signal(ProgressStatus::of(0, plan.size)),
            cancel: cancel.clone(),
        };
        self.jobs.borrow_mut().push(job);
        let (clock, sender) = (self.clock.clone(), self.sender.get().expect("built").clone());
        let started = clock.now();
        std::thread::Builder::new()
            .name(format!("job-{id}"))
            .spawn(move || sim::run_job(id, plan, started, clock, cancel, sender))
            .map_err(|error| RuntimeError::from(ApplicationError::new(error)))?;
        self.publish()
    }

    /// Asks a running job to stop. The row shows it cancelled at once; the
    /// worker sees the flag within a few milliseconds and ends.
    pub fn cancel_job(&self, id: u64) -> Result<(), RuntimeError> {
        {
            let mut jobs = self.jobs.borrow_mut();
            let Some(job) = jobs.iter_mut().find(|job| job.id == id) else { return Ok(()) };
            if job.state != JobState::Running {
                return Ok(());
            }
            job.cancel.store(true, Ordering::Release);
            job.state = JobState::Cancelled;
            job.status.set(job.progress_status())?;
        }
        self.publish()
    }

    fn deliver(&self, report: Report) -> Result<(), RuntimeError> {
        let mut failure = None;
        {
            let mut jobs = self.jobs.borrow_mut();
            let id = match &report {
                Report::Progress { id, .. } | Report::Finished { id, .. } => *id,
            };
            let Some(job) = jobs.iter_mut().find(|job| job.id == id) else { return Ok(()) };
            match report {
                Report::Progress { done, elapsed, .. } => {
                    if job.state == JobState::Running {
                        job.done = done;
                        job.elapsed = elapsed;
                    }
                }
                Report::Finished { done, elapsed, end, .. } => {
                    job.worker_ended = true;
                    if job.state == JobState::Running {
                        job.done = done;
                        job.elapsed = elapsed;
                        job.state = match end {
                            End::Done => JobState::Done,
                            End::Cancelled => JobState::Cancelled,
                            End::Failed(error) => {
                                failure = Some(format!(
                                    "{} failed at {}%: {error}",
                                    job.name,
                                    job.percent()
                                ));
                                job.error = Some(error);
                                JobState::Failed
                            }
                        };
                    }
                }
            }
            job.status.set(job.progress_status())?;
        }
        if let Some(message) = failure {
            let receipt = self.toasts.push(Toast::new(message.clone()))?;
            self.notices.borrow_mut().push((message, receipt));
        }
        self.publish()
    }

    fn set_sort(&self, sort: &TableSort) -> Result<(), RuntimeError> {
        *self.sort.borrow_mut() = Some(sort.clone());
        self.publish()
    }

    /// Sorts the jobs and shows them: the table's rows and the summary.
    fn publish(&self) -> Result<(), RuntimeError> {
        if let Some(sort) = self.sort.borrow().as_ref() {
            self.jobs.borrow_mut().sort_by(|a, b| {
                let order = compare(a, b, &sort.column);
                match sort.direction {
                    SortDirection::Descending => order.reverse(),
                    _ => order,
                }
            });
        }
        self.summary.set(self.counts().text())?;
        let rows = Rows { jobs: self.jobs(), monitor: self.weak() };
        self.table.set_source(TableSource::new(rows))
    }

    /// The rows hold the monitor weakly, so that the controller inside it
    /// does not keep itself alive through its own source.
    fn weak(&self) -> Weak<Monitor> {
        self.me.get().cloned().unwrap_or_default()
    }
}

/// A snapshot of the jobs as table rows.
struct Rows {
    jobs: Vec<Job>,
    monitor: Weak<Monitor>,
}

impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(format!("job{}", self.jobs.get(index)?.id))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.jobs.get(index)?;
        Some(match column {
            "name" => Label::new(job.name.clone()).key("name"),
            "progress" => Row::new([
                Label::new(format!("{}%", job.percent())).key("percent"),
                // The bar prefers three control widths, more than the cell may
                // have; a small preferred size lets the row's grow decide.
                Container::new(ProgressBar::new(job.status.read_only()).key("bar"))
                    .preferred_size(Size::new(24.0, 6.0).ok()?)
                    .key("track")
                    .with_grow(1),
            ])
            .key("progress"),
            "state" => {
                let label = Label::new(job.state_text()).overflow(TextOverflow::Ellipsis);
                let label = match job.state {
                    JobState::Failed => label.color_role(ColorRole::Error),
                    JobState::Done => label.color_role(ColorRole::Success),
                    _ => label,
                };
                label.key("state")
            }
            "elapsed" => Label::new(job.elapsed_text()).key("elapsed"),
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

/// What [`build`] mounted.
pub struct Built {
    pub root: WidgetId,
    pub monitor: Rc<Monitor>,
}

/// Mounts the monitor on `ui`.
pub fn build(ui: &mut Runtime, deps: Deps) -> Result<Built, RuntimeError> {
    let columns = [
        TableColumn::new("name", "Name").sortable(true).resizable(true),
        TableColumn::new("progress", "Progress").flex(2.0)?.sortable(true).resizable(true),
        TableColumn::new("state", "State").flex(2.0)?.sortable(true).resizable(true),
        TableColumn::new("elapsed", "Elapsed").sortable(true).resizable(true),
        TableColumn::new("action", "").width(Length::new(96.0)?),
    ];
    let table = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        columns,
        TableSource::new(Rows { jobs: Vec::new(), monitor: Weak::new() }),
    )?;
    let monitor = Rc::new(Monitor {
        jobs: RefCell::new(Vec::new()),
        sort: RefCell::new(None),
        table: table.clone(),
        summary: ui.signal(Counts::default().text()),
        toasts: ToastController::new(ui),
        clock: deps.clock,
        planner: RefCell::new(deps.planner),
        mailbox: OnceCell::new(),
        sender: OnceCell::new(),
        me: OnceCell::new(),
        notices: RefCell::new(Vec::new()),
    });
    let _ = monitor.me.set(Rc::downgrade(&monitor));
    let weak = Rc::downgrade(&monitor);
    let mailbox = ui.mailbox(Capacity::new(256)?, move |report: Report| match weak.upgrade() {
        Some(monitor) => monitor.deliver(report),
        None => Ok(()),
    })?;
    let _ = monitor.sender.set(mailbox.sender());
    let _ = monitor.mailbox.set(mailbox);

    let starter = monitor.clone();
    let sorter = Rc::downgrade(&monitor);
    let content = Column::new([
        Row::new([
            Button::new("Start job", move |cx| starter.start_job(cx)).key("start"),
            Label::new(monitor.summary()).key("summary"),
        ])
        .key("toolbar"),
        Table::new(&table, 36.0)?
            .on_sort(move |_, sort| match sorter.upgrade() {
                Some(monitor) => monitor.set_sort(sort),
                None => Ok(()),
            })
            .key("jobs")
            .with_grow(1),
    ]);
    let root = ui.mount_tree(ToastHost::new(monitor.toasts.clone(), content.key("content")).key("app"))?;
    Ok(Built { root, monitor })
}

/// The native application around [`build`], and a slot that holds what it
/// built once its initializer has run (for `App::headless` checks).
pub fn app(deps: Deps) -> (App, Rc<OnceCell<Built>>) {
    let slot: Rc<OnceCell<Built>> = Rc::default();
    let built = slot.clone();
    let app = App::new("Job monitor", move |ui| {
        let _ = built.set(build(ui, deps)?);
        Ok(())
    });
    (app, slot)
}
