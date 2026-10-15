//! The Appearance & tools category's pages.
use super::support::*;
use pert_ui::*;

#[test]
fn profiler_controls_capture_pause_clear_and_do_not_poll() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "profiling");
    let refresh = find(&ui, &root, "refresh-profile");
    click(&mut ui, &refresh);
    assert!(text(&ui, "profile-report")[0].contains("Recording: true"));
    assert!(text(&ui, "profile-report")[0].contains("Total ms"));
    let pause = find(&ui, &root, "disable-profile");
    click(&mut ui, &pause);
    let snapshot = ui.profile();
    assert!(!snapshot.enabled);
    let clear = find(&ui, &root, "clear-profile");
    click(&mut ui, &clear);
    assert!(ui.profile().stats.is_empty());
    let enable = find(&ui, &root, "enable-profile");
    click(&mut ui, &enable);
    assert!(ui.profile().enabled);
    assert!(ui.next_deadline().is_none());
    ui.paint().unwrap();
}

#[test]
fn gallery_theme_controls_switch_modes_and_preserve_overrides() {
    let (mut ui, root) = mounted();
    // Start focused: choosing a setting returns focus to where it was, and with
    // nothing focused the titles would keep it.
    ui.focus(&find(&ui, &root, "gallery-tabs")).unwrap();
    settle(&mut ui);
    ui.paint().unwrap();
    choose_setting(&mut ui, 0, 1);
    assert_eq!(ui.theme().mode().unwrap(), ThemeMode::Light);
    open(&mut ui, &root, "theme");
    ui.paint().unwrap();
    let apply = find(&ui, &root, "override-primary");
    click(&mut ui, &apply);
    let palette = ui.theme().palette(ThemeMode::Light).unwrap();
    assert_eq!(
        palette.color(ColorRole::Primary),
        palette.color(ColorRole::Secondary)
    );
    choose_setting(&mut ui, 0, 0);
    assert_eq!(
        ui.theme().palette(ThemeMode::Dark).unwrap(),
        Palette::dark()
    );
    choose_setting(&mut ui, 0, 1);
    assert_eq!(ui.theme().palette(ThemeMode::Light).unwrap(), palette);
    let reset = find(&ui, &root, "reset-palette");
    click(&mut ui, &reset);
    assert_eq!(
        ui.theme().palette(ThemeMode::Light).unwrap(),
        Palette::light()
    );
    // Exercise every actual page in light mode as well as the default dark pass.
    for key in page_keys(&ui, &root) {
        open(&mut ui, &root, &key);
        assert!(!ui.paint().unwrap().masks().is_empty());
        assert!(ui.validation_report().issues.is_empty());
    }
    assert!(ui.next_deadline().is_none());
    assert!(!ui.needs_paint());
}

#[test]
fn gallery_seed_presets_seed_both_palettes_and_blue_restores_them() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "theme");
    ui.paint().unwrap();
    let red = find(&ui, &root, "seed-red");
    click(&mut ui, &red);
    for (mode, built_in) in [
        (ThemeMode::Dark, Palette::dark()),
        (ThemeMode::Light, Palette::light()),
    ] {
        let palette = ui.theme().palette(mode).unwrap();
        assert_ne!(palette, built_in);
        let [red, green, blue, _] = palette.color(ColorRole::Primary).channels();
        assert!(
            red > green && red > blue,
            "{mode:?} accent follows the red seed"
        );
    }
    assert!(!ui.paint().unwrap().masks().is_empty());
    let blue = find(&ui, &root, "seed-blue");
    click(&mut ui, &blue);
    assert_eq!(
        ui.theme().palette(ThemeMode::Dark).unwrap(),
        Palette::dark()
    );
    assert_eq!(
        ui.theme().palette(ThemeMode::Light).unwrap(),
        Palette::light()
    );
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn gallery_seed_pickers_apply_a_chosen_primary_and_an_optional_secondary() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "theme");
    ui.paint().unwrap();
    let secondary_disabled = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| {
                component.key == "swatch"
                    && component
                        .path
                        .as_ref()
                        .is_some_and(|path| path.iter().any(|segment| segment == "secondary-seed"))
            })
            .and_then(|component| component.details.semantics)
            .map(|semantics| semantics.disabled)
    };
    assert_eq!(secondary_disabled(&ui), Some(true));
    // Turn the primary seed's hue to red, keeping its saturation and value.
    let primary = find(&ui, &root, "primary-seed");
    let swatch = ui.find_descendant(&primary, "swatch").unwrap().unwrap();
    click(&mut ui, &swatch);
    let hue = ui.find_descendant(&primary, "hue").unwrap().unwrap();
    ui.focus(&hue).unwrap();
    press(&mut ui, KeyCode::Home);
    press(&mut ui, KeyCode::Escape);
    settle(&mut ui);
    let apply = find(&ui, &root, "apply-seeds");
    click(&mut ui, &apply);
    let primary_only =
        [ThemeMode::Dark, ThemeMode::Light].map(|mode| ui.theme().palette(mode).unwrap());
    for palette in &primary_only {
        let [red, green, blue, _] = palette.color(ColorRole::Primary).channels();
        assert!(red > green && red > blue, "the accent follows the red seed");
    }
    // The secondary seed starts at the built-in dark Secondary, so seeding
    // with it keeps that accent instead of rotating it with the primary.
    let toggle = find(&ui, &root, "use-secondary");
    ui.focus(&toggle).unwrap();
    press(&mut ui, KeyCode::Space);
    settle(&mut ui);
    assert_eq!(secondary_disabled(&ui), Some(false));
    click(&mut ui, &apply);
    let dark = ui.theme().palette(ThemeMode::Dark).unwrap();
    assert_ne!(dark, primary_only[0]);
    assert_eq!(
        dark.color(ColorRole::Primary),
        primary_only[0].color(ColorRole::Primary)
    );
    assert_ne!(
        dark.color(ColorRole::Secondary),
        primary_only[0].color(ColorRole::Secondary)
    );
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_gallery_captures_and_restores_its_own_arrangement() {
    let (mut ui, _) = mounted();
    let workspace = ui.capture_workspace().unwrap();
    assert_eq!(
        Workspace::decode(&workspace.encode()).unwrap(),
        workspace,
        "the gallery's own document round trips",
    );
    let report = ui.restore_workspace(&workspace).unwrap();
    assert!(report.is_complete(), "{report:?}");
    settle(&mut ui);
    assert_eq!(
        ui.capture_workspace().unwrap(),
        workspace,
        "restoring a capture of the current arrangement changes nothing",
    );
}

