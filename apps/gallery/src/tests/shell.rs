//! The shell: every page, the category strips, the settings menu, zoom and
//! detachable strips.
use super::support::*;
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;

#[test]
fn every_gallery_tab_paints_at_desktop_and_narrow_fractional_scale() {
    let (mut ui, root) = mounted();
    // Every page the gallery mounts, so a new page is covered without an edit.
    let keys = page_keys(&ui, &root);
    assert!(keys.len() > 20, "only {} pages mounted", keys.len());
    let categories = ui
        .children(&find(&ui, &root, "gallery-tabs"))
        .unwrap()
        .len();
    assert!(categories < keys.len(), "pages are grouped into categories");
    // The desktop width at a fractional scale is the case that found the
    // touching-peer false positive: at 960 by 1.25 the grid's rows land on
    // coordinates f32 cannot represent, so neighbours that tile exactly were
    // reported as overlapping.
    for (width, scale) in [(960.0, 1.0), (320.0, 1.25), (960.0, 1.25)] {
        ui.set_viewport(Viewport::new(width, 640.0, scale).unwrap())
            .unwrap();
        for key in &keys {
            open(&mut ui, &root, key);
            assert!(!ui.paint().unwrap().masks().is_empty());
            assert!(
                ui.validation_report().issues.is_empty(),
                "page {key}: {:?}",
                ui.validation_report().issues
            );
            assert!(ui.next_deadline().is_none(), "idle page {key}");
        }
    }
}
#[test]
fn fixed_gallery_chrome_stays_put_while_split_content_scrolls() {
    let (mut ui, root) = mounted();
    let tabs = find(&ui, &root, "gallery-tabs");
    let heading = find(&ui, &root, "title/settings");
    let chrome = ui.bounds(&heading).unwrap();
    let top = ui.bounds(&tabs).unwrap().unwrap().y();
    open(&mut ui, &root, "splits");
    let split = find(&ui, &find(&ui, &root, "content-first"), "split");
    let before = ui.bounds(&split).unwrap().unwrap().y();
    ui.focus(&split).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scrolled = ui.bounds(&split).unwrap().unwrap().y();
    assert!(scrolled < before);
    assert_eq!(ui.bounds(&heading).unwrap(), chrome);
    assert_eq!(ui.bounds(&tabs).unwrap().unwrap().y(), top);
    let point = input_category_point(&mut ui, &root);
    pointer_click(&mut ui, point);
    assert_eq!(ui.bounds(&heading).unwrap(), chrome);
    assert_eq!(ui.bounds(&tabs).unwrap().unwrap().y(), top);
    open(&mut ui, &root, "splits");
    assert_eq!(ui.bounds(&split).unwrap().unwrap().y(), scrolled);
    assert!(ui.validation_report().issues.is_empty());
}
#[test]
fn appearance_controls_change_density_zoom_and_keep_retained_views() {
    let (mut ui, root) = mounted();
    let tabs = find(&ui, &root, "gallery-tabs");
    let identities = ui.children(&tabs).unwrap();
    // Start focused: choosing a setting returns focus to where it was, and with
    // nothing focused the titles would keep it.
    ui.focus(&tabs).unwrap();
    settle(&mut ui);
    for (index, density) in [
        (0, Density::Compact),
        (2, Density::Comfortable),
        (1, Density::Standard),
    ] {
        choose_setting(&mut ui, 1, index);
        assert_eq!(ui.theme().density().unwrap(), density);
        assert_eq!(ui.children(&tabs).unwrap(), identities);
    }
    for (key, factor) in [
        ("zoom-in", 1.1),
        ("zoom-in", 1.25),
        ("zoom-out", 1.1),
        ("zoom-reset", 1.0),
    ] {
        let button = find(&ui, &root, key);
        click(&mut ui, &button);
        assert_eq!(ui.ui_settings().zoom().unwrap().get(), factor);
        assert_eq!(
            text(&ui, "zoom-value"),
            vec![format!("{:.0}%", factor * 100.0)]
        );
        assert_eq!(ui.children(&tabs).unwrap(), identities);
    }
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        ui.theme().set_mode(mode).unwrap();
        for density in [Density::Compact, Density::Standard, Density::Comfortable] {
            ui.theme().set_density(density).unwrap();
            for zoom in [0.75, 1.25, 2.0] {
                ui.ui_settings()
                    .set_zoom(UiZoom::new(zoom).unwrap())
                    .unwrap();
                open(&mut ui, &root, "theme");
                let scene = ui.paint().unwrap();
                assert_eq!(scene.viewport().scale(), zoom);
                assert!(!scene.masks().is_empty());
                assert_eq!(ui.children(&tabs).unwrap(), identities);
                assert!(
                    ui.validation_report().issues.is_empty(),
                    "{mode:?} {density:?} {zoom}: {:?}",
                    ui.validation_report().issues
                );
                assert_eq!(ui.next_deadline(), None);
            }
        }
    }
}

