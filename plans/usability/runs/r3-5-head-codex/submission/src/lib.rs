use pert_ui::live::{Capacity, Mailbox, UiSender};
use pert_ui::prelude::*;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Running,
    Done,
    Cancelled,
    Failed,
}
impl State {
    pub fn text(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed: simulated error",
        }
    }
}
#[derive(Clone)]
pub struct Job {
    pub id: usize,
    pub total: u32,
    pub processed: u32,
    pub state: State,
    pub elapsed: Duration,
    cancel: mpsc::Sender<()>,
}
impl Job {
    pub fn name(&self) -> String {
        format!("Job {}", self.id)
    }
    pub fn percent(&self) -> f64 {
        self.processed as f64 / self.total as f64
    }
}
#[derive(Clone)]
pub struct Plan {
    pub total: u32,
    pub step: Option<u32>,
    pub fail_at: Option<u32>,
}
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, low: u32, high: u32) -> u32 {
        low + (self.next() % (high - low + 1) as u64) as u32
    }
}
struct Update {
    id: usize,
    processed: u32,
    elapsed: Duration,
    state: State,
}
struct Model {
    jobs: Vec<Job>,
    rng: Rng,
    plans: std::collections::VecDeque<Plan>,
}
struct Rows(Vec<Job>);
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|j| j.id.to_string())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let j = self.0.get(index)?;
        Some(match column {
            "name" => Label::new(j.name()).key("value"),
            "progress" => Row::new([
                ProgressBar::new(ProgressStatus::of(j.processed as u64, j.total as u64))
                    .key("bar")
                    .with_grow(1),
                Label::new(format!("{:.0}%", j.percent() * 100.0)).key("percent"),
            ])
            .key("value"),
            "state" => Label::new(j.state.text()).key("value"),
            "elapsed" => Label::new(format!("{:.1} s", j.elapsed.as_secs_f64())).key("value"),
            "action" if j.state == State::Running => {
                let cancel = j.cancel.clone();
                Button::new("Cancel", move |_| {
                    let _ = cancel.send(());
                    Ok(())
                })
                .key("cancel")
            }
            "action" => Label::new("").key("value"),
            _ => return None,
        })
    }
}
#[derive(Clone)]
struct View {
    model: Rc<RefCell<Model>>,
    table: TableController,
    summary: Signal<String>,
    notice: Signal<String>,
}
impl View {
    fn publish(&self) -> Result<(), RuntimeError> {
        let mut jobs = self.model.borrow().jobs.clone();
        let counts = [State::Running, State::Done, State::Cancelled, State::Failed]
            .map(|s| jobs.iter().filter(|j| j.state == s).count());
        self.summary.set(format!(
            "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
            counts[0], counts[1], counts[2], counts[3]
        ))?;
        if let Some(sort) = self.table.sort().get()? {
            jobs.sort_by(|a, b| {
                let order = match sort.column.as_str() {
                    "name" => a.name().cmp(&b.name()),
                    "progress" => a.percent().total_cmp(&b.percent()),
                    "state" => a.state.text().cmp(b.state.text()),
                    "elapsed" => a.elapsed.cmp(&b.elapsed),
                    _ => a.id.cmp(&b.id),
                }
                .then(a.id.cmp(&b.id));
                if sort.direction == SortDirection::Descending {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        self.table.set_source(TableSource::new(Rows(jobs)))
    }
    fn start(&self, sender: UiSender<Update>) -> Result<(), RuntimeError> {
        let mut model = self.model.borrow_mut();
        let id = model.jobs.len() + 1;
        let plan = model.plans.pop_front().unwrap_or_else(|| {
            let total = model.rng.range(50, 500);
            Plan {
                total,
                step: None,
                fail_at: (id % 5 == 0).then(|| model.rng.range(1, total - 1)),
            }
        });
        let seed = model.rng.next();
        let (cancel, receiver) = mpsc::channel();
        model.jobs.push(Job {
            id,
            total: plan.total,
            processed: 0,
            state: State::Running,
            elapsed: Duration::ZERO,
            cancel,
        });
        drop(model);
        self.publish()?;
        thread::spawn(move || {
            let start = Instant::now();
            let mut rng = Rng(seed);
            let mut processed = 0;
            loop {
                let state = match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => State::Cancelled,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        processed = (processed + plan.step.unwrap_or_else(|| rng.range(5, 20)))
                            .min(plan.total);
                        if let Some(point) = plan.fail_at {
                            if processed >= point {
                                processed = point;
                                State::Failed
                            } else {
                                State::Running
                            }
                        } else if processed == plan.total {
                            State::Done
                        } else {
                            State::Running
                        }
                    }
                };
                let mut update = Update {
                    id,
                    processed,
                    state,
                    elapsed: start.elapsed(),
                };
                // Preserve terminal results even if the UI is temporarily busy.
                loop {
                    match sender.send(update) {
                        Ok(()) => break,
                        Err(pert_ui::SendError::Full(value)) => {
                            update = value;
                            thread::sleep(Duration::from_millis(1));
                        }
                        Err(pert_ui::SendError::Closed(_)) => return,
                    }
                }
                if state != State::Running {
                    return;
                }
            }
        });
        Ok(())
    }
}
pub struct Monitor {
    pub root: WidgetId,
    view: View,
    _mailbox: Mailbox<Update>,
}
impl Monitor {
    pub fn jobs(&self) -> Vec<Job> {
        self.view.model.borrow().jobs.clone()
    }
}
pub fn mount(ui: &mut Runtime, plans: Vec<Plan>) -> Result<Monitor, RuntimeError> {
    let model = Rc::new(RefCell::new(Model {
        jobs: vec![],
        rng: Rng(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            | 1),
        plans: plans.into(),
    }));
    let columns = [
        ("name", "Name", 120),
        ("progress", "Progress", 230),
        ("state", "State", 240),
        ("elapsed", "Elapsed", 110),
        ("action", "Action", 110),
    ]
    .map(|(key, title, width)| {
        TableColumn::new(key, title)
            .width(Length::from(width))
            .sortable(key != "action")
    });
    let table = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        columns,
        TableSource::new(Rows(vec![])),
    )?;
    let view = View {
        model,
        table,
        summary: ui.signal(String::new()),
        notice: ui.signal(String::new()),
    };
    let target = view.clone();
    let mailbox = ui.mailbox(Capacity::new(4096).unwrap(), move |update: Update| {
        {
            let mut model = target.model.borrow_mut();
            if let Some(job) = model.jobs.iter_mut().find(|j| j.id == update.id) {
                if job.state != State::Running {
                    return Ok(());
                }
                job.processed = update.processed;
                job.elapsed = update.elapsed;
                job.state = update.state;
                if update.state == State::Failed {
                    target.notice.set(format!(
                        "Notification: {} failed — simulated error at {} / {} units",
                        job.name(),
                        job.processed,
                        job.total
                    ))?;
                }
            }
        }
        target.publish()
    })?;
    let start = view.clone();
    let sender = mailbox.sender();
    let sorted = view.clone();
    let theme = ui.theme();
    let height = theme
        .metrics(theme.density()?)?
        .metric(MetricRole::ControlHeight);
    let root = ui.mount_tree(
        Column::new([
            Label::new("Background jobs").key("title"),
            Button::new("Start job", move |_| start.start(sender.clone())).key("start"),
            Label::new(view.summary.clone()).key("summary"),
            Table::new(&view.table, height)?
                .on_sort(move |_, _| sorted.publish())
                .key("jobs")
                .with_grow(1),
            Label::new(view.notice.clone()).key("notification"),
        ])
        .key("monitor"),
    )?;
    view.publish()?;
    Ok(Monitor {
        root,
        view,
        _mailbox: mailbox,
    })
}
pub fn app() -> App<Monitor> {
    App::new("Background job monitor", |ui| Ok(mount(ui, vec![])?))
        .size(Size::new(1000.0, 460.0).unwrap())
        .unwrap()
}
