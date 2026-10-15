//! The reusable level and trace widgets through the public facade: what they
//! draw, what they hold, and that they schedule nothing of their own.
use pert_ui::*;
use std::time::Duration;

const FRAME: Duration = Duration::from_nanos(16_666_667);
type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Rig {
    ui: Runtime,
    root: WidgetId,
    endpoint: SeriesEndpoint<f32>,
    now: Duration,
    tick: u64,
}
impl Rig {
    fn new(capacity: usize, build: impl FnOnce(&Runtime, SeriesSource<f32>) -> Element) -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
            .unwrap();
        let config = SeriesConfig::new(Capacity::new(capacity).unwrap());
        let (endpoint, consumer) = series_bridge::<f32>(config);
        let source = ui.series_source(consumer, Delivery::Polled);
        let element = build(&ui, source);
        let root = ui
            .mount_tree(Element::new("column", Column::new([element])))
            .unwrap();
        // The first update subscribes; the runtime then activates the source.
        while ui.next_deadline() == Some(Duration::ZERO) {
            ui.flush(Duration::ZERO).unwrap();
        }
        Self {
            ui,
            root,
            endpoint,
            now: Duration::ZERO,
            tick: 0,
        }
    }
    fn publish(&mut self, values: &[f32]) {
        for &value in values {
            self.tick += 1;
            let _ = self.endpoint.publish(value, Timestamp::new(self.tick));
        }
    }
    /// Advances one presentation frame and returns how many components updated.
    fn frame(&mut self) -> usize {
        self.now += FRAME;
        self.ui.flush(self.now).unwrap().components_updated
    }
    fn property(&self, key: &str, name: &str) -> InspectValue {
        let snapshot = self.ui.inspect();
        let item = snapshot.components.iter().find(|c| c.key == key).unwrap();
        item.details.properties[name].clone()
    }
    fn role(&self, role: ColorRole) -> Color {
        let theme = self.ui.theme();
        theme.palette(theme.mode().unwrap()).unwrap().color(role)
    }
    fn rects(&mut self, role: ColorRole) -> Vec<[f32; 4]> {
        let color = self.role(role);
        let scene = self.ui.paint().unwrap();
        scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color: c } if *c == color => Some(rect.values()),
                _ => None,
            })
            .collect()
    }
}
fn unit(name: &str) -> ParameterSpec<f32> {
    ParameterSpec::new(name, 0.0, 1.0, 0.0).unwrap()
}

#[test]
fn the_meter_shows_the_peak_of_what_arrived_and_holds_it_when_nothing_does() {
    let mut rig = Rig::new(64, |_, source| {
        Element::new("meter", LevelMeter::new(source, unit("Level")))
    });
    // Nothing delivered yet: the rest value, muted.
    assert_eq!(
        rig.property("meter", "available"),
        InspectValue::Bool(false)
    );
    assert!(rig.rects(ColorRole::Primary).is_empty());

    rig.publish(&[0.25, 0.75, 0.5]);
    assert_eq!(rig.frame(), 1);
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.75));
    let width = 3.0 * ThemeMetrics::default().get(MetricRole::ControlWidth);
    let thickness = ThemeMetrics::default().get(MetricRole::MeterThickness);
    assert_eq!(
        rig.rects(ColorRole::Primary),
        [[0.0, 0.0, width * 0.75, thickness]]
    );

    // A drain that finds nothing updates nobody, so the level simply stays.
    assert_eq!(rig.frame(), 0);
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.75));

    // A lower arrival replaces it: the fold is per arrival, not a lifetime peak.
    rig.publish(&[0.5]);
    rig.frame();
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.5));
}

