//! Typed role, name, value, state and actions in the inspection snapshot, read
//! through the public facade exactly as a tool or a later adapter would.
use crate::support;
use pert_ui::*;
use std::time::Duration;
use support::{press, widget};

fn ui() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 600.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn component(ui: &Runtime, key: &str) -> ComponentInspection {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .unwrap_or_else(|| panic!("no component {key}"))
}
fn semantics(ui: &Runtime, key: &str) -> Semantics {
    component(ui, key)
        .details
        .semantics
        .unwrap_or_else(|| panic!("{key} reports no semantics"))
}

#[test]
fn controls_report_role_name_value_and_the_actions_their_state_allows() {
    let mut ui = ui();
    let armed = ui.signal(false);
    let mode = ui.signal("b");
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("title", Label::new("Transport")),
            Element::new("play", Button::new("Play", |_| Ok(()))),
            Element::new("stop", Button::new("Stop", |_| Ok(())).disabled()),
            Element::new("arm", Checkbox::new(armed.clone(), "Arm")),
            Element::new("loop", Toggle::new(ui.signal(true), "Loop")),
            Element::new(
                "mode",
                RadioGroup::new(
                    mode,
                    [Choice::new("a", "First"), Choice::new("b", "Second")],
                )
                .unwrap(),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    let title = semantics(&ui, "title");
    assert_eq!(title.role, SemanticRole::Label);
    assert_eq!(title.name.as_deref(), Some("Transport"));
    assert!(title.actions.is_empty());

    let play = semantics(&ui, "play");
    assert_eq!((play.role, play.disabled), (SemanticRole::Button, false));
    assert_eq!(play.name.as_deref(), Some("Play"));
    assert_eq!(play.actions, [SemanticAction::Activate]);
    // A disabled control keeps its role and name and offers nothing.
    let stop = semantics(&ui, "stop");
    assert!(stop.disabled && stop.actions.is_empty());
    assert_eq!(stop.name.as_deref(), Some("Stop"));

    let arm = semantics(&ui, "arm");
    assert_eq!(arm.role, SemanticRole::Checkbox);
    assert_eq!(arm.value, Some(SemanticValue::Bool(false)));
    assert_eq!(arm.actions, [SemanticAction::Toggle]);
    armed.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(semantics(&ui, "arm").value, Some(SemanticValue::Bool(true)));
    assert_eq!(semantics(&ui, "loop").role, SemanticRole::Switch);

    let mode = semantics(&ui, "mode");
    assert_eq!(mode.role, SemanticRole::RadioGroup);
    assert_eq!(
        mode.value,
        Some(SemanticValue::Choice {
            selected: Some(1),
            count: 2,
            label: Some("Second".into()),
        })
    );
    assert_eq!(mode.actions, [SemanticAction::Select]);

    // A layout-only container describes nothing rather than claiming a role.
    assert_eq!(component(&ui, "column").details.semantics, None);
}

#[test]
fn editors_and_numbers_report_typed_values_and_editability() {
    let mut ui = ui();
    let count = ui.signal(7_i64);
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("name", TextField::new(ui.signal("Ada".to_owned()))),
            Element::new(
                "notes",
                TextArea::new(ui.signal("One\nTwo".to_owned())).read_only(),
            ),
            Element::new("count", IntegerInput::new(count).range(0, 10).unwrap()),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    let name = semantics(&ui, "name");
    assert_eq!(name.role, SemanticRole::TextInput);
    assert_eq!(name.value, Some(SemanticValue::Text("Ada".into())));
    assert_eq!(name.actions, [SemanticAction::SetValue]);
    assert_eq!(name.name, None, "a bare editor has no name of its own");

    let notes = semantics(&ui, "notes");
    assert_eq!(notes.role, SemanticRole::MultilineTextInput);
    assert!(notes.read_only && !notes.disabled && notes.actions.is_empty());

    let count = semantics(&ui, "count");
    assert_eq!(count.role, SemanticRole::NumberInput);
    assert_eq!(
        count.value,
        Some(SemanticValue::Number {
            value: 7.0,
            minimum: Some(0.0),
            maximum: Some(10.0),
        })
    );
    assert_eq!(
        count.actions,
        [
            SemanticAction::SetValue,
            SemanticAction::Increment,
            SemanticAction::Decrement
        ],
        "actions are sorted in declaration order and unique"
    );
    assert!(!count.invalid);
}

#[test]
fn a_field_names_describes_and_invalidates_the_control_it_wraps() {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::Normal);
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "field",
                Field::new(Element::new(
                    "email",
                    TextField::new(ui.signal(String::new())),
                ))
                .label("Email")
                .help("Used for receipts")
                .status_signal(status.read_only()),
            ),
            Element::new(
                "agree-field",
                Field::new(Element::new(
                    "agree",
                    Checkbox::new(ui.signal(false), "I agree"),
                ))
                .label("Terms"),
            ),
            Element::new("bare", TextField::new(ui.signal(String::new()))),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    let field = component(&ui, "field");
    let field_semantics = field.details.semantics.clone().unwrap();
    assert_eq!(field_semantics.role, SemanticRole::Field);
    assert!(field_semantics.names_children);

    // The control had no name, so it takes the field's and records where from.
    let email = component(&ui, "email");
    let email_semantics = email.details.semantics.unwrap();
    assert_eq!(email.labelled_by, Some(field.id));
    assert_eq!(email_semantics.name.as_deref(), Some("Email"));
    assert_eq!(
        email_semantics.description.as_deref(),
        Some("Used for receipts")
    );
    assert!(!email_semantics.invalid);

    // A control with its own name keeps it and is not relabelled.
    let agree = component(&ui, "agree");
    assert_eq!(agree.labelled_by, None);
    assert_eq!(
        agree.details.semantics.unwrap().name.as_deref(),
        Some("I agree")
    );
    // Outside a field nothing is invented.
    let bare = component(&ui, "bare");
    assert_eq!(
        (bare.labelled_by, bare.details.semantics.unwrap().name),
        (None, None)
    );

    // An error replaces the help text as the description and marks the control
    // invalid; a warning describes it without doing so.
    status.set(FieldStatus::error("Not an address")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let email = semantics(&ui, "email");
    assert!(email.invalid);
    assert_eq!(email.description.as_deref(), Some("Not an address"));
    status.set(FieldStatus::warning("Looks unusual")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let email = semantics(&ui, "email");
    assert!(!email.invalid);
    assert_eq!(email.description.as_deref(), Some("Looks unusual"));
}

#[test]
fn a_field_error_or_warning_outranks_the_description_a_drafting_control_gives_itself() {
    let mut ui = ui();
    let status = ui.signal(FieldStatus::Normal);
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([
                Element::new(
                    "field",
                    Field::new(Element::new(
                        "name",
                        TextField::new(ui.signal(String::from("Ada"))).normalize(Ok),
                    ))
                    .label("Name")
                    .help("As on the card")
                    .status_signal(status.read_only()),
                ),
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let name = widget(&ui, &root, "name");
    ui.focus(&name).unwrap();
    ui.text_input("!").unwrap();
    ui.flush(Duration::ZERO).unwrap();
    // A valid field keeps the control's own description of its draft.
    assert_eq!(
        semantics(&ui, "name").description.as_deref(),
        Some("Unsaved changes")
    );
    status.set(FieldStatus::error("No !")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let name = semantics(&ui, "name");
    assert!(name.invalid && !name.warning);
    assert_eq!(
        name.description.as_deref(),
        Some("No !"),
        "the field's error describes the control it marks invalid"
    );
    status.set(FieldStatus::warning("Looks unusual")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let name = semantics(&ui, "name");
    assert!(!name.invalid && name.warning);
    assert_eq!(
        name.description.as_deref(),
        Some("Looks unusual"),
        "the field's warning describes the control too"
    );
    let field = semantics(&ui, "field");
    assert!(field.warning, "the field marks its own description");
    // A warning without a message leaves the help text, which is no warning
    // and leaves the control's own description.
    status.set(FieldStatus::warning("")).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let name = semantics(&ui, "name");
    assert!(!name.warning);
    assert_eq!(name.description.as_deref(), Some("Unsaved changes"));
}

#[test]
fn a_select_reports_its_choice_and_whether_its_list_is_open() {
    let mut ui = ui();
    let value = ui.signal(2_u8);
    let select = Select::new(
        &ui,
        value,
        [
            Choice::new(1_u8, "One"),
            Choice::new(2_u8, "Two"),
            Choice::new(3_u8, "Three"),
        ],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("owner", select)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    // The owner only hosts the surface; the anchor is the combo box.
    assert_eq!(component(&ui, "owner").details.semantics, None);
    let anchor = semantics(&ui, "select");
    assert_eq!(anchor.role, SemanticRole::ComboBox);
    assert_eq!(
        anchor.value,
        Some(SemanticValue::Choice {
            selected: Some(1),
            count: 3,
            label: Some("Two".into()),
        })
    );
    assert_eq!(anchor.expanded, Some(false));
    assert_eq!(anchor.actions, [SemanticAction::Expand]);

    ui.focus(&widget(&ui, &root, "select")).unwrap();
    press(&mut ui, KeyCode::Enter);
    let anchor = semantics(&ui, "select");
    assert_eq!(anchor.expanded, Some(true));
    assert_eq!(anchor.actions, [SemanticAction::Collapse]);
}

#[test]
fn a_custom_component_describes_itself_through_the_same_contract() {
    struct Dial(f32);
    impl Component for Dial {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn inspect(&self) -> ComponentDetails {
            ComponentDetails {
                semantics: Some(
                    Semantics::new(SemanticRole::Slider)
                        .name("Cutoff")
                        .value(SemanticValue::Number {
                            value: self.0.into(),
                            minimum: Some(0.0),
                            maximum: Some(1.0),
                        })
                        // Repeated and unordered input still yields a sorted set.
                        .actions([
                            SemanticAction::Decrement,
                            SemanticAction::SetValue,
                            SemanticAction::Decrement,
                        ]),
                ),
                ..ComponentDetails::default()
            }
        }
    }
    let mut ui = ui();
    ui.mount_tree(Element::new("dial", Dial(0.25))).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let dial = semantics(&ui, "dial");
    assert_eq!(dial.name.as_deref(), Some("Cutoff"));
    assert_eq!(
        dial.actions,
        [SemanticAction::SetValue, SemanticAction::Decrement]
    );
    // Disabling withdraws every action, including ones offered afterwards.
    let disabled = Semantics::new(SemanticRole::Button)
        .action(SemanticAction::Activate)
        .disabled(true)
        .enabled_actions([SemanticAction::Activate]);
    assert!(disabled.actions.is_empty());
    let disabled_first = Semantics::new(SemanticRole::Button)
        .disabled(true)
        .action(SemanticAction::Activate)
        .actions([SemanticAction::Increment]);
    assert!(disabled_first.actions.is_empty());
    // Empty text is absence, not an empty name.
    assert_eq!(Semantics::new(SemanticRole::Label).name("").name, None);
    assert_eq!(ui.next_deadline(), None, "describing schedules nothing");
}

fn names(semantics: &Semantics) -> Vec<&str> {
    semantics
        .items
        .iter()
        .map(|item| item.name.as_deref().unwrap_or_default())
        .collect()
}

#[test]
fn rows_drawn_by_one_component_are_described_as_items() {
    let mut ui = ui();
    let mode = ui.signal("b");
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "mode",
                RadioGroup::new(
                    mode,
                    [
                        Choice::new("a", "First"),
                        Choice::new("b", "Second"),
                        Choice::new("c", "Third").disabled(),
                    ],
                )
                .unwrap(),
            ),
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new(
                        "gear",
                        "Settings",
                        Element::new("gear-body", Label::new("0")),
                    )
                    .rich_title(RichText::new().icon(Icon::Settings)),
                    Tab::new("one", "One", Element::new("one-body", Label::new("1"))),
                    Tab::new("two", "Two", Element::new("two-body", Label::new("2"))),
                ]),
            ),
            Element::new(
                "owner",
                Select::new(
                    &ui,
                    ui.signal(2_u8),
                    [Choice::new(1_u8, "One"), Choice::new(2_u8, "Two")],
                )
                .unwrap(),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    let mode = semantics(&ui, "mode");
    assert_eq!(names(&mode), ["First", "Second", "Third"]);
    assert_eq!(
        mode.items
            .iter()
            .map(|item| item.selected)
            .collect::<Vec<_>>(),
        [false, true, false]
    );
    assert_eq!(mode.items[0].actions, [SemanticAction::Select]);
    // A disabled row keeps its name and offers nothing.
    assert!(mode.items[2].disabled && mode.items[2].actions.is_empty());

    let tabs = semantics(&ui, "tabs");
    // An icon-only title keeps its plain title as its name, also as the choice.
    assert_eq!(names(&tabs), ["Settings", "One", "Two"]);
    assert!(tabs.items[0].selected && !tabs.items[1].selected);
    assert_eq!(
        tabs.value,
        Some(SemanticValue::Choice {
            selected: Some(0),
            count: 3,
            label: Some("Settings".into()),
        })
    );
    assert_eq!(tabs.items[1].actions, [SemanticAction::Select]);

    let anchor = semantics(&ui, "select");
    assert_eq!(names(&anchor), ["One", "Two"]);
    assert!(anchor.items[1].selected);

    // Components whose parts are real children report no rows of their own.
    assert!(semantics(&ui, "one-body").items.is_empty());
}

#[test]
fn a_menu_describes_commands_disabled_commands_and_open_branches() {
    let mut ui = ui();
    let live = ui.signal(false);
    let menu = Menu::new(
        &ui,
        "File",
        vec![
            MenuItem::new("new", "New", |_| Ok(())),
            MenuItem::new("save", "Save", |_| Ok(())).enabled(live.read_only()),
            MenuItem::submenu(
                "recent",
                "Recent",
                vec![MenuItem::new("first", "first.txt", |_| Ok(()))],
            ),
        ],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("menu", menu)]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(semantics(&ui, "menu").role, SemanticRole::Menu);
    assert_eq!(semantics(&ui, "menu").expanded, Some(false));

    // Open it from its anchor button, which focuses the command list.
    let anchor = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| {
            component.details.semantics.as_ref().is_some_and(|s| {
                s.role == SemanticRole::Button && s.name.as_deref() == Some("File")
            })
        })
        .map(|component| component.key)
        .unwrap();
    ui.focus(&widget(&ui, &root, &anchor)).unwrap();
    press(&mut ui, KeyCode::Enter);
    assert_eq!(semantics(&ui, "menu").expanded, Some(true));

    let items = semantics(&ui, "items");
    assert_eq!(items.role, SemanticRole::Menu);
    assert_eq!(names(&items), ["New", "Save", "Recent"]);
    assert_eq!(items.items[0].actions, [SemanticAction::Activate]);
    assert!(items.items[1].disabled && items.items[1].actions.is_empty());
    assert_eq!(items.items[2].expanded, Some(false));
    assert_eq!(items.items[2].actions, [SemanticAction::Expand]);

    // A live enablement binding is reflected without reopening the menu.
    live.set(true).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let items = semantics(&ui, "items");
    assert!(!items.items[1].disabled);
    assert_eq!(items.items[1].actions, [SemanticAction::Activate]);

    // Moving to the branch and opening it is reported on that row.
    press(&mut ui, KeyCode::End);
    press(&mut ui, KeyCode::ArrowRight);
    let items = semantics(&ui, "items");
    assert_eq!(items.items[2].expanded, Some(true));
    assert_eq!(items.items[2].actions, [SemanticAction::Collapse]);
}

#[test]
fn a_hidden_banner_describes_itself_without_reporting_a_box_it_does_not_have() {
    let mut ui = ui();
    let visible = ui.signal(true);
    ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new(
            "banner",
            Banner::new("Saved", []).visible_signal(visible.read_only()),
        )]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(semantics(&ui, "banner").expanded, Some(true));
    visible.set(false).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let hidden = semantics(&ui, "banner");
    assert_eq!(
        (hidden.role, hidden.expanded),
        (SemanticRole::Alert, Some(false))
    );
    assert_eq!(hidden.name.as_deref(), Some("Saved"));
    // Its container's padding describes no box while it has no extent.
    assert_eq!(component(&ui, "banner").details.box_model.padding, None);
    assert!(ui.validation_report().issues.is_empty());
}

/// A custom component that fills the public semantic fields directly,
/// bypassing the builders.
struct Assigned {
    disabled: bool,
}
impl Component for Assigned {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 20.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut semantics = Semantics::new(SemanticRole::TextInput);
        semantics.protected = true;
        semantics.value = Some(SemanticValue::Text("secret".into()));
        semantics.description = Some(String::new());
        semantics.disabled = self.disabled;
        semantics.actions = vec![
            SemanticAction::SetValue,
            SemanticAction::Activate,
            SemanticAction::SetValue,
        ];
        let mut row = SemanticItem::new("Row");
        row.actions = vec![SemanticAction::Activate, SemanticAction::Activate];
        let mut gone = SemanticItem::new("Gone");
        gone.name = Some(String::new());
        gone.disabled = true;
        gone.actions = vec![SemanticAction::Activate];
        semantics.items = vec![row, gone];
        ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        }
    }
}

