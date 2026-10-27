//! Queries over the mounted interface and the inspection service that carries
//! them from another thread, through the public facade as an application's
//! tooling adapter would use them.
use pert_ui::testing::HostIntegration;
use std::{collections::BTreeMap, sync::mpsc, thread, time::Duration};

use pert_ui::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn ui() -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    Ok(ui)
}

/// A mixer-like tree: a toolbar with named buttons, tabs holding channels,
/// and a field that names its slider.
fn mount_mixer(ui: &mut Runtime) -> TestResult<WidgetId> {
    let root = ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new(
                "toolbar",
                Row::new([
                    Element::new("save", Button::new("Save", |_| Ok(()))),
                    Element::new("undo", Button::new("Undo", |_| Ok(()))),
                ]),
            ),
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new(
                        "channels",
                        "Channels",
                        Element::new(
                            "strip",
                            Column::new([
                                Element::new("mute-1", Checkbox::new(ui.signal(false), "Mute")),
                                Element::new("mute-2", Checkbox::new(ui.signal(true), "Mute")),
                            ]),
                        ),
                    ),
                    Tab::new(
                        "master",
                        "Master",
                        Element::new("label", Label::new("Master")),
                    ),
                ]),
            ),
            Element::new("other", Button::new("Save", |_| Ok(()))),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(root)
}

fn keys(result: &QueryResult) -> Vec<&str> {
    result
        .components
        .iter()
        .map(|component| component.key.as_str())
        .collect()
}

fn find(ui: &Runtime, key: &str) -> u64 {
    ui.query(&InspectionQuery::components(Selector::key(key)))
        .components
        .first()
        .unwrap_or_else(|| panic!("no component {key}"))
        .id
}

#[test]
fn paths_find_a_view_its_children_and_its_descendants() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let exact = ui.query(&InspectionQuery::components(Selector::path([
        "main", "toolbar",
    ])));
    assert_eq!(keys(&exact), ["toolbar"]);
    assert_eq!(exact.matched, 1);
    assert_eq!(
        exact.components[0].path.as_deref(),
        Some(["main".to_owned(), "toolbar".to_owned()].as_slice())
    );
    let children = ui.query(&InspectionQuery::components(Selector::children(["main"])));
    assert_eq!(keys(&children), ["toolbar", "tabs", "other"]);
    // A tab's content is addressed through the tab key, not Tabs' internals,
    // so the strip is a descendant of the tabs but not a child.
    let tab_children = ui.query(&InspectionQuery::components(Selector::children([
        "main", "tabs",
    ])));
    assert!(
        !keys(&tab_children).contains(&"strip"),
        "{:?}",
        keys(&tab_children)
    );
    let descendants = ui.query(&InspectionQuery::components(Selector::descendants([
        "main", "tabs",
    ])));
    assert!(keys(&descendants).contains(&"strip") && keys(&descendants).contains(&"mute-2"));
    // Unknown paths and paths naming structural wrappers match nothing.
    assert!(
        ui.query(&InspectionQuery::components(Selector::path([
            "main", "nope"
        ])))
        .components
        .is_empty()
    );
    Ok(())
}