#[test]
fn every_gallery_tab_strip_is_detachable_and_a_dragged_tab_moves_and_redocks() {
    let (mut ui, root) = mounted();
    // Inspection covers the nested strips on the State & tabs and Floating
    // windows pages as well as the application strip.
    let strips: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|item| item.details.properties.contains_key("tab_count"))
        .collect();
    assert!(
        strips.len() >= 3,
        "expected the gallery's nested tab strips"
    );
    for strip in &strips {
        assert_eq!(
            strip.details.properties.get("detachable"),
            Some(&InspectValue::Bool(true)),
            "tab strip {:?} is not detachable",
            strip.key
        );
    }

    // Dragging a category out moves its whole strip of pages into a window.
    let tabs = find(&ui, &root, "gallery-tabs");
    let category = category_of(&ui, &root, "input");
    let point = input_category_point(&mut ui, &root);
    let view = ui.children(&tabs).unwrap()[category].clone();
    ui.pointer_input(PointerEvent::new(
        point,
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    let outside = Point::new(point.x() + 40.0, point.y() + 200.0).unwrap();
    ui.pointer_input(PointerEvent::new(outside, PointerKind::Move))
        .unwrap();
    ui.pointer_input(PointerEvent::new(
        outside,
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    let mut requests = ui.take_presentation_requests().unwrap();
    assert_eq!(requests.len(), 1);
    let request = requests.pop().unwrap();
    let PresentationAction::Detach {
        view: requested,
        options,
    } = request.action()
    else {
        panic!("expected a detach request");
    };
    assert_eq!(requested, &view);
    assert_eq!(options.title(), "Controls");
    settle(&mut ui);
    let prepared = ui.prepare_detach(requested, options.clone()).unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    request
        .complete(PresentationStatus::Presented(detached.clone()))
        .unwrap();
    // The same incarnation now presents in its own window and still paints.
    assert_eq!(ui.children(&tabs).unwrap()[category], view);
    assert_eq!(ui.window_of(&view).unwrap(), detached);
    ui.flush_windows(Duration::ZERO).unwrap();
    assert!(!ui.paint().unwrap().masks().is_empty());
    ui.select_window(&ui.primary_window()).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    assert!(!ui.paint().unwrap().masks().is_empty());
    assert!(ui.validation_report().issues.is_empty());

    ui.redock(&detached).unwrap();
    ui.flush_windows(Duration::ZERO).unwrap();
    assert_eq!(ui.window_of(&view).unwrap(), ui.primary_window());
    assert_eq!(ui.children(&tabs).unwrap()[category], view);
    open(&mut ui, &root, "input");
    let increase = find(&ui, &find(&ui, &root, "editor-0"), "increase");
    click(&mut ui, &increase);
    assert_eq!(first_parameter(&mut ui).base, 5.0);
}

/// Roles that present a current value, which a complete description reports.
const VALUED: [SemanticRole; 12] = [
    SemanticRole::Checkbox,
    SemanticRole::Switch,
    SemanticRole::RadioGroup,
    SemanticRole::SegmentedChoice,
    SemanticRole::Slider,
    SemanticRole::NumberInput,
    SemanticRole::TextInput,
    SemanticRole::SearchInput,
    SemanticRole::MultilineTextInput,
    SemanticRole::ComboBox,
    SemanticRole::Readout,
    SemanticRole::ProgressIndicator,
];

/// The semantic completeness rule (docs/verification.md): on every page, each
/// presented component that reports itself interactive has semantics with a
/// role, a resolved non-blank name (its own, its element's label, or one a
/// wrapping `Field` gives it, also through a composite's internal structure)
/// and, for a role that presents a value, that value. A concealed component,
/// one whose semantics are `protected`, reports no value instead. A disabled
/// component reports itself not interactive, so disabled components are
/// checked by their semantics instead: each one is named and offers no
/// actions, and the gallery's known disabled controls must report semantics
/// at all.
///
/// Hidden controls (an unselected tab inside a page, a collapsed section, a
/// closed popup) are checked only where a page shows them, and components
/// that are not interactive are structure and are not required to report
/// semantics. The rule covers the gallery's compositions, not every
/// composition a consumer can build.
#[test]
fn every_interactive_gallery_component_has_complete_semantics() {
    // Disabled controls the gallery always shows; each must report disabled
    // semantics, so a disabled control cannot escape the check by dropping
    // its semantics along with its interactivity.
    const KNOWN_DISABLED: [&str; 3] = ["integer-disabled", "disabled-field", "jump-unavailable"];
    let (mut ui, root) = mounted();
    let mut gaps = std::collections::BTreeSet::new();
    let mut disabled_seen = std::collections::BTreeSet::new();
    for key in page_keys(&ui, &root) {
        open(&mut ui, &root, &key);
        for component in ui.inspect().components {
            let details = &component.details;
            let disabled = details
                .semantics
                .as_ref()
                .is_some_and(|semantics| semantics.disabled);
            if !component.visible || !(details.validation.interactive || disabled) {
                continue;
            }
            if disabled {
                disabled_seen.insert(component.key.clone());
            }
            let at = format!("page {key}: {}", component.key);
            let Some(semantics) = &details.semantics else {
                gaps.insert(format!("{at} reports no semantics"));
                continue;
            };
            let named = semantics
                .name
                .as_deref()
                .is_some_and(|name| !name.trim().is_empty());
            if !named {
                gaps.insert(format!("{at} ({:?}) has no name", semantics.role));
            }
            let concealed = semantics.protected;
            if concealed && semantics.value.is_some() {
                gaps.insert(format!("{at} is concealed but reports a value"));
            }
            if !concealed && VALUED.contains(&semantics.role) && semantics.value.is_none() {
                gaps.insert(format!("{at} ({:?}) reports no value", semantics.role));
            }
            if semantics.disabled && !semantics.actions.is_empty() {
                gaps.insert(format!(
                    "{at} is disabled but offers {:?}",
                    semantics.actions
                ));
            }
        }
    }
    for key in KNOWN_DISABLED {
        if !disabled_seen.contains(key) {
            gaps.insert(format!("{key} does not report disabled semantics"));
        }
    }
    assert!(gaps.is_empty(), "incomplete semantics: {gaps:#?}");
}

#[test]
fn categories_group_the_pages_and_remember_the_page_last_shown() {
    let (mut ui, root) = mounted();
    let top = find(&ui, &root, "gallery-tabs");
    let selected = |ui: &Runtime, strip: &WidgetId| {
        let key = ui.key(strip).unwrap().to_string();
        let component = ui
            .inspect()
            .components
            .into_iter()
            .find(|item| item.key == key && item.visible)
            .unwrap();
        component.details.properties.get("selected").cloned()
    };
    open(&mut ui, &root, "input");
    assert_eq!(
        selected(&ui, &top),
        Some(InspectValue::Text("controls-category".into()))
    );
    open(&mut ui, &root, "images");
    // Returning to Controls by its category tab alone shows Input again.
    let controls = category_of(&ui, &root, "input");
    select(&mut ui, &top, controls);
    let strip = pages(&ui, &root)
        .into_iter()
        .find(|page| page.3 == "input")
        .unwrap()
        .1;
    assert_eq!(
        selected(&ui, &strip),
        Some(InspectValue::Text("input".into()))
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_gallery_menu_bar_opens_about_and_help_dialogs() {
    let (mut ui, root) = mounted();
    let tabs = find(&ui, &root, "gallery-tabs");
    assert!(!dialog_shown(&ui, "about"));
    // Nothing has focus at start, and F10 still reaches the bar. Help's last
    // row opens About.
    assert_eq!(ui.focused(), None);
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert!(dialog_shown(&ui, "about"));
    let version = format!("pert-ui {}", env!("CARGO_PKG_VERSION"));
    assert!(
        text(&ui, "paragraph-0")
            .iter()
            .any(|t| t.starts_with(&version))
    );
    let close = find(&ui, &find(&ui, &root, "about"), "close");
    click(&mut ui, &close);
    assert!(!dialog_shown(&ui, "about"));
    // Help › Keyboard shortcuts, then Help › Using the gallery; Escape closes.
    for (row, key) in [(0, "shortcuts"), (1, "guide")] {
        ui.focus(&tabs).unwrap();
        settle(&mut ui);
        press(&mut ui, KeyCode::F10);
        press(&mut ui, KeyCode::End);
        press(&mut ui, KeyCode::ArrowDown);
        for _ in 0..row {
            press(&mut ui, KeyCode::ArrowDown);
        }
        press(&mut ui, KeyCode::Enter);
        assert!(dialog_shown(&ui, key), "{key}");
        press(&mut ui, KeyCode::Escape);
        assert!(!dialog_shown(&ui, key), "{key}");
    }
    // The About dialog opens again after its first session ended.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    assert!(dialog_shown(&ui, "about"));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn settings_menu_selects_text_rendering() {
    let (mut ui, root) = mounted();
    ui.focus(&find(&ui, &root, "gallery-tabs")).unwrap();
    settle(&mut ui);
    assert_eq!(
        ui.ui_settings().text_rendering().unwrap(),
        TextRenderingMode::LineMask
    );
    choose_setting(&mut ui, 5, 1);
    assert_eq!(
        ui.ui_settings().text_rendering().unwrap(),
        TextRenderingMode::PhaseLine
    );
    choose_setting(&mut ui, 5, 0);
    assert_eq!(
        ui.ui_settings().text_rendering().unwrap(),
        TextRenderingMode::LineMask
    );
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_brand_mark_leads_the_menu_bar_follows_the_theme_and_opens_about() {
    let (mut ui, root) = mounted();
    let mark = find(&ui, &root, "brand-mark");
    let details = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == "brand-mark")
            .unwrap()
            .details
    };
    let semantics = details(&ui).semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::Button);
    assert_eq!(semantics.name.as_deref(), Some("pert-ui"));
    // It sits before the first title, centered on the title row.
    let [mx, my, _, mh] = ui.bounds(&mark).unwrap().unwrap().values();
    let title = find(&ui, &root, "title/settings");
    let [tx, ty, _, th] = ui.bounds(&title).unwrap().unwrap().values();
    assert!(mx < tx);
    assert!((my + mh / 2.0 - (ty + th / 2.0)).abs() < 0.01);
    // The variant follows the theme mode without describing the bar again.
    let variant = |ui: &Runtime| details(ui).properties["variant"].clone();
    assert_eq!(variant(&ui), InspectValue::Text("dark".into()));
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    settle(&mut ui);
    assert_eq!(variant(&ui), InspectValue::Text("light".into()));
    assert_eq!(find(&ui, &root, "brand-mark"), mark);
    // Enter, or a click, opens About.
    ui.focus(&mark).unwrap();
    settle(&mut ui);
    let contrast = Color::from_srgb8(20, 230, 20, 255);
    ui.theme()
        .set_color(ThemeMode::Light, ColorRole::FocusRingContrast, contrast)
        .unwrap();
    settle(&mut ui);
    assert!(
        !ui.paint()
            .unwrap()
            .commands()
            .iter()
            .any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. }
                    if *color == contrast
            ))
    );
    press(&mut ui, KeyCode::Enter);
    assert!(dialog_shown(&ui, "about"));
    press(&mut ui, KeyCode::Escape);
    assert!(!dialog_shown(&ui, "about"));
    // Real pointer input: a press released outside the mark cancels, a press
    // released on it opens About.
    let [x, y, w, h] = ui.bounds(&mark).unwrap().unwrap().values();
    let at = |px: f32, py: f32| Point::new(px, py).unwrap();
    let primary = PointerButton::Primary;
    let center = at(x + w / 2.0, y + h / 2.0);
    ui.pointer_input(PointerEvent::new(center, PointerKind::Press(primary)))
        .unwrap();
    let outside = at(x + w / 2.0, y + h * 4.0);
    ui.pointer_input(PointerEvent::new(outside, PointerKind::Move))
        .unwrap();
    ui.pointer_input(PointerEvent::new(outside, PointerKind::Release(primary)))
        .unwrap();
    settle(&mut ui);
    assert!(
        !dialog_shown(&ui, "about"),
        "a cancelled press opened About"
    );
    ui.pointer_input(PointerEvent::new(center, PointerKind::Press(primary)))
        .unwrap();
    ui.pointer_input(PointerEvent::new(center, PointerKind::Release(primary)))
        .unwrap();
    settle(&mut ui);
    assert!(dialog_shown(&ui, "about"));
    assert!(ui.validation_report().issues.is_empty());
    ui.paint().unwrap();
}

