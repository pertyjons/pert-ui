//! The Surfaces category's pages.
use super::support::*;
use pert_ui::*;

#[test]
fn floating_window_dialog_uses_the_properties_view_as_its_modal_area() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "windows");
    let owner = find(&ui, &root, "properties-dialog");
    let opener = find(&ui, &owner, "open-window-dialog");
    let owner_bounds = ui.bounds(&owner).unwrap().unwrap();
    let opener_bounds = ui.bounds(&opener).unwrap().unwrap();
    assert!(owner_bounds.width() > opener_bounds.width() * 2.0);
    assert!(owner_bounds.height() > opener_bounds.height() * 3.0);

    click(&mut ui, &opener);
    let panel = find(&ui, &owner, "dialog-body");
    let panel_bounds = ui.bounds(&panel).unwrap().unwrap();
    assert!(panel_bounds.width() > opener_bounds.width());
    assert!(panel_bounds.height() > opener_bounds.height());
    assert!(panel_bounds.x() >= owner_bounds.x());
    assert!(panel_bounds.y() >= owner_bounds.y());
    assert!(panel_bounds.x() + panel_bounds.width() <= owner_bounds.x() + owner_bounds.width());
    assert!(panel_bounds.y() + panel_bounds.height() <= owner_bounds.y() + owner_bounds.height());
    let dismiss = find(&ui, &owner, "dialog-dismiss");
    assert_eq!(ui.focused(), Some(dismiss.clone()));
    click(&mut ui, &dismiss);
    assert!(
        !ui.inspect()
            .components
            .iter()
            .any(|component| { component.key == "dialog-body" && component.visible })
    );
}

#[test]
fn gallery_dialogs_nested_menus_banner_and_toast_use_public_input() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "surfaces");
    let trigger = find(&ui, &root, "open-task");
    click(&mut ui, &trigger);
    let nested = find(&ui, &root, "open-nested");
    click(&mut ui, &nested);
    assert_eq!(ui.key(&ui.focused().unwrap()).unwrap(), "cancel");
    press(&mut ui, KeyCode::Escape);
    assert_eq!(ui.focused(), Some(nested));
    let accept = find(&ui, &root, "task-accept");
    click(&mut ui, &accept);
    assert_eq!(text(&ui, "task-result"), ["Dialog: Finished(Accepted)"]);
    let alert = find(&ui, &root, "open-alert");
    click(&mut ui, &alert);
    press(&mut ui, KeyCode::Escape);
    assert_eq!(text(&ui, "alert-result"), ["Dialog: Finished(Cancelled)"]);
    let menu = find(&ui, &root, "nested-menu");
    let menu_trigger = find(&ui, &menu, "trigger");
    click(&mut ui, &menu_trigger);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "transient-status"), ["Nested action applied."]);
    let dismiss = find(&ui, &root, "dismiss-banner");
    click(&mut ui, &dismiss);
    let show = find(&ui, &root, "show-banner");
    click(&mut ui, &show);
    let toast = find(&ui, &root, "post-action-toast");
    click(&mut ui, &toast);
    let stack = find(&ui, &root, "toasts");
    ui.focus(&stack).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "transient-status"),
        ["Notification action applied."]
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    assert!(ui.next_deadline().is_none());
    ui.paint().unwrap();
}

#[test]
fn the_menu_bar_page_runs_commands_and_settings_from_the_keyboard() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "menus");
    let document = find(&ui, &root, "document-focus");
    ui.focus(&document).unwrap();
    settle(&mut ui);

    // F10, Down opens File; its first row is the New sketch command.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(
        text(&ui, "menu-bar-status"),
        ["Created sketch 1; Save is now available."]
    );
    assert_eq!(ui.focused(), Some(document.clone()));

    // Save became available through the command's enabled binding.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(text(&ui, "menu-bar-status"), ["Saved."]);

    // View › Show grid flips the setting shown in the document.
    press(&mut ui, KeyCode::F10);
    press(&mut ui, KeyCode::ArrowRight);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "grid-setting"), ["Grid hidden."]);
    // The row's `on_change` reports the user's own choice.
    assert_eq!(text(&ui, "menu-bar-status"), ["You turned the grid off."]);
    assert_eq!(ui.focused(), Some(document.clone()));

    // The trailing slot's button is a Tab stop on the title row that F10 skips.
    let sync = find(&ui, &root, "menu-bar-sync");
    let title = find(&ui, &root, "title/view");
    let [_, sync_y, _, sync_h] = ui.bounds(&sync).unwrap().unwrap().values();
    let [_, title_y, _, title_h] = ui.bounds(&title).unwrap().unwrap().values();
    assert!((sync_y + sync_h / 2.0 - (title_y + title_h / 2.0)).abs() < 0.01);
    ui.focus(&sync).unwrap();
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(
        text(&ui, "menu-bar-status"),
        ["Synchronized from the bar's trailing slot."]
    );
    press(&mut ui, KeyCode::F10);
    assert_eq!(ui.key(&ui.focused().unwrap()).unwrap(), "title/file");
}
