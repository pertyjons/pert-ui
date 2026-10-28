//! The Controls › Property grid page: an inspector of labeled editors in
//! accordion sections whose grids share one label column width.
use crate::gallery::{label, page, section};
use pert_ui::parameter::*;
use pert_ui::prelude::*;

/// The sample object's rendering modes, shown by the mode chooser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Linear,
    Stepped,
    Smooth,
}

/// Builds the page. `level` identifies the knob's sample parameter.
pub(crate) fn property_grid(ui: &mut Runtime, level: ParameterId) -> Result<Element, AppError> {
    let form = FormFields::new(ui);
    let name_status = form.field("name")?;
    let name = ui.signal(String::from("Channel 1"));
    let enabled = ui.signal(true);
    let mode = ui.signal(Mode::Linear);
    let voices = ui.signal(4_i64);
    let offset = ui.signal(0.25_f64);
    let color = ui.signal(Color::from_srgb8(0x40, 0x80, 0xd0, 0xff));
    let visible = ui.signal(true);
    let width = ui.signal(Length::new(140.0)?);
    let (knob, _host) = ui.parameter(level, ParameterSpec::new("Level", 0.0, 1.0, 0.8)?)?;
    let outcome = ui.signal("Press Check to apply the gallery's rule for the name.".to_owned());

    let general = PropertyGrid::new([
        Property::new(
            "name",
            "Name",
            Element::new("name-input", TextField::new(name.clone())),
        )
        .help("Required. Clear it and press Check.")
        .form(&name_status),
        Property::new(
            "enabled",
            "Enabled",
            Element::new("enabled-input", Checkbox::new(enabled, "Process audio")),
        ),
        Property::new(
            "mode",
            "Interpolation mode",
            Element::new(
                "mode-input",
                Select::new(
                    ui,
                    mode,
                    [
                        Choice::new(Mode::Linear, "Linear"),
                        Choice::new(Mode::Stepped, "Stepped"),
                        Choice::new(Mode::Smooth, "Smooth"),
                    ],
                )?,
            ),
        ),
        Property::new(
            "voices",
            "Voices",
            Element::new(
                "voices-input",
                IntegerInput::new(voices).range(1, 64)?.stepper(true),
            ),
        ),
    ])
    .bind_label_width(width.clone());
    let appearance = PropertyGrid::new([
        Property::new(
            "offset",
            "Offset",
            Element::new(
                "offset-input",
                DecimalInput::new(offset)
                    .range(-1.0, 1.0)?
                    .step(0.05)?
                    .stepper(true),
            ),
        ),
        Property::new(
            "level",
            "Level",
            Element::new("level-input", Knob::new(knob).show_value(true)),
        ),
        Property::new(
            "color",
            "Color",
            Element::new(
                "color-input",
                ColorPicker::new(color).name("Color").into_popover(),
            ),
        ),
        Property::new(
            "visible",
            "Visible in the overview list of every open window",
            Element::new("visible-input", Toggle::new(visible, "Visible")),
        ),
    ])
    .bind_label_width(width.clone());

    let (check_name, check_status, check_form) = (name.clone(), name_status.clone(), form.clone());
    let check_outcome = outcome.clone();
    let reset_width = width.clone();
    Ok(page([section(
        "property-grid-section",
        "Property grid",
        [
            label(
                "property-grid-help",
                "Two accordion sections hold one grid each, bound to one label width, so their \
                 columns stay aligned. Drag the gap between labels and editors, or Tab to a \
                 grid and press Left or Right (Shift for larger steps); Escape cancels a \
                 resize. Long labels wrap in the label column. Help text and status appear \
                 under each editor.",
            ),
            Element::new(
                "inspector",
                Accordion::new([
                    Section::new("general", "General", Element::new("general-grid", general)),
                    Section::new(
                        "appearance",
                        "Appearance",
                        Element::new("appearance-grid", appearance),
                    ),
                ])
                .expanded(["general", "appearance"]),
            ),
            Element::new(
                "property-actions",
                Wrap::new([
                    Element::new(
                        "property-check",
                        Button::new("Check", move |cx| {
                            check_status.set(if check_name.get()?.trim().is_empty() {
                                FieldStatus::error("A name is required")
                            } else {
                                FieldStatus::Normal
                            })?;
                            let summary = check_form.summary().get()?;
                            check_outcome.set(format!("{} error(s)", summary.errors()))?;
                            check_form.focus_first_error(cx).map(|_| ())
                        }),
                    ),
                    Element::new(
                        "property-reset-width",
                        Button::new("Reset label width", move |_| {
                            reset_width.set(Length::new(140.0)?)
                        }),
                    ),
                ]),
            ),
            Element::new("property-outcome", Label::new(outcome.read_only())),
            Element::new(
                "property-width",
                Label::new(width.map(|width| format!("Label width: {} px", width.get()))?),
            ),
        ],
    )]))
}
