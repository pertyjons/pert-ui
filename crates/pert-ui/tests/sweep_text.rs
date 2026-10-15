//! Text fitting, number presentation and glyph regressions from the
//! post-repair sweep, through the public facade.
use pert_ui::*;
use std::time::Duration;

fn label_details(text: &str, overflow: TextOverflow, width: f32) -> ComponentDetails {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(width, 100.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new(
        "column",
        Column::new([Element::new("label", Label::new(text).overflow(overflow))]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap();
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == "label")
        .unwrap()
        .details
}
fn shown(details: &ComponentDetails) -> Option<String> {
    match details.properties.get("shown") {
        Some(InspectValue::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

#[test]
fn an_ellipsis_truncates_text_wider_than_the_raster_limit() {
    // Several thousand wide letters exceed the raster side unwrapped, but
    // the truncation shown in 200 pixels fits.
    let text = "W".repeat(4000);
    let details = label_details(&text, TextOverflow::Ellipsis, 200.0);
    let shown = shown(&details).expect("the label is truncated");
    assert!(shown.ends_with('\u{2026}') && shown.len() < 200, "{shown}");
}

#[test]
fn unicode_line_and_paragraph_separators_end_the_single_line() {
    for separator in ['\u{2028}', '\u{2029}'] {
        let details = label_details(
            &format!("First{separator}Second"),
            TextOverflow::Ellipsis,
            400.0,
        );
        assert_eq!(
            shown(&details).as_deref(),
            Some("First\u{2026}"),
            "{separator:?} ends the first line"
        );
    }
}

#[test]
fn trimming_beside_the_ellipsis_keeps_clusters_whole() {
    // A no-break space carrying a combining accent is one cluster, not
    // whitespace to trim away from its accent.
    let text = "Alpha Beta Gamma Delta\u{00A0}\u{0301}";
    let mut truncated = 0;
    for width in (20..=220).step_by(4) {
        let details = label_details(text, TextOverflow::MiddleEllipsis, width as f32);
        if let Some(shown) = shown(&details) {
            truncated += 1;
            assert!(
                !shown.contains("\u{2026}\u{0301}"),
                "width {width}: {shown:?} split a cluster"
            );
        }
    }
    assert!(truncated > 0, "some widths truncate");
}

#[test]
fn line_separators_cannot_separate_digits() {
    for separator in ['\u{2028}', '\u{2029}'] {
        assert_eq!(
            NumberLocale::new('.', Some(separator)),
            Err(NumberLocaleError::InvalidSeparator(separator))
        );
        assert_eq!(
            NumberLocale::new(separator, None),
            Err(NumberLocaleError::InvalidSeparator(separator))
        );
    }
}

#[test]
fn values_shown_as_zero_have_no_minus_sign() {
    let locale = NumberLocale::new(',', None).unwrap();
    assert_eq!(locale.format_decimal(-0.001, Some(1)), "0,0");
    assert_eq!(locale.format_decimal(-0.0, None), "0");
    assert_eq!(locale.format_decimal(-0.5, Some(1)), "-0,5");
    let mut ui = Runtime::new();
    let value = ui.signal(-0.04_f64);
    ui.mount_tree(Element::new(
        "field",
        DecimalInput::new(value).decimal_places(1).unwrap(),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let text = ui
        .inspect()
        .components
        .into_iter()
        .find(|component| component.key == "field")
        .and_then(|component| component.details.properties.get("text").cloned());
    assert_eq!(text, Some(InspectValue::Text("0.0".into())));
}

#[test]
fn inline_toggle_glyphs_keep_whole_clusters() {
    let mut ui = Runtime::new();
    let on = ui.signal(false);
    ui.mount_tree(Element::new(
        "row",
        Row::new([
            Element::new(
                "named",
                InlineToggle::new(on.clone(), "\u{1F44D}\u{1F3FD} Like"),
            ),
            Element::new(
                "glyph",
                InlineToggle::new(on, "Accent").glyph("e\u{0301}x\u{0301}y"),
            ),
        ]),
    ))
    .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let glyph = |key: &str| {
        ui.inspect()
            .components
            .into_iter()
            .find(|component| component.key == key)
            .and_then(|component| component.details.properties.get("glyph").cloned())
    };
    assert_eq!(
        glyph("named"),
        Some(InspectValue::Text("\u{1F44D}\u{1F3FD}".into()))
    );
    assert_eq!(
        glyph("glyph"),
        Some(InspectValue::Text("e\u{0301}x\u{0301}".into()))
    );
}
