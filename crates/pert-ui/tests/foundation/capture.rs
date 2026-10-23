//! Screenshots: how a request is admitted, what a host is asked to render,
//! and how the frame is cropped to the target, with a scripted host in place
//! of a renderer.
use std::time::{Duration, Instant};

use pert_ui::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn ui(scale: f64) -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::from_physical(
        PhysicalExtent::new((200.0 * scale) as u32, (100.0 * scale) as u32),
        scale,
    )?)?;
    Ok(ui)
}

fn mount(ui: &mut Runtime) -> TestResult<WidgetId> {
    let root = ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new("title", Label::new("Title")),
            Element::new(
                "scroll",
                ScrollArea::new(Element::new(
                    "tall",
                    Column::new(
                        (0..40).map(|i| Element::new(format!("row-{i}"), Label::new("Row"))),
                    ),
                )),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(root)
}

/// A frame whose every pixel encodes its physical position, so a crop can be
/// checked against the region it claims.
fn frame(width: u32, height: u32) -> FrameImage {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            rgba.extend_from_slice(&[x as u8, y as u8, (x >> 8) as u8, 255]);
        }
    }
    FrameImage::new(width, height, rgba).expect("valid frame")
}

fn bounds(ui: &Runtime, key: &str) -> [f32; 4] {
    ui.query(&InspectionQuery::components(Selector::key(key)))
        .components
        .first()
        .and_then(|c| c.details.box_model.border_box)
        .unwrap_or_else(|| panic!("{key} has no bounds"))
}

#[test]
fn without_a_rendering_host_every_request_is_refused_at_once() -> TestResult {
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    assert!(!ui.captures_supported());
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    assert_eq!(pending.try_reply()?.value, Err(CaptureError::Unsupported));
    assert!(ui.pending_captures().is_empty());
    let service = ui.inspection_service(Capacity::new(2)?)?;
    let pending = service
        .client()
        .capture(CaptureRequest::new(CaptureTarget::Window(1)))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(pending.wait()?.value, Err(CaptureError::Unsupported));
    Ok(())
}

#[test]
fn a_host_renders_the_window_and_the_runtime_crops_to_the_target() -> TestResult {
    let mut ui = ui(2.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    ui.paint()?;
    assert!(!ui.needs_paint());
    let window = ui.inspect().windows[0].id;
    let title = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
        "title".into(),
    ])));
    let whole = ui
        .request_capture(CaptureRequest::new(CaptureTarget::Window(window)).include_overlay(true));
    assert_eq!(title.try_reply().map(|_| ()), Err(WaitError::Timeout));
    assert!(ui.needs_paint(), "an admitted capture asks for one frame");
    let jobs = ui.pending_captures();
    assert_eq!(jobs.len(), 2);
    assert_eq!(
        (jobs[0].window(), jobs[0].include_overlay()),
        (window, false)
    );
    assert!(jobs[1].include_overlay());

    let scene = ui.capture_scene(&jobs[0])?;
    assert_eq!(
        scene.viewport().physical(),
        Some(PhysicalExtent::new(400, 200))
    );
    assert!(ui.complete_capture(&jobs[0], Ok(frame(400, 200))));
    assert!(
        !ui.complete_capture(&jobs[0], Ok(frame(400, 200))),
        "completed once"
    );
    let capture = title.wait()?.value?;
    let [x, y, width, height] = bounds(&ui, "title");
    assert_eq!(capture.region, [x, y, width, height]);
    assert_eq!(
        (capture.scale, capture.clipped, capture.overlay),
        (2.0, false, false)
    );
    assert_eq!(
        capture.component,
        Some(
            ui.query(&InspectionQuery::components(Selector::key("title")))
                .components[0]
                .id
        )
    );
    let expected_width = ((x + width) * 2.0).ceil() as u32 - (x * 2.0).floor() as u32;
    assert_eq!(capture.image.width(), expected_width);
    // The first pixel of the crop is the frame pixel at the region's corner.
    let left = (x * 2.0).floor() as u32;
    let top = (y * 2.0).floor() as u32;
    assert_eq!(
        &capture.image.pixels()[..4],
        &[left as u8, top as u8, (left >> 8) as u8, 255]
    );
    let png = capture.image.encode_png();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");

    ui.complete_capture(&jobs[1], Ok(frame(400, 200)));
    let capture = whole.wait()?.value?;
    assert_eq!((capture.image.width(), capture.image.height()), (400, 200));
    assert_eq!(capture.region, [0.0, 0.0, 200.0, 100.0]);
    assert!(!capture.overlay && capture.component.is_none());
    assert!(!ui.needs_paint() && ui.pending_captures().is_empty());
    Ok(())
}

