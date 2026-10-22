//! Toolbar layout, overflow and wrapping through the public facade.
use crate::support;
use pert_ui::*;
use support::{find, frame, key};

fn visible(ui: &Runtime, key: &str) -> Vec<bool> {
    ui.inspect()
        .components
        .iter()
        .filter(|c| c.key == key)
        .map(|c| c.visible)
        .collect()
}
fn property(ui: &Runtime, key: &str, name: &str) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .unwrap()
        .details
        .properties[name]
        .clone()
}
fn collapsed_keys(ui: &Runtime) -> Vec<String> {
    match property(ui, "toolbar", "collapsed_keys") {
        InspectValue::List(items) => items
            .into_iter()
            .map(|item| match item {
                InspectValue::Text(text) => text,
                other => panic!("{other:?}"),
            })
            .collect(),
        other => panic!("{other:?}"),
    }
}
fn button(key: &str, icon: Icon) -> Element {
    Element::new(key, Button::icon(icon, key, |_| Ok(())).frameless())
}
fn toolbar(ui: &Runtime, count: Signal<i64>, policy: ToolbarOverflow) -> Toolbar {
    Toolbar::new(
        ui,
        [
            ToolGroup::new(
                "transport",
                [
                    button("play", Icon::PlayArrow),
                    button("stop", Icon::Stop),
                    button("record", Icon::Record),
                ],
            )
            .essential(),
            ToolGroup::separator("divider"),
            ToolGroup::new("tempo", [button("timer", Icon::Timer)]),
            ToolGroup::new(
                "edit",
                [button("undo", Icon::Undo), button("redo", Icon::Redo)],
            ),
            ToolGroup::spacer("spacer"),
            ToolGroup::new(
                "session",
                [
                    button("save", Icon::Save),
                    Element::new(
                        "bpm",
                        InlineInteger::new(count)
                            .range(20, 300)
                            .unwrap()
                            .label("Tempo")
                            .unit("bpm"),
                    ),
                ],
            )
            .collapse_first(),
        ],
    )
    .unwrap()
    .overflow(policy)
}
fn mount(ui: &mut Runtime, width: f32, policy: ToolbarOverflow) -> (WidgetId, Signal<i64>) {
    let count = ui.signal(120_i64);
    ui.set_viewport(Viewport::new(width, 400.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("toolbar", toolbar(ui, count.clone(), policy)),
                Element::new("body", Label::new("content")),
            ]),
        ))
        .unwrap();
    frame(ui);
    (root, count)
}

