A headless test builds the application's content on a [`Runtime`], drives it
through the input the native host delivers and reads what it shows through
inspection. It needs no window, GPU or operating-system service. One `use` block
covers it:

```rust
use pert_ui::prelude::*;
use pert_ui::testing::*;
```

[`testing`](crate::testing) holds [`Driver`](crate::testing::Driver), which
clicks, types, presses keys and chords and settles after each step, its
[`TestError`](crate::testing::TestError), and the other names a test writes:
the [`Viewport`] every harness sets, key, focus and pointer input by hand,
[`SemanticValue`], [`ClickCounter`], and the inspection-query and capture
names; with `cpu` it adds [`CpuRenderer`] and with `native` it
adds [`HeadlessApp`].

# Reading what the runtime shows

A test holds the [`WidgetId`] its code mounted, or finds one by key path with
[`Runtime::find`]. What the component shows is one [`Runtime::query`] with
[`Selector::id`] of its [`Runtime::inspection_id`]: a single lookup, not a
search of a whole snapshot. [`Driver::semantics`](crate::testing::Driver::semantics)
is that query for the [`Semantics`]. Static text, a label's or a button's, is
the semantic name; an editable or stateful control's content is the value.
Empty text is no name: an empty label's `name` is `None`, not `Some("")`.

```rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
let name = ui.signal(String::from("Ada"));
let root = ui.mount_tree(Element::new("form", Column::new([
    Element::new("title", Label::new("Profile")),
    Element::new("name", TextField::new(name.clone())),
])))?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;

let title = driver.find(&root, &["title"])?;
assert_eq!(driver.semantics(&title)?.name.as_deref(), Some("Profile"));
let field = driver.find(&root, &["name"])?;
assert_eq!(driver.semantics(&field)?.value, Some(SemanticValue::Text("Ada".into())));

// The same lookup by hand, at any detail.
let id = driver.runtime().inspection_id(&field)?;
let result = driver.runtime().query(
    &InspectionQuery::components(Selector::id(id)).detail(Detail::Geometry),
);
assert!(result.components[0].details.box_model.border_box.is_some());
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Clicking

[`Driver::click`](crate::testing::Driver::click) reveals the target, opening a
closed container and scrolling it into view, then presses and releases the
primary button at the middle of its bounds and settles. A reveal that stops is
[`TestError::NotRevealed`](crate::testing::TestError::NotRevealed). The reveal
flushes, so a revealed target has bounds; a target without them would be the
driver's report of a broken layout invariant,
[`TestError::NotLaidOut`](crate::testing::TestError::NotLaidOut).
Whether the press reaches the target is not checked: a surface covering it is a
result to assert on.

The clock moves only by the settle, so two clicks at one point are 16 ms apart.
That is within [`ClickCounter::DEFAULT_INTERVAL`] (500 ms), so a built-in
control counts them as a double click, unless a press series was already
running at that point, which the two clicks then continue; a component with an
interval of its own counts by that interval. Two separate single clicks put
[`Driver::advance`](crate::testing::Driver::advance) beyond the component's
interval between them:

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

A point that is not one component, such as a cell of a canvas the application
draws, takes [`Runtime::pointer_input`] with [`PointerEvent`]s at that point;
its documentation shows a click built by hand.

# Typing and replacing text

[`Driver::type_text`](crate::testing::Driver::type_text) types at the caret of
the focused component. Focusing an editor does not move its caret, which is at
the start of the text before the first edit, so typing into a focused field
with text inserts before it.
[`Driver::replace_text`](crate::testing::Driver::replace_text) focuses the
editor, selects all of its text and types the new text, or presses Backspace
for empty text, since empty committed text inserts nothing. Both write what the
editor's own rules write: a plain field writes its binding at once, a field
with a normalizer or a length limit holds a draft until Enter or focus loss,
and a parameter field holds its draft under the edit protocol.

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

# In-place editors

An in-place editor, such as [`InlineText`], shows its text at rest and edits
it in a child it mounts under the key `"editor"`. A test begins editing on the
label, with F2 or Enter through [`Driver::key`](crate::testing::Driver::key),
a double click, or [`TextCommand::BeginEdit`]. Focus then moves to the child
and typed text goes there: the child's semantic value is the draft, while the
label's stays the saved text until Enter or focus loss saves the draft. The
label is not itself an editor, so
[`Driver::replace_text`](crate::testing::Driver::replace_text) takes the
child, and on the label reports
[`TestError::NotAnEditor`](crate::testing::TestError::NotAnEditor).

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Keys and chords

[`Driver::key`](crate::testing::Driver::key) presses and releases one named key,
and [`Driver::chord`](crate::testing::Driver::chord) a [`KeyChord`] with the
physical key that produces it, which a character chord does not name. Both go
through [`Runtime::command_key_input`], so application commands see them as a
user's keys. They are synthetic: no text, no composition, and a chord's
modifiers are pressed in a fixed order, Control, Alt, Shift, then Super, each
as its physical key with the modifiers held reported through
[`Runtime::native_modifiers`]. A test of a layout, of text a key carries, of a
dead key or an input method builds a [`CommandKeyInput`] itself, and a test of
one component's own key handling uses [`Runtime::key_input`].

A modifier held across other input, such as Control held while the wheel
zooms, is pressed with `command_key_input` and reported with
`native_modifiers`, as the native host reports it; after a chord, physical
modifier keys alone no longer change the reported modifiers.

The driver's keys need neutral input: a key held or a modifier reported through
the low-level methods is
[`TestError::InputHeld`](crate::testing::TestError::InputHeld), so the driver
never releases a press it did not make. Whatever fails, it releases every key it
pressed.

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Settling

Input is delivered to its component at once: a plain editor's binding is
written when `type_text` returns. The tree, derived bindings, dynamic children
and layout show the result after the next flush, which is why every driver
action ends with [`Driver::settle`](crate::testing::Driver::settle), one
[`Runtime::flush`] 16 ms after the runtime's clock. One flush settles updates,
layout, focus and hover feedback and the notifications the input requested; it
does not wait for work deferred to a later flush, background results or timers
due later. [`Driver::advance`](crate::testing::Driver::advance) moves the clock
by more in one jump, and missed deadlines produce one update, so a test of
something counted in frames, such as a debounce, calls `settle` in a loop.

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

# List rows

Each row of an [`Each`] is keyed by its item's key, not by the key its
element had, and its address is the `Each` element's address followed by
that key: the default container that holds the rows is internal structure
and adds no segment. [`Driver::find`](crate::testing::Driver::find) takes the
row's key below the `Each`, and [`Selector::children`] of the `Each`'s path
returns the rows with their semantics in one query:

```rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
let root = ui.mount_tree(
    // The row's own key, "task", is replaced by the item's key.
    Each::new(&tasks, |task| task.clone(), |_, task| Label::new(task.as_str()).key("task"))
        .key("tasks"),
)?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
let ship = driver.find(&root, &["ship"])?;
assert_eq!(driver.semantics(&ship)?.name.as_deref(), Some("ship"));

