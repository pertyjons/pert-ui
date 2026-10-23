//! Close admission through `App::headless` and `HeadlessApp::admit_close`:
//! focused drafts are released before `App::on_close` reads the
//! application's state, focus comes back only after a synchronous
//! `KeepOpen`, and admission's errors come back directly.
#![cfg(feature = "native")]

use pert_ui::prelude::*;
use pert_ui::testing::{Driver, HeadlessApp};
use pert_ui::{InspectValue, NativeWindowOptions, ParameterId, ParameterSpec};
use std::{cell::Cell, cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Policy<T> = Box<dyn FnMut(&mut Runtime, &T) -> Result<NativeCloseAction, RuntimeError>>;

/// Starts `build` as a headless application whose close policy is
/// `policy`, and returns what `build` made, which the policy reads too.
fn start<T: 'static>(
    build: impl FnOnce(&mut Runtime) -> Result<T, RuntimeError> + 'static,
    mut policy: Policy<T>,
) -> Result<(HeadlessApp, Rc<T>), Box<dyn std::error::Error>> {
    let slot: Rc<RefCell<Option<Rc<T>>>> = Rc::default();
    let (filled, read) = (slot.clone(), slot.clone());
    let app = App::new("Close", move |ui| {
        *filled.borrow_mut() = Some(Rc::new(build(ui)?));
        Ok(())
    })
    .on_close(move |ui, _| {
        let built = read
            .borrow()
            .clone()
            .ok_or(ApplicationError::from("not built"))?;
        policy(ui, &built)
    })
    .headless()?;
    let built = slot.borrow().clone().ok_or("not built")?;
    Ok((app, built))
}

/// Two fields that hold a draft until Enter or focus loss.
struct Form {
    first: WidgetId,
    first_text: Signal<String>,
    second: WidgetId,
}
fn form(ui: &mut Runtime) -> Result<Form, RuntimeError> {
    let first_text = ui.signal(String::new());
    let second_text = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "form",
        Column::new([
            Element::new(
                "first",
                TextField::new(first_text.clone()).max_length(Some(100)),
            ),
            Element::new("second", TextField::new(second_text).max_length(Some(100))),
        ]),
    ))?;
    let find = |key: &str| ui.find(&root, &[key])?.ok_or(RuntimeError::Unmounted);
    Ok(Form {
        first: find("first")?,
        first_text,
        second: find("second")?,
    })
}

/// Types `text` into `form.first`, which keeps it as a draft.
fn draft(app: &mut HeadlessApp, form: &Form, text: &str) -> TestResult {
    Driver::new(app.runtime_mut()).replace_text(&form.first, text)?;
    assert_eq!(form.first_text.get()?, "", "still a draft");
    Ok(())
}

#[test]
fn the_close_policy_sees_a_draft_admission_saved() -> TestResult {
    let seen = Rc::new(RefCell::new(None));
    let record = seen.clone();
    let (mut app, form) = start(
        form,
        Box::new(move |_, form: &Form| {
            *record.borrow_mut() = Some(form.first_text.get()?);
            Ok(NativeCloseAction::Close)
        }),
    )?;
    draft(&mut app, &form, "Unsaved notes")?;
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
    assert_eq!(seen.borrow().as_deref(), Some("Unsaved notes"));
    // `Close` restores nothing; the decision is not applied.
    assert_eq!(app.runtime().focused(), None);
    assert!(app.runtime().contains_window(&main));
    app.shutdown()?;
    Ok(())
}

#[test]
fn a_parameter_draft_is_cancelled_not_committed() -> TestResult {
    struct Gain {
        field: WidgetId,
        gain: Parameter<f32>,
        _host: ParameterHost<f32>,
    }
    let seen = Rc::new(Cell::new(f32::NAN));
    let record = seen.clone();
    let (mut app, gain) = start(
        |ui| {
            let (gain, host) = ui.parameter(
                ParameterId::new(1, 1, "gain")?,
                ParameterSpec::new("Gain", 0.0, 1.0, 0.1)?,
            )?;
            let field = ui.mount_tree(Element::new(
                "gain",
                DecimalInput::for_parameter(gain.clone()),
            ))?;
            Ok(Gain {
                field,
                gain,
                _host: host,
            })
        },
        Box::new(move |_, gain: &Gain| {
            record.set(gain.gain.snapshot().get()?.base);
            Ok(NativeCloseAction::Close)
        }),
    )?;
    let shown = |app: &HeadlessApp| {
        app.runtime()
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "gain")
            .and_then(|c| c.details.properties.get("text").cloned())
    };
    Driver::new(app.runtime_mut()).replace_text(&gain.field, "0.75")?;
    assert_eq!(shown(&app), Some(InspectValue::Text("0.75".into())));
    let main = app.main_window();
    app.admit_close(&main)?;
    // A valid draft, cancelled: parameter authority commits only on an
    // explicit commit.
    assert_eq!(seen.get(), 0.1);
    assert_eq!(gain.gain.snapshot().get()?.base, 0.1);
    assert_eq!(shown(&app), Some(InspectValue::Text("0.1".into())));
    app.shutdown()?;
    Ok(())
}