#[test]
fn a_clipped_target_reports_its_visible_part_and_a_hidden_one_is_refused() -> TestResult {
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    // The tall column overflows its scroll area: only the visible part is captured.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
        "scroll".into(),
        "tall".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    ui.complete_capture(&job, Ok(frame(200, 100)));
    let capture = pending.wait()?.value?;
    let [_, _, _, tall] = bounds(&ui, "tall");
    assert!(capture.clipped && capture.region[3] < tall, "{capture:?}");
    assert!(capture.image.height() <= 100);
    // A row scrolled out of view is hidden by its clip, an unmounted target
    // unknown, and a hidden component hidden.
    let out_of_view = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
        "scroll".into(),
        "tall".into(),
        "row-39".into(),
    ])));
    assert_eq!(out_of_view.try_reply()?.value, Err(CaptureError::Hidden));
    let unknown = ui.request_capture(CaptureRequest::new(CaptureTarget::Component(u64::MAX)));
    assert_eq!(unknown.try_reply()?.value, Err(CaptureError::UnknownTarget));
    let title = ui
        .query(&InspectionQuery::components(Selector::key("title")))
        .components[0]
        .id;
    let hidden = ui.request_capture(CaptureRequest::new(CaptureTarget::Window(u64::MAX)));
    assert_eq!(hidden.try_reply()?.value, Err(CaptureError::UnknownTarget));
    ui.set_window_visible(false)?;
    let invisible = ui.request_capture(CaptureRequest::new(CaptureTarget::Component(title)));
    assert_eq!(invisible.try_reply()?.value, Err(CaptureError::Hidden));
    Ok(())
}

#[test]
fn a_target_that_disappears_before_the_frame_is_reported_not_replaced() -> TestResult {
    let mut ui = ui(1.0)?;
    let root = mount(&mut ui)?;
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
        "title".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    let title = ui.children(&root)?[0].clone();
    ui.unmount(&title)?;
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert_eq!(pending.wait()?.value, Err(CaptureError::UnknownTarget));
    // A host that cannot render reports that, and the request is gone.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    ui.complete_capture(&job, Err(CaptureError::SurfaceUnavailable));
    assert_eq!(pending.wait()?.value, Err(CaptureError::SurfaceUnavailable));
    assert!(ui.pending_captures().is_empty());
    // A frame of the wrong size is a renderer mistake.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    ui.complete_capture(&job, Ok(frame(20, 10)));
    assert!(matches!(
        pending.wait()?.value,
        Err(CaptureError::Frame(FrameError::ExtentMismatch {
            frame_width: 20,
            frame_height: 10,
            ..
        }))
    ));
    // Shutdown cancels what a host never completed.
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    ui.shutdown();
    assert_eq!(pending.wait().map(|_| ()), Err(WaitError::Cancelled));
    Ok(())
}

#[test]
fn a_tool_thread_gets_its_screenshot_after_the_host_renders() -> TestResult {
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let client = service.client();
    let tool = std::thread::spawn(move || {
        client
            .capture(CaptureRequest::new(CaptureTarget::Path(vec![
                "main".into(),
                "title".into(),
            ])))
            .expect("accepted")
            .wait()
    });
    // The host: flush admits the request, then the redraw renders the job.
    // Bounded, so a tool thread that fails before its request is admitted
    // fails the test instead of hanging it.
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.pending_captures().is_empty() {
        if tool.is_finished() {
            let outcome = tool.join().map_err(|_| "the tool thread panicked")?;
            return Err(format!("the tool finished before admission: {outcome:?}").into());
        }
        if Instant::now() > deadline {
            return Err("no capture was admitted within 10 s".into());
        }
        ui.flush(Duration::ZERO)?;
        std::thread::sleep(Duration::from_millis(5));
    }
    let job = ui.pending_captures().remove(0);
    let _scene = ui.capture_scene(&job)?;
    ui.complete_capture(&job, Ok(frame(200, 100)));
    let reply = tool.join().expect("the tool finishes")?;
    let capture = reply.value?;
    assert_eq!(
        capture.target,
        CaptureTarget::Path(vec!["main".into(), "title".into()])
    );
    assert_eq!(reply.flush_error, None);
    Ok(())
}

