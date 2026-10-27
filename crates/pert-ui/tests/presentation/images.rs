//! Color images: the scene's deduplicating resource tables, the `DrawImage`
//! command, the `Image` component and, with a format feature, decoding.
use pert_ui::*;
use std::time::Duration;

fn ui(width: f32, height: f32) -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, height, 1.0).unwrap())
        .unwrap();
    ui
}
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
fn solid(width: u32, height: u32, pixel: [u8; 4]) -> RasterImage {
    RasterImage::from_srgba8(width, height, pixel.repeat((width * height) as usize)).unwrap()
}
fn images(scene: &DisplayList) -> Vec<(ImageId, [f32; 4], Option<[f32; 4]>)> {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawImage {
                image, rect, clip, ..
            } => Some((*image, rect.values(), clip.map(Rect::values))),
            _ => None,
        })
        .collect()
}

// Draws each image at its rectangle, inside an optional local clip.
struct Draw(Vec<(RasterImage, Rect, Option<Rect>)>);
impl Component for Draw {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        for (image, rect, clip) in &self.0 {
            match clip {
                Some(clip) => painter.with_clip(*clip, |painter| {
                    painter.draw_image(image, *rect, ImageSampling::Linear);
                }),
                None => painter.draw_image(image, *rect, ImageSampling::Linear),
            }
        }
        Ok(())
    }
}

#[test]
fn equal_images_from_different_allocations_share_one_scene_entry() {
    let mut ui = ui(100.0, 100.0);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0).unwrap();
    let red = [255, 0, 0, 255];
    // Two components, each with its own allocation of the same pixels.
    ui.mount_tree(Element::new(
        "a",
        Draw(vec![(solid(2, 2, red), rect, None)]),
    ))
    .unwrap();
    ui.mount_tree(Element::new(
        "b",
        Draw(vec![
            (solid(2, 2, red), rect, None),
            (solid(2, 2, [0, 0, 255, 255]), rect, None),
        ]),
    ))
    .unwrap();
    let scene = draw(&mut ui);
    assert_eq!(scene.images().len(), 2);
    let ids: Vec<_> = images(&scene).iter().map(|(id, ..)| id.index()).collect();
    assert_eq!(ids, [0, 0, 1]);
    assert_eq!(
        scene
            .image(images(&scene)[1].0)
            .map(RasterImage::content_key),
        Some(solid(2, 2, red).content_key())
    );
}

#[test]
fn resources_enter_the_scene_only_through_a_surviving_command() {
    let mut ui = ui(100.0, 100.0);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0).unwrap();
    let hidden = Rect::new(50.0, 50.0, 10.0, 10.0).unwrap();
    ui.mount_tree(Element::new(
        "images",
        Draw(vec![
            (solid(1, 1, [1, 1, 1, 255]), rect, Some(hidden)),
            (solid(1, 1, [2, 2, 2, 255]), rect, None),
        ]),
    ))
    .unwrap();
    let scene = draw(&mut ui);
    assert_eq!(
        scene.images().len(),
        1,
        "a clipped-out image is not uploaded"
    );
    assert_eq!(scene.images()[0], solid(1, 1, [2, 2, 2, 255]));
    assert_eq!(images(&scene)[0].0.index(), 0);
}

#[test]
fn an_image_keeps_its_destination_and_carries_its_clip_like_a_mask() {
    let mut ui = ui(100.0, 100.0);
    let rect = Rect::new(10.0, 10.0, 40.0, 20.0).unwrap();
    let clip = Rect::new(20.0, 0.0, 10.0, 100.0).unwrap();
    ui.mount_tree(Element::new(
        "image",
        Draw(vec![(solid(2, 2, [9, 9, 9, 255]), rect, Some(clip))]),
    ))
    .unwrap();
    let scene = draw(&mut ui);
    assert_eq!(
        images(&scene),
        [(
            images(&scene)[0].0,
            [10.0, 10.0, 40.0, 20.0],
            Some([20.0, 0.0, 10.0, 100.0])
        )]
    );
}

