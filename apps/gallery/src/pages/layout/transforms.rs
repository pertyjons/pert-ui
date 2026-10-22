//! The Transforms page: the built-in `ZoomView`, a consumer-written timeline
//! that zooms each axis through an `AxisMapping`, and mapped shapes.
use crate::gallery::{label, page, section, stage};
use pert_ui::*;

/// A timeline that zooms and pans in time while its rows keep their height,
/// written as a consumer would write one.
///
/// Each zooming axis is an `AxisMapping`, from seconds and from amplitude to
/// local pixels. The component maps positions itself and draws undistorted, so
/// text, strokes and row heights stay the same at every zoom: per-axis zoom is
/// a mapping, not a view transform.
struct Timeline {
    time: AxisMapping,
    level: AxisMapping,
    readout: Signal<String>,
    drag: Option<(f32, AxisMapping)>,
    width: std::cell::Cell<f32>,
}
impl Timeline {
    const RULER: f32 = 24.0;
    const LANE: f32 = 120.0;
    const CLIPS: f32 = 32.0;
    const GAP: f32 = 4.0;
    /// One hour of material; the view starts on its first twenty seconds.
    const LENGTH: f64 = 3600.0;
    const CLIP_LIST: [(f64, f64, &'static str); 6] = [
        (1.0, 6.5, "Intro"),
        (7.0, 15.0, "Verse"),
        (15.0, 19.5, "Chorus"),
        (600.0, 900.0, "Long take"),
        (1800.25, 1800.5, "Click"),
        (3500.0, 3600.0, "Outro"),
    ];
    /// Scale bounds in pixels per second: an hour in 360 pixels at one end, a
    /// millisecond over twenty pixels at the other.
    const MIN_ZOOM: f64 = 0.1;
    const MAX_ZOOM: f64 = 20_000.0;

    fn new(readout: Signal<String>) -> Result<Self, RuntimeError> {
        let mapping = |start, scale| AxisMapping::new(start, scale);
        Ok(Self {
            time: mapping(0.0, 32.0)?,
            // Amplitude 1 at the lane's top and -1 at its bottom: the axis maps
            // the negated amplitude, which grows downward like pixels do.
            level: mapping(-1.0, f64::from(Self::LANE) / 2.0)?,
            readout,
            drag: None,
            width: std::cell::Cell::new(640.0),
        })
    }
    /// A synthetic signal: a slow swell over a steady tone.
    fn signal(time: f64) -> f64 {
        let swell = 0.5 + 0.5 * (std::f64::consts::TAU * 0.01 * time).sin();
        0.7 * swell * (std::f64::consts::TAU * 0.5 * time).sin()
            + 0.2 * (std::f64::consts::TAU * 7.0 * time).sin()
    }
    fn show(
        &mut self,
        cx: &mut EventContext,
        time: AxisMapping,
        level: AxisMapping,
    ) -> Result<(), RuntimeError> {
        if (time, level) == (self.time, self.level) {
            return Ok(());
        }
        (self.time, self.level) = (time, level);
        // The start and the scale, which do not depend on the allocation; the
        // end follows from them and the timeline's width.
        self.readout.set(format!(
            "From {} at {} px/s, amplitude ×{:.1}",
            seconds(time.start()),
            if time.pixels_per_unit() < 10.0 {
                format!("{:.2}", time.pixels_per_unit())
            } else {
                format!("{:.0}", time.pixels_per_unit())
            },
            level.pixels_per_unit() / f64::from(Self::LANE / 2.0)
        ))?;
        // Paint only: the component's geometry does not depend on the mapping.
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn zoom_level(&self, factor: f64) -> AxisMapping {
        let scale = (self.level.pixels_per_unit() * factor)
            .clamp(f64::from(Self::LANE) / 2.0, f64::from(Self::LANE) * 8.0);
        self.level
            .zoomed_at(scale, f64::from(Self::LANE) / 2.0)
            .unwrap_or(self.level)
    }
    fn zoom_time(&self, factor: f64, anchor: f32) -> AxisMapping {
        let scale = (self.time.pixels_per_unit() * factor).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        self.time
            .zoomed_at(scale, f64::from(anchor))
            .unwrap_or(self.time)
    }
}
/// A time in seconds, as minutes and seconds with as many decimals as help.
fn seconds(time: f64) -> String {
    let sign = if time < 0.0 { "-" } else { "" };
    // Whole milliseconds first, so 59.9999 s reads 1:00.000 rather than 0:60.000.
    let millis = (time.abs() * 1000.0).round();
    let minutes = (millis / 60_000.0).floor();
    format!(
        "{sign}{minutes:.0}:{:06.3}",
        (millis - 60_000.0 * minutes) / 1000.0
    )
}
impl Component for Timeline {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            available.width().unwrap_or(640.0),
            Self::RULER + Self::LANE + Self::CLIPS + 2.0 * Self::GAP,
        )?)
    }
    fn arrange(&self, _: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.width.set(size.width());
        Ok(())
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.drag.is_some().then_some(CursorIcon::Grabbing)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(InputOutcome::Ignored);
        };
        let modifiers = cx.modifiers();
        let primary = modifiers.primary();
        match event.kind() {
            PointerKind::Scroll(delta) if primary && modifiers.shift => {
                // Amplitude zooms around the lane's middle; time stays put.
                let level = self.zoom_level(f64::from((-delta.y() / 200.0).exp2()));
                self.show(cx, self.time, level)?;
            }
            PointerKind::Scroll(delta) if primary => {
                let time = self.zoom_time(f64::from((-delta.y() / 200.0).exp2()), point.x());
                self.show(cx, time, self.level)?;
            }
            PointerKind::Scroll(delta) => {
                let pan = if delta.x() == 0.0 {
                    delta.y()
                } else {
                    delta.x()
                };
                let time = self.time.panned(f64::from(-pan)).unwrap_or(self.time);
                self.show(cx, time, self.level)?;
            }
            PointerKind::Pinch(factor) if modifiers.shift => {
                let level = self.zoom_level(f64::from(factor.get()));
                self.show(cx, self.time, level)?;
            }
            PointerKind::Pinch(factor) => {
                let time = self.zoom_time(f64::from(factor.get()), point.x());
                self.show(cx, time, self.level)?;
            }
            PointerKind::Press(PointerButton::Primary) if cx.pointer_inside() => {
                cx.capture_pointer()?;
                cx.request_focus();
                self.drag = Some((point.x(), self.time));
            }
            PointerKind::Move => {
                let Some((start, time)) = self.drag else {
                    return Ok(InputOutcome::Ignored);
                };
                let time = time
                    .panned(f64::from(point.x() - start))
                    .unwrap_or(self.time);
                self.show(cx, time, self.level)?;
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.take().is_some() => {
                cx.release_pointer();
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.code == KeyCode::Home && event.transition == KeyTransition::Pressed {
            // The whole hour at any width, with the amplitude reset; only
            // gestures are held within the zoom limits.
            let width = f64::from(self.width.get()).max(1.0);
            let whole = AxisMapping::new(0.0, width / Self::LENGTH)?;
            let level = AxisMapping::new(-1.0, f64::from(Self::LANE) / 2.0)?;
            self.show(cx, whole, level)?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.drag = None;
        cx.invalidate(Invalidation::Paint);
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let width = size.width();
        let hairline = 1.0 / painter.viewport().scale() as f32;
        let (lane_top, clips_top) = (
            Self::RULER + Self::GAP,
            Self::RULER + Self::LANE + 2.0 * Self::GAP,
        );
        painter.fill_rect(
            Rect::new(0.0, lane_top, width, Self::LANE)?,
            painter.color(ColorRole::SurfaceContainer),
        );
        // Ruler: marks at least 72 pixels apart, labelled with the time.
        let ink = painter.color(ColorRole::OnSurface);
        let outline = painter.color(ColorRole::Outline);
        for mark in self.time.ticks(f64::from(width), 72.0) {
            let x = self.time.pixel(mark) as f32;
            painter.fill_rect(
                Rect::new(x, 0.0, hairline, Self::RULER + Self::LANE + Self::GAP)?,
                outline,
            );
            let text = painter.text(&seconds(mark), None, TextStyle::Caption)?;
            text.paint(painter, x + 4.0, 2.0, ink)?;
        }
        // The signal sampled every two pixels across the view; the stroke keeps
        // its width at every zoom because only positions are mapped.
        let points = (0..=(width / 2.0) as usize)
            .map(|index| {
                let x = index as f32 * 2.0;
                let amplitude = Self::signal(self.time.value(f64::from(x)));
                let y = (self.level.pixel(-amplitude) as f32).clamp(0.0, Self::LANE);
                Point::new(x, lane_top + y)
            })
            .collect::<Result<Vec<_>, _>>()?;
        painter.stroke_polyline(
            &points,
            StrokeStyle::new(Length::new(1.5)?).with_join(LineJoin::Round),
            painter.color(ColorRole::Primary),
        )?;
        // Clips keep the row's height; only their horizontal extent is mapped.
        for (start, end, name) in Self::CLIP_LIST {
            let left = (self.time.pixel(start) as f32).max(-1.0);
            let right = (self.time.pixel(end) as f32).min(width + 1.0);
            if right <= left {
                continue;
            }
            let clip = Rect::new(left, clips_top, (right - left).max(hairline), Self::CLIPS)?;
            painter.fill_rect(clip, painter.color(ColorRole::PrimaryContainer));
            let text = painter.text(name, None, TextStyle::Body)?;
            if text.size().width() + 8.0 <= right - left {
                let y = clips_top + (Self::CLIPS - text.size().height()) / 2.0;
                text.paint(
                    painter,
                    left.max(0.0) + 4.0,
                    y,
                    painter.color(ColorRole::OnPrimaryContainer),
                )?;
            }
        }
        Ok(())
    }
}

fn card(ui: &Runtime, key: &str, title: &str, status: &Signal<String>) -> Element {
    let enabled = ui.signal(true);
    let name = ui.signal(title.to_owned());
    let pressed = format!("{title}: pressed");
    let status = status.clone();
    Element::new(
        key,
        Container::new(Element::new(
            "rows",
            Column::new([
                Element::new("title", Label::new(title).style(TextStyle::Heading)),
                Element::new("enabled", Checkbox::new(enabled, "Enabled")),
                Element::new("name", TextField::new(name)).with_label("Name"),
                Element::new(
                    "press",
                    Button::new("Press", move |_| status.set(pressed.clone())),
                ),
            ]),
        ))
        .padding(Length::from(12))
        .background_role(ColorRole::Surface),
    )
}

/// Arrows made from one path by affine maps, and a trace fitted to its area
/// with a different factor per axis while its stroke keeps its width.
struct MappedShapes;
impl Component for MappedShapes {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([560, 96]))
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let point = |x: f32, y: f32| Point::new(x, y);
        let arrow = PathBuilder::new()
            .move_to(point(-18.0, -4.0)?)
            .line_to(point(6.0, -4.0)?)
            .line_to(point(6.0, -12.0)?)
            .line_to(point(22.0, 0.0)?)
            .line_to(point(6.0, 12.0)?)
            .line_to(point(6.0, 4.0)?)
            .line_to(point(-18.0, 4.0)?)
            .close()
            .build()?;
        let color = painter.color(ColorRole::Primary);
        for (index, degrees) in [0.0, 45.0, 90.0, 135.0, 180.0].into_iter().enumerate() {
            let placed = Transform2D::rotation_degrees(degrees).and_then(|turn| {
                turn.then(Transform2D::translation(36.0 + 60.0 * index as f32, 48.0)?)
            })?;
            painter.fill_path(&arrow.transformed(&placed)?, FillRule::NonZero, color)?;
        }
        // One period of a sine in data units, mapped onto a 220 × 72 area.
        let samples = (0..=64).map(|n| {
            let t = n as f32 / 64.0;
            point(t, (t * std::f32::consts::TAU).sin())
        });
        let wave = Path::polyline(samples.collect::<Result<Vec<_>, _>>()?)?;
        let fitted = Transform2D::scale(220.0, -32.0)
            .and_then(|fit| fit.then(Transform2D::translation(330.0, 48.0)?))?;
        painter.stroke_path(
            &wave.transformed(&fitted)?,
            StrokeStyle::new(Length::new(2.0)?).with_join(LineJoin::Round),
            painter.color(ColorRole::OnSurface),
        )
    }
}

