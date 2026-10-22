//! UI-local numeric presentation, separate from canonical values and mappings.
use crate::NumberInputError;
use std::rc::Rc;

type Format<T> = Rc<dyn Fn(T) -> String>;
type Parse<T> = Rc<dyn Fn(&str) -> Result<T, NumberInputError>>;
/// Cloneable presentation and parsing for a numeric control on the UI thread.
///
/// Callbacks receive/return canonical numeric values. They must be deterministic,
/// bounded and free of side effects, binding writes and I/O. The control validates
/// parsed values against its type and range. Formatting never changes stored values.
/// Output obeys the single-line editor's 16 KiB and text preparation limits.
///
/// `width_hint` reserves the complete value slot, including any unit and space.
/// Supply an example at least as wide as every intended display; longer output
/// clips instead of moving adjacent controls. It is measured in the active font,
/// density and zoom. Editing scrolls within the same slot.
///
/// Cloning a format preserves its callback identity. Re-describing a numeric
/// control with an unchanged clone retains its layout. Constructing a new format
/// or replacing one of its callbacks counts as changed presentation even when
/// the callback happens to return the same text.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, InlineDecimal, NumberFormat, NumberInputError, Runtime};
/// let mut ui = Runtime::new();
/// let seconds = ui.signal(0.125_f64);
/// let milliseconds = NumberFormat::new("9999.0 ms",
///     |v: f64| format!("{:.1}", v * 1000.0),
///     |s| s.trim().parse::<f64>().map(|v| v / 1000.0)
///         .map_err(|_| NumberInputError::InvalidNumber))
///     .edit_text(|v| (v * 1000.0).to_string());
/// ui.mount_tree(Element::new("time", InlineDecimal::new(seconds)
///     .range(0.0, 9.0)?.label("Time").unit("ms").format(milliseconds)))?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct NumberFormat<T> {
    pub(crate) hint: String,
    pub(crate) display: Format<T>,
    pub(crate) edit: Format<T>,
    pub(crate) parse: Parse<T>,
    pub(crate) unit: Option<Format<T>>,
    pub(crate) edit_unit: Option<String>,
}
impl<T> Clone for NumberFormat<T> {
    fn clone(&self) -> Self {
        Self {
            hint: self.hint.clone(),
            display: self.display.clone(),
            edit: self.edit.clone(),
            parse: self.parse.clone(),
            unit: self.unit.clone(),
            edit_unit: self.edit_unit.clone(),
        }
    }
}
impl<T> NumberFormat<T> {
    /// Whether optional formats share their callbacks and presentation metadata.
    /// Callback behavior is not compared; a newly constructed callback is new.
    pub(crate) fn same_optional(a: &Option<Self>, b: &Option<Self>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                a.hint == b.hint
                    && a.edit_unit == b.edit_unit
                    && Rc::ptr_eq(&a.display, &b.display)
                    && Rc::ptr_eq(&a.edit, &b.edit)
                    && Rc::ptr_eq(&a.parse, &b.parse)
                    && match (&a.unit, &b.unit) {
                        (None, None) => true,
                        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                        _ => false,
                    }
            }
            _ => false,
        }
    }
    /// Creates a custom format with a complete value-slot width example.
    /// `display` supplies the value text without a unit; it also supplies editing
    /// text until `edit_text` overrides it. `parse` interprets text in those display
    /// units and returns a canonical value or a user-visible validation error.
    pub fn new(
        width_hint: impl Into<String>,
        display: impl Fn(T) -> String + 'static,
        parse: impl Fn(&str) -> Result<T, NumberInputError> + 'static,
    ) -> Self {
        let display: Format<T> = Rc::new(display);
        Self {
            hint: width_hint.into(),
            edit: display.clone(),
            display,
            parse: Rc::new(parse),
            unit: None,
            edit_unit: None,
        }
    }
    /// Overrides initial editing text, for example with unrounded display units.
    /// Unchanged submissions preserve the original exact value even if text rounds.
    pub fn edit_text(mut self, edit: impl Fn(T) -> String + 'static) -> Self {
        self.edit = Rc::new(edit);
        self
    }
    /// Supplies a value-dependent suffix, overriding the control's literal unit.
    /// The suffix stays outside selection/IME text. `parse` must understand the
    /// editing text without this suffix; use a stable editing scale if the resting
    /// suffix changes scale, such as seconds/milliseconds. Pair it with `edit_unit`.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, InlineDecimal, NumberFormat, NumberInputError, Runtime};
    /// let mut ui = Runtime::new();
    /// let seconds = ui.signal(0.25_f64);
    /// let format = NumberFormat::new("9999 ms",
    ///     |v: f64| if v < 1.0 { format!("{:.0}", v * 1000.0) } else { format!("{v:.2}") },
    ///     |s| s.parse::<f64>().map_err(|_| NumberInputError::InvalidNumber))
    ///     .unit(|v| if v < 1.0 { "ms".into() } else { "s".into() })
    ///     .edit_text(|v| v.to_string()).edit_unit("s");
    /// ui.mount_tree(Element::new("time", InlineDecimal::new(seconds).format(format)))?;
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn unit(mut self, unit: impl Fn(T) -> String + 'static) -> Self {
        self.unit = Some(Rc::new(unit));
        self
    }
    /// Overrides the suffix shown during text editing with a fixed editing unit.
    /// Use with `edit_text` and `parse` when resting display changes unit/scale.
    /// Default uses the display suffix; this builder changes text only, not values.
    pub fn edit_unit(mut self, unit: impl Into<String>) -> Self {
        self.edit_unit = Some(unit.into());
        self
    }
}

