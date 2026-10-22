//! Display text beyond the text limits: long lines are cut before they are
//! fitted, and a label refuses what it cannot prepare without failing the
//! flush, as do the components built on it.
use pert_ui::*;
use std::time::Duration;

const LIMIT: usize = TextEditError::MAX_TEXT_BYTES;
const ELLIPSIS: char = '\u{2026}';

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}
fn property(ui: &Runtime, key: &str, name: &str) -> Option<InspectValue> {
    component(ui, key).details.properties.get(name).cloned()
}
fn text(ui: &Runtime, key: &str, name: &str) -> Option<String> {
    match property(ui, key, name) {
        Some(InspectValue::Text(text)) => Some(text),
        _ => None,
    }
}
fn invalid(ui: &Runtime, key: &str) -> bool {
    component(ui, key)
        .details
        .semantics
        .is_some_and(|semantics| semantics.invalid)
}
fn size(ui: &Runtime, key: &str) -> [f32; 2] {
    let [_, _, width, height] = component(ui, key).details.box_model.border_box.unwrap();
    [width, height]
}
fn runtime(width: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}

/// A long log line: words, so it can be cut anywhere.
fn long_line() -> String {
    let mut line = String::from("start ");
    while line.len() < 2 * LIMIT {
        line.push_str("lorem ipsum ");
    }
    line.push_str(" end");
    line
}

struct Lines(String);
impl TableItems for Lines {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "only".to_owned())
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        (index == 0).then(|| {
            Label::new(self.0.clone())
                .overflow(TextOverflow::Ellipsis)
                .key("line")
        })
    }
}

