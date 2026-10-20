//! A mixer panel over an application-owned simulation.
//!
//! A producer thread stands in for an audio engine: it receives gain and mute
//! changes through a bounded command queue and publishes one level per channel
//! through a `series_bridge` to the `LevelMeter`s. Parameter sinks send the
//! engine the base each edit leaves, without blocking (so a cancelled
//! audition is undone too), record an undo history and refresh the toolbar
//! through a capacity-one mailbox. A full or stopped engine faults the
//! parameter, which keeps its outcome; the toolbar shows that the engine is
//! behind, and Retry delivers faulted outcomes with `retry_outcome` and the
//! application's own writes again, always sending current values so nothing
//! stale overwrites a newer one. Undo and reset write the base, also during
//! an edit, which they end, and forward the value themselves. The layout is
//! saved and restored as a workspace document, and decode and partial
//! restores are reported in the status line.
//!
//! The simulation is not real-time safe: it is an ordinary thread, and a real
//! producer follows the time-critical endpoint rules in AGENTS.md.
//!
//! Without arguments the example runs a deterministic headless session and
//! checks its results, which needs no window:
//! `cargo run -p pert-ui --example live_mixer --no-default-features`. With the
//! `native` feature, `--window` opens it in a native window instead.
use pert_ui::prelude::*;
use pert_ui::{
    Capacity, CommandReceipt, CommandScope, Delivery, DeliveryMonitor, DeliverySnapshot,
    EditOutcome, InspectValue, KeyAction, KeyChord, KeyCode, LevelMeter, Mailbox, MeterAxis,
    MetricRole, Orientation, PowerExponent, SeriesConfig, SeriesEndpoint, SinkError, SinkErrorKind,
    Timestamp, UiSender, Workspace, series_bridge,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError, sync_channel},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const CHANNELS: usize = 3;
/// Commands the engine may hold before senders see `Full`.
const QUEUE: usize = 16;

/// What the UI asks of the engine.
enum Command {
    Gain(usize, f32),
    Mute(usize, bool),
    /// Publish one level per channel; the headless session steps time this way.
    Tick,
    /// Publish on a timer, as a native session does.
    Run(bool),
    /// Confirm on the first channel, then wait on the second, as an engine
    /// stalled by other work would; the headless session fills the queue
    /// this way.
    Hold(SyncSender<()>, Receiver<()>),
    Stop,
}

/// The engine's confirmation that it applied a gain, delivered to the UI.
struct Applied(usize, f32);

