//! Lanes, orientation and keyed navigation and selection for virtualized
//! collections, through the facade. `virtual_list.rs` keeps the list's original
//! contracts; these are what the shared pieces added.
use pert_ui::*;
mod support;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc, time::Duration};
use support::press;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 40.0;

/// Rows of plain labels, so a press on a row is not handled by its content.
struct Labels(usize);
impl ListItems for Labels {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("item-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(
            self.key(index)?,
            Label::new(format!("Item {index}")),
        ))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("item-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}

fn mounted(
    list: impl FnOnce(ReadSignal<ListSource>) -> Result<VirtualList, RuntimeError>,
    count: usize,
    size: (f32, f32),
) -> Result<(Runtime, WidgetId), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(size.0, size.1, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(count)));
    let id = ui.mount_tree(Element::new("list", list(items.read_only())?))?;
    ui.flush(Duration::ZERO)?;
    Ok((ui, id))
}
fn bounds(ui: &Runtime, list: &WidgetId, key: &str) -> Rect {
    let child = ui
        .children(list)
        .unwrap()
        .into_iter()
        .find(|child| ui.key(child).unwrap() == key)
        .unwrap_or_else(|| panic!("{key} is not mounted"));
    ui.bounds(&child).unwrap().unwrap()
}
fn is_mounted(ui: &Runtime, list: &WidgetId, key: &str) -> bool {
    ui.children(list)
        .unwrap()
        .iter()
        .any(|child| ui.key(child).unwrap() == key)
}
fn held(ui: &mut Runtime, modifier: KeyCode, key: KeyCode) {
    ui.key_input(modifier, KeyAction::Press).unwrap();
    press(ui, key);
    ui.key_input(modifier, KeyAction::Release).unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
fn click(ui: &mut Runtime, x: f32, y: f32) {
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(x, y).unwrap(), kind))
            .unwrap();
    }
    ui.flush(Duration::ZERO).unwrap();
}
/// The selection's keys, shared as `ListSelection::selected` holds them.
fn set(keys: &[&str]) -> Rc<BTreeSet<String>> {
    Rc::new(keys.iter().map(|key| (*key).to_owned()).collect())
}

#[test]
fn lanes_make_a_grid_with_equal_shares_and_whole_lines() -> TestResult {
    let (ui, list) = mounted(
        |items| Ok(VirtualList::new(items, ROW)?.overscan(0).lanes(3)?),
        1_000,
        (300.0, 100.0),
    )?;
    // Three 40-pixel lines touch a 100-pixel viewport: nine items, not three.
    assert_eq!(ui.children(&list)?.len(), 9);
    assert_eq!(
        bounds(&ui, &list, "item-0").values(),
        [0.0, 0.0, 100.0, ROW]
    );
    assert_eq!(
        bounds(&ui, &list, "item-2").values(),
        [200.0, 0.0, 100.0, ROW]
    );
    assert_eq!(
        bounds(&ui, &list, "item-4").values(),
        [100.0, ROW, 100.0, ROW]
    );
    assert_eq!(
        VirtualList::new(ui.signal(ListSource::new(Labels(1))).read_only(), ROW)?
            .lanes(0)
            .err(),
        Some(LayoutError::NoLanes)
    );
    Ok(())
}

#[test]
fn a_horizontal_list_scrolls_along_x_even_from_a_vertical_wheel() -> TestResult {
    let (mut ui, list) = mounted(
        |items| {
            Ok(VirtualList::new(items, 50.0)?
                .overscan(0)
                .orientation(Orientation::Horizontal))
        },
        1_000,
        (200.0, 60.0),
    )?;
    assert_eq!(ui.children(&list)?.len(), 4);
    assert_eq!(
        bounds(&ui, &list, "item-1").values(),
        [50.0, 0.0, 50.0, 60.0]
    );
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, 500.0)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert!(is_mounted(&ui, &list, "item-10") && !is_mounted(&ui, &list, "item-0"));
    assert_eq!(bounds(&ui, &list, "item-10").x(), 0.0);
    Ok(())
}

