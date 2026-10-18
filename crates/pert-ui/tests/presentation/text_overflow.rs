//! Single-line truncation with an ellipsis, its help and highlighted ranges.
use pert_ui::*;
use std::time::Duration;

const PATH: &str = "/home/user/projects/mix/stems/final-master.wav";

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap()
}
fn property(ui: &Runtime, key: &str, name: &str) -> Option<InspectValue> {
    component(ui, key).details.properties.get(name).cloned()
}
fn shown(ui: &Runtime, key: &str) -> Option<String> {
    match property(ui, key, "shown") {
        Some(InspectValue::Text(text)) => Some(text),
        _ => None,
    }
}
fn width(ui: &Runtime, key: &str) -> f32 {
    component(ui, key).details.box_model.border_box.unwrap()[2]
}
fn mount(ui: &mut Runtime, label: Label) -> WidgetId {
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("label", label)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    root
}

#[test]
fn a_truncated_label_keeps_its_full_name_and_fits_its_allocation() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    mount(&mut ui, Label::new(PATH).overflow(TextOverflow::Ellipsis));
    let end = shown(&ui, "label").expect("the path is wider than the window");
    assert!(end.ends_with('\u{2026}') && PATH.starts_with(end.trim_end_matches('\u{2026}')));
    assert_eq!(
        property(&ui, "label", "truncated"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(
        property(&ui, "label", "text"),
        Some(InspectValue::Text(PATH.into()))
    );
    let semantics = component(&ui, "label").details.semantics.unwrap();
    assert_eq!(semantics.name.as_deref(), Some(PATH));
    assert!(width(&ui, "label") <= 140.0);

    // The middle variant keeps both ends.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    mount(
        &mut ui,
        Label::new(PATH).overflow(TextOverflow::MiddleEllipsis),
    );
    let middle = shown(&ui, "label").unwrap();
    let (head, tail) = middle.split_once('\u{2026}').unwrap();
    assert!(PATH.starts_with(head) && PATH.ends_with(tail) && !tail.is_empty());

    // Text that fits is untouched and reports nothing shown.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 80.0, 1.0).unwrap())
        .unwrap();
    mount(
        &mut ui,
        Label::new("Short").overflow(TextOverflow::Ellipsis),
    );
    assert_eq!(
        property(&ui, "label", "truncated"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(shown(&ui, "label"), None);
}

#[test]
fn truncation_follows_width_scale_zoom_and_text_but_not_the_theme() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(160.0, 80.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(PATH.to_owned());
    mount(
        &mut ui,
        Label::new(text.read_only()).overflow(TextOverflow::Ellipsis),
    );
    let at_one = shown(&ui, "label").unwrap();
    // A wider window keeps more of the text.
    ui.set_viewport(Viewport::new(260.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let wider = shown(&ui, "label").unwrap();
    assert!(wider.len() > at_one.len());
    // A physical scale keeps the logical fit.
    ui.set_viewport(Viewport::new(160.0, 80.0, 2.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(shown(&ui, "label").is_some());
    assert!(width(&ui, "label") <= 160.0);
    // UI zoom halves the logical window, so less text fits.
    ui.set_viewport(Viewport::new(320.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let zoomed = shown(&ui, "label").unwrap();
    assert!(zoomed.len() <= at_one.len());
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.0).unwrap())
        .unwrap();
    ui.set_viewport(Viewport::new(160.0, 80.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(shown(&ui, "label").as_deref(), Some(at_one.as_str()));
    // A theme change repaints with the same cut.
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(shown(&ui, "label").as_deref(), Some(at_one.as_str()));
    // New text is cut again, and short text is no longer cut.
    text.set("Short".into()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(shown(&ui, "label"), None);
}

#[test]
fn truncation_never_splits_a_grapheme_cluster() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(90.0, 80.0, 1.0).unwrap())
        .unwrap();
    // Each "e" carries a combining accent; a cut between them would show a
    // bare letter followed by a stray mark.
    let text = "e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}e\u{301}";
    mount(&mut ui, Label::new(text).overflow(TextOverflow::Ellipsis));
    let cut = shown(&ui, "label").unwrap();
    let head = cut.trim_end_matches('\u{2026}');
    assert!(!head.is_empty() && head.ends_with('\u{301}'));
    assert_eq!(head.chars().count() % 2, 0);
}

#[test]
fn help_shows_the_full_text_only_for_a_truncated_label() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    let root = mount(&mut ui, Label::new(PATH).overflow(TextOverflow::Ellipsis));
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 8.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(500)));
    ui.flush(Duration::from_millis(500)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
    assert!(ui.find_descendant(&root, "help").unwrap().is_some());
    assert!(component(&ui, "help").visible);
    let texts: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.key == "text")
        .filter_map(|c| c.details.properties.get("text").cloned())
        .collect();
    assert!(texts.contains(&InspectValue::Text(PATH.into())));
    assert!(ui.focused().is_none());
    // Leaving closes it.
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 70.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::from_millis(600)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );

    // A label that fits offers no help and schedules nothing.
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 80.0, 1.0).unwrap())
        .unwrap();
    mount(
        &mut ui,
        Label::new("Short").overflow(TextOverflow::Ellipsis),
    );
    ui.pointer_input(PointerEvent::new(
        Point::new(5.0, 8.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.next_deadline(), None);
    ui.flush(Duration::from_millis(600)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );
}

#[test]
fn help_follows_a_bound_label_starting_or_stopping_to_be_cut_while_hovered() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal(PATH.to_owned());
    mount(
        &mut ui,
        Label::new(text.read_only()).overflow(TextOverflow::Ellipsis),
    );
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 8.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.flush(Duration::from_millis(500)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
    // Short text fits: the open help closes in the same flush.
    text.set("Short".to_owned()).unwrap();
    ui.flush(Duration::from_millis(510)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(ui.next_deadline(), None);
    // Long text again: the delay starts at once, without another event.
    text.set(PATH.to_owned()).unwrap();
    ui.flush(Duration::from_millis(520)).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(1020)));
}

#[test]
fn overflow_applies_on_re_description_and_rich_content_is_clipped() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    let before = Element::new("label", Label::new(PATH));
    let after = Element::new("label", Label::new(PATH).overflow(TextOverflow::Ellipsis));
    let fresh = Element::new("label", Label::new(PATH).overflow(TextOverflow::Ellipsis));
    check_redescription(&mut ui, before, after, fresh).unwrap();

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(140.0, 80.0, 1.0).unwrap())
        .unwrap();
    mount(
        &mut ui,
        Label::new(RichText::new().icon(Icon::Folder).gap().text(PATH))
            .overflow(TextOverflow::Ellipsis),
    );
    assert_eq!(
        property(&ui, "label", "truncated"),
        Some(InspectValue::Bool(true))
    );
    assert!(width(&ui, "label") <= 140.0);
}

