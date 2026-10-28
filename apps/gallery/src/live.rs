//! Synthetic static/live workload over the public live-data sources, using
//! only public consumer APIs. A producer thread publishes a repeating
//! synthetic signal on two bridges, a waveform and a level; the visuals
//! subscribe and the runtime drains. Peak-hold is the meter widget's own
//! policy over the items it has not folded yet, not something the bridge
//! does for it. No device, capture library or platform service is involved.
//!
//! The thread publishes only while a Run toggle of one of the two pages is on
//! and is parked otherwise, so a gallery whose pages are stopped wakes nothing.
use pert_ui::component::*;
use pert_ui::host::HostIntegration;
use pert_ui::live::*;
use pert_ui::prelude::*;
use pert_ui::windowing::*;
use pert_ui::{ProfileOperation, ProfileSnapshot};
use std::{
    cell::RefCell,
    io::Write,
    rc::Rc,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{JoinHandle, Thread},
    time::{Duration, Instant},
};

const CONTROL_COUNT: usize = 1_000;
/// Items the trace shows, and the items one published level covers.
const SAMPLE_COUNT: usize = 512;
/// Levels the meter retains; its widget folds the ones it has not seen.
const LEVEL_COUNT: usize = 8;
/// Items per second of the synthetic clock: one window per 60 Hz frame.
const RATE: f64 = SAMPLE_COUNT as f64 * 60.0;
/// The synthetic signal repeats every eight seconds.
const PERIOD_SECONDS: f64 = 8.0;

/// The page whose Run toggle a [`ProducerGate`] follows.
#[derive(Clone, Copy)]
enum Page {
    /// The Static & live page and its hand-written visuals.
    Visuals,
    /// The Level & trace page and the library's widgets.
    Widgets,
}

/// What the UI thread tells the producer thread. The UI thread writes the
/// flags and then unparks the thread, so a thread about to park sees the
/// write on its next check.
#[derive(Default)]
struct Control {
    visuals: AtomicBool,
    widgets: AtomicBool,
    stop: AtomicBool,
    /// The producer thread, once spawned; a gate before that only sets its flag.
    thread: OnceLock<Thread>,
}
impl Control {
    fn flag(&self, page: Page) -> &AtomicBool {
        match page {
            Page::Visuals => &self.visuals,
            Page::Widgets => &self.widgets,
        }
    }
    fn running(&self) -> bool {
        self.visuals.load(Ordering::Acquire) || self.widgets.load(Ordering::Acquire)
    }
    fn wake(&self) {
        if let Some(thread) = self.thread.get() {
            thread.unpark();
        }
    }
}

/// Follows one page's Run toggle and lets the producer thread run while it is
/// on. It wraps the toggle, so it lays out exactly as the toggle alone, and
/// dropping it with its page withdraws its claim.
///
/// Reading the flag in `update` subscribes to it. Storing it in an atomic and
/// unparking a thread writes no UI state and is idempotent, so a retried
/// evaluation repeats it harmlessly.
struct ProducerGate {
    running: ReadSignal<bool>,
    page: Page,
    control: Arc<Control>,
    /// Keeps the thread alive while the page is mounted, also for a host that
    /// does not hold the handle itself.
    _thread: ProducerHandle,
}
impl Component for ProducerGate {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let running = cx.read(&self.running)?;
        self.control
            .flag(self.page)
            .store(running, Ordering::Release);
        self.control.wake();
        Ok(())
    }
}
impl Drop for ProducerGate {
    fn drop(&mut self) {
        self.control.flag(self.page).store(false, Ordering::Release);
    }
}

/// Why the producer thread ended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ProducerEnd {
    /// [`ProducerThread::join`] asked it to.
    Stopped,
    /// The `run_for` limit of [`Producer::spawn`] passed.
    Elapsed,
    /// An endpoint reported this terminal outcome, `Closed` once the UI side
    /// has dropped its source.
    Endpoint(PublishOutcome),
}

/// The running producer thread. Dropping it without joining also stops and
/// joins it, discarding the result.
struct ProducerThread {
    control: Arc<Control>,
    handle: Option<JoinHandle<ProducerEnd>>,
}
impl ProducerThread {
    /// Stops the thread and waits for it.
    ///
    /// # Errors
    /// Reports a panic of the thread and an endpoint that could not advance
    /// ([`PublishOutcome::Exhausted`]). A closed endpoint is a normal end.
    fn join(mut self) -> Result<ProducerEnd, Box<dyn std::error::Error>> {
        let handle = self
            .handle
            .take()
            .ok_or("the producer thread was already joined")?;
        self.control.stop.store(true, Ordering::Release);
        self.control.wake();
        match handle.join() {
            Ok(ProducerEnd::Endpoint(PublishOutcome::Exhausted)) => {
                Err("a live endpoint could not advance and closed".into())
            }
            Ok(end) => Ok(end),
            Err(_) => Err("the live producer thread panicked".into()),
        }
    }
}
impl Drop for ProducerThread {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.control.stop.store(true, Ordering::Release);
            self.control.wake();
            // Nobody asked for the result; `join` is the path that reports it.
            let _ = handle.join();
        }
    }
}

/// Shared, UI-thread ownership of the producer thread: the workload and every
/// mounted gate hold it, and the host keeps a clone to join the thread after
/// its event loop returns. When the last clone drops, the thread is stopped
/// and joined without a report.
#[derive(Clone, Default)]
pub(super) struct ProducerHandle(Rc<RefCell<Option<ProducerThread>>>);
impl ProducerHandle {
    /// Stops and joins the thread, if one was spawned and not joined yet.
    ///
    /// # Errors
    /// As [`ProducerThread::join`].
    pub(super) fn join(&self) -> Result<Option<ProducerEnd>, Box<dyn std::error::Error>> {
        let thread = self.0.borrow_mut().take();
        thread.map(ProducerThread::join).transpose()
    }
}

