//! Close admission through `App::headless` and `HeadlessApp::admit_close`:
//! focused drafts are released before `App::on_close` reads the
//! application's state, focus comes back only after a synchronous
//! `KeepOpen`, and admission's errors come back directly.
#![cfg(feature = "native")]

use pert_ui::testing::*;
use pert_ui::{InspectValue, NativeWindowOptions, ParameterId, ParameterSpec};
use pert_ui::{component::*, parameter::*, prelude::*, windowing::*};
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

/// A page whose editors feed a `FormFields` from `on_modified`, for a close
/// policy that keeps the window open while any field is unsaved.
struct Unsaved {
    form: FormFields,
    root: WidgetId,
    count: Signal<i64>,
    enabled: Signal<bool>,
    name: Signal<String>,
}
fn unsaved_page(ui: &mut Runtime) -> Result<Unsaved, RuntimeError> {
    let form = FormFields::new(ui);
    let count = ui.signal(7_i64);
    let enabled = ui.signal(true);
    let name = ui.signal("Bass".to_owned());
    let (count_field, name_field) = (form.field("count")?, form.field("name")?);
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([
            IntegerInput::new(count.clone())
                .enabled(&enabled)
                .on_modified(move |_, modified| count_field.set_modified(modified).map(|_| ()))
                .key("count"),
            InlineText::new(name.clone())
                .normalize(|text: String| {
                    if text.trim().is_empty() {
                        Err(InputRejection::new("Enter a name"))
                    } else {
                        Ok(text)
                    }
                })
                .on_modified(move |_, modified| name_field.set_modified(modified).map(|_| ()))
                .key("name"),
            Button::new("Other", |_| Ok(())).key("other"),
        ]),
    ))?;
    Ok(Unsaved {
        form,
        root,
        count,
        enabled,
        name,
    })
}
/// Keeps the window open while any field reports an unsaved draft.
fn while_unsaved() -> Policy<Unsaved> {
    Box::new(|_, page: &Unsaved| {
        Ok(if page.form.summary().get()?.modified() > 0 {
            NativeCloseAction::KeepOpen
        } else {
            NativeCloseAction::Close
        })
    })
}
fn text_property(ui: &Runtime, id: &WidgetId, name: &str) -> Option<String> {
    let inspection = ui.inspection_id(id).ok()?;
    match ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == inspection)?
        .details
        .properties
        .get(name)
    {
        Some(InspectValue::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

/// New-behavior test 19: the close policy sees a disabled field's kept
/// draft. Named assertion: `KeepOpen`, and the field still shows the draft.
/// Mutation checked: test 1's (the draft discarded on revocation).
#[test]
fn the_close_policy_sees_a_disabled_fields_kept_draft() -> TestResult {
    let (mut app, page) = start(unsaved_page, while_unsaved())?;
    let field = app.runtime().find(&page.root, &["count"])?.ok_or("count")?;
    Driver::new(app.runtime_mut()).replace_text(&field, "42")?;
    page.enabled.set(false)?;
    Driver::new(app.runtime_mut()).settle()?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(
        text_property(app.runtime(), &field, "text").as_deref(),
        Some("42")
    );
    assert_eq!(page.count.get()?, 7);
    app.shutdown()?;
    Ok(())
}

/// New-behavior test 19, test 2's sequence: an unfocused refused draft, a
/// pointer move over it and a bound revocation before the close. Named
/// assertion: `KeepOpen`. Mutation checked: test 2's.
#[test]
fn the_close_policy_sees_an_unfocused_refused_draft_after_revocation() -> TestResult {
    let (mut app, page) = start(unsaved_page, while_unsaved())?;
    let field = app.runtime().find(&page.root, &["count"])?.ok_or("count")?;
    let other = app.runtime().find(&page.root, &["other"])?.ok_or("other")?;
    Driver::new(app.runtime_mut()).replace_text(&field, "4x")?;
    app.runtime_mut().focus(&other)?;
    let [x, y, w, h] = app.runtime().bounds(&field)?.ok_or("bounds")?.values();
    let point = Point::new(x + w / 2.0, y + h / 2.0)?;
    app.runtime_mut()
        .pointer_input(PointerEvent::new(point, PointerKind::Move))?;
    page.enabled.set(false)?;
    Driver::new(app.runtime_mut()).settle()?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(
        text_property(app.runtime(), &field, "text").as_deref(),
        Some("4x")
    );
    app.shutdown()?;
    Ok(())
}

/// New-behavior test 19: focused editors with refused drafts. Named
/// assertion: `KeepOpen`, and the released editor (for `InlineText` its
/// child editor) is focused again. Mutation checked: removing the delivery
/// request of `on_modified` lets the window close.
#[test]
fn the_close_policy_sees_focused_refused_drafts() -> TestResult {
    // An integer field with an invalid draft.
    let (mut app, page) = start(unsaved_page, while_unsaved())?;
    let field = app.runtime().find(&page.root, &["count"])?.ok_or("count")?;
    Driver::new(app.runtime_mut()).replace_text(&field, "4x")?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(app.runtime().focused(), Some(field));
    app.shutdown()?;

    // An editable label whose normalizer refuses the draft.
    let (mut app, page) = start(unsaved_page, while_unsaved())?;
    let label = app.runtime().find(&page.root, &["name"])?.ok_or("name")?;
    app.runtime_mut()
        .text_command(&label, TextCommand::BeginEdit)?;
    Driver::new(app.runtime_mut()).settle()?;
    Driver::new(app.runtime_mut()).type_text(" ")?;
    let editor = app.runtime().focused().ok_or("the label's editor")?;
    let main = app.main_window();
    assert!(matches!(
        app.admit_close(&main)?,
        NativeCloseAction::KeepOpen
    ));
    assert_eq!(page.name.get()?, "Bass");
    assert_eq!(app.runtime().focused(), Some(editor));
    app.shutdown()?;
    Ok(())
}

/// A table whose quantity cells edit with a `DecimalInput` that feeds the
/// edited cell's modified count.
struct Ledger {
    form: FormFields,
    root: WidgetId,
    state: TableController,
    amounts: Vec<Signal<f64>>,
}
struct LedgerRows(Vec<Signal<f64>>);
impl TableItems for LedgerRows {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0.len()).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        let amount = self.0.get(index)?.get().ok()?;
        Some(Element::new("amount", Label::new(amount.to_string())))
    }
}
fn ledger(ui: &mut Runtime) -> Result<Ledger, RuntimeError> {
    let form = FormFields::new(ui);
    let amounts = vec![ui.signal(1.0_f64), ui.signal(2.0_f64)];
    let state = TableController::new(
        ui,
        ListSelection::new(ui, SelectionMode::Single),
        [TableColumn::new("amount", "Amount").editable(true)],
        TableSource::new(LedgerRows(amounts.clone())),
    )?;
    let handles: Vec<FieldHandle> = (0..amounts.len())
        .map(|index| form.field(format!("r{index}")))
        .collect::<Result<_, _>>()?;
    let rows = amounts.clone();
    let table = Table::new(&state, 24.0)?.cell_editor(move |edit| {
        let index: usize = edit.row().trim_start_matches('r').parse().unwrap_or(0);
        let handle = handles[index].clone();
        Ok(Element::new(
            "editor",
            DecimalInput::new(rows[index].clone())
                .step(0.5)
                .map_err(|_| RuntimeError::Exhausted)?
                .stepper(true)
                .on_modified(move |_, modified| handle.set_modified(modified).map(|_| ()))
                .session(edit.session()),
        ))
    });
    let root = ui.mount_tree(Element::new(
        "page",
        Column::new([Element::new("table", table).with_grow(1)]),
    ))?;
    Ok(Ledger {
        form,
        root,
        state,
        amounts,
    })
}

