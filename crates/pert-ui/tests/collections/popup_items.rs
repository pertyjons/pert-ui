//! The items of the `MultiSelect` and `QuickPick` popup lists: the state
//! they describe and when it changes, the result list's identity, and what
//! content search finds of each list next to the trigger and the rows.
use crate::support;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::{ContentField, ContentHit, ContentQuery, ContentSearchError, ContentTarget};
use std::{cell::RefCell, rc::Rc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FRUIT: [(&str, &str); 4] = [
    ("apple", "Apple"),
    ("banana", "Banana"),
    ("cherry", "Cherry"),
    ("date", "Date"),
];

struct Chooser {
    ui: Runtime,
    root: WidgetId,
    value: Signal<Vec<&'static str>>,
}

fn chooser(
    options: Vec<Choice<&'static str>>,
    build: impl FnOnce(MultiSelect<&'static str>) -> MultiSelect<&'static str>,
) -> TestResult<Chooser> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 420.0, 1.0)?)?;
    let value = ui.signal(Vec::new());
    let control = build(MultiSelect::new(&ui, value.clone(), options)?);
    let root = ui.mount_tree(
        Column::new([
            control.key("tags"),
            Button::new("After", |_| Ok(())).key("after"),
        ])
        .key("root"),
    )?;
    Driver::new(&mut ui).settle()?;
    Ok(Chooser { ui, root, value })
}

fn fruit() -> Vec<Choice<&'static str>> {
    FRUIT
        .iter()
        .map(|(value, label)| Choice::new(*value, *label))
        .collect()
}

