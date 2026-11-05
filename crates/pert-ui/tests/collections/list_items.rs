//! `VirtualList::report_items` and `ListItems::semantic_item`: a list reports
//! its rows as semantic items with the rectangles it placed them in, keyed by
//! row identity, absent where the rows do not describe the adopted source,
//! and recovering after rejected rows without a new source.
use crate::support;
use pert_ui::component::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{ContentField, ContentQuery, ContentTarget, GeometryError, LayoutError};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Each row's slot along the axis; rows are shorter, so a slot and its row
/// coincide.
const ROW: f32 = 40.0;

/// What the rows do wrong, shared by a source and the test.
#[derive(Default)]
struct Faults {
    /// Keys whose row is built with two children under one key, which the
    /// runtime rejects when it admits the row.
    rejected: RefCell<HashSet<String>>,
    /// Keys whose row is not built at all.
    missing: RefCell<HashSet<String>>,
    /// Every row fails its arrangement while set.
    arrange: Cell<bool>,
}

/// A row that fills its slot and fails to arrange while its faults say so.
struct Probe(Rc<Faults>);
impl Component for Probe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(ROW),
            available.height().unwrap_or(ROW / 2.0).min(ROW),
        )?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if self.0.arrange.get() {
            Err(LayoutError::InvalidChild.into())
        } else {
            Ok(())
        }
    }
}

/// Rows by key, each described as an item named by its key in capitals.
struct Keys {
    keys: Vec<String>,
    faults: Rc<Faults>,
    /// Whether rows offer searchable text as well.
    text: bool,
}
impl ListItems for Keys {
    fn len(&self) -> usize {
        self.keys.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.keys.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        let key = self.keys.get(index)?;
        if self.faults.missing.borrow().contains(key) {
            return None;
        }
        if self.faults.rejected.borrow().contains(key) {
            return Some(
                Element::new(
                    "row",
                    Column::new([Label::new("One").key("same"), Label::new("Two").key("same")]),
                )
                .into(),
            );
        }
        Some(Probe(self.faults.clone()).into())
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.keys.iter().position(|candidate| candidate == key)
    }
    fn text(&self, index: usize) -> Option<String> {
        self.text
            .then(|| self.keys.get(index).map(|key| format!("{key} text")))
            .flatten()
    }
    fn semantic_item(&self, index: usize) -> Option<SemanticItem> {
        let key = self.keys.get(index)?;
        Some(SemanticItem::new(key.to_uppercase()).action(SemanticAction::Activate))
    }
}

fn keys(names: impl IntoIterator<Item = usize>) -> Vec<String> {
    names.into_iter().map(|index| format!("r{index}")).collect()
}