#[test]
fn ids_keys_focus_windows_and_semantics_select_components() -> TestResult {
    let mut ui = ui()?;
    let root = mount_mixer(&mut ui)?;
    let save = find(&ui, "save");
    assert_eq!(
        keys(&ui.query(&InspectionQuery::components(Selector::id(save)))),
        ["save"]
    );
    assert_eq!(
        keys(&ui.query(&InspectionQuery::components(Selector::key("save")))),
        ["save"]
    );
    let both_saves = ui.query(&InspectionQuery::components(Selector::semantic(
        SemanticRole::Button,
        Some("Save"),
    )));
    assert_eq!(keys(&both_saves), ["save", "other"]);
    let in_toolbar = ui.query(&InspectionQuery::components(Selector::within(
        Selector::path(["main", "toolbar"]),
        Selector::semantic(SemanticRole::Button, Some("Save")),
    )));
    assert_eq!(keys(&in_toolbar), ["save"]);
    let any_button = ui.query(&InspectionQuery::components(Selector::within(
        Selector::key("toolbar"),
        Selector::semantic(SemanticRole::Button, None::<&str>),
    )));
    assert_eq!(keys(&any_button), ["save", "undo"]);
    assert!(
        ui.query(&InspectionQuery::components(Selector::focused()))
            .components
            .is_empty()
    );
    let undo = ui.children(&ui.children(&root)?[0])?[1].clone();
    ui.focus(&undo)?;
    ui.flush(Duration::ZERO)?;
    let focused = ui.query(&InspectionQuery::components(Selector::focused()));
    assert_eq!(keys(&focused), ["undo"]);
    assert!(focused.components[0].focused);
    let window = ui.inspect().windows[0].id;
    let everything = ui.query(&InspectionQuery::components(Selector::window(window)));
    assert_eq!(everything.matched, ui.inspect().components.len());
    assert!(
        ui.query(&InspectionQuery::components(Selector::window(
            window + 1000
        )))
        .components
        .is_empty()
    );
    // A stale incarnation is an empty result, never the replacement mounted
    // under the same key.
    let old = find(&ui, "undo");
    let toolbar = ui.children(&root)?[0].clone();
    ui.unmount(&undo)?;
    ui.reconcile_children(
        &toolbar,
        vec![
            Element::new("save", Button::new("Save", |_| Ok(()))),
            Element::new("undo", Button::new("Undo", |_| Ok(()))),
        ],
    )?;
    ui.flush(Duration::ZERO)?;
    let replacement = find(&ui, "undo");
    assert_ne!(old, replacement);
    assert!(
        ui.query(&InspectionQuery::components(Selector::id(old)))
            .components
            .is_empty()
    );
    assert_eq!(
        keys(&ui.query(&InspectionQuery::components(Selector::id(replacement)))),
        ["undo"]
    );
    Ok(())
}

#[test]
fn depth_detail_and_limit_shape_what_comes_back() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let strip = Selector::path(["main", "tabs", "channels", "strip"]);
    let matches = ui.query(&InspectionQuery::components(strip.clone()));
    assert_eq!(keys(&matches), ["strip"]);
    let children = ui.query(&InspectionQuery::components(strip.clone()).depth(Depth::Children));
    assert_eq!(keys(&children), ["strip", "mute-1", "mute-2"]);
    let subtree =
        ui.query(&InspectionQuery::components(Selector::key("tabs")).depth(Depth::Subtree));
    assert!(keys(&subtree).contains(&"mute-2") && keys(&subtree)[0] == "tabs");

    let identity =
        ui.query(&InspectionQuery::components(Selector::key("save")).detail(Detail::Identity));
    let component = &identity.components[0];
    assert_eq!(component.details, ComponentDetails::default());
    assert!(component.visible);
    let geometry = ui.query(&InspectionQuery::components(Selector::key("save")));
    let component = &geometry.components[0];
    assert!(component.details.box_model.border_box.is_some());
    assert_eq!(component.details.semantics, None);
    let semantics =
        ui.query(&InspectionQuery::components(Selector::key("save")).detail(Detail::Semantics));
    let component = &semantics.components[0];
    let button = component.details.semantics.as_ref().expect("semantics");
    assert_eq!(
        (button.role, button.name.as_deref()),
        (SemanticRole::Button, Some("Save"))
    );
    assert!(component.details.properties.is_empty());
    let full = ui.query(&InspectionQuery::components(Selector::key("save")).detail(Detail::Full));
    let snapshot = ui.inspect();
    let from_snapshot = snapshot
        .components
        .iter()
        .find(|c| c.key == "save")
        .expect("in snapshot");
    assert_eq!(&full.components[0], from_snapshot);

    let limited = ui.query(
        &InspectionQuery::components(strip)
            .depth(Depth::Children)
            .limit(2),
    );
    assert_eq!(keys(&limited), ["strip", "mute-1"]);
    assert_eq!((limited.matched, limited.truncated), (3, true));
    assert!(!children.truncated);
    Ok(())
}

