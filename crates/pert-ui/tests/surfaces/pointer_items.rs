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

#[test]
fn accordion_headers_have_their_painted_rectangles_and_click_through_them() -> TestResult {
    // A narrow accordion wraps its long titles over several lines.
    let mut ui = runtime(140.0)?;
    let accordion = ui.mount_tree(
        Accordion::new([
            Section::new(
                "first",
                "A first section title long enough to wrap",
                Label::new("1").key("one"),
            ),
            Section::new("second", "Second", Label::new("2").key("two")),
            Section::new(
                "third",
                "A third section title long enough to wrap",
                Label::new("3").key("three"),
            ),
        ])
        .key("accordion"),
    )?;
    assert!(
        items(&ui, &accordion)
            .iter()
            .all(|item| item.bounds.is_none()),
        "no rectangle before the first layout"
    );
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let bounds: Vec<[f32; 4]> = items(driver.runtime(), &accordion)
        .iter()
        .map(|item| item.bounds.expect("laid out"))
        .collect();
    // Headers stack top to bottom, and a wrapped title makes its header taller.
    assert!(bounds[0][3] > bounds[1][3], "{bounds:?}");
    assert!(
        bounds[1][1] >= bounds[0][1] + bounds[0][3] - 0.01,
        "{bounds:?}"
    );
    driver.click_item(&accordion, 2)?;
    let third = &items(driver.runtime(), &accordion)[2];
    assert_eq!(
        third.expanded,
        Some(true),
        "the click reached the third header"
    );
    Ok(())
}

#[test]
fn radio_and_segmented_choices_click_through_their_rectangles() -> TestResult {
    for segmented in [false, true] {
        let mut ui = runtime(400.0)?;
        let value = ui.signal(0_u32);
        let choices = [
            Choice::new(0, "Low"),
            Choice::new(1, "Medium"),
            Choice::new(2, "High"),
        ];
        let element = if segmented {
            SegmentedControl::new(value.clone(), choices)?.key("choice")
        } else {
            RadioGroup::new(value.clone(), choices)?.key("choice")
        };
        let group = ui.mount_tree(element)?;
        let mut driver = Driver::new(&mut ui);
        driver.settle()?;
        let bounds: Vec<[f32; 4]> = items(driver.runtime(), &group)
            .iter()
            .map(|item| item.bounds.expect("laid out"))
            .collect();
        // Segments go left to right, radio rows top to bottom.
        let along = if segmented { 0 } else { 1 };
        assert!(bounds[2][along] > bounds[1][along] && bounds[1][along] > bounds[0][along]);
        driver.click_item(&group, 2)?;
        assert_eq!(value.get()?, 2, "segmented {segmented}");
        driver.click_item(&group, 1)?;
        assert_eq!(value.get()?, 1, "segmented {segmented}");
    }
    Ok(())
}

