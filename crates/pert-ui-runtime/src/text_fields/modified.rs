//! The modified report the built-in editors share: whether a draft differs
//! from the value its editor shows at rest, delivered once per change.
use super::ModifiedReport;
use crate::{EventContext, RuntimeError};
use std::cell::Cell;

/// An editor's `on_modified` callback and what it last delivered.
///
/// The owner asks [`Self::wants`] after each change with its current
/// modified state and, when it answers `true`, requests one delivery through
/// its notifier; the delivery calls [`Self::deliver`] with the state current
/// then. A change and its reversal before the delivery report nothing, and
/// requests made while one is pending fold into it, so each delivered change
/// is reported once and in order with the owner's other notifications.
#[derive(Default)]
pub(super) struct ModifiedReporter {
    callback: Option<ModifiedReport>,
    /// The state last delivered.
    reported: bool,
    /// A delivery is requested and not yet made.
    pending: Cell<bool>,
}

impl ModifiedReporter {
    /// Installs the callback, replacing an earlier one.
    pub(super) fn set(&mut self, callback: ModifiedReport) {
        self.callback = Some(callback);
    }
    /// Whether a callback is installed.
    pub(super) fn installed(&self) -> bool {
        self.callback.is_some()
    }
    /// Whether the owner should request a delivery for the modified state
    /// `dirty`: a callback is installed, no delivery is pending and the state
    /// differs from the one last delivered. Answering `true` records the
    /// request, so the owner must then request it.
    pub(super) fn wants(&self, dirty: bool) -> bool {
        if self.callback.is_some() && !self.pending.get() && dirty != self.reported {
            self.pending.set(true);
            true
        } else {
            false
        }
    }
    /// Makes a requested delivery with the current state `dirty`: calls the
    /// callback when the state differs from the one last delivered. Without a
    /// callback the state still counts as delivered.
    pub(super) fn deliver(
        &mut self,
        cx: &mut EventContext,
        dirty: bool,
    ) -> Result<(), RuntimeError> {
        self.pending.set(false);
        if dirty == self.reported {
            return Ok(());
        }
        self.reported = dirty;
        match &mut self.callback {
            Some(report) => report(cx, dirty),
            None => Ok(()),
        }
    }
    /// Applies a new description's callback, keeping what was delivered.
    pub(super) fn adopt(&mut self, next: Self) {
        self.callback = next.callback;
    }
}