/// A main-window field and a field in a detached window, each drafting.
struct Windows {
    main_field: WidgetId,
    main_text: Signal<String>,
    detached_field: WidgetId,
    detached_text: Signal<String>,
    detached: NativeWindowId,
}
fn windows(ui: &mut Runtime) -> Result<Windows, RuntimeError> {
    let main_text = ui.signal(String::new());
    let detached_text = ui.signal(String::new());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "main",
                TextField::new(main_text.clone()).max_length(Some(100)),
            ),
            Element::new(
                "panel",
                View::new(Element::new(
                    "detached",
                    TextField::new(detached_text.clone()).max_length(Some(100)),
                )),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let panel = ui.find(&root, &["panel"])?.ok_or(RuntimeError::Unmounted)?;
    let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
    let detached = ui.commit_detach(prepared)?;
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    Ok(Windows {
        main_field: ui.find(&root, &["main"])?.ok_or(RuntimeError::Unmounted)?,
        main_text,
        detached_field: ui
            .find(&root, &["panel", "detached"])?
            .ok_or(RuntimeError::Unmounted)?,
        detached_text,
        detached,
    })
}

/// Drafts `text` in both windows' fields; the main window ends selected.
fn draft_both(app: &mut HeadlessApp, windows: &Windows) -> TestResult {
    let main = app.main_window();
    let ui = app.runtime_mut();
    ui.select_window(&windows.detached)?;
    Driver::new(ui).replace_text(&windows.detached_field, "detached draft")?;
    ui.select_window(&main)?;
    Driver::new(ui).replace_text(&windows.main_field, "main draft")?;
    assert_eq!(windows.main_text.get()?, "");
    assert_eq!(windows.detached_text.get()?, "");
    Ok(())
}

#[test]
fn closing_the_main_window_releases_every_windows_draft() -> TestResult {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = seen.clone();
    let (mut app, windows) = start(
        windows,
        Box::new(move |_, windows: &Windows| {
            record
                .borrow_mut()
                .push((windows.main_text.get()?, windows.detached_text.get()?));
            Ok(NativeCloseAction::Close)
        }),
    )?;
    draft_both(&mut app, &windows)?;
    let main = app.main_window();
    app.admit_close(&main)?;
    assert_eq!(
        *seen.borrow(),
        [("main draft".to_owned(), "detached draft".to_owned())]
    );
    // The window selected before the call is selected again.
    assert_eq!(app.runtime().current_window(), main);
    app.shutdown()?;
    Ok(())
}

#[test]
fn closing_a_detached_window_releases_only_its_own_draft() -> TestResult {
    let (mut app, windows) = start(
        windows,
        Box::new(|_, _: &Windows| Ok(NativeCloseAction::Close)),
    )?;
    draft_both(&mut app, &windows)?;
    let detached = windows.detached.clone();
    app.admit_close(&detached)?;
    assert_eq!(windows.detached_text.get()?, "detached draft");
    assert_eq!(
        windows.main_text.get()?,
        "",
        "the main window's draft stays"
    );
    assert_eq!(app.runtime().focused(), Some(windows.main_field.clone()));
    app.shutdown()?;
    Ok(())
}

#[test]
fn keep_open_focuses_the_released_editor_again() -> TestResult {
    let (mut app, form) = start(
        form,
        Box::new(|_, _: &Form| Ok(NativeCloseAction::KeepOpen)),
    )?;
    draft(&mut app, &form, "Kept open")?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(form.first_text.get()?, "Kept open");
    assert_eq!(app.runtime().focused(), Some(form.first.clone()));
    app.shutdown()?;
    Ok(())
}

