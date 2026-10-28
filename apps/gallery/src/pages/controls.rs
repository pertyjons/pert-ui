//! The Controls category: text and buttons, standard controls, parameters,
//! the toolbar, field status, adaptive forms, progress and colors.
use crate::gallery::{action, counter, label, page, section, stage};
use crate::samples::{Canvas, KeyPad};
use pert_ui::IntervalTimer;
use pert_ui::parameter::*;
use pert_ui::prelude::*;
use pert_ui::windowing::*;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

fn inline_section(ui: &mut Runtime, id: ParameterId) -> Result<Element, AppError> {
    let count = ui.signal(8_i64);
    let time = ui.signal(0.0009_f64);
    let (glide, host) = ui.parameter(id, ParameterSpec::new("Glide", 0.0, 2.0, 0.9)?)?;
    let feedback = ui.signal(String::from("Reset restores all three controls."));
    let reset_feedback = feedback.clone();
    let count_reset = count.clone();
    let time_reset = time.clone();
    let milliseconds = NumberFormat::new(
        "9999.00 ms",
        |value: f64| format!("{:.2}", value * 1000.0),
        |text| {
            text.trim()
                .replace(',', ".")
                .parse::<f64>()
                .map(|value| value / 1000.0)
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    )
    .edit_text(|value| (value * 1000.0).to_string());
    let inline = section(
        "inline-numbers",
        "Inline controls",
        [
            label(
                "inline-help",
                "Drag a value to adjust it; click it or press Enter/F2 to type. Arrows/wheel adjust the focused control, Shift adjusts finely, Escape cancels.",
            ),
            Element::new(
                "inline-top-row",
                Wrap::new([
                    Element::new(
                        "inline-glide",
                        InlineSlider::new(glide.clone()).label("Glide").unit("s"),
                    ),
                    Element::new(
                        "inline-volume",
                        InlineInteger::new(count.clone())
                            .range(0, 100)?
                            .label("Vol")
                            .unit("%"),
                    ),
                    Element::new(
                        "inline-time",
                        InlineDecimal::new(time.clone())
                            .range(0.0, 2.0)?
                            .step(0.0001)?
                            .label("Delay")
                            .unit("ms")
                            .format(milliseconds),
                    ),
                    Element::new(
                        "inline-reset",
                        Button::new("Reset", move |_| {
                            count_reset.set(8)?;
                            time_reset.set(0.0009)?;
                            let result = host.write_base(0.9);
                            reset_feedback.set(format!("Parameter reset: {result:?}"))?;
                            Ok(())
                        }),
                    ),
                ]),
            ),
            Element::new("inline-feedback", Label::new(feedback.map(Clone::clone)?)),
            Element::new(
                "inline-count-saved",
                Label::new(count.map(|value| format!("Saved integer: {value}"))?),
            ),
            Element::new(
                "inline-time-saved",
                Label::new(time.map(|value| format!("Saved canonical time: {value} seconds"))?),
            ),
            Element::new(
                "inline-glide-saved",
                Label::new(glide.snapshot().map(|value| {
                    format!("Glide: base {}, preview {:?}", value.base, value.preview)
                })?),
            ),
            Element::new(
                "inline-readonly",
                InlineInteger::new(count)
                    .range(0, 100)?
                    .label("Read-only")
                    .unit("%")
                    .read_only(),
            ),
        ],
    );
    Ok(inline)
}
pub(crate) fn toolbar_page(
    ui: &mut Runtime,
    glide: ParameterId,
    strip_ids: [ParameterId; 3],
) -> Result<Element, AppError> {
    let playing = ui.signal(false);
    let armed = ui.signal(false);
    let looping = ui.signal(true);
    let metronome = ui.signal(false);
    let tempo = ui.signal(120_i64);
    let snap = ui.signal("1/16");
    let tool = ui.signal("select");
    let erasing = ui.signal(false);
    let name = ui.signal(String::from("Lead synth"));
    let position = ui.signal(String::from("001:01:000"));
    let load = ui.signal(0.18_f64);
    let status = ui.signal(String::from(
        "Toolbar actions are synthetic; nothing plays.",
    ));
    let percent = || {
        NumberFormat::new(
            "100",
            |v: f64| format!("{:.0}", v * 100.0),
            |_| Err(NumberInputError::InvalidNumber),
        )
        .unit(|_| "%".into())
    };
    // Stop is a registered command: its icon button grays out with the
    // command and its tooltip shows the shortcut.
    let stop = {
        let stop_status = status.clone();
        let stop_state = playing.clone();
        let stop_position = position.clone();
        ui.register_command(
            "Stop",
            CommandScope::Application,
            vec![KeyChord::named(KeyCode::F7)],
            move |_, _| {
                stop_state.set(false)?;
                stop_position.set("001:01:000".into())?;
                stop_status.set("Stop pressed (synthetic).".into())?;
                Ok(())
            },
        )?
    };
    ui.command_enabled_signal(&stop, playing.read_only())?;
    let transport = {
        let play_status = status.clone();
        let play_state = playing.clone();
        ToolGroup::new(
            "transport",
            [
                Element::new(
                    "play",
                    Button::icon(Icon::PlayArrow, "Play", move |_| {
                        play_state.set(true)?;
                        play_status.set("Play pressed (synthetic).".into())?;
                        Ok(())
                    })
                    .frameless(),
                ),
                Element::new(
                    "stop",
                    Tooltip::command(
                        ui,
                        Element::new(
                            "button",
                            Button::command_icon(ui, &stop, Icon::Stop, |_, _| Ok(()))?.frameless(),
                        ),
                        &stop,
                    )?,
                ),
                Element::new(
                    "record",
                    InlineToggle::new(armed.clone(), "Record arm").icon(Icon::Record),
                ),
                Element::new(
                    "loop",
                    InlineToggle::new(looping.clone(), "Loop").icon(Icon::Repeat),
                ),
                Element::new(
                    "metronome",
                    InlineToggle::new(metronome.clone(), "Metronome").icon(Icon::Timer),
                ),
            ],
        )
        .essential()
    };
    let tempo_group = ToolGroup::new(
        "tempo",
        [
            Element::new(
                "bpm",
                InlineInteger::new(tempo.clone())
                    .range(20, 300)?
                    .label("Tempo")
                    .unit("bpm")
                    .stepper(true),
            ),
            Element::new(
                "snap",
                InlineSelect::new(
                    ui,
                    snap.clone(),
                    [
                        Choice::new("bar", "Bar"),
                        Choice::new("1/4", "1/4"),
                        Choice::new("1/8", "1/8"),
                        Choice::new("1/16", "1/16"),
                        Choice::new("off", "Off"),
                    ],
                )?
                .label("Snap"),
            ),
        ],
    );
    let tools = {
        let erasing = erasing.clone();
        ToolGroup::new(
            "tools",
            [Element::new(
                "mode",
                InlineChoice::new(
                    tool.clone(),
                    [
                        Choice::icon("select", Icon::Edit, "Select"),
                        Choice::icon("erase", Icon::Delete, "Erase"),
                        Choice::icon("cut", Icon::Remove, "Split"),
                    ],
                )?
                .on_change(move |_, value| erasing.set(*value == "erase")),
            )
            .with_label("Tool")],
        )
    };
    let erase_options = {
        let whole = ui.signal(false);
        ToolGroup::new(
            "erase-options",
            [Element::new(
                "whole-notes",
                InlineToggle::new(whole, "Erase whole notes").glyph("W"),
            )],
        )
        .visible_signal(erasing.read_only())
    };
    let session = ToolGroup::new(
        "session",
        [
            Element::new(
                "name",
                InlineText::new(name.clone()).placeholder("Untitled"),
            )
            .with_label("Session name"),
            Element::new(
                "position",
                InlineReadout::new(position.read_only())
                    .label("Pos")
                    .width_hint("000:00:000"),
            ),
            Element::new(
                "load",
                InlineReadout::formatted(load.read_only(), percent()).label("CPU"),
            ),
        ],
    )
    .collapse_first();
    let toolbar = Toolbar::new(
        ui,
        [
            transport,
            ToolGroup::separator("transport-divider"),
            tempo_group,
            tools,
            erase_options,
            ToolGroup::spacer("spacer"),
            session,
        ],
    )?;
    let status_bar = Toolbar::new(
        ui,
        [
            ToolGroup::new(
                "status-text",
                [Element::new("message", Label::new(status.read_only()))],
            )
            .essential(),
            ToolGroup::spacer("status-spacer"),
            ToolGroup::new(
                "status-readouts",
                [
                    Element::new(
                        "position",
                        InlineReadout::new(position.read_only())
                            .label("Pos")
                            .width_hint("000:00:000"),
                    ),
                    Element::new(
                        "load",
                        InlineReadout::formatted(load.read_only(), percent()).label("CPU"),
                    ),
                ],
            ),
        ],
    )?
    .attached_below();
    let strip = channel_strip(ui, strip_ids)?;
    let bump_load = load.clone();
    let bump_position = position.clone();
    Ok(page([
        label(
            "toolbar-help",
            "A global toolbar: icon actions, latched toggles, a stepped tempo, a snap select, an icon choice with a contextual group that appears only for the erase tool, an editable name and read-only readouts. Compare the smaller muted captions with the full-size values, units and action text. Narrow the window and groups move behind the More button; widen it and they return. Change density above: the strip scales with every other control.",
        ),
        Element::new("global-toolbar", toolbar),
        Element::new(
            "toolbar-mirror",
            Wrap::new([
                Element::new(
                    "playing-mirror",
                    Label::new(playing.map(|v| format!("Playing: {v}"))?),
                ),
                Element::new(
                    "armed-mirror",
                    Label::new(armed.map(|v| format!("Armed: {v}"))?),
                ),
                Element::new(
                    "tempo-mirror",
                    Label::new(tempo.map(|v| format!("Tempo: {v} bpm"))?),
                ),
                Element::new(
                    "snap-mirror",
                    Label::new(snap.map(|v| format!("Snap: {v}"))?),
                ),
                Element::new(
                    "tool-mirror",
                    Label::new(tool.map(|v| format!("Tool: {v}"))?),
                ),
                Element::new(
                    "name-mirror",
                    Label::new(name.map(|v| format!("Name: {v}"))?),
                ),
                Element::new(
                    "advance",
                    Button::new("Advance readouts", move |_| {
                        bump_position.set("017:03:120".into())?;
                        bump_load.set(0.63)?;
                        Ok(())
                    }),
                ),
            ]),
        ),
        inline_section(ui, glide)?,
        strip,
        section(
            "status-bar-section",
            "Status bar",
            [
                label(
                    "status-help",
                    "A toolbar attached below its content draws its border on the top edge.",
                ),
                Element::new("status-bar", status_bar),
            ],
        ),
    ]))
}

fn channel_strip(ui: &mut Runtime, ids: [ParameterId; 3]) -> Result<Element, AppError> {
    let [pan_id, send_id, level_id] = ids;
    let (pan, _pan_host) = ui.parameter(pan_id, ParameterSpec::new("Pan", -1.0, 1.0, 0.0)?)?;
    let (send, _send_host) = ui.parameter(send_id, ParameterSpec::new("Send", 0.0, 1.0, 0.25)?)?;
    let (level, _level_host) =
        ui.parameter(level_id, ParameterSpec::new("Level", 0.0, 1.0, 0.8)?)?;
    let muted = ui.signal(false);
    let solo = ui.signal(false);
    // Decibel presentation is supplied by the application, not the library.
    let decibels = NumberFormat::new(
        "-inf dB",
        |gain: f32| {
            if gain <= 0.0 {
                "-inf".into()
            } else {
                format!("{:.1}", 20.0 * gain.log10())
            }
        },
        |text| {
            let text = text.trim();
            if text.eq_ignore_ascii_case("-inf") {
                return Ok(0.0);
            }
            text.replace(',', ".")
                .parse::<f32>()
                .map(|db| 10f32.powf(db / 20.0))
                .map_err(|_| NumberInputError::InvalidNumber)
        },
    );
    Ok(section(
        "channel-strip",
        "Channel strip",
        [
            label(
                "strip-help",
                "Inline sliders for pan and send, inline toggles for mute and solo, and an inline level slider with a decibel format supplied by the gallery.",
            ),
            Element::new(
                "strip-row",
                Wrap::new([
                    Element::new("pan", InlineSlider::new(pan).label("Pan")),
                    Element::new("send", InlineSlider::new(send).label("Send")),
                    Element::new("mute", InlineToggle::new(muted.clone(), "Mute").glyph("M")),
                    Element::new("solo", InlineToggle::new(solo.clone(), "Solo").glyph("S")),
                    Element::new(
                        "level",
                        InlineSlider::new(level).label("Level").format(decibels),
                    ),
                ]),
            ),
            Element::new(
                "strip-mirror",
                Label::new(muted.map(|v| format!("Muted: {v}"))?),
            ),
        ],
    ))
}

pub(crate) fn typography(ui: &Runtime, status: &Signal<String>) -> Result<Element, AppError> {
    let message = ui.signal("This label follows a signal.".to_owned());
    Ok(page([
        section(
            "typography",
            "Bundled typography",
            [
                Element::new(
                    "heading",
                    Label::new("Heading — Noto Sans").style(TextStyle::Heading),
                ),
                label("body", "Body — Noto Sans. Åäö, café, Ελληνικά, Кириллица."),
                Element::new(
                    "caption",
                    Label::new("Caption — supporting text").style(TextStyle::Caption),
                ),
                Element::new(
                    "mono",
                    Label::new("Monospace — Noto Sans Mono: 0123456789")
                        .style(TextStyle::Monospace),
                ),
                label(
                    "wrapping",
                    "Long labels wrap automatically. Resize this window to see text reflow while controls preserve their state.",
                ),
                Element::new(
                    "table",
                    Grid::new(
                        [
                            Track::CONTENT,
                            // Narrow enough to stay inside the section at the
                            // narrowest width the page sweep lays out.
                            Track::fixed(72.into()),
                            Track::fixed(88.into()),
                        ],
                        [
                            "Item", "Count", "Value", "Input A", "12", "0.25", "Input B", "8",
                            "-1.50", "Total", "20", "42.00",
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(i, text)| {
                            Element::new(
                                format!("cell-{i}"),
                                Label::new(text).style(TextStyle::Monospace),
                            )
                        }),
                    )?,
                ),
            ],
        ),
        section(
            "buttons",
            "Buttons and bound labels",
            [
                label(
                    "help",
                    "Click or use Tab and Enter/Space. Press then drag outside, or press Escape, to cancel. Disabled buttons are skipped.",
                ),
                Element::new(
                    "actions",
                    Wrap::new([
                        action("standard", "Standard action", status),
                        Element::new("disabled", Button::new("Disabled", |_| Ok(())).disabled()),
                        Element::new(
                            "change",
                            Button::new("Change bound text", {
                                let message = message.clone();
                                move |_| {
                                    message
                                        .set("The signal changed; this label refreshed.".to_owned())
                                }
                            }),
                        ),
                    ]),
                ),
                Element::new("bound", Label::new(message.read_only())),
                counter(ui, "counter")?,
            ],
        ),
        section(
            "drawing",
            "Custom drawing through Painter",
            [
                label(
                    "help",
                    "Solid fill, a clipped rectangle and a tinted coverage mask. This static example uses the public custom-component API.",
                ),
                Element::new("canvas", Canvas::new()?),
            ],
        ),
    ]))
}
/// The application side of the parameter-families page: host owners, their
/// delivery monitors and an undo stack fed only by terminal notifications.
struct FamilyHost {
    hosts: HashMap<ParameterId, AnyParameterHost>,
    _monitors: Vec<AnyDeliveryMonitor>,
    undo: Rc<RefCell<Vec<(ParameterId, AnyValue)>>>,
}

/// Four parameter families created from host descriptors, each bound to its
/// standard and inline controls. A synthetic UI-thread host records an undo
/// entry for every committed edit from its terminal sink; "Undo" writes the
/// recorded value back as an authoritative host write.
pub(crate) fn parameter_families(
    ui: &mut Runtime,
    status: &Signal<String>,
) -> Result<Element, AppError> {
    Ok(family_page(ui, status)?)
}
fn family_page(ui: &mut Runtime, status: &Signal<String>) -> Result<Element, RuntimeError> {
    let choice = |id: &str, label: &str| -> Result<ChoiceOption, ParameterError> {
        Ok(ChoiceOption::new(ChoiceId::new(id)?, label))
    };
    let descriptors = [
        (
            ParameterId::new(3, 1, "cutoff")?,
            AnySpec::from(
                ParameterSpec::new("Cutoff", 20.0, 20_000.0, 1_000.0)?
                    .with_mapping(ParameterMapping::Logarithmic)?,
            ),
        ),
        (
            // A power mapping gives the low end of a range that includes zero
            // more travel, which a logarithmic mapping cannot.
            ParameterId::new(3, 6, "drive")?,
            AnySpec::from(
                ParameterSpec::new("Drive", 0.0, 10.0, 1.0)?
                    .with_mapping(ParameterMapping::Power(PowerExponent::new(2.0)?))?,
            ),
        ),
        (
            ParameterId::new(3, 2, "voices")?,
            AnySpec::from(ParameterSpec::stepped("Voices", 1, 16, 8)?),
        ),
        (
            ParameterId::new(3, 3, "bypass")?,
            AnySpec::from(ParameterSpec::boolean("Bypass", false)?),
        ),
        (
            ParameterId::new(3, 4, "waveform")?,
            AnySpec::from(ParameterSpec::choices(
                "Waveform",
                [
                    choice("sine", "Sine")?,
                    choice("saw", "Saw")?,
                    choice("square", "Square")?,
                ],
                ChoiceId::new("sine")?,
            )?),
        ),
    ];
    let undo = Rc::new(RefCell::new(Vec::new()));
    let mut hosts = HashMap::new();
    let mut monitors = Vec::new();
    let mut rows = Vec::new();
    for (id, spec) in descriptors {
        let (parameter, host) = ui.parameter_any(id.clone(), spec)?;
        let stack = undo.clone();
        let parameter_id = id.clone();
        monitors.push(host.on_outcome(move |event| {
            // Record application undo for commits only, keyed by the parameter.
            if let EditOutcome::Committed { .. } = event.outcome() {
                stack
                    .borrow_mut()
                    .push((parameter_id.clone(), event.begin_value()));
            }
            Ok(())
        })?);
        rows.push(match parameter {
            AnyParameter::Continuous(parameter)
                if matches!(parameter.spec()?.mapping(), ParameterMapping::Power(_)) =>
            {
                section(
                    "power",
                    "Continuous (power)",
                    [
                        Element::new("slider", Slider::new(parameter.clone()).show_value(true)),
                        Element::new("inline", InlineSlider::new(parameter).label("Drive")),
                    ],
                )
            }
            AnyParameter::Continuous(parameter) => section(
                "continuous",
                "Continuous (logarithmic)",
                [
                    Element::new(
                        "slider",
                        Slider::new(parameter.clone()).show_value(true).unit("Hz"),
                    ),
                    Element::new(
                        "inline",
                        InlineSlider::new(parameter).label("Cutoff").unit("Hz"),
                    ),
                ],
            ),
            AnyParameter::Stepped(parameter) => section(
                "stepped",
                "Stepped",
                [
                    Element::new("field", IntegerInput::for_parameter(parameter.clone())),
                    Element::new(
                        "inline",
                        InlineInteger::for_parameter(parameter)
                            .label("Voices")
                            .stepper(true),
                    ),
                ],
            ),
            AnyParameter::Boolean(parameter) => section(
                "boolean",
                "Boolean",
                [
                    Element::new("toggle", Toggle::for_parameter(parameter.clone())),
                    Element::new("checkbox", Checkbox::for_parameter(parameter.clone())),
                    Element::new("inline", InlineToggle::for_parameter(parameter)),
                ],
            ),
            AnyParameter::Choice(parameter) => section(
                "choice",
                "Choice",
                [
                    Element::new(
                        "segments",
                        SegmentedControl::for_parameter(parameter.clone()),
                    ),
                    Element::new("select", Select::for_parameter(ui, parameter.clone()))
                        .with_label("Choice"),
                    Element::new("inline", InlineChoice::for_parameter(parameter)),
                ],
            ),
        });
        hosts.insert(id, host);
    }
    let family = FamilyHost {
        hosts,
        _monitors: monitors,
        undo,
    };
    let status = status.clone();
    rows.push(Element::new(
        "undo",
        Button::new("Undo last edit", move |_| {
            let last = family.undo.borrow_mut().pop();
            let message = match last {
                None => "Nothing to undo.".to_owned(),
                Some((id, value)) => match family.hosts.get(&id) {
                    Some(host) => match host.write_base(value.clone()) {
                        Ok(()) => format!("Undo restored {value:?}."),
                        Err(error) => format!("Undo rejected: {error}"),
                    },
                    None => format!("Undo entry names no parameter: {id:?}."),
                },
            };
            status.set(message)
        }),
    ));
    Ok(page(rows))
}

const KEY_PAD_HELP: &str = "This custom input area shows held physical keys, modifiers and repeat/release classification. Moving focus cancels its local input context.";
const SHARED_PARAMETER_HELP: &str = "The sliders, knobs and button editors share one -100 to 100 parameter. Drag (vertically on a knob), use arrows or scroll over the focused control; Shift adjusts finely, Escape cancels, and Backspace or a double-click on a knob restores the default. The second knob fills its arc from the default instead of the minimum; the third formats its readout with an application-supplied number format. External host updates take precedence.";
pub(crate) fn input(
    ui: &mut Runtime,
    id: ParameterId,
    spec: ParameterSpec<f32>,
    status: &Signal<String>,
) -> Result<Element, AppError> {
    let editable_spec = spec.clone();
    let (parameter, host) = ui.parameter(id, spec)?;
    let host = Rc::new(host);
    let mut editors = Vec::new();
    for editor in 0..2 {
        let mut controls = Vec::new();
        for (key, title, delta) in [
            ("decrease", "Decrease", -5.0),
            ("increase", "Increase", 5.0),
        ] {
            let parameter = parameter.clone();
            let status = status.clone();
            controls.push(Element::new(
                key,
                Button::new(title, move |cx| {
                    let result = (|| -> Result<EditOutcome<f32>, RuntimeError> {
                        let snapshot = parameter.snapshot().get()?;
                        let edit = parameter.begin_edit(cx, snapshot.revision)?;
                        let spec = parameter.spec()?;
                        let value = (snapshot.base + delta).clamp(spec.minimum(), spec.maximum());
                        edit.update(value)?;
                        edit.commit(value)
                    })();
                    status.set(match result {
                        Ok(outcome) => format!("{outcome:?}"),
                        Err(error) => format!("Edit rejected: {error}"),
                    })
                }),
            ));
        }
        controls.push(Element::new(
            "readout",
            Label::new(parameter.snapshot().map(|value| {
                format!(
                    "Base: {:7.2}  Preview: {:?}  Effective: {:7.2}",
                    value.base, value.preview, value.effective
                )
            })?)
            .style(TextStyle::Monospace),
        ));
        editors.push(section(
            &format!("editor-{editor}"),
            &format!("Shared parameter editor {}", editor + 1),
            [Element::new("controls", Wrap::new(controls))],
        ));
    }
    let preview = parameter.clone();
    let preview_status = status.clone();
    editors.push(Element::new(
        "cancel-preview",
        Button::new("Preview +20, then cancel", move |cx| {
            let result = (|| -> Result<EditOutcome<f32>, RuntimeError> {
                let current = preview.snapshot().get()?;
                let edit = preview.begin_edit(cx, current.revision)?;
                let spec = preview.spec()?;
                edit.update((current.base + 20.0).clamp(spec.minimum(), spec.maximum()))?;
                edit.cancel()
            })();
            preview_status.set(format!("Preview/cancel result: {result:?}"))
        }),
    ));
    let overlay = ui.signal(false);
    let readonly = ui.signal(false);
    let overlay_writer = overlay.clone();
    let overlay_host = host.clone();
    let overlay_status = status.clone();
    editors.push(Element::new(
        "toggle-effective",
        Button::new("Toggle effective overlay at 25", move |_| {
            let next = !overlay_writer.get()?;
            let result = overlay_host.set_effective(next.then_some(25.0));
            if result.is_ok() {
                overlay_writer.set(next)?;
            }
            overlay_status.set(format!("Effective overlay: {result:?}"))
        }),
    ));
    editors.push(Element::new(
        "effective-status",
        Label::new(overlay.map(|value| {
            format!(
                "Effective overlay: {}",
                if *value {
                    "25 (base stays independent)"
                } else {
                    "off"
                }
            )
        })?),
    ));
    let readonly_writer = readonly.clone();
    let readonly_host = host.clone();
    let readonly_status = status.clone();
    editors.push(Element::new(
        "toggle-readonly",
        Button::new("Toggle parameter read-only", move |_| {
            let next = !readonly_writer.get()?;
            let spec = if next {
                editable_spec.clone().read_only()
            } else {
                editable_spec.clone()
            };
            let result = readonly_host.replace_spec(spec);
            if result.is_ok() {
                readonly_writer.set(next)?;
            }
            readonly_status.set(format!("Parameter metadata: {result:?}"))
        }),
    ));
    editors.push(Element::new(
        "readonly-status",
        Label::new(
            readonly
                .map(|value| format!("Read-only: {value}. Rejected edits appear in the footer."))?,
        ),
    ));
    editors.push(Element::new(
        "slider",
        Slider::new(parameter.clone()).show_value(true).unit("%"),
    ));
    editors.push(Element::new(
        "vertical-slider",
        Slider::new(parameter.clone())
            .orientation(Orientation::Vertical)
            .show_value(true)
            .unit("%"),
    ));
    editors.push(Element::new(
        "knobs",
        Row::new([
            Element::new(
                "knob",
                Knob::new(parameter.clone()).show_value(true).unit("%"),
            ),
            Element::new(
                "knob-centered",
                Knob::new(parameter.clone())
                    .arc_origin(ArcOrigin::Default)
                    .show_value(true)
                    .unit("%"),
            ),
            // A readout formatted by the application: one decimal, an
            // explicit sign and a unit, in a slot reserved by the width hint.
            Element::new(
                "knob-formatted",
                Knob::new(parameter).show_value(true).format(
                    NumberFormat::new(
                        "+100.0 %",
                        |value: f32| format!("{value:+.1}"),
                        |text| {
                            text.trim()
                                .parse()
                                .map_err(|_| NumberInputError::InvalidNumber)
                        },
                    )
                    .unit(|_| "%".into()),
                ),
            ),
        ]),
    ));
    let reset_status = status.clone();
    editors.push(Element::new(
        "host-reset",
        Button::new("Host reset to zero", move |_| {
            let result = host.write_base(0.0);
            reset_status.set(format!("Host reset: {result:?}"))
        }),
    ));
    Ok(page([
        section(
            "keyboard",
            "Multi-key input",
            [
                label("help", KEY_PAD_HELP),
                Element::new("key-pad", KeyPad::default()),
            ],
        ),
        section(
            "parameters",
            "One parameter, multiple editing views",
            std::iter::once(label("help", SHARED_PARAMETER_HELP)).chain(editors),
        ),
    ]))
}

pub(crate) fn controls(ui: &Runtime) -> Result<Element, RuntimeError> {
    let enabled = ui.signal(true);
    let mode = ui.signal("first");
    let options = [
        Choice::new("first", "First option"),
        Choice::new("unavailable", "Unavailable option").disabled(),
        Choice::new("second", "Second option"),
    ];
    let status = ui.signal("Choose an action from the menu.".to_owned());
    let menu_status = status.clone();
    let open = ui.signal(false);
    let show = open.clone();
    let close = open.clone();
    Ok(page([
        section(
            "boolean",
            "Checkbox and toggle share one value",
            [
                Element::new("checkbox", Checkbox::new(enabled.clone(), "Enabled")),
                Element::new("toggle", Toggle::new(enabled.clone(), "Enabled")),
                Element::new(
                    "disabled",
                    Checkbox::new(enabled, "Disabled presentation").disabled(),
                ),
            ],
        ),
        section(
            "radio",
            "Radio group and select share one choice",
            [
                Element::new("radio", RadioGroup::new(mode.clone(), options.clone())?)
                    .with_label("Mode"),
                Element::new("dropdown", Select::new(ui, mode, options)?).with_label("Mode"),
            ],
        ),
        section(
            "surfaces",
            "Menus, popovers and delayed help",
            [
                Element::new(
                    "action-menu",
                    Menu::new(
                        ui,
                        "Actions",
                        vec![
                            MenuItem::new("apply", "Apply", move |_| {
                                menu_status.set("Apply selected.".into())
                            }),
                            MenuItem::new("unavailable", "Unavailable", |_| Ok(())).disabled(),
                        ],
                    )?,
                ),
                Element::new("menu-status", Label::new(status.read_only())),
                Element::new(
                    "help-tooltip",
                    Tooltip::new(
                        Element::new(
                            "help-trigger",
                            Button::new("Hover or focus for help", |_| Ok(())),
                        ),
                        "Help appears after 500 ms, uses no polling and never takes focus.",
                    ),
                ),
                Element::new(
                    "popover-demo",
                    Popover::new(
                        open,
                        Element::new(
                            "popover-trigger",
                            Button::new("Open popover", move |_| show.set(true)),
                        ),
                        Element::new(
                            "popover-content",
                            Column::new([
                                Element::new(
                                    "explanation",
                                    Label::new(
                                        "This surface escapes the page clip. Escape or an outside click closes it.",
                                    ),
                                ),
                                Element::new(
                                    "close-popover",
                                    Button::new("Close", move |_| close.set(false)),
                                ),
                            ]),
                        ),
                    ),
                ),
            ],
        ),
    ]))
}

// The gallery plays the application: it owns the task and publishes each step.
// The live bar starts idle, so the page schedules nothing until Start is pressed.
pub(crate) fn progress(ui: &Runtime) -> Element {
    const STEPS: u64 = 4;
    let status = ui.signal(ProgressStatus::Idle);
    let done = ui.signal(0_u64);
    let caption = ui.signal("Idle. Nothing is scheduled.".to_owned());
    // The spinner follows a plain flag, which the gallery keeps in step with the
    // task: it spins exactly while the status says the task is running.
    let busy = ui.signal(false);
    let always = ui.signal(false);
    // Each action reads the steps done and the current status, so ending the
    // task keeps the fraction it reached.
    let action =
        |key: &str, title: &str, next: fn(u64, ProgressStatus) -> (ProgressStatus, u64, String)| {
            let (status, done, caption) = (status.clone(), done.clone(), caption.clone());
            let busy = busy.clone();
            Element::new(
                key,
                Button::new(title, move |_| {
                    let (next_status, next_done, text) = next(done.get()?, status.get()?);
                    done.set(next_done)?;
                    busy.set(next_status.is_running())?;
                    status.set(next_status)?;
                    caption.set(text)
                }),
            )
        };
    // A status the page never changes is described, not bound.
    let fixed = |key: &str, title: &str, help: &str, value: ProgressStatus| {
        let field = Field::new(Element::new(format!("{key}-bar"), ProgressBar::new(value)))
            .label(title)
            .help(help);
        Element::new(
            key,
            if matches!(value, ProgressStatus::Failed(_)) {
                field.status(FieldStatus::error("The disk is full"))
            } else {
                field
            },
        )
    };
    page([
        section(
            "progress-live",
            "A task the application drives",
            [
                label(
                    "progress-help",
                    "Start shows activity with no known extent, the only status that animates. Step reports a known fraction, which the bar draws without scheduling anything; the spinner beside the caption ticks while the task runs. Ending the task, hiding this tab or switching pages stops the animation.",
                ),
                Element::new(
                    "progress-live-field",
                    Field::new(Element::new(
                        "progress-live-bar",
                        ProgressBar::bound(status.read_only()),
                    ))
                    .label("Export"),
                ),
                Element::new(
                    "progress-caption-row",
                    Row::new([
                        Element::new("progress-spinner", Spinner::bound(busy.read_only())),
                        Element::new("progress-caption", Label::new(caption.read_only())),
                    ]),
                ),
                Element::new(
                    "progress-actions",
                    Wrap::new([
                        action("progress-start", "Start", |_, _| {
                            (
                                ProgressStatus::Indeterminate,
                                0,
                                "Running, extent unknown. Animating.".to_owned(),
                            )
                        }),
                        action("progress-step", "Step", |done, _| {
                            let done = (done + 1).min(STEPS);
                            (
                                ProgressStatus::of(done, STEPS),
                                done,
                                format!(
                                    "Step {done} of {STEPS}. The bar is still; only the spinner ticks."
                                ),
                            )
                        }),
                        action("progress-succeed", "Succeed", |done, _| {
                            (ProgressStatus::Succeeded, done, "Succeeded.".to_owned())
                        }),
                        action("progress-fail", "Fail", |done, current| {
                            (
                                ProgressStatus::Failed(current.fraction()),
                                done,
                                "Failed where it was.".to_owned(),
                            )
                        }),
                        action("progress-cancel", "Cancel", |done, current| {
                            (
                                ProgressStatus::Cancelled(current.fraction()),
                                done,
                                "Cancelled where it was.".to_owned(),
                            )
                        }),
                        action("progress-reset", "Reset", |_, _| {
                            (
                                ProgressStatus::Idle,
                                0,
                                "Idle. Nothing is scheduled.".to_owned(),
                            )
                        }),
                    ]),
                ),
            ],
        ),
        section(
            "spinner-section",
            "A spinner for places too small for a bar",
            [
                label(
                    "spinner-help",
                    "Eight fixed dots whose brightness travels around the ring, one step every 100 milliseconds. The spinner above follows the task; this one has its own switch. Resting, it draws nothing and keeps its place, so the text beside it never moves.",
                ),
                Element::new(
                    "spinner-row",
                    Row::new([
                        Element::new("spinner-demo", Spinner::bound(always.read_only())),
                        Element::new("spinner-toggle", Toggle::new(always.clone(), "Busy")),
                    ]),
                ),
            ],
        ),
        section(
            "progress-states",
            "Every status that does not animate",
            [
                fixed(
                    "progress-idle",
                    "Idle",
                    "An empty track.",
                    ProgressStatus::Idle,
                ),
                fixed(
                    "progress-partial",
                    "Determinate",
                    "Three of eight steps, drawn where it is.",
                    ProgressStatus::of(3, 8),
                ),
                fixed(
                    "progress-succeeded",
                    "Succeeded",
                    "A full track in the Success role.",
                    ProgressStatus::Succeeded,
                ),
                fixed(
                    "progress-failed",
                    "Failed",
                    "A full track in the Error role, for a fraction nobody knows. The reason is the application's to present.",
                    ProgressStatus::Failed(None),
                ),
                fixed(
                    "progress-failed-partial",
                    "Failed at three of eight",
                    "The track filled to the fraction the task reached, in the Error role.",
                    ProgressStatus::Failed(ProgressStatus::of(3, 8).fraction()),
                ),
                fixed(
                    "progress-cancelled",
                    "Cancelled",
                    "A full track in the Muted role.",
                    ProgressStatus::Cancelled(None),
                ),
                fixed(
                    "progress-cancelled-partial",
                    "Cancelled at three of eight",
                    "The track filled to the fraction the task reached, in the Muted role.",
                    ProgressStatus::Cancelled(ProgressStatus::of(3, 8).fraction()),
                ),
            ],
        ),
        clock(ui),
    ])
}

/// An application timer: `Runtime::start_interval` ticks once a second until
/// stopped, also while focus is elsewhere or the window is hidden, and only
/// while it runs does the gallery schedule anything.
fn clock(ui: &Runtime) -> Element {
    let ticks = ui.signal(0_u64);
    let timer = Rc::new(RefCell::new(None::<IntervalTimer>));
    let (counter, running) = (ticks.clone(), timer.clone());
    let start = Button::new("Start the clock", move |cx| {
        if running.borrow().is_some() {
            return Ok(());
        }
        let (counter, running) = (counter.clone(), running.clone());
        cx.defer(move |ui| {
            // A second Start queued before this task ran finds the clock
            // running and starts nothing.
            if running.borrow().is_some() {
                return Ok(());
            }
            let started = ui.start_interval(std::time::Duration::from_secs(1), move |_| {
                counter.update(|ticks| *ticks += 1)
            })?;
            *running.borrow_mut() = Some(started);
            Ok(())
        })?;
        Ok(())
    });
    let stop = Button::new("Stop", move |_| {
        if let Some(timer) = timer.borrow_mut().take() {
            timer.cancel();
        }
        Ok(())
    });
    let shown = ticks.map(|ticks| format!("Ticks: {ticks}"));
    section(
        "clock-section",
        "An application timer",
        [
            label(
                "clock-help",
                "Start the clock: a timer the application owns ticks once a second, also while another control has focus or the window is hidden, until you stop it. It is not a component's, so cancelled input and focus loss leave it running.",
            ),
            Element::new(
                "clock-row",
                Row::new([
                    Element::new("clock-start", start),
                    Element::new("clock-stop", stop),
                    match shown {
                        Ok(text) => Element::new("clock-ticks", Label::new(text)),
                        Err(error) => label("clock-ticks", error.to_string()),
                    },
                ]),
            ),
        ],
    )
}

// The application owns the rules: the library only presents what is published.
pub(crate) fn adaptive_form(ui: &Runtime) -> Result<Element, RuntimeError> {
    let profile = Form::new([
        Element::new(
            "name-field",
            Field::new(Element::new(
                "name",
                TextField::new(ui.signal("Ada".to_owned())),
            ))
            .label("Name")
            .help("Keep typing while resizing this panel."),
        ),
        Element::new(
            "city-field",
            Field::new(Element::new(
                "city",
                TextField::new(ui.signal("Stockholm".to_owned())),
            ))
            .label("City"),
        ),
        Element::new(
            "count-field",
            Field::new(Element::new("count", IntegerInput::new(ui.signal(3))))
                .label("Copies")
                .help("A draft saves on Enter or focus loss."),
        ),
        Element::new(
            "notes-field",
            Field::new(Element::new(
                "notes",
                TextArea::new(ui.signal("Optional notes".to_owned())),
            ))
            .label("Notes"),
        ),
    ]);
    Ok(page([
        label(
            "form-help",
            "Drag the divider: fields use the panel width to choose columns. Narrow panels scroll instead of shrinking controls below a usable width. Labels and messages stay with their control.",
        ),
        stage(
            "form-stage",
            360,
            Element::new(
                "form-split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new(
                        "profile-viewport",
                        ScrollArea::new(Element::new("profile", profile)).axes(ScrollAxes::Both),
                    ),
                    Element::new(
                        "explanation",
                        Label::new(
                            "The form adapts to its allocation, independently of the window.",
                        ),
                    ),
                )
                .sizing(SplitSizing::ratio(0.75)?),
            ),
        ),
    ]))
}

