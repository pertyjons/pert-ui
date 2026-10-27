//! Value-driven children through the facade: `Dynamic`, `Show` and `Each`
//! describe children from a binding, retain children whose key and type stay,
//! release the rest, and follow a new description of themselves.
use crate::support;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn runtime() -> Result<Runtime, Box<dyn std::error::Error>> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    Ok(ui)
}

/// The semantic name of the one shown component keyed `key`.
fn name_of(ui: &Runtime, key: &str) -> Option<String> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key && component.visible)
        .and_then(|component| component.details.semantics)
        .and_then(|semantics| semantics.name)
}

#[test]
fn a_child_that_keeps_its_key_and_type_is_retained_with_its_focus() -> TestResult {
    let mut ui = runtime()?;
    let count = ui.signal(1);
    let root = ui.mount_tree(
        Dynamic::new(&count, |_, count| {
            Button::new(format!("Clicked {count} times"), |_| Ok(())).key("button")
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    let button = support::key_path(&ui, &root, &["button"]).ok_or("the button is mounted")?;
    ui.focus(&button)?;
    count.set(2)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        support::key_path(&ui, &root, &["button"]),
        Some(button.clone())
    );
    assert_eq!(ui.focused(), Some(button.clone()));
    assert_eq!(name_of(&ui, "button").as_deref(), Some("Clicked 2 times"));
    Ok(())
}

#[test]
fn a_child_with_another_key_replaces_the_old_one_and_a_type_change_is_refused() -> TestResult {
    let mut ui = runtime()?;
    let mode = ui.signal(0);
    let root = ui.mount_tree(
        Dynamic::new(&mode, |_, mode| match mode {
            0 => Label::new("Zero").key("a"),
            1 => Label::new("One").key("b"),
            2 => Button::new("Two", |_| Ok(())).key("b"),
            _ => Button::new("Three", |_| Ok(())).key("c"),
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    let zero = support::key_path(&ui, &root, &["a"]).ok_or("mounted")?;
    mode.set(1)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.key(&zero).is_err(), "the old key unmounted");
    let one = support::key_path(&ui, &root, &["b"]).ok_or("mounted")?;
    // A kept key cannot change its component type: the flush fails and the
    // previous child stays until a valid description arrives.
    mode.set(2)?;
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::ChildTypeChanged(_))
    ));
    assert_eq!(support::key_path(&ui, &root, &["b"]), Some(one.clone()));
    mode.set(3)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.key(&one).is_err());
    assert!(support::key_path(&ui, &root, &["c"]).is_some());
    Ok(())
}

#[test]
fn an_unchanged_value_describes_nothing_again() -> TestResult {
    let mut ui = runtime()?;
    let value = ui.signal(1);
    let built = Rc::new(Cell::new(0));
    let count = built.clone();
    ui.mount_tree(
        Dynamic::new(&value, move |_, value| {
            count.set(count.get() + 1);
            Label::new(value.to_string()).key("value")
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(built.get(), 1);
    value.set(1)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(built.get(), 1);
    value.set(2)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(built.get(), 2);
    Ok(())
}

#[test]
fn a_new_description_replaces_the_build_callback_and_the_binding() -> TestResult {
    let mut ui = runtime()?;
    let (first, second) = (ui.signal(1), ui.signal(10));
    let describe = |source: &Signal<i32>, prefix: &'static str| {
        vec![
            Dynamic::new(source, move |_, value| {
                Label::new(format!("{prefix} {value}")).key("value")
            })
            .key("dynamic"),
        ]
    };
    let root = ui.mount_tree(Column::new(describe(&first, "Old")).key("root"))?;
    ui.flush(Duration::ZERO)?;
    let value = support::key_path(&ui, &root, &["dynamic", "value"]).ok_or("mounted")?;
    // The same value with a new callback still describes the child again.
    ui.reconcile_children(&root, describe(&first, "New"))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(name_of(&ui, "value").as_deref(), Some("New 1"));
    // A new binding is followed, and the old one no longer is.
    ui.reconcile_children(&root, describe(&second, "New"))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(name_of(&ui, "value").as_deref(), Some("New 10"));
    first.set(2)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(name_of(&ui, "value").as_deref(), Some("New 10"));
    assert_eq!(
        support::key_path(&ui, &root, &["dynamic", "value"]),
        Some(value)
    );
    Ok(())
}

#[test]
fn redescribing_a_dynamic_matches_a_fresh_mount() -> TestResult {
    let mut ui = runtime()?;
    let (before, after) = (
        ui.signal("Before".to_owned()),
        ui.signal("After".to_owned()),
    );
    let dynamic = |source: &Signal<String>| {
        Dynamic::new(source, |_, text| Label::new(text.as_str()).key("caption")).key("dynamic")
    };
    check_redescription(&mut ui, dynamic(&before), dynamic(&after), dynamic(&after))?;
    Ok(())
}

/// Counts its updates; reads `source` so a write to it would update it.
struct Reader {
    source: ReadSignal<i32>,
    updates: Rc<Cell<u32>>,
}
impl Component for Reader {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read(&self.source)?;
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
}

#[test]
fn show_unmounts_a_hidden_child_and_releases_its_subscriptions() -> TestResult {
    let mut ui = runtime()?;
    let shown = ui.signal(true);
    let source = ui.signal(0);
    let updates = Rc::new(Cell::new(0));
    let (read, counted) = (source.read_only(), updates.clone());
    let column = ui.mount_tree(
        Column::new([Show::new(&shown, move |_| {
            Reader {
                source: read.clone(),
                updates: counted.clone(),
            }
            .key("reader")
        })
        .key("show")])
        .key("column"),
    )?;
    ui.flush(Duration::ZERO)?;
    let root = support::key_path(&ui, &column, &["show"]).ok_or("mounted")?;
    let reader = support::key_path(&ui, &root, &["reader"]).ok_or("mounted")?;
    source.set(1)?;
    ui.flush(Duration::ZERO)?;
    let while_shown = updates.get();
    assert!(while_shown >= 2);
    shown.set(false)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.key(&reader).is_err(), "hiding unmounts the child");
    assert!(ui.children(&root)?.is_empty());
    let bounds = ui.bounds(&root)?.ok_or("laid out")?;
    assert_eq!((bounds.width(), bounds.height()), (0.0, 0.0));
    source.set(2)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        updates.get(),
        while_shown,
        "the unmounted reader is not updated"
    );
    shown.set(true)?;
    ui.flush(Duration::ZERO)?;
    let again = support::key_path(&ui, &root, &["reader"]).ok_or("mounted again")?;
    assert_ne!(again, reader, "showing again builds a fresh child");
    Ok(())
}

