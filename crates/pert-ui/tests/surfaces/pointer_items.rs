//! Item rectangles exercise real pointer input through public composite parts.
use pert_ui::testing::{Driver, TestError};
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime(width: f32) -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 300.0, 1.0)?)?;
    Ok(ui)
}
fn tab(key: &str) -> Tab {
    Tab::new(key, format!("Long tab {key}"), Label::new(key).key("body"))
}
fn items(ui: &Runtime, id: &WidgetId) -> Vec<SemanticItem> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.id == ui.inspection_id(id).unwrap())
        .and_then(|component| component.details.semantics)
        .unwrap()
        .items
}

#[test]
fn menu_titles_commands_disabled_rows_separators_and_submenus_use_pointer_input() -> TestResult {
    let mut ui = runtime(600.0)?;
    let count = ui.signal(0_u32);
    let chosen = count.clone();
    let forbidden = count.clone();
    let bar = MenuBar::new(
        &ui,
        vec![MenuItem::submenu(
            "file",
            "File",
            vec![
                MenuItem::new("disabled", "Unavailable", move |_| forbidden.set(100)).disabled(),
                MenuItem::separator(),
                MenuItem::submenu(
                    "more",
                    "More",
                    vec![MenuItem::new("run", "Run", move |_| {
                        chosen.update(|count| *count += 1)
                    })],
                ),
            ],
        )],
    )?;
    let bar = ui.mount_tree(bar.key("bar"))?;
    let mut driver = Driver::new(&mut ui);
    let title = driver.exposed_part(&bar, "title/file")?;
    let list = driver.exposed_part(&title, "items")?;
    driver.settle()?;
    assert!(
        matches!(
            driver.click_item(&list, 0),
            Err(TestError::NotRevealed { .. })
        ),
        "a closed list is not revealed by a query/click"
    );
    driver.click(&title)?;
    assert_eq!(
        driver.semantics(&list)?.items.len(),
        2,
        "separator has no item"
    );
    driver.click_item(&list, 0)?;
    assert_eq!(count.get()?, 0);
    let row_bounds = driver
        .semantics(&list)?
        .items
        .iter()
        .map(|item| item.bounds.unwrap())
        .collect::<Vec<_>>();
    assert!(
        row_bounds[0][1] + row_bounds[0][3] < row_bounds[1][1],
        "separator occupies its own band"
    );
    driver.click_item(&list, 1)?;
    let submenu = driver.exposed_part(&title, "submenu/more")?;
    let nested = driver.exposed_part(&submenu, "items")?;
    driver.click_item(&nested, 0)?;
    assert_eq!(count.get()?, 1);
    assert!(
        matches!(
            driver.click_item(&list, 1),
            Err(TestError::NotRevealed { .. })
        ),
        "command closed the chain"
    );
    assert_eq!(count.get()?, 1);
    Ok(())
}

#[test]
fn tabs_click_unselected_titles_and_report_settled_selection_once() -> TestResult {
    let mut ui = runtime(600.0)?;
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let tabs = ui.mount_tree(
        Tabs::new([tab("a"), tab("b")])
            .on_select(move |_, change| {
                seen.borrow_mut()
                    .push((change.selected.clone(), change.origin));
                Ok(())
            })
            .key("tabs"),
    )?;
    assert!(items(&ui, &tabs).iter().all(|item| item.bounds.is_none()));
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let body = driver.find(&tabs, &["b", "body"])?;
    assert!(
        !driver
            .runtime()
            .inspect()
            .components
            .iter()
            .find(|component| component.id == driver.runtime().inspection_id(&body).unwrap())
            .unwrap()
            .visible
    );
    driver.click_item(&tabs, 1)?;
    assert!(
        driver
            .runtime()
            .inspect()
            .components
            .iter()
            .find(|component| component.id == driver.runtime().inspection_id(&body).unwrap())
            .unwrap()
            .visible
    );
    assert_eq!(*changes.borrow(), [(Some("b".into()), ChangeOrigin::User)]);
    driver.runtime_mut().paint()?;
    for _ in 0..3 {
        let _ = driver.semantics(&tabs)?;
    }
    use pert_ui::testing::HostIntegration;
    let record = driver
        .runtime()
        .inspect()
        .components
        .into_iter()
        .find(|component| component.id == driver.runtime().inspection_id(&tabs).unwrap())
        .ok_or("tabs record")?;
    assert_eq!(record.dirty, None);
    assert!(!driver.runtime().needs_paint());
    assert_eq!(driver.runtime().next_deadline(), None);
    assert!(driver.quiet_for(Duration::from_millis(50))?.is_idle());
    Ok(())
}

