//! Standard traits and conversions of public values: hashing and ordering that
//! agree with equality, `Display` for colors, and the `From`/`AsRef`
//! conversions beside the named constructors and accessors.

use std::collections::{BTreeSet, HashSet};
use std::hash::Hash;

use pert_ui::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// How many distinct values a hash set keeps of `same` twice and `other` once:
/// 2 when hashing agrees with equality and the two differ.
fn hashed<T: Clone + Eq + Hash>(same: &T, other: &T) -> usize {
    [same.clone(), same.clone(), other.clone()]
        .into_iter()
        .collect::<HashSet<_>>()
        .len()
}

#[test]
fn identities_keys_and_tokens_key_hash_sets() -> TestResult {
    let mut ui = Runtime::new();
    let save = ui.register_command(
        "Save",
        CommandScope::Capture,
        vec![KeyChord::primary('s')],
        |_, _| Ok(()),
    )?;
    let open = ui.register_command(
        "Open",
        CommandScope::Capture,
        vec![KeyChord::primary('o')],
        |_, _| Ok(()),
    )?;
    assert_eq!(hashed(&save, &open), 2);

    let fonts = ui.fonts();
    let body = fonts.default_font(FontRole::Body)?;
    let mono = fonts.default_font(FontRole::Monospace)?;
    assert_eq!(hashed(&body, &mono), 2);

    let tracker = RequestTracker::new();
    let first = tracker.issue("a")?;
    let second = tracker.issue("a")?;
    assert_eq!(hashed(&first, &second), 2, "generations differ");

    assert_eq!(
        hashed(&KeyChord::primary('s'), &KeyChord::primary('s').shifted()),
        2
    );
    assert_eq!(hashed(&KeyCode::Enter, &KeyCode::Escape), 2);
    assert_eq!(hashed(&Modifiers::default(), &Modifiers::PRIMARY), 2);
    assert_eq!(hashed(&ColorRole::Primary, &ColorRole::Surface), 2);
    assert_eq!(hashed(&Icon::Folder, &Icon::Settings), 2);
    assert_eq!(hashed(&ThemeMode::Dark, &ThemeMode::Light), 2);
    assert_eq!(
        hashed(&WorkspacePath::new(["a"])?, &WorkspacePath::new(["b"])?),
        2
    );
    assert_eq!(
        hashed(
            &ValidationCode::DuplicateKey,
            &ValidationCode::InvalidBounds
        ),
        2
    );
    assert_eq!(hashed(&Revision::INITIAL, &Revision::default()), 1);
    Ok(())
}

#[test]
fn validated_scalars_order_and_hash_like_their_numbers() -> TestResult {
    let small = Capacity::new(8)?;
    let large = Capacity::new(64)?;
    assert!(small < large);
    assert_eq!(BTreeSet::from([large, small]).first(), Some(&small));

    let low = PowerExponent::new(0.5)?;
    let high = PowerExponent::new(2.0)?;
    assert!(low < high);
    assert_eq!(hashed(&low, &high), 2);

    let shrink = Magnification::new(0.9)?;
    let grow = Magnification::new(1.1)?;
    assert_eq!(shrink.max(grow), grow);
    assert_eq!(hashed(&shrink, &grow), 2);

    assert_eq!(UiZoom::MIN.max(UiZoom::MAX), UiZoom::MAX);
    assert_eq!(hashed(&UiZoom::default(), &UiZoom::MAX), 2);
    assert_eq!(ViewScale::MIN.min(ViewScale::ONE), ViewScale::MIN);
    assert_eq!(hashed(&ViewScale::ONE, &ViewScale::MAX), 2);
    assert!(PresentationRate::MIN < PresentationRate::MAX);
    assert_eq!(
        hashed(&PresentationRate::new(60.0)?, &PresentationRate::MAX),
        2
    );
    Ok(())
}

