//! Updating bindings after a parameter sink: a capacity-one refresh mailbox
//! the sink sends to, and host writes the application forwards itself.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The application's undo history and the refresh path that shows its depth.
struct App {
    ui: Runtime,
    gain: Parameter<f32>,
    host: ParameterHost<f32>,
    history: Rc<RefCell<Vec<f32>>>,
    depth: Signal<usize>,
    refresh: Mailbox<()>,
    monitor: DeliveryMonitor<f32>,
    fader: WidgetId,
}

fn app(capacity: usize) -> Result<App, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (gain, host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let history = Rc::new(RefCell::new(Vec::new()));
    let depth = ui.signal(0usize);
    let (shown, read) = (depth.clone(), history.clone());
    // The handler rereads the application state; the token carries nothing.
    let refresh = ui.mailbox(Capacity::new(capacity)?, move |()| {
        shown.set(read.borrow().len())
    })?;
    let (sender, record) = (refresh.sender(), history.clone());
    let monitor = host.on_outcome(move |event| {
        if let EditOutcome::Committed { .. } = &event.outcome {
            record.borrow_mut().push(event.begin_value);
        }
        // Full means a refresh is already pending; it will read this outcome.
        let _ = sender.send(());
        Ok(())
    })?;
    let fader = ui.mount_tree(Element::new("fader", Slider::new(gain.clone())))?;
    ui.flush(Duration::ZERO)?;
    Ok(App {
        ui,
        gain,
        host,
        history,
        depth,
        refresh,
        monitor,
        fader,
    })
}

fn step(app: &mut App) -> TestResult {
    app.ui.focus(&app.fader)?;
    app.ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    app.ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    Ok(())
}

#[test]
fn a_sink_refreshes_a_readout_through_a_mailbox() -> TestResult {
    let mut app = app(1)?;
    step(&mut app)?;
    step(&mut app)?;
    assert_eq!(app.history.borrow().len(), 2);
    // The second commit found the refresh already queued; the one refresh
    // shows both.
    assert_eq!(app.refresh.pending(), 1);
    app.ui.flush(Duration::from_millis(1))?;
    assert_eq!(app.depth.get()?, 2);
    assert!(!app.monitor.snapshot().is_faulted());
    // An undo is the application's own write: no sink runs, so the
    // application refreshes the readout itself.
    let previous = app.history.borrow_mut().pop().ok_or("an entry")?;
    app.host.write_base(previous)?;
    app.ui.flush(Duration::from_millis(2))?;
    assert_eq!(app.refresh.pending(), 0);
    assert_eq!(
        app.depth.get()?,
        2,
        "stale until the application refreshes it"
    );
    app.refresh
        .sender()
        .send(())
        .map_err(|_| "refresh queued")?;
    app.ui.flush(Duration::from_millis(3))?;
    assert_eq!(app.depth.get()?, 1);
    Ok(())
}

#[test]
fn a_sink_cannot_write_while_it_runs() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (gain, host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let depth = ui.signal(0usize);
    let refused = Rc::new(RefCell::new(None));
    let (target, seen) = (depth.clone(), refused.clone());
    let _monitor = host.on_outcome(move |_| {
        *seen.borrow_mut() = Some(target.set(1));
        Ok(())
    })?;
    let fader = ui.mount_tree(Element::new("fader", Slider::new(gain)))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&fader)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert_eq!(
        *refused.borrow(),
        Some(Err(RuntimeError::WriteDuringDelivery))
    );
    assert_eq!(depth.get()?, 0);
    Ok(())
}

#[test]
fn a_failing_refresh_is_reported_by_its_flush_without_faulting_the_parameter() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (gain, host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let refresh = ui.mailbox(Capacity::new(1)?, |()| {
        Err(ApplicationError::from("the readout is gone").into())
    })?;
    let sender = refresh.sender();
    let monitor = host.on_outcome(move |_| {
        let _ = sender.send(());
        Ok(())
    })?;
    let fader = ui.mount_tree(Element::new("fader", Slider::new(gain)))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&fader)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    let error = ui.flush(Duration::from_millis(1)).unwrap_err();
    assert_eq!(error.to_string(), "the readout is gone");
    assert_eq!(refresh.pending(), 0, "the token was consumed");
    assert!(!monitor.snapshot().is_faulted());
    ui.flush(Duration::from_millis(2))?;
    // A closed mailbox refuses the token; the sink still succeeds.
    refresh.close();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release)?;
    assert_eq!(
        monitor.snapshot().terminal.map(|event| event.outcome),
        None,
        "delivered and released"
    );
    assert!(!monitor.snapshot().is_faulted());
    Ok(())
}

