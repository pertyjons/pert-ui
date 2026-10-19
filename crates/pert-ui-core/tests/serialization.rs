//! The `serde` feature: every owned parameter type round-trips through
//! `serde_json`, and deserialization rejects what the constructors reject.
#![cfg(feature = "serde")]

use std::num::NonZeroU64;

use pert_ui_core::{
    AnyParameterSnapshot, ChoiceId, ChoiceOption, EditSource, NormalizedValue, ParameterId,
    ParameterMapping, ParameterSnapshot, ParameterSpec, PowerExponent, Revision,
};

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let text = serde_json::to_string(value).expect("serializes");
    let back: T = serde_json::from_str(&text).expect("deserializes its own output");
    assert_eq!(&back, value, "{text}");
}

fn choice(id: &str) -> ChoiceId {
    ChoiceId::new(id).expect("valid id")
}

#[test]
fn every_family_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let continuous = ParameterSpec::new("Cutoff", 20.0, 20_000.0, 1_000.0)?
        .with_mapping(ParameterMapping::Logarithmic)?
        .read_only();
    let stepped =
        ParameterSpec::stepped("Voices", -3, 5, 1)?.with_step(NonZeroU64::new(2).unwrap())?;
    let boolean = ParameterSpec::boolean("Bypass", false)?;
    let choices = ParameterSpec::choices(
        "Filter",
        [
            ChoiceOption::new(choice("lp"), "Low pass"),
            ChoiceOption::new(choice("hp"), "High pass"),
        ],
        choice("hp"),
    )?;
    round_trip(&continuous);
    round_trip(&stepped);
    round_trip(&boolean);
    round_trip(&choices);
    let state = ParameterSnapshot {
        edit_source: Some(EditSource::new(7)),
        base: choice("lp"),
        preview: Some(choice("hp")),
        effective: choice("lp"),
        revision: Revision::INITIAL,
        available: true,
    };
    round_trip(&AnyParameterSnapshot::new(choices, state));
    round_trip(&AnyParameterSnapshot::new(
        continuous,
        ParameterSnapshot {
            edit_source: None,
            base: 440.0,
            preview: None,
            effective: 440.0,
            revision: Revision::INITIAL,
            available: false,
        },
    ));
    Ok(())
}

#[test]
fn the_written_shape_is_the_documented_one() -> Result<(), Box<dyn std::error::Error>> {
    let spec = ParameterSpec::stepped("Voices", 1, 8, 4)?;
    let value: serde_json::Value = serde_json::to_value(&spec)?;
    assert_eq!(
        value,
        serde_json::json!({
            "name": "Voices",
            "default": 4,
            "read_only": false,
            "domain": { "minimum": 1, "maximum": 8, "step": 1 }
        })
    );
    Ok(())
}

#[test]
fn identifiers_keep_128_bits_as_text() -> Result<(), Box<dyn std::error::Error>> {
    let id = ParameterId::new(u128::MAX, 1 << 100, "cutoff")?.next_incarnation()?;
    let text = serde_json::to_string(&id)?;
    assert!(text.contains(&format!("\"{}\"", u128::MAX)), "{text}");
    assert!(text.contains("\"incarnation\":1"), "{text}");
    assert_eq!(serde_json::from_str::<ParameterId>(&text)?, id);
    Ok(())
}