impl Chooser {
    fn tags(&self) -> TestResult<WidgetId> {
        Ok(self.ui.find(&self.root, &["tags"])?.ok_or("tags")?)
    }
    fn part(&self, name: &str) -> TestResult<WidgetId> {
        Ok(self
            .ui
            .find_exposed_part(&self.tags()?, name)?
            .ok_or_else(|| format!("no part {name}"))?)
    }
    fn open(&mut self) -> TestResult {
        let control = self.part("control")?;
        let mut driver = Driver::new(&mut self.ui);
        driver.runtime_mut().focus(&control)?;
        driver.key(KeyCode::Enter)?;
        Ok(())
    }
    fn items(&self, part: &str) -> TestResult<Vec<SemanticItem>> {
        let id = self.part(part)?;
        Ok(support::semantics_of(&self.ui, &id)
            .ok_or("semantics")?
            .items)
    }
    fn item(&self, part: &str, name: &str) -> TestResult<SemanticItem> {
        self.items(part)?
            .into_iter()
            .find(|item| item.name.as_deref() == Some(name))
            .ok_or_else(|| format!("no {part} item {name}").into())
    }
    fn highlighted(&self) -> TestResult<Vec<String>> {
        Ok(self
            .items("options")?
            .into_iter()
            .filter(|item| item.selected)
            .filter_map(|item| item.name)
            .collect())
    }
    fn redescribe(
        &mut self,
        build: impl FnOnce(MultiSelect<&'static str>) -> MultiSelect<&'static str>,
    ) -> TestResult {
        let control = build(MultiSelect::new(&self.ui, self.value.clone(), fruit())?);
        self.ui.reconcile_children(
            &self.root,
            vec![
                control.key("tags"),
                Button::new("After", |_| Ok(())).key("after"),
            ],
        )?;
        Driver::new(&mut self.ui).settle()?;
        Ok(())
    }
}

#[test]
fn popup_items_follow_permission_redescription() -> TestResult {
    let mut c = chooser(fruit(), |control| control)?;
    c.open()?;
    let item = c.item("options", "Banana")?;
    assert!(!item.disabled);
    assert_eq!(item.actions, [SemanticAction::Toggle]);
    // Denied while open: the popup stays open with its options disabled.
    c.redescribe(|control| control.enabled(false))?;
    let items = c.items("options")?;
    assert!(
        items
            .iter()
            .all(|item| item.disabled && item.actions.is_empty())
    );
    assert!(items[1].bounds.is_some(), "disabled rows keep rectangles");
    c.redescribe(|control| control)?;
    let item = c.item("options", "Banana")?;
    assert!(!item.disabled);
    assert_eq!(item.actions, [SemanticAction::Toggle]);
    // A bound permission is observed as well.
    let allowed = c.ui.signal(true);
    let permission = allowed.read_only();
    c.redescribe(|control| control.enabled(permission))?;
    allowed.set(false)?;
    Driver::new(&mut c.ui).settle()?;
    assert!(c.item("options", "Banana")?.disabled);
    allowed.set(true)?;
    Driver::new(&mut c.ui).settle()?;
    assert!(!c.item("options", "Banana")?.disabled);
    Ok(())
}

#[test]
fn popup_items_show_external_writes_and_the_highlight_without_rebuilding_rows() -> TestResult {
    let mut c = chooser(fruit(), |control| control)?;
    c.open()?;
    assert_eq!(c.highlighted()?, ["Apple"]);
    // An application write to the bound value.
    c.value.set(vec!["cherry"])?;
    // Until the next flush the items show the previous state, as the rows do.
    assert_eq!(c.item("options", "Cherry")?.checked, Some(false));
    Driver::new(&mut c.ui).settle()?;
    assert_eq!(c.item("options", "Cherry")?.checked, Some(true));
    assert!(c.item("control", "Cherry")?.selected);
    // Moving the highlight changes the items without new rows.
    let options = c.part("options")?;
    let rows = c.ui.children(&options)?;
    Driver::new(&mut c.ui).key(KeyCode::ArrowDown)?;
    assert_eq!(c.highlighted()?, ["Banana"]);
    assert_eq!(c.ui.children(&options)?, rows, "the same rows");
    Ok(())
}

#[test]
fn on_close_items_show_the_draft_until_commit_or_cancel() -> TestResult {
    let mut c = chooser(fruit(), |control| {
        control.commit(MultiSelectCommit::OnClose)
    })?;
    c.open()?;
    Driver::new(&mut c.ui).key(KeyCode::ArrowDown)?;
    Driver::new(&mut c.ui).key(KeyCode::Enter)?;
    // The popup shows the draft; the trigger keeps the committed value.
    assert_eq!(c.item("options", "Banana")?.checked, Some(true));
    assert!(!c.item("control", "Banana")?.selected);
    // Escape cancels: nothing is committed, and a new session starts again
    // from the bound value.
    Driver::new(&mut c.ui).key(KeyCode::Escape)?;
    assert!(c.value.get()?.is_empty());
    assert!(!c.item("control", "Banana")?.selected);
    c.open()?;
    assert_eq!(c.item("options", "Banana")?.checked, Some(false));
    // Toggled again and committed by focus leaving the popup.
    Driver::new(&mut c.ui).key(KeyCode::ArrowDown)?;
    Driver::new(&mut c.ui).key(KeyCode::Enter)?;
    Driver::new(&mut c.ui).key(KeyCode::Tab)?;
    assert_eq!(c.value.get()?, ["banana"]);
    assert!(c.item("control", "Banana")?.selected);
    Ok(())
}

fn search(ui: &Runtime, query: ContentQuery) -> TestResult<Vec<ContentHit>> {
    let mut search = ui.content_search(query);
    let mut hits = Vec::new();
    loop {
        let step = search.step(ui)?;
        hits.extend(step.hits);
        if step.done {
            return Ok(hits);
        }
    }
}

type Kind = (String, ContentField, String);

/// The hits as (target kind, field, text): `control`, `options`, `row` for a
/// mounted row component, `other` for anything else.
fn kinds(control: &WidgetId, options: Option<&WidgetId>, hits: &[ContentHit]) -> Vec<Kind> {
    hits.iter()
        .map(|hit| {
            let kind = match &hit.target {
                ContentTarget::Widget(id) if id == control => "control",
                ContentTarget::Widget(id) if Some(id) == options => "options",
                ContentTarget::Widget(_) if hit.role == Some(SemanticRole::Checkbox) => "row",
                _ => "other",
            };
            (kind.to_owned(), hit.field, hit.text.clone())
        })
        .collect()
}

impl Chooser {
    /// What a search for `query` finds, by kind.
    fn found(&self, query: ContentQuery) -> TestResult<Vec<Kind>> {
        let control = self.part("control")?;
        let options = self.part("options").ok();
        Ok(kinds(&control, options.as_ref(), &search(&self.ui, query)?))
    }
}

fn kind(kind: &str, field: ContentField, text: &str) -> Kind {
    (kind.to_owned(), field, text.to_owned())
}

#[test]
fn content_search_keeps_trigger_popup_and_row_reports_apart() -> TestResult {
    let mut c = chooser(fruit(), |control| control)?;
    let visible = || ContentQuery::new("cherry").map(|query| query.hidden(false));
    // Closed: the visible trigger's item is found.
    assert_eq!(
        c.found(visible()?)?,
        [kind("control", ContentField::Item { index: 2 }, "Cherry")]
    );
    // Open: the trigger's item, the popup list's item and the row.
    c.open()?;
    assert_eq!(
        c.found(visible()?)?,
        [
            kind("control", ContentField::Item { index: 2 }, "Cherry"),
            kind("options", ContentField::Item { index: 2 }, "Cherry"),
            kind("row", ContentField::Name, "Cherry"),
        ]
    );
    // Closed again: the popup's list and rows are hidden; a search that
    // includes hidden components still finds them.
    Driver::new(&mut c.ui).key(KeyCode::Escape)?;
    assert_eq!(
        c.found(visible()?)?,
        [kind("control", ContentField::Item { index: 2 }, "Cherry")]
    );
    let hidden = search(&c.ui, ContentQuery::new("cherry")?)?;
    assert_eq!(
        kinds(&c.part("control")?, Some(&c.part("options")?), &hidden),
        [
            kind("control", ContentField::Item { index: 2 }, "Cherry"),
            kind("options", ContentField::Item { index: 2 }, "Cherry"),
            kind("row", ContentField::Name, "Cherry"),
        ]
    );
    assert!(hidden[0].visible && !hidden[1].visible && !hidden[2].visible);
    Ok(())
}

#[test]
fn content_search_finds_filtered_offscreen_and_duplicate_options_as_distinct_hits() -> TestResult {
    let options = (0..30)
        .map(|index| Choice::new(index, format!("Option {index}")))
        .chain([
            Choice::new(100, "Twin".to_owned()),
            Choice::new(101, "Twin".to_owned()),
        ])
        .collect::<Vec<_>>();
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 420.0, 1.0)?)?;
    let value = ui.signal(Vec::new());
    let root = ui.mount_tree(
        Column::new([MultiSelect::new(&ui, value, options)?.key("tags")]).key("root"),
    )?;
    Driver::new(&mut ui).settle()?;
    let tags = ui.find(&root, &["tags"])?.ok_or("tags")?;
    let control = ui.find_exposed_part(&tags, "control")?.ok_or("control")?;
    let mut driver = Driver::new(&mut ui);
    driver.runtime_mut().focus(&control)?;
    driver.key(KeyCode::Enter)?;
    let options = ui.find_exposed_part(&tags, "options")?.ok_or("options")?;
    let kinds = |ui: &Runtime, text: &str| -> TestResult<_> {
        let query = ContentQuery::new(text)?.hidden(false);
        Ok(kinds(&control, Some(&options), &search(ui, query)?))
    };
    // An offscreen option, not mounted: the trigger's and the list's items,
    // but no row.
    assert_eq!(
        kinds(&ui, "Option 25")?,
        [
            kind("control", ContentField::Item { index: 25 }, "Option 25"),
            kind("options", ContentField::Item { index: 25 }, "Option 25"),
        ]
    );
    // Equal labels are distinct options: nothing is merged.
    let twins = kinds(&ui, "Twin")?;
    let count = |wanted: &str| twins.iter().filter(|(kind, ..)| kind == wanted).count();
    assert_eq!((count("control"), count("options")), (2, 2), "{twins:?}");
    // A filter leaves out what it does not match from the list, not from the
    // trigger.
    Driver::new(&mut ui).type_text("Option 1")?;
    assert_eq!(
        kinds(&ui, "Option 25")?,
        [kind(
            "control",
            ContentField::Item { index: 25 },
            "Option 25"
        )]
    );
    let late = kinds(&ui, "Option 19")?;
    assert!(late.contains(&kind(
        "options",
        ContentField::Item { index: 10 },
        "Option 19"
    )));
    Ok(())
}

