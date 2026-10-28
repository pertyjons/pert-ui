//! The Text category: text and number input, fonts and icons.
use crate::gallery::{label, page, section, stage};
use pert_ui::prelude::*;
use pert_ui::{FontData, FontRole, GeometryError, RegionLayout, TextRenderingMode};

pub(crate) fn text_rendering_page(ui: &Runtime) -> Result<Element, AppError> {
    let settings = ui.ui_settings();
    let line = settings.clone();
    let recomposed = settings.clone();
    let active = settings.text_rendering_binding()?;
    let scrolling = Column::new((0..24).map(|index| {
        Element::new(
            format!("sample-{index}"),
            Label::new(format!("Hamburgefonts 0123456789 — Il1 O0, row {index}")),
        )
    }));
    Ok(page([section(
        "live-text",
        "Live text rendering modes",
        [
            label(
                "live-help",
                "Switch between the default and phase-aware text masks. Edit and scroll the sample, then compare at different zoom levels and themes. Settings > Text rendering changes the same runtime setting.",
            ),
            Element::new(
                "live-mode-controls",
                Wrap::new([
                    Element::new(
                        "live-line-mask",
                        Button::new("Original line mask", move |_| {
                            line.set_text_rendering(TextRenderingMode::LineMask)
                                .map(|_| ())
                        }),
                    ),
                    Element::new(
                        "live-phase-line",
                        Button::new("Phase-aware line mask", move |_| {
                            recomposed
                                .set_text_rendering(TextRenderingMode::PhaseLine)
                                .map(|_| ())
                        }),
                    ),
                ])
                .gap(MetricRole::ItemGap),
            ),
            Element::new(
                "live-current-mode",
                Label::new(active.map(|mode| format!("Active mode: {mode:?}"))?),
            ),
            Element::new(
                "live-input",
                TextField::new(ui.signal("Edit this text to compare caret and glyphs".to_owned())),
            )
            .with_label("Text rendering comparison"),
            stage(
                "live-scroll-stage",
                220,
                Element::new(
                    "live-scroll",
                    ScrollArea::new(Element::new("rows", scrolling)),
                ),
            ),
        ],
    )]))
}

