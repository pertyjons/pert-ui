//! The `serde` feature through the public facade: inspection snapshots,
//! profiles and workspace documents round-trip through `serde_json`, non-finite
//! diagnostic numbers stay explicit, and a JSON workspace is validated exactly
//! as the text encoding is.
#![cfg(feature = "serde")]

use std::{collections::BTreeMap, time::Duration};

use pert_ui::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn ui() -> TestResult<Runtime> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(600.0, 400.0, 1.0)?)?;
    Ok(ui)
}

fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let text = serde_json::to_string(value)?;
    let back: T = serde_json::from_str(&text)?;
    assert_eq!(&back, value, "{text}");
    Ok(back)
}

fn mount_workspace(ui: &mut Runtime) -> TestResult {
    ui.mount_tree(Element::new(
        "main",
        Column::new([
            Element::new(
                "tabs",
                Tabs::new([
                    Tab::new("overview", "Overview", Element::new("a", Label::new("A"))),
                    Tab::new("scope", "Scope", Element::new("b", Label::new("B"))),
                ]),
            ),
            Element::new(
                "split",
                SplitPane::new(
                    SplitAxis::LeftRight,
                    Element::new("left", Label::new("Left")),
                    Element::new("right", Label::new("Right")),
                ),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

#[test]
fn a_snapshot_with_components_parameters_and_history_round_trips() -> TestResult {
    let mut ui = ui()?;
    ui.set_debug_mode(DebugMode::Inspect);
    let (bypass, _host) = ui.parameter(
        ParameterId::new(1, 7, "bypass")?,
        ParameterSpec::boolean("Bypass", false)?,
    )?;
    let (cutoff, _cutoff_host) = ui.parameter(
        ParameterId::new(1, 7, "cutoff")?,
        ParameterSpec::new("Cutoff", 20.0, 20_000.0, 1_000.0)?
            .with_mapping(ParameterMapping::Logarithmic)?,
    )?;
    let mode = ui.signal("b");
    ui.mount_tree(Element::new(
        "column",
        Column::new([
            Element::new(
                "cutoff",
                Field::new(Element::new("slider", Slider::new(cutoff))).label("Cutoff"),
            ),
            Element::new("play", Button::new("Play", |_| Ok(()))),
            Element::new("bypass", Toggle::for_parameter(bypass)),
            Element::new(
                "mode",
                RadioGroup::new(
                    mode,
                    [Choice::new("a", "First"), Choice::new("b", "Second")],
                )?,
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.key_input(KeyCode::Tab, KeyAction::Press)?;
    ui.key_input(KeyCode::Tab, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;

    let snapshot = ui.inspect_with(|| {
        BTreeMap::from([
            ("document".to_owned(), InspectValue::Text("untitled".into())),
            (
                "levels".to_owned(),
                InspectValue::List(vec![
                    InspectValue::Number(-6.0),
                    InspectValue::Number(f64::NEG_INFINITY),
                ]),
            ),
        ])
    });
    assert!(!snapshot.parameters.is_empty() && !snapshot.events.is_empty());
    let text = serde_json::to_string(&snapshot)?;
    let back: InspectionSnapshot = serde_json::from_str(&text)?;
    assert_eq!(back, snapshot);
    let value: serde_json::Value = serde_json::from_str(&text)?;
    assert_eq!(
        value["schema_version"],
        serde_json::json!(InspectionSnapshot::SCHEMA_VERSION)
    );
    assert_eq!(
        value["application"]["levels"]["List"][1],
        serde_json::json!({ "Number": "-inf" })
    );
    Ok(())
}

#[test]
fn a_profile_round_trips() -> TestResult {
    let mut ui = ui()?;
    ui.mount_tree(Element::new("label", Label::new("Timed")))?;
    ui.set_profiling(true)?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let profile = ui.profile();
    assert!(!profile.stats.is_empty());
    round_trip(&profile)?;
    Ok(())
}

#[test]
fn non_finite_diagnostic_numbers_are_explicit_and_survive() -> TestResult {
    let value = InspectValue::Object(BTreeMap::from([
        ("nan".to_owned(), InspectValue::Number(f64::NAN)),
        ("inf".to_owned(), InspectValue::Number(f64::INFINITY)),
        ("finite".to_owned(), InspectValue::Number(0.5)),
    ]));
    let text = serde_json::to_string(&value)?;
    assert_eq!(
        text,
        r#"{"Object":{"finite":{"Number":0.5},"inf":{"Number":"inf"},"nan":{"Number":"NaN"}}}"#
    );
    let back: InspectValue = serde_json::from_str(&text)?;
    let InspectValue::Object(fields) = &back else {
        panic!("{back:?}");
    };
    assert!(matches!(fields["nan"], InspectValue::Number(n) if n.is_nan()));
    assert!(matches!(fields["inf"], InspectValue::Number(n) if n == f64::INFINITY));
    assert_eq!(serde_json::to_string(&back)?, text);

    let boxes = BoxModelInspection {
        border_box: Some([0.0, 0.0, f32::NAN, 10.0]),
        margin: None,
        padding: Some([1.0, 1.0, 1.0, 1.0]),
        foreground: None,
        background: None,
        border: Some(BorderInspection {
            width: f32::INFINITY,
            color: [0.0, 0.0, 0.0, 1.0],
        }),
    };
    let text = serde_json::to_string(&boxes)?;
    assert!(
        text.contains(r#""border_box":[0.0,0.0,"NaN",10.0]"#),
        "{text}"
    );
    assert!(text.contains(r#""width":"inf""#), "{text}");
    let back: BoxModelInspection = serde_json::from_str(&text)?;
    assert!(back.border_box.unwrap()[2].is_nan());
    assert_eq!(back.border.unwrap().width, f32::INFINITY);
    // A word that is not one of the three spellings is an error, not a number.
    assert!(serde_json::from_str::<InspectValue>(r#"{"Number":"infinity"}"#).is_err());
    Ok(())
}

#[test]
fn a_workspace_round_trips_and_agrees_with_the_text_encoding() -> TestResult {
    let mut ui = ui()?;
    ui.ui_settings().set_zoom(UiZoom::new(1.25)?)?;
    mount_workspace(&mut ui)?;
    let workspace = ui.capture_workspace()?;
    assert!(workspace.len() >= 2);
    let back = round_trip(&workspace)?;
    assert_eq!(Workspace::decode(&workspace.encode())?, back);
    assert_eq!(back.encode(), workspace.encode());
    let value: serde_json::Value = serde_json::to_value(&workspace)?;
    assert_eq!(
        value["schema_version"],
        serde_json::json!(Workspace::SCHEMA_VERSION)
    );
    assert_eq!(value["settings"]["zoom"], serde_json::json!(1.25));
    assert_eq!(
        value["entries"][0]["path"],
        serde_json::json!(["main", "split"])
    );
    Ok(())
}

#[test]
fn a_json_workspace_is_validated_like_the_text_one() -> TestResult {
    let document = |entries: &str, settings: &str, version: u32| {
        format!(r#"{{"schema_version":{version},"settings":{{{settings}}},"entries":[{entries}]}}"#)
    };
    let entry = |path: &str, kind: &str, values: &str| {
        format!(r#"{{"path":{path},"state":{{"kind":"{kind}","values":{{{values}}}}}}}"#)
    };
    let accepted = document(
        &entry(r#"["main","split"]"#, "split", r#""ratio":{"Number":0.25}"#),
        "",
        2,
    );
    let workspace: Workspace = serde_json::from_str(&accepted)?;
    assert_eq!(
        workspace.encode(),
        "pert-ui-workspace 2\nentry \"main/split\" split ratio=0.25\n"
    );

    let rejected: Vec<(&str, String)> = vec![
        (
            "newer schema",
            document(&entry(r#"["a"]"#, "split", ""), "", 3),
        ),
        (
            "duplicate path",
            document(
                &format!(
                    "{},{}",
                    entry(r#"["a"]"#, "split", ""),
                    entry(r#"["a"]"#, "tabs", "")
                ),
                "",
                2,
            ),
        ),
        (
            "empty segment",
            document(&entry(r#"["a",""]"#, "split", ""), "", 2),
        ),
        ("empty path", document(&entry("[]", "split", ""), "", 2)),
        (
            "control character",
            document(&entry(r#"["a\u0007"]"#, "split", ""), "", 2),
        ),
        (
            "invalid kind",
            document(&entry(r#"["a"]"#, "bad kind", ""), "", 2),
        ),
        (
            "invalid name",
            document(
                &entry(r#"["a"]"#, "split", r#""bad name":{"Flag":true}"#),
                "",
                2,
            ),
        ),
        (
            "nested map",
            document(
                &entry(r#"["a"]"#, "table", r#""w":{"Map":{"x":{"Map":{}}}}"#),
                "",
                2,
            ),
        ),
        (
            "non-finite number",
            document(
                &entry(r#"["a"]"#, "split", r#""ratio":{"Number":"NaN"}"#),
                "",
                2,
            ),
        ),
        (
            "text too long",
            document(
                &entry(
                    r#"["a"]"#,
                    "tabs",
                    &format!(r#""selected":{{"Text":"{}"}}"#, "x".repeat(257)),
                ),
                "",
                2,
            ),
        ),
        (
            "path too long",
            document(
                // Each segment fits the text limit; together they exceed
                // the path limit.
                &entry(
                    &format!(r#"["{0}","{0}","{0}"]"#, "p".repeat(200)),
                    "split",
                    "",
                ),
                "",
                2,
            ),
        ),
        ("zoom out of range", document("", r#""zoom":3.0"#, 2)),
        ("unknown setting", document("", r#""motion":"Full""#, 2)),
        (
            "unknown entry field",
            document(
                r#"{"path":["a"],"state":{"kind":"split","values":{}},"extra":1}"#,
                "",
                2,
            ),
        ),
    ];
    for (case, text) in rejected {
        assert!(
            serde_json::from_str::<serde_json::Value>(&text).is_ok(),
            "{case}: the input must be valid JSON"
        );
        // The reason matters: a case rejected for another one proves nothing.
        let expected = match case {
            "newer schema" => "schema version 3 is not supported",
            "duplicate path" => "two workspace entries address `a`",
            "empty segment" | "empty path" => "need at least one nonempty segment",
            "control character" => "contains a control character",
            "invalid kind" => "`bad kind` is not a valid workspace name",
            "invalid name" => "`bad name` is not a valid workspace name",
            "nested map" => "maps hold scalars only",
            // JSON has no non-finite number; the text is refused by type.
            "non-finite number" => "expected f64",
            "text too long" => "text is longer than 256 bytes",
            "path too long" => "path is longer than 512 bytes",
            "zoom out of range" => "zoom factor is outside 0.75..=2",
            "unknown setting" => "unknown field `motion`",
            "unknown entry field" => "unknown field `extra`",
            other => panic!("no expected reason for {other}"),
        };
        match serde_json::from_str::<Workspace>(&text) {
            Ok(_) => panic!("{case} was accepted: {text}"),
            Err(error) => assert!(
                error.to_string().contains(expected),
                "{case} was rejected for another reason: {error}"
            ),
        }
    }
    let many: Vec<String> = (0..=Workspace::MAX_ENTRIES)
        .map(|i| entry(&format!(r#"["item-{i}"]"#), "split", ""))
        .collect();
    let error = serde_json::from_str::<Workspace>(&document(&many.join(","), "", 2))
        .err()
        .map(|error| error.to_string());
    assert!(
        error
            .as_deref()
            .is_some_and(|error| error.contains("more than 4096 entries")),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn repeated_keys_in_a_json_workspace_are_rejected_like_the_text_ones() {
    // Built as text on purpose: a `serde_json::Value` would already have
    // collapsed the duplicates before the document saw them.
    let values = r#"{"schema_version":2,"settings":{},"entries":[{"path":["t"],"state":{"kind":"tabs","values":{"selected":{"Text":"a"},"selected":{"Text":"b"}}}}]}"#;
    let map = r#"{"schema_version":2,"settings":{},"entries":[{"path":["t"],"state":{"kind":"table","values":{"widths":{"Map":{"a":{"Number":1.0},"a":{"Number":2.0}}}}}}]}"#;
    let settings = r#"{"schema_version":2,"settings":{"zoom":1.0,"zoom":1.5},"entries":[]}"#;
    // The reason matters: a document refused for anything but the repeated key
    // proves nothing about duplicate detection.
    for (case, text, expected) in [
        ("value name", values, r#"duplicate key "selected""#),
        ("map key", map, r#"duplicate key "a""#),
        ("setting", settings, "duplicate field `zoom`"),
    ] {
        match serde_json::from_str::<Workspace>(text) {
            Ok(_) => panic!("a repeated {case} was accepted: {text}"),
            Err(error) => assert!(
                error.to_string().contains(expected),
                "a repeated {case} was rejected for another reason: {error}"
            ),
        }
    }
    let distinct = r#"{"schema_version":2,"settings":{},"entries":[{"path":["t"],"state":{"kind":"table","values":{"widths":{"Map":{"a":{"Number":1.0},"b":{"Number":2.0}}}}}}]}"#;
    assert!(serde_json::from_str::<Workspace>(distinct).is_ok());
}

#[test]
fn a_viewport_keeps_its_physical_extent_and_refuses_an_impossible_one() -> TestResult {
    let physical = Viewport::from_physical(PhysicalExtent::new(1001, 601), 1.5)?;
    assert_eq!(
        round_trip(&physical)?.physical(),
        Some(PhysicalExtent::new(1001, 601))
    );
    round_trip(&Viewport::from_physical(PhysicalExtent::new(0, 0), 2.0)?)?;
    round_trip(&Viewport::new(300.0, 200.0, 1.0)?)?;
    for text in [
        r#"{"width":100.0,"height":100.0,"scale":1.0,"physical":{"width":0,"height":0}}"#,
        r#"{"width":0.0,"height":100.0,"scale":1.0,"physical":{"width":10,"height":100}}"#,
        r#"{"width":100.0,"height":100.0,"scale":0.0,"physical":null}"#,
        r#"{"width":-1.0,"height":100.0,"scale":1.0,"physical":null}"#,
    ] {
        assert!(serde_json::from_str::<Viewport>(text).is_err(), "{text}");
    }
    Ok(())
}

#[test]
fn a_finite_number_beyond_f32_is_rejected_rather_than_becoming_infinity() {
    let text = r#"{"width":1e100,"color":[0.0,0.0,0.0,1.0]}"#;
    assert!(serde_json::from_str::<BorderInspection>(text).is_err());
    let text = r#"{"width":"inf","color":[0.0,0.0,0.0,1.0]}"#;
    assert_eq!(
        serde_json::from_str::<BorderInspection>(text)
            .unwrap()
            .width,
        f32::INFINITY
    );
    let text = r#"{"width":1.0,"color":[0.0,0.0,1e40,1.0]}"#;
    assert!(serde_json::from_str::<BorderInspection>(text).is_err());
}

#[test]
fn an_omitted_optional_field_reads_as_absent_like_null() -> TestResult {
    let omitted: BoxModelInspection = serde_json::from_str("{}")?;
    assert_eq!(omitted, BoxModelInspection::default());
    let null: BoxModelInspection = serde_json::from_str(
        r#"{"border_box":null,"margin":null,"padding":null,"foreground":null,"background":null,"border":null}"#,
    )?;
    assert_eq!(null, BoxModelInspection::default());
    let value: SemanticValue = serde_json::from_str(r#"{"Number":{"value":"NaN"}}"#)?;
    assert!(matches!(
        value,
        SemanticValue::Number { value, minimum: None, maximum: None } if value.is_nan()
    ));
    Ok(())
}

#[test]
fn struct_variants_refuse_unknown_fields_too() {
    assert!(
        serde_json::from_str::<SemanticValue>(
            r#"{"Choice":{"selected":0,"count":1,"label":"A","extra":true}}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<EventKind>(r#"{"TextInput":{"bytes":3,"more":1}}"#).is_err());
    assert!(
        serde_json::from_str::<Freshness>(r#"{"Stale":{"age":{"secs":1,"nanos":0},"x":1}}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<EventKind>(r#"{"TextInput":{"bytes":3}}"#).is_ok());
}

#[test]
fn drop_operation_sets_are_lists_of_operations() -> TestResult {
    for set in [
        DropOperations::NONE,
        DropOperations::MOVE,
        DropOperations::COPY,
        DropOperations::LINK,
        DropOperations::MOVE | DropOperations::LINK,
        DropOperations::ALL,
    ] {
        round_trip(&set)?;
    }
    assert_eq!(
        serde_json::to_string(&DropOperations::ALL)?,
        r#"["Move","Copy","Link"]"#
    );
    assert_eq!(
        serde_json::from_str::<DropOperations>(r#"["Link","Link","Move"]"#)?,
        DropOperations::MOVE | DropOperations::LINK
    );
    assert!(serde_json::from_str::<DropOperations>(r#"["Teleport"]"#).is_err());
    Ok(())
}

#[test]
fn the_unbounded_collection_code_round_trips() -> TestResult {
    // Added with inspection schema 14.
    assert!(Runtime::new().inspect().schema_version >= 14);
    // Only the runtime builds a report, so a tool reads it as JSON like this.
    let report: ValidationReport = serde_json::from_str(
        r#"{
            "logging_error": null,
            "enabled": true,
            "pending": false,
            "issues": [{
                "code": "UnboundedCollection",
                "components": [7],
                "message": "given its whole content extent"
            }],
            "markers": [{
                "component": 7,
                "bounds": [0.0, 0.0, 300.0, 500.0],
                "color": [1.0, 0.0, 0.0, 1.0],
                "codes": ["UnboundedCollection"]
            }]
        }"#,
    )?;
    assert_eq!(report.issues[0].code, ValidationCode::UnboundedCollection);
    assert_eq!(report.issues[0].components, [7]);
    assert_eq!(
        report.markers[0].codes,
        [ValidationCode::UnboundedCollection]
    );
    assert_eq!(report.markers[0].bounds, Some([0.0, 0.0, 300.0, 500.0]));
    let text = serde_json::to_string(&report)?;
    assert!(text.contains(r#""code":"UnboundedCollection""#), "{text}");
    round_trip(&report)?;
    Ok(())
}

#[test]
fn validation_reports_and_semantic_bounds_carry_non_finite_numbers() -> TestResult {
    let report: ValidationReport = serde_json::from_str(
        r#"{
            "logging_error": null,
            "enabled": true,
            "pending": false,
            "issues": [{
                "code": "InvalidBounds",
                "components": [3],
                "message": "bounds contain NaN"
            }],
            "markers": [{
                "component": 3,
                "bounds": [0.0, 0.0, "NaN", 4.0],
                "color": [1.0, 0.0, 0.0, 1.0],
                "codes": ["InvalidBounds"]
            }]
        }"#,
    )?;
    assert!(report.markers[0].bounds.unwrap()[2].is_nan());
    let text = serde_json::to_string(&report)?;
    assert!(text.contains(r#"[0.0,0.0,"NaN",4.0]"#), "{text}");
    let back: ValidationReport = serde_json::from_str(&text)?;
    assert!(back.markers[0].bounds.unwrap()[2].is_nan());
    assert_eq!(back.issues, report.issues);

    let value = SemanticValue::Number {
        value: 1.0,
        minimum: Some(f64::NEG_INFINITY),
        maximum: None,
    };
    let text = serde_json::to_string(&value)?;
    assert_eq!(
        text,
        r#"{"Number":{"value":1.0,"minimum":"-inf","maximum":null}}"#
    );
    assert_eq!(serde_json::from_str::<SemanticValue>(&text)?, value);
    Ok(())
}

#[test]
fn queries_and_their_results_round_trip() -> TestResult {
    let mut ui = ui()?;
    ui.set_debug_mode(DebugMode::Inspect);
    mount_workspace(&mut ui)?;
    let query = InspectionQuery::components(Selector::within(
        Selector::path(["main"]),
        Selector::semantic(SemanticRole::TabList, None::<&str>),
    ))
    .depth(Depth::Children)
    .detail(Detail::Semantics)
    .limit(8)
    .events_after(0)
    .windows()
    .validation()
    .application();
    let query = round_trip(&query)?;
    let result = ui.query(&query);
    assert!(!result.components.is_empty());
    let back = round_trip(&result)?;
    assert!(back.events.is_some() && back.windows.is_some() && back.sources.is_none());
    let value: serde_json::Value = serde_json::to_value(&result)?;
    assert_eq!(value["sources"], serde_json::Value::Null);
    assert_eq!(
        value["components"][0]["path"],
        serde_json::json!(["main", "tabs"])
    );
    round_trip(&DiagnosticCommand::SetOverlay(DebugOverlay::Boxes))?;
    assert!(serde_json::from_str::<Selector>(r#"{"Teleport":1}"#).is_err());
    Ok(())
}

#[test]
fn typed_capture_targets_serialize_as_their_variants() -> TestResult {
    let ui = Runtime::new();
    let primary = ui.primary_window();
    let window = CaptureTarget::window(&primary);
    assert_eq!(window, CaptureTarget::Window(primary.id()));
    assert_eq!(
        serde_json::to_value(&window)?,
        serde_json::json!({ "Window": primary.id() })
    );
    assert_eq!(round_trip(&window)?, window);
    let region = CaptureTarget::region(&primary, Rect::new(1.0, 2.0, 30.0, 40.0)?);
    assert_eq!(
        serde_json::to_value(&region)?,
        serde_json::json!({ "Region": {
            "window": primary.id(), "x": 1.0, "y": 2.0, "width": 30.0, "height": 40.0
        } })
    );
    assert_eq!(round_trip(&region)?, region);
    let request = round_trip(&CaptureRequest::new(window.clone()))?;
    assert_eq!(request.target(), &window);
    Ok(())
}

#[test]
fn capture_types_round_trip() -> TestResult {
    let request =
        CaptureRequest::new(CaptureTarget::Path(vec!["main".into()])).include_overlay(true);
    assert!(round_trip(&request)?.overlay_included());
    // A region's numbers are diagnostic floats: a non-finite one is text that
    // reads back, for the runtime to refuse.
    let region = CaptureTarget::Region {
        window: 1,
        x: 2.5,
        y: f32::NAN,
        width: 10.0,
        height: f32::INFINITY,
    };
    let text = serde_json::to_value(&region)?;
    assert_eq!(text["Region"]["y"], "NaN");
    assert_eq!(text["Region"]["height"], "inf");
    let CaptureTarget::Region { x, y, height, .. } = serde_json::from_value::<CaptureTarget>(text)?
    else {
        panic!("a region reads back as a region");
    };
    assert_eq!(x, 2.5);
    assert!(y.is_nan() && height == f32::INFINITY);
    let image = FrameImage::new(2, 1, vec![1, 2, 3, 255, 4, 5, 6, 255])?;
    let capture = Capture {
        target: CaptureTarget::Component(7),
        window: 1,
        component: Some(7),
        region: [0.5, 1.0, 2.0, 1.0],
        clipped: false,
        scale: 1.5,
        at: Duration::from_millis(3),
        overlay: false,
        image,
    };
    let back = round_trip(&capture)?;
    assert_eq!(back.image.encode_png(), capture.image.encode_png());
    // The region and scale follow the diagnostic float rules: an `f32`
    // overflow is rejected instead of reading as infinity, and a non-finite
    // value is explicit text that reads back.
    let mut text = serde_json::to_value(&capture)?;
    text["region"] = serde_json::json!([1e39, 0.0, 1.0, 1.0]);
    assert!(serde_json::from_value::<Capture>(text.clone()).is_err());
    let odd = Capture {
        region: [f32::INFINITY, 0.0, 1.0, 1.0],
        scale: f64::NAN,
        ..capture.clone()
    };
    let text = serde_json::to_value(&odd)?;
    assert_eq!(text["region"][0], "inf");
    assert_eq!(text["scale"], "NaN");
    let back: Capture = serde_json::from_value(text)?;
    assert_eq!(back.region[0], f32::INFINITY);
    assert!(back.scale.is_nan());
    assert!(
        serde_json::from_str::<FrameImage>(r#"{"width":2,"height":1,"rgba":[0,0,0]}"#).is_err()
    );
    assert!(serde_json::from_str::<FrameImage>(r#"{"width":0,"height":1,"rgba":[]}"#).is_err());
    round_trip(&CaptureError::Backend("gpu".into()))?;
    round_trip(&CaptureError::Frame(FrameError::ExtentMismatch {
        frame_width: 20,
        frame_height: 10,
        window_width: 40,
        window_height: 30,
    }))?;
    round_trip(&CaptureError::Frame(FrameError::PixelCount {
        width: 2,
        height: 1,
        expected: 8,
        actual: 3,
    }))?;
    Ok(())
}

#[test]
fn a_warning_description_round_trips_and_is_a_required_field() -> TestResult {
    // Added with inspection schema 15.
    assert!(Runtime::new().inspect().schema_version >= 15);
    let warned = Semantics::new(SemanticRole::Field)
        .description("Looks unusual")
        .warning(true);
    let text = serde_json::to_string(&warned)?;
    assert!(text.contains(r#""warning":true"#), "{text}");
    assert_eq!(round_trip(&warned)?, warned);
    // The schema is closed: semantics written before the field existed, or
    // with an unknown field, do not decode as this revision.
    let older = text.replace(r#","warning":true"#, "");
    assert!(!older.contains("warning"), "{older}");
    assert!(serde_json::from_str::<Semantics>(&older).is_err());
    let unknown = text.replace(r#""warning":true"#, r#""warning":true,"caution":true"#);
    assert!(serde_json::from_str::<Semantics>(&unknown).is_err());
    Ok(())
}

/// New-behavior test. Named assertion: a revision-1 JSON workspace, valid
/// otherwise, fails to deserialize with `UnsupportedVersion`'s message, so no
/// `Workspace` holding paths under the old `Each` addressing is built.
#[test]
fn a_revision_one_json_workspace_is_rejected() {
    let document = r#"{"schema_version":1,"settings":{},"entries":[{"path":["list","items","7","scroll"],"state":{"kind":"scroll","values":{"x":{"Number":0.0},"y":{"Number":10.0}}}}]}"#;
    let error = serde_json::from_str::<Workspace>(document)
        .expect_err("revision 1 is not supported")
        .to_string();
    assert!(
        error.starts_with("workspace schema version 1 is not supported (this build writes 2)"),
        "{error}"
    );
}

/// A 200 × 40 leaf reporting two items, the first with a rectangle.
struct TwoRows;
impl Component for TwoRows {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(200.0, 40.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        let first =
            SemanticItem::new("first").local_bounds(Rect::new(0.0, 0.0, 100.0, 20.0).unwrap());
        ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Group).items([first, SemanticItem::new("second")]),
            ),
            ..ComponentDetails::default()
        }
    }
}

/// `text` with its leading `schema_version` replaced by `version`.
fn with_version(text: &str, version: u32) -> String {
    let current = format!(
        r#"{{"schema_version":{}"#,
        InspectionSnapshot::SCHEMA_VERSION
    );
    assert!(text.starts_with(&current), "{text}");
    text.replacen(&current, &format!(r#"{{"schema_version":{version}"#), 1)
}

fn rejects_other_revisions<T: serde::de::DeserializeOwned + std::fmt::Debug>(text: &str) {
    for version in [15, 17] {
        let error = serde_json::from_str::<T>(&with_version(text, version))
            .expect_err("another revision is rejected")
            .to_string();
        assert!(
            error.starts_with(&format!(
                "inspection schema version {version} is not supported (this build writes 16)"
            )),
            "{error}"
        );
    }
}

#[test]
fn snapshots_and_query_results_carry_item_bounds_and_decode_only_schema_16() -> TestResult {
    assert_eq!(InspectionSnapshot::SCHEMA_VERSION, 16);
    let mut ui = ui()?;
    let rows = ui.mount_tree(Element::new("rows", TwoRows))?;
    ui.flush(Duration::ZERO)?;
    let snapshot = ui.inspect();
    assert_eq!(snapshot.schema_version, 16);
    let items = |components: &[ComponentInspection]| -> Vec<Option<[f32; 4]>> {
        components
            .iter()
            .find(|component| component.key == "rows")
            .and_then(|component| component.details.semantics.as_ref())
            .map(|semantics| semantics.items.iter().map(|item| item.bounds).collect())
            .unwrap_or_default()
    };
    let expected = [Some([0.0, 0.0, 100.0, 20.0]), None];
    assert_eq!(items(&round_trip(&snapshot)?.components), expected);
    let query = ui.query(
        &InspectionQuery::components(Selector::id(ui.inspection_id(&rows)?))
            .detail(Detail::Semantics),
    );
    assert_eq!(items(&round_trip(&query)?.components), expected);

    // Empty envelopes decode at 16, and no other revision does.
    let empty = serde_json::to_string(&Runtime::new().inspect())?;
    assert_eq!(
        serde_json::from_str::<InspectionSnapshot>(&empty)?.schema_version,
        16
    );
    rejects_other_revisions::<InspectionSnapshot>(&empty);
    let empty_query = serde_json::to_string(&ui.query(&InspectionQuery::new()))?;
    assert!(
        serde_json::from_str::<QueryResult>(&empty_query)?
            .components
            .is_empty()
    );
    rejects_other_revisions::<QueryResult>(&empty_query);
    let unknown = empty.replacen(r#""windows""#, r#""tool":1,"windows""#, 1);
    assert!(serde_json::from_str::<InspectionSnapshot>(&unknown).is_err());

    // An item written without the field has no bounds.
    let text = serde_json::to_string(&query)?;
    let without = text.replacen(r#","bounds":null"#, "", 1);
    assert_ne!(without, text);
    let decoded: QueryResult = serde_json::from_str(&without)?;
    assert_eq!(items(&decoded.components), expected);
    Ok(())
}

#[test]
fn the_schema_version_is_written_first_and_read_wherever_it_is() -> TestResult {
    let mut ui = ui()?;
    ui.mount_tree(Element::new("rows", TwoRows))?;
    ui.flush(Duration::ZERO)?;
    let snapshot = serde_json::to_string(&ui.inspect())?;
    let query = serde_json::to_string(
        &ui.query(&InspectionQuery::components(Selector::key("rows")).detail(Detail::Semantics)),
    )?;
    for text in [&snapshot, &query] {
        assert!(text.starts_with(r#"{"schema_version":16,"#), "{text}");
    }
    // Moved to the end, the version is still checked.
    let moved = |text: &str, version: u32| {
        let body = text.replacen(r#""schema_version":16,"#, "", 1);
        format!(
            r#"{},"schema_version":{version}}}"#,
            body.strip_suffix('}').unwrap_or(&body)
        )
    };
    assert!(serde_json::from_str::<InspectionSnapshot>(&moved(&snapshot, 16)).is_ok());
    assert!(serde_json::from_str::<QueryResult>(&moved(&query, 16)).is_ok());
    assert!(serde_json::from_str::<InspectionSnapshot>(&moved(&snapshot, 15)).is_err());
    assert!(serde_json::from_str::<QueryResult>(&moved(&query, 15)).is_err());
    Ok(())
}

/// Rows of a button, lower than the button's height.
struct Buttons;
impl TableItems for Buttons {
    fn len(&self) -> usize {
        2
    }
    fn key(&self, index: usize) -> Option<String> {
        (index < 2).then(|| format!("r{index}"))
    }
    fn cell(&self, _: usize, _: &str) -> Option<Element> {
        Some(Element::new("button", Button::new("Go", |_| Ok(()))))
    }
}

#[cfg(debug_assertions)]
#[test]
fn a_fixed_height_hint_and_its_issue_round_trip_and_the_hint_is_required() -> TestResult {
    let mut ui = ui()?;
    let controller = TableController::new(
        &ui,
        ListSelection::new(&ui, SelectionMode::Single),
        [TableColumn::new("go", "Go")],
        TableSource::new(Buttons),
    )?;
    ui.mount_tree(Element::new("table", Table::new(&controller, 28.0)?))?;
    ui.flush(Duration::ZERO)?;
    let snapshot = ui.inspect();
    assert!(
        snapshot
            .components
            .iter()
            .any(|component| component.details.validation.fixed_height)
    );
    let issue = snapshot
        .validation
        .issues
        .iter()
        .find(|issue| issue.code == ValidationCode::ExceedsFixedHeight)
        .ok_or("the row is lower than the button")?;
    assert!(
        snapshot
            .validation
            .markers
            .iter()
            .any(|marker| marker.component == issue.components[0]
                && marker.codes.contains(&ValidationCode::ExceedsFixedHeight))
    );
    let back = round_trip(&snapshot)?;
    assert_eq!(back.validation, snapshot.validation);
    let text = serde_json::to_string(&snapshot)?;
    let without = text.replacen(r#","fixed_height":false"#, "", 1);
    assert_ne!(without, text);
    assert!(serde_json::from_str::<InspectionSnapshot>(&without).is_err());
    Ok(())
}
