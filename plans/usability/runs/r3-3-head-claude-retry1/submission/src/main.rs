//! The point picker application.
//!
//! `point_picker` opens the window. `point_picker --capture <dir>` runs
//! headless instead and writes `dark.png` and `light.png` there: the
//! application rendered on the CPU with the picker focused.

use pert_ui::ThemeMode;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--capture" => {
            std::fs::create_dir_all(dir)?;
            for (name, mode) in [("dark", ThemeMode::Dark), ("light", ThemeMode::Light)] {
                let path = std::path::Path::new(dir).join(format!("{name}.png"));
                std::fs::write(&path, point_picker::headless::capture_png(mode)?)?;
                println!("wrote {}", path.display());
            }
            Ok(())
        }
        [] => Ok(point_picker::app().run()?),
        _ => Err("usage: point_picker [--capture <dir>]".into()),
    }
}
