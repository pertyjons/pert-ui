//! The sealed set of item types a live-data endpoint accepts.

mod sealed {
    /// Crate-private conversions between an item and the 64-bit atomic that carries it.
    ///
    /// It seals [`LiveItem`](super::LiveItem). The distinct name keeps rustc
    /// from pointing at another crate's `Sealed` trait when an application
    /// tries to implement `LiveItem`, and the message below replaces the
    /// private path in that error.
    #[diagnostic::on_unimplemented(
        message = "`LiveItem` is sealed and cannot be implemented for `{Self}`",
        label = "not one of pert-ui's live-data item types",
        note = "only `f32`, `f64`, `i32`, `i64`, `u32` and `u64` can be published; map an application's own value onto one of them and keep its meaning on the host side"
    )]
    pub trait LiveItemSeal: Copy {
        /// Encodes the item so that `from_bits(to_bits(x)) == x` for every valid `x`.
        fn to_bits(self) -> u64;
        /// Decodes an item encoded by [`Self::to_bits`].
        fn from_bits(bits: u64) -> Self;
        /// Reports whether the producer may publish this item; false for a non-finite float.
        fn is_valid(self) -> bool;
    }
}

pub(crate) use sealed::LiveItemSeal;

/// A value type a live-data endpoint can publish: `f32`, `f64`, `i32`, `i64`, `u32` or `u64`.
///
/// The set is sealed and deliberately small. Every implementer fits one 64-bit
/// atomic, so a ring slot is a table of standard atomics and the producer path
/// is safe Rust from the standard library alone, with nothing to allocate and
/// nothing whose drop could free memory. Floats are validated on publish: a NaN
/// or infinity is rejected with [`PublishOutcome::Invalid`] and never enters
/// storage, which is what keeps a consumer's own minimum or maximum over a
/// window well defined. Integers need no validation.
///
/// A value carries no unit; a host maps its domain onto one of these types and
/// keeps the meaning on its side. A structured item type would need a different
/// storage protocol and is not offered.
///
/// `PartialOrd` is required so that a consumer can reduce a window it was
/// given, such as a meter holding the largest value it has not folded yet; the
/// library itself never compares two item values.
///
/// [`PublishOutcome::Invalid`]: super::PublishOutcome::Invalid
pub trait LiveItem:
    LiveItemSeal + Copy + PartialOrd + Send + Sync + core::fmt::Debug + 'static
{
}

impl LiveItemSeal for f32 {
    fn to_bits(self) -> u64 {
        u64::from(self.to_bits())
    }
    fn from_bits(bits: u64) -> Self {
        // A slot only ever holds bits this type wrote, so truncation is exact.
        Self::from_bits(bits as u32)
    }
    fn is_valid(self) -> bool {
        self.is_finite()
    }
}
impl LiveItem for f32 {}

impl LiveItemSeal for f64 {
    fn to_bits(self) -> u64 {
        self.to_bits()
    }
    fn from_bits(bits: u64) -> Self {
        Self::from_bits(bits)
    }
    fn is_valid(self) -> bool {
        self.is_finite()
    }
}
impl LiveItem for f64 {}

impl LiveItemSeal for i32 {
    fn to_bits(self) -> u64 {
        u64::from(self as u32)
    }
    fn from_bits(bits: u64) -> Self {
        bits as u32 as Self
    }
    fn is_valid(self) -> bool {
        true
    }
}
impl LiveItem for i32 {}

impl LiveItemSeal for i64 {
    fn to_bits(self) -> u64 {
        self as u64
    }
    fn from_bits(bits: u64) -> Self {
        bits as Self
    }
    fn is_valid(self) -> bool {
        true
    }
}
impl LiveItem for i64 {}

impl LiveItemSeal for u32 {
    fn to_bits(self) -> u64 {
        u64::from(self)
    }
    fn from_bits(bits: u64) -> Self {
        bits as Self
    }
    fn is_valid(self) -> bool {
        true
    }
}
impl LiveItem for u32 {}

impl LiveItemSeal for u64 {
    fn to_bits(self) -> u64 {
        self
    }
    fn from_bits(bits: u64) -> Self {
        bits
    }
    fn is_valid(self) -> bool {
        true
    }
}
impl LiveItem for u64 {}