#[cfg(feature = "png")]
#[test]
fn the_png_decodes_to_the_same_pixels() -> TestResult {
    let image = frame(37, 5);
    let decoded = pert_ui::images::decode(&image.encode_png())?;
    assert_eq!((decoded.width(), decoded.height()), (37, 5));
    // Opaque pixels premultiply to themselves.
    assert_eq!(decoded.pixels(), image.pixels());
    Ok(())
}

#[cfg(feature = "png")]
#[test]
fn a_translucent_png_decodes_to_the_same_premultiplied_pixels() -> TestResult {
    // Straight colors at several coverages, premultiplied as the renderer
    // stores them.
    let straight = vec![
        255, 255, 255, 128, 200, 40, 10, 64, 30, 90, 250, 1, 5, 6, 7, 0,
    ];
    let premultiplied = RasterImage::from_srgba8(4, 1, straight)?;
    let image = FrameImage::new(4, 1, premultiplied.pixels().to_vec())?;
    let decoded = pert_ui::images::decode(&image.encode_png())?;
    for (back, original) in decoded.pixels().iter().zip(image.pixels()) {
        assert!(back.abs_diff(*original) <= 1, "{back} vs {original}");
    }
    Ok(())
}

#[test]
fn a_replacement_at_the_same_path_is_not_the_admitted_target() -> TestResult {
    let mut ui = ui(1.0)?;
    let root = ui.mount_tree(Element::new(
        "main",
        Column::new([Element::new("title", Label::new("Title"))]),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.set_capture_support(true);
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
        "title".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    let title = ui.children(&root)?[0].clone();
    ui.unmount(&title)?;
    ui.reconcile_children(
        &root,
        vec![Element::new("title", Label::new("Replacement"))],
    )?;
    ui.flush(Duration::ZERO)?;
    assert!(bounds(&ui, "title")[2] > 0.0, "the replacement is laid out");
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert_eq!(pending.wait()?.value, Err(CaptureError::UnknownTarget));
    Ok(())
}

#[test]
fn admission_is_bounded_and_refuses_frames_over_the_limit() -> TestResult {
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    let mut pending = Vec::new();
    for _ in 0..Runtime::MAX_PENDING_CAPTURES {
        pending.push(
            ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
                "main".into(),
            ]))),
        );
    }
    let refused = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    assert_eq!(refused.try_reply()?.value, Err(CaptureError::Busy));
    let job = ui.pending_captures().remove(0);
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert!(pending.remove(0).wait()?.value.is_ok());
    let admitted = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    assert_eq!(
        admitted.try_reply().map(|_| ()),
        Err(WaitError::Timeout),
        "room again"
    );
    // A window too large for one image is refused at admission, before any
    // host renders it.
    let big = Runtime::new();
    big.set_viewport(Viewport::from_physical(
        PhysicalExtent::new(5000, 4000),
        1.0,
    )?)?;
    big.set_capture_support(true);
    let window = big.inspect().windows[0].id;
    let refused = big.request_capture(CaptureRequest::new(CaptureTarget::Window(window)));
    assert_eq!(
        refused.try_reply()?.value,
        Err(CaptureError::Limit(LimitError::FrameSize {
            width: 5000,
            height: 4000
        }))
    );
    Ok(())
}

#[test]
fn closing_the_service_cancels_an_admitted_capture_and_a_closed_runtime_admits_none() -> TestResult
{
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    let service = ui.inspection_service(Capacity::new(2)?)?;
    let pending = service
        .client()
        .capture(CaptureRequest::new(CaptureTarget::Path(vec![
            "main".into(),
        ])))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.pending_captures().len(), 1);
    drop(service);
    // The waiter is released by the close itself, before the host lists or
    // completes jobs again: a host that stops painting cannot strand it.
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Cancelled));
    assert!(
        ui.pending_captures().is_empty(),
        "a closed service's job is dropped"
    );
    assert_eq!(pending.wait().map(|_| ()), Err(WaitError::Cancelled));
    // Closed after the host already picked the job up: completion cancels.
    let service = ui.inspection_service(Capacity::new(2)?)?;
    let pending = service
        .client()
        .capture(CaptureRequest::new(CaptureTarget::Path(vec![
            "main".into(),
        ])))?;
    ui.flush(Duration::ZERO)?;
    let job = ui.pending_captures().remove(0);
    drop(service);
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Cancelled));
    assert!(ui.complete_capture(&job, Ok(frame(200, 100))));
    assert_eq!(pending.wait().map(|_| ()), Err(WaitError::Cancelled));
    ui.shutdown();
    let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    assert_eq!(pending.try_reply().map(|_| ()), Err(WaitError::Cancelled));
    Ok(())
}