#[test]
fn navigation_crosses_items_that_are_not_mounted() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(100_000)));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let activated = Rc::new(RefCell::new(Vec::new()));
    let log = activated.clone();
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?
            .selection(selection.clone())
            .on_activate(move |_, key| {
                log.borrow_mut().push(key.to_owned());
                Ok(())
            }),
    ))?;
    ui.flush(Duration::ZERO)?;
    // With a selection the list is a Tab stop even before it has an active item.
    ui.focus(&list)?;
    assert_eq!(selection.active().get()?, None);
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-0"));
    assert_eq!(
        selection.selected().get()?,
        set(&["item-0"]),
        "single follows active"
    );

    press(&mut ui, KeyCode::End);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-99999"));
    assert!(
        is_mounted(&ui, &list, "item-99999"),
        "the active item is mounted"
    );
    assert!(!is_mounted(&ui, &list, "item-0"));
    // It is fully inside the 200-pixel viewport, at its bottom edge.
    assert_eq!(bounds(&ui, &list, "item-99999").y(), 200.0 - ROW);

    // A page is the viewport's five whole lines.
    press(&mut ui, KeyCode::PageUp);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-99994"));
    press(&mut ui, KeyCode::Home);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-0"));
    assert_eq!(bounds(&ui, &list, "item-0").y(), 0.0);
    // The ends hold, and a single lane leaves the cross arrows to an owner.
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-0"));
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-0"));

    press(&mut ui, KeyCode::Enter);
    assert_eq!(*activated.borrow(), ["item-0"]);
    assert_eq!(ui.next_deadline(), None, "navigation schedules nothing");
    Ok(())
}

#[test]
fn a_grid_moves_by_item_across_and_by_line_along() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(100)));
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let list = ui.mount_tree(Element::new(
        "grid",
        VirtualList::new(items.read_only(), ROW)?
            .lanes(4)?
            .selection(selection.clone()),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&list)?;
    let active = |selection: &ListSelection| selection.active().get().unwrap().unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(active(&selection), "item-0");
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(active(&selection), "item-1");
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(active(&selection), "item-5", "one line is four items");
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(active(&selection), "item-4");
    Ok(())
}

#[test]
fn multiple_selection_toggles_extends_and_moves_without_selecting() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(50)));
    let selection = ListSelection::new(&ui, SelectionMode::Multiple);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?.selection(selection.clone()),
    ))?;
    ui.flush(Duration::ZERO)?;
    let selected = |selection: &ListSelection| selection.selected().get().unwrap();

    // A press on a row its content did not handle selects it and focuses the list.
    click(&mut ui, 20.0, ROW * 2.0 + 5.0);
    assert_eq!(selected(&selection), set(&["item-2"]));
    assert_eq!(ui.focused(), Some(list.clone()));

    // Shift extends from the anchor, whichever direction, and replaces the rest.
    held(&mut ui, KeyCode::ShiftLeft, KeyCode::ArrowDown);
    held(&mut ui, KeyCode::ShiftLeft, KeyCode::ArrowDown);
    assert_eq!(selected(&selection), set(&["item-2", "item-3", "item-4"]));
    held(&mut ui, KeyCode::ShiftLeft, KeyCode::Home);
    assert_eq!(selected(&selection), set(&["item-0", "item-1", "item-2"]));

    // Control moves the active item and leaves the selection alone; Space toggles.
    held(&mut ui, KeyCode::ControlLeft, KeyCode::End);
    assert_eq!(selection.active().get()?.as_deref(), Some("item-49"));
    assert_eq!(selected(&selection), set(&["item-0", "item-1", "item-2"]));
    press(&mut ui, KeyCode::Space);
    assert!(selection.is_selected("item-49")?);
    press(&mut ui, KeyCode::Space);
    assert!(!selection.is_selected("item-49")?);

    // A plain move replaces the selection with the active item.
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(selected(&selection), set(&["item-48"]));
    Ok(())
}

