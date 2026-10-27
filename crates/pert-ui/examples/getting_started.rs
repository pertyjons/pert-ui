//! The getting-started guide's profile form: two text fields in an adaptive
//! form, and a button that resets both through their bindings.
//!
//! Needs the default `desktop` feature: `cargo run -p pert-ui --example getting_started`.
use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    App::new("Profile", |ui| {
        let name = ui.signal("Ada".to_owned());
        let city = ui.signal("Stockholm".to_owned());
        let (reset_name, reset_city) = (name.clone(), city.clone());
        let fields = Form::new([
            Field::new(TextField::new(&name).key("name"))
                .label("Name")
                .help("Shown to other people")
                .key("name-field"),
            Field::new(TextField::new(&city).key("city"))
                .label("City")
                .key("city-field"),
        ]);
        let reset = Button::new("Reset", move |_| {
            reset_name.set("Ada".to_owned())?;
            reset_city.set("Stockholm".to_owned())
        });
        let content = Column::new([
            Label::new("Your profile").key("heading"),
            fields.key("fields"),
            reset.key("reset"),
        ]);
        ui.mount_tree(
            ScrollArea::new(content.key("content"))
                .axes(ScrollAxes::Both)
                .key("profile-viewport"),
        )?;
        Ok(())
    })
    .run()
}
