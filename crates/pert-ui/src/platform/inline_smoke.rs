//! Scripted inline editing presented in a real native window.
use crate::host::HostIntegration;
use crate::*;
use std::cell::Cell;

/// Consumer-style dial proving `Painter::stroke_arc`/`fill_triangle` rasterize in
/// a real window. It reads a parameter and never edits it.
struct ArcDial {
    parameter: Parameter<f32>,
    value: ReadSignal<ParameterSnapshot<f32>>,
    normalized: Cell<f32>,
}
impl Component for ArcDial {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let base = cx.read_paint(&self.value)?.base;
        // Metadata can change, so the live specification maps the value.
        let normalized = self.parameter.spec()?.normalize(&base)?;
        self.normalized.set(normalized.get());
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([48, 48]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let side = size.width().min(size.height());
        let half = side / 2.0;
        let center = Point::new(half, half)?;
        let radius = Length::new(side * 0.44)?;
        let stroke = Length::new((side * 0.06).max(1.0))?;
        let n = self.normalized.get();
        p.stroke_arc(
            center,
            radius,
            135.0,
            270.0,
            stroke,
            p.color(ColorRole::Outline),
        )?;
        let active = p.color(ColorRole::Primary);
        p.stroke_arc(center, radius, 135.0, 270.0 * n, stroke, active)?;
        let angle = (225.0 - 270.0 * n).to_radians();
        let (dx, dy) = (angle.cos(), -angle.sin());
        let point = |radial: f32, tangent: f32| {
            Point::new(
                half + side * (dx * radial - dy * tangent),
                half + side * (dy * radial + dx * tangent),
            )
            .map_err(RuntimeError::from)
        };
        p.fill_triangle(
            [point(0.46, 0.0)?, point(0.33, -0.065)?, point(0.33, 0.065)?],
            active,
        )
    }
}
pub(super) struct InlineSmoke {
    root: WidgetId,
    integer: Signal<i64>,
    decimal: Signal<f64>,
    slider: Parameter<f32>,
    _host: ParameterHost<f32>,
    fields: Vec<WidgetId>,
}
impl InlineSmoke {
    pub(super) fn unmount(self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        ui.unmount(&self.root)
    }
    pub(super) fn new(ui: &mut Runtime) -> Result<Self, AppError> {
        let integer = ui.signal(8_i64);
        let decimal = ui.signal(0.125_f64);
        let (slider, host) = ui.parameter(
            ParameterId::new(9, 9, "inline-smoke").unwrap(),
            ParameterSpec::new("Glide", 0.0, 2.0, 0.9).unwrap(),
        )?;
        let format = NumberFormat::new(
            "9999.0 ms",
            |v: f64| format!("{:.1}", v * 1000.0),
            |s| {
                s.parse::<f64>()
                    .map(|v| v / 1000.0)
                    .map_err(|_| NumberInputError::InvalidNumber)
            },
        );
        let root = ui.mount_tree(Element::new(
            "inline-smoke",
            Row::new([
                Element::new(
                    "inline-slider",
                    InlineSlider::new(slider.clone()).label("Glide").unit("s"),
                ),
                Element::new(
                    "inline-integer",
                    InlineInteger::new(integer.clone())
                        .range(0, 100)
                        .unwrap()
                        .label("Vol")
                        .unit("%"),
                ),
                Element::new(
                    "inline-decimal",
                    InlineDecimal::new(decimal.clone())
                        .range(0.0, 2.0)
                        .unwrap()
                        .label("Delay")
                        .unit("ms")
                        .format(format),
                ),
                Element::new(
                    "arc-dial",
                    ArcDial {
                        value: slider.snapshot(),
                        parameter: slider.clone(),
                        normalized: Cell::new(0.0),
                    },
                ),
            ]),
        ))?;
        ui.flush(ui.inspect().at)?;
        let fields = ui.children(&root)?;
        Ok(Self {
            root,
            integer,
            decimal,
            slider,
            _host: host,
            fields,
        })
    }
    pub(super) fn step(&mut self, ui: &mut Runtime, stage: u8) -> Result<(), RuntimeError> {
        fn key(ui: &mut Runtime, k: KeyCode) -> Result<(), RuntimeError> {
            ui.key_input(k, KeyAction::Press)?;
            ui.key_input(k, KeyAction::Release)?;
            Ok(())
        }
        match stage {
            29 => {
                ui.focus(&self.fields[1])?;
                key(ui, KeyCode::F2)?;
                ui.text_input("42")?;
            }
            30 => {
                assert!(ui.text_input_state().is_some());
                key(ui, KeyCode::Enter)?;
                assert_eq!(self.integer.get()?, 42);
                ui.focus(&self.fields[2])?;
                key(ui, KeyCode::F2)?;
                ui.text_input("250")?;
            }
            31 => {
                key(ui, KeyCode::Enter)?;
                assert_eq!(self.decimal.get()?, 0.25);
                ui.focus(&self.fields[0])?;
                key(ui, KeyCode::F2)?;
                ui.text_input("1.25")?;
            }
            32 => {
                key(ui, KeyCode::Enter)?;
                assert_eq!(self.slider.snapshot().get()?.base, 1.25);
                ui.set_visible(&self.root, false)?;
            }
            _ => {}
        }
        ui.flush(ui.inspect().at)?;
        Ok(())
    }
}