/// New-behavior test 19: a table's cell with a valid typed draft, and with a
/// held step button, at close. Named assertions: the model holds the value
/// when the policy runs, the table has closed the edit, the count is zero
/// and the decision is `Close`. Mutations checked: test 6's for the held
/// press, and requesting the modified report after the session report (the
/// count stays one and the decision is `KeepOpen`).
#[test]
fn a_table_cell_saved_by_admission_leaves_nothing_unsaved() -> TestResult {
    for held in [false, true] {
        let seen = Rc::new(RefCell::new(None));
        let record = seen.clone();
        let (mut app, page) = start(
            ledger,
            Box::new(move |ui, page: &Ledger| {
                let open = ui.find_descendant(&page.root, "editor")?.is_some();
                *record.borrow_mut() = Some((page.amounts[0].get()?, open));
                Ok(if page.form.summary().get()?.modified() > 0 {
                    NativeCloseAction::KeepOpen
                } else {
                    NativeCloseAction::Close
                })
            }),
        )?;
        let ui = app.runtime_mut();
        ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
        Driver::new(ui).settle()?;
        page.state.selection().set_active(Some("r0".into()))?;
        page.state.set_active_column(Some("amount".into()))?;
        let body = ui.find_descendant(&page.root, "body")?.ok_or("body")?;
        ui.focus(&body)?;
        Driver::new(ui).key(KeyCode::Enter)?;
        let editor = ui.find_descendant(&page.root, "editor")?.ok_or("editor")?;
        ui.text_command(&editor, TextCommand::SelectAll)?;
        Driver::new(ui).type_text("3.5")?;
        if held {
            let [x, y, w, h] = ui.bounds(&editor)?.ok_or("bounds")?.values();
            let metrics = ui.theme().metrics(pert_ui::Density::Standard)?;
            let plus = Point::new(
                x + w
                    - metrics.metric(MetricRole::ControlPaddingX)
                    - metrics.metric(MetricRole::IconSize) / 2.0,
                y + h / 2.0,
            )?;
            ui.pointer_input(PointerEvent::new(
                plus,
                PointerKind::Press(PointerButton::Primary),
            ))?;
            Driver::new(ui).settle()?;
        }
        assert_eq!(page.form.summary().get()?.modified(), 1, "held {held}");
        let main = app.main_window();
        let decision = app.admit_close(&main)?;
        let expected = if held { 4.0 } else { 3.5 };
        assert_eq!(*seen.borrow(), Some((expected, false)), "held {held}");
        assert_eq!(page.form.summary().get()?.modified(), 0, "held {held}");
        assert!(matches!(decision, NativeCloseAction::Close), "held {held}");
        app.shutdown()?;
    }
    Ok(())
}

