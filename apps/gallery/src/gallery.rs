//! The gallery shell: its menu bar, category strips and status bar, and the
//! small helpers the pages in `crate::pages` share.
use crate::brand::BrandMark;
use crate::history::GalleryHistory;
use crate::pages::{
    chips::chips_and_breadcrumbs,
    controls::{
        adaptive_form, colors, controls, field_status, input, parameter_families, progress,
        toolbar_page, typography,
    },
    data::{collections, drag_and_drop, file_browser, images, vector_shapes, virtual_list},
    layout::{
        NavigationTargets, layout, navigation, region_layouts, scrolling, sections, splits, state,
        tool_windows, transforms,
    },
    properties::property_grid,
    surfaces::{floating_windows, menu_bar, transient_surfaces},
    text::{fonts_and_icons, number_inputs, text_editing, text_rendering_page},
    tools::{debug_overlay, profiling, themes, workspace_page},
};
use crate::search::{SearchTargets, search_items, search_picker};
use pert_ui::*;
use std::{cell::RefCell, rc::Rc};

/// The gallery's strip of categories, found once by its key below the root.
///
/// The frame around the strip (the dialogs, the palette, the search and the
/// history buttons) is the gallery's own composition and gains wrappers as
/// features are added, so it is not spelled out as an address; the key is
/// unique, and [`Runtime::find_descendant`] fails rather than guessing if it
/// ever repeats. Everything below the strip is found by its exact address.
pub(crate) fn gallery_tabs(ui: &Runtime, root: &WidgetId) -> Result<WidgetId, AppError> {
    ui.find_descendant(root, "gallery-tabs")?.ok_or_else(|| {
        ApplicationError::from("the gallery's category strip is not mounted".to_owned()).into()
    })
}

/// The address, below the category strip, of `path` inside the page `page`
/// of `category`: the category's tab, its strip of pages, the page's tab and
/// the page frame [`page`] builds. Pages reuse keys such as `help`, so only a
/// full address names one component.
pub(crate) fn page_address<'a>(category: &'a str, page: &'a str, path: &[&'a str]) -> Vec<&'a str> {
    let mut address = vec![category, "category-pages", page, "page", "sections"];
    address.extend_from_slice(path);
    address
}

/// The component at `address` below `from`, through
/// [`Runtime::find`]. A missing component is an application error
/// naming the address: the gallery's commands and links need these
/// components, and a composition change that moved one must fail at mount
/// rather than leave them doing nothing.
pub(crate) fn find_at(
    ui: &Runtime,
    from: &WidgetId,
    address: &[&str],
) -> Result<WidgetId, AppError> {
    ui.find(from, address)?.ok_or_else(|| {
        ApplicationError::from(format!(
            "the gallery has nothing at `{}`",
            address.join(" › ")
        ))
        .into()
    })
}

