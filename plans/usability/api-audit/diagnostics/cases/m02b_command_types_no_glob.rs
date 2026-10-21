// m02 variant: no prelude glob, the runtime named by path, the command types unimported.
fn main() -> Result<(), pert_ui::RuntimeError> {
    let mut ui = pert_ui::Runtime::new();
    let _ = ui.register_command("New", CommandScope::Application,
        vec![KeyChord::new(CommandKey::Character('n'), Modifiers::default())], |_, _| Ok(()))?;
    Ok(())
}
