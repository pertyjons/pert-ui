fn main() -> Result<(), Box<dyn std::error::Error>> {
    let folder = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("notes"));
    std::fs::create_dir_all(&folder)?;
    notes_editor::app(folder).run()?;
    Ok(())
}
