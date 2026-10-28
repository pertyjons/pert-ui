pub mod picker;
use pert_ui::prelude::*;
use picker::PointPicker;

pub struct State {
    pub root: WidgetId,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    pub changes: Signal<u32>,
}

pub fn mount(ui: &mut Runtime) -> Result<State, RuntimeError> {
    let x = ui.signal(0.5);
    let y = ui.signal(0.5);
    let changes = ui.signal(0);
    let reported = changes.clone();
    let (rx, ry) = (x.clone(), y.clone());
    let root = ui.mount_tree(
        Column::new([
            Label::new("Point picker").key("title"),
            Row::new([
                PointPicker::new(x.clone(), y.clone())
                    .on_change(move |_, _| reported.update(|n| *n += 1))
                    .key("picker"),
                Column::new([
                    Label::new("X · left to right").key("x-label"),
                    DecimalInput::new(x.clone())
                        .range(0.0, 1.0)?
                        .decimal_places(2)?
                        .step(0.01)?
                        .key("x"),
                    Label::new("Y · top to bottom").key("y-label"),
                    DecimalInput::new(y.clone())
                        .range(0.0, 1.0)?
                        .decimal_places(2)?
                        .step(0.01)?
                        .key("y"),
                    Button::new("Reset", move |_| {
                        rx.set(0.5)?;
                        ry.set(0.5)
                    })
                    .key("reset"),
                ])
                .key("fields"),
            ])
            .key("body"),
            Label::new("Drag to position · Escape to cancel · Arrows to nudge · Shift for 0.1")
                .key("help"),
        ])
        .key("app"),
    )?;
    Ok(State {
        root,
        x,
        y,
        changes,
    })
}

pub fn app() -> App<State> {
    App::new("Point picker", |ui| Ok(mount(ui)?))
}
