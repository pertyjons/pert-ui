//! The application: the picker beside two numeric fields for `x` and `y`,
//! bound to the same signals, and a Reset button.

use pert_ui::ApplicationError;
use pert_ui::prelude::*;

use crate::picker::PointPicker;

/// What the application builds; tests and the headless mode read it.
pub struct PickerApp {
    pub root: WidgetId,
    pub picker: WidgetId,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    /// The last point the user wrote through the picker, as text.
    pub last_change: Signal<String>,
}

/// Mounts the application's content on `ui`.
pub fn build(ui: &mut Runtime) -> Result<PickerApp, RuntimeError> {
    let x = ui.signal(0.5_f64);
    let y = ui.signal(0.5_f64);
    let last_change = ui.signal(String::from("No change yet"));

    let report = last_change.clone();
    let picker = PointPicker::new(&x, &y)
        .name("Point")
        .on_change(move |_, (x, y)| report.set(format!("Picked ({x:.2}, {y:.2})")));

    let (reset_x, reset_y) = (x.clone(), y.clone());
    let reset = Button::new("Reset", move |_| {
        reset_x.set(0.5)?;
        reset_y.set(0.5)
    });

    let fields = Column::new([
        Field::new(
            DecimalInput::new(x.clone())
                .range(0.0, 1.0)?
                .step(0.01)?
                .key("x"),
        )
        .label("x")
        .key("x-field"),
        Field::new(
            DecimalInput::new(y.clone())
                .range(0.0, 1.0)?
                .step(0.01)?
                .key("y"),
        )
        .label("y")
        .key("y-field"),
        reset.key("reset"),
        Label::new(last_change.read_only()).key("status"),
    ]);
    let layout = Row::new([Element::new("picker", picker), fields.key("fields")])
        .gap(MetricRole::SectionGap);
    let root = ui.mount_tree(
        Container::new(layout.key("layout"))
            .padding(MetricRole::ContentMargin)
            .key("content"),
    )?;
    let picker = ui
        .find(&root, &["layout", "picker"])?
        .ok_or(ApplicationError::from("the picker is mounted"))?;
    Ok(PickerApp {
        root,
        picker,
        x,
        y,
        last_change,
    })
}

/// The native application.
pub fn app() -> App<PickerApp> {
    App::new("Point picker", |ui| Ok(build(ui)?))
}