#[test]
fn settings_menu_marks_follow_a_workspace_restore() {
    let (mut ui, root) = mounted();
    ui.focus(&find(&ui, &root, "gallery-tabs")).unwrap();
    settle(&mut ui);
    let dark = ui.capture_workspace().unwrap();
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Dark);
    choose_setting(&mut ui, 0, 1);
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Light);
    // Restoring sets the theme directly, not through the menu.
    assert!(ui.restore_workspace(&dark).unwrap().is_complete());
    settle(&mut ui);
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Dark);
    // Open Settings › Theme without choosing a row and read its marks.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::ArrowRight);
    let marks: Vec<_> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|component| component.key == "items" && component.visible)
        .filter_map(|component| component.details.semantics)
        .find(|semantics| {
            semantics
                .items
                .iter()
                .map(|item| item.name.as_deref())
                .eq([Some("Dark"), Some("Light")])
        })
        .expect("the Theme submenu is open")
        .items
        .iter()
        .map(|item| item.checked)
        .collect();
    assert_eq!(marks, [Some(true), Some(false)]);
}

/// The gallery command whose name is `name`, as the palette lists it.
fn gallery_command(ui: &mut Runtime, name: &str) -> CommandEntry {
    let context = ui.command_context(CommandSource::Programmatic);
    ui.commands_for(&context)
        .unwrap()
        .into_iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("no gallery command {name}"))
}
fn palette_open(ui: &Runtime) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|c| c.key == "command-palette")
        .and_then(|c| c.details.properties.get("open").cloned())
}