#[test]
fn wide_toolbar_shows_everything_and_aligns_after_the_spacer() {
    let mut ui = Runtime::new();
    let (root, _) = mount(&mut ui, 900.0, ToolbarOverflow::Collapse);
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    assert_eq!(visible(&ui, "more"), vec![false]);
    assert_eq!(visible(&ui, "overflow"), vec![false]);
    let bar = ui
        .bounds(&find(&ui, &root, "toolbar").unwrap())
        .unwrap()
        .unwrap();
    let session = find(&ui, &root, "session").unwrap();
    let [x, _, w, _] = ui.bounds(&session).unwrap().unwrap().values();
    let [bx, _, bw, bh] = bar.values();
    assert!(
        x + w > bx + bw - 24.0,
        "trailing group hugs the trailing edge"
    );
    let metrics = ui.theme().metrics(Density::Standard).unwrap();
    let band = metrics.metric(MetricRole::ControlHeight);
    let pad = metrics.metric(MetricRole::ControlPaddingY);
    assert_eq!(bh, band + 2.0 * pad);
    assert_eq!(
        property(&ui, "toolbar", "oversized"),
        InspectValue::Number(0.0)
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    assert!(ui.next_deadline().is_none());
}

#[test]
fn narrow_toolbar_collapses_first_marked_then_trailing_groups_never_essential() {
    let mut ui = Runtime::new();
    let (root, count) = mount(&mut ui, 360.0, ToolbarOverflow::Collapse);
    // collapse_first goes first, then trailing non-essential groups, never the
    // essential transport group; the report lists them in strip order.
    let collapsed = collapsed_keys(&ui);
    assert!(!collapsed.is_empty());
    assert!(collapsed.contains(&"session".to_string()));
    assert!(!collapsed.contains(&"transport".to_string()));
    let order = ["session", "edit", "tempo"];
    assert!(
        order[..collapsed.len()]
            .iter()
            .all(|k| collapsed.contains(&k.to_string()))
    );
    assert_eq!(
        collapsed,
        ["tempo", "edit", "session"]
            .iter()
            .filter(|k| collapsed.contains(&k.to_string()))
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(visible(&ui, "more"), vec![true]);
    assert_eq!(
        visible(&ui, "transport"),
        vec![true],
        "a group is mounted once and moves between the strip and the surface"
    );
    assert_eq!(
        visible(&ui, "session"),
        vec![false],
        "the collapsed group is hidden while its surface is closed"
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    // Open the overflow surface through the More button and edit the moved control.
    let more = find(&ui, &root, "more").unwrap();
    let button = find(&ui, &more, "button").unwrap();
    ui.focus(&button).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    assert_eq!(visible(&ui, "session"), vec![true]);
    let overflow = find(&ui, &root, "overflow").unwrap();
    assert!(
        find(&ui, &overflow, "session").is_some(),
        "the collapsed group is owned by the overflow surface"
    );
    let bpm = find(&ui, &overflow, "bpm").unwrap();
    ui.focus(&bpm).unwrap();
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(count.get().unwrap(), 121);
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(false)
    );
    // Focus returns to the More button, whose tooltip owns the only deadline.
    ui.clear_focus();
    frame(&mut ui);
    assert!(ui.next_deadline().is_none());
}

#[test]
fn growing_the_window_restores_groups_and_closes_an_open_surface() {
    let mut ui = Runtime::new();
    let (root, _) = mount(&mut ui, 360.0, ToolbarOverflow::Collapse);
    let session = find(&ui, &root, "session").unwrap();
    let more = find(&ui, &root, "more").unwrap();
    let button = find(&ui, &more, "button").unwrap();
    ui.focus(&button).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    ui.set_viewport(Viewport::new(900.0, 400.0, 1.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(false)
    );
    assert_eq!(visible(&ui, "session"), vec![true]);
    assert_eq!(visible(&ui, "more"), vec![false]);
    assert_eq!(
        find(&ui, &root, "session"),
        Some(session.clone()),
        "the restored group is the same retained subtree"
    );
    let toolbar = find(&ui, &root, "toolbar").unwrap();
    let keys: Vec<_> = ui
        .children(&toolbar)
        .unwrap()
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect();
    assert_eq!(
        keys,
        [
            "transport",
            "divider",
            "tempo",
            "edit",
            "spacer",
            "session",
            "more",
            "overflow"
        ],
        "a returning group takes its original position in the strip"
    );
    ui.set_viewport(Viewport::new(360.0, 400.0, 1.0).unwrap())
        .unwrap();
    frame(&mut ui);
    assert!(!collapsed_keys(&ui).is_empty());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    assert!(ui.next_deadline().is_none());
}

#[test]
fn wrap_policy_adds_lines_instead_of_hiding() {
    let mut ui = Runtime::new();
    let (root, _) = mount(&mut ui, 360.0, ToolbarOverflow::Wrap);
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    assert_eq!(visible(&ui, "more"), vec![false]);
    let lines = property(&ui, "toolbar", "lines");
    assert!(
        matches!(lines, InspectValue::Number(n) if n >= 2.0),
        "{lines:?}"
    );
    let bar = ui
        .bounds(&find(&ui, &root, "toolbar").unwrap())
        .unwrap()
        .unwrap();
    let body = ui
        .bounds(&find(&ui, &root, "body").unwrap())
        .unwrap()
        .unwrap();
    assert!(body.y() >= bar.y() + bar.height());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn invalid_groups_are_rejected_at_construction() {
    let ui = Runtime::new();
    assert!(Toolbar::new(&ui, [ToolGroup::new("empty", Vec::<Element>::new())]).is_err());
    assert!(Toolbar::new(&ui, [ToolGroup::new("more", [button("a", Icon::Add)])]).is_err());
    assert!(Toolbar::new(&ui, [ToolGroup::separator(" ")]).is_err());
}

#[test]
fn contextual_groups_take_no_space_and_never_overflow() {
    let mut ui = Runtime::new();
    let options = ui.signal(false);
    ui.set_viewport(Viewport::new(220.0, 400.0, 1.0).unwrap())
        .unwrap();
    let toolbar = Toolbar::new(
        &ui,
        [
            ToolGroup::new("transport", [button("play", Icon::PlayArrow)]).essential(),
            ToolGroup::new(
                "erase-options",
                [
                    button("soft", Icon::Tune),
                    button("hard", Icon::Edit),
                    button("all", Icon::Delete),
                ],
            )
            .visible_signal(options.read_only()),
            ToolGroup::separator("divider"),
            ToolGroup::new(
                "edit",
                [button("undo", Icon::Undo), button("redo", Icon::Redo)],
            ),
        ],
    )
    .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("toolbar", toolbar)]),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "hidden"),
        InspectValue::Number(1.0)
    );
    assert_eq!(visible(&ui, "erase-options"), vec![false]);
    assert_eq!(visible(&ui, "divider"), vec![false, false]);
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    let edit = find(&ui, &root, "edit").unwrap();
    let before = ui.bounds(&edit).unwrap().unwrap();
    // Showing the group makes the strip too narrow: it collapses, never the hidden one.
    options.set(true).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "hidden"),
        InspectValue::Number(0.0)
    );
    let collapsed = collapsed_keys(&ui);
    assert!(collapsed.contains(&"edit".to_string()), "{collapsed:?}");
    // The separator follows the shown group: in the strip while that group
    // stays, hidden once the group itself collapses.
    let options_collapsed = collapsed.contains(&"erase-options".to_string());
    assert_eq!(visible(&ui, "divider")[0], !options_collapsed);
    options.set(false).unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    assert_eq!(ui.bounds(&edit).unwrap().unwrap(), before);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    assert!(ui.next_deadline().is_none());
}

