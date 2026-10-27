use pert_ui::prelude::*;
use pert_ui::{Capacity, Mailbox, UiSender};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::mpsc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
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
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    pub id: u64,
    pub total: u64,
    pub processed: u64,
    pub state: State,
    pub elapsed: Duration,
}
impl Job {
    fn percent(&self) -> f64 {
        self.processed as f64 / self.total as f64 * 100.0
    }
}
#[derive(Clone, Copy)]
pub struct Plan {
    pub total: u64,
    pub step: u64,
    pub fail_at: Option<u64>,
}
enum Command {
    Tick,
    Cancel,
}
type Controls = Rc<RefCell<BTreeMap<u64, mpsc::Sender<Command>>>>;
#[derive(Clone)]
struct Rows {
    jobs: Vec<Job>,
    store: Signal<Vec<Job>>,
    controls: Controls,
}
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.jobs.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        Some(self.jobs.get(i)?.id.to_string())
    }
    fn cell(&self, i: usize, col: &str) -> Option<Element> {
        let j = self.jobs.get(i)?;
        Some(match col {
            "name" => Label::new(format!("Job {}", j.id)).key("value"),
            "progress" => Column::new([
                ProgressBar::new(self.store.sibling(ProgressStatus::of(j.processed, j.total)))
                    .key("bar")
                    .with_grow(1),
                Label::new(format!("{:.0}%", j.percent())).key("percent"),
            ])
            .key("value"),
            "state" => Label::new(if j.state == State::Failed {
                "failed: simulated error"
            } else {
                j.state.text()
            })
            .key("value"),
            "elapsed" => Label::new(format!("{:.1} s", j.elapsed.as_secs_f64())).key("value"),
            "action" if j.state == State::Running => {
                let (controls, id) = (self.controls.clone(), j.id);
                Button::new("Cancel", move |_| {
                    if let Some(tx) = controls.borrow().get(&id) {
                        let _ = tx.send(Command::Cancel);
                    }
                    Ok(())
                })
                .key("cancel")
            }
            _ => Label::new("").key("value"),
        })
    }
    fn text(&self, i: usize, col: &str) -> Option<String> {
        let j = self.jobs.get(i)?;
        Some(match col {
            "name" => format!("Job {}", j.id),
            "state" => j.state.text().into(),
            "progress" => format!("{:.0}%", j.percent()),
            "elapsed" => format!("{:.1} s", j.elapsed.as_secs_f64()),
            _ => String::new(),
        })
    }
}
fn ordered(mut jobs: Vec<Job>, sort: Option<TableSort>) -> Vec<Job> {
    if let Some(s) = sort {
        jobs.sort_by(|a, b| {
            let order = match s.column.as_str() {
                "name" => a.id.cmp(&b.id),
                "progress" => (a.processed * b.total).cmp(&(b.processed * a.total)),
                "state" => a.state.text().cmp(b.state.text()),
                "elapsed" => a.elapsed.cmp(&b.elapsed),
                _ => a.id.cmp(&b.id),
            };
            let order = if s.direction == SortDirection::Descending {
                order.reverse()
            } else {
                order
            };
            order.then(a.id.cmp(&b.id))
        });
    }
    jobs
}
fn refresh(
    jobs: &Signal<Vec<Job>>,
    table: &TableController,
    controls: &Controls,
) -> Result<(), RuntimeError> {
    table.set_source(TableSource::new(Rows {
        jobs: ordered(jobs.get()?, table.sort().get()?),
        store: jobs.clone(),
        controls: controls.clone(),
    }))
}
fn random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
fn worker(
    mut job: Job,
    plan: Plan,
    manual: bool,
    rx: mpsc::Receiver<Command>,
    sender: UiSender<Job>,
) {
    let start = Instant::now();
    let mut ticks = 0;
    loop {
        let cmd = if manual {
            match rx.recv() {
                Ok(c) => c,
                Err(_) => return,
            }
        } else {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(c) => c,
                Err(mpsc::RecvTimeoutError::Timeout) => Command::Tick,
                Err(_) => return,
            }
        };
        match cmd {
            Command::Cancel => job.state = State::Cancelled,
            Command::Tick => {
                ticks += 1;
                job.processed = (job.processed + plan.step).min(job.total);
                if plan.fail_at.is_some_and(|at| job.processed >= at) {
                    job.state = State::Failed;
                } else if job.processed == job.total {
                    job.state = State::Done;
                }
            }
        }
        job.elapsed = if manual {
            Duration::from_millis(ticks * 100)
        } else {
            start.elapsed()
        };
        // Retry a full mailbox without losing a terminal event; disconnection ends the worker.
        let mut value = job.clone();
        loop {
            match sender.send(value) {
                Ok(()) => break,
                Err(pert_ui::SendError::Full(v)) => {
                    value = v;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => return,
            }
        }
        if job.state != State::Running {
            return;
        }
    }
}
pub struct Monitor {
    pub root: WidgetId,
    pub jobs: Signal<Vec<Job>>,
    pub table: TableController,
    controls: Controls,
    _mailbox: Mailbox<Job>,
}
impl Monitor {
    pub fn tick(&self, id: u64) {
        if let Some(tx) = self.controls.borrow().get(&id) {
            let _ = tx.send(Command::Tick);
        }
    }
    pub fn displayed(&self) -> Result<Vec<Job>, RuntimeError> {
        Ok(ordered(self.jobs.get()?, self.table.sort().get()?))
    }
}
pub fn build(ui: &mut Runtime, plans: Option<Vec<Plan>>) -> Result<Monitor, RuntimeError> {
    let jobs = ui.signal(Vec::<Job>::new());
    let controls: Controls = Rc::default();
    let table = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        [
            TableColumn::new("name", "Name").sortable(true),
            TableColumn::new("progress", "Progress").sortable(true),
            TableColumn::new("state", "State").sortable(true),
            TableColumn::new("elapsed", "Elapsed").sortable(true),
            TableColumn::new("action", "Action"),
        ],
        TableSource::new(Rows {
            jobs: vec![],
            store: jobs.clone(),
            controls: controls.clone(),
        }),
    )?;
    let notice = ui.signal("Notifications: no failures".to_owned());
    let (store, view, cancel, notification) = (
        jobs.clone(),
        table.clone(),
        controls.clone(),
        notice.clone(),
    );
    let mailbox = ui.mailbox(Capacity::new(256)?, move |job: Job| {
        if job.state == State::Failed {
            notification.set(format!(
                "Notification: Job {} failed — simulated error",
                job.id
            ))?;
        }
        if job.state != State::Running {
            cancel.borrow_mut().remove(&job.id);
        }
        store.update(|all| {
            if let Some(old) = all.iter_mut().find(|j| j.id == job.id) {
                *old = job;
            }
        })?;
        refresh(&store, &view, &cancel)
    })?;
    let sender = mailbox.sender();
    let (store, view, cancel) = (jobs.clone(), table.clone(), controls.clone());
    let manual = plans.is_some();
    let mut plans = plans.unwrap_or_default().into_iter();
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        | 1;
    let start = Button::new("Start job", move |_| {
        let id = store.get()?.len() as u64 + 1;
        let plan = if manual {
            plans.next().expect("test plan available")
        } else {
            let total = 50 + random(&mut seed) % 451;
            Plan {
                total,
                step: 5 + random(&mut seed) % 16,
                fail_at: if id % 5 == 0 {
                    Some(1 + random(&mut seed) % (total - 1))
                } else {
                    None
                },
            }
        };
        let job = Job {
            id,
            total: plan.total,
            processed: 0,
            state: State::Running,
            elapsed: Duration::ZERO,
        };
        store.update(|all| all.push(job.clone()))?;
        let (tx, rx) = mpsc::channel();
        cancel.borrow_mut().insert(id, tx);
        let sender = sender.clone();
        std::thread::spawn(move || worker(job, plan, manual, rx, sender));
        refresh(&store, &view, &cancel)
    });
    let summary = jobs.map(|all| {
        format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            all.iter().filter(|j| j.state == State::Running).count(),
            all.iter().filter(|j| j.state == State::Done).count(),
            all.iter().filter(|j| j.state == State::Cancelled).count(),
            all.iter().filter(|j| j.state == State::Failed).count()
        )
    })?;
    let (store, view, cancel) = (jobs.clone(), table.clone(), controls.clone());
    let grid = Table::new(&table, 64.0)?.on_sort(move |_, _| refresh(&store, &view, &cancel));
    let root = ui.mount_tree(
        Column::new([
            Label::new("Background job monitor").key("title"),
            start.key("start"),
            Label::new(summary).key("summary"),
            grid.key("jobs").with_grow(1),
            Banner::new(notice, []).key("notification"),
        ])
        .key("monitor"),
    )?;
    Ok(Monitor {
        root,
        jobs,
        table,
        controls,
        _mailbox: mailbox,
    })
}