#[test]
fn the_component_places_the_image_by_its_fit() {
    let image = solid(4, 2, [128, 128, 128, 255]);
    let placed = |fit| {
        let mut ui = ui(100.0, 100.0);
        ui.mount_tree(Element::new("image", Image::new(image.clone()).fit(fit)))
            .unwrap();
        images(&draw(&mut ui))
            .into_iter()
            .map(|(_, rect, clip)| (rect, clip))
            .collect::<Vec<_>>()
    };
    // Every composed component carries its own allocation as its clip; Cover
    // relies on it to crop what overflows.
    let bounds = Some([0.0, 0.0, 100.0, 100.0]);
    assert_eq!(
        placed(ImageFit::Contain),
        [([0.0, 25.0, 100.0, 50.0], bounds)]
    );
    assert_eq!(
        placed(ImageFit::Cover),
        [([-50.0, 0.0, 200.0, 100.0], bounds)]
    );
    assert_eq!(placed(ImageFit::Fill), [([0.0, 0.0, 100.0, 100.0], bounds)]);
}

#[test]
fn the_component_prefers_the_image_size_and_shrinks_uniformly() {
    let measure = |width: f32| {
        let mut ui = ui(width, 200.0);
        ui.mount_tree(Element::new(
            "column",
            Column::new([Element::new(
                "image",
                Image::new(solid(40, 20, [0, 0, 0, 255])),
            )]),
        ))
        .unwrap();
        images(&draw(&mut ui))[0].1
    };
    assert_eq!(measure(200.0)[2..], [40.0, 20.0]);
    assert_eq!(measure(20.0)[2..], [20.0, 10.0]);
}

#[test]
fn a_bound_image_follows_its_signal() {
    let mut ui = ui(200.0, 200.0);
    let source = ui.signal(None);
    ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("image", Image::bound(source.read_only()))]),
    ))
    .unwrap();
    assert!(images(&draw(&mut ui)).is_empty(), "None draws nothing");
    source.set(Some(solid(40, 20, [0, 0, 0, 255]))).unwrap();
    assert_eq!(images(&draw(&mut ui))[0].1[2..], [40.0, 20.0]);
    // New dimensions lay the component out again.
    source.set(Some(solid(10, 30, [0, 0, 0, 255]))).unwrap();
    assert_eq!(images(&draw(&mut ui))[0].1[2..], [10.0, 30.0]);
    source.set(None).unwrap();
    assert!(images(&draw(&mut ui)).is_empty());
}

#[test]
fn unchanged_bound_image_redescription_and_same_size_pixels_keep_layout() {
    let mut ui = ui(200.0, 200.0);
    let source = ui.signal(Some(solid(40, 20, [0, 0, 0, 255])));
    let root = ui
        .mount_tree(Element::new(
            "column",
            Column::new([Element::new("image", Image::bound(source.read_only()))]),
        ))
        .unwrap();
    draw(&mut ui);
    ui.set_profiling(true).unwrap();
    ui.reconcile_children(
        &root,
        vec![Element::new("image", Image::bound(source.read_only()))],
    )
    .unwrap();
    draw(&mut ui);
    let image_work: Vec<_> = ui
        .profile()
        .stats
        .into_iter()
        .filter(|stat| {
            stat.key == "image"
                && matches!(
                    stat.operation,
                    ProfileOperation::Update
                        | ProfileOperation::Measure
                        | ProfileOperation::Arrange
                        | ProfileOperation::Paint
                )
        })
        .collect();
    assert!(image_work.is_empty(), "{image_work:?}");
    ui.clear_profile().unwrap();
    source.set(Some(solid(40, 20, [255, 0, 0, 255]))).unwrap();
    let scene = draw(&mut ui);
    assert_eq!(images(&scene)[0].1[2..], [40.0, 20.0]);
    assert!(!ui.profile().stats.iter().any(|stat| stat.key == "image"
        && matches!(
            stat.operation,
            ProfileOperation::Measure | ProfileOperation::Arrange
        )));
    assert!(
        ui.profile()
            .stats
            .iter()
            .any(|stat| stat.key == "image" && stat.operation == ProfileOperation::Paint)
    );
}

