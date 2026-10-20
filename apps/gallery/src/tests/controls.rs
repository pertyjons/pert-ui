//! The Controls category's pages.
use super::support::*;
use pert_ui::*;
use std::time::Duration;

#[test]
fn parameter_editors_share_values_cancel_and_host_reset() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "input");
    for key in ["editor-0", "editor-1"] {
        let button = find(&ui, &find(&ui, &root, key), "increase");
        click(&mut ui, &button);
    }
    assert_eq!(first_parameter(&mut ui).base, 10.0);
    assert_eq!(text(&ui, "readout").len(), 2);
    assert!(
        text(&ui, "readout")
            .iter()
            .all(|value| value.contains("10.00"))
    );
    let cancel = find(&ui, &root, "cancel-preview");
    click(&mut ui, &cancel);
    assert_eq!(first_parameter(&mut ui).base, 10.0);
    assert!(first_parameter(&mut ui).preview.is_none());
    let reset = find(&ui, &root, "host-reset");
    click(&mut ui, &reset);
    assert_eq!(first_parameter(&mut ui).base, 0.0);
    let pad = find(&ui, &root, "key-pad");
    ui.focus(&pad).unwrap();
    ui.key_input(KeyCode::KeyA, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::KeyS, KeyAction::Press).unwrap();
    settle(&mut ui);
    let values = text(&ui, "key-pad");
    assert!(values[0].contains("KeyA") && values[0].contains("KeyS"));
    ui.clear_focus();
    settle(&mut ui);
    assert!(!text(&ui, "key-pad")[0].contains("KeyA"));
}

#[test]
fn parameter_host_overlay_and_readonly_are_observable() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "input");
    let overlay = find(&ui, &root, "toggle-effective");
    click(&mut ui, &overlay);
    assert_eq!(first_parameter(&mut ui).effective, 25.0);
    let increase = find(&ui, &find(&ui, &root, "editor-0"), "increase");
    click(&mut ui, &increase);
    assert_eq!(first_parameter(&mut ui).base, 5.0);
    assert_eq!(first_parameter(&mut ui).effective, 25.0);
    click(&mut ui, &overlay);
    assert_eq!(first_parameter(&mut ui).effective, 5.0);
    let readonly = find(&ui, &root, "toggle-readonly");
    click(&mut ui, &readonly);
    click(&mut ui, &increase);
    assert_eq!(first_parameter(&mut ui).base, 5.0);
    assert!(text(&ui, "status")[0].starts_with("Edit rejected:"));
    click(&mut ui, &readonly);
    click(&mut ui, &increase);
    assert_eq!(first_parameter(&mut ui).base, 10.0);
}

#[test]
fn standard_controls_and_owned_surfaces_work_in_the_actual_gallery() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "controls");
    // The parameter families page has a checkbox keyed "checkbox" as well.
    let checkbox = find(&ui, &page(&ui, &root, "controls"), "checkbox");
    click(&mut ui, &checkbox);
    ui.flush(Duration::ZERO).unwrap();
    for key in ["checkbox", "toggle"] {
        assert!(ui.inspect().components.iter().any(|c| c.key == key
            && c.details.properties.get("checked") == Some(&InspectValue::Bool(false))));
    }
    let dropdown = find(&ui, &root, "dropdown");
    let anchor = find(&ui, &dropdown, "select");
    click(&mut ui, &anchor);
    press(&mut ui, KeyCode::ArrowDown);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert!(ui.inspect().components.iter().any(|c| c.key == "radio"
        && c.details.properties.get("selected")
            == Some(&InspectValue::Text("Second option".into()))));
    let menu = find(&ui, &root, "action-menu");
    let trigger = find(&ui, &menu, "trigger");
    click(&mut ui, &trigger);
    press(&mut ui, KeyCode::Enter);
    ui.flush(Duration::ZERO).unwrap();
    assert!(text(&ui, "menu-status").contains(&"Apply selected.".into()));
    let trigger = find(&ui, &root, "popover-trigger");
    click(&mut ui, &trigger);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.focused(), Some(find(&ui, &root, "close-popover")));
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    press(&mut ui, KeyCode::Escape);
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.focused(), Some(trigger));
    assert!(ui.next_deadline().is_none());
}

