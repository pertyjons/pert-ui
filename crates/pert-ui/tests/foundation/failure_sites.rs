//! Failure sites through the facade: a failing flush or paint names the
//! component where it arose, and the next one starts without a site.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Fails measurement or painting while its switch reads true.
struct Faulty {
    measure: Rc<Cell<bool>>,
    paint: Rc<Cell<bool>>,
}
impl Component for Faulty {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        if self.measure.get() {
            return Err(LayoutError::InvalidChild.into());
        }
        Ok(Size::new(10.0, 10.0)?)
    }
    fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
        if self.paint.get() {
            return Err(DrawError::Runtime(RuntimeError::Exhausted));
        }
        Ok(())
    }
}

#[test]
fn a_failure_is_named_where_it_arose_and_cleared_by_the_next_success() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (measure, paint) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(false)));
    let root = ui.mount_tree(
        Column::new([Container::new(
            Faulty {
                measure: measure.clone(),
                paint: paint.clone(),
            }
            .key("leaf"),
        )
        .key("frame")])
        .key("form"),
    )?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let site = ui.last_failure().ok_or("a component failed")?;
    assert_eq!(site.path, ["form", "frame", "leaf"]);
    assert_eq!(site.phase, FailurePhase::Measure);
    assert!(site.description.is_empty());
    assert_eq!(site.to_string(), "measurement of form/frame/leaf");
    measure.set(false);
    let leaf = ui.find_path(&root, &["frame", "leaf"])?.ok_or("mounted")?;
    ui.invalidate(&leaf, Invalidation::Layout)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    paint.set(true);
    ui.invalidate(&leaf, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.paint().is_err());
    let site = ui.last_failure().ok_or("painting failed")?;
    assert_eq!(site.phase, FailurePhase::Paint);
    assert_eq!(site.to_string(), "painting of form/frame/leaf");
    paint.set(false);
    ui.invalidate(&leaf, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(ui.last_failure(), None);
    Ok(())
}

/// Fails its update while its switch reads true.
struct Switch(Rc<Cell<bool>>);
impl Component for Switch {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.0.get() {
            return Err(RuntimeError::Exhausted);
        }
        Ok(())
    }
}

#[test]
fn flushing_every_window_names_its_own_failure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (a, b) = (Rc::new(Cell::new(true)), Rc::new(Cell::new(false)));
    let root = ui.mount_tree(
        Column::new([Switch(a.clone()).key("a"), Switch(b.clone()).key("b")]).key("root"),
    )?;
    assert!(ui.flush_windows(Duration::ZERO).is_err());
    assert_eq!(
        ui.last_failure().map(|site| site.path),
        Some(vec!["root".to_owned(), "a".to_owned()])
    );
    a.set(false);
    for key in ["a", "b"] {
        let id = ui.find_path(&root, &[key])?.ok_or("mounted")?;
        ui.invalidate(&id, Invalidation::Paint)?;
    }
    ui.flush_windows(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    b.set(true);
    let id = ui.find_path(&root, &["b"])?.ok_or("mounted")?;
    ui.invalidate(&id, Invalidation::Paint)?;
    assert!(ui.flush_windows(Duration::ZERO).is_err());
    assert_eq!(
        ui.last_failure().map(|site| site.path),
        Some(vec!["root".to_owned(), "b".to_owned()])
    );
    Ok(())
}

/// A container that places none of its children.
struct Forgetful;
impl Component for Forgetful {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn a_child_left_unplaced_is_the_containers_arrangement_failure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    ui.mount_tree(
        Element::new("forgetful", Forgetful).children([Label::new("child").key("child")]),
    )?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let site = ui.last_failure().ok_or("a component failed")?;
    assert_eq!(site.phase, FailurePhase::Arrange);
    assert_eq!(site.path.last().map(String::as_str), Some("forgetful"));
    assert_eq!(site.to_string(), "arrangement of forgetful");
    Ok(())
}

/// Fails its description check.
struct Undescribable;
impl Component for Undescribable {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn check_description(&self) -> Result<(), RuntimeError> {
        Err(LayoutError::InvalidChild.into())
    }
}