#[test]
fn keep_open_leaves_a_component_the_policy_focused() -> TestResult {
    let (mut app, form) = start(
        form,
        Box::new(|ui, form: &Form| {
            ui.focus(&form.second)?;
            Ok(NativeCloseAction::KeepOpen)
        }),
    )?;
    draft(&mut app, &form, "Moved")?;
    let main = app.main_window();
    app.admit_close(&main)?;
    assert_eq!(app.runtime().focused(), Some(form.second.clone()));
    app.shutdown()?;
    Ok(())
}

#[test]
fn keep_open_leaves_focus_cleared_for_a_hidden_editor() -> TestResult {
    let (mut app, form) = start(
        form,
        Box::new(|ui, form: &Form| {
            ui.set_visible(&form.first, false)?;
            Ok(NativeCloseAction::KeepOpen)
        }),
    )?;
    draft(&mut app, &form, "Hidden")?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(app.runtime().focused(), None);
    app.shutdown()?;
    Ok(())
}

#[test]
fn a_deferred_decision_restores_nothing() -> TestResult {
    let decision = Rc::new(RefCell::new(None));
    let keep = decision.clone();
    let (mut app, form) = start(
        form,
        Box::new(move |ui, _: &Form| {
            let signal = ui.signal(None);
            *keep.borrow_mut() = Some(signal.clone());
            Ok(NativeCloseAction::Defer(signal.read_only()))
        }),
    )?;
    draft(&mut app, &form, "Deferred")?;
    let main = app.main_window();
    let NativeCloseAction::Defer(pending) = app.admit_close(&main)? else {
        return Err("a deferred decision".into());
    };
    assert_eq!(app.runtime().read_signal(&pending)?, None);
    assert_eq!(form.first_text.get()?, "Deferred");
    assert_eq!(app.runtime().focused(), None);
    // The confirmation decides later; admission does not come back to it.
    let signal = decision.borrow().clone().ok_or("no decision signal")?;
    signal.set(Some(WindowCloseAction::KeepOpen))?;
    Driver::new(app.runtime_mut()).settle()?;
    assert_eq!(
        app.runtime().read_signal(&pending)?,
        Some(WindowCloseAction::KeepOpen)
    );
    assert_eq!(app.runtime().focused(), None);
    app.shutdown()?;
    Ok(())
}

#[test]
fn a_defer_signal_of_another_runtime_is_refused_at_once() -> TestResult {
    let other = Rc::new(RefCell::new(Runtime::new()));
    let (mut app, _) = start(
        form,
        Box::new(move |_, _: &Form| {
            let foreign = other.borrow().signal(None);
            Ok(NativeCloseAction::Defer(foreign.read_only()))
        }),
    )?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main),
        Err(AppError::Runtime(RuntimeError::ForeignRuntime))
    ));
    app.shutdown()?;
    Ok(())
}

/// A focusable component whose focus loss fails.
struct FailingFocusLoss;
impl Component for FailingFocusLoss {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn focus_lost(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        Err(ApplicationError::from("focus loss failed").into())
    }
}

#[test]
fn a_failed_focus_loss_aborts_admission_before_the_policy() -> TestResult {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let (mut app, failing) = start(
        |ui| ui.mount_tree(Element::new("failing", FailingFocusLoss)),
        Box::new(move |_, _: &WidgetId| {
            count.set(count.get() + 1);
            Ok(NativeCloseAction::Close)
        }),
    )?;
    app.runtime_mut().focus(&failing)?;
    let main = app.main_window();
    let error = app.admit_close(&main).unwrap_err();
    assert!(
        matches!(&error, AppError::Runtime(RuntimeError::Application(inner)) if inner.to_string() == "focus loss failed"),
        "{error:?}"
    );
    assert_eq!(calls.get(), 0, "the policy did not run");
    // The request is not replayed: the next admission runs the policy.
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));
    assert_eq!(calls.get(), 1);
    app.shutdown()?;
    Ok(())
}

#[test]
fn admission_refuses_a_window_that_is_not_open() -> TestResult {
    let (mut app, windows) = start(
        windows,
        Box::new(|_, _: &Windows| Ok(NativeCloseAction::Close)),
    )?;
    let foreign = Runtime::new().primary_window();
    assert!(matches!(
        app.admit_close(&foreign),
        Err(AppError::Runtime(RuntimeError::ForeignRuntime))
    ));
    app.runtime_mut().redock(&windows.detached)?;
    assert!(matches!(
        app.admit_close(&windows.detached),
        Err(AppError::Runtime(RuntimeError::Unmounted))
    ));
    app.shutdown()?;
    Ok(())
}