#[test]
fn images_hash_by_content_not_by_allocation() -> TestResult {
    let red = RasterImage::from_srgba8(1, 1, vec![255, 0, 0, 255])?;
    let same_red = RasterImage::from_srgba8(1, 1, vec![255, 0, 0, 255])?;
    let green = RasterImage::from_srgba8(1, 1, vec![0, 255, 0, 255])?;
    assert_eq!(
        [red, same_red, green]
            .into_iter()
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    let mask = MaskImage::new(2, 1, vec![0, 255])?;
    let same_mask = MaskImage::new(2, 1, vec![0, 255])?;
    let other_mask = MaskImage::new(1, 2, vec![0, 255])?;
    assert_eq!(
        [mask, same_mask, other_mask]
            .into_iter()
            .collect::<HashSet<_>>()
            .len(),
        2
    );
    Ok(())
}

#[test]
fn a_color_displays_as_its_quantized_hex_text() -> TestResult {
    let translucent = Color::from_srgb8(255, 136, 0, 128);
    assert_eq!(translucent.to_string(), "#ff880080");
    assert_eq!(translucent.to_string(), translucent.to_hex());
    assert_eq!(format!("{translucent}").parse::<Color>()?, translucent);
    // Linear channels between two bytes print rounded.
    let between = Color::from_linear(0.5, 0.5, 0.5, 1.0)?;
    let text = between.to_string();
    assert_eq!(text, between.to_hex());
    assert_eq!(text.parse::<Color>()?.to_srgb8(), between.to_srgb8());
    Ok(())
}

#[test]
fn validated_quantities_convert_into_their_raw_numbers() -> TestResult {
    assert_eq!(f32::from(NormalizedValue::try_from(0.25)?), 0.25);
    assert_eq!(usize::from(Capacity::new(12)?), 12);
    assert_eq!(f32::from(Length::new(3.5)?), 3.5);
    assert_eq!(f32::from(Magnification::new(1.25)?), 1.25);
    assert_eq!(f32::from(ViewScale::ONE), 1.0);
    assert_eq!(f64::from(PresentationRate::new(120.0)?), 120.0);
    Ok(())
}

#[test]
fn choice_identities_and_font_bytes_convert_through_std_traits() -> TestResult {
    fn text_of(value: impl AsRef<str>) -> String {
        value.as_ref().to_owned()
    }
    assert_eq!(text_of(ChoiceId::new("low pass")?), "low pass");
    assert!(ContentQuery::new(ChoiceId::new("pass")?).is_ok());

    static NOT_A_FONT: [u8; 30] = [0; 30];
    let ui = Runtime::new();
    let fonts = ui.fonts();
    for data in [FontData::from(vec![0; 30]), FontData::from(&NOT_A_FONT[..])] {
        assert_eq!(
            fonts.register(data),
            Err(RuntimeError::Font(FontError::InvalidData))
        );
    }
    Ok(())
}

#[test]
fn keyed_descriptions_convert_into_elements() -> TestResult {
    let ui = Runtime::new();
    let content = || Element::new("body", Label::new("Body"));
    let tabs: Vec<Element> = [
        Tab::new("mix", "Mix", content()),
        Tab::new("edit", "Edit", content()),
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    assert_eq!(tabs[1].key(), "edit");
    assert_eq!(tabs[0].label(), Some("Mix"));

    let section = Element::from(Section::new("general", "General", content()));
    assert_eq!(
        (section.key(), section.label()),
        ("general", Some("General"))
    );
    let property = Element::from(Property::new("gain", "Gain", content()));
    assert_eq!((property.key(), property.label()), ("gain", Some("Gain")));
    let window = Element::from(FloatingWindow::new(&ui, "tools", "Tools", content()));
    assert_eq!(window.key(), "tools");
    Ok(())
}

/// Requires `Debug`, so the call fails to compile for a type without it.
fn debug<T: std::fmt::Debug>() {}

#[test]
fn state_owners_handles_contexts_and_backends_implement_debug() {
    debug::<ListSelection>();
    debug::<ListController>();
    debug::<ListFilter>();
    debug::<TableController>();
    debug::<TreeController>();
    debug::<FieldHandle>();
    debug::<FormFields>();
    debug::<ContextMenuController>();
    debug::<ThemeManager>();
    debug::<FontManager>();
    debug::<IconManager>();
    debug::<UiSettings>();
    debug::<FilterTarget>();
    debug::<ChildInfo>();
    debug::<Runtime>();
    debug::<EventContext>();
    debug::<UpdateContext>();
    debug::<MeasureContext<'static>>();
    debug::<ArrangeContext<'static>>();
    debug::<DescribeContext<'static>>();
    debug::<CellEdit<'static>>();
    debug::<Painter>();
    debug::<VectorRasterizer>();
    #[cfg(feature = "native")]
    debug::<App>();
}

#[test]
fn application_state_derives_debug_without_debug_values() -> TestResult {
    /// A value with no `Debug`, as application data often is.
    #[derive(Clone)]
    struct Milliseconds;

    #[derive(Debug)]
    #[expect(dead_code, reason = "the fields exist to be formatted")]
    struct Settings {
        selection: ListSelection,
        form: FormFields,
        name: FieldHandle,
        theme: ThemeManager,
        delay: NumberFormat<Milliseconds>,
        runtime: Runtime,
    }

    let ui = Runtime::new();
    let form = FormFields::new(&ui);
    let settings = Settings {
        selection: ListSelection::new(&ui, SelectionMode::Single),
        name: form.field("name")?,
        form,
        theme: ui.theme(),
        delay: NumberFormat::new(
            "0 ms",
            |_: Milliseconds| "0".to_owned(),
            |_| Ok(Milliseconds),
        ),
        runtime: ui,
    };
    let text = format!("{settings:?}");
    assert!(
        text.contains("ListSelection { mode: Single, .. }"),
        "{text}"
    );
    assert!(
        text.contains(r#"FieldHandle { key: "name", .. }"#),
        "{text}"
    );
    assert!(text.contains(r#"NumberFormat { hint: "0 ms""#), "{text}");
    Ok(())
}