#[test]
fn partial_meter_levels_leave_a_visible_gap_and_track_in_both_axes_and_themes() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for axis in [MeterAxis::Horizontal, MeterAxis::Vertical] {
            let mut rig = Rig::new(8, |_, source| {
                Element::new("meter", LevelMeter::new(source, unit("Level")).axis(axis))
            });
            rig.ui.theme().set_mode(mode).unwrap();
            rig.ui.flush(Duration::ZERO).unwrap();
            let size = rig
                .ui
                .bounds(&rig.ui.children(&rig.root).unwrap()[0])
                .unwrap()
                .unwrap();
            let length = if axis == MeterAxis::Horizontal {
                size.width()
            } else {
                size.height()
            };
            let thickness = ThemeMetrics::default().get(MetricRole::MeterThickness);

            for level in [0.0, 0.5, 1.0 - thickness / (2.0 * length), 1.0] {
                rig.publish(&[level]);
                rig.frame();
                let fill = rig.rects(ColorRole::Primary);
                let track = rig.rects(ColorRole::Outline);
                if level == 0.0 {
                    assert!(fill.is_empty(), "{mode:?} {axis:?}");
                    assert_eq!(track.len(), 1, "{mode:?} {axis:?}");
                    continue;
                }
                assert_eq!(fill.len(), 1, "{mode:?} {axis:?} {level}");
                if level == 1.0 {
                    assert!(track.is_empty(), "{mode:?} {axis:?}");
                    continue;
                }
                assert_eq!(track.len(), 1, "{mode:?} {axis:?} {level}");
                let gap = if axis == MeterAxis::Horizontal {
                    track[0][0] - (fill[0][0] + fill[0][2])
                } else {
                    fill[0][1] - (track[0][1] + track[0][3])
                };
                let remaining_track = if axis == MeterAxis::Horizontal {
                    track[0][2]
                } else {
                    track[0][3]
                };
                assert!(
                    gap > 0.0 && remaining_track > 0.0,
                    "{mode:?} {axis:?} {level}: gap {gap}, track {remaining_track}"
                );
                let expected = (thickness / 2.0).min(length * (1.0 - level) / 2.0);
                assert!(
                    (gap - expected).abs() < 0.001,
                    "{mode:?} {axis:?} {level}: gap {gap}, expected {expected}"
                );
            }
        }
    }
}

#[test]
fn values_outside_the_scale_rest_at_its_ends_and_non_finite_values_are_ignored() {
    let mut rig = Rig::new(64, |_, source| {
        let scale = ParameterSpec::new("Gain", -60.0, 0.0, -60.0).unwrap();
        Element::new("meter", LevelMeter::new(source, scale))
    });
    rig.publish(&[12.0]);
    rig.frame();
    let width = 3.0 * ThemeMetrics::default().get(MetricRole::ControlWidth);
    assert_eq!(rig.rects(ColorRole::Primary)[0][2], width);
    rig.publish(&[f32::NAN, f32::INFINITY]);
    rig.frame();
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(12.0));
    rig.publish(&[-90.0]);
    rig.frame();
    assert!(rig.rects(ColorRole::Primary).is_empty(), "an empty bar");

    // Semantics carry the value on the scale, under the scale's name.
    let snapshot = rig.ui.inspect();
    let meter = snapshot
        .components
        .iter()
        .find(|c| c.key == "meter")
        .unwrap();
    let semantics = meter.details.semantics.as_ref().unwrap();
    assert_eq!(semantics.role, SemanticRole::Readout);
    assert_eq!(semantics.name.as_deref(), Some("Gain"));
    assert_eq!(
        semantics.value,
        Some(SemanticValue::Number {
            value: -90.0,
            minimum: Some(-60.0),
            maximum: Some(0.0),
        })
    );
}

#[test]
fn a_logarithmic_scale_places_the_level_by_ratio() {
    let mut rig = Rig::new(8, |_, source| {
        let scale = ParameterSpec::new("Rate", 1.0, 100.0, 1.0)
            .unwrap()
            .with_mapping(ParameterMapping::Logarithmic)
            .unwrap();
        Element::new("meter", LevelMeter::new(source, scale))
    });
    rig.publish(&[10.0]);
    rig.frame();
    let width = 3.0 * ThemeMetrics::default().get(MetricRole::ControlWidth);
    let drawn = rig.rects(ColorRole::Primary)[0][2];
    assert!((drawn - width / 2.0).abs() < 1e-3, "{drawn}");
}