#[test]
fn inline_top_row_uses_real_bindings_and_resets_values() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "toolbar");
    let integer = find(&ui, &root, "inline-volume");
    ui.focus(&integer).unwrap();
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(text(&ui, "inline-count-saved"), vec!["Saved integer: 9"]);
    let decimal = find(&ui, &root, "inline-time");
    ui.focus(&decimal).unwrap();
    press(&mut ui, KeyCode::F2);
    ui.text_input("250").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(
        text(&ui, "inline-time-saved"),
        vec!["Saved canonical time: 0.25 seconds"]
    );
    let slider = find(&ui, &root, "inline-glide");
    ui.focus(&slider).unwrap();
    press(&mut ui, KeyCode::F2);
    ui.text_input("1.25").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert!(text(&ui, "inline-glide-saved")[0].contains("base 1.25"));
    let reset = find(&ui, &root, "inline-reset");
    click(&mut ui, &reset);
    assert_eq!(text(&ui, "inline-count-saved"), vec!["Saved integer: 8"]);
    assert!(ui.next_deadline().is_none());
    ui.paint().unwrap();
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn toolbar_page_collapses_when_narrow_and_drives_real_bindings() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "toolbar");
    let toolbar = find(&ui, &root, "global-toolbar");
    let collapsed = |ui: &Runtime| match ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "global-toolbar")
        .unwrap()
        .details
        .properties["collapsed"]
    {
        InspectValue::Number(n) => n,
        _ => unreachable!(),
    };
    assert_eq!(collapsed(&ui), 0.0);
    // The tempo stepper saves one step per press.
    let bpm = find(&ui, &toolbar, "bpm");
    ui.focus(&bpm).unwrap();
    press(&mut ui, KeyCode::ArrowUp);
    assert_eq!(text(&ui, "tempo-mirror"), vec!["Tempo: 121 bpm"]);
    let record = find(&ui, &toolbar, "record");
    click(&mut ui, &record);
    assert_eq!(text(&ui, "armed-mirror"), vec!["Armed: true"]);
    let name = find(&ui, &toolbar, "name");
    ui.focus(&name).unwrap();
    press(&mut ui, KeyCode::F2);
    ui.text_input("Bass").unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(text(&ui, "name-mirror"), vec!["Name: Bass"]);
    // The erase options group exists only while the erase tool is selected.
    let hidden = |ui: &Runtime| match ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "global-toolbar")
        .unwrap()
        .details
        .properties["hidden"]
    {
        InspectValue::Number(n) => n,
        _ => unreachable!(),
    };
    assert_eq!(hidden(&ui), 1.0);
    let mode = find(&ui, &toolbar, "mode");
    ui.focus(&mode).unwrap();
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(text(&ui, "tool-mirror"), vec!["Tool: erase"]);
    assert_eq!(hidden(&ui), 0.0);
    press(&mut ui, KeyCode::ArrowLeft);
    assert_eq!(text(&ui, "tool-mirror"), vec!["Tool: select"]);
    assert_eq!(hidden(&ui), 1.0);
    // Narrowing the window moves groups into the overflow surface; widening
    // restores them and the moved controls keep their bound values.
    ui.set_viewport(Viewport::new(420.0, 640.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert!(collapsed(&ui) >= 1.0);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
        .unwrap();
    settle(&mut ui);
    assert_eq!(collapsed(&ui), 0.0);
    assert_eq!(text(&ui, "name-mirror"), vec!["Name: Bass"]);
    ui.clear_focus();
    settle(&mut ui);
    assert!(ui.next_deadline().is_none());
    ui.paint().unwrap();
}

#[test]
fn the_field_status_page_checks_publishes_and_focuses_the_first_error() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "fields");
    let check = find(&ui, &root, "check-fields");
    click(&mut ui, &check);
    assert!(text(&ui, "field-status-outcome").contains(&"1 error(s), 0 warning(s)".into()));
    assert_eq!(ui.focused(), Some(find(&ui, &root, "status-name")));
    ui.text_input("Ada").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    click(&mut ui, &check);
    assert!(text(&ui, "field-status-outcome").contains(&"0 error(s), 0 warning(s)".into()));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_progress_page_animates_only_while_its_task_runs_and_is_visible() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "progress");
    assert!(ui.next_deadline().is_none(), "the page starts idle");
    let press = |ui: &mut Runtime, key: &str| {
        let button = find(ui, &root, key);
        click(ui, &button);
    };
    press(&mut ui, "progress-start");
    assert!(text(&ui, "progress-caption").contains(&"Running, extent unknown. Animating.".into()));
    assert!(
        ui.next_deadline().is_some(),
        "indeterminate activity animates"
    );

    // Settings › Motion › Reduced stills the bar; Full resumes the sweep.
    choose_setting(&mut ui, 2, 1);
    assert!(
        ui.next_deadline().is_none(),
        "reduced motion schedules nothing"
    );
    choose_setting(&mut ui, 2, 0);
    assert!(ui.next_deadline().is_some());

    // Leaving the page hides the bar, which withdraws its deadline; returning
    // resumes it, because the task is still running.
    open(&mut ui, &root, "layout");
    assert!(ui.next_deadline().is_none(), "a hidden bar leaves no wake");
    open(&mut ui, &root, "progress");
    assert!(ui.next_deadline().is_some());

    // A known fraction stops the sweep. The task is still running, so the spinner
    // beside the caption keeps its slow tick, and nothing at frame rate remains.
    press(&mut ui, "progress-step");
    assert!(
        text(&ui, "progress-caption")
            .contains(&"Step 1 of 4. The bar is still; only the spinner ticks.".into())
    );
    let now = ui.inspect().at;
    assert!(
        ui.next_deadline()
            .is_some_and(|at| at > now + Duration::from_millis(17)),
        "only the spinner's step remains"
    );
    press(&mut ui, "progress-start");
    press(&mut ui, "progress-fail");
    assert!(
        ui.next_deadline().is_none(),
        "a finished task leaves no wake"
    );
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn parameter_families_edit_through_controls_and_undo_through_the_host() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "families");
    let page = find(&ui, &root, "families");
    let boolean = find(&ui, &page, "boolean");
    let toggle = find(&ui, &boolean, "toggle");
    ui.focus(&toggle).unwrap();
    press(&mut ui, KeyCode::Space);
    assert_eq!(parameter_base(&mut ui, "bypass"), AnyValue::Boolean(true));
    let choice = find(&ui, &page, "choice");
    let segments = find(&ui, &choice, "segments");
    ui.focus(&segments).unwrap();
    press(&mut ui, KeyCode::End);
    assert_eq!(
        parameter_base(&mut ui, "waveform"),
        AnyValue::Choice(ChoiceId::new("square").unwrap())
    );
    let stepped = find(&ui, &page, "stepped");
    let inline = find(&ui, &stepped, "inline");
    ui.focus(&inline).unwrap();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Press).unwrap();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Repeat).unwrap();
    ui.key_input(KeyCode::ArrowUp, KeyAction::Release).unwrap();
    settle(&mut ui);
    assert_eq!(parameter_base(&mut ui, "voices"), AnyValue::Stepped(10));
    // The host's undo stack was fed by terminal notifications only.
    let undo = find(&ui, &page, "undo");
    for expected in [
        ("voices", AnyValue::Stepped(8)),
        ("waveform", AnyValue::Choice(ChoiceId::new("sine").unwrap())),
        ("bypass", AnyValue::Boolean(false)),
    ] {
        click(&mut ui, &undo);
        assert_eq!(parameter_base(&mut ui, expected.0), expected.1);
    }
}

