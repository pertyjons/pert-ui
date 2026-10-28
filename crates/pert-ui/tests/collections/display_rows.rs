//! Display controls in collection rows (F16): a row describes its record's
//! value with the control's fixed form, `ProgressBar::new(record.status)` or
//! `Spinner::new(record.running)`, and holds no signal per row. A
//! republished source describes the mounted rows again, and a row mounted by
//! a scroll is built from the current record.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const ROW: f32 = 32.0;

#[derive(Clone, PartialEq)]
struct Job {
    id: &'static str,
    status: ProgressStatus,
    running: bool,
}

fn property(ui: &Runtime, id: &WidgetId, name: &str) -> Result<Option<InspectValue>, RuntimeError> {
    let id = ui.inspection_id(id)?;
    Ok(ui
        .query(&InspectionQuery::components(Selector::id(id)).detail(Detail::Full))
        .components
        .pop()
        .and_then(|component| component.details.properties.get(name).cloned()))
}

fn text(value: &str) -> Option<InspectValue> {
    Some(InspectValue::Text(value.to_owned()))
}

/// The records as table rows: a bar and a spinner described from each record.
struct Jobs(Vec<Job>);
impl TableItems for Jobs {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.0.get(index)?.id.to_owned())
    }
    fn cell(&self, index: usize, column: &str) -> Option<Element> {
        let job = self.0.get(index)?;
        match column {
            "progress" => Some(ProgressBar::new(job.status).key("bar")),
            "busy" => Some(Spinner::new(job.running).key("spinner")),
            _ => None,
        }
    }
}

/// Publishes the records as the table's rows, as an application does after
/// each change of its records.
fn publish(store: &Signal<Vec<Job>>, table: &TableController) -> Result<(), RuntimeError> {
    table.set_source(TableSource::new(Jobs(store.get()?)))
}

#[test]
fn table_rows_describe_their_bars_and_spinners_from_the_records() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 200.0, 1.0)?)?;
    let store = ui.signal(vec![
        Job {
            id: "a",
            status: ProgressStatus::of(1, 4),
            running: true,
        },
        Job {
            id: "b",
            status: ProgressStatus::Failed(ProgressStatus::of(3, 8).fraction()),
            running: false,
        },
    ]);
    let table = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [
            TableColumn::new("progress", "Progress").width(Length::new(300.0)?),
            TableColumn::new("busy", "Busy").width(Length::new(60.0)?),
        ],
        TableSource::new(Jobs(store.get()?)),
    )?;
    let root = ui.mount_tree(Table::new(&table, ROW)?.key("table"))?;
    ui.flush(Duration::ZERO)?;
    let cell =
        |ui: &Runtime, row: &str, key: &str| -> Result<WidgetId, Box<dyn std::error::Error>> {
            let row = ui.find_descendant(&root, row)?.ok_or("a row")?;
            Ok(ui.find_descendant(&row, key)?.ok_or("a cell")?)
        };
    let a_bar = cell(&ui, "a", "bar")?;
    assert_eq!(property(&ui, &a_bar, "status")?, text("determinate"));
    let a_spinner = cell(&ui, "a", "spinner")?;
    assert_eq!(
        property(&ui, &a_spinner, "busy")?,
        Some(InspectValue::Bool(true))
    );

    // Job a finishes: the application writes its record and publishes.
    store.update(|jobs| {
        jobs[0].status = ProgressStatus::Succeeded;
        jobs[0].running = false;
    })?;
    publish(&store, &table)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        cell(&ui, "a", "bar")?,
        a_bar,
        "described again, not remounted"
    );
    assert_eq!(property(&ui, &a_bar, "status")?, text("succeeded"));
    assert_eq!(
        property(&ui, &cell(&ui, "a", "spinner")?, "busy")?,
        Some(InspectValue::Bool(false))
    );
    let b_bar = cell(&ui, "b", "bar")?;
    assert_eq!(property(&ui, &b_bar, "status")?, text("failed"));
    assert_eq!(
        property(&ui, &b_bar, "fraction")?,
        Some(InspectValue::Number(0.375))
    );
    Ok(())
}

/// The records as list rows: a title and a bar described from each record.
struct Tasks(Vec<(u32, ProgressStatus)>);
impl ListItems for Tasks {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        Some(self.0.get(index)?.0.to_string())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let (id, progress) = *self.0.get(index)?;
        Some(
            Row::new([
                Label::new(format!("Task {id}")).key("title"),
                ProgressBar::new(progress).key("progress"),
            ])
            .key("row"),
        )
    }
}

#[test]
fn a_list_row_mounted_by_a_scroll_shows_its_records_status() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 120.0, 1.0)?)?;
    let records = ui.signal(
        (1..=30)
            .map(|id| (id, ProgressStatus::of(1, 4)))
            .collect::<Vec<_>>(),
    );
    let source = records.map(|records| ListSource::new(Tasks(records.clone())))?;
    let root = ui.mount_tree(VirtualList::new(source, ROW)?.key("list"))?;
    ui.flush(Duration::ZERO)?;
    let first = ui.find(&root, &["1", "progress"])?.ok_or("row 1's bar")?;
    assert_eq!(property(&ui, &first, "status")?, text("determinate"));

    let wheel = |ui: &mut Runtime, dy: f32| -> Result<(), RuntimeError> {
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 10.0)?,
            PointerKind::Scroll(Point::new(0.0, dy)?),
        ))?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    };
    wheel(&mut ui, 600.0)?;
    assert_eq!(ui.find(&root, &["1"])?, None, "scrolled out and unmounted");
    // The record changes while its row is not mounted.
    records.update(|records| records[0].1 = ProgressStatus::Succeeded)?;
    ui.flush(Duration::ZERO)?;
    wheel(&mut ui, -600.0)?;
    let again = ui.find(&root, &["1", "progress"])?.ok_or("row 1's bar")?;
    assert_ne!(again, first, "a remounted row");
    assert_eq!(property(&ui, &again, "status")?, text("succeeded"));
    let second = ui.find(&root, &["2", "progress"])?.ok_or("row 2's bar")?;
    assert_eq!(
        property(&ui, &second, "fraction")?,
        Some(InspectValue::Number(0.25))
    );
    Ok(())
}
