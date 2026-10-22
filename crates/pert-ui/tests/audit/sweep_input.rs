//! Input admission regressions from the post-repair review sweep.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn send(ui: &mut Runtime, code: KeyCode, action: KeyAction) {
    ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
        .unwrap();
    ui.flush(ui.now()).unwrap();
}
fn modified() -> [Modifiers; 3] {
    [
        Modifiers::PRIMARY,
        Modifiers {
            alt: true,
            ..Modifiers::default()
        },
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    ]
}
fn counting_command(
    ui: &mut Runtime,
    code: KeyCode,
    modifiers: Modifiers,
) -> (Rc<Cell<usize>>, CommandId) {
    let hits = Rc::new(Cell::new(0));
    let counted = hits.clone();
    let command = ui
        .register_command(
            "Navigation",
            CommandScope::Application,
            vec![KeyChord::new(CommandKey::Named(code), modifiers)],
            move |_, _| {
                counted.set(counted.get() + 1);
                Ok(())
            },
        )
        .unwrap();
    (hits, command)
}

#[test]
fn modified_choice_navigation_yields_to_application_commands() {
    for kind in 0..3 {
        for code in [KeyCode::ArrowRight, KeyCode::ArrowDown, KeyCode::End] {
            for modifiers in modified() {
                let mut ui = runtime();
                let value = ui.signal(0_u32);
                let choices = (0..3).map(|n| Choice::new(n, n.to_string()));
                let element = match kind {
                    0 => Element::new("control", RadioGroup::new(value.clone(), choices).unwrap()),
                    1 => Element::new(
                        "control",
                        SegmentedControl::new(value.clone(), choices).unwrap(),
                    ),
                    _ => Element::new(
                        "control",
                        InlineChoice::new(value.clone(), choices).unwrap(),
                    ),
                };
                let root = ui.mount_tree(element).unwrap();
                ui.flush(Duration::ZERO).unwrap();
                ui.focus(&root).unwrap();
                let (hits, _command) = counting_command(&mut ui, code, modifiers);
                ui.native_modifiers(modifiers).unwrap();
                send(&mut ui, code, KeyAction::Press);
                send(&mut ui, code, KeyAction::Release);
                assert_eq!(hits.get(), 1, "{kind} {code:?} {modifiers:?}");
                assert_eq!(value.get().unwrap(), 0, "{kind} {code:?} {modifiers:?}");
                ui.native_modifiers(Modifiers::default()).unwrap();
                send(&mut ui, code, KeyAction::Press);
                send(&mut ui, code, KeyAction::Release);
                assert_ne!(value.get().unwrap(), 0, "{kind} {code:?} unmodified");
            }
        }
    }
}

#[test]
fn modified_numeric_steps_yield_to_application_commands() {
    // 0, 1: idle and typing InlineSlider; 2: Slider; 3: IntegerInput;
    // 4, 5: idle and typing InlineInteger.
    for kind in 0..6 {
        for modifiers in [
            Modifiers::PRIMARY,
            Modifiers {
                alt: true,
                ..Modifiers::default()
            },
        ] {
            let mut ui = runtime();
            let (parameter, _host) = ui
                .parameter(
                    ParameterId::new(1, 1, "amount").unwrap(),
                    ParameterSpec::new("Amount", 0.0, 100.0, 10.0).unwrap(),
                )
                .unwrap();
            let number = ui.signal(5_i64);
            let element = match kind {
                0 | 1 => Element::new("control", InlineSlider::new(parameter.clone())),
                2 => Element::new("control", Slider::new(parameter.clone())),
                3 => Element::new("control", IntegerInput::new(number.clone())),
                _ => Element::new("control", InlineInteger::new(number.clone())),
            };
            let root = ui.mount_tree(element).unwrap();
            ui.flush(Duration::ZERO).unwrap();
            ui.focus(&root).unwrap();
            if kind == 1 || kind == 5 {
                send(&mut ui, KeyCode::F2, KeyAction::Press);
                send(&mut ui, KeyCode::F2, KeyAction::Release);
            }
            let (hits, _command) = counting_command(&mut ui, KeyCode::ArrowUp, modifiers);
            ui.native_modifiers(modifiers).unwrap();
            send(&mut ui, KeyCode::ArrowUp, KeyAction::Press);
            send(&mut ui, KeyCode::ArrowUp, KeyAction::Release);
            ui.native_modifiers(Modifiers::default()).unwrap();
            // A number field's text editor keeps its own chords, such as
            // Control+Up to the start, as TextField does; it never steps.
            if kind != 3 && kind != 5 {
                assert_eq!(hits.get(), 1, "{kind} {modifiers:?}");
            }
            assert_eq!(parameter.snapshot().get().unwrap().base, 10.0, "{kind}");
            assert_eq!(number.get().unwrap(), 5, "{kind}");
        }
    }
}