#[test]
fn a_snapshot_keeps_the_builders_invariants_for_directly_assigned_fields() {
    let mut ui = ui();
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new("enabled", Assigned { disabled: false }),
            Element::new("disabled", Assigned { disabled: true }),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();

    let enabled = semantics(&ui, "enabled");
    assert_eq!(enabled.value, None, "a protected value is never exposed");
    assert_eq!(enabled.description, None, "never an empty placeholder");
    assert_eq!(
        enabled.actions,
        [SemanticAction::Activate, SemanticAction::SetValue],
        "sorted and distinct"
    );
    assert_eq!(enabled.items[0].actions, [SemanticAction::Activate]);
    assert_eq!(enabled.items[1].name, None, "a row's name is never empty");
    assert!(
        enabled.items[1].actions.is_empty(),
        "a disabled row offers no actions"
    );

    let disabled = semantics(&ui, "disabled");
    assert!(
        disabled.actions.is_empty(),
        "a disabled component offers none"
    );
}

/// A 200 × 40 leaf reporting the items `items` builds, at an offset from its
/// window's origin when mounted below a title.
struct Rows(fn() -> Vec<SemanticItem>);
impl Component for Rows {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 40.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(Semantics::new(SemanticRole::Group).items((self.0)())),
            ..ComponentDetails::default()
        }
    }
}

