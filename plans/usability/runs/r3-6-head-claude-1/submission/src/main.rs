use std::path::PathBuf;

fn main() -> Result<(), pert_ui::AppError> {
    let dir = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("notes"));
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot use the notes folder {}: {error}", dir.display());
        std::process::exit(1);
    }
    notes_editor::app(dir).run()
}