#[test]
fn a_field_names_its_control_in_a_query_too() -> TestResult {
    let mut ui = ui()?;
    let text = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "form",
        Column::new([Element::new(
            "name",
            Field::new(Element::new("field", TextField::new(text))).label("Name"),
        )]),
    ))?;
    ui.flush(Duration::ZERO)?;
    // The name a field gives is part of the semantics, so it is reported at
    // that detail; a geometry-only result leaves `labelled_by` unresolved.
    let named = ui.query(
        &InspectionQuery::components(Selector::semantic(SemanticRole::TextInput, Some("Name")))
            .detail(Detail::Semantics),
    );
    assert_eq!(keys(&named), ["field"]);
    let field = find(&ui, "name");
    assert_eq!(named.components[0].labelled_by, Some(field));
    // Asking for the control alone still resolves the name its field gives,
    // without returning the field.
    let only_control =
        ui.query(&InspectionQuery::components(Selector::key("field")).detail(Detail::Semantics));
    assert_eq!(keys(&only_control), ["field"]);
    assert_eq!(
        only_control.components[0]
            .details
            .semantics
            .as_ref()
            .and_then(|s| s.name.as_deref()),
        Some("Name")
    );
    assert_eq!(only_control.components[0].labelled_by, Some(field));
    let geometry = ui.query(&InspectionQuery::components(Selector::key("field")));
    assert_eq!(geometry.components[0].labelled_by, None);
    Ok(())
}