#[test]
fn tool_window_stripe_buttons_click_through_their_rectangles() -> TestResult {
    let mut ui = runtime(800.0)?;
    let window = |key: &str| {
        ToolWindow::new(
            key,
            format!("{key} title"),
            Label::new(key).key(format!("{key}-body")),
        )
        .icon(Icon::Folder)
        .edge(ToolEdge::Left)
    };
    let root = ui.mount_tree(
        ToolWindowLayout::new(Label::new("main").key("main"))
            .edge(ToolEdge::Left, EdgeMode::Stripe)
            .window(window("files"))
            .window(window("search"))
            .key("frame"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let stripe = driver
        .runtime()
        .find_descendant(&root, "stripe-left")?
        .ok_or("the left stripe")?;
    let before = items(driver.runtime(), &stripe);
    assert!(
        before.iter().all(|item| item.bounds.is_some()),
        "{before:?}"
    );
    assert_eq!(before[1].expanded, Some(false));
    driver.click_item(&stripe, 1)?;
    assert_eq!(
        items(driver.runtime(), &stripe)[1].expanded,
        Some(true),
        "the click reached the second button"
    );
    Ok(())
}

#[test]
fn a_header_below_open_content_and_variable_width_choices_click_through() -> TestResult {
    // The first section's content displaces the later headers.
    let mut ui = runtime(300.0)?;
    let accordion = ui.mount_tree(
        Accordion::new([
            Section::new(
                "first",
                "First",
                Column::new([
                    Label::new("line one").key("a"),
                    Label::new("line two").key("b"),
                    Label::new("line three").key("c"),
                ])
                .key("content"),
            ),
            Section::new("second", "Second", Label::new("2").key("two")),
        ])
        .mode(ExpandMode::Multiple)
        .key("accordion"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.click_item(&accordion, 0)?;
    let before = items(driver.runtime(), &accordion);
    assert_eq!(before[0].expanded, Some(true));
    let [_, first_y, _, first_h] = before[0].bounds.ok_or("laid out")?;
    let [_, second_y, _, _] = before[1].bounds.ok_or("laid out")?;
    assert!(
        second_y > first_y + first_h + 20.0,
        "content between the headers"
    );
    driver.click_item(&accordion, 1)?;
    assert_eq!(items(driver.runtime(), &accordion)[1].expanded, Some(true));

    // Inline choices are as wide as their labels.
    let mut ui = runtime(500.0)?;
    let value = ui.signal(0_u32);
    let inline = ui.mount_tree(
        InlineChoice::new(
            value.clone(),
            [
                Choice::new(0, "A"),
                Choice::new(1, "A much longer label"),
                Choice::new(2, "B"),
            ],
        )?
        .key("inline"),
    )?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let widths: Vec<f32> = items(driver.runtime(), &inline)
        .iter()
        .map(|item| item.bounds.expect("laid out")[2])
        .collect();
    assert!(widths[1] > widths[0] * 2.0, "{widths:?}");
    driver.click_item(&inline, 2)?;
    assert_eq!(value.get()?, 2);
    driver.click_item(&inline, 1)?;
    assert_eq!(value.get()?, 1);
    Ok(())
}

#[test]
fn a_scrolled_stripe_reports_the_buttons_it_shows() -> TestResult {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 160.0, 1.0)?)?;
    let mut ui = ui;
    let mut layout = ToolWindowLayout::new(Label::new("main").key("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe);
    for index in 0..12 {
        layout = layout.window(
            ToolWindow::new(
                format!("w{index}"),
                format!("Window {index}"),
                Label::new("body").key(format!("w{index}-body")),
            )
            .icon(Icon::Folder)
            .edge(ToolEdge::Left),
        );
    }
    let root = ui.mount_tree(layout.key("frame"))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let stripe = driver
        .runtime()
        .find_descendant(&root, "stripe-left")?
        .ok_or("the left stripe")?;
    let shown = |ui: &Runtime| -> Vec<bool> {
        items(ui, &stripe)
            .iter()
            .map(|item| item.bounds.is_some())
            .collect()
    };
    let before = shown(driver.runtime());
    assert!(before[0] && !before[11], "the stripe overflows: {before:?}");
    // Scrolling the stripe moves its buttons with it.
    let [x, y, w, h] = driver
        .runtime()
        .bounds(&stripe)?
        .ok_or("laid out")?
        .values();
    let center = Point::new(x + w / 2.0, y + h / 2.0)?;
    drop(driver);
    for _ in 0..20 {
        ui.pointer_input(PointerEvent::new(
            center,
            PointerKind::Scroll(Point::new(0.0, 60.0)?),
        ))?;
    }
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let after = shown(driver.runtime());
    assert!(!after[0] && after[11], "scrolled to the end: {after:?}");
    driver.click_item(&stripe, 11)?;
    assert_eq!(items(driver.runtime(), &stripe)[11].expanded, Some(true));
    Ok(())
}

#[test]
fn a_partly_scrolled_stripe_clips_its_edge_button_and_clicks_its_visible_part() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 160.0, 1.0)?)?;
    let mut layout = ToolWindowLayout::new(Label::new("main").key("main"))
        .edge(ToolEdge::Left, EdgeMode::Stripe);
    for index in 0..12 {
        layout = layout.window(
            ToolWindow::new(
                format!("w{index}"),
                format!("Window {index}"),
                Label::new("body").key(format!("w{index}-body")),
            )
            .icon(Icon::Folder)
            .edge(ToolEdge::Left),
        );
    }
    let root = ui.mount_tree(layout.key("frame"))?;
    ui.flush(Duration::ZERO)?;
    let stripe = ui.find_descendant(&root, "stripe-left")?.ok_or("stripe")?;
    let full = items(&ui, &stripe)[0].bounds.ok_or("laid out")?[3];
    // A small scroll leaves the first button partly above the stripe.
    let [x, y, w, h] = ui.bounds(&stripe)?.ok_or("laid out")?.values();
    let center = Point::new(x + w / 2.0, y + h / 2.0)?;
    ui.pointer_input(PointerEvent::new(
        center,
        PointerKind::Scroll(Point::new(0.0, full / 3.0)?),
    ))?;
    ui.flush(ui.now())?;
    let scrolled = items(&ui, &stripe);
    let [_, first_y, _, first_h] = scrolled[0].bounds.ok_or("partly visible")?;
    assert!(
        first_h < full - 0.5 && first_h > 0.5,
        "clipped to {first_h} of {full}"
    );
    assert!(
        (first_y - y).abs() < 0.5,
        "the clipped part starts at the stripe's top"
    );
    assert!(
        scrolled.iter().any(|item| item.bounds.is_none()),
        "others clipped away"
    );
    let mut driver = Driver::new(&mut ui);
    driver.click_item(&stripe, 0)?;
    assert_eq!(items(driver.runtime(), &stripe)[0].expanded, Some(true));
    Ok(())
}

// ---- MultiSelect and QuickPick popups ----

/// A chooser over `count` options "Option 0", "Option 1" and so on, with
/// `disabled` disabled.
struct Chooser {
    ui: Runtime,
    root: WidgetId,
    value: Signal<Vec<usize>>,
    changes: Rc<RefCell<Vec<Vec<usize>>>>,
}
fn chooser(count: usize, disabled: &[usize], commit: MultiSelectCommit) -> TestResult<Chooser> {
    let mut ui = runtime(400.0)?;
    let value = ui.signal(Vec::new());
    let changes = Rc::new(RefCell::new(Vec::new()));
    let record = changes.clone();
    let options = (0..count).map(|index| {
        let choice = Choice::new(index, format!("Option {index}"));
        if disabled.contains(&index) {
            choice.disabled()
        } else {
            choice
        }
    });
    let control = MultiSelect::new(&ui, value.clone(), options)?
        .commit(commit)
        .on_change(move |_, values| {
            record.borrow_mut().push(values.to_vec());
            Ok(())
        });
    let root = ui.mount_tree(
        Column::new([
            control.key("tags"),
            Button::new("After", |_| Ok(())).key("after"),
        ])
        .key("root"),
    )?;
    Driver::new(&mut ui).settle()?;
    Ok(Chooser {
        ui,
        root,
        value,
        changes,
    })
}
impl Chooser {
    fn part(&self, name: &str) -> TestResult<WidgetId> {
        let tags = self.ui.find(&self.root, &["tags"])?.ok_or("tags")?;
        Ok(self
            .ui
            .find_exposed_part(&tags, name)?
            .ok_or_else(|| format!("no part {name}"))?)
    }
    /// Opens the popup from the keyboard, as the trigger's rustdoc says.
    fn open(&mut self) -> TestResult {
        let control = self.part("control")?;
        let mut driver = Driver::new(&mut self.ui);
        driver.runtime_mut().focus(&control)?;
        driver.key(KeyCode::Enter)?;
        Ok(())
    }
    fn is_open(&self) -> TestResult<bool> {
        let tags = self.ui.find(&self.root, &["tags"])?.ok_or("tags")?;
        let tags = self.ui.inspection_id(&tags)?;
        Ok(self
            .ui
            .inspect()
            .components
            .into_iter()
            .find(|component| component.id == tags)
            .is_some_and(|component| {
                component.details.properties["open"] == InspectValue::Bool(true)
            }))
    }
    /// The index of the popup item named `name`.
    fn index(&self, name: &str) -> TestResult<usize> {
        let options = self.part("options")?;
        items(&self.ui, &options)
            .iter()
            .position(|item| item.name.as_deref() == Some(name))
            .ok_or_else(|| format!("no item {name}").into())
    }
    /// The mounted row of the option named `name`, cut to the list's box.
    fn row_bounds(&self, name: &str) -> TestResult<[f32; 4]> {
        let options = self.part("options")?;
        let list = self.ui.bounds(&options)?.ok_or("laid out")?;
        let row = row_box(&self.ui, SemanticRole::Checkbox, name)?;
        let row = Rect::new(row[0], row[1], row[2], row[3])?;
        Ok(row.intersection(list).values())
    }
}

/// The border box of the visible component with `role` named `name`.
fn row_box(ui: &Runtime, role: SemanticRole, name: &str) -> TestResult<[f32; 4]> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| {
            component.visible
                && component
                    .details
                    .semantics
                    .as_ref()
                    .is_some_and(|semantics| {
                        semantics.role == role && semantics.name.as_deref() == Some(name)
                    })
        })
        .and_then(|component| component.details.box_model.border_box)
        .ok_or_else(|| format!("no mounted row {name}").into())
}

