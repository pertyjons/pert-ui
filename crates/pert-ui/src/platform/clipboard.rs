//! Native clipboard adapter; the runtime only sees owned UTF-8 and typed errors.
//!
//! One service covers every supported desktop. On Linux the backend tries the
//! Wayland data-control protocol first and falls back to X11, because the
//! data-control protocols are optional: wlroots-based compositors and KDE provide
//! them, while GNOME's Mutter does not, and there the X11 path reaches the same
//! desktop clipboard through XWayland. Selecting the backend per session rather
//! than per display server keeps one code path and needs no unsafe interop with
//! the window's Wayland handles.
use crate::{Clipboard, ClipboardError};

/// Lazily connected platform clipboard.
///
/// The service is created on first use and then retained, because a text owner
/// must keep serving requests for as long as the application holds the selection.
/// Dropping it releases ownership, exactly as it does for any other client.
pub(super) struct NativeClipboard(Option<arboard::Clipboard>);

impl NativeClipboard {
    pub(super) fn new() -> Self {
        Self(None)
    }
    fn service(&mut self) -> Result<&mut arboard::Clipboard, ClipboardError> {
        if self.0.is_none() {
            // A failed connection is not retained, so the next use tries again.
            self.0 = Some(arboard::Clipboard::new().map_err(|error| connection_failure(&error))?);
        }
        self.0.as_mut().ok_or(ClipboardError::Unavailable)
    }
}

/// Classifies an arboard failure. Every class arboard distinguishes keeps its
/// own variant; its unclassified `Unknown` error, whose description arboard
/// documents as unfit for identifying a case, becomes `Failed` with that
/// description kept for diagnostics, and any other failure `Failed` with
/// arboard's message.
/// `ConversionFailure` on a read means the clipboard holds no text that
/// converts to UTF-8, which is what `NoText` states.
fn classify(error: &arboard::Error, reading: bool) -> ClipboardError {
    match error {
        arboard::Error::ContentNotAvailable => ClipboardError::NoText,
        arboard::Error::ConversionFailure if reading => ClipboardError::NoText,
        arboard::Error::ClipboardNotSupported => ClipboardError::Unavailable,
        arboard::Error::ClipboardOccupied => ClipboardError::Busy,
        arboard::Error::Unknown { description } => ClipboardError::Failed(description.clone()),
        // Oversized payloads are rejected by the editor's own limit, not here.
        error => ClipboardError::Failed(error.to_string()),
    }
}

/// Classifies a failure to connect to the platform clipboard. A missing service
/// or protocol is `Unavailable`; any other cause, such as an X11 or Wayland
/// connection error, keeps its description in `Failed` rather than being
/// reported as a bare `Unavailable`.
fn connection_failure(error: &arboard::Error) -> ClipboardError {
    classify(error, false)
}

impl Clipboard for NativeClipboard {
    fn read_text(&mut self) -> Result<String, ClipboardError> {
        self.service()?
            .get_text()
            .map_err(|error| classify(&error, true))
    }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        self.service()?
            .set_text(text)
            .map_err(|error| classify(&error, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arboard_failures_keep_their_class() {
        let unknown = || arboard::Error::Unknown {
            description: "X11 server connection timed out".into(),
        };
        for (error, reading, expected) in [
            (
                arboard::Error::ContentNotAvailable,
                true,
                ClipboardError::NoText,
            ),
            (
                arboard::Error::ConversionFailure,
                true,
                ClipboardError::NoText,
            ),
            (
                arboard::Error::ConversionFailure,
                false,
                ClipboardError::Failed(arboard::Error::ConversionFailure.to_string()),
            ),
            (
                arboard::Error::ClipboardNotSupported,
                false,
                ClipboardError::Unavailable,
            ),
            (
                arboard::Error::ClipboardOccupied,
                true,
                ClipboardError::Busy,
            ),
            (
                arboard::Error::ClipboardOccupied,
                false,
                ClipboardError::Busy,
            ),
            (
                unknown(),
                true,
                ClipboardError::Failed("X11 server connection timed out".into()),
            ),
        ] {
            assert_eq!(classify(&error, reading), expected, "{error:?}");
        }
    }

    #[test]
    fn connection_failures_keep_their_cause() {
        assert_eq!(
            connection_failure(&arboard::Error::ClipboardNotSupported),
            ClipboardError::Unavailable
        );
        assert_eq!(
            connection_failure(&arboard::Error::Unknown {
                description: "could not connect to the X11 server".into(),
            }),
            ClipboardError::Failed("could not connect to the X11 server".into())
        );
    }

    /// Round-trips the real desktop clipboard through the installed adapter.
    ///
    /// Opt-in, because it needs a desktop session and takes ownership of the
    /// user's clipboard for the duration of the test. It is the only check that
    /// distinguishes a working backend from one whose protocol the compositor
    /// does not implement, which compiles and silently does nothing.
    #[test]
    #[ignore = "requires a native desktop session and takes over the clipboard"]
    fn native_clipboard_round_trips_text() {
        let mut clipboard = NativeClipboard::new();
        let payload = "pert-ui clipboard round trip";
        clipboard
            .write_text(payload)
            .expect("the desktop clipboard must accept text");
        assert_eq!(
            clipboard
                .read_text()
                .expect("written text must be readable back"),
            payload
        );
    }
}