#[test]
fn shift_still_steps_numeric_controls_finely() {
    let mut ui = runtime();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 10.0).unwrap(),
        )
        .unwrap();
    let root = ui
        .mount_tree(Element::new("control", Slider::new(parameter.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&root).unwrap();
    ui.native_modifiers(Modifiers {
        shift: true,
        ..Modifiers::default()
    })
    .unwrap();
    send(&mut ui, KeyCode::ArrowUp, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowUp, KeyAction::Release);
    let base = parameter.snapshot().get().unwrap().base;
    assert!(base > 10.0 && base < 11.0, "{base}");
}

/// Rebuilds its buttons from `count` and empties them when asked to reveal.
struct Shrinking {
    count: Signal<usize>,
    built: Option<usize>,
}
impl Component for Shrinking {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let count = cx.read(&self.count)?;
        if self.built != Some(count) {
            self.built = Some(count);
            let buttons: Vec<_> = (0..count)
                .map(|n| Element::new(format!("b{n}"), Button::new(format!("B{n}"), |_| Ok(()))))
                .collect();
            cx.request_children(vec![Element::new("column", Column::new(buttons))]);
        }
        Ok(())
    }
    fn reveal(&mut self, _cx: &mut EventContext, _rect: Rect) -> Result<bool, RuntimeError> {
        if self.count.get()? > 0 {
            self.count.set(0)?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[test]
fn tab_traversal_survives_a_reveal_that_removes_the_candidates() {
    let mut ui = runtime();
    let count = ui.signal(3);
    ui.mount_tree(Element::new(
        "host",
        Shrinking {
            count: count.clone(),
            built: None,
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus_next(false).unwrap();
    assert_eq!(count.get().unwrap(), 0, "the reveal removed every button");
    assert_eq!(ui.focused(), None);
}

#[test]
fn a_command_that_shuts_down_keeps_its_own_error() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("label", Label::new("Content")))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let command = ui
        .register_command("Quit", CommandScope::Application, vec![], |ui, _| {
            ui.shutdown();
            Err(ApplicationError::from("save failed").into())
        })
        .unwrap();
    let context = ui.command_context(CommandSource::Programmatic);
    let error = ui.invoke_command(&command, context).unwrap_err();
    assert!(
        matches!(&error, RuntimeError::Application(cause) if cause.to_string() == "save failed"),
        "{error:?}"
    );
}

#[test]
fn a_key_command_that_shuts_down_keeps_its_own_error() {
    let mut ui = runtime();
    ui.mount_tree(Element::new("label", Label::new("Content")))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let _command = ui
        .register_command(
            "Quit",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::F9),
                Modifiers::default(),
            )],
            |ui, _| {
                ui.shutdown();
                Err(ApplicationError::from("save failed").into())
            },
        )
        .unwrap();
    let error = ui
        .command_key_input(CommandKeyInput::new(
            KeyCode::F9,
            KeyAction::Press,
            CommandKey::Named(KeyCode::F9),
        ))
        .unwrap_err();
    assert!(
        matches!(&error, RuntimeError::Application(cause) if cause.to_string() == "save failed"),
        "{error:?}"
    );
}

// S4: navigation keys follow the admission of activation keys.

/// The whole inspection, which a yielded chord must leave unchanged.
fn snapshot(ui: &Runtime) -> String {
    format!("{:?}", ui.inspect().components)
}
/// Focuses `target`, sends `code` with `modifiers` and checks that an
/// application command bound to that chord runs and nothing else changes.
fn chord_yields(
    ui: &mut Runtime,
    target: &WidgetId,
    code: KeyCode,
    modifiers: Modifiers,
    what: &str,
) {
    if ui.focused().as_ref() != Some(target) {
        ui.focus(target).unwrap();
    }
    ui.flush(ui.now()).unwrap();
    let (hits, command) = counting_command(ui, code, modifiers);
    let before = snapshot(ui);
    ui.native_modifiers(modifiers).unwrap();
    send(ui, code, KeyAction::Press);
    send(ui, code, KeyAction::Release);
    ui.native_modifiers(Modifiers::default()).unwrap();
    assert_eq!(
        hits.get(),
        1,
        "{what}: {code:?} {modifiers:?} reaches the command"
    );
    assert_eq!(
        snapshot(ui),
        before,
        "{what}: {code:?} {modifiers:?} changes nothing"
    );
    ui.unregister_command(&command).unwrap();
}
fn alt() -> Modifiers {
    Modifiers {
        alt: true,
        ..Modifiers::default()
    }
}
fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..Modifiers::default()
    }
}
fn super_key() -> Modifiers {
    Modifiers {
        super_key: true,
        ..Modifiers::default()
    }
}
fn width(ui: &Runtime, root: &WidgetId, key: &str) -> f32 {
    let id = ui.find_descendant(root, key).unwrap().unwrap();
    ui.bounds(&id).unwrap().unwrap().width()
}
fn leaf(key: &str) -> Element {
    Element::new(key, Button::new(key, |_| Ok(())))
}

