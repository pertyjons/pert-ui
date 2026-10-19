//! The Tool windows page: stripes along the edges that expand docked panels,
//! a locked and an open configuration with shared sample content, and a nested layout.
use crate::gallery::{label, page, section, toggle};
use pert_ui::*;

const HELP: &str = "Both layouts show the same sample workspace and share its selected file. Change the selection in either layout and watch the main view and Properties in both layouts update. The layouts differ in what the user can rearrange.";
const OPEN_HELP: &str = "Open: Files starts expanded. Open Search from the left stripe, resize a docked panel by dragging its divider, and use a panel's three-dot menu to move, overlay, float or open it in its own window. Notes on the bottom stripe is an overlay: it covers the main view and hides when focus leaves. Escape from a panel returns focus to the main view.";
const LOCKED_HELP: &str = "Locked: the same panels and selection, but the stripes cannot toggle them and there are no resize handles or three-dot menus. The button below opens Properties through the application controller; locking restricts user rearrangement, not application commands.";

const NESTED_HELP: &str = "The Structure tool window holds a second layout with only a left stripe. Escape inside the inner Outline returns focus to the inner editor, and a reveal opens the outer and the inner tool window together.";

/// The tool windows of one demonstration layout; keys are prefixed so both
/// layouts on the page stay distinct.
fn layout(
    prefix: &str,
    names: &Signal<bool>,
    controller: &ToolWindowController,
    changes: &Signal<String>,
    selected: &Signal<String>,
    activity: &Signal<String>,
) -> ToolWindowLayout {
    let key = |name: &str| format!("{prefix}-{name}");
    let writer = changes.clone();
    let prefix_owned = prefix.to_owned();
    let review_activity = activity.clone();
    let reviewed_file = selected.clone();
    let select_file = selected.clone();
    let find_file = selected.clone();
    ToolWindowLayout::new(Element::new(
        key("main"),
        Column::new([
            label(&key("main-text"), "Workspace preview"),
            label(
                &key("main-help"),
                "Choose a file in Files or Search. The selection is shared by both layouts.",
            ),
            label(&key("main-selection-label"), "Selected file:"),
            Element::new(key("main-selection"), Label::new(selected.read_only())),
            Element::new(
                key("main-button"),
                Button::new("Review selected file", move |_| {
                    review_activity.set(format!(
                        "Reviewed {} in the main view",
                        reviewed_file.get()?
                    ))
                }),
            ),
            Element::new(key("main-activity"), Label::new(activity.read_only())),
        ]),
    ))
    .edge(ToolEdge::Left, EdgeMode::Stripe)
    .edge(ToolEdge::Bottom, EdgeMode::Stripe)
    .edge(ToolEdge::Right, EdgeMode::Stripe)
    .window(
        ToolWindow::new(
            key("files"),
            "Files",
            Element::new(
                key("files-body"),
                Column::new([
                    label(
                        &key("files-help"),
                        "Pick a file to update both workspace previews:",
                    ),
                    Element::new(
                        key("files-open"),
                        Button::new("Select src/main.rs", move |_| {
                            select_file.set("src/main.rs".to_owned())
                        }),
                    ),
                    label(&key("files-text"), "Also in this sample: docs/layout.md"),
                ]),
            ),
        )
        .icon(Icon::Folder)
        .expanded(true),
    )
    .window(
        ToolWindow::new(
            key("search"),
            "Search",
            Element::new(
                key("search-body"),
                Button::new("Find docs/layout.md", move |_| {
                    find_file.set("docs/layout.md".to_owned())
                }),
            ),
        )
        .icon(Icon::Search),
    )
    .window(
        ToolWindow::new(
            key("settings"),
            "Settings",
            Element::new(
                key("settings-body"),
                Column::new([
                    label(&key("settings-title"), "Workspace settings"),
                    label(
                        &key("settings-text"),
                        "Move this panel to another edge from its menu in the open layout.",
                    ),
                ]),
            ),
        )
        .icon(Icon::Settings)
        .group(StripeGroup::End),
    )
    .window(
        ToolWindow::new(
            key("log"),
            "Log",
            Element::new(
                key("log-body"),
                Column::new([
                    label(&key("log-first"), "09:41  Workspace opened"),
                    label(&key("log-second"), "09:42  Files indexed"),
                    label(&key("log-third"), "09:43  Build finished"),
                ]),
            ),
        )
        .icon(Icon::Info)
        .edge(ToolEdge::Bottom),
    )
    .window(
        ToolWindow::new(
            key("properties"),
            "Properties",
            Element::new(
                key("properties-body"),
                Column::new([
                    label(&key("properties-title"), "Selected file:"),
                    Element::new(
                        key("properties-selection"),
                        Label::new(selected.read_only()),
                    ),
                    label(
                        &key("properties-help"),
                        "This panel follows the selection even when collapsed.",
                    ),
                ]),
            ),
        )
        .icon(Icon::Tune)
        .edge(ToolEdge::Right),
    )
    .window(
        ToolWindow::new(
            key("notes"),
            "Notes",
            Element::new(
                key("notes-body"),
                Column::new([
                    label(
                        &key("notes-text"),
                        "This overlay covers the workspace without resizing it.",
                    ),
                    label(
                        &key("notes-help"),
                        "Move focus back to the main view to hide it.",
                    ),
                ]),
            ),
        )
        .icon(Icon::Edit)
        .edge(ToolEdge::Bottom)
        .group(StripeGroup::End)
        .overlay(true),
    )
    .names(names.clone())
    .controller(controller)
    .on_change(move |_, change| {
        let what = match change.kind {
            ToolWindowChangeKind::Placement => {
                format!("moved to {:?} {:?}", change.edge, change.group)
            }
            ToolWindowChangeKind::Mode => match change.mode {
                ToolWindowMode::Floating => "floated".to_owned(),
                ToolWindowMode::Window => "set to open in its own window".to_owned(),
                _ if change.overlay => "made an overlay".to_owned(),
                _ => "docked".to_owned(),
            },
            _ if change.expanded => "expanded".to_owned(),
            _ => "collapsed".to_owned(),
        };
        writer.set(format!(
            "{prefix_owned}: {} {what} ({:?})",
            change.key, change.origin
        ))
    })
}

