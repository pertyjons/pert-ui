//! Follow-up verification probes for round 3 triage leads P7 to P11.
//!
//! Compiled unchanged against the frozen head (0.3.88) and baseline (0.3.75)
//! kits through the public facade only. Probes print observations with a
//! `P<n>|` prefix; assertions check only that the input reached the intended
//! scope (the menu opened, the intended row or tab exists), never the
//! behaviour under investigation, so every observation is printed.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests {
    use pert_ui::testing::{CpuRenderer, Driver, TestError};
    use pert_ui::*;
    use std::{cell::RefCell, collections::BTreeMap, error::Error, rc::Rc};

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

    fn property(ui: &Runtime, id: &WidgetId, name: &str) -> String {
        let Ok(serial) = ui.inspection_id(id) else {
            return "<unmounted>".into();
        };
        ui.inspect()
            .components
            .iter()
            .find(|c| c.id == serial)
            .and_then(|c| c.details.properties.get(name).map(|v| format!("{v:?}")))
            .unwrap_or_else(|| "<absent>".into())
    }

    fn descendant(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, Box<dyn Error>> {
        ui.find_descendant(root, key)?
            .ok_or_else(|| format!("missing {key}").into())
    }

    // ------------------------------------------------------------------ P8

    /// A menu bar whose File menu holds a command row (Save) and distinct
    /// plain rows, over content of stacked buttons. Every callback logs.
    fn menu_fixture(
        ui: &mut Runtime,
        rows: &[String],
    ) -> Result<(WidgetId, Log), Box<dyn Error>> {
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
        let receipts = log.clone();
        let mut items = vec![MenuItem::command(ui, "save", &save, move |_, receipt| {
            receipts.borrow_mut().push(format!("receipt:{receipt:?}"));
            Ok(())
        })?];
        for row in rows {
            let l = log.clone();
            let name = row.clone();
            items.push(MenuItem::new(row.to_lowercase(), row.clone(), move |_| {
                l.borrow_mut().push(format!("row:{name}"));
                Ok(())
            }));
        }
        let zoom = log.clone();
        let mut content = Vec::new();
        for i in 0..12 {
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
                MenuItem::submenu("file", "File", items),
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

    fn file_items(ui: &Runtime, root: &WidgetId) -> Result<Option<WidgetId>, Box<dyn Error>> {
        let title = descendant(ui, root, "title/file")?;
        Ok(ui.find_descendant(&title, "items")?)
    }

    fn active_of(ui: &Runtime, root: &WidgetId) -> String {
        match file_items(ui, root) {
            Ok(Some(items)) => format!(
                "active={} visible={}",
                property(ui, &items, "active"),
                ui.inspect()
                    .components
                    .iter()
                    .find(|c| Some(c.id) == ui.inspection_id(&items).ok())
                    .is_some_and(|c| c.visible)
            ),
            Ok(None) => "items=<not mounted>".into(),
            Err(error) => format!("items=<{error}>"),
        }
    }

    fn rows(extra: usize) -> Vec<String> {
        if extra == 0 {
            vec!["Beta".into(), "Gamma".into(), "Delta".into()]
        } else {
            (1..=extra).map(|i| format!("R{i:02}")).collect()
        }
    }

    /// Opens File by F10, Down on a fixture of its own and returns the menu
    /// panel's bounds and, for each shown row, the window point at its middle
    /// with the label a host move there highlights.
    fn calibrate(rows: &[String]) -> Result<Vec<(String, Point, [f32; 4])>, Box<dyn Error>> {
        let mut ui = runtime(640.0, 640.0)?;
        let (root, _log) = menu_fixture(&mut ui, rows)?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime_mut().focus_next(FocusDirection::Forward)?;
        d.key(KeyCode::F10)?;
        d.key(KeyCode::ArrowDown)?;
        let items = file_items(d.runtime(), &root)?.ok_or("File menu rows not mounted")?;
        let [x, y, w, h] = d.runtime().bounds(&items)?.ok_or("rows not laid out")?.values();
        let shown = (rows.len() + 1).min(8);
        let row = h / shown as f32;
        println!("P8|calibrate rows={} items_bounds=[{x},{y},{w},{h}] row_height={row}", rows.len() + 1);
        let mut points = Vec::new();
        for i in 0..shown {
            let at = Point::new(x + w / 2.0, y + (i as f32 + 0.5) * row)?;
            d.runtime_mut()
                .pointer_input(PointerEvent::new(at, PointerKind::Move))?;
            d.settle()?;
            let active = property(d.runtime(), &items, "active");
            println!("P8|calibrate host move to row {i} at {at:?} -> {active}");
            points.push((active, at, [x, y + i as f32 * row, w, row]));
        }
        Ok(points)
    }

    enum Park {
        Never,
        Move(Point),
        MoveThenLeave(Point),
        Click(String),
    }

    /// Focus content, park the pointer, F10, Down (raw, observed before and
    /// after its settle), further Downs, Enter. Returns the callback log.
    fn menu_scenario(
        label: &str,
        rows: &[String],
        park: Park,
        downs: usize,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let mut ui = runtime(640.0, 640.0)?;
        let (root, log) = menu_fixture(&mut ui, rows)?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime_mut().focus_next(FocusDirection::Forward)?;
        d.settle()?;
        match &park {
            Park::Never => {}
            Park::Move(at) => {
                d.runtime_mut()
                    .pointer_input(PointerEvent::new(*at, PointerKind::Move))?;
                d.settle()?;
            }
            Park::MoveThenLeave(at) => {
                d.runtime_mut()
                    .pointer_input(PointerEvent::new(*at, PointerKind::Move))?;
                d.settle()?;
                d.runtime_mut()
                    .pointer_input(PointerEvent::new(*at, PointerKind::Leave))?;
                d.settle()?;
            }
            Park::Click(key) => {
                let target = descendant(d.runtime(), &root, key)?;
                d.click(&target)?;
            }
        }
        println!(
            "P8|{label} parked focus={} log={:?}",
            focused_key(d.runtime()),
            log.borrow()
        );
        d.key(KeyCode::F10)?;
        let after_f10 = focused_key(d.runtime());
        println!("P8|{label} F10 focus={after_f10} {}", active_of(d.runtime(), &root));
        assert!(after_f10.starts_with("title/file"), "F10 did not focus File");
        {
            let ui = d.runtime_mut();
            ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
            ui.key_input(KeyCode::ArrowDown, KeyAction::Release)?;
        }
        println!("P8|{label} Down before settle {}", active_of(d.runtime(), &root));
        d.settle()?;
        let open = active_of(d.runtime(), &root);
        println!(
            "P8|{label} Down after settle focus={} {open}",
            focused_key(d.runtime())
        );
        assert!(open.contains("visible=true"), "File menu did not open");
        d.settle()?;
        println!("P8|{label} Down after second settle {}", active_of(d.runtime(), &root));
        for n in 1..=downs {
            d.key(KeyCode::ArrowDown)?;
            println!("P8|{label} Down#{} {}", n + 1, active_of(d.runtime(), &root));
        }
        d.key(KeyCode::Enter)?;
        d.settle()?;
        let ran = log.borrow().clone();
        println!(
            "P8|{label} Enter log={ran:?} focus={} {}",
            focused_key(d.runtime()),
            active_of(d.runtime(), &root)
        );
        Ok(ran)
    }

    #[test]
    fn p8_keyboard_menu_highlight_and_stale_pointer() -> Check {
        let short = rows(0);
        let points = calibrate(&short)?;
        assert_eq!(points.len(), 4);
        let gamma = points[2].1;
        let mut summary = Vec::new();
        summary.push(("never", menu_scenario("short/never", &short, Park::Never, 0)?));
        summary.push(("move-over-gamma", menu_scenario("short/move", &short, Park::Move(gamma), 0)?));
        summary.push((
            "move-over-gamma-then-leave",
            menu_scenario("short/leave", &short, Park::MoveThenLeave(gamma), 0)?,
        ));
        summary.push((
            "move-outside-menu",
            menu_scenario("short/outside", &short, Park::Move(Point::new(620.0, 620.0)?), 0)?,
        ));
        summary.push((
            "never+Down",
            menu_scenario("short/never-down", &short, Park::Never, 1)?,
        ));
        summary.push((
            "move-over-gamma+Down",
            menu_scenario("short/move-down", &short, Park::Move(gamma), 1)?,
        ));
        // A Driver::click on a content button whose middle a menu row will
        // cover, as a test that clicked content earlier leaves the pointer.
        {
            let mut ui = runtime(640.0, 640.0)?;
            let (root, _) = menu_fixture(&mut ui, &short)?;
            Driver::new(&mut ui).settle()?;
            let mut chosen = None;
            for i in 0..12 {
                let key = format!("c{i}");
                let id = descendant(&ui, &root, &key)?;
                let [x, y, w, h] = ui.bounds(&id)?.ok_or("content not laid out")?.values();
                let (cx, cy) = (x + w / 2.0, y + h / 2.0);
                for (row, (label, _, [rx, ry, rw, rh])) in points.iter().enumerate().skip(1) {
                    if chosen.is_none() && cx >= *rx && cx < rx + rw && cy >= *ry && cy < ry + rh {
                        println!(
                            "P8|click target {key} middle=({cx},{cy}) lies in row {row} {label}"
                        );
                        chosen = Some(key.clone());
                    }
                }
            }
            match chosen {
                Some(key) => summary.push((
                    "driver-click-under-row",
                    menu_scenario("short/click", &short, Park::Click(key), 0)?,
                )),
                None => println!("P8|click target: no content button middle lies in a row"),
            }
        }
        // A long menu that scrolls: a reveal that changes layout repeats the
        // refreshed move while the pointer stays parked.
        let long = rows(13);
        let long_points = calibrate(&long)?;
        let parked = long_points[2].1;
        summary.push(("long/never", menu_scenario("long/never", &long, Park::Never, 12)?));
        summary.push((
            "long/move-over-row2",
            menu_scenario("long/move", &long, Park::Move(parked), 12)?,
        ));
        summary.push((
            "long/move-then-leave",
            menu_scenario("long/leave", &long, Park::MoveThenLeave(parked), 12)?,
        ));
        for (name, ran) in summary {
            println!("P8|SUMMARY {name}: {ran:?}");
        }
        Ok(())
    }

    // ------------------------------------------------------------------ P7

    struct Rows {
        keys: Vec<String>,
        selection: ListSelection,
        log: Log,
        list: Rc<RefCell<Option<WidgetId>>>,
        set_active: bool,
        refocus: bool,
    }

    impl ListItems for Rows {
        fn len(&self) -> usize {
            self.keys.len()
        }
        fn key(&self, index: usize) -> Option<String> {
            self.keys.get(index).cloned()
        }
        fn build(&self, index: usize) -> Option<Element> {
            let key = self.keys.get(index)?.clone();
            let (selection, log, list) = (self.selection.clone(), self.log.clone(), self.list.clone());
            let (set_active, refocus) = (self.set_active, self.refocus);
            let row = key.clone();
            Some(Element::new(
                key.clone(),
                Button::new(format!("Open {key}"), move |cx| {
                    log.borrow_mut().push(format!("button:{row}"));
                    if set_active {
                        selection.set_active(Some(row.clone()))?;
                    }
                    if refocus && let Some(list) = list.borrow().as_ref() {
                        cx.request_focus_for(list)?;
                    }
                    Ok(())
                }),
            ))
        }
        fn index_of(&self, key: &str) -> Option<usize> {
            self.keys.iter().position(|k| k == key)
        }
    }

    fn list_state(ui: &Runtime, selection: &ListSelection, log: &Log) -> Result<String, Box<dyn Error>> {
        Ok(format!(
            "focus={} active={:?} selected={:?} log={:?}",
            focused_key(ui),
            selection.active().get()?,
            selection.selected().get()?,
            log.borrow()
        ))
    }

    fn p7_variant(label: &str, set_active: bool, refocus: bool) -> Check {
        let mut ui = runtime(400.0, 360.0)?;
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let log: Log = Rc::default();
        let slot = Rc::new(RefCell::new(None));
        let source = ui.signal(ListSource::new(Rows {
            keys: ["a", "b", "c", "d", "e"].map(String::from).to_vec(),
            selection: selection.clone(),
            log: log.clone(),
            list: slot.clone(),
            set_active,
            refocus,
        }));
        let activated = log.clone();
        let list = VirtualList::new(source.read_only(), 32.0)?
            .selection(selection.clone())
            .item_tab_stops(false)
            .on_activate(move |_, key| {
                activated.borrow_mut().push(format!("activate:{key}"));
                Ok(())
            });
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("before", Button::new("Before", |_| Ok(()))),
                Element::new("list", list),
            ]),
        ))?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        let list = descendant(d.runtime(), &root, "list")?;
        *slot.borrow_mut() = Some(list.clone());
        println!("P7|{label} start {}", list_state(d.runtime(), &selection, &log)?);
        let row_b = d.runtime().find(&list, &["b"])?.ok_or("row b not mounted")?;
        d.click(&row_b)?;
        assert!(log.borrow().iter().any(|e| e == "button:b"), "click missed row b");
        println!("P7|{label} click b {}", list_state(d.runtime(), &selection, &log)?);
        d.key(KeyCode::ArrowDown)?;
        println!("P7|{label} Down {}", list_state(d.runtime(), &selection, &log)?);
        d.key(KeyCode::Enter)?;
        println!("P7|{label} Enter {}", list_state(d.runtime(), &selection, &log)?);
        d.key(KeyCode::ArrowDown)?;
        println!("P7|{label} Down2 {}", list_state(d.runtime(), &selection, &log)?);
        d.key(KeyCode::Tab)?;
        println!("P7|{label} Tab {}", list_state(d.runtime(), &selection, &log)?);
        Ok(())
    }

    #[test]
    fn p7_button_rows_pointer_selection_focus_and_arrows() -> Check {
        p7_variant("plain-button", false, false)?;
        p7_variant("button+set_active", true, false)?;
        p7_variant("button+set_active+request_focus_for(list)", true, true)?;
        Ok(())
    }

    // ------------------------------------------------------------------ P9

    fn dialog_fixture(ui: &mut Runtime) -> Result<(WidgetId, DialogController), Box<dyn Error>> {
        let controller = DialogController::new(ui);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("doc", Button::new("Document", |_| Ok(()))),
                Element::new(
                    "confirm",
                    Dialog::new(
                        controller.clone(),
                        Element::new("anchor", Button::new("Close tab", |_| Ok(()))),
                        Element::new("save", Button::new("Save", |_| Ok(()))),
                    ),
                ),
            ]),
        ))?;
        Driver::new(ui).settle()?;
        Ok((root, controller))
    }

    /// The shape of BC2's failing test: the runtime is a local of the
    /// function that returns the error, so it is dropped before the caller
    /// (or the test harness) formats it.
    fn test_shaped_click() -> Result<(), Box<dyn Error>> {
        let mut ui = runtime(480.0, 320.0)?;
        let (root, _controller) = dialog_fixture(&mut ui)?;
        let save = descendant(&ui, &root, "save")?;
        Driver::new(&mut ui).click(&save)?;
        Ok(())
    }

    #[test]
    fn p9_not_revealed_diagnostic_before_and_after_runtime_drop() -> Check {
        let mut ui = runtime(480.0, 320.0)?;
        let (root, controller) = dialog_fixture(&mut ui)?;
        let owner = descendant(&ui, &root, "confirm")?;
        let save = descendant(&ui, &root, "save")?;
        println!(
            "P9|owner key={} inspection_id={} save inspection_id={}",
            ui.key(&owner)?,
            ui.inspection_id(&owner)?,
            ui.inspection_id(&save)?
        );
        for c in ui.inspect().components.iter().filter(|c| {
            ["root", "doc", "confirm", "anchor", "dialog", "save"].contains(&c.key.as_str())
        }) {
            println!(
                "P9|component id={} key={} path={:?} visible={} parent={:?}",
                c.id, c.key, c.path, c.visible, c.parent
            );
        }
        println!("P9|Runtime::reveal(save) = {:?}", ui.reveal(&save)?);
        let error = Driver::new(&mut ui)
            .click(&save)
            .expect_err("closed dialog content was clicked");
        let TestError::NotRevealed { key, refusal } = &error else {
            return Err(format!("unexpected error {error:?}").into());
        };
        let RevealRefusal::AncestorRefused { ancestor } = refusal else {
            return Err(format!("unexpected refusal {refusal:?}").into());
        };
        println!(
            "P9|error key={key} ancestor key={} ancestor inspection_id={} ancestor==owner:{}",
            ui.key(ancestor)?,
            ui.inspection_id(ancestor)?,
            *ancestor == owner
        );
        println!("P9|alive Display: {error}");
        println!("P9|alive Debug:   {error:?}");
        controller.open()?;
        Driver::new(&mut ui).settle()?;
        println!("P9|alive, dialog now open, same error Display: {error}");
        Driver::new(&mut ui).click(&save)?;
        println!("P9|alive, dialog open: click(save) now Ok");
        drop(ui);
        println!("P9|after runtime drop Display: {error}");
        println!("P9|after runtime drop Debug:   {error:?}");

        // The same refusal, its owner unmounted while the runtime lives.
        let mut ui = runtime(480.0, 320.0)?;
        let (root, _controller) = dialog_fixture(&mut ui)?;
        let save = descendant(&ui, &root, "save")?;
        let error = Driver::new(&mut ui).click(&save).expect_err("closed dialog");
        println!("P9|second runtime alive Display: {error}");
        ui.unmount(&root)?;
        println!("P9|second runtime alive, owner unmounted Display: {error}");

        let shaped = test_shaped_click().expect_err("closed dialog");
        println!("P9|test-shaped Err as the harness prints it: Error: {shaped:?}");
        Ok(())
    }

    // ------------------------------------------------------------------ P10

    fn ascii(image: &FrameImage, [x, y, w, h]: [f32; 4]) -> (Vec<String>, Vec<(u32, u32)>) {
        let inset = 3;
        let (x0, y0) = (x.round() as u32 + inset, y.round() as u32 + inset);
        let (x1, y1) = (
            ((x + w).round() as u32).saturating_sub(inset).min(image.width()),
            ((y + h).round() as u32).saturating_sub(inset).min(image.height()),
        );
        let px = |px: u32, py: u32| {
            let i = ((py * image.width() + px) * 4) as usize;
            let p = image.pixels();
            [p[i], p[i + 1], p[i + 2], p[i + 3]]
        };
        let mut counts: BTreeMap<[u8; 4], usize> = BTreeMap::new();
        for py in y0..y1 {
            for qx in x0..x1 {
                *counts.entry(px(qx, py)).or_default() += 1;
            }
        }
        let background = counts.into_iter().max_by_key(|(_, n)| *n).map(|(c, _)| c).unwrap_or_default();
        let diff = |a: [u8; 4]| {
            (0..3)
                .map(|i| (i32::from(a[i]) - i32::from(background[i])).unsigned_abs())
                .max()
                .unwrap_or(0)
        };
        let mut lines = Vec::new();
        let mut ink_columns = vec![false; (x1 - x0) as usize];
        for py in y0..y1 {
            let mut line = String::new();
            for qx in x0..x1 {
                let d = diff(px(qx, py));
                line.push(if d > 96 {
                    '#'
                } else if d > 32 {
                    '+'
                } else {
                    '.'
                });
                if d > 32 {
                    ink_columns[(qx - x0) as usize] = true;
                }
            }
            if line.contains(['#', '+']) {
                lines.push(format!("y={py:3} {}", line.trim_end_matches('.')));
            }
        }
        let mut runs = Vec::new();
        let mut start = None;
        for (i, ink) in ink_columns.iter().chain([&false]).enumerate() {
            match (start, *ink) {
                (None, true) => start = Some(i as u32),
                (Some(s), false) => {
                    runs.push((x0 + s, x0 + i as u32));
                    start = None;
                }
                _ => {}
            }
        }
        lines.insert(0, format!("crop x={x0}..{x1} y={y0}..{y1} background={background:?}"));
        (lines, runs)
    }

    #[test]
    fn p10_text_area_literal_tab_cpu_capture() -> Check {
        let mut ui = runtime(320.0, 440.0)?;
        let cases = [("tab", "A\tB"), ("space", "A B"), ("adjacent", "AB")];
        let mut signals = Vec::new();
        let mut children = Vec::new();
        for (key, text) in cases {
            let signal = ui.signal(text.to_owned());
            signals.push(signal.clone());
            children.push(Element::new(key, TextArea::new(signal)));
        }
        let root = ui.mount_tree(Element::new("root", Column::new(children)))?;
        let mut d = Driver::new(&mut ui);
        d.settle()?;
        d.runtime().set_capture_support(true);
        let pending = d.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
        d.settle()?;
        CpuRenderer::new().serve_captures(d.runtime_mut());
        let capture = pending.try_reply()?.value?;
        let image = &capture.image;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures");
        std::fs::create_dir_all(&dir)?;
        let file = dir.join("p10-textareas.png");
        std::fs::write(&file, image.encode_png())?;
        println!("P10|capture {}x{} written to {}", image.width(), image.height(), file.display());
        for ((key, text), signal) in cases.iter().zip(&signals) {
            let id = descendant(d.runtime(), &root, key)?;
            let bounds = d.runtime().bounds(&id)?.ok_or("not laid out")?.values();
            let semantics = d.semantics(&id)?;
            println!(
                "P10|{key} canonical={:?} bytes={:?} inspect.text={} semantics.value={:?} bounds={bounds:?}",
                signal.get()?,
                signal.get()?.as_bytes(),
                property(d.runtime(), &id, "text"),
                semantics.value
            );
            assert_eq!(signal.get()?, *text);
            let (lines, runs) = ascii(image, bounds);
            println!("P10|{key} ink column runs (x ranges) = {runs:?}");
            for line in lines {
                println!("P10|{key} {line}");
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------ P11

    fn tab_state(ui: &mut Runtime, tabs: &WidgetId, log: &Log) -> Result<String, Box<dyn Error>> {
        let semantics = Driver::new(ui).semantics(tabs)?;
        let names: Vec<_> = semantics
            .items
            .iter()
            .map(|item| format!("{}{}", item.name.clone().unwrap_or_default(), if item.selected { "*" } else { "" }))
            .collect();
        Ok(format!("titles={names:?} focus={} log={:?}", focused_key(ui), log.borrow()))
    }

    fn editor(ui: &Runtime, tabs: &WidgetId, key: &str) -> String {
        match ui.find(tabs, &[key, "editor"]) {
            Ok(Some(id)) => format!("{key}/editor#{}", ui.inspection_id(&id).unwrap_or(0)),
            Ok(None) => format!("{key}/editor=<none>"),
            Err(error) => format!("{key}/editor=<{error:?}>"),
        }
    }

    fn on_select_log(log: &Log) -> impl FnMut(&mut EventContext, &SelectionChange) -> Result<(), RuntimeError> + 'static {
        let log = log.clone();
        move |_, change| {
            log.borrow_mut().push(format!(
                "{:?}->{:?} {:?}",
                change.previous, change.selected, change.origin
            ));
            Ok(())
        }
    }

    /// Runs the same steps against a strip that `set` redescribes with the
    /// given keys.
    fn p11_steps(
        label: &str,
        ui: &mut Runtime,
        tabs: &WidgetId,
        log: &Log,
        texts: &BTreeMap<&'static str, Signal<String>>,
        set: &dyn Fn(&mut Runtime, &[&str]) -> Result<(), Box<dyn Error>>,
    ) -> Check {
        println!("P11|{label} start {} {}", tab_state(ui, tabs, log)?, editor(ui, tabs, "a"));
        set(ui, &["a", "b", "c"])?;
        Driver::new(ui).settle()?;
        println!("P11|{label} add c, no reveal: {} {}", tab_state(ui, tabs, log)?, editor(ui, tabs, "a"));
        let outcome = ui.reveal_child(tabs, "b")?;
        Driver::new(ui).settle()?;
        println!("P11|{label} reveal_child(b)={outcome:?}: {}", tab_state(ui, tabs, log)?);
        let editor_b = ui.find(tabs, &["b", "editor"])?.ok_or("b editor missing")?;
        ui.focus(&editor_b)?;
        Driver::new(ui).type_text("1")?;
        println!(
            "P11|{label} focus b editor and type: {} b_text={:?}",
            tab_state(ui, tabs, log)?,
            texts["b"].get()?
        );
        set(ui, &["a", "c"])?;
        Driver::new(ui).settle()?;
        println!(
            "P11|{label} remove focused+selected b: {} {} {} b_editor_mounted={}",
            tab_state(ui, tabs, log)?,
            editor(ui, tabs, "a"),
            editor(ui, tabs, "c"),
            ui.key(&editor_b).is_ok()
        );
        Driver::new(ui).type_text("Z")?;
        println!(
            "P11|{label} type after removal: texts a={:?} b={:?} c={:?} focus={}",
            texts["a"].get()?,
            texts["b"].get()?,
            texts["c"].get()?,
            focused_key(ui)
        );
        Driver::new(ui).key(KeyCode::ArrowLeft)?;
        println!("P11|{label} ArrowLeft after removal: {}", tab_state(ui, tabs, log)?);
        set(ui, &["a", "c", "d"])?;
        Driver::new(ui).settle()?;
        println!("P11|{label} add d: {}", tab_state(ui, tabs, log)?);
        set(ui, &[])?;
        Driver::new(ui).settle()?;
        println!("P11|{label} remove all: {}", tab_state(ui, tabs, log)?);
        set(ui, &["d"])?;
        Driver::new(ui).settle()?;
        println!("P11|{label} describe d into empty: {}", tab_state(ui, tabs, log)?);
        Ok(())
    }

    fn p11_texts(ui: &Runtime) -> BTreeMap<&'static str, Signal<String>> {
        ["a", "b", "c", "d"]
            .into_iter()
            .map(|key| (key, ui.signal(format!("text {key}"))))
            .collect()
    }

    fn p11_tab(texts: &BTreeMap<&'static str, Signal<String>>, key: &str) -> Tab {
        Tab::new(
            key.to_owned(),
            key.to_uppercase(),
            Element::new("editor", TextArea::new(texts[key].clone())),
        )
    }

    #[test]
    fn p11_dynamic_tabs_add_remove_without_reveal() -> Check {
        let mut ui = runtime(480.0, 320.0)?;
        let texts = p11_texts(&ui);
        let log: Log = Rc::default();
        let docs = ui.signal(vec!["a".to_owned(), "b".to_owned()]);
        let (build_texts, build_log) = (texts.clone(), log.clone());
        let root = ui.mount_tree(Element::new(
            "dynamic",
            Dynamic::new(&docs, move |_, docs: &Vec<String>| {
                Element::new(
                    "tabs",
                    Tabs::new(docs.iter().map(|key| p11_tab(&build_texts, key)))
                        .on_select(on_select_log(&build_log)),
                )
            }),
        ))?;
        Driver::new(&mut ui).settle()?;
        let tabs = descendant(&ui, &root, "tabs")?;
        let set = move |_: &mut Runtime, keys: &[&str]| -> Result<(), Box<dyn Error>> {
            docs.set(keys.iter().map(|k| (*k).to_owned()).collect())?;
            Ok(())
        };
        p11_steps("dynamic", &mut ui, &tabs, &log, &texts, &set)?;
        let tabs_now = descendant(&ui, &root, "tabs")?;
        println!("P11|dynamic strip identity retained: {}", tabs_now == tabs);
        Ok(())
    }

    #[test]
    fn p11_reconcile_children_tabs_add_remove_without_reveal() -> Check {
        let mut ui = runtime(480.0, 320.0)?;
        let texts = p11_texts(&ui);
        let log: Log = Rc::default();
        let tabs = ui.mount_tree(Element::new(
            "tabs",
            Tabs::new([p11_tab(&texts, "a"), p11_tab(&texts, "b")]).on_select(on_select_log(&log)),
        ))?;
        Driver::new(&mut ui).settle()?;
        let (set_texts, target) = (texts.clone(), tabs.clone());
        let set = move |ui: &mut Runtime, keys: &[&str]| -> Result<(), Box<dyn Error>> {
            ui.reconcile_children(
                &target,
                keys.iter().map(|key| p11_tab(&set_texts, key).into_element()).collect(),
            )?;
            Ok(())
        };
        p11_steps("reconcile_children", &mut ui, &tabs, &log, &texts, &set)?;
        Ok(())
    }
}
