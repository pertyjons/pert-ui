use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    App::new("Tasks", |ui| {
        let app = todo::build(ui, Vec::new())?;
        ui.keep_alive(app);
        Ok(())
    })
    .run()
}