/// Plain decimal text without the sign of a value that shows as zero, such as
/// `-0`, `-0.0` or Rust's `-0` for negative zero: a minus sign there would
/// claim a negative value the digits do not show.
pub(crate) fn unsigned_zero(text: String) -> String {
    match text.strip_prefix('-') {
        Some(rest) if rest.chars().all(|c| c == '0' || c == '.') => rest.to_owned(),
        _ => text,
    }
}

/// A refused [`NumberLocale`] separator.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NumberLocaleError {
    /// A separator is a digit, a sign (`+`, `-`, `−`), `e`/`E`, a control
    /// character, which would make text ambiguous, or a line or paragraph
    /// separator, which would break a single-line value.
    #[error("{0:?} cannot separate digits")]
    InvalidSeparator(char),
    /// The decimal separator and the digit-grouping separator are the same,
    /// or both are spaces, which grouping treats as one class.
    #[error("the decimal and grouping separators are the same")]
    SameSeparators,
}

/// An application-chosen decimal separator and optional digit grouping, for
/// [`NumberFormat::localized_decimal`] and [`NumberFormat::localized_integer`].
///
/// The library ships no locale data: the application picks the separators,
/// for example from its own settings. Formatting and parsing are
/// deterministic and depend on nothing else. Without this helper, numbers
/// keep their default presentation.
///
/// **Formatting** writes an optional `-`, the integer digits with the grouping
/// separator between groups of three counted from the decimal separator, and
/// then the separator and the fraction.
///
/// **Parsing** trims surrounding whitespace and accepts an optional sign (`+`,
/// `-` or the minus sign `−`), at least one digit, the configured decimal
/// separator at most once, and the grouping separator only between integer
/// digits in groups of exactly three counted from the decimal separator, so
/// `1 234,5` is accepted and `12 34,5` is refused. When the grouping separator
/// is a space, the space, the no-break space and the narrow no-break space are
/// accepted interchangeably. Exponents, non-finite values and any other
/// character are refused with [`NumberInputError::InvalidNumber`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{NumberFormat, NumberInputError, NumberLocale};
/// let swedish = NumberLocale::new(',', Some('\u{a0}'))?;
/// assert_eq!(swedish.format_decimal(-1234567.25, Some(2)), "-1\u{a0}234\u{a0}567,25");
/// assert_eq!(swedish.parse_decimal("1 234,5"), Ok(1234.5));
/// assert_eq!(swedish.parse_decimal("12 34,5"), Err(NumberInputError::InvalidNumber));
/// let format: NumberFormat<f64> = NumberFormat::localized_decimal(swedish, Some(2), "-9 999,99");
/// # let _ = format;
/// # Ok::<(), pert_ui::NumberLocaleError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumberLocale {
    decimal: char,
    grouping: Option<char>,
}

impl Default for NumberLocale {
    /// A point and no grouping: the same text as the default number display.
    fn default() -> Self {
        Self {
            decimal: '.',
            grouping: None,
        }
    }
}

