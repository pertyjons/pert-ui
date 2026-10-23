//! Revealing a hidden target: the container hook, the walk, focus, the queued
//! form and keyed collection reveals.
use pert_ui::*;
use std::{cell::Cell, rc::Rc};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0).unwrap())
        .unwrap();
    ui
}

fn flush(ui: &mut Runtime) {
    ui.flush(ui.now()).unwrap();
}

fn tab(key: &str, content: Element) -> Tab {
    Tab::new(key, format!("Tab {key}"), content)
}

fn label(key: &str) -> Element {
    Element::new(key, Label::new(key))
}

fn button(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}

fn find(ui: &Runtime, root: &WidgetId, key: &str) -> WidgetId {
    ui.find_descendant(root, key)
        .unwrap()
        .unwrap_or_else(|| panic!("{key} is not mounted"))
}

fn shown(ui: &Runtime, id: &WidgetId) -> bool {
    let key = ui.key(id).unwrap().to_owned();
    ui.inspect()
        .components
        .iter()
        .filter(|component| component.key == key)
        .any(|component| component.visible)
}

fn selected(ui: &Runtime, key: &str) -> Option<String> {
    ui.inspect()
        .components
        .iter()
        .find(|component| component.key == key)
        .and_then(
            |component| match component.details.properties.get("selected") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            },
        )
}

const REVEALED: RevealOutcome = RevealOutcome::Revealed { activation: None };
const FOCUSED: RevealOutcome = RevealOutcome::RevealedAndFocused { activation: None };

#[test]
fn a_target_in_an_unselected_tab_selects_it_without_moving_focus() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            button("elsewhere"),
            Element::new(
                "tabs",
                Tabs::new([tab("a", label("a-text")), tab("b", button("b-button"))]),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let elsewhere = find(&ui, &root, "elsewhere");
    ui.focus(&elsewhere)?;
    let target = find(&ui, &root, "b-button");
    assert!(!shown(&ui, &target));

    assert_eq!(ui.reveal(&target)?, REVEALED);
    assert!(shown(&ui, &target));
    assert_eq!(selected(&ui, "tabs").as_deref(), Some("b"));
    assert_eq!(ui.focused(), Some(elsewhere), "reveal alone never focuses");
    // Workspace capture records the revealed tab like a clicked one.
    let document = ui.capture_workspace()?.encode();
    assert!(document.contains("selected=\"b\""), "{document}");
    Ok(())
}

#[test]
fn reveal_and_focus_focuses_a_focusable_target_only() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            tab("a", button("a-button")),
            tab(
                "b",
                Element::new(
                    "b-column",
                    Column::new([button("b-button"), label("b-text")]),
                ),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let button = find(&ui, &root, "b-button");
    assert_eq!(ui.reveal_and_focus(&button)?, FOCUSED);
    assert_eq!(ui.focused(), Some(button.clone()));

    // A label is revealed but never claims focus.
    let text = find(&ui, &root, "b-text");
    assert_eq!(ui.reveal_and_focus(&text)?, REVEALED);
    assert!(shown(&ui, &text));
    assert_eq!(ui.focused(), Some(button));
    Ok(())
}

