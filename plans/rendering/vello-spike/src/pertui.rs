//! The workloads as a pert-ui component, through the public facade only.
use crate::workloads::{self, Item, Rgba};
use pert_ui::{
    Color, Component, DisplayList, DrawError, Element, ImageSampling, Length, LineJoin, Painter,
    Point, RasterImage, ReadSignal, Rect, Runtime, RuntimeError, Shape, Signal, StrokeStyle,
    UpdateContext, Viewport,
};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Panels { panels: usize, plain: usize },
    Polyline { samples: usize },
    Bars,
}

fn color(c: Rgba) -> Color {
    Color::from_srgb8(c[0], c[1], c[2], c[3])
}

fn rect(r: workloads::Rect) -> Result<Rect, DrawError> {
    Ok(Rect::new(r.x, r.y, r.w, r.h)?)
}

struct Content {
    frame: ReadSignal<u64>,
    kind: Kind,
    items: Arc<Vec<Item>>,
    points: Vec<(f32, f32)>,
    bars: Vec<(workloads::Rect, workloads::Rect)>,
    /// One CPU-built image of the whole meter gradient, one texel wide and
    /// one texel per physical row, shared by every bar.
    gradient: Option<RasterImage>,
}

impl Component for Content {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let frame = cx.read_paint(&self.frame)?;
        match self.kind {
            Kind::Panels { .. } => {}
            Kind::Polyline { samples } => self.points = workloads::polyline(frame, samples),
            Kind::Bars => {
                self.bars = workloads::bars(frame);
                if self.gradient.is_none() {
                    let scale = cx.viewport().scale();
                    let rows = (f64::from(workloads::METER.h) * scale).round() as u32;
                    let mut pixels = Vec::with_capacity(rows as usize * 4);
                    for row in 0..rows {
                        // Texel centers sample the gradient where pixel
                        // centers do, from the meter's bottom (t = 0).
                        let t = 1.0 - (row as f32 + 0.5) / rows as f32;
                        pixels.extend_from_slice(&workloads::gradient_at(t));
                    }
                    self.gradient = Some(
                        RasterImage::from_srgba8(1, rows, pixels).expect("gradient image"),
                    );
                }
            }
        }
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let (w, h) = workloads::LOGICAL;
        painter.fill_rect(Rect::new(0.0, 0.0, w as f32, h as f32)?, color(workloads::BACKGROUND));
        match self.kind {
            Kind::Panels { .. } => {
                let border = Length::new(1.0)?;
                for item in self.items.iter() {
                    match *item {
                        Item::Panel(p) => {
                            let shape = Shape::Rounded(Length::new(p.radius)?);
                            painter.fill_shape(rect(p.rect)?, shape, color(p.fill))?;
                            painter.stroke_shape(rect(p.rect)?, shape, border, color(p.border))?;
                        }
                        Item::Plain(r, c) => painter.fill_rect(rect(r)?, color(c)),
                    }
                }
            }
            Kind::Polyline { .. } => {
                let points = self
                    .points
                    .iter()
                    .map(|&(x, y)| Point::new(x, y))
                    .collect::<Result<Vec<_>, _>>()?;
                let style = StrokeStyle::new(Length::new(workloads::LINE_WIDTH)?)
                    .with_join(LineJoin::Round);
                painter.stroke_polyline(&points, style, color(workloads::LINE))?;
            }
            Kind::Bars => {
                let image = self.gradient.as_ref().ok_or(DrawError::CoverageTooLarge)?;
                for &(full, lit) in &self.bars {
                    painter.fill_rect(rect(full)?, color(workloads::TRACK));
                    if lit.h > 0.0 {
                        painter.with_clip(rect(lit)?, |painter| {
                            painter.draw_image(image, rect(full)?, ImageSampling::Linear);
                            Ok::<(), DrawError>(())
                        })?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// A headless runtime holding one workload, advanced one frame at a time.
pub struct Scene {
    ui: Runtime,
    frame: Signal<u64>,
    now: Duration,
}

impl Scene {
    pub fn new(kind: Kind, scale: f64) -> Result<Self, Box<dyn std::error::Error>> {
        let (w, h) = workloads::LOGICAL;
        Self::sized(kind, (w as f32, h as f32), scale)
    }

    /// As [`Self::new`] with another logical viewport (the content stays at
    /// its logical coordinates; what falls outside is cropped).
    pub fn sized(
        kind: Kind,
        (w, h): (f32, f32),
        scale: f64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(w, h, scale)?)?;
        let frame = ui.signal(0_u64);
        let items = match kind {
            Kind::Panels { panels, plain } => Arc::new(workloads::panels(panels, plain)),
            _ => Arc::new(Vec::new()),
        };
        ui.mount_tree(Element::new(
            "content",
            Content {
                frame: frame.read_only(),
                kind,
                items,
                points: Vec::new(),
                bars: Vec::new(),
                gradient: None,
            },
        ))?;
        ui.flush(Duration::ZERO)?;
        Ok(Self { ui, frame, now: Duration::ZERO })
    }

    /// Sets the frame, flushes and paints; returns the scene and the time of
    /// update (signal + flush) and of paint (composition).
    pub fn compose(
        &mut self,
        frame: u64,
    ) -> Result<(DisplayList, Duration, Duration), Box<dyn std::error::Error>> {
        let started = Instant::now();
        self.frame.set(frame)?;
        self.now += Duration::from_micros(16_667);
        self.ui.flush(self.now)?;
        let update = started.elapsed();
        let started = Instant::now();
        let scene = self.ui.paint()?;
        Ok((scene, update, started.elapsed()))
    }
}
