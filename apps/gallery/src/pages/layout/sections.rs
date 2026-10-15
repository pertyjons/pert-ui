//! The Sections page: collapsible sections and accordions.
use crate::gallery::{label, page, section, toggle};
use pert_ui::*;

const HELP: &str = "Click a header, or focus it with Tab and press Enter or Space, to show or hide its content. Collapsed content keeps its state: type in the note, collapse and expand again. The first section remembers its expansion in a saved workspace; the second follows the application's switch instead and is not saved.";

const ACCORDION_HELP: &str = "An accordion is one Tab stop: Up and Down move between its headers, Home and End jump to the first and last, Enter or Space toggles. In the first, opening a section closes the open one; in the second, sections open independently.";

fn settings(prefix: &str) -> [Section; 3] {
    [
        Section::new(
            format!("{prefix}-general"),
            "General",
            label(
                &format!("{prefix}-general-body"),
                "Name, size and location.",
            ),
        )
        .leading_icon(Icon::Settings),
        Section::new(
            format!("{prefix}-display"),
            "Display",
            Element::new(
                format!("{prefix}-display-body"),
                Column::new([
                    label(&format!("{prefix}-display-text"), "Theme and density."),
                    Element::new(
                        format!("{prefix}-display-button"),
                        Button::new("Apply", |_| Ok(())),
                    ),
                ]),
            ),
        ),
        Section::new(
            format!("{prefix}-advanced"),
            "Advanced",
            label(
                &format!("{prefix}-advanced-body"),
                "Options most people never change.",
            ),
        ),
    ]
}

pub(crate) fn sections(ui: &Runtime) -> Result<Element, RuntimeError> {
    let note = ui.signal(String::new());
    let shown = ui.signal(false);
    Ok(page([
        section(
            "collapsible-demo",
            "Collapsible sections",
            [
                label("help", HELP),
                Element::new(
                    "remembered",
                    Collapsible::new(
                        "Remembered section",
                        Element::new(
                            "remembered-body",
                            Column::new([
                                label("remembered-text", "Type a note, then collapse and expand."),
                                Element::new("section-note", TextField::new(note))
                                    .with_label("Note"),
                            ]),
                        ),
                    )
                    .expanded(true),
                ),
                toggle("bound-switch", "Show the bound section", &shown)?,
                Element::new(
                    "bound",
                    Collapsible::new(
                        "Bound section",
                        label("bound-body", "The application's switch decides."),
                    )
                    .bound(shown),
                ),
                Element::new(
                    "disabled",
                    Collapsible::new("Disabled section", label("disabled-body", "Unreachable."))
                        .disabled(),
                ),
            ],
        ),
        section(
            "accordion-demo",
            "Accordions",
            [
                label("accordion-help", ACCORDION_HELP),
                Element::new(
                    "single",
                    Accordion::new(settings("single"))
                        .mode(ExpandMode::Single)
                        .expanded(["single-general"]),
                ),
                Element::new("multiple", Accordion::new(settings("multiple"))),
            ],
        ),
    ]))
}
