//! A headless session over the monitor: a runtime with a viewport, a manual
//! job clock, simulated pointer input and CPU captures. The tests and the
//! `--headless` mode of the binary drive the application through it.

use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pert_ui::cpu::CpuRenderer;
use pert_ui::prelude::*;
use pert_ui::{PointerButton, PointerEvent, PointerKind};

use crate::monitor::{self, COLUMNS, Job, JobState, Monitor, row_key};
use crate::sim::{ManualClock, Planner};

pub type Fallible<T> = Result<T, Box<dyn Error>>;

/// Logical size of the headless window.
pub const WIDTH: f32 = 820.0;
pub const HEIGHT: f32 = 420.0;

pub struct Session {
    pub ui: Runtime,
    pub monitor: Monitor,
    pub clock: Arc<ManualClock>,
    now: Duration,
}

impl Session {
    pub fn new(planner: Box<dyn Planner>) -> Fallible<Session> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(WIDTH, HEIGHT, 1.0)?)?;
        let clock = Arc::new(ManualClock::default());
        let monitor = monitor::build(&mut ui, clock.clone(), planner)?;
        let mut session = Session { ui, monitor, clock, now: Duration::ZERO };
        session.flush()?;
        Ok(session)
    }

    /// Flushes at the current runtime time.
    pub fn flush(&mut self) -> Fallible<()> {
        self.ui.flush(self.now)?;
        Ok(())
    }

    /// Advances the runtime's clock (not the jobs') and flushes.
    pub fn advance_ui(&mut self, elapsed: Duration) -> Fallible<()> {
        self.now += elapsed;
        self.flush()
    }

    /// Flushes until `done` holds, waiting for worker reports to arrive.
    pub fn wait_until(&mut self, what: &str, done: impl Fn(&Monitor) -> bool) -> Fallible<()> {
        let limit = Instant::now() + Duration::from_secs(5);
        loop {
            self.flush()?;
            if done(&self.monitor) {
                return Ok(());
            }
            if Instant::now() > limit {
                return Err(format!("timed out waiting for {what}").into());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Lets the jobs run `ticks` ticks and waits until every running job has
    /// reported them, or ended.
    pub fn tick(&mut self, ticks: u32) -> Fallible<()> {
        let expected: Vec<(u32, u32)> = self
            .monitor
            .jobs()
            .iter()
            .filter(|job| job.state == JobState::Running)
            .map(|job| (job.id, job.ticks + ticks))
            .collect();
        self.clock.advance(u64::from(ticks));
        self.wait_until("the jobs to report their ticks", |monitor| {
            let jobs = monitor.jobs();
            expected.iter().all(|&(id, ticks)| {
                jobs.iter()
                    .find(|job| job.id == id)
                    .is_some_and(|job| job.state != JobState::Running || job.ticks >= ticks)
            })
        })
    }

    /// Presses and releases the primary button at a window position.
    pub fn click_at(&mut self, x: f32, y: f32) -> Fallible<()> {
        let at = Point::new(x, y)?;
        for kind in [
            PointerKind::Move,
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui.pointer_input(PointerEvent::new(at, kind))?;
        }
        self.flush()
    }

    /// The widget at a key path below the monitor's root.
    pub fn widget(&self, path: &[&str]) -> Fallible<WidgetId> {
        self.ui
            .find(&self.monitor.root, path)?
            .ok_or_else(|| format!("nothing mounted at {path:?}").into())
    }

    /// Clicks the middle of the widget at a key path.
    pub fn click(&mut self, path: &[&str]) -> Fallible<()> {
        let widget = self.widget(path)?;
        let bounds = self.ui.bounds(&widget)?.ok_or("not laid out")?;
        self.click_at(bounds.x() + bounds.width() / 2.0, bounds.y() + bounds.height() / 2.0)
    }

    /// Clicks Start job and returns the new job's id.
    pub fn start_job(&mut self) -> Fallible<u32> {
        let before = self.monitor.jobs().len();
        self.click(&["content", "toolbar", "start"])?;
        let jobs = self.monitor.jobs();
        if jobs.len() != before + 1 {
            return Err("Start job did not add a job".into());
        }
        Ok(jobs[before].id)
    }

    /// Clicks the Cancel button in a job's row.
    pub fn click_cancel(&mut self, id: u32) -> Fallible<()> {
        self.click(&["content", "jobs", &row_key(id), "action", "cancel"])
    }

    /// Clicks a column's title in the table header.
    pub fn click_header(&mut self, column: &str) -> Fallible<()> {
        let table = self.widget(&["content", "jobs"])?;
        let bounds = self.ui.bounds(&table)?.ok_or("table not laid out")?;
        let mut left = bounds.x();
        for &(key, _, width) in &COLUMNS {
            if key == column {
                return self.click_at(left + width / 2.0, bounds.y() + 8.0);
            }
            left += width;
        }
        Err(format!("no column {column}").into())
    }

    /// The text a component at a key path presents, from inspection.
    pub fn shown_text(&self, path: &[&str]) -> Fallible<String> {
        let widget = self.widget(path)?;
        let id = self.ui.inspection_id(&widget)?;
        let snapshot = self.ui.inspect();
        let component = snapshot
            .components
            .iter()
            .find(|component| component.id == id)
            .ok_or("not in the inspection snapshot")?;
        component
            .details
            .semantics
            .as_ref()
            .and_then(|semantics| semantics.name.clone())
            .ok_or_else(|| format!("{path:?} has no name").into())
    }

    /// The summary line as shown.
    pub fn summary(&self) -> Fallible<String> {
        self.shown_text(&["content", "toolbar", "summary"])
    }

    /// A cell's label text as shown, by job id and column.
    pub fn cell_text(&self, id: u32, column: &str) -> Fallible<String> {
        let row = row_key(id);
        match column {
            "progress" => self.shown_text(&["content", "jobs", &row, "progress", "progress", "percent"]),
            _ => self.shown_text(&["content", "jobs", &row, column, column]),
        }
    }

    /// The job ids in the order their rows appear on screen, top to bottom.
    pub fn row_order(&self) -> Fallible<Vec<u32>> {
        let mut rows = Vec::new();
        for job in self.monitor.jobs() {
            let widget = self.widget(&["content", "jobs", &row_key(job.id)])?;
            let bounds = self.ui.bounds(&widget)?.ok_or("row not laid out")?;
            rows.push((bounds.y(), job.id));
        }
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        Ok(rows.into_iter().map(|(_, id)| id).collect())
    }

    pub fn job(&self, id: u32) -> Fallible<Job> {
        self.monitor
            .jobs()
            .into_iter()
            .find(|job| job.id == id)
            .ok_or_else(|| format!("no job {id}").into())
    }

    /// Renders the window on the CPU and encodes it as a PNG.
    pub fn capture_png(&mut self) -> Fallible<Vec<u8>> {
        self.flush()?;
        let scene = self.ui.paint()?;
        let frame = CpuRenderer::new().render(&scene)?;
        Ok(frame.encode_png())
    }
}
