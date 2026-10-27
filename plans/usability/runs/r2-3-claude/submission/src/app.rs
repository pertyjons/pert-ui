//! The application: the picker beside numeric fields for `x` and `y`, and Reset.

use pert_ui::prelude::*;
use pert_ui::ThemeMode;

use crate::PointPicker;

/// What the application builds, for its tests and tools.
#[derive(Clone)]
pub struct Handles {
    pub root: WidgetId,
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    /// Committed edits the picker reported.
    pub edits: Signal<u32>,
}

pub const DEFAULT: (f64, f64) = (0.5, 0.5);

/// Key paths below [`Handles::root`].
pub const PICKER: &[&str] = &["row", "picker"];
pub const X_FIELD: &[&str] = &["row", "side", "fields", "x-field", "x"];
pub const Y_FIELD: &[&str] = &["row", "side", "fields", "y-field", "y"];
pub const RESET: &[&str] = &["row", "side", "reset"];

/// Mounts the application's content on `ui`.
pub fn build(ui: &mut Runtime) -> Result<Handles, RuntimeError> {
    let x = ui.signal(DEFAULT.0);
    let y = ui.signal(DEFAULT.1);
    let edits = ui.signal(0_u32);

    let counter = edits.clone();
    let picker = PointPicker::new(&x, &y)
        .name("Point")
        .on_change(move |_, _| counter.update(|n| *n += 1));

    let field = |signal: &Signal<f64>| -> Result<DecimalInput, RuntimeError> {
        DecimalInput::new(signal.clone())
            .range(0.0, 1.0)
            .and_then(|f| f.step(0.01))
            .and_then(|f| f.decimal_places(2))
            .map_err(|e| RuntimeError::from(ApplicationError::new(e)))
    };

    let (rx, ry) = (x.clone(), y.clone());
    let reset = Button::new("Reset", move |_| {
        rx.set(DEFAULT.0)?;
        ry.set(DEFAULT.1)
    });

    let fields = Form::new([
        Field::new(field(&x)?.key("x")).label("x").key("x-field"),
        Field::new(field(&y)?.key("y")).label("y").key("y-field"),
    ]);
    let row = Row::new([
        Element::new("picker", picker),
        Element::new("side", Column::new([fields.key("fields"), reset.key("reset")])),
    ])
    .gap(MetricRole::SectionGap)
    .align(Align::Start);
    let root = ui.mount_tree(Element::new(
        "content",
        Container::new(Element::new("row", row)).padding(MetricRole::ContentMargin),
    ))?;
    Ok(Handles { root, x, y, edits })
}

/// Where the application leaves its handles once the initializer ran.
pub type Slot = std::rc::Rc<std::cell::RefCell<Option<Handles>>>;

/// The application, filling `slot` with its handles when it initializes.
pub fn application(slot: Slot) -> App {
    let app = App::new("Point picker", move |ui| {
        *slot.borrow_mut() = Some(build(ui)?);
        Ok(())
    });
    app.size(Size::new(560.0, 300.0).expect("a valid size")).expect("a valid window size")
}

/// Renders the main window of a headless application to PNG bytes on the CPU,
/// in `mode`, with the picker focused.
pub fn capture_png(
    app: &mut pert_ui::testing::HeadlessApp,
    handles: &Handles,
    mode: ThemeMode,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use pert_ui::testing::*;
    let mut driver = Driver::new(app.runtime_mut());
    driver.runtime().theme().set_mode(mode)?;
    let picker = driver.find(&handles.root, PICKER)?;
    driver.runtime_mut().focus(&picker)?;
    driver.settle()?;
    driver.runtime().set_capture_support(true);
    let pending = driver.runtime().request_capture(CaptureRequest::new(CaptureTarget::window(
        &driver.runtime().primary_window(),
    )));
    driver.settle()?;
    CpuRenderer::new().serve_captures(driver.runtime_mut());
    let capture = pending.wait()?.value?;
    Ok(capture.image.encode_png())
}