fn stage(key: &str, height: u16, layout: ToolWindowLayout) -> Result<Element, RuntimeError> {
    Ok(Element::new(
        format!("{key}-width"),
        Grid::new(
            [Track::weighted(1.0)?],
            [Element::new(
                key,
                Container::new(Element::new("frame", layout))
                    .preferred_size(Size::from([0, height])),
            )],
        )?,
    ))
}

pub(crate) fn tool_windows(ui: &Runtime) -> Result<Element, RuntimeError> {
    let names = ui.signal(false);
    let changes = ui.signal("No change yet".to_owned());
    let selected = ui.signal("README.md".to_owned());
    let activity = ui.signal("Select a file, then review it here.".to_owned());
    let open = ToolWindowController::new();
    let locked = ToolWindowController::new();
    let toggle_properties = |controller: &ToolWindowController, prefix: &'static str| {
        let controller = controller.clone();
        move |_: &mut EventContext| {
            controller
                .toggle(&format!("{prefix}-properties"))
                .map(|_| ())
        }
    };
    let inner = ToolWindowLayout::new(Element::new(
        "inner-editor",
        Button::new("Inner editor", |_| Ok(())),
    ))
    .edge(ToolEdge::Left, EdgeMode::Stripe)
    .window(
        ToolWindow::new(
            "nested-outline",
            "Outline",
            Element::new(
                "nested-outline-body",
                Button::new("Outline item", |_| Ok(())),
            ),
        )
        .icon(Icon::MoreVertical),
    );
    let nested = ToolWindowLayout::new(Element::new(
        "outer-editor",
        Button::new("Outer editor", |_| Ok(())),
    ))
    .edge(ToolEdge::Right, EdgeMode::Stripe)
    .window(
        ToolWindow::new("structure", "Structure", Element::new("inner-frame", inner))
            .icon(Icon::Tune)
            .edge(ToolEdge::Right),
    );
    Ok(page([
        section(
            "tool-window-demo",
            "Open and locked",
            [
                label("help", HELP),
                toggle("show-names", "Show tool window names", &names)?,
                label("open-heading", "Open layout"),
                label("open-help", OPEN_HELP),
                stage(
                    "open-stage",
                    440,
                    layout("open", &names, &open, &changes, &selected, &activity),
                )?,
                label("locked-heading", "Locked layout"),
                label("locked-help", LOCKED_HELP),
                stage(
                    "locked-stage",
                    340,
                    layout("locked", &names, &locked, &changes, &selected, &activity).locked(),
                )?,
                Element::new(
                    "actions",
                    Wrap::new([
                        Element::new(
                            "toggle-open-properties",
                            Button::new(
                                "Toggle properties (open)",
                                toggle_properties(&open, "open"),
                            ),
                        ),
                        Element::new(
                            "toggle-locked-properties",
                            Button::new(
                                "Toggle properties (locked)",
                                toggle_properties(&locked, "locked"),
                            ),
                        ),
                    ]),
                ),
                Element::new("last-change", Label::new(changes.read_only())),
            ],
        ),
        section(
            "nested-demo",
            "Nested layouts",
            [
                label("nested-help", NESTED_HELP),
                Element::new(
                    "nested-stage",
                    Container::new(Element::new("nested", nested))
                        .preferred_size(Size::from([640, 280])),
                ),
            ],
        ),
    ]))
}
