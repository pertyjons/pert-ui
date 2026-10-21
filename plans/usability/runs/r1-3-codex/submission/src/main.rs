fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--window") {
        pert_ui::App::new("Point picker", |ui| {
            point_picker::mount(ui)?;
            Ok(())
        })
        .run()?;
    } else {
        point_picker::headless::check(std::path::Path::new("artifacts"))?;
    }
    Ok(())
}
