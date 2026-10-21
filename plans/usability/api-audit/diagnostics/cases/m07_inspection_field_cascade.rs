// E0609 on entry.properties plus an E0614 cascade on the same expression.
use pert_ui::{InspectValue, prelude::*};
fn number(ui: &Runtime, key: &str) -> Result<f64, String> {
    let snapshot = ui.inspect();
    let entry = snapshot.components.first().ok_or("none")?;
    match entry.properties.get(key) {
        Some(InspectValue::Number(n)) => Ok(*n),
        other => Err(format!("{other:?}")),
    }
}
fn main() { let ui = Runtime::new(); let _ = number(&ui, "x"); }
