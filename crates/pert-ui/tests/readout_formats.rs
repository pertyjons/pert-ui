//! Slider and knob readouts formatted by an application `NumberFormat`.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A decibel presentation: the application's business, applied by the readout.
fn decibels() -> NumberFormat<f32> {
    NumberFormat::new(
        "-inf dB",
        |gain: f32| {
            if gain <= 0.0 {
                "-inf".into()
            } else {
                format!("{:.1}", 20.0 * gain.log10())
            }
        },
        |text| text.parse().map_err(|_| NumberInputError::InvalidNumber),
    )
    .unit(|_| "dB".into())
}

/// A format without a unit callback, so the literal unit applies.
fn percent() -> NumberFormat<f32> {
    NumberFormat::new(
        "100.0",
        |gain: f32| format!("{:.1}", gain * 100.0),
        |text| text.parse().map_err(|_| NumberInputError::InvalidNumber),
    )
}

fn gain(ui: &mut Runtime) -> Result<(Parameter<f32>, ParameterHost<f32>), RuntimeError> {
    ui.parameter(
        ParameterId::new(1, 1, "gain")?,
        ParameterSpec::new("Gain", 0.0, 1.0, 0.5)?,
    )
}

fn readout(ui: &Runtime, key: &str) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|entry| entry.key == key)
        .and_then(|entry| entry.details.properties.get("readout").cloned())
}

/// The readout glyph masks: their rectangles and clips.
fn readout_rects(scene: &DisplayList) -> Vec<(Rect, Option<Rect>)> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, clip, color, ..
            } if *color == Palette::dark().color(ColorRole::OnSurface) => Some((*rect, *clip)),
            _ => None,
        })
        .collect()
}

fn layout_work(ui: &Runtime) -> Vec<ProfileStats> {
    ui.profile()
        .stats
        .into_iter()
        .filter(|stat| {
            matches!(
                stat.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange
            )
        })
        .collect()
}

#[test]
fn formatted_readouts_show_the_display_text_and_unit() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0)?)?;
    let (parameter, host) = gain(&mut ui)?;
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "slider",
                Slider::new(parameter.clone())
                    .show_value(true)
                    .unit("%")
                    .format(decibels()),
            ),
            Element::new(
                "knob",
                Knob::new(parameter.clone())
                    .show_value(true)
                    .format(decibels()),
            ),
            Element::new(
                "literal-unit",
                Slider::new(parameter.clone())
                    .show_value(true)
                    .unit("%")
                    .format(percent()),
            ),
            Element::new("hidden", Slider::new(parameter.clone()).format(decibels())),
            Element::new(
                "default",
                Slider::new(parameter.clone())
                    .show_value(true)
                    .decimal_places(1)?,
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let text = |s: &str| Some(InspectValue::Text(s.into()));
    // The unit callback wins over the literal unit; the literal applies without one.
    assert_eq!(readout(&ui, "slider"), text("-6.0 dB"));
    assert_eq!(readout(&ui, "knob"), text("-6.0 dB"));
    assert_eq!(readout(&ui, "literal-unit"), text("50.0 %"));
    assert_eq!(readout(&ui, "hidden"), None);
    assert_eq!(readout(&ui, "default"), text("0.5"));
    // The format never quantizes: the parameter keeps its exact value.
    host.write_base(0.0)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(readout(&ui, "slider"), text("-inf dB"));
    assert_eq!(parameter.snapshot().get()?.base, 0.0);
    Ok(())
}

#[test]
fn the_width_hint_reserves_the_readout_and_values_keep_its_geometry() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0)?)?;
    let (parameter, host) = gain(&mut ui)?;
    let wide = NumberFormat::new(
        "-999.99 dB",
        |gain: f32| format!("{gain:.2}"),
        |text| text.parse().map_err(|_| NumberInputError::InvalidNumber),
    );
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "narrow",
                Slider::new(parameter.clone())
                    .show_value(true)
                    .format(decibels()),
            ),
            Element::new(
                "wide",
                Slider::new(parameter.clone()).show_value(true).format(wide),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let before = readout_rects(&ui.paint()?);
    assert_eq!(before.len(), 2, "one readout per slider");
    let (narrow, wide) = (before[0].1.ok_or("clip")?, before[1].1.ok_or("clip")?);
    assert!(
        wide.width() > narrow.width(),
        "the longer hint reserves a wider slot: {wide:?} vs {narrow:?}"
    );
    let slider = ui.children(&root)?[0].clone();
    let bounds = ui.bounds(&slider)?.ok_or("bounds")?;
    assert!(
        narrow.x() + narrow.width() <= bounds.x() + bounds.width() + 0.5,
        "the slot ends inside the control"
    );
    // A shorter text ("0.0 dB" for unity gain) keeps the slot and ends at its right edge.
    host.write_base(1.0)?;
    ui.flush(Duration::ZERO)?;
    let after = readout_rects(&ui.paint()?);
    assert_eq!(after[0].1, before[0].1, "the slot is unchanged");
    assert_eq!(after[1].1, before[1].1);
    let right = |(rect, _): &(Rect, Option<Rect>)| rect.x() + rect.width();
    assert!((right(&after[0]) - right(&before[0])).abs() < 0.5);
    assert!(
        after[0].0.x() > before[0].0.x(),
        "shorter text starts later"
    );
    Ok(())
}

#[test]
fn unchanged_cloned_formats_retain_layout_and_changed_ones_remeasure() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 300.0, 1.0)?)?;
    let (parameter, host) = gain(&mut ui)?;
    let format = decibels();
    let describe = |format: &NumberFormat<f32>| {
        vec![
            Element::new(
                "slider",
                Slider::new(parameter.clone())
                    .show_value(true)
                    .format(format.clone()),
            ),
            Element::new(
                "knob",
                Knob::new(parameter.clone())
                    .show_value(true)
                    .format(format.clone()),
            ),
        ]
    };
    let root = ui.mount_tree(Element::new("root", Row::new(describe(&format))))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    ui.set_profiling(true)?;
    ui.reconcile_children(&root, describe(&format))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let work = layout_work(&ui);
    assert!(
        work.is_empty(),
        "unchanged cloned formats caused layout: {work:?}"
    );
    // A wider range changes the default presentation's digits, but a
    // formatted readout reserves its hint and stays put.
    host.replace_spec(ParameterSpec::new("Gain", -1000.0, 1000.0, 0.5)?)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let work = layout_work(&ui);
    assert!(work.is_empty(), "a range change caused layout: {work:?}");
    // A new format, even with the same text, is changed presentation.
    ui.reconcile_children(&root, describe(&decibels()))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert!(
        !layout_work(&ui).is_empty(),
        "a replaced format did not remeasure"
    );
    Ok(())
}