/// The producer half: owns both endpoints and the synthetic clock.
///
/// `advance` publishes every item up to a synthetic time, so a test or the
/// benchmark drives it deterministically; `spawn` runs it on a thread for the
/// native gallery and smoke, advancing the clock only while a Run toggle is on.
pub(super) struct Producer {
    trace: SeriesEndpoint<f32>,
    level: SeriesEndpoint<f32>,
    published: u64,
    control: Arc<Control>,
    /// The first terminal outcome of either endpoint; nothing is published after it.
    ended: Option<PublishOutcome>,
}
impl Producer {
    /// Publishes the items between the previous call and `now`. Returns the
    /// terminal outcome once either endpoint reports one, `Closed` when the UI
    /// side has dropped a source, so a thread can stop; later calls publish
    /// nothing and return it again.
    pub(super) fn advance(&mut self, now: Duration) -> Option<PublishOutcome> {
        // Truncation is intended: the item index is the floor of elapsed items.
        let target = (now.as_secs_f64() * RATE) as u64;
        while self.ended.is_none() && self.published < target {
            let index = self.published;
            let (level, sample) = synthetic(index);
            self.ended = terminal(self.trace.publish(sample, Timestamp::new(index)));
            if self.ended.is_none() && index.is_multiple_of(SAMPLE_COUNT as u64) {
                self.ended = terminal(self.level.publish(level, Timestamp::new(index)));
            }
            self.published += 1;
        }
        self.ended
    }
    /// Runs the producer on its own thread. The synthetic clock advances only
    /// while a page's Run toggle is on; otherwise the thread is parked. It ends
    /// when [`ProducerThread::join`] stops it, when an endpoint reports a
    /// terminal outcome, or once `run_for` of wall-clock time has passed, and
    /// then drops the endpoints, which the sources observe.
    fn spawn(mut self, run_for: Option<Duration>) -> Result<ProducerThread, std::io::Error> {
        let control = self.control.clone();
        let handle = std::thread::Builder::new()
            .name("gallery-live-producer".into())
            .spawn(move || {
                let start = Instant::now();
                // Synthetic time published during earlier running intervals.
                let mut clock = Duration::ZERO;
                let mut resumed: Option<Instant> = None;
                loop {
                    if self.control.stop.load(Ordering::Acquire) {
                        return ProducerEnd::Stopped;
                    }
                    let left = run_for.map(|limit| limit.saturating_sub(start.elapsed()));
                    if left == Some(Duration::ZERO) {
                        return ProducerEnd::Elapsed;
                    }
                    if !self.control.running() {
                        if let Some(since) = resumed.take() {
                            clock += since.elapsed();
                        }
                        match left {
                            Some(left) => std::thread::park_timeout(left),
                            None => std::thread::park(),
                        }
                        continue;
                    }
                    let since = *resumed.get_or_insert_with(Instant::now);
                    if let Some(outcome) = self.advance(clock + since.elapsed()) {
                        return ProducerEnd::Endpoint(outcome);
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            })?;
        // Only this call sets the thread, once. A gate that ran earlier only
        // set its flag, which the thread reads before it first parks.
        let _ = control.thread.set(handle.thread().clone());
        Ok(ProducerThread {
            control,
            handle: Some(handle),
        })
    }
}

/// The outcomes after which an endpoint accepts nothing more.
fn terminal(outcome: PublishOutcome) -> Option<PublishOutcome> {
    matches!(outcome, PublishOutcome::Closed | PublishOutcome::Exhausted).then_some(outcome)
}

/// The level and the sample at item `index` of the repeating signal.
fn synthetic(index: u64) -> (f32, f32) {
    let seconds = (index as f64 / RATE) % PERIOD_SECONDS;
    let phase = (seconds * std::f64::consts::TAU / PERIOD_SECONDS) as f32;
    let level = 0.5 + 0.4 * phase.sin();
    let x = (index % SAMPLE_COUNT as u64) as f32 / (SAMPLE_COUNT - 1) as f32;
    (
        level,
        level * (x * 4.0 * std::f32::consts::TAU + phase).sin(),
    )
}

pub(super) struct Workload {
    running: Signal<bool>,
    shown: Signal<bool>,
    controls: Vec<Signal<bool>>,
    trace: SeriesSource<f32>,
    level: SeriesSource<f32>,
    /// Run flag of the library-widget page, separate so each page idles alone.
    widgets_running: Signal<bool>,
    /// Shared with the producer, which runs while either Run flag is on.
    control: Arc<Control>,
    /// Where [`Self::spawn`] keeps the producer thread.
    thread: ProducerHandle,
}
const LIVE_HELP: &str = "1,000 retained controls beside a waveform and level from a producer thread. Start, scroll, toggle controls, or switch tabs; returning discards what arrived unseen and continues from current data.";
impl Workload {
    /// Builds the UI side and returns the producer for the caller to drive.
    pub(super) fn new(ui: &Runtime, running: bool) -> Result<(Self, Producer), LiveError> {
        Self::with_thread(ui, running, ProducerHandle::default())
    }

    /// As [`Self::new`], keeping a spawned producer thread in `thread`, which
    /// the host holds to join it.
    pub(super) fn with_thread(
        ui: &Runtime,
        running: bool,
        thread: ProducerHandle,
    ) -> Result<(Self, Producer), LiveError> {
        let (trace_endpoint, trace_consumer) =
            series_bridge::<f32>(SeriesConfig::new(Capacity::new(SAMPLE_COUNT)?));
        // One level per waveform block, so a handful of items covers every
        // frame the UI can present; the meter folds them when it reads.
        let (level_endpoint, level_consumer) =
            series_bridge::<f32>(SeriesConfig::new(Capacity::new(LEVEL_COUNT)?));
        let workload = Self {
            running: ui.signal(running),
            shown: ui.signal(true),
            controls: (0..CONTROL_COUNT).map(|_| ui.signal(false)).collect(),
            trace: ui.series_source(trace_consumer, Delivery::Polled),
            level: ui.series_source(level_consumer, Delivery::Polled),
            widgets_running: ui.signal(false),
            control: Arc::default(),
            thread,
        };
        let producer = Producer {
            trace: trace_endpoint,
            level: level_endpoint,
            published: 0,
            control: workload.control.clone(),
            ended: None,
        };
        Ok((workload, producer))
    }

    /// Runs `producer` on its own thread (see [`Producer::spawn`]) and keeps
    /// the thread where the host and the mounted pages hold it.
    pub(super) fn spawn(
        &self,
        producer: Producer,
        run_for: Option<Duration>,
    ) -> Result<(), std::io::Error> {
        let thread = producer.spawn(run_for)?;
        // A thread spawned earlier is stopped and joined when replaced.
        *self.thread.0.borrow_mut() = Some(thread);
        Ok(())
    }

    /// Wraps a page's Run toggle in the gate that lets the producer run.
    fn gate(&self, page: Page, toggle: Element) -> Element {
        let (key, running) = match page {
            Page::Visuals => ("run-live-gate", self.running.read_only()),
            Page::Widgets => ("widgets-run-gate", self.widgets_running.read_only()),
        };
        Element::new(
            key,
            ProducerGate {
                running,
                page,
                control: self.control.clone(),
                _thread: self.thread.clone(),
            },
        )
        .children([toggle])
    }

    /// The same two sources presented by the library's own `Trace` and
    /// `LevelMeter`, which need no consumer component at all.
    pub(super) fn widgets_element(&self) -> Result<Element, RuntimeError> {
        let running = self.widgets_running.read_only();
        let signal = ParameterSpec::new("Synthetic waveform", -1.0, 1.0, 0.0)?;
        let level = ParameterSpec::new("Synthetic level", 0.0, 1.0, 0.0)?;
        Ok(Element::new(
            "live-widgets-page",
            ScrollArea::new(Element::new(
                "live-widgets",
                Column::new([
                    Element::new(
                        "widgets-help",
                        Label::new(
                            "The library's Trace and LevelMeter on the producer thread's two series. Each takes a source and a ParameterSpec as its scale; no unit is assumed. The horizontal meter holds its peak for a second; the vertical one reads the same source. Stopped, neither reads, so nothing is drained.",
                        ),
                    ),
                    self.gate(
                        Page::Widgets,
                        Element::new(
                            "widgets-run",
                            Toggle::new(self.widgets_running.clone(), "Run"),
                        ),
                    ),
                    Element::new(
                        "widgets-row",
                        Row::new([
                            Element::new("widgets-scope", PlotBackground).children([Element::new(
                                "widgets-trace",
                                Trace::new(self.trace.clone(), signal)
                                    .running_signal(running.clone()),
                            )]),
                            Element::new(
                                "widgets-meter-vertical",
                                LevelMeter::new(self.level.clone(), level.clone())
                                    .axis(MeterAxis::Vertical)
                                    .color(ColorRole::Secondary)
                                    .running_signal(running.clone()),
                            ),
                        ]),
                    ),
                    Element::new(
                        "widgets-meter",
                        LevelMeter::new(self.level.clone(), level)
                            .peak_hold(Duration::from_secs(1))
                            .running_signal(running),
                    ),
                ]),
            )),
        ))
    }

    pub(super) fn element(&self) -> Result<Element, RuntimeError> {
        let plots = Element::new(
            "live-visuals",
            Column::new([
                Element::new("scope-title", Label::new("Synthetic waveform")),
                Element::new("scope-background", PlotBackground).children([Element::new(
                    "live-trace",
                    LiveVisual::new(
                        self.running.read_only(),
                        VisualKind::Trace(self.trace.clone()),
                    ),
                )]),
                Element::new("meter-title", Label::new("Synthetic level")),
                Element::new(
                    "live-meter",
                    LiveVisual::new(
                        self.running.read_only(),
                        VisualKind::Meter(self.level.clone()),
                    ),
                ),
            ]),
        );
        let controls = self.controls.iter().enumerate().map(|(index, value)| {
            Element::new(
                format!("static-{index:04}"),
                Checkbox::new(value.clone(), format!("Control {:04}", index + 1)),
            )
        });
        Ok(Element::new(
            "static-live-page",
            RegionLayout::new()
                .fixed()
                .top(Element::new(
                    "live-header",
                    Group::new(Element::new(
                        "live-intro",
                        Column::new([
                            Element::new("live-help", Label::new(LIVE_HELP)),
                            Element::new(
                                "live-actions",
                                Wrap::new([
                                    self.gate(
                                        Page::Visuals,
                                        Element::new(
                                            "run-live",
                                            Toggle::new(self.running.clone(), "Run"),
                                        ),
                                    ),
                                    Element::new(
                                        "show-live",
                                        Checkbox::new(self.shown.clone(), "Show visuals"),
                                    ),
                                    Element::new(
                                        "clear-live-profile",
                                        Button::new("Reset measurements", |cx| {
                                            cx.set_profiling(true)?;
                                            cx.clear_profile()
                                        }),
                                    ),
                                ]),
                            ),
                        ]),
                    )),
                ))
                .center(Element::new(
                    "live-split",
                    SplitPane::new(
                        SplitAxis::TopBottom,
                        Element::new(
                            "live-view",
                            View::new(plots).visible_signal(self.shown.read_only()),
                        ),
                        Element::new(
                            "static-scroll",
                            ScrollArea::new(Element::new("static-controls", Wrap::new(controls))),
                        ),
                    ),
                )),
        ))
    }
}

// The default child arrangement overlays the trace on a separately retained
// background. Trace drains cannot invalidate these axes or their text siblings.
struct PlotBackground;
impl Component for PlotBackground {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([480, 120]))
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        painter.fill_rect(Rect::from_size(size), painter.color(ColorRole::Surface));
        for line in 1..4 {
            painter.fill_rect(
                Rect::new(0.0, size.height() * line as f32 / 4.0, size.width(), 1.0)?,
                painter.color(ColorRole::Outline),
            );
        }
        for line in 1..8 {
            painter.fill_rect(
                Rect::new(size.width() * line as f32 / 8.0, 0.0, 1.0, size.height())?,
                painter.color(ColorRole::Outline),
            );
        }
        Ok(())
    }
}

