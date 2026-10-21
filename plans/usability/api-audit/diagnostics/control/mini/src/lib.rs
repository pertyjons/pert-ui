//! Probe of rustdoc's compile_fail error-code check.
//!
//! ```compile_fail,E0599
//! let x: u8 = "wrong type"; // E0308, not E0599
//! ```
pub struct OwnThing;
pub use inner::*;
/// Prelude.
pub mod prelude { pub use inner::Notifier; }
