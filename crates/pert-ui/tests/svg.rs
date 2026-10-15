//! SVG rasterization through the public facade: what a drawing becomes, when it
//! is rasterized again, and what is deliberately not supported.
#![cfg(feature = "svg")]
use pert_ui::svg::{self, SvgError};
use pert_ui::*;
use std::time::Duration;

const CHECK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
  <path d="M2 8 L6 12 L14 3" stroke="black" stroke-width="3" fill="none"/>
</svg>"##;
const WIDE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 40 10">
  <rect x="0" y="0" width="40" height="10" fill="#3060c0"/>
</svg>"##;
const WITH_TEXT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
  <text x="1" y="12" font-size="12">Hi</text>
</svg>"##;

#[test]
fn extreme_raster_dimensions_are_rejected_before_allocation() {
    assert_eq!(
        svg::rasterize(CHECK, u32::MAX, u32::MAX),
        Err(SvgError::TooLarge {
            width: u32::MAX,
            height: u32::MAX
        })
    );
    assert_eq!(
        svg::rasterize_mask(CHECK, u32::MAX, u32::MAX),
        Err(SvgError::TooLarge {
            width: u32::MAX,
            height: u32::MAX
        })
    );
}

fn ui(width: f32, height: f32, scale: f64) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, scale).unwrap())
        .unwrap();
    ui
}

fn scene(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}

fn property(ui: &Runtime, key: &str, name: &str) -> Option<InspectValue> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key)
        .and_then(|component| component.details.properties.get(name).cloned())
}

fn coverage(mask: &MaskImage) -> usize {
    mask.pixels().iter().filter(|value| **value > 0).count()
}

#[test]
fn a_drawing_becomes_either_coverage_or_color() {
    let mask = svg::rasterize_mask(CHECK, 32, 32).unwrap();
    assert_eq!((mask.width(), mask.height()), (32, 32));
    assert!(coverage(&mask) > 0, "the stroke left coverage");
    assert!(
        coverage(&mask) < 32 * 32,
        "an icon is a shape, not a filled square",
    );
    let image = svg::rasterize(WIDE, 40, 10).unwrap();
    assert_eq!((image.width(), image.height()), (40, 10));
    assert_eq!(
        svg::intrinsic_size(WIDE).unwrap(),
        Size::new(40.0, 10.0).unwrap()
    );
}

#[test]
fn svg_text_is_not_rendered_and_says_so() {
    // The scope decision: no second font registry, so `<text>` contributes
    // nothing. It must not fail either, or an asset with a stray label would be
    // unusable as a whole.
    let mask = svg::rasterize_mask(WITH_TEXT, 32, 32).unwrap();
    assert_eq!(coverage(&mask), 0, "text left no coverage");
}

#[test]
fn a_source_that_is_not_a_drawing_is_rejected_before_it_is_mounted() {
    assert!(matches!(Svg::new(b"not an svg"), Err(SvgError::Parse(_))));
    assert!(matches!(
        svg::rasterize(CHECK, 0, 16),
        Err(SvgError::InvalidSize),
    ));
}

#[test]
fn a_tinted_icon_is_one_mask_that_every_theme_color_reuses() {
    let mut ui = ui(64.0, 64.0, 1.0);
    ui.mount_tree(Element::new(
        "icon",
        Svg::tinted(CHECK, ColorRole::Primary).unwrap(),
    ))
    .unwrap();
    let dark = scene(&mut ui);
    let dark_key = dark.masks()[0].content_key();
    let tint = |scene: &DisplayList| {
        scene
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::DrawMask { color, .. } => Some(*color),
                _ => None,
            })
            .expect("the icon drew a mask")
    };
    let theme = ui.theme();
    let dark_color = theme
        .palette(ThemeMode::Dark)
        .unwrap()
        .color(ColorRole::Primary);
    assert_eq!(tint(&dark), dark_color);

    theme.set_mode(ThemeMode::Light).unwrap();
    let light = scene(&mut ui);
    let light_color = theme
        .palette(ThemeMode::Light)
        .unwrap()
        .color(ColorRole::Primary);
    assert_eq!(tint(&light), light_color);
    assert_ne!(
        dark_color, light_color,
        "the modes differ, so this proves something"
    );
    assert_eq!(
        light.masks()[0].content_key(),
        dark_key,
        "the coverage is the same rasterization; only the tint changed",
    );
}

