//! Density, typography and custom-widget measurement dependencies.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
struct MetricProbe {
    role: MetricRole,
    measures: Rc<Cell<usize>>,
    paints: Rc<Cell<usize>>,
    paint_only: bool,
}
impl Component for MetricProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.measures.set(self.measures.get() + 1);
        Ok(Size::new(
            if self.paint_only {
                10.0
            } else {
                cx.metric(self.role)
            },
            10.0,
        )?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        if self.paint_only {
            let _ = painter.metric(self.role);
        }
        Ok(())
    }
}
#[test]
fn inactive_presets_noops_and_color_modes_preserve_independent_overrides() {
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let custom = ThemeMetrics::for_density(Density::Compact)
        .with_metric(MetricRole::ItemGap, 7.into())
        .unwrap();
    assert_eq!(theme.density().unwrap(), Density::Standard);
    draw(&mut ui);
    theme.set_metrics(Density::Compact, custom).unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert!(!ui.needs_paint());
    theme.set_density(Density::Compact).unwrap();
    theme.set_mode(ThemeMode::Light).unwrap();
    assert_eq!(theme.density().unwrap(), Density::Compact);
    assert_eq!(theme.metrics(Density::Compact).unwrap(), custom);
    assert!(!theme.set_metrics(Density::Compact, custom).unwrap());
    assert!(!theme.set_density(Density::Compact).unwrap());
    drop(ui);
    assert_eq!(
        theme.set_density(Density::Standard),
        Err(RuntimeError::Closed)
    );
}
#[test]
fn metric_changes_invalidate_only_layout_or_paint_readers_and_defer_hidden_work() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 100.0, 1.0).unwrap())
        .unwrap();
    let mut probes = Vec::new();
    let elements = [
        (MetricRole::ItemGap, false),
        (MetricRole::ItemGap, true),
        (MetricRole::HeadingSize, false),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (role, paint_only))| {
        let measures = Rc::new(Cell::new(0));
        let paints = Rc::new(Cell::new(0));
        probes.push((measures.clone(), paints.clone()));
        Element::new(
            format!("probe-{i}"),
            MetricProbe {
                role,
                measures,
                paints,
                paint_only,
            },
        )
    })
    .collect::<Vec<_>>();
    let root = ui
        .mount_tree(Element::new("row", Row::new(elements).gap(Length::from(0))))
        .unwrap();
    draw(&mut ui);
    let before: Vec<_> = probes.iter().map(|(m, p)| (m.get(), p.get())).collect();
    ui.theme().set_density(Density::Compact).unwrap();
    draw(&mut ui);
    assert!(probes[0].0.get() > before[0].0);
    assert!(probes[0].1.get() > before[0].1);
    assert_eq!(probes[1].0.get(), before[1].0);
    // This sibling also moves because the first changes width; it still need not remeasure.
    assert!(probes[1].1.get() > before[1].1);
    assert_eq!(probes[2].0.get(), before[2].0);
    ui.set_visible(&root, false).unwrap();
    draw(&mut ui);
    ui.theme().set_density(Density::Comfortable).unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
    assert!(!ui.needs_paint());
    assert_eq!(ui.next_deadline(), None);
    ui.set_visible(&root, true).unwrap();
    draw(&mut ui);
    ui.unmount(&root).unwrap();
    draw(&mut ui);
    ui.theme().set_density(Density::Standard).unwrap();
    assert_eq!(ui.flush(Duration::ZERO).unwrap().components_updated, 0);
}
#[test]
fn live_group_padding_fixed_overrides_and_text_sizes_are_independent() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(500.0, 300.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "group",
            Group::new(Element::new(
                "column",
                Column::new([
                    Element::new(
                        "fixed",
                        Container::new(Element::new("label", Label::new("Fixed")))
                            .padding(Length::from(3)),
                    ),
                    Element::new("button", Button::new("Action", |_| Ok(()))),
                ]),
            )),
        ))
        .unwrap();
    draw(&mut ui);
    let column = ui.children(&root).unwrap()[0].clone();
    let children = ui.children(&column).unwrap();
    let label = ui.children(&children[0]).unwrap()[0].clone();
    let old_label_height = ui.bounds(&label).unwrap().unwrap().height();
    for (density, padding) in [
        (Density::Compact, 8.0),
        (Density::Standard, 12.0),
        (Density::Comfortable, 16.0),
    ] {
        ui.theme().set_density(density).unwrap();
        draw(&mut ui);
        assert_eq!(ui.bounds(&column).unwrap().unwrap().x(), padding);
        assert_eq!(ui.bounds(&label).unwrap().unwrap().x(), padding + 3.0);
        assert_eq!(
            ui.bounds(&label).unwrap().unwrap().height(),
            old_label_height
        );
        assert_eq!(ui.children(&column).unwrap(), children);
    }
    let metrics = ui
        .theme()
        .metrics(Density::Comfortable)
        .unwrap()
        .with_metric(MetricRole::BodySize, 22.into())
        .unwrap();
    ui.theme()
        .set_metrics(Density::Comfortable, metrics)
        .unwrap();
    draw(&mut ui);
    assert!(ui.bounds(&label).unwrap().unwrap().height() > old_label_height);
    assert!(
        ThemeMetrics::default()
            .with_metric(MetricRole::BodySize, 0.into())
            .is_err()
    );
    assert!(Length::new(f32::NAN).is_err());
    assert!(Length::new(f32::INFINITY).is_err());
    assert!(Length::new(-1.0).is_err());
}

#[test]
fn the_overlay_scrollbar_follows_density_like_every_other_control_extent() {
    fn thumb(ui: &mut Runtime) -> Rect {
        let theme = ui.theme();
        let resting = theme
            .palette(theme.mode().unwrap())
            .unwrap()
            .color(ColorRole::ScrollbarThumb);
        draw(ui)
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color } if *color == resting => Some(*rect),
                _ => None,
            })
            .expect("an overflowing scroll area paints its thumb")
    }
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0).unwrap())
        .unwrap();
    // Forty thousand pixels of content make the proportional thumb shorter than
    // any minimum, so its length shows the minimum-thumb metric as well.
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "tall",
            MetricProbeContent(Size::new(50.0, 40_000.0).unwrap()),
        )),
    ))
    .unwrap();
    for density in [Density::Compact, Density::Standard, Density::Comfortable] {
        ui.theme().set_density(density).unwrap();
        let metrics = ThemeMetrics::for_density(density);
        let thumb = thumb(&mut ui);
        assert_eq!(thumb.width(), metrics.get(MetricRole::ScrollbarThumb));
        assert_eq!(
            thumb.height(),
            metrics.get(MetricRole::ScrollbarMinimumThumb)
        );
        assert_eq!(thumb.x() + thumb.width(), 200.0);
    }
    // Standard keeps the extents the bar had before it read the theme.
    let standard = ThemeMetrics::default();
    assert_eq!(standard.get(MetricRole::ScrollbarLane), 8.0);
    assert_eq!(standard.get(MetricRole::ScrollbarThumb), 4.0);
    assert_eq!(standard.get(MetricRole::ScrollbarMinimumThumb), 12.0);
}
struct MetricProbeContent(Size);
impl Component for MetricProbeContent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(self.0)
    }
}