#[test]
fn modified_resize_chords_yield_and_shift_still_resizes() {
    let mut ui = runtime();
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(SplitAxis::LeftRight, leaf("first"), leaf("second")),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for modifiers in [Modifiers::PRIMARY, alt()] {
        chord_yields(&mut ui, &split, KeyCode::ArrowRight, modifiers, "split");
    }
    let before = width(&ui, &split, "first");
    ui.native_modifiers(shift()).unwrap();
    send(&mut ui, KeyCode::ArrowRight, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowRight, KeyAction::Release);
    ui.native_modifiers(Modifiers::default()).unwrap();
    assert!(
        (width(&ui, &split, "first") - before - 32.0).abs() < 1e-3,
        "Shift resizes"
    );

    let mut ui = runtime();
    let grid = ui
        .mount_tree(Element::new(
            "grid",
            PropertyGrid::new(vec![Property::new("a", "Label a", leaf("a-editor"))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for modifiers in [Modifiers::PRIMARY, alt()] {
        chord_yields(&mut ui, &grid, KeyCode::ArrowRight, modifiers, "grid");
    }

    let mut ui = runtime();
    let tools = ui
        .mount_tree(Element::new(
            "tools",
            ToolWindowLayout::new(leaf("main"))
                .edge(ToolEdge::Left, EdgeMode::Stripe)
                .window(
                    ToolWindow::new("a", "Tool", leaf("body-a"))
                        .edge(ToolEdge::Left)
                        .expanded(true),
                ),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let divider = ui.find_descendant(&tools, "divider-left").unwrap().unwrap();
    for modifiers in [Modifiers::PRIMARY, alt()] {
        chord_yields(
            &mut ui,
            &divider,
            KeyCode::ArrowRight,
            modifiers,
            "tool divider",
        );
    }
    let stripe = ui.find_descendant(&tools, "stripe-left").unwrap().unwrap();
    for (code, modifiers) in [
        (KeyCode::ArrowDown, Modifiers::PRIMARY),
        (KeyCode::End, alt()),
    ] {
        chord_yields(&mut ui, &stripe, code, modifiers, "stripe");
    }
}

/// Rows keyed `r0`, `r1`, … with plain labels.
#[derive(Clone)]
struct Rows(usize);
impl ListItems for Rows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("r{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        let key = self.key(index)?;
        Some(Element::new(key.clone(), Label::new(key)))
    }
}

#[test]
fn modified_scrolling_chords_yield_to_commands() {
    let mut ui = runtime();
    let rows: Vec<_> = (0..40).map(|n| leaf(&format!("b{n}"))).collect();
    let area = ui
        .mount_tree(Element::new(
            "area",
            ScrollArea::new(Element::new("rows", Column::new(rows))),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for (code, modifiers) in [
        (KeyCode::PageDown, alt()),
        (KeyCode::End, Modifiers::PRIMARY),
    ] {
        chord_yields(&mut ui, &area, code, modifiers, "scroll area");
    }

    let mut ui = runtime();
    let source = ui.signal(ListSource::new(Rows(100)));
    let list = ui
        .mount_tree(Element::new(
            "list",
            VirtualList::new(source.read_only(), 24.0).unwrap(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    for (code, modifiers) in [
        (KeyCode::PageDown, alt()),
        (KeyCode::ArrowDown, Modifiers::PRIMARY),
    ] {
        chord_yields(&mut ui, &list, code, modifiers, "plain list");
    }
}

#[test]
fn modified_list_navigation_yields_and_documented_modifiers_remain() {
    let mut ui = runtime();
    let source = ui.signal(ListSource::new(Rows(10)));
    let selection = ListSelection::new(&ui, SelectionMode::Multiple);
    let list = ui
        .mount_tree(Element::new(
            "list",
            VirtualList::new(source.read_only(), 24.0)
                .unwrap()
                .selection(selection.clone()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    selection.set_active(Some("r0".to_owned())).unwrap();
    selection.select(["r0".to_owned()]).unwrap();
    for (code, modifiers) in [
        (KeyCode::ArrowDown, alt()),
        (KeyCode::PageDown, alt()),
        (KeyCode::End, super_key()),
    ] {
        chord_yields(&mut ui, &list, code, modifiers, "selection list");
    }
    // Shift still extends the range from the anchor.
    ui.native_modifiers(shift()).unwrap();
    send(&mut ui, KeyCode::ArrowDown, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowDown, KeyAction::Release);
    ui.native_modifiers(Modifiers::default()).unwrap();
    assert_eq!(selection.selected().get().unwrap().len(), 2);
}

/// `folder` holds `child`; `leaf` has no children.
struct Outline;
impl TreeItems for Outline {
    fn roots(&self) -> Vec<String> {
        vec!["folder".into(), "leaf".into()]
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key == "folder" {
            vec!["child".into()]
        } else {
            Vec::new()
        }
    }
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(key)))
    }
}
/// Three rows whose cells show their column key.
struct Cells;
impl TableItems for Cells {
    fn len(&self) -> usize {
        3
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 3).then(|| format!("r{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        (index < 3).then(|| Element::new("text", Label::new(column)))
    }
}

#[test]
fn modified_tree_and_table_cross_arrows_yield_to_commands() {
    let mut ui = runtime();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let state = TreeController::new(&ui, selection.clone(), TreeSource::new(Outline)).unwrap();
    let tree = ui
        .mount_tree(Element::new("tree", Tree::new(&state, 24.0).unwrap()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    selection.set_active(Some("folder".to_owned())).unwrap();
    for modifiers in [Modifiers::PRIMARY, alt()] {
        chord_yields(&mut ui, &tree, KeyCode::ArrowRight, modifiers, "tree");
    }
    assert!(!state.is_expanded("folder").unwrap());

    let mut ui = runtime();
    let selection = ListSelection::new(&ui, SelectionMode::Single);
    let controller = TableController::new(
        &ui,
        selection.clone(),
        ["a", "b"].map(|key| TableColumn::new(key, key)),
        TableSource::new(Cells),
    )
    .unwrap();
    let table = ui
        .mount_tree(Element::new(
            "table",
            Table::new(&controller, 24.0).unwrap(),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    selection.set_active(Some("r0".to_owned())).unwrap();
    let body = ui.children(&table).unwrap()[0].clone();
    for modifiers in [Modifiers::PRIMARY, super_key()] {
        chord_yields(&mut ui, &body, KeyCode::ArrowRight, modifiers, "table");
    }
    assert_eq!(controller.active_column().get().unwrap(), None);
}

#[test]
fn modified_menu_and_dropdown_navigation_yields_to_commands() {
    let mut ui = runtime();
    let bar = MenuBar::new(
        &ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![MenuItem::new("new", "New", |_| Ok(()))],
            ),
            MenuItem::submenu(
                "edit",
                "Edit",
                vec![MenuItem::new("undo", "Undo", |_| Ok(()))],
            ),
        ],
    )
    .unwrap()
    .content(leaf("document"));
    let root = ui.mount_tree(Element::new("bar", bar)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let document = ui.find_descendant(&root, "document").unwrap().unwrap();
    ui.focus(&document).unwrap();
    send(&mut ui, KeyCode::F10, KeyAction::Press);
    send(&mut ui, KeyCode::F10, KeyAction::Release);
    let title = ui.focused().expect("F10 focuses the first title");
    for (code, modifiers) in [
        (KeyCode::ArrowDown, Modifiers::PRIMARY),
        (KeyCode::ArrowRight, Modifiers::PRIMARY),
        (KeyCode::End, alt()),
    ] {
        chord_yields(&mut ui, &title, code, modifiers, "menu bar");
    }

    let mut ui = runtime();
    let value = ui.signal(1);
    let multi = ui.signal(Vec::<u32>::new());
    let root = ui
        .mount_tree(Element::new(
            "form",
            Column::new([
                Element::new(
                    "dropdown",
                    Select::new(
                        &ui,
                        value.clone(),
                        [1, 2, 3].map(|n| Choice::new(n, n.to_string())),
                    )
                    .unwrap(),
                ),
                Element::new(
                    "tags",
                    MultiSelect::new(&ui, multi, [1, 2].map(|n| Choice::new(n, n.to_string())))
                        .unwrap(),
                ),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let anchor = ui.find_descendant(&root, "select").unwrap().unwrap();
    chord_yields(
        &mut ui,
        &anchor,
        KeyCode::ArrowDown,
        alt(),
        "select trigger",
    );
    let tags = ui.find_descendant(&root, "tags").unwrap().unwrap();
    let trigger = ui.find_descendant(&tags, "anchor").unwrap().unwrap();
    chord_yields(
        &mut ui,
        &trigger,
        KeyCode::ArrowDown,
        alt(),
        "multi-select trigger",
    );
    // An open list leaves modified chords alone too.
    ui.focus(&anchor).unwrap();
    send(&mut ui, KeyCode::ArrowDown, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowDown, KeyAction::Release);
    let list = ui.focused().expect("the open list has focus");
    for (code, modifiers) in [
        (KeyCode::End, Modifiers::PRIMARY),
        (KeyCode::ArrowDown, alt()),
    ] {
        chord_yields(&mut ui, &list, code, modifiers, "option list");
    }
    assert_eq!(value.get().unwrap(), 1);
}

#[test]
fn modified_keys_in_readout_text_mode_yield_to_commands() {
    let mut ui = runtime();
    let position = ui.signal(String::from("001:01:000"));
    let readout = ui
        .mount_tree(Element::new(
            "field",
            InlineReadout::new(position.read_only()),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&readout).unwrap();
    send(&mut ui, KeyCode::F2, KeyAction::Press);
    send(&mut ui, KeyCode::F2, KeyAction::Release);
    let focused = ui.focused().unwrap();
    for (code, modifiers) in [(KeyCode::PageDown, alt()), (KeyCode::ArrowUp, alt())] {
        chord_yields(&mut ui, &focused, code, modifiers, "readout text mode");
    }
}

#[test]
fn a_floating_frame_moves_only_on_alt_and_alt_shift_arrows() {
    let mut ui = runtime();
    let window = FloatingWindow::new(&ui, "one", "One", Element::new("body", Label::new("Body")))
        .position(Point::new(20.0, 20.0).unwrap())
        .unwrap()
        .initial_size(Size::from([200, 120]))
        .unwrap();
    let root = ui
        .mount_tree(Element::new("layer", WindowLayer::new([window])))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let frame = ui.find_descendant(&root, "one").unwrap().unwrap();
    for extra in [Modifiers::PRIMARY, super_key()] {
        let chord = Modifiers { alt: true, ..extra };
        chord_yields(&mut ui, &frame, KeyCode::ArrowLeft, chord, "floating frame");
    }
    let before = ui.bounds(&frame).unwrap();
    ui.native_modifiers(alt()).unwrap();
    send(&mut ui, KeyCode::ArrowLeft, KeyAction::Press);
    send(&mut ui, KeyCode::ArrowLeft, KeyAction::Release);
    ui.native_modifiers(Modifiers::default()).unwrap();
    ui.flush(ui.now()).unwrap();
    assert_ne!(ui.bounds(&frame).unwrap(), before, "Alt+Left still moves");
}

#[test]
fn a_wheel_step_at_an_endpoint_records_no_edit() {
    let mut ui = runtime();
    let (parameter, host) = ui
        .parameter(
            ParameterId::new(1, 1, "amount").unwrap(),
            ParameterSpec::new("Amount", 0.0, 100.0, 100.0).unwrap(),
        )
        .unwrap();
    let outcomes = Rc::new(Cell::new(0));
    let counted = outcomes.clone();
    let _monitor = host
        .on_outcome(move |_| {
            counted.set(counted.get() + 1);
            Ok(())
        })
        .unwrap();
    let root = ui
        .mount_tree(Element::new("control", Slider::new(parameter.clone())))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.focus(&root).unwrap();
    let [x, y, w, h] = ui.bounds(&root).unwrap().unwrap().values();
    let before = parameter.snapshot().get().unwrap().revision;
    let outcome = ui
        .pointer_input(PointerEvent::new(
            Point::new(x + w / 2.0, y + h / 2.0).unwrap(),
            PointerKind::Scroll(Point::new(0.0, -1.0).unwrap()),
        ))
        .unwrap();
    ui.flush(ui.now()).unwrap();
    assert_eq!(
        outcome,
        InputOutcome::Handled,
        "the wheel stays the slider's"
    );
    assert_eq!(parameter.snapshot().get().unwrap().revision, before);
    assert_eq!(outcomes.get(), 0, "the host sees no edit");
}

#[test]
fn alt_reordering_takes_alt_alone() {
    let chords = [
        Modifiers {
            alt: true,
            control: true,
            ..Modifiers::default()
        },
        Modifiers {
            alt: true,
            super_key: true,
            ..Modifiers::default()
        },
    ];
    // A reorderable list: a chord adding Control or Super reaches commands.
    for chord in chords {
        let mut ui = runtime();
        let moved = Rc::new(Cell::new(0));
        let count = moved.clone();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let source = ui.signal(ListSource::new(Rows(3)));
        let list = VirtualList::new(source.read_only(), 24.0)
            .unwrap()
            .selection(selection.clone())
            .reorderable(move |_, _| {
                count.set(count.get() + 1);
                Ok(true)
            });
        let root = ui.mount_tree(Element::new("list", list)).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        selection.set_active(Some("r0".to_owned())).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        chord_yields(&mut ui, &root, KeyCode::ArrowDown, chord, "list reorder");
        assert_eq!(moved.get(), 0, "{chord:?}");
    }
    // A reorderable tree and a table with movable columns likewise.
    for chord in chords {
        let mut ui = runtime();
        let moved = Rc::new(Cell::new(0));
        let count = moved.clone();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let state = TreeController::new(&ui, selection.clone(), TreeSource::new(Outline)).unwrap();
        let tree = ui
            .mount_tree(Element::new(
                "tree",
                Tree::new(&state, 24.0).unwrap().reorderable(move |_, _| {
                    count.set(count.get() + 1);
                    Ok(true)
                }),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        selection.set_active(Some("leaf".to_owned())).unwrap();
        chord_yields(&mut ui, &tree, KeyCode::ArrowUp, chord, "tree reorder");
        assert_eq!(moved.get(), 0, "{chord:?}");

        let mut ui = runtime();
        let selection = ListSelection::new(&ui, SelectionMode::Single);
        let controller = TableController::new(
            &ui,
            selection.clone(),
            ["a", "b"].map(|key| TableColumn::new(key, key).movable(true)),
            TableSource::new(Cells),
        )
        .unwrap();
        let table = ui
            .mount_tree(Element::new(
                "table",
                Table::new(&controller, 24.0).unwrap(),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        selection.set_active(Some("r0".to_owned())).unwrap();
        let body = ui.children(&table).unwrap()[0].clone();
        chord_yields(&mut ui, &body, KeyCode::ArrowRight, chord, "table reorder");
        let order: Vec<String> = controller
            .columns()
            .iter()
            .map(|column| column.key().to_owned())
            .collect();
        assert_eq!(order, ["a", "b"], "{chord:?}");
    }
}