/// A toolbar with the named items in the given order, drawn from the groups of
/// [`toolbar`] plus a `tool` group.
fn described(ui: &Runtime, count: &Signal<i64>, order: &[&str]) -> Toolbar {
    let groups = order.iter().map(|name| match *name {
        "transport" => ToolGroup::new(
            "transport",
            [
                button("play", Icon::PlayArrow),
                button("stop", Icon::Stop),
                button("record", Icon::Record),
            ],
        )
        .essential(),
        "divider" => ToolGroup::separator("divider"),
        "tempo" => ToolGroup::new("tempo", [button("timer", Icon::Timer)]),
        "tool" => ToolGroup::new("tool", [button("brush", Icon::Edit)]),
        "edit" => ToolGroup::new(
            "edit",
            [button("undo", Icon::Undo), button("redo", Icon::Redo)],
        ),
        "spacer" => ToolGroup::spacer("spacer"),
        "session" => ToolGroup::new(
            "session",
            [
                button("save", Icon::Save),
                Element::new(
                    "bpm",
                    InlineInteger::new(count.clone())
                        .range(20, 300)
                        .unwrap()
                        .label("Tempo")
                        .unit("bpm"),
                ),
            ],
        )
        .collapse_first(),
        other => panic!("unknown item {other}"),
    });
    Toolbar::new(ui, groups).unwrap()
}
fn redescribe(ui: &mut Runtime, root: &WidgetId, count: &Signal<i64>, order: &[&str]) {
    let toolbar = described(ui, count, order);
    ui.reconcile_children(
        root,
        vec![
            Element::new("toolbar", toolbar),
            Element::new("body", Label::new("content")),
        ],
    )
    .unwrap();
    frame(ui);
}
fn strip_keys(ui: &Runtime, root: &WidgetId) -> Vec<String> {
    let toolbar = find(ui, root, "toolbar").unwrap();
    ui.children(&toolbar)
        .unwrap()
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect()
}
fn focused_key(ui: &Runtime) -> Option<String> {
    ui.focused().map(|id| ui.key(&id).unwrap().to_owned())
}
fn left(ui: &Runtime, root: &WidgetId, key: &str) -> f32 {
    ui.bounds(&find(ui, root, key).unwrap())
        .unwrap()
        .unwrap()
        .x()
}