#[test]
fn the_command_palette_opens_from_its_command_and_goes_to_a_page() {
    let (mut ui, root) = mounted();
    let show = gallery_command(&mut ui, "Show all commands");
    assert_eq!(show.shortcuts, [KeyChord::primary('p').shifted()]);
    let context = ui.command_context(CommandSource::KeyBinding);
    ui.invoke_command(&show.id, context).unwrap();
    settle(&mut ui);
    assert_eq!(palette_open(&ui), Some(InspectValue::Bool(true)));
    ui.text_input("go to chips").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(palette_open(&ui), Some(InspectValue::Bool(false)));
    let chips = page(&ui, &root, "chips");
    assert!(ui.bounds(&chips).unwrap().is_some(), "the page is shown");
    assert_eq!(text(&ui, "status"), ["Opened Chips & breadcrumbs."]);
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn every_go_to_command_opens_its_page() {
    let (mut ui, root) = mounted();
    for (_, page_key, title) in crate::gallery::GO_TO_PAGES {
        let entry = gallery_command(&mut ui, &format!("Go to: {title}"));
        let context = ui.command_context(CommandSource::Programmatic);
        assert_eq!(
            ui.invoke_command(&entry.id, context).unwrap(),
            CommandOutcome::Invoked,
            "{title}"
        );
        settle(&mut ui);
        let page = page(&ui, &root, page_key);
        assert!(ui.bounds(&page).unwrap().is_some(), "{title} is not shown");
    }
}

/// Every hit for `text`, searching the whole gallery including collections'
/// rows that are not mounted.
fn content_hits(ui: &Runtime, text: &str) -> Vec<ContentHit> {
    let mut search = ui.content_search(ContentQuery::new(text).unwrap());
    let mut hits = Vec::new();
    loop {
        let step = search.step(ui).unwrap();
        hits.extend(step.hits);
        if step.done {
            return hits;
        }
    }
}

#[test]
fn content_search_finds_a_button_in_a_tab_that_is_not_shown_and_jumps_to_it() {
    let (mut ui, root) = mounted();
    // The Navigation page sits in another category, and the button in a tab
    // of its own strip that is not selected.
    let hits = content_hits(&ui, "output level");
    let hit = hits
        .iter()
        .find(|hit| hit.role == Some(SemanticRole::Button) && hit.text == "Output level")
        .expect("the hidden button is found");
    assert!(!hit.visible);
    let ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. }) =
        ui.jump_to_content(&hit.target).unwrap()
    else {
        panic!("the jump did not focus the button");
    };
    let focused = ui.focused().expect("the button has focus");
    assert_eq!(ui.key(&focused).unwrap(), "output-level");
    let navigation = page(&ui, &root, "navigation");
    assert!(
        ui.bounds(&navigation).unwrap().is_some(),
        "the page is shown"
    );
}

