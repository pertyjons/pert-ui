//! The Layout category: containers, splits, regions, scrolling, view
//! transforms and retained state.
mod navigation;
mod sections;
mod tool_windows;
mod transforms;
use crate::gallery::{action, counter, counter_of, label, page, section, stage, tile, toggle};
use crate::samples::{Pulse, ReceiptReadout};
pub(crate) use navigation::{NavigationTargets, describe_outcome, navigation};
use pert_ui::*;
pub(crate) use sections::sections;
use std::{cell::RefCell, rc::Rc};
pub(crate) use tool_windows::tool_windows;
pub(crate) use transforms::{transforms, zoom_benchmark};

pub(crate) fn layout(status: &Signal<String>) -> Result<Element, RuntimeError> {
    Ok(page([
        section(
            "linear",
            "Row and Column",
            [
                label(
                    "help",
                    "Row stays on one line. Column stacks children. Both use the same standard spacing.",
                ),
                Element::new(
                    "row",
                    Row::new([tile("a", "Row A"), tile("b", "Row B"), tile("c", "Row C")]),
                ),
                Element::new(
                    "column",
                    Column::new([tile("a", "Column A"), tile("b", "Column B")]),
                ),
            ],
        ),
        section(
            "wrap",
            "Wrap",
            [
                label(
                    "help",
                    "Narrow the window: buttons move to the next line and keep their identities.",
                ),
                Element::new(
                    "flow",
                    Wrap::new((0..12).map(|i| {
                        action(&format!("item-{i}"), format!("Action {}", i + 1), status)
                    })),
                ),
            ],
        ),
        section(
            "grid",
            "Grid: content, fixed and weighted columns",
            [
                label(
                    "help",
                    "The first column follows content, the second prefers 120 pixels, and the last receives spare width.",
                ),
                Element::new(
                    "tracks",
                    Grid::new(
                        [
                            Track::CONTENT,
                            Track::fixed(120.into()),
                            Track::weighted(1.0)?,
                        ],
                        [
                            tile("a", "Name"),
                            tile("b", "Fixed"),
                            tile("c", "Flexible"),
                            tile("d", "Alpha"),
                            tile("e", "120 px"),
                            tile("f", "A longer description wraps within its column."),
                            tile("g", "Beta"),
                            tile("h", "120 px"),
                            tile("i", "Shared column alignment"),
                        ],
                    )?,
                ),
            ],
        ),
        section(
            "containers",
            "Container and Group",
            [
                label(
                    "help",
                    "Transparent by default; this example adds a fill, padding and explicit size limits.",
                ),
                Element::new(
                    "box",
                    Container::new(Element::new(
                        "body",
                        Label::new("Padded and constrained")
                            .color_role(ColorRole::OnPrimaryContainer),
                    ))
                    .padding(Length::from(16))
                    .background_role(ColorRole::PrimaryContainer)
                    .limits(Size::from([200, 64]), Some(Size::from([320, 120])))?,
                ),
                Element::new(
                    "untitled",
                    Group::new(label(
                        "body",
                        "A group without a heading uses the same spacing.",
                    )),
                ),
            ],
        ),
    ]))
}
const CONTENT_SPLIT_HELP: &str = "Content rules follow the label until a drag commits a fixed extent; cancelling restores the previous rule.";
pub(crate) fn splits(ui: &Runtime) -> Result<Element, RuntimeError> {
    let outer = ui.signal(true);
    let inner = ui.signal(true);
    let content = ui.signal("Short content".to_owned());
    let content_writer = content.clone();
    let mut examples = vec![
        label(
            "help",
            "Drag a divider or focus it with Tab and use arrows (Shift: larger steps). Release commits; Escape cancels. Locks preserve sizing preferences.",
        ),
        toggle("outer-lock", "Toggle outer resizing", &outer)?,
        toggle("inner-lock", "Toggle inner resizing", &inner)?,
        stage(
            "nested-stage",
            380,
            Element::new(
                "outer-split",
                SplitPane::new(
                    SplitAxis::TopBottom,
                    Element::new(
                        "middle-split",
                        SplitPane::new(
                            SplitAxis::LeftRight,
                            tile("editor", "Ratio 60% / 40% — independently resizable"),
                            Element::new(
                                "locked-split",
                                SplitPane::new(
                                    SplitAxis::TopBottom,
                                    tile("details", "Locked first extent: 100 px"),
                                    tile("preview", "Preview fills the rest"),
                                )
                                .sizing(SplitSizing::first(100.into()))
                                .locked(),
                            ),
                        )
                        .sizing(SplitSizing::ratio(0.6)?)
                        .resizable(inner.read_only()),
                    ),
                    tile("log", "Outer second extent: 90 px"),
                )
                .sizing(SplitSizing::second(90.into()))
                .resizable(outer.read_only()),
            ),
        ),
        label(
            "rules-help",
            "Sizing rules below are separate examples. Resize the window, drag, then cancel with Escape to compare them.",
        ),
    ];
    for (key, title, rule) in [
        ("equal", "Equal shares (default)", SplitSizing::EQUAL),
        (
            "first",
            "Fixed first extent: 180 px",
            SplitSizing::first(180.into()),
        ),
        (
            "second",
            "Fixed second extent: 180 px",
            SplitSizing::second(180.into()),
        ),
        ("ratio", "Locked ratio: 30% / 70%", SplitSizing::ratio(0.3)?),
    ] {
        let split = SplitPane::new(
            SplitAxis::LeftRight,
            tile("a", title),
            tile("b", "Second pane"),
        )
        .sizing(rule);
        let split = if key == "ratio" {
            split.locked()
        } else {
            split
        };
        examples.push(stage(key, 120, Element::new("split", split)));
    }
    examples.push(Element::new(
        "change-content",
        Button::new("Toggle content width", move |_| {
            let next = if content_writer.get()? == "Short content" {
                "Longer content changes the preferred edge width"
            } else {
                "Short content"
            };
            content_writer.set(next.into()).map(|_| ())
        }),
    ));
    for (key, rule) in [
        ("content-first", SplitSizing::CONTENT_FIRST),
        ("content-second", SplitSizing::CONTENT_SECOND),
    ] {
        examples.push(label(
            &format!("{key}-title"),
            if key == "content-first" {
                "Content-sized first pane"
            } else {
                "Content-sized second pane"
            },
        ));
        examples.push(stage(
            key,
            110,
            Element::new(
                "split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new("first", Label::new(content.read_only())),
                    Element::new("second", Label::new(content.read_only())),
                )
                .sizing(rule),
            ),
        ));
    }
    examples.push(label("content-help", CONTENT_SPLIT_HELP));
    examples.push(stage(
        "limits",
        140,
        Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new(
                    "first",
                    Container::new(label("text", "Width 100–180 px"))
                        .limits(Size::from([100, 60]), Some(Size::from([180, 140])))?,
                ),
                Element::new(
                    "second",
                    Container::new(label(
                        "text",
                        "Width 120–240 px; spare space remains after both maxima",
                    ))
                    .limits(Size::from([120, 60]), Some(Size::from([240, 140])))?,
                ),
            ),
        ),
    ));
    Ok(page([section(
        "splits",
        "Independent nested splits and sizing rules",
        examples,
    )]))
}
pub(crate) fn region_layouts(ui: &Runtime) -> Result<Element, RuntimeError> {
    let permissions: Vec<_> = (0..4).map(|_| ui.signal(true)).collect();
    let whole = ui.signal(true);
    let mut examples = vec![label(
        "help",
        "All five regions are present. The center contains another RegionLayout. Each outer edge can be locked independently, and the layout-wide switch locks every edge at once: the stricter setting wins, so an edge resizes only while both allow it. Each region scrolls overflow.",
    )];
    examples.push(toggle("lock-all", "Allow resizing any edge", &whole)?);
    for (i, name) in ["top", "bottom", "left", "right"].into_iter().enumerate() {
        examples.push(toggle(
            &format!("lock-{name}"),
            &format!("Toggle {name} resizing"),
            &permissions[i],
        )?);
    }
    examples.push(stage(
        "shell-stage",
        420,
        Element::new(
            "outer-shell",
            RegionLayout::new()
                .top(tile("global-tools", "Top — full-width global tools"))
                .top_height(64.into())
                .top_resizable(permissions[0].read_only())
                .bottom(tile("global-status", "Bottom — full-width global status"))
                .bottom_height(48.into())
                .bottom_resizable(permissions[1].read_only())
                .left(tile("navigation", "Left navigation"))
                .left_width(140.into())
                .left_resizable(permissions[2].read_only())
                .right(tile("inspector", "Right inspector"))
                .right_width(160.into())
                .right_resizable(permissions[3].read_only())
                .resizable(whole.read_only())
                .center(Element::new(
                    "inner-shell",
                    RegionLayout::new()
                        .top(label(
                            "local-tools",
                            "Local toolbar (content-sized, locked by default)",
                        ))
                        .center(Element::new(
                            "local-content",
                            Group::titled("Nested center", counter(ui, "local-counter")?),
                        ))
                        .bottom(label("local-status", "Local status")),
                )),
        ),
    ));
    examples.push(stage(
        "sparse-shell",
        100,
        Element::new(
            "sparse",
            RegionLayout::new().top(tile("only", "A lone top region: no empty-slot divider")),
        ),
    ));
    Ok(page([section(
        "shells",
        "RegionLayout inside RegionLayout",
        examples,
    )]))
}
const PULSE_HELP: &str =
    "The highlighted cell advances while running; hide this view or switch tabs to suspend.";
