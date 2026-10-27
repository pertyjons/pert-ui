mod headless;
use pert_ui::prelude::*;
use point_picker::PointPicker;

fn mount(ui: &mut Runtime) -> Result<WidgetId, RuntimeError> {
    let x = ui.signal(0.5_f64);
    let y = ui.signal(0.5_f64);
    let status = ui.signal(String::from("Pick a point"));
    let changed = status.clone();
    let (reset_x, reset_y) = (x.clone(), y.clone());
    ui.mount_tree(Element::new(
        "app",
        Container::new(Element::new(
            "content",
            Column::new([
                Element::new("title", Label::new("Point picker")),
                Element::new(
                    "body",
                    Row::new([
                        Element::new(
                            "picker",
                            PointPicker::new(x.clone(), y.clone()).on_change(move |_, (x, y)| {
                                changed.set(format!("Last picker input: {x:.2}, {y:.2}"))
                            }),
                        ),
                        Element::new(
                            "fields",
                            Column::new([
                                Element::new("x-label", Label::new("X · horizontal")),
                                Element::new(
                                    "x",
                                    DecimalInput::new(x).range(0.0, 1.0)?.decimal_places(2)?,
                                ),
                                Element::new("y-label", Label::new("Y · vertical")),
                                Element::new(
                                    "y",
                                    DecimalInput::new(y).range(0.0, 1.0)?.decimal_places(2)?,
                                ),
                                Element::new(
                                    "reset",
                                    Button::new("Reset", move |_| {
                                        reset_x.set(0.5)?;
                                        reset_y.set(0.5)
                                    }),
                                ),
                            ]),
                        ),
                    ]),
                ),
                Element::new(
                    "help",
                    Label::new("Drag · Arrows: 0.01 · Shift: 0.1 · Escape: cancel"),
                ),
                Element::new("status", Label::new(status)),
            ]),
        ))
        .padding(Length::from(24))
        .background_role(ColorRole::Background),
    ))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|arg| arg == "--headless") {
        return headless::check(std::path::Path::new("artifacts"));
    }
    App::new("Point picker", |ui| {
        mount(ui)?;
        Ok(())
    })
    .run()?;
    Ok(())
}
