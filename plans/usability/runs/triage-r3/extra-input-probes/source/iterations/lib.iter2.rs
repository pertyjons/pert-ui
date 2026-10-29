//! Follow-up input probes for round 3 triage leads P12 to P14.
//!
//! Compiled unchanged against the frozen head (0.3.88) and baseline (0.3.75)
//! kits through the public facade only. Probes print observations with a
//! `P<n>|` prefix; assertions check only that input reached the intended
//! scope, never the behaviour under investigation.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use pert_ui::host::HostIntegration;
    use pert_ui::testing::Driver;
    use pert_ui::*;
    use std::{
        cell::{Cell, RefCell},
        error::Error,
        rc::Rc,
    };

    type Check = Result<(), Box<dyn Error>>;
    type Log = Rc<RefCell<Vec<String>>>;

    fn runtime(width: f32, height: f32) -> Result<Runtime, Box<dyn Error>> {
        let ui = Runtime::new();
        ui.set_viewport(Viewport::new(width, height, 1.0)?)?;
        Ok(ui)
    }

    fn focused_key(ui: &Runtime) -> String {
        match ui.focused() {
            None => "<none>".into(),
            Some(id) => match ui.key(&id) {
                Ok(key) => format!("{key}#{}", ui.inspection_id(&id).unwrap_or(0)),
                Err(error) => format!("<{error:?}>"),
            },
        }
    }

    fn inspected(ui: &Runtime, id: &WidgetId) -> Option<ComponentInspection> {
        let serial = ui.inspection_id(id).ok()?;
        ui.inspect().components.into_iter().find(|c| c.id == serial)
    }

    fn property(ui: &Runtime, id: &WidgetId, name: &str) -> String {
        inspected(ui, id)
            .and_then(|c| c.details.properties.get(name).map(|v| format!("{v:?}")))
            .unwrap_or_else(|| "<absent>".into())
    }

    fn text_of(value: Option<&InspectValue>) -> String {
        match value {
            Some(InspectValue::Text(text)) => text.clone(),
            Some(other) => format!("{other:?}"),
            None => "<absent>".into(),
        }
    }

    fn descendant(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, Box<dyn Error>> {
        ui.find_descendant(root, key)?
            .ok_or_else(|| format!("missing {key}").into())
    }

    fn middle(ui: &Runtime, id: &WidgetId) -> Result<Point, Box<dyn Error>> {
        let [x, y, w, h] = ui.bounds(id)?.ok_or("not laid out")?.values();
        Ok(Point::new(x + w / 2.0, y + h / 2.0)?)
    }

    fn host_move(d: &mut Driver<'_>, at: Point) -> Check {
        d.runtime_mut()
            .pointer_input(PointerEvent::new(at, PointerKind::Move))?;
        d.settle()?;
        Ok(())
    }

    fn host_leave(d: &mut Driver<'_>, at: Point) -> Check {
        d.runtime_mut()
            .pointer_input(PointerEvent::new(at, PointerKind::Leave))?;
        d.settle()?;
        Ok(())
    }

    fn raw_down(d: &mut Driver<'_>) -> Check {
        let ui = d.runtime_mut();
        ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
        ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
        Ok(())
    }

    #[derive(Clone, Copy)]
    enum Park {
        Never,
        Move(Point),
        MoveTwice(Point),
        MoveThenLeave(Point),
    }

    fn park(d: &mut Driver<'_>, how: Park) -> Check {
        match how {
            Park::Never => Ok(()),
            Park::Move(at) => host_move(d, at),
            Park::MoveTwice(at) => {
                host_move(d, at)?;
                host_move(d, at)
            }
            Park::MoveThenLeave(at) => {
                host_move(d, at)?;
                host_leave(d, at)
            }
        }
    }

    // ------------------------------------------------------------------ P12

    fn bar_fixture(ui: &mut Runtime) -> Result<(WidgetId, Log), Box<dyn Error>> {
        let log: Log = Rc::default();
        let saves = log.clone();
        let save = ui.register_command(
            "Save",
            CommandScope::Application,
            vec![KeyChord::primary('s')],
            move |_, _| {
                saves.borrow_mut().push("command:Save".into());
                Ok(())
            },
        )?;
        let (beta, zoom) = (log.clone(), log.clone());
        let mut content = Vec::new();
        for i in 0..6 {
            let l = log.clone();
            content.push(Element::new(
                format!("c{i}"),
                Button::new(format!("Content {i}"), move |_| {
                    l.borrow_mut().push(format!("content:{i}"));
                    Ok(())
                }),
            ));
        }
        let bar = MenuBar::new(
            ui,
            vec![
                MenuItem::submenu(
                    "file",
                    "File",
                    vec![
                        MenuItem::command(ui, "save", &save, |_, _| Ok(()))?,
                        MenuItem::new("beta", "Beta", move |_| {
                            beta.borrow_mut().push("row:Beta".into());
                            Ok(())
                        }),
                    ],
                ),
                MenuItem::submenu(
                    "view",
                    "View",
                    vec![MenuItem::new("zoom", "Zoom", move |_| {
                        zoom.borrow_mut().push("row:Zoom".into());
                        Ok(())
                    })],
                ),
            ],
        )?
        .content(Element::new("document", Column::new(content)));
        let root = ui.mount_tree(Element::new("window", Column::new([Element::new("bar", bar)])))?;
        Ok((root, log))
    }

    fn bar_state(ui: &Runtime, root: &WidgetId) -> String {
        let mut out = Vec::new();
        for title in ["file", "view"] {
            let Ok(id) = descendant(ui, root, &format!("title/{title}")) else {
                out.push(format!("{title}=<missing>"));
                continue;
            };
            let expanded = inspected(ui, &id)
                .and_then(|c| c.details.semantics.map(|s| format!("{:?}", s.expanded)))
                .unwrap_or_default();
            let items = match ui.find_descendant(&id, "items") {
                Ok(Some(items)) => format!(
                    "items_visible={} active={}",
                    inspected(ui, &items).is_some_and(|c| c.visible),
                    property(ui, &items, "active")
                ),
                Ok(None) => "items=<not mounted>".into(),
                Err(error) => format!("items=<{error:?}>"),
            };
            out.push(format!(
                "{title}[open={} expanded={expanded} {items}]",
                property(ui, &id, "open")
            ));
        }
        format!("{} focus={}", out.join(" "), focused_key(ui))
    }

    #[derive(Clone, Copy)]
    enum After {
        Nothing,
        HostMove(Point),
        HostMoveTwice(Point),
    }

    fn bar_scenario(label: &str, how: Park, after: After) -> Result<Vec<String>, Box<dyn Error>> {
        let mut ui = runtime(640.0, 480.0)?;
        let (root, log) = bar_fixture(&mut ui)?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime_mut().focus_next(FocusDirection::Forward)?;
        d.settle()?;
        park(&mut d, how)?;
        println!("P12|{label} parked {}", bar_state(d.runtime(), &root));
        d.key(KeyCode::F10)?;
        let focus = focused_key(d.runtime());
        println!("P12|{label} F10 {}", bar_state(d.runtime(), &root));
        assert!(focus.starts_with("title/file"), "F10 did not focus File");
        raw_down(&mut d)?;
        println!("P12|{label} Down before settle {}", bar_state(d.runtime(), &root));
        d.settle()?;
        let opened = bar_state(d.runtime(), &root);
        println!("P12|{label} Down after settle {opened}");
        assert!(opened.contains("items_visible=true"), "no menu opened");
        let moves = match after {
            After::Nothing => vec![],
            After::HostMove(at) => vec![at],
            After::HostMoveTwice(at) => vec![at, at],
        };
        for (n, at) in moves.into_iter().enumerate() {
            host_move(&mut d, at)?;
            println!(
                "P12|{label} host Move#{} at {at:?} {}",
                n + 1,
                bar_state(d.runtime(), &root)
            );
        }
        d.key(KeyCode::Enter)?;
        d.settle()?;
        let ran = log.borrow().clone();
        println!("P12|{label} Enter log={ran:?} {}", bar_state(d.runtime(), &root));
        Ok(ran)
    }

    #[test]
    fn p12_menu_title_switch_on_refreshed_move() -> Check {
        let (view, file) = {
            let mut ui = runtime(640.0, 480.0)?;
            let (root, _) = bar_fixture(&mut ui)?;
            Driver::new(&mut ui).settle()?;
            let view = middle(&ui, &descendant(&ui, &root, "title/view")?)?;
            let file = middle(&ui, &descendant(&ui, &root, "title/file")?)?;
            println!(
                "P12|calibrate title/file bounds={:?} title/view bounds={:?}",
                ui.bounds(&descendant(&ui, &root, "title/file")?)?,
                ui.bounds(&descendant(&ui, &root, "title/view")?)?
            );
            (view, file)
        };
        println!("P12|calibrate view middle={view:?} file middle={file:?}");
        let mut summary = Vec::new();
        summary.push(("never", bar_scenario("never", Park::Never, After::Nothing)?));
        summary.push((
            "parked-over-view",
            bar_scenario("parked", Park::Move(view), After::Nothing)?,
        ));
        summary.push((
            "parked-over-view-twice",
            bar_scenario("parked-twice", Park::MoveTwice(view), After::Nothing)?,
        ));
        summary.push((
            "parked-over-view-then-leave",
            bar_scenario("leave", Park::MoveThenLeave(view), After::Nothing)?,
        ));
        summary.push((
            "never+host-move-to-view-after-open",
            bar_scenario("genuine", Park::Never, After::HostMove(view))?,
        ));
        summary.push((
            "never+host-move-to-view-twice-after-open",
            bar_scenario("genuine-twice", Park::Never, After::HostMoveTwice(view))?,
        ));
        summary.push((
            "parked-leave+identical-host-move-after-open",
            bar_scenario("leave-identical", Park::MoveThenLeave(view), After::HostMove(view))?,
        ));
        summary.push((
            "parked-over-file-title",
            bar_scenario("parked-file", Park::Move(file), After::Nothing)?,
        ));
        for (name, ran) in summary {
            println!("P12|SUMMARY {name}: {ran:?}");
        }
        Ok(())
    }

    // ------------------------------------------------------------------ P13

    fn options(long: bool) -> Vec<String> {
        if long {
            (1..=20).map(|i| format!("O{i:02}")).collect()
        } else {
            ["Alpha", "Beta", "Gamma", "Delta"].map(String::from).to_vec()
        }
    }

    fn chooser_fixture(
        ui: &mut Runtime,
        long: bool,
    ) -> Result<(WidgetId, Signal<Vec<String>>, Log), Box<dyn Error>> {
        let log: Log = Rc::default();
        let value = ui.signal(Vec::<String>::new());
        let changes = log.clone();
        let chooser = MultiSelect::new(
            ui,
            value.clone(),
            options(long)
                .into_iter()
                .map(|label| Choice::new(label.to_lowercase(), label)),
        )?
        .on_change(move |_, values| {
            changes.borrow_mut().push(format!("change:{values:?}"));
            Ok(())
        });
        let root = ui.mount_tree(Element::new(
            "form",
            Column::new([
                Element::new("tags", chooser),
                Element::new("after", Button::new("After", |_| Ok(()))),
            ]),
        ))?;
        Ok((root, value, log))
    }

    /// The visible popup rows in vertical order: label, highlighted, checked
    /// and border box, read from inspection.
    fn popup_rows(ui: &Runtime) -> Vec<(String, bool, bool, [f32; 4])> {
        let mut rows: Vec<_> = ui
            .inspect()
            .components
            .into_iter()
            .filter(|c| {
                c.visible
                    && c.details.properties.contains_key("highlighted")
                    && c.details.properties.contains_key("checked")
            })
            .filter_map(|c| {
                let props = &c.details.properties;
                Some((
                    text_of(props.get("text")),
                    props.get("highlighted") == Some(&InspectValue::Bool(true)),
                    props.get("checked") == Some(&InspectValue::Bool(true)),
                    c.details.box_model.border_box?,
                ))
            })
            .collect();
        rows.sort_by(|a, b| a.3[1].total_cmp(&b.3[1]));
        rows
    }

    fn highlight(ui: &Runtime) -> String {
        let rows = popup_rows(ui);
        if rows.is_empty() {
            return "rows=<none visible>".into();
        }
        let lit: Vec<_> = rows.iter().filter(|r| r.1).map(|r| r.0.clone()).collect();
        format!("highlighted={lit:?} shown={}..{}", rows[0].0, rows[rows.len() - 1].0)
    }

    fn calibrate_rows(long: bool) -> Result<Vec<(String, Point)>, Box<dyn Error>> {
        let mut ui = runtime(480.0, 640.0)?;
        let (_root, _value, _log) = chooser_fixture(&mut ui, long)?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime_mut().focus_next(FocusDirection::Forward)?;
        d.key(KeyCode::ArrowDown)?;
        let rows = popup_rows(d.runtime());
        println!("P13|calibrate long={long} visible rows={}", rows.len());
        let mut points = Vec::new();
        for (label, _, _, [x, y, w, h]) in rows {
            let at = Point::new(x + w / 2.0, y + h / 2.0)?;
            host_move(&mut d, at)?;
            println!("P13|calibrate row {label} box=[{x},{y},{w},{h}] host Move -> {}", highlight(d.runtime()));
            points.push((label, at));
        }
        Ok(points)
    }

    fn chooser_scenario(
        label: &str,
        long: bool,
        how: Park,
        downs: usize,
    ) -> Result<(Vec<String>, Vec<String>), Box<dyn Error>> {
        let mut ui = runtime(480.0, 640.0)?;
        let (_root, value, log) = chooser_fixture(&mut ui, long)?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime_mut().focus_next(FocusDirection::Forward)?;
        d.settle()?;
        park(&mut d, how)?;
        println!("P13|{label} parked focus={} {}", focused_key(d.runtime()), highlight(d.runtime()));
        raw_down(&mut d)?;
        println!("P13|{label} Down before settle {}", highlight(d.runtime()));
        d.settle()?;
        let opened = highlight(d.runtime());
        println!("P13|{label} Down after settle focus={} {opened}", focused_key(d.runtime()));
        assert!(!opened.starts_with("rows=<none"), "popup did not open");
        for n in 1..=downs {
            d.key(KeyCode::ArrowDown)?;
            println!("P13|{label} Down#{} {}", n + 1, highlight(d.runtime()));
        }
        d.key(KeyCode::Enter)?;
        let chosen = value.get()?;
        println!(
            "P13|{label} Enter value={chosen:?} log={:?} {}",
            log.borrow(),
            highlight(d.runtime())
        );
        d.key(KeyCode::Escape)?;
        println!("P13|{label} Escape focus={} {}", focused_key(d.runtime()), highlight(d.runtime()));
        Ok((chosen, log.borrow().clone()))
    }

    #[test]
    fn p13_multi_select_rows_and_stale_pointer() -> Check {
        let short = calibrate_rows(false)?;
        assert!(short.len() >= 4);
        let gamma = short[2].1;
        let mut summary = Vec::new();
        summary.push(("never", chooser_scenario("short/never", false, Park::Never, 0)?));
        summary.push(("parked-over-row3", chooser_scenario("short/parked", false, Park::Move(gamma), 0)?));
        summary.push((
            "parked-then-leave",
            chooser_scenario("short/leave", false, Park::MoveThenLeave(gamma), 0)?,
        ));
        summary.push((
            "outside",
            chooser_scenario("short/outside", false, Park::Move(Point::new(470.0, 630.0)?), 0)?,
        ));
        summary.push(("never+Down", chooser_scenario("short/never-down", false, Park::Never, 1)?));
        summary.push((
            "parked+Down",
            chooser_scenario("short/parked-down", false, Park::Move(gamma), 1)?,
        ));
        let long = calibrate_rows(true)?;
        let parked = long[2].1;
        summary.push(("long/never", chooser_scenario("long/never", true, Park::Never, 12)?));
        summary.push(("long/parked", chooser_scenario("long/parked", true, Park::Move(parked), 12)?));
        summary.push((
            "long/leave",
            chooser_scenario("long/leave", true, Park::MoveThenLeave(parked), 12)?,
        ));
        for (name, (value, log)) in summary {
            println!("P13|SUMMARY {name}: value={value:?} on_change={log:?}");
        }
        Ok(())
    }

    // ------------------------------------------------------------------ P14

    fn reveal_fixture(ui: &mut Runtime) -> Result<(WidgetId, WidgetId, WidgetId), Box<dyn Error>> {
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([Element::new(
                "group",
                Column::new([Element::new("target", Button::new("Target", |_| Ok(())))]),
            )]),
        ))?;
        Driver::new(ui).settle()?;
        let group = descendant(ui, &root, "group")?;
        let target = descendant(ui, &root, "target")?;
        Ok((root, group, target))
    }

    fn visibility(ui: &Runtime, ids: &[(&str, &WidgetId)]) -> String {
        ids.iter()
            .map(|(name, id)| format!("{name}.visible={:?}", inspected(ui, id).map(|c| c.visible)))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// A target below the fold of a scroll area, so that revealing it
    /// scrolls; `on_scroll` may queue `action` once, as application work.
    fn scroll_fixture(
        ui: &mut Runtime,
        action: Rc<RefCell<Option<Box<dyn FnOnce(&mut Runtime) -> Result<(), RuntimeError>>>>>,
        log: Log,
    ) -> Result<(WidgetId, WidgetId), Box<dyn Error>> {
        let mut rows: Vec<Element> = (0..30)
            .map(|i| Element::new(format!("row-{i}"), Label::new(format!("Row {i}"))))
            .collect();
        rows.push(Element::new("target", Button::new("Target", |_| Ok(()))));
        let area = ScrollArea::new(Element::new("rows", Column::new(rows))).on_scroll(
            move |cx, offset, origin| {
                log.borrow_mut().push(format!("on_scroll y={} {origin:?}", offset.y()));
                if let Some(work) = action.borrow_mut().take() {
                    let queued = log.clone();
                    cx.defer(move |ui| {
                        queued.borrow_mut().push("deferred op runs".into());
                        work(ui)
                    })
                    .map(|_| ())?;
                }
                Ok(())
            },
        );
        // Mounted as the root so that it fills the viewport and overflows.
        let root = ui.mount_tree(Element::new("area", area))?;
        Driver::new(ui).settle()?;
        Ok((root.clone(), descendant(ui, &root, "target")?))
    }

    type Work = Option<Box<dyn FnOnce(&mut Runtime) -> Result<(), RuntimeError>>>;

    fn scroll_case(label: &str, work: impl FnOnce(WidgetId) -> Work) -> Check {
        let mut ui = runtime(300.0, 200.0)?;
        let action = Rc::new(RefCell::new(None));
        let log: Log = Rc::default();
        let (root, target) = scroll_fixture(&mut ui, action.clone(), log.clone())?;
        let area = root.clone();
        *action.borrow_mut() = work(area.clone());
        println!(
            "P14|{label} before: {} bounds={:?}",
            visibility(&ui, &[("area", &area), ("target", &target)]),
            ui.bounds(&target)?
        );
        let outcome = ui.reveal(&target);
        println!("P14|{label} Runtime::reveal(target) = {outcome:?}");
        let mounted = ui.key(&target).is_ok();
        println!(
            "P14|{label} after: log={:?} target_mounted={mounted} {}",
            log.borrow(),
            if mounted {
                visibility(&ui, &[("area", &area), ("target", &target)])
            } else {
                String::new()
            }
        );
        Driver::new(&mut ui).settle()?;
        let mounted = ui.key(&target).is_ok();
        println!(
            "P14|{label} after one more settle: log={:?} target_mounted={mounted} {}",
            log.borrow(),
            if mounted {
                visibility(&ui, &[("area", &area), ("target", &target)])
            } else {
                String::new()
            }
        );
        Ok(())
    }

    #[test]
    fn p14_reveal_hidden_refusal_paths() -> Check {
        // Control: an explicit set_visible on an ancestor.
        {
            let mut ui = runtime(300.0, 200.0)?;
            let (_root, group, target) = reveal_fixture(&mut ui)?;
            ui.set_visible(&group, false)?;
            Driver::new(&mut ui).settle()?;
            println!(
                "P14|explicit group={} target={} {}",
                ui.inspection_id(&group)?,
                ui.inspection_id(&target)?,
                visibility(&ui, &[("group", &group), ("target", &target)])
            );
            println!("P14|explicit Runtime::reveal(target) = {:?}", ui.reveal(&target));
            match Driver::new(&mut ui).click(&target) {
                Ok(()) => println!("P14|explicit Driver::click Ok"),
                Err(error) => println!("P14|explicit Driver::click Err Display: {error} | Debug: {error:?}"),
            }
        }
        // A window the host reports hidden (minimized), nothing hidden
        // explicitly and no container refusing.
        {
            let mut ui = runtime(300.0, 200.0)?;
            let (_root, group, target) = reveal_fixture(&mut ui)?;
            HostIntegration::set_window_visible(&mut ui, false)?;
            Driver::new(&mut ui).settle()?;
            println!(
                "P14|native-hidden target={} {}",
                ui.inspection_id(&target)?,
                visibility(&ui, &[("group", &group), ("target", &target)])
            );
            println!("P14|native-hidden Runtime::reveal(target) = {:?}", ui.reveal(&target));
            println!(
                "P14|native-hidden Runtime::reveal_and_focus(target) = {:?}",
                ui.reveal_and_focus(&target)
            );
            match Driver::new(&mut ui).click(&target) {
                Ok(()) => println!("P14|native-hidden Driver::click Ok"),
                Err(error) => {
                    println!("P14|native-hidden Driver::click Err Display: {error} | Debug: {error:?}")
                }
            }
            HostIntegration::set_window_visible(&mut ui, true)?;
            Driver::new(&mut ui).settle()?;
            println!("P14|native-shown-again Runtime::reveal(target) = {:?}", ui.reveal(&target));
        }
        // Revealing a target below the fold scrolls its ScrollArea inside the
        // reveal; on_scroll queues application work that runs during it.
        scroll_case("scroll/control", |_| None)?;
        scroll_case("scroll/deferred-set_visible(area,false)", |area| {
            Some(Box::new(move |ui: &mut Runtime| ui.set_visible(&area, false)))
        })?;
        scroll_case("scroll/deferred-unmount(area)", |area| {
            Some(Box::new(move |ui: &mut Runtime| ui.unmount(&area)))
        })?;
        // Boundary experiment, not a legitimate host: application work that
        // reports the window hidden in the middle of the reveal.
        let ran = Rc::new(Cell::new(false));
        let flag = ran.clone();
        scroll_case("scroll/deferred-host-set_window_visible(false)", move |_| {
            Some(Box::new(move |ui: &mut Runtime| {
                flag.set(true);
                HostIntegration::set_window_visible(ui, false)
            }))
        })?;
        println!("P14|boundary deferred host call ran={}", ran.get());
        Ok(())
    }
}
