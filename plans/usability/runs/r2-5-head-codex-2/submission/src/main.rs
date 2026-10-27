use pert_ui::prelude::*;
use pert_ui::{
    Capacity, ProgressStatus, SortDirection, TableColumn, TableController, TableItems, TableSort,
    TableSource,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

type Shared = Rc<RefCell<Vec<Job>>>;
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
    completed: u64,
    state: State,
    elapsed: Duration,
    cancel: mpsc::Sender<()>,
    progress: Signal<ProgressStatus>,
}
impl Job {
    fn name(&self) -> String {
        format!("Job {}", self.id)
    }
}
#[derive(Clone)]
struct Rows(Vec<Job>);
impl TableItems for Rows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, i: usize) -> Option<String> {
        self.0.get(i).map(|j| j.id.to_string())
    }
    fn cell(&self, i: usize, col: &str) -> Option<Element> {
        let j = self.0.get(i)?;
        Some(match col {
            "name" => Label::new(j.name()).key("name-label"),
            "progress" => Row::new([
                ProgressBar::new(&j.progress).key("bar").with_grow(1),
                Label::new(format!("{}%", j.completed * 100 / j.total)).key("percent"),
            ])
            .key("progress-content"),
            "state" => Label::new(if j.state == State::Failed {
                "failed: simulated error"
            } else {
                j.state.text()
            })
            .key("state-label"),
            "elapsed" => {
                Label::new(format!("{:.1}s", j.elapsed.as_secs_f64())).key("elapsed-label")
            }
            "action" if j.state == State::Running => {
                let cancel = j.cancel.clone();
                Button::new("Cancel", move |_| {
                    let _ = cancel.send(());
                    Ok(())
                })
                .key("cancel")
            }
            _ => Label::new("").key("empty"),
        })
    }
}
fn compare(a: &Job, b: &Job, col: &str) -> std::cmp::Ordering {
    match col {
        "name" => a.name().cmp(&b.name()),
        "progress" => (a.completed * b.total).cmp(&(b.completed * a.total)),
        "state" => a.state.text().cmp(b.state.text()),
        "elapsed" => a.elapsed.cmp(&b.elapsed),
        _ => a.id.cmp(&b.id),
    }
    .then(a.id.cmp(&b.id))
}
fn ordered(jobs: &Shared, sort: Option<TableSort>) -> Vec<Job> {
    let mut rows = jobs.borrow().clone();
    if let Some(s) = sort {
        rows.sort_by(|a, b| {
            let c = compare(a, b, &s.column);
            if s.direction == SortDirection::Descending {
                c.reverse()
            } else {
                c
            }
        });
    }
    rows
}
fn publish(
    jobs: &Shared,
    table: &TableController,
    summary: &Signal<String>,
) -> Result<(), RuntimeError> {
    let rows = ordered(jobs, table.sort().get()?);
    let counts = [State::Running, State::Done, State::Cancelled, State::Failed]
        .map(|s| rows.iter().filter(|j| j.state == s).count());
    summary.set(format!(
        "Running: {}   Done: {}   Cancelled: {}   Failed: {}",
        counts[0], counts[1], counts[2], counts[3]
    ))?;
    table.set_source(TableSource::new(Rows(rows)))
}
struct Update {
    id: usize,
    completed: u64,
    state: State,
    elapsed: Duration,
}
#[derive(Clone)]
struct Config {
    interval: Duration,
    deterministic: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(100),
            deterministic: false,
        }
    }
}
struct Random(u64);
impl Random {
    fn range(&mut self, min: u64, max: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        min + self.0 % (max - min + 1)
    }
}
struct Monitor {
    root: WidgetId,
    jobs: Shared,
    table: TableController,
}
fn build(ui: &mut Runtime, config: Config) -> Result<Monitor, RuntimeError> {
    let jobs: Shared = Rc::default();
    let summary = ui.signal("Running: 0   Done: 0   Cancelled: 0   Failed: 0".to_owned());
    let notification = ui.signal("Notifications: none".to_owned());
    let table = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        [
            TableColumn::new("name", "Name").sortable(true),
            TableColumn::new("progress", "Progress")
                .flex(2.0)?
                .sortable(true),
            TableColumn::new("state", "State").sortable(true),
            TableColumn::new("elapsed", "Elapsed").sortable(true),
            TableColumn::new("action", "Action"),
        ],
        TableSource::new(Rows(vec![])),
    )?;
    let (js, t, s, n) = (
        jobs.clone(),
        table.clone(),
        summary.clone(),
        notification.clone(),
    );
    let mailbox = ui.mailbox(Capacity::new(4096)?, move |u: Update| {
        {
            let mut all = js.borrow_mut();
            if let Some(j) = all.iter_mut().find(|j| j.id == u.id) {
                if j.state != State::Running {
                    return Ok(());
                }
                j.completed = u.completed;
                j.elapsed = u.elapsed;
                j.state = u.state;
                j.progress.set(ProgressStatus::of(j.completed, j.total))?;
                if u.state == State::Failed {
                    n.set(format!(
                        "Notification: {} failed — simulated error at {}/{} units",
                        j.name(),
                        j.completed,
                        j.total
                    ))?;
                }
            }
        }
        publish(&js, &t, &s)
    })?;
    let sender = mailbox.sender();
    ui.keep_alive(mailbox);
    let (js, t, s) = (jobs.clone(), table.clone(), summary.clone());
    let start = Button::new("Start job", move |cx| {
        let (js, t, s, sender, config) = (
            js.clone(),
            t.clone(),
            s.clone(),
            sender.clone(),
            config.clone(),
        );
        let _receipt = cx.defer(move |ui| {
            let id = js.borrow().len() + 1;
            let mut rng = Random(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos() as u64
                    | 1,
            );
            let total = if config.deterministic {
                if id == 2 || id == 4 { 500 } else { 50 }
            } else {
                rng.range(50, 500)
            };
            let fail_at = if id % 5 == 0 {
                Some(if config.deterministic {
                    30
                } else {
                    rng.range(1, total - 1)
                })
            } else {
                None
            };
            let (cancel, receive) = mpsc::channel();
            js.borrow_mut().push(Job {
                id,
                total,
                completed: 0,
                state: State::Running,
                elapsed: Duration::ZERO,
                cancel,
                progress: ui.signal(ProgressStatus::of(0, total)),
            });
            publish(&js, &t, &s)?;
            thread::spawn(move || {
                let start = Instant::now();
                let mut completed = 0;
                loop {
                    let cancelled = match receive.recv_timeout(config.interval) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => true,
                        Err(mpsc::RecvTimeoutError::Timeout) => false,
                    };
                    let state = if cancelled {
                        State::Cancelled
                    } else {
                        completed = (completed
                            + if config.deterministic {
                                10
                            } else {
                                rng.range(5, 20)
                            })
                        .min(total);
                        if let Some(limit) = fail_at {
                            completed = completed.min(limit);
                        }
                        if fail_at.is_some_and(|at| completed >= at) {
                            State::Failed
                        } else if completed == total {
                            State::Done
                        } else {
                            State::Running
                        }
                    };
                    // Backpressure retries preserve terminal events. No UI handles cross this boundary.
                    let mut event = Update {
                        id,
                        completed,
                        state,
                        elapsed: start.elapsed(),
                    };
                    loop {
                        match sender.send(event) {
                            Ok(()) => break,
                            Err(e) => {
                                event = e.into_inner();
                                if sender.is_closed() {
                                    return;
                                }
                                thread::sleep(Duration::from_millis(1));
                            }
                        }
                    }
                    if state != State::Running {
                        break;
                    }
                }
            });
            Ok(())
        })?;
        Ok(())
    });
    let (js, t, s) = (jobs.clone(), table.clone(), summary.clone());
    let content = Column::new([
        Label::new("Background jobs").key("title"),
        start.key("start"),
        Label::new(summary).key("summary"),
        Table::new(&table, 42.0)?
            .on_sort(move |_, _| publish(&js, &t, &s))
            .key("table")
            .with_grow(1),
        Label::new(notification).key("notification"),
    ])
    .align(Align::Stretch)
    .key("monitor");
    let root = ui.mount_tree(content)?;
    Ok(Monitor { root, jobs, table })
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--headless") {
        headless()?;
    } else {
        App::new("Background job monitor", |ui| {
            build(ui, Config::default())?;
            Ok(())
        })
        .run()?;
    }
    Ok(())
}