#[test]
fn the_colors_page_parses_hex_applies_it_and_activates_its_swatch() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "colors");
    let semantics = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == key)
            .and_then(|item| item.details.semantics)
            .unwrap()
    };
    assert_eq!(
        semantics(&ui, "swatch-half").value,
        Some(SemanticValue::Text("#ff880080".into()))
    );
    let field = find(&ui, &root, "hex-field");
    ui.focus(&field).unwrap();
    settle(&mut ui);
    let key = |ui: &mut Runtime, code| {
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
        settle(ui);
    };
    key(&mut ui, KeyCode::End);
    for _ in 0.."#2090a0cc".len() {
        key(&mut ui, KeyCode::Backspace);
    }
    // Invalid text is refused and changes nothing.
    ui.text_input("#12345").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!(text(&ui, "hex-channels")[0].contains("sRGB bytes 32, 144, 160, alpha 204"));
    for _ in 0.."#12345".len() {
        key(&mut ui, KeyCode::Backspace);
    }
    ui.text_input("F80").unwrap();
    key(&mut ui, KeyCode::Enter);
    assert!(text(&ui, "hex-channels")[0].contains("sRGB bytes 255, 136, 0, alpha 255"));
    assert_eq!(
        semantics(&ui, "hex-swatch").value,
        Some(SemanticValue::Text("#ff8800".into()))
    );
    let swatch = find(&ui, &root, "hex-swatch");
    click(&mut ui, &swatch);
    assert!(text(&ui, "hex-activations")[0].starts_with("Activated 1 time(s)"));
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn the_colors_page_picker_edits_the_chosen_color_and_opens_from_a_swatch() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "colors");
    let inline = find(&ui, &root, "picker-inline");
    let alpha = ui.find_descendant(&inline, "alpha").unwrap().unwrap();
    ui.focus(&alpha).unwrap();
    settle(&mut ui);
    let key = |ui: &mut Runtime, code| {
        ui.key_input(code, KeyAction::Press).unwrap();
        ui.key_input(code, KeyAction::Release).unwrap();
        settle(ui);
    };
    key(&mut ui, KeyCode::End);
    // The picker writes the same color the hex entry and swatch present.
    assert!(text(&ui, "hex-channels")[0].contains("alpha 255"));
    let accent = find(&ui, &root, "accent");
    let swatch = ui.find_descendant(&accent, "swatch").unwrap().unwrap();
    click(&mut ui, &swatch);
    let area = ui.find_descendant(&accent, "area").unwrap().unwrap();
    assert_eq!(ui.focused(), Some(area));
    key(&mut ui, KeyCode::Escape);
    assert!(ui.validation_report().issues.is_empty());
    assert!(ui.next_deadline().is_none());
}

