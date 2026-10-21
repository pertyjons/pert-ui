// CommandScope, KeyChord and CommandKey are not in the prelude.
use pert_ui::prelude::*;
fn main() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    let _ = ui.register_command("New", CommandScope::Application,
        vec![KeyChord::new(CommandKey::Character('n'), Default::default())], |_, _| Ok(()))?;
    Ok(())
}
