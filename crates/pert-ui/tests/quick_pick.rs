//! `QuickPick` contracts: item validation, kinds labelled in rows, the three
//! answers a choose callback can give (done, refused, pending), pending
//! results that resolve or are superseded, disabled items still chosen, the
//! callback replaced on re-description and the reported names.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc};
use support::{key, settle};

type Answer = Rc<RefCell<Box<dyn FnMut(&PickItem) -> PickChoice>>>;

struct Setup {
    ui: Runtime,
    root: WidgetId,
    picker: QuickPickController,
    chosen: Rc<RefCell<Vec<String>>>,
    answer: Answer,
}

/// An application with a focused button beside a picker that records every
/// chosen key and answers with the replaceable `answer`.
fn setup(build: impl FnOnce(QuickPick) -> QuickPick) -> Setup {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let picker = QuickPickController::new(&ui);
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let answer: Answer = Rc::new(RefCell::new(Box::new(|_| PickChoice::Done)));
    let (record, reply) = (chosen.clone(), answer.clone());
    let quick = QuickPick::new(
        picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        move |_, item| {
            record.borrow_mut().push(item.key.clone());
            Ok((reply.borrow_mut())(item))
        },
    );
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([Element::new("picker", build(quick))]),
        ))
        .unwrap();
    settle(&mut ui);
    Setup {
        ui,
        root,
        picker,
        chosen,
        answer,
    }
}

fn items() -> Vec<PickItem> {
    vec![
        PickItem::new("page/colors", "Colors").kind("Page"),
        PickItem::new("page/knobs", "Knobs")
            .detail("Controls")
            .kind("Page"),
        PickItem::new("recent/knobs", "Knobs")
            .detail("2 minutes ago")
            .kind("History"),
        PickItem::new("page/old", "Old page").disabled(true),
    ]
}

impl Setup {
    fn open(&mut self) {
        self.picker.open(items()).unwrap();
        settle(&mut self.ui);
    }
    fn semantics(&self, key: &str) -> Semantics {
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == key && c.visible)
            .and_then(|c| c.details.semantics)
            .expect("component semantics")
    }
    fn rows(&self) -> Vec<String> {
        let snapshot = self.ui.inspect();
        snapshot
            .components
            .iter()
            .find(|c| c.key == "content" && c.details.properties.contains_key("results"))
            .and_then(|c| c.details.semantics.clone())
            .expect("picker content")
            .items
            .iter()
            .map(|item| item.name.clone().unwrap_or_default())
            .collect()
    }
    fn message(&self) -> Option<String> {
        self.ui
            .inspect()
            .components
            .iter()
            .find(|c| c.key == "message" && c.visible)
            .and_then(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
    }
    fn answer(&self, answer: impl FnMut(&PickItem) -> PickChoice + 'static) {
        *self.answer.borrow_mut() = Box::new(answer);
    }
}

#[test]
fn blank_and_duplicate_keys_are_refused_without_opening() {
    let s = setup(|p| p);
    let duplicate = vec![PickItem::new("a", "One"), PickItem::new("a", "Two")];
    assert!(matches!(
        s.picker.open(duplicate),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    ));
    assert!(matches!(
        s.picker.open(vec![PickItem::new(" ", "Blank")]),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    ));
    assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
}

