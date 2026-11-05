//! Content search contracts through the public facade: what is found, what
//! never is, the work budget, staleness and jumping to a hit.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;

struct Squares(usize);
impl TableItems for Squares {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("n{index}"))
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let value = if column == "n" { index } else { index * index };
        Some(Element::new("value", Label::new(value.to_string())))
    }
    fn text(&self, index: usize, column: &str) -> Option<String> {
        (index < self.0).then(|| {
            if column == "n" {
                format!("row {index}")
            } else {
                format!("square {}", index * index)
            }
        })
    }
}

fn settle(ui: &mut Runtime) {
    ui.flush(Duration::ZERO).unwrap();
}

fn search(ui: &Runtime, text: &str) -> Vec<ContentHit> {
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

/// Whether a component whose text matches is presented now.
fn shown(ui: &Runtime, text: &str) -> bool {
    let query = ContentQuery::new(text).unwrap().hidden(false);
    !ui.content_search(query).step(ui).unwrap().hits.is_empty()
}

fn widget(hit: &ContentHit) -> WidgetId {
    match &hit.target {
        ContentTarget::Widget(id) => id.clone(),
        other => panic!("expected a widget, got {other:?}"),
    }
}

#[test]
fn a_button_in_an_unselected_tab_is_found_and_the_jump_selects_the_tab() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let tabs = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new("one", "One", Element::new("intro", Label::new("Welcome"))),
                Tab::new(
                    "two",
                    "Two",
                    Element::new("output", Button::new("Output level", |_| Ok(()))),
                ),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let hits = search(&ui, "OUTPUT");
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.text, "Output level");
    assert_eq!(hit.field, ContentField::Name);
    assert_eq!(hit.role, Some(SemanticRole::Button));
    assert!(!hit.visible);
    assert_eq!(
        hit.path.as_deref(),
        Some(&["tabs".to_owned(), "two".to_owned(), "output".to_owned()][..])
    );
    let target = widget(hit);
    let jump = ui.jump_to_content(&hit.target).unwrap();
    assert!(
        matches!(
            jump,
            ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. })
        ),
        "{jump:?}"
    );
    assert_eq!(ui.focused().as_ref(), Some(&target));
    assert!(shown(&ui, "output"));
    let _ = tabs;
}

#[test]
fn a_value_in_a_collapsed_section_is_found_and_the_jump_expands_it() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let text = ui.signal("Reverb tail".to_owned());
    let open = ui.signal(true);
    ui.mount_tree(Element::new(
        "section",
        Collapsible::new(
            "Effects",
            Element::new(
                "field",
                Field::new(Element::new("input", TextField::new(text))).label("Name"),
            ),
        )
        .bind_expanded(open.clone()),
    ))
    .unwrap();
    settle(&mut ui);
    // A component reports the values it has read: collapsed after it has
    // shown its value once, the field keeps reporting it.
    open.set(false).unwrap();
    settle(&mut ui);
    let hits = search(&ui, "reverb");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].field, ContentField::Value);
    assert!(!hits[0].visible);
    // The field names its control, so the control is the hit for its label.
    let named = search(&ui, "name");
    assert_eq!(named.len(), 1, "{named:?}");
    assert_eq!(widget(&named[0]), widget(&hits[0]));
    let jump = ui.jump_to_content(&hits[0].target).unwrap();
    assert!(matches!(
        jump,
        ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. })
    ));
    assert!(shown(&ui, "reverb"));
}

fn table(ui: &mut Runtime, rows: usize) -> (TableController, WidgetId) {
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let state = TableController::new(
        &*ui,
        ListSelection::new(&*ui, SelectionMode::Single),
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(rows)),
    )
    .unwrap();
    let table = ui
        .mount_tree(Element::new("squares", Table::new(&state, 24.0).unwrap()))
        .unwrap();
    settle(ui);
    (state, table)
}

#[test]
fn a_table_value_outside_the_mounted_rows_is_found_and_mounted_by_the_jump() {
    let mut ui = Runtime::new();
    let (_, _table) = table(&mut ui, 10_000);
    let hits = search(&ui, "row 4321");
    assert_eq!(hits.len(), 1, "{hits:?}");
    let ContentTarget::Row { key, .. } = &hits[0].target else {
        panic!("expected a row, got {:?}", hits[0].target);
    };
    assert_eq!(key, "n4321");
    assert_eq!(hits[0].field, ContentField::Row);
    assert_eq!(hits[0].role, None);
    // The table's body list is internal structure: the path is the table's.
    assert_eq!(hits[0].path.as_deref(), Some(&["squares".to_owned()][..]));
    let jump = ui.jump_to_content(&hits[0].target).unwrap();
    let ContentJump::Reveal(outcome) = jump else {
        panic!("the row was not mounted: {jump:?}");
    };
    assert!(
        matches!(
            outcome,
            RevealOutcome::Revealed { .. } | RevealOutcome::RevealedAndFocused { .. }
        ),
        "{outcome:?}"
    );
    // Mounted rows are found as components, not again as source rows.
    let again = search(&ui, "18671041");
    assert_eq!(again.len(), 1, "{again:?}");
    assert!(matches!(again[0].target, ContentTarget::Widget(_)));
}

#[test]
fn concealed_values_and_excluded_subtrees_are_never_found() {
    let mut ui = Runtime::new();
    let secret = ui.signal("hunter2".to_owned());
    let plain = ui.signal("hunter2".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("secret", TextField::new(secret).concealed(true)).with_label("Password"),
            Element::new(
                "hidden-panel",
                Column::new([Element::new("copy", TextField::new(plain))]),
            )
            .with_searchable(false),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    assert!(search(&ui, "hunter").is_empty());
    // The concealed field's name is not secret and is found.
    let hits = search(&ui, "password");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].field, ContentField::Name);
}

#[test]
fn a_search_with_no_matches_is_bounded_per_step_and_resumable() {
    let mut ui = Runtime::new();
    let labels: Vec<_> = (0..100)
        .map(|index| Element::new(format!("l{index}"), Label::new(format!("Item {index}"))))
        .collect();
    ui.mount_tree(Element::new("root", Column::new(labels)))
        .unwrap();
    settle(&mut ui);
    let mut search = ui.content_search(ContentQuery::new("absent").unwrap().budget(10).unwrap());
    let mut steps = 0;
    let mut work = 0;
    loop {
        let step = search.step(&ui).unwrap();
        assert!(step.hits.is_empty());
        assert!(step.work <= 10);
        work += step.work;
        steps += 1;
        if step.done {
            break;
        }
    }
    assert!(search.is_done());
    // Every component is examined once while the window roots are found,
    // then visited once.
    assert_eq!(work, 202);
    assert_eq!(steps, 21);
    // A finished search answers again with nothing and no work.
    let after = search.step(&ui).unwrap();
    assert!(after.done && after.hits.is_empty() && after.work == 0);
}

