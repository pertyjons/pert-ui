//! Virtualized list contracts through the facade: bounded mounting, identity
//! across scrolling, pinned focus and capture, reveal by focus and by key, and
//! content changes.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Taller than a button's minimum, so a slot and its row coincide.
const ROW: f32 = 40.0;

struct Rows {
    count: usize,
    generation: u32,
}
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.count
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.count).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        let label = format!("Row {index} (gen {})", self.generation);
        Some(Element::new(key, Button::new(label, |_| Ok(()))))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("row-")?
            .parse()
            .ok()
            .filter(|index| *index < self.count)
    }
}

fn rows(count: usize) -> ListSource {
    ListSource::new(Rows {
        count,
        generation: 0,
    })
}

struct Fixture {
    ui: Runtime,
    items: Signal<ListSource>,
    controller: ListController,
    list: WidgetId,
}

fn fixture(count: usize, height: f32) -> Result<Fixture, Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, height, 1.0)?)?;
    let items = ui.signal(rows(count));
    let controller = ListController::new(&ui);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?.controller(controller.clone()),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(Fixture {
        ui,
        items,
        controller,
        list,
    })
}

fn keys(ui: &Runtime, list: &WidgetId) -> Vec<String> {
    ui.children(list)
        .unwrap()
        .iter()
        .map(|child| ui.key(child).unwrap().to_owned())
        .collect()
}

fn property(ui: &Runtime, key: &str, name: &str) -> f64 {
    match ui
        .inspect()
        .components
        .iter()
        .find(|item| item.key == key)
        .unwrap()
        .details
        .properties[name]
    {
        InspectValue::Number(value) => value,
        ref other => panic!("{name}: {other:?}"),
    }
}

fn wheel(ui: &mut Runtime, dy: f32) -> Result<(), RuntimeError> {
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, dy)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

#[test]
fn only_rows_near_the_viewport_are_mounted_at_their_logical_positions() -> TestResult {
    let f = fixture(100_000, 200.0)?;
    let mounted = keys(&f.ui, &f.list);
    // Five visible rows plus four overscan rows below; none above row 0.
    assert_eq!(mounted.len(), 9);
    assert_eq!(mounted[0], "row-0");
    assert_eq!(mounted[8], "row-8");
    let children = f.ui.children(&f.list)?;
    assert_eq!(
        f.ui.bounds(&children[0])?.unwrap().values(),
        [0.0, 0.0, 300.0, ROW]
    );
    assert_eq!(
        f.ui.bounds(&children[8])?.unwrap().values(),
        [0.0, 8.0 * ROW, 300.0, ROW]
    );
    assert_eq!(property(&f.ui, "list", "rows"), 100_000.0);
    assert_eq!(property(&f.ui, "list", "mounted"), 9.0);
    assert_eq!(
        f.ui.next_deadline(),
        None,
        "nothing is scheduled once settled"
    );
    Ok(())
}

#[test]
fn scrolling_reuses_rows_that_stay_in_range_and_mounts_the_rest() -> TestResult {
    let mut f = fixture(100_000, 200.0)?;
    let before = f.ui.children(&f.list)?;
    wheel(&mut f.ui, 5.0 * ROW)?;
    let after = f.ui.children(&f.list)?;
    let mounted = keys(&f.ui, &f.list);
    assert_eq!(mounted.first().map(String::as_str), Some("row-1"));
    assert_eq!(mounted.last().map(String::as_str), Some("row-13"));
    // Rows 1..=8 were mounted before and keep their identity.
    for index in 1..=8 {
        let key = format!("row-{index}");
        let old = before
            .iter()
            .find(|id| f.ui.key(id).is_ok_and(|k| k == key))
            .expect("mounted before");
        let new = after
            .iter()
            .find(|id| f.ui.key(id).is_ok_and(|k| k == key))
            .expect("mounted after");
        assert_eq!(old, new, "{key}");
    }
    assert_eq!(f.ui.bounds(&after[0])?.unwrap().y(), ROW - 5.0 * ROW);
    // Far away: an entirely new set, the old rows unmounted.
    wheel(&mut f.ui, 50_000.0 * ROW)?;
    let far = keys(&f.ui, &f.list);
    assert!(
        far.iter()
            .all(|key| key.strip_prefix("row-").unwrap().parse::<usize>().unwrap() > 49_000)
    );
    assert_eq!(far.len(), 13);
    assert_eq!(f.ui.inspect().components.len(), 1 + 13, "old rows are gone");
    Ok(())
}