/// Requests the children its switch selects, as a custom container would,
/// stacked at its origin.
struct Requester(Signal<bool>, fn(bool) -> Vec<Element>);
impl Component for Requester {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let on = cx.read(&self.0)?;
        cx.request_children((self.1)(on));
        Ok(())
    }
    fn stacks_children(&self) -> bool {
        true
    }
}

/// One list row.
struct OneRow;
impl ListItems for OneRow {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "only".to_owned())
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index == 0).then(|| Label::new("row").key("text"))
    }
}

fn reconciliation(ui: &Runtime) -> Result<(Vec<String>, Vec<String>, String), String> {
    let site = ui.last_failure().ok_or("a reconciliation failed")?;
    if site.phase != FailurePhase::Reconcile {
        return Err(format!("expected a reconciliation, found {site}"));
    }
    Ok((
        site.path.clone(),
        site.description.clone(),
        site.to_string(),
    ))
}

#[test]
fn a_duplicated_key_from_each_names_the_owner_and_the_key() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let items = ui.signal(vec!["a".to_owned()]);
    let root = ui.mount_tree(
        Column::new([Each::new(
            &items,
            |_| "dup".to_owned(),
            |_, item| Label::new(item.as_str()).key("x"),
        )
        .key("each")])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    let kept = ui.find_path(&root, &["each", "items", "dup"])?;
    assert!(kept.is_some());
    items.set(vec!["a".to_owned(), "b".to_owned()])?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("dup".into()))
    );
    let (path, description, text) = reconciliation(&ui)?;
    assert_eq!(path, ["root", "each"]);
    assert_eq!(description, ["items", "dup"]);
    assert_eq!(text, "reconciliation of root/each at items/dup");
    // The rejected request changed nothing and is consumed: an unchanged
    // flush succeeds without a site, and a valid value applies.
    assert_eq!(ui.find_path(&root, &["each", "items", "dup"])?, kept);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    items.set(vec!["c".to_owned()])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    assert_eq!(ui.find_path(&root, &["each", "items", "dup"])?, kept);
    Ok(())
}

#[test]
fn a_changed_type_under_a_kept_key_names_the_element_at_any_depth() -> TestResult {
    for nested in [false, true] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
        let label = ui.signal(true);
        ui.mount_tree(
            Dynamic::new(&label, move |_, label| {
                let leaf = if *label {
                    Label::new("text").key("leaf")
                } else {
                    Button::new("press", |_| Ok(())).key("leaf")
                };
                if nested {
                    Column::new([leaf]).key("box")
                } else {
                    leaf
                }
            })
            .key("dynamic"),
        )?;
        ui.flush(Duration::ZERO)?;
        label.set(false)?;
        assert_eq!(
            ui.flush(Duration::ZERO),
            Err(RuntimeError::ChildTypeChanged("leaf".into()))
        );
        let (path, description, _) = reconciliation(&ui)?;
        assert_eq!(path, ["dynamic"]);
        let expected: &[&str] = if nested { &["box", "leaf"] } else { &["leaf"] };
        assert_eq!(description, expected);
    }
    Ok(())
}

#[test]
fn a_failing_description_check_and_an_empty_key_name_their_element() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let shown = ui.signal(false);
    ui.mount_tree(Show::new(&shown, |_| Undescribable.key("odd")).key("show"))?;
    ui.flush(Duration::ZERO)?;
    shown.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    );
    let (path, description, _) = reconciliation(&ui)?;
    assert_eq!(
        (path.as_slice(), description.as_slice()),
        (&["show".to_owned()][..], &["odd".to_owned()][..])
    );

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let empty = ui.signal(false);
    ui.mount_tree(Element::new(
        "requester",
        Requester(empty.clone(), |empty| {
            let key = if empty { "" } else { "label" };
            vec![Column::new([Label::new("text").key(key)]).key("group")]
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    empty.set(true)?;
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::EmptyKey));
    let (path, description, _) = reconciliation(&ui)?;
    assert_eq!(path, ["requester"]);
    assert_eq!(description, ["group"]);
    Ok(())
}

#[test]
fn a_new_child_failing_its_update_is_named_as_that_update() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let shown = ui.signal(false);
    let failing = Rc::new(Cell::new(true));
    let switch = failing.clone();
    ui.mount_tree(Show::new(&shown, move |_| Switch(switch.clone()).key("child")).key("show"))?;
    ui.flush(Duration::ZERO)?;
    shown.set(true)?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let site = ui.last_failure().ok_or("the child failed")?;
    assert_eq!(site.phase, FailurePhase::Update);
    assert_eq!(site.path, ["show", "child"]);
    assert!(site.description.is_empty());
    assert_eq!(site.to_string(), "update of show/child");
    Ok(())
}