struct Fx {
    ui: Runtime,
    items: Signal<ListSource>,
    faults: Rc<Faults>,
    list: WidgetId,
}
impl Fx {
    /// A vertical, fixed-extent list of `count` rows, three rows tall,
    /// without overscan unless `overscan` says otherwise.
    fn new(count: usize, overscan: usize) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with(count, |items| {
            Ok(VirtualList::new(items, ROW)?
                .overscan(overscan)
                .report_items(SemanticRole::ListBox))
        })
    }
    fn with(
        count: usize,
        list: impl FnOnce(ReadSignal<ListSource>) -> Result<VirtualList, Box<dyn std::error::Error>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 3.0 * ROW, 1.0)?)?;
        let faults = Rc::new(Faults::default());
        let items = ui.signal(ListSource::new(Keys {
            keys: keys(0..count),
            faults: faults.clone(),
            text: false,
        }));
        let list = ui.mount_tree(list(items.read_only())?.key("list"))?;
        ui.flush(Duration::ZERO)?;
        Ok(Self {
            ui,
            items,
            faults,
            list,
        })
    }
    fn replace(&self, names: Vec<String>) -> TestResult {
        self.items.set(ListSource::new(Keys {
            keys: names,
            faults: self.faults.clone(),
            text: false,
        }))?;
        Ok(())
    }
    fn semantics(&self) -> Semantics {
        support::semantics_of(&self.ui, &self.list).expect("list semantics")
    }
    /// The item named after `key`, with its index.
    fn item(&self, key: &str) -> Option<(usize, SemanticItem)> {
        let name = key.to_uppercase();
        self.semantics()
            .items
            .into_iter()
            .enumerate()
            .find(|(_, item)| item.name.as_deref() == Some(name.as_str()))
    }
    fn bounds(&self, key: &str) -> Option<[f32; 4]> {
        self.item(key).and_then(|(_, item)| item.bounds)
    }
    /// The mounted row's border box cut to the list's own rectangle.
    fn row_bounds(&self, key: &str) -> Option<[f32; 4]> {
        let row = self.ui.find_descendant(&self.list, key).ok()??;
        let row = self.ui.bounds(&row).ok()??;
        let list = self.ui.bounds(&self.list).ok()??;
        let visible = row.intersection(list);
        (!visible.is_empty()).then(|| visible.values())
    }
    fn row(&self, key: &str) -> Option<WidgetId> {
        self.ui.find_descendant(&self.list, key).unwrap()
    }
    /// Scrolls the list by `dy` logical pixels with the wheel, returning the
    /// flush's outcome.
    fn wheel(&mut self, dy: f32) -> Result<(), RuntimeError> {
        let [x, y, w, h] = self
            .ui
            .bounds(&self.list)?
            .ok_or(RuntimeError::Hidden)?
            .values();
        let at = Point::new(x + w / 2.0, y + h / 2.0)?;
        self.ui.pointer_input(PointerEvent::new(
            at,
            PointerKind::Scroll(Point::new(0.0, dy)?),
        ))?;
        self.ui.flush(self.ui.now()).map(|_| ())
    }
}

fn reject(fx: &Fx, key: &str) {
    fx.faults.rejected.borrow_mut().insert(key.to_owned());
}

#[test]
fn placed_rows_give_their_items_the_visible_part_of_their_slots() -> TestResult {
    let mut fx = Fx::new(100, 2)?;
    let semantics = fx.semantics();
    assert_eq!(semantics.role, SemanticRole::ListBox);
    assert_eq!(semantics.items.len(), 100, "one item per row of the source");
    assert_eq!(semantics.items[5].name.as_deref(), Some("R5"));
    assert_eq!(semantics.items[5].actions, [SemanticAction::Activate]);
    // The list keeps its value and actions.
    assert!(matches!(
        semantics.value,
        Some(SemanticValue::Number { value, .. }) if value == 100.0
    ));
    assert!(semantics.actions.contains(&SemanticAction::Scroll));
    for key in ["r0", "r1", "r2"] {
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
        assert!(fx.bounds(key).is_some(), "{key}");
    }
    // Overscan rows are mounted but out of view; unmounted rows have no row.
    assert!(fx.row("r4").is_some());
    assert_eq!(fx.bounds("r4"), None);
    assert_eq!(fx.bounds("r50"), None);

    // Half a row down: the first and the fourth row are partly visible.
    fx.wheel(ROW / 2.0)?;
    let [_, top, _, height] = fx.bounds("r0").ok_or("partly visible")?;
    assert_eq!((top, height), (0.0, ROW / 2.0));
    assert_eq!(fx.bounds("r3"), fx.row_bounds("r3"));
    assert_eq!(fx.bounds("r3").map(|[_, _, _, h]| h), Some(ROW / 2.0));

    // Far down, a later row's rectangle is its row's.
    fx.wheel(40.0 * ROW)?;
    let (_, item) = fx.item("r41").ok_or("listed")?;
    assert!(item.bounds.is_some());
    assert_eq!(item.bounds, fx.row_bounds("r41"));
    assert_eq!(fx.bounds("r0"), None);
    Ok(())
}