#[test]
fn a_single_selection_is_the_active_item_whichever_is_written() -> TestResult {
    let ui = Runtime::new();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    selection.select(["a".to_owned(), "b".to_owned()])?;
    assert_eq!(selection.active().get()?.as_deref(), Some("a"));
    assert_eq!(selection.selected().get()?, set(&["a"]));
    selection.set_active(Some("b".into()))?;
    assert_eq!(
        selection.selected().get()?,
        set(&["b"]),
        "follows the active item"
    );
    selection.set_active(None)?;
    assert_eq!(selection.selected().get()?, set(&[]));
    selection.select(["c".to_owned()])?;
    selection.select(Vec::new())?;
    assert_eq!(selection.active().get()?, None);
    Ok(())
}

#[test]
fn a_rejected_write_leaves_the_range_anchor_where_it_was() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 400.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(10)));
    let selection = ListSelection::new(&ui, SelectionMode::Multiple);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?.selection(selection.clone()),
    ))?;
    selection.set_active(Some("item-2".into()))?;
    ui.flush(Duration::ZERO)?;
    // A derived binding evaluates where writes are refused.
    let writer = selection.clone();
    let refused = ui
        .signal(0_u8)
        .read_only()
        .map(move |_| writer.set_active(Some("item-7".into())))?;
    assert_eq!(refused.get()?, Err(RuntimeError::WriteDuringUpdate));
    assert_eq!(selection.active().get()?.as_deref(), Some("item-2"));
    ui.focus(&list)?;
    held(&mut ui, KeyCode::ShiftLeft, KeyCode::ArrowDown);
    assert_eq!(
        selection.selected().get()?,
        set(&["item-2", "item-3"]),
        "the range starts at the anchor the refused write did not move"
    );
    Ok(())
}

#[test]
fn the_selection_is_application_state_that_the_list_draws_and_never_prunes() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Labels(10)));
    let selection = ListSelection::new(&ui, SelectionMode::Multiple);
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::new(items.read_only(), ROW)?.selection(selection.clone()),
    ))?;
    ui.flush(Duration::ZERO)?;
    let fill = |ui: &mut Runtime| {
        let theme = ui.theme();
        let color = theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(ColorRole::TabSelected);
        ui.paint()
            .unwrap()
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color: c } if *c == color => Some(rect.y()),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert!(fill(&mut ui).is_empty());
    // The application writes the selection; the list draws it beneath the rows.
    selection.select(["item-1".to_owned(), "item-3".to_owned(), "gone".to_owned()])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(fill(&mut ui), [ROW, ROW * 3.0]);
    // The list holds the selection's own set, not a copy of it: the signal,
    // the list and this read share one allocation.
    assert_eq!(Rc::strong_count(&selection.selected().get()?), 3);

    // A smaller source drops rows from view; the selection is left as it was.
    items.set(ListSource::new(Labels(2)))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(fill(&mut ui), [ROW]);
    assert_eq!(
        selection.selected().get()?,
        set(&["gone", "item-1", "item-3"])
    );

    let semantics = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "list")
        .and_then(|component| component.details.semantics)
        .unwrap();
    assert_eq!(semantics.role, SemanticRole::List);
    assert_eq!(
        semantics.value,
        Some(SemanticValue::Choice {
            selected: None,
            count: 2,
            label: None,
        })
    );
    assert!(semantics.items.is_empty(), "rows are never enumerated");
    let _ = list;
    Ok(())
}

#[test]
fn neighboring_lanes_share_an_exact_edge_at_awkward_widths() -> TestResult {
    // Widths that do not divide evenly are where a rounded lane width, multiplied
    // back, makes neighbors overlap by a fraction of a pixel.
    for width in [616.0, 333.0, 1001.0, 97.5] {
        let (ui, list) = mounted(
            |items| Ok(VirtualList::new(items, ROW)?.overscan(0).lanes(5)?),
            50,
            (width, 80.0),
        )?;
        for lane in 0..4 {
            let left = bounds(&ui, &list, &format!("item-{lane}"));
            let right = bounds(&ui, &list, &format!("item-{}", lane + 1));
            assert_eq!(
                left.x() + left.width(),
                right.x(),
                "width {width}, lane {lane}"
            );
        }
        let last = bounds(&ui, &list, "item-4");
        assert!(
            (last.x() + last.width() - width).abs() < 0.001,
            "width {width}"
        );
        assert!(ui.validation_report().issues.is_empty(), "width {width}");
    }
    Ok(())
}