#[test]
fn content_search_finds_text_in_a_collapsed_section_and_the_jump_opens_it() {
    let (mut ui, _root) = mounted();
    let hits = content_hits(&ui, "options most people never change");
    let hit = hits
        .iter()
        .find(|hit| {
            hit.path
                .as_ref()
                .is_some_and(|path| path.iter().any(|segment| segment == "multiple"))
        })
        .expect("the text in the closed section is found");
    assert!(!hit.visible);
    let ContentJump::Reveal(outcome) = ui.jump_to_content(&hit.target).unwrap() else {
        panic!("not a reveal");
    };
    assert!(outcome.is_revealed(), "{outcome:?}");
    let shown = ui
        .content_search(
            ContentQuery::new("options most people never change")
                .unwrap()
                .hidden(false),
        )
        .step(&ui)
        .unwrap()
        .hits;
    assert!(
        shown.iter().any(|shown| shown.target == hit.target),
        "the section opened"
    );
}

#[test]
fn content_search_finds_a_table_value_outside_the_mounted_rows() {
    let (mut ui, _root) = mounted();
    let hits = content_hits(&ui, "sample 54321");
    let hit = hits
        .iter()
        .find(|hit| matches!(&hit.target, ContentTarget::Row { key, .. } if key == "sample-54321"))
        .expect("the unmounted row is found by its text");
    assert!(hit.text.starts_with("Sample 54321 "), "{}", hit.text);
    let ContentJump::Reveal(outcome) = ui.jump_to_content(&hit.target).unwrap() else {
        panic!("the row was not mounted");
    };
    assert!(outcome.is_revealed(), "{outcome:?}");
    // The row is mounted now and is found as components.
    assert!(
        content_hits(&ui, "sample 54321")
            .iter()
            .any(|hit| matches!(hit.target, ContentTarget::Widget(_)) && hit.text == "Sample 54321")
    );
}