#[test]
fn nested_containers_open_from_the_outermost_inwards() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "outer",
        Tabs::new([
            tab("first", label("first-text")),
            tab(
                "second",
                Element::new(
                    "inner",
                    Tabs::new([tab("x", label("x-text")), tab("y", label("y-text"))]),
                ),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "y-text");
    assert_eq!(ui.reveal(&target)?, REVEALED);
    assert_eq!(selected(&ui, "outer").as_deref(), Some("second"));
    assert_eq!(selected(&ui, "inner").as_deref(), Some("y"));
    assert!(shown(&ui, &target));
    Ok(())
}

#[test]
fn an_application_bound_view_refuses_and_earlier_containers_stay_open() -> TestResult {
    let mut ui = ui();
    let visible = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            tab("a", label("a-text")),
            tab(
                "b",
                Element::new(
                    "bound",
                    View::new(label("b-text")).visible_signal(visible.read_only()),
                ),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let bound = find(&ui, &root, "bound");
    let target = find(&ui, &root, "b-text");
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Refused(RevealRefusal::ApplicationControlled { ancestor: bound })
    );
    assert!(!visible.get()?, "the application's signal is untouched");
    assert_eq!(
        selected(&ui, "tabs").as_deref(),
        Some("b"),
        "opening the path is not atomic"
    );
    // Once the application shows it, the same target reveals.
    visible.set(true)?;
    assert_eq!(ui.reveal(&target)?, REVEALED);
    Ok(())
}

#[test]
fn explicit_runtime_hiding_refuses_before_anything_opens() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([tab("a", label("a-text")), tab("b", label("b-text"))]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "b-text");
    ui.set_visible(&target, false)?;
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Refused(RevealRefusal::Hidden {
            component: target.clone()
        })
    );
    assert_eq!(selected(&ui, "tabs").as_deref(), Some("a"));
    Ok(())
}

#[test]
fn a_modal_blocks_targets_outside_it_and_admits_targets_inside() -> TestResult {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "tabs",
                Tabs::new([tab("a", label("a-text")), tab("b", label("b-text"))]),
            ),
            Element::new(
                "owner",
                Dialog::new(
                    state.clone(),
                    label("opener"),
                    Element::new(
                        "dialog-tabs",
                        Tabs::new([tab("p", button("p-button")), tab("q", button("q-button"))]),
                    ),
                ),
            ),
        ]),
    ))?;
    flush(&mut ui);
    state.open()?;
    flush(&mut ui);
    let outside = find(&ui, &root, "b-text");
    assert_eq!(
        ui.reveal(&outside)?,
        RevealOutcome::Refused(RevealRefusal::ModalBlocked)
    );
    assert_eq!(
        selected(&ui, "tabs").as_deref(),
        Some("a"),
        "admission is checked before anything opens"
    );
    let inside = find(&ui, &root, "q-button");
    assert_eq!(ui.reveal_and_focus(&inside)?, FOCUSED);
    assert_eq!(ui.focused(), Some(inside));
    Ok(())
}

/// A custom container: it hides its child until it is opened, through the
/// public hook only.
struct Disclosure {
    child: Option<Element>,
    open: bool,
    asked: Rc<Cell<usize>>,
}
impl Component for Disclosure {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        self.open
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        _: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.asked.set(self.asked.get() + 1);
        self.open = true;
        cx.invalidate(Invalidation::Layout);
        Ok(ChildReveal::Changed)
    }
}

#[test]
fn a_modal_that_closes_in_the_same_event_no_longer_blocks_a_queued_reveal() -> TestResult {
    let mut ui = ui();
    let state = DialogController::new(&ui);
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "tabs",
                Tabs::new([tab("a", label("a-text")), tab("b", button("b-button"))]),
            ),
            Element::new(
                "owner",
                Dialog::new(state.clone(), label("opener"), button("jump")),
            ),
        ]),
    ))?;
    flush(&mut ui);
    state.open()?;
    flush(&mut ui);
    let jump = find(&ui, &root, "jump");
    let target = find(&ui, &root, "b-button");
    // A palette row: it closes its modal surface and asks for the destination.
    let receipt = ui.event(&jump, |cx| {
        state.cancel()?;
        cx.request_reveal_and_focus(&target)
    })??;
    flush(&mut ui);
    assert_eq!(receipt.status(), RevealStatus::Finished(Ok(FOCUSED)));
    assert_eq!(ui.focused(), Some(target));
    Ok(())
}