#[test]
fn rasterization_follows_the_physical_size_and_nothing_else() {
    let mut ui = ui(64.0, 64.0, 1.0);
    ui.mount_tree(Element::new(
        "icon",
        Svg::tinted(CHECK, ColorRole::Primary)
            .unwrap()
            .size(Size::new(16.0, 16.0).unwrap()),
    ))
    .unwrap();
    let first = scene(&mut ui);
    let pixels = |ui: &Runtime| match property(ui, "icon", "pixel_width") {
        Some(InspectValue::Number(width)) => width,
        other => panic!("no rasterized width: {other:?}"),
    };
    // As the root the icon is allocated the whole window and fills it, keeping
    // its proportions, exactly as a raster Image does.
    let logical = pixels(&ui);
    let key = first.masks()[0].content_key();

    // A repaint at the same size reuses the rasterization.
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    assert_eq!(scene(&mut ui).masks()[0].content_key(), key);

    // Zoom enlarges the drawing in logical pixels, but the window still covers
    // the same physical ones, so there is nothing new to rasterize.
    ui.ui_settings()
        .set_zoom(UiZoom::new(2.0).unwrap())
        .unwrap();
    let zoomed = scene(&mut ui);
    assert_eq!(pixels(&ui), logical);
    assert_eq!(zoomed.masks()[0].content_key(), key);

    // A display with twice the scale does have twice the pixels.
    ui.ui_settings()
        .set_zoom(UiZoom::new(1.0).unwrap())
        .unwrap();
    ui.set_viewport(Viewport::new(64.0, 64.0, 2.0).unwrap())
        .unwrap();
    let dense = scene(&mut ui);
    assert_eq!(
        pixels(&ui),
        logical * 2.0,
        "twice the scale rasterizes twice the pixels",
    );
    assert_ne!(dense.masks()[0].content_key(), key);
}

#[test]
fn a_drawing_keeps_its_proportions_inside_a_wider_allocation() {
    let mut ui = ui(200.0, 200.0, 1.0);
    ui.mount_tree(Element::new("wide", Svg::new(WIDE).unwrap()))
        .unwrap();
    let scene = scene(&mut ui);
    let rect = scene
        .commands()
        .iter()
        .find_map(|command| match command {
            DrawCommand::DrawImage { rect, .. } => Some(*rect),
            _ => None,
        })
        .expect("the drawing drew an image");
    // 40x10 in a 200x200 window: as wide as the window, centered vertically.
    assert_eq!(rect.width(), 200.0);
    assert_eq!(rect.height(), 50.0);
    assert_eq!(rect.y(), 75.0);
}

#[test]
fn replacing_the_source_replaces_the_drawing() {
    let mut ui = ui(64.0, 64.0, 1.0);
    let root = ui
        .mount_tree(Element::new(
            "page",
            Container::new(Element::new("icon", Svg::new(CHECK).unwrap())),
        ))
        .unwrap();
    scene(&mut ui);
    let before = property(&ui, "icon", "pixel_height");
    ui.reconcile_children(&root, vec![Element::new("icon", Svg::new(WIDE).unwrap())])
        .unwrap();
    scene(&mut ui);
    assert_ne!(
        property(&ui, "icon", "pixel_height"),
        before,
        "the new drawing has its own proportions",
    );
}

fn images_and_masks(scene: &DisplayList) -> (usize, usize) {
    scene
        .commands()
        .iter()
        .fold((0, 0), |(images, masks), command| match command {
            DrawCommand::DrawImage { .. } => (images + 1, masks),
            DrawCommand::DrawMask { .. } => (images, masks + 1),
            _ => (images, masks),
        })
}

#[test]
fn switching_between_color_and_tint_rasterizes_the_other_kind() {
    let mut ui = ui(64.0, 64.0, 1.0);
    let root = ui
        .mount_tree(Element::new(
            "page",
            Container::new(Element::new("icon", Svg::new(CHECK).unwrap())),
        ))
        .unwrap();
    assert_eq!(images_and_masks(&scene(&mut ui)), (1, 0));

    let tinted = |role| Element::new("icon", Svg::tinted(CHECK, role).unwrap());
    ui.reconcile_children(&root, vec![tinted(ColorRole::Primary)])
        .unwrap();
    let primary = scene(&mut ui);
    assert_eq!(images_and_masks(&primary), (0, 1), "the tint draws a mask");
    let key = primary.masks()[0].content_key();

    // Another role is the same coverage in another color: a repaint.
    ui.reconcile_children(&root, vec![tinted(ColorRole::Error)])
        .unwrap();
    let error = scene(&mut ui);
    assert_eq!(images_and_masks(&error), (0, 1));
    assert_eq!(error.masks()[0].content_key(), key);

    ui.reconcile_children(&root, vec![Element::new("icon", Svg::new(CHECK).unwrap())])
        .unwrap();
    assert_eq!(
        images_and_masks(&scene(&mut ui)),
        (1, 0),
        "without the tint the drawing keeps its colors again",
    );
}

