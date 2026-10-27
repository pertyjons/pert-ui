//! The job monitor's interface: a Start button, a summary line and a table of
//! jobs, inside a toast host that shows failures.
//!
//! Workers never touch UI objects: each one owns its plan, a cancel flag, the
//! shared clock and a `UiSender<JobEvent>`. The mailbox handler, on the UI
//! thread, updates the model and republishes the table.

use std::cell::RefCell;
use std::cmp::Ordering as CmpOrdering;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use pert_ui::prelude::*;
use pert_ui::{
    Capacity, Container, ListSelection, ProgressBar, ProgressStatus, SelectionMode, SendError, SortDirection,
    Table, TableColumn, TableController, TableItems, TableSort, TableSource, Toast, ToastHost,
    ToastQueue, ToastReceipt, UiSender,
};

use crate::sim::{Clock, JobEvent, Outcome, Planner, Reporter, TICK, run_job};

/// The columns, in order: key, title and fixed width in logical pixels.
pub const COLUMNS: [(&str, &str, f32); 5] = [
    ("name", "Name", 110.0),
    ("progress", "Progress", 230.0),
    ("state", "State", 250.0),
    ("elapsed", "Elapsed", 100.0),
    ("action", "", 110.0),
];
/// Width of a row's progress bar, leaving room for its percentage.
const BAR_WIDTH: f32 = 150.0;
/// Height of one table row.
pub const ROW_HEIGHT: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    Running,
    Done,
    Cancelled,
    Failed,
}

impl JobState {
    pub fn name(self) -> &'static str {
        match self {
            JobState::Running => "running",
            JobState::Done => "done",
            JobState::Cancelled => "cancelled",
            JobState::Failed => "failed",
        }
    }
}

/// The UI thread's record of one job.
#[derive(Clone)]
pub struct Job {
    pub id: u32,
    pub name: String,
    pub total: u32,
    pub done: u32,
    pub ticks: u32,
    pub state: JobState,
    pub error: Option<String>,
    progress: Signal<ProgressStatus>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    /// The state as the row shows it, with the failure reason.
    pub fn state_text(&self) -> String {
        match &self.error {
            Some(reason) => format!("failed: {reason}"),
            None => self.state.name().to_owned(),
        }
    }

    pub fn percent(&self) -> u32 {
        (u64::from(self.done) * 100 / u64::from(self.total.max(1))) as u32
    }

    pub fn elapsed(&self) -> Duration {
        TICK * self.ticks
    }

    pub fn elapsed_text(&self) -> String {
        format!("{:.1} s", self.elapsed().as_secs_f64())
    }

    fn status(&self) -> ProgressStatus {
        match self.state {
            JobState::Running => ProgressStatus::of(u64::from(self.done), u64::from(self.total)),
            JobState::Done => ProgressStatus::Succeeded,
            JobState::Cancelled => ProgressStatus::Cancelled,
            JobState::Failed => ProgressStatus::Failed,
        }
    }
}

/// Counts for the summary line.
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

/// A failure notification that was posted, with its receipt.
#[derive(Clone, Debug)]
pub struct Notification {
    pub job: u32,
    pub message: String,
    pub receipt: ToastReceipt,
}

#[derive(Default)]
struct Model {
    jobs: Vec<Job>,
    notifications: Vec<Notification>,
    sort: Option<TableSort>,
    next_number: u32,
}

impl Model {
    fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for job in &self.jobs {
            match job.state {
                JobState::Running => counts.running += 1,
                JobState::Done => counts.done += 1,
                JobState::Cancelled => counts.cancelled += 1,
                JobState::Failed => counts.failed += 1,
            }
        }
        counts
    }

    /// The jobs in the order the table shows them.
    fn sorted(&self) -> Vec<Job> {
        let mut rows = self.jobs.clone();
        if let Some(sort) = &self.sort {
            rows.sort_by(|a, b| {
                let primary = compare(&sort.column, a, b);
                let primary = match sort.direction {
                    SortDirection::Ascending => primary,
                    SortDirection::Descending => primary.reverse(),
                };
                primary.then(a.id.cmp(&b.id))
            });
        }
        rows
    }
}