/// Finds a view of the State & tabs page's detach section; the detach
/// commands act on it.
fn view_of(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, AppError> {
    let top = gallery_tabs(ui, root)?;
    find_at(
        ui,
        &top,
        &page_address("layout-category", "state", &["detach-view", "content", key]),
    )
}

pub(super) struct Gallery {
    inline_id: ParameterId,
    strip_ids: [ParameterId; 3],
    id: ParameterId,
    property_id: ParameterId,
    spec: ParameterSpec<f32>,
    /// The live pages' producer thread, which the host joins after `App::run`.
    producer: crate::live::ProducerHandle,
}
impl Gallery {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            inline_id: ParameterId::new(2, 2, "inline-glide")?,
            strip_ids: [
                ParameterId::new(2, 3, "pan")?,
                ParameterId::new(2, 4, "send")?,
                ParameterId::new(2, 5, "level")?,
            ],
            id: ParameterId::new(2, 1, "amount")?,
            property_id: ParameterId::new(2, 6, "property-level")?,
            spec: ParameterSpec::new("Amount", -100.0, 100.0, 0.0)?,
            producer: crate::live::ProducerHandle::default(),
        })
    }
    /// The handle through which the host joins the live producer thread once
    /// its event loop has returned; the thread exists after [`Self::mount`].
    pub(super) fn producer(&self) -> crate::live::ProducerHandle {
        self.producer.clone()
    }
    pub(super) fn mount(self, ui: &mut Runtime) -> Result<WidgetId, AppError> {
        ui.set_profiling(true)?;
        let status = ui.signal("Choose a tab and try its examples.".to_owned());
        // Detaching is not limited to tabs: any mounted view can move to its own
        // native window. The command needs the identity, which exists after mount.
        let detach_receipt = ui.signal(None::<PresentationReceipt>);
        let detached_view = Rc::new(RefCell::new(None::<WidgetId>));
        let guarded_view = Rc::new(RefCell::new(None::<WidgetId>));
        let command_view = detached_view.clone();
        let command_receipt = detach_receipt.clone();
        let command_status = status.clone();
        let detach_command = ui.register_command(
            "Open the retained view in its own window",
            CommandScope::Application,
            vec![KeyChord::primary('d')],
            move |ui, _| {
                let Some(view) = command_view.borrow().clone() else {
                    return command_status.set("The view to detach is not mounted yet.".to_owned());
                };
                if ui.window_of(&view)? != ui.primary_window() {
                    return command_status.set("That view already has its own window.".to_owned());
                }
                let receipt =
                    ui.request_detach(&view, NativeWindowOptions::new("Detached view")?)?;
                command_receipt.set(Some(receipt))
            },
        )?;
        // The same request refuses a view whose description forbids its own window.
        let guarded_target = guarded_view.clone();
        let guarded_status = status.clone();
        let guarded_command = ui.register_command(
            "Try to open the protected view in its own window",
            CommandScope::Application,
            Vec::new(),
            move |ui, _| {
                let Some(view) = guarded_target.borrow().clone() else {
                    return guarded_status.set("The protected view is not mounted yet.".to_owned());
                };
                let result = ui.request_detach(&view, NativeWindowOptions::new("Protected")?);
                guarded_status.set(match result {
                    Ok(_) => "The protected view was detached after all.".to_owned(),
                    Err(error) => format!("Detach refused: {error}"),
                })
            },
        )?;
        // Selecting a tab by key needs the runtime, which a command action has.
        let navigation_targets = NavigationTargets::default();
        let select_targets = navigation_targets.clone();
        let select_status = status.clone();
        let select_by_key = ui.register_command(
            "Select the General tab of the navigation demo",
            CommandScope::Application,
            Vec::new(),
            move |ui, _| {
                let strip = select_targets.get(NavigationTargets::STRIP)?;
                let outcome = ui.reveal_child(&strip, "general")?;
                select_status.set(format!("Select General by key: {outcome:?}"))
            },
        )?;
        // The command palette lists every registered command. The gallery binds
        // its own shortcut to a command that opens it, and registers one
        // "Go to" command per page, so searching the palette jumps across pages.
        let palette = CommandPaletteController::new(ui);
        let opener = palette.clone();
        let palette_command = ui.register_command(
            "Show all commands",
            CommandScope::Application,
            vec![KeyChord::primary('p').shifted()],
            move |ui, context| opener.open(ui, context),
        )?;
        // The gallery's history: the user's page changes and the Go to
        // commands are visits, and back, forward and the history list in the
        // menu bar show a place again without recording it.
        let places = GalleryHistory::new(ui)?;
        // The search lists the gallery's content and commands in one picker,
        // each row labelled by kind; typing filters it.
        let search = QuickPickController::new(ui);
        let search_targets = SearchTargets::default();
        let (search_opener, opened_targets, search_places) =
            (search.clone(), search_targets.clone(), places.clone());
        let own_command = Rc::new(RefCell::new(None::<CommandId>));
        let own = own_command.clone();
        let search_command = ui.register_command(
            "Search the gallery",
            CommandScope::Application,
            vec![KeyChord::primary('f').shifted()],
            move |ui, context| {
                let (items, targets) =
                    search_items(ui, context, own.borrow().as_ref(), &search_places)?;
                *opened_targets.borrow_mut() = targets;
                search_opener.open(items)
            },
        )?;
        *own_command.borrow_mut() = Some(search_command.clone());
        for (category, page, title) in GO_TO_PAGES {
            let (go_places, go_status) = (places.clone(), status.clone());
            // The gallery keeps its commands for its whole run.
            let _ = ui.register_command(
                format!("Go to: {title}"),
                CommandScope::Application,
                Vec::new(),
                move |ui, _| {
                    go_places.go_to(ui, category, page)?;
                    go_status.set(format!("Opened {title}."))
                },
            )?;
        }
        for (name, chord, back) in [
            ("Go back", KeyChord::navigate_back(), true),
            ("Go forward", KeyChord::navigate_forward(), false),
        ] {
            let (step_places, step_status) = (places.clone(), status.clone());
            let _ = ui.register_command(
                name,
                CommandScope::Application,
                vec![chord],
                move |ui, _| step_places.step(ui, back, &step_status),
            )?;
        }
        let menus = menu_bar(ui)?;
        let top_history = places.clone();
        // Categories at the top, each with its own strip of pages. A page
        // keeps its key, so a page moves between categories without changing
        // its identity, and an inactive category's pages stay mounted but hidden.
        let (workload, producer) =
            crate::live::Workload::with_thread(ui, false, self.producer.clone())?;
        // The producer thread stays parked until a live page's Run toggle is on,
        // and ends when the host joins it or the runtime closes the bridges.
        workload
            .spawn(producer, None)
            .map_err(ApplicationError::new)?;
        let tabs = Tabs::new([
            places.category(
                "layout-category",
                "Layout",
                [
                    places.page("layout", "Layout", layout(&status)?),
                    places.page("splits", "Split panes", splits(ui)?),
                    places.page("shells", "Nested regions", region_layouts(ui)?),
                    places.page("scroll", "Scrolling", scrolling(&status)?),
                    places.page("sections", "Sections", sections(ui)?),
                    places.page("tool-windows", "Tool windows", tool_windows(ui)?),
                    places.page("transforms", "Transforms", transforms(ui)?),
                    places.page(
                        "state",
                        "State & tabs",
                        state(
                            ui,
                            [&detach_command, &guarded_command],
                            detach_receipt.read_only(),
                        )?,
                    ),
                    places.page(
                        "navigation",
                        "Navigation",
                        navigation(ui, &navigation_targets, &select_by_key)?,
                    ),
                ],
            ),
            places.category(
                "controls-category",
                "Controls",
                [
                    places.page("text", "Text & buttons", typography(ui, &status)?),
                    places.page("controls", "Controls", controls(ui)?),
                    places.page(
                        "input",
                        "Input & parameters",
                        input(ui, self.id, self.spec, &status)?,
                    ),
                    places.page(
                        "families",
                        "Parameter families",
                        parameter_families(ui, &status)?,
                    ),
                    places.page(
                        "toolbar",
                        "Toolbar",
                        toolbar_page(ui, self.inline_id, self.strip_ids)?,
                    ),
                    places.page("fields", "Field status", field_status(ui)?),
                    places.page("forms", "Adaptive forms", adaptive_form(ui)?),
                    places.page(
                        "properties",
                        "Property grid",
                        property_grid(ui, self.property_id)?,
                    ),
                    places.page("progress", "Progress", progress(ui)),
                    places.page("colors", "Colors", colors(ui)?),
                    places.page(
                        "chips",
                        "Chips & breadcrumbs",
                        chips_and_breadcrumbs(ui, &status)?,
                    ),
                ],
            ),
            places.category(
                "text-category",
                "Text",
                [
                    places.page("editing", "Text input", text_editing(ui)?),
                    places.page("numbers", "Number input", number_inputs(ui)?),
                    places
                        .page("fonts", "Fonts & icons", fonts_and_icons(ui)?)
                        .leading_icon(Icon::Settings),
                    places.page("text-rendering", "Text rendering", text_rendering_page(ui)?),
                ],
            ),
            places.category(
                "surfaces-category",
                "Surfaces",
                [
                    places.page(
                        "surfaces",
                        "Dialogs & notifications",
                        transient_surfaces(ui)?,
                    ),
                    places.page("windows", "Floating windows", floating_windows(ui)?),
                    places.page("menus", "Menu bar", menus),
                ],
            ),
            places.category(
                "data-category",
                "Data & media",
                [
                    places.page("virtual", "Virtual list", virtual_list(ui, &status)?),
                    places.page("collections", "Collections", collections(ui)?),
                    places.page("files", "File browser", file_browser(ui)?),
                    places.page("dragdrop", "Drag & drop", drag_and_drop(ui)?),
                    places
                        .page("live", "Static & live", workload.element()?)
                        .fixed(),
                    places.page("meters", "Level & trace", workload.widgets_element()?),
                    places.page("images", "Images", images(ui)?),
                    places.page("shapes", "Vector shapes", vector_shapes()),
                ],
            ),
            places.category(
                "tools-category",
                "Appearance & tools",
                [
                    places.page("theme", "Themes", themes(ui)),
                    places.page("workspace", "Workspace", workspace_page(ui)),
                    places.page("profiling", "Profiler", profiling(ui)),
                    places.page("overlay", "Debug overlay", debug_overlay(ui)),
                ],
            ),
        ])
        .detachable(true)
        .on_select(move |_, change| top_history.category_selected(change));
        // The gallery's own menu bar holds the appearance settings and the
        // project information, with the pert-ui mark leading its title row;
        // zoom lives at the right end of the status bar.
        let about = DialogController::new(ui);
        let mark = Tooltip::new(
            Element::new("brand-mark", BrandMark::new(ui, about.clone())?),
            "About pert-ui",
        );
        let shortcuts = DialogController::new(ui);
        let guide = DialogController::new(ui);
        let menu = gallery_menu(
            ui,
            &about,
            &shortcuts,
            &guide,
            &palette_command,
            &search_command,
        )?;
        let footer = Toolbar::new(
            ui,
            [
                ToolGroup::new(
                    "status-text",
                    [Element::new("status", Label::new(status.read_only()))],
                )
                .essential(),
                ToolGroup::spacer("status-spacer"),
                ToolGroup::new("zoom", zoom_controls(ui)?).essential(),
            ],
        )?;
        let body = RegionLayout::new()
            .fixed()
            .center(Element::new("gallery-tabs", tabs))
            .bottom(Element::new("status-area", footer));
        // The mouse's back and forward buttons move through the history
        // anywhere over the body that does not use them itself.
        let (side_places, side_status) = (places.clone(), status.clone());
        let body = Element::new(
            "gallery-body",
            HistoryButtons::new(
                places.history(),
                // Structural, so saved workspace paths stay as they were
                // before the wrapper existed.
                Element::new("gallery-regions", body).structural(),
                move |cx, entry| side_places.restore_later(cx, entry, &side_status),
            ),
        );
        // Each dialog anchors on the whole body, so it has the window to open in.
        let body = info_dialog(guide, body, "guide", "Using the gallery", GUIDE)?;
        let body = info_dialog(
            shortcuts,
            body,
            "shortcuts",
            "Keyboard shortcuts",
            SHORTCUTS,
        )?;
        let body = info_dialog(about, body, "about", "About pert-ui", &about_text())?;
        let body = Element::new("command-palette", CommandPalette::new(palette, body));
        let (back_places, back_status) = (places.clone(), status.clone());
        let navigator = HistoryNavigator::new(ui, places.history(), move |cx, entry| {
            back_places.restore_later(cx, entry, &back_status)
        })?
        .name("Gallery history");
        let body = search_picker(search, search_targets, body);
        let root = ui.mount_tree(Element::new(
            "gallery",
            MenuBar::new(ui, menu)?
                .leading(Element::new("brand", mark))
                .trailing(Element::new("gallery-history", navigator))
                .content(body),
        ))?;
        ui.flush(std::time::Duration::ZERO)?;
        *detached_view.borrow_mut() = Some(view_of(ui, &root, "detachable-view")?);
        *guarded_view.borrow_mut() = Some(view_of(ui, &root, "guarded-view")?);
        navigation_targets.resolve(ui, &root)?;
        places.start(ui, &root)?;
        ui.focus_next(false)?;
        Ok(root)
    }
}

