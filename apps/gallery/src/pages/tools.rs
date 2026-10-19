//! The Appearance & tools category: themes, the workspace document, the
//! profiler and the debug overlay.
use crate::gallery::{label, page, section, stage};
use pert_ui::*;

/// The Violet palette preset used by the theme page and screenshot mode.
pub(crate) const VIOLET_SEED: [f32; 3] = [0.35, 0.06, 0.60];

pub(crate) fn profiling(ui: &Runtime) -> Element {
    let report = ui.signal(
        "Profiling is enabled. Exercise another tab, then return and refresh the snapshot."
            .to_owned(),
    );
    let mut controls = [
        ("refresh-profile", "Refresh snapshot", None),
        ("enable-profile", "Enable", Some(true)),
        ("disable-profile", "Pause", Some(false)),
    ]
    .into_iter()
    .map(|(key, title, enabled)| {
        let report = report.clone();
        Element::new(
            key,
            Button::new(title, move |cx| {
                if let Some(enabled) = enabled {
                    cx.set_profiling(enabled)?;
                }
                report.set(profile_report(&cx.profile()))
            }),
        )
    })
    .collect::<Vec<_>>();
    let clear = report.clone();
    controls.push(Element::new(
        "clear-profile",
        Button::new("Clear", move |cx| {
            cx.clear_profile()?;
            clear.set(profile_report(&cx.profile()))
        }),
    ));
    page([section(
        "profiling",
        "UI event and view timings",
        [
            label(
                "help",
                "Elapsed UI-thread time in milliseconds, including waiting/preemption. Total includes nested work; Self excludes measured descendants. These are not CPU percentages or GPU timings. Profiling is optional and snapshots refresh only on request.",
            ),
            Element::new("controls", Wrap::new(controls)),
            Element::new(
                "profile-report",
                Label::new(report.read_only()).style(TextStyle::Monospace),
            ),
        ],
    )])
}

/// The development overlay, with a sample that is only wrong on request so the
/// gallery itself stays free of validation issues.
pub(crate) fn debug_overlay(ui: &Runtime) -> Element {
    let broken = ui.signal(false);
    page([section(
        "overlay",
        "Drawing development validation in the window",
        [
            label(
                "help",
                "Settings › Debug overlay draws into the scene itself. Validation issues outlines every component a development check names, in red, and counts markers it cannot place in a badge at the top right. Component boxes outlines each border box in blue and each padded content box in green. Both draw only in builds with debug assertions, and neither schedules frames: the scene is repainted once when the setting or the marked set changes.",
            ),
            Element::new(
                "break",
                Toggle::new("Arrange the sample wrongly", broken.clone()),
            ),
            Element::new(
                "sample",
                crate::samples::BrokenLayout::new(
                    broken.read_only(),
                    [
                        Element::new("first", Button::new("First", |_| Ok(()))),
                        Element::new("second", Button::new("Second", |_| Ok(()))),
                        Element::new("third", Button::new("Third", |_| Ok(()))),
                    ],
                ),
            ),
            label(
                "explain",
                "Wrongly arranged, Second overlaps First and Third hangs out below the sample. Both are reported on stderr once and marked while they last; switching the sample back removes the marks on the next paint.",
            ),
        ],
    )])
}
fn profile_report(profile: &ProfileSnapshot) -> String {
    let mut text = format!(
        "Recording: {} | Recent: {} | Evicted samples: {} | Omitted scopes: {}\n\n Calls  Total ms   Self ms    Max ms  Operation / component\n",
        profile.enabled,
        profile.recent.len(),
        profile.dropped_samples,
        profile.dropped_scopes
    );
    for row in profile.stats.iter().take(30) {
        // Each row is formatted into its own string and appended, so building
        // the report has no fallible `write!` step to handle.
        let line = format!(
            "{:>6} {:>9.3} {:>9.3} {:>9.3}  {:?} {} {}\n",
            row.calls,
            row.total.as_secs_f64() * 1000.0,
            row.self_time.as_secs_f64() * 1000.0,
            row.maximum.as_secs_f64() * 1000.0,
            row.operation,
            row.component
                .map_or_else(|| "runtime".into(), |id| format!("#{id}")),
            row.key.chars().take(24).collect::<String>()
        );
        text.push_str(&line);
    }
    text
}