#[test]
fn a_focused_row_stays_mounted_while_scrolled_away_and_traversal_reveals_the_next() -> TestResult {
    let mut f = fixture(1_000, 200.0)?;
    let third = f.ui.children(&f.list)?[3].clone();
    f.ui.focus(&third)?;
    f.ui.flush(Duration::ZERO)?;
    wheel(&mut f.ui, 400.0 * ROW)?;
    let mounted = keys(&f.ui, &f.list);
    assert_eq!(mounted[0], "row-3", "the focused row is pinned first");
    assert_eq!(f.ui.focused(), Some(third.clone()));
    assert!(
        f.ui.bounds(&third)?.unwrap().y() < 0.0,
        "outside the viewport"
    );
    // Traversal moves to the next mounted focusable row and reveals it.
    f.ui.focus_next(false)?;
    let focused = f.ui.focused().expect("focus moved");
    let [_, y, _, h] = f.ui.bounds(&focused)?.unwrap().values();
    assert!(y >= 0.0 && y + h <= 200.0, "revealed: {y}");
    assert_ne!(focused, third);
    // The old row lost focus and is no longer pinned.
    f.ui.flush(Duration::ZERO)?;
    assert!(!keys(&f.ui, &f.list).contains(&"row-3".to_owned()));
    Ok(())
}

#[test]
fn explicit_focus_on_a_mounted_but_clipped_row_scrolls_it_into_view() -> TestResult {
    let mut f = fixture(1_000, 200.0)?;
    // Row 8 is mounted as overscan but below the 200-pixel viewport.
    let clipped = f.ui.children(&f.list)?[8].clone();
    assert!(f.ui.bounds(&clipped)?.unwrap().y() >= 200.0);
    f.ui.focus(&clipped)?;
    f.ui.flush(Duration::ZERO)?;
    let [_, y, _, h] = f.ui.bounds(&clipped)?.unwrap().values();
    assert!(y >= 0.0 && y + h <= 200.0);
    assert_eq!(property(&f.ui, "list", "scroll_y"), 160.0);
    Ok(())
}

#[test]
fn a_controller_reveals_an_unmounted_row_by_key() -> TestResult {
    let mut f = fixture(100_000, 200.0)?;
    let _ = f.controller.reveal("row-77777")?;
    f.ui.flush(Duration::ZERO)?;
    let mounted = keys(&f.ui, &f.list);
    assert!(mounted.contains(&"row-77777".to_owned()));
    let row =
        f.ui.children(&f.list)?
            .into_iter()
            .find(|id| f.ui.key(id).unwrap() == "row-77777")
            .unwrap();
    let [_, y, _, h] = f.ui.bounds(&row)?.unwrap().values();
    assert!(y >= 0.0 && y + h <= 200.0);
    // Revealed from above, the row ends up at the bottom edge.
    let expected = 77_778.0 * f64::from(ROW) - 200.0;
    assert_eq!(property(&f.ui, "list", "scroll_y"), expected);
    // An unknown key changes nothing.
    let _ = f.controller.reveal("row-nope")?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(property(&f.ui, "list", "scroll_y"), expected);
    Ok(())
}