#[test]
fn each_retains_items_by_key_across_reorder_and_removal() -> TestResult {
    let mut ui = runtime()?;
    let items = ui.signal(vec![1, 2, 3]);
    let root = ui.mount_tree(
        Each::new(
            &items,
            |item| format!("item-{item}"),
            |_, item| Button::new(format!("Item {item}"), |_| Ok(())).key("ignored"),
        )
        .key("each"),
    )?;
    ui.flush(Duration::ZERO)?;
    let ids = |ui: &Runtime| -> Result<Vec<String>, RuntimeError> {
        // The default container has no address, but it keeps its key.
        let column = ui
            .find_descendant(&root, "items")?
            .ok_or(RuntimeError::Unmounted)?;
        ui.children(&column)?
            .iter()
            .map(|child| ui.key(child).map(str::to_owned))
            .collect()
    };
    assert_eq!(ids(&ui)?, ["item-1", "item-2", "item-3"]);
    let two = ui.find(&root, &["item-2"])?.ok_or("mounted")?;
    ui.focus(&two)?;
    items.set(vec![3, 2])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ids(&ui)?, ["item-3", "item-2"]);
    assert_eq!(ui.find(&root, &["item-2"])?, Some(two.clone()));
    assert_eq!(ui.focused(), Some(two));
    Ok(())
}

#[test]
fn a_duplicate_key_fails_the_flush_and_a_valid_list_recovers() -> TestResult {
    let mut ui = runtime()?;
    let items = ui.signal(vec!["a".to_owned()]);
    let root = ui.mount_tree(
        Each::new(&items, Clone::clone, |_, item| {
            Label::new(item.as_str()).key("item")
        })
        .key("each"),
    )?;
    ui.flush(Duration::ZERO)?;
    let a = ui.find(&root, &["a"])?.ok_or("mounted")?;
    items.set(vec!["a".to_owned(), "a".to_owned()])?;
    assert!(ui.flush(Duration::ZERO).is_err());
    assert_eq!(
        ui.find(&root, &["a"])?,
        Some(a.clone()),
        "the previous items stay"
    );
    items.set(vec!["a".to_owned(), "b".to_owned()])?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.find(&root, &["a"])?, Some(a));
    assert!(ui.find(&root, &["b"])?.is_some());
    Ok(())
}

#[test]
fn each_places_items_in_the_container_it_is_given() -> TestResult {
    let mut ui = runtime()?;
    let items = ui.signal(vec![1, 2]);
    let root = ui.mount_tree(
        Each::new(&items, ToString::to_string, |_, item| {
            Label::new(item.to_string()).key("item")
        })
        .within(|items| Row::new(items).key("row"))
        .key("each"),
    )?;
    ui.flush(Duration::ZERO)?;
    let first = ui.find(&root, &["row", "1"])?.ok_or("mounted")?;
    let second = ui.find(&root, &["row", "2"])?.ok_or("mounted")?;
    let (a, b) = (
        ui.bounds(&first)?.ok_or("laid out")?,
        ui.bounds(&second)?.ok_or("laid out")?,
    );
    assert!(b.x() > a.x(), "a row places items side by side");
    assert_eq!(a.y(), b.y());
    Ok(())
}