// ---- QuickPick ----

struct Picker {
    ui: Runtime,
    app: WidgetId,
    controller: QuickPickController,
    answer: Rc<RefCell<PickChoice>>,
}

fn picker(build: impl FnOnce(QuickPick) -> QuickPick) -> TestResult<Picker> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let controller = QuickPickController::new(&ui);
    let answer = Rc::new(RefCell::new(PickChoice::Done));
    let reply = answer.clone();
    let app = ui.mount_tree(
        build(QuickPick::new(
            controller.clone(),
            Label::new("Document").key("doc"),
            move |_, _| Ok(reply.borrow().clone()),
        ))
        .key("app"),
    )?;
    Driver::new(&mut ui).settle()?;
    Ok(Picker {
        ui,
        app,
        controller,
        answer,
    })
}

fn pages(count: usize) -> Vec<PickItem> {
    (0..count)
        .map(|index| PickItem::new(format!("p{index}"), format!("Page {index}")))
        .collect()
}

impl Picker {
    fn options(&self) -> TestResult<Option<WidgetId>> {
        let surface = self.ui.find(&self.app, &["surface"])?.ok_or("surface")?;
        Ok(self.ui.find_exposed_part(&surface, "options")?)
    }
    fn semantics(&self) -> TestResult<Semantics> {
        let options = self.options()?.ok_or("no options part")?;
        Ok(support::semantics_of(&self.ui, &options).ok_or("semantics")?)
    }
    fn open(&mut self, items: Vec<PickItem>) -> TestResult {
        self.controller.open(items)?;
        Driver::new(&mut self.ui).settle()?;
        Ok(())
    }
}

