[pert_ui](../../index.html)::[guide](../index.html)

# Module testing Copy item path

[Source](../../../src/pert_ui/guide.rs.html#183)

Expand description

Testing an application: reading what a headless runtime shows, clicks,
text, keys and chords, settling, list rows, background work, the close
policy and a CPU capture.

A headless test builds the application’s content on a
[`Runtime`](../../struct.Runtime.html "struct pert_ui::Runtime"), drives
it through the input the native host delivers and reads what it shows
through inspection. It needs no window, GPU or operating-system service.
One `use` block covers it:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
```

[`testing`](../../testing/index.html "mod pert_ui::testing") holds
[`Driver`](../../testing/struct.Driver.html "struct pert_ui::testing::Driver"),
which clicks a component or an item a component draws itself, such as a
table’s column title, types, presses keys and chords, clears focus and
settles after each step, checks that a window stays quiet over an
interval and lists a list’s rows in order, its
[`TestError`](../../testing/enum.TestError.html "enum pert_ui::testing::TestError"),
and the other names a test writes: the
[`Viewport`](../../struct.Viewport.html "struct pert_ui::Viewport")
every harness sets, key, focus and pointer input by hand, what the
driver’s steps return, semantics, the query names and the
[`QueryResult`](../../struct.QueryResult.html "struct pert_ui::QueryResult")
a query returns, development validation, and the capture names; with
`cpu` it adds
[`CpuRenderer`](../../cpu/struct.CpuRenderer.html "struct pert_ui::cpu::CpuRenderer")
and its
[`CpuRenderError`](../../cpu/enum.CpuRenderError.html "enum pert_ui::cpu::CpuRenderError"),
and with `native` it adds
[`HeadlessApp`](../../testing/struct.HeadlessApp.html "struct pert_ui::testing::HeadlessApp").
The module is closed: every type those names take, return or hold is in
`testing` or the prelude, so a test that writes such a type, such as a
helper’s return type, needs no other import. The exceptions are records
a test reads by field or passes on: the window, event, parameter and
source answers of a query are in
[`inspection`](../../inspection/index.html "mod pert_ui::inspection"),
and a host’s records, such as its physical surface size and key routing,
in [`host`](../../host/index.html "mod pert_ui::host").

## [§](#reading-what-the-runtime-shows)Reading what the runtime shows

A test holds the
[`WidgetId`](../../struct.WidgetId.html "struct pert_ui::WidgetId") its
code mounted, or finds one by key path with
[`Runtime::find`](../../struct.Runtime.html#method.find "method pert_ui::Runtime::find").
What the component shows is one
[`Runtime::query`](../../struct.Runtime.html#method.query "method pert_ui::Runtime::query")
with
[`Selector::id`](../../enum.Selector.html#method.id "associated function pert_ui::Selector::id")
of its
[`Runtime::inspection_id`](../../struct.Runtime.html#method.inspection_id "method pert_ui::Runtime::inspection_id"):
a single lookup, not a search of a whole snapshot.
[`Driver::semantics`](../../testing/struct.Driver.html#method.semantics "method pert_ui::testing::Driver::semantics")
is that query for the
[`Semantics`](../../struct.Semantics.html "struct pert_ui::Semantics").
Static text, a label’s or a button’s, is the semantic name; an editable
or stateful control’s content is the value. Empty text is no name: an
empty label’s `name` is `None`, not `Some("")`.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

/// The semantics of the component at `path` under `root`.
fn semantics_at(
    driver: &Driver<'_>,
    root: &WidgetId,
    path: &[&str],
) -> Result<Semantics, TestError> {
    driver.semantics(&driver.find(root, path)?)
}

/// One property of a component, from a query at full detail.
fn property(
    ui: &Runtime,
    id: &WidgetId,
    name: &str,
) -> Result<Option<InspectValue>, Box<dyn std::error::Error>> {
    let query = InspectionQuery::components(Selector::id(ui.inspection_id(id)?));
    let result: QueryResult = ui.query(&query.detail(Detail::Full));
    let component = result.components.first().ok_or("no such component")?;
    Ok(component.details.properties.get(name).cloned())
}

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let name = ui.signal(String::from("Ada"));
let age = ui.signal(36_i64);
let root = ui.mount_tree(Element::new("form", Column::new([
    Element::new("title", Label::new("Profile")),
    Element::new("name", TextField::new(name.clone())),
    Element::new("age", IntegerInput::new(age.clone()).range(0, 150)?),
])))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;

assert_eq!(semantics_at(&driver, &root, &["title"])?.name.as_deref(), Some("Profile"));
let field = driver.find(&root, &["name"])?;
assert_eq!(driver.semantics(&field)?.value, Some(SemanticValue::Text("Ada".into())));

// A number input's draft is a property: typed, not yet committed.
let age_field = driver.find(&root, &["age"])?;
assert_eq!(semantics_at(&driver, &root, &["age"])?.role, SemanticRole::NumberInput);
driver.replace_text(&age_field, "37")?;
assert_eq!(property(driver.runtime(), &age_field, "dirty")?, Some(InspectValue::Bool(true)));
assert_eq!(age.get()?, 36);

// The same lookup by hand, at any detail.
let id = driver.runtime().inspection_id(&field)?;
let result = driver.runtime().query(
    &InspectionQuery::components(Selector::id(id)).detail(Detail::Geometry),
);
assert!(result.components[0].details.box_model.border_box.is_some());
```

## [§](#clicking)Clicking

[`Driver::click`](../../testing/struct.Driver.html#method.click "method pert_ui::testing::Driver::click")
reveals the target, opening a closed container and scrolling it into
view, then presses and releases the primary button at the middle of its
bounds and settles. A reveal that stops is
[`TestError::NotRevealed`](../../testing/enum.TestError.html#variant.NotRevealed "variant pert_ui::testing::TestError::NotRevealed").
The reveal flushes, so a revealed target has bounds; a target without
them would be the driver’s report of a broken layout invariant,
[`TestError::NotLaidOut`](../../testing/enum.TestError.html#variant.NotLaidOut "variant pert_ui::testing::TestError::NotLaidOut").
Whether the press reaches the target is not checked: a surface covering
it is a result to assert on.

The clock moves only by the settle, so two clicks at one point are 16 ms
apart. That is within
[`ClickCounter::DEFAULT_INTERVAL`](../../struct.ClickCounter.html#associatedconstant.DEFAULT_INTERVAL "associated constant pert_ui::ClickCounter::DEFAULT_INTERVAL")
(500 ms), so a built-in control counts them as a double click, unless a
press series was already running at that point, which the two clicks
then continue; a component with an interval of its own counts by that
interval. Two separate single clicks put
[`Driver::advance`](../../testing/struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
beyond the component’s interval between them:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let clicks = ui.signal(0);
let counter = clicks.clone();
let button = ui.mount_tree(Element::new("add",
    Button::new("Add", move |_| counter.update(|n| *n += 1))))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.click(&button)?;
driver.advance(ClickCounter::DEFAULT_INTERVAL + Duration::from_millis(1))?;
driver.click(&button)?;
assert_eq!(clicks.get()?, 2);
```

A point that is not one component, such as a cell of a canvas the
application draws, takes
[`Runtime::pointer_input`](../../struct.Runtime.html#method.pointer_input "method pert_ui::Runtime::pointer_input")
with
[`PointerEvent`](../../struct.PointerEvent.html "struct pert_ui::PointerEvent")s
at that point; its documentation shows a click built by hand.

## [§](#typing-and-replacing-text)Typing and replacing text

[`Driver::type_text`](../../testing/struct.Driver.html#method.type_text "method pert_ui::testing::Driver::type_text")
types at the caret of the focused component. Focusing an editor does not
move its caret, which is at the start of the text before the first edit,
so typing into a focused field with text inserts before it.
[`Driver::replace_text`](../../testing/struct.Driver.html#method.replace_text "method pert_ui::testing::Driver::replace_text")
focuses the editor, selects all of its text and types the new text, or
presses Backspace for empty text, since empty committed text inserts
nothing. Both write what the editor’s own rules write: a plain field
writes its binding at once, a field with a normalizer or a length limit
holds a draft until Enter or focus loss, and a parameter field holds its
draft under the edit protocol.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let tag = ui.signal(String::from("draft"));
let field = ui.mount_tree(Element::new("tag",
    TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_lowercase()))))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.replace_text(&field, " Reports ")?;
assert_eq!(tag.get()?, "draft"); // a draft until Enter or focus loss
driver.key(KeyCode::Enter)?;
assert_eq!(tag.get()?, "reports");
```

## [§](#in-place-editors)In-place editors

An in-place editor, such as
[`InlineText`](../../struct.InlineText.html "struct pert_ui::InlineText"),
shows its text at rest and edits it in a child it mounts under the key
`"editor"`. A test begins editing on the label, with F2 or Enter through
[`Driver::key`](../../testing/struct.Driver.html#method.key "method pert_ui::testing::Driver::key"),
a double click, or
[`TextCommand::BeginEdit`](../../enum.TextCommand.html#variant.BeginEdit "variant pert_ui::TextCommand::BeginEdit").
Focus then moves to the child and typed text goes there: the child’s
semantic value is the draft, while the label’s stays the saved text
until Enter or focus loss saves the draft. The label is not itself an
editor, so
[`Driver::replace_text`](../../testing/struct.Driver.html#method.replace_text "method pert_ui::testing::Driver::replace_text")
takes the child, and on the label reports
[`TestError::NotAnEditor`](../../testing/enum.TestError.html#variant.NotAnEditor "variant pert_ui::testing::TestError::NotAnEditor").

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let name = ui.signal(String::from("Draft"));
let label = ui.mount_tree(Element::new("name", InlineText::new(name.clone())))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&label)?;
driver.key(KeyCode::F2)?; // begins editing with the text selected
let editor = driver.find(&label, &["editor"])?;
assert_eq!(driver.runtime().focused(), Some(editor.clone()));
driver.type_text("Final")?;
assert_eq!(driver.semantics(&editor)?.value, Some(SemanticValue::Text("Final".into())));
assert_eq!(driver.semantics(&label)?.value, Some(SemanticValue::Text("Draft".into())));
assert!(matches!(driver.replace_text(&label, "Other"), Err(TestError::NotAnEditor { .. })));
driver.replace_text(&editor, "Done")?;
driver.key(KeyCode::Enter)?; // saves the draft; focus returns to the label
assert_eq!(name.get()?, "Done");
assert_eq!(driver.runtime().focused(), Some(label));
```

## [§](#keys-and-chords)Keys and chords

[`Driver::key`](../../testing/struct.Driver.html#method.key "method pert_ui::testing::Driver::key")
presses and releases one named key, and
[`Driver::chord`](../../testing/struct.Driver.html#method.chord "method pert_ui::testing::Driver::chord")
a [`KeyChord`](../../struct.KeyChord.html "struct pert_ui::KeyChord")
with the physical key that produces it, which a character chord does not
name. Both go through
[`Runtime::command_key_input`](../../struct.Runtime.html#method.command_key_input "method pert_ui::Runtime::command_key_input"),
so application commands see them as a user’s keys. They are synthetic:
no text, no composition, and a chord’s modifiers are pressed in a fixed
order, Control, Alt, Shift, then Super, each as its physical key with
the modifiers held reported through
[`Runtime::native_modifiers`](../../struct.Runtime.html#method.native_modifiers "method pert_ui::Runtime::native_modifiers").
A test of a layout, of text a key carries, of a dead key or an input
method builds a
[`CommandKeyInput`](../../struct.CommandKeyInput.html "struct pert_ui::CommandKeyInput")
itself, and a test of one component’s own key handling uses
[`Runtime::key_input`](../../struct.Runtime.html#method.key_input "method pert_ui::Runtime::key_input").

A modifier held across other input, such as Control held while the wheel
zooms, is pressed with `command_key_input` and reported with
`native_modifiers`, as the native host reports it; after a chord,
physical modifier keys alone no longer change the reported modifiers.

The driver’s keys need neutral input: a key held or a modifier reported
through the low-level methods is
[`TestError::InputHeld`](../../testing/enum.TestError.html#variant.InputHeld "variant pert_ui::testing::TestError::InputHeld"),
so the driver never releases a press it did not make. Whatever fails, it
releases every key it pressed.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
let undone = ui.signal(0);
let counter = undone.clone();
ui.register_command("Undo", CommandScope::Application, vec![KeyChord::primary('z')],
    move |_, _| counter.update(|n| *n += 1))?;
let mut driver = Driver::new(&mut ui);
driver.chord(KeyChord::primary('z'), KeyCode::KeyZ)?;
assert_eq!(undone.get()?, 1);
assert!(driver.runtime().keyboard().pressed().next().is_none());
```

## [§](#nothing-focused)Nothing focused

[`Driver::clear_focus`](../../testing/struct.Driver.html#method.clear_focus "method pert_ui::testing::Driver::clear_focus")
is the selected window’s deactivation, as when the user switches to
another application, followed by a settle. It ends what the window’s
focus held: pressed keys and reported modifiers, menus and popovers
(modal dialogs stay), and the focused editor’s edit, which applies its
focus-loss rules, so a valid draft is saved as the [draft
policy](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#draft-policy)
says. Releasing held keys is what a deactivation does, so it needs no
neutral input, and the next action needs no release first. Application
commands still receive chords with nothing focused.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let tag = ui.signal(String::new());
let field = ui.mount_tree(Element::new("tag",
    TextField::new(tag.clone()).normalize(|text| Ok(text.trim().to_owned()))))?;
let saves = ui.signal(0);
let counter = saves.clone();
ui.register_command("Save", CommandScope::Application, vec![KeyChord::primary('s')],
    move |_, _| counter.update(|n| *n += 1))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
driver.type_text(" urgent ")?;
assert_eq!(tag.get()?, ""); // a draft until Enter or focus loss
driver.clear_focus()?;
assert_eq!(tag.get()?, "urgent");
assert_eq!(driver.runtime().focused(), None);
driver.chord(KeyChord::primary('s'), KeyCode::KeyS)?;
assert_eq!(saves.get()?, 1);
```

## [§](#settling)Settling

Input is delivered to its component at once: a plain editor’s binding is
written when `type_text` returns. The tree, derived bindings, dynamic
children and layout show the result after the next flush, which is why
every driver action ends with
[`Driver::settle`](../../testing/struct.Driver.html#method.settle "method pert_ui::testing::Driver::settle"),
one
[`Runtime::flush`](../../struct.Runtime.html#method.flush "method pert_ui::Runtime::flush")
16 ms after the runtime’s clock. One flush settles updates, layout,
focus and hover feedback and the notifications the input requested; it
does not wait for work deferred to a later flush, background results or
timers due later.
[`Driver::advance`](../../testing/struct.Driver.html#method.advance "method pert_ui::testing::Driver::advance")
moves the clock by more in one jump, and missed deadlines produce one
update, so a test of something counted in frames, such as a debounce,
calls `settle` in a loop.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let text = ui.signal(String::new());
let length = text.map(|text| text.len())?;
let field = ui.mount_tree(Element::new("text", TextField::new(text.clone())))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
driver.runtime_mut().text_input("abc")?; // low level: no settle yet
assert_eq!(text.get()?, "abc");
driver.settle()?;
assert_eq!(length.get()?, 3);
```

## [§](#list-rows)List rows

[`Driver::address_children`](../../testing/struct.Driver.html#method.address_children "method pert_ui::testing::Driver::address_children")
lists the components one address segment below a parent, in child order,
each with the segment
[`Driver::find`](../../testing/struct.Driver.html#method.find "method pert_ui::testing::Driver::find")
takes for it. Each row of an
[`Each`](../../struct.Each.html "struct pert_ui::Each") is keyed by its
item’s key, and its address is the `Each` element’s address followed by
that key: the default container that holds the rows is internal
structure and adds no segment, so the `Each`’s address children are its
rows, in list order.
[`Selector::children`](../../enum.Selector.html#method.children "associated function pert_ui::Selector::children")
of the `Each`’s path returns the same rows with their semantics in one
query, in incarnation order rather than list order:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
let root = ui.mount_tree(
    Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str())).key("tasks"),
)?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
tasks.set(vec!["ship".to_owned(), "write".to_owned()])?;
driver.settle()?;
let rows = driver.address_children(&root)?;
let segments: Vec<&str> = rows.iter().map(|row| row.segment.as_str()).collect();
assert_eq!(segments, ["ship", "write"]);
assert_eq!(driver.semantics(&rows[0].id)?.name.as_deref(), Some("ship"));

let result = driver.runtime().query(
    &InspectionQuery::components(Selector::children(["tasks"])).detail(Detail::Semantics),
);
assert_eq!(result.components.len(), 2);
```

A container the application gives with `within` keeps its key in the
rows’ addresses, so the `Each` has one address child, the container, and
the rows are the container’s:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
let root = ui.mount_tree(
    Each::new(&tasks, Clone::clone, |_, task| Label::new(task.as_str()))
        .within(|rows| Column::new(rows).key("rows"))
        .key("tasks"),
)?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
let container = driver.find(&root, &["rows"])?;
assert_eq!(driver.address_children(&root)?[0].id, container);
let segments: Vec<String> = driver.address_children(&container)?
    .into_iter()
    .map(|row| row.segment)
    .collect();
assert_eq!(segments, ["write", "ship"]);
```

A
[`VirtualList`](../../struct.VirtualList.html "struct pert_ui::VirtualList")
mounts only the rows near its viewport, its overscan and the rows it
keeps mounted for focus, a pointer gesture or `keep_mounted`, and its
address children are exactly those, in index order. A row that is not
mounted is not listed: reveal it with the list’s
[`ListController`](../../struct.ListController.html "struct pert_ui::ListController"),
settle, and it is:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

struct Numbers;
impl ListItems for Numbers {
    fn len(&self) -> usize {
        100
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 100).then(|| format!("row-{index}"))
    }
    fn build(&self, index: usize) -> Option<Element> {
        self.key(index).map(|key| Element::new(key, Label::new(format!("Row {index}"))))
    }
}

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let controller = ListController::new(&ui);
let items = ui.signal(ListSource::new(Numbers));
let list = ui.mount_tree(Element::new(
    "numbers",
    VirtualList::new(items.read_only(), 20.0)?.controller(controller.clone()),
))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
let mounted = |driver: &Driver<'_>| -> Result<Vec<String>, TestError> {
    Ok(driver.address_children(&list)?.into_iter().map(|row| row.segment).collect())
};
assert_eq!(mounted(&driver)?[0], "row-0");
assert!(!mounted(&driver)?.contains(&"row-80".to_owned()));
let revealed = controller.reveal("row-80")?;
driver.settle()?;
assert!(matches!(revealed.status(), ListRevealStatus::Mounted(_)));
assert!(mounted(&driver)?.contains(&"row-80".to_owned()));
```

A [`Table`](../../struct.Table.html "struct pert_ui::Table")’s body and
the component that reports edit ends are internal structure, so the
table’s address children are its mounted rows, in source order, with the
same retention as a list’s:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

struct Items(Vec<&'static str>);
impl TableItems for Items {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|name| name.to_lowercase())
    }
    fn cell(&self, index: usize, _column: &str) -> Option<Element> {
        self.0.get(index).map(|name| Element::new("name", Label::new(*name)))
    }
}

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
let state = TableController::new(
    &ui,
    ListSelection::new(&ui, SelectionMode::Single),
    [TableColumn::new("name", "Name")],
    TableSource::new(Items(vec!["Cable", "Mouse", "Laptop"])),
)?;
let table = ui.mount_tree(Element::new("parts", Table::new(&state, 24.0)?))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
let segments: Vec<String> = driver.address_children(&table)?
    .into_iter()
    .map(|row| row.segment)
    .collect();
assert_eq!(segments, ["cable", "mouse", "laptop"]);
```

[`Runtime::children`](../../struct.Runtime.html#method.children "method pert_ui::Runtime::children")
stays the structure walk for hosts and custom components: it lists
structural wrappers too, such as an `Each`’s default container and a
table’s body.

## [§](#tables)Tables

A [`Table`](../../struct.Table.html "struct pert_ui::Table") cell is a
component keyed by its column, below the row, and the content
[`TableItems::cell`](../../trait.TableItems.html#tymethod.cell "method pert_ui::TableItems::cell")
returns is below the cell under its own key, so a cell’s content is at
`<table>/<row key>/<column key>/<content key>`; the table’s [addresses
and
tests](../../struct.Table.html#addresses-and-tests "struct pert_ui::Table")
describe them. Content keyed like its column is allowed, since keys are
unique only among siblings, but
[`Runtime::find_descendant`](../../struct.Runtime.html#method.find_descendant "method pert_ui::Runtime::find_descendant")
below the row then sees the cell and the content and refuses the lookup;
the full address finds the content.

The column titles are the table’s [semantic
items](../../struct.Semantics.html#method.items "method pert_ui::Semantics::items"),
and each has the
[bounds](../../struct.SemanticItem.html#structfield.bounds "field pert_ui::SemanticItem::bounds")
of its visible part of the header, in window coordinates.
[`Driver::click_item`](../../testing/struct.Driver.html#method.click_item "method pert_ui::testing::Driver::click_item")
clicks the middle of one, as a pointer would, so a sortable column asks
for its sort; the keyboard route is Alt with Up or Down on the active
column. The row height comes from the theme’s ControlHeight when a row
holds controls, so development validation reports nothing taller than a
row:

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::rc::Rc;

#[derive(Clone)]
struct Job {
    name: &'static str,
    status: ProgressStatus,
}
struct Jobs(Rc<Vec<Job>>);
impl TableItems for Jobs {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.0.get(index).map(|job| job.name.to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.0.get(index)?;
        Some(match column {
            // Keyed like its column, which the full address tells apart.
            "name" => Label::new(job.name).key("name"),
            "progress" => Row::new([
                ProgressBar::new(job.status).key("bar").with_grow(1),
                Label::new(match job.status.fraction() {
                    Some(done) => format!("{:.0}%", done.get() * 100.0),
                    None => String::new(),
                })
                .key("percent"),
            ])
            .key("cell"),
            _ => return None,
        })
    }
}

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let jobs = Rc::new(vec![
    Job { name: "upload", status: ProgressStatus::of(100, 100) },
    Job { name: "render", status: ProgressStatus::of(42, 100) },
]);
let controller = TableController::new(
    &ui,
    ListSelection::new(&ui, SelectionMode::Single),
    [
        TableColumn::new("name", "Job").width(Length::from(120)).sortable(true),
        TableColumn::new("progress", "Progress").width(Length::from(160)),
    ],
    TableSource::new(Jobs(jobs.clone())),
)?;
let theme = ui.theme();
let row_height = theme.metrics(theme.density()?)?.metric(MetricRole::ControlHeight);
let republish = controller.clone();
let table = Table::new(&controller, row_height)?.on_sort(move |_, sort| {
    let mut sorted = (*jobs).clone();
    sorted.sort_by_key(|job| job.name);
    if sort.direction == SortDirection::Descending {
        sorted.reverse();
    }
    republish.set_source(TableSource::new(Jobs(Rc::new(sorted))))
});
let table = ui.mount_tree(Element::new("jobs", table))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;

let rows = |driver: &Driver<'_>| -> Result<Vec<String>, TestError> {
    Ok(driver.address_children(&table)?.into_iter().map(|row| row.segment).collect())
};
assert_eq!(rows(&driver)?, ["upload", "render"]);
let percent = driver.find(&table, &["render", "progress", "cell", "percent"])?;
assert_eq!(driver.semantics(&percent)?.name.as_deref(), Some("42%"));
let name = driver.find(&table, &["render", "name", "name"])?;
let row = driver.find(&table, &["render"])?;
assert_eq!(
    driver.runtime().find_descendant(&row, "name"),
    Err(RuntimeError::AmbiguousKey("name".into())),
);
assert_eq!(driver.runtime().key(&name)?, "name");

let titles = driver.semantics(&table)?.items;
assert_eq!(titles[0].name.as_deref(), Some("Job"));
let [_, _, width, _] = titles[0].bounds.ok_or("the title is in view")?;
assert_eq!(width, 120.0);
driver.click_item(&table, 0)?;
assert_eq!(rows(&driver)?, ["render", "upload"]);
#[cfg(debug_assertions)]
assert!(driver.runtime().validation_report().issues.is_empty());
```

## [§](#background-work)Background work

[`Driver::wait_until`](../../testing/struct.Driver.html#method.wait_until "method pert_ui::testing::Driver::wait_until")
flushes until a condition holds, sleeping 1 ms of wall time between
rounds, and reports
[`TestError::TimedOut`](../../testing/enum.TestError.html#variant.TimedOut "variant pert_ui::testing::TestError::TimedOut")
after its timeout. It flushes at the same clock time, so work already
due and delivered results run while future deadlines do not. Polling is
the test’s choice; the library’s hosts never poll.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
let total = ui.signal(None::<u64>);
let target = total.clone();
let _task = ui.spawn(|| (1..=100).sum::<u64>(), move |sum| target.set(Some(sum)))?;
let mut driver = Driver::new(&mut ui);
driver.wait_until(Duration::from_secs(5), |_| Ok(total.get()?.is_some()))?;
assert_eq!(total.get()?, Some(5050));
```

## [§](#nothing-scheduled-while-idle)Nothing scheduled while idle

[`Driver::quiet_for`](../../testing/struct.Driver.html#method.quiet_for "method pert_ui::testing::Driver::quiet_for")
checks that a window is idle: it paints the selected window, lets an
interval of runtime time pass in 16 ms frames without painting, and
returns a
[`QuietReport`](../../testing/struct.QuietReport.html "struct pert_ui::testing::QuietReport")
whose `is_idle()` is the usual assertion. It paints first so that damage
from before the call does not count, and it lets time pass in frames
because one jump of the whole interval runs a periodic component once.
The report’s fields say what remains: the components updated during the
interval (`components_updated`), the deadline still scheduled at its end
(`next_deadline`), whether the window needs painting (`needs_paint`),
and whether the baseline could be painted at all (`baseline_painted`,
false while the runtime still had queued work to run).

The check covers the selected window and the work all windows share,
such as interval timers and background results that have arrived;
another window’s own animation does not make it busy, and a test checks
that window by selecting it and calling `quiet_for` again. A background
worker still computing is not seen: its thread is outside the runtime
until its result arrives, so a test of background work waits for the
result with
[`Driver::wait_until`](../../testing/struct.Driver.html#method.wait_until "method pert_ui::testing::Driver::wait_until")
first and then checks that the window is quiet. Choose an interval
longer than the longest period to rule out: a spinner steps every 100 ms
and a focused text field’s caret blinks every 500 ms. A later deadline,
such as a toast’s expiry, still shows in `next_deadline`. A zero
interval checks that nothing is scheduled now without letting time pass.

``` rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
let field = ui.mount_tree(Element::new("name", TextField::new(ui.signal(String::new()))))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
driver.runtime_mut().focus(&field)?;
// The focused field's caret blinks.
let report = driver.quiet_for(Duration::from_secs(1))?;
assert!(!report.is_idle());
assert!(report.next_deadline.is_some());
driver.clear_focus()?;
assert!(driver.quiet_for(Duration::from_secs(1))?.is_idle());
```

## [§](#the-close-policy)The close policy

With the `native` feature, which `desktop` and `desktop-cpu` enable,
[`App::headless`](../../struct.App.html#method.headless "method pert_ui::App::headless")
starts an [`App`](../../struct.App.html "struct pert_ui::App") without
an event loop or window: the viewport is the main window’s configured
size, the initializer runs and the content is laid out.
[`HeadlessApp::admit_close`](../../testing/struct.HeadlessApp.html#method.admit_close "method pert_ui::testing::HeadlessApp::admit_close")
runs a close request’s admission and the
[`App::on_close`](../../struct.App.html#method.on_close "method pert_ui::App::on_close")
policy and returns the decision. Admission releases the focus of every
window the close would discard first, so a focused editor saves a valid
draft by its own focus-loss rules before the policy reads the
application’s state, and a synchronous `KeepOpen` focuses the released
editor again.

The initializer’s value is the [application
state](../../struct.App.html#application-state "struct pert_ui::App")
that `on_close` and the test share:
[`HeadlessApp::state`](../../testing/struct.HeadlessApp.html#method.state "method pert_ui::testing::HeadlessApp::state")
reads it, and
[`HeadlessApp::runtime_and_state_mut`](../../testing/struct.HeadlessApp.html#method.runtime_and_state_mut "method pert_ui::testing::HeadlessApp::runtime_and_state_mut")
lends the runtime and the state at once, so the test drives the runtime
with the handles the state holds. An application’s builder function
returns `App<State>`, so a test starts it with `app().headless()?`, as
`main` runs it with `app().run()`.

``` rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    use pert_ui::testing::*;

    // What the application builds; its close policy and the test read it.
    struct Notes {
        field: WidgetId,
        text: Signal<String>,
    }
    fn app() -> App<Notes> {
        App::new("Notes", |ui| {
            let text = ui.signal(String::new());
            // A length limit makes the field hold a draft until Enter or focus loss.
            let field = ui.mount_tree(Element::new("note",
                TextField::new(text.clone()).max_length(Some(200))))?;
            Ok(Notes { field, text })
        })
        .on_close(|_, notes, _| {
            Ok(if notes.text.get()?.is_empty() {
                NativeCloseAction::Close
            } else {
                NativeCloseAction::KeepOpen
            })
        })
    }

    let mut app = app().headless()?;
    let (ui, notes) = app.runtime_and_state_mut();
    Driver::new(ui).replace_text(&notes.field, "Call the bank")?;
    assert_eq!(notes.text.get()?, ""); // still a draft
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let notes = app.state();
    assert_eq!(notes.text.get()?, "Call the bank"); // saved before the policy ran
    assert_eq!(app.runtime().focused(), Some(notes.field.clone()));
    app.shutdown()?;
    Ok(())
}
```

`admit_close` does not apply the decision. For `Defer`, the test drives
the confirmation and reads the decision signal with
[`Runtime::read_signal`](../../struct.Runtime.html#method.read_signal "method pert_ui::Runtime::read_signal").
Each call runs the policy again.

Admission saves only what a focused editor’s focus loss saves. A draft a
field keeps otherwise, such as a disabled field’s typed text or a
refused draft left in a field that lost focus earlier, is not saved; a
close policy that should keep such a window open counts the unsaved
fields with `on_modified` and a
[`FormFields`](../../struct.FormFields.html "struct pert_ui::FormFields"),
as
[`IntegerInput::on_modified`](../../struct.IntegerInput.html#method.on_modified "method pert_ui::IntegerInput::on_modified")’s
example does, and reads
[`FormSummary::modified`](../../struct.FormSummary.html#method.modified "method pert_ui::FormSummary::modified").
The [settings-form
recipe](../state/index.html#a-settings-form "mod pert_ui::guide::state")
shows such a policy with a Revert that discards the kept drafts.

## [§](#a-capture)A capture

With the `cpu` feature,
[`CpuRenderer`](../../cpu/struct.CpuRenderer.html "struct pert_ui::cpu::CpuRenderer")
renders a capture without a GPU. Declare capture support, request the
capture, flush and let the renderer serve the pending jobs; the
[diagnostics
chapter](../diagnostics/index.html "mod pert_ui::guide::diagnostics")
describes the capture targets. The renderer answers the request on the
test’s own thread, so the test takes the reply with
[`Pending::try_reply`](../../struct.Pending.html#method.try_reply "method pert_ui::Pending::try_reply")
once it has served it:
[`Pending::wait`](../../struct.Pending.html#method.wait "method pert_ui::Pending::wait")
there would block forever on a reply that has not arrived.
[`FrameImage::encode_png`](../../struct.FrameImage.html#method.encode_png "method pert_ui::FrameImage::encode_png")
encodes the image for a file. Cargo gives an integration test a scratch
directory in the compile-time variable `CARGO_TARGET_TMPDIR`; a doctest
has none, so this example falls back to the system’s temporary
directory.

``` rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    use pert_ui::testing::*;

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
    ui.mount_tree(Element::new("title", Label::new("Hello")))?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(
        CaptureTarget::window(&driver.runtime().primary_window()),
    ));
    driver.settle()?;
    // Not served yet: the flush only queued the job, and `wait` would block forever.
    assert!(matches!(pending.try_reply(), Err(WaitError::Timeout)));
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.try_reply()?.value?; // served above
    let image: &FrameImage = &capture.image;
    assert_eq!((image.width(), image.height()), (300, 200));

    let directory = option_env!("CARGO_TARGET_TMPDIR")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from);
    let file = directory.join("hello.png");
    std::fs::write(&file, image.encode_png())?;
    assert!(std::fs::read(&file)?.starts_with(b"\x89PNG"));
    Ok(())
}
```
