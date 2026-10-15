//! The getting-started guide's profile form: two text fields in an adaptive
//! form, and a button that resets both through their bindings.
//!
//! Needs the default `desktop` feature: `cargo run -p pert-ui --example getting_started`.
fn main() -> Result<(), pert_ui::AppError> {
    use pert_ui::prelude::*;
    App::new("Profile", |ui| {
        let name = ui.signal("Ada".to_owned());
        let city = ui.signal("Stockholm".to_owned());
        let reset_name = name.clone();
        let reset_city = city.clone();
        ui.mount_tree(Element::new(
            "profile-viewport",
            ScrollArea::new(Element::new(
                "content",
                Column::new([
                    Element::new("heading", Label::new("Your profile")),
                    Element::new(
                        "fields",
                        Form::new([
                            Element::new(
                                "name-field",
                                Field::new(Element::new("name", TextField::new(name)))
                                    .label("Name")
                                    .help("Shown to other people"),
                            ),
                            Element::new(
                                "city-field",
                                Field::new(Element::new("city", TextField::new(city)))
                                    .label("City"),
                            ),
                        ]),
                    ),
                    Element::new(
                        "reset",
                        Button::new("Reset", move |_| {
                            reset_name.set("Ada".to_owned())?;
                            reset_city.set("Stockholm".to_owned())?;
                            Ok(())
                        }),
                    ),
                ]),
            ))
            .axes(ScrollAxes::Both),
        ))?;
        Ok(())
    })
    .run()
}