enum VisualKind {
    Trace(SeriesSource<f32>),
    Meter(SeriesSource<f32>),
}
struct LiveVisual {
    running: ReadSignal<bool>,
    kind: VisualKind,
    updates: u64,
    latest_seconds: f64,
    freshness: Option<Freshness>,
    /// Retained copy of the window: the value and whether a break precedes it.
    samples: Vec<(f32, bool)>,
    level: f32,
    /// The newest level sequence this meter has already folded.
    folded_through: Option<u64>,
}
impl LiveVisual {
    fn new(running: ReadSignal<bool>, kind: VisualKind) -> Self {
        Self {
            running,
            kind,
            updates: 0,
            latest_seconds: 0.0,
            freshness: None,
            samples: Vec::with_capacity(SAMPLE_COUNT),
            level: 0.0,
            folded_through: None,
        }
    }
}
impl Component for LiveVisual {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Not running: no read, so no subscription, so the source is not drained.
        if !cx.read_paint(&self.running)? {
            return Ok(());
        }
        // An inspection count only: the maximum keeps the maximum, as in `counter_of`.
        self.updates = self.updates.saturating_add(1);
        match &self.kind {
            VisualKind::Trace(source) => {
                let samples = &mut self.samples;
                let (seconds, freshness) = cx.read_series(source, |telemetry, window| {
                    samples.clear();
                    samples.extend(
                        window
                            .iter()
                            .map(|entry| (entry.sample.value, entry.discontinuity_before)),
                    );
                    (
                        telemetry
                            .latest
                            .map_or(0.0, |sample| sample.timestamp.get() as f64 / RATE),
                        telemetry.freshness,
                    )
                })?;
                self.latest_seconds = seconds;
                self.freshness = Some(freshness);
            }
            VisualKind::Meter(source) => {
                // Peak-hold is this widget's policy, not the bridge's: fold the
                // items that arrived since the previous read, and hold the
                // previous level when nothing did.
                let folded_through = self.folded_through;
                let (peak, seconds, freshness) = cx.read_series(source, |telemetry, window| {
                    let mut peak: Option<f32> = None;
                    let mut newest = None;
                    for entry in window.iter() {
                        let sequence = entry.sample.sequence.get();
                        if folded_through.is_some_and(|folded| sequence <= folded) {
                            continue;
                        }
                        let value = entry.sample.value;
                        peak = Some(peak.map_or(value, |high: f32| high.max(value)));
                        newest = Some(sequence);
                    }
                    (
                        peak.zip(newest),
                        telemetry
                            .latest
                            .map_or(0.0, |sample| sample.timestamp.get() as f64 / RATE),
                        telemetry.freshness,
                    )
                })?;
                if let Some((level, newest)) = peak {
                    self.level = level;
                    self.folded_through = Some(newest);
                }
                self.latest_seconds = seconds;
                self.freshness = Some(freshness);
            }
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(match self.kind {
            VisualKind::Trace(_) => Size::from([480, 120]),
            VisualKind::Meter(_) => Size::from([480, 24]),
        })
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        if size.width() == 0.0 || size.height() == 0.0 {
            return Ok(());
        }
        match self.kind {
            VisualKind::Meter(_) => {
                painter.fill_rect(
                    Rect::from_size(size),
                    painter.color(ColorRole::SurfaceHover),
                );
                painter.fill_rect(
                    Rect::new(0.0, 0.0, size.width() * self.level, size.height())?,
                    painter.color(ColorRole::Secondary),
                );
            }
            VisualKind::Trace(_) => {
                // At most 512 columns and 1,023 sample reads per paint. Each
                // column preserves min/max and includes its preceding sample
                // to connect adjacent columns, except across a reported break.
                let count = self.samples.len();
                if count == 0 {
                    return Ok(());
                }
                let columns = ((f64::from(size.width()) * painter.viewport().scale()).ceil()
                    as usize)
                    .clamp(1, count);
                let width = size.width() / columns as f32;
                for column in 0..columns {
                    let first = column * count / columns;
                    let end = (column + 1) * count / columns;
                    let start = if self.samples[first].1 {
                        first
                    } else {
                        first.saturating_sub(1)
                    };
                    let (low, high) = self.samples[start..end]
                        .iter()
                        .fold((1.0_f32, -1.0_f32), |(low, high), &(sample, _)| {
                            (low.min(sample), high.max(sample))
                        });
                    if low > high {
                        continue;
                    }
                    let top = (1.0 - high) * size.height() * 0.5;
                    let bottom = (1.0 - low) * size.height() * 0.5;
                    painter.fill_rect(
                        Rect::new(
                            column as f32 * width,
                            top,
                            width,
                            (bottom - top).max(1.0).min(size.height() - top),
                        )?,
                        painter.color(ColorRole::Primary),
                    );
                }
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("updates".into(), InspectValue::Number(self.updates as f64));
        details.properties.insert(
            "latest_seconds".into(),
            InspectValue::Number(self.latest_seconds),
        );
        details
            .properties
            .insert("level".into(), InspectValue::Number(f64::from(self.level)));
        details.properties.insert(
            "freshness".into(),
            InspectValue::Text(format!("{:?}", self.freshness)),
        );
        details
    }
}

pub(super) fn profile_report(profile: &ProfileSnapshot) -> String {
    let mut report = String::from("Operation            Calls    Total ms     Self ms\n");
    for operation in [
        ProfileOperation::Update,
        ProfileOperation::Reconcile,
        ProfileOperation::Layout,
        ProfileOperation::Measure,
        ProfileOperation::Arrange,
        ProfileOperation::Text,
        ProfileOperation::Paint,
        ProfileOperation::Compose,
        ProfileOperation::Flush,
    ] {
        let mut calls = 0;
        let mut total = Duration::ZERO;
        let mut own = Duration::ZERO;
        for row in profile
            .stats
            .iter()
            .filter(|row| row.operation == operation)
        {
            calls += row.calls;
            total += row.total;
            own += row.self_time;
        }
        report.push_str(&format!(
            "{:<20} {calls:>5} {:>11.3} {:>11.3}\n",
            format!("{operation:?}"),
            total.as_secs_f64() * 1000.0,
            own.as_secs_f64() * 1000.0
        ));
    }
    report.push_str(&format!(
        "Omitted scopes: {}; evicted recent samples: {}\n",
        profile.dropped_scopes, profile.dropped_samples
    ));
    report
}

/// Flushes every deadline due at `now`, then paints; the headless equivalent of
/// one host turn.
fn settle(ui: &mut Runtime, now: Duration) -> Result<usize, Box<dyn std::error::Error>> {
    let mut updated = 0;
    loop {
        updated += ui.flush(now)?.components_updated;
        if !ui.next_deadline().is_some_and(|deadline| deadline <= now) {
            break;
        }
    }
    ui.paint()?;
    Ok(updated)
}

pub(super) fn benchmark() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let (workload, mut producer) = Workload::new(&ui, false)?;
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
    let root = ui.mount_tree(workload.element()?)?;
    settle(&mut ui, Duration::ZERO)?;
    workload.running.set(true)?;
    settle(&mut ui, Duration::ZERO)?;
    ui.set_profiling(true)?;
    ui.clear_profile()?;
    let mut scene = None;
    for _ in 0..300 {
        let deadline = ui.next_deadline().ok_or("live workload stopped early")?;
        // The producer is driven from here, one frame of items per deadline,
        // so the counts below are assertions rather than timing observations.
        producer.advance(deadline);
        ui.flush(deadline)?;
        scene = Some(ui.paint()?);
    }
    let snapshot = ui.profile();
    let scene = scene.ok_or("no frame was composed")?;
    // Composition drops what a clip removes, so a scrolled-out control costs no
    // command and no coverage entry. A command with no pixels would mean that
    // the scene had grown with the mounted count again.
    if scene.commands().iter().any(|command| match command {
        DrawCommand::FillRect { rect, .. } | DrawCommand::DrawMask { rect, .. } => rect.is_empty(),
        _ => false,
    }) {
        return Err("composition recorded a command that draws nothing".into());
    }
    let on_screen = ui
        .inspect()
        .components
        .iter()
        .filter(|item| {
            item.key.starts_with("static-")
                && item.key.len() == 11
                && item.visible
                && item
                    .details
                    .validation
                    .clip
                    .is_some_and(|[_, _, w, h]| w > 0.0 && h > 0.0)
        })
        .count();
    if snapshot.dropped_scopes != 0
        || snapshot.stats.iter().any(|row| {
            matches!(
                row.operation,
                ProfileOperation::Reconcile
                    | ProfileOperation::Layout
                    | ProfileOperation::Measure
                    | ProfileOperation::Arrange
                    | ProfileOperation::Text
            ) || (matches!(
                row.operation,
                ProfileOperation::Update | ProfileOperation::Paint
            ) && !matches!(row.key.as_str(), "live-trace" | "live-meter"))
        })
    {
        return Err("static/live isolation check failed".into());
    }
    let sources = ui.inspect().sources;
    writeln!(
        std::io::stdout().lock(),
        "Headless synthetic workload: {CONTROL_COUNT} mounted checkboxes, {on_screen} intersect the viewport; 300 scheduled frames.\nComposed scene: {} commands and {} coverage masks.\nSources: {} items dropped, {} rejected across {} bridges.\n{}Inclusive durations overlap; Paint measures CPU drawing/geometry recording, not GPU execution.",
        scene.len(),
        scene.masks().len(),
        sources
            .iter()
            .map(|source| source.dropped_total)
            .sum::<u64>(),
        sources
            .iter()
            .map(|source| source.rejected_total)
            .sum::<u64>(),
        sources.len(),
        profile_report(&snapshot)
    )?;
    workload.running.set(false)?;
    let stopped_at = ui.now();
    settle(&mut ui, stopped_at)?;
    if ui.next_deadline().is_some() || ui.needs_paint() {
        return Err("stopped workload did not settle".into());
    }
    ui.clear_profile()?;
    let started = std::time::Instant::now();
    std::thread::sleep(Duration::from_secs(30));
    if !ui.profile().stats.is_empty() || ui.next_deadline().is_some() || ui.needs_paint() {
        return Err("idle workload changed".into());
    }
    writeln!(
        std::io::stdout().lock(),
        "Headless idle: {:.3} real seconds; no runtime calls, deadlines or damage. Native event-loop/GPU evidence is separate.",
        started.elapsed().as_secs_f64()
    )?;
    ui.unmount(&root)?;
    drop(producer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const INTERVAL: Duration = Duration::from_nanos(16_666_667);

    fn mounted() -> (Runtime, Workload, Producer, WidgetId) {
        let mut ui = Runtime::new();
        let (workload, producer) = Workload::new(&ui, false).unwrap();
        ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
            .unwrap();
        let root = ui.mount_tree(workload.element().unwrap()).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        (ui, workload, producer, root)
    }
    fn measure(ui: &Runtime) {
        ui.set_profiling(true).unwrap();
        ui.clear_profile().unwrap();
    }
    fn calls(ui: &Runtime, operation: ProfileOperation) -> u64 {
        ui.profile()
            .stats
            .iter()
            .filter(|row| row.operation == operation)
            .map(|row| row.calls)
            .sum()
    }
    fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
        ui.inspect()
            .components
            .iter()
            .find(|item| item.key == key)
            .unwrap()
            .details
            .properties[name]
            .clone()
    }
    fn check_static(ui: &Runtime) {
        let profile = ui.profile();
        assert_eq!(profile.dropped_scopes, 0);
        for row in &profile.stats {
            assert!(
                !row.key.starts_with("static-"),
                "unexpected static work: {row:?}"
            );
            assert!(
                !matches!(
                    row.operation,
                    ProfileOperation::Reconcile
                        | ProfileOperation::Layout
                        | ProfileOperation::Measure
                        | ProfileOperation::Arrange
                        | ProfileOperation::Text
                ),
                "unexpected layout/text work: {row:?}"
            );
            if matches!(
                row.operation,
                ProfileOperation::Update | ProfileOperation::Paint
            ) {
                assert!(
                    matches!(row.key.as_str(), "live-trace" | "live-meter"),
                    "unexpected callback: {row:?}"
                );
            }
        }
    }
    #[test]
    fn three_hundred_live_frames_reuse_static_controls_axes_layout_and_text() {
        let (mut ui, workload, mut producer, _) = mounted();
        let controls = ui
            .inspect()
            .components
            .into_iter()
            .filter(|item| item.key.starts_with("static-") && item.key.len() == 11)
            .count();
        assert_eq!(controls, CONTROL_COUNT);
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        measure(&ui);
        for frame in 1..=300u32 {
            let deadline = ui.next_deadline().unwrap();
            assert_eq!(deadline, INTERVAL * frame);
            producer.advance(deadline);
            let report = ui.flush(deadline).unwrap();
            assert_eq!(report.components_updated, 2, "frame {frame}");
            ui.paint().unwrap();
        }
        check_static(&ui);
        assert_eq!(calls(&ui, ProfileOperation::Update), 600);
        assert_eq!(calls(&ui, ProfileOperation::Paint), 600);
        assert_eq!(calls(&ui, ProfileOperation::Compose), 300);
        let sources = ui.inspect().sources;
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().all(|source| source.dropped_total == 0));
        assert_eq!(
            property(&ui, "live-trace", "freshness"),
            InspectValue::Text("Some(Fresh)".into())
        );
    }
    #[test]
    fn a_frame_without_new_items_updates_and_paints_nothing() {
        let (mut ui, workload, mut producer, _) = mounted();
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        producer.advance(INTERVAL);
        settle(&mut ui, INTERVAL).unwrap();
        measure(&ui);
        let deadline = ui.next_deadline().unwrap();
        assert_eq!(ui.flush(deadline).unwrap().components_updated, 0);
        assert!(!ui.needs_paint());
        assert_eq!(calls(&ui, ProfileOperation::Update), 0);
        assert_eq!(ui.next_deadline(), Some(deadline + INTERVAL));
    }
    #[test]
    fn changing_one_checkbox_during_live_updates_only_its_binding() {
        let (mut ui, workload, _producer, _) = mounted();
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        measure(&ui);
        workload.controls[0].set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        let updated: Vec<_> = ui
            .profile()
            .stats
            .into_iter()
            .filter(|row| row.operation == ProfileOperation::Update)
            .map(|row| row.key)
            .collect();
        assert_eq!(updated, ["static-0000"]);
        assert_eq!(calls(&ui, ProfileOperation::Layout), 0);
        assert_eq!(calls(&ui, ProfileOperation::Text), 0);
        assert_eq!(calls(&ui, ProfileOperation::Paint), 1);
        assert_eq!(
            property(&ui, "static-0000", "checked"),
            InspectValue::Bool(true)
        );
        assert!(ui.next_deadline().is_some());
    }
    #[test]
    fn hiding_discards_the_unseen_backlog_and_unmount_releases_subscriptions() {
        let (mut ui, workload, mut producer, root) = mounted();
        workload.running.set(true).unwrap();
        producer.advance(INTERVAL);
        settle(&mut ui, INTERVAL).unwrap();
        let before = property(&ui, "live-trace", "updates");
        workload.shown.set(false).unwrap();
        settle(&mut ui, INTERVAL).unwrap();
        assert_eq!(ui.next_deadline(), None);
        measure(&ui);
        // Hidden: the producer runs on, the ring fills, nothing is drained.
        producer.advance(Duration::from_secs(30));
        assert_eq!(
            ui.flush(Duration::from_secs(30))
                .unwrap()
                .components_updated,
            0
        );
        assert!(!ui.needs_paint());
        assert_eq!(property(&ui, "live-trace", "updates"), before);
        assert_eq!(calls(&ui, ProfileOperation::Update), 0);
        // Shown: the backlog is discarded, then current items arrive.
        workload.shown.set(true).unwrap();
        settle(&mut ui, Duration::from_secs(30)).unwrap();
        producer.advance(Duration::from_secs(30) + INTERVAL);
        settle(&mut ui, Duration::from_secs(30) + INTERVAL).unwrap();
        let InspectValue::Number(seconds) = property(&ui, "live-trace", "latest_seconds") else {
            panic!("latest_seconds");
        };
        assert!((seconds - 30.0).abs() < 0.05, "{seconds}");
        // Both rings overflowed while nothing watched them, which is what a
        // suspended source is: the producer kept publishing into a full ring.
        assert!(
            ui.inspect()
                .sources
                .iter()
                .all(|source| source.dropped_total > 0)
        );
        assert!(ui.next_deadline().unwrap() > Duration::from_secs(30));
        ui.unmount(&root).unwrap();
        workload.running.set(false).unwrap();
        workload.controls[0].set(true).unwrap();
        assert_eq!(
            ui.flush(Duration::from_secs(60))
                .unwrap()
                .components_updated,
            0
        );
        assert_eq!(ui.next_deadline(), None);
        assert!(ui.inspect().components.is_empty());
    }
    /// Peak-hold moved out of the library when the scalar bridge was removed, so
    /// the meter's own fold needs its own coverage. Each assertion here was
    /// checked against a mutation of the fold: folding the whole retained window
    /// again on every read, keeping the newest level instead of the largest
    /// unfolded one, and assigning a value when a drain delivered nothing.
    #[test]
    fn the_meter_folds_every_unfolded_level_and_holds_when_nothing_arrives() {
        let (mut ui, workload, mut producer, _root) = mounted();
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();

        let level_of = |level: u64| f64::from(synthetic(level * SAMPLE_COUNT as u64).0);
        let newest = |producer: &Producer| (producer.published - 1) / SAMPLE_COUNT as u64;
        let read = |ui: &Runtime| match property(ui, "live-meter", "level") {
            InspectValue::Number(level) => level,
            other => panic!("level: {other:?}"),
        };

        // Step past the signal's maximum one frame at a time, so the ring never
        // overflows and the window ends up holding consecutive falling levels
        // that the meter has already folded. A ring that overflowed would keep
        // its oldest items instead, which is a different and useless fixture.
        let mut now = Duration::ZERO;
        while now < Duration::from_millis(2_500) {
            now += INTERVAL;
            producer.advance(now);
            settle(&mut ui, now).unwrap();
        }
        let filled = newest(&producer);
        assert!(
            level_of(filled + 1 - LEVEL_COUNT as u64) > level_of(filled),
            "the retained window must be falling for this fixture to mean anything"
        );

        // One new level, smaller than every level already in the window: the
        // meter takes exactly it. Folding the window again would read higher.
        now += INTERVAL;
        producer.advance(now);
        let after = newest(&producer);
        assert_eq!(after, filled + 1, "one level per frame");
        settle(&mut ui, now).unwrap();
        let level = read(&ui);
        assert!((level - level_of(after)).abs() < 1e-6, "{level}");
        assert!(
            level_of(after + 1 - LEVEL_COUNT as u64) > level + 1e-6,
            "the window still holds larger levels this fold must not take again"
        );

        // A drain that finds nothing evaluates nobody and changes nothing.
        now += INTERVAL;
        assert_eq!(ui.flush(now).unwrap().components_updated, 0);
        assert!((read(&ui) - level).abs() < f64::EPSILON);

        // Several levels in one drain: every one of them is folded, so while the
        // signal falls the oldest of the batch wins. Keeping only the newest,
        // which is all a latest-value source would give, would read lower.
        let before = newest(&producer);
        for _ in 0..3 {
            now += INTERVAL;
        }
        producer.advance(now);
        let after = newest(&producer);
        assert!(after - before >= 2, "a batch of levels: {before}..={after}");
        settle(&mut ui, now).unwrap();
        let folded = read(&ui);
        let expected = (before + 1..=after).map(level_of).fold(f64::MIN, f64::max);
        assert!((folded - expected).abs() < 1e-6, "{folded} vs {expected}");
        assert!(
            folded > level_of(after) + 1e-6,
            "the newest level alone would be lower than the fold"
        );

        // Stopping and starting again evaluates the meter with nothing to fold,
        // because the subscription lapsed and resuming cleared the window. It
        // must hold its last value rather than fall back to an invented one.
        workload.running.set(false).unwrap();
        for _ in 0..4 {
            now += INTERVAL;
            ui.flush(now).unwrap();
        }
        workload.running.set(true).unwrap();
        for _ in 0..4 {
            now += INTERVAL;
            ui.flush(now).unwrap();
        }
        assert!(
            (read(&ui) - folded).abs() < f64::EPSILON,
            "the meter holds across a stop and start that delivered nothing"
        );
    }

