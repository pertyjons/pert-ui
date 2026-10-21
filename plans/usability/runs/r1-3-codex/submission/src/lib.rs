pub mod picker;
use pert_ui::*;
use picker::{PickPoint, PointPicker};

pub struct Application {
    pub point: Signal<PickPoint>,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    pub root: WidgetId,
}
pub fn mount(ui: &mut Runtime) -> Result<Application, RuntimeError> {
    let point = ui.signal(PickPoint::new(0.5, 0.5));
    let x = ui.signal(0.5_f64);
    let y = ui.signal(0.5_f64);
    let (sx, sy) = (x.clone(), y.clone());
    let picker = PointPicker::new(point.clone()).on_change(move |_, p| {
        if sx.get()? != p.x {
            sx.set(p.x)?;
        }
        if sy.get()? != p.y {
            sy.set(p.y)?;
        }
        Ok(())
    });
    let px = point.clone();
    let x_field = DecimalInput::new(x.clone())
        .range(0.0, 1.0)?
        .step(0.01)?
        .decimal_places(2)?
        .on_save(move |_, x| {
            let p = px.get()?;
            px.set(PickPoint::new(x, p.y))
        });
    let py = point.clone();
    let y_field = DecimalInput::new(y.clone())
        .range(0.0, 1.0)?
        .step(0.01)?
        .decimal_places(2)?
        .on_save(move |_, y| {
            let p = py.get()?;
            py.set(PickPoint::new(p.x, y))
        });
    let (rp, rx, ry) = (point.clone(), x.clone(), y.clone());
    let reset = Button::new("Reset", move |_| {
        rp.set(PickPoint::new(0.5, 0.5))?;
        rx.set(0.5)?;
        ry.set(0.5)
    });
    let root = ui.mount_tree(Element::new(
        "app",
        Container::new(Element::new(
            "content",
            Column::new([
                Element::new("title", Label::new("Point picker")),
                Element::new(
                    "body",
                    Row::new([
                        Element::new("picker", picker),
                        Element::new(
                            "fields",
                            Column::new([
                                Element::new("x-label", Label::new("X")),
                                Element::new("x", x_field),
                                Element::new("y-label", Label::new("Y")),
                                Element::new("y", y_field),
                                Element::new("reset", reset),
                            ])
                            .gap(Length::new(8.0)?),
                        ),
                    ])
                    .gap(Length::new(24.0)?),
                ),
                Element::new(
                    "help",
                    Label::new("Drag or use arrows. Shift: 0.1 · Escape: cancel"),
                ),
            ])
            .gap(Length::new(16.0)?),
        ))
        .padding(Length::new(24.0)?)
        .background_role(ColorRole::Background),
    ))?;
    Ok(Application { point, x, y, root })
}
pub mod headless;
