//! Bundled typography selection, equal advances and retained font-specific masks.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

struct TypographyProbe(Rc<RefCell<Vec<Size>>>);
impl Component for TypographyProbe {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let mut sizes = Vec::new();
        for style in [TextStyle::Monospace, TextStyle::Body] {
            for text in ["iiiiiiii", "WWWWWWWW", "01234567", "i      i"] {
                sizes.push(cx.text(text, None, style)?.size());
            }
        }
        *self.0.borrow_mut() = sizes;
        Ok(Size::ZERO)
    }
}

#[test]
fn mono_letters_digits_and_spaces_align_at_multiple_scales() {
    let mut ui = Runtime::new();
    let sizes = Rc::new(RefCell::new(Vec::new()));
    ui.mount_tree(Element::new("probe", TypographyProbe(sizes.clone())))
        .unwrap();
    for scale in [1.0, 1.25, 2.0] {
        ui.set_viewport(Viewport::new(400.0, 200.0, scale).unwrap())
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let sizes = sizes.borrow();
        for size in &sizes[1..4] {
            assert_eq!(size.width(), sizes[0].width(), "scale {scale}: {sizes:?}");
        }
        assert!(sizes[4].width() < sizes[5].width());
        assert!(sizes[0].width() > sizes[4].width());
        assert!(sizes[0].width() < sizes[5].width());
    }
}

#[test]
fn body_and_mono_masks_remain_distinct_and_reused() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let value = ui.signal("Åäö 0123".to_owned());
    ui.mount_tree(Element::new(
        "labels",
        Column::new([
            Element::new("body", Label::new("Åäö 0123")),
            Element::new(
                "mono",
                Label::new(value.read_only()).style(TextStyle::Monospace),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let original = ui.paint().unwrap();
    assert_eq!(original.masks().len(), 2);
    assert_ne!(
        original.masks()[0].pixels().as_ptr(),
        original.masks()[1].pixels().as_ptr()
    );
    assert!(original.masks()[1].pixels().iter().any(|alpha| *alpha > 0));
    value.set("Changed".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let changed = ui.paint().unwrap();
    assert_eq!(
        original.masks()[0].pixels().as_ptr(),
        changed.masks()[0].pixels().as_ptr()
    );
    value.set("Åäö 0123".to_owned()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let restored = ui.paint().unwrap();
    assert_eq!(
        original.masks()[1].pixels().as_ptr(),
        restored.masks()[1].pixels().as_ptr()
    );
    assert!(ui.next_deadline().is_none());
}

#[test]
fn row_centers_label_and_button_text_at_each_scale() {
    for scale in [1.0, 1.25, 2.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 160.0, scale).unwrap())
            .unwrap();
        let root = ui
            .mount_tree(Element::new(
                "row",
                Row::new([
                    Element::new("increment", Button::new("Increment", |_| Ok(()))),
                    Element::new("reset", Button::new("Reset", |_| Ok(()))),
                    Element::new("count", Label::new("Count: 0").style(TextStyle::Monospace)),
                ]),
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        let centers: Vec<_> = scene
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::DrawMask { rect, color, .. } if *color == Color::TEXT => {
                    let [_, y, _, h] = rect.values();
                    Some(y + h / 2.0)
                }
                _ => None,
            })
            .collect();
        assert_eq!(centers.len(), 3);
        assert!(
            centers
                .iter()
                .all(|center| (*center - centers[0]).abs() < 0.001),
            "scale {scale}: {centers:?}"
        );
        // Extra allocation below the natural row must not move the entire line.
        let first = ui.children(&root).unwrap()[0].clone();
        assert_eq!(ui.bounds(&first).unwrap().unwrap().y(), 0.0);
    }
}

#[test]
fn filled_buttons_and_tab_cards_have_distinct_default_surfaces() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new("button", Button::new("Action", |_| Ok(()))),
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("a", "First", Element::new("label", Label::new("Text"))),
                    Tab::new("b", "Second", Element::new("label", Label::new("Text"))),
                ]),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let scene = ui.paint().unwrap();
    for expected in [Color::BUTTON, Color::TAB, Color::TAB_SELECTED] {
        assert!(scene.commands().iter().any(|c| matches!(c, DrawCommand::DrawMask { color, rect, .. } if *color == expected && rect.width() > 0.0 && rect.height() > 0.0)));
        assert_ne!(expected, Color::SURFACE);
        assert_eq!(expected.alpha(), 1.0);
    }
    assert_ne!(Color::BUTTON, Color::TAB);
    assert_ne!(Color::BUTTON, Color::TAB_SELECTED);
}

struct PaintedReadout {
    source: ReadSignal<String>,
    text: String,
}
impl Component for PaintedReadout {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.text = cx.read_paint(&self.source)?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 40.0)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let text = p.text(&self.text, None, TextStyle::MonospaceCaption)?;
        text.paint(p, Point::ZERO, p.color(ColorRole::OnSurface))
    }
}
#[test]
fn public_paint_text_reuses_coverage_across_colors_and_recovers_from_text_errors() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    let source = ui.signal("12.00".to_owned());
    ui.mount_tree(Element::new(
        "readout",
        PaintedReadout {
            source: source.read_only(),
            text: String::new(),
        },
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let first = ui.paint().unwrap();
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let recolored = ui.paint().unwrap();
    assert_eq!(
        first.masks()[0].pixels().as_ptr(),
        recolored.masks()[0].pixels().as_ptr()
    );
    source.set("0".repeat(17 * 1024)).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    assert!(matches!(
        ui.paint(),
        Err(DrawError::Runtime(RuntimeError::TextEdit(
            TextEditError::TooLarge
        )))
    ));
    assert!(ui.needs_paint());
    source.set("12.00".into()).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let recovered = ui.paint().unwrap();
    assert_eq!(
        first.masks()[0].pixels().as_ptr(),
        recovered.masks()[0].pixels().as_ptr()
    );
    assert!(!ui.needs_paint());
}

#[test]
fn text_above_the_raster_limit_is_stretched_under_scale_and_zoom() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1600.0, 800.0, 2.0).unwrap())
        .unwrap();
    let metrics = ThemeMetrics::default()
        .with_metric(MetricRole::HeadingSize, Length::new(150.0).unwrap())
        .unwrap();
    ui.theme().set_metrics(Density::Standard, metrics).unwrap();
    ui.mount_tree(Element::new(
        "title",
        Label::new("Mix").style(TextStyle::Heading),
    ))
    .unwrap();
    // The mask's physical height over its drawn logical height is the scale
    // the text was rasterized at.
    let raster = |ui: &mut Runtime| {
        ui.flush(Duration::ZERO).unwrap();
        let scene = ui.paint().unwrap();
        let (mask, rect) = scene
            .commands()
            .iter()
            .find_map(|c| match c {
                DrawCommand::DrawMask { mask, rect, .. } => Some((*mask, *rect)),
                _ => None,
            })
            .unwrap();
        let mask = scene.mask(mask).unwrap();
        (mask.height() as f32 / rect.height(), rect.height())
    };
    // 150 logical pixels at scale 2 would be 300 physical; 256 is the ceiling.
    let (scale, height) = raster(&mut ui);
    assert!((scale - 256.0 / 150.0).abs() < 0.01, "{scale}");
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    let (zoomed, zoomed_height) = raster(&mut ui);
    assert!((zoomed - scale).abs() < 0.01, "{zoomed}");
    assert!(
        (zoomed_height - height).abs() < 1.0,
        "{zoomed_height} vs {height}"
    );
    assert!(!ui.needs_paint());
}