#[test]
fn the_result_limit_stops_a_step_and_the_next_one_continues() {
    let mut ui = Runtime::new();
    let labels: Vec<_> = (0..7)
        .map(|index| Element::new(format!("l{index}"), Label::new(format!("Item {index}"))))
        .collect();
    ui.mount_tree(Element::new("root", Column::new(labels)))
        .unwrap();
    settle(&mut ui);
    let mut search = ui.content_search(ContentQuery::new("item").unwrap().limit(3).unwrap());
    let first = search.step(&ui).unwrap();
    assert_eq!(first.hits.len(), 3);
    assert!(!first.done);
    assert_eq!(first.hits[0].text, "Item 0");
    let mut rest = Vec::new();
    loop {
        let step = search.step(&ui).unwrap();
        rest.extend(step.hits);
        if step.done {
            break;
        }
    }
    assert_eq!(rest.len(), 4);
    assert_eq!(rest[0].text, "Item 3");
}

#[test]
fn a_search_is_stale_after_the_tree_or_a_collection_changes() {
    let mut ui = Runtime::new();
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([Element::new("a", Label::new("Alpha"))]),
        ))
        .unwrap();
    settle(&mut ui);
    let mut search = ui.content_search(ContentQuery::new("absent").unwrap().budget(1).unwrap());
    search.step(&ui).unwrap();
    ui.reconcile_children(
        &root,
        vec![
            Element::new("a", Label::new("Alpha")),
            Element::new("b", Label::new("Beta")),
        ],
    )
    .unwrap();
    settle(&mut ui);
    assert_eq!(search.step(&ui), Err(ContentSearchError::Stale));

    // Replacing a collection's rows while its rows are being searched.
    let mut ui = Runtime::new();
    let (state, _) = table(&mut ui, 1_000);
    let mut search = ui.content_search(ContentQuery::new("absent").unwrap().budget(60).unwrap());
    let first = search.step(&ui).unwrap();
    assert!(!first.done);
    state.set_source(TableSource::new(Squares(500))).unwrap();
    settle(&mut ui);
    assert_eq!(search.step(&ui), Err(ContentSearchError::Stale));
}

#[test]
fn hidden_content_can_be_left_out_and_a_foreign_search_is_refused() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new("one", "One", Element::new("a", Label::new("Visible text"))),
            Tab::new("two", "Two", Element::new("b", Label::new("Hidden text"))),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let query = ContentQuery::new("text").unwrap().hidden(false);
    let hits = ui.content_search(query.clone()).step(&ui).unwrap().hits;
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].text, "Visible text");
    let other = Runtime::new();
    assert_eq!(
        other.content_search(query).step(&ui),
        Err(ContentSearchError::ForeignRuntime)
    );
}

#[test]
fn a_jump_into_application_controlled_visibility_is_refused_with_its_reason() {
    let mut ui = Runtime::new();
    let shown = ui.signal(false);
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "advanced",
            View::new(Element::new(
                "expert",
                Button::new("Expert option", |_| Ok(())),
            ))
            .visible_signal(shown.read_only()),
        )]),
    ))
    .unwrap();
    settle(&mut ui);
    let hits = search(&ui, "expert");
    assert_eq!(hits.len(), 1);
    let jump = ui.jump_to_content(&hits[0].target).unwrap();
    assert!(
        matches!(
            jump,
            ContentJump::Reveal(RevealOutcome::Refused(
                RevealRefusal::ApplicationControlled { .. }
            ))
        ),
        "{jump:?}"
    );
}

#[test]
fn content_behind_a_modal_dialog_is_not_found_while_it_is_open() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let state = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("behind", Button::new("Mixer settings", |_| Ok(()))),
            Element::new(
                "prompt",
                AlertDialog::new(
                    state.clone(),
                    Element::new("open", Button::new("Delete…", |_| Ok(()))),
                    "Delete the mixer track?",
                    "This cannot be undone.",
                    "Delete",
                ),
            ),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    assert_eq!(search(&ui, "mixer settings").len(), 1);
    state.open().unwrap();
    settle(&mut ui);
    assert!(search(&ui, "mixer settings").is_empty());
    // The dialog's own content is found.
    assert!(!search(&ui, "delete the mixer track").is_empty());
}

/// 300 roots, each with one child; text names every node.
struct Folders;
impl TreeItems for Folders {
    fn roots(&self) -> Vec<String> {
        (0..300).map(|index| format!("f{index}")).collect()
    }
    fn children(&self, key: &str) -> Vec<String> {
        if key.contains('/') {
            Vec::new()
        } else {
            vec![format!("{key}/file")]
        }
    }
    fn can_expand(&self, key: &str) -> bool {
        !key.contains('/')
    }
    fn build(&self, key: &str) -> Option<RowItem> {
        Some(Element::new(key, Label::new(format!("Node {key}"))).into())
    }
    fn text(&self, key: &str) -> Option<String> {
        Some(format!("Node {key}"))
    }
}

#[test]
fn tree_rows_are_searched_when_shown_and_not_inside_collapsed_nodes() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0).unwrap())
        .unwrap();
    let state = TreeController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        TreeSource::new(Folders),
    )
    .unwrap();
    ui.mount_tree(Element::new("tree", Tree::new(&state, 24.0).unwrap()))
        .unwrap();
    settle(&mut ui);
    // A root far below the viewport is a shown row that is not mounted.
    let hits = search(&ui, "node f250");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(matches!(&hits[0].target, ContentTarget::Row { key, .. } if key == "f250"));
    // Its child is inside a collapsed node and is not a row.
    assert!(search(&ui, "f250/file").is_empty());
    state.set_expanded("f250", true).unwrap();
    settle(&mut ui);
    assert_eq!(search(&ui, "f250/file").len(), 1);
    let jump = ui.jump_to_content(&hits[0].target).unwrap();
    assert!(matches!(jump, ContentJump::Reveal(outcome) if outcome.is_revealed()));
}