impl NumberLocale {
    /// A locale with `decimal` as the decimal separator and `grouping`, if any,
    /// between groups of three integer digits.
    ///
    /// # Errors
    /// [`NumberLocaleError::InvalidSeparator`] for a digit, a sign, `e`/`E`, a
    /// control character or a line or paragraph separator (U+2028, U+2029),
    /// which single-line controls cannot show, and [`NumberLocaleError::SameSeparators`] when both
    /// separators are the same character or both are spaces: a space grouping
    /// also accepts the other spaces (space, no-break space, narrow no-break
    /// space), so a space decimal separator beside it would make text such as
    /// `1 234` ambiguous.
    pub fn new(decimal: char, grouping: Option<char>) -> Result<Self, NumberLocaleError> {
        for separator in std::iter::once(decimal).chain(grouping) {
            if separator.is_ascii_digit()
                || matches!(separator, '+' | '-' | '\u{2212}' | 'e' | 'E')
                || separator.is_control()
                || matches!(separator, '\u{2028}' | '\u{2029}')
            {
                return Err(NumberLocaleError::InvalidSeparator(separator));
            }
        }
        if grouping.is_some_and(|grouping| {
            grouping == decimal || (is_space(grouping) && is_space(decimal))
        }) {
            return Err(NumberLocaleError::SameSeparators);
        }
        Ok(Self { decimal, grouping })
    }
    /// The decimal separator.
    #[must_use]
    pub const fn decimal(self) -> char {
        self.decimal
    }
    /// The digit-grouping separator, if any.
    #[must_use]
    pub const fn grouping(self) -> Option<char> {
        self.grouping
    }
    /// Formats a finite value with `places` fractional digits, or with the
    /// shortest text that reads back as the same value when `places` is
    /// `None`. A value that shows as zero, including negative zero and a
    /// small negative value rounded away, has no minus sign. A non-finite value
    /// is written as Rust writes it (`NaN`, `inf`); controls never hold one.
    #[must_use]
    pub fn format_decimal(self, value: f64, places: Option<u8>) -> String {
        if !value.is_finite() {
            return value.to_string();
        }
        let text = match places {
            Some(places) => format!("{value:.*}", usize::from(places)),
            None => value.to_string(),
        };
        self.localize(&unsigned_zero(text))
    }
    /// Formats an integer with grouping.
    #[must_use]
    pub fn format_integer(self, value: i64) -> String {
        self.localize(&value.to_string())
    }
    /// Parses decimal text written in this locale, as described on the type.
    ///
    /// # Errors
    /// [`NumberInputError::InvalidNumber`] for text that does not follow the
    /// rules, and [`NumberInputError::NonFinite`] for a value too large to
    /// represent.
    pub fn parse_decimal(self, text: &str) -> Result<f64, NumberInputError> {
        let canonical = self.canonical(text, true)?;
        let value: f64 = canonical
            .parse()
            .map_err(|_| NumberInputError::InvalidNumber)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(NumberInputError::NonFinite)
        }
    }
    /// Parses an integer written in this locale; a decimal separator is
    /// refused.
    ///
    /// # Errors
    /// [`NumberInputError::InvalidNumber`] for text that does not follow the
    /// rules or does not fit `i64`.
    pub fn parse_integer(self, text: &str) -> Result<i64, NumberInputError> {
        self.canonical(text, false)?
            .parse()
            .map_err(|_| NumberInputError::InvalidNumber)
    }
    /// Rewrites Rust's plain number text (`-1234.5`) in this locale.
    fn localize(self, text: &str) -> String {
        let (sign, digits) = match text.strip_prefix('-') {
            Some(rest) => ("-", rest),
            None => ("", text),
        };
        let (integer, fraction) = match digits.split_once('.') {
            Some((integer, fraction)) => (integer, Some(fraction)),
            None => (digits, None),
        };
        let mut out = String::with_capacity(text.len() + integer.len());
        out.push_str(sign);
        for (index, digit) in integer.chars().enumerate() {
            if index > 0
                && (integer.len() - index) % 3 == 0
                && let Some(grouping) = self.grouping
            {
                out.push(grouping);
            }
            out.push(digit);
        }
        if let Some(fraction) = fraction {
            out.push(self.decimal);
            out.push_str(fraction);
        }
        out
    }
    /// Validates locale text and returns it in Rust's plain form.
    fn canonical(self, text: &str, decimal_allowed: bool) -> Result<String, NumberInputError> {
        let text = text.trim();
        let (negative, rest) = if let Some(rest) = text.strip_prefix('-') {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix('\u{2212}') {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix('+') {
            (false, rest)
        } else {
            (false, text)
        };
        let is_grouping = |character: char| {
            self.grouping.is_some_and(|grouping| {
                character == grouping || (is_space(grouping) && is_space(character))
            })
        };
        let (integer, fraction) = match rest.split_once(self.decimal) {
            Some(_) if !decimal_allowed => return Err(NumberInputError::InvalidNumber),
            Some((integer, fraction)) => (integer, Some(fraction)),
            None => (rest, None),
        };
        let groups: Vec<&str> = integer.split(is_grouping).collect();
        let grouped = groups.len() > 1;
        let mut canonical = String::with_capacity(text.len());
        if negative {
            canonical.push('-');
        }
        for (index, group) in groups.iter().enumerate() {
            let valid_length = if !grouped {
                true
            } else if index == 0 {
                (1..=3).contains(&group.len())
            } else {
                group.len() == 3
            };
            if !valid_length || !group.chars().all(|c| c.is_ascii_digit()) {
                return Err(NumberInputError::InvalidNumber);
            }
            canonical.push_str(group);
        }
        if let Some(fraction) = fraction {
            if !fraction.chars().all(|c| c.is_ascii_digit()) {
                return Err(NumberInputError::InvalidNumber);
            }
            canonical.push('.');
            canonical.push_str(fraction);
        }
        if !canonical.chars().any(|c| c.is_ascii_digit()) {
            return Err(NumberInputError::InvalidNumber);
        }
        Ok(canonical)
    }
}

/// The spaces accepted interchangeably for a space grouping separator.
fn is_space(character: char) -> bool {
    matches!(character, ' ' | '\u{a0}' | '\u{202f}')
}

impl NumberFormat<f64> {
    /// A decimal format written in `locale` with `places` fractional digits,
    /// or the shortest exact text when `places` is `None`, parsed with
    /// [`NumberLocale::parse_decimal`]. Editing text uses the same locale with
    /// the shortest exact text, so starting an edit never rounds the value.
    /// `width_hint` reserves the value slot as for [`Self::new`].
    pub fn localized_decimal(
        locale: NumberLocale,
        places: Option<u8>,
        width_hint: impl Into<String>,
    ) -> Self {
        Self::new(
            width_hint,
            move |value| locale.format_decimal(value, places),
            move |text| locale.parse_decimal(text),
        )
        .edit_text(move |value| locale.format_decimal(value, None))
    }
}

impl NumberFormat<i64> {
    /// An integer format written in `locale` with its grouping, parsed with
    /// [`NumberLocale::parse_integer`]. `width_hint` reserves the value slot as
    /// for [`Self::new`].
    pub fn localized_integer(locale: NumberLocale, width_hint: impl Into<String>) -> Self {
        Self::new(
            width_hint,
            move |value| locale.format_integer(value),
            move |text| locale.parse_integer(text),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_format_and_parse_round_trip() {
        let swedish = NumberLocale::new(',', Some(' ')).unwrap();
        for value in [0.0, -0.5, 1234.5, -1_234_567.125, 999.0, 1000.0] {
            let text = swedish.format_decimal(value, None);
            assert_eq!(swedish.parse_decimal(&text), Ok(value), "{text}");
        }
        assert_eq!(swedish.format_integer(-1_234_567), "-1 234 567");
        assert_eq!(swedish.parse_integer("\u{2212}1 234"), Ok(-1234));
        assert_eq!(swedish.parse_integer("1\u{202f}234"), Ok(1234));
        assert_eq!(swedish.format_decimal(1234.0, Some(2)), "1 234,00");
        let german = NumberLocale::new(',', Some('.')).unwrap();
        assert_eq!(german.parse_decimal("1.234,5"), Ok(1234.5));
        assert_eq!(german.format_decimal(1234.5, Some(1)), "1.234,5");
        assert_eq!(
            NumberLocale::default().format_decimal(1234.5, None),
            "1234.5"
        );
    }

    #[test]
    fn locales_refuse_ambiguous_text_and_separators() {
        let english = NumberLocale::new('.', Some(',')).unwrap();
        for text in [
            "12,34.5", "1,2345", ",123", "1.2.3", "1e3", "", "-", "1,234,", "inf", "1 234",
        ] {
            assert_eq!(
                english.parse_decimal(text),
                Err(NumberInputError::InvalidNumber),
                "{text}"
            );
        }
        assert_eq!(
            english.parse_integer("1.5"),
            Err(NumberInputError::InvalidNumber)
        );
        assert_eq!(english.parse_decimal(" +1,234.5 "), Ok(1234.5));
        assert_eq!(
            NumberLocale::new(',', Some(',')),
            Err(NumberLocaleError::SameSeparators)
        );
        // Grouping accepts every space, so no space may separate decimals
        // beside a space grouping: "1 234" would mean 1.234 and 1234.
        for (decimal, grouping) in [(' ', '\u{a0}'), ('\u{202f}', ' '), ('\u{a0}', '\u{202f}')] {
            assert_eq!(
                NumberLocale::new(decimal, Some(grouping)),
                Err(NumberLocaleError::SameSeparators),
                "{decimal:?} {grouping:?}"
            );
        }
        assert!(NumberLocale::new(',', Some('\u{a0}')).is_ok());
        assert_eq!(
            NumberLocale::new('5', None),
            Err(NumberLocaleError::InvalidSeparator('5'))
        );
        assert_eq!(
            NumberLocale::new('.', Some('-')),
            Err(NumberLocaleError::InvalidSeparator('-'))
        );
    }
}