pub(crate) fn field_status(ui: &Runtime) -> Result<Element, RuntimeError> {
    let form = FormFields::new(ui);
    let name = ui.signal(String::new());
    let notes = ui.signal("Notes are optional.".to_owned());
    let name_status = form.field("name")?;
    let notes_status = form.field("notes")?;
    let outcome = ui.signal("Press Check to apply the application's rules.".to_owned());
    let (check_form, check_name, check_notes) = (form.clone(), name.clone(), notes.clone());
    let (check_name_status, check_notes_status) = (name_status.clone(), notes_status.clone());
    let check_outcome = outcome.clone();
    let (clear_name_status, clear_notes_status) = (name_status.clone(), notes_status.clone());
    let clear_outcome = outcome.clone();
    Ok(page([Element::new(
        "field-status-page",
        Group::titled(
            "Label, help and status around any control",
            Element::new(
                "field-status-form",
                Column::new([
                    label(
                        "field-status-help",
                        "A Field wraps one control. The application checks its own rules and publishes a status; an error outlines the control, replaces the help line and receives focus.",
                    ),
                    Element::new(
                        "name-status-field",
                        Field::new(Element::new("status-name", TextField::new(name)))
                            .label("Name")
                            .help("Required. Leave it empty and press Check.")
                            .form(&name_status),
                    ),
                    Element::new(
                        "notes-status-field",
                        Field::new(Element::new("status-notes", TextArea::new(notes)))
                            .label("Notes")
                            .help("More than 40 characters produces a warning.")
                            .form(&notes_status),
                    ),
                    Element::new(
                        "field-status-actions",
                        Wrap::new([
                            Element::new(
                                "check-fields",
                                Button::new("Check", move |cx| {
                                    check_name_status.set(
                                        if check_name.get()?.trim().is_empty() {
                                            FieldStatus::error("A name is required")
                                        } else {
                                            FieldStatus::Normal
                                        },
                                    )?;
                                    check_notes_status.set(
                                        if check_notes.get()?.chars().count() > 40 {
                                            FieldStatus::warning(
                                                "Long notes are truncated in lists",
                                            )
                                        } else {
                                            FieldStatus::Normal
                                        },
                                    )?;
                                    let summary = check_form.summary().get()?;
                                    check_outcome.set(format!(
                                        "{} error(s), {} warning(s)",
                                        summary.errors(),
                                        summary.warnings()
                                    ))?;
                                    check_form.focus_first_error(cx).map(|_| ())
                                }),
                            ),
                            Element::new(
                                "clear-fields",
                                Button::new("Clear status", move |_| {
                                    clear_name_status.clear()?;
                                    clear_notes_status.clear()?;
                                    clear_outcome.set("Status cleared.".to_owned())
                                }),
                            ),
                        ]),
                    ),
                    Element::new("field-status-outcome", Label::new(outcome.read_only())),
                ]),
            ),
        ),
    )]))
}