#[test]
fn a_rejected_preparation_rolls_back_new_siblings() -> TestResult {
    // The list requests its own rows, so a description that gives it
    // children is refused after the new sibling before it was mounted.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let rows = ui.signal(ListSource::new(OneRow));
    let given = ui.signal(false);
    let source = rows.clone();
    let root = ui.mount_tree(
        Dynamic::new(&given, move |_, given| {
            let list = VirtualList::new(source.read_only(), 20.0)
                .map_or_else(|_| Label::new("").key("list"), |list| list.key("list"));
            let list = if *given {
                list.children([Label::new("row").key("row")])
            } else {
                list
            };
            let mut children = Vec::new();
            if *given {
                children.push(Label::new("fresh").key("fresh"));
            }
            children.push(list);
            Column::new(children).key("box")
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    given.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    );
    let (_, description, _) = reconciliation(&ui)?;
    assert_eq!(description, ["box", "list"]);
    assert_eq!(ui.find_path(&root, &["box", "fresh"])?, None);
    assert!(ui.find_path(&root, &["box", "list"])?.is_some());
    Ok(())
}

#[test]
fn successive_owners_are_each_named_and_a_removed_owners_request_is_dropped() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (first, second) = (ui.signal(false), ui.signal(false));
    let described = |flag: &Signal<bool>, key: &'static str| {
        Dynamic::new(flag, |_, dup| {
            let other = if *dup { "a" } else { "b" };
            Column::new([Label::new("1").key("a"), Label::new("2").key(other)]).key("box")
        })
        .key(key)
    };
    let root = ui.mount_tree(
        Column::new([described(&first, "first"), described(&second, "second")]).key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    first.set(true)?;
    second.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("a".into()))
    );
    assert_eq!(reconciliation(&ui)?.0, ["root", "first"]);
    // The second request waits; once its owner is gone it is dropped.
    first.set(false)?;
    ui.reconcile_children(&root, vec![described(&first, "first")])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    assert_eq!(ui.find_path(&root, &["second"])?, None);
    // A later failure of another owner names that owner.
    let second = ui.signal(false);
    ui.reconcile_children(
        &root,
        vec![described(&first, "first"), described(&second, "second")],
    )?;
    ui.flush(Duration::ZERO)?;
    second.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("a".into()))
    );
    assert_eq!(
        reconciliation(&ui)?,
        (
            vec!["root".to_owned(), "second".to_owned()],
            vec!["box".to_owned(), "a".to_owned()],
            "reconciliation of root/second at box/a".to_owned()
        )
    );
    Ok(())
}

#[test]
fn a_direct_reconciliation_records_no_site_and_keeps_an_earlier_one() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let root = ui.mount_tree(Column::new([Label::new("a").key("a")]).key("root"))?;
    ui.flush(Duration::ZERO)?;
    let twice = || vec![Label::new("1").key("x"), Label::new("2").key("x")];
    assert_eq!(
        ui.reconcile_children(&root, twice()),
        Err(RuntimeError::DuplicateKey("x".into()))
    );
    assert_eq!(ui.last_failure(), None);
    let failing = Rc::new(Cell::new(true));
    ui.reconcile_children(&root, vec![Switch(failing.clone()).key("switch")])?;
    assert!(ui.flush(Duration::ZERO).is_err());
    let earlier = ui.last_failure();
    assert!(earlier.is_some());
    assert_eq!(
        ui.reconcile_children(&root, twice()),
        Err(RuntimeError::DuplicateKey("x".into()))
    );
    assert_eq!(ui.last_failure(), earlier);
    Ok(())
}

