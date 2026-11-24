//! Interactive gradient controls, a layered landscape and material samples.
use crate::gallery::{page, section};
use pert_ui::{AppError, Gradient, GradientExtend, component::*, parameter::*, prelude::*};

struct Swatch(Gradient);
impl Component for Swatch {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(space.width().unwrap_or(480.).min(480.), 88.)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let rect = Rect::new(0., 0., size.width().min(480.), size.height().min(88.))?;
        p.fill_shape(rect, Shape::rounded(Length::new(8.)?), self.0.clone())
    }
}
fn swatches() -> Result<Vec<Element>, DrawError> {
    let stops = [
        (0., Color::from_srgb8(42, 61, 168, 255)),
        (0.44, Color::from_srgb8(138, 76, 213, 255)),
        (0.75, Color::from_srgb8(243, 102, 138, 255)),
        (1., Color::from_srgb8(255, 202, 112, 255)),
    ];
    let row = |key: &str, title: &str, gradient| {
        section(key, title, [Element::new("swatch", Swatch(gradient))])
    };
    Ok(vec![
        row(
            "linear",
            "Linear",
            Gradient::linear(Point::default(), Point::new(480., 0.)?, stops)?,
        ),
        row(
            "radial",
            "Radial",
            Gradient::radial(Point::new(155., 44.)?, Size::from([250, 92]), 0., stops)?,
        ),
        row(
            "fixed",
            "Fixed 40 px transition",
            Gradient::linear(Point::default(), Point::new(40., 0.)?, stops)?,
        ),
    ])
}

#[derive(Clone, Copy, PartialEq)]
enum Sky {
    Afterglow,
    Fjord,
    Moonrise,
}

struct LandscapeColors {
    zenith: Color,
    haze: Color,
    light: Color,
    ridge: Color,
    water: Color,
}

fn rgb([r, g, b]: [u8; 3]) -> Color {
    Color::from_srgb8(r, g, b, 255)
}

impl Sky {
    fn next(self) -> Self {
        match self {
            Self::Afterglow => Self::Fjord,
            Self::Fjord => Self::Moonrise,
            Self::Moonrise => Self::Afterglow,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Afterglow => "Afterglow",
            Self::Fjord => "Arctic fjord",
            Self::Moonrise => "Moonrise",
        }
    }
    fn colors(self) -> LandscapeColors {
        let (zenith, haze, light, ridge, water) = match self {
            Self::Afterglow => (
                [47, 34, 98],
                [251, 137, 115],
                [255, 227, 158],
                [87, 55, 102],
                [45, 43, 96],
            ),
            Self::Fjord => (
                [10, 55, 93],
                [111, 216, 191],
                [221, 255, 205],
                [28, 96, 112],
                [14, 59, 87],
            ),
            Self::Moonrise => (
                [20, 20, 58],
                [155, 130, 201],
                [237, 231, 255],
                [63, 60, 108],
                [34, 32, 74],
            ),
        };
        LandscapeColors {
            zenith: rgb(zenith),
            haze: rgb(haze),
            light: rgb(light),
            ridge: rgb(ridge),
            water: rgb(water),
        }
    }
}

struct Landscape {
    palette: ReadSignal<Sky>,
    position: ReadSignal<f32>,
    sky: Sky,
    sun: f32,
}

impl Component for Landscape {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.sky = cx.read_paint(&self.palette)?;
        self.sun = cx.read_paint(&self.position)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, space: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(space.width().unwrap_or(680.), 165.)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        p.with_clip(Rect::from_size(p.size()), |p| self.paint_scene(p))
    }
}