/// Orders two jobs by one column, ascending.
pub fn compare(column: &str, a: &Job, b: &Job) -> CmpOrdering {
    match column {
        "name" => a.id.cmp(&b.id),
        // done/total compared without rounding: a.done * b.total vs b.done * a.total.
        "progress" => (u64::from(a.done) * u64::from(b.total))
            .cmp(&(u64::from(b.done) * u64::from(a.total))),
        "state" => a.state.name().cmp(b.state.name()),
        "elapsed" => a.ticks.cmp(&b.ticks),
        _ => CmpOrdering::Equal,
    }
}

/// A snapshot of the rows, rebuilt whenever a job changes.
struct Rows {
    jobs: Vec<Job>,
    clock: Arc<dyn Clock>,
}

impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }

    fn key(&self, index: usize) -> Option<String> {
        self.jobs.get(index).map(|job| row_key(job.id))
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.jobs.iter().position(|job| row_key(job.id) == key)
    }

    fn text(&self, index: usize, column: &str) -> Option<String> {
        let job = self.jobs.get(index)?;
        match column {
            "name" => Some(job.name.clone()),
            "progress" => Some(format!("{}%", job.percent())),
            "state" => Some(job.state_text()),
            "elapsed" => Some(job.elapsed_text()),
            _ => None,
        }
    }

    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.jobs.get(index)?;
        Some(match column {
            "name" => Label::new(job.name.clone()).key("name"),
            "progress" => Row::new([
                // A bar prefers three control widths, wider than the cell, and rows
                // never shrink a child: size it here.
                Container::new(ProgressBar::new(job.progress.read_only()).key("bar"))
                    .preferred_size(Size::new(BAR_WIDTH, 8.0).expect("valid size"))
                    .key("track"),
                Label::new(format!("{}%", job.percent())).key("percent"),
            ])
            .key("progress"),
            "state" => {
                let label = Label::new(job.state_text());
                let label = if job.state == JobState::Failed { label.color_role(ColorRole::Error) } else { label };
                label.key("state")
            }
            "elapsed" => Label::new(job.elapsed_text()).key("elapsed"),
            "action" if job.state == JobState::Running => {
                let (cancel, clock) = (job.cancel.clone(), self.clock.clone());
                Button::new("Cancel", move |_| {
                    cancel.store(true, Ordering::SeqCst);
                    clock.wake();
                    Ok(())
                })
                .key("cancel")
            }
            _ => return None,
        })
    }
}

pub fn row_key(id: u32) -> String {
    format!("job-{id}")
}

/// Sends reports into the UI mailbox, retrying terminal ones while it is full.
struct MailboxReporter(UiSender<JobEvent>);

