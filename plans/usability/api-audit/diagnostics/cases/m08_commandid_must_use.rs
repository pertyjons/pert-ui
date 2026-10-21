// F19: an application-lifetime command whose CommandId is dropped.
#![deny(unused_must_use)]
use pert_ui::{CommandScope, KeyChord, prelude::*};
fn main() -> Result<(), RuntimeError> {
    let mut ui = Runtime::new();
    ui.register_command("New", CommandScope::Application, vec![KeyChord::primary('n')], |_, _| Ok(()))?;
    Ok(())
}