#[test]
fn the_result_list_reports_value_scroll_highlight_and_items() -> TestResult {
    let mut p = picker(|pick| pick.list_name("Places"))?;
    p.open(pages(20))?;
    let semantics = p.semantics()?;
    assert_eq!(semantics.role, SemanticRole::ListBox);
    assert_eq!(semantics.name.as_deref(), Some("Places"));
    assert!(matches!(
        semantics.value,
        Some(SemanticValue::Number { value, .. }) if value == 20.0
    ));
    assert!(
        semantics.actions.contains(&SemanticAction::Scroll),
        "it overflows"
    );
    assert_eq!(semantics.items.len(), 20);
    assert!(semantics.items[0].selected, "the best match is highlighted");
    assert_eq!(semantics.items[0].actions, [SemanticAction::Activate]);
    // The highlight is retained by the picker, not read at inspection.
    Driver::new(&mut p.ui).key(KeyCode::ArrowDown)?;
    let items = p.semantics()?.items;
    assert!(!items[0].selected && items[1].selected);
    // A few results fit: no Scroll.
    p.open(pages(2))?;
    assert!(!p.semantics()?.actions.contains(&SemanticAction::Scroll));
    Ok(())
}

#[test]
fn the_result_list_is_a_new_component_after_each_query_or_session() -> TestResult {
    let mut p = picker(|pick| pick)?;
    p.open(vec![
        PickItem::new("a", "Alpha"),
        PickItem::new("b", "Beta"),
        PickItem::new("g", "Gamma"),
    ])?;
    let first = p.options()?.ok_or("options")?;
    // A query matching every result still makes a new list.
    p.ui.text_input("a")?;
    Driver::new(&mut p.ui).settle()?;
    let second = p.options()?.ok_or("options")?;
    assert_ne!(second, first);
    assert_eq!(p.semantics()?.items.len(), 3, "the same results");
    assert!(matches!(p.ui.key(&first), Err(RuntimeError::Unmounted)));
    assert!(matches!(
        Driver::new(&mut p.ui).semantics(&first),
        Err(TestError::Runtime(RuntimeError::Unmounted))
    ));
    // So does a new session.
    p.open(pages(3))?;
    let third = p.options()?.ok_or("options")?;
    assert_ne!(third, second);
    assert!(matches!(p.ui.key(&second), Err(RuntimeError::Unmounted)));
    // A new list has its rectangles once laid out.
    assert!(p.semantics()?.items[0].bounds.is_some());
    Ok(())
}

