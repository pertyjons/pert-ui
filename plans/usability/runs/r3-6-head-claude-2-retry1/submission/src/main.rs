use std::path::PathBuf;

fn main() -> Result<(), pert_ui::AppError> {
    let dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    notes::app(dir).run()
}
