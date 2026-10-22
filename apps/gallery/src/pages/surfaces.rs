//! The Surfaces category: dialogs and notifications, floating windows and
//! the menu bar.
use crate::gallery::{label, page, section, stage};
use crate::samples::Canvas;
use pert_ui::*;

/// An application menu bar over a small document: command rows with live
/// shortcuts and enablement, submenus, separators, check and radio settings and
/// a title that runs an action directly.
/// A radio row's `on_change`: reports the user's choice on the status line.
fn chose(
    status: &Signal<String>,
    zoom: &'static str,
) -> impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static {
    let status = status.clone();
    move |_, _| status.set(format!("You chose {zoom}."))
}

pub(crate) fn menu_bar(ui: &mut Runtime) -> Result<Element, AppError> {
    let status = ui.signal("Press a title, or focus the document and press F10.".to_owned());
    let dirty = ui.signal(false);
    let grid = ui.signal(true);
    let zoom = ui.signal(100u32);
    let sketches = ui.signal(0u32);
    // Commands own the shortcuts; the menu rows present them and follow their
    // enablement, and the chords work without opening a menu.
    let (new_status, new_dirty, count) = (status.clone(), dirty.clone(), sketches.clone());
    let new = ui.register_command(
        "New sketch",
        CommandScope::Application,
        vec![KeyChord::primary('n')],
        move |_, _| {
            let number = count.get()? + 1;
            count.set(number)?;
            new_dirty.set(true)?;
            new_status.set(format!("Created sketch {number}; Save is now available."))
        },
    )?;
    let (save_status, save_dirty) = (status.clone(), dirty.clone());
    let save = ui.register_command(
        "Save",
        CommandScope::Application,
        vec![KeyChord::primary('s')],
        move |_, _| {
            save_dirty.set(false)?;
            save_status.set("Saved.".to_owned())
        },
    )?;
    ui.command_enabled_signal(&save, dirty.read_only())?;
    // A refused or failed invocation is reported, never lost.
    let receipts = |status: &Signal<String>| {
        let status = status.clone();
        move |_: &mut EventContext, receipt: CommandReceipt| {
            if let CommandStatus::Finished(Err(error)) = receipt.status() {
                status.set(format!("Command failed: {error}"))?;
            }
            Ok(())
        }
    };
    let recent = |key: &str, name: &str| {
        let status = status.clone();
        let name = name.to_owned();
        MenuItem::new(key, name.clone(), move |_| {
            status.set(format!("Opened {name}."))
        })
    };
    let about_status = status.clone();
    let sync_status = status.clone();
    let sync = Tooltip::new(
        Element::new(
            "menu-bar-sync",
            Button::icon(Icon::Refresh, "Synchronize", move |_| {
                sync_status.set("Synchronized from the bar's trailing slot.".to_owned())
            })
            .frameless(),
        ),
        "Synchronize",
    );
    let bar = MenuBar::new(
        ui,
        vec![
            MenuItem::submenu(
                "file",
                "File",
                vec![
                    MenuItem::command(ui, "new", &new, receipts(&status))?,
                    MenuItem::submenu(
                        "recent",
                        "Open recent",
                        vec![
                            recent("first", "first.sketch"),
                            recent("second", "second.sketch"),
                        ],
                    ),
                    MenuItem::separator(),
                    MenuItem::command(ui, "save", &save, receipts(&status))?
                        .leading_icon(Icon::Save),
                ],
            ),
            MenuItem::submenu(
                "view",
                "View",
                vec![
                    // `on_change` reports the user's own choice, never an
                    // application write.
                    MenuItem::toggle("grid", grid.clone(), "Show grid").on_change({
                        let status = status.clone();
                        move |_, shown| {
                            status.set(format!(
                                "You turned the grid {}.",
                                if shown { "on" } else { "off" }
                            ))
                        }
                    }),
                    MenuItem::separator(),
                    MenuItem::radio("zoom-50", zoom.clone(), 50, "50%")
                        .on_change(chose(&status, "50%")),
                    MenuItem::radio("zoom-100", zoom.clone(), 100, "100%")
                        .on_change(chose(&status, "100%")),
                    MenuItem::radio("zoom-200", zoom.clone(), 200, "200%")
                        .on_change(chose(&status, "200%")),
                ],
            ),
            MenuItem::new("about", "About", move |_| {
                about_status
                    .set("pert-ui menu bar: a title can also run an action.".to_owned())
                    .map(|_| ())
            }),
        ],
    )?
    .trailing(Element::new("menu-bar-sync-tip", sync));
    let document = Column::new([
        Element::new(
            "document-focus",
            Button::new("Document: focus me, then press F10", |_| Ok(())),
        ),
        Element::new("menu-bar-status", Label::new(status.read_only())),
        Element::new(
            "grid-setting",
            Label::new(
                grid.map(|grid| format!("Grid {}.", if *grid { "shown" } else { "hidden" }))?,
            ),
        ),
        Element::new(
            "zoom-setting",
            Label::new(zoom.map(|zoom| format!("Zoom {zoom}%."))?),
        ),
    ])
    .gap(MetricRole::ItemGap);
    Ok(page([
        label(
            "menu-bar-help",
            "Titles open on press; while one is open, pointing at another switches. F10 moves focus to the titles, Left/Right move between them and Down opens. Inside a menu, Left/Right at the edge move to the neighboring menu. Escape closes one level; choosing a command returns focus to the document. Shortcuts are the commands' own chords and also work with the menus closed. The synchronize button at the right of the bar sits in its trailing slot: an ordinary Tab stop that F10 and the title arrows skip.",
        ),
        stage(
            "menu-bar-stage",
            260,
            Element::new(
                "menu-bar",
                bar.content(Element::new(
                    "document",
                    Container::new(Element::new("document-content", document))
                        .padding(MetricRole::ContentMargin),
                )),
            ),
        ),
    ]))
}
pub(crate) fn transient_surfaces(ui: &Runtime) -> Result<Element, RuntimeError> {
    let dialog = DialogController::new(ui);
    let alert = DialogController::new(ui);
    let nested = DialogController::new(ui);
    let local = DialogController::new(ui);
    let banner = ui.signal(true);
    let queue = ToastController::new(ui);
    let status = ui.signal("Try dialogs, nested menus and notifications.".to_owned());
    fn opener(key: &str, title: &str, state: &DialogController) -> Element {
        let state = state.clone();
        Element::new(
            key,
            Button::new(title, move |_| {
                // This demo displays each retained outcome before the next opening.
                state.take_outcome()?;
                state.open()
            }),
        )
    }
    let accept = dialog.clone();
    let cancel = dialog.clone();
    let banner_close = banner.clone();
    let banner_show = banner.clone();
    let toast_queue = queue.clone();
    let action_queue = queue.clone();
    let toast_status = status.clone();
    let context_status = status.clone();
    let menu_status = status.clone();
    let content = page([
        section(
            "dialogs",
            "Modal task and safe confirmation",
            [
                Element::new(
                    "task-dialog",
                    Dialog::new(
                        dialog.clone(),
                        opener("open-task", "Open dialog", &dialog),
                        Element::new(
                            "task",
                            Column::new([
                                label("heading", "Dialog content uses ordinary components"),
                                Element::new(
                                    "draft",
                                    TextField::new(ui.signal("Editable local draft".to_owned())),
                                ),
                                Element::new(
                                    "nested-confirmation",
                                    AlertDialog::new(
                                        nested.clone(),
                                        opener("open-nested", "Nested confirmation", &nested),
                                        "Nested confirmation",
                                        "Escape closes this level first.",
                                        "Confirm",
                                    ),
                                ),
                                Element::new(
                                    "task-actions",
                                    Row::new([
                                        Element::new(
                                            "task-cancel",
                                            Button::new("Cancel", move |_| {
                                                cancel.cancel().map(|_| ())
                                            }),
                                        ),
                                        Element::new(
                                            "task-accept",
                                            Button::new("Accept", move |_| {
                                                accept.accept().map(|_| ())
                                            }),
                                        ),
                                    ]),
                                ),
                            ]),
                        ),
                    ),
                ),
                Element::new(
                    "task-result",
                    Label::new(
                        dialog
                            .status()
                            .map(|status| format!("Dialog: {status:?}"))?,
                    ),
                ),
                Element::new(
                    "alert-dialog",
                    AlertDialog::new(
                        alert.clone(),
                        opener("open-alert", "Open confirmation", &alert),
                        "Delete example item?",
                        "Cancel has initial focus. This demo does not delete application data.",
                        "Delete",
                    ),
                ),
                Element::new(
                    "alert-result",
                    Label::new(alert.status().map(|status| format!("Dialog: {status:?}"))?),
                ),
            ],
        ),
        section(
            "local-dialog",
            "View modality leaves other sections interactive",
            [stage(
                "local-stage",
                210,
                Element::new(
                    "local-owner",
                    Dialog::new(
                        local.clone(),
                        Element::new(
                            "local-view",
                            Column::new([
                                label("local-help", "Only this view is blocked."),
                                opener("open-local", "Open view dialog", &local),
                            ]),
                        ),
                        Element::new(
                            "local-close",
                            Button::new("Close view dialog", {
                                let local = local.clone();
                                move |_| local.cancel().map(|_| ())
                            }),
                        ),
                    )
                    .view_modal(),
                ),
            )],
        ),
        section(
            "nested-menus",
            "Context menu and nested action menus",
            [
                Element::new(
                    "nested-menu",
                    Menu::new(
                        ui,
                        "Nested actions",
                        vec![
                            MenuItem::new("disabled", "Unavailable", |_| Ok(())).disabled(),
                            MenuItem::submenu(
                                "more",
                                "More",
                                vec![MenuItem::submenu(
                                    "deeper",
                                    "More options",
                                    vec![MenuItem::new("apply", "Apply", move |_| {
                                        menu_status.set("Nested action applied.".into())
                                    })],
                                )],
                            ),
                        ],
                    )?,
                ),
                {
                    let context_menu = ContextMenu::new(
                        ui,
                        Element::new(
                            "context-target",
                            Button::new("Right-click here, or use Shift+F10", |_| Ok(())),
                        ),
                        vec![MenuItem::submenu(
                            "actions",
                            "Actions",
                            vec![MenuItem::new("inspect", "Inspect", move |_| {
                                context_status
                                    .set("Context action selected.".into())
                                    .map(|_| ())
                            })],
                        )],
                    )?;
                    let opener = context_menu.controller();
                    Element::new(
                        "context-row",
                        Row::new([
                            Element::new("context-example", context_menu),
                            Element::new(
                                "context-open",
                                Button::new("Open it from code", move |cx| {
                                    // The same menu, opened by a button with the
                                    // context the button's press captured.
                                    opener
                                        .open(None, &cx.command_context(CommandSource::Menu))
                                        .map(|_| ())
                                }),
                            ),
                        ]),
                    )
                },
            ],
        ),
        section(
            "notifications",
            "Persistent banner and timed notifications",
            [
                Element::new(
                    "banner",
                    Banner::new(
                        "This banner stays until dismissed.",
                        [Element::new(
                            "dismiss-banner",
                            Button::new("Dismiss", move |_| banner_close.set(false)),
                        )],
                    )
                    .visible_signal(banner.read_only()),
                ),
                Element::new(
                    "show-banner",
                    Button::new("Restore banner", move |_| banner_show.set(true)),
                ),
                Element::new(
                    "post-toast",
                    Button::new("Show notification", move |_| {
                        let _receipt = toast_queue.push(Toast::new(
                            "A notification with five seconds of visible lifetime.",
                        ))?;
                        Ok(())
                    }),
                ),
                Element::new(
                    "post-action-toast",
                    Button::new("Show actionable notification", move |_| {
                        let status = toast_status.clone();
                        let _receipt = action_queue.push(
                            Toast::new("Hover or focus pauses expiry.")
                                .action("Apply", move |_| {
                                    status.set("Notification action applied.".into())
                                }),
                        )?;
                        Ok(())
                    }),
                ),
                label(
                    "toast-help",
                    format!(
                        "Up to {} visible notifications; up to {} waiting. Tab/Up/Down navigate, Enter acts, Delete dismisses.",
                        ToastController::MAX_VISIBLE,
                        ToastController::MAX_PENDING,
                    ),
                ),
            ],
        ),
        Element::new("transient-status", Label::new(status.read_only())),
    ]);
    Ok(Element::new(
        "notification-host",
        ToastHost::new(queue, content),
    ))
}

