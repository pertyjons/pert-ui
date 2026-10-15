//! Regression checks for selectable text rendering and retained scrolling text.
use super::support::{click, find, mounted, open, settle};
use pert_ui::{
    Color, Column, Component, Container, DisplayList, DrawCommand, DrawError, Element, FrameImage,
    Icon, Label, MaskImage, Painter, Point, PointerEvent, PointerKind, Rect, RichText, Runtime,
    RuntimeError, ScrollArea, Size, TextColor, TextRenderingMode, TextStyle, UpdateContext,
    Viewport, cpu::CpuRenderer,
};
use std::time::Duration;
use std::{cell::Cell, rc::Rc};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

struct LiveText;

impl Component for LiveText {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 400.0, 70.0)?, Color::WHITE);
        painter
            .text("Hamburgefonts 0123456789 Il1 O0", None, TextStyle::Body)?
            .paint(painter, 10.25, 10.25, Color::from_srgb8(0, 0, 0, 255))
    }
}

struct PhaseProbeText(f32);

impl Component for PhaseProbeText {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_rect(Rect::new(0.0, 0.0, 400.0, 70.0)?, Color::WHITE);
        painter
            .text("Hamburgefonts 0123456789 Il1 O0", None, TextStyle::Body)?
            .paint(painter, 10.25, self.0, Color::from_srgb8(0, 0, 0, 255))
    }
}

struct RichPhaseProbe {
    x: f32,
    y: f32,
    icon: Icon,
}

struct PaintCounter(Rc<Cell<usize>>);

struct LongText(usize);

impl Component for LongText {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter
            .text(&"H".repeat(self.0), None, TextStyle::Body)?
            .paint(painter, 0.75, 0.75, Color::from_srgb8(0, 0, 0, 255))
    }
}

#[test]
fn phase_line_accepts_text_at_line_mask_raster_limit() -> TestResult<()> {
    let paints = |count, mode| -> TestResult<bool> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(300.0, 80.0, 1.0)?)?;
        ui.ui_settings().set_text_rendering(mode)?;
        ui.mount("long", LongText(count))?;
        ui.flush(Duration::ZERO)?;
        Ok(ui.paint().is_ok())
    };
    let mut accepted = 1;
    let mut rejected = 2000;
    assert!(paints(accepted, TextRenderingMode::LineMask)?);
    assert!(!paints(rejected, TextRenderingMode::LineMask)?);
    while rejected - accepted > 1 {
        let middle = (accepted + rejected) / 2;
        if paints(middle, TextRenderingMode::LineMask)? {
            accepted = middle;
        } else {
            rejected = middle;
        }
    }
    assert!(
        paints(accepted, TextRenderingMode::PhaseLine)?,
        "phase mode rejected {accepted} glyphs accepted by the default"
    );
    Ok(())
}

impl Component for PaintCounter {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn measure(
        &self,
        _: &pert_ui::MeasureContext<'_>,
        _: pert_ui::Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::from([250, 24]))
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.0.set(self.0.get() + 1);
        painter.text("Moving text", None, TextStyle::Body)?.paint(
            painter,
            0.0,
            0.0,
            Color::from_srgb8(0, 0, 0, 255),
        )
    }
}

#[test]
fn phase_line_reuses_local_commands_after_whole_pixel_scroll() -> TestResult<()> {
    let count = Rc::new(Cell::new(0));
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    ui.ui_settings()
        .set_text_rendering(TextRenderingMode::PhaseLine)?;
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "column",
            Column::new([
                Element::new(
                    "lead",
                    Container::default().preferred_size(Size::from([150, 60])),
                ),
                Element::new("text", PaintCounter(count.clone())),
                Element::new(
                    "tail",
                    Container::default().preferred_size(Size::from([150, 300])),
                ),
            ]),
        )),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let original = count.get();
    for delta in [20.0, 1.0] {
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 10.0)?,
            PointerKind::Scroll(Point::new(0.0, delta)?),
        ))?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        assert_eq!(count.get(), original, "whole-pixel scroll repainted text");
    }
    ui.pointer_input(PointerEvent::new(
        Point::new(10.0, 10.0)?,
        PointerKind::Scroll(Point::new(0.0, 0.25)?),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    assert_eq!(
        count.get(),
        original + 1,
        "fractional scroll did not repaint text"
    );
    Ok(())
}

impl Component for RichPhaseProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let text = RichText::new().text("H").gap().tinted_icon(
            self.icon,
            TextColor::Literal(Color::from_srgb8(255, 0, 0, 255)),
        );
        painter.rich_text(&text, None, TextStyle::Body)?.paint(
            painter,
            self.x,
            self.y,
            Color::from_srgb8(0, 0, 0, 255),
        )
    }
}