    #[test]
    fn late_deadlines_skip_frames_and_stopping_has_no_thirty_second_work() {
        let (mut ui, workload, mut producer, _) = mounted();
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        measure(&ui);
        producer.advance(Duration::from_secs(30));
        assert_eq!(
            ui.flush(Duration::from_secs(30))
                .unwrap()
                .components_updated,
            2
        );
        ui.paint().unwrap();
        // One late drain delivers the ring's tail, not thirty seconds of frames.
        assert_eq!(calls(&ui, ProfileOperation::Update), 2);
        assert_eq!(ui.next_deadline(), Some(Duration::from_secs(30) + INTERVAL));
        check_static(&ui);
        workload.running.set(false).unwrap();
        settle(&mut ui, Duration::from_secs(30)).unwrap();
        measure(&ui);
        assert_eq!(ui.next_deadline(), None);
        producer.advance(Duration::from_secs(60));
        assert_eq!(
            ui.flush(Duration::from_secs(60))
                .unwrap()
                .components_updated,
            0
        );
        assert!(!ui.needs_paint());
        assert_eq!(calls(&ui, ProfileOperation::Update), 0);
        assert_eq!(calls(&ui, ProfileOperation::Compose), 0);
    }
    #[test]
    fn scrolling_dense_controls_preserves_live_geometry() {
        let (mut ui, workload, _producer, root) = mounted();
        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        let scroll = ui.find_descendant(&root, "static-scroll").unwrap().unwrap();
        let trace = ui.find_descendant(&root, "live-trace").unwrap().unwrap();
        let before = ui.bounds(&trace).unwrap();
        ui.focus(&scroll).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        measure(&ui);
        for _ in 0..3 {
            ui.key_input(KeyCode::PageDown, KeyAction::Press).unwrap();
            ui.key_input(KeyCode::PageDown, KeyAction::Release).unwrap();
            settle(&mut ui, Duration::ZERO).unwrap();
        }
        assert_eq!(calls(&ui, ProfileOperation::Text), 0);
        assert!(
            ui.profile()
                .stats
                .iter()
                .filter(|row| row.operation == ProfileOperation::Arrange)
                .all(|row| !row.key.starts_with("static-0"))
        );
        assert_eq!(ui.bounds(&trace).unwrap(), before);
        assert_ne!(
            property(&ui, "static-scroll", "scroll_y"),
            InspectValue::Number(0.0)
        );
        assert!(ui.next_deadline().is_some());
    }
    #[test]
    fn a_producer_that_stops_leaves_no_deadline_behind() {
        let mut ui = Runtime::new();
        let (workload, mut producer) = Workload::new(&ui, true).unwrap();
        ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
            .unwrap();
        ui.mount_tree(workload.element().unwrap()).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        producer.advance(Duration::from_secs(4));
        settle(&mut ui, Duration::from_secs(4)).unwrap();
        assert!(ui.next_deadline().is_some());
        // The smoke's producer thread drops its endpoints after five seconds;
        // the sources observe the disconnection once and stop scheduling.
        drop(producer);
        settle(&mut ui, Duration::from_secs(5)).unwrap();
        assert_eq!(
            property(&ui, "live-meter", "freshness"),
            InspectValue::Text("Some(Unavailable)".into())
        );
        assert_eq!(ui.next_deadline(), None);
        assert!(!ui.needs_paint());
    }