#[test]
fn a_search_held_across_a_picker_query_or_session_is_stale() -> TestResult {
    let mut p = picker(|pick| pick)?;
    p.open(vec![
        PickItem::new("a", "Alpha"),
        PickItem::new("b", "Alphabet"),
    ])?;
    // One hit per step, so the search is still running when the query or
    // the session changes.
    let query = || -> TestResult<ContentQuery> { Ok(ContentQuery::new("alpha")?.limit(1)?) };
    let mut held = p.ui.content_search(query()?);
    let first = held.step(&p.ui)?;
    assert_eq!(first.hits.len(), 1);
    assert!(!first.done);
    // A query rebuilds the result list: the held search is stale and never
    // reports the replaced list or its rows.
    p.ui.text_input("b")?;
    Driver::new(&mut p.ui).settle()?;
    assert_eq!(held.step(&p.ui), Err(ContentSearchError::Stale));
    // A new search sees the current results only: every hit names the
    // remaining result, its item belongs to the current list and every other
    // hit to the current list's mounted row or its descendants.
    let current = |p: &Picker, text: &str| -> TestResult {
        let options = p.options()?.ok_or("options")?;
        let mut inside = Vec::new();
        let mut pending = p.ui.children(&options)?;
        while let Some(id) = pending.pop() {
            pending.extend(p.ui.children(&id)?);
            inside.push(id);
        }
        let hits = search(&p.ui, query()?)?;
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|hit| hit.text == text), "{hits:?}");
        let items: Vec<&ContentHit> = hits
            .iter()
            .filter(|hit| matches!(hit.field, ContentField::Item { .. }))
            .collect();
        assert_eq!(items.len(), 1, "{hits:?}");
        assert_eq!(items[0].target, ContentTarget::Widget(options));
        for hit in hits.iter().filter(|hit| hit.field == ContentField::Name) {
            match &hit.target {
                ContentTarget::Widget(id) => assert!(inside.contains(id), "{hit:?}"),
                other => panic!("unexpected target {other:?}"),
            }
        }
        Ok(())
    };
    current(&p, "Alphabet")?;
    // So does a new session, with a search held across it.
    let mut held = p.ui.content_search(query()?);
    assert!(!held.step(&p.ui)?.done);
    p.open(vec![PickItem::new("c", "Alpha Centauri")])?;
    assert_eq!(held.step(&p.ui), Err(ContentSearchError::Stale));
    current(&p, "Alpha Centauri")?;
    Ok(())
}

#[test]
fn a_message_keeps_the_result_list() -> TestResult {
    let mut p = picker(|pick| pick)?;
    p.open(pages(3))?;
    let list = p.options()?.ok_or("options")?;
    // A refused choice shows a message above the list.
    *p.answer.borrow_mut() = PickChoice::Refused("Not now".to_owned());
    Driver::new(&mut p.ui).key(KeyCode::Enter)?;
    assert!(
        p.ui.inspect()
            .components
            .iter()
            .any(|component| component.key == "message" && component.visible)
    );
    assert_eq!(p.options()?, Some(list.clone()), "the same list");
    Ok(())
}

#[test]
fn a_translated_list_name_updates_the_same_list() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let controller = QuickPickController::new(&ui);
    let name = ui.signal("Places".to_owned());
    let app = ui.mount_tree(
        QuickPick::new(controller.clone(), Label::new("Doc").key("doc"), |_, _| {
            Ok(PickChoice::Done)
        })
        .list_name(name.read_only())
        .key("app"),
    )?;
    controller.open(pages(3))?;
    Driver::new(&mut ui).settle()?;
    let p = Picker {
        ui,
        app,
        controller,
        answer: Rc::new(RefCell::new(PickChoice::Done)),
    };
    let list = p.options()?.ok_or("options")?;
    assert_eq!(p.semantics()?.name.as_deref(), Some("Places"));
    name.set("Platser".to_owned())?;
    let mut p = p;
    Driver::new(&mut p.ui).settle()?;
    assert_eq!(p.options()?, Some(list), "the same list");
    assert_eq!(p.semantics()?.name.as_deref(), Some("Platser"));
    Ok(())
}

/// A hit as role, field, text, path and visibility.
type Hit<'a> = (
    Option<SemanticRole>,
    ContentField,
    &'a str,
    Option<Vec<String>>,
    bool,
);

