Where state lives decides which contract to use. UI-local values are signals
that components read and the runtime tracks. Values the application owns
authoritatively are parameters, edited through gestures that the host observes;
the [authoritative values](crate::guide::authoritative_values) chapter covers
them. Results from other threads arrive through mailboxes, and lossy high-rate data
through live-data sources.

# Choosing a way

Each table's first row is the way to start with; the rows after it serve the
need their last column names.

**Where a control's value comes from**

| Way | Use it when |
|---|---|
| An editing control's `new` constructor with a [`Signal`], such as [`Checkbox::new`] | the value is UI-local state; the control writes it |
| `controlled`: [`Checkbox::controlled`], [`Toggle::controlled`], [`InlineText::controlled`] | the application's model owns the value, as the records behind list rows and table cells do; the control proposes each change to a callback |
| A value at construction: a display control's `new`, such as [`ProgressBar::new`], or a builder such as [`Field::status`] | the control shows the application's value and writes nothing, as a list row's progress does; `bound` ([`ProgressBar::bound`]) or the `_signal` builder follows a binding instead |
| `for_parameter`, such as [`Checkbox::for_parameter`] | the value is authoritative: a host validates edits, keeps revisions and observes gestures ([authoritative values](crate::guide::authoritative_values)) |
| Advanced: a per-row binding from [`Signal::sibling`] | a list row's editing control without a controlled form and without a draft; each description of the rows gives it a new binding |
| Advanced: a retained row component | a list row's control without a controlled form that keeps a draft, such as a text editor; [`ListItems::build`] describes it |

**Work in the background**

| Way | Use it when |
|---|---|
| [`EventContext::spawn`] | a callback starts one unit of work, such as reading a file, and shows its result |
| [`Runtime::spawn`] | code holding the runtime does the same, or a callback must be able to discard the result: it queues this call with [`EventContext::defer`] and keeps the [`BackgroundTask`] |
| [`Runtime::mailbox`] | a long-lived worker thread sends owned values over time |
| Live data: a [`series_bridge`] read as a [`SeriesSource`] | a producer publishes lossy, high-rate data such as levels with bounded work; the only path safe from a time-critical callback |

**Work deferred from a callback**

| Way | Use it when |
|---|---|
| [`EventContext::defer`] | a callback needs the [`Runtime`], such as to open a document or describe new content. The work runs at the next flush even when the callback's surface closes first, such as a dialog's Save button, a menu row, [`Select::on_change`] or a [`QuickPick`] choice; only shutdown cancels it |
| [`Runtime::defer`] | code holding the runtime queues work; it runs with whichever window the flush has selected |
| Advanced: [`EventContext::defer_while_shown`] | a custom component queues work on itself, such as describing its own children again; the work is skipped when the component is hidden or removed first |