#[test]
fn headless_lays_the_content_out_at_the_configured_size_in_the_main_window() -> TestResult {
    let (app, windows) = {
        let slot: Rc<RefCell<Option<Windows>>> = Rc::default();
        let filled = slot.clone();
        let app = App::new("Sized", move |ui| {
            // The initializer leaves the detached window selected.
            *filled.borrow_mut() = Some(windows(ui)?);
            Ok(())
        })
        .size(Size::from([500, 300]))?
        .min_size(Size::from([600, 400]))?
        .headless()?;
        let windows = slot.borrow_mut().take().ok_or("not built")?;
        (app, windows)
    };
    let main = app.main_window();
    assert_eq!(main, app.runtime().primary_window());
    assert_eq!(app.runtime().current_window(), main);
    // Geometry is simulated: no window system applies the minimum.
    let viewport = app.runtime().viewport();
    assert_eq!(
        (viewport.width(), viewport.height(), viewport.scale()),
        (500.0, 300.0, 1.0)
    );
    let bounds = app
        .runtime()
        .bounds(&windows.main_field)?
        .ok_or("not laid out")?;
    assert!(bounds.width() > 0.0 && bounds.height() > 0.0);
    assert_eq!(
        format!("{app:?}"),
        "HeadlessApp { title: \"Sized\", window: MainWindow { size: Size { width: 500.0, height: 300.0 }, min: Some(Size { width: 600.0, height: 400.0 }), max: None, resizable: true }, .. }"
    );
    app.shutdown()?;
    Ok(())
}

#[test]
fn headless_refuses_a_blank_title_and_returns_an_initialization_failure() {
    assert!(matches!(
        App::new(" ", |_| Ok(())).headless(),
        Err(AppError::InvalidTitle)
    ));
    let failed = App::new("Failing", |_| {
        Err(ApplicationError::from("initialization failed").into())
    })
    .headless();
    assert!(
        matches!(&failed, Err(AppError::Runtime(RuntimeError::Application(inner))) if inner.to_string() == "initialization failed"),
        "{failed:?}"
    );
}

#[test]
fn a_failed_start_and_the_shutdown_report_cleanup_failures() -> TestResult {
    // Clearing focus queues the focus loss's error for the next flush, or for
    // shutdown's report.
    fn queue_failure(ui: &mut Runtime) -> Result<(), RuntimeError> {
        let failing = ui.mount_tree(Element::new("failing", FailingFocusLoss))?;
        ui.flush(ui.now())?;
        ui.focus(&failing)?;
        ui.clear_focus();
        Ok(())
    }
    let is_focus_failure = |error: &AppError| matches!(error, AppError::Runtime(RuntimeError::Application(inner)) if inner.to_string() == "focus loss failed");

    // A failed initializer shuts the runtime down, keeping both failures.
    let failed = App::new("Failing", |ui| {
        queue_failure(ui)?;
        Err(ApplicationError::from("initialization failed").into())
    })
    .headless();
    let Err(AppError::Shutdown(shutdown)) = failed else {
        return Err(format!("the cleanup failure is reported: {failed:?}").into());
    };
    assert!(
        matches!(shutdown.failure(), Some(AppError::Runtime(RuntimeError::Application(inner))) if inner.to_string() == "initialization failed")
    );
    assert_eq!(shutdown.errors().len(), 1);

    // The first flush propagates the queued error, which fails the start.
    let failed = App::new("First flush", |ui| Ok(queue_failure(ui)?)).headless();
    assert!(
        matches!(&failed, Err(error) if is_focus_failure(error)),
        "{failed:?}"
    );

    // `shutdown` reports what is queued when the test ends.
    let mut app = App::new("Running", |ui| {
        ui.mount_tree(Element::new("failing", FailingFocusLoss))?;
        Ok(())
    })
    .headless()?;
    app.runtime_mut().focus_next(FocusDirection::Forward)?;
    assert!(app.runtime().focused().is_some());
    app.runtime_mut().clear_focus();
    let Err(AppError::Shutdown(shutdown)) = app.shutdown() else {
        return Err("shutdown reports the queued focus loss".into());
    };
    assert!(shutdown.failure().is_none());
    assert_eq!(shutdown.errors().len(), 1);
    Ok(())
}