#[test]
fn an_unmeasured_target_and_a_scene_that_cannot_paint_are_reported() -> TestResult {
    let mut ui = ui(1.0)?;
    ui.set_capture_support(true);
    let late = || Element::new("late", Label::new("Not laid out yet"));
    let root = ui.mount_tree(Element::new("root", Column::new([late()])))?;
    let target = || CaptureTarget::Path(vec!["root".into(), "late".into()]);
    let pending = ui.request_capture(CaptureRequest::new(target()));
    assert_eq!(pending.try_reply()?.value, Err(CaptureError::Unmeasured));
    ui.flush(Duration::ZERO)?;
    let pending = ui.request_capture(CaptureRequest::new(target()));
    let job = ui.pending_captures().remove(0);
    // A scene cannot be composed while updates are pending; the error keeps
    // its typed cause instead of turning into a generic one. A sibling added below
    // the target leaves updates pending.
    ui.reconcile_children(
        &root,
        vec![late(), Element::new("more", Label::new("pending"))],
    )?;
    let error = ui.capture_scene(&job).expect_err("pending updates");
    assert_eq!(error, CaptureError::Drawing(DrawError::PendingUpdates));
    ui.flush(Duration::ZERO)?;
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");
    assert!(ui.capture_scene(&job).is_ok());
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert!(pending.wait()?.value.is_ok());
    assert_eq!(
        ui.capture_scene(&job).err(),
        Some(CaptureError::UnknownTarget),
        "a completed job is stale"
    );
    Ok(())
}

#[test]
fn the_overlay_flag_reports_what_was_drawn() -> TestResult {
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    let asked = ui.request_capture(
        CaptureRequest::new(CaptureTarget::Path(vec!["main".into()])).include_overlay(true),
    );
    let job = ui.pending_captures().remove(0);
    assert!(job.include_overlay());
    ui.capture_scene(&job)?;
    ui.complete_capture(&job, Ok(frame(200, 100)));
    // Nothing was drawn: no overlay is active.
    assert!(!asked.wait()?.value?.overlay);
    ui.set_debug_overlay(DebugOverlay::Boxes);
    let asked = ui.request_capture(
        CaptureRequest::new(CaptureTarget::Path(vec!["main".into()])).include_overlay(true),
    );
    let job = ui.pending_captures().remove(0);
    ui.capture_scene(&job)?;
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert_eq!(asked.wait()?.value?.overlay, cfg!(debug_assertions));
    let not_asked = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
        "main".into(),
    ])));
    let job = ui.pending_captures().remove(0);
    ui.capture_scene(&job)?;
    ui.complete_capture(&job, Ok(frame(200, 100)));
    assert!(!not_asked.wait()?.value?.overlay);
    Ok(())
}

#[test]
fn capture_keeps_the_admission_failure_after_a_later_successful_flush() -> TestResult {
    struct Broken(bool);
    impl Component for Broken {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            if std::mem::replace(&mut self.0, false) {
                Err(RuntimeError::BorrowConflict)
            } else {
                Ok(())
            }
        }
    }
    let mut ui = ui(1.0)?;
    mount(&mut ui)?;
    ui.set_capture_support(true);
    ui.mount("broken", Broken(true))?;
    let service = ui.inspection_service(Capacity::new(4)?)?;
    let pending = service
        .client()
        .capture(CaptureRequest::new(CaptureTarget::Path(vec![
            "main".into(),
            "title".into(),
        ])))?;
    assert_eq!(ui.flush(Duration::ZERO), Err(RuntimeError::BorrowConflict));
    let job = ui.pending_captures().remove(0);
    ui.flush(Duration::ZERO)?;
    assert!(ui.capture_scene(&job).is_ok());
    assert!(ui.complete_capture(&job, Ok(frame(200, 100))));
    let reply = std::thread::spawn(move || pending.wait())
        .join()
        .expect("worker finishes")?;
    assert!(reply.value.is_ok());
    assert_eq!(reply.flush_error, Some(RuntimeError::BorrowConflict));
    Ok(())
}

#[test]
fn capture_causes_normalize_runtime_errors_and_remain_owned() {
    fn owned<T: Send + Sync + Clone>() {}
    owned::<RuntimeError>();
    owned::<DrawError>();
    owned::<CaptureError>();
    owned::<InspectionReply<Result<Capture, CaptureError>>>();
    let runtime = RuntimeError::Geometry(GeometryError::Negative);
    assert_eq!(
        CaptureError::from(DrawError::Runtime(runtime.clone())),
        CaptureError::Runtime(runtime)
    );
    assert_eq!(
        CaptureError::from(DrawError::InvalidColor),
        CaptureError::Drawing(DrawError::InvalidColor)
    );
}

