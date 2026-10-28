//! The Text category's pages.
use super::support::*;
use pert_ui::prelude::*;
use pert_ui::testing::HostIntegration;
use pert_ui::{InspectValue, KeyAction};
use std::time::Duration;

#[test]
fn font_and_icon_overrides_update_the_gallery_and_restore_idle() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "fonts");
    let before = ui.paint().unwrap();
    let alternate = find(&ui, &root, "alternate-font");
    click(&mut ui, &alternate);
    let changed = ui.paint().unwrap();
    assert_ne!(before, changed);
    let reset = find(&ui, &root, "default-font");
    click(&mut ui, &reset);
    ui.paint().unwrap();
    let alternate = find(&ui, &root, "alternate-icon");
    click(&mut ui, &alternate);
    ui.paint().unwrap();
    let reset = find(&ui, &root, "default-icons");
    click(&mut ui, &reset);
    ui.paint().unwrap();
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    assert!(ui.next_deadline().is_none());
    assert!(!ui.needs_paint());
}

#[test]
fn the_fonts_page_shows_every_library_icon() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "fonts");
    let grid = find(&ui, &root, "icons");
    for icon in Icon::ALL {
        find(&ui, &grid, &format!("{icon:?}"));
    }
    assert!(Icon::ALL.contains(&Icon::OpenInNew));
}

