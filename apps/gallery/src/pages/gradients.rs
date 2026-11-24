//! Paint the same stops along a line and concentric ellipses.
use crate::gallery::{page, section};
use pert_ui::{Gradient, component::*};

struct Swatch(Gradient);
impl Component for Swatch {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([480, 88]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        let rect = Rect::new(0., 0., size.width().min(480.), size.height().min(88.))?;
        p.fill_shape(rect, Shape::rounded(Length::new(8.)?), self.0.clone())
    }
}
pub(crate) fn gradients() -> Result<Element, DrawError> {
    let stops = [
        (0., Color::from_srgb8(42, 61, 168, 255)),
        (0.44, Color::from_srgb8(138, 76, 213, 255)),
        (0.75, Color::from_srgb8(243, 102, 138, 255)),
        (1., Color::from_srgb8(255, 202, 112, 255)),
    ];
    let row = |key: &str, title: &str, gradient| {
        section(key, title, [Element::new("swatch", Swatch(gradient))])
    };
    Ok(page([
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
    ]))
}