#[test]
fn a_redescribed_toolbar_takes_added_removed_and_reordered_groups() {
    let mut ui = Runtime::new();
    let (root, count) = mount(&mut ui, 900.0, ToolbarOverflow::Collapse);
    let undo = find(&ui, &root, "undo").unwrap();
    ui.focus(&undo).unwrap();
    frame(&mut ui);

    // A tool-dependent group appears and the tempo group goes away.
    let order = ["transport", "divider", "edit", "tool", "spacer", "session"];
    redescribe(&mut ui, &root, &count, &order);
    assert_eq!(
        strip_keys(&ui, &root),
        [&order[..], &["more", "overflow"]].concat()
    );
    assert!(find(&ui, &root, "tempo").is_none());
    assert_eq!(visible(&ui, "brush"), vec![true]);
    assert_eq!(
        find(&ui, &root, "undo"),
        Some(undo.clone()),
        "a surviving group keeps its controls"
    );
    assert_eq!(ui.focused(), Some(undo.clone()), "and their focus");
    assert!(left(&ui, &root, "edit") < left(&ui, &root, "tool"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("redo"));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("brush"));

    // Reordering moves the retained groups; keyboard order follows.
    let order = ["transport", "divider", "tool", "edit", "spacer", "session"];
    redescribe(&mut ui, &root, &count, &order);
    assert_eq!(
        strip_keys(&ui, &root),
        [&order[..], &["more", "overflow"]].concat()
    );
    assert!(left(&ui, &root, "tool") < left(&ui, &root, "edit"));
    assert_eq!(find(&ui, &root, "undo"), Some(undo));
    ui.focus_next(FocusDirection::Forward).unwrap();
    assert_eq!(focused_key(&ui).as_deref(), Some("undo"));
    assert_eq!(
        property(&ui, "toolbar", "collapsed"),
        InspectValue::Number(0.0)
    );
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );

    // The runtime stays usable once the toolbar is gone; a second full-window
    // root beside it would overlap it.
    ui.unmount(&root).unwrap();
    ui.mount_tree(Element::new("other", Label::new("Other")))
        .unwrap();
    frame(&mut ui);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn a_redescribed_narrow_toolbar_keeps_its_open_overflow_surface() {
    let mut ui = Runtime::new();
    let (root, count) = mount(&mut ui, 360.0, ToolbarOverflow::Collapse);
    let more = find(&ui, &root, "more").unwrap();
    let button = find(&ui, &more, "button").unwrap();
    ui.focus(&button).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    assert!(collapsed_keys(&ui).contains(&"session".to_owned()));

    // A new group joins; the surface stays open and the collapsed groups
    // return to it.
    let order = [
        "transport",
        "tool",
        "divider",
        "tempo",
        "edit",
        "spacer",
        "session",
    ];
    redescribe(&mut ui, &root, &count, &order);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    let collapsed = collapsed_keys(&ui);
    assert!(collapsed.contains(&"session".to_owned()), "{collapsed:?}");
    let overflow = find(&ui, &root, "overflow").unwrap();
    let bpm = find(&ui, &overflow, "bpm").unwrap();
    assert_eq!(visible(&ui, "bpm"), vec![true]);
    ui.focus(&bpm).unwrap();
    key(&mut ui, KeyCode::ArrowUp);
    frame(&mut ui);
    assert_eq!(count.get().unwrap(), 121);

    // Removing a collapsed group removes it from the surface.
    let order = ["transport", "tool", "divider", "tempo", "edit", "spacer"];
    redescribe(&mut ui, &root, &count, &order);
    assert!(find(&ui, &root, "session").is_none());
    assert!(!collapsed_keys(&ui).contains(&"session".to_owned()));
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
    // The runtime stays usable once the toolbar is gone; a second full-window
    // root beside it would overlap it.
    ui.unmount(&root).unwrap();
    ui.mount_tree(Element::new("other", Label::new("Other")))
        .unwrap();
    frame(&mut ui);
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn a_localized_placeholder_and_overflow_label_follow_the_description() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 200.0, 1.0).unwrap())
        .unwrap();
    let snap = ui.signal("unknown");
    let describe = |ui: &Runtime, placeholder: &str, more: &str| {
        let select = InlineSelect::new(ui, snap.clone(), [Choice::new("1/4", "1/4")])
            .unwrap()
            .label("Snap")
            .placeholder(placeholder);
        Toolbar::new(ui, [ToolGroup::new("grid", [Element::new("snap", select)])])
            .unwrap()
            .more_label(more)
    };
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("toolbar", describe(&ui, "Välj…", "Mer"))]),
        ))
        .unwrap();
    frame(&mut ui);
    let names = |ui: &Runtime| {
        let components = ui.inspect().components;
        let name = |key: &str| {
            components
                .iter()
                .find(|c| c.key == key)
                .and_then(|c| c.details.semantics.as_ref())
                .and_then(|s| s.name.clone())
        };
        (
            property(ui, "select", "text"),
            name("button"),
            property(ui, "help", "text"),
        )
    };
    assert_eq!(
        names(&ui),
        (
            InspectValue::Text("Välj…".into()),
            Some("Mer".into()),
            InspectValue::Text("Mer".into())
        )
    );
    ui.reconcile_children(
        &root,
        vec![Element::new("toolbar", describe(&ui, "Ange…", "Fler"))],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(
        names(&ui),
        (
            InspectValue::Text("Ange…".into()),
            Some("Fler".into()),
            InspectValue::Text("Fler".into())
        )
    );
    assert_eq!(
        snap.get().unwrap(),
        "unknown",
        "the value is never rewritten"
    );
}