/// Scrolls with the wheel at the middle of `bounds`, then settles.
fn wheel(ui: &mut Runtime, bounds: [f32; 4], dy: f32) -> TestResult {
    let [x, y, w, h] = bounds;
    ui.pointer_input(PointerEvent::new(
        Point::new(x + w / 2.0, y + h / 2.0)?,
        PointerKind::Scroll(Point::new(0.0, dy)?),
    ))?;
    Driver::new(ui).settle()?;
    Ok(())
}

#[test]
fn multi_select_options_click_through_their_rectangles_and_toggle_once() -> TestResult {
    let mut c = chooser(6, &[], MultiSelectCommit::EachToggle)?;
    let control = c.part("control")?;
    assert!(
        items(&c.ui, &control)
            .iter()
            .all(|item| item.bounds.is_none())
    );
    c.open()?;
    let options = c.part("options")?;
    let index = c.index("Option 3")?;
    assert!(index > 0);
    Driver::new(&mut c.ui).click_item(&options, index)?;
    assert_eq!(c.value.get()?, [3]);
    assert_eq!(*c.changes.borrow(), [vec![3]], "reported once");
    let item = &items(&c.ui, &options)[index];
    assert_eq!(item.checked, Some(true));
    assert_eq!(item.actions, [SemanticAction::Toggle]);
    // The popup stays open across toggles.
    Driver::new(&mut c.ui).click_item(&options, 1)?;
    assert_eq!(c.value.get()?, [3, 1]);
    // The trigger's items describe the options but never have bounds.
    assert!(
        items(&c.ui, &control)
            .iter()
            .all(|item| item.bounds.is_none())
    );
    assert!(items(&c.ui, &control)[3].selected);
    Ok(())
}