fn brush_centers(scene: &DisplayList) -> TestResult<[(f64, f64); 2]> {
    let mut centers = [None, None];
    let scale = scene.viewport().scale();
    for command in scene.commands() {
        let DrawCommand::DrawMask {
            rect, mask, color, ..
        } = command
        else {
            continue;
        };
        let brush = if *color == Color::from_srgb8(0, 0, 0, 255) {
            0
        } else if *color == Color::from_srgb8(255, 0, 0, 255) {
            1
        } else {
            continue;
        };
        let image = scene.mask(*mask).ok_or("missing rich text mask")?;
        let width = image.width() as usize;
        let mut total = 0.0;
        let mut x_sum = 0.0;
        let mut y_sum = 0.0;
        for (i, coverage) in image.pixels().iter().enumerate() {
            let weight = f64::from(*coverage);
            total += weight;
            x_sum += (i % width) as f64 * weight;
            y_sum += (i / width) as f64 * weight;
        }
        if total > 0.0 {
            centers[brush] = Some((
                f64::from(rect.x()) * scale + x_sum / total,
                f64::from(rect.y()) * scale + y_sum / total,
            ));
        }
    }
    Ok([
        centers[0].ok_or("missing text brush")?,
        centers[1].ok_or("missing icon brush")?,
    ])
}

#[test]
fn inline_icon_tracks_text_across_pixel_phases_and_zoom() -> TestResult<()> {
    for icon in [Icon::Settings, Icon::Search, Icon::Save, Icon::Warning] {
        for scale in [0.75, 0.9, 1.0, 1.25, 1.5] {
            for physical_x in [10.0, 10.25, 10.5, 10.75] {
                for physical_y in [10.0, 10.25, 10.5, 10.75] {
                    let mut centers = Vec::new();
                    for mode in [TextRenderingMode::LineMask, TextRenderingMode::PhaseLine] {
                        let mut ui = Runtime::new();
                        ui.set_viewport(Viewport::new(300.0, 70.0, scale)?)?;
                        ui.ui_settings().set_text_rendering(mode)?;
                        ui.mount(
                            "rich",
                            RichPhaseProbe {
                                x: (physical_x / scale) as f32,
                                y: (physical_y / scale) as f32,
                                icon,
                            },
                        )?;
                        ui.flush(Duration::ZERO)?;
                        let scene = ui.paint()?;
                        if mode == TextRenderingMode::PhaseLine {
                            for command in scene.commands() {
                                if let DrawCommand::DrawMask { rect, .. } = command {
                                    for coordinate in [rect.x(), rect.y()] {
                                        let physical = f64::from(coordinate) * scale;
                                        assert!((physical - physical.round()).abs() < 0.001);
                                    }
                                }
                            }
                        }
                        centers.push(brush_centers(&scene)?);
                    }
                    for brush in 0..2 {
                        for axis in 0..2 {
                            let coordinate = |point: (f64, f64)| {
                                if axis == 0 { point.0 } else { point.1 }
                            };
                            let shift = (coordinate(centers[1][brush])
                                - coordinate(centers[0][brush]))
                            .abs();
                            assert!(
                                shift < 0.3,
                                "{icon:?} brush {brush} shifted {shift:.3} physical px on axis {axis} at scale {scale}, origin {physical_x}/{physical_y}: {centers:?}"
                            );
                        }
                    }
                    for axis in 0..2 {
                        let relative = |pair: [(f64, f64); 2]| {
                            let coordinate =
                                |point: (f64, f64)| if axis == 0 { point.0 } else { point.1 };
                            coordinate(pair[1]) - coordinate(pair[0])
                        };
                        let drift = (relative(centers[1]) - relative(centers[0])).abs();
                        assert!(
                            drift < 0.25,
                            "{icon:?} icon/text drift {drift:.3} physical px on axis {axis} at scale {scale}, origin {physical_x}/{physical_y}: {centers:?}"
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn live_text_modes_change_coverage_without_adding_commands() -> TestResult<()> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 70.0, 1.0)?)?;
    ui.mount("text", LiveText)?;
    ui.flush(Duration::ZERO)?;
    let mut frames = Vec::new();
    let mut counts = Vec::new();
    for mode in [TextRenderingMode::LineMask, TextRenderingMode::PhaseLine] {
        ui.ui_settings().set_text_rendering(mode)?;
        ui.flush(Duration::ZERO)?;
        let scene = ui.paint()?;
        counts.push(
            scene
                .commands()
                .iter()
                .filter(|command| matches!(command, DrawCommand::DrawMask { .. }))
                .count(),
        );
        frames.push(CpuRenderer::new().render(&scene)?);
    }
    assert_eq!(counts[0], 1);
    assert_eq!(counts[1], 1);
    assert_ne!(frames[0].pixels(), frames[1].pixels());
    Ok(())
}

fn vertical_ink_center(frame: &FrameImage) -> f64 {
    let width = frame.width() as usize;
    let rows: Vec<u64> = frame
        .pixels()
        .chunks_exact(width * 4)
        .map(|row| {
            row.as_chunks::<4>()
                .0
                .iter()
                .map(|pixel| u64::from(255 - pixel[0]))
                .sum()
        })
        .collect();
    let total: u64 = rows.iter().sum();
    assert!(total > 0);
    rows.iter()
        .enumerate()
        .map(|(y, weight)| y as f64 * *weight as f64)
        .sum::<f64>()
        / total as f64
}

#[test]
fn phase_modes_keep_vertical_ink_aligned_with_line_mask() -> TestResult<()> {
    for scale in [0.75, 0.9, 1.0, 1.25, 1.5] {
        let mut previous: Option<Vec<f64>> = None;
        for physical_y in [10.0, 10.25, 10.5, 10.75, 11.0] {
            let y = physical_y / scale;
            let mut ui = Runtime::new();
            ui.set_viewport(Viewport::new(400.0, 70.0, scale as f64)?)?;
            ui.mount("text", PhaseProbeText(y))?;
            ui.flush(Duration::ZERO)?;
            let mut centers = Vec::new();
            for mode in [TextRenderingMode::LineMask, TextRenderingMode::PhaseLine] {
                ui.ui_settings().set_text_rendering(mode)?;
                ui.flush(Duration::ZERO)?;
                let frame = CpuRenderer::new().render(&ui.paint()?)?;
                centers.push(vertical_ink_center(&frame));
            }
            assert!(
                (centers[0] - centers[1]).abs() < 0.25,
                "at scale={scale}, y={y}, phase line ink moved from {} to {}",
                centers[0],
                centers[1]
            );
            if let Some(previous) = &previous {
                assert!(
                    centers[1] > previous[1],
                    "at scale={scale}, the phase text moved upward as y increased: {previous:?} -> {centers:?}"
                );
            }
            previous = Some(centers);
        }
    }
    Ok(())
}

#[test]
fn phase_line_repaints_when_scroll_changes_physical_origin() -> TestResult<()> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 100.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "column",
            Column::new([
                Element::new(
                    "lead",
                    Container::default().preferred_size(Size::from([150, 60])),
                ),
                Element::new("label", Label::new("Hamburgefonts 0123456789")),
                Element::new(
                    "tail",
                    Container::default().preferred_size(Size::from([150, 300])),
                ),
            ]),
        )),
    ))?;
    ui.ui_settings()
        .set_text_rendering(TextRenderingMode::PhaseLine)?;
    ui.flush(Duration::ZERO)?;
    let mut masks = Vec::new();
    for delta in [20.0, 0.5, 0.5] {
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 10.0)?,
            PointerKind::Scroll(Point::new(0.0, delta)?),
        ))?;
        ui.flush(Duration::ZERO)?;
        let (mask, phase) = source_mask(&ui.paint()?)?;
        assert!(phase.iter().all(|value| value.abs() < 0.001));
        masks.push(mask.content_key());
    }
    assert_ne!(masks[0], masks[1]);
    assert_ne!(masks[1], masks[2]);
    Ok(())
}

