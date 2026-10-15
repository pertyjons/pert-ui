//! Explicit encoding of diagnostic floating-point numbers.
//!
//! Diagnostics report what a component actually said, and a component can say
//! `NaN` or infinity; that is what some validation codes exist to catch. Many
//! text encodings have no such number, and `serde_json` would write `null` and
//! read it back as an error. In a human-readable format these helpers write a
//! finite number as a number and a non-finite one as the text `"NaN"`, `"inf"`
//! or `"-inf"`, and accept either form when reading, which needs a
//! self-describing format. A binary format (`Serializer::is_human_readable`
//! false) carries IEEE floats, non-finite ones included, so there the plain
//! number is written and read. Either way a snapshot never loses or invents a
//! value. Used through `#[serde(with = "...")]` on the fields that can carry
//! them; an `Option` field also takes `serde(default)`, so that an omitted
//! field reads as absent like a `null` one.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum Float<'a> {
    Number(f64),
    Text(std::borrow::Cow<'a, str>),
}

fn write<S: Serializer>(value: f64, serializer: S) -> Result<S::Ok, S::Error> {
    if !serializer.is_human_readable() || value.is_finite() {
        Float::Number(value).serialize(serializer)
    } else if value.is_nan() {
        Float::Text("NaN".into()).serialize(serializer)
    } else if value > 0.0 {
        Float::Text("inf".into()).serialize(serializer)
    } else {
        Float::Text("-inf".into()).serialize(serializer)
    }
}

fn read<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    if !deserializer.is_human_readable() {
        return f64::deserialize(deserializer);
    }
    match Float::deserialize(deserializer)? {
        Float::Number(value) => Ok(value),
        Float::Text(text) => match &*text {
            "NaN" => Ok(f64::NAN),
            "inf" => Ok(f64::INFINITY),
            "-inf" => Ok(f64::NEG_INFINITY),
            other => Err(D::Error::custom(format!(
                "expected a number or one of \"NaN\", \"inf\" and \"-inf\", found {other:?}"
            ))),
        },
    }
}

/// An `f64` that may be non-finite.
pub(crate) mod f64 {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(value: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        write(*value, serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        read(deserializer)
    }
}

/// An `f32` that may be non-finite.
pub(crate) mod f32 {
    use super::*;

    pub(crate) fn serialize<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
        write(f64::from(*value), serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f32, D::Error> {
        let wide = read(deserializer)?;
        let narrow = wide as f32;
        // A finite number beyond f32 would become infinity, which is a value
        // the input did not carry; the explicit spellings still read as such.
        if wide.is_finite() && !narrow.is_finite() {
            return Err(D::Error::custom(format!("{wide} does not fit an f32")));
        }
        Ok(narrow)
    }
}

/// An `Option<f64>` whose value may be non-finite.
pub(crate) mod option_f64 {
    use super::*;

    #[derive(Serialize)]
    struct Write<'a>(#[serde(with = "super::f64")] &'a f64);
    #[derive(Deserialize)]
    struct Read(#[serde(with = "super::f64")] f64);

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<f64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_ref().map(Write).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<f64>, D::Error> {
        Ok(Option::<Read>::deserialize(deserializer)?.map(|read| read.0))
    }
}

/// Four `f32` values, a rectangle, insets or a color, any of which may be
/// non-finite.
pub(crate) mod array4 {
    use super::*;

    #[derive(Serialize)]
    struct Write<'a>(#[serde(with = "super::f32")] &'a f32);
    #[derive(Deserialize)]
    struct Read(#[serde(with = "super::f32")] f32);

    pub(crate) fn serialize<S: Serializer>(
        value: &[f32; 4],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        [
            Write(&value[0]),
            Write(&value[1]),
            Write(&value[2]),
            Write(&value[3]),
        ]
        .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[f32; 4], D::Error> {
        let [a, b, c, d] = <[Read; 4]>::deserialize(deserializer)?;
        Ok([a.0, b.0, c.0, d.0])
    }
}

/// An optional [`array4`].
pub(crate) mod option_array4 {
    use super::*;

    #[derive(Serialize)]
    struct Write<'a>(#[serde(with = "super::array4")] &'a [f32; 4]);
    #[derive(Deserialize)]
    struct Read(#[serde(with = "super::array4")] [f32; 4]);

    pub(crate) fn serialize<S: Serializer>(
        value: &Option<[f32; 4]>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_ref().map(Write).serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<[f32; 4]>, D::Error> {
        Ok(Option::<Read>::deserialize(deserializer)?.map(|read| read.0))
    }
}