#[test]
fn rows_show_detail_and_kind_and_rank_by_label() {
    let mut s = setup(|p| p.name("Go to").list_name("Places"));
    s.open();
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.semantics("picker").name.as_deref(), Some("Go to"));
    assert_eq!(s.rows(), ["Colors", "Knobs", "Knobs", "Old page"]);
    // The kind follows the detail at the end of the row and in its description.
    let row = s.semantics("recent/knobs");
    assert_eq!(row.description.as_deref(), Some("History"));
    let detail =
        s.ui.inspect()
            .components
            .into_iter()
            .filter(|c| c.key == "detail" && c.visible)
            .filter_map(|c| match c.details.properties.get("text") {
                Some(InspectValue::Text(text)) => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
    assert!(detail.contains(&"2 minutes ago · History".to_owned()));
    assert!(detail.contains(&"Controls · Page".to_owned()));
    s.ui.text_input("kno").unwrap();
    settle(&mut s.ui);
    assert_eq!(s.rows(), ["Knobs", "Knobs"]);
}

#[test]
fn a_long_detail_yields_to_the_label() {
    let mut s = setup(|p| p);
    let location = "Layout › Navigation › ".repeat(20);
    s.picker
        .open(vec![
            PickItem::new("action", "Action 7")
                .detail(location)
                .kind("Content"),
        ])
        .unwrap();
    settle(&mut s.ui);
    let width = |ui: &Runtime, key: &str| {
        let component = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == key && c.visible)
            .expect("row part");
        component.details.box_model.border_box.expect("arranged")[2]
    };
    let (row, name, detail) = (
        width(&s.ui, "action"),
        width(&s.ui, "name"),
        width(&s.ui, "detail"),
    );
    // The label keeps the width it needs and the detail is cut to what is
    // left, instead of pushing the label out.
    assert!(name > 30.0, "the label is squeezed to {name}");
    assert!(
        detail > 0.0 && name + detail < row,
        "label {name}, detail {detail} of {row}"
    );
    // A label longer than the row leaves the detail its third.
    s.picker
        .open(vec![
            PickItem::new("long", "Long label ".repeat(40)).detail("Layout › Navigation"),
        ])
        .unwrap();
    settle(&mut s.ui);
    let (row, detail) = (width(&s.ui, "long"), width(&s.ui, "detail"));
    assert!(
        detail > 0.0 && detail <= row / 3.0 + 0.5,
        "detail {detail} of {row}"
    );
}

#[test]
fn done_closes_refused_stays_open_and_disabled_items_are_still_offered() {
    let mut s = setup(|p| p);
    s.open();
    s.ui.text_input("old").unwrap();
    settle(&mut s.ui);
    s.answer(|_| PickChoice::Refused("That page was removed.".to_owned()));
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    // The disabled item reached the callback, and the refusal is shown.
    assert_eq!(*s.chosen.borrow(), ["page/old"]);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.message().as_deref(), Some("That page was removed."));
    s.answer(|_| PickChoice::Done);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
}

#[test]
fn a_pending_choice_closes_then_reopens_with_its_failure_or_stays_closed() {
    let mut s = setup(|p| p);
    let result = s.ui.signal(None::<Result<(), String>>);
    let binding = result.read_only();
    s.answer(move |_| {
        PickChoice::Pending(PickPending::new(binding.clone(), |value| value.clone()))
    });
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Pending);
    assert_eq!(*s.chosen.borrow(), ["page/colors"]);
    result
        .set(Some(Err("Colors could not be shown.".into())))
        .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.message().as_deref(), Some("Colors could not be shown."));
    // A success closes it.
    result.set(None).unwrap();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Pending);
    result.set(Some(Ok(()))).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
}

#[test]
fn a_pending_choice_from_an_earlier_session_no_longer_reopens_it() {
    let mut s = setup(|p| p);
    let result = s.ui.signal(None::<Result<(), String>>);
    let binding = result.read_only();
    s.answer(move |_| {
        PickChoice::Pending(PickPending::new(binding.clone(), |value| value.clone()))
    });
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    // Reopening replaces the session; the old result then fails.
    s.open();
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    result.set(Some(Err("Too late.".into()))).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.message(), None);
}

#[test]
fn a_pending_choice_of_a_removed_picker_does_not_reopen_in_its_successor() {
    let mut s = setup(|p| p);
    let result = s.ui.signal(None::<Result<(), String>>);
    let binding = result.read_only();
    s.answer(move |_| {
        PickChoice::Pending(PickPending::new(binding.clone(), |value| value.clone()))
    });
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Pending);
    // The picker is removed while its surface is hidden for the pending
    // result, and another picker takes the controller.
    s.ui.reconcile_children(&s.root, vec![]).unwrap();
    settle(&mut s.ui);
    let successor = QuickPick::new(
        s.picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        |_, _| Ok(PickChoice::Done),
    );
    s.ui.reconcile_children(&s.root, vec![Element::new("successor", successor)])
        .unwrap();
    settle(&mut s.ui);
    result.set(Some(Err("Too late.".into()))).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
    assert_eq!(s.message(), None);
}