#[test]
fn query_rules_reject_empty_text_and_zero_limits() {
    assert_eq!(
        ContentQuery::new(" \t"),
        Err(ContentSearchError::EmptyQuery)
    );
    assert_eq!(
        ContentQuery::new("a").unwrap().limit(0),
        Err(ContentSearchError::InvalidLimit)
    );
    assert_eq!(
        ContentQuery::new("a").unwrap().budget(0),
        Err(ContentSearchError::InvalidLimit)
    );
    assert_eq!(ContentQuery::everything().text(), None);
}

#[test]
fn an_opt_out_made_while_a_search_runs_hides_the_queued_subtree() {
    let mut ui = Runtime::new();
    let panel = |searchable: bool| {
        Element::new(
            "panel",
            Column::new([Element::new("needle", Label::new("Needle"))]),
        )
        .with_searchable(searchable)
    };
    let root = ui
        .mount_tree(Element::new("root", Column::new([panel(true)])))
        .unwrap();
    settle(&mut ui);
    // Three units find the roots, two visit the column and the panel, which
    // queues the label.
    let mut search = ui.content_search(ContentQuery::new("needle").unwrap().budget(5).unwrap());
    let first = search.step(&ui).unwrap();
    assert!(first.hits.is_empty() && !first.done, "{first:?}");
    ui.reconcile_children(&root, vec![panel(false)]).unwrap();
    settle(&mut ui);
    assert_eq!(search.step(&ui), Err(ContentSearchError::Stale));
    assert!(search_hits(&ui, "needle").is_empty());
}

fn search_hits(ui: &Runtime, text: &str) -> Vec<ContentHit> {
    search(ui, text)
}

#[test]
fn the_budget_counts_finding_the_roots_and_indexing_mounted_rows() {
    let mut ui = Runtime::new();
    let (_, _table) = table(&mut ui, 50);
    let components = ui.inspect().components;
    let entries = components.len();
    let mounted_rows = components
        .iter()
        .filter(|component| {
            component
                .key
                .strip_prefix('n')
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
        })
        .count();
    assert!(mounted_rows > 0);
    let mut search = ui.content_search(ContentQuery::new("absent").unwrap().budget(1).unwrap());
    let mut work = 0;
    loop {
        let step = search.step(&ui).unwrap();
        assert!(step.work <= 1, "{step:?}");
        work += step.work;
        if step.done {
            break;
        }
    }
    // Roots, visits, the body list's mounted rows and its 50 source rows.
    assert_eq!(work, 2 * entries + mounted_rows + 50);
}

#[test]
fn replacing_a_collection_the_search_has_finished_makes_it_stale() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(1_000)),
    )
    .unwrap();
    // A split gives the table a bounded pane, so most of its rows stay
    // unmounted; the label in the second pane is searched after it.
    ui.mount_tree(Element::new(
        "split",
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("squares", Table::new(&state, 24.0).unwrap()),
            Element::new("after", Label::new("Trailing")),
        ),
    ))
    .unwrap();
    settle(&mut ui);
    // The last row ends the table's scan; the label after it is still to come.
    let mut search = ui.content_search(ContentQuery::new("row 999").unwrap().limit(1).unwrap());
    let first = search.step(&ui).unwrap();
    assert_eq!(first.hits.len(), 1, "{first:?}");
    assert!(!first.done);
    // The same rows from a new source: nothing mounts or unmounts, only the
    // finished collection's rows are replaced.
    state.set_source(TableSource::new(Squares(1_000))).unwrap();
    settle(&mut ui);
    assert_eq!(search.step(&ui), Err(ContentSearchError::Stale));
}

struct Words(Vec<&'static str>);
impl ListItems for Words {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|word| format!("{word}-{index}"))
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        let word = *self.0.get(index)?;
        Some(Element::new(format!("{word}-{index}"), Button::new(word, |_| Ok(()))).into())
    }
    fn text(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|word| (*word).to_owned())
    }
}

#[test]
fn replacing_the_source_of_a_list_never_shown_makes_the_search_stale() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let words: Vec<&'static str> = (0..200).map(|_| "filler").collect();
    let source = ui.signal(ListSource::new(Words(words)));
    ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new("one", "One", Element::new("intro", Label::new("Welcome"))),
            Tab::new(
                "two",
                "Two",
                Element::new("words", VirtualList::new(source.read_only(), 24.0).unwrap()),
            ),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    // Step until the hidden list's rows, which it offers before it has ever
    // updated, are being scanned: a step then spends its whole budget on rows.
    let mut search = ui.content_search(ContentQuery::new("absent").unwrap().budget(40).unwrap());
    let mut steps = 0;
    while steps < 3 {
        let step = search.step(&ui).unwrap();
        assert!(!step.done, "the rows take several steps");
        steps += 1;
    }
    source
        .set(ListSource::new(Words(vec!["replaced"])))
        .unwrap();
    assert_eq!(search.step(&ui), Err(ContentSearchError::Stale));
}

#[test]
fn previously_shown_hidden_lists_search_the_current_source_without_presenting_it() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Words(vec!["filler"; 200])));
    let list = ui
        .mount_tree(Element::new(
            "words",
            VirtualList::new(source.read_only(), 24.0).unwrap(),
        ))
        .unwrap();
    settle(&mut ui);
    let mounted = ui.children(&list).unwrap();
    assert!(!mounted.is_empty() && mounted.len() < 200);
    ui.set_visible(&list, false).unwrap();
    settle(&mut ui);
    let mut cursor = ui.content_search(ContentQuery::new("filler").unwrap().budget(40).unwrap());
    loop {
        let step = cursor.step(&ui).unwrap();
        assert!(!step.done, "the old rows require multiple steps");
        if step.hits.iter().any(|hit| matches!(&hit.target, ContentTarget::Row { collection, .. } if *collection == list)) {
            break;
        }
    }
    source
        .set(ListSource::new(Words(vec!["replacement"])))
        .unwrap();
    assert_eq!(cursor.step(&ui), Err(ContentSearchError::Stale));
    let hits = search(&ui, "replacement");
    assert_eq!(hits.len(), 1);
    assert!(
        matches!(&hits[0].target, ContentTarget::Row { collection, key } if *collection == list && key == "replacement-0")
    );
    assert!(search(&ui, "filler").is_empty());
    assert!(
        ui.content_search(ContentQuery::new("filler").unwrap().rows(false))
            .step(&ui)
            .unwrap()
            .hits
            .is_empty()
    );
    assert_eq!(ui.children(&list).unwrap(), mounted);
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    ui.set_visible(&list, true).unwrap();
    settle(&mut ui);
    let hits = search(&ui, "replacement");
    assert_eq!(hits.len(), 1, "current mounted rows must not be duplicated");
    assert!(matches!(hits[0].target, ContentTarget::Widget(_)));
}