#[test]
fn a_custom_container_opens_through_the_public_hook() -> TestResult {
    let mut ui = ui();
    let asked = Rc::new(Cell::new(0));
    let root = ui.mount_tree(Element::new(
        "disclosure",
        Disclosure {
            child: Some(button("inside")),
            open: false,
            asked: asked.clone(),
        },
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "inside");
    assert!(!shown(&ui, &target));
    assert_eq!(ui.reveal_and_focus(&target)?, FOCUSED);
    assert!(shown(&ui, &target));
    assert_eq!(asked.get(), 1);
    // An open path asks nobody.
    assert_eq!(ui.reveal(&target)?, REVEALED);
    assert_eq!(asked.get(), 1);
    Ok(())
}

/// A container that refuses by default, as every component does.
struct Closed(Option<Element>);
impl Component for Closed {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        false
    }
}

#[test]
fn the_default_hook_refuses_and_names_the_container() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("closed", Closed(Some(label("inside")))))?;
    flush(&mut ui);
    let target = find(&ui, &root, "inside");
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Refused(RevealRefusal::AncestorRefused { ancestor: root })
    );
    Ok(())
}

/// A container that replaces its child under a new key when it opens, so the
/// revealed identity no longer exists.
struct Replacing {
    children: Option<Vec<Element>>,
    open: bool,
}
impl Component for Replacing {
    fn take_children(&mut self) -> Vec<Element> {
        self.children.take().unwrap_or_default()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.open && cx.children().iter().any(|child| child.key() == "old") {
            cx.request_children(vec![label("new")]);
        }
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        self.open
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        _: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.open = true;
        cx.invalidate(Invalidation::Layout);
        Ok(ChildReveal::Changed)
    }
}

#[test]
fn a_target_the_reveal_unmounts_is_reported_and_never_retargeted() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "replacing",
        Replacing {
            children: Some(vec![label("old")]),
            open: false,
        },
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "old");
    assert_eq!(ui.reveal(&target)?, RevealOutcome::Unmounted);
    assert!(ui.find_descendant(&root, "new")?.is_some());
    // The stale identity is now an operational error.
    assert_eq!(ui.reveal(&target), Err(RuntimeError::Unmounted));
    Ok(())
}

/// A container whose hook claims a change it did not make.
struct Liar(Option<Element>);
impl Component for Liar {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        false
    }
    fn reveal_child(&mut self, _: &mut EventContext, _: &str) -> Result<ChildReveal, RuntimeError> {
        Ok(ChildReveal::Changed)
    }
}

#[test]
fn a_change_that_leaves_the_child_hidden_is_a_refusal() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new("liar", Liar(Some(label("inside")))))?;
    flush(&mut ui);
    let target = find(&ui, &root, "inside");
    assert_eq!(
        ui.reveal(&target)?,
        RevealOutcome::Refused(RevealRefusal::AncestorRefused { ancestor: root })
    );
    Ok(())
}

#[test]
fn a_target_far_down_a_scroll_area_is_scrolled_into_view() -> TestResult {
    let mut ui = ui();
    let rows: Vec<Element> = (0..60).map(|row| button(&format!("row-{row}"))).collect();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            tab("a", label("a-text")),
            tab("b", Element::new("rows", Column::new(rows))),
        ]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "row-59");
    assert_eq!(ui.reveal(&target)?, REVEALED);
    let [_, y, _, height] = ui
        .inspect()
        .components
        .iter()
        .find(|component| component.key == "row-59")
        .and_then(|component| component.details.box_model.border_box)
        .ok_or("row has no bounds")?;
    assert!(
        y >= 0.0 && y + height <= 400.0,
        "{y} + {height} is inside the 400 px window"
    );
    Ok(())
}