#[test]
fn tab_bounds_follow_live_scroll_exclude_arrows_and_do_not_retarget_recreated_keys() -> TestResult {
    let mut ui = runtime(230.0)?;
    let tabs = ui.mount_tree(Tabs::new([tab("a"), tab("b"), tab("c"), tab("d")]).key("tabs"))?;
    ui.flush(Duration::ZERO)?;
    let before = items(&ui, &tabs);
    assert!(before[0].bounds.is_some());
    assert!(before[3].bounds.is_none());
    let first = before[0].bounds.unwrap();
    assert!(first[0] > 0.0 && first[0] + first[2] < 230.0);
    ui.pointer_input(PointerEvent::new(
        Point::new(100.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, 40.0)?),
    ))?;
    let immediate = items(&ui, &tabs);
    assert_ne!(
        immediate[0].bounds, before[0].bounds,
        "no settlement before inspection"
    );
    assert!(
        immediate
            .iter()
            .filter_map(|item| item.bounds)
            .all(|bounds| bounds[0] >= first[0] && bounds[0] + bounds[2] <= 230.0 - first[0])
    );
    let old = ui.find(&tabs, &["a"])?.ok_or("old tab")?;
    ui.reconcile_children(&tabs, vec![tab("b").into_element()])?;
    ui.flush(ui.now())?;
    ui.reconcile_children(
        &tabs,
        vec![tab("a").into_element(), tab("b").into_element()],
    )?;
    let new = ui.find(&tabs, &["a"])?.ok_or("new tab")?;
    assert_ne!(old, new);
    ui.flush(ui.now())?;
    Driver::new(&mut ui).click_item(&tabs, 0)?;
    assert!(items(&ui, &tabs)[0].selected);
    // Tabs' hard minimum is 96 px. Make the two arrow widths exceed that
    // actual allocation, rather than testing only a narrow window clip.
    let theme = ui.theme();
    let density = theme.density()?;
    let metrics = theme
        .metrics(density)?
        .with_metric(MetricRole::ControlHeight, 64.into())?;
    theme.set_metrics(density, metrics)?;
    ui.set_viewport(Viewport::new(96.0, 300.0, 1.0)?)?;
    ui.flush(ui.now())?;
    assert_eq!(
        ui.bounds(&tabs)?.ok_or("narrow strip bounds")?.width(),
        96.0
    );
    assert!(items(&ui, &tabs).iter().all(|item| item.bounds.is_none()));
    assert!(matches!(
        Driver::new(&mut ui).click_item(&tabs, 0),
        Err(TestError::ItemWithoutBounds { .. })
    ));
    Ok(())
}

#[test]
fn select_options_use_pointer_bounds_and_commit_then_close() -> TestResult {
    let mut ui = runtime(600.0)?;
    let value = ui.signal(0_u8);
    let root = ui.mount_tree(
        Column::new([Select::new(
            &ui,
            value.clone(),
            [Choice::new(0, "A"), Choice::new(1, "B")],
        )?
        .key("mode")])
        .key("root"),
    )?;
    let select = ui.find(&root, &["mode"])?.ok_or("select")?;
    let mut driver = Driver::new(&mut ui);
    let anchor = driver.exposed_part(&select, "control")?;
    let list = driver.exposed_part(&select, "options")?;
    driver.settle()?;
    assert!(
        driver
            .semantics(&anchor)?
            .items
            .iter()
            .all(|item| item.bounds.is_none()),
        "closed combo metadata is not a list geometry promise"
    );
    assert!(matches!(
        driver.click_item(&list, 1),
        Err(TestError::NotRevealed { .. })
    ));
    driver.click(&anchor)?;
    driver.click_item(&list, 1)?;
    assert_eq!(value.get()?, 1);
    assert!(matches!(
        driver.click_item(&list, 0),
        Err(TestError::NotRevealed { .. })
    ));
    Ok(())
}