/// A toolbar whose `name` group holds an inline text field, with the named
/// items in the given order.
fn with_field(ui: &Runtime, name: &Signal<String>, order: &[&str]) -> Toolbar {
    let groups = order.iter().map(|item| match *item {
        "transport" => ToolGroup::new(
            "transport",
            [button("play", Icon::PlayArrow), button("stop", Icon::Stop)],
        )
        .essential(),
        "divider" => ToolGroup::separator("divider"),
        "edit" => ToolGroup::new(
            "edit",
            [button("undo", Icon::Undo), button("redo", Icon::Redo)],
        )
        .collapse_first(),
        "name" => ToolGroup::new(
            "name",
            [Element::new("field", InlineText::new(name.clone()))],
        )
        .collapse_first(),
        "tool" => ToolGroup::new("tool", [button("brush", Icon::Edit)]),
        other => panic!("unknown item {other}"),
    });
    Toolbar::new(ui, groups).unwrap()
}
fn overflow_keys(ui: &Runtime, root: &WidgetId) -> Vec<String> {
    let overflow = find(ui, root, "overflow").unwrap();
    ui.children(&overflow)
        .unwrap()
        .iter()
        .map(|id| ui.key(id).unwrap().to_owned())
        .collect()
}

#[test]
fn a_collapsed_group_keeps_its_draft_when_the_toolbar_is_described_again() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(160.0, 400.0, 1.0).unwrap())
        .unwrap();
    let name = ui.signal(String::from("Lead"));
    let body = || Element::new("body", Label::new("content"));
    let order = ["transport", "divider", "name", "edit"];
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("toolbar", with_field(&ui, &name, &order)),
                body(),
            ]),
        ))
        .unwrap();
    frame(&mut ui);
    assert_eq!(collapsed_keys(&ui), ["name", "edit"]);
    let more = find(&ui, &root, "more").unwrap();
    ui.focus(&find(&ui, &more, "button").unwrap()).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    let field = find(&ui, &root, "field").unwrap();
    ui.focus(&field).unwrap();
    key(&mut ui, KeyCode::F2);
    ui.text_input("Bass").unwrap();
    frame(&mut ui);
    assert_eq!(
        property(&ui, "field", "draft"),
        InspectValue::Text("Bass".into())
    );

    // A new group joins and the two collapsed groups swap places.
    let order = ["transport", "tool", "divider", "edit", "name"];
    ui.reconcile_children(
        &root,
        vec![
            Element::new("toolbar", with_field(&ui, &name, &order)),
            body(),
        ],
    )
    .unwrap();
    frame(&mut ui);
    assert_eq!(
        find(&ui, &root, "field"),
        Some(field.clone()),
        "the collapsed group keeps its controls"
    );
    assert_eq!(
        ui.focused(),
        find(&ui, &field, "editor"),
        "and their focus, on the label's editor while it edits"
    );
    assert_eq!(
        property(&ui, "field", "draft"),
        InspectValue::Text("Bass".into()),
        "the uncommitted draft survives"
    );
    assert_eq!(property(&ui, "field", "editing"), InspectValue::Bool(true));
    assert_eq!(name.get().unwrap(), "Lead");
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    let collapsed = collapsed_keys(&ui);
    let surface: Vec<String> = overflow_keys(&ui, &root)
        .into_iter()
        .filter(|key| collapsed.contains(key))
        .collect();
    let expected: Vec<&str> = order
        .iter()
        .copied()
        .filter(|item| collapsed.iter().any(|key| key == item))
        .collect();
    assert!(expected.ends_with(&["edit", "name"]), "{expected:?}");
    assert_eq!(surface, expected, "the surface follows item order");
    assert!(!strip_keys(&ui, &root).contains(&"name".to_owned()));
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(name.get().unwrap(), "Bass");

    // Omitting the collapsed group unmounts it from the surface.
    let order = ["transport", "tool", "divider", "edit"];
    ui.reconcile_children(
        &root,
        vec![
            Element::new("toolbar", with_field(&ui, &name, &order)),
            body(),
        ],
    )
    .unwrap();
    frame(&mut ui);
    assert!(find(&ui, &root, "name").is_none());
    assert!(find(&ui, &root, "field").is_none());
    assert!(
        ui.validation_report().issues.is_empty(),
        "{:?}",
        ui.validation_report().issues
    );
}