#[test]
fn multi_select_on_close_clicks_edit_the_draft() -> TestResult {
    let mut c = chooser(4, &[], MultiSelectCommit::OnClose)?;
    c.open()?;
    let options = c.part("options")?;
    Driver::new(&mut c.ui).click_item(&options, 2)?;
    assert!(c.value.get()?.is_empty(), "a draft until the popup closes");
    assert!(c.changes.borrow().is_empty());
    assert_eq!(items(&c.ui, &options)[2].checked, Some(true));
    let control = c.part("control")?;
    assert!(
        !items(&c.ui, &control)[2].selected,
        "the trigger shows the committed value"
    );
    // Focus leaving the popup closes it and commits the draft.
    Driver::new(&mut c.ui).key(KeyCode::Tab)?;
    assert!(!c.is_open()?);
    assert_eq!(c.value.get()?, [2]);
    assert!(items(&c.ui, &control)[2].selected);
    Ok(())
}

#[test]
fn a_disabled_multi_select_option_keeps_its_rectangle_and_arms_nothing() -> TestResult {
    let mut c = chooser(4, &[1], MultiSelectCommit::EachToggle)?;
    c.open()?;
    let options = c.part("options")?;
    let item = items(&c.ui, &options)[1].clone();
    assert!(item.disabled && item.actions.is_empty() && item.bounds.is_some());
    Driver::new(&mut c.ui).click_item(&options, 1)?;
    assert!(c.value.get()?.is_empty());
    assert!(c.changes.borrow().is_empty());
    assert!(c.is_open()?, "the popup stays open");
    Ok(())
}