// Swatches present colors the gallery owns; the hex field parses with
// `Color::from_hex`, saves the canonical text and applies it on Enter.
pub(crate) fn colors(ui: &Runtime) -> Result<Element, RuntimeError> {
    let parse = |text: &str| Color::from_hex(text.trim()).map_err(|error| error.to_string());
    let initial = Color::from_srgb8(0x20, 0x90, 0xa0, 0xcc);
    let color = ui.signal(initial);
    let hex = ui.signal(initial.to_hex());
    let channels = color.map(|color| describe_color(*color))?;
    let activations = ui.signal("Activate the swatch with a click, Enter or Space.".to_owned());
    let apply_color = color.clone();
    let activated = activations.clone();
    let edits = ui.signal("No committed picker edit yet.".to_owned());
    let reported = edits.clone();
    let mut committed = 0;
    let mut count = 0_u32;
    let swatch = |key: &str, bytes: [u8; 4], name: &str| {
        let [r, g, b, a] = bytes;
        Element::new(
            key,
            ColorSwatch::new(Color::from_srgb8(r, g, b, a)).name(name),
        )
    };
    let accent = ui.signal(Color::from_srgb8(0xe0, 0x60, 0x20, 0xff));
    let named = named_colors(ui, &color, &hex)?;
    Ok(page([
        section(
            "swatches",
            "Swatches show translucent colors over a checkerboard",
            [Element::new(
                "swatch-row",
                Wrap::new([
                    swatch("swatch-opaque", [0x20, 0x90, 0xa0, 0xff], "Opaque teal"),
                    swatch("swatch-half", [0xff, 0x88, 0x00, 0x80], "Half-transparent orange"),
                    swatch("swatch-faint", [0xff, 0xff, 0xff, 0x33], "Faint white"),
                    swatch("swatch-clear", [0, 0, 0, 0], "Fully transparent"),
                ]),
            )],
        ),
        section(
            "hex-entry",
            "Hexadecimal sRGB entry and an activatable swatch",
            [
                label(
                    "hex-help",
                    "Type #rgb, #rgba, #rrggbb or #rrggbbaa and press Enter or move focus away. Invalid text is refused with the parser's reason.",
                ),
                Element::new(
                    "hex-row",
                    Wrap::new([
                        Element::new(
                            "hex-field",
                            TextField::new(hex.clone())
                                .normalize(move |text| {
                                    parse(&text)
                                        .map(Color::to_hex)
                                        .map_err(InputRejection::new)
                                })
                                .on_save(move |_, text| {
                                    if let Ok(parsed) = parse(text) {
                                        apply_color.set(parsed)?;
                                    }
                                    Ok(())
                                }),
                        ).with_label("Hexadecimal color"),
                        Element::new(
                            "hex-swatch",
                            ColorSwatch::bound(color.read_only())
                                .name("Chosen color")
                                .on_activate(move |_| {
                                    count += 1;
                                    activated.set(format!(
                                        "Activated {count} time(s); an application would open its color picker here."
                                    ))
                                }),
                        ),
                    ]),
                ),
                Element::new("hex-channels", Label::new(channels)),
                Element::new("hex-activations", Label::new(activations.read_only())),
            ],
        ),
        section(
            "picker",
            "Color picker editing the chosen color, with alpha",
            [
                label(
                    "picker-help",
                    "Drag the area or a strip: the swatch and readout above change on release, and Escape cancels a drag. Arrow keys step a focused part, Shift steps finer. Drag to black and back: the hue stays. Each committed edit is reported below, as an application would record it for undo; typing in the hex field above is the application's write and is not.",
                ),
                Element::new(
                    "picker-inline",
                    ColorPicker::new(color)
                        .alpha(true)
                        .name("Chosen color")
                        .on_change(move |_, written| {
                            committed += 1;
                            reported.set(format!(
                                "Committed edit {committed}: {}",
                                written.to_hex()
                            ))
                        }),
                ),
                Element::new("picker-edits", Label::new(edits.read_only())),
            ],
        ),
        section(
            "picker-popover",
            "A color picker in a popover opened from its swatch",
            [Element::new(
                "accent",
                ColorPicker::new(accent).name("Accent color").into_popover(),
            )],
        ),
        named,
    ]))
}