#[test]
fn the_peak_marker_is_released_by_one_deadline_and_nothing_else_is_scheduled() {
    let hold = Duration::from_millis(500);
    let mut rig = Rig::new(64, |_, source| {
        Element::new(
            "meter",
            LevelMeter::new(source, unit("Level")).peak_hold(hold),
        )
    });
    rig.publish(&[0.75]);
    rig.frame();
    assert_eq!(rig.property("meter", "peak"), InspectValue::Number(0.75));
    rig.publish(&[0.25]);
    rig.frame();
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.25));
    assert_eq!(rig.property("meter", "peak"), InspectValue::Number(0.75));
    assert_eq!(rig.rects(ColorRole::OnSurface).len(), 1, "the marker");

    // The data stops. Polling continues at the frame ceiling, but the meter's
    // own deadline is the release, after which the marker rests on the level.
    while rig.now < FRAME + hold {
        rig.frame();
    }
    assert_eq!(rig.property("meter", "peak"), InspectValue::Number(0.25));
    let updates_before = rig.frame();
    assert_eq!(
        updates_before, 0,
        "released: the meter asks for nothing more"
    );
}

#[test]
fn a_dropped_endpoint_mutes_the_meter_and_ends_scheduling() {
    let mut rig = Rig::new(8, |_, source| {
        Element::new("meter", LevelMeter::new(source, unit("Level")))
    });
    rig.publish(&[0.5]);
    rig.frame();
    let Rig {
        mut ui,
        endpoint,
        now,
        ..
    } = rig;
    drop(endpoint);
    ui.flush(now + FRAME).unwrap();
    assert_eq!(ui.next_deadline(), None);
    let snapshot = ui.inspect();
    let meter = snapshot
        .components
        .iter()
        .find(|c| c.key == "meter")
        .unwrap();
    assert_eq!(
        meter.details.properties["available"],
        InspectValue::Bool(false)
    );
}

#[test]
fn a_paused_or_hidden_widget_is_not_subscribed_so_nothing_is_drained() -> TestResult {
    let mut running = None;
    let mut rig = Rig::new(64, |ui, source| {
        let signal = ui.signal(true);
        let element = Element::new(
            "trace",
            Trace::new(source, unit("Signal")).running_signal(signal.read_only()),
        );
        running = Some(signal);
        element
    });
    let running = running.ok_or("signal")?;
    assert!(
        rig.ui.next_deadline().is_some(),
        "a running trace is polled"
    );
    running.set(false)?;
    rig.frame();
    rig.frame();
    assert_eq!(
        rig.ui.next_deadline(),
        None,
        "paused: no subscriber remains"
    );
    running.set(true)?;
    rig.frame();
    assert!(rig.ui.next_deadline().is_some());
    Ok(())
}

#[test]
fn the_trace_draws_one_span_per_column_and_breaks_at_gaps() {
    let mut rig = Rig::new(1024, |_, source| {
        let scale = ParameterSpec::new("Signal", -1.0, 1.0, 0.0).unwrap();
        Element::new("trace", Trace::new(source, scale))
    });
    assert!(rig.rects(ColorRole::Primary).is_empty(), "no data, no line");
    let width = 3.0 * ThemeMetrics::default().get(MetricRole::ControlWidth);
    let height = 8.0 * ThemeMetrics::default().get(MetricRole::MeterThickness);

    // Four points across the full width: a rising ramp, joined column to column.
    rig.publish(&[-1.0, 0.0, 0.0, 1.0]);
    rig.frame();
    let spans = rig.rects(ColorRole::Primary);
    assert_eq!(spans.len(), 4);
    assert_eq!(spans[0], [0.0, height - 1.0, width / 4.0, 1.0], "the floor");
    // The second column joins up from the first point to its own.
    assert_eq!(
        spans[1],
        [width / 4.0, height / 2.0, width / 4.0, height / 2.0]
    );
    assert_eq!(spans[3][1], 0.0, "the ceiling");
    assert_eq!(rig.property("trace", "points"), InspectValue::Number(4.0));

    // A value that is not a number is not drawn and is not joined across.
    rig.publish(&[f32::NAN, -1.0]);
    rig.frame();
    let spans = rig.rects(ColorRole::Primary);
    assert_eq!(spans.len(), 5, "six points, one of them undrawn");
    let last = spans[4];
    assert_eq!(last[3], 1.0, "not joined to the ceiling before the gap");

    // Denser than the screen: never more columns than physical pixels.
    let many: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin()).collect();
    rig.publish(&many);
    rig.frame();
    assert_eq!(rig.rects(ColorRole::Primary).len(), width as usize);
}