fn search_status(ui: &Runtime) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|item| item.key == "gallery-search")
        .and_then(|item| item.details.properties.get("status").cloned())
}

#[test]
fn the_gallery_search_lists_content_and_commands_and_jumps_to_a_choice() {
    let (mut ui, root) = mounted();
    let search = gallery_command(&mut ui, "Search the gallery");
    assert_eq!(search.shortcuts, [KeyChord::primary('f').shifted()]);
    let context = ui.command_context(CommandSource::KeyBinding);
    ui.invoke_command(&search.id, context).unwrap();
    settle(&mut ui);
    assert_eq!(search_status(&ui), Some(InspectValue::Text("Open".into())));
    // Content and commands share the picker, each row labelled by kind.
    let shown_kind = |ui: &Runtime, kind: &str| {
        ui.inspect()
            .components
            .into_iter()
            .filter(|item| item.visible)
            .filter_map(|item| item.details.semantics)
            .filter_map(|semantics| semantics.description)
            .any(|text| text.contains(kind))
    };
    assert!(shown_kind(&ui, "Content"));
    ui.text_input("show all commands").unwrap();
    settle(&mut ui);
    assert!(shown_kind(&ui, "Command"));
    // Escape clears the query first and leaves the picker open.
    press(&mut ui, KeyCode::Escape);
    settle(&mut ui);
    assert_eq!(search_status(&ui), Some(InspectValue::Text("Open".into())));
    ui.text_input("output level").unwrap();
    settle(&mut ui);
    // A content row names the page it is on, not its key path.
    let details: Vec<String> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|item| item.visible && item.key == "detail")
        .filter_map(|item| match item.details.properties.get("text") {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        details.contains(&"Layout › Navigation · Content".to_owned()),
        "{details:?}"
    );
    // Enter arrives from inside a flush, as a native check's driver delivers
    // it, so the picker closes before the jump's task is delivered.
    let _enter = ui
        .defer(|ui| {
            ui.key_input(KeyCode::Enter, KeyAction::Press)?;
            ui.key_input(KeyCode::Enter, KeyAction::Release)?;
            Ok(())
        })
        .unwrap();
    settle(&mut ui);
    // The choice queued the jump during that flush; the next one delivers it.
    settle(&mut ui);
    assert_eq!(
        search_status(&ui),
        Some(InspectValue::Text("Closed".into()))
    );
    let focused = ui.focused().expect("the chosen button has focus");
    assert_eq!(ui.key(&focused).unwrap(), "output-level");
    let navigation = page(&ui, &root, "navigation");
    assert!(
        ui.bounds(&navigation).unwrap().is_some(),
        "the page is shown"
    );
}