#[test]
fn gallery_switches_live_text_modes() -> TestResult<()> {
    let (mut ui, root) = mounted();
    ui.set_viewport(Viewport::new(1280.0, 800.0, 1.0)?)?;
    open(&mut ui, &root, "text-rendering");
    settle(&mut ui);
    for (key, mode) in [
        ("live-phase-line", TextRenderingMode::PhaseLine),
        ("live-line-mask", TextRenderingMode::LineMask),
    ] {
        let button = find(&ui, &root, key);
        click(&mut ui, &button);
        assert_eq!(ui.ui_settings().text_rendering()?, mode);
    }
    Ok(())
}

fn source_mask(scene: &DisplayList) -> TestResult<(MaskImage, [f32; 2])> {
    let (rect, mask) = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask { rect, mask, .. } => Some((*rect, *mask)),
            _ => None,
        })
        .next_back()
        .ok_or("source scene has no text mask")?;
    let mask = scene.mask(mask).ok_or("source mask is missing")?.clone();
    let viewport = scene.viewport();
    let physical = viewport.frame_extent();
    let scale_x = physical.width() as f64 / viewport.width() as f64;
    let scale_y = physical.height() as f64 / viewport.height() as f64;
    Ok((
        mask,
        [
            (rect.x() as f64 * scale_x).rem_euclid(1.0) as f32,
            (rect.y() as f64 * scale_y).rem_euclid(1.0) as f32,
        ],
    ))
}
