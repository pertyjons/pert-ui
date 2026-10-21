use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use pert_ui::{App, AppError, NativeCloseAction};
use settings_editor::{build, Editor};

fn main() -> Result<(), AppError> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("export-settings.json"));
    let editor: Rc<RefCell<Option<Editor>>> = Rc::default();
    let built = editor.clone();
    App::new("Export settings", move |ui| {
        *built.borrow_mut() = Some(build(ui, path)?);
        Ok(())
    })
    .on_close(move |ui, window| match editor.borrow().as_ref() {
        Some(editor) => editor.on_close(ui, window),
        None => Ok(NativeCloseAction::Close),
    })
    .run()
}