#[test]
fn a_mounted_only_search_stales_when_a_visited_hidden_list_changes_source() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Words(vec!["filler"; 200])));
    let list = ui
        .mount_tree(Element::new(
            "words",
            VirtualList::new(source.read_only(), 24.0).unwrap(),
        ))
        .unwrap();
    settle(&mut ui);
    ui.set_visible(&list, false).unwrap();
    settle(&mut ui);
    let budget = ui.inspect().components.len() + 1;
    let mut cursor = ui.content_search(
        ContentQuery::new("filler")
            .unwrap()
            .rows(false)
            .budget(budget)
            .unwrap(),
    );
    let first = cursor.step(&ui).unwrap();
    assert!(!first.done && first.hits.is_empty());
    source
        .set(ListSource::new(Words(vec!["replacement"])))
        .unwrap();
    assert_eq!(cursor.step(&ui), Err(ContentSearchError::Stale));
}

#[test]
fn dirty_derived_list_source_read_errors_are_not_obsolete_widget_hits() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Words(vec!["filler"])));
    let derived = source.map(Clone::clone).unwrap();
    ui.mount_tree(Element::new(
        "words",
        VirtualList::new(derived, 24.0).unwrap(),
    ))
    .unwrap();
    settle(&mut ui);
    source
        .set(ListSource::new(Words(vec!["replacement"])))
        .unwrap();
    assert!(matches!(
        ui.content_search(ContentQuery::new("filler").unwrap())
            .step(&ui),
        Err(ContentSearchError::Runtime(
            RuntimeError::WriteDuringInspection
        ))
    ));
    settle(&mut ui);
    assert!(search(&ui, "filler").is_empty());
    assert_eq!(search(&ui, "replacement").len(), 1);
}

/// A row the source counts and keys but cannot build.
struct InvalidRow;
impl ListItems for InvalidRow {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "filler-0".to_owned())
    }
    fn build(&self, _: usize) -> Option<pert_ui::RowItem> {
        None
    }
    fn text(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "replacement".to_owned())
    }
}

#[test]
fn failed_row_build_does_not_mark_retained_children_as_current() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Words(vec!["filler"])));
    ui.mount_tree(Element::new(
        "words",
        VirtualList::new(source.read_only(), 24.0).unwrap(),
    ))
    .unwrap();
    settle(&mut ui);
    source.set(ListSource::new(InvalidRow)).unwrap();
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Layout(LayoutError::InvalidChild))
    ));
    assert!(search(&ui, "filler").is_empty());
    let hits = search(&ui, "replacement");
    assert_eq!(hits.len(), 1);
    assert!(matches!(hits[0].target, ContentTarget::Row { .. }));
}

struct RejectedRow;
impl Component for RejectedRow {
    fn check_description(&self) -> Result<(), RuntimeError> {
        Err(RuntimeError::Exhausted)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}
struct RejectedSource;
impl ListItems for RejectedSource {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "filler-0".to_owned())
    }
    fn build(&self, _: usize) -> Option<pert_ui::RowItem> {
        Some(Element::new("filler-0", RejectedRow).into())
    }
    fn text(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "replacement".to_owned())
    }
}

#[test]
fn rejected_child_reconciliation_does_not_mark_retained_rows_as_current() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(Words(vec!["filler"])));
    ui.mount_tree(Element::new(
        "words",
        VirtualList::new(source.read_only(), 24.0).unwrap(),
    ))
    .unwrap();
    settle(&mut ui);
    source.set(ListSource::new(RejectedSource)).unwrap();
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::Exhausted)
    ));
    assert!(search(&ui, "filler").is_empty());
    let hits = search(&ui, "replacement");
    assert_eq!(hits.len(), 1);
    assert!(matches!(hits[0].target, ContentTarget::Row { .. }));
    source
        .set(ListSource::new(Words(vec!["recovered"])))
        .unwrap();
    settle(&mut ui);
    assert_eq!(search(&ui, "recovered").len(), 1);
    assert!(search(&ui, "replacement").is_empty());
}

#[test]
fn rows_of_a_collection_behind_a_modal_dialog_are_not_found() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let state = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(10_000)),
    )
    .unwrap();
    let dialog = DialogController::new(&ui);
    // A split gives the table a bounded pane, so most of its rows stay
    // unmounted.
    ui.mount_tree(Element::new(
        "split",
        SplitPane::new(
            SplitAxis::TopBottom,
            Element::new("squares", Table::new(&state, 24.0).unwrap()),
            Element::new(
                "prompt",
                AlertDialog::new(
                    dialog.clone(),
                    Element::new("open", Button::new("Delete…", |_| Ok(()))),
                    "Delete the table?",
                    "This cannot be undone.",
                    "Delete",
                ),
            ),
        ),
    ))
    .unwrap();
    settle(&mut ui);
    assert_eq!(search(&ui, "row 4321").len(), 1);
    dialog.open().unwrap();
    settle(&mut ui);
    assert!(search(&ui, "row 4321").is_empty());
}