#[test]
fn a_default_list_reports_no_items_and_inspecting_one_that_does_schedules_nothing() -> TestResult {
    let fx = Fx::with(10, |items| Ok(VirtualList::new(items, ROW)?))?;
    let semantics = fx.semantics();
    assert_eq!(semantics.role, SemanticRole::List);
    assert!(semantics.items.is_empty());

    let mut fx = Fx::new(10, 0)?;
    for _ in 0..3 {
        assert_eq!(fx.semantics().items.len(), 10);
    }
    let report = fx.ui.flush(fx.ui.now())?;
    assert_eq!(report.components_updated, 0, "inspection schedules nothing");
    assert_eq!(fx.ui.next_deadline(), None);
    Ok(())
}

#[test]
fn each_description_sets_replaces_or_removes_the_opt_in() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 3.0 * ROW, 1.0)?)?;
    let faults = Rc::new(Faults::default());
    let items = ui.signal(ListSource::new(Keys {
        keys: keys(0..5),
        faults,
        text: false,
    }));
    let describe = |role: Option<SemanticRole>| -> Result<Vec<Element>, GeometryError> {
        let list = VirtualList::new(items.read_only(), ROW)?;
        let list = match role {
            Some(role) => list.report_items(role),
            None => list,
        };
        Ok(vec![list.key("list")])
    };
    let root = ui.mount_tree(Column::new(describe(None)?).key("root"))?;
    ui.flush(Duration::ZERO)?;
    let list = ui.find(&root, &["list"])?.ok_or("list")?;
    let semantics = |ui: &Runtime| support::semantics_of(ui, &list).expect("semantics");
    assert!(semantics(&ui).items.is_empty());

    ui.reconcile_children(&root, describe(Some(SemanticRole::ListBox))?)?;
    ui.flush(ui.now())?;
    assert_eq!(semantics(&ui).role, SemanticRole::ListBox);
    assert_eq!(semantics(&ui).items.len(), 5);
    // Records are kept without the opt-in, so the first inspection after it
    // already has rectangles.
    assert!(semantics(&ui).items[0].bounds.is_some());

    ui.reconcile_children(&root, describe(Some(SemanticRole::Menu))?)?;
    ui.flush(ui.now())?;
    assert_eq!(semantics(&ui).role, SemanticRole::Menu);
    assert_eq!(semantics(&ui).items.len(), 5);

    ui.reconcile_children(&root, describe(None)?)?;
    ui.flush(ui.now())?;
    assert_eq!(semantics(&ui).role, SemanticRole::List);
    assert!(semantics(&ui).items.is_empty());
    // The same list throughout.
    assert_eq!(ui.find(&root, &["list"])?, Some(list));
    Ok(())
}