Each callback's documentation names its timing class: whether it runs in the
input that caused it, after the flush has settled, at the next flush, at
parameter delivery or whenever a value is evaluated. The
[callback timing](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#callback-timing)
contract defines the classes.

# Signals and controls

A control bound to a [`Signal`] writes it as the user edits, and everything
that reads the signal follows: here a label shows a greeting derived from a
text field's value.

```rust
use std::time::Duration;
use pert_ui::prelude::*;
use pert_ui::{KeyAction, KeyCode};

let mut ui = Runtime::new();
let name = ui.signal(String::from("Ada"));
let greeting = name.read_only().map(|name| format!("Hello, {name}"))?;
let form = Column::new([
    TextField::new(&name).key("name"),
    Label::new(greeting.clone()).key("greeting"),
]);
let root = ui.mount_tree(form.key("form"))?;
ui.flush(Duration::ZERO)?;
let field = ui.find(&root, &["name"])?.expect("the name field");
ui.focus(&field)?;
ui.key_input(KeyCode::End, KeyAction::Press)?;
ui.text_input(" Lovelace")?;
assert_eq!(name.get()?, "Ada Lovelace"); // the field wrote its binding
ui.flush(Duration::ZERO)?;
assert_eq!(greeting.get()?, "Hello, Ada Lovelace");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The field writes its binding at once. The derived binding and the label follow
at the next flush, where only the components that read a changed binding
update, and several writes before a flush coalesce. A hidden component keeps
its change pending without being updated until it is shown.

A value derived from several bindings is one [`ReadSignal::combine`] over a
tuple of them, such as `ReadSignal::combine((&name, &port), |(name, port)|
format!("{name}:{port}"))?`. It evaluates at most once per batch of writes,
however many of its inputs changed, and an equal result updates no component
that reads it; a binding derived from it in turn still evaluates again and
filters its own equal result.

A signal's value type needs [`PartialEq`] to be written, since a write that
leaves the value equal notifies nobody, and [`Clone`] to be read, since
[`ReadSignal::get`] returns a copy. Creating a signal needs neither.

This example drives the runtime directly. The default native [`App`] supplies
elapsed time and waits for work and deadlines automatically; custom hosts
drive the same API. A custom component that reads bindings itself, with its
own retained data, painting and timers, is in the [custom widgets
chapter](crate::guide::custom_widgets#a-component-that-follows-a-binding).

# Worker results and live data

Occasional work that must not block the interface, such as reading or writing
a file, runs through [`Runtime::spawn`]: the work runs on a new thread and its
result reaches a delivery callback on the UI thread, where it may write
bindings. A button or component callback receives an [`EventContext`], not the
runtime, so it starts the work with [`EventContext::spawn`], which queues the
start for the beginning of the next flush.

```rust
use pert_ui::prelude::*;

/// A status line and a button that counts words in the background.
fn build(ui: &mut Runtime) -> Result<(Signal<String>, WidgetId), RuntimeError> {
    let status = ui.signal("Ready".to_owned());
    let shown = status.clone();
    let load = Button::new("Count words", move |cx| {
        shown.set("Counting".to_owned())?;
        // Real work would read a file; the result carries any failure.
        let text = String::from("one two three");
        let shown = shown.clone();
        cx.spawn(
            move || text.split_whitespace().count(),
            move |words| shown.set(format!("{words} words")),
        )?;
        Ok(())
    });
    let button = ui.mount_tree(load.key("load"))?;
    Ok((status, button))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut ui = Runtime::new();
    let (status, button) = build(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    // A native host flushes when the result wakes it; a headless one polls.
    while status.get()? == "Counting" {
        std::thread::sleep(Duration::from_millis(1));
        ui.flush(Duration::ZERO)?;
    }
    assert_eq!(status.get()?, "3 words");
    Ok(())
}
```

The work receives no UI handles. The start belongs to the application, as
work queued with [`EventContext::defer`] does: it runs at the next flush
whatever happens to the button or its window, as when a Save button closes
its dialog, and only shutdown cancels it before it starts; the receipt reports
that start, not the result. To be able to discard a result that has not
arrived, queue [`Runtime::spawn`] with [`EventContext::defer`] and keep the
[`BackgroundTask`] it returns.
[`Runtime::spawn`] lists the limits and what happens to a result after
cancellation or shutdown.

A long-lived worker thread hands owned results to the UI thread through a
[`Runtime::mailbox`]: its [`UiSender`] is [`Send`], a full or closed mailbox returns
the undelivered value in [`SendError`], and the handler runs on the UI thread,
where it may write bindings. Mailboxes take locks and can wake the event loop, so
they are not endpoints for time-critical producer callbacks.

High-rate, lossy data such as levels and traces travels separately. A
`series_bridge` splits into a `Send` producer [`SeriesEndpoint`], which publishes
with bounded work and no allocation into a ring preallocated when the bridge is
created, and a
[`SeriesConsumer`] that [`Runtime::series_source`] turns into a UI-thread
[`SeriesSource`]. A visible reader drains it at the window's presentation rate
([`Delivery::Polled`]), or when a [`LiveWaker`] signals new data
([`Delivery::Notified`]); hidden readers schedule nothing. Components read the
latest item with its [`Freshness`] as [`Telemetry`], and older items as a window.
[`LevelMeter`] and [`Trace`] are the built-in readers; custom components use the
same `SeriesSource` API.

# A settings form

A settings form edits a copy of what the application has applied: Apply is
offered while the copy differs, Revert while anything is unsaved, and the
window stays open over unsaved work. Each editor sits in its own [`Field`]
bound to a [`FormFields`] handle with [`Field::form`], and reports a refused
save to the handle's status and a kept draft to
[`FieldHandle::set_modified`]. Whether the copy differs is one
[`ReadSignal::combine`] over the fields and the applied record; Revert's
availability adds [`FormSummary::modified`], so Revert is offered for a
refused draft even while the bound value is the applied one. Apply reads the
values, writes them on a worker with [`EventContext::spawn`] and records them
as applied when the result arrives.

Revert writes the applied values back, clears the statuses the application
published and calls [`FormFields::discard_drafts`]. The write alone is not
enough: writing the value a field already shows changes nothing and keeps its
draft, as the
[draft policy](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#draft-policy)
says. The click on Revert first takes focus, so a valid draft is saved by
focus loss and then overwritten by the write, and a refused one is kept until
the write ends it (a changed value) or the discard does (an equal one). The
statuses are the application's own: a draft the write ended reports no end
of its refusal, so Revert clears them itself. The close policy reads the same
[application state][app-state] through [`App::on_close`].

```rust
# #[cfg(feature = "native")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;

    /// What the settings file holds.
    #[derive(Clone, Debug, PartialEq)]
    struct Applied {
        name: String,
        copies: i64,
    }
    /// What the initializer builds; the close policy and the test read it.
    struct Settings {
        name: Signal<String>,
        copies: Signal<i64>,
        applied: Signal<Applied>,
        form: FormFields,
        copies_field: FieldHandle,
        changed: ReadSignal<bool>,
        root: WidgetId,
    }

    fn build(ui: &mut Runtime) -> Result<Settings, RuntimeError> {
        let saved = Applied { name: "Office".to_owned(), copies: 1 };
        let name = ui.signal(saved.name.clone());
        let copies = ui.signal(saved.copies);
        let applied = ui.signal(saved);
        let form = FormFields::new(ui);
        let (name_field, copies_field) = (form.field("name")?, form.field("copies")?);
        let changed = ReadSignal::combine((&name, &copies, &applied), |(name, copies, applied)| {
            *name != applied.name || *copies != applied.copies
        })?;
        // A kept draft is unsaved too, even over the applied value.
        let unsaved = ReadSignal::combine((&changed, &form.summary()), |(changed, summary)| {
            *changed || summary.modified() > 0
        })?;

        let (status, mark) = (name_field.clone(), name_field.clone());
        let name_input = TextField::new(&name)
            .normalize(|text| match text.trim() {
                "" => Err(InputRejection::new("Enter a name")),
                trimmed => Ok(trimmed.to_owned()),
            })
            .on_rejection(move |_, refusal| match refusal {
                Some(refusal) => status.set(FieldStatus::error(refusal.to_string())),
                None => status.clear().map(|_| ()),
            })
            .on_modified(move |_, modified| mark.set_modified(modified).map(|_| ()));
        let (status, mark) = (copies_field.clone(), copies_field.clone());
        let copies_input = IntegerInput::new(&copies)
            .range(1, 99)?
            .on_rejection(move |_, refusal| match refusal {
                Some(refusal) => status.set(FieldStatus::error(refusal.to_string())),
                None => status.clear().map(|_| ()),
            })
            .on_modified(move |_, modified| mark.set_modified(modified).map(|_| ()));

        let (values, record) = ((name.clone(), copies.clone()), applied.clone());
        let apply = Button::new("Apply", move |cx| {
            let values = Applied { name: values.0.get()?, copies: values.1.get()? };
            let record = record.clone();
            cx.spawn(
                // The worker would write the settings file here.
                move || values,
                move |values| record.set(values),
            )?;
            Ok(())
        })
        .enabled(&changed);
        let (values, record) = ((name.clone(), copies.clone()), applied.clone());
        let (handles, drafts) = ((name_field.clone(), copies_field.clone()), form.clone());
        let revert = Button::new("Revert", move |cx| {
            let saved = record.get()?;
            // A value a field already shows keeps its draft,
            values.0.set(saved.name)?;
            values.1.set(saved.copies)?;
            // the statuses are the application's own,
            handles.0.clear()?;
            handles.1.clear()?;
            // so the drafts are discarded too.
            drafts.discard_drafts(cx)?;
            Ok(())
        })
        .enabled(&unsaved);

        let root = ui.mount_tree(Column::new([
            Field::new(name_input.key("name")).label("Printer").form(&name_field).key("name-field"),
            Field::new(copies_input.key("copies")).label("Copies").form(&copies_field).key("copies-field"),
            Row::new([apply.key("apply"), revert.key("revert")]).key("buttons"),
        ]).key("settings"))?;
        Ok(Settings { name, copies, applied, form, copies_field, changed, root })
    }

    fn app() -> App<Settings> {
        App::new("Settings", |ui| Ok(build(ui)?)).on_close(|_, settings, _| {
            let unsaved = settings.changed.get()? || settings.form.summary().get()?.modified() > 0;
            Ok(if unsaved { NativeCloseAction::KeepOpen } else { NativeCloseAction::Close })
        })
    }

    // The test.
    use pert_ui::testing::*;
    use std::time::Duration;

    let mut app = app().headless()?;
    let main = app.main_window();
    let (ui, settings) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    let root = settings.root.clone();
    let name = driver.find(&root, &["name-field", "name"])?;
    let copies = driver.find(&root, &["copies-field", "copies"])?;
    let apply = driver.find(&root, &["buttons", "apply"])?;
    let revert = driver.find(&root, &["buttons", "revert"])?;

    // A refused draft over the applied value: the window stays open.
    driver.replace_text(&copies, "150")?;
    driver.key(KeyCode::Enter)?;
    assert!(driver.semantics(&copies)?.invalid);
    assert!(settings.copies_field.status().get()?.is_error());
    assert_eq!(settings.form.summary().get()?.modified(), 1);
    assert_eq!(settings.copies.get()?, 1);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::KeepOpen));
    let (ui, settings) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.click(&revert)?;
    assert!(!driver.semantics(&copies)?.invalid);
    assert_eq!(settings.copies_field.status().get()?, FieldStatus::Normal);
    assert_eq!(settings.form.summary().get()?.modified(), 0);
    assert_eq!(settings.copies.get()?, 1);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));

    // A refused draft over a changed value: the write ends it.
    let (ui, settings) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.replace_text(&copies, "8")?;
    driver.key(KeyCode::Enter)?;
    assert!(settings.changed.get()?);
    driver.replace_text(&copies, "150")?;
    driver.key(KeyCode::Enter)?;
    assert!(settings.copies_field.status().get()?.is_error());
    driver.click(&revert)?;
    assert_eq!(settings.copies.get()?, 1);
    assert!(!driver.semantics(&copies)?.invalid);
    assert_eq!(settings.copies_field.status().get()?, FieldStatus::Normal);
    assert_eq!(settings.form.summary().get()?.modified(), 0);
    assert!(matches!(app.admit_close(&main)?, NativeCloseAction::Close));

    // Apply records the values when the worker's result arrives.
    let (ui, settings) = app.runtime_and_state_mut();
    let mut driver = Driver::new(ui);
    driver.replace_text(&name, "Studio")?;
    driver.key(KeyCode::Enter)?;
    assert!(settings.changed.get()?);
    driver.click(&apply)?;
    driver.wait_until(Duration::from_secs(5), |_| Ok(settings.applied.get()?.name == "Studio"))?;
    assert!(!settings.changed.get()?);
    app.shutdown()?;
    Ok(())
}
# #[cfg(not(feature = "native"))]
# fn main() {}
```

# Bounded updates and recovery

[`Runtime::flush`] limits update/settlement feedback to [`Runtime::MAX_FLUSH_WAVES`],
including internal focus/hover/layout follow-ups. [`RuntimeError::UpdateLimit`]
preserves pending work for repair and explicit retry; development validation names
pending components. This does not bound the duration of a user callback or tree depth.
Custom hosts can call [`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input) after a failed operation to clear
held actions and capture while preserving focus and numeric drafts without saving.

With the default desktop feature, component input/update/drawing errors are reported
to stderr and retried after 100/200/400 ms. A fourth consecutive failure returns
from [`App::run`]; only successful presentation resets the budget. [`App::on_error`]
can repair state and choose [`ErrorAction::Retry`] or [`ErrorAction::Exit`] on the UI
thread. Failed input actions are never replayed. Native editing events during
backoff are discarded; close and lifecycle events remain available. Closing during
backoff returns its pending error. Initialization/native/GPU failures bypass this
callback and keep their separate recovery rules.
