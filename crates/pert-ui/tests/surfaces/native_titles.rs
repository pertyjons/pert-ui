//! Native window titles through the public facade: a detached tab's window is
//! retitled when the tab's title changes, queued retitles for one window
//! coalesce, and `EventContext::set_window_title` validates its request.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

/// Flushes and paints the selected window, as a host frame does.
fn flush(ui: &mut Runtime) {
    ui.flush(ui.now()).unwrap();
    ui.paint().unwrap();
}

/// Flushes every listed window, then the primary one.
fn flush_all(ui: &mut Runtime, windows: &[NativeWindowId]) {
    let primary = ui.primary_window();
    for window in windows {
        if ui.select_window(window).is_ok() {
            flush(ui);
        }
    }
    ui.select_window(&primary).unwrap();
    flush(ui);
}

fn retitles(requests: &[PresentationRequest], window: &NativeWindowId) -> Vec<String> {
    requests
        .iter()
        .filter_map(|request| match request.action() {
            PresentationAction::Retitle { window: w, title } if w == window => Some(title.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_detached_tab_retitles_its_window_when_its_title_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let title = ui.signal("Mix".to_owned());
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "mix",
                title.read_only(),
                Element::new("mix-body", Label::new("Mixer")),
            ),
            Tab::new(
                "edit",
                "Edit",
                Element::new("edit-body", Label::new("Editor")),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let views = ui.children(&root)?;
    let prepared = ui.prepare_detach(&views[0], NativeWindowOptions::new("Mix")?)?;
    let window = ui.commit_detach(prepared)?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(retitles(&ui.take_presentation_requests()?, &window).is_empty());

    title.set("Mix *".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    let requests = ui.take_presentation_requests()?;
    assert_eq!(retitles(&requests, &window), ["Mix *"]);
    drop(requests);

    // Once the view is back, its window needs no title.
    ui.redock(&window)?;
    let primary = ui.primary_window();
    ui.select_window(&primary)?;
    title.set("Mixer".to_owned())?;
    flush(&mut ui);
    assert!(retitles(&ui.take_presentation_requests()?, &window).is_empty());
    Ok(())
}

/// Calls `set_window_title` from its delivery for each queued title, on the
/// given window or its own, and records the results.
struct Retitler {
    titles: Vec<(Option<NativeWindowId>, String)>,
    results: Rc<RefCell<Vec<Result<(), RuntimeError>>>>,
}
impl Component for Retitler {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !self.titles.is_empty() {
            cx.notifier().request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        for (window, title) in std::mem::take(&mut self.titles) {
            let window = window.unwrap_or_else(|| cx.native_window());
            let result = cx.set_window_title(&window, title).map(|_| ());
            self.results.borrow_mut().push(result);
        }
        Ok(())
    }
}

#[test]
fn set_window_title_validates_and_coalesces() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let elsewhere = Runtime::new();
    let results = Rc::new(RefCell::new(Vec::new()));
    ui.mount_tree(Element::new(
        "retitler",
        Retitler {
            titles: vec![
                (None, " ".to_owned()),
                (Some(elsewhere.primary_window()), "Other".to_owned()),
                (None, "First".to_owned()),
                (None, "Second".to_owned()),
            ],
            results: results.clone(),
        },
    ))?;
    flush(&mut ui);
    assert_eq!(
        *results.borrow(),
        [
            Err(RuntimeError::EmptyWindowTitle),
            Err(RuntimeError::Unmounted),
            Ok(()),
            Ok(())
        ]
    );
    let primary = ui.primary_window();
    let requests = ui.take_presentation_requests()?;
    assert_eq!(requests.len(), 1);
    assert_eq!(
        retitles(&requests, &primary),
        ["Second"],
        "one request, latest title"
    );
    Ok(())
}

#[test]
fn application_code_requests_a_window_title_with_the_same_rules()
-> Result<(), Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    let primary = ui.primary_window();
    assert_eq!(
        ui.request_window_title(&primary, "  ").err(),
        Some(RuntimeError::EmptyWindowTitle)
    );
    assert_eq!(
        ui.request_window_title(&Runtime::new().primary_window(), "Other")
            .err(),
        Some(RuntimeError::Unmounted)
    );
    let first = ui.request_window_title(&primary, "Song.pert")?;
    let second = ui.request_window_title(&primary, "Song.pert *")?;
    let requests = ui.take_presentation_requests()?;
    assert_eq!(retitles(&requests, &primary), ["Song.pert *"]);
    for request in requests {
        request.complete(PresentationStatus::Presented(primary.clone()))?;
    }
    // Both receipts observe the one request.
    assert_eq!(
        first.status(),
        PresentationStatus::Presented(primary.clone())
    );
    assert_eq!(second.status(), PresentationStatus::Presented(primary));
    Ok(())
}

#[test]
fn a_tab_detached_with_another_title_takes_its_tab_title() -> Result<(), Box<dyn std::error::Error>>
{
    let mut ui = Runtime::new();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new("mix", "Mix", Element::new("mix-body", Label::new("Mixer"))),
            Tab::new(
                "edit",
                "Edit",
                Element::new("edit-body", Label::new("Editor")),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let views = ui.children(&root)?;
    let prepared = ui.prepare_detach(&views[0], NativeWindowOptions::new("Session")?)?;
    let window = ui.commit_detach(prepared)?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert_eq!(
        retitles(&ui.take_presentation_requests()?, &window),
        ["Mix"]
    );
    Ok(())
}

#[test]
fn a_retitle_the_host_dropped_is_asked_again() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let title = ui.signal("Mix".to_owned());
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "mix",
                title.read_only(),
                Element::new("mix-body", Label::new("Mixer")),
            ),
            Tab::new(
                "edit",
                "Edit",
                Element::new("edit-body", Label::new("Editor")),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let views = ui.children(&root)?;
    let prepared = ui.prepare_detach(&views[0], NativeWindowOptions::new("Mix")?)?;
    let window = ui.commit_detach(prepared)?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    title.set("Mix *".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    // The host takes the request and drops it unfinished.
    let dropped = ui.take_presentation_requests()?;
    assert_eq!(retitles(&dropped, &window), ["Mix *"]);
    drop(dropped);
    flush_all(&mut ui, std::slice::from_ref(&window));
    let requests = ui.take_presentation_requests()?;
    assert_eq!(retitles(&requests, &window), ["Mix *"], "asked again");
    for request in requests {
        request.complete(PresentationStatus::Presented(window.clone()))?;
    }
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(
        retitles(&ui.take_presentation_requests()?, &window).is_empty(),
        "a completed retitle is not repeated"
    );
    Ok(())
}

/// A tab strip with a bound first title, its first tab detached into a
/// window opened with `opened`.
fn detached_tab(
    ui: &mut Runtime,
    title: &Signal<String>,
    opened: &str,
) -> Result<NativeWindowId, Box<dyn std::error::Error>> {
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "mix",
                title.read_only(),
                Element::new("mix-body", Label::new("Mixer")),
            ),
            Tab::new(
                "edit",
                "Edit",
                Element::new("edit-body", Label::new("Editor")),
            ),
        ]),
    ))?;
    ui.flush(ui.now())?;
    let views = ui.children(&root)?;
    let prepared = ui.prepare_detach(&views[0], NativeWindowOptions::new(opened)?)?;
    let window = ui.commit_detach(prepared)?;
    flush_all(ui, std::slice::from_ref(&window));
    Ok(window)
}