#[test]
fn filtered_and_scrolled_multi_select_options_have_their_rows_rectangles() -> TestResult {
    let mut c = chooser(30, &[], MultiSelectCommit::EachToggle)?;
    c.open()?;
    let options = c.part("options")?;
    // Rows beyond the popup's eight are not in view.
    let late = c.index("Option 20")?;
    assert_eq!(items(&c.ui, &options)[late].bounds, None);
    assert!(matches!(
        Driver::new(&mut c.ui).click_item(&options, late),
        Err(TestError::ItemWithoutBounds { .. })
    ));
    // Half a row down: the first row is partly visible.
    let first = items(&c.ui, &options)[0].bounds.ok_or("first row")?;
    wheel(&mut c.ui, first, first[3] / 2.0)?;
    let partial = items(&c.ui, &options)[0].bounds.ok_or("partly visible")?;
    assert!(partial[3] < first[3], "{partial:?}");
    assert_eq!(partial, c.row_bounds("Option 0")?);
    // Further down, a later row is clicked through its row's rectangle.
    wheel(&mut c.ui, first, 15.0 * first[3])?;
    let index = c.index("Option 20")?;
    let bounds = items(&c.ui, &options)[index]
        .bounds
        .ok_or("scrolled into view")?;
    assert_eq!(bounds, c.row_bounds("Option 20")?);
    Driver::new(&mut c.ui).click_item(&options, index)?;
    assert_eq!(c.value.get()?, [20]);

    // Filtering lists only the matches, in the popup's order.
    Driver::new(&mut c.ui).type_text("2")?;
    let names: Vec<_> = items(&c.ui, &options)
        .into_iter()
        .map(|item| item.name.unwrap_or_default())
        .collect();
    assert_eq!(names[..3], ["Option 2", "Option 12", "Option 20"]);
    // Every match in view has its row's rectangle; the list kept its offset.
    let shown: Vec<_> = items(&c.ui, &options)
        .into_iter()
        .filter_map(|item| Some((item.name?, item.bounds?)))
        .collect();
    assert!(shown.len() >= 4, "{shown:?}");
    for (name, bounds) in &shown {
        assert_eq!(*bounds, c.row_bounds(name)?, "{name}");
    }
    let index = c.index("Option 25")?;
    Driver::new(&mut c.ui).click_item(&options, index)?;
    assert_eq!(c.value.get()?, [20, 25]);
    Ok(())
}

#[test]
fn closed_and_empty_multi_select_lists_are_refused_before_input() -> TestResult {
    let mut c = chooser(3, &[], MultiSelectCommit::EachToggle)?;
    c.open()?;
    let options = c.part("options")?;
    // Closed: the list is hidden, so the reveal refuses it.
    Driver::new(&mut c.ui).key(KeyCode::Escape)?;
    assert!(!c.is_open()?);
    assert!(
        items(&c.ui, &options)
            .iter()
            .all(|item| item.bounds.is_none())
    );
    assert!(matches!(
        Driver::new(&mut c.ui).click_item(&options, 1),
        Err(TestError::NotRevealed { .. })
    ));
    // Open with no match: the list has no items.
    c.open()?;
    Driver::new(&mut c.ui).type_text("zzz")?;
    assert!(items(&c.ui, &options).is_empty());
    assert!(matches!(
        Driver::new(&mut c.ui).click_item(&options, 0),
        Err(TestError::NoItem { count: 0, .. })
    ));
    assert!(c.value.get()?.is_empty());
    Ok(())
}