#[test]
fn a_scrolled_partly_visible_option_has_only_its_visible_click_rectangle() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 155.0, 1.0)?)?;
    let value = ui.signal(0_u8);
    let root = ui.mount_tree(
        Column::new([Select::new(
            &ui,
            value.clone(),
            (0..20).map(|index| Choice::new(index, format!("Option {index}"))),
        )?
        .key("mode")])
        .key("root"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let select = driver.find(&root, &["mode"])?;
    let control = driver.exposed_part(&select, "control")?;
    let options = driver.exposed_part(&select, "options")?;
    driver.click(&control)?;
    let before = driver.semantics(&options)?.items;
    let [x, y, width, height] = before[0].bounds.ok_or("first visible option")?;
    driver.runtime_mut().pointer_input(PointerEvent::new(
        Point::new(x + width / 2.0, y + height / 2.0)?,
        PointerKind::Scroll(Point::new(0.0, 45.0)?),
    ))?;
    driver.settle()?;
    let after = driver.semantics(&options)?.items;
    assert!(after[0].bounds.is_none());
    let (index, clipped) = after
        .iter()
        .enumerate()
        .filter_map(|(index, item)| item.bounds.map(|bounds| (index, bounds)))
        .next()
        .ok_or("scrolled visible option")?;
    assert!(
        index > 0 && clipped[3] < height,
        "top row is clipped: {clipped:?}"
    );
    driver.click_item(&options, index)?;
    assert_eq!(usize::from(value.get()?), index);
    assert!(matches!(
        driver.click_item(&options, index),
        Err(TestError::NotRevealed { .. })
    ));
    Ok(())
}

#[test]
fn hidden_tab_title_items_have_no_window_bounds() -> TestResult {
    let mut ui = runtime(600.0)?;
    let tabs = ui.mount_tree(Tabs::new([tab("a"), tab("b")]).key("tabs"))?;
    ui.flush(Duration::ZERO)?;
    assert!(items(&ui, &tabs)[0].bounds.is_some());
    ui.set_visible(&tabs, false)?;
    ui.flush(ui.now())?;
    assert!(items(&ui, &tabs).iter().all(|item| item.bounds.is_none()));
    Ok(())
}

#[test]
fn a_detached_tab_title_still_has_bounds_and_requests_window_activation() -> TestResult {
    use pert_ui::testing::HostIntegration;
    let mut ui = runtime(600.0)?;
    let tabs = ui.mount_tree(Tabs::new([tab("a"), tab("b")]).key("tabs"))?;
    ui.flush(Duration::ZERO)?;
    let view = ui.find(&tabs, &["a"])?.ok_or("tab a")?;
    let primary = ui.primary_window();
    let prepared = ui.prepare_detach(&view, NativeWindowOptions::new("A")?)?;
    let window = ui.commit_detach(prepared)?;
    ui.select_window(&primary)?;
    ui.flush(ui.now())?;
    assert!(items(&ui, &tabs)[0].bounds.is_some());
    assert!(items(&ui, &tabs)[1].selected);
    Driver::new(&mut ui).click_item(&tabs, 0)?;
    assert!(
        items(&ui, &tabs)[1].selected,
        "detached titles activate their window without changing selection"
    );
    let requests = ui.take_presentation_requests()?;
    assert!(requests.iter().any(|request| matches!(request.action(), PresentationAction::Activate(target) if *target == window)));
    Ok(())
}

struct ScaledClip(Option<Element>);
impl Component for ScaledClip {
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        ViewTransform::new(ViewScale::new(1.5).unwrap(), Point::new(0.0, 0.0).unwrap())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(360.0, 200.0)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        cx.place_clipped(
            0,
            Rect::new(0.0, 0.0, 360.0, 100.0)?,
            Rect::new(20.0, 0.0, 80.0, 100.0)?,
        )
    }
}

#[test]
fn title_bounds_use_committed_view_scale_ancestor_clip_and_bound_title_widths() -> TestResult {
    let mut ui = runtime(600.0)?;
    let title = ui.signal("A".to_owned());
    let root = ui.mount_tree(
        ScaledClip(Some(
            Tabs::new([
                Tab::new("a", title.read_only(), Label::new("A").key("body")),
                tab("b"),
            ])
            .key("tabs"),
        ))
        .key("frame"),
    )?;
    ui.flush(Duration::ZERO)?;
    let tabs = ui.find(&root, &["tabs"])?.ok_or("tabs")?;
    let before = items(&ui, &tabs);
    let record = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.id == ui.inspection_id(&tabs).unwrap())
        .ok_or("record")?;
    assert_eq!(record.view_scale, 1.5);
    let clip = record.details.validation.clip.ok_or("clip")?;
    for bounds in before.iter().filter_map(|item| item.bounds) {
        assert!(bounds[0] >= clip[0] && bounds[0] + bounds[2] <= clip[0] + clip[2]);
        assert_eq!(bounds[3], 36.0 * 1.5);
    }
    Driver::new(&mut ui).click_item(&tabs, 1)?;
    assert!(items(&ui, &tabs)[1].selected);
    title.set("A substantially longer dynamic title".to_owned())?;
    ui.flush(ui.now())?;
    assert_eq!(
        items(&ui, &tabs)[0].name.as_deref(),
        Some("A substantially longer dynamic title")
    );
    assert!(
        items(&ui, &tabs)[1].bounds.is_none(),
        "the second title moved outside the ancestor clip"
    );
    Ok(())
}

