//! Progress and activity presentation through the public facade: what each status
//! draws, and that indeterminate activity is the only thing that schedules work.
use pert_ui::*;
use std::time::Duration;

const FRAME: Duration = Duration::from_nanos(16_666_667);

/// A bar in a column, bound to a status signal of the same runtime.
fn setup(initial: ProgressStatus) -> (Runtime, Signal<ProgressStatus>, WidgetId) {
    let mut ui = Runtime::new();
    let status = ui.signal(initial);
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("bar", ProgressBar::new(status.read_only()))]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, status, root)
}
fn bar(ui: &Runtime, root: &WidgetId) -> WidgetId {
    ui.children(root).unwrap()[0].clone()
}
fn role(ui: &Runtime, role: ColorRole) -> Color {
    let theme = ui.theme();
    theme.palette(theme.mode().unwrap()).unwrap().color(role)
}
/// The fill drawn in a role, as `(x, width)`; the track is drawn in Outline.
fn fill(ui: &mut Runtime, color: ColorRole) -> Option<(f32, f32)> {
    let color = role(ui, color);
    ui.paint()
        .unwrap()
        .commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect { rect, color: c } if *c == color => {
                Some((rect.x(), rect.width()))
            }
            _ => None,
        })
}
#[test]
fn each_status_draws_its_own_fill_and_only_a_known_fraction_has_a_value() {
    let (mut ui, status, root) = setup(ProgressStatus::Idle);
    let bounds = ui.bounds(&bar(&ui, &root)).unwrap().unwrap();
    let standard = ThemeMetrics::default();
    assert_eq!(bounds.height(), standard.metric(MetricRole::ProgressHeight));
    assert_eq!(
        bounds.width(),
        3.0 * standard.metric(MetricRole::ControlWidth)
    );

    // Idle is an empty track: the Outline track and nothing else.
    assert_eq!(
        fill(&mut ui, ColorRole::Outline),
        Some((0.0, bounds.width()))
    );
    assert_eq!(fill(&mut ui, ColorRole::Primary), None);

    let quarter = NormalizedValue::try_from(0.25).unwrap();
    status.set(ProgressStatus::Determinate(quarter)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(
        fill(&mut ui, ColorRole::Primary),
        Some((0.0, bounds.width() * 0.25))
    );
    for (next, color) in [
        (ProgressStatus::Succeeded, ColorRole::Success),
        (ProgressStatus::Failed, ColorRole::Error),
        (ProgressStatus::Cancelled, ColorRole::Muted),
    ] {
        status.set(next).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(
            fill(&mut ui, color),
            Some((0.0, bounds.width())),
            "{next:?}"
        );
        assert_eq!(fill(&mut ui, ColorRole::Primary), None, "{next:?}");
        assert!(next.is_finished() && !next.is_running());
    }
    // A status change repaints inside fixed geometry and never moves the bar.
    assert_eq!(ui.bounds(&bar(&ui, &root)).unwrap().unwrap(), bounds);
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn indeterminate_activity_is_the_only_status_that_schedules_work() {
    let (mut ui, status, root) = setup(ProgressStatus::Idle);
    assert_eq!(ui.next_deadline(), None, "idle schedules nothing");

    status.set(ProgressStatus::Indeterminate).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert_eq!(ui.next_deadline(), Some(FRAME), "one frame ahead");

    // At phase zero the segment is just outside the left edge, so nothing is
    // filled yet; it then moves with the runtime's clock, and each frame asks for
    // the next.
    assert_eq!(fill(&mut ui, ColorRole::Primary), None);
    ui.flush(Duration::from_millis(400)).unwrap();
    let first = fill(&mut ui, ColorRole::Primary).unwrap();
    let later = Duration::from_millis(600);
    ui.flush(later).unwrap();
    let second = fill(&mut ui, ColorRole::Primary).unwrap();
    assert!(second.0 > first.0, "{first:?} then {second:?}");
    assert_eq!(ui.next_deadline(), Some(later + FRAME));
    // A missed stretch of frames produces one update, not a catch-up burst.
    assert_eq!(
        ui.flush(Duration::from_secs(5)).unwrap().components_updated,
        1
    );

    // Hidden: the deadline is withdrawn, and returns when shown again.
    let bar = bar(&ui, &root);
    ui.set_visible(&bar, false).unwrap();
    ui.flush(Duration::from_secs(6)).unwrap();
    assert_eq!(ui.next_deadline(), None, "hidden activity leaves no wake");
    ui.set_visible(&bar, true).unwrap();
    ui.flush(Duration::from_secs(7)).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_secs(7) + FRAME));

    // Finished: the next update simply does not ask again.
    status.set(ProgressStatus::Succeeded).unwrap();
    ui.flush(Duration::from_secs(8)).unwrap();
    assert_eq!(ui.next_deadline(), None, "a finished task leaves no wake");
    assert_eq!(fill(&mut ui, ColorRole::Success).map(|(x, _)| x), Some(0.0));

    // Unmounted while running: nothing is left behind either.
    status.set(ProgressStatus::Indeterminate).unwrap();
    ui.flush(Duration::from_secs(9)).unwrap();
    assert!(ui.next_deadline().is_some());
    ui.unmount(&bar).unwrap();
    ui.flush(Duration::from_secs(10)).unwrap();
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn the_sweep_stays_inside_the_track_for_every_phase() {
    let (mut ui, _status, _root) = setup(ProgressStatus::Indeterminate);
    let width = 3.0 * ThemeMetrics::default().metric(MetricRole::ControlWidth);
    for step in 0..=60 {
        ui.flush(Duration::from_millis(step * 25)).unwrap();
        if let Some((x, w)) = fill(&mut ui, ColorRole::Primary) {
            assert!(x >= 0.0 && x + w <= width + 0.001, "step {step}: {x} + {w}");
        }
    }
}

#[test]
fn counts_map_to_a_fraction_and_semantics_report_it() {
    assert_eq!(
        ProgressStatus::of(0, 4),
        ProgressStatus::Determinate(NormalizedValue::MIN)
    );
    assert_eq!(
        ProgressStatus::of(9, 4),
        ProgressStatus::Determinate(NormalizedValue::MAX)
    );
    assert_eq!(ProgressStatus::of(1, 0), ProgressStatus::Indeterminate);
    assert_eq!(ProgressStatus::default(), ProgressStatus::Idle);

    let mut ui = Runtime::new();
    let status = ui.signal(ProgressStatus::of(1, 4));
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "field",
        Field::new(Element::new("bar", ProgressBar::new(status.read_only()))).label("Export"),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let semantics = |ui: &Runtime| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == "bar")
            .and_then(|component| component.details.semantics)
            .unwrap()
    };
    let now = semantics(&ui);
    assert_eq!(now.role, SemanticRole::ProgressIndicator);
    assert_eq!(now.name.as_deref(), Some("Export"), "named by its field");
    assert_eq!(
        now.value,
        Some(SemanticValue::Number {
            value: 0.25,
            minimum: Some(0.0),
            maximum: Some(1.0),
        })
    );
    assert!(now.actions.is_empty() && !now.invalid);

    status.set(ProgressStatus::Failed).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let failed = semantics(&ui);
    assert!(failed.invalid);
    assert_eq!(failed.value, None, "an ended task has no fraction");
}