#[test]
fn a_property_grid_can_truncate_its_labels() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(260.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal(String::new());
    ui.mount_tree(Element::new(
        "grid",
        PropertyGrid::new([
            Property::new(
                "long",
                "An unusually long property label that cannot fit",
                Element::new("long-editor", TextField::new(value.clone())),
            ),
            Property::new(
                "short",
                "Gain",
                Element::new("short-editor", TextField::new(value)),
            ),
        ])
        .label_width(Length::new(90.0).unwrap())
        .label_overflow(TextOverflow::Ellipsis),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        property(&ui, "grid", "truncated_rows"),
        Some(InspectValue::List(vec![InspectValue::Text("long".into())]))
    );
    // The editor is still named by the whole label.
    let editor = component(&ui, "long-editor").details.semantics.unwrap();
    assert_eq!(
        editor.name.as_deref(),
        Some("An unusually long property label that cannot fit")
    );
}

#[test]
fn highlighted_text_paints_through_the_highlight_role() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 80.0, 1.0).unwrap())
        .unwrap();
    let content = RichText::highlighted("Reverb send", std::iter::once(0..3)).unwrap();
    mount(&mut ui, Label::new(content));
    assert_eq!(
        property(&ui, "label", "text"),
        Some(InspectValue::Text("Reverb send".into()))
    );
    assert!(matches!(
        RichText::highlighted("abc", std::iter::once(1..9)),
        Err(HighlightError::OutOfBounds { .. })
    ));
    ui.paint().unwrap();
}