#[test]
fn reveal_child_selects_a_tab_by_key() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            button("elsewhere"),
            Element::new(
                "tabs",
                Tabs::new([tab("a", label("a-text")), tab("b", label("b-text"))]),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let elsewhere = find(&ui, &root, "elsewhere");
    ui.focus(&elsewhere)?;
    let tabs = find(&ui, &root, "tabs");
    assert_eq!(ui.reveal_child(&tabs, "b")?, ChildReveal::Changed);
    assert_eq!(selected(&ui, "tabs").as_deref(), Some("b"));
    assert!(shown(&ui, &find(&ui, &root, "b-text")));
    assert_eq!(ui.focused(), Some(elsewhere));
    assert_eq!(ui.reveal_child(&tabs, "b")?, ChildReveal::AlreadyVisible);
    assert_eq!(
        ui.reveal_child(&tabs, "missing"),
        Err(RuntimeError::Layout(LayoutError::UnknownKey(
            "missing".into()
        )))
    );
    let document = ui.capture_workspace()?.encode();
    assert!(document.contains("selected=\"b\""), "{document}");
    Ok(())
}

#[test]
fn a_detached_tab_reports_a_requested_window_activation() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([tab("a", button("a-button")), tab("b", button("b-button"))]),
    ))?;
    flush(&mut ui);
    let primary = ui.primary_window();
    ui.reveal_child(&root, "b")?;
    let view = ui.children(&root)?[1].clone();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("b")?)?;
    let window = ui.commit_detach(prepared)?;
    ui.select_window(&primary)?;
    flush(&mut ui);
    let _ = ui.take_presentation_requests()?;

    let target = find(&ui, &root, "b-button");
    let outcome = ui.reveal_and_focus(&target)?;
    let RevealOutcome::RevealedAndFocused {
        activation: Some(activation),
    } = outcome
    else {
        panic!("expected a focused reveal with an activation, got {outcome:?}");
    };
    assert_eq!(activation.status(), PresentationStatus::Pending);
    let requests = ui.take_presentation_requests()?;
    assert!(
        requests.iter().any(
            |request| matches!(request.action(), PresentationAction::Activate(w) if *w == window)
        ),
        "{requests:?}"
    );
    assert_eq!(ui.current_window(), primary, "the selected window is kept");
    assert_eq!(
        selected(&ui, "tabs").as_deref(),
        Some("a"),
        "the strip is untouched"
    );

    // Selecting the detached tab by key asks for access through the strip.
    assert_eq!(ui.reveal_child(&root, "b")?, ChildReveal::AlreadyVisible);
    let requests = ui.take_presentation_requests()?;
    assert!(requests.iter().any(
        |request| matches!(request.action(), PresentationAction::Activate(w) if *w == window)
    ));
    Ok(())
}

#[test]
fn a_target_in_the_requesting_window_requests_no_activation() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([tab("a", label("a-text")), tab("b", label("b-text"))]),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "b-text");
    assert_eq!(ui.reveal(&target)?.activation(), None);
    assert!(ui.take_presentation_requests()?.is_empty());
    Ok(())
}

#[test]
fn a_queued_reveal_reaches_its_receipt_after_the_flush() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            button("requester"),
            Element::new(
                "tabs",
                Tabs::new([tab("a", label("a-text")), tab("b", button("b-button"))]),
            ),
        ]),
    ))?;
    flush(&mut ui);
    let requester = find(&ui, &root, "requester");
    let target = find(&ui, &root, "b-button");
    let receipt = ui.event(&requester, |cx| cx.request_reveal_and_focus(&target))??;
    assert_eq!(receipt.status(), RevealStatus::Pending);
    assert!(ui.next_deadline().is_some(), "a queued reveal is due work");
    flush(&mut ui);
    assert_eq!(receipt.status(), RevealStatus::Finished(Ok(FOCUSED)));
    assert_eq!(ui.focused(), Some(target));
    assert!(ui.next_deadline().is_none());
    Ok(())
}

#[test]
fn a_queued_reveal_of_a_removed_target_finishes_with_an_error() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([button("requester"), label("target")]),
    ))?;
    flush(&mut ui);
    let requester = find(&ui, &root, "requester");
    let target = find(&ui, &root, "target");
    let receipt = ui.event(&requester, |cx| cx.request_reveal(&target))??;
    ui.unmount(&target)?;
    flush(&mut ui);
    assert_eq!(
        receipt.status(),
        RevealStatus::Finished(Err(RuntimeError::Unmounted))
    );
    Ok(())
}