impl Reporter for MailboxReporter {
    fn send(&self, mut event: JobEvent, terminal: bool) -> bool {
        loop {
            match self.0.send(event) {
                Ok(()) => return true,
                Err(SendError::Closed(_)) => return false,
                // A newer progress report follows soon; drop this one.
                Err(SendError::Full(_)) if !terminal => return true,
                Err(SendError::Full(back)) => {
                    event = back;
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
    }
}

/// The application's handles. Keep it alive as long as the UI: it owns the
/// mailbox that workers report to.
pub struct Monitor {
    pub root: WidgetId,
    pub table: TableController,
    pub toasts: ToastQueue,
    pub summary: ReadSignal<String>,
    model: Rc<RefCell<Model>>,
    _mailbox: pert_ui::Mailbox<JobEvent>,
}

impl Monitor {
    /// A copy of every job, in start order.
    pub fn jobs(&self) -> Vec<Job> {
        self.model.borrow().jobs.clone()
    }

    /// The jobs in the order the table shows them.
    pub fn shown(&self) -> Vec<Job> {
        self.model.borrow().sorted()
    }

    pub fn counts(&self) -> Counts {
        self.model.borrow().counts()
    }

    /// The failure notifications posted so far.
    pub fn notifications(&self) -> Vec<Notification> {
        self.model.borrow().notifications.clone()
    }
}

/// Shared by the callbacks that change the table.
#[derive(Clone)]
struct Publisher {
    model: Rc<RefCell<Model>>,
    table: TableController,
    summary: Signal<String>,
    clock: Arc<dyn Clock>,
}

impl Publisher {
    fn publish(&self) -> Result<(), RuntimeError> {
        let (rows, counts) = {
            let model = self.model.borrow();
            (model.sorted(), model.counts())
        };
        self.table.set_source(TableSource::new(Rows { jobs: rows, clock: self.clock.clone() }))?;
        self.summary.set_changed(counts.text())?;
        Ok(())
    }
}

/// Builds the monitor into `ui` and mounts it.
pub fn build(
    ui: &mut Runtime,
    clock: Arc<dyn Clock>,
    planner: Box<dyn Planner>,
) -> Result<Monitor, RuntimeError> {
    let model = Rc::new(RefCell::new(Model { next_number: 1, ..Model::default() }));
    let columns = COLUMNS.iter().map(|&(key, title, width)| {
        let column = TableColumn::new(key, title).width(Length::new(width).expect("valid width"));
        if key == "action" { column } else { column.sortable(true) }
    });
    let table = TableController::new(
        ui,
        columns,
        TableSource::new(Rows { jobs: Vec::new(), clock: clock.clone() }),
        ListSelection::new(ui, SelectionMode::Single),
    )?;
    let summary = ui.signal(Counts::default().text());
    let toasts = ToastQueue::new(ui);
    let publisher = Publisher { model: model.clone(), table: table.clone(), summary: summary.clone(), clock: clock.clone() };

    // Worker reports arrive here, on the UI thread.
    let (on_event, notify) = (publisher.clone(), toasts.clone());
    let mailbox = ui.mailbox(Capacity::new(256)?, move |event: JobEvent| {
        let failure = {
            let mut model = on_event.model.borrow_mut();
            let Some(job) = model.jobs.iter_mut().find(|job| job.id == event.id) else {
                return Ok(());
            };
            job.done = event.done;
            job.ticks = event.ticks;
            job.state = match &event.outcome {
                Outcome::Running => JobState::Running,
                Outcome::Done => JobState::Done,
                Outcome::Cancelled => JobState::Cancelled,
                Outcome::Failed(reason) => {
                    job.error = Some(reason.clone());
                    JobState::Failed
                }
            };
            job.progress.set_changed(job.status())?;
            job.error.as_ref().map(|reason| format!("{} failed: {reason}", job.name))
        };
        if let Some(message) = failure {
            match notify.push(Toast::new(message.clone())) {
                Ok(receipt) => on_event.model.borrow_mut().notifications.push(Notification {
                    job: event.id,
                    message,
                    receipt,
                }),
                // The row still shows the failure when the queue is full.
                Err(RuntimeError::ToastFull) => {}
                Err(error) => return Err(error),
            }
        }
        on_event.publish()
    })?;

    let sender = mailbox.sender();
    let (start, start_clock) = (publisher.clone(), clock.clone());
    let planner = RefCell::new(planner);
    let start_button = Button::new("Start job", move |cx| {
        let (id, plan, job) = {
            let mut model = start.model.borrow_mut();
            let id = model.next_number;
            model.next_number += 1;
            let plan = planner.borrow_mut().plan(id);
            let job = Job {
                id,
                name: format!("Job {id}"),
                total: plan.total,
                done: 0,
                ticks: 0,
                state: JobState::Running,
                error: None,
                progress: cx.signal(ProgressStatus::of(0, u64::from(plan.total))),
                cancel: Arc::new(AtomicBool::new(false)),
            };
            model.jobs.push(job.clone());
            (id, plan, job)
        };
        let (clock, cancel, reporter) = (start_clock.clone(), job.cancel.clone(), MailboxReporter(sender.clone()));
        std::thread::Builder::new()
            .name(format!("job-{id}"))
            .spawn(move || run_job(id, plan, clock, cancel, reporter))
            .map_err(|error| RuntimeError::BackgroundSpawn(error.kind()))?;
        start.publish()
    });

    let on_sort = publisher.clone();
    let table_view = Table::new(&table, ROW_HEIGHT)?.on_sort(move |_, sort| {
        on_sort.model.borrow_mut().sort = Some(sort.clone());
        on_sort.publish()
    });

    let content = Column::new([
        Row::new([
            start_button.key("start"),
            Label::new(summary.read_only()).key("summary"),
        ])
        .key("toolbar"),
        table_view.key("jobs").with_grow(1),
    ]);
    let root = ui.mount_tree(ToastHost::new(toasts.clone(), content.key("content")).key("monitor"))?;

    Ok(Monitor { root, table, toasts, summary: summary.read_only(), model, _mailbox: mailbox })
}
