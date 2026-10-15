//! `TextSource` captions of fields, inline rows and table columns through the
//! public facade: a binding is followed without redescription, the same
//! binding keeps its read text across redescription, fixed and bound text
//! replace each other, a parameter row without a caption keeps the
//! specification's name, and a table header follows a bound title while
//! sorting keeps working.
use pert_ui::*;
mod support;
use std::{cell::RefCell, rc::Rc};
use support::frame;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(480.0, 320.0, 1.0)?)?;
    Ok(ui)
}

fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}

fn name(ui: &Runtime, key: &str) -> Option<String> {
    component(ui, key).details.semantics.and_then(|s| s.name)
}

/// A field, two numeric rows and a readout, each captioned by `caption`.
fn rows(
    caption: &TextSource,
    integer: &Signal<i64>,
    gain: &Parameter<f32>,
    load: &Signal<String>,
) -> Vec<Element> {
    vec![
        Element::new(
            "field",
            Field::new(Element::new("button", Button::new("Apply", |_| Ok(()))))
                .label(caption.clone())
                .help(caption.clone()),
        ),
        Element::new(
            "integer",
            InlineInteger::new(integer.clone()).label(caption.clone()),
        ),
        Element::new(
            "slider",
            InlineSlider::new(gain.clone()).label(caption.clone()),
        ),
        Element::new(
            "readout",
            InlineReadout::new(load.read_only()).label(caption.clone()),
        ),
    ]
}

const KEYS: [&str; 4] = ["field", "integer", "slider", "readout"];

#[test]
fn captions_follow_their_binding_and_keep_it_across_redescription() -> TestResult {
    let mut ui = runtime()?;
    let integer = ui.signal(3_i64);
    let (gain, _host) = ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )?;
    let load = ui.signal("12".to_owned());
    let text = ui.signal("Level".to_owned());
    let bound = TextSource::from(text.read_only());
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new(rows(&bound, &integer, &gain, &load)),
    ))?;
    frame(&mut ui);
    for key in KEYS {
        assert_eq!(name(&ui, key).as_deref(), Some("Level"), "{key}");
    }
    let before = ui.paint()?;
    text.set("Pegel".to_owned())?;
    frame(&mut ui);
    for key in KEYS {
        assert_eq!(name(&ui, key).as_deref(), Some("Pegel"), "{key}");
    }
    assert_ne!(before, ui.paint()?, "the new caption is drawn");
    assert_eq!(
        component(&ui, "field").details.properties["help"],
        InspectValue::Text("Pegel".into())
    );

    // The same binding keeps the read text before any update.
    ui.reconcile_children(&root, rows(&bound, &integer, &gain, &load))?;
    for key in KEYS {
        assert_eq!(name(&ui, key).as_deref(), Some("Pegel"), "{key}");
    }
    frame(&mut ui);

    // Fixed text replaces the binding, which no longer applies.
    let fixed = TextSource::from("Fixed");
    ui.reconcile_children(&root, rows(&fixed, &integer, &gain, &load))?;
    frame(&mut ui);
    text.set("Ignored".to_owned())?;
    frame(&mut ui);
    for key in KEYS {
        assert_eq!(name(&ui, key).as_deref(), Some("Fixed"), "{key}");
    }

    // And a binding replaces fixed text again.
    ui.reconcile_children(&root, rows(&bound, &integer, &gain, &load))?;
    frame(&mut ui);
    for key in KEYS {
        assert_eq!(name(&ui, key).as_deref(), Some("Ignored"), "{key}");
    }
    Ok(())
}

#[test]
fn a_bound_rich_field_label_matches_the_same_fixed_label() -> TestResult {
    let content = RichText::from("Gain").leading_icon(Icon::Info);
    let describe = |label: TextSource| {
        Element::new(
            "root",
            Column::new([Element::new(
                "field",
                Field::new(Element::new("button", Button::new("Apply", |_| Ok(()))))
                    .label(label)
                    .required(true),
            )]),
        )
    };
    let mut bound = runtime()?;
    let source = bound.signal(RichText::from("Before"));
    bound.mount_tree(describe(source.read_only().into()))?;
    frame(&mut bound);
    source.set(content.clone())?;
    frame(&mut bound);
    let mut fixed = runtime()?;
    fixed.mount_tree(describe(content.into()))?;
    frame(&mut fixed);
    assert_eq!(bound.paint()?, fixed.paint()?);
    assert_eq!(name(&bound, "field").as_deref(), Some("Gain"));
    Ok(())
}

fn waveforms() -> Result<ParameterSpec<ChoiceId>, ParameterError> {
    ParameterSpec::choices(
        "Waveform",
        [
            ChoiceOption::new(ChoiceId::new("sine")?, "Sine"),
            ChoiceOption::new(ChoiceId::new("saw")?, "Saw"),
        ],
        ChoiceId::new("sine")?,
    )
}

