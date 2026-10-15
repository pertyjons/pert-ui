//! Keeping the native titles of detached views in step with their titles.

use crate::{
    EventContext, NativeWindowId, PresentationReceipt, PresentationStatus, RuntimeError,
    UpdateContext, WindowError,
};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// How long a container waits before asking again for a retitle that a full
/// request queue refused.
const FULL_QUEUE_RETRY: Duration = Duration::from_millis(250);

/// What one detached view's window is known to show.
struct Shown {
    window: NativeWindowId,
    /// The title the window opened with or was last asked to show; empty when
    /// unknown, so the next update asks again.
    title: String,
    /// The last retitle request, observed so a request the host dropped is
    /// asked again.
    receipt: Option<PresentationReceipt>,
    /// A title the host refused to show, which is not asked for again until
    /// the view's title changes.
    refused: Option<String>,
}

/// A container's record of its detached views' native titles, keyed by the
/// view's key: it compares each view's current title with what its window
/// shows, starting from the title the window opened with, and asks for a
/// retitle with [`EventContext::set_window_title`] from the event phase.
#[derive(Default)]
pub(crate) struct WindowTitles {
    shown: BTreeMap<String, Shown>,
    /// Retitles found in an update and requested at the next delivery.
    pending: Vec<(String, NativeWindowId, String)>,
    /// Whether a full request queue refused a retitle, so the next update
    /// schedules another attempt.
    retry: bool,
    /// Until when no retitle is requested after a full queue, so the
    /// container does not ask again within the same flush.
    hold_until: Option<Duration>,
}

impl WindowTitles {
    /// Compares the detached `views`, as (key, window, current title), with
    /// what their windows show, in the container's update. Returns whether a
    /// retitle waits for delivery, for which the caller requests a
    /// notification. Views no longer listed are forgotten. Each request's
    /// status is subscribed to here: a request the host dropped unfinished
    /// is asked for again, while a title the host refused with another
    /// failure is asked for again only once the view's title changes. After
    /// a full request queue refused a retitle, the update wakes again after
    /// a short delay to ask once more.
    ///
    /// # Errors
    /// The runtime error of reading a request's status.
    pub(crate) fn update(
        &mut self,
        cx: &mut UpdateContext,
        views: impl IntoIterator<Item = (String, NativeWindowId, String)>,
    ) -> Result<bool, RuntimeError> {
        if std::mem::take(&mut self.retry) {
            self.hold_until = Some(cx.now().saturating_add(FULL_QUEUE_RETRY));
        }
        // Every update replaces the component's wake, so one that comes
        // while holding asks for the rest of the delay again.
        let holding = match self.hold_until {
            Some(until) if cx.now() < until => {
                cx.wake_after(until - cx.now())?;
                true
            }
            _ => {
                self.hold_until = None;
                false
            }
        };
        let mut listed = BTreeSet::new();
        for (key, window, title) in views {
            listed.insert(key.clone());
            let shown = self
                .shown
                .entry(key.clone())
                .and_modify(|shown| {
                    if shown.window != window {
                        // Another window under the key starts from its own
                        // opening title.
                        *shown = Shown::opened(&window);
                    }
                })
                .or_insert_with(|| Shown::opened(&window));
            if let Some(receipt) = &shown.receipt {
                match cx.read_paint(&receipt.status_binding())? {
                    // Dropped unfinished: ask again.
                    PresentationStatus::Failed(WindowError::Cancelled) => {
                        shown.title.clear();
                        shown.receipt = None;
                    }
                    // Refused: the window shows something else, and this
                    // title is not asked for again.
                    PresentationStatus::Failed(_) => {
                        shown.refused = Some(std::mem::take(&mut shown.title));
                        shown.receipt = None;
                    }
                    _ => {}
                }
            }
            if holding
                || title.trim().is_empty()
                || shown.title == title
                || shown.refused.as_deref() == Some(title.as_str())
            {
                continue;
            }
            shown.title.clone_from(&title);
            shown.refused = None;
            match self.pending.iter_mut().find(|(queued, ..)| *queued == key) {
                Some(queued) => {
                    // The view may have moved to another window meanwhile.
                    queued.1 = window;
                    queued.2 = title;
                }
                None => self.pending.push((key, window, title)),
            }
        }
        self.shown.retain(|key, _| listed.contains(key));
        self.pending.retain(|(key, ..)| listed.contains(key));
        Ok(!self.pending.is_empty())
    }

    /// Requests the pending retitles, in the container's delivery. A window
    /// that closed meanwhile needs none; a full request queue leaves the title
    /// unknown and updates the container, which wakes again shortly to ask
    /// once more.
    ///
    /// # Errors
    /// Any other refusal of [`EventContext::set_window_title`], after keeping
    /// the retitles not yet requested.
    pub(crate) fn deliver(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let mut pending = std::mem::take(&mut self.pending).into_iter();
        while let Some((key, window, title)) = pending.next() {
            match cx.set_window_title(&window, title) {
                Ok(receipt) => {
                    if let Some(shown) = self.shown.get_mut(&key) {
                        shown.receipt = Some(receipt);
                        // The next update subscribes to the request's status.
                        cx.invalidate(crate::Invalidation::Paint);
                    }
                }
                Err(RuntimeError::Unmounted) => {}
                Err(error) => {
                    if let Some(shown) = self.shown.get_mut(&key) {
                        shown.title.clear();
                        shown.receipt = None;
                    }
                    if error == RuntimeError::WindowRequestFull {
                        self.retry = true;
                        cx.invalidate(crate::Invalidation::Paint);
                    } else {
                        self.pending.extend(pending);
                        return Err(error);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Shown {
    fn opened(window: &NativeWindowId) -> Self {
        Self {
            window: window.clone(),
            title: window.opening_title().unwrap_or_default(),
            receipt: None,
            refused: None,
        }
    }
}
