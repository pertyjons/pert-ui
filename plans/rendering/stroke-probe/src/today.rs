//! Today's path through the public facade: strokes recorded as `DrawVector`
//! by the painter, lowered to masks by `VectorRasterizer`, and the CPU
//! renderer. Only strokes at the command scale 1 can be recorded here.
use crate::stroke::{Cap, Frame, Join, Stroke};
use pert_ui::cpu::CpuRenderer;
use pert_ui::{VectorRasterizer, component::*, prelude::*};
use std::cell::RefCell;
use std::rc::Rc;

type Shared = Rc<RefCell<Vec<Stroke>>>;

struct Strokes {
    strokes: Shared,
    tick: ReadSignal<u64>,
}
impl Component for Strokes {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read_paint(&self.tick)?;
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        for stroke in self.strokes.borrow().iter() {
            let mut builder = PathBuilder::new();
            for subpath in &stroke.subpaths {
                let mut points = subpath.points.iter();
                let Some(first) = points.next() else {
                    continue;
                };
                builder = builder.move_to(Point::new(first[0], first[1])?);
                if subpath.points.len() == 1 {
                    builder = builder.line_to(Point::new(first[0], first[1])?);
                }
                for point in points {
                    builder = builder.line_to(Point::new(point[0], point[1])?);
                }
                if subpath.closed {
                    builder = builder.close();
                }
            }
            let style = StrokeStyle::new(Length::new(stroke.width)?)
                .with_cap(match stroke.cap {
                    Cap::Butt => LineCap::Butt,
                    Cap::Round => LineCap::Round,
                    Cap::Square => LineCap::Square,
                })
                .with_join(match stroke.join {
                    Join::Round => LineJoin::Round,
                    Join::Bevel => LineJoin::Bevel,
                    Join::Miter => LineJoin::Miter,
                })
                .with_miter_limit(stroke.miter_limit)?;
            let [r, g, b, a] = stroke.color;
            p.stroke_path(&builder.build()?, style, Color::from_linear(r, g, b, a)?)?;
        }
        Ok(())
    }
}

pub struct Today {
    ui: Runtime,
    strokes: Shared,
    tick: Signal<u64>,
    count: u64,
    pub rasterizer: VectorRasterizer,
    pub cpu: CpuRenderer,
    scale: f64,
}

impl Today {
    pub fn new(frame: Frame) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(
            (f64::from(frame.width) / frame.scale) as f32,
            (f64::from(frame.height) / frame.scale) as f32,
            frame.scale,
        )?)?;
        let strokes: Shared = Rc::default();
        let tick = ui.signal(0_u64);
        ui.mount_tree(Element::new(
            "strokes",
            Strokes {
                strokes: strokes.clone(),
                tick: tick.read_only(),
            },
        ))?;
        Ok(Self {
            ui,
            strokes,
            tick,
            count: 0,
            rasterizer: VectorRasterizer::default(),
            cpu: CpuRenderer::new(),
            scale: frame.scale,
        })
    }

    /// Records `strokes` (command scale 1, no clip) as today's display list.
    pub fn scene(&mut self, strokes: &[Stroke]) -> Result<DisplayList, Box<dyn std::error::Error>> {
        *self.strokes.borrow_mut() = strokes.to_vec();
        self.count += 1;
        self.tick.set(self.count)?;
        self.ui
            .flush(std::time::Duration::from_millis(self.count))?;
        Ok(self.ui.paint()?)
    }

    /// The masks of a rasterized scene with their rectangles in frame pixels
    /// and their straight linear colors.
    pub fn masks(&self, rasterized: &DisplayList) -> Vec<(MaskImage, [f32; 4], [f32; 4])> {
        let s = self.scale as f32;
        rasterized
            .commands()
            .iter()
            .filter_map(|command| match *command {
                DrawCommand::DrawMask {
                    rect, mask, color, ..
                } => Some((
                    rasterized.mask(mask)?.clone(),
                    [
                        rect.x() * s,
                        rect.y() * s,
                        rect.width() * s,
                        rect.height() * s,
                    ],
                    color.channels(),
                )),
                _ => None,
            })
            .collect()
    }
}