/// The gallery's menu bar: appearance settings, project information and help.
fn gallery_menu(
    ui: &Runtime,
    about: &DialogController,
    shortcuts: &DialogController,
    guide: &DialogController,
    palette: &CommandId,
    search: &CommandId,
) -> Result<Vec<MenuItem>, RuntimeError> {
    let theme = ui.theme();
    let (mode_theme, density_theme) = (theme.clone(), theme.clone());
    let modes = setting_rows(
        &theme.mode_signal()?,
        &[
            ("dark", "Dark", ThemeMode::Dark),
            ("light", "Light", ThemeMode::Light),
        ],
        move |mode| mode_theme.set_mode(mode),
    )?;
    let densities = setting_rows(
        &theme.density_signal()?,
        &[
            ("compact", "Compact", Density::Compact),
            ("standard", "Standard", Density::Standard),
            ("comfortable", "Comfortable", Density::Comfortable),
        ],
        move |density| density_theme.set_density(density),
    )?;
    let motion_settings = ui.ui_settings();
    let motions = setting_rows(
        &motion_settings.motion_signal()?,
        &[
            ("full", "Full", MotionPreference::Full),
            ("reduced", "Reduced", MotionPreference::Reduced),
        ],
        move |motion| motion_settings.set_motion(motion),
    )?;
    let text_settings = ui.ui_settings();
    let text_modes = setting_rows(
        &text_settings.text_rendering_signal()?,
        &[
            (
                "line-mask",
                "Original line mask",
                TextRenderingMode::LineMask,
            ),
            (
                "phase-line",
                "Phase-aware line mask",
                TextRenderingMode::PhaseLine,
            ),
        ],
        move |mode| text_settings.set_text_rendering(mode),
    )?;
    let language_settings = ui.ui_settings();
    let languages = setting_rows(
        &language_settings.strings_signal()?,
        &[
            ("english", "English", UiStrings::english()),
            ("swedish", "Svenska", crate::language::swedish()?),
        ],
        move |strings| language_settings.set_strings(strings),
    )?;
    let settings = ui.ui_settings();
    let overlays = setting_rows(
        &settings.debug_overlay_signal()?,
        &[
            ("off", "Off", DebugOverlay::Off),
            ("issues", "Validation issues", DebugOverlay::Issues),
            ("boxes", "Component boxes", DebugOverlay::Boxes),
        ],
        move |overlay| settings.set_debug_overlay(overlay),
    )?;
    let opener = |state: &DialogController| {
        let state = state.clone();
        move |_: &mut EventContext| {
            // Each session's outcome is only acknowledged; the next open needs it gone.
            state.take_outcome()?;
            state.open()
        }
    };
    Ok(vec![
        MenuItem::submenu(
            "settings",
            "Settings",
            vec![
                MenuItem::submenu("theme", "Theme", modes),
                MenuItem::submenu("density", "Density", densities),
                MenuItem::submenu("motion", "Motion", motions),
                MenuItem::separator(),
                MenuItem::submenu("overlay", "Debug overlay", overlays),
                MenuItem::submenu("language", "Library language", languages),
                MenuItem::separator(),
                MenuItem::submenu("text-rendering", "Text rendering", text_modes),
            ],
        ),
        MenuItem::submenu(
            "help",
            "Help",
            vec![
                MenuItem::new("shortcuts", "Keyboard shortcuts", opener(shortcuts)),
                MenuItem::new("guide", "Using the gallery", opener(guide)),
                MenuItem::command(ui, "commands", palette, |_, _| Ok(()))?,
                MenuItem::command(ui, "search", search, |_, _| Ok(()))?,
                MenuItem::separator(),
                MenuItem::new("about", "About", opener(about)),
            ],
        ),
    ])
}
/// Exclusive menu rows for one setting, drawn with the radio dot. Each row's
/// mark is derived from the setting's own signal, so it follows every change,
/// including one a workspace restore makes, not only a choice in this menu.
fn setting_rows<T: Clone + PartialEq + 'static>(
    current: &ReadSignal<T>,
    options: &[(&str, &str, T)],
    apply: impl Fn(T) -> Result<bool, RuntimeError> + 'static,
) -> Result<Vec<MenuItem>, RuntimeError> {
    let apply = Rc::new(apply);
    options
        .iter()
        .map(|(key, title, value)| {
            let apply = apply.clone();
            let (choice, option) = (value.clone(), value.clone());
            Ok(
                MenuItem::new(*key, *title, move |_| apply(choice.clone()).map(|_| ()))
                    .selected_signal(current.map(move |setting| *setting == option)?),
            )
        })
        .collect()
}
/// Zoom out, zoom in, reset and the current percentage, for the status bar.
fn zoom_controls(ui: &Runtime) -> Result<Vec<Element>, RuntimeError> {
    let mut controls = Vec::new();
    for (key, title, direction) in [
        ("zoom-out", "Zoom out", -1),
        ("zoom-in", "Zoom in", 1),
        ("zoom-reset", "Reset zoom to 100%", 0),
    ] {
        let settings = ui.ui_settings();
        let activate = move |_: &mut EventContext| {
            let zoom = settings.zoom()?;
            settings
                .set_zoom(match direction {
                    -1 => zoom.decreased(),
                    1 => zoom.increased(),
                    _ => UiZoom::default(),
                })
                .map(|_| ())
        };
        let icon = match direction {
            -1 => Icon::Remove,
            1 => Icon::Add,
            _ => Icon::Refresh,
        };
        controls.push(Element::new(key, Button::icon(icon, title, activate)));
    }
    // The percentage follows the zoom setting's signal, whoever changes it.
    let percent = ui
        .ui_settings()
        .zoom_signal()?
        .map(|zoom| format!("{:.0}%", zoom.get() * 100.0))?;
    controls.push(Element::new("zoom-value", Label::new(percent)));
    Ok(controls)
}
/// A modal dialog of paragraphs with a Close button, opened from the menu bar.
fn info_dialog(
    state: DialogController,
    anchor: Element,
    key: &str,
    title: &str,
    paragraphs: &[&str],
) -> Result<Element, RuntimeError> {
    let close = state.clone();
    let mut rows: Vec<Element> = paragraphs
        .iter()
        .enumerate()
        .map(|(index, text)| label(&format!("paragraph-{index}"), *text))
        .collect();
    rows.push(Element::new(
        "close",
        Button::new("Close", move |_| close.accept().map(|_| ())),
    ));
    Ok(Element::new(
        format!("{key}-dialog"),
        Dialog::new(state, anchor, section(key, title, rows)),
    ))
}
fn about_text() -> [&'static str; 4] {
    [
        concat!(
            "pert-ui ",
            env!("CARGO_PKG_VERSION"),
            ": a Rust GUI library with general widgets and opinionated defaults, shaped by the needs of audio workstations."
        ),
        "This gallery composes every implemented widget family through the public pert-ui facade only.",
        concat!("License: ", env!("CARGO_PKG_LICENSE"), "."),
        concat!("Source: ", env!("CARGO_PKG_REPOSITORY")),
    ]
}
const SHORTCUTS: &[&str] = &[
    "Tab and Shift+Tab move focus. Arrows move within a group, such as a tab strip or a segmented control.",
    "F10 focuses this menu bar. Left and Right move between menus, Down opens one and Escape closes one level.",
    "Right-click or Shift+F10 opens a context menu where a page offers one.",
    "Alt+Left and Alt+Right (Cmd+[ and Cmd+] on macOS), or the mouse's side buttons, go back and forward through the pages visited; the clock icon at the right of the menu bar lists them.",
    "Ctrl+D (Cmd+D on macOS) moves the single view on State & tabs into its own window.",
    "Ctrl+Shift+P (Cmd+Shift+P on macOS) opens the command palette: type to filter, Up and Down to choose, Enter to run. Its Go to commands open each page.",
    "Ctrl+Shift+F (Cmd+Shift+F on macOS) searches the gallery's content and commands together: choose a button, a label, a field's value or a table row on any page to open the page and focus it.",
    "Escape closes this dialog.",
];
const GUIDE: &[&str] = &[
    "The top strip holds categories; each category has its own strip of pages and remembers the page last shown.",
    "Drag a tab title out of its strip to open that view in its own window; closing the window docks it again.",
    "Settings changes the color theme, density, motion and text rendering and turns on the debug overlay. Zoom is at the right end of the status bar.",
    "The status bar reports what the page examples do.",
];