#[test]
fn a_preview_sink_refreshes_the_same_way() -> TestResult {
    let mut app = app(1)?;
    let previews = app.ui.signal(0usize);
    let seen = Rc::new(RefCell::new(0usize));
    let (count, shown) = (seen.clone(), previews.clone());
    let preview_refresh = app
        .ui
        .mailbox(Capacity::new(1)?, move |()| shown.set(*count.borrow()))?;
    let (sender, count) = (preview_refresh.sender(), seen.clone());
    // Preview sinks need no active edit to register; replace the host's sinks.
    app.host.clear_sinks()?;
    let _monitor = app.host.on_outcome(|_| Ok(()))?;
    app.host.on_preview(move |_| {
        *count.borrow_mut() += 1;
        let _ = sender.send(());
        Ok(())
    })?;
    let [x, y, w, h] = app.ui.bounds(&app.fader)?.ok_or("laid out")?.values();
    let press = |ui: &mut Runtime, x: f32, kind| {
        ui.pointer_input(PointerEvent::new(Point::new(x, y + h / 2.0).unwrap(), kind))
    };
    press(
        &mut app.ui,
        x + w * 0.3,
        PointerKind::Press(PointerButton::Primary),
    )?;
    press(&mut app.ui, x + w * 0.6, PointerKind::Move)?;
    press(&mut app.ui, x + w * 0.7, PointerKind::Move)?;
    app.ui.flush(Duration::from_millis(1))?;
    assert!(*seen.borrow() >= 2);
    assert_eq!(previews.get()?, *seen.borrow());
    press(
        &mut app.ui,
        x + w * 0.7,
        PointerKind::Release(PointerButton::Primary),
    )?;
    assert!(app.gain.snapshot().get()?.preview.is_none());
    Ok(())
}

#[test]
fn a_host_write_during_a_gesture_ends_it_before_returning() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let (gain, host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let _monitor = host.on_outcome(move |event| {
        log.borrow_mut().push((event.begin_value, event.outcome));
        Ok(())
    })?;
    let fader = ui.mount_tree(Element::new("fader", Slider::new(gain.clone())))?;
    ui.flush(Duration::ZERO)?;
    // Without an edit a host write calls no sink.
    host.write_base(0.4)?;
    assert!(events.borrow().is_empty());
    ui.flush(Duration::ZERO)?;
    for undo in [0.25, 0.4] {
        let [x, y, w, h] = ui.bounds(&fader)?.ok_or("laid out")?.values();
        let at = |x| Point::new(x, y + h / 2.0).unwrap();
        ui.pointer_input(PointerEvent::new(
            at(x + w * 0.3),
            PointerKind::Press(PointerButton::Primary),
        ))?;
        ui.pointer_input(PointerEvent::new(at(x + w * 0.8), PointerKind::Move))?;
        assert!(gain.snapshot().get()?.preview.is_some());
        // An undo during the drag, also of the value already there: the
        // drag's terminal event is delivered before write_base returns.
        host.write_base(undo)?;
        let (begin, outcome) = events.borrow().last().cloned().ok_or("delivered")?;
        assert_eq!(begin, 0.4);
        assert_eq!(outcome, EditOutcome::Rejected(EditError::Conflict));
        assert_eq!(gain.snapshot().get()?.base, undo);
        ui.pointer_input(PointerEvent::new(
            at(x + w * 0.8),
            PointerKind::Release(PointerButton::Primary),
        ))?;
        ui.flush(Duration::ZERO)?;
        host.write_base(0.4)?;
        ui.flush(Duration::ZERO)?;
    }
    assert_eq!(events.borrow().len(), 2);
    Ok(())
}