#[test]
fn knobs_edit_the_shared_parameter() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "input");
    let knob = find(&ui, &root, "knob");
    let centered = find(&ui, &root, "knob-centered");
    ui.focus(&knob).unwrap();
    press(&mut ui, KeyCode::End);
    assert_eq!(first_parameter(&mut ui).base, 100.0);
    // Both knobs present the one parameter; the centered one resets it too.
    ui.focus(&centered).unwrap();
    press(&mut ui, KeyCode::Backspace);
    assert_eq!(first_parameter(&mut ui).base, 0.0);
}

#[test]
fn the_property_grid_page_aligns_sections_and_checks_its_name() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "properties");
    let width = |ui: &Runtime, key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == key)
            .and_then(|item| item.details.properties.get("label_width").cloned())
            .unwrap()
    };
    assert_eq!(width(&ui, "general-grid"), InspectValue::Number(140.0));
    assert_eq!(width(&ui, "appearance-grid"), InspectValue::Number(140.0));
    let x = |ui: &Runtime, key: &str| ui.bounds(&find(ui, &root, key)).unwrap().unwrap().x();
    assert_eq!(x(&ui, "name-input"), x(&ui, "offset-input"));
    // Keyboard resizing on one grid moves both columns once it commits.
    let general = find(&ui, &root, "general-grid");
    ui.focus(&general).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::ArrowRight);
    assert_eq!(width(&ui, "appearance-grid"), InspectValue::Number(148.0));
    assert_eq!(x(&ui, "name-input"), x(&ui, "offset-input"));
    assert!(text(&ui, "property-width")[0].contains("148"));
    let reset = find(&ui, &root, "property-reset-width");
    click(&mut ui, &reset);
    assert_eq!(width(&ui, "general-grid"), InspectValue::Number(140.0));

    // An empty name fails the gallery's rule; focus moves to its editor.
    let name = find(&ui, &root, "name-input");
    ui.focus(&name).unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::End);
    for _ in 0.."Channel 1".len() {
        press(&mut ui, KeyCode::Backspace);
    }
    let check = find(&ui, &root, "property-check");
    click(&mut ui, &check);
    assert_eq!(text(&ui, "property-outcome")[0], "1 error(s)");
    assert_eq!(ui.focused(), Some(name));
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn the_colors_page_quick_pick_chooses_a_named_color_and_refuses_a_disabled_one() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "colors");
    let status = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|item| item.key == "named-picker")
            .and_then(|item| item.details.properties.get("status").cloned())
    };
    let opener = find(&ui, &root, "named-open");
    click(&mut ui, &opener);
    settle(&mut ui);
    assert_eq!(status(&ui), Some(InspectValue::Text("Open".into())));
    ui.text_input("steel").unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(text(&ui, "named-status"), ["Chose Steel blue."]);
    assert!(text(&ui, "hex-channels")[0].contains("sRGB bytes 70, 130, 180, alpha 255"));
    assert_eq!(status(&ui), Some(InspectValue::Text("Closed".into())));
    // The disabled row is offered, but choosing it is refused and the picker
    // stays open.
    click(&mut ui, &opener);
    settle(&mut ui);
    ui.text_input("ultra").unwrap();
    settle(&mut ui);
    press(&mut ui, KeyCode::Enter);
    settle(&mut ui);
    assert_eq!(status(&ui), Some(InspectValue::Text("Open".into())));
    assert_eq!(text(&ui, "named-status"), ["Chose Steel blue."]);
    assert!(ui.validation_report().issues.is_empty());
}

