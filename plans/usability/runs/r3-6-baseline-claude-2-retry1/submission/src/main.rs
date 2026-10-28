use pert_ui::prelude::*;
use std::path::PathBuf;

fn main() -> Result<(), AppError> {
    let dir = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("notes"));
    if let Err(error) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot use {}: {error}", dir.display());
    }
    App::new("Notes", move |ui| {
        notes::build(ui, dir)?;
        Ok(())
    })
    .run()
}
