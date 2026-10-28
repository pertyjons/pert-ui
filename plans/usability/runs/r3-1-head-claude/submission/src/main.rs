use std::path::PathBuf;

fn main() -> Result<(), pert_ui::AppError> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("export-settings.json"));
    settings_editor::app(path).run()
}