#[test]
fn toolbar_stop_is_a_command_button_enabled_while_playing() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "toolbar");
    let toolbar = find(&ui, &root, "global-toolbar");
    let f7 = |ui: &mut Runtime| {
        for action in [KeyAction::Press, KeyAction::Release] {
            ui.command_key_input(CommandKeyInput::new(
                KeyCode::F7,
                action,
                CommandKey::Named(KeyCode::F7),
            ))
            .unwrap();
        }
        settle(ui);
    };
    // Other pages have "message" labels too: read the status bar's own.
    let message = find(&ui, &find(&ui, &root, "status-bar"), "message");
    let message = ui.inspection_id(&message).unwrap();
    let stopped = |ui: &Runtime| {
        ui.inspect()
            .components
            .iter()
            .find(|c| c.id == message)
            .and_then(|c| c.details.properties.get("text").cloned())
            == Some(InspectValue::Text("Stop pressed (synthetic).".into()))
    };
    // Nothing plays yet, so the command and its button are disabled.
    f7(&mut ui);
    assert!(!stopped(&ui));
    let play = find(&ui, &toolbar, "play");
    click(&mut ui, &play);
    let stop = find(&ui, &find(&ui, &toolbar, "stop"), "button");
    click(&mut ui, &stop);
    settle(&mut ui);
    assert!(stopped(&ui));
    click(&mut ui, &play);
    f7(&mut ui);
    assert!(stopped(&ui));
}

#[test]
fn the_progress_page_clock_ticks_until_stopped_also_without_focus() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "progress");
    assert!(ui.next_deadline().is_none(), "nothing runs before Start");
    let start = find(&ui, &root, "clock-start");
    click(&mut ui, &start);
    let now = ui.inspect().at;
    assert!(ui.next_deadline().is_some(), "the timer is scheduled");
    // Focus elsewhere, then cancelled input: the application timer runs on.
    ui.clear_focus();
    ui.cancel_input().unwrap();
    ui.flush(now + Duration::from_secs(1)).unwrap();
    ui.flush(now + Duration::from_secs(2)).unwrap();
    assert_eq!(text(&ui, "clock-ticks"), ["Ticks: 2"]);
    let stop = find(&ui, &root, "clock-stop");
    click(&mut ui, &stop);
    ui.flush(now + Duration::from_secs(5)).unwrap();
    // Two Starts in a row start one clock, which Stop ends.
    ui.focus(&start).unwrap();
    for _ in 0..2 {
        ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
        ui.key_input(KeyCode::Enter, KeyAction::Release).unwrap();
    }
    settle(&mut ui);
    click(&mut ui, &stop);
    settle(&mut ui);
    assert!(
        ui.next_deadline().is_none(),
        "Stop ends every clock started"
    );
    assert_eq!(text(&ui, "clock-ticks"), ["Ticks: 2"]);
    assert!(
        ui.next_deadline().is_none(),
        "nothing is scheduled once stopped"
    );
}

#[test]
fn the_power_mapped_parameter_spreads_its_low_end() {
    let (mut ui, root) = mounted();
    open(&mut ui, &root, "families");
    let page = find(&ui, &root, "families");
    let power = find(&ui, &page, "power");
    let slider = find(&ui, &power, "slider");
    ui.focus(&slider).unwrap();
    press(&mut ui, KeyCode::Home);
    assert_eq!(parameter_base(&mut ui, "drive"), AnyValue::Continuous(0.0));
    press(&mut ui, KeyCode::End);
    assert_eq!(parameter_base(&mut ui, "drive"), AnyValue::Continuous(10.0));
    // Halfway along the travel: 2.5 with the exponent of two, where a linear
    // mapping would give 5.
    press(&mut ui, KeyCode::Home);
    for _ in 0..5 {
        press(&mut ui, KeyCode::PageUp);
    }
    let AnyValue::Continuous(half) = parameter_base(&mut ui, "drive") else {
        panic!("a continuous parameter");
    };
    assert!((half - 2.5).abs() < 0.05, "{half}");
}