#[test]
fn a_list_mounted_later_does_not_replay_a_served_reveal() -> TestResult {
    let mut f = fixture(100_000, 200.0)?;
    let _ = f.controller.reveal("row-77777")?;
    f.ui.flush(Duration::ZERO)?;
    f.ui.unmount(&f.list)?;
    // A list rebuilt with the same long-lived controller opens at its start.
    let list = f.ui.mount_tree(Element::new(
        "again",
        VirtualList::new(f.items.read_only(), ROW)?.controller(f.controller.clone()),
    ))?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(property(&f.ui, "again", "scroll_y"), 0.0);
    assert_eq!(keys(&f.ui, &list)[0], "row-0");
    // It serves the requests made after it was given the controller.
    let _ = f.controller.reveal("row-500")?;
    f.ui.flush(Duration::ZERO)?;
    assert!(keys(&f.ui, &list).contains(&"row-500".to_owned()));
    Ok(())
}

#[test]
fn rows_far_past_the_precision_of_f32_keep_distinct_slots() -> TestResult {
    // 20 million rows of 24 pixels are 480 million pixels, where neighbouring
    // `f32` values are 32 pixels apart.
    const COUNT: usize = 20_000_000;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 240.0, 1.0)?)?;
    let items = ui.signal(rows(COUNT));
    let controller = ListController::new(&ui);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), 24.0)?.controller(controller.clone()),
    ))?;
    ui.flush(Duration::ZERO)?;
    let _ = controller.reveal(format!("row-{}", COUNT - 1))?;
    ui.flush(Duration::ZERO)?;
    let top = |ui: &Runtime, index: usize| -> f32 {
        let row = ui
            .children(&list)
            .unwrap()
            .into_iter()
            .find(|id| ui.key(id).unwrap() == format!("row-{index}"))
            .unwrap();
        ui.bounds(&row).unwrap().unwrap().y()
    };
    // The last ten rows fill the viewport one row apart.
    for back in 1..=10 {
        assert_eq!(top(&ui, COUNT - back), 240.0 - 24.0 * back as f32);
    }
    // A one-pixel scroll still moves them by one pixel.
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, -1.0)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(top(&ui, COUNT - 1), 217.0);
    assert_eq!(
        property(&ui, "list", "scroll_y"),
        COUNT as f64 * 24.0 - 240.0 - 1.0
    );
    Ok(())
}

#[test]
fn replacing_the_items_rebuilds_rows_and_clamps_the_offset() -> TestResult {
    let mut f = fixture(1_000, 200.0)?;
    wheel(&mut f.ui, 990.0 * ROW)?;
    assert_eq!(property(&f.ui, "list", "scroll_y"), 990.0 * f64::from(ROW));
    f.items.set(ListSource::new(Rows {
        count: 50,
        generation: 1,
    }))?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(property(&f.ui, "list", "rows"), 50.0);
    assert_eq!(
        property(&f.ui, "list", "scroll_y"),
        50.0 * f64::from(ROW) - 200.0
    );
    let mounted = keys(&f.ui, &f.list);
    assert_eq!(mounted.last().map(String::as_str), Some("row-49"));
    f.items.set(rows(0))?;
    f.ui.flush(Duration::ZERO)?;
    assert!(f.ui.children(&f.list)?.is_empty());
    assert_eq!(property(&f.ui, "list", "scroll_y"), 0.0);
    Ok(())
}

#[test]
fn keyboard_paging_scrolls_the_focused_list() -> TestResult {
    let mut f = fixture(1_000, 200.0)?;
    f.ui.focus(&f.list)?;
    f.ui.key_input(KeyCode::PageDown, KeyAction::Press)?;
    f.ui.key_input(KeyCode::PageDown, KeyAction::Release)?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(property(&f.ui, "list", "scroll_y"), 180.0);
    f.ui.key_input(KeyCode::End, KeyAction::Press)?;
    f.ui.key_input(KeyCode::End, KeyAction::Release)?;
    f.ui.flush(Duration::ZERO)?;
    assert_eq!(
        property(&f.ui, "list", "scroll_y"),
        1_000.0 * f64::from(ROW) - 200.0
    );
    assert_eq!(
        keys(&f.ui, &f.list).last().map(String::as_str),
        Some("row-999")
    );
    Ok(())
}