pub(crate) fn transforms(ui: &Runtime) -> Result<Element, RuntimeError> {
    let zoom = ZoomController::new(ui);
    let readout = zoom.view().map(|view| {
        format!(
            "{:.0}% at {:.0}, {:.0}",
            view.scale().get() * 100.0,
            view.offset().x(),
            view.offset().y()
        )
    })?;
    let status = ui.signal("Nothing pressed yet.".to_owned());
    let timeline_readout = ui.signal("From 0:00.000 at 32 px/s, amplitude ×1.0".to_owned());
    let at = |x: f32, y: f32| Point::new(x, y);
    let cards = [
        (at(40.0, 40.0)?, card(ui, "card-a", "Oscillator", &status)),
        (at(320.0, 80.0)?, card(ui, "card-b", "Filter", &status)),
    ];
    let turned = |key: &str, text: &str, turn: QuarterTurn| {
        Element::new(
            key,
            Container::new(Element::new("text", Label::new(text).turned(turn)))
                .padding(Length::from(8))
                .background_role(ColorRole::SurfaceContainer),
        )
    };
    let (fit, reset) = (zoom.clone(), zoom.clone());
    Ok(page([
        section(
            "view",
            "Zoom and pan",
            [
                label(
                    "help",
                    "A ZoomView. The wheel pans and zooms at the pointer with Control (Command on macOS); dragging the background or dragging with the middle button pans. Once focused, arrows pan, = and - zoom, 0 shows 100% and Home fits everything. The cards are ordinary controls laid out once in content space: they keep working at every zoom, their text is prepared for the size it is shown at, and a pan only moves retained drawing.",
                ),
                stage(
                    "canvas-stage",
                    320,
                    Element::new(
                        "canvas-background",
                        Container::new(Element::new(
                            "canvas",
                            ZoomView::new(cards)
                                .grid(Length::new(40.0)?)
                                .controller(&zoom),
                        ))
                        .background_role(ColorRole::SurfaceContainer),
                    ),
                ),
                Element::new(
                    "canvas-actions",
                    Row::new([
                        Element::new(
                            "canvas-fit",
                            Button::new("Fit", move |_| fit.fit().map(|_| ())),
                        ),
                        Element::new(
                            "canvas-reset",
                            Button::new("Reset", move |_| reset.set(ViewTransform::IDENTITY)),
                        ),
                        Element::new("zoom-readout", Label::new(readout)),
                    ]),
                ),
                Element::new("press-status", Label::new(status.read_only())),
            ],
        ),
        section(
            "axis",
            "Zoom per axis",
            [
                label(
                    "help",
                    "A timeline written as a consumer would write one, with an AxisMapping for time and one for amplitude. The wheel scrolls through time, Control (Command on macOS) with the wheel or a pinch zooms time at the pointer, and Shift added to either zooms the amplitude instead. Drag to pan and press Home for the whole hour. Rows, text and the trace's stroke keep their size, because only positions are mapped.",
                ),
                Element::new("timeline", Timeline::new(timeline_readout.clone())?),
                Element::new("timeline-readout", Label::new(timeline_readout.read_only())),
            ],
        ),
        section(
            "turns",
            "Quarter turns",
            [
                label(
                    "help",
                    "A turned label exchanges its width and height and draws the upright label's glyph coverage; nothing is rasterized again.",
                ),
                Element::new(
                    "labels",
                    Wrap::new([
                        turned("upright", "Amplitude", QuarterTurn::None),
                        turned("left", "Amplitude", QuarterTurn::Left),
                        turned("right", "Amplitude", QuarterTurn::Right),
                        turned("half", "Amplitude", QuarterTurn::Half),
                    ]),
                ),
            ],
        ),
        section(
            "paths",
            "Affine maps of path geometry",
            [
                label(
                    "help",
                    "One arrow path mapped by five rotations, and a sine in data units fitted to its area with a different factor per axis. Only the outline is mapped, so the trace keeps its two-pixel stroke.",
                ),
                Element::new("shapes", MappedShapes),
            ],
        ),
    ]))
}