#[test]
fn the_text_input_page_shows_every_field_status_and_a_live_rule() {
    fn status(ui: &Runtime, key: &str) -> String {
        let snapshot = ui.inspect();
        let field = snapshot
            .components
            .iter()
            .find(|item| item.key == key)
            .unwrap();
        match field.details.properties.get("status") {
            Some(InspectValue::Text(status)) => status.clone(),
            other => panic!("{key} has no status: {other:?}"),
        }
    }
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    for (key, expected) in [
        ("field-normal", "normal"),
        ("field-warning", "warning"),
        ("field-error", "error"),
        ("field-marked", "error"),
        ("field-area", "warning"),
        ("field-read-only", "normal"),
        ("field-unlabelled", "error"),
    ] {
        assert_eq!(status(&ui, key), expected, "{key}");
    }

    // The live field starts normal; its rule runs on Enter and is the gallery's.
    assert_eq!(status(&ui, "field-live"), "normal");
    let input = find(&ui, &root, "field-live-input");
    let enter = |ui: &mut Runtime| {
        ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
        ui.flush(Duration::ZERO).unwrap();
    };
    ui.focus(&input).unwrap();
    enter(&mut ui);
    assert_eq!(status(&ui, "field-live"), "error");
    ui.text_input("Demo").unwrap();
    enter(&mut ui);
    assert_eq!(status(&ui, "field-live"), "normal");
    ui.text_input("A considerably longer ").unwrap();
    enter(&mut ui);
    assert_eq!(status(&ui, "field-live"), "warning");
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_search_example_filters_highlights_chooses_and_clears() {
    const ALL: &str =
        "Matches: Compressor, Delay, Equalizer, Gate, Limiter, Phaser, Reverb, Saturation";
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    let field = find(&ui, &root, "search-field");
    ui.focus(&field).unwrap();
    settle(&mut ui);
    ui.text_input("a").unwrap();
    settle(&mut ui);
    assert_eq!(
        text(&ui, "search-results"),
        [ALL],
        "the debounced query has not reached the list yet"
    );
    ui.text_input("t").unwrap();
    let now = ui.inspect().at;
    ui.flush(now + Duration::from_millis(200)).unwrap();
    assert_eq!(text(&ui, "search-results"), ["Matches: Gate, Saturation"]);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "search-chosen"), ["Chosen: Saturation"]);
    assert_eq!(ui.focused(), Some(field.clone()));
    press(&mut ui, KeyCode::Escape);
    assert_eq!(text(&ui, "search-results"), [ALL]);
    assert_eq!(ui.focused(), Some(field));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_sign_in_example_conceals_limits_and_shows_a_pending_check() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    let user = find(&ui, &root, "user-input");
    ui.focus(&user).unwrap();
    settle(&mut ui);
    ui.text_input("ad").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "sign-in-summary"),
        ["Errors: 1, pending checks: 0, unsaved fields: 1"],
        "a two-character name is refused and stays an unsaved draft"
    );
    ui.text_input("min").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "sign-in-summary"),
        ["Errors: 0, pending checks: 0, unsaved fields: 0"]
    );
    let check = find(&ui, &root, "check-name");
    click(&mut ui, &check);
    assert_eq!(
        text(&ui, "sign-in-summary"),
        ["Errors: 0, pending checks: 1, unsaved fields: 0"]
    );
    assert!(ui.next_deadline().is_some(), "the pending spinner steps");
    let finish = find(&ui, &root, "finish-check");
    click(&mut ui, &finish);
    assert_eq!(
        text(&ui, "sign-in-summary"),
        ["Errors: 1, pending checks: 0, unsaved fields: 0"]
    );
    // The password field conceals what is typed.
    let password = find(&ui, &root, "password-input");
    ui.focus(&password).unwrap();
    settle(&mut ui);
    ui.text_input("correct horse").unwrap();
    settle(&mut ui);
    let details = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "password-input")
        .unwrap()
        .details;
    assert!(!details.properties.contains_key("text"));
    assert!(details.semantics.unwrap().protected);
    // The field reports its unsaved draft while the flush delivers
    // notifications; the summary label is derived from the form's summary and
    // follows on the next flush, which a native host runs at once.
    settle(&mut ui);
    assert_eq!(
        text(&ui, "sign-in-summary"),
        ["Errors: 1, pending checks: 0, unsaved fields: 1"],
        "the typed password is an unsaved draft until it saves"
    );
    assert_eq!(details.properties["clear_shown"], InspectValue::Bool(true));
    // The delay field shows its icon and unit around the text.
    let delay = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "delay-input")
        .unwrap()
        .details;
    assert_eq!(delay.properties["suffix"], InspectValue::Text("ms".into()));
    assert_eq!(delay.properties["leading_icon"], InspectValue::Bool(true));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_truncation_section_cuts_only_labels_that_do_not_fit() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "fonts");
    let truncated = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == key)
            .unwrap()
            .details
            .properties
            .get("truncated")
            .cloned()
    };
    assert_eq!(truncated(&ui, "narrow-end"), Some(InspectValue::Bool(true)));
    assert_eq!(
        truncated(&ui, "narrow-middle"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(truncated(&ui, "wide-end"), Some(InspectValue::Bool(false)));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_search_example_highlights_what_matched() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    let field = find(&ui, &root, "search-field");
    ui.focus(&field).unwrap();
    ui.text_input("verb").unwrap();
    ui.flush(Duration::from_millis(500)).unwrap();
    let results = ui
        .inspect()
        .components
        .into_iter()
        .find(|item| item.key == "search-results")
        .unwrap();
    assert_eq!(
        results.details.properties.get("text"),
        Some(&InspectValue::Text("Matches: Reverb".into()))
    );
    ui.paint().unwrap();
}

#[test]
fn an_ordinary_number_field_shows_its_format_at_rest_and_edits_plain_text() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "numbers");
    let property = |ui: &Runtime, name: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == "formatted-input")
            .and_then(|item| item.details.properties.get(name).cloned())
    };
    // At rest the field shows the format's display text and its unit.
    assert_eq!(
        property(&ui, "shows_display"),
        Some(InspectValue::Bool(true))
    );
    assert_eq!(
        property(&ui, "display_value"),
        Some(InspectValue::Text("35.0".into()))
    );
    let field = find(&ui, &root, "formatted-input");
    ui.focus(&field).unwrap();
    settle(&mut ui);
    // Focused, it edits the plain percentage; switching never changes the value.
    assert_eq!(
        property(&ui, "shows_display"),
        Some(InspectValue::Bool(false))
    );
    assert_eq!(property(&ui, "text"), Some(InspectValue::Text("35".into())));
    assert_eq!(text(&ui, "formatted-saved"), ["Saved fraction: 0.35"]);
    press(&mut ui, KeyCode::End);
    for _ in 0.."35".len() {
        press(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("50").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "formatted-saved"), ["Saved fraction: 0.5"]);
    // A step of 1 % adds 0.01 to the fraction, as a decimal number.
    press(&mut ui, KeyCode::ArrowUp);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "formatted-saved"), ["Saved fraction: 0.51"]);
    ui.clear_focus();
    settle(&mut ui);
    assert_eq!(
        property(&ui, "display_value"),
        Some(InspectValue::Text("51.0".into()))
    );
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn search_highlight_readout_and_submission_share_the_current_query() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    let field = find(&ui, &root, "search-field");
    ui.focus(&field).unwrap();
    press(&mut ui, KeyCode::ArrowDown);
    assert_eq!(text(&ui, "search-highlight"), ["Highlighted match: 2"]);
    ui.text_input("a").unwrap();
    let now = ui.inspect().at;
    ui.flush(now + Duration::from_millis(200)).unwrap();
    assert_eq!(text(&ui, "search-highlight"), ["Highlighted match: 1"]);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "search-chosen"), ["Chosen: Delay"]);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(text(&ui, "search-highlight"), ["Highlighted match: 1"]);
    ui.text_input("xyz").unwrap();
    let now = ui.inspect().at;
    ui.flush(now + Duration::from_millis(200)).unwrap();
    assert_eq!(text(&ui, "search-results"), ["No matches"]);
    assert_eq!(text(&ui, "search-highlight"), ["No highlighted match."]);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "search-chosen"), ["No match to choose."]);
}

#[test]
fn the_refusing_field_reports_a_refused_write_and_its_end() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "editing");
    assert_eq!(
        text(&ui, "refusal-report"),
        ["The application's writes are accepted."]
    );
    let refuse = find(&ui, &root, "write-line-break");
    click(&mut ui, &refuse);
    let report = text(&ui, "refusal-report").join("");
    assert!(
        report.starts_with("The field refused the write: "),
        "{report}"
    );
    let field = find(&ui, &root, "refusal-field");
    let id = ui.inspection_id(&field).unwrap();
    let shown = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.id == id)
        .and_then(|c| c.details.properties.get("text").cloned());
    assert_eq!(
        shown,
        Some(InspectValue::Text("Single line".into())),
        "the previous text stays"
    );
    let accept = find(&ui, &root, "write-single-line");
    click(&mut ui, &accept);
    assert_eq!(
        text(&ui, "refusal-report"),
        ["The application's writes are accepted again."]
    );
}