#[test]
fn an_oversized_raster_is_refused_before_it_is_rendered() {
    // A color image may hold 64 MiB, which 5000x5000 RGBA exceeds.
    assert_eq!(
        svg::rasterize(CHECK, 5000, 5000).unwrap_err(),
        SvgError::TooLarge {
            width: 5000,
            height: 5000
        },
    );
    // A mask may hold 16 Mi pixels; these would need gigabytes to render.
    assert_eq!(
        svg::rasterize_mask(CHECK, 20000, 20000).unwrap_err(),
        SvgError::TooLarge {
            width: 20000,
            height: 20000
        },
    );
    assert!(matches!(
        svg::rasterize_mask(CHECK, 60000, 60000),
        Err(SvgError::TooLarge { .. })
    ));
    // Exactly at each limit still rasterizes.
    assert_eq!(svg::rasterize(CHECK, 4096, 4096).unwrap().width(), 4096);
    assert_eq!(
        svg::rasterize_mask(CHECK, 4096, 4096).unwrap().width(),
        4096
    );
    // The component reports the limit rather than failing layout.
    let mut ui = ui(9000.0, 9000.0, 1.0);
    ui.mount_tree(Element::new("huge", Svg::new(CHECK).unwrap()))
        .unwrap();
    assert_eq!(images_and_masks(&scene(&mut ui)), (0, 0));
    assert!(matches!(
        property(&ui, "huge", "failure"),
        Some(InspectValue::Text(message)) if message.contains("9000x9000")
    ));
}

#[test]
fn an_image_element_reads_no_file_and_no_embedded_data() {
    let directory = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"));
    let referenced = directory.join("svg-external-red.svg");
    let red = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><rect width="16" height="16" fill="#ff0000"/></svg>"##;
    std::fs::write(&referenced, red).unwrap();
    // The same drawing embedded as a data URL, percent-encoded.
    let embedded = red
        .replace('%', "%25")
        .replace('#', "%23")
        .replace('"', "%22")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace(' ', "%20");
    for href in [
        referenced.display().to_string(),
        format!("data:image/svg+xml,{embedded}"),
    ] {
        let source = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="0 0 16 16">
  <image xlink:href="{href}" width="16" height="16"/>
</svg>"#
        );
        let mask = svg::rasterize_mask(source.as_bytes(), 16, 16).unwrap();
        assert_eq!(coverage(&mask), 0, "{href} drew nothing");
    }
}

fn nested(depth: usize) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">{}<rect width="8" height="8"/>{}</svg>"#,
        r#"<g opacity="0.99">"#.repeat(depth - 2),
        "</g>".repeat(depth - 2),
    )
}