#[test]
fn a_choice_does_not_close_a_session_its_callback_opened() {
    let mut s = setup(|p| p);
    let picker = s.picker.clone();
    s.answer(move |_| {
        picker
            .open(vec![PickItem::new("next", "Next step")])
            .unwrap();
        PickChoice::Done
    });
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.rows(), ["Next step"]);
}

#[test]
fn describing_the_picker_again_replaces_its_choose_callback() {
    let mut s = setup(|p| p);
    let second = Rc::new(RefCell::new(Vec::new()));
    let record = second.clone();
    let quick = QuickPick::new(
        s.picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        move |_, item| {
            record.borrow_mut().push(item.key.clone());
            Ok(PickChoice::Done)
        },
    );
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", quick)])
        .unwrap();
    settle(&mut s.ui);
    s.open();
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert!(s.chosen.borrow().is_empty());
    assert_eq!(*second.borrow(), ["page/colors"]);
}

#[test]
fn a_callback_error_leaves_the_picker_open() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let picker = QuickPickController::new(&ui);
    ui.mount_tree(Element::new(
        "picker",
        QuickPick::new(
            picker.clone(),
            Element::new("doc", Label::new("Document")),
            |_, _| Err(RuntimeError::Exhausted),
        ),
    ))
    .unwrap();
    settle(&mut ui);
    picker.open(items()).unwrap();
    settle(&mut ui);
    assert!(matches!(
        ui.key_input(KeyCode::Enter, KeyAction::Press),
        Err(RuntimeError::Exhausted)
    ));
    ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(picker.status().unwrap(), PickStatus::Open);
}

#[test]
fn native_focus_loss_closes_modal_and_nonmodal_picker() {
    for nonmodal in [false, true] {
        let mut s = setup(|picker| if nonmodal { picker.nonmodal() } else { picker });
        s.picker.open(items()).unwrap();
        settle(&mut s.ui);
        s.ui.cancel_input().unwrap();
        settle(&mut s.ui);
        assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
        s.ui.clear_focus();
        assert_eq!(s.ui.focused(), None);
        settle(&mut s.ui);
        assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
    }
}

#[test]
fn highlighted_picker_labels_use_the_foreground_paired_with_the_fill() {
    let mut s = setup(|p| p);
    let tab_text = Color::rgba(0.1, 0.9, 0.2, 1.0).unwrap();
    let surface_text = Color::rgba(0.8, 0.1, 0.7, 1.0).unwrap();
    s.ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::OnTab, tab_text)
        .unwrap();
    s.ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::OnSurface, surface_text)
        .unwrap();
    s.open();
    let identities = ["page/colors", "page/knobs"].map(|key| {
        let row = support::widget(&s.ui, &s.root, key);
        (
            row.clone(),
            support::widget(&s.ui, &row, "name"),
            support::widget(&s.ui, &row, "detail"),
        )
    });
    let check = |s: &mut Setup, key: &str, child: &str, expected: Color| {
        let row = support::widget(&s.ui, &s.root, key);
        let name = support::widget(&s.ui, &row, child);
        let bounds = s.ui.bounds(&name).unwrap().unwrap();
        let scene = s.ui.paint().unwrap();
        let colors = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::DrawMask { rect, color, .. }
                    if bounds.contains(
                        Point::new(
                            rect.x() + rect.width() / 2.0,
                            rect.y() + rect.height() / 2.0,
                        )
                        .unwrap(),
                    ) =>
                {
                    Some(*color)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(!colors.is_empty());
        assert!(
            colors.iter().all(|color| *color == expected),
            "{key}: {colors:?}"
        );
    };
    check(&mut s, "page/colors", "name", tab_text);
    check(&mut s, "page/colors", "detail", tab_text);
    check(&mut s, "page/knobs", "name", surface_text);
    key(&mut s.ui, KeyCode::ArrowDown);
    settle(&mut s.ui);
    check(&mut s, "page/colors", "name", surface_text);
    check(&mut s, "page/knobs", "name", tab_text);
    check(&mut s, "page/knobs", "detail", tab_text);
    key(&mut s.ui, KeyCode::ArrowUp);
    settle(&mut s.ui);
    check(&mut s, "page/colors", "name", tab_text);
    check(&mut s, "page/colors", "detail", tab_text);
    for (row, name, detail) in identities {
        assert_eq!(
            support::widget(&s.ui, &s.root, s.ui.key(&row).unwrap()),
            row
        );
        assert_eq!(support::widget(&s.ui, &row, "name"), name);
        assert_eq!(support::widget(&s.ui, &row, "detail"), detail);
    }
    assert_eq!(s.ui.flush(s.ui.now()).unwrap().components_updated, 0);
}