/// Measures what continuous zooming and panning cost on a canvas of cards.
///
/// Sixty-four cards of four controls each sit in a 960×640 window at scale 1.
/// A continuous zoom is modeled as one wheel step of ten logical pixels per
/// 60 Hz frame at the window's center with the primary modifier held, each one
/// flush and paint: from 100% up to 800%, back, and down to 12.5%. After each
/// gesture the runtime's clock passes the rest interval and the one flush that
/// prepares the content again at its final scale is timed separately. Pans of
/// ten pixels are measured the same way for comparison. Times are headless CPU
/// observations of update, layout, text preparation, painting and composition,
/// not GPU or OS event-loop measurements.
pub(crate) fn zoom_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    use std::{io::Write, time::Duration, time::Instant};
    const COLUMNS: usize = 8;
    const ROWS: usize = 8;
    const STEP: f32 = 10.0;
    const FRAME: Duration = Duration::from_micros(16_667);
    const REST: Duration = Duration::from_millis(500);
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
    let status = ui.signal(String::new());
    let mut cards = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let index = row * COLUMNS + column;
            let origin = Point::new(40.0 + 280.0 * column as f32, 40.0 + 200.0 * row as f32)?;
            let title = format!("Module {index}");
            cards.push((origin, card(&ui, &format!("card-{index}"), &title, &status)));
        }
    }
    let root = ui.mount_tree(Element::new("canvas", ZoomView::new(cards)))?;
    let mut now = Duration::ZERO;
    ui.flush(now)?;
    ui.paint()?;
    let primary = KeyCode::primary();
    let center = Point::new(480.0, 320.0)?;
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "Headless zoom workload: {} cards, {} components, 960×640 at scale 1, one step per 60 Hz frame.",
        COLUMNS * ROWS,
        ui.inspect().components.len()
    )?;
    let summary = |times: &[f64]| {
        let mut sorted = times.to_vec();
        sorted.sort_by(f64::total_cmp);
        let at = |fraction: f64| sorted[((sorted.len() - 1) as f64 * fraction).round() as usize];
        format!(
            "mean {:.2} ms, median {:.2}, p95 {:.2}, max {:.2}",
            times.iter().sum::<f64>() / times.len() as f64,
            at(0.5),
            at(0.95),
            at(1.0)
        )
    };
    let calls = |profile: &ProfileSnapshot, operation: ProfileOperation| -> u64 {
        profile
            .stats
            .iter()
            .filter(|row| row.operation == operation)
            .map(|row| row.calls)
            .sum()
    };
    let mut phase = |ui: &mut Runtime,
                     now: &mut Duration,
                     name: &str,
                     steps: usize,
                     zoom: bool,
                     delta: Point|
     -> Result<(), Box<dyn std::error::Error>> {
        if zoom {
            ui.key_input(primary, KeyAction::Press)?;
        }
        ui.set_profiling(true)?;
        ui.clear_profile()?;
        let mut times = Vec::with_capacity(steps);
        for _ in 0..steps {
            *now += FRAME;
            let started = Instant::now();
            ui.flush(*now)?;
            ui.pointer_input(PointerEvent::new(center, PointerKind::Scroll(delta)))?;
            ui.flush(*now)?;
            ui.paint()?;
            times.push(started.elapsed().as_secs_f64() * 1000.0);
        }
        let profile = ui.profile();
        if zoom {
            ui.key_input(primary, KeyAction::Release)?;
        }
        // The gesture rests; the next flush after the rest interval prepares
        // whatever it stretched.
        ui.clear_profile()?;
        *now += REST;
        let started = Instant::now();
        ui.flush(*now)?;
        ui.paint()?;
        let settled = started.elapsed().as_secs_f64() * 1000.0;
        let rest = ui.profile();
        ui.set_profiling(false)?;
        if ui.next_deadline().is_some() {
            return Err(format!("{name}: work remained scheduled after the rest").into());
        }
        let scale = ui
            .inspect()
            .components
            .iter()
            .find(|item| item.key == "canvas")
            .and_then(|item| match item.details.properties.get("view_scale") {
                Some(InspectValue::Number(value)) => Some(*value),
                _ => None,
            })
            .unwrap_or(f64::NAN);
        writeln!(
            out,
            "{name}: {steps} steps to {:.1}%. Per step: {}; on average {:.0} updates and {:.0} text-service calls. Rest: {settled:.2} ms, {} updates, {} text-service calls.",
            scale * 100.0,
            summary(&times),
            calls(&profile, ProfileOperation::Update) as f64 / steps as f64,
            calls(&profile, ProfileOperation::Text) as f64 / steps as f64,
            calls(&rest, ProfileOperation::Update),
            calls(&rest, ProfileOperation::Text),
        )?;
        Ok(())
    };
    phase(&mut ui, &mut now, "Pan", 60, false, Point::new(STEP, STEP)?)?;
    phase(
        &mut ui,
        &mut now,
        "Pan back",
        60,
        false,
        Point::new(-STEP, -STEP)?,
    )?;
    phase(
        &mut ui,
        &mut now,
        "Zoom in",
        60,
        true,
        Point::new(0.0, -STEP)?,
    )?;
    phase(
        &mut ui,
        &mut now,
        "Zoom out",
        60,
        true,
        Point::new(0.0, STEP)?,
    )?;
    phase(
        &mut ui,
        &mut now,
        "Zoom out further",
        60,
        true,
        Point::new(0.0, STEP)?,
    )?;
    ui.unmount(&root)?;
    Ok(())
}
