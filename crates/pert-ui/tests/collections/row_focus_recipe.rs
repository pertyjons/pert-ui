//! The layout guide's Button-row recipe: a row opens on a pointer press, makes
//! itself active and requests focus for its list, so the arrows and Enter
//! continue from the opened row and Enter runs `VirtualList::on_activate`.
//! The rows hold the list's `WidgetId`, published after the list is mounted
//! over an empty source; a stale ID is ignored and keeps nothing mounted.
use pert_ui::testing::Driver;
use pert_ui::*;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<String>>>;

/// The rows before the list's ID is known.
struct NoRows;
impl ListItems for NoRows {
    fn len(&self) -> usize {
        0
    }
    fn key(&self, _index: usize) -> Option<String> {
        None
    }
    fn build(&self, _index: usize) -> Option<Element> {
        None
    }
}

struct Files {
    names: Vec<String>,
    list: WidgetId,
    selection: ListSelection,
    log: Log,
    /// Dropped with the rows, so a test can tell nothing else retains them.
    _alive: Rc<()>,
}
impl ListItems for Files {
    fn len(&self) -> usize {
        self.names.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.names.get(index).cloned()
    }
    fn build(&self, index: usize) -> Option<Element> {
        let name = self.names.get(index)?.clone();
        let (list, selection, log) = (self.list.clone(), self.selection.clone(), self.log.clone());
        let open = Button::new(format!("Open {name}"), move |cx| {
            log.borrow_mut().push(format!("open:{name}"));
            selection.set_active(Some(name.clone()))?;
            cx.request_focus_for(&list)
        });
        Some(open.key("row"))
    }
}

struct Fixture {
    root: WidgetId,
    rows: Signal<ListSource>,
    selection: ListSelection,
    log: Log,
}

fn files_list(rows: &Signal<ListSource>, selection: &ListSelection, log: &Log) -> Element {
    let activated = log.clone();
    VirtualList::new(rows.read_only(), 32.0)
        .expect("a positive row extent")
        .selection(selection.clone())
        .item_tab_stops(false)
        .on_activate(move |_, name| {
            activated.borrow_mut().push(format!("activate:{name}"));
            Ok(())
        })
        .key("files")
        .with_grow(1)
}

/// One root: a button before the list, the list (inside `Show` while `shown`
/// reads true) and a button after it.
fn mount(ui: &mut Runtime, shown: &Signal<bool>) -> Result<Fixture, RuntimeError> {
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let selection = ListSelection::new(ui, SelectionMode::Single);
    let log: Log = Rc::default();
    let rows = ui.signal(ListSource::new(NoRows));
    let (build_rows, build_selection, build_log) = (rows.clone(), selection.clone(), log.clone());
    let root = ui.mount_tree(
        Column::new([
            Button::new("Before", |_| Ok(())).key("before"),
            Show::new(shown, move |_| {
                files_list(&build_rows, &build_selection, &build_log)
            })
            .key("panel")
            .with_grow(1),
            Button::new("After", |_| Ok(())).key("after"),
        ])
        .key("app"),
    )?;
    ui.flush(Duration::ZERO)?;
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    Ok(Fixture {
        root,
        rows,
        selection,
        log,
    })
}

fn list(ui: &Runtime, root: &WidgetId) -> Result<WidgetId, Box<dyn std::error::Error>> {
    Ok(ui.find_descendant(root, "files")?.ok_or("the list")?)
}

/// Publishes the rows holding `list`, returning the token they drop with.
fn publish(fixture: &Fixture, list: &WidgetId) -> Result<Weak<()>, RuntimeError> {
    let alive = Rc::new(());
    let token = Rc::downgrade(&alive);
    fixture.rows.set(ListSource::new(Files {
        names: ["alpha", "beta", "gamma"].map(String::from).to_vec(),
        list: list.clone(),
        selection: fixture.selection.clone(),
        log: fixture.log.clone(),
        _alive: alive,
    }))?;
    Ok(token)
}

fn selected(keys: &[&str]) -> Rc<BTreeSet<String>> {
    Rc::new(keys.iter().map(|key| (*key).to_owned()).collect())
}