fn descendant(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("missing {key}"))
}
fn row_order(ui: &Runtime, node: &WidgetId) -> Vec<usize> {
    let children = ui.children(node).unwrap();
    let direct: Vec<_> = children
        .iter()
        .filter_map(|w| ui.key(w).unwrap().parse::<usize>().ok())
        .collect();
    if !direct.is_empty() {
        return direct;
    }
    children.iter().flat_map(|c| row_order(ui, c)).collect()
}
fn headless() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::host::HostIntegration;
    use pert_ui::testing::*;
    use pert_ui::{CommandKey, Modifiers};
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1100.0, 480.0, 1.0)?)?;
    let m = build(
        &mut ui,
        Config {
            interval: Duration::from_millis(20),
            deterministic: true,
        },
    )?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let start = d.find(&m.root, &["start"])?;
    for _ in 0..5 {
        d.runtime_mut().focus(&start)?;
        d.key(KeyCode::Space)?;
    }
    d.wait_until(Duration::from_secs(2), |_| {
        Ok(m.jobs.borrow().iter().all(|j| j.completed > 0))
    })?;
    let row2 = descendant(d.runtime(), &m.root, "2");
    let percent = descendant(d.runtime(), &row2, "percent");
    assert!(d.semantics(&percent)?.name.unwrap().contains('%'));
    let bar = descendant(d.runtime(), &row2, "bar");
    assert!(
        d.semantics(&bar)?.value.is_some(),
        "progress bar has a value"
    );
    let summary = d.find(&m.root, &["summary"])?;
    assert!(d.semantics(&summary)?.name.unwrap().contains("Running: 5"));
    let cancel = descendant(d.runtime(), &row2, "cancel");
    d.click(&cancel)?;
    d.wait_until(Duration::from_secs(1), |_| {
        Ok(m.jobs.borrow()[1].state == State::Cancelled)
    })?;
    let stopped = m.jobs.borrow()[1].completed;
    let row2 = descendant(d.runtime(), &m.root, "2");
    assert_eq!(
        d.semantics(&descendant(d.runtime(), &row2, "state-label"))?
            .name
            .as_deref(),
        Some("cancelled")
    );
    d.wait_until(Duration::from_secs(2), |_| {
        Ok(m.jobs.borrow()[0].state == State::Done
            && m.jobs.borrow()[2].state == State::Done
            && m.jobs.borrow()[4].state == State::Failed)
    })?;
    assert_eq!(m.jobs.borrow()[1].completed, stopped);
    let row5 = descendant(d.runtime(), &m.root, "5");
    assert!(
        d.semantics(&descendant(d.runtime(), &row5, "state-label"))?
            .name
            .unwrap()
            .contains("failed")
    );
    let notification = d.find(&m.root, &["notification"])?;
    assert!(
        d.semantics(&notification)?
            .name
            .unwrap()
            .contains("Job 5 failed")
    );
    assert_eq!(
        d.semantics(&summary)?.name.as_deref(),
        Some("Running: 1   Done: 2   Cancelled: 1   Failed: 1")
    );
    // Capture while all four states coexist, using the same runtime the checks drive.
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.wait()?.value?;
    std::fs::write("jobs.png", capture.image.encode_png())?;
    // Finish the last job before checking stable ordering in both directions.
    d.wait_until(Duration::from_secs(3), |_| {
        Ok(m.jobs.borrow().iter().all(|j| j.state != State::Running))
    })?;
    let table = descendant(d.runtime(), &m.root, "table");
    for column in ["name", "progress", "state", "elapsed"] {
        let row = descendant(d.runtime(), &table, "1");
        let label = if column == "progress" {
            "percent".to_owned()
        } else {
            format!("{column}-label")
        };
        let cell = descendant(d.runtime(), &row, &label);
        d.click(&cell)?;
        m.table.set_active_column(Some(column.to_owned()))?;
        for (key, direction) in [
            (KeyCode::ArrowUp, SortDirection::Ascending),
            (KeyCode::ArrowDown, SortDirection::Descending),
        ] {
            d.chord(
                KeyChord::new(
                    CommandKey::Named(key),
                    Modifiers {
                        alt: true,
                        ..Default::default()
                    },
                ),
                key,
            )?;
            assert_eq!(
                m.table.sort().get()?,
                Some(TableSort {
                    column: column.to_owned(),
                    direction
                })
            );
            let actual = row_order(d.runtime(), &table);
            let mut expected = match column {
                "name" => vec![1, 2, 3, 4, 5],
                "progress" => vec![2, 5, 1, 3, 4],
                "state" => vec![2, 1, 3, 4, 5],
                "elapsed" => {
                    let all = m.jobs.borrow();
                    let mut times: Vec<_> = all.iter().map(|j| (j.elapsed, j.id)).collect();
                    times.sort();
                    times.into_iter().map(|(_, id)| id).collect()
                }
                _ => unreachable!(),
            };
            if direction == SortDirection::Descending {
                expected.reverse();
            }
            assert_eq!(actual, expected, "mounted order for {column} {direction:?}");
        }
    }
    assert_eq!(
        d.semantics(&summary)?.name.as_deref(),
        Some("Running: 0   Done: 3   Cancelled: 1   Failed: 1")
    );
    d.runtime_mut().paint()?;
    assert_eq!(
        d.runtime().next_deadline(),
        None,
        "idle has no timer or worker wake pending"
    );
    d.advance(Duration::from_secs(10))?;
    assert!(!d.runtime().needs_paint(), "idle does not request repaint");
    assert_eq!(
        m.jobs.borrow()[1].completed,
        stopped,
        "cancelled job never resumed"
    );
    println!(
        "Headless checks passed: progress, completion, summary, cancellation, failure notification, all column sorts, idle. Capture: jobs.png"
    );
    d.runtime_mut().shutdown();
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn monitor_acceptance() {
        super::headless().unwrap();
    }
}