fn mount_rows(ui: &mut Runtime, items: fn() -> Vec<SemanticItem>) -> WidgetId {
    let root = ui
        .mount_tree(Element::new(
            "page",
            Column::new([
                Element::new("title", Label::new("Rows")),
                Element::new("rows", Rows(items)),
            ]),
        ))
        .unwrap();
    ui.find(&root, &["rows"]).unwrap().unwrap()
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::new(x, y, width, height).unwrap()
}

#[test]
fn item_rectangles_are_reported_in_window_coordinates_cut_to_the_component() {
    let mut ui = ui();
    let rows = mount_rows(&mut ui, || {
        vec![
            SemanticItem::new("first").local_bounds(rect(0.0, 0.0, 100.0, 20.0)),
            // Partly outside the component: cut to it.
            SemanticItem::new("wide").local_bounds(rect(150.0, 20.0, 100.0, 40.0)),
            SemanticItem::new("none"),
        ]
    });
    // Before the first layout nothing is guessed.
    assert!(
        semantics(&ui, "rows")
            .items
            .iter()
            .all(|item| item.bounds.is_none())
    );
    ui.flush(Duration::ZERO).unwrap();
    let [x, y, ..] = ui.bounds(&rows).unwrap().unwrap().values();
    assert!(y > 0.0, "the title is above");
    let items = semantics(&ui, "rows").items;
    assert_eq!(items[0].bounds, Some([x, y, 100.0, 20.0]));
    assert_eq!(items[1].bounds, Some([x + 150.0, y + 20.0, 50.0, 20.0]));
    assert_eq!(items[2].bounds, None);
}

#[test]
fn a_directly_assigned_item_bounds_is_replaced_by_the_runtime() {
    let mut ui = ui();
    mount_rows(&mut ui, || {
        let mut item = SemanticItem::new("assigned");
        item.bounds = Some([1.0, 2.0, 3.0, 4.0]);
        vec![item]
    });
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(semantics(&ui, "rows").items[0].bounds, None);
}