#[test]
fn the_component_reports_image_semantics_with_its_text_alternative() {
    let mut ui = ui(100.0, 100.0);
    ui.mount_tree(Element::new(
        "logo",
        Image::new(solid(4, 2, [0, 0, 0, 255])).description("Company logo"),
    ))
    .unwrap();
    draw(&mut ui);
    let component = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "logo")
        .unwrap();
    let semantics = component.details.semantics.unwrap();
    assert_eq!(semantics.role, SemanticRole::Image);
    assert_eq!(semantics.name.as_deref(), Some("Company logo"));
    assert_eq!(
        component.details.properties.get("pixels"),
        Some(&InspectValue::Text("4x2".into()))
    );
}

#[test]
fn an_empty_allocation_draws_nothing() {
    // A zero-size allocation records nothing; the component must not divide by
    // zero into non-finite geometry.
    let mut ui = ui(0.0, 0.0);
    ui.mount_tree(Element::new(
        "image",
        Image::new(solid(4, 2, [0, 0, 0, 255])),
    ))
    .unwrap();
    assert!(images(&draw(&mut ui)).is_empty());
}

#[cfg(feature = "images")]
mod decoding {
    use pert_ui::images::{self, ImageError, ImageFormat};

    #[cfg(feature = "png")]
    const RGBA_PNG: &[u8] = include_bytes!("../fixtures/rgba.png");
    const OPAQUE: [(&str, &[u8], ImageFormat); 7] = [
        (
            "gif",
            include_bytes!("../fixtures/opaque.gif"),
            ImageFormat::Gif,
        ),
        (
            "webp",
            include_bytes!("../fixtures/opaque.webp"),
            ImageFormat::WebP,
        ),
        (
            "bmp",
            include_bytes!("../fixtures/opaque.bmp"),
            ImageFormat::Bmp,
        ),
        (
            "ico",
            include_bytes!("../fixtures/opaque.ico"),
            ImageFormat::Ico,
        ),
        (
            "tiff",
            include_bytes!("../fixtures/opaque.tiff"),
            ImageFormat::Tiff,
        ),
        (
            "ppm",
            include_bytes!("../fixtures/opaque.ppm"),
            ImageFormat::Pnm,
        ),
        (
            "qoi",
            include_bytes!("../fixtures/opaque.qoi"),
            ImageFormat::Qoi,
        ),
    ];