#[test]
fn a_modal_in_another_window_blocks_its_content_whichever_window_is_selected() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
        .unwrap();
    let dialog = DialogController::new(&ui);
    let root = ui
        .mount_tree(Element::new(
            "tabs",
            Tabs::new([
                Tab::new(
                    "editor",
                    "Editor",
                    Element::new(
                        "editor-content",
                        Column::new([
                            Element::new("behind", Button::new("Mixer settings", |_| Ok(()))),
                            Element::new(
                                "prompt",
                                AlertDialog::new(
                                    dialog.clone(),
                                    Element::new("open", Button::new("Delete…", |_| Ok(()))),
                                    "Delete the mixer track?",
                                    "This cannot be undone.",
                                    "Delete",
                                ),
                            ),
                        ]),
                    ),
                ),
                Tab::new("other", "Other", Element::new("note", Label::new("Other"))),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let view = ui.children(&root).unwrap()[0].clone();
    let primary = ui.primary_window();
    let prepared = ui
        .prepare_detach(&view, NativeWindowOptions::new("Editor").unwrap())
        .unwrap();
    let detached = ui.commit_detach(prepared).unwrap();
    settle(&mut ui);
    ui.select_window(&detached).unwrap();
    dialog.open().unwrap();
    settle(&mut ui);
    assert!(search(&ui, "mixer settings").is_empty());
    ui.select_window(&primary).unwrap();
    settle(&mut ui);
    assert!(search(&ui, "mixer settings").is_empty());
    assert!(!search(&ui, "delete the mixer track").is_empty());
}

/// A collection whose rows a list inside an application-controlled view
/// mounts, so a reveal waits until the application shows the view.
struct Shelf {
    children: Vec<Element>,
    items: ListSource,
    controller: ListController,
}
struct ShelfRows {
    items: ListSource,
    controller: ListController,
}
impl ContentRows for ShelfRows {
    fn len(&self) -> usize {
        self.items.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.items.key(index)
    }
    fn text(&self, index: usize) -> Option<String> {
        self.items.text(index)
    }
    fn revision(&self) -> u64 {
        1
    }
    fn reveal(&self, key: &str) -> Result<ListRevealReceipt, RuntimeError> {
        self.controller.reveal(key)
    }
}
impl Component for Shelf {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        cx.limits(0)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        cx.place(0, Rect::new(0.0, 0.0, size.width(), size.height())?)
    }
    fn content_rows(&self) -> Option<std::rc::Rc<dyn ContentRows>> {
        Some(std::rc::Rc::new(ShelfRows {
            items: self.items.clone(),
            controller: self.controller.clone(),
        }))
    }
}

#[test]
fn a_jump_to_a_row_mounted_later_keeps_its_completion_and_focuses_it() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    let words: Vec<&'static str> = (0..100)
        .map(|index| if index == 80 { "target" } else { "filler" })
        .collect();
    let items = ListSource::new(Words(words));
    let source = ui.signal(items.clone());
    let controller = ListController::new(&ui);
    let shown = ui.signal(false);
    let list = VirtualList::new(source.read_only(), 24.0)
        .unwrap()
        .controller(controller.clone());
    let shelf = Shelf {
        children: vec![
            Element::new(
                "view",
                View::new(Element::new("list", list)).visible_signal(shown.read_only()),
            )
            .with_searchable(false),
        ],
        items,
        controller,
    };
    let shelf = ui.mount_tree(Element::new("shelf", shelf)).unwrap();
    settle(&mut ui);
    let hit = search(&ui, "target")
        .into_iter()
        .find(|hit| {
            matches!(&hit.target, ContentTarget::Row { collection, .. } if *collection == shelf)
        })
        .expect("the shelf offers the row");
    let ContentJump::Pending(pending) = ui.jump_to_content(&hit.target).unwrap() else {
        panic!("the hidden list cannot mount the row yet");
    };
    assert_eq!(pending.receipt().status(), ListRevealStatus::Pending);
    // Still pending: resuming returns the pending jump again.
    assert!(matches!(
        ui.resume_content_jump(&pending).unwrap(),
        ContentJump::Pending(_)
    ));
    shown.set(true).unwrap();
    settle(&mut ui);
    assert!(matches!(
        pending.receipt().status(),
        ListRevealStatus::Mounted(_)
    ));
    let jump = ui.resume_content_jump(&pending).unwrap();
    assert!(
        matches!(
            jump,
            ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. })
        ),
        "{jump:?}"
    );
    let focused = ui.focused().expect("the row has focus");
    assert_eq!(ui.key(&focused).unwrap(), "target-80");
}

#[test]
fn a_hit_carries_the_components_other_text() {
    let mut ui = Runtime::new();
    let name = ui.signal("Reverb tail".to_owned());
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("name", TextField::new(name)).with_label("Name")]),
    ))
    .unwrap();
    settle(&mut ui);
    let mut everything = ui.content_search(ContentQuery::everything());
    let hits = everything.step(&ui).unwrap().hits;
    let hit = hits
        .iter()
        .find(|hit| hit.text == "Name")
        .expect("the field is listed");
    assert_eq!(hit.field, ContentField::Name);
    assert_eq!(hit.other.as_deref(), Some("Reverb tail"));
    let by_value = search(&ui, "reverb");
    assert_eq!(by_value[0].field, ContentField::Value);
    assert_eq!(by_value[0].other.as_deref(), Some("Name"));
}

/// New-behavior test. Named assertion: a content search for a `Field`'s name
/// finds an ordinary, unnamed text field inside an `Each` the application
/// marks structural: the search's naming fallback climbs through the
/// structural `Each` and its default container, which used to stop it. The
/// inspection assertions are contract checks: inspection names only a
/// structural component through structure, so it does not name this one.
#[test]
fn a_field_names_an_ordinary_item_of_a_structural_each_in_content_search() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let entries = ui.signal(vec!["one".to_owned()]);
    let root = ui
        .mount_tree(Element::new(
            "field",
            Field::new(
                Each::new(
                    &entries,
                    |_| "entry".to_owned(),
                    |cx, _| TextField::new(cx.signal(String::new())),
                )
                .key("entries")
                .structural(),
            )
            .label("Batch"),
        ))
        .unwrap();
    settle(&mut ui);
    let entry = ui.find_descendant(&root, "entry").unwrap().unwrap();
    let hits = search(&ui, "Batch");
    assert!(
        hits.iter()
            .any(|hit| hit.field == ContentField::Name && widget(hit) == entry),
        "{hits:?}"
    );
    let inspected = ui
        .query(&InspectionQuery::components(Selector::key("entry")).detail(Detail::Semantics))
        .components;
    assert_eq!(
        inspected[0].details.semantics.as_ref().unwrap().name,
        None,
        "inspection does not name an ordinary component through structure"
    );
    assert!(
        ui.query(&InspectionQuery::components(Selector::semantic(
            SemanticRole::TextInput,
            Some("Batch"),
        )))
        .components
        .is_empty()
    );
}

/// A component named `name` that reports `items` as semantic items, which
/// a test can change between search steps; `protected` conceals them.
struct Itemized {
    name: &'static str,
    items: std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>,
    protected: bool,
    inspected: std::rc::Rc<std::cell::Cell<usize>>,
}
impl Component for Itemized {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn inspect(&self) -> ComponentDetails {
        self.inspected.set(self.inspected.get() + 1);
        let items = self
            .items
            .borrow()
            .iter()
            .map(|name| SemanticItem::new(*name))
            .collect::<Vec<_>>();
        let mut semantics = Semantics::new(SemanticRole::List)
            .name(self.name)
            .items(items);
        if self.protected {
            semantics = semantics.protected(true);
        }
        ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        }
    }
}

type Shared = std::rc::Rc<std::cell::RefCell<Vec<&'static str>>>;