#[test]
fn sections_are_absent_unless_asked_and_events_follow_a_cursor() -> TestResult {
    let mut ui = ui()?;
    ui.set_debug_mode(DebugMode::Inspect);
    mount_mixer(&mut ui)?;
    let bare = ui.query(&InspectionQuery::new());
    assert!(bare.components.is_empty() && bare.matched == 0);
    assert!(bare.parameters.is_none() && bare.events.is_none() && bare.windows.is_none());
    assert!(bare.sources.is_none() && bare.validation.is_none() && bare.application.is_none());
    assert_eq!(bare.schema_version, InspectionSnapshot::SCHEMA_VERSION);

    let all = ui.query(
        &InspectionQuery::new()
            .events_after(0)
            .windows()
            .sources()
            .validation()
            .parameters(),
    );
    let history = all.events.as_ref().expect("events");
    assert!(!history.events.is_empty());
    assert_eq!(history.generation, ui.inspect().history_generation);
    assert_eq!(all.windows.as_ref().map(Vec::len), Some(1));
    assert_eq!(all.sources.as_ref().map(Vec::len), Some(0));
    assert_eq!(all.parameters.as_ref().map(Vec::len), Some(0));
    assert!(all.validation.is_some());

    let last = history
        .events
        .last()
        .map(|event| event.sequence)
        .unwrap_or_default();
    let nothing_new = ui.query(&InspectionQuery::new().events_after(last));
    assert!(
        nothing_new
            .events
            .as_ref()
            .expect("events")
            .events
            .is_empty()
    );
    ui.key_input(KeyCode::Tab, KeyAction::Press)?;
    ui.key_input(KeyCode::Tab, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    let newer = ui.query(&InspectionQuery::new().events_after(last));
    let newer = newer.events.expect("events");
    assert!(newer.events.iter().all(|event| event.sequence > last) && !newer.events.is_empty());
    assert_eq!(newer.generation, history.generation);
    // A reset is visible even when the reply is otherwise empty.
    ui.clear_event_history();
    let reset = ui
        .query(&InspectionQuery::new().events_after(last))
        .events
        .expect("events");
    assert!(reset.events.is_empty() && reset.dropped_events == 0);
    assert_eq!(reset.generation, history.generation + 1);
    ui.set_debug_mode(DebugMode::Off);
    assert_eq!(ui.inspect().history_generation, history.generation + 2);
    ui.set_debug_mode(DebugMode::Off);
    assert_eq!(ui.inspect().history_generation, history.generation + 2);

    let provided = ui.query_with(&InspectionQuery::new().application(), || {
        BTreeMap::from([("document".to_owned(), InspectValue::Text("x".into()))])
    });
    assert_eq!(provided.application.as_ref().map(BTreeMap::len), Some(1));
    let empty = ui.query(&InspectionQuery::new().application());
    assert_eq!(empty.application, Some(BTreeMap::new()));
    Ok(())
}

#[test]
fn a_tool_thread_asks_and_the_host_answers_after_its_flush() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let (wake, woken) = mpsc::channel();
    ui.set_wake_delivery(move || wake.send(()).is_ok());
    let service = ui.inspection_service(Capacity::new(4)?)?;
    service.set_application(|| BTreeMap::from([("app".to_owned(), InspectValue::Bool(true))]));
    let client = service.client();
    assert_eq!(ui.next_deadline(), None, "idle before any request");

    let tool = thread::spawn(move || -> Result<(QueryResult, InspectionSnapshot), Box<dyn std::error::Error + Send + Sync>> {
        let query = client.query(
            InspectionQuery::components(Selector::path(["main", "toolbar"]))
                .depth(Depth::Children)
                .detail(Detail::Semantics)
                .application(),
        )?;
        let snapshot = client.snapshot()?;
        // The reply comes from the host's flush, which this thread never runs;
        // whether it has arrived yet depends on scheduling, so only wait.
        let query = query.wait()?;
        let snapshot = snapshot.wait_timeout(Duration::from_secs(10))?;
        assert_eq!(query.flush_error, None);
        Ok((query.value, snapshot.value))
    });
    woken.recv_timeout(Duration::from_secs(10))?;
    assert_eq!(
        ui.next_deadline(),
        Some(Duration::ZERO),
        "a request makes the runtime due"
    );
    // The tool may still be queuing its second request; answer whatever is
    // there and keep flushing while it wakes the host again.
    ui.flush(Duration::ZERO)?;
    while woken.recv_timeout(Duration::from_millis(200)).is_ok() || service.pending() > 0 {
        ui.flush(Duration::ZERO)?;
    }
    let (query, snapshot) = tool
        .join()
        .expect("the tool thread finishes")
        .map_err(|e| e.to_string())?;
    assert_eq!(keys(&query), ["toolbar", "save", "undo"]);
    assert_eq!(query.application.as_ref().map(BTreeMap::len), Some(1));
    assert_eq!(snapshot.application.len(), 1);
    assert_eq!(snapshot.components.len(), ui.inspect().components.len());
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

#[test]
fn a_full_service_refuses_without_waking_and_a_closed_one_cancels() -> TestResult {
    let mut ui = ui()?;
    let (wake, woken) = mpsc::channel();
    ui.set_wake_delivery(move || wake.send(()).is_ok());
    let service = ui.inspection_service(Capacity::new(1)?)?;
    let client = service.client();
    let first = client.profile()?;
    assert_eq!(woken.try_recv(), Ok(()));
    assert_eq!(client.validation().map(|_| ()), Err(RequestError::Full));
    assert!(woken.try_recv().is_err(), "a refused request does not wake");
    assert_eq!((service.pending(), client.pending()), (1, 1));
    drop(service);
    assert_eq!(first.wait(), Err(WaitError::Cancelled));
    assert_eq!(client.snapshot().map(|_| ()), Err(RequestError::Closed));
    assert!(client.is_closed());

    let service = ui.inspection_service(Capacity::new(1)?)?;
    let client = service.client();
    let pending = client.workspace()?;
    ui.shutdown();
    assert_eq!(pending.try_reply(), Err(WaitError::Cancelled));
    assert_eq!(client.snapshot().map(|_| ()), Err(RequestError::Closed));
    Ok(())
}

#[test]
fn commands_are_off_by_default_and_apply_before_reads_of_the_same_flush() -> TestResult {
    let mut ui = ui()?;
    ui.set_debug_mode(DebugMode::Inspect);
    mount_mixer(&mut ui)?;
    assert!(!ui.inspect().events.is_empty());
    let service = ui.inspection_service(Capacity::new(8)?)?;
    let client = service.client();
    assert_eq!(
        client
            .command(DiagnosticCommand::ClearEventHistory)
            .map(|_| ()),
        Err(RequestError::CommandsDisabled)
    );
    assert!(!service.commands_allowed());
    service.allow_diagnostic_commands(true);
    // A read queued before the command still sees the command applied.
    let read_first = client.snapshot()?;
    let clear = client.command(DiagnosticCommand::ClearEventHistory)?;
    let overlay = client.command(DiagnosticCommand::SetOverlay(DebugOverlay::Boxes))?;
    let profiling = client.command(DiagnosticCommand::SetProfiling(true))?;
    let read_after = client.query(InspectionQuery::new().events_after(0))?;
    ui.flush(Duration::ZERO)?;
    clear.wait()?;
    overlay.wait()?;
    profiling.wait()?;
    let snapshot = read_first.wait()?.value;
    assert!(snapshot.events.is_empty(), "{}", snapshot.events.len());
    assert_eq!(snapshot.overlay, DebugOverlay::Boxes);
    assert!(
        read_after
            .wait()?
            .value
            .events
            .expect("events")
            .events
            .is_empty()
    );
    assert!(ui.profile().enabled);
    let mode = client.command(DiagnosticCommand::SetDebugMode(DebugMode::Off))?;
    let cleared = client.command(DiagnosticCommand::ClearProfile)?;
    ui.flush(Duration::ZERO)?;
    mode.wait()?;
    cleared.wait()?;
    assert_eq!(ui.inspect().mode, DebugMode::Off);
    Ok(())
}

#[test]
fn a_command_queued_before_commands_are_disabled_is_not_applied() -> TestResult {
    let mut ui = ui()?;
    let service = ui.inspection_service(Capacity::new(8)?)?;
    service.allow_diagnostic_commands(true);
    let client = service.client();
    let overlay = client.command(DiagnosticCommand::SetOverlay(DebugOverlay::Boxes))?;
    let read = client.snapshot()?;
    service.allow_diagnostic_commands(false);
    ui.flush(Duration::ZERO)?;
    assert_eq!(overlay.wait().map(|_| ()), Err(WaitError::Cancelled));
    // Reads in the same batch are still answered, from the unchanged state.
    assert_eq!(read.wait()?.value.overlay, DebugOverlay::Off);
    assert_eq!(ui.inspect().overlay, DebugOverlay::Off);
    Ok(())
}

#[test]
fn a_failing_flush_still_answers_with_its_error() -> TestResult {
    struct Broken;
    impl Component for Broken {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Err(RuntimeError::BorrowConflict)
        }
    }
    let mut ui = ui()?;
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new("ok", Label::new("Fine")),
            Element::new("broken", Broken),
        ]),
    ))?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let pending = client.query(InspectionQuery::components(Selector::key("ok")).validation())?;
    let error = ui
        .flush(Duration::ZERO)
        .expect_err("the broken component fails the flush");
    let reply = pending.wait()?;
    assert_eq!(reply.flush_error.as_ref(), Some(&error));
    assert_eq!(keys(&reply.value), ["ok"]);
    assert!(reply.value.validation.is_some());
    // The next request is answered by the next flush, failing or not.
    let again = client.snapshot()?;
    assert!(ui.flush(Duration::ZERO).is_err());
    assert!(again.wait()?.flush_error.is_some());
    Ok(())
}