#[test]
fn invalid_input_is_rejected_not_clamped() {
    let rejected = |text: &str| {
        assert!(
            serde_json::from_str::<serde_json::Value>(text).is_ok(),
            "the test input must be valid JSON: {text}"
        );
        text.to_owned()
    };
    // A default outside the bounds, the constructor's OutOfRange.
    let text = rejected(
        r#"{"name":"Cutoff","default":5.0,"read_only":false,"domain":{"minimum":0.0,"maximum":1.0,"mapping":"Linear"}}"#,
    );
    assert!(serde_json::from_str::<ParameterSpec<f32>>(&text).is_err());
    // Logarithmic mapping over a domain that reaches zero.
    let text = rejected(
        r#"{"name":"Cutoff","default":0.5,"read_only":false,"domain":{"minimum":0.0,"maximum":1.0,"mapping":"Logarithmic"}}"#,
    );
    assert!(serde_json::from_str::<ParameterSpec<f32>>(&text).is_err());
    // A maximum off the step lattice.
    let text = rejected(
        r#"{"name":"Voices","default":1,"read_only":false,"domain":{"minimum":1,"maximum":8,"step":3}}"#,
    );
    assert!(serde_json::from_str::<ParameterSpec<i64>>(&text).is_err());
    // A duplicate option and a default that is not an option.
    let text = rejected(
        r#"{"name":"Filter","default":"lp","read_only":false,"domain":{"options":[{"id":"lp","label":"a"},{"id":"lp","label":"b"}]}}"#,
    );
    assert!(serde_json::from_str::<ParameterSpec<ChoiceId>>(&text).is_err());
    let text = rejected(
        r#"{"name":"Filter","default":"bp","read_only":false,"domain":{"options":[{"id":"lp","label":"a"}]}}"#,
    );
    assert!(serde_json::from_str::<ParameterSpec<ChoiceId>>(&text).is_err());
    // An unknown field, which the closed representation refuses.
    let text =
        rejected(r#"{"name":"Bypass","default":true,"read_only":false,"domain":{},"extra":1}"#);
    assert!(serde_json::from_str::<ParameterSpec<bool>>(&text).is_err());
    // Identities and positions keep their own rules.
    assert!(serde_json::from_str::<ChoiceId>("\" padded\"").is_err());
    assert!(serde_json::from_str::<ChoiceId>("\"\"").is_err());
    assert!(serde_json::from_str::<NormalizedValue>("1.5").is_err());
    assert!(serde_json::from_str::<NormalizedValue>("0.25").is_ok());
    let text = rejected(r#"{"session":"1","object":"2","key":"  ","incarnation":0}"#);
    assert!(serde_json::from_str::<ParameterId>(&text).is_err());
    for session in ["-1", "+1", "01", "0x1", " 1", "1 ", ""] {
        let text = rejected(&format!(
            r#"{{"session":"{session}","object":"2","key":"cutoff","incarnation":0}}"#
        ));
        assert!(
            serde_json::from_str::<ParameterId>(&text).is_err(),
            "non-canonical {session:?}"
        );
    }
    let text = rejected(r#"{"session":"0","object":"+0002","key":"cutoff","incarnation":0}"#);
    assert!(serde_json::from_str::<ParameterId>(&text).is_err());
    let text = rejected(r#"{"session":"0","object":"2","key":"cutoff","incarnation":0}"#);
    assert!(serde_json::from_str::<ParameterId>(&text).is_ok());
}

#[test]
fn snapshots_reject_values_their_family_or_specification_cannot_hold()
-> Result<(), Box<dyn std::error::Error>> {
    // A value too large for an f32 would otherwise read as infinity and then
    // write back as `null`, which cannot be read again.
    let snapshot = |base: &str, preview: &str, effective: &str| {
        format!(
            r#"{{"edit_source":null,"base":{base},"preview":{preview},"effective":{effective},"revision":0,"available":true}}"#
        )
    };
    assert!(
        serde_json::from_str::<ParameterSnapshot<f32>>(&snapshot("0.5", "null", "0.5")).is_ok()
    );
    for (base, preview, effective) in [
        ("1e39", "null", "0.5"),
        ("0.5", "-1e39", "0.5"),
        ("0.5", "null", "-1e39"),
    ] {
        let text = snapshot(base, preview, effective);
        assert!(
            serde_json::from_str::<serde_json::Value>(&text).is_ok(),
            "{text}"
        );
        assert!(
            serde_json::from_str::<ParameterSnapshot<f32>>(&text).is_err(),
            "{text}"
        );
    }
    let text = r#"{"edit_source":null,"base":1,"preview":null,"effective":1,"revision":0,"available":true,"extra":0}"#;
    assert!(serde_json::from_str::<ParameterSnapshot<i64>>(text).is_err());

    // Values are checked against the specification the snapshot carries.
    let unit = ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?;
    let valid = serde_json::to_value(AnyParameterSnapshot::new(
        unit,
        ParameterSnapshot {
            edit_source: None,
            base: 0.5,
            preview: None,
            effective: 0.5,
            revision: Revision::INITIAL,
            available: true,
        },
    ))?;
    let with = |field: &str, value: serde_json::Value| {
        let mut text = valid.clone();
        text["Continuous"]["state"][field] = value;
        serde_json::from_value::<AnyParameterSnapshot>(text)
    };
    assert!(with("base", 1.0.into()).is_ok());
    assert!(with("base", 5.0.into()).is_err(), "base outside the domain");
    assert!(
        with("preview", 5.0.into()).is_err(),
        "preview outside the domain"
    );
    // An effective overlay may lie outside the editable domain, but must be finite.
    assert!(with("effective", 5.0.into()).is_ok());
    assert!(with("effective", 1e39.into()).is_err());

    let stepped =
        ParameterSpec::stepped("Voices", 0, 8, 2)?.with_step(NonZeroU64::new(2).unwrap())?;
    let valid = serde_json::to_value(AnyParameterSnapshot::new(
        stepped,
        ParameterSnapshot {
            edit_source: None,
            base: 2,
            preview: Some(4),
            effective: 3,
            revision: Revision::INITIAL,
            available: true,
        },
    ))?;
    assert!(serde_json::from_value::<AnyParameterSnapshot>(valid.clone()).is_ok());
    let mut off_lattice = valid;
    off_lattice["Stepped"]["state"]["base"] = 3.into();
    assert!(serde_json::from_value::<AnyParameterSnapshot>(off_lattice).is_err());

    let filter = ParameterSpec::choices(
        "Filter",
        [
            ChoiceOption::new(choice("lp"), "Low pass"),
            ChoiceOption::new(choice("hp"), "High pass"),
        ],
        choice("lp"),
    )?;
    let valid = serde_json::to_value(AnyParameterSnapshot::new(
        filter,
        ParameterSnapshot {
            edit_source: None,
            base: choice("lp"),
            preview: None,
            effective: choice("hp"),
            revision: Revision::INITIAL,
            available: true,
        },
    ))?;
    assert!(serde_json::from_value::<AnyParameterSnapshot>(valid.clone()).is_ok());
    for field in ["base", "preview", "effective"] {
        let mut unknown = valid.clone();
        unknown["Choice"]["state"][field] = "bp".into();
        assert!(
            serde_json::from_value::<AnyParameterSnapshot>(unknown).is_err(),
            "{field} names no option"
        );
    }
    Ok(())
}

#[test]
fn a_power_mapping_round_trips_and_an_invalid_exponent_is_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let spec = ParameterSpec::new("Gain", -60.0, 12.0, 0.0)?
        .with_mapping(ParameterMapping::Power(PowerExponent::new(2.0)?))?;
    round_trip(&spec);
    let text = serde_json::to_string(&spec)?;
    assert!(text.contains(r#""mapping":{"Power":2.0}"#), "{text}");
    for invalid in ["0.0", "20.0", "-1.0"] {
        let bad = text.replace(r#"{"Power":2.0}"#, &format!(r#"{{"Power":{invalid}}}"#));
        assert!(
            serde_json::from_str::<ParameterSpec<f32>>(&bad).is_err(),
            "{bad}"
        );
    }
    assert!(serde_json::from_str::<PowerExponent>("0.5").is_ok());
    assert!(serde_json::from_str::<PowerExponent>("0.05").is_err());
    Ok(())
}
