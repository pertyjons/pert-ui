use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = match std::env::args_os().nth(1) {
        Some(dir) => PathBuf::from(dir),
        None => {
            let dir = std::env::current_dir()?.join("notes");
            std::fs::create_dir_all(&dir)?;
            dir
        }
    };
    notes::build_app(dir)?.run()?;
    Ok(())
}