/// Named colors offered by the quick pick, with the kind each row shows.
const NAMED_COLORS: [(&str, &str, [u8; 3]); 7] = [
    ("tomato", "Tomato", [0xff, 0x63, 0x47]),
    ("gold", "Gold", [0xff, 0xd7, 0x00]),
    ("coral", "Coral", [0xff, 0x7f, 0x50]),
    ("teal", "Teal", [0x00, 0x80, 0x80]),
    ("steel-blue", "Steel blue", [0x46, 0x82, 0xb4]),
    ("slate", "Slate gray", [0x70, 0x80, 0x90]),
    ("silver", "Silver", [0xc0, 0xc0, 0xc0]),
];

/// A nonmodal quick pick over named colors that sets the chosen color, with
/// one disabled row whose choice is refused with a message.
fn named_colors(
    ui: &Runtime,
    color: &Signal<Color>,
    hex: &Signal<String>,
) -> Result<Element, RuntimeError> {
    let picker = QuickPickController::new(ui);
    let status = ui.signal("No named color chosen yet.".to_owned());
    let (opener, chosen, text, report) =
        (picker.clone(), color.clone(), hex.clone(), status.clone());
    let kind = |name: &str| match name {
        "tomato" | "gold" | "coral" => "Warm",
        "teal" | "steel-blue" => "Cool",
        _ => "Neutral",
    };
    let items = move || {
        let mut items: Vec<PickItem> = NAMED_COLORS
            .iter()
            .map(|(key, name, [r, g, b])| {
                PickItem::new(*key, *name)
                    .detail(Color::from_srgb8(*r, *g, *b, 0xff).to_hex())
                    .kind(kind(key))
            })
            .collect();
        items.push(
            PickItem::new("ultraviolet", "Ultraviolet")
                .detail("outside sRGB")
                .kind("Unavailable")
                .disabled(true),
        );
        items
    };
    let quick = QuickPick::new(
        picker,
        Element::new(
            "named-open",
            Button::new("Pick a named color…", move |_| opener.open(items())),
        ),
        move |_, item| {
            let Some((_, name, [r, g, b])) = NAMED_COLORS.iter().find(|(key, ..)| *key == item.key)
            else {
                return Ok(PickChoice::Refused(format!(
                    "{} cannot be shown on this display.",
                    item.label
                )));
            };
            let picked = Color::from_srgb8(*r, *g, *b, 0xff);
            chosen.set(picked)?;
            text.set(picked.to_hex())?;
            report.set(format!("Chose {name}."))?;
            Ok(PickChoice::Done)
        },
    )
    .nonmodal()
    .name("Named colors")
    .list_name("Colors")
    .placeholder("Type a color name")
    .empty_text("No color with that name");
    Ok(section(
        "named-colors",
        "A quick pick over named colors",
        [
            label(
                "named-help",
                "Open the nonmodal picker, type to filter, move with Up and Down and press Enter, or click a row. Each row shows its hexadecimal value and its kind. Ultraviolet is disabled: choosing it is refused with a message and the picker stays open.",
            ),
            Element::new("named-picker", quick),
            Element::new("named-status", Label::new(status.read_only())),
        ],
    ))
}

fn describe_color(color: Color) -> String {
    let [r, g, b, a] = color.to_srgb8();
    format!(
        "sRGB bytes {r}, {g}, {b}, alpha {a}; linear {:.4}, {:.4}, {:.4}, alpha {:.4}",
        color.red(),
        color.green(),
        color.blue(),
        color.alpha()
    )
}
