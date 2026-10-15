//! The Controls › Chips & breadcrumbs page.
use super::support::*;
use pert_ui::*;

#[test]
fn removable_chips_report_removal_and_focus_moves_along_the_row() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "chips");
    let editor = find(&ui, &root, "tag-editor");
    ui.focus(&find(&ui, &editor, "Review")).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Delete);
    assert_eq!(text(&ui, "status"), ["Removed tag Review"]);
    let focused = ui.focused().unwrap();
    assert_eq!(ui.key(&focused).unwrap(), "Urgent");
    assert!(ui.find_descendant(&editor, "Review").unwrap().is_none());
    let reset = find(&ui, &root, "reset-tags");
    click(&mut ui, &reset);
    assert!(ui.find_descendant(&editor, "Review").unwrap().is_some());
}

#[test]
fn multi_select_choosers_filter_toggle_and_commit_as_documented() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "chips");
    let each = find(&ui, &root, "instruments");
    let trigger = find(&ui, &each, "anchor");
    // Enter on the trigger opens the popup and focuses its search field.
    click(&mut ui, &trigger);
    ui.text_input("tr").unwrap();
    settle(&mut ui);
    // Trombone, then Trumpet; Down moves to Trumpet and Enter toggles it.
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "instruments-value"),
        ["Bound value: [piano, violin, trumpet]"]
    );
    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::Escape, KeyAction::Release).unwrap();
    settle(&mut ui);

    let ensemble = find(&ui, &root, "ensemble");
    let trigger = find(&ui, &ensemble, "anchor");
    click(&mut ui, &trigger);
    press(&mut ui, KeyCode::Enter); // Bass into the draft
    assert_eq!(text(&ui, "ensemble-value"), ["Bound value: [cello]"]);
    // Tab visits the scrolling list, then leaves the popup and writes the draft.
    press(&mut ui, KeyCode::Tab);
    assert_eq!(text(&ui, "ensemble-value"), ["Bound value: [cello]"]);
    press(&mut ui, KeyCode::Tab);
    assert_eq!(text(&ui, "ensemble-value"), ["Bound value: [cello, bass]"]);
    assert!(ui.validation_report().issues.is_empty());
}

/// How many segments the breadcrumb keyed `key` keeps behind its overflow button.
fn collapsed(ui: &Runtime, key: &str) -> f64 {
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no breadcrumb {key}"));
    match component.details.properties.get("collapsed") {
        Some(InspectValue::Number(count)) => *count,
        other => panic!("collapsed is {other:?}"),
    }
}

#[test]
fn breadcrumbs_navigate_and_the_narrow_one_collapses_its_middle() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "chips");
    let wide = find(&ui, &root, "wide-location");
    let narrow = find(&ui, &root, "narrow-location");
    assert_eq!(collapsed(&ui, "wide-crumbs"), 0.0);
    assert!(collapsed(&ui, "narrow-crumbs") > 0.0);
    let user = find(&ui, &wide, "/home/user");
    click(&mut ui, &user);
    assert_eq!(text(&ui, "status"), ["Navigated to /home/user"]);
    for location in [&wide, &narrow] {
        let gone = ui.find_descendant(location, "/home/user/projects").unwrap();
        assert!(gone.is_none());
    }
    let reset = find(&ui, &root, "open-deepest");
    click(&mut ui, &reset);
    let mixes = "/home/user/projects/album/mixes";
    assert!(ui.find_descendant(&narrow, mixes).unwrap().is_some());
    assert!(collapsed(&ui, "narrow-crumbs") > 0.0);
}