    fn queued(ui: &Runtime) -> u64 {
        ui.inspect()
            .sources
            .iter()
            .map(|source| source.fill + source.dropped_total)
            .sum()
    }

    #[test]
    fn the_producer_thread_publishes_only_while_a_run_toggle_is_on() {
        let mut ui = Runtime::new();
        let thread = ProducerHandle::default();
        let (workload, producer) = Workload::with_thread(&ui, false, thread.clone()).unwrap();
        ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
            .unwrap();
        ui.mount_tree(workload.element().unwrap()).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        workload.spawn(producer, None).unwrap();
        // Parked: a stopped page publishes nothing, however long it waits.
        std::thread::sleep(Duration::from_millis(100));
        assert!(!workload.control.running());
        assert_eq!(queued(&ui), 0, "a stopped page published");

        workload.running.set(true).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        assert!(workload.control.running());
        let start = Instant::now();
        while queued(&ui) == 0 {
            assert!(start.elapsed() < Duration::from_secs(10), "never published");
            std::thread::sleep(Duration::from_millis(5));
        }

        workload.running.set(false).unwrap();
        settle(&mut ui, Duration::ZERO).unwrap();
        assert!(!workload.control.running());
        assert_eq!(thread.join().unwrap(), Some(ProducerEnd::Stopped));
        assert_eq!(thread.join().unwrap(), None, "joined once");
    }