const STEP: Duration = Duration::from_millis(100);

fn spinner(busy: bool) -> (Runtime, Signal<bool>, WidgetId) {
    let mut ui = Runtime::new();
    let signal = ui.signal(busy);
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                Element::new("spinner", Spinner::new(signal.read_only())),
                Element::new("text", Label::new("Saving")),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, signal, root)
}
/// Every mask drawn inside the spinner's bounds, as `(rect, alpha, mask)`.
fn dots(ui: &mut Runtime, root: &WidgetId) -> Vec<([f32; 4], f32, MaskImage)> {
    let bounds = ui.bounds(&ui.children(root).unwrap()[0]).unwrap().unwrap();
    let scene = ui.paint().unwrap();
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, mask, color, ..
            } if rect.x() >= bounds.x() && rect.x() < bounds.x() + bounds.width() => Some((
                rect.values(),
                color.alpha(),
                scene.mask(*mask).unwrap().clone(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn the_spinner_moves_color_not_geometry_and_ticks_once_per_step() {
    let (mut ui, busy, root) = spinner(true);
    // It wakes at the next step boundary, not at the 60 Hz ceiling.
    assert_eq!(ui.next_deadline(), Some(STEP));
    let first = dots(&mut ui, &root);
    assert!(!first.is_empty());

    // A late update lands on the clock's step and re-aligns to the boundary.
    ui.flush(Duration::from_millis(330)).unwrap();
    assert_eq!(ui.next_deadline(), Some(Duration::from_millis(400)));
    let later = dots(&mut ui, &root);

    // The dots are where they were, with the same coverage; only alpha moved.
    let geometry = |dots: &[([f32; 4], f32, MaskImage)]| {
        dots.iter()
            .map(|(rect, _, mask)| (*rect, mask.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(geometry(&first), geometry(&later));
    let alphas = |dots: &[([f32; 4], f32, MaskImage)]| {
        dots.iter().map(|(_, alpha, _)| *alpha).collect::<Vec<_>>()
    };
    assert_ne!(alphas(&first), alphas(&later));
    // Every dot is the same size, so one step's dots share their coverage images.
    let mut images: Vec<_> = first.iter().map(|(_, _, mask)| mask.clone()).collect();
    images.dedup();
    assert!(images.len() < first.len(), "dots reuse coverage");

    // A full turn returns to the same picture.
    ui.flush(Duration::from_millis(800)).unwrap();
    assert_eq!(alphas(&dots(&mut ui, &root)), alphas(&first));

    // Idle: nothing is drawn, nothing is scheduled, and nothing beside it moves.
    let text = ui.bounds(&ui.children(&root).unwrap()[1]).unwrap().unwrap();
    busy.set(false).unwrap();
    ui.flush(Duration::from_millis(900)).unwrap();
    assert!(dots(&mut ui, &root).is_empty());
    assert_eq!(ui.next_deadline(), None);
    assert_eq!(
        ui.bounds(&ui.children(&root).unwrap()[1]).unwrap().unwrap(),
        text,
        "stopping does not reflow its neighbor"
    );
}

#[test]
fn a_hidden_or_unmounted_spinner_leaves_no_wake() {
    let (mut ui, _busy, root) = spinner(true);
    let id = ui.children(&root).unwrap()[0].clone();
    ui.set_visible(&id, false).unwrap();
    ui.flush(STEP).unwrap();
    assert_eq!(ui.next_deadline(), None);
    ui.set_visible(&id, true).unwrap();
    ui.flush(STEP * 2).unwrap();
    assert_eq!(ui.next_deadline(), Some(STEP * 3));
    ui.unmount(&id).unwrap();
    ui.flush(STEP * 3).unwrap();
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn reduced_motion_stills_the_bar_and_full_motion_resumes_from_the_clock() {
    let (mut ui, status, root) = setup(ProgressStatus::Indeterminate);
    let settings = ui.ui_settings();
    assert_eq!(settings.motion().unwrap(), MotionPreference::Full);
    assert!(ui.next_deadline().is_some());
    let width = ui.bounds(&bar(&ui, &root)).unwrap().unwrap().width();

    assert!(settings.set_motion(MotionPreference::Reduced).unwrap());
    assert!(!settings.set_motion(MotionPreference::Reduced).unwrap());
    ui.flush(FRAME).unwrap();
    assert_eq!(ui.next_deadline(), None, "a still bar schedules nothing");
    // The segment rests in the middle, where no determinate fill can be.
    let (x, segment) = fill(&mut ui, ColorRole::Primary).unwrap();
    assert!(x > 0.0 && (x + segment / 2.0 - width / 2.0).abs() < 1e-3);
    // Time passing neither moves it nor wakes anything.
    ui.flush(FRAME * 30).unwrap();
    assert_eq!(fill(&mut ui, ColorRole::Primary), Some((x, segment)));
    assert!(!ui.needs_paint());

    // Other statuses are unaffected by the preference.
    status.set(ProgressStatus::of(1, 2)).unwrap();
    ui.flush(FRAME * 31).unwrap();
    assert_eq!(fill(&mut ui, ColorRole::Primary), Some((0.0, width / 2.0)));

    status.set(ProgressStatus::Indeterminate).unwrap();
    settings.set_motion(MotionPreference::Full).unwrap();
    ui.flush(FRAME * 32).unwrap();
    assert!(ui.next_deadline().is_some(), "full motion sweeps again");
}

#[test]
fn reduced_motion_makes_a_busy_spinner_a_still_even_ring() {
    let (mut ui, busy, root) = spinner(true);
    assert!(ui.next_deadline().is_some());
    ui.ui_settings()
        .set_motion(MotionPreference::Reduced)
        .unwrap();
    ui.flush(Duration::from_millis(10)).unwrap();
    assert_eq!(ui.next_deadline(), None);
    let ring = dots(&mut ui, &root);
    assert!(!ring.is_empty());
    assert!(ring.iter().all(|(_, alpha, _)| *alpha == ring[0].1));

    // Resting still draws nothing, so the ring alone says "busy".
    busy.set(false).unwrap();
    ui.flush(Duration::from_millis(20)).unwrap();
    assert!(dots(&mut ui, &root).is_empty());
    assert_eq!(ui.next_deadline(), None);
}

#[test]
fn only_components_that_read_the_motion_preference_are_invalidated() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let busy = ui.signal(false);
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new("spinner", Spinner::new(busy.read_only())),
            Element::new("text", Label::new("Saving")),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.ui_settings()
        .set_motion(MotionPreference::Reduced)
        .unwrap();
    let report = ui.flush(FRAME).unwrap();
    assert_eq!(report.components_updated, 1, "the spinner, not the label");
    assert_eq!(report.next_deadline, None);
}

#[test]
fn spinner_animation_multiplies_the_configured_primary_alpha() {
    for reduced in [false, true] {
        let (mut ui, _, root) = spinner(true);
        if reduced {
            ui.ui_settings()
                .set_motion(MotionPreference::Reduced)
                .unwrap();
            ui.flush(ui.now()).unwrap();
        }
        let original = dots(&mut ui, &root);
        assert!(!original.is_empty());
        let [r, g, b, _] = Palette::dark().color(ColorRole::Primary).channels();
        for alpha in [0.4, 0.0] {
            ui.theme()
                .set_color(
                    ThemeMode::Dark,
                    ColorRole::Primary,
                    Color::from_linear(r, g, b, alpha).unwrap(),
                )
                .unwrap();
            ui.flush(ui.now()).unwrap();
            let changed = dots(&mut ui, &root);
            if alpha == 0.0 && changed.is_empty() {
                continue;
            }
            assert_eq!(changed.len(), original.len());
            for ((rect, before, mask), (after_rect, after, after_mask)) in
                original.iter().zip(&changed)
            {
                assert_eq!(rect, after_rect);
                assert_eq!(mask, after_mask);
                assert!((after - before * alpha).abs() < 0.00001);
            }
        }
    }
}

#[test]
fn a_fill_is_set_apart_from_its_track_on_every_neutral_background() {
    let half = NormalizedValue::try_from(0.5).unwrap();
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for background in [
            None,
            Some(ColorRole::Surface),
            Some(ColorRole::SurfaceContainer),
        ] {
            for status in [
                ProgressStatus::Determinate(half),
                ProgressStatus::Succeeded,
                ProgressStatus::Failed,
                ProgressStatus::Cancelled,
            ] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
                    .unwrap();
                ui.theme().set_mode(mode).unwrap();
                let bar = Element::new("bar", ProgressBar::new(ui.signal(status).read_only()));
                let tree = match background {
                    None => bar,
                    Some(role) => Element::new(
                        "panel",
                        Container::new(bar)
                            .background_role(role)
                            .padding(Length::new(4.0).unwrap()),
                    ),
                };
                let root = ui.mount_tree(tree).unwrap();
                ui.flush(Duration::ZERO).unwrap();
                let id = ui.find_descendant(&root, "bar").unwrap().unwrap_or(root);
                let bounds = ui.bounds(&id).unwrap().unwrap();
                let scene = ui.paint().unwrap();
                let fill_color = role(
                    &ui,
                    match status {
                        ProgressStatus::Succeeded => ColorRole::Success,
                        ProgressStatus::Failed => ColorRole::Error,
                        ProgressStatus::Cancelled => ColorRole::Muted,
                        _ => ColorRole::Primary,
                    },
                );
                let behind = background.map_or(scene.background(), |r| role(&ui, r));
                // Everything the bar itself drew, inside its bounds.
                let drawn: Vec<(Rect, Color)> = scene
                    .commands()
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::FillRect { rect, color }
                            if rect.x() >= bounds.x()
                                && rect.x() + rect.width() <= bounds.x() + bounds.width()
                                && rect.y() >= bounds.y()
                                && rect.y() + rect.height() <= bounds.y() + bounds.height() =>
                        {
                            Some((*rect, *color))
                        }
                        _ => None,
                    })
                    .collect();
                let case = format!("{mode:?} {background:?} {status:?}");
                let [fill] = drawn
                    .iter()
                    .filter(|(_, color)| *color == fill_color)
                    .map(|(rect, _)| *rect)
                    .collect::<Vec<_>>()[..]
                else {
                    panic!("{case}: drew {drawn:?}");
                };
                let gap = fill.height() / 2.0;
                let partial = matches!(status, ProgressStatus::Determinate(_));
                assert_eq!(
                    drawn.len(),
                    if partial { 2 } else { 1 },
                    "{case}: {drawn:?}"
                );
                for (rect, color) in drawn.iter().filter(|(rect, _)| *rect != fill) {
                    assert_eq!(*color, role(&ui, ColorRole::Outline), "{case}");
                    let apart = rect.x() + rect.width() + gap <= fill.x() + 0.001
                        || fill.x() + fill.width() + gap <= rect.x() + 0.001;
                    assert!(apart, "{case}: track {rect:?} touches fill {fill:?}");
                }
                let ratio = OpaqueColor::try_from(fill_color)
                    .unwrap()
                    .contrast_ratio(OpaqueColor::try_from(behind).unwrap());
                assert!(ratio >= 3.0, "{case}: fill on background {ratio:.2}");
            }
        }
    }
}

/// The bar's own rectangles as `(x, width)` relative to the bar, in drawing
/// order, with the role each was drawn in.
fn pieces(ui: &mut Runtime, root: &WidgetId) -> Vec<(f32, f32, ColorRole)> {
    let bounds = ui.bounds(&bar(ui, root)).unwrap().unwrap();
    let roles = [ColorRole::Outline, ColorRole::Primary];
    let scene = ui.paint().unwrap();
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color } => roles
                .into_iter()
                .find(|role| *color == self::role(ui, *role))
                .map(|role| (rect.x() - bounds.x(), rect.width(), role)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_gap_beside_a_fill_narrows_near_either_end_so_some_track_remains() {
    let (mut ui, status, root) = setup(ProgressStatus::Idle);
    let width = ui.bounds(&bar(&ui, &root)).unwrap().unwrap().width();
    let gap = ThemeMetrics::default().metric(MetricRole::ProgressHeight) / 2.0;
    let close = |found: Vec<(f32, f32, ColorRole)>, expected: &[(f32, f32, ColorRole)]| {
        assert_eq!(found.len(), expected.len(), "{found:?} vs {expected:?}");
        for (a, b) in found.iter().zip(expected) {
            assert!(
                (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3 && a.2 == b.2,
                "{found:?} vs {expected:?}"
            );
        }
    };
    let at =
        |fraction: f32| ProgressStatus::Determinate(NormalizedValue::try_from(fraction).unwrap());
    use ColorRole::{Outline, Primary};

    // Half: the full gap, then the rest of the track.
    status.set(at(0.5)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let half = width / 2.0;
    close(
        pieces(&mut ui, &root),
        &[
            (half + gap, width - half - gap, Outline),
            (0.0, half, Primary),
        ],
    );
    // A tiny fill keeps the full gap after it.
    status.set(at(0.001)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let tiny = width * 0.001;
    close(
        pieces(&mut ui, &root),
        &[
            (tiny + gap, width - tiny - gap, Outline),
            (0.0, tiny, Primary),
        ],
    );
    // Near full, the gap takes half the remainder, so the track still shows.
    let fraction = 1.0 - gap / width;
    status.set(at(fraction)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let filled = width * fraction;
    let rest = width - filled;
    close(
        pieces(&mut ui, &root),
        &[
            (filled + rest / 2.0, rest / 2.0, Outline),
            (0.0, filled, Primary),
        ],
    );

    // A still indeterminate segment has the track on both sides.
    assert!(
        ui.ui_settings()
            .set_motion(MotionPreference::Reduced)
            .unwrap()
    );
    status.set(ProgressStatus::Indeterminate).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let segment = width * 0.3;
    let start = (width - segment) / 2.0;
    close(
        pieces(&mut ui, &root),
        &[
            (0.0, start - gap, Outline),
            (start + segment + gap, start - gap, Outline),
            (start, segment, Primary),
        ],
    );
}