#[test]
fn a_hidden_detached_window_is_not_flushed_by_its_admission() -> TestResult {
    struct Hidden {
        failing: WidgetId,
        detached: NativeWindowId,
    }
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let (mut app, hidden) = start(
        |ui| {
            let root = ui.mount_tree(Element::new(
                "root",
                Column::new([Element::new(
                    "panel",
                    View::new(Element::new("failing", FailingFocusLoss)),
                )]),
            ))?;
            ui.flush(Duration::ZERO)?;
            let panel = ui.find(&root, &["panel"])?.ok_or(RuntimeError::Unmounted)?;
            let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
            let detached = ui.commit_detach(prepared)?;
            ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
            let failing = ui
                .find(&root, &["panel", "failing"])?
                .ok_or(RuntimeError::Unmounted)?;
            Ok(Hidden { failing, detached })
        },
        Box::new(move |_, _: &Hidden| {
            count.set(count.get() + 1);
            Ok(NativeCloseAction::Close)
        }),
    )?;
    let ui = app.runtime_mut();
    ui.select_window(&hidden.detached)?;
    ui.flush(ui.now())?;
    ui.focus(&hidden.failing)?;
    // Hiding releases focus; the failed focus loss waits in the hidden window.
    ui.set_window_visible(false)?;
    let main = ui.primary_window();
    ui.select_window(&main)?;
    app.admit_close(&hidden.detached)?;
    assert_eq!(calls.get(), 1, "the policy ran");
    // Shown again, the window's flush reports the waiting error.
    let ui = app.runtime_mut();
    ui.select_window(&hidden.detached)?;
    ui.set_window_visible(true)?;
    let error = ui.flush(ui.now()).unwrap_err();
    assert!(
        matches!(&error, RuntimeError::Application(inner) if inner.to_string() == "focus loss failed"),
        "{error:?}"
    );
    app.shutdown()?;
    Ok(())
}

#[test]
fn keep_open_leaves_focus_cleared_for_an_editor_the_policy_disabled() -> TestResult {
    struct Disabling {
        field: WidgetId,
        text: Signal<String>,
        enabled: Signal<bool>,
    }
    let (mut app, form) = start(
        |ui| {
            let text = ui.signal(String::new());
            let enabled = ui.signal(true);
            let field = ui.mount_tree(Element::new(
                "field",
                TextField::new(text.clone())
                    .max_length(Some(100))
                    .enabled(&enabled),
            ))?;
            Ok(Disabling {
                field,
                text,
                enabled,
            })
        },
        Box::new(|_, form: &Disabling| {
            form.enabled.set(false)?;
            Ok(NativeCloseAction::KeepOpen)
        }),
    )?;
    Driver::new(app.runtime_mut()).replace_text(&form.field, "Saved first")?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(form.text.get()?, "Saved first");
    assert_eq!(app.runtime().focused(), None);
    assert!(!app.runtime().can_focus(&form.field)?);
    app.shutdown()?;
    Ok(())
}

#[test]
fn a_detached_windows_failed_focus_loss_aborts_closing_the_main_window() -> TestResult {
    struct Detached {
        failing: WidgetId,
        detached: NativeWindowId,
    }
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let (mut app, built) = start(
        |ui| {
            let root = ui.mount_tree(Element::new(
                "root",
                Column::new([Element::new(
                    "panel",
                    View::new(Element::new("failing", FailingFocusLoss)),
                )]),
            ))?;
            ui.flush(Duration::ZERO)?;
            let panel = ui.find(&root, &["panel"])?.ok_or(RuntimeError::Unmounted)?;
            let prepared = ui.prepare_detach(&panel, NativeWindowOptions::new("Panel")?)?;
            let detached = ui.commit_detach(prepared)?;
            ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
            let failing = ui
                .find(&root, &["panel", "failing"])?
                .ok_or(RuntimeError::Unmounted)?;
            Ok(Detached { failing, detached })
        },
        Box::new(move |_, _: &Detached| {
            count.set(count.get() + 1);
            Ok(NativeCloseAction::Close)
        }),
    )?;
    let ui = app.runtime_mut();
    ui.select_window(&built.detached)?;
    ui.flush(ui.now())?;
    ui.focus(&built.failing)?;
    let main = ui.primary_window();
    ui.select_window(&main)?;
    let error = app.admit_close(&main).unwrap_err();
    assert!(
        matches!(&error, AppError::Runtime(RuntimeError::Application(inner)) if inner.to_string() == "focus loss failed"),
        "{error:?}"
    );
    assert_eq!(calls.get(), 0, "the policy did not run");
    app.shutdown()?;
    Ok(())
}
