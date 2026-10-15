//! Content search contracts through the public facade: what is found, what
//! never is, the work budget, staleness and jumping to a hit.
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
        .bound(open.clone()),
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
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(rows)),
        ListSelection::new(&*ui, SelectionMode::Single),
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
    fn build(&self, key: &str) -> Option<Element> {
        Some(Element::new(key, Label::new(format!("Node {key}"))))
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
        TreeSource::new(Folders),
        ListSelection::new(&ui, SelectionMode::Single),
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
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(1_000)),
        ListSelection::new(&ui, SelectionMode::Single),
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
    fn build(&self, index: usize) -> Option<Element> {
        let word = *self.0.get(index)?;
        Some(Element::new(
            format!("{word}-{index}"),
            Button::new(word, |_| Ok(())),
        ))
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
    let derived = source.read_only().map(Clone::clone).unwrap();
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

struct InvalidRow;
impl ListItems for InvalidRow {
    fn len(&self) -> usize {
        1
    }
    fn key(&self, index: usize) -> Option<String> {
        (index == 0).then(|| "filler-0".to_owned())
    }
    fn build(&self, _: usize) -> Option<Element> {
        Some(Element::new("wrong-key", Label::new("replacement")))
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
    fn build(&self, _: usize) -> Option<Element> {
        Some(Element::new("filler-0", RejectedRow))
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
        [TableColumn::new("n", "n"), TableColumn::new("square", "n²")],
        TableSource::new(Squares(10_000)),
        ListSelection::new(&ui, SelectionMode::Single),
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
    assert_eq!(pending.status(), ListRevealStatus::Pending);
    // Still pending: resuming returns the pending jump again.
    assert!(matches!(
        ui.resume_content_jump(&pending).unwrap(),
        ContentJump::Pending(_)
    ));
    shown.set(true).unwrap();
    settle(&mut ui);
    assert!(matches!(pending.status(), ListRevealStatus::Mounted(_)));
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
