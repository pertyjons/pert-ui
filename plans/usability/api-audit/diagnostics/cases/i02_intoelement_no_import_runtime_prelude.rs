// `.key(..)` with explicit root imports of several items but not IntoElement.
use pert_ui::{Button, Column, Label, Runtime, RuntimeError};
fn main() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.mount_tree(Column::new([Label::new("x").key("title"), Button::new("Save", |_| Ok(())).key("save")]).key("root"))?;
    Ok(())
}