/// Close admission and focused in-place and search editors (contract).
#[test]
fn admission_applies_the_focus_loss_rule_of_focused_in_place_editors() -> TestResult {
    struct Page {
        root: WidgetId,
        name: Signal<String>,
        model: Rc<RefCell<String>>,
        count: Signal<i64>,
        gain: Parameter<f32>,
        _host: ParameterHost<f32>,
        outcomes: Rc<RefCell<Vec<String>>>,
        _monitor: DeliveryMonitor<f32>,
        query: Signal<String>,
        searchable: Signal<bool>,
    }
    fn page(ui: &mut Runtime) -> Result<Page, RuntimeError> {
        let name = ui.signal("Bass".to_owned());
        let model = Rc::new(RefCell::new("Bass".to_owned()));
        let count = ui.signal(8_i64);
        let (gain, host) = ui.parameter(
            ParameterId::new(1, 1, "gain")?,
            ParameterSpec::new("Gain", 0.0, 1.0, 0.25)?,
        )?;
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let sink = outcomes.clone();
        let monitor = host.on_outcome(move |event| {
            sink.borrow_mut().push(format!("{:?}", event.outcome));
            Ok(())
        })?;
        let query = ui.signal(String::new());
        let searchable = ui.signal(true);
        let store = model.clone();
        let root = ui.mount_tree(Element::new(
            "page",
            Column::new([
                InlineText::new(name.clone()).key("bound"),
                InlineText::controlled("Bass", move |_, text| {
                    text.clone_into(&mut store.borrow_mut());
                    Ok(())
                })
                .key("controlled"),
                InlineInteger::new(count.clone()).key("count"),
                InlineSlider::new(gain.clone()).key("gain"),
                SearchField::new(query.clone())
                    .debounce(Duration::from_millis(300))
                    .editable(&searchable)
                    .key("search"),
            ]),
        ))?;
        Ok(Page {
            root,
            name,
            model,
            count,
            gain,
            _host: host,
            outcomes,
            _monitor: monitor,
            query,
            searchable,
        })
    }
    type Check = fn(&Page) -> Result<(), Box<dyn std::error::Error>>;
    let cases: [(&str, &str, Check); 5] = [
        ("bound", "Lead", |p| {
            assert_eq!(p.name.get()?, "Lead");
            Ok(())
        }),
        ("controlled", "Lead", |p| {
            assert_eq!(*p.model.borrow(), "Lead", "the proposal reached the model");
            Ok(())
        }),
        ("count", "42", |p| {
            assert_eq!(p.count.get()?, 42);
            Ok(())
        }),
        ("gain", "0.75", |p| {
            assert_eq!(
                p.gain.snapshot().get()?.base,
                0.25,
                "cancelled, not committed"
            );
            assert_eq!(*p.outcomes.borrow(), ["Cancelled"]);
            Ok(())
        }),
        ("search", "abc", |p| {
            assert_eq!(p.query.get()?, "abc", "the pending query is written");
            Ok(())
        }),
    ];
    for (key, draft, check) in cases {
        let checked = Rc::new(RefCell::new(None));
        let record = checked.clone();
        let (mut app, page) = start(
            page,
            Box::new(move |_, page: &Page| {
                *record.borrow_mut() = Some(check(page).map_err(|error| error.to_string()));
                Ok(NativeCloseAction::Close)
            }),
        )?;
        let ui = app.runtime_mut();
        let id = ui.find(&page.root, &[key])?.ok_or("the editor")?;
        ui.focus(&id)?;
        Driver::new(ui).settle()?;
        if key != "search" {
            ui.text_command(&id, TextCommand::BeginEdit)?;
            Driver::new(ui).settle()?;
        }
        Driver::new(ui).type_text(draft)?;
        let main = app.main_window();
        app.admit_close(&main)?;
        assert_eq!(*checked.borrow(), Some(Ok(())), "{key}");
        app.shutdown()?;
    }

    // A read-only search field discards its pending query.
    let (mut app, page) = start(page, Box::new(|_, _: &Page| Ok(NativeCloseAction::Close)))?;
    let ui = app.runtime_mut();
    let search = ui.find(&page.root, &["search"])?.ok_or("search")?;
    ui.focus(&search)?;
    Driver::new(ui).type_text("abc")?;
    page.searchable.set(false)?;
    Driver::new(ui).settle()?;
    let main = app.main_window();
    app.admit_close(&main)?;
    assert_eq!(page.query.get()?, "");
    assert_eq!(
        text_property(app.runtime(), &search, "text").as_deref(),
        Some("")
    );
    app.shutdown()?;
    Ok(())
}