    #[test]
    fn every_format_is_recognized_and_decodes_exactly_when_enabled() {
        let expected = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
            [255, 255, 255, 255],
            [0, 0, 0, 255],
        ]
        .concat();
        for (name, bytes, format) in OPAQUE {
            assert_eq!(ImageFormat::guess(bytes), Some(format), "{name}");
            match images::decode(bytes) {
                Ok(image) => {
                    assert!(format.is_enabled(), "{name}");
                    assert_eq!((image.width(), image.height()), (3, 2), "{name}");
                    assert_eq!(image.pixels(), expected, "{name}");
                }
                Err(ImageError::FormatDisabled(disabled)) => {
                    assert!(!format.is_enabled(), "{name}");
                    assert_eq!(disabled, format);
                }
                Err(error) => panic!("{name}: {error}"),
            }
        }
        assert_eq!(
            ImageFormat::enabled().count(),
            ImageFormat::ALL.iter().filter(|f| f.is_enabled()).count()
        );
    }

    #[test]
    fn unknown_content_is_reported_as_such() {
        assert!(matches!(
            images::decode(b"not an image"),
            Err(ImageError::UnknownFormat)
        ));
        assert!(matches!(
            images::decode(&[]),
            Err(ImageError::UnknownFormat)
        ));
    }

    #[cfg(feature = "png")]
    #[test]
    fn png_alpha_is_premultiplied_in_linear_light() {
        let image = images::decode(RGBA_PNG).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        let pixels = image.pixels();
        assert_eq!(pixels[..4], [255, 0, 0, 255]);
        assert_eq!(
            pixels[12..16],
            [0, 0, 0, 0],
            "a transparent pixel loses its color"
        );
        // White at alpha 128 is linear 0.502, which encodes to 188.
        assert_eq!(pixels[16..20], [188, 188, 188, 128]);
    }

    #[cfg(feature = "png")]
    #[test]
    fn an_oversized_header_is_rejected_before_decoding() {
        let bytes = include_bytes!("../fixtures/huge-header.png");
        assert!(matches!(
            images::decode(bytes),
            Err(ImageError::TooLarge {
                width: 60000,
                height: 60000
            })
        ));
    }

    #[cfg(feature = "png")]
    #[test]
    fn damaged_data_and_missing_files_are_distinct_errors() {
        assert!(matches!(
            images::decode(&RGBA_PNG[..40]),
            Err(ImageError::Malformed(_))
        ));
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        assert_eq!(images::load(fixtures.join("rgba.png")).unwrap().width(), 3);
        let missing = fixtures.join("missing.png");
        match images::load(&missing) {
            Err(ImageError::Io { path, source }) => {
                assert_eq!(path, missing);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("expected an I/O error, got {other:?}"),
        }
        let error = images::load(&missing).unwrap_err();
        assert!(error.to_string().contains(&missing.display().to_string()));
        assert!(std::error::Error::source(&error).is_some());
    }

    #[cfg(feature = "pnm")]
    #[test]
    fn a_declared_zero_dimension_is_malformed_not_too_large() {
        for header in [&b"P5\n0 0\n255\n"[..], b"P5\n0 4\n255\n", b"P5\n4 0\n255\n"] {
            assert!(
                matches!(images::decode(header), Err(ImageError::Malformed(_))),
                "{:?}",
                std::str::from_utf8(header),
            );
        }
    }

    #[test]
    fn every_format_is_listed_once_with_its_own_feature() {
        let formats: std::collections::HashSet<_> = ImageFormat::ALL.iter().collect();
        assert_eq!(formats.len(), ImageFormat::ALL.len());
        let features: std::collections::HashSet<_> = ImageFormat::ALL
            .iter()
            .map(|format| format.feature())
            .collect();
        assert_eq!(features.len(), ImageFormat::ALL.len());
    }

    #[cfg(feature = "jpeg")]
    #[test]
    fn jpeg_decodes_and_applies_its_exif_orientation() {
        let gray = images::decode(include_bytes!("../fixtures/gray.jpg")).unwrap();
        assert_eq!((gray.width(), gray.height()), (8, 8));
        assert!(
            gray.pixels()
                .iter()
                .step_by(4)
                .all(|v| v.abs_diff(128) <= 3)
        );
        // Stored 4x2 with orientation 6, which is displayed rotated to 2x4.
        let rotated = images::decode(include_bytes!("../fixtures/orientation-6.jpg")).unwrap();
        assert_eq!((rotated.width(), rotated.height()), (2, 4));
    }

    #[cfg(not(feature = "jpeg"))]
    #[test]
    fn a_disabled_format_names_its_feature() {
        let error = images::decode(include_bytes!("../fixtures/gray.jpg")).unwrap_err();
        assert!(matches!(
            error,
            ImageError::FormatDisabled(ImageFormat::Jpeg)
        ));
        assert_eq!(
            error.to_string(),
            "decoding JPEG is disabled; enable the pert-ui `jpeg` feature"
        );
    }
}