#[test]
fn no_results_leave_no_list_and_an_opted_out_picker_is_never_found() -> TestResult {
    let mut p = picker(|pick| pick)?;
    p.open(vec![
        PickItem::new("a", "Alpha"),
        PickItem::new("b", "Beta"),
    ])?;
    // A searchable picker (the default) is found like other content: the
    // result list's item and the mounted row are distinct hits.
    let options = p.options()?.ok_or("the results list")?;
    let hits = search(&p.ui, ContentQuery::new("alpha")?)?;
    let target = |hit: &ContentHit| match &hit.target {
        ContentTarget::Widget(id) => Some(id.clone()),
        _ => None,
    };
    let mounted_row =
        p.ui.children(&options)?
            .into_iter()
            .next()
            .ok_or("a mounted row")?;
    // The complete result in order: the list's item, then the row (a Button
    // named by the result) and its own caption and help labels. Nothing
    // else reports the result: the internal content no longer does.
    let path = |keys: &[&str]| Some(keys.iter().map(|key| (*key).to_owned()).collect());
    let expected: Vec<Hit> = vec![
        (
            Some(SemanticRole::ListBox),
            ContentField::Item { index: 0 },
            "Alpha",
            path(&["app", "surface"]),
            true,
        ),
        (
            Some(SemanticRole::Button),
            ContentField::Name,
            "Alpha",
            path(&["app", "surface", "a"]),
            true,
        ),
        (
            Some(SemanticRole::Label),
            ContentField::Name,
            "Alpha",
            path(&["app", "surface", "a"]),
            true,
        ),
        (
            Some(SemanticRole::Label),
            ContentField::Name,
            "Alpha",
            path(&["app", "surface", "a", "help", "text"]),
            false,
        ),
    ];
    let found: Vec<_> = hits
        .iter()
        .map(|hit| {
            (
                hit.role,
                hit.field,
                hit.text.as_str(),
                hit.path.clone(),
                hit.visible,
            )
        })
        .collect();
    assert_eq!(found, expected);
    // The item targets the list, the row hit the mounted row and the labels
    // the row's own descendants.
    assert_eq!(
        target(&hits[0]),
        Some(options.clone()),
        "the item is the list's"
    );
    assert_eq!(
        target(&hits[1]),
        Some(mounted_row.clone()),
        "the mounted row"
    );
    let mut descendants = Vec::new();
    let mut pending = p.ui.children(&mounted_row)?;
    while let Some(id) = pending.pop() {
        pending.extend(p.ui.children(&id)?);
        descendants.push(id);
    }
    for hit in &hits[2..] {
        let id = target(hit).ok_or("a widget")?;
        assert!(descendants.contains(&id), "{hit:?} is inside the row");
    }
    // One that presents search results opts out: nothing of its own is found.
    let mut quiet = picker(|pick| pick.searchable(false))?;
    quiet.open(vec![PickItem::new("a", "Alpha")])?;
    let hits = search(&quiet.ui, ContentQuery::new("alpha")?)?;
    assert!(hits.is_empty(), "{hits:?}");
    p.ui.text_input("zzz")?;
    Driver::new(&mut p.ui).settle()?;
    assert_eq!(p.options()?, None);
    assert!(
        !p.ui
            .inspect()
            .components
            .iter()
            .any(|component| component.visible
                && component
                    .details
                    .semantics
                    .as_ref()
                    .is_some_and(|semantics| semantics.role == SemanticRole::ListBox)),
        "no ListBox without results"
    );
    Ok(())
}

#[test]
fn the_command_palette_forwards_its_result_list_for_clicks() -> TestResult {
    use pert_ui::commands::{CommandPalette, CommandPaletteController};
    use pert_ui::{CommandScope, CommandSource};
    use std::cell::Cell;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let ran = Rc::new(Cell::new(0));
    for name in ["Open", "Save"] {
        let ran = ran.clone();
        let mark = if name == "Save" { 10 } else { 1 };
        ui.register_command(name, CommandScope::Application, vec![], move |_, _| {
            ran.set(ran.get() + mark);
            Ok(())
        })?;
    }
    let controller = CommandPaletteController::new(&ui);
    let palette = ui.mount_tree(
        CommandPalette::new(controller.clone(), Label::new("Content").key("content"))
            .key("palette"),
    )?;
    Driver::new(&mut ui).settle()?;
    let context = ui.command_context(CommandSource::Programmatic);
    controller.open(&mut ui, &context)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let surface = driver.find(&palette, &["surface"])?;
    let options = driver.exposed_part(&surface, "options")?;
    let save = driver
        .semantics(&options)?
        .items
        .iter()
        .position(|item| item.name.as_deref() == Some("Save"))
        .ok_or("listed")?;
    driver.click_item(&options, save)?;
    driver.settle()?;
    assert_eq!(ran.get(), 10);
    Ok(())
}