/// Close admission and a table's session editor (contract): a valid draft
/// reports `(FocusLeft, Saved)` and the table closes the edit within
/// admission; a refused one stays open and `KeepOpen` focuses it again.
#[test]
fn admission_completes_a_table_cells_session() -> TestResult {
    struct Names {
        root: WidgetId,
        state: TableController,
        names: Vec<Signal<String>>,
        ends: Rc<RefCell<Vec<String>>>,
    }
    struct NameRows(Vec<Signal<String>>);
    impl TableItems for NameRows {
        fn len(&self) -> usize {
            self.0.len()
        }
        fn key(&self, index: usize) -> Option<String> {
            (index < self.0.len()).then(|| format!("r{index}"))
        }
        fn cell(&self, index: usize, _: &str) -> Option<Element> {
            Some(Element::new(
                "name",
                Label::new(self.0.get(index)?.get().ok()?),
            ))
        }
    }
    fn names(ui: &mut Runtime) -> Result<Names, RuntimeError> {
        let names = vec![ui.signal("Cable".to_owned()), ui.signal("Mouse".to_owned())];
        let state = TableController::new(
            ui,
            ListSelection::new(ui, SelectionMode::Single),
            [TableColumn::new("name", "Name").editable(true)],
            TableSource::new(NameRows(names.clone())),
        )?;
        let ends = Rc::new(RefCell::new(Vec::new()));
        let (rows, sink) = (names.clone(), ends.clone());
        let table = Table::new(&state, 24.0)?
            .cell_editor(move |edit| {
                let index: usize = edit.row().trim_start_matches('r').parse().unwrap_or(0);
                Ok(Element::new(
                    "editor",
                    TextField::new(rows[index].clone())
                        .normalize(|text: String| {
                            if text.trim().is_empty() {
                                Err(InputRejection::new("A name is required"))
                            } else {
                                Ok(text)
                            }
                        })
                        .session(edit.session()),
                ))
            })
            .on_edit_end(move |_, end| {
                sink.borrow_mut().push(format!("{:?}", end.reason()));
                Ok(())
            });
        let root = ui.mount_tree(Element::new(
            "page",
            Column::new([Element::new("table", table).with_grow(1)]),
        ))?;
        Ok(Names {
            root,
            state,
            names,
            ends,
        })
    }
    for refused in [false, true] {
        let (mut app, page) = start(
            names,
            Box::new(|_, _: &Names| Ok(NativeCloseAction::KeepOpen)),
        )?;
        let ui = app.runtime_mut();
        ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
        Driver::new(ui).settle()?;
        page.state.selection().set_active(Some("r0".into()))?;
        page.state.set_active_column(Some("name".into()))?;
        let body = ui.find_descendant(&page.root, "body")?.ok_or("body")?;
        ui.focus(&body)?;
        Driver::new(ui).key(KeyCode::Enter)?;
        let editor = ui.find_descendant(&page.root, "editor")?.ok_or("editor")?;
        if refused {
            Driver::new(ui).key(KeyCode::Backspace)?;
        } else {
            Driver::new(ui).type_text("Laptop")?;
        }
        let main = app.main_window();
        app.admit_close(&main)?;
        let ui = app.runtime();
        if refused {
            assert!(page.ends.borrow().is_empty(), "the refused edit stays open");
            assert_eq!(ui.focused(), Some(editor), "KeepOpen focuses it again");
            assert_eq!(page.names[0].get()?, "Cable");
        } else {
            assert_eq!(
                *page.ends.borrow(),
                ["Completed(EditorCompletion { trigger: FocusLeft, outcome: Saved })"]
            );
            assert_eq!(page.names[0].get()?, "Laptop");
            assert!(ui.find_descendant(&page.root, "editor")?.is_none());
        }
        app.shutdown()?;
    }
    Ok(())
}