impl Landscape {
    fn paint_scene(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let (w, h) = (size.width(), size.height());
        if w <= 0. || h <= 0. {
            return Ok(());
        }
        let c = self.sky.colors();
        let horizon = h * 0.69;
        let sun = Point::new(
            w * (0.12 + 0.76 * self.sun),
            h * (0.46 - 0.23 * (std::f32::consts::PI * self.sun).sin()),
        )?;
        let sky = Gradient::linear(
            Point::default(),
            Point::new(0., horizon)?,
            [(0., c.zenith), (0.62, c.ridge), (1., c.haze)],
        )?;
        p.fill_rect(Rect::from_size(size), sky);
        let radius = 0.47 * h;
        let halo = Gradient::radial(
            sun,
            Size::new(radius, radius)?,
            0.,
            [
                (0., c.light),
                (0.25, c.light),
                (0.25, Color::from_srgb8(255, 225, 206, 82)),
                (1., Color::from_srgb8(255, 225, 206, 0)),
            ],
        )?;
        p.fill_rect(
            Rect::new(sun.x() - radius, sun.y() - radius, 2. * radius, 2. * radius)?,
            halo,
        );
        for (left, peak, top, right) in [
            (-0.10, 0.12, 0.46, 0.36),
            (0.11, 0.39, 0.59, 0.64),
            (0.49, 0.80, 0.43, 1.13),
            (0.76, 0.97, 0.55, 1.24),
        ] {
            p.fill_triangle(
                [
                    Point::new(w * left, horizon)?,
                    Point::new(w * peak, h * top)?,
                    Point::new(w * right, horizon)?,
                ],
                c.ridge,
            )?;
        }
        let water = Gradient::linear(
            Point::new(0., horizon)?,
            Point::new(0., h)?,
            [(0., c.haze), (0.18, c.ridge), (1., c.water)],
        )?;
        p.fill_rect(Rect::new(0., horizon, w, h - horizon)?, water);
        for (depth, half) in [(0.04, 0.34), (0.11, 0.24), (0.19, 0.16), (0.30, 0.09)] {
            let half = half * radius;
            let y = horizon + (h - horizon) * depth;
            let glint = Gradient::linear(
                Point::new(sun.x() - half, y)?,
                Point::new(sun.x() + half, y)?,
                [(0., c.haze), (0.5, c.light), (1., c.haze)],
            )?;
            p.fill_shape(
                Rect::new(sun.x() - half, y, 2. * half, 1.5)?,
                Shape::rounded(Length::from(1)),
                glint,
            )?;
        }
        Ok(())
    }
}

enum Finish {
    SpectrumRail,
    Enamel,
    OpalBorder,
}

/// Paint the backdrop while the ordinary child owns input, focus and semantics.
struct GradientControl {
    child: Option<Element>,
    finish: Finish,
}
impl Component for GradientControl {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let (w, h) = (size.width(), size.height());
        if w <= 0. || h <= 0. {
            return Ok(());
        }
        let (bounds, stops) = match self.finish {
            Finish::OpalBorder if p.focus_within() => (
                Rect::from_size(size),
                [rgb([28, 101, 105]), rgb([62, 76, 144]), rgb([119, 51, 114])],
            ),
            Finish::OpalBorder => {
                return p.fill_shape(
                    Rect::from_size(size),
                    Shape::rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                    p.color(ColorRole::Button),
                );
            }
            Finish::SpectrumRail => {
                let height = h.min(20.);
                (
                    Rect::new(0., (h - height) / 2., w, height)?,
                    [
                        rgb([99, 83, 184]),
                        rgb([237, 128, 139]),
                        rgb([255, 205, 108]),
                    ],
                )
            }
            Finish::Enamel => (
                Rect::from_size(size),
                [rgb([67, 47, 143]), rgb([122, 56, 139]), rgb([141, 62, 84])],
            ),
        };
        let gradient = Gradient::linear(
            Point::default(),
            Point::new(w, 0.)?,
            [(0., stops[0]), (0.52, stops[1]), (1., stops[2])],
        )?;
        p.fill_shape(
            bounds,
            Shape::rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
            gradient,
        )
    }
    fn paint_foreground(&self, p: &mut Painter) -> Result<(), DrawError> {
        if !matches!(self.finish, Finish::OpalBorder) {
            return Ok(());
        }
        let size = p.size();
        // Leave the button's keyboard focus ring visible outside the gradient.
        let inset = if p.focus_within() {
            p.metric(MetricRole::FocusWidth)
        } else {
            0.
        };
        let w = size.width() - 2. * inset;
        let h = size.height() - 2. * inset;
        if w <= 0. || h <= 0. {
            return Ok(());
        }
        let rim = Gradient::linear(
            Point::new(inset, inset)?,
            Point::new(inset + w, inset + h)?,
            [
                (0., rgb([85, 236, 210])),
                (0.42, rgb([135, 177, 255])),
                (0.74, rgb([234, 150, 246])),
                (1., rgb([255, 204, 125])),
            ],
        )?;
        p.draw_shape(
            Rect::new(inset, inset, w, h)?,
            Shape::rounded(Length::new(
                (p.metric(MetricRole::CornerRadius) - inset).max(0.),
            )?),
            ShapeStyle::border(Length::from(2), rim),
        )
    }
}