fn itemized(name: &'static str, items: &[&'static str], protected: bool) -> (Itemized, Shared) {
    let shared: Shared = std::rc::Rc::new(std::cell::RefCell::new(items.to_vec()));
    (
        Itemized {
            name,
            items: shared.clone(),
            protected,
            inspected: std::rc::Rc::default(),
        },
        shared,
    )
}

#[test]
fn a_visible_toast_is_found_by_its_message() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 480.0, 1.0).unwrap())
        .unwrap();
    let queue = ToastController::new(&ui);
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(queue.clone(), Element::new("body", Label::new("Content"))),
    ))
    .unwrap();
    queue.push(Toast::new("Upload failed")).unwrap();
    settle(&mut ui);
    let hits = search(&ui, "upload");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].text, "Upload failed");
    assert_eq!(hits[0].field, ContentField::Item { index: 0 });
}

#[test]
fn an_unselected_tab_title_is_found_as_an_item_of_the_tabs() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new(
                "one",
                "Overview",
                Element::new("intro", Label::new("Welcome")),
            ),
            Tab::new(
                "two",
                "Statistics",
                Element::new("data", Label::new("Data")),
            ),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let hits = search(&ui, "statistics");
    assert!(
        hits.iter()
            .any(|hit| hit.field == ContentField::Item { index: 1 }),
        "{hits:?}"
    );
}

#[test]
fn a_components_name_comes_before_its_items_and_protected_items_are_never_found() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (open, _) = itemized(
        "Recent files",
        &["Recent report", "Old notes", "Recent draft"],
        false,
    );
    let (secret, _) = itemized("Recent passwords", &["Recent bank"], true);
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("open", open), Element::new("secret", secret)]),
    ))
    .unwrap();
    settle(&mut ui);
    let found: Vec<(String, ContentField, Option<String>)> = search(&ui, "recent")
        .into_iter()
        .map(|hit| (hit.text, hit.field, hit.other))
        .collect();
    let owner = Some("Recent files".to_owned());
    assert_eq!(
        found,
        [
            ("Recent files".to_owned(), ContentField::Name, None),
            (
                "Recent report".to_owned(),
                ContentField::Item { index: 0 },
                owner.clone()
            ),
            (
                "Recent draft".to_owned(),
                ContentField::Item { index: 2 },
                owner
            ),
            // A protected component's name is found; its items are not.
            ("Recent passwords".to_owned(), ContentField::Name, None),
        ]
    );
}

fn search_texts(ui: &Runtime, text: &str) -> Vec<String> {
    search(ui, text).into_iter().map(|hit| hit.text).collect()
}

#[test]
fn item_scanning_continues_across_tiny_steps_without_duplicates_or_skips() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (list, items) = itemized("Item list", &["item a", "skip", "item b", "item c"], false);
    let (after, _) = itemized("Item after", &[], false);
    ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new("list", list), Element::new("after", after)]),
    ))
    .unwrap();
    settle(&mut ui);
    for (limit, budget) in [(1, 1), (1, 64), (64, 1), (2, 3)] {
        let query = ContentQuery::new("item")
            .unwrap()
            .limit(limit)
            .unwrap()
            .budget(budget)
            .unwrap();
        let mut search = ui.content_search(query);
        let mut found = Vec::new();
        let mut inserted = false;
        loop {
            let step = search.step(&ui).unwrap();
            assert!(step.hits.len() <= limit && step.work <= budget.max(1));
            found.extend(step.hits.into_iter().map(|hit| hit.text));
            if !inserted && found.iter().any(|text| text == "Item list") {
                // Items added after the component was visited are seen only
                // by a new search, so the continuation neither repeats nor
                // skips one.
                items.borrow_mut().insert(0, "item inserted");
                inserted = true;
            }
            if step.done {
                break;
            }
        }
        assert_eq!(
            found,
            ["Item list", "item a", "item b", "item c", "Item after"],
            "limit {limit}, budget {budget}"
        );
        items.borrow_mut().remove(0);
        // A new search sees the current items.
        assert!(
            !search_texts(&ui, "item").contains(&"item inserted".to_owned()),
            "the inserted item was removed again"
        );
    }
}

#[test]
fn jumping_to_an_item_hit_focuses_its_component() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (list, _) = itemized("Commands", &["Save all"], false);
    ui.mount_tree(Element::new("list", list)).unwrap();
    settle(&mut ui);
    let hits = search(&ui, "save all");
    assert_eq!(hits.len(), 1);
    ui.jump_to_content(&hits[0].target).unwrap();
    assert_eq!(ui.focused(), Some(widget(&hits[0])));
}

#[test]
fn resuming_an_item_scan_does_not_inspect_its_component_again() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (list, _) = itemized("Item list", &["item a", "item b", "item c"], false);
    let inspected = list.inspected.clone();
    ui.mount_tree(Element::new("list", list)).unwrap();
    settle(&mut ui);
    let query = ContentQuery::new("item").unwrap().limit(1).unwrap();
    let mut search = ui.content_search(query);
    let before = inspected.get();
    let mut steps = 0;
    while !search.step(&ui).unwrap().done {
        steps += 1;
    }
    assert!(steps >= 3, "the items took several steps");
    assert_eq!(
        inspected.get() - before,
        1,
        "one inspection for the whole scan"
    );
}

#[test]
fn a_modal_opened_between_steps_blocks_the_rest_of_an_item_scan() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (list, _) = itemized("Item list", &["item a", "item b"], false);
    let dialog = DialogController::new(&ui);
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("list", list),
            Element::new(
                "dialog",
                Dialog::new(
                    dialog.clone(),
                    Element::new("open", Button::new("Open", |_| Ok(()))),
                    Element::new("body", Label::new("Modal body")),
                ),
            ),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let mut search = ui.content_search(ContentQuery::new("item").unwrap().limit(1).unwrap());
    let first = search.step(&ui).unwrap();
    assert_eq!(first.hits.len(), 1);
    assert_eq!(first.hits[0].text, "Item list");
    dialog.open().unwrap();
    settle(&mut ui);
    let mut rest = Vec::new();
    loop {
        let step = search.step(&ui).unwrap();
        rest.extend(step.hits.into_iter().map(|hit| hit.text));
        if step.done {
            break;
        }
    }
    assert!(
        rest.is_empty(),
        "the modal blocks the saved items: {rest:?}"
    );
}