#[test]
fn widgets_of_another_runtime_are_rejected_not_silently_empty() {
    let other = Runtime::new();
    let config = SeriesConfig::new(Capacity::new(8).unwrap());
    let (_endpoint, consumer) = series_bridge::<f32>(config);
    let foreign = other.series_source(consumer, Delivery::Polled);
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("trace", Trace::new(foreign, unit("Signal"))))
        .unwrap();
    assert!(matches!(
        ui.flush(Duration::ZERO),
        Err(RuntimeError::ForeignRuntime)
    ));
}

/// Re-describes the column's only child and checks that it kept its identity.
fn redescribe(rig: &mut Rig, next: Element) {
    let before = rig.ui.children(&rig.root).unwrap()[0].clone();
    rig.ui.reconcile_children(&rig.root, vec![next]).unwrap();
    assert_eq!(
        rig.ui.children(&rig.root).unwrap()[0],
        before,
        "reconciled, not remounted"
    );
}

#[test]
fn a_redescribed_meter_reads_its_new_source_and_scale() {
    let mut rig = Rig::new(64, |_, source| {
        Element::new("meter", LevelMeter::new(source, unit("Level")))
    });
    rig.publish(&[0.75]);
    rig.frame();
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.75));

    let config = SeriesConfig::new(Capacity::new(64).unwrap());
    let (mut other, consumer) = series_bridge::<f32>(config);
    let source = rig.ui.series_source(consumer, Delivery::Polled);
    let scale = ParameterSpec::new("Other", 0.0, 2.0, 0.5).unwrap();
    redescribe(
        &mut rig,
        Element::new("meter", LevelMeter::new(source, scale)),
    );
    rig.frame();
    // The old source's level is gone: the new scale's rest value until the new
    // source delivers, and the old endpoint no longer drives the meter.
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(0.5));
    assert_eq!(
        rig.property("meter", "available"),
        InspectValue::Bool(false)
    );
    // The first drain of a newly subscribed source discards its backlog, so
    // publish once the runtime has activated it.
    while rig.ui.next_deadline() == Some(rig.now) {
        rig.ui.flush(rig.now).unwrap();
    }
    let _ = other.publish(1.5, Timestamp::new(1));
    rig.publish(&[0.25]);
    rig.frame();
    assert_eq!(rig.property("meter", "level"), InspectValue::Number(1.5));
    let name = rig
        .ui
        .inspect()
        .components
        .iter()
        .find(|c| c.key == "meter")
        .unwrap()
        .details
        .semantics
        .clone()
        .unwrap()
        .name;
    assert_eq!(name.as_deref(), Some("Other"));
}

#[test]
fn a_redescribed_trace_draws_its_points_on_the_new_scale_in_the_new_color() {
    let kept = std::cell::RefCell::new(None);
    let mut rig = Rig::new(64, |_, source| {
        kept.replace(Some(source.clone()));
        Element::new("trace", Trace::new(source, unit("Signal")))
    });
    rig.publish(&[0.5, 0.5]);
    rig.frame();
    let before = rig.rects(ColorRole::Primary);
    assert!(!before.is_empty());

    // Same source, a paused run, a new scale and color: the retained points
    // are redrawn at once without a new read.
    let source = kept.take().unwrap();
    let paused = rig.ui.signal(false);
    let scale = ParameterSpec::new("Signal", 0.0, 2.0, 0.0).unwrap();
    redescribe(
        &mut rig,
        Element::new(
            "trace",
            Trace::new(source, scale)
                .color(ColorRole::Secondary)
                .running_signal(paused.read_only()),
        ),
    );
    rig.frame();
    assert!(rig.rects(ColorRole::Primary).is_empty());
    let after = rig.rects(ColorRole::Secondary);
    assert_eq!(after.len(), before.len());
    assert!(
        after[0][1] > before[0][1],
        "0.5 of 2 sits lower than 0.5 of 1"
    );
}
