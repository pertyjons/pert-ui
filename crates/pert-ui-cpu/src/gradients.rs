//! Shape paints that vary across the frame. Solid-only shapes retain their run path.
use super::*;
use pert_ui_runtime::{Gradient, GradientId, GradientMapping};

#[derive(Clone, Copy)]
pub(super) enum ShapePaint {
    Solid([f32; 4]),
    Gradient(GradientId, GradientMapping),
}
impl ShapePaint {
    pub(super) fn new(
        scene: &DisplayList,
        paint: Option<pert_ui_runtime::RecordedPaint>,
        mapping: &FrameMapping,
    ) -> Result<Self, CpuRenderError> {
        Ok(match paint {
            None => Self::Solid([0.; 4]),
            Some(Paint::Solid(color)) => Self::Solid(premultiplied(color)),
            Some(Paint::Gradient(id)) => Self::Gradient(
                id,
                scene
                    .gradient(id)
                    .ok_or(CpuRenderError::InvalidReference)?
                    .prepare(mapping)
                    .map_err(DrawError::from)?,
            ),
            _ => return Err(CpuRenderError::Unsupported),
        })
    }
    pub(super) fn visible(self) -> bool {
        !matches!(self,Self::Solid(c) if c[3]==0.)
    }
}
enum Samples<'a> {
    Solid([f32; 4]),
    Direct(&'a Gradient, GradientMapping),
    Rows {
        pixels: Vec<[f32; 4]>,
        left: u32,
        top: u32,
        width: usize,
    },
    Columns {
        gradient: &'a Gradient,
        params: GradientMapping,
        row: Option<u32>,
        colors: [[f32; 4]; 8],
    },
}
impl<'a> Samples<'a> {
    fn new(
        scene: &'a DisplayList,
        paint: ShapePaint,
        [left, top, right, bottom]: [u32; 4],
    ) -> Self {
        match paint {
            ShapePaint::Solid(c) => Self::Solid(c),
            ShapePaint::Gradient(id, params) => {
                let gradient = &scene.gradients()[id.index()];
                if gradient.inner().is_none() && params.coefficients()[3] == 0. {
                    let pixels = (top..bottom.min(top.saturating_add(8)))
                        .flat_map(|y| (left..right).map(move |x| gradient.sample(params, x, y)))
                        .collect();
                    Self::Rows {
                        pixels,
                        left,
                        top,
                        width: (right - left) as usize,
                    }
                } else if gradient.inner().is_none() && params.coefficients()[2] == 0. {
                    Self::Columns {
                        gradient,
                        params,
                        row: None,
                        colors: [[0.; 4]; 8],
                    }
                } else {
                    Self::Direct(gradient, params)
                }
            }
        }
    }
    fn at(&mut self, x: u32, y: u32) -> [f32; 4] {
        match self {
            Self::Solid(c) => *c,
            Self::Direct(g, p) => g.sample(*p, x, y),
            Self::Rows {
                pixels,
                left,
                top,
                width,
            } => pixels[((y - *top) % 8) as usize * *width + (x - *left) as usize],
            Self::Columns {
                gradient,
                params,
                row,
                colors,
            } => {
                if *row != Some(y) {
                    *colors = std::array::from_fn(|x| gradient.sample(*params, x as u32, y));
                    *row = Some(y);
                }
                colors[(x % 8) as usize]
            }
        }
    }
}
impl Frame {
    pub(super) fn draw_gradient_shape(
        &mut self,
        scene: &DisplayList,
        shape: &PreparedShape,
        fill: ShapePaint,
        border: ShapePaint,
    ) {
        let span = shape.span();
        let mut fill = Samples::new(scene, fill, span);
        let mut border = Samples::new(scene, border, span);
        for y in span[1]..span[3] {
            for x in span[0]..span[2] {
                let coverage = shape.coverage(x, y);
                if coverage.outer() > 0. {
                    let f = fill.at(x, y);
                    let b = border.at(x, y);
                    let band = coverage.border();
                    let inside = coverage.outer() - b[3] * band;
                    let source = std::array::from_fn(|i| b[i] * band + f[i] * inside);
                    self.blend(y as usize * self.width + x as usize, source);
                }
            }
        }
    }
}
