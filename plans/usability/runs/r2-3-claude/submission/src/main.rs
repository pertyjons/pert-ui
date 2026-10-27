//! `point_picker` opens the application. `point_picker --capture DIR` instead
//! runs it headless, writes `DIR/dark.png` and `DIR/light.png` (CPU captures
//! with the picker focused) and prints the picker's inspection entry.

use pert_ui::ThemeMode;
use point_picker::app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--capture" => capture(std::path::Path::new(dir)),
        [] => Ok(app::application(Default::default()).run()?),
        _ => Err("usage: point_picker [--capture DIR]".into()),
    }
}

fn capture(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let slot = app::Slot::default();
    let mut headless = app::application(slot.clone()).headless()?;
    let handles = slot.borrow().clone().ok_or("application not built")?;
    handles.x.set(0.3)?;
    handles.y.set(0.7)?;
    for (mode, name) in [(ThemeMode::Dark, "dark.png"), (ThemeMode::Light, "light.png")] {
        let png = app::capture_png(&mut headless, &handles, mode)?;
        std::fs::write(dir.join(name), png)?;
        println!("wrote {}", dir.join(name).display());
    }
    let snapshot = headless.runtime().inspect();
    for component in snapshot.components.iter().filter(|c| c.key == "picker") {
        println!("inspection: focused={} semantics={:?} properties={:?}",
            component.focused, component.details.semantics, component.details.properties);
    }
    headless.shutdown()?;
    Ok(())
}