#[test]
fn the_gallery_search_finds_a_named_field_by_its_value() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "properties");
    let field = find(&ui, &root, "name-input");
    ui.focus(&field).unwrap();
    settle(&mut ui);
    let primary = KeyCode::primary();
    ui.key_input(primary, KeyAction::Press).unwrap();
    press(&mut ui, KeyCode::KeyA);
    ui.key_input(primary, KeyAction::Release).unwrap();
    ui.text_input("Reverb tail").unwrap();
    settle(&mut ui);
    let search = gallery_command(&mut ui, "Search the gallery");
    let context = ui.command_context(CommandSource::KeyBinding);
    ui.invoke_command(&search.id, context).unwrap();
    settle(&mut ui);
    // The field is named "Name"; typing its value still finds it.
    ui.text_input("reverb").unwrap();
    settle(&mut ui);
    let shown: Vec<String> = ui
        .inspect()
        .components
        .into_iter()
        .filter(|item| item.visible)
        .filter_map(|item| item.details.semantics?.name)
        .collect();
    assert!(
        shown.iter().any(|name| name.contains("Name: Reverb tail")),
        "{shown:?}"
    );
}

#[test]
fn the_library_language_setting_switches_the_library_strings() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "navigation");
    let names = |ui: &Runtime| -> Vec<String> {
        ui.inspect()
            .components
            .into_iter()
            .filter(|component| component.visible)
            .filter_map(|component| component.details.semantics?.name)
            .collect()
    };
    assert!(names(&ui).iter().any(|name| name == "Back"));
    // Settings › Library language › Svenska.
    choose_setting(&mut ui, 4, 1);
    assert_eq!(
        ui.ui_settings().strings().unwrap(),
        crate::language::swedish().unwrap()
    );
    let swedish = names(&ui);
    assert!(swedish.iter().any(|name| name == "Bakåt"), "{swedish:?}");
    assert!(!swedish.iter().any(|name| name == "Back"), "{swedish:?}");
    choose_setting(&mut ui, 4, 0);
    assert!(names(&ui).iter().any(|name| name == "Back"));
    assert!(ui.validation_report().issues.is_empty());
}