const WINDOW_NOTES_TEXT: &str = "This editor keeps its text and scroll state when minimized.
Tabs, splits and modal surfaces use the same public API inside a window.";
const WINDOWS_HELP: &str = "Overlapping windows: drag titles, resize corners, or minimize. Closed windows are removed; restart the gallery to recreate them.";
const RESTORED_MESSAGE: &str = "Restored minimized windows. Closed windows stay closed.";
pub(crate) fn floating_windows(ui: &Runtime) -> Result<Element, AppError> {
    let message = ui.signal("Drag a title bar; resize at the bottom-right corner.".to_owned());
    let value = ui.signal(1.0_f64);
    let enabled = ui.signal(true);
    let lock = ui.signal(true);
    let info = message.clone();
    let menu_message = message.clone();
    let output = FloatingWindow::new(
        ui,
        "output-window",
        "Out-1",
        Element::new(
            "output-content",
            Container::new(Element::new(
                "output-controls",
                Column::new([
                    Element::new(
                        "output-level",
                        InlineDecimal::new(value.clone()).label("Level"),
                    ),
                    Element::new(
                        "output-options",
                        Row::new([
                            Element::new(
                                "output-enabled",
                                Checkbox::new(enabled.clone(), "Enabled"),
                            ),
                            Element::new("output-muted", Toggle::new(ui.signal(false), "Mute")),
                        ]),
                    ),
                    Element::new(
                        "output-pan",
                        InlineDecimal::new(ui.signal(0.0)).label("Balance"),
                    ),
                    Element::new(
                        "output-footer",
                        Row::new([
                            Element::new(
                                "download",
                                Button::icon(Icon::Download, "Export", |_| Ok(())),
                            ),
                            Element::new("link", Button::icon(Icon::Link, "Link", |_| Ok(()))),
                        ]),
                    ),
                ]),
            ))
            .padding(MetricRole::GroupPadding),
        ),
    )
    .position(Point::new(20.0, 20.0)?)?
    .initial_size(Size::from([340, 240]))?
    .size_limits(SizeLimits::at_least(Size::from([280, 180])))
    .minimizable(true)
    .resizable(lock.read_only())
    .title_action(Element::new(
        "output-info",
        Tooltip::new(
            Element::new(
                "info",
                Button::icon(Icon::Info, "Information", move |_| {
                    info.set("The window contains ordinary numeric controls and bindings.".into())
                })
                .frameless(),
            ),
            "Information",
        ),
    ))
    .title_action(Element::new(
        "output-menu",
        Menu::icon(
            ui,
            Icon::MoreHorizontal,
            "More actions",
            vec![MenuItem::new("inspect", "Inspect", move |_| {
                menu_message
                    .set("Window actions use ordinary menus.".into())
                    .map(|_| ())
            })],
        )?,
    ));
    let first_state = output.controller();
    let dialog = DialogController::new(ui);
    let open_dialog = dialog.clone();
    let cancel = dialog.clone();
    let notes = ui.signal(WINDOW_NOTES_TEXT.to_owned());
    let properties = FloatingWindow::new(
        ui,
        "properties-window",
        "Properties",
        Element::new(
            "properties-tabs",
            Tabs::new([
                Tab::new(
                    "properties-general",
                    "General",
                    Element::new(
                        "properties-dialog",
                        Dialog::new(
                            dialog,
                            Element::new(
                                "properties-general-content",
                                Container::new(Element::new(
                                    "properties-form",
                                    Column::new([
                                        Element::new(
                                            "properties-name",
                                            TextField::new(ui.signal("Untitled view".to_owned())),
                                        )
                                        .with_label("View name"),
                                        Element::new(
                                            "properties-value",
                                            InlineDecimal::new(value).label("Shared level"),
                                        ),
                                        Element::new(
                                            "properties-enabled",
                                            Checkbox::new(enabled, "Enabled"),
                                        ),
                                        Element::new(
                                            "open-window-dialog",
                                            Button::new("Open local dialog", move |_| {
                                                open_dialog.take_outcome()?;
                                                open_dialog.open()
                                            }),
                                        ),
                                    ]),
                                ))
                                .padding(MetricRole::GroupPadding),
                            ),
                            Element::new(
                                "dialog-body",
                                Column::new([
                                    Element::new(
                                        "dialog-text",
                                        Label::new("Dialog inside a floating window"),
                                    ),
                                    Element::new(
                                        "dialog-dismiss",
                                        Button::new("Close dialog", move |_| {
                                            cancel.cancel().map(|_| ())
                                        }),
                                    ),
                                ]),
                            ),
                        )
                        .view_modal(),
                    ),
                ),
                Tab::new(
                    "properties-editor",
                    "Editor",
                    Element::new(
                        "editor-split",
                        SplitPane::new(
                            SplitAxis::LeftRight,
                            Element::new("notes", TextArea::new(notes)),
                            Element::new(
                                "preview",
                                Group::titled("Preview", Element::new("drawing", Canvas::new()?)),
                            ),
                        ),
                    ),
                ),
            ])
            .detachable(true),
        ),
    )
    .position(Point::new(280.0, 110.0)?)?
    .initial_size(Size::from([400, 300]))?
    .size_limits(SizeLimits::at_least(Size::from([300, 220])))
    .minimizable(true);
    let properties_state = properties.controller();
    let palette = FloatingWindow::new(
        ui,
        "palette-window",
        "Quick controls",
        Element::new(
            "palette-controls",
            Group::new(Element::new(
                "palette-actions",
                Row::new([
                    Element::new("palette-save", Button::icon(Icon::Save, "Save", |_| Ok(()))),
                    Element::new(
                        "palette-settings",
                        Button::icon(Icon::Settings, "Settings", |_| Ok(())),
                    ),
                    Element::new("palette-add", Button::icon(Icon::Add, "Add", |_| Ok(()))),
                ]),
            )),
        ),
    )
    .position(Point::new(60.0, 290.0)?)?
    .decorated(false)
    .drag_area(Element::new(
        "palette-drag",
        Label::new("Quick controls — drag here"),
    ))
    .fixed_size(Size::from([280, 110]))?;
    let restore_message = message.clone();
    Ok(Element::new(
        "windows-page",
        Container::new(Element::new(
            "windows-regions",
            RegionLayout::new()
                .fixed()
                .top(Element::new(
                    "windows-toolbar",
                    Column::new([
                        label("windows-help", WINDOWS_HELP),
                        Element::new(
                            "window-options",
                            Row::new([
                                Element::new(
                                    "window-resize-permission",
                                    Checkbox::new(lock, "Allow Out-1 resizing"),
                                ),
                                Element::new(
                                    "restore-windows",
                                    Button::new("Restore minimized", move |_| {
                                        for state in [&first_state, &properties_state] {
                                            if state.status().get()? == WindowStatus::Minimized {
                                                state.restore()?;
                                            }
                                        }
                                        restore_message.set(RESTORED_MESSAGE.into())
                                    }),
                                ),
                            ]),
                        ),
                    ]),
                ))
                .center(Element::new(
                    "window-layer",
                    WindowLayer::new([output, properties, palette]),
                ))
                .bottom(Element::new(
                    "window-message",
                    Label::new(message.read_only()),
                )),
        ))
        .padding(MetricRole::ContentMargin),
    ))
}