#[test]
fn a_long_line_in_an_ellipsis_cell_is_cut_and_keeps_its_full_name() {
    let mut ui = runtime(400.0);
    let line = long_line();
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("text", "Text")],
        TableSource::new(Lines(line.clone())),
    )
    .unwrap();
    ui.mount_tree(Element::new("table", Table::new(&state, 24.0).unwrap()))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let shown = text(&ui, "line", "shown").expect("cut");
    assert!(shown.starts_with("start") && shown.ends_with(ELLIPSIS));
    assert!(shown.len() <= LIMIT);
    assert_eq!(
        property(&ui, "line", "truncated"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(text(&ui, "line", "text").as_deref(), Some(line.as_str()));
    let name = component(&ui, "line").details.semantics.unwrap().name;
    assert_eq!(name.as_deref(), Some(line.as_str()));
    assert!(!invalid(&ui, "line"));
}

#[test]
fn a_middle_ellipsis_keeps_both_ends_of_a_long_line() {
    let mut ui = runtime(400.0);
    // ASCII at the start and multi-byte text at the end, so the two halves
    // hold different numbers of bytes per cluster.
    let line = format!(
        "{}{}",
        "ascii ".repeat(LIMIT / 5),
        "ÅÄÖ é\u{301} ".repeat(LIMIT / 10)
    );
    ui.mount_tree(
        Label::new(line.clone())
            .overflow(TextOverflow::MiddleEllipsis)
            .key("label"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let shown = text(&ui, "label", "shown").expect("cut");
    let (head, tail) = shown.split_once(ELLIPSIS).expect("one ellipsis");
    assert!(!tail.contains(ELLIPSIS));
    assert!(line.starts_with(head) && line.ends_with(tail));
    assert!(head.starts_with("ascii"));
    assert!(tail.contains("ÅÄÖ"), "{tail:?}");
}

#[test]
fn a_wrapped_label_refuses_oversized_text_and_recovers() {
    let mut ui = runtime(400.0);
    let value = ui.signal("a".repeat(LIMIT + 1));
    ui.mount_tree(Column::new([Label::new(value.read_only()).key("label")]).key("column"))
        .unwrap();
    // Refused before any success: empty and without size.
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    assert!(invalid(&ui, "label"));
    assert_eq!(size(&ui, "label"), [0.0, 0.0]);
    assert_eq!(text(&ui, "label", "text").map(|t| t.len()), Some(LIMIT + 1));
    // A valid text recovers.
    value.set("short text".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_none());
    assert!(!invalid(&ui, "label"));
    let valid = size(&ui, "label");
    assert!(valid[0] > 0.0);
    // Oversized again: the valid layout and size stay.
    value.set("b".repeat(LIMIT + 1)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    assert_eq!(size(&ui, "label"), valid);
    ui.paint().unwrap();
    // Same-key redescription with a fixed oversized text stays refused.
    let mut ui = runtime(400.0);
    let root = ui
        .mount_tree(Column::new([Label::new("ok").key("label")]).key("column"))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.reconcile_children(&root, vec![Label::new("c".repeat(LIMIT + 1)).key("label")])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    ui.reconcile_children(&root, vec![Label::new("fine").key("label")])
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_none());
}

#[test]
fn a_wrapped_label_beyond_the_raster_limit_is_refused_until_it_fits() {
    for scale in [1.0, 2.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(1000.0, 200.0, scale).unwrap())
            .unwrap();
        // Too many wrapped lines at a narrow width, well within the byte limit.
        let long = "ab ".repeat(1500);
        ui.mount_tree(Column::new([Label::new(long).key("label")]).key("column"))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(text(&ui, "label", "text_error").is_none(), "at {scale}");
        let wide = size(&ui, "label");
        ui.set_viewport(Viewport::new(30.0, 200.0, scale).unwrap())
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(text(&ui, "label", "text_error").is_some(), "at {scale}");
        assert_eq!(
            size(&ui, "label")[1],
            wide[1],
            "the wide layout's height stays"
        );
        ui.paint().unwrap();
        ui.set_viewport(Viewport::new(1000.0, 200.0, scale).unwrap())
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(text(&ui, "label", "text_error").is_none(), "at {scale}");
    }
}

#[test]
fn rich_and_very_wide_ellipsis_labels_are_refused() {
    // Rich content is clipped, not cut, so its length is refused.
    let mut ui = runtime(400.0);
    let rich = RichText::from("d".repeat(LIMIT + 1)).leading_icon(Icon::Info);
    ui.mount_tree(
        Label::new(rich)
            .overflow(TextOverflow::Ellipsis)
            .key("label"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    // Too many segments.
    let mut ui = runtime(400.0);
    let mut many = RichText::new();
    for _ in 0..=RichText::MAX_SEGMENTS {
        many = many.icon(Icon::Info);
    }
    ui.mount_tree(Label::new(many).key("label")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    // A cut line shown in an allocation wider than the raster limit.
    let mut ui = runtime(9000.0);
    ui.mount_tree(
        Label::new(long_line())
            .overflow(TextOverflow::Ellipsis)
            .key("label"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
}

fn hover(ui: &mut Runtime, at: Duration) {
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 8.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    ui.flush(at).unwrap();
}

#[test]
fn help_is_offered_only_for_text_it_can_show() {
    let mut ui = runtime(140.0);
    let value = ui.signal("/home/user/projects/mix/stems/final-master.wav".to_owned());
    ui.mount_tree(
        Column::new([Label::new(value.read_only())
            .overflow(TextOverflow::Ellipsis)
            .key("label")])
        .key("column"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    hover(&mut ui, Duration::ZERO);
    ui.flush(Duration::from_millis(500)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
    // Over the byte limit: cut, but without help, and the open help closes.
    value.set(long_line()).unwrap();
    ui.flush(Duration::from_millis(510)).unwrap();
    assert_eq!(
        property(&ui, "label", "truncated"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );
    ui.flush(Duration::from_millis(1100)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );
    // Back within the limit while hovered: the help returns after its delay.
    value
        .set("/home/user/projects/mix/stems/other-master.wav".to_owned())
        .unwrap();
    ui.flush(Duration::from_millis(1200)).unwrap();
    ui.flush(Duration::from_millis(1700)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
}

#[test]
fn a_refused_help_is_empty_rather_than_stale() {
    let mut ui = runtime(1000.0);
    let value = ui.signal("Short help".to_owned());
    ui.mount_tree(
        Column::new([Tooltip::new(
            Element::new("anchor", Button::new("Hover", |_| Ok(()))),
            value.read_only(),
        )
        .key("tooltip")])
        .key("column"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    hover(&mut ui, Duration::ZERO);
    ui.flush(Duration::from_millis(600)).unwrap();
    let help = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "help" && c.details.properties.contains_key("text"))
        .expect("the help label is shown");
    assert!(help.details.box_model.border_box.unwrap()[2] > 0.0);
    // Help too long to lay out at the panel's width is refused and empty.
    value.set("x".repeat(LIMIT + 1)).unwrap();
    ui.flush(Duration::from_millis(700)).unwrap();
    let help = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "help" && c.details.properties.contains_key("text"))
        .expect("the help label is still shown");
    assert!(help.details.properties.contains_key("text_error"));
    assert_eq!(help.details.box_model.border_box.unwrap()[2], 0.0);
}

#[test]
fn components_built_on_a_label_do_not_fail_on_oversized_text() {
    let mut ui = runtime(600.0);
    let state = DialogController::new(&ui);
    ui.mount_tree(
        Column::new([
            Group::titled("x".repeat(LIMIT + 1), Label::new("body").key("body")).key("group"),
            AlertDialog::new(
                state.clone(),
                Element::new("open", Button::new("Delete…", |_| Ok(()))),
                "Delete?",
                "y".repeat(LIMIT + 1),
                "Delete",
            )
            .key("prompt"),
        ])
        .key("column"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    state.open().unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    let refused = ui
        .inspect()
        .components
        .into_iter()
        .filter(|c| c.details.properties.contains_key("text_error"))
        .count();
    assert_eq!(refused, 2, "the heading and the message");
}

/// Measures its child at one width and places it at another, as a container
/// that measures generously and then allocates less would.
struct Host {
    measure: f32,
    arrange: f32,
}
impl Component for Host {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        cx.measure(0, Constraints::new(Some(self.measure), None)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(0.0, 0.0, self.arrange, size.height())?)
    }
}

#[test]
fn the_arrangement_decides_a_refusal_when_measurement_disagrees() {
    let long = "ab ".repeat(1500);
    for (measure, arrange, refused) in [(1000.0, 30.0, true), (30.0, 1000.0, false)] {
        let ui = runtime(1000.0);
        let mut ui = ui;
        ui.mount_tree(
            Element::new("host", Host { measure, arrange })
                .children([Label::new(long.clone()).key("label")]),
        )
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(
            text(&ui, "label", "text_error").is_some(),
            refused,
            "measured at {measure}, arranged at {arrange}"
        );
    }
}

/// The text masks a display list draws, with their clips.
fn masks(list: &DisplayList) -> Vec<Option<Rect>> {
    list.commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { clip, .. } => Some(*clip),
            _ => None,
        })
        .collect()
}

#[test]
fn a_refused_label_draws_nothing_at_first_and_its_last_text_clipped_later() {
    let mut ui = runtime(1000.0);
    let value = ui.signal("a".repeat(LIMIT + 1));
    ui.mount_tree(Label::new(value.read_only()).key("label"))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(
        masks(&ui.paint().unwrap()).is_empty(),
        "nothing before a success"
    );
    value.set("ab ".repeat(1500)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let drawn = masks(&ui.paint().unwrap());
    assert!(!drawn.is_empty());
    // Narrower than the text can be laid out: the wide layout stays, clipped
    // to the new allocation.
    ui.set_viewport(Viewport::new(30.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    let kept = masks(&ui.paint().unwrap());
    assert_eq!(kept.len(), drawn.len(), "the last text stays");
    assert!(
        kept.iter()
            .all(|clip| clip.is_some_and(|clip| clip.width() <= 30.0)),
        "{kept:?}"
    );
}

#[test]
fn open_help_closes_when_its_label_is_refused_and_returns_after() {
    // A text within the byte limit whose truncation in a very wide window is
    // wider than the raster limit. The column stretches the label, so it is
    // arranged at the window's width even while its measurement is refused.
    let words = "word ".repeat(1500);
    let mut ui = runtime(140.0);
    ui.mount_tree(
        Column::new([Label::new(words)
            .overflow(TextOverflow::Ellipsis)
            .key("label")])
        .align(Align::Stretch)
        .key("column"),
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    hover(&mut ui, Duration::ZERO);
    ui.flush(Duration::from_millis(500)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
    ui.set_viewport(Viewport::new(9000.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::from_millis(510)).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(false))
    );
    ui.set_viewport(Viewport::new(140.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.flush(Duration::from_millis(520)).unwrap();
    assert!(text(&ui, "label", "text_error").is_none());
    ui.flush(Duration::from_millis(1100)).unwrap();
    assert_eq!(
        property(&ui, "label", "help_open"),
        Some(InspectValue::Bool(true))
    );
}

#[test]
fn an_overflow_change_into_a_refusal_keeps_the_shown_text_with_its_layout() {
    let mut ui = runtime(140.0);
    let path = "/home/user/projects/mix/stems/final-master.wav";
    let root = ui
        .mount_tree(
            Column::new([Label::new(path)
                .overflow(TextOverflow::Ellipsis)
                .key("label")])
            .key("column"),
        )
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let before = text(&ui, "label", "shown").expect("cut");
    let rich = RichText::from("r".repeat(LIMIT + 1)).leading_icon(Icon::Info);
    ui.reconcile_children(
        &root,
        vec![
            Label::new(rich)
                .overflow(TextOverflow::MiddleEllipsis)
                .key("label"),
        ],
    )
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "label", "text_error").is_some());
    assert_eq!(text(&ui, "label", "shown"), Some(before));
    assert_eq!(
        property(&ui, "label", "truncated"),
        Some(InspectValue::Bool(true))
    );
}