const WORKSPACE_DOCUMENT_HELP: &str = "The encoded document, when it fits here. A host stores it wherever it keeps its settings. A line looks like: entry \"page/sample\" split extent=0.25 rule=\"ratio\"";
/// Capture and restore the gallery's own arrangement, without touching the file
/// system: the document lives in a text area, so the page shows the whole path
/// while the host keeps ownership of where such a document is stored.
///
/// Both buttons defer their work, because a capture wants `&Runtime` and a
/// restore `&mut Runtime`, which no component has during event dispatch. That is
/// the ordinary host pattern: the application runs these around its own
/// lifecycle, not from inside a callback.
///
/// A `TextArea` accepts a bound string only when `TextArea::check` passes, which
/// limits it to `TextEditError::MAX_TEXT_BYTES`. The gallery's own document is
/// larger, so this page keeps the captured text aside and puts it in the editor
/// only when it qualifies: the editor would refuse an oversized binding, keep
/// its previous text and accept no edits until the binding qualifies again.
pub(crate) fn workspace_page(ui: &Runtime) -> Element {
    // The captured document, whatever its size, and the smaller text the editor
    // may hold. Restore prefers what the editor holds, so an edited or
    // hand-written document wins over the last capture.
    let document = ui.signal(String::new());
    let editor = ui.signal(String::new());
    let status = ui.signal("Arrange the sample below, then capture.".to_owned());
    let capture = {
        let (document, editor, status) = (document.clone(), editor.clone(), status.clone());
        Element::new(
            "capture",
            Button::new("Capture", move |cx| {
                let (document, editor, status) = (document.clone(), editor.clone(), status.clone());
                cx.defer(move |ui| {
                    match ui.capture_workspace() {
                        Ok(workspace) => {
                            let text = workspace.encode();
                            let (entries, bytes) = (workspace.len(), text.len());
                            match TextArea::check(&text) {
                                Ok(()) => {
                                    editor.set(text.clone())?;
                                    status.set(format!(
                                        "Captured {entries} entries, {bytes} bytes. Edit below and restore."
                                    ))?;
                                }
                                Err(error) => {
                                    status.set(format!(
                                        "Captured {entries} entries, {bytes} bytes; the field below does not take it ({error}), so the document is kept aside. Restore uses it unless the field below holds a document of its own."
                                    ))?;
                                }
                            }
                            document.set(text)?;
                        }
                        Err(error) => {
                            status.set(format!("Capture failed: {error}"))?;
                        }
                    }
                    Ok(())
                })
                .map(|_| ())
            }),
        )
    };
    let restore = {
        let (document, editor, status) = (document.clone(), editor.clone(), status.clone());
        Element::new(
            "restore",
            Button::new("Restore", move |cx| {
                let (document, editor, status) = (document.clone(), editor.clone(), status.clone());
                cx.defer(move |ui| {
                    let edited = editor.get()?;
                    let text = if edited.trim().is_empty() {
                        document.get()?
                    } else {
                        edited
                    };
                    if text.trim().is_empty() {
                        status.set("Capture first, or write a document below.".to_owned())?;
                        return Ok(());
                    }
                    match Workspace::decode(&text) {
                        Err(error) => status.set(format!("Document rejected: {error}"))?,
                        Ok(workspace) => match ui.restore_workspace(&workspace) {
                            Err(error) => status.set(format!("Restore failed: {error}"))?,
                            Ok(report) => status.set(format!(
                                "Applied {}, pending {}, ignored {}, rejected {}.",
                                report.applied().count(),
                                report.pending().count(),
                                report.ignored().count(),
                                report.rejected().count(),
                            ))?,
                        },
                    };
                    Ok(())
                })
                .map(|_| ())
            }),
        )
    };
    let sample = Element::new(
        "sample",
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new(
                "left",
                Tabs::new([
                    Tab::new(
                        "first",
                        "First",
                        Element::new("first-body", Label::new("Select the other tab.")),
                    ),
                    Tab::new(
                        "second",
                        "Second",
                        Element::new("second-body", Label::new("This selection is captured.")),
                    ),
                ])
                // Every tab strip in the gallery offers detachment.
                .detachable(true),
            ),
            Element::new(
                "right",
                ScrollArea::new(Element::new(
                    "lines",
                    Column::new((0..24).map(|line| {
                        Element::new(format!("line-{line}"), Label::new(format!("Line {line}")))
                    })),
                )),
            ),
        ),
    );
    page([
        Element::new(
            "intro",
            Label::new(
                "Drag the divider, pick a tab, scroll the list, then capture: the document covers the whole gallery, not only this page. Rearrange the sample and restore to put it back.",
            ),
        ),
        stage("workspace-sample", 200, sample),
        Element::new(
            "actions",
            Row::new([capture, restore]).gap(MetricRole::ItemGap),
        ),
        Element::new("workspace-status", Label::new(status.read_only())),
        Element::new(
            "document-field",
            Field::new(Element::new("document", TextArea::new(editor)))
                .label("Workspace document")
                .help(WORKSPACE_DOCUMENT_HELP),
        ),
    ])
}
pub(crate) fn themes(ui: &Runtime) -> Element {
    let theme = ui.theme();
    let reset = theme.clone();
    // The seeds the pickers edit and the presets write. They start at the
    // built-in dark accents, which seed the dark palette unchanged.
    let primary = ui.signal(Palette::dark().color(ColorRole::Primary));
    let secondary = ui.signal(Palette::dark().color(ColorRole::Secondary));
    let use_secondary = ui.signal(false);
    // Linear RGB seeds; Blue restores the built-in palettes. Red sits near the
    // Error hue on purpose, to show what luminance contrast does not measure.
    let seeds = [
        ("seed-blue", "Blue (built in)", None),
        ("seed-red", "Red", Some([0.70, 0.04, 0.03])),
        ("seed-green", "Green", Some([0.03, 0.45, 0.15])),
        ("seed-violet", "Violet", Some(VIOLET_SEED)),
        ("seed-gray", "Gray", Some([0.20, 0.20, 0.20])),
    ]
    .map(|(key, name, seed)| {
        let (theme, primary, use_secondary) =
            (theme.clone(), primary.clone(), use_secondary.clone());
        Element::new(
            key,
            Button::new(name, move |_| {
                use_secondary.set(false)?;
                match seed {
                    Some([red, green, blue]) => {
                        let seed =
                            Color::rgba(red, green, blue, 1.0).map_err(ApplicationError::new)?;
                        primary.set(seed)?;
                        apply_seeds(&theme, seed, None)
                    }
                    None => {
                        primary.set(Palette::dark().color(ColorRole::Primary))?;
                        theme.set_palette(ThemeMode::Dark, Palette::dark())?;
                        theme.set_palette(ThemeMode::Light, Palette::light())?;
                        Ok(())
                    }
                }
            }),
        )
    });
    let apply = {
        let (theme, primary, secondary, use_secondary) = (
            theme.clone(),
            primary.clone(),
            secondary.clone(),
            use_secondary.clone(),
        );
        Element::new(
            "apply-seeds",
            Button::new("Apply seeds", move |_| {
                let secondary = use_secondary.get()?.then(|| secondary.get()).transpose()?;
                apply_seeds(&theme, primary.get()?, secondary)
            }),
        )
    };
    let custom = [
        label("primary-seed-label", "Primary"),
        Element::new(
            "primary-seed",
            ColorPicker::new(primary)
                .name("Primary seed")
                .into_popover(),
        ),
        Element::new(
            "use-secondary",
            Checkbox::new("Secondary seed", use_secondary.clone()),
        ),
        Element::new(
            "secondary-seed",
            ColorPicker::new(secondary)
                .name("Secondary seed")
                .enabled(use_secondary.read_only())
                .into_popover(),
        ),
        apply,
    ];
    let roles = [
        ("Primary", ColorRole::Primary, ColorRole::OnPrimary),
        ("Secondary", ColorRole::Secondary, ColorRole::OnSecondary),
        ("Error", ColorRole::Error, ColorRole::OnError),
        ("Warning", ColorRole::Warning, ColorRole::OnWarning),
        ("Info", ColorRole::Info, ColorRole::OnInfo),
        ("Success", ColorRole::Success, ColorRole::OnSuccess),
    ];
    let containers = [
        (
            "Surface container",
            ColorRole::SurfaceContainer,
            ColorRole::OnSurfaceContainer,
        ),
        (
            "Primary container",
            ColorRole::PrimaryContainer,
            ColorRole::OnPrimaryContainer,
        ),
        (
            "Secondary container",
            ColorRole::SecondaryContainer,
            ColorRole::OnSecondaryContainer,
        ),
        (
            "Error container",
            ColorRole::ErrorContainer,
            ColorRole::OnErrorContainer,
        ),
        (
            "Warning container",
            ColorRole::WarningContainer,
            ColorRole::OnWarningContainer,
        ),
        (
            "Info container",
            ColorRole::InfoContainer,
            ColorRole::OnInfoContainer,
        ),
        (
            "Success container",
            ColorRole::SuccessContainer,
            ColorRole::OnSuccessContainer,
        ),
    ];
    page([
        label(
            "intro",
            "Choose color mode and density in the Settings menu and UI zoom at the right of the status bar. Palettes and density overrides are independent. Text size stays readable in Compact; zoom scales the whole interface.",
        ),
        section(
            "nested-groups",
            "Visible groups",
            [
                Element::new(
                    "nested",
                    Group::titled(
                        "Nested group",
                        label("body", "The border stays visible without focus."),
                    ),
                ),
                Element::new("separator", Divider::new()),
                Element::new(
                    "untitled",
                    Group::new(label("body", "Untitled group with a default frame.")),
                ),
                Element::new(
                    "frameless",
                    Group::new(label("body", "Explicitly frameless group.")).frameless(),
                ),
            ],
        ),
        stage(
            "boundary-demo",
            72,
            Element::new(
                "boundary",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new(
                        "fixed",
                        SplitPane::new(
                            SplitAxis::TopBottom,
                            label("fixed-top", "Fixed boundary below"),
                            label("fixed-bottom", "No grip or resize cursor"),
                        )
                        .locked(),
                    ),
                    Element::new(
                        "resizable",
                        SplitPane::new(
                            SplitAxis::TopBottom,
                            label("drag-top", "Draggable boundary below"),
                            label("drag-bottom", "Grip and resize cursor at rest"),
                        ),
                    ),
                ),
            ),
        ),
        label(
            "roles-help",
            "Accent fills mark active state and status. Container fills are their static counterparts for noninteractive content.",
        ),
        Element::new(
            "roles",
            Wrap::new(roles.map(|(name, role, on)| {
                Element::new(
                    name,
                    Container::new(Element::new("name", Label::new(name).color_role(on)))
                        .padding(Length::from(16))
                        .background_role(role),
                )
            })),
        ),
        Element::new(
            "containers",
            Wrap::new(containers.map(|(name, role, on)| {
                Element::new(
                    name,
                    Container::new(Element::new("name", Label::new(name).color_role(on)))
                        .padding(Length::from(16))
                        .background_role(role),
                )
            })),
        ),
        label(
            "seeds-help",
            "Seed both palettes from a brand color. Accents take its hue, neutrals a tint of it, and every color keeps its built-in contrast. Compare the Red seed with the Error fill. Pick your own primary, and optionally a secondary, then apply them; without a secondary seed, the secondary accent keeps its built-in hue offset from the primary.",
        ),
        Element::new("seeds", Wrap::new(seeds)),
        Element::new("custom-seeds", Wrap::new(custom)),
        Element::new(
            "override-primary",
            Button::new("Use secondary as primary", move |_| {
                let mode = theme.mode()?;
                let palette = theme.palette(mode)?;
                theme
                    .set_palette(
                        mode,
                        palette
                            .with_color(ColorRole::Primary, palette.color(ColorRole::Secondary))
                            .with_color(
                                ColorRole::OnPrimary,
                                palette.color(ColorRole::OnSecondary),
                            ),
                    )
                    .map(|_| ())
            }),
        ),
        Element::new(
            "reset-palette",
            Button::new("Reset current palette", move |_| {
                let mode = reset.mode()?;
                reset
                    .set_palette(
                        mode,
                        match mode {
                            ThemeMode::Dark => Palette::dark(),
                            ThemeMode::Light => Palette::light(),
                        },
                    )
                    .map(|_| ())
            }),
        ),
    ])
}

/// Seeds both palettes from `primary` and, when given, `secondary`. The pickers
/// edit without alpha, so their colors stay opaque.
fn apply_seeds(
    theme: &ThemeManager,
    primary: Color,
    secondary: Option<Color>,
) -> Result<(), RuntimeError> {
    let primary = OpaqueColor::try_from(primary).map_err(ApplicationError::new)?;
    let secondary = secondary
        .map(OpaqueColor::try_from)
        .transpose()
        .map_err(ApplicationError::new)?;
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        theme.set_palette(mode, Palette::from_seeds(mode, primary, secondary))?;
    }
    Ok(())
}
