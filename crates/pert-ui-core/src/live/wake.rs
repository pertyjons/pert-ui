//! The coalescing flag behind a notified live-data source.

use super::sync::{AtomicBool, Ordering};

/// A pending flag that coalesces wake requests without losing one.
///
/// A non-real-time producer that wants the UI to drain after it published calls
/// [`Self::request`]; exactly one of any number of concurrent callers is told to
/// deliver the wake, the rest have coalesced into it. The UI side calls
/// [`Self::reset`] before it loads a drain frontier. Both operations are
/// acquire-release swaps, which is what makes the handshake exact: a producer
/// whose request the reset observed published before the reset, so its items
/// are inside the frontier loaded afterwards; a producer whose request came
/// later observes the reset and is told to wake again. A release-only reset
/// would not give this and could leave an item undelivered until an unrelated
/// wake.
///
/// The flag carries no data and knows nothing about how a wake is delivered;
/// the platform adapter owns that and, when delivery fails because the event
/// loop has exited, calls [`Self::reset`] so the failure cannot leave the flag
/// set and suppress every later wake. A real-time producer must not use this
/// path at all, because delivering a wake reaches the operating system; it uses
/// the polled schedule instead. The flag is `Send` and `Sync`, allocates
/// nothing and is the model-checked half of the notified protocol.
///
/// ```
/// use pert_ui_core::WakeFlag;
///
/// let flag = WakeFlag::new();
/// assert!(flag.request(), "the first request delivers the wake");
/// assert!(!flag.request(), "a second request before the reset coalesces");
/// assert!(flag.reset(), "the reset reports that a wake was pending");
/// assert!(flag.request(), "after the reset the next request delivers again");
/// ```
#[derive(Debug, Default)]
pub struct WakeFlag {
    pending: AtomicBool,
}

impl WakeFlag {
    /// Creates a flag with no wake pending.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending: AtomicBool::new(false),
        }
    }

    /// Requests a wake after publishing, returning whether this caller must deliver it.
    ///
    /// Call this after the publish that should be drained, never before it.
    /// `false` means an earlier request is still pending and will drain this
    /// publish too.
    #[must_use = "the caller that receives true must deliver the wake"]
    pub fn request(&self) -> bool {
        !self.pending.swap(true, Ordering::AcqRel)
    }

    /// Clears the flag before a drain, returning whether a wake was pending.
    ///
    /// Load the drain frontier only after this call returns. The adapter also
    /// calls this when it could not deliver a wake, so the flag never stays set
    /// for a wake that will not arrive.
    pub fn reset(&self) -> bool {
        self.pending.swap(false, Ordering::AcqRel)
    }

    /// Reports whether a wake is pending, for inspection; it synchronizes nothing.
    #[must_use]
    pub fn is_pending(&self) -> bool {
        self.pending.load(Ordering::Relaxed)
    }
}
