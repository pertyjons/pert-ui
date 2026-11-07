//! Parity-lead diagnosis: the polyline's prepared mask near the worst pixel.
#[path = "../workloads.rs"]
#[allow(dead_code)]
mod workloads;
use pert_ui::{
    Color, Component, DrawCommand, DrawError, Element, Length, LineJoin, Painter, Point,
    Runtime, RuntimeError, StrokeStyle, UpdateContext, VectorRasterizer, Viewport,
};

struct Line(Vec<(f32, f32)>);
impl Component for Line {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let points = self.0.iter().map(|&(x, y)| Point::new(x, y)).collect::<Result<Vec<_>, _>>()?;
        let style = StrokeStyle::new(Length::new(workloads::LINE_WIDTH)?).with_join(LineJoin::Round);
        p.stroke_polyline(&points, style, Color::from_srgb8(100, 170, 255, 255))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let frame: u64 = std::env::args().nth(1).map_or(Ok(219), |a| a.parse())?;
    let scale = 2.0;
    let points = workloads::polyline(frame, 800);
    println!("first points {:?}", &points[..3]);
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1280.0, 800.0, scale)?)?;
    ui.mount_tree(Element::new("line", Line(points)))?;
    ui.flush(std::time::Duration::ZERO)?;
    let scene = ui.paint()?;
    let prepared = VectorRasterizer::default().prepare(&scene)?;
    for c in prepared.commands() {
        if let &DrawCommand::DrawMask { rect, mask, .. } = c {
            let m = prepared.mask(mask).unwrap();
            let [x, y, w, h] = rect.values();
            println!("mask rect logical ({x}, {y}, {w}, {h}) physical ({}, {}, {}, {}); mask {}x{}",
                x * scale as f32, y * scale as f32, w * scale as f32, h * scale as f32, m.width(), m.height());
            let (px, py) = (x as f64 * scale, y as f64 * scale);
            for row in 600..620usize {
                let my = row as f64 - py;
                let mut line = format!("row {row} (mask row {my:.2}):");
                for col in 474..486usize {
                    let mx = col as f64 - px;
                    let v = if mx >= 0.0 && my >= 0.0 && (mx as u32) < m.width() && (my as u32) < m.height() {
                        m.pixels()[my as usize * m.width() as usize + mx as usize] as i32
                    } else { -1 };
                    line += &format!(" {v:4}");
                }
                println!("{line}");
            }
        }
    }
    Ok(())
}