#[test]
fn the_reveal_queue_is_bounded_and_shutdown_cancels_it() -> TestResult {
    let mut ui = ui();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([button("requester"), label("target")]),
    ))?;
    flush(&mut ui);
    let requester = find(&ui, &root, "requester");
    let target = find(&ui, &root, "target");
    let receipts = ui.event(&requester, |cx| {
        let mut receipts = Vec::new();
        for _ in 0..Runtime::MAX_REVEAL_REQUESTS {
            receipts.push(cx.request_reveal(&target)?);
        }
        assert_eq!(
            cx.request_reveal(&target),
            Err(RuntimeError::RevealQueueFull)
        );
        Ok::<_, RuntimeError>(receipts)
    })??;
    ui.shutdown();
    assert!(
        receipts
            .iter()
            .all(|receipt| receipt.status() == RevealStatus::Cancelled)
    );
    Ok(())
}

#[test]
fn a_request_for_another_runtime_is_refused() {
    let mut ui = ui();
    let mut other = Runtime::new();
    let foreign = other.mount("x", Label::new("x")).unwrap();
    let requester = ui
        .mount_tree(Element::new("requester", Button::new("r", |_| Ok(()))))
        .unwrap();
    flush(&mut ui);
    let result = ui
        .event(&requester, |cx| cx.request_reveal(&foreign).map(|_| ()))
        .unwrap();
    assert_eq!(result, Err(RuntimeError::ForeignRuntime));
    assert_eq!(ui.reveal(&foreign), Err(RuntimeError::ForeignRuntime));
}

struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index < self.0).then(|| button(&format!("row-{index}")))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("row-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}

#[test]
fn a_list_reveal_reports_the_mounted_row_which_can_then_take_focus() -> TestResult {
    let mut ui = ui();
    let controller = ListController::new(&ui);
    let items = ui.signal(ListSource::new(Rows(10_000)));
    ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), 24.0)?.controller(controller.clone()),
    ))?;
    flush(&mut ui);

    let receipt = controller.reveal("row-7777")?;
    assert_eq!(receipt.status(), ListRevealStatus::Pending);
    flush(&mut ui);
    let ListRevealStatus::Mounted(row) = receipt.status() else {
        panic!("{:?}", receipt.status());
    };
    assert_eq!(ui.key(&row)?, "row-7777");
    assert_eq!(ui.reveal_and_focus(&row)?, FOCUSED);
    assert_eq!(ui.focused(), Some(row));

    let missing = controller.reveal("row-nope")?;
    flush(&mut ui);
    assert_eq!(missing.status(), ListRevealStatus::NotFound);

    // A newer request supersedes one that has not completed.
    let first = controller.reveal("row-10")?;
    let second = controller.reveal("row-20")?;
    assert_eq!(first.status(), ListRevealStatus::Superseded);
    flush(&mut ui);
    assert!(matches!(second.status(), ListRevealStatus::Mounted(_)));
    Ok(())
}

#[test]
fn a_hidden_list_serves_its_request_when_shown() -> TestResult {
    let mut ui = ui();
    let controller = ListController::new(&ui);
    let items = ui.signal(ListSource::new(Rows(10_000)));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), 24.0)?.controller(controller.clone()),
    ))?;
    flush(&mut ui);
    ui.set_visible(&list, false)?;
    let receipt = controller.reveal("row-500")?;
    flush(&mut ui);
    assert_eq!(receipt.status(), ListRevealStatus::Pending);
    ui.set_visible(&list, true)?;
    flush(&mut ui);
    assert!(matches!(receipt.status(), ListRevealStatus::Mounted(_)));
    Ok(())
}