#[test]
fn rectangles_follow_row_keys_across_reorder_insert_and_removal() -> TestResult {
    let fx = Fx::new(6, 0)?;
    let before = fx.bounds("r0");
    assert!(before.is_some());
    // Reversed: the row that was first is now last and out of view.
    fx.replace(keys((0..6).rev()))?;
    let mut fx = fx;
    fx.ui.flush(fx.ui.now())?;
    assert_eq!(fx.item("r5").map(|(index, _)| index), Some(0));
    assert_eq!(fx.bounds("r5"), before, "the new first row");
    assert_eq!(fx.bounds("r5"), fx.row_bounds("r5"));
    assert_eq!(fx.bounds("r0"), None, "no rectangle inherited by position");
    // An insert at the front and a removal move the others by key.
    fx.replace(
        ["new".to_owned()]
            .into_iter()
            .chain(keys([5, 3, 2]))
            .collect(),
    )?;
    fx.ui.flush(fx.ui.now())?;
    for key in ["new", "r5", "r3"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    assert!(fx.item("r4").is_none(), "removed rows are not items");
    Ok(())
}

#[test]
fn a_row_that_leaves_and_returns_is_a_new_row_with_its_own_rectangle() -> TestResult {
    let mut fx = Fx::new(20, 0)?;
    let first = fx.row("r0").ok_or("mounted")?;
    let before = fx.bounds("r0");
    fx.wheel(5.0 * ROW)?;
    assert_eq!(fx.row("r0"), None, "unmounted");
    assert_eq!(fx.bounds("r0"), None);
    // A newly mounted row has its own placement.
    assert!(fx.bounds("r6").is_some());
    assert_eq!(fx.bounds("r6"), fx.row_bounds("r6"));
    fx.wheel(-5.0 * ROW)?;
    let again = fx.row("r0").ok_or("mounted again")?;
    assert_ne!(again, first, "built again");
    assert_eq!(fx.bounds("r0"), fx.row_bounds("r0"));
    assert_eq!(fx.bounds("r0"), before);
    Ok(())
}

#[test]
fn a_row_that_is_not_built_leaves_no_rectangles_until_the_rows_are_built() -> TestResult {
    let mut fx = Fx::new(10, 0)?;
    fx.faults.missing.borrow_mut().insert("r1".to_owned());
    // The new source is adopted, then building its rows fails.
    fx.replace(keys(0..10))?;
    assert!(matches!(
        fx.ui.flush(fx.ui.now()),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    ));
    let semantics = fx.semantics();
    assert_eq!(semantics.items.len(), 10);
    assert!(semantics.items.iter().all(|item| item.bounds.is_none()));
    // Once the row can be built, the next update and arrangement restore them.
    fx.faults.missing.borrow_mut().clear();
    fx.replace(keys(0..10))?;
    fx.ui.flush(fx.ui.now())?;
    for key in ["r0", "r1", "r2"] {
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
        assert!(fx.bounds(key).is_some(), "{key}");
    }
    // With the same source, a row entering the view that is not built
    // leaves every row without a rectangle while construction is unfinished.
    fx.faults.missing.borrow_mut().insert("r3".to_owned());
    assert!(fx.wheel(ROW / 2.0).is_err());
    assert!(
        fx.semantics()
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    fx.faults.missing.borrow_mut().clear();
    fx.wheel(ROW / 4.0)?;
    assert!(fx.bounds("r3").is_some());
    assert_eq!(fx.bounds("r1"), fx.row_bounds("r1"));
    Ok(())
}

#[test]
fn rejected_rows_keep_surviving_rectangles_and_the_list_retries_without_a_new_source() -> TestResult
{
    // Half a row down, every row stays and the fourth enters; its row is
    // built but rejected when the runtime admits it.
    let mut fx = Fx::new(10, 0)?;
    let before: Vec<_> = ["r0", "r1", "r2"].map(|key| fx.row(key)).into();
    reject(&fx, "r3");
    let failed = fx.wheel(ROW / 2.0);
    assert!(failed.is_err(), "{failed:?}");
    // Inspected right after the failure: the surviving rows keep their
    // rectangles, which are their rows' committed boxes, and the rejected
    // row has none.
    assert_eq!(fx.row("r3"), None);
    for key in ["r0", "r1", "r2"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    assert_eq!(fx.bounds("r3"), None);

    // Once the row can be admitted, the list requests it again by itself:
    // the source and the wanted rows did not change.
    fx.faults.rejected.borrow_mut().clear();
    fx.ui.flush(fx.ui.now())?;
    assert!(fx.row("r3").is_some(), "requested again and admitted");
    assert!(fx.bounds("r3").is_some());
    assert_eq!(fx.bounds("r3"), fx.row_bounds("r3"));
    let after: Vec<_> = ["r0", "r1", "r2"].map(|key| fx.row(key)).into();
    assert_eq!(after, before, "surviving rows keep their components");
    Ok(())
}

#[test]
fn a_rejected_request_that_drops_a_row_hides_it_and_recovers() -> TestResult {
    // One row down: the first row leaves and the fourth enters, rejected.
    let mut fx = Fx::new(10, 0)?;
    reject(&fx, "r3");
    assert!(fx.wheel(ROW).is_err());
    // The first row is still mounted but no longer requested: no rectangle,
    // whatever its committed box.
    assert!(fx.row("r0").is_some());
    assert_eq!(fx.bounds("r0"), None);
    assert!(fx.bounds("r1").is_some());
    assert_eq!(fx.bounds("r1"), fx.row_bounds("r1"));
    assert_eq!(fx.bounds("r3"), None);
    // While the row is still rejected, each flush retries and fails again.
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    assert_eq!(fx.bounds("r3"), None);
    fx.faults.rejected.borrow_mut().clear();
    fx.ui.flush(fx.ui.now())?;
    assert_eq!(fx.row("r0"), None, "unmounted by the retried request");
    for key in ["r1", "r2", "r3"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    Ok(())
}

#[test]
fn rows_rejected_for_a_replaced_source_have_no_rectangles() -> TestResult {
    let mut fx = Fx::new(10, 0)?;
    reject(&fx, "new");
    // A replaced source with a new row in view, which is rejected: the rows
    // left mounted still show the previous source.
    fx.replace(["new".to_owned()].into_iter().chain(keys(0..10)).collect())?;
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    let semantics = fx.semantics();
    assert_eq!(semantics.items.len(), 11);
    assert!(semantics.items.iter().all(|item| item.bounds.is_none()));
    // A retry that fails again lays out the old rows, still without
    // rectangles.
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    assert!(fx.row("r0").is_some());
    assert!(
        fx.semantics()
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    fx.faults.rejected.borrow_mut().clear();
    fx.ui.flush(fx.ui.now())?;
    for key in ["new", "r0", "r1"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    Ok(())
}

#[test]
fn a_rejected_description_of_the_same_rows_is_retried() -> TestResult {
    let mut fx = Fx::new(10, 0)?;
    let first = fx.row("r1").ok_or("mounted")?;
    reject(&fx, "r1");
    // A new source with the same keys describes the mounted rows again; the
    // runtime rejects the new description of one of them.
    fx.replace(keys(0..10))?;
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    assert_eq!(fx.row("r1"), Some(first.clone()), "the previous row stays");
    assert!(
        fx.semantics()
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    // The same children are mounted, yet the request was not adopted: a
    // retry that fails again still reports no rectangles.
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    assert!(
        fx.semantics()
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    fx.faults.rejected.borrow_mut().clear();
    fx.ui.flush(fx.ui.now())?;
    for key in ["r0", "r1", "r2"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    Ok(())
}

#[test]
fn a_failed_arrangement_reports_no_rectangles_until_a_layout_succeeds() -> TestResult {
    let mut fx = Fx::new(10, 0)?;
    assert!(fx.bounds("r1").is_some());
    fx.faults.arrange.set(true);
    // A new width arranges the list and its rows again.
    fx.ui.set_viewport(Viewport::new(280.0, 3.0 * ROW, 1.0)?)?;
    assert!(fx.ui.flush(fx.ui.now()).is_err());
    assert!(
        fx.semantics()
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    fx.faults.arrange.set(false);
    fx.ui.flush(fx.ui.now())?;
    assert_eq!(fx.bounds("r1"), fx.row_bounds("r1"));
    assert_eq!(fx.bounds("r1").map(|[_, _, w, _]| w), Some(280.0));
    Ok(())
}

#[test]
fn estimated_horizontal_and_multi_lane_lists_report_their_slots() -> TestResult {
    // Estimated: rows measure ROW / 2 and the slots follow the measurement.
    let fx = Fx::with(10, |items| {
        Ok(VirtualList::estimated(items, ROW)?
            .overscan(0)
            .report_items(SemanticRole::ListBox))
    })?;
    for key in ["r0", "r3", "r5"] {
        assert!(fx.bounds(key).is_some(), "{key}");
        assert_eq!(fx.bounds(key), fx.row_bounds(key), "{key}");
    }
    assert_eq!(
        fx.bounds("r1").map(|[_, y, _, h]| (y, h)),
        Some((ROW / 2.0, ROW / 2.0))
    );

    // Horizontal: slots ROW wide across the list's height.
    let fx = Fx::with(20, |items| {
        Ok(VirtualList::new(items, ROW)?
            .orientation(Orientation::Horizontal)
            .overscan(0)
            .report_items(SemanticRole::ListBox))
    })?;
    assert_eq!(
        fx.bounds("r2").map(|[x, _, w, _]| (x, w)),
        Some((2.0 * ROW, ROW))
    );
    assert_eq!(fx.bounds("r2"), fx.row_bounds("r2"));
    assert_eq!(fx.bounds("r19"), None);

    // Three lanes: each item a third of the width, three to a line.
    let fx = Fx::with(30, |items| {
        Ok(VirtualList::new(items, ROW)?
            .lanes(3)?
            .overscan(0)
            .report_items(SemanticRole::ListBox))
    })?;
    assert_eq!(
        fx.bounds("r4").map(|[x, y, w, _]| (x, y, w)),
        Some((100.0, ROW, 100.0))
    );
    assert_eq!(fx.bounds("r4"), fx.row_bounds("r4"));
    assert_eq!(fx.bounds("r8"), fx.row_bounds("r8"));
    Ok(())
}

#[test]
fn a_row_kept_mounted_out_of_view_has_no_rectangle() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 3.0 * ROW, 1.0)?)?;
    let items = ui.signal(ListSource::new(Keys {
        keys: keys(0..30),
        faults: Rc::new(Faults::default()),
        text: false,
    }));
    let keep = ui.signal(Some("r0".to_owned()));
    let list = ui.mount_tree(
        VirtualList::new(&items, ROW)?
            .overscan(0)
            .keep_mounted(keep.read_only())
            .report_items(SemanticRole::ListBox)
            .key("list"),
    )?;
    ui.flush(Duration::ZERO)?;
    let mut fx = Fx {
        ui,
        items,
        faults: Rc::new(Faults::default()),
        list,
    };
    fx.wheel(10.0 * ROW)?;
    assert!(fx.row("r0").is_some(), "kept mounted");
    assert_eq!(fx.bounds("r0"), None, "its slot is out of view");
    assert_eq!(fx.bounds("r11"), fx.row_bounds("r11"));
    assert!(fx.bounds("r11").is_some());
    let _ = keep;
    Ok(())
}

/// Shows its one child at twice its size, 10 by 5 pixels in.
struct Scaled(Option<Element>);
impl Component for Scaled {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        ViewTransform::new(ViewScale::new(2.0).unwrap(), Point::new(0.0, 0.0).unwrap())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(500.0, 300.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(10.0, 5.0, 200.0, 3.0 * ROW)?)
    }
}

#[test]
fn a_view_transform_scales_and_moves_the_items() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    let items = ui.signal(ListSource::new(Keys {
        keys: keys(0..10),
        faults: Rc::new(Faults::default()),
        text: false,
    }));
    let root = ui.mount_tree(
        Scaled(Some(
            VirtualList::new(&items, ROW)?
                .overscan(0)
                .report_items(SemanticRole::ListBox)
                .key("list"),
        ))
        .key("frame"),
    )?;
    ui.flush(Duration::ZERO)?;
    let list = support::widget(&ui, &root, "list");
    let item = support::semantics_of(&ui, &list).ok_or("semantics")?.items[1]
        .bounds
        .ok_or("bounds")?;
    let row = ui
        .bounds(&ui.find_descendant(&list, "r1")?.ok_or("row")?)?
        .ok_or("laid out")?;
    assert_eq!(item, row.values());
    assert_eq!((item[2], item[3]), (400.0, 2.0 * ROW), "scaled twice");
    Ok(())
}

/// A picker-like composite that builds its own list and exposes it, using
/// only the public extension points the built-in pickers use.
struct Chooser {
    items: Signal<ListSource>,
    chosen: Rc<Cell<Option<usize>>>,
    described: bool,
}
/// Rows that are buttons recording their index when clicked.
struct Buttons(Rc<Cell<Option<usize>>>);
impl ListItems for Buttons {
    fn len(&self) -> usize {
        4
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 4).then(|| format!("b{index}"))
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        let chosen = self.0.clone();
        (index < 4).then(|| {
            Button::new(format!("Button {index}"), move |_| {
                chosen.set(Some(index));
                Ok(())
            })
            .into()
        })
    }
    fn semantic_item(&self, index: usize) -> Option<SemanticItem> {
        (index < 4).then(|| SemanticItem::new(format!("Option {index}")))
    }
}
impl Component for Chooser {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !self.described {
            self.described = true;
            let _ = &self.chosen;
            let list =
                VirtualList::new(self.items.read_only(), ROW)?.report_items(SemanticRole::ListBox);
            cx.request_children(vec![
                Element::new("caption", Label::new("Pick one")).structural(),
                Element::new("list", list)
                    .structural()
                    .with_label("Choices")
                    .expose_part("options"),
            ]);
        }
        Ok(())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() == 2 {
            cx.place(0, Rect::new(0.0, 0.0, size.width(), ROW)?)?;
            cx.place(
                1,
                Rect::new(0.0, ROW, size.width(), (size.height() - ROW).max(0.0))?,
            )?;
        }
        Ok(())
    }
}

#[test]
fn an_external_composite_exposes_its_list_and_clicks_its_items() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 4.0 * ROW, 1.0)?)?;
    let chosen = Rc::new(Cell::new(None));
    let items = ui.signal(ListSource::new(Buttons(chosen.clone())));
    let chooser = ui.mount_tree(
        Element::new(
            "chooser",
            Chooser {
                items,
                chosen: chosen.clone(),
                described: false,
            },
        )
        .structural(),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let options = driver.exposed_part(&chooser, "options")?;
    let semantics = driver.semantics(&options)?;
    assert_eq!(semantics.name.as_deref(), Some("Choices"));
    assert_eq!(semantics.items[2].name.as_deref(), Some("Option 2"));
    driver.click_item(&options, 2)?;
    assert_eq!(chosen.get(), Some(2));
    // The fourth row is below the composite's allocation: no rectangle.
    assert!(matches!(
        driver.click_item(&options, 3),
        Err(TestError::ItemWithoutBounds { index: 3, .. })
    ));
    Ok(())
}

#[test]
fn an_unmounted_row_with_text_gives_an_item_hit_and_a_row_hit() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 3.0 * ROW, 1.0)?)?;
    let items = ui.signal(ListSource::new(Keys {
        keys: keys(0..50),
        faults: Rc::new(Faults::default()),
        text: true,
    }));
    let list = ui.mount_tree(
        Element::new(
            "list",
            VirtualList::new(&items, ROW)?
                .overscan(0)
                .report_items(SemanticRole::ListBox),
        )
        .with_label("Records"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.find_descendant(&list, "r40")?, None, "not mounted");
    let mut search = ui.content_search(ContentQuery::new("r40")?);
    let mut hits = Vec::new();
    loop {
        let step = search.step(&ui)?;
        hits.extend(step.hits);
        if step.done {
            break;
        }
    }
    assert_eq!(hits.len(), 2, "{hits:?}");
    let item = hits
        .iter()
        .find(|hit| hit.field == ContentField::Item { index: 40 })
        .ok_or("the item hit")?;
    assert_eq!(item.target, ContentTarget::Widget(list.clone()));
    assert_eq!(item.text, "R40");
    assert_eq!(item.role, Some(SemanticRole::ListBox));
    assert_eq!(item.other.as_deref(), Some("Records"));
    let row = hits
        .iter()
        .find(|hit| hit.field == ContentField::Row)
        .ok_or("the row hit")?;
    assert_eq!(
        row.target,
        ContentTarget::Row {
            collection: list.clone(),
            key: "r40".to_owned(),
        }
    );
    assert_eq!(row.text, "r40 text");
    assert_eq!(row.role, None);
    assert!(!row.visible);
    Ok(())
}