    #[test]
    fn the_producer_stops_at_a_closed_endpoint_on_its_thread_and_when_driven() {
        let ui = Runtime::new();
        let thread = ProducerHandle::default();
        let (workload, producer) = Workload::with_thread(&ui, true, thread.clone()).unwrap();
        let (driven_workload, mut driven) = Workload::new(&ui, false).unwrap();
        // Dropping the runtime and the workloads drops the consumers. Nothing
        // is mounted, so no gate ran and the thread is told to run by hand.
        workload.control.visuals.store(true, Ordering::Release);
        workload.spawn(producer, None).unwrap();
        drop(ui);
        drop(workload);
        drop(driven_workload);
        assert_eq!(
            driven.advance(Duration::from_secs(1)),
            Some(PublishOutcome::Closed)
        );
        assert_eq!(
            driven.advance(Duration::from_secs(2)),
            Some(PublishOutcome::Closed),
            "later calls publish nothing"
        );
        let start = Instant::now();
        while thread.0.borrow().as_ref().is_some_and(|thread| {
            thread
                .handle
                .as_ref()
                .is_some_and(|handle| !handle.is_finished())
        }) {
            assert!(start.elapsed() < Duration::from_secs(10), "never ended");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            thread.join().unwrap(),
            Some(ProducerEnd::Endpoint(PublishOutcome::Closed))
        );
    }
}