#[test]
fn click_then_arrows_and_enter_continue_from_the_opened_row() -> TestResult {
    let mut ui = Runtime::new();
    let shown = ui.signal(true);
    let fixture = mount(&mut ui, &shown)?;
    let list = list(&ui, &fixture.root)?;
    publish(&fixture, &list)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    let alpha = driver.find(&list, &["alpha"])?;

    driver.click(&alpha)?;
    assert_eq!(*fixture.log.borrow(), ["open:alpha"]);
    assert_eq!(driver.runtime().focused(), Some(list.clone()));
    assert_eq!(fixture.selection.active().get()?.as_deref(), Some("alpha"));
    assert_eq!(fixture.selection.selected().get()?, selected(&["alpha"]));

    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(fixture.selection.active().get()?.as_deref(), Some("beta"));
    assert_eq!(fixture.selection.selected().get()?, selected(&["beta"]));
    assert_eq!(driver.runtime().focused(), Some(list.clone()));

    // Enter runs on_activate for the active row, not the clicked Button.
    driver.key(KeyCode::Enter)?;
    assert_eq!(*fixture.log.borrow(), ["open:alpha", "activate:beta"]);
    assert_eq!(driver.runtime().focused(), Some(list.clone()));
    assert_eq!(fixture.selection.active().get()?.as_deref(), Some("beta"));

    // The rows are no Tab stops: Tab leaves the list in one step.
    driver.key(KeyCode::Tab)?;
    let after = driver.find(&fixture.root, &["after"])?;
    assert_eq!(driver.runtime().focused(), Some(after));
    Ok(())
}

#[test]
fn rows_holding_a_remounted_list_request_nothing_until_published_again() -> TestResult {
    let mut ui = Runtime::new();
    let shown = ui.signal(true);
    let fixture = mount(&mut ui, &shown)?;
    let first = list(&ui, &fixture.root)?;
    publish(&fixture, &first)?;
    ui.flush(Duration::ZERO)?;
    shown.set(false)?;
    ui.flush(Duration::ZERO)?;
    shown.set(true)?;
    ui.flush(Duration::ZERO)?;
    let second = list(&ui, &fixture.root)?;
    assert_ne!(
        second, first,
        "mounting again under the key is a new identity"
    );
    assert_eq!(ui.key(&first), Err(RuntimeError::Unmounted));

    // The rows still hold the first list: its request is ignored without an
    // error, so the clicked Button keeps focus and Enter presses it again.
    let mut driver = Driver::new(&mut ui);
    let alpha = driver.find(&second, &["alpha"])?;
    driver.click(&alpha)?;
    assert_eq!(*fixture.log.borrow(), ["open:alpha"]);
    assert_eq!(driver.runtime().focused(), Some(alpha.clone()));
    assert_eq!(fixture.selection.active().get()?.as_deref(), Some("alpha"));
    driver.key(KeyCode::ArrowDown)?;
    assert_eq!(fixture.selection.active().get()?.as_deref(), Some("beta"));
    driver.key(KeyCode::Enter)?;
    assert_eq!(*fixture.log.borrow(), ["open:alpha", "open:alpha"]);

    // Published again with the mounted list, the rows hand it focus.
    publish(&fixture, &second)?;
    driver.settle()?;
    let gamma = driver.find(&second, &["gamma"])?;
    driver.click(&gamma)?;
    assert_eq!(driver.runtime().focused(), Some(second.clone()));
    driver.key(KeyCode::ArrowUp)?;
    driver.key(KeyCode::Enter)?;
    assert_eq!(
        *fixture.log.borrow(),
        ["open:alpha", "open:alpha", "open:gamma", "activate:beta"]
    );
    Ok(())
}

#[test]
fn rows_holding_the_list_id_keep_neither_the_list_nor_themselves_alive() -> TestResult {
    let mut ui = Runtime::new();
    let shown = ui.signal(true);
    let fixture = mount(&mut ui, &shown)?;
    let list = list(&ui, &fixture.root)?;
    let rows = publish(&fixture, &list)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.find(&list, &["alpha"])?.is_some());

    ui.unmount(&fixture.root)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.key(&list), Err(RuntimeError::Unmounted));
    assert!(
        rows.upgrade().is_some(),
        "the source signal still holds the rows"
    );
    fixture.rows.set(ListSource::new(NoRows))?;
    ui.flush(Duration::ZERO)?;
    assert!(rows.upgrade().is_none(), "nothing else retains the rows");
    Ok(())
}