#[test]
fn a_key_taken_by_a_moved_child_names_the_colliding_description() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let collide = ui.signal(false);
    let root = ui.mount_tree(Element::new(
        "requester",
        Requester(collide.clone(), |collide| {
            let inner = if collide {
                vec![Label::new("again").key("moved")]
            } else {
                Vec::new()
            };
            vec![
                Label::new("moved").key("moved"),
                Column::new(inner).key("box"),
            ]
        }),
    ))?;
    ui.flush(Duration::ZERO)?;
    let moved = ui.find_path(&root, &["moved"])?.ok_or("mounted")?;
    let target = ui.find_path(&root, &["box"])?.ok_or("mounted")?;
    ui.reparent(&moved, &target, 0)?;
    ui.flush(Duration::ZERO)?;
    collide.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("moved".into()))
    );
    let (path, description, _) = reconciliation(&ui)?;
    assert_eq!(path, ["requester"]);
    assert_eq!(description, ["box", "moved"]);
    Ok(())
}

/// Keeps the children of its first description.
struct Keeper;
impl Component for Keeper {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn retain_initial_children(&self) -> bool {
        true
    }
}

#[test]
fn retained_constructor_children_are_discarded_before_validation() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let twice = ui.signal(false);
    ui.mount_tree(
        Dynamic::new(&twice, |_, twice| {
            let children = if *twice {
                vec![Label::new("1").key("x"), Label::new("2").key("x")]
            } else {
                vec![Label::new("1").key("x")]
            };
            Element::new("keeper", Keeper).children(children)
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    twice.set(true)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    Ok(())
}

#[test]
fn a_request_after_a_rejected_one_waits_for_the_next_flush() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let (first, second) = (ui.signal(false), ui.signal(false));
    let described = |flag: &Signal<bool>, key: &'static str| {
        Dynamic::new(flag, |_, dup| {
            let other = if *dup { "a" } else { "b" };
            Column::new([Label::new("1").key("a"), Label::new("2").key(other)]).key("box")
        })
        .key(key)
    };
    ui.mount_tree(
        Column::new([described(&first, "first"), described(&second, "second")]).key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    first.set(true)?;
    second.set(true)?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("a".into()))
    );
    assert_eq!(reconciliation(&ui)?.0, ["root", "first"]);
    // Nothing changed, yet the second owner's request is still queued.
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("a".into()))
    );
    assert_eq!(reconciliation(&ui)?.0, ["root", "second"]);
    // Both rejected requests are consumed.
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.last_failure(), None);
    Ok(())
}

#[test]
fn an_earlier_site_of_the_same_flush_stays_authoritative() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let failing = Rc::new(Cell::new(false));
    let dup = ui.signal(false);
    let root = ui.mount_tree(
        Column::new([
            Switch(failing.clone()).key("switch"),
            Dynamic::new(&dup, |_, dup| {
                let other = if *dup { "a" } else { "b" };
                Column::new([Label::new("1").key("a"), Label::new("2").key(other)]).key("box")
            })
            .key("dynamic"),
        ])
        .key("root"),
    )?;
    ui.flush(Duration::ZERO)?;
    // A deferred task catches a nested flush's failure, repairs its cause and
    // then makes the outer flush reject requested children.
    let switch = ui.find_path(&root, &["switch"])?.ok_or("mounted")?;
    let repair = failing.clone();
    let outer_dup = dup.clone();
    let _task = ui.defer(move |ui| {
        repair.set(true);
        ui.invalidate(&switch, Invalidation::Layout)?;
        assert!(ui.flush(Duration::ZERO).is_err());
        repair.set(false);
        outer_dup.set(true)?;
        Ok(())
    })?;
    assert_eq!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::DuplicateKey("a".into()))
    );
    let site = ui.last_failure().ok_or("a site")?;
    assert_eq!(site.phase, FailurePhase::Update);
    assert_eq!(site.path, ["root", "switch"]);
    Ok(())
}

#[test]
fn a_call_refused_at_entry_keeps_the_earlier_site() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 200.0, 1.0)?)?;
    let failing = Rc::new(Cell::new(true));
    ui.mount_tree(Switch(failing).key("switch"))?;
    assert!(ui.flush(Duration::from_millis(5)).is_err());
    let earlier = ui.last_failure();
    assert!(earlier.is_some());
    assert_eq!(
        ui.service_intervals(Duration::ZERO),
        Err(RuntimeError::TimeReversed)
    );
    assert_eq!(ui.last_failure(), earlier);
    Ok(())
}