type Picked = (
    Runtime,
    WidgetId,
    QuickPickController,
    Rc<RefCell<Vec<String>>>,
);

/// A picker presenting `items`, recording each chosen key.
fn picker(items: Vec<PickItem>) -> TestResult<Picked> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let controller = QuickPickController::new(&ui);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let record = chosen.clone();
    let app = ui.mount_tree(
        QuickPick::new(
            controller.clone(),
            Label::new("Document").key("doc"),
            move |_, item| {
                record.borrow_mut().push(item.key.clone());
                Ok(PickChoice::Done)
            },
        )
        .key("app"),
    )?;
    controller.open(items)?;
    Driver::new(&mut ui).settle()?;
    Ok((ui, app, controller, chosen))
}

#[test]
fn quick_pick_results_click_through_their_rectangles_including_disabled_ones() -> TestResult {
    let pages = || {
        vec![
            PickItem::new("colors", "Colors"),
            PickItem::new("knobs", "Knobs").shortcut("Ctrl+K"),
            PickItem::new("old", "Old page").disabled(true),
        ]
    };
    let (mut ui, app, controller, chosen) = picker(pages())?;
    let mut driver = Driver::new(&mut ui);
    let surface = driver.find(&app, &["surface"])?;
    let options = driver.exposed_part(&surface, "options")?;
    let semantics = driver.semantics(&options)?;
    assert_eq!(semantics.role, SemanticRole::ListBox);
    assert_eq!(semantics.items[1].shortcut.as_deref(), Some("Ctrl+K"));
    assert!(semantics.items.iter().all(|item| item.bounds.is_some()));
    driver.click_item(&options, 1)?;
    assert_eq!(*chosen.borrow(), ["knobs"]);
    // Closed: the surface is hidden, so the reveal refuses the list.
    assert!(matches!(
        driver.click_item(&options, 0),
        Err(TestError::NotRevealed { .. })
    ));
    // A disabled result offers no action, yet a click passes it to the
    // callback, as a press and release on its row do.
    controller.open(pages())?;
    driver.settle()?;
    let options = driver.exposed_part(&surface, "options")?;
    let disabled = driver.semantics(&options)?.items[2].clone();
    assert!(disabled.disabled && disabled.actions.is_empty() && disabled.bounds.is_some());
    driver.click_item(&options, 2)?;
    assert_eq!(*chosen.borrow(), ["knobs", "old"]);
    Ok(())
}

#[test]
fn filtered_quick_pick_results_have_their_rows_rectangles() -> TestResult {
    let (mut ui, app, _, chosen) = picker(
        (0..20)
            .map(|index| PickItem::new(format!("p{index}"), format!("Page {index}")))
            .collect(),
    )?;
    let mut driver = Driver::new(&mut ui);
    let surface = driver.find(&app, &["surface"])?;
    let options = driver.exposed_part(&surface, "options")?;
    // Results beyond the visible rows have no rectangle.
    assert_eq!(driver.semantics(&options)?.items[15].bounds, None);
    driver.type_text("12")?;
    // A query makes a new list: look the part up again.
    let options = driver.exposed_part(&surface, "options")?;
    let items = driver.semantics(&options)?.items;
    let index = items
        .iter()
        .position(|item| item.name.as_deref() == Some("Page 12"))
        .ok_or("listed")?;
    let bounds = items[index].bounds.ok_or("in view")?;
    assert_eq!(
        bounds,
        row_box(driver.runtime(), SemanticRole::Button, "Page 12")?
    );
    driver.click_item(&options, index)?;
    assert_eq!(*chosen.borrow(), ["p12"]);
    Ok(())
}

#[test]
fn a_quick_pick_without_results_has_no_options_part() -> TestResult {
    let (mut ui, app, _, _) = picker(vec![PickItem::new("a", "Alpha")])?;
    let mut driver = Driver::new(&mut ui);
    let surface = driver.find(&app, &["surface"])?;
    driver.type_text("zzz")?;
    assert!(matches!(
        driver.exposed_part(&surface, "options"),
        Err(TestError::ExposedPartNotFound { .. })
    ));
    Ok(())
}