const INITIAL_SUN: f32 = 0.68;

pub(crate) fn gradients(ui: &mut Runtime, id: ParameterId) -> Result<Element, AppError> {
    let palette = ui.signal(Sky::Afterglow);
    let (position, host) = ui.parameter(
        id,
        ParameterSpec::new("Sun position", 0_f32, 1., INITIAL_SUN)?,
    )?;
    let landscape = Element::new(
        "landscape",
        Landscape {
            palette: palette.read_only(),
            position: position.snapshot().map(|s| s.effective)?,
            sky: Sky::Afterglow,
            sun: INITIAL_SUN,
        },
    );
    let button_palette = palette.clone();
    let button = Button::new("Change sky", move |_| {
        button_palette.set(button_palette.get()?.next())
    })
    .frameless();
    let controls = Element::new(
        "sky-controls",
        Wrap::new([
            Element::new(
                "palette-button-surface",
                GradientControl {
                    child: Some(Element::new("next-sky", button)),
                    finish: Finish::Enamel,
                },
            ),
            Element::new(
                "reset-button-surface",
                GradientControl {
                    child: Some(Element::new(
                        "reset-sun",
                        Button::new("Reset sun", move |_| host.write_base(INITIAL_SUN)).frameless(),
                    )),
                    finish: Finish::OpalBorder,
                },
            ),
            Element::new(
                "sky-name",
                Label::new(palette.map(|p| p.name().to_owned())?),
            ),
        ]),
    );
    let slider = Element::new(
        "sun-rail",
        GradientControl {
            child: Some(Element::new("sun-position", Slider::new(position))),
            finish: Finish::SpectrumRail,
        },
    );
    let mut sections = vec![section(
        "landscape-demo",
        "Horizon",
        [
            landscape,
            controls,
            Element::new("sun-label", Label::new("Sun position")),
            slider,
        ],
    )];

    let metal = Gradient::linear(
        Point::default(),
        Point::new(65., 28.)?,
        [
            (0., rgb([32, 47, 62])),
            (0.36, rgb([114, 151, 160])),
            (0.53, rgb([231, 240, 228])),
            (1., rgb([70, 86, 106])),
        ],
    )
    .map_err(DrawError::from)?
    .with_extend(GradientExtend::Reflect);
    let ink = rgb([35, 37, 66]);
    let coral = rgb([245, 136, 110]);
    let stripes = Gradient::linear(
        Point::default(),
        Point::new(30., 15.)?,
        [(0., ink), (0.55, ink), (0.55, coral), (1., coral)],
    )
    .map_err(DrawError::from)?
    .with_extend(GradientExtend::Repeat);
    let material = |key: &str, title: &str, paint| {
        Element::new(
            key,
            Column::new([
                Element::new("name", Label::new(title)),
                Element::new("sample", Swatch(paint)),
            ]),
        )
    };
    sections.push(section(
        "materials",
        "Pattern & finish",
        [Element::new(
            "finishes",
            Wrap::new([
                material("metal", "Brushed silver · reflected light", metal),
                material("stripes", "Screen print · repeating hard stops", stripes),
            ]),
        )],
    ));
    sections.extend(swatches()?);
    Ok(page(sections))
}