#[test]
fn highlighted_rows_survive_scroll_reconciliation() {
    let mut s = setup(|p| p);
    s.picker
        .open(
            (0..40)
                .map(|index| PickItem::new(format!("item-{index}"), format!("Item {index}")))
                .collect(),
        )
        .unwrap();
    settle(&mut s.ui);
    let row = support::widget(&s.ui, &s.root, "item-0");
    let label = support::widget(&s.ui, &row, "name");
    let bounds = s.ui.bounds(&row).unwrap().unwrap();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(bounds.x() + 20.0, bounds.y() + bounds.height() / 2.0).unwrap(),
        PointerKind::Scroll(Point::new(0.0, 40.0).unwrap()),
    ))
    .unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    settle(&mut s.ui);
    assert!(s.ui.bounds(&row).unwrap().unwrap().y() < bounds.y());
    assert_eq!(support::widget(&s.ui, &s.root, "item-0"), row);
    assert_eq!(support::widget(&s.ui, &row, "name"), label);
}

#[test]
fn highlighted_rows_accept_query_and_new_session_content() {
    let mut s = setup(|p| p);
    s.open();
    s.ui.text_input("col").unwrap();
    s.ui.flush(s.ui.now()).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.rows(), ["Colors"]);
    s.picker
        .open(vec![
            PickItem::new("page/knobs", "Another"),
            PickItem::new("page/colors", "Updated").detail("New detail"),
        ])
        .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.rows(), ["Another", "Updated"]);
    let snapshot = s.ui.inspect();
    assert!(snapshot.components.iter().any(|c| c.key == "name"
        && c.visible
        && c.details.properties.get("text") == Some(&InspectValue::Text("Updated".into()))));
    assert!(snapshot.components.iter().any(|c| c.key == "detail"
        && c.visible
        && c.details.properties.get("text") == Some(&InspectValue::Text("New detail".into()))));
    key(&mut s.ui, KeyCode::ArrowDown);
    settle(&mut s.ui);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(*s.chosen.borrow(), ["page/colors"]);
}