#[test]
fn a_picker_is_searchable_unless_it_presents_search_results() {
    for searchable in [true, false] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 600.0, 1.0).unwrap())
            .unwrap();
        let picker = QuickPickController::new(&ui);
        ui.mount_tree(Element::new(
            "app",
            QuickPick::new(
                picker.clone(),
                Element::new("doc", Label::new("Searchable document")),
                |_, _| Ok(PickChoice::Done),
            )
            .name("Search")
            .searchable(searchable),
        ))
        .unwrap();
        for _ in 0..2 {
            picker
                .open(vec![PickItem::new("result", "Searchable result")])
                .unwrap();
            settle(&mut ui);
            picker.close().unwrap();
            settle(&mut ui);
            let found: Vec<String> = search(&ui, "searchable")
                .into_iter()
                .map(|hit| hit.text)
                .collect();
            // The anchor's content is found either way; the picker's own
            // retained results only when the picker is searchable.
            assert_eq!(found[0], "Searchable document");
            assert_eq!(
                found.iter().any(|text| text == "Searchable result"),
                searchable,
                "searchable {searchable}: {found:?}"
            );
        }
    }
}

/// A list of rows that also offers its unmounted rows' text, between a
/// component with matching items and a matching sibling.
struct MatchRows(usize);
impl ListItems for MatchRows {
    fn len(&self) -> usize {
        self.0
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("w{index}"))
    }
    fn build(&self, index: usize) -> Option<RowItem> {
        (index < self.0).then(|| Label::new(format!("match row {index}")).into())
    }
    fn text(&self, index: usize) -> Option<String> {
        (index < self.0).then(|| format!("match row {index}"))
    }
}

#[test]
fn items_rows_and_children_keep_their_order_across_one_unit_steps() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 120.0, 1.0).unwrap())
        .unwrap();
    let (owner, _) = itemized("match owner", &["match item"], false);
    let rows = ui.signal(ListSource::new(MatchRows(40)));
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("owner", owner),
            Element::new("list", VirtualList::new(rows.read_only(), 24.0).unwrap())
                .with_label("match list")
                .with_grow(1),
            Element::new("after", Label::new("match after")),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let collect = |limit: usize, budget: usize| {
        let query = ContentQuery::new("match")
            .unwrap()
            .limit(limit)
            .unwrap()
            .budget(budget)
            .unwrap();
        let mut search = ui.content_search(query);
        let mut found = Vec::new();
        loop {
            let step = search.step(&ui).unwrap();
            found.extend(step.hits.into_iter().map(|hit| (hit.field, hit.text)));
            if step.done {
                return found;
            }
        }
    };
    let unbounded = collect(1000, 100_000);
    assert_eq!(collect(1, 1), unbounded, "one-unit steps keep the order");
    // The owner's name, then its item, before the list and the sibling.
    assert_eq!(unbounded[0], (ContentField::Name, "match owner".to_owned()));
    assert_eq!(
        unbounded[1],
        (ContentField::Item { index: 0 }, "match item".to_owned())
    );
    assert_eq!(
        unbounded.last(),
        Some(&(ContentField::Name, "match after".to_owned()))
    );
    // Every row is found exactly once, mounted or not.
    let rows: Vec<&String> = unbounded
        .iter()
        .filter(|(_, text)| text.starts_with("match row"))
        .map(|(_, text)| text)
        .collect();
    assert_eq!(rows.len(), 40, "{rows:?}");
}

#[test]
fn a_new_search_sees_an_item_added_after_an_earlier_one() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let (list, items) = itemized("Item list", &["item a"], false);
    ui.mount_tree(Element::new("list", list)).unwrap();
    settle(&mut ui);
    assert_eq!(search_texts(&ui, "item"), ["Item list", "item a"]);
    items.borrow_mut().push("item added");
    assert_eq!(
        search_texts(&ui, "item"),
        ["Item list", "item a", "item added"]
    );
}

/// A component with matching items and unmounted rows, borrowed from an
/// inner list's source. The inner list is never mounted, so it reports its
/// mounted rows as not current and the search visits no children here; the
/// order of rows and mounted children is tested with a mounted list above.
struct Combined {
    rows: VirtualList,
    child: Option<Element>,
}
impl Component for Combined {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::List)
                    .name("match combined")
                    .items([
                        SemanticItem::new("match item one"),
                        SemanticItem::new("match item two"),
                    ]),
            ),
            ..ComponentDetails::default()
        }
    }
    fn try_content_rows(&self) -> Result<Option<std::rc::Rc<dyn ContentRows>>, RuntimeError> {
        self.rows.try_content_rows()
    }
}

#[test]
fn one_unit_steps_report_name_items_rows_and_siblings_in_order() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal(ListSource::new(MatchRows(3)));
    let combined = Combined {
        rows: VirtualList::new(source.read_only(), 24.0).unwrap(),
        child: None,
    };
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("combined", combined),
            Label::new("match after").key("after"),
        ]),
    ))
    .unwrap();
    settle(&mut ui);
    let expected: Vec<(ContentField, String, Option<String>)> = vec![
        (ContentField::Name, "match combined".into(), None),
        (
            ContentField::Item { index: 0 },
            "match item one".into(),
            None,
        ),
        (
            ContentField::Item { index: 1 },
            "match item two".into(),
            None,
        ),
        (ContentField::Row, "match row 0".into(), Some("w0".into())),
        (ContentField::Row, "match row 1".into(), Some("w1".into())),
        (ContentField::Row, "match row 2".into(), Some("w2".into())),
        (ContentField::Name, "match after".into(), None),
    ];
    for (limit, budget) in [(1, 1), (1000, 100_000), (2, 3)] {
        let query = ContentQuery::new("match")
            .unwrap()
            .limit(limit)
            .unwrap()
            .budget(budget)
            .unwrap();
        let mut search = ui.content_search(query);
        let mut found = Vec::new();
        loop {
            let step = search.step(&ui).unwrap();
            assert!(step.hits.len() <= limit);
            found.extend(step.hits.into_iter().map(|hit| {
                let row = match &hit.target {
                    ContentTarget::Row { key, .. } => Some(key.clone()),
                    _ => None,
                };
                (hit.field, hit.text, row)
            }));
            if step.done {
                break;
            }
        }
        assert_eq!(found, expected, "limit {limit}, budget {budget}");
    }
}