pub(crate) fn number_inputs(ui: &mut Runtime) -> Result<Element, AppError> {
    Ok(page([
        label(
            "number-inline-note",
            "One-line inline numeric rows for toolbars and strips live on the Toolbar page.",
        ),
        number_input_fields(ui)?,
    ]))
}
/// Presents a fraction as a percentage: one decimal at rest, the plain
/// percentage while editing, and typed percentages parsed back to fractions.
/// Formatting is presentation only; the field still rejects a parsed value
/// outside its range instead of clamping it.
fn percent() -> NumberFormat<f64> {
    NumberFormat::new(
        "100.0",
        |value: f64| format!("{:.1}", value * 100.0),
        |text| {
            text.trim()
                .replace(',', ".")
                .parse::<f64>()
                .map(|value| value / 100.0)
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .edit_text(|value| format!("{}", (value * 100.0 * 1e9).round() / 1e9))
}

fn number_input_fields(ui: &Runtime) -> Result<Element, RuntimeError> {
    let count = ui.signal(12_i64);
    let amount = ui.signal(1.234567_f64);
    let voices = ui.signal(8_i64);
    let level = ui.signal(-6.0_f64);
    let mix = ui.signal(0.35_f64);
    let external_count = count.clone();
    let external_amount = amount.clone();
    Ok(Element::new(
        "number-input-page",
        Group::titled(
            "Integer and decimal fields",
            Element::new(
                "number-fields",
                Column::new([
                    label(
                        "number-help",
                        "Enter or leaving the field saves. Escape restores the saved value. Up/Down or the wheel over a focused field changes the draft. Shift: finer steps.",
                    ),
                    label("integer-label", "Integer: −100 to 100, step 1"),
                    Element::new(
                        "integer-input",
                        IntegerInput::new(count.clone()).range(-100, 100)?,
                    )
                    .with_label("Integer"),
                    Element::new(
                        "integer-saved",
                        Label::new(count.map(|v| format!("Saved integer: {v}"))?),
                    ),
                    label(
                        "decimal-label",
                        "Decimal: −10 to 10, step 0.1, Shift for finer steps; shown with two decimals but saved and stepped at full precision",
                    ),
                    Element::new(
                        "decimal-input",
                        DecimalInput::new(amount.clone())
                            .range(-10.0, 10.0)?
                            .decimal_places(2)?,
                    )
                    .with_label("Decimal"),
                    Element::new(
                        "decimal-saved",
                        Label::new(amount.map(|v| format!("Saved decimal (full precision): {v}"))?),
                    ),
                    label(
                        "stepper-label",
                        "Step buttons: 1 to 16. A press saves one step; holding repeats.",
                    ),
                    Element::new(
                        "stepper-input",
                        IntegerInput::new(voices).range(1, 16)?.stepper(true),
                    )
                    .with_label("Voices"),
                    label(
                        "normalized-label",
                        "Rounded to tenths when saved, shown with its unit at rest; ±24 dB",
                    ),
                    Element::new(
                        "normalized-input",
                        DecimalInput::new(level.clone())
                            .range(-24.0, 24.0)?
                            .unit("dB")
                            .normalize(|value| Ok((value * 10.0).round() / 10.0))?,
                    )
                    .with_label("Level"),
                    Element::new(
                        "normalized-saved",
                        Label::new(level.map(|v| format!("Saved level: {v}"))?),
                    ),
                    label(
                        "formatted-label",
                        "A NumberFormat on an ordinary field: the value is a fraction from 0 to 1, shown as a percentage at rest and edited as a plain percentage while focused; steps of 1 %",
                    ),
                    Element::new(
                        "formatted-input",
                        DecimalInput::new(mix.clone())
                            .range(0.0, 1.0)?
                            .step(0.01)?
                            .unit("%")
                            .stepper(true)
                            .format(percent()),
                    )
                    .with_label("Mix"),
                    Element::new(
                        "formatted-saved",
                        Label::new(mix.map(|v| format!("Saved fraction: {v}"))?),
                    ),
                    label(
                        "number-invalid-help",
                        "Try an empty field, a lone minus or a value outside the range. The outline marks invalid drafts; saved values stay unchanged. Both . and , work as decimal separators.",
                    ),
                    label(
                        "number-readonly-label",
                        "Read-only mirror and disabled integer",
                    ),
                    Element::new("decimal-readonly", DecimalInput::new(amount).read_only())
                        .with_label("Saved decimal"),
                    Element::new("integer-disabled", IntegerInput::new(count).disabled())
                        .with_label("Disabled integer"),
                    Element::new(
                        "number-external",
                        Button::new("Replace numeric values", move |_| {
                            external_count.set(25)?;
                            external_amount.set(2.3456789)?;
                            Ok(())
                        }),
                    ),
                ]),
            ),
        ),
    ))
}
const NOTES_TEXT: &str = "Multiline text uses monospace by default.
Double-click a word; triple-click to select everything.
Try selection, copy/paste, undo and line wrapping.
Text taller than the field shows the standard scrollbar.
Drag its thumb or click its track to scroll this text.
The wheel scrolls it as well, leaving the caret alone.
The next edit or caret movement reveals the caret again.";
/// A single-line field the application writes a value it refuses, such as a
/// line break, with `on_binding_refusal` reporting the refusal and its end.
fn refused_binding(ui: &Runtime) -> Element {
    let value = ui.signal("Single line".to_owned());
    let report = ui.signal("The application's writes are accepted.".to_owned());
    let (refuse, accept, shown) = (value.clone(), value.clone(), report.clone());
    Element::new(
        "refusal-example",
        Column::new([
            Element::new(
                "refusal-field",
                TextField::new(value).on_binding_refusal(move |_, refusal| {
                    shown.set(match refusal {
                        Some(reason) => format!("The field refused the write: {reason}"),
                        None => "The application's writes are accepted again.".to_owned(),
                    })
                }),
            )
            .with_label("Refusing field"),
            Element::new(
                "refusal-buttons",
                Row::new([
                    Element::new(
                        "write-line-break",
                        Button::new("Write a line break", move |_| {
                            refuse.set("Two\nlines".to_owned())
                        }),
                    ),
                    Element::new(
                        "write-single-line",
                        Button::new("Write one line", move |_| accept.set("One line".to_owned())),
                    ),
                ]),
            ),
            Element::new("refusal-report", Label::new(report.read_only())),
        ]),
    )
}

pub(crate) fn text_editing(ui: &Runtime) -> Result<Element, RuntimeError> {
    let name = ui.signal("Editable name".to_owned());
    let notes = ui.signal(NOTES_TEXT.to_owned());
    let submitted = ui.signal("Press Enter in the name field to submit.".to_owned());
    let tag = ui.signal("drums".to_owned());
    let tag_status = ui.signal(FieldStatus::Normal);
    let tag_status_view = tag_status.read_only();
    let submit_result = submitted.clone();
    let reset_name = name.clone();
    let reset_notes = notes.clone();
    Ok(page([
        Element::new(
            "text-editing-page",
            Group::titled(
                "Text fields and areas",
                Element::new(
                    "text-fields",
                    Column::new([
                        label(
                            "editing-help",
                            "Tab changes focus. Shift extends selection. Ctrl/Cmd+A/C/X/V/Z selects, copies, cuts, pastes and undoes.",
                        ),
                        label("name-label", "Single-line — body font"),
                        Element::new(
                            "name-field",
                            TextField::new(name.clone()).on_submit(move |_, text| {
                                submit_result.set(format!("Submitted: {text}"))
                            }),
                        )
                        .with_label("Name"),
                        label(
                            "tall-field-label",
                            "Tall single-line field and button: compare their text height and center.",
                        ),
                        Element::new(
                            "tall-field-row",
                            Wrap::new([
                                Element::new(
                                    "tall-field-box",
                                    Container::new(
                                        Element::new(
                                            "tall-field",
                                            TextField::new(ui.signal("Centered text".to_owned())),
                                        )
                                        .with_label("Tall field"),
                                    )
                                    .preferred_size(Size::from([240, 64])),
                                ),
                                Element::new(
                                    "tall-button-box",
                                    Container::new(Element::new(
                                        "tall-button",
                                        Button::new("Compare", |_| Ok(())),
                                    ))
                                    .preferred_size(Size::from([120, 64])),
                                ),
                            ])
                            .gap(MetricRole::ItemGap),
                        ),
                        Element::new("submitted-name", Label::new(submitted.read_only())),
                        label(
                            "tag-label",
                            "Normalized: trimmed and lower-cased on Enter or focus loss; an empty tag is refused",
                        ),
                        Element::new(
                            "tag-field",
                            Field::new(Element::new(
                                "tag-input",
                                TextField::new(tag.clone())
                                    .normalize(|text| {
                                        let text = text.trim().to_lowercase();
                                        if text.is_empty() {
                                            Err(InputRejection::new("Enter a tag"))
                                        } else {
                                            Ok(text)
                                        }
                                    })
                                    .on_rejection(move |_, rejection| {
                                        let next = rejection.map_or(FieldStatus::Normal, |r| {
                                            FieldStatus::error(r.message())
                                        });
                                        tag_status.set(next)
                                    }),
                            ))
                            .label("Tag")
                            .status_signal(tag_status_view),
                        ),
                        Element::new("tag-saved", Label::new(tag.read_only())),
                        label("notes-label", "Multiline — monospace by default"),
                        Element::new("notes-field", TextArea::new(notes.clone()))
                            .with_label("Notes"),
                        label(
                            "readonly-label",
                            "Read-only mirror — selection and copy remain available",
                        ),
                        Element::new("notes-mirror", TextArea::new(notes).read_only())
                            .with_label("Notes mirror"),
                        label(
                            "selectable-label",
                            "Selectable display text — drag, double-click or Shift+arrows select; Ctrl/Cmd+C or the context menu copies",
                        ),
                        SelectableText::new(
                            "12:00:03.412 WARN http request timeout after 30 s (retry 2 of 3), id=48211",
                        )
                        .key("selectable-text"),
                        label("proportional-label", "Explicit body typography override"),
                        Element::new(
                            "body-area",
                            TextArea::new(ui.signal(
                                "Multiline can also use proportional body typography.".to_owned(),
                            ))
                            .style(TextStyle::Body),
                        )
                        .with_label("Body text"),
                        Element::new("disabled-field", TextField::new(name).disabled())
                            .with_label("Disabled field"),
                        Element::new(
                            "reset-text",
                            Button::new("Replace bound text", move |_| {
                                reset_name.set("External name".to_owned())?;
                                reset_notes.set(
                                    "External update\nEditing follows the new binding.".to_owned(),
                                )?;
                                Ok(())
                            }),
                        ),
                        refused_binding(ui),
                    ]),
                ),
            ),
        ),
        search_example(ui)?,
        sign_in_example(ui)?,
        field_examples(ui),
    ]))
}

// A sign-in style form showing the input states: placeholders, required
// fields, length limits with a counter, a concealed password with a reveal
// button, and a pending check the application finishes later.
fn sign_in_example(ui: &Runtime) -> Result<Element, RuntimeError> {
    let form = FormFields::new(ui);
    let user = form.field("user")?;
    let password = form.field("password")?;
    let user_name = ui.signal(String::new());
    let secret = ui.signal(String::new());
    let outcome = ui.signal("Fill in both fields and press Sign in.".to_owned());
    let publish = |handle: &FieldHandle| {
        let handle = handle.clone();
        move |_: &mut EventContext, rejection: Option<&InputRejection>| {
            let next = rejection.map_or(FieldStatus::Normal, |r| FieldStatus::error(r.message()));
            handle.set(next)
        }
    };
    let delay = form.field("delay")?;
    let delay_text = ui.signal("250".to_owned());
    // Each field reports whether it holds an unsaved draft, and the form
    // counts them.
    let unsaved = |handle: &FieldHandle| {
        let handle = handle.clone();
        move |_: &mut EventContext, modified: bool| handle.set_modified(modified).map(|_| ())
    };
    let (check_user, finish_user, finish_name) = (user.clone(), user.clone(), user_name.clone());
    let (sign_form, sign_name, sign_secret, sign_outcome) = (
        form.clone(),
        user_name.clone(),
        secret.clone(),
        outcome.clone(),
    );
    let summary = form.summary();
    Ok(section(
        "sign-in-example",
        "Input states",
        [
            label(
                "sign-in-help",
                "Both fields are required (*). The user name keeps 3–20 characters and the password at least 8, counted as you see them and checked when the field saves on Enter or focus loss. The password shows dots: press the eye or Alt+F8 to reveal it; it hides again when focus leaves. Check the name to see a pending status, then finish the check (the name \"admin\" is taken). The × button clears a field; a dot in a field's corner marks an edit not saved yet, and the summary counts them. The delay field shows an icon and its unit around the text.",
            ),
            Element::new(
                "user-field",
                Field::new(Element::new(
                    "user-input",
                    TextField::new(user_name.clone())
                        .placeholder("Your user name")
                        .required(true)
                        .min_length(Some(3))
                        .max_length(Some(20))
                        .counter(true)
                        .clearable(true)
                        .modified_indicator(true)
                        .normalize(|text| Ok(text.trim().to_owned()))
                        .on_rejection(publish(&user))
                        .on_modified(unsaved(&user)),
                ))
                .label("User name")
                .required(true)
                .form(&user),
            ),
            Element::new(
                "password-field",
                Field::new(Element::new(
                    "password-input",
                    TextField::new(secret.clone())
                        .concealed(true)
                        .revealable(true)
                        .clearable(true)
                        .placeholder("At least 8 characters")
                        .required(true)
                        .min_length(Some(8))
                        .counter(true)
                        .modified_indicator(true)
                        .on_rejection(publish(&password))
                        .on_modified(unsaved(&password)),
                ))
                .label("Password")
                .required(true)
                .form(&password),
            ),
            Element::new(
                "delay-field",
                Field::new(Element::new(
                    "delay-input",
                    TextField::new(delay_text)
                        .leading_icon(Icon::Timer)
                        .suffix("ms")
                        .max_length(Some(6))
                        .modified_indicator(true)
                        .on_modified(unsaved(&delay)),
                ))
                .label("Login delay")
                .form(&delay),
            ),
            Element::new(
                "sign-in-actions",
                Wrap::new([
                    Element::new(
                        "check-name",
                        Button::new("Check the name", move |_| {
                            check_user
                                .set(FieldStatus::pending("Checking whether the name is free"))
                        }),
                    ),
                    Element::new(
                        "finish-check",
                        Button::new("Finish the check", move |_| {
                            let next = if finish_name.get()? == "admin" {
                                FieldStatus::error("That name is taken")
                            } else {
                                FieldStatus::Normal
                            };
                            finish_user.set(next)
                        }),
                    ),
                    Element::new(
                        "sign-in",
                        Button::new("Sign in", move |cx| {
                            let summary = sign_form.summary().get()?;
                            let text = if summary.pending() > 0 {
                                "Wait until the name check has finished.".to_owned()
                            } else if summary.errors() > 0 {
                                sign_form.focus_first_error(cx)?;
                                "Correct the marked fields first.".to_owned()
                            } else if sign_name.get()?.is_empty() || sign_secret.get()?.is_empty() {
                                "Both fields are required.".to_owned()
                            } else {
                                // The password's length is not shown: the
                                // field counts grapheme clusters, which
                                // `chars` would not match.
                                format!("Signed in as {}.", sign_name.get()?)
                            };
                            sign_outcome.set(text)
                        }),
                    ),
                ])
                .gap(MetricRole::ItemGap),
            ),
            Element::new(
                "sign-in-summary",
                Label::new(summary.map(|summary| {
                    format!(
                        "Errors: {}, pending checks: {}, unsaved fields: {}",
                        summary.errors(),
                        summary.pending(),
                        summary.modified()
                    )
                })?),
            ),
            Element::new("sign-in-outcome", Label::new(outcome.read_only())),
        ],
    ))
}

const SEARCH_ITEMS: [&str; 8] = [
    "Compressor",
    "Delay",
    "Equalizer",
    "Gate",
    "Limiter",
    "Phaser",
    "Reverb",
    "Saturation",
];
fn search_matches(query: &str) -> Vec<&'static str> {
    let query = query.trim().to_lowercase();
    SEARCH_ITEMS
        .into_iter()
        .filter(|item| item.to_lowercase().contains(&query))
        .collect()
}

/// The result line with each match's matched part highlighted. The items are
/// ASCII, so a lowercase match position is also the original's.
fn match_line(query: &str) -> RichText {
    let needle = query.trim().to_lowercase();
    let matches = search_matches(query);
    if matches.is_empty() {
        return RichText::from("No matches");
    }
    let mut line = String::from("Matches: ");
    let mut ranges = Vec::new();
    for (index, item) in matches.iter().enumerate() {
        if index > 0 {
            line.push_str(", ");
        }
        if !needle.is_empty()
            && let Some(at) = item.to_lowercase().find(&needle)
        {
            ranges.push(line.len() + at..line.len() + at + needle.len());
        }
        line.push_str(item);
    }
    RichText::highlighted(&line, ranges).unwrap_or_else(|_| RichText::from(line))
}

// The saved index belongs to one committed query; presentation and submission
// resolve the same first-match fallback when that query changes.
fn search_highlight(query: &str, selected: &Option<(String, usize)>) -> Option<usize> {
    let count = search_matches(query).len();
    (count > 0).then(|| {
        selected
            .as_ref()
            .filter(|(saved, _)| saved == query)
            .map_or(0, |(_, index)| (*index).min(count - 1))
    })
}

// A search field as a palette or chooser uses it: the query filters a list,
// Up/Down move a highlight while focus stays in the field, Enter chooses and
// Escape clears. Debounced, so the list filters once the typing pauses.
fn search_example(ui: &Runtime) -> Result<Element, RuntimeError> {
    let query = ui.signal(String::new());
    let highlight = ui.signal(None::<(String, usize)>);
    let chosen = ui.signal("Nothing chosen yet.".to_owned());
    let (read_query, moved, chose) = (query.clone(), highlight.clone(), chosen.clone());
    Ok(section(
        "search-example",
        "Search field",
        [
            label(
                "search-help",
                "Type to filter. Up/Down move the highlight without leaving the field, Enter chooses, Escape clears the query and a second Escape is left to the page. The list follows the query 150 ms after typing pauses.",
            ),
            Element::new(
                "search-field",
                SearchField::new(query.clone())
                    .placeholder("Search effects")
                    .debounce(std::time::Duration::from_millis(150))
                    .on_key(move |_, key| {
                        let query = read_query.get()?;
                        let count = search_matches(&query).len();
                        let current = search_highlight(&query, &moved.get()?).unwrap_or(0);
                        let next = match key {
                            SearchKey::Up => current.saturating_sub(1),
                            SearchKey::Down => (current + 1).min(count.saturating_sub(1)),
                            SearchKey::PageUp => 0,
                            SearchKey::PageDown => count.saturating_sub(1),
                            _ => current,
                        };
                        moved.set((count > 0).then(|| (query.clone(), next)))?;
                        if key == SearchKey::Submit {
                            let text = search_matches(&query)
                                .get(current)
                                .map_or("No match to choose.".to_owned(), |item| {
                                    format!("Chosen: {item}")
                                });
                            chose.set(text)?;
                        }
                        Ok(())
                    }),
            )
            .with_label("Search effects"),
            Element::new(
                "search-results",
                Label::new(query.map(|query| match_line(query))?),
            ),
            Element::new(
                "search-highlight",
                Label::new(ReadSignal::combine(
                    (&query, &highlight),
                    |(query, selected)| {
                        search_highlight(query, selected).map_or_else(
                            || "No highlighted match.".to_owned(),
                            |index| format!("Highlighted match: {}", index + 1),
                        )
                    },
                )?),
            ),
            Element::new("search-chosen", Label::new(chosen.read_only())),
        ],
    ))
}

// Every status a Field can present, around the text editors of this page. The
// first field is live: its rule is the application's and runs on Enter. The rest
// hold a fixed status so each presentation can be compared side by side.
fn field_examples(ui: &Runtime) -> Element {
    let text = |value: &str| ui.signal(value.to_owned());
    let project = text("");
    let project_status = ui.signal(FieldStatus::Normal);
    let checked = project_status.clone();
    section(
        "field-examples",
        "Fields: label, help and status around an editor",
        [
            label(
                "field-examples-help",
                "A Field adds a label above any control and a help line below it. A warning or error replaces the help line and outlines the control; the focus ring stays visible inside the outline.",
            ),
            Element::new(
                "field-live",
                Field::new(Element::new(
                    "field-live-input",
                    TextField::new(project).on_submit(move |_, value| {
                        let length = value.trim().chars().count();
                        checked.set(match length {
                            0 => FieldStatus::error("A project name is required"),
                            1..=20 => FieldStatus::Normal,
                            _ => FieldStatus::warning(
                                "Names over 20 characters are shortened in tab titles",
                            ),
                        })
                    }),
                ))
                .label("Project name — live")
                .help("Press Enter to check: empty is an error, over 20 characters a warning.")
                .status_signal(project_status.read_only()),
            ),
            Element::new(
                "field-normal",
                Field::new(Element::new(
                    "field-normal-input",
                    TextField::new(text("Ada Lovelace")),
                ))
                .label("Normal")
                .help("Help text stays visible in Muted while nothing is reported."),
            ),
            Element::new(
                "field-warning",
                Field::new(Element::new(
                    "field-warning-input",
                    TextField::new(text("untitled")),
                ))
                .label("Warning")
                .help("This help text is replaced while the warning is set.")
                .status(FieldStatus::warning(
                    "Accepted, but another project already uses this name",
                )),
            ),
            Element::new(
                "field-error",
                Field::new(Element::new(
                    "field-error-input",
                    TextField::new(text("")),
                ))
                .label("Error")
                .help("This help text is replaced while the error is set.")
                .status(FieldStatus::error("A value is required")),
            ),
            Element::new(
                "field-marked",
                Field::new(Element::new(
                    "field-marked-input",
                    TextField::new(text("reported elsewhere")),
                ))
                .label("Error with an empty message")
                .help("An empty message outlines the control and keeps the help line.")
                .status(FieldStatus::error("")),
            ),
            Element::new(
                "field-area",
                Field::new(Element::new(
                    "field-area-input",
                    TextArea::new(text(
                        "A multiline editor takes the same label, help and status.\nA long message wraps to the field's width instead of widening the form.",
                    )),
                ))
                .label("Multiline with a long warning")
                .status(FieldStatus::warning(
                    "This message is deliberately long, to show that a status wraps to the width of its field and lengthens it rather than pushing the surrounding form wider than its container.",
                )),
            ),
            Element::new(
                "field-read-only",
                Field::new(Element::new(
                    "field-read-only-input",
                    TextField::new(text("Generated identifier")).read_only(),
                ))
                .label("Read-only control")
                .help("The field never changes its control's own read-only or disabled state."),
            ),
            Element::new(
                "field-unlabelled",
                Field::new(Element::new(
                    "field-unlabelled-input",
                    TextField::new(text("No label, status only")),
                ).with_label("Status-only field"))
                .status(FieldStatus::error("A label is optional too")),
            ),
        ],
    )
}

/// Labels that keep one line with an ellipsis, in columns of three widths.
fn truncation_example() -> Result<Element, GeometryError> {
    const PATH: &str = "/home/user/projects/mix/stems/final-master.wav";
    const TITLE: &str = "Automation lane for the reverb send on the drum bus";
    let column = |key: &str, width: f32| -> Result<Element, GeometryError> {
        Ok(Element::new(
            key.to_owned(),
            Container::new(Element::new(
                "labels",
                Column::new([
                    Element::new(
                        format!("{key}-end"),
                        Label::new(TITLE).overflow(TextOverflow::Ellipsis),
                    ),
                    Element::new(
                        format!("{key}-middle"),
                        Label::new(PATH).overflow(TextOverflow::MiddleEllipsis),
                    ),
                ]),
            ))
            .border_role(ColorRole::PanelBorder)
            .padding(MetricRole::ControlPaddingX)
            .limits(Size::new(0.0, 0.0)?, Some(Size::new(width, f32::MAX)?))?,
        ))
    };
    Ok(section(
        "truncation",
        "Truncation",
        [
            label(
                "truncation-help",
                "Each box keeps its labels on one line: the first ends in an ellipsis, the second keeps both ends of a path. Rest the pointer on a cut label for its full text; inspection names each label with its full text.",
            ),
            Element::new(
                "truncation-columns",
                Wrap::new([
                    column("narrow", 140.0)?,
                    column("medium", 220.0)?,
                    column("wide", 420.0)?,
                ]),
            ),
        ],
    ))
}

pub(crate) fn fonts_and_icons(ui: &Runtime) -> Result<Element, AppError> {
    let fonts = ui.fonts();
    let alternate = fonts.register(FontData::bundled(FontRole::Monospace))?;
    let alternate_icon = fonts.material_icon("expand_circle_down")?;
    let status = ui.signal("Ready".to_owned());
    let rich = RichText::new()
        .icon(Icon::Settings)
        .gap()
        .text("Settings ")
        .span(
            TextSpan::new("42.00")
                .font(alternate.clone())
                .color_role(ColorRole::Warning),
        );
    let mut examples = Vec::new();
    // Every icon the library has, so a new one is shown without listing it.
    for &icon in Icon::ALL {
        let name = format!("{icon:?}");
        let output = status.clone();
        let message = name.clone();
        examples.push(Element::new(
            name.clone(),
            Tooltip::new(
                Element::new(
                    "icon",
                    Button::icon(icon, name.clone(), move |_| {
                        output.set(format!("{message} activated"))
                    }),
                ),
                name,
            ),
        ));
    }
    let mut trailing_buttons = Vec::new();
    for (key, title, icon, role) in [
        ("save-right", "Save changes", Icon::Save, ColorRole::Success),
        (
            "download-right",
            "Download",
            Icon::Download,
            ColorRole::Primary,
        ),
        (
            "review-right",
            "Review warnings",
            Icon::Warning,
            ColorRole::Warning,
        ),
        (
            "delete-right",
            "Delete item",
            Icon::Delete,
            ColorRole::Error,
        ),
        (
            "search-right",
            "Search library",
            Icon::Search,
            ColorRole::Primary,
        ),
        (
            "continue-right",
            "Continue",
            Icon::ChevronRight,
            ColorRole::Success,
        ),
    ] {
        let output = status.clone();
        // Semantic color stays in the icon; labels on neutral buttons keep OnButton.
        let content = RichText::new()
            .span(TextSpan::new(title))
            .gap()
            .tinted_icon(icon, TextColor::Role(role));
        trailing_buttons.push(Element::new(
            key,
            Button::new(content, move |_| output.set(format!("{title} activated"))),
        ));
    }
    let inline_font = alternate.clone();
    let font_manager = fonts.clone();
    let reset_fonts = fonts.clone();
    let icon_manager = ui.icons();
    let reset_icons = ui.icons();
    Ok(page([
        truncation_example()?,
        section(
            "rich-labels",
            "Mixed fonts and colors",
            [
                Element::new("mixed", Label::new(rich.clone())),
                Element::new("rich-button", Button::new(rich, |_| Ok(()))),
                Element::new(
                    "font-actions",
                    Wrap::new([
                        Element::new(
                            "alternate-font",
                            Button::new("Use alternate font", move |_| {
                                font_manager.set_default(FontRole::Body, alternate.clone())
                            })
                            .leading_icon(Icon::Settings),
                        ),
                        Element::new(
                            "default-font",
                            Button::new("Restore default font", move |_| {
                                reset_fonts.reset_default(FontRole::Body)
                            })
                            .leading_icon(Icon::Refresh),
                        ),
                    ]),
                ),
            ],
        ),
        section(
            "inline-icon-examples",
            "Icons within text",
            [
                Element::new(
                    "inline-workflow",
                    Label::new(
                        RichText::from("Find a file")
                            .gap()
                            .tinted_icon(Icon::Search, TextColor::Role(ColorRole::Primary))
                            .gap()
                            .text("then save")
                            .gap()
                            .tinted_icon(Icon::Save, TextColor::Role(ColorRole::Success))
                            .gap()
                            .text("or download a copy")
                            .gap()
                            .icon(Icon::Download)
                            .text("."),
                    ),
                ),
                Element::new(
                    "inline-status",
                    Label::new(
                        RichText::from("Connection")
                            .gap()
                            .tinted_icon(Icon::Link, TextColor::Role(ColorRole::Primary))
                            .gap()
                            .span(TextSpan::new("online").color_role(ColorRole::Success))
                            .text(" · Checks")
                            .gap()
                            .tinted_icon(Icon::Check, TextColor::Role(ColorRole::Success))
                            .gap()
                            .text("passed · Updates")
                            .gap()
                            .tinted_icon(Icon::Refresh, TextColor::Role(ColorRole::Warning))
                            .gap()
                            .text("pending"),
                    ),
                ),
                Element::new(
                    "inline-feedback",
                    Label::new(
                        RichText::from("One warning")
                            .gap()
                            .tinted_icon(Icon::Warning, TextColor::Role(ColorRole::Warning))
                            .gap()
                            .text("needs attention; an error")
                            .gap()
                            .tinted_icon(Icon::Error, TextColor::Role(ColorRole::Error))
                            .gap()
                            .text("can be opened for details")
                            .gap()
                            .tinted_icon(Icon::Info, TextColor::Role(ColorRole::Primary))
                            .text("."),
                    ),
                ),
                Element::new(
                    "inline-mixed-size",
                    Label::new(
                        RichText::new()
                            .span(TextSpan::new("Workspace").size(Length::new(20.0)?))
                            .gap()
                            .tinted_icon(Icon::Settings, TextColor::Role(ColorRole::Primary))
                            .gap()
                            .text("contains ")
                            .span(
                                TextSpan::new("24")
                                    .font(inline_font)
                                    .color_role(ColorRole::Warning),
                            )
                            .text(" saved items")
                            .gap()
                            .tinted_icon(Icon::Save, TextColor::Role(ColorRole::Success))
                            .gap()
                            .text("ready to share")
                            .gap()
                            .icon(Icon::Link)
                            .text("."),
                    ),
                ),
            ],
        ),
        section(
            "trailing-icon-buttons",
            "Buttons with icons on the right",
            [Element::new("colored-actions", Wrap::new(trailing_buttons))],
        ),
        section(
            "shared-icons",
            "Shared icons across controls",
            [
                Element::new(
                    "select",
                    Select::new(
                        ui,
                        ui.signal(0),
                        [
                            Choice::new(0, "Settings").leading_icon(Icon::Settings),
                            Choice::new(1, "Saved items").leading_icon(Icon::Save),
                        ],
                    )?,
                )
                .with_label("Shared icon choice"),
                Element::new(
                    "menu",
                    Menu::new(
                        ui,
                        RichText::from("Actions").leading_icon(Icon::MoreHorizontal),
                        vec![
                            MenuItem::new("save", "Save", |_| Ok(())).leading_icon(Icon::Save),
                            MenuItem::submenu(
                                "more",
                                "More",
                                vec![
                                    MenuItem::new("info", "Information", |_| Ok(()))
                                        .leading_icon(Icon::Info),
                                ],
                            ),
                        ],
                    )?,
                ),
                Element::new(
                    "icon-actions",
                    Wrap::new([
                        Element::new(
                            "alternate-icon",
                            Button::new("Change expand icon", move |_| {
                                icon_manager.set(Icon::ChevronDown, alternate_icon.clone())
                            })
                            .leading_icon(Icon::ChevronDown),
                        ),
                        Element::new(
                            "default-icons",
                            Button::new("Restore icons", move |_| reset_icons.set_overrides([]))
                                .leading_icon(Icon::Refresh),
                        ),
                    ]),
                ),
            ],
        ),
        section(
            "icon-catalog",
            "Icon buttons",
            [
                Element::new("icons", Wrap::new(examples)),
                Element::new(
                    "icon-status",
                    Label::new(status.read_only()).leading_icon(Icon::Info),
                ),
            ],
        ),
        section(
            "panel-header",
            "Panel header composition",
            [
                Element::new(
                    "header",
                    RegionLayout::new()
                        .center(Element::new(
                            "title",
                            Label::new("Output 1").leading_icon(Icon::Power),
                        ))
                        .right(Element::new(
                            "actions",
                            Row::new([
                                Element::new(
                                    "settings",
                                    Button::icon(Icon::Settings, "Panel settings", |_| Ok(())),
                                ),
                                Element::new(
                                    "more",
                                    Button::icon(Icon::MoreHorizontal, "Panel actions", |_| Ok(())),
                                ),
                                Element::new(
                                    "close",
                                    Button::icon(Icon::Close, "Close panel", |_| Ok(())),
                                ),
                            ]),
                        )),
                ),
                label("content", "Ordinary content can follow the header."),
                Element::new(
                    "footer",
                    Row::new([
                        Element::new(
                            "download",
                            Button::new("Export", |_| Ok(())).leading_icon(Icon::Download),
                        ),
                        Element::new(
                            "link",
                            Button::new("Link", |_| Ok(())).leading_icon(Icon::Link),
                        ),
                    ]),
                ),
            ],
        ),
    ]))
}