#[test]
fn an_inline_select_caption_follows_a_binding_and_an_empty_one_falls_back() -> TestResult {
    let mut ui = runtime()?;
    let (waveform, _host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let caption = ui.signal("Wave".to_owned());
    let select = InlineSelect::for_parameter(&ui, waveform).label(caption.read_only());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("control", select)]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "select").as_deref(), Some("Wave"));
    assert_eq!(
        component(&ui, "select").details.properties["label"],
        InspectValue::Text("Wave".into())
    );
    caption.set("Welle".to_owned())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "select").as_deref(), Some("Welle"));
    // An empty bound caption falls back to the specification's name.
    caption.set(String::new())?;
    frame(&mut ui);
    assert_eq!(name(&ui, "select").as_deref(), Some("Waveform"));

    // So does an empty fixed caption.
    let mut ui = runtime()?;
    let (waveform, _host) = ui.parameter(ParameterId::new(1, 1, "waveform")?, waveforms()?)?;
    let select = InlineSelect::for_parameter(&ui, waveform).label("");
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("control", select)]),
    ))?;
    frame(&mut ui);
    assert_eq!(name(&ui, "select").as_deref(), Some("Waveform"));
    Ok(())
}

/// Rows numbered `0..count`, keyed `n0`, `n1`, … in the order given.
struct Numbers(Vec<u32>);
impl TableItems for Numbers {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|number| format!("n{number}"))
    }
    fn cell(&self, index: usize, _: &str) -> Option<Element> {
        let number = *self.0.get(index)?;
        Some(Element::new("text", Label::new(number.to_string())))
    }
}

fn header_names(ui: &Runtime) -> Vec<String> {
    component(ui, "table")
        .details
        .semantics
        .unwrap()
        .items
        .into_iter()
        .filter_map(|item| item.name)
        .collect()
}

#[test]
fn a_table_header_follows_a_bound_title_and_still_sorts() -> TestResult {
    let mut ui = runtime()?;
    let title = ui.signal("Number".to_owned());
    let state = TableController::new(
        &ui,
        [
            TableColumn::new("number", title.read_only())
                .width(Length::new(120.0)?)
                .sortable(true),
            TableColumn::new("other", "Other"),
        ],
        TableSource::new(Numbers((0..5).collect())),
        ListSelection::new(&ui, SelectionMode::Single),
    )?;
    let sorts = Rc::new(RefCell::new(Vec::new()));
    let log = sorts.clone();
    let describe = |state: &TableController, log: Rc<RefCell<Vec<TableSort>>>| {
        Table::new(state, 24.0).map(|table| {
            Element::new(
                "table",
                table.on_sort(move |_, sort| {
                    log.borrow_mut().push(sort.clone());
                    Ok(())
                }),
            )
        })
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(&state, log.clone())?]),
    ))?;
    frame(&mut ui);
    assert_eq!(header_names(&ui), ["Number", "Other"]);
    assert_eq!(state.columns()[0].title(), "Number");
    let before = ui.paint()?;

    title.set("Nummer".to_owned())?;
    frame(&mut ui);
    assert_eq!(header_names(&ui), ["Nummer", "Other"]);
    assert_eq!(state.columns()[0].title(), "Nummer");
    assert_eq!(
        state.columns()[0],
        TableColumn::new("number", title.read_only())
            .width(Length::new(120.0)?)
            .sortable(true),
        "the text read from the binding is not compared"
    );
    assert_ne!(before, ui.paint()?, "the new title is drawn");
    // Mounted cells take their name from the current title.
    let cell = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "number")
        .and_then(|c| c.details.semantics)
        .and_then(|s| s.name);
    assert_eq!(cell.as_deref(), Some("Nummer"));

    // Describing the table again keeps the title it read.
    ui.reconcile_children(&root, vec![describe(&state, log)?])?;
    assert_eq!(header_names(&ui), ["Nummer", "Other"]);
    frame(&mut ui);

    // A press on the retitled header still sorts by its key.
    for kind in [
        PointerKind::Press(PointerButton::Primary),
        PointerKind::Release(PointerButton::Primary),
    ] {
        ui.pointer_input(PointerEvent::new(Point::new(40.0, 10.0)?, kind))?;
    }
    frame(&mut ui);
    assert_eq!(
        sorts.borrow().as_slice(),
        [TableSort {
            column: "number".into(),
            direction: SortDirection::Ascending,
        }]
    );
    Ok(())
}

#[test]
fn table_columns_compare_their_title_source() -> TestResult {
    let ui = Runtime::new();
    let title = ui.signal("Name".to_owned());
    assert_eq!(TableColumn::new("a", "Name"), TableColumn::new("a", "Name"));
    assert_ne!(
        TableColumn::new("a", "Name"),
        TableColumn::new("a", "Other")
    );
    assert_eq!(
        TableColumn::new("a", title.read_only()),
        TableColumn::new("a", title.read_only())
    );
    assert_ne!(
        TableColumn::new("a", title.read_only()),
        TableColumn::new("a", "Name")
    );
    // A bound title is empty until a mounted table reads it.
    assert_eq!(TableColumn::new("a", title.read_only()).title(), "");
    Ok(())
}