#[test]
fn toolbar_overflow_has_the_shared_panel_border_after_its_content() {
    let mut ui = Runtime::new();
    let (root, _) = mount(&mut ui, 360.0, ToolbarOverflow::Collapse);
    let more = find(&ui, &root, "more").unwrap();
    let button = find(&ui, &more, "button").unwrap();
    ui.focus(&button).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    let overflow = find(&ui, &root, "overflow").unwrap();
    let bounds = ui.bounds(&overflow).unwrap().unwrap();
    let border = Palette::dark().color(ColorRole::PanelBorder);
    let scene = ui.paint().unwrap();
    let border_index = scene.commands().iter().position(|command| {
        matches!(command, DrawCommand::FillRect { rect, color } if *color == border && rect.height() == 1.0 && rect.y() == bounds.y() && rect.width() > bounds.width() / 2.0)
    }).expect("overflow top border");
    let content = scene
        .commands()
        .iter()
        .rposition(|command| {
            matches!(command, DrawCommand::DrawMask { rect, color, .. }
            if *color != border && *color != Palette::dark().color(ColorRole::Surface)
                && rect.y() > bounds.y() && rect.y() < bounds.y() + bounds.height()
                && rect.x() >= bounds.x() && rect.x() < bounds.x() + bounds.width())
        })
        .expect("painted overflow content");
    assert!(
        border_index > content,
        "panel border must paint after popup content"
    );
    key(&mut ui, KeyCode::Escape);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(false)
    );
}

