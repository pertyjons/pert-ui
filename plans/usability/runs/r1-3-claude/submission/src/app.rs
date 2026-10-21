//! The application: a point picker beside numeric fields for `x` and `y`,
//! and a Reset button.

use crate::picker::PointPicker;
use pert_ui::{
    Button, Column, Container, DecimalInput, Field, IntoElement, Label, MetricRole, Row, Runtime, RuntimeError, Signal,
    WidgetId,
};

/// The point the application starts with and Reset restores.
pub const DEFAULT_POINT: (f64, f64) = (0.5, 0.5);

/// The application's state: one signal per coordinate, shared by the picker
/// and the fields.
#[derive(Clone)]
pub struct AppState {
    pub x: Signal<f64>,
    pub y: Signal<f64>,
    /// How many user changes the picker reported.
    pub changes: Signal<u32>,
}

impl AppState {
    pub fn new(ui: &Runtime) -> Self {
        Self {
            x: ui.signal(DEFAULT_POINT.0),
            y: ui.signal(DEFAULT_POINT.1),
            changes: ui.signal(0),
        }
    }

    /// Sets the point, as the owner of the picker.
    pub fn set(&self, x: f64, y: f64) -> Result<(), RuntimeError> {
        self.x.set(x)?;
        self.y.set(y)
    }
}

/// Builds and mounts the application in `ui`, returning its state and root.
pub fn mount(ui: &mut Runtime) -> Result<(AppState, WidgetId), RuntimeError> {
    let state = AppState::new(ui);
    let changes = state.changes.clone();
    let picker = PointPicker::new(state.x.clone(), state.y.clone())
        .name("Point")
        .on_change(move |_, _, _| changes.update(|n| *n += 1));
    let field = |signal: &Signal<f64>| -> Result<DecimalInput, RuntimeError> {
        Ok(DecimalInput::new(signal.clone())
            .range(0.0, 1.0)?
            .step(0.01)?
            .decimal_places(2)?)
    };
    let reset_state = state.clone();
    let reset = Button::new("Reset", move |_| reset_state.set(DEFAULT_POINT.0, DEFAULT_POINT.1));
    let controls = Column::new([
        Field::new(field(&state.x)?.key("x")).label("x").key("x-field"),
        Field::new(field(&state.y)?.key("y")).label("y").key("y-field"),
        reset.key("reset"),
    ]);
    let content = Column::new([
        Label::new("Point picker").key("title"),
        Row::new([picker.key("picker"), controls.key("controls")]).key("main"),
    ]);
    let root = ui.mount_tree(
        Container::new(content.key("content"))
            .padding(MetricRole::ContentMargin)
            .key("app"),
    )?;
    Ok((state, root))
}