fn next_picker(s: &Setup) -> QuickPick {
    QuickPick::new(
        s.picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        |_, _| Ok(PickChoice::Done),
    )
    .name("New picker")
    .placeholder("New search")
    .list_name("New results")
    .empty_text("New empty")
    .matcher(PickMatcher::new(|_, item| {
        Some(if item.key == "page/knobs" { 100 } else { 0 })
    }))
    .width(420.0)
    .unwrap()
    .nonmodal()
}
fn panel_width(s: &Setup) -> f32 {
    let frame = support::widget(&s.ui, &s.root, "frame");
    s.ui.bounds(&frame).unwrap().unwrap().width()
}
#[test]
fn picker_description_configuration_waits_for_the_next_session() {
    let mut s = setup(|p| {
        p.name("Old picker")
            .placeholder("Old search")
            .list_name("Old results")
            .empty_text("Old empty")
            .width(280.0)
            .unwrap()
    });
    s.open();
    let before = panel_width(&s);
    let next = next_picker(&s);
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", next)])
        .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.semantics("picker").name.as_deref(), Some("Old picker"));
    assert_eq!(s.semantics("picker").role, SemanticRole::Dialog);
    assert_eq!(s.semantics("content").name.as_deref(), Some("Old results"));
    assert_eq!(panel_width(&s), before);
    assert!(
        s.ui.inspect()
            .components
            .iter()
            .any(|c| c.details.properties.get("placeholder")
                == Some(&InspectValue::Text("Old search".into())))
    );
    s.ui.text_input("zz").unwrap();
    settle(&mut s.ui);
    assert!(s.rows().is_empty());
    assert!(
        s.ui.inspect()
            .components
            .iter()
            .any(|c| c.details.properties.get("text")
                == Some(&InspectValue::Text("Old empty".into())))
    );
    s.open();
    assert_eq!(s.semantics("picker").name.as_deref(), Some("New picker"));
    assert_eq!(s.semantics("picker").role, SemanticRole::Popover);
    assert_eq!(s.semantics("content").name.as_deref(), Some("New results"));
    assert!(panel_width(&s) > before);
    assert_eq!(s.rows().first().map(String::as_str), Some("Knobs"));
    assert!(
        s.ui.inspect()
            .components
            .iter()
            .any(|c| c.details.properties.get("placeholder")
                == Some(&InspectValue::Text("New search".into())))
    );
}
#[test]
fn failed_pending_result_reopens_the_same_configuration_snapshot() {
    let mut s = setup(|p| p.name("Old picker").width(280.0).unwrap());
    let result = s.ui.signal(None::<Result<(), String>>);
    let binding = result.read_only();
    s.answer(move |_| {
        PickChoice::Pending(PickPending::new(binding.clone(), |value| value.clone()))
    });
    s.open();
    let before = panel_width(&s);
    key(&mut s.ui, KeyCode::Enter);
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Pending);
    let next = next_picker(&s);
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", next)])
        .unwrap();
    settle(&mut s.ui);
    result.set(Some(Err("Retry".into()))).unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Open);
    assert_eq!(s.semantics("picker").name.as_deref(), Some("Old picker"));
    assert_eq!(s.semantics("picker").role, SemanticRole::Dialog);
    assert_eq!(panel_width(&s), before);
    s.open();
    assert_eq!(s.semantics("picker").name.as_deref(), Some("New picker"));
    assert_eq!(s.semantics("picker").role, SemanticRole::Popover);
}
#[test]
fn equivalent_picker_description_does_not_invalidate_layout() {
    let mut s = setup(|p| p);
    s.open();
    s.ui.paint().unwrap();
    s.ui.set_profiling(true).unwrap();
    s.ui.clear_profile().unwrap();
    let next = QuickPick::new(
        s.picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        |_, _| Ok(PickChoice::Done),
    );
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", next)])
        .unwrap();
    settle(&mut s.ui);
    assert!(s.ui.profile().stats.iter().all(|stat| !matches!(
        stat.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
}

#[test]
fn latest_picker_description_can_reset_pending_configuration_to_defaults() {
    let mut s = setup(|p| p.name("Old picker").width(280.0).unwrap());
    s.open();
    let next = next_picker(&s);
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", next)])
        .unwrap();
    let reset = QuickPick::new(
        s.picker.clone(),
        Element::new("doc", Button::new("Edit", |_| Ok(()))),
        |_, _| Ok(PickChoice::Done),
    );
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", reset)])
        .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.semantics("picker").name.as_deref(), Some("Old picker"));
    s.open();
    assert_eq!(s.semantics("picker").name.as_deref(), Some("Quick pick"));
    assert_eq!(s.semantics("picker").role, SemanticRole::Dialog);
    assert_eq!(s.rows().first().map(String::as_str), Some("Colors"));
    assert_eq!(panel_width(&s), QuickPick::DEFAULT_WIDTH);
}

#[test]
fn picker_mount_adopts_an_already_open_controller_and_keeps_role_localization_live() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let picker = QuickPickController::new(&ui);
    picker.open(items()).unwrap();
    ui.mount_tree(Element::new(
        "picker",
        QuickPick::new(
            picker.clone(),
            Element::new("doc", Label::new("Document")),
            |_, _| Ok(PickChoice::Done),
        ),
    ))
    .unwrap();
    settle(&mut ui);
    assert_eq!(picker.status().unwrap(), PickStatus::Open);
    assert_eq!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "picker")
            .unwrap()
            .details
            .semantics
            .as_ref()
            .unwrap()
            .name
            .as_deref(),
        Some("Quick pick")
    );
    ui.ui_settings()
        .set_strings(
            UiStrings::english()
                .with(StringRole::PickerName, "Translated picker")
                .unwrap()
                .with(StringRole::SearchPlaceholder, "Translated search")
                .unwrap(),
        )
        .unwrap();
    settle(&mut ui);
    assert_eq!(
        ui.inspect()
            .components
            .iter()
            .find(|c| c.key == "picker")
            .unwrap()
            .details
            .semantics
            .as_ref()
            .unwrap()
            .name
            .as_deref(),
        Some("Translated picker")
    );
    assert!(
        ui.inspect()
            .components
            .iter()
            .any(|c| c.details.properties.get("placeholder")
                == Some(&InspectValue::Text("Translated search".into())))
    );
}