/// The application side of the producer thread.
struct Engine {
    commands: SyncSender<Command>,
    thread: Option<JoinHandle<()>>,
}
impl Engine {
    fn start(endpoints: Vec<SeriesEndpoint<f32>>, applied: UiSender<Applied>) -> Self {
        let (commands, inbox) = sync_channel(QUEUE);
        let thread = std::thread::spawn(move || produce(&inbox, endpoints, &applied));
        Self {
            commands,
            thread: Some(thread),
        }
    }
    /// Whether the producer thread has returned, so dropping the engine
    /// joins it at once.
    fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }
    /// Queues a command without blocking the UI thread; a full or closed
    /// queue is the caller's to report.
    fn send(&self, command: Command) -> Result<(), SinkError> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                TrySendError::Full(_) => SinkError::new(SinkErrorKind::Full, "the engine is busy"),
                TrySendError::Disconnected(_) => {
                    SinkError::new(SinkErrorKind::Closed, "the engine has stopped")
                }
            })
    }
}
impl Drop for Engine {
    /// Stops the producer and waits for it, so it never outlives the UI.
    fn drop(&mut self) {
        // A full queue still stops it: the thread ends when the sender drops.
        let _ = self.commands.try_send(Command::Stop);
        let (stopped, _) = sync_channel(0);
        drop(std::mem::replace(&mut self.commands, stopped));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The producer thread: applies commands and publishes levels.
fn produce(
    inbox: &Receiver<Command>,
    mut endpoints: Vec<SeriesEndpoint<f32>>,
    applied: &UiSender<Applied>,
) {
    // Gains in dB, as the faders hold them; every channel starts at 0 dB.
    let mut gains = [0.0f32; CHANNELS];
    let mut muted = [false; CHANNELS];
    let (mut running, mut tick) = (false, 0u64);
    loop {
        let command = if running {
            match inbox.recv_timeout(Duration::from_millis(20)) {
                Ok(command) => command,
                Err(RecvTimeoutError::Timeout) => Command::Tick,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match inbox.recv() {
                Ok(command) => command,
                Err(_) => return,
            }
        };
        match command {
            Command::Gain(channel, gain) => {
                gains[channel] = gain;
                // A full or closed mailbox loses only this confirmation.
                let _ = applied.send(Applied(channel, gain));
            }
            Command::Mute(channel, on) => muted[channel] = on,
            Command::Run(on) => running = on,
            Command::Hold(holding, release) => {
                let _ = holding.send(());
                let _ = release.recv();
            }
            Command::Stop => return,
            Command::Tick => {
                tick += 1;
                for (channel, endpoint) in endpoints.iter_mut().enumerate() {
                    let phase = (tick as f32 * 0.3 + channel as f32).sin() * 0.25 + 0.75;
                    let level = if muted[channel] {
                        0.0
                    } else {
                        10f32.powf(gains[channel] / 20.0).min(1.0) * phase
                    };
                    // A closed endpoint (its meter's source was dropped) only
                    // stops that channel's publishing.
                    let _outcome = endpoint.publish(level, Timestamp::new(tick));
                }
            }
        }
    }
}

/// One channel strip's parameters and whether the engine follows them.
struct Channel {
    gain: Parameter<f32>,
    gain_host: ParameterHost<f32>,
    gain_monitor: DeliveryMonitor<f32>,
    mute: Parameter<bool>,
    mute_host: ParameterHost<bool>,
    mute_monitor: DeliveryMonitor<bool>,
    /// The gain the engine last confirmed.
    applied: Signal<String>,
}

/// The application state the toolbar shows.
struct Mixer {
    engine: Rc<Engine>,
    channels: Vec<Channel>,
    /// Undo entries: the channel and the gain before the edit.
    history: Rc<RefCell<Vec<(usize, f32)>>>,
    /// Channels whose last write of the application's own did not reach the
    /// engine; Retry sends their current values.
    unsynced: Rc<[Cell<bool>]>,
    depth: Signal<usize>,
    /// Whether the engine follows every channel, refreshed with `depth`.
    health: Signal<String>,
    status: Signal<String>,
    saved: RefCell<Option<String>>,
    refresh: Mailbox<()>,
    _applied: Mailbox<Applied>,
}
impl Mixer {
    /// Undo: the application's own write, forwarded to the engine here.
    fn undo(&self) -> Result<(), RuntimeError> {
        let Some((channel, gain)) = self.history.borrow_mut().pop() else {
            return self.status.set("Nothing to undo".to_owned());
        };
        self.channels[channel].gain_host.write_base(gain)?;
        self.synced(channel, "Undone")
    }
    /// Reset a channel to its default, also during an edit, which it ends.
    fn reset(&self, channel: usize) -> Result<(), RuntimeError> {
        let gain = self.channels[channel].gain.spec()?.default_value();
        self.channels[channel].gain_host.write_base(gain)?;
        self.synced(channel, "Reset")
    }
    /// Sends a channel's current values; a failure is kept for Retry.
    fn sync(&self, channel: usize) -> Result<Result<(), SinkError>, RuntimeError> {
        let gain = self.channels[channel].gain.snapshot().get()?.base;
        let muted = self.channels[channel].mute.snapshot().get()?.base;
        let sent = self
            .engine
            .send(Command::Gain(channel, gain))
            .and_then(|()| self.engine.send(Command::Mute(channel, muted)));
        self.unsynced[channel].set(sent.is_err());
        Ok(sent)
    }
    fn synced(&self, channel: usize, done: &str) -> Result<(), RuntimeError> {
        let sent = self.sync(channel)?;
        self.refresh_now();
        self.status.set(match sent {
            Ok(()) => done.to_owned(),
            Err(error) => format!("{done}; {error}, press Retry"),
        })
    }
    /// Delivers what did not reach the engine: faulted parameter outcomes
    /// through `retry_outcome`, which runs their sinks again, and the
    /// application's own writes. A stopped engine stays stopped; Retry then
    /// reports it again.
    fn retry(&self) -> Result<(), RuntimeError> {
        let mut failure = None;
        for (index, channel) in self.channels.iter().enumerate() {
            if let Some(event) = faulted(&channel.gain_monitor)
                && let Err(error) = channel.gain_host.retry_outcome(&event)
            {
                failure.get_or_insert(cause(&channel.gain_monitor, &error));
            }
            if let Some(event) = faulted(&channel.mute_monitor)
                && let Err(error) = channel.mute_host.retry_outcome(&event)
            {
                failure.get_or_insert(cause(&channel.mute_monitor, &error));
            }
            if self.unsynced[index].get()
                && let Err(error) = self.sync(index)?
            {
                failure.get_or_insert(error.to_string());
            }
        }
        self.refresh_now();
        self.status
            .set(failure.unwrap_or_else(|| "Engine in sync".to_owned()))
    }
    fn refresh_now(&self) {
        // Full: a refresh is already pending and reads the same state.
        let _ = self.refresh.sender().send(());
    }
    fn save_layout(&self, ui: &Runtime) -> Result<(), RuntimeError> {
        *self.saved.borrow_mut() = Some(ui.capture_workspace()?.encode());
        self.status.set("Layout saved".to_owned())
    }
    /// Restores `text`, reporting a document or restore failure to the user.
    fn restore_layout(&self, ui: &mut Runtime, text: &str) -> Result<(), RuntimeError> {
        let restored =
            Workspace::decode(text).and_then(|workspace| ui.restore_workspace(&workspace));
        self.status.set(match restored {
            Ok(report) if report.is_complete() => "Layout restored".to_owned(),
            Ok(report) => format!(
                "Layout partly restored: {} entries ignored, {} rejected",
                report.ignored().count(),
                report.rejected().count()
            ),
            Err(error) => format!("Layout not restored: {error}"),
        })
    }
}

/// Why a retry failed: the sink's own error, which the monitor keeps, rather
/// than the retry's report that the sink failed.
fn cause<T: pert_ui::ParameterValue>(monitor: &DeliveryMonitor<T>, error: &RuntimeError) -> String {
    monitor
        .snapshot()
        .last_error
        .map_or_else(|| error.to_string(), |sink| sink.to_string())
}

/// The token of a parameter's undelivered outcome, if its sink failed.
fn faulted<T: pert_ui::ParameterValue>(monitor: &DeliveryMonitor<T>) -> Option<pert_ui::EditToken> {
    let snapshot = monitor.snapshot();
    snapshot
        .is_faulted()
        .then(|| snapshot.terminal.map(|event| event.token))
        .flatten()
}

/// A sink's view of the engine: a failed send faults the parameter, which
/// keeps the outcome until Retry delivers it again.
fn current<T: pert_ui::ParameterValue>(parameter: &Parameter<T>) -> Result<T, SinkError> {
    parameter
        .snapshot()
        .get()
        .map(|snapshot| snapshot.base)
        .map_err(|error| SinkError::new(SinkErrorKind::Other, error.to_string()))
}

fn build(ui: &mut Runtime) -> Result<(Rc<Mixer>, WidgetId), RuntimeError> {
    let mut endpoints = Vec::new();
    let mut sources = Vec::new();
    for _ in 0..CHANNELS {
        let (endpoint, consumer) = series_bridge::<f32>(SeriesConfig::new(Capacity::new(64)?));
        endpoints.push(endpoint);
        sources.push(ui.series_source(consumer, Delivery::Polled));
    }
    let applied_signals: Vec<Signal<String>> = (0..CHANNELS)
        .map(|_| ui.signal(String::from("0.0 dB")))
        .collect();
    let shown = applied_signals.clone();
    let applied = ui.mailbox(Capacity::new(QUEUE)?, move |Applied(channel, gain)| {
        shown[channel].set(format!("{gain:.1} dB"))
    })?;
    let engine = Rc::new(Engine::start(endpoints, applied.sender()));

    let history: Rc<RefCell<Vec<(usize, f32)>>> = Rc::default();
    let unsynced: Rc<[Cell<bool>]> = (0..CHANNELS).map(|_| Cell::new(false)).collect();
    let depth = ui.signal(0usize);
    let health = ui.signal(String::from("Engine in sync"));
    let status = ui.signal(String::from("Ready"));

    let mut channels: Vec<Channel> = Vec::new();
    // The refresh is created first so that every sink can send to it; its
    // handler reads the channels through this shared slot.
    type Reports = (
        ReadSignal<DeliverySnapshot<f32>>,
        ReadSignal<DeliverySnapshot<bool>>,
    );
    let slot: Rc<RefCell<Vec<Reports>>> = Rc::default();
    let (depth_out, health_out, read, monitors, flags) = (
        depth.clone(),
        health.clone(),
        history.clone(),
        slot.clone(),
        unsynced.clone(),
    );
    // The sinks cannot write; this refresh rereads the state afterwards.
    let refresh = ui.mailbox(Capacity::new(1)?, move |()| {
        depth_out.set(read.borrow().len())?;
        let mut behind = flags.iter().any(Cell::get);
        for (gain, mute) in monitors.borrow().iter() {
            behind |= gain.get()?.is_faulted() || mute.get()?.is_faulted();
        }
        health_out.set(if behind {
            "Engine behind: press Retry".to_owned()
        } else {
            "Engine in sync".to_owned()
        })
    })?;

    for (index, applied) in applied_signals.into_iter().enumerate() {
        let object = index as u128 + 1;
        let (gain, gain_host) = ui.parameter(
            ParameterId::new(1, object, "gain")?,
            ParameterSpec::new("Gain", -60.0, 12.0, 0.0)?
                .with_mapping(ParameterMapping::Power(PowerExponent::new(2.0)?))?,
        )?;
        let (mute, mute_host) = ui.parameter(
            ParameterId::new(1, object, "mute")?,
            ParameterSpec::boolean("Mute", false)?,
        )?;
        let (forward, record, sender, base) = (
            engine.clone(),
            history.clone(),
            refresh.sender(),
            gain.clone(),
        );
        // The edit a retried outcome belongs to is recorded once.
        let recorded: RefCell<Option<pert_ui::EditToken>> = RefCell::new(None);
        let gain_monitor = gain_host.on_outcome(move |event| {
            // Whatever ended the edit, the engine follows the base the
            // parameter now holds: a committed value, or the base a cancelled
            // audition returns to. Reading it here also keeps a retried
            // outcome from overwriting a newer write.
            // A commit is undoable whether or not the engine took it.
            if matches!(event.outcome, EditOutcome::Committed { .. })
                && recorded.borrow().as_ref() != Some(&event.token)
            {
                record.borrow_mut().push((index, event.begin_value));
                *recorded.borrow_mut() = Some(event.token.clone());
            }
            let sent = current(&base).and_then(|gain| forward.send(Command::Gain(index, gain)));
            let _ = sender.send(()); // full: a refresh is already pending
            sent
        })?;
        let audition = engine.clone();
        gain_host.on_preview(move |event| {
            // An audition that does not fit is dropped; the terminal outcome
            // still sends the base.
            let _ = audition.send(Command::Gain(index, event.value));
            Ok(())
        })?;
        let (forward, sender, base) = (engine.clone(), refresh.sender(), mute.clone());
        let mute_monitor = mute_host.on_outcome(move |_| {
            let sent = current(&base).and_then(|muted| forward.send(Command::Mute(index, muted)));
            let _ = sender.send(());
            sent
        })?;
        slot.borrow_mut()
            .push((gain_monitor.observe(), mute_monitor.observe()));
        channels.push(Channel {
            gain,
            gain_host,
            gain_monitor,
            mute,
            mute_host,
            mute_monitor,
            applied,
        });
    }

    let mixer = Rc::new(Mixer {
        engine,
        channels,
        history,
        unsynced,
        depth,
        health,
        status,
        saved: RefCell::new(None),
        refresh,
        _applied: applied,
    });
    let root = mount(ui, &mixer, sources)?;
    Ok((mixer, root))
}

fn mount(
    ui: &mut Runtime,
    mixer: &Rc<Mixer>,
    sources: Vec<pert_ui::SeriesSource<f32>>,
) -> Result<WidgetId, RuntimeError> {
    let level = ParameterSpec::new("Level", 0.0, 1.0, 0.0)?;
    let strips =
        mixer
            .channels
            .iter()
            .zip(sources)
            .enumerate()
            .map(|(index, (channel, source))| {
                Element::new(
                    format!("ch-{index}"),
                    Column::new([
                        Element::new("name", Label::new(format!("Channel {}", index + 1))),
                        Element::new(
                            "faders",
                            Row::new([
                                Element::new(
                                    "gain",
                                    Slider::new(channel.gain.clone())
                                        .orientation(Orientation::Vertical),
                                ),
                                Element::new(
                                    "meter",
                                    LevelMeter::new(source, level.clone())
                                        .axis(MeterAxis::Vertical),
                                ),
                            ])
                            .align(Align::Stretch),
                        )
                        .with_grow(1),
                        Element::new("mute", Toggle::for_parameter(channel.mute.clone())),
                        Element::new("applied", Label::new(channel.applied.read_only())),
                    ]),
                )
            });
    let undo = mixer.clone();
    let undo = ui.register_command(
        "Undo",
        CommandScope::Application,
        vec![KeyChord::primary('z')],
        move |_, _| undo.undo(),
    )?;
    let saving = mixer.clone();
    let save = ui.register_command(
        "Save layout",
        CommandScope::Application,
        vec![],
        move |ui, _| saving.save_layout(ui),
    )?;
    let restoring = mixer.clone();
    let restore = ui.register_command(
        "Restore layout",
        CommandScope::Application,
        vec![],
        move |ui, _| {
            let saved = restoring.saved.borrow().clone();
            match saved {
                Some(text) => restoring.restore_layout(ui, &text),
                None => restoring.status.set("No saved layout".to_owned()),
            }
        },
    )?;
    let reset = mixer.clone();
    let retry = mixer.clone();
    let depth = mixer.depth.map(|depth| format!("Undo: {depth}"))?;
    // These commands report their own outcome in the status line; a refused
    // or failed invocation would also be in the receipt.
    let report = |_: CommandReceipt, _: &mut EventContext| Ok(());
    let toolbar = Row::new([
        Element::new("undo", Button::command(ui, &undo, report)?),
        Element::new(
            "reset",
            Button::new("Reset channel 1", move |_| reset.reset(0)),
        ),
        Element::new("save", Button::command(ui, &save, report)?),
        Element::new("restore", Button::command(ui, &restore, report)?),
        Element::new("retry", Button::new("Retry", move |_| retry.retry())),
    ]);
    let state = Row::new([
        Element::new("depth", Label::new(depth)),
        Element::new("health", Label::new(mixer.health.read_only())),
    ]);
    // Messages vary in length: the column gives this line the window's
    // width, and a longer message is cut there instead of pushing anything.
    let status = Label::new(mixer.status.read_only()).overflow(TextOverflow::Ellipsis);
    ui.mount_tree(Element::new(
        "mixer",
        Container::new(Element::new(
            "panel",
            Column::new([
                Element::new("toolbar", toolbar),
                Element::new(
                    "body",
                    SplitPane::new(
                        SplitAxis::LeftRight,
                        Element::new("strips", Row::new(strips).align(Align::Stretch)),
                        Element::new("notes", Label::new("Levels are simulated")),
                    ),
                )
                .with_grow(1),
                Element::new("state", state),
                Element::new("status", status),
            ])
            .align(Align::Stretch),
        ))
        .padding(MetricRole::ContentMargin),
    ))
}

/// Flushes until `done` holds, or fails after a bounded wait for the engine.
/// How long one step of the headless session waits for the engine before it
/// fails, so a stalled engine ends the session instead of hanging it.
const STEP: Duration = Duration::from_secs(5);

fn settle(
    ui: &mut Runtime,
    start: Instant,
    mut done: impl FnMut(&Runtime) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + STEP;
    loop {
        ui.flush(start.elapsed())?;
        if done(ui) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err("the engine did not answer in time".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn level(ui: &Runtime, channel: usize) -> f64 {
    let key = format!("ch-{channel}");
    ui.inspect()
        .components
        .iter()
        .find(|component| {
            component.key == "meter"
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.contains(&key))
        })
        .and_then(
            |component| match component.details.properties.get("level") {
                Some(InspectValue::Number(level)) => Some(*level),
                _ => None,
            },
        )
        .unwrap_or(0.0)
}

/// The deterministic session: every step waits, bounded, for what it checks.
fn headless() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 520.0, 1.0)?)?;
    let (mixer, root) = build(&mut ui)?;
    settle(&mut ui, start, |_| true)?;
    // This session steps time itself, one tick at a time.
    mixer.engine.send(Command::Run(false))?;
    let applied = |channel: usize| mixer.channels[channel].applied.get().unwrap_or_default();
    let base = |channel: usize| -> Result<f32, RuntimeError> {
        Ok(mixer.channels[channel].gain.snapshot().get()?.base)
    };
    let shown = |gain: f32| format!("{gain:.1} dB");

    // Two keyboard steps on the first fader: two commits, each forwarded.
    let fader = ui
        .find(
            &root,
            &["panel", "body", "strips", "ch-0", "faders", "gain"],
        )?
        .ok_or("fader")?;
    ui.focus(&fader)?;
    for _ in 0..2 {
        ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
        ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    }
    let committed = base(0)?;
    settle(&mut ui, start, |_| applied(0) == shown(committed))?;
    assert_eq!(mixer.depth.get()?, 2);

    // One engine tick reaches every meter.
    mixer.engine.send(Command::Tick)?;
    settle(&mut ui, start, |ui| {
        (0..CHANNELS).all(|channel| level(ui, channel) > 0.0)
    })?;

    // A drag auditions through the engine; cancelling it with Escape sends
    // the base it returns to.
    let [x, y, w, h] = ui.bounds(&fader)?.ok_or("laid out")?.values();
    let at = |fraction: f32| Point::new(x + w / 2.0, y + h * fraction);
    let drag = |ui: &mut Runtime| -> Result<(), RuntimeError> {
        ui.pointer_input(PointerEvent::new(
            at(0.5)?,
            PointerKind::Press(PointerButton::Primary),
        ))?;
        ui.pointer_input(PointerEvent::new(at(0.2)?, PointerKind::Move))?;
        Ok(())
    };
    let release = |ui: &mut Runtime| -> Result<(), RuntimeError> {
        ui.pointer_input(PointerEvent::new(
            at(0.2)?,
            PointerKind::Release(PointerButton::Primary),
        ))?;
        Ok(())
    };
    drag(&mut ui)?;
    let audition = mixer.channels[0]
        .gain
        .snapshot()
        .get()?
        .preview
        .ok_or("auditioning")?;
    settle(&mut ui, start, |_| applied(0) == shown(audition))?;
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    ui.key_input(KeyCode::Escape, KeyAction::Release)?;
    release(&mut ui)?;
    settle(&mut ui, start, |_| applied(0) == shown(committed))?;
    assert_eq!(base(0)?, committed);

    // Undo during a drag ends the drag and returns one step.
    drag(&mut ui)?;
    assert!(mixer.channels[0].gain.snapshot().get()?.preview.is_some());
    mixer.undo()?;
    assert!(mixer.channels[0].gain.snapshot().get()?.preview.is_none());
    release(&mut ui)?;
    let undone = base(0)?;
    assert!(undone < committed);
    settle(&mut ui, start, |_| applied(0) == shown(undone))?;
    assert_eq!(mixer.depth.get()?, 1);

    // Reset during a drag likewise; neither adds to the history.
    drag(&mut ui)?;
    mixer.reset(0)?;
    release(&mut ui)?;
    settle(&mut ui, start, |_| applied(0) == "0.0 dB")?;
    assert_eq!(base(0)?, 0.0);
    assert_eq!(mixer.depth.get()?, 1);

    // A busy engine: hold it and fill its queue. A commit then faults its
    // parameter, and the undo that follows cannot be delivered either; the
    // toolbar says so, and Retry delivers both once the engine has room,
    // sending the undone value rather than the faulted commit's.
    // Both channels have room for their one message, so neither side blocks.
    let (holding, held) = sync_channel(1);
    let (resume, waiting) = sync_channel(1);
    mixer.engine.send(Command::Hold(holding, waiting))?;
    // Once the engine confirms, it takes nothing more from its queue.
    held.recv_timeout(STEP)
        .map_err(|_| "the engine did not confirm that it holds")?;
    while mixer.engine.send(Command::Tick).is_ok() {}
    ui.focus(&fader)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert!(mixer.channels[0].gain_monitor.snapshot().is_faulted());
    settle(&mut ui, start, |_| {
        mixer
            .health
            .get()
            .is_ok_and(|health| health.contains("Retry"))
    })?;
    assert_eq!(mixer.depth.get()?, 2, "the faulted commit is undoable");
    mixer.undo()?;
    assert!(
        mixer.status.get()?.ends_with("press Retry"),
        "{}",
        mixer.status.get()?
    );
    let undone = base(0)?;
    resume
        .try_send(())
        .map_err(|_| "the engine is not waiting to resume")?;
    // Retry until the engine has drained enough to take everything.
    settle(&mut ui, start, |_| {
        mixer.retry().is_ok()
            && mixer
                .status
                .get()
                .is_ok_and(|status| status == "Engine in sync")
    })?;
    settle(&mut ui, start, |_| applied(0) == shown(undone))?;
    settle(&mut ui, start, |_| {
        mixer
            .health
            .get()
            .is_ok_and(|health| health == "Engine in sync")
    })?;
    assert!(!mixer.channels[0].gain_monitor.snapshot().is_faulted());
    // The retried commit was not recorded again.
    assert_eq!(mixer.depth.get()?, 1);
    assert_eq!(base(0)?, undone);

    // The layout round-trips; a damaged document is reported, not applied,
    // and one naming parts this layout lacks is applied only in part.
    mixer.save_layout(&ui)?;
    mixer.restore_layout(&mut ui, "{ damaged")?;
    assert!(mixer.status.get()?.starts_with("Layout not restored"));
    let saved = mixer.saved.borrow().clone().ok_or("saved")?;
    let foreign = saved.replace("mixer/panel/body", "mixer/panel/elsewhere");
    assert_ne!(foreign, saved, "the saved layout names the split");
    mixer.restore_layout(&mut ui, &foreign)?;
    assert!(
        mixer.status.get()?.starts_with("Layout partly restored"),
        "{}",
        mixer.status.get()?
    );
    mixer.restore_layout(&mut ui, &saved)?;
    assert_eq!(mixer.status.get()?, "Layout restored");
    settle(&mut ui, start, |_| true)?;

    // A stopped engine: an outcome stays faulted, and Retry says why.
    // Closed means the engine dropped its queue; it has stopped once its
    // thread has also returned, so dropping the mixer below joins at once.
    mixer.engine.send(Command::Stop)?;
    let deadline = Instant::now() + STEP;
    while !matches!(
        mixer.engine.send(Command::Tick),
        Err(error) if error.kind() == SinkErrorKind::Closed
    ) || !mixer.engine.finished()
    {
        if Instant::now() > deadline {
            return Err("the engine did not stop".into());
        }
        std::thread::yield_now();
    }
    ui.focus(&fader)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert!(mixer.channels[0].gain_monitor.snapshot().is_faulted());
    mixer.retry()?;
    assert!(
        mixer.status.get()?.contains("stopped"),
        "{}",
        mixer.status.get()?
    );

    // Shutting down drops the engine, which stops and joins its thread.
    drop(mixer);
    ui.shutdown();
    println!("live mixer: headless session passed");
    Ok(())
}

#[cfg(feature = "native")]
fn window() -> Result<(), Box<dyn std::error::Error>> {
    App::new("Live mixer", |ui| {
        let (mixer, _root) = build(ui)?;
        mixer
            .engine
            .send(Command::Run(true))
            .map_err(pert_ui::ApplicationError::new)?;
        ui.keep_alive(mixer);
        Ok(())
    })
    .size(Size::from([900, 520]))?
    .run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|argument| argument == "--window") {
        #[cfg(feature = "native")]
        return window();
        #[cfg(not(feature = "native"))]
        return Err("--window needs the `native` feature".into());
    }
    headless()
}