#[test]
fn a_list_removed_before_mounting_the_row_cancels_its_receipt() -> TestResult {
    // Without a window extent the list has no viewport: it serves the
    // request but mounts nothing.
    let mut ui = Runtime::new();
    let controller = ListController::new(&ui);
    let items = ui.signal(ListSource::new(Rows(10_000)));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), 24.0)?.controller(controller.clone()),
    ))?;
    flush(&mut ui);
    let receipt = controller.reveal("row-500")?;
    flush(&mut ui);
    assert_eq!(receipt.status(), ListRevealStatus::Pending);
    ui.unmount(&list)?;
    assert_eq!(receipt.status(), ListRevealStatus::Cancelled);
    Ok(())
}

#[test]
fn a_hidden_list_removed_before_reading_a_request_cancels_its_receipt() -> TestResult {
    let mut ui = ui();
    let controller = ListController::new(&ui);
    let items = ui.signal(ListSource::new(Rows(10_000)));
    let shown = ui.signal(true);
    let root = ui.mount_tree(Element::new(
        "view",
        View::new(Element::new(
            "list",
            VirtualList::new(items.read_only(), 24.0)?.controller(controller.clone()),
        ))
        .visible_signal(shown.read_only()),
    ))?;
    flush(&mut ui);
    shown.set(false)?;
    flush(&mut ui);
    let receipt = controller.reveal("row-500")?;
    flush(&mut ui);
    assert_eq!(receipt.status(), ListRevealStatus::Pending);
    ui.unmount(&find(&ui, &root, "list"))?;
    assert_eq!(receipt.status(), ListRevealStatus::Cancelled);
    Ok(())
}

#[test]
fn a_tree_reveal_reports_rows_and_collapsed_nodes() -> TestResult {
    struct Nodes;
    impl TreeItems for Nodes {
        fn roots(&self) -> Vec<String> {
            vec!["top".into()]
        }
        fn children(&self, key: &str) -> Vec<String> {
            if key == "top" {
                (0..500).map(|index| format!("top/{index}")).collect()
            } else {
                Vec::new()
            }
        }
        fn can_expand(&self, key: &str) -> bool {
            key == "top"
        }
        fn build(&self, key: &str) -> Option<Element> {
            Some(label(key))
        }
    }
    let mut ui = ui();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(&ui, selection, TreeSource::new(Nodes))?;
    ui.mount_tree(Element::new("tree", Tree::new(&state, 24.0)?))?;
    flush(&mut ui);
    let collapsed = state.reveal("top/400")?;
    flush(&mut ui);
    assert_eq!(
        collapsed.status(),
        ListRevealStatus::NotFound,
        "a row inside a collapsed node is not shown"
    );
    state.set_expanded("top", true)?;
    let expanded = state.reveal("top/400")?;
    flush(&mut ui);
    assert!(matches!(expanded.status(), ListRevealStatus::Mounted(_)));
    Ok(())
}