/// Keys the library creates for its own structure, none of which this gallery
/// uses as a key of its own. None may appear in a workspace path: a document
/// that carried them would be invalidated by an ordinary refactor inside a
/// container, and the chain of anchors, panels and viewports around a deeply
/// nested view pushes paths toward their byte limit. A page that legitimately
/// wants one of these words as its key removes it from this list.
const INTERNAL_KEYS: [&str; 16] = [
    "anchor",
    "popup",
    "surface",
    "dialog",
    "panel",
    "toasts",
    "branch",
    "confirmation",
    "group-content",
    "fixed-viewport",
    "regions",
    "lone-region",
    "shell-viewport",
    "tab-viewport",
    "empty",
    "custom",
];

#[test]
fn no_library_internal_key_reaches_a_workspace_path() {
    let (ui, _) = mounted();
    let workspace = ui.capture_workspace().unwrap();
    let mut longest = 0;
    for (path, _) in workspace.entries() {
        longest = longest.max(path.to_string().len());
        for segment in path.segments() {
            assert!(
                !INTERNAL_KEYS.contains(&segment) && !segment.starts_with("split-"),
                "internal key `{segment}` in {path}",
            );
        }
    }
    // Paths are bounded by Workspace::MAX_PATH_BYTES, and a capture fails
    // entirely when one exceeds it. The gallery is a deep tree; keeping its
    // longest path at less than half the limit is the margin an ordinary
    // application should have.
    assert!(longest < 256, "longest workspace path is {longest} bytes");
}

#[test]
fn the_workspace_page_keeps_an_oversized_document_out_of_its_editor() {
    let (mut ui, root) = mounted();
    // The gallery is far larger than a bound text field may hold, which is the
    // case the page has to survive: writing the document into the editor would
    // fail the component's update and end the flush.
    assert!(
        ui.capture_workspace().unwrap().encode().len() > TextEditError::MAX_TEXT_BYTES,
        "this test only means something while the gallery's document is oversized",
    );
    open(&mut ui, &root, "workspace");
    let capture = find(&ui, &root, "capture");
    click(&mut ui, &capture);
    settle(&mut ui);
    assert_eq!(
        text(&ui, "document"),
        [String::new()],
        "the editor stays empty; the document is kept aside",
    );
    let status = text(&ui, "workspace-status").join(" ");
    assert!(status.contains("kept aside"), "{status}");
    // Restoring uses the document that was kept aside, not the empty editor.
    let restore = find(&ui, &root, "restore");
    click(&mut ui, &restore);
    settle(&mut ui);
    let status = text(&ui, "workspace-status").join(" ");
    assert!(
        status.starts_with("Applied ") && status.contains("rejected 0"),
        "{status}",
    );
}

#[cfg(debug_assertions)]
#[test]
fn the_debug_overlay_page_marks_its_broken_sample_and_costs_no_frames() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "overlay");
    let clean = ui.paint().unwrap().commands().len();

    // Settings › Debug overlay › Validation issues, through the keyboard.
    choose_setting(&mut ui, 3, 1);
    assert_eq!(ui.debug_overlay(), DebugOverlay::Issues);
    assert!(ui.needs_paint(), "a changed overlay repaints once");
    assert_eq!(ui.paint().unwrap().commands().len(), clean);
    assert!(!ui.needs_paint());

    let toggle = find(&ui, &root, "break");
    ui.focus(&toggle).unwrap();
    press(&mut ui, KeyCode::Space);
    let mut codes: Vec<_> = ui
        .validation_report()
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect();
    codes.sort();
    assert_eq!(
        codes,
        [
            ValidationCode::UnexpectedOverlap,
            ValidationCode::EscapesParent
        ]
    );
    let marked = ui.validation_report().markers.len();
    assert_eq!(marked, 3, "First, Second and Third");
    let broken = ui.paint().unwrap().commands().len();
    assert!(
        broken >= clean + marked * 4,
        "{broken} commands, {clean} clean"
    );
    assert!(!ui.needs_paint());
    assert!(ui.next_deadline().is_none(), "marks schedule nothing");

    press(&mut ui, KeyCode::Space);
    assert!(ui.validation_report().issues.is_empty());
    ui.paint().unwrap();
    choose_setting(&mut ui, 3, 0);
    assert_eq!(ui.debug_overlay(), DebugOverlay::Off);
}