/// The pages the command palette's "Go to" commands open, as (category key,
/// page key, title). A curated list of destinations: the palette searches
/// commands, not every widget.
pub(crate) const GO_TO_PAGES: [(&str, &str, &str); 9] = [
    ("layout-category", "navigation", "Navigation"),
    ("layout-category", "splits", "Split panes"),
    ("controls-category", "controls", "Controls"),
    ("controls-category", "chips", "Chips & breadcrumbs"),
    ("text-category", "editing", "Text input"),
    ("surfaces-category", "surfaces", "Dialogs & notifications"),
    ("data-category", "collections", "Collections"),
    ("tools-category", "theme", "Themes"),
    ("tools-category", "workspace", "Workspace"),
];

pub(crate) fn label(key: &str, text: impl Into<TextSource>) -> Element {
    Element::new(key, Label::new(text))
}
pub(crate) fn section(
    key: &str,
    title: &str,
    children: impl IntoIterator<Item = Element>,
) -> Element {
    Element::new(
        key,
        Group::titled(title, Element::new("content", Column::new(children))),
    )
}
pub(crate) fn page(children: impl IntoIterator<Item = Element>) -> Element {
    Element::new(
        "page",
        Container::new(Element::new(
            "sections",
            Column::new(children).gap(MetricRole::SectionGap),
        ))
        .padding(MetricRole::ContentMargin),
    )
}
pub(crate) fn stage(key: &str, height: u16, child: Element) -> Element {
    Element::new(
        key,
        Container::new(child).preferred_size(Size::from([640, height])),
    )
}
pub(crate) fn tile(key: &str, title: impl Into<TextSource>) -> Element {
    Element::new(
        key,
        Container::new(Element::new(
            "label",
            Label::new(title).color_role(ColorRole::OnSurfaceContainer),
        ))
        .padding(Length::from(12))
        .size_limits(SizeLimits::at_least(Size::from([24, 24])))
        .background_role(ColorRole::SurfaceContainer),
    )
}
pub(crate) fn action(key: &str, text: impl Into<String>, status: &Signal<String>) -> Element {
    let text = text.into();
    let response = format!("{text} activated");
    let status = status.clone();
    Element::new(
        key,
        Button::new(text, move |_| status.set(response.clone())),
    )
}
pub(crate) fn toggle(
    key: &str,
    title: &str,
    value: &Signal<bool>,
) -> Result<Element, RuntimeError> {
    let writer = value.clone();
    Ok(Element::new(
        key,
        Row::new([
            Element::new(
                "toggle",
                Button::new(title, move |_| writer.set(!writer.get()?)),
            ),
            Element::new(
                "value",
                Label::new(value.map(|value| if *value { "On" } else { "Off" }.to_owned())?),
            ),
        ]),
    ))
}
pub(crate) fn counter(ui: &Runtime, key: &str) -> Result<Element, RuntimeError> {
    counter_of(ui.signal(0u64), key)
}
/// A counter over an existing value, so that describing it again, as a
/// reordered strip does, shows the same count.
pub(crate) fn counter_of(value: Signal<u64>, key: &str) -> Result<Element, RuntimeError> {
    let increment = value.clone();
    Ok(Element::new(
        key,
        Row::new([
            Element::new(
                "increment",
                Button::new("Increment", move |_| {
                    // A click at the maximum keeps the maximum.
                    increment.set(increment.get()?.saturating_add(1))
                }),
            ),
            Element::new(
                "reset",
                Button::new("Reset", {
                    let value = value.clone();
                    move |_| value.set(0)
                }),
            ),
            Element::new(
                "count",
                Label::new(value.map(|value| format!("Count: {value}"))?),
            ),
        ]),
    ))
}