#[test]
fn button_and_icon_menus_expose_the_actual_trigger_and_pointer_rows() -> TestResult {
    let mut ui = runtime(600.0)?;
    let count = ui.signal(0_u32);
    let first = count.clone();
    let second = count.clone();
    let root = ui.mount_tree(
        Column::new([
            Menu::new(
                &ui,
                "Actions",
                vec![MenuItem::new("run", "Run", move |_| {
                    first.update(|n| *n += 1)
                })],
            )?
            .key("plain"),
            Menu::icon(
                &ui,
                Icon::MoreHorizontal,
                "More actions",
                vec![MenuItem::new("run", "Run", move |_| {
                    second.update(|n| *n += 10)
                })],
            )?
            .key("icon"),
        ])
        .key("root"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    for (key, name, expected) in [("plain", "Actions", 1), ("icon", "More actions", 11)] {
        let menu = driver.find(&root, &[key])?;
        let trigger = driver.exposed_part(&menu, "trigger")?;
        let semantics = driver.semantics(&trigger)?;
        assert_eq!(semantics.role, SemanticRole::Button);
        assert_eq!(semantics.name.as_deref(), Some(name));
        let rows = driver.exposed_part(&menu, "items")?;
        assert!(matches!(
            driver.click_item(&rows, 0),
            Err(TestError::NotRevealed { .. })
        ));
        driver.click(&trigger)?;
        driver.click_item(&rows, 0)?;
        assert_eq!(count.get()?, expected);
        assert!(matches!(
            driver.click_item(&rows, 0),
            Err(TestError::NotRevealed { .. })
        ));
    }
    Ok(())
}

#[test]
fn repeated_row_keys_in_distinct_menu_scopes_select_only_their_own_commands() -> TestResult {
    let mut ui = runtime(600.0)?;
    let count = ui.signal(0_u32);
    let (direct, nested, other) = (count.clone(), count.clone(), count.clone());
    let bar = ui.mount_tree(
        MenuBar::new(
            &ui,
            vec![
                MenuItem::submenu(
                    "file",
                    "File",
                    vec![
                        MenuItem::new("run", "Run", move |_| direct.update(|n| *n += 1)),
                        MenuItem::submenu(
                            "tools",
                            "Tools",
                            vec![MenuItem::new("run", "Run", move |_| {
                                nested.update(|n| *n += 10)
                            })],
                        ),
                    ],
                ),
                MenuItem::submenu(
                    "edit",
                    "Edit",
                    vec![MenuItem::new("run", "Run", move |_| {
                        other.update(|n| *n += 100)
                    })],
                ),
            ],
        )?
        .key("bar"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let file = driver.exposed_part(&bar, "title/file")?;
    let edit = driver.exposed_part(&bar, "title/edit")?;
    assert_ne!(file, edit);
    let root_rows = driver.exposed_part(&file, "items")?;
    let tools = driver.exposed_part(&file, "submenu/tools")?;
    let nested_rows = driver.exposed_part(&tools, "items")?;
    let edit_rows = driver.exposed_part(&edit, "items")?;
    assert_ne!(root_rows, nested_rows);
    assert_ne!(root_rows, edit_rows);
    driver.click(&file)?;
    driver.click_item(&root_rows, 1)?;
    driver.click_item(&nested_rows, 0)?;
    assert_eq!(count.get()?, 10);
    driver.click(&file)?;
    driver.click_item(&root_rows, 0)?;
    assert_eq!(count.get()?, 11);
    driver.click(&edit)?;
    driver.click_item(&edit_rows, 0)?;
    assert_eq!(count.get()?, 111);
    Ok(())
}

#[test]
fn a_pointer_context_request_opens_its_scoped_menu_and_a_row_click_runs_it() -> TestResult {
    let mut ui = runtime(600.0)?;
    let count = ui.signal(0_u32);
    let chosen = count.clone();
    let root = ui.mount_tree(
        Column::new([ContextMenu::new(
            &ui,
            Button::new("Target", |_| Ok(())).key("target"),
            vec![MenuItem::new("run", "Run", move |_| {
                chosen.update(|n| *n += 1)
            })],
        )?
        .key("context")])
        .key("root"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let context = driver.find(&root, &["context"])?;
    let target = driver.find(&context, &["target"])?;
    let rows = driver.exposed_part(&context, "items")?;
    let [x, y, w, h] = driver
        .runtime()
        .bounds(&target)?
        .ok_or("target bounds")?
        .values();
    let at = Point::new(x + w / 2.0, y + h / 2.0)?;
    driver.runtime_mut().pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Secondary),
    ))?;
    driver.settle()?;
    driver.runtime_mut().pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Secondary),
    ))?;
    driver.settle()?;
    driver.click_item(&rows, 0)?;
    assert_eq!(count.get()?, 1);
    assert!(matches!(
        driver.click_item(&rows, 0),
        Err(TestError::NotRevealed { .. })
    ));
    Ok(())
}