#[test]
fn a_description_callback_creates_state_but_cannot_write() -> TestResult {
    let mut ui = runtime()?;
    let mode = ui.signal(0);
    let written = Rc::new(std::cell::RefCell::new(None));
    let outcome = written.clone();
    let target = mode.clone();
    let root = ui.mount_tree(
        Dynamic::new(&mode, move |cx, mode| {
            *outcome.borrow_mut() = Some(target.set(5));
            let choice = cx.signal(*mode);
            match Select::new(cx, choice, [Choice::new(0, "Zero"), Choice::new(1, "One")]) {
                Ok(select) => select.key("select"),
                Err(error) => Label::new(error.to_string()).key("error"),
            }
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    assert!(support::key_path(&ui, &root, &["select"]).is_some());
    assert_eq!(
        *written.borrow(),
        Some(Err(RuntimeError::WriteDuringUpdate))
    );
    assert_eq!(mode.get()?, 0);
    Ok(())
}

#[test]
fn a_new_value_is_described_even_when_the_old_one_compares_equal_later() -> TestResult {
    let mut ui = runtime()?;
    let first = Rc::new(Cell::new(0));
    let source = ui.signal(first.clone());
    let root = ui.mount_tree(
        Dynamic::new(&source, |_, cell| {
            Label::new(cell.get().to_string()).key("value")
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    // A distinct allocation is a change; changing the old one afterwards makes
    // the two compare equal, which must not hide the accepted write.
    source.set(Rc::new(Cell::new(1)))?;
    first.set(1);
    ui.flush(Duration::ZERO)?;
    assert!(support::key_path(&ui, &root, &["value"]).is_some());
    assert_eq!(name_of(&ui, "value").as_deref(), Some("1"));
    Ok(())
}

/// Mounts `child` under a `Dynamic` over `mode`, presses the button keyed
/// `button` and returns the press position, so a test can change `mode`
/// before releasing.
fn press_button(ui: &mut Runtime, root: &WidgetId) -> Result<Point, Box<dyn std::error::Error>> {
    let button = ui.find_descendant(root, "button")?.ok_or("mounted")?;
    let bounds = ui.bounds(&button)?.ok_or("laid out")?;
    let at = Point::new(bounds.x() + 4.0, bounds.y() + 4.0)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    Ok(at)
}

fn release(ui: &mut Runtime, at: Point) -> TestResult {
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

#[test]
fn a_press_survives_a_compatible_description_and_ends_with_its_child() -> TestResult {
    let mut ui = runtime()?;
    let mode = ui.signal(0);
    let clicks = ui.signal(0);
    let counter = clicks.clone();
    let root = ui.mount_tree(
        Dynamic::new(&mode, move |_, mode| {
            let counter = counter.clone();
            let action = Button::new(format!("Action {mode}"), move |_| {
                counter.update(|n| *n += 1)
            });
            // Modes 0 and 1 keep the key; mode 2 replaces the button.
            action.key(if *mode < 2 { "button" } else { "other" })
        })
        .key("dynamic"),
    )?;
    ui.flush(Duration::ZERO)?;
    let at = press_button(&mut ui, &root)?;
    mode.set(1)?;
    ui.flush(Duration::ZERO)?;
    release(&mut ui, at)?;
    assert_eq!(clicks.get()?, 1, "a kept button keeps its press");
    let at = press_button(&mut ui, &root)?;
    mode.set(2)?;
    ui.flush(Duration::ZERO)?;
    release(&mut ui, at)?;
    assert_eq!(
        clicks.get()?,
        1,
        "a replaced button's press activates nothing"
    );
    Ok(())
}

#[test]
fn hiding_a_pressed_child_cancels_its_activation() -> TestResult {
    let mut ui = runtime()?;
    let shown = ui.signal(true);
    let clicks = ui.signal(0);
    let counter = clicks.clone();
    let root = ui.mount_tree(
        Show::new(&shown, move |_| {
            let counter = counter.clone();
            Button::new("Action", move |_| counter.update(|n| *n += 1)).key("button")
        })
        .key("show"),
    )?;
    ui.flush(Duration::ZERO)?;
    let at = press_button(&mut ui, &root)?;
    shown.set(false)?;
    ui.flush(Duration::ZERO)?;
    shown.set(true)?;
    ui.flush(Duration::ZERO)?;
    release(&mut ui, at)?;
    assert_eq!(clicks.get()?, 0);
    Ok(())
}

#[test]
fn redescribing_show_and_each_matches_a_fresh_mount() -> TestResult {
    let mut ui = runtime()?;
    let (hidden, shown) = (ui.signal(false), ui.signal(true));
    let show =
        |flag: &Signal<bool>| Show::new(flag, |_| Label::new("Shown").key("text")).key("show");
    check_redescription(&mut ui, show(&hidden), show(&shown), show(&shown))?;
    // The check leaves the retained tree mounted; check `Each` in a runtime
    // of its own.
    let mut ui = runtime()?;
    let (few, more) = (ui.signal(vec![1]), ui.signal(vec![1, 2]));
    let each = |items: &Signal<Vec<i32>>| {
        Each::new(items, ToString::to_string, |_, item| {
            Label::new(item.to_string()).key("item")
        })
        .key("each")
    };
    check_redescription(&mut ui, each(&few), each(&more), each(&more))?;
    Ok(())
}