pub(crate) fn state(
    ui: &Runtime,
    commands: [&CommandId; 2],
    receipt: ReadSignal<Option<PresentationReceipt>>,
) -> Result<Element, RuntimeError> {
    let shown = ui.signal(true);
    let running = ui.signal(false);
    let [detach, guarded] = commands.map(CommandId::clone);
    // The latest button invocation of either command, shown with its outcome.
    let invocation = ui.signal(None::<CommandReceipt>);
    let command_status = invocation.read_only();
    let guarded_invocation = invocation.clone();
    Ok(page([
        section(
            "visibility",
            "Retained View visibility",
            [
                label(
                    "help",
                    "Increment, hide, then show: the count is retained. Start updates and hide this view or switch the main tab to suspend them.",
                ),
                toggle("show-view", "Show / hide retained view", &shown)?,
                toggle("run-pulse", "Start / stop updates", &running)?,
                Element::new(
                    "retained-view",
                    View::new(Element::new(
                        "content",
                        Column::new([
                            counter(ui, "retained-counter")?,
                            label("pulse-help", PULSE_HELP),
                            Element::new("pulse", Pulse::new(running.read_only())),
                        ]),
                    ))
                    .visible_signal(shown.read_only()),
                ),
            ],
        ),
        section(
            "detach-view",
            "Detach a single view",
            [
                label(
                    "detach-help",
                    "Tabs are not the only detachable unit. The first View moves to its own native window through the same public request; its counter keeps counting there, and closing that window docks it again. The second View sets Element::with_detachable(false), so the identical request is refused.",
                ),
                Element::new(
                    "detach-action",
                    Button::new("Open this view in a window (Primary+D)", move |cx| {
                        let receipt =
                            cx.invoke_command(&detach, cx.command_context(CommandSource::Button))?;
                        invocation.set(Some(receipt)).map(|_| ())
                    }),
                ),
                Element::new(
                    "detach-status",
                    ReceiptReadout::new(
                        receipt,
                        PresentationReceipt::status_binding,
                        |status| format!("Window operation: {status:?}"),
                        "No window operation requested yet.",
                    ),
                ),
                Element::new(
                    "guarded-action",
                    Button::new("Try the protected view", move |cx| {
                        let receipt =
                            cx.invoke_command(&guarded, cx.command_context(CommandSource::Button))?;
                        guarded_invocation.set(Some(receipt)).map(|_| ())
                    }),
                ),
                Element::new(
                    "command-status",
                    ReceiptReadout::new(
                        command_status,
                        CommandReceipt::status_binding,
                        |status| match status {
                            CommandStatus::Pending => "Button command: pending".to_owned(),
                            CommandStatus::Finished(Ok(_)) => "Button command: done".to_owned(),
                            CommandStatus::Finished(Err(error)) => {
                                format!("Button command failed: {error}")
                            }
                            CommandStatus::Cancelled => "Button command: cancelled".to_owned(),
                        },
                        "No button command invoked yet.",
                    ),
                ),
                Element::new(
                    "detachable-view",
                    View::new(Element::new(
                        "detachable-content",
                        Group::titled("Retained view", counter(ui, "detachable-counter")?),
                    )),
                ),
                Element::new(
                    "guarded-view",
                    View::new(Element::new(
                        "guarded-content",
                        Group::titled(
                            "Protected view",
                            label("guarded-text", "This view never leaves the window."),
                        ),
                    )),
                )
                .with_detachable(false),
            ],
        ),
        section(
            "tabs",
            "Nested Tabs with independent state",
            [
                label(
                    "help",
                    "Each tab has its own counter. Switch and return. Arrow keys select focused tabs; Ctrl+Tab switches from their content. Narrow titles scroll with arrows or the wheel. Drag a title along the strip, or press Alt with Left or Right, to reorder; drag it off the strip to detach it.",
                ),
                stage("tabs-stage", 200, nested_tabs(ui)?),
            ],
        ),
    ]))
}
/// Six retained views in a strip that is both detachable and reorderable. The
/// application owns the order: a move edits it and describes the tabs again,
/// and each view keeps its key, and with it its count.
fn nested_tabs(ui: &Runtime) -> Result<Element, RuntimeError> {
    let counts: Rc<Vec<Signal<u64>>> = Rc::new((0..6).map(|_| ui.signal(0u64)).collect());
    let order = Rc::new(RefCell::new((0..6).collect::<Vec<usize>>()));
    let describe = {
        let counts = counts.clone();
        move |order: &[usize]| -> Result<Vec<Tab>, RuntimeError> {
            order
                .iter()
                .map(|&i| {
                    Ok(Tab::new(
                        format!("view-{i}"),
                        format!("Retained view {}", i + 1),
                        counter_of(counts[i].clone(), "counter")?,
                    )
                    // The last view is excluded, so one strip shows both states.
                    .detachable(i < 5))
                })
                .collect()
        }
    };
    let tabs = describe(&order.borrow())?;
    Ok(Element::new(
        "nested-tabs",
        Tabs::new(tabs)
            .detachable(true)
            .reorderable(move |cx, drop| {
                let index = |key: &str| key.strip_prefix("view-")?.parse::<usize>().ok();
                let Some(moved) = drop.items().first().and_then(|key| index(key)) else {
                    return Ok(false);
                };
                let mut order = order.borrow_mut();
                let Some(from) = order.iter().position(|&i| i == moved) else {
                    return Ok(false);
                };
                // Resolved before the removal, since the position may name the
                // moved tab itself, which leaves the order as it is.
                let place = |key: &str| index(key).and_then(|t| order.iter().position(|&i| i == t));
                let boundary = match drop.position() {
                    DropPosition::Before(key) => place(key),
                    DropPosition::After(key) => place(key).map(|position| position + 1),
                    _ => None,
                }
                .unwrap_or(order.len());
                order.remove(from);
                order.insert(boundary - usize::from(from < boundary), moved);
                let strip = cx.id();
                let tabs: Vec<Element> = describe(&order)?
                    .into_iter()
                    .map(Tab::into_element)
                    .collect();
                let _task = cx.defer(move |ui| ui.reconcile_children(&strip, tabs))?;
                Ok(true)
            }),
    ))
}
pub(crate) fn scrolling(status: &Signal<String>) -> Result<Element, RuntimeError> {
    Ok(page([
        section(
            "fixed",
            "Fixed View",
            [
                label(
                    "help",
                    "This viewport stays at local zero and clips overflow. It does not scroll; place a ScrollArea inside a fixed frame when content should scroll independently.",
                ),
                stage(
                    "fixed-stage",
                    80,
                    Element::new(
                        "fixed-view",
                        View::new(Element::new(
                            "items",
                            Column::new((0..6).map(|i| {
                                label(
                                    &format!("fixed-{i}"),
                                    format!("Fixed content line {}", i + 1),
                                )
                            })),
                        ))
                        .fixed(),
                    ),
                ),
            ],
        ),
        section(
            "vertical",
            "Vertical ScrollArea (default)",
            [
                label(
                    "help",
                    "Wheel or focus the viewport and use arrows/Page Up/Page Down/Home/End. Tab automatically reveals offscreen buttons.",
                ),
                stage(
                    "vertical-stage",
                    180,
                    Element::new(
                        "vertical-scroll",
                        ScrollArea::new(Element::new(
                            "items",
                            Column::new((0..30).map(|i| {
                                action(
                                    &format!("item-{i}"),
                                    format!("Vertical item {}", i + 1),
                                    status,
                                )
                            })),
                        )),
                    ),
                ),
            ],
        ),
        section(
            "horizontal",
            "Horizontal ScrollArea",
            [stage(
                "horizontal-stage",
                100,
                Element::new(
                    "horizontal-scroll",
                    ScrollArea::new(Element::new(
                        "items",
                        Row::new((0..12).map(|i| {
                            tile(&format!("item-{i}"), format!("Horizontal card {}", i + 1))
                        })),
                    ))
                    .axes(ScrollAxes::Horizontal),
                ),
            )],
        ),
        section(
            "both",
            "ScrollArea on both axes",
            [stage(
                "both-stage",
                200,
                Element::new(
                    "both-scroll",
                    ScrollArea::new(Element::new(
                        "grid",
                        Grid::new(
                            [Track::fixed(220.into()); 5],
                            (0..40).map(|i| {
                                Element::new(
                                    format!("cell-{i}"),
                                    Container::new(action(
                                        "action",
                                        format!("Cell {}", i + 1),
                                        status,
                                    ))
                                    .size_limits(SizeLimits::at_least(Size::from([220, 48]))),
                                )
                            }),
                        )?,
                    ))
                    .axes(ScrollAxes::Both),
                ),
            )],
        ),
    ]))
}