#[test]
fn requests_queued_while_answering_wait_for_the_next_flush() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let service = ui.inspection_service(Capacity::new(8)?)?;
    let client = service.client();
    let queued = client.snapshot()?;
    // A provider that enqueues from the UI thread while answering: what it
    // adds is not answered by this flush.
    let late = std::sync::Mutex::new(None);
    let late_client = client.clone();
    let late_ref: &'static std::sync::Mutex<Option<Pending<ProfileSnapshot>>> =
        Box::leak(Box::new(late));
    service.set_application(move || {
        if let Ok(mut slot) = late_ref.lock()
            && slot.is_none()
        {
            *slot = late_client.profile().ok();
        }
        BTreeMap::new()
    });
    ui.flush(Duration::ZERO)?;
    queued.wait()?;
    let late = late_ref
        .lock()
        .unwrap()
        .take()
        .expect("queued while answering");
    assert_eq!(late.try_reply(), Err(WaitError::Timeout));
    assert_eq!(service.pending(), 1);
    ui.flush(Duration::ZERO)?;
    late.wait()?;
    assert_eq!(service.pending(), 0);
    Ok(())
}

#[test]
fn a_reply_is_handed_out_once_and_a_huge_timeout_is_fine() -> TestResult {
    let mut ui = ui()?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let pending = client.profile()?;
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Timeout));
    ui.flush(Duration::ZERO)?;
    assert!(pending.wait_timeout(Duration::MAX).is_ok());
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Taken));
    assert_eq!(
        pending.wait_timeout(Duration::MAX).map(|_| ()),
        Err(WaitError::Taken)
    );
    assert_eq!(pending.wait().map(|_| ()), Err(WaitError::Taken));
    // Cancellation is also terminal and reported as such after the fact.
    let pending = client.profile()?;
    drop(service);
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Cancelled));
    assert_eq!(pending.wait().map(|_| ()), Err(WaitError::Cancelled));
    Ok(())
}