/// A row whose height depends on its position: 20, 40 or 60 logical pixels.
struct Tall(f32);
impl Component for Tall {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(a.width().unwrap_or(10.0), self.0)?)
    }
}
struct Mixed(usize);
impl Mixed {
    fn height(index: usize) -> f32 {
        20.0 + (index % 3) as f32 * 20.0
    }
}
impl ListItems for Mixed {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        Some(Element::new(self.key(index)?, Tall(Self::height(index))))
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        key.strip_prefix("row-")?
            .parse()
            .ok()
            .filter(|index| *index < self.0)
    }
}
fn wheel(ui: &mut Runtime, delta: f32) {
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, delta).unwrap()),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
}
fn mounted_rows(ui: &Runtime, list: &WidgetId) -> Vec<(usize, Rect)> {
    let mut rows: Vec<_> = ui
        .children(list)
        .unwrap()
        .iter()
        .map(|child| {
            let key = ui.key(child).unwrap().to_owned();
            let index = key.strip_prefix("row-").unwrap().parse().unwrap();
            (index, ui.bounds(child).unwrap().unwrap())
        })
        .collect();
    rows.sort_by_key(|(index, _)| *index);
    rows
}

#[test]
fn estimated_rows_take_their_own_heights_and_stack_without_gaps() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Mixed(3_000)));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::estimated(items.read_only(), 30.0)?.overscan(1),
    ))?;
    ui.flush(Duration::ZERO)?;
    let rows = mounted_rows(&ui, &list);
    assert_eq!(rows[0].1.y(), 0.0);
    for pair in rows.windows(2) {
        let ((index, above), (_, below)) = (&pair[0], &pair[1]);
        assert_eq!(above.height(), Mixed::height(*index), "row {index}");
        assert_eq!(above.y() + above.height(), below.y(), "after row {index}");
    }
    // Far fewer than 3,000 rows are mounted, and the list settled.
    assert!(rows.len() < 20, "{} rows", rows.len());
    assert_eq!(ui.next_deadline(), None);

    // Jumping to the end lands on the real last row, flush with the viewport.
    ui.focus(&list)?;
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::End);
    let rows = mounted_rows(&ui, &list);
    let (last, bounds) = rows.last().unwrap();
    assert_eq!(*last, 2_999);
    assert!(
        (bounds.y() + bounds.height() - 200.0).abs() < 0.01,
        "{bounds:?}"
    );
    Ok(())
}

#[test]
fn rows_measured_above_the_viewport_do_not_move_what_is_on_screen() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Mixed(3_000)));
    let list = ui.mount_tree(Element::new(
        "list",
        VirtualList::estimated(items.read_only(), 30.0)?.overscan(0),
    ))?;
    ui.flush(Duration::ZERO)?;
    // Jump far down in one step, so every row above is still an estimate.
    wheel(&mut ui, 30_000.0);
    for _ in 0..40 {
        let before = mounted_rows(&ui, &list);
        // Follow a row in the middle of the viewport through a small scroll up,
        // which mounts and measures rows above it whose heights differ from the
        // estimate. It must move by the scroll and by nothing else.
        let (tracked, was) = before[before.len() / 2];
        wheel(&mut ui, -7.0);
        let after = mounted_rows(&ui, &list);
        let (_, now) = after
            .iter()
            .find(|(index, _)| *index == tracked)
            .copied()
            .expect("the tracked row stays mounted");
        assert!(
            (now.y() - (was.y() + 7.0)).abs() < 0.01,
            "row {tracked} moved from {} to {}",
            was.y(),
            now.y()
        );
    }
    Ok(())
}