#[cfg(feature = "serde")]
#[test]
fn capture_serde_preserves_portable_outcomes_and_rejects_rust_causes() -> TestResult {
    for (error, wire) in [
        (CaptureError::Unsupported, "\"Unsupported\""),
        (
            CaptureError::Backend("lost".into()),
            "{\"Backend\":\"lost\"}",
        ),
        (
            CaptureError::Limit(LimitError::FrameSize {
                width: 5000,
                height: 4000,
            }),
            "{\"Limit\":{\"FrameSize\":{\"width\":5000,\"height\":4000}}}",
        ),
    ] {
        assert_eq!(serde_json::to_string(&error)?, wire);
        assert_eq!(serde_json::from_str::<CaptureError>(wire)?, error);
    }
    for error in [
        CaptureError::Runtime(RuntimeError::BorrowConflict),
        CaptureError::Drawing(DrawError::PendingUpdates),
        CaptureError::Runtime(ApplicationError::from("offline").into()),
    ] {
        assert!(serde_json::to_string(&error).is_err());
    }
    for wire in [
        "{\"Runtime\":\"BorrowConflict\"}",
        "{\"Drawing\":\"PendingUpdates\"}",
    ] {
        assert!(serde_json::from_str::<CaptureError>(wire).is_err());
    }
    Ok(())
}

#[test]
fn capture_forwards_owned_paint_causes_to_the_client() -> TestResult {
    struct FailingPaint(DrawError);
    impl Component for FailingPaint {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::from([20, 20]))
        }
        fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
            Err(self.0.clone())
        }
    }
    for cause in [
        DrawError::InvalidColor,
        DrawError::Runtime(RuntimeError::Geometry(GeometryError::Negative)),
        DrawError::Runtime(ApplicationError::from("host paint failed").into()),
    ] {
        let mut ui = ui(1.0)?;
        ui.mount_tree(Element::new("paint", FailingPaint(cause.clone())))?;
        ui.flush(Duration::ZERO)?;
        ui.set_capture_support(true);
        let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Path(vec![
            "paint".into(),
        ])));
        let job = ui.pending_captures().remove(0);
        let error = ui.capture_scene(&job).expect_err("paint refused");
        assert_eq!(error, CaptureError::from(cause));
        assert!(ui.complete_capture(&job, Err(error.clone())));
        let reply = pending.wait()?;
        assert_eq!(reply.value, Err(error));
        assert_eq!(reply.flush_error, None);
    }
    Ok(())
}

#[test]
fn a_job_listed_by_another_runtime_neither_renders_nor_completes() -> TestResult {
    // With the same overlay flag, the two jobs look identical: the same id,
    // window and flag, since every runtime numbers its own requests.
    for overlay in [false, true] {
        let mut listing = ui(1.0)?;
        mount(&mut listing)?;
        listing.set_capture_support(true);
        let mut other = ui(1.0)?;
        mount(&mut other)?;
        other.set_capture_support(true);
        let main = || CaptureTarget::Path(vec!["main".into()]);
        let _plain = listing.request_capture(CaptureRequest::new(main()));
        let asked = other.request_capture(CaptureRequest::new(main()).include_overlay(overlay));
        let foreign = listing.pending_captures().remove(0);
        let own = other.pending_captures().remove(0);
        assert_eq!(
            (foreign.id(), foreign.window(), foreign.include_overlay()),
            (own.id(), own.window(), false),
            "the public observations agree but for the flag asked"
        );
        assert_eq!(own.include_overlay(), overlay);
        assert_ne!(foreign, own, "different requests");
        assert_eq!(own, other.pending_captures().remove(0), "the same request");
        assert_eq!(
            other.capture_scene(&foreign).err(),
            Some(CaptureError::UnknownTarget),
            "the other runtime's request decides nothing here"
        );
        assert!(
            !other.complete_capture(&foreign, Ok(frame(200, 100))),
            "nor can it complete the local request"
        );
        assert!(matches!(asked.try_reply(), Err(WaitError::Timeout)));
        assert_eq!(other.pending_captures(), std::slice::from_ref(&own));
        other.capture_scene(&own)?;
        assert!(other.complete_capture(&own, Ok(frame(200, 100))));
        assert!(asked.wait()?.value.is_ok());
    }
    Ok(())
}