#[test]
fn a_blocked_waiter_is_released_when_the_service_drops() -> TestResult {
    let ui = ui()?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let (started, wait_started) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let pending = client.snapshot().expect("accepted");
        started.send(()).expect("the test waits");
        pending.wait()
    });
    wait_started.recv_timeout(Duration::from_secs(10))?;
    // Give the waiter a moment to block; a spin is not needed for correctness.
    thread::sleep(Duration::from_millis(20));
    drop(service);
    assert_eq!(
        waiter.join().expect("the waiter finishes").map(|_| ()),
        Err(WaitError::Cancelled)
    );
    Ok(())
}

#[test]
fn commands_of_every_service_apply_before_reads_of_any() -> TestResult {
    let mut ui = ui()?;
    ui.set_debug_mode(DebugMode::Inspect);
    mount_mixer(&mut ui)?;
    let first = ui.inspection_service(Capacity::new(4)?)?;
    let second = ui.inspection_service(Capacity::new(4)?)?;
    second.allow_diagnostic_commands(true);
    let read = first.client().snapshot()?;
    let clear = second
        .client()
        .command(DiagnosticCommand::ClearEventHistory)?;
    ui.flush(Duration::ZERO)?;
    clear.wait()?;
    assert!(read.wait()?.value.events.is_empty());
    // A request one service's provider makes at another waits for the next flush.
    let late = std::sync::Arc::new(std::sync::Mutex::new(None));
    let other = second.client();
    let slot = late.clone();
    first.set_application(move || {
        if let Ok(mut slot) = slot.lock()
            && slot.is_none()
        {
            *slot = other.profile().ok();
        }
        BTreeMap::new()
    });
    let read = first.client().query(InspectionQuery::new().application())?;
    ui.flush(Duration::ZERO)?;
    read.wait()?;
    let late = late.lock().unwrap().take().expect("queued while answering");
    assert_eq!(late.try_reply().map(|_| ()), Err(WaitError::Timeout));
    ui.flush(Duration::ZERO)?;
    late.wait()?;
    Ok(())
}