#[test]
fn excessive_nesting_is_rejected_before_it_can_exhaust_the_stack() {
    // A spawned thread's default stack, which a worker reading assets has.
    let on_small_stack = |source: String| {
        std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn(move || {
                (
                    svg::rasterize_mask(source.as_bytes(), 16, 16).map(|mask| coverage(&mask)),
                    Svg::new(source.as_bytes()).err(),
                )
            })
            .unwrap()
            .join()
            .expect("parsing stays within the stack")
    };
    let (deepest, _) = on_small_stack(nested(svg::MAX_NESTING));
    assert!(deepest.unwrap() > 0, "the deepest accepted drawing renders");
    // Entities without markup, as some editors declare namespaces, and tags
    // inside comments do not count.
    let (declared, _) = on_small_stack(format!(
        r#"<?xml version="1.0"?>
<!DOCTYPE svg [<!ENTITY ns_svg "http://www.w3.org/2000/svg">]>
<!-- {} -->
<svg xmlns="&ns_svg;" viewBox="0 0 16 16"><rect width="8" height="8"/></svg>"#,
        "<g>".repeat(svg::MAX_NESTING * 2),
    ));
    assert!(declared.unwrap() > 0);

    let half = svg::MAX_NESTING / 2;
    for source in [
        nested(svg::MAX_NESTING + 1),
        nested(20_000),
        // An entity's elements count where it is expanded.
        format!(
            r#"<!DOCTYPE svg [<!ENTITY deep "{}{}">]>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">{}&deep;{}</svg>"#,
            "<g>".repeat(half + 1),
            "</g>".repeat(half + 1),
            "<g>".repeat(half),
            "</g>".repeat(half),
        ),
        // A chain of entities, each nearly as deep as allowed, expands the
        // deepest nesting the XML parser itself recurses through.
        {
            let near = "<g>".repeat(svg::MAX_NESTING - 2);
            let close = "</g>".repeat(svg::MAX_NESTING - 2);
            let entities: String = (0..9)
                .map(|level| {
                    let inner = if level < 8 {
                        format!("&e{};", level + 1)
                    } else {
                        String::new()
                    };
                    format!(r#"<!ENTITY e{level} "{near}{inner}{close}">"#)
                })
                .collect();
            format!(
                r#"<!DOCTYPE svg [{entities}]>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">&e0;</svg>"#
            )
        },
    ] {
        let (rasterized, constructed) = on_small_stack(source);
        assert!(matches!(rasterized, Err(SvgError::Parse(_))));
        assert!(matches!(constructed, Some(SvgError::Parse(_))));
    }
}

struct SvgView {
    view: ReadSignal<ViewTransform>,
    current: ViewTransform,
    child: Option<Element>,
}
impl Component for SvgView {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.current = cx.read(&self.view)?;
        Ok(())
    }
    fn child_transform(&self) -> ViewTransform {
        self.current
    }
    fn measure(&self, _: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(
            a.width().unwrap_or(128.0),
            a.height().unwrap_or(128.0),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        let size = cx.measure(0, Constraints::default())?;
        cx.place(0, Rect::from_size(size))
    }
}
fn svg_view(scale: f32, x: f32, y: f32) -> ViewTransform {
    ViewTransform::new(ViewScale::new(scale).unwrap(), Point::new(x, y).unwrap())
}
fn magnified_svg(
    tinted: bool,
    window: f64,
    scales: &[f32],
) -> (Runtime, WidgetId, Vec<Signal<ViewTransform>>) {
    let mut ui = ui(256.0, 256.0, window);
    let art = if tinted {
        Svg::tinted(CHECK, ColorRole::Primary).unwrap()
    } else {
        Svg::new(CHECK).unwrap()
    };
    let mut child = Element::new("art", art.size(Size::new(8.0, 8.0).unwrap()));
    let mut transforms = Vec::new();
    for (index, scale) in scales.iter().rev().enumerate() {
        let transform = ui.signal(svg_view(*scale, 0.0, 0.0));
        child = Element::new(
            format!("view{index}"),
            SvgView {
                view: transform.read_only(),
                current: ViewTransform::IDENTITY,
                child: Some(child),
            },
        );
        transforms.push(transform);
    }
    let root = ui.mount_tree(child).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    (ui, root, transforms)
}
fn svg_pixels(scene: &DisplayList, tinted: bool) -> (u32, u32, u64) {
    if tinted {
        assert_eq!(scene.masks().len(), 1);
        assert!(scene.images().is_empty());
        let mask = &scene.masks()[0];
        (mask.width(), mask.height(), mask.content_key())
    } else {
        assert_eq!(scene.images().len(), 1);
        assert!(scene.masks().is_empty());
        let image = &scene.images()[0];
        (image.width(), image.height(), image.content_key())
    }
}

#[test]
fn svg_rasters_follow_accumulated_view_density_and_its_cap() {
    for tinted in [false, true] {
        for (window, scales, expected) in [
            (1.25, vec![4.0], 40),
            (1.25, vec![2.0, 4.0], 64),
            (10.0, vec![4.0], 80),
            (1.25, vec![0.5], 5),
        ] {
            let (mut ui, _, _) = magnified_svg(tinted, window, &scales);
            let (width, height, _) = svg_pixels(&ui.paint().unwrap(), tinted);
            assert_eq!(
                (width, height),
                (expected, expected),
                "DPI={window}, views={scales:?}"
            );
        }
    }
}

#[test]
fn svg_pan_and_theme_changes_reuse_coverage_and_keep_identity() {
    for tinted in [false, true] {
        let (mut ui, root, transforms) = magnified_svg(tinted, 1.25, &[4.0]);
        let id = ui.find_descendant(&root, "art").unwrap().unwrap();
        let original = svg_pixels(&ui.paint().unwrap(), tinted);
        transforms[0].set(svg_view(4.0, 20.0, 15.0)).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(svg_pixels(&ui.paint().unwrap(), tinted), original);
        ui.theme().set_mode(ThemeMode::Light).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(svg_pixels(&ui.paint().unwrap(), tinted), original);
        assert_eq!(ui.find_descendant(&root, "art").unwrap(), Some(id));
        assert_eq!(ui.next_deadline(), None);
    }
}

#[test]
fn svg_continuous_zoom_reuses_coverage_until_its_settled_deadline() {
    for tinted in [false, true] {
        let (mut ui, root, transforms) = magnified_svg(tinted, 1.25, &[1.0]);
        let id = ui.find_descendant(&root, "art").unwrap().unwrap();
        let frame = Duration::from_millis(16);
        transforms[0].set(svg_view(1.5, 0.0, 0.0)).unwrap();
        ui.flush(frame).unwrap();
        let single = svg_pixels(&ui.paint().unwrap(), tinted);
        assert_eq!((single.0, single.1), (15, 15));
        transforms[0].set(svg_view(4.0, 0.0, 0.0)).unwrap();
        ui.flush(frame * 2).unwrap();
        assert_eq!(svg_pixels(&ui.paint().unwrap(), tinted), single);
        let settle = ui
            .next_deadline()
            .expect("continuous zoom schedules settled preparation");
        assert!(settle > frame * 2);
        ui.flush(settle - Duration::from_millis(1)).unwrap();
        assert_eq!(svg_pixels(&ui.paint().unwrap(), tinted), single);
        assert_eq!(ui.next_deadline(), Some(settle));
        ui.flush(settle).unwrap();
        let final_raster = svg_pixels(&ui.paint().unwrap(), tinted);
        assert_eq!((final_raster.0, final_raster.1), (40, 40));
        assert_ne!(final_raster.2, single.2);
        assert_eq!(ui.find_descendant(&root, "art").unwrap(), Some(id));
        assert_eq!(ui.next_deadline(), None);
    }
}

mod svg_prelude_consumer {
    use pert_ui::prelude::*;
    #[test]
    fn svg_is_available_from_the_feature_enabled_prelude() {
        let mut ui = Runtime::new();
        let art = Svg::new(super::CHECK)
            .unwrap()
            .size(Size::new(8.0, 8.0).unwrap());
        ui.mount_tree(Element::new("art", art)).unwrap();
        ui.flush(std::time::Duration::ZERO).unwrap();
        assert_eq!(ui.inspect().components[0].key, "art");
    }
}

#[test]
fn promoting_svg_to_a_surface_resets_density_and_demoting_restores_it() {
    struct Promoter {
        promote: ReadSignal<bool>,
        promoted: bool,
        child: Option<Element>,
    }
    impl Component for Promoter {
        fn take_children(&mut self) -> Vec<Element> {
            self.child.take().into_iter().collect()
        }
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.promoted = cx.read(&self.promote)?;
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(32.0, 32.0)?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            cx.place(0, Rect::from_size(Size::new(8.0, 8.0)?))
        }
        fn child_surface(&self, _: &str) -> Option<SurfaceKind> {
            self.promoted.then_some(SurfaceKind::Tooltip)
        }
    }
    for tinted in [false, true] {
        let mut ui = ui(256.0, 256.0, 1.25);
        let transform = ui.signal(svg_view(4.0, 0.0, 0.0));
        let promote = ui.signal(false);
        let art = if tinted {
            Svg::tinted(CHECK, ColorRole::Primary).unwrap()
        } else {
            Svg::new(CHECK).unwrap()
        };
        let root = ui
            .mount_tree(Element::new(
                "view",
                SvgView {
                    view: transform.read_only(),
                    current: ViewTransform::IDENTITY,
                    child: Some(Element::new(
                        "promoter",
                        Promoter {
                            promote: promote.read_only(),
                            promoted: false,
                            child: Some(Element::new(
                                "art",
                                art.size(Size::new(8.0, 8.0).unwrap()),
                            )),
                        },
                    )),
                },
            ))
            .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let id = ui.find_descendant(&root, "art").unwrap().unwrap();
        let original = svg_pixels(&ui.paint().unwrap(), tinted);
        assert_eq!((original.0, original.1), (40, 40));
        promote.set(true).unwrap();
        ui.flush(ui.now()).unwrap();
        let promoted = svg_pixels(&ui.paint().unwrap(), tinted);
        assert_eq!((promoted.0, promoted.1), (10, 10));
        assert_ne!(promoted.2, original.2);
        promote.set(false).unwrap();
        ui.flush(ui.now()).unwrap();
        assert_eq!(svg_pixels(&ui.paint().unwrap(), tinted), original);
        assert_eq!(ui.find_descendant(&root, "art").unwrap(), Some(id));
        assert_eq!(ui.next_deadline(), None);
    }
}