#[test]
fn toolbar_overflow_reports_its_painted_frame_and_resets_inherited_cursor() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(360.0, 400.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Container::new(Element::new(
                "layout",
                Column::new([Element::new(
                    "toolbar",
                    toolbar(&ui, ui.signal(120), ToolbarOverflow::Collapse),
                )]),
            ))
            .cursor(CursorIcon::Text),
        ))
        .unwrap();
    frame(&mut ui);
    let more = find(&ui, &root, "more").unwrap();
    let button = find(&ui, &more, "button").unwrap();
    ui.focus(&button).unwrap();
    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(
        property(&ui, "toolbar", "overflow_open"),
        InspectValue::Bool(true)
    );
    assert_eq!(visible(&ui, "overflow"), vec![true]);
    let overflow = find(&ui, &root, "overflow").unwrap();
    let bounds = ui.bounds(&overflow).unwrap().unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(bounds.x() + 1.0, bounds.y() + 1.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    let details = ui
        .inspect()
        .components
        .into_iter()
        .find(|c| c.key == "overflow")
        .unwrap()
        .details;
    assert_eq!(
        details.box_model.background,
        Some(Palette::dark().color(ColorRole::Surface).channels())
    );
    let border = details.box_model.border.unwrap();
    assert_eq!(
        border.width,
        ui.theme()
            .metrics(Density::Standard)
            .unwrap()
            .metric(MetricRole::BorderWidth)
    );
    assert_eq!(
        border.color,
        Palette::dark().color(ColorRole::PanelBorder).channels()
    );
}

fn semantics(ui: &Runtime, key: &str) -> Semantics {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == key)
        .and_then(|c| c.details.semantics.clone())
        .unwrap()
}
/// The help text once the focused anchor's help has had time to show.
fn help_text(ui: &mut Runtime) -> InspectValue {
    frame(ui);
    let later = ui.now() + std::time::Duration::from_millis(600);
    ui.flush(later).unwrap();
    assert_eq!(visible(ui, "help"), vec![true]);
    property(ui, "help", "text")
}

#[test]
fn command_icon_buttons_follow_the_command_and_show_its_shortcut_as_help() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let runs = ui.signal(0_i64);
    let counter = runs.clone();
    let undo = ui
        .register_command(
            "Undo",
            CommandScope::Application,
            vec![KeyChord::primary('z')],
            move |_, _| counter.update(|n| *n += 1),
        )
        .unwrap();
    let can_undo = ui.signal(true);
    ui.command_enabled_signal(&undo, can_undo.read_only())
        .unwrap();
    let button = Button::command_icon(&ui, &undo, Icon::Undo, |_, _| Ok(())).unwrap();
    let help = Tooltip::command(&ui, Element::new("undo", button), &undo).unwrap();
    // In a column, so the anchor leaves room below it for the help.
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("undo-help", help)]),
        ))
        .unwrap();
    frame(&mut ui);
    let button = find(&ui, &root, "undo").unwrap();
    ui.focus(&button).unwrap();

    // The command's name is the accessible name; the icon is the content.
    assert_eq!(semantics(&ui, "undo").name.as_deref(), Some("Undo"));
    let shortcut = KeyChord::primary('z').to_string();
    assert_eq!(
        help_text(&mut ui),
        InspectValue::Text(format!("Undo ({shortcut})"))
    );

    // Rebinding updates the shown help; no shortcut leaves the name alone.
    ui.rebind_command(&undo, vec![KeyChord::named(KeyCode::F9)])
        .unwrap();
    frame(&mut ui);
    assert_eq!(help_text(&mut ui), InspectValue::Text("Undo (F9)".into()));
    ui.rebind_command(&undo, Vec::new()).unwrap();
    frame(&mut ui);
    assert_eq!(help_text(&mut ui), InspectValue::Text("Undo".into()));
    ui.rebind_command(&undo, vec![KeyChord::primary('z')])
        .unwrap();

    key(&mut ui, KeyCode::Enter);
    frame(&mut ui);
    assert_eq!(runs.get().unwrap(), 1);

    // Enablement follows the command's binding.
    can_undo.set(false).unwrap();
    frame(&mut ui);
    assert!(semantics(&ui, "undo").disabled);
}