/// A pass-through scroll container whose `reveal` runs `on_reveal` and
/// reports a change, so the walk settles layout after it.
struct Scroller {
    children: Vec<Element>,
    on_reveal: Option<Box<dyn FnMut() -> Result<(), RuntimeError>>>,
}
impl Scroller {
    fn new(child: Element) -> Self {
        Self {
            children: vec![child],
            on_reveal: None,
        }
    }
    fn on_reveal(mut self, action: impl FnMut() -> Result<(), RuntimeError> + 'static) -> Self {
        self.on_reveal = Some(Box::new(action));
        self
    }
}
impl Component for Scroller {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 100.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 0.0, 200.0, 100.0)?)?;
        }
        Ok(())
    }
    fn can_reveal(&self) -> bool {
        true
    }
    fn reveal(&mut self, _: &mut EventContext, _: Rect) -> Result<bool, RuntimeError> {
        match &mut self.on_reveal {
            Some(action) => {
                action()?;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

/// Shows its child while `keep` is true and removes it otherwise.
struct Host {
    keep: Signal<bool>,
    child: Option<Box<dyn Fn() -> Element>>,
    shown: bool,
}
impl Component for Host {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let keep = cx.read(&self.keep)?;
        if keep != self.shown {
            self.shown = keep;
            let children = if keep {
                vec![(self.child.as_ref().unwrap())()]
            } else {
                Vec::new()
            };
            cx.request_children(children);
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 100.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, Rect::new(0.0, 0.0, 200.0, 100.0)?)?;
        }
        Ok(())
    }
}

#[test]
fn a_scroll_reveal_that_removes_its_outer_ancestor_ends_as_unmounted() -> TestResult {
    let mut ui = ui();
    let keep = ui.signal(true);
    let keep_read = keep.clone();
    let write = keep.clone();
    let child = move || {
        let write = write.clone();
        Element::new(
            "outer",
            Scroller::new(Element::new(
                "inner",
                Scroller::new(button("target")).on_reveal(move || {
                    write.set(false)?;
                    Ok(())
                }),
            )),
        )
    };
    let root = ui.mount_tree(Element::new(
        "host",
        Host {
            keep,
            child: Some(Box::new(child)),
            shown: false,
        },
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "target");
    let outcome = ui.reveal(&target)?;
    assert!(!keep_read.get()?);
    assert_eq!(outcome, RevealOutcome::Unmounted);
    Ok(())
}

#[test]
fn a_scroll_reveal_that_hides_the_target_is_not_reported_as_revealed() -> TestResult {
    let mut ui = ui();
    let shown = ui.signal(true);
    let write = shown.clone();
    let root = ui.mount_tree(Element::new(
        "outer",
        Scroller::new(Element::new(
            "view",
            View::new(Element::new(
                "inner",
                Scroller::new(button("target")).on_reveal(move || {
                    write.set(false)?;
                    Ok(())
                }),
            ))
            .visible_signal(shown.read_only()),
        )),
    ))?;
    flush(&mut ui);
    let target = find(&ui, &root, "target");
    let outcome = ui.reveal_and_focus(&target)?;
    assert!(
        matches!(outcome, RevealOutcome::Refused(_)),
        "hidden again by the scroll: {outcome:?}"
    );
    assert!(!shown.get()?);
    Ok(())
}

/// A container whose hook opens it and queues another reveal.
struct Chaining {
    child: Option<Element>,
    open: bool,
    next: Rc<std::cell::RefCell<Option<WidgetId>>>,
    receipt: Rc<std::cell::RefCell<Option<RevealReceipt>>>,
}
impl Component for Chaining {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_visible(&self, _: &str) -> bool {
        self.open
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        _: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        self.open = true;
        if let Some(next) = self.next.borrow_mut().take() {
            *self.receipt.borrow_mut() = Some(cx.request_reveal(&next)?);
        }
        cx.invalidate(Invalidation::Layout);
        Ok(ChildReveal::Changed)
    }
}

#[test]
fn a_reveal_queued_during_dispatch_is_due_for_the_application() -> TestResult {
    let mut ui = ui();
    let next = Rc::new(std::cell::RefCell::new(None));
    let receipt = Rc::new(std::cell::RefCell::new(None));
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            button("requester"),
            label("other"),
            Element::new(
                "chain",
                Chaining {
                    child: Some(button("inside")),
                    open: false,
                    next: next.clone(),
                    receipt: receipt.clone(),
                },
            ),
        ]),
    ))?;
    flush(&mut ui);
    *next.borrow_mut() = Some(find(&ui, &root, "other"));
    let requester = find(&ui, &root, "requester");
    let target = find(&ui, &root, "inside");
    let first = ui.event(&requester, |cx| cx.request_reveal(&target))??;
    flush(&mut ui);
    assert_eq!(first.status(), RevealStatus::Finished(Ok(REVEALED)));
    let second = receipt.borrow().clone().expect("the hook queued a reveal");
    assert_eq!(
        second.status(),
        RevealStatus::Pending,
        "it waits for the next flush"
    );
    assert_eq!(
        ui.application_deadline(),
        Some(ui.now()),
        "a waiting reveal is due work"
    );
    flush(&mut ui);
    assert_eq!(second.status(), RevealStatus::Finished(Ok(REVEALED)));
    Ok(())
}
