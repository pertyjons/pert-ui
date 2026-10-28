//! Opens the bookmark list in a native window.
fn main() -> Result<(), pert_ui::AppError> {
    use pert_ui::prelude::*;
    App::new("Bookmarks", |ui| {
        bookmarks::build(ui)?;
        Ok(())
    })
    .run()
}