#[test]
fn a_mounted_list_reports_its_unmounted_rows_before_its_mounted_children_with_targets() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 120.0, 1.0).unwrap())
        .unwrap();
    let (owner, _) = itemized("match owner", &["match item"], false);
    let rows = ui.signal(ListSource::new(MatchRows(12)));
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("owner", owner),
                Element::new("list", VirtualList::new(rows.read_only(), 24.0).unwrap())
                    .with_label("match list")
                    .with_grow(1),
                Element::new("after", Label::new("match after")),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let list = ui.find_descendant(&root, "list").unwrap().unwrap();
    let after = ui.find_descendant(&root, "after").unwrap().unwrap();
    let mounted: Vec<(String, WidgetId)> = ui
        .children(&list)
        .unwrap()
        .into_iter()
        .map(|id| (ui.key(&id).unwrap().to_owned(), id))
        .collect();
    assert!(
        !mounted.is_empty() && mounted.len() < 12,
        "{} mounted",
        mounted.len()
    );
    // The expected sequence: the owner's name and item, the list's name, its
    // unmounted rows in source order, its mounted rows in child order, then
    // the sibling.
    #[derive(Debug, PartialEq)]
    enum Target {
        Widget(WidgetId),
        Row(WidgetId, String),
    }
    let mut expected: Vec<(ContentField, String, Target)> = vec![
        (
            ContentField::Name,
            "match owner".into(),
            Target::Widget(owner.clone()),
        ),
        (
            ContentField::Item { index: 0 },
            "match item".into(),
            Target::Widget(owner),
        ),
        (
            ContentField::Name,
            "match list".into(),
            Target::Widget(list.clone()),
        ),
    ];
    for index in 0..12 {
        let key = format!("w{index}");
        if !mounted.iter().any(|(mounted, _)| mounted == &key) {
            expected.push((
                ContentField::Row,
                format!("match row {index}"),
                Target::Row(list.clone(), key),
            ));
        }
    }
    for (key, id) in &mounted {
        let index: usize = key[1..].parse().unwrap();
        expected.push((
            ContentField::Name,
            format!("match row {index}"),
            Target::Widget(id.clone()),
        ));
    }
    expected.push((
        ContentField::Name,
        "match after".into(),
        Target::Widget(after),
    ));
    for (limit, budget) in [(1, 1), (1000, 100_000)] {
        let query = ContentQuery::new("match")
            .unwrap()
            .limit(limit)
            .unwrap()
            .budget(budget)
            .unwrap();
        let mut search = ui.content_search(query);
        let mut found = Vec::new();
        loop {
            let step = search.step(&ui).unwrap();
            found.extend(step.hits.into_iter().map(|hit| {
                let target = match hit.target {
                    ContentTarget::Widget(id) => Target::Widget(id),
                    ContentTarget::Row { collection, key } => Target::Row(collection, key),
                    other => panic!("unexpected target {other:?}"),
                };
                (hit.field, hit.text, target)
            }));
            if step.done {
                break;
            }
        }
        assert_eq!(found, expected, "limit {limit}, budget {budget}");
    }
}

/// A component with matching items whose child is a mounted list: a
/// collection nested inside an item-reporting component.
struct ItemOwner {
    child: Option<Element>,
}
impl Component for ItemOwner {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::List)
                    .name("match owner")
                    .items([
                        SemanticItem::new("match item one"),
                        SemanticItem::new("match item two"),
                    ]),
            ),
            ..ComponentDetails::default()
        }
    }
}

#[test]
fn a_list_nested_in_an_item_reporter_continues_its_order_across_steps() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 120.0, 1.0).unwrap())
        .unwrap();
    let rows = ui.signal(ListSource::new(MatchRows(12)));
    let list = Element::new("list", VirtualList::new(rows.read_only(), 24.0).unwrap())
        .with_label("match list")
        .with_grow(1);
    let root = ui
        .mount_tree(Element::new(
            "root",
            Column::new([
                Element::new("owner", ItemOwner { child: Some(list) }).with_grow(1),
                Element::new("after", Label::new("match after")),
            ]),
        ))
        .unwrap();
    settle(&mut ui);
    let owner = ui.find_descendant(&root, "owner").unwrap().unwrap();
    let list = ui.find_descendant(&owner, "list").unwrap().unwrap();
    let after = ui.find_descendant(&root, "after").unwrap().unwrap();
    let mounted: Vec<(String, WidgetId)> = ui
        .children(&list)
        .unwrap()
        .into_iter()
        .map(|id| (ui.key(&id).unwrap().to_owned(), id))
        .collect();
    assert!(
        !mounted.is_empty() && mounted.len() < 12,
        "{} mounted",
        mounted.len()
    );
    #[derive(Debug, PartialEq)]
    enum Target {
        Widget(WidgetId),
        Row(WidgetId, String),
    }
    // The owner's name and items, then its child list: the list's name, its
    // unmounted rows in source order and its mounted rows in child order;
    // then the owner's sibling.
    let mut expected: Vec<(ContentField, String, Target)> = vec![
        (
            ContentField::Name,
            "match owner".into(),
            Target::Widget(owner.clone()),
        ),
        (
            ContentField::Item { index: 0 },
            "match item one".into(),
            Target::Widget(owner.clone()),
        ),
        (
            ContentField::Item { index: 1 },
            "match item two".into(),
            Target::Widget(owner),
        ),
        (
            ContentField::Name,
            "match list".into(),
            Target::Widget(list.clone()),
        ),
    ];
    for index in 0..12 {
        let key = format!("w{index}");
        if !mounted.iter().any(|(mounted, _)| mounted == &key) {
            expected.push((
                ContentField::Row,
                format!("match row {index}"),
                Target::Row(list.clone(), key),
            ));
        }
    }
    for (key, id) in &mounted {
        let index: usize = key[1..].parse().unwrap();
        expected.push((
            ContentField::Name,
            format!("match row {index}"),
            Target::Widget(id.clone()),
        ));
    }
    expected.push((
        ContentField::Name,
        "match after".into(),
        Target::Widget(after),
    ));
    for (limit, budget) in [(1, 1), (2, 3), (1000, 100_000)] {
        let query = ContentQuery::new("match")
            .unwrap()
            .limit(limit)
            .unwrap()
            .budget(budget)
            .unwrap();
        let mut search = ui.content_search(query);
        let mut found = Vec::new();
        loop {
            let step = search.step(&ui).unwrap();
            assert!(step.hits.len() <= limit);
            found.extend(step.hits.into_iter().map(|hit| {
                let target = match hit.target {
                    ContentTarget::Widget(id) => Target::Widget(id),
                    ContentTarget::Row { collection, key } => Target::Row(collection, key),
                    other => panic!("unexpected target {other:?}"),
                };
                (hit.field, hit.text, target)
            }));
            if step.done {
                break;
            }
        }
        assert_eq!(found, expected, "limit {limit}, budget {budget}");
    }
}
