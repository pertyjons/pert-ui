use pert_ui::prelude::*;
fn main() -> Result<(), AppError> {
    App::new("Background job monitor", |ui| {
        let monitor = job_monitor::build(ui, None)?;
        ui.keep_alive(monitor);
        Ok(())
    })
    .run()
}