#[test]
fn the_flush_report_and_a_hidden_application_still_settle() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let pending = client.profile()?;
    assert_eq!(ui.next_deadline(), Some(Duration::ZERO));
    let report = ui.flush(Duration::ZERO)?;
    assert_eq!(
        report.next_deadline, None,
        "the request it counted is answered"
    );
    pending.wait()?;
    // Every root hidden: flush_windows flushes no window but still answers.
    ui.set_window_visible(false)?;
    let pending = client.snapshot()?;
    assert!(ui.application_deadline().is_some());
    let report = ui.flush_windows(Duration::ZERO)?;
    assert_eq!(report.components_updated, 0);
    assert!(!pending.wait()?.value.windows[0].presentable);
    assert_eq!(service.pending(), 0);
    Ok(())
}

#[test]
fn a_host_that_withholds_flushes_answers_from_the_last_one() -> TestResult {
    let mut ui = ui()?;
    mount_mixer(&mut ui)?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let before = ui.inspect().at;
    let pending = client.snapshot()?;
    ui.service_inspection()?;
    let reply = pending.wait()?;
    assert_eq!(reply.value.at, before);
    assert_eq!(reply.flush_error, None);
    // The request's wake still asks for one flush, which finds nothing to do.
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.next_deadline(), None);
    Ok(())
}

#[test]
fn a_caught_panic_does_not_silence_later_flushes() -> TestResult {
    struct Panics(std::cell::Cell<bool>);
    impl Component for Panics {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            if self.0.replace(false) {
                panic!("once");
            }
            Ok(())
        }
    }
    let mut ui = ui()?;
    ui.mount_tree(Element::new("panics", Panics(std::cell::Cell::new(true))))?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let pending = client.profile()?;
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ui.flush_windows(Duration::ZERO);
    }));
    assert!(unwound.is_err(), "the component panicked");
    // The boundary was released on unwinding: the next flush answers.
    let _ = ui.flush_windows(Duration::ZERO);
    let _ = ui.flush(Duration::ZERO);
    assert_eq!(service.pending(), 0);
    assert!(matches!(
        pending.wait().map(|_| ()),
        Ok(()) | Err(WaitError::Cancelled)
    ));
    let again = client.profile()?;
    let _ = ui.flush(Duration::ZERO);
    assert!(again.wait().is_ok());
    Ok(())
}

#[test]
fn a_failed_flush_reply_keeps_the_application_cause_on_a_worker() -> TestResult {
    #[derive(Debug, thiserror::Error)]
    #[error("host configuration is invalid")]
    struct HostFailure;
    struct Broken(RuntimeError);
    impl Component for Broken {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Err(self.0.clone())
        }
    }
    for application_flush in [false, true] {
        let cause = RuntimeError::Application(ApplicationError::new(HostFailure));
        let mut ui = ui()?;
        ui.mount_tree(Element::new("broken", Broken(cause.clone())))?;
        let service = ui.inspection_service(Capacity::new(4)?)?;
        let pending = service.client().snapshot()?;
        let worker = thread::spawn(move || pending.wait());
        let error = if application_flush {
            ui.flush_windows(Duration::ZERO)
                .expect_err("visible root failed")
        } else {
            ui.flush(Duration::ZERO).expect_err("root failed")
        };
        assert_eq!(error, cause);
        let reply = worker.join().expect("worker finishes")?;
        assert_eq!(reply.flush_error, Some(cause));
        let Some(RuntimeError::Application(error)) = reply.flush_error else {
            panic!("owned application failure")
        };
        assert!(error.downcast_ref::<HostFailure>().is_some());
    }
    Ok(())
}