#[test]
fn a_refused_retitle_waits_for_the_next_title_change() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    let title = ui.signal("Mix".to_owned());
    let window = detached_tab(&mut ui, &title, "Mix")?;
    title.set("Mix *".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    for request in ui.take_presentation_requests()? {
        request.complete(PresentationStatus::Failed(WindowError::Surface(
            "refused".into(),
        )))?;
    }
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(
        retitles(&ui.take_presentation_requests()?, &window).is_empty(),
        "the refused title is not asked for again"
    );
    title.set("Mix **".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert_eq!(
        retitles(&ui.take_presentation_requests()?, &window),
        ["Mix **"]
    );
    Ok(())
}

#[test]
fn a_retitle_refused_by_a_full_queue_is_asked_again_later() -> Result<(), Box<dyn std::error::Error>>
{
    let mut ui = Runtime::new();
    let title = ui.signal("Mix".to_owned());
    let window = detached_tab(&mut ui, &title, "Mix")?;
    let primary = ui.primary_window();
    let mut receipts = Vec::new();
    while receipts.len() < Runtime::MAX_WINDOW_REQUESTS {
        receipts.push(ui.request_close(&primary)?);
    }
    title.set("Mix *".to_owned())?;
    flush_all(&mut ui, std::slice::from_ref(&window));
    let full = ui.take_presentation_requests()?;
    assert!(retitles(&full, &window).is_empty(), "the queue was full");
    drop(full);
    // Within the delay nothing is asked; after it the retitle is.
    flush_all(&mut ui, std::slice::from_ref(&window));
    assert!(retitles(&ui.take_presentation_requests()?, &window).is_empty());
    let later = ui.now() + Duration::from_millis(300);
    for root in [window.clone(), primary.clone()] {
        ui.select_window(&root)?;
        ui.flush(later)?;
        ui.paint()?;
    }
    assert_eq!(
        retitles(&ui.take_presentation_requests()?, &window),
        ["Mix *"]
    );
    Ok(())
}