#[test]
fn reopening_modal_picker_as_nonmodal_updates_dismissal_and_restores_original_opener() {
    let mut s = setup(|p| p);
    let opener = support::widget(&s.ui, &s.root, "doc");
    s.ui.focus(&opener).unwrap();
    settle(&mut s.ui);
    s.open();
    let next = next_picker(&s);
    s.ui.reconcile_children(&s.root, vec![Element::new("picker", next)])
        .unwrap();
    s.open();
    s.ui.pointer_input(PointerEvent::new(
        Point::new(790.0, 590.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    settle(&mut s.ui);
    assert_eq!(s.picker.status().unwrap(), PickStatus::Closed);
    assert_eq!(s.ui.focused(), Some(opener));
}
#[test]
fn reopening_nonmodal_picker_as_modal_runs_conflict_admission() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let picker = QuickPickController::new(&ui);
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "app",
            Column::new([
                Element::new(
                    "picker",
                    QuickPick::new(
                        picker.clone(),
                        Element::new("doc", Label::new("Document")),
                        |_, _| Ok(PickChoice::Done),
                    )
                    .nonmodal(),
                ),
                Element::new(
                    "dialog",
                    AlertDialog::new(
                        dialog.clone(),
                        Element::new("open", Button::new("Open", |_| Ok(()))),
                        "Confirm",
                        "Proceed?",
                        "Yes",
                    ),
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    picker.open(items()).unwrap();
    settle(&mut ui);
    let next = QuickPick::new(
        picker.clone(),
        Element::new("doc", Label::new("Document")),
        |_, _| Ok(PickChoice::Done),
    );
    let next_dialog = AlertDialog::new(
        dialog.clone(),
        Element::new("open", Button::new("Open", |_| Ok(()))),
        "Confirm",
        "Proceed?",
        "Yes",
    );
    ui.reconcile_children(
        &root,
        vec![
            Element::new("picker", next),
            Element::new("dialog", next_dialog),
        ],
    )
    .unwrap();
    picker.open(items()).unwrap();
    dialog.open().unwrap();
    assert_eq!(ui.flush(ui.now()), Err(RuntimeError::ModalConflict));
    assert_eq!(picker.status().unwrap(), PickStatus::Open);
    let outside = support::widget(&ui, &root, "open");
    assert_eq!(ui.focus(&outside), Err(RuntimeError::ModalBlocked));
    assert_eq!(
        ui.event(&outside, |_| panic!("blocked")),
        Err(RuntimeError::ModalBlocked)
    );
    settle(&mut ui);
}