#[test]
fn describing_the_list_again_keeps_its_mounted_rows_and_their_state() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(rows(1000));
    let describe = |items: &Signal<ListSource>| -> Result<Vec<Element>, GeometryError> {
        Ok(vec![Element::new(
            "list",
            VirtualList::new(items.read_only(), ROW)?,
        )])
    };
    let root = ui.mount_tree(Element::new("root", Column::new(describe(&items)?)))?;
    ui.flush(Duration::ZERO)?;
    let list = ui.children(&root)?[0].clone();
    let before = ui.children(&list)?;
    assert!(!before.is_empty());
    ui.focus(&before[1])?;

    // An ordinary declarative rebuild of the surrounding view.
    ui.reconcile_children(&root, describe(&items)?)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.children(&root)?[0], list);
    assert_eq!(
        ui.children(&list)?,
        before,
        "the rows survive with their identity"
    );
    assert_eq!(ui.focused(), Some(before[1].clone()));

    // Children described for a list that manages its own are rejected.
    let with_rows = Element::new("list", VirtualList::new(items.read_only(), ROW)?)
        .children([Element::new("row-0", Button::new("stray", |_| Ok(())))]);
    let result = ui.reconcile_children(&root, vec![with_rows]);
    assert!(
        matches!(result, Err(RuntimeError::Layout(LayoutError::InvalidChild))),
        "{result:?}"
    );
    assert_eq!(ui.children(&list)?, before);
    Ok(())
}

/// Rows built under one shared key: the list keys each by its position's key.
struct SharedKeyRows(usize);
impl ListItems for SharedKeyRows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        (index < self.0).then(|| Label::new(format!("Row {index}")).key("row"))
    }
}

#[test]
fn the_list_keys_each_built_row_with_its_key() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 3.0 * ROW, 1.0)?)?;
    let items = ui.signal(ListSource::new(SharedKeyRows(10)));
    let list = ui.mount_tree(VirtualList::new(&items, ROW)?.key("list"))?;
    ui.flush(Duration::ZERO)?;
    let keys: Vec<String> = ui
        .children(&list)?
        .iter()
        .map(|row| ui.key(row).map(str::to_owned))
        .collect::<Result<_, _>>()?;
    assert!(!keys.is_empty());
    for (index, key) in keys.iter().enumerate() {
        assert_eq!(*key, format!("row-{index}"));
    }
    Ok(())
}

#[test]
fn a_kept_item_stays_mounted_out_of_range_until_released() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(rows(100));
    let kept = ui.signal(Some("row-0".to_owned()));
    let controller = ListController::new(&ui);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?
            .controller(controller.clone())
            .keep_mounted(kept.read_only()),
    ))?;
    ui.flush(Duration::ZERO)?;
    let _ = controller.reveal("row-99")?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.find_descendant(&list, "row-0")?.is_some(), "kept");
    assert!(ui.find_descendant(&list, "row-1")?.is_none(), "not kept");
    kept.set(None)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.find_descendant(&list, "row-0")?.is_none(), "released");
    // A key the source does not hold keeps nothing.
    kept.set(Some("row-500".to_owned()))?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.find_descendant(&list, "row-500")?.is_none());
    Ok(())
}

#[test]
fn typed_text_reaches_the_focused_list_but_not_from_its_items() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(rows(10));
    let typed = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let sink = typed.clone();
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?
            .selection(ListSelection::new(&ui, SelectionMode::Single))
            .on_text_input(move |_, text| {
                sink.borrow_mut().push_str(text);
                Ok(())
            }),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&list)?;
    ui.text_input("ab")?;
    assert_eq!(*typed.borrow(), "ab");
    let row = ui.find_descendant(&list, "row-0")?.ok_or("mounted")?;
    ui.focus(&row)?;
    ui.text_input("c")?;
    assert_eq!(*typed.borrow(), "ab", "an item's text is its own");
    Ok(())
}