let result = driver.runtime().query(
    &InspectionQuery::components(Selector::children(["tasks"])).detail(Detail::Semantics),
);
assert_eq!(result.components.len(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

For the rows in display order, name the container with `within`: a
container the application gives keeps its key in the rows' addresses, so
[`Runtime::children`] of that container lists the mounted rows in order:

```rust
use pert_ui::prelude::*;
use pert_ui::testing::*;

let mut ui = Runtime::new();
ui.set_viewport(Viewport::new(300.0, 200.0, 1.0)?)?;
let tasks = ui.signal(vec!["write".to_owned(), "ship".to_owned()]);
let root = ui.mount_tree(
    Each::new(&tasks, |task| task.clone(), |_, task| Label::new(task.as_str()).key("task"))
        .within(|rows| Column::new(rows).key("rows"))
        .key("tasks"),
)?;
let mut driver = Driver::new(&mut ui);
driver.settle()?;
let rows = driver.find(&root, &["rows"])?;
let keys: Vec<String> = driver.runtime().children(&rows)?.iter()
    .map(|row| driver.runtime().key(row).map(str::to_owned))
    .collect::<Result<_, _>>()?;
assert_eq!(keys, ["write", "ship"]);
assert!(driver.find(&root, &["rows", "ship"]).is_ok());
# Ok::<(), Box<dyn std::error::Error>>(())
```

# Background work

[`Driver::wait_until`](crate::testing::Driver::wait_until) flushes until a
condition holds, sleeping 1 ms of wall time between rounds, and reports
[`TestError::TimedOut`](crate::testing::TestError::TimedOut) after its timeout.
It flushes at the same clock time, so work already due and delivered results
run while future deadlines do not. Polling is the test's choice; the library's
hosts never poll.

```rust
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
# Ok::<(), Box<dyn std::error::Error>>(())
```

# The close policy

With the `native` feature, which `desktop` and `desktop-cpu` enable,
[`App::headless`] starts an [`App`] without an event loop or window: the
viewport is the main window's configured size, the initializer runs and the
content is laid out. [`HeadlessApp::admit_close`] runs a close request's
admission and the [`App::on_close`] policy and returns the decision. Admission
releases the focus of every window the close would discard first, so a focused
editor saves a valid draft by its own focus-loss rules before the policy reads
the application's state, and a synchronous `KeepOpen` focuses the released
editor again.

```rust
# #[cfg(feature = "native")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    use pert_ui::testing::*;
    use std::{cell::RefCell, rc::Rc};

    // What the application builds; its close policy and the test read it.
    let built: Rc<RefCell<Option<(WidgetId, Signal<String>)>>> = Rc::default();
    let (slot, policy) = (built.clone(), built.clone());
    let mut app = App::new("Notes", move |ui| {
        let text = ui.signal(String::new());
        // A length limit makes the field hold a draft until Enter or focus loss.
        let field = ui.mount_tree(Element::new("note",
            TextField::new(text.clone()).max_length(Some(200))))?;
        *slot.borrow_mut() = Some((field, text));
        Ok(())
    })
    .on_close(move |_, _| {
        let text = policy.borrow().as_ref().map(|(_, text)| text.clone());
        let unsaved = match text {
            Some(text) => !text.get()?.is_empty(),
            None => false,
        };
        Ok(if unsaved { NativeCloseAction::KeepOpen } else { NativeCloseAction::Close })
    })
    .headless()?;
    let (field, text) = built.borrow().clone().ok_or("not built")?;

    Driver::new(app.runtime_mut()).replace_text(&field, "Call the bank")?;
    assert_eq!(text.get()?, ""); // still a draft
    let main = app.main_window();
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    assert_eq!(text.get()?, "Call the bank"); // saved before the policy ran
    assert_eq!(app.runtime().focused(), Some(field));
    app.shutdown()?;
    Ok(())
}
# #[cfg(not(feature = "native"))]
# fn main() {}
```

`admit_close` does not apply the decision. For `Defer`, the test drives the
confirmation and reads the decision signal with [`Runtime::read_signal`]. Each
call runs the policy again.

Admission saves only what a focused editor's focus loss saves. A draft a field
keeps otherwise, such as a disabled field's typed text or a refused draft left
in a field that lost focus earlier, is not saved; a close policy that should
keep such a window open counts the unsaved fields with `on_modified` and a
[`FormFields`], as [`IntegerInput::on_modified`]'s example does, and reads
[`FormSummary::modified`].

# A capture

With the `cpu` feature, [`CpuRenderer`] renders a capture without a GPU. Declare
capture support, request the capture, flush and let the renderer serve the
pending jobs; the [diagnostics chapter](crate::guide::diagnostics) describes the
capture targets. The renderer answers the request on the test's own thread, so
the test takes the reply with [`Pending::try_reply`] once it has served it:
[`Pending::wait`] there would block forever on a reply that has not arrived.
[`FrameImage::encode_png`] encodes the image for a file. Cargo gives an
integration test a scratch directory in the compile-time variable
`CARGO_TARGET_TMPDIR`; a doctest has none, so this example falls back to the
system's temporary directory.

```rust
# #[cfg(feature = "cpu")]
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
# #[cfg(not(feature = "cpu"))]
# fn main() {}
```
