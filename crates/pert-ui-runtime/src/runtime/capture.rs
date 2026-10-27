//! Admission, resolution and completion of rendered-frame captures.
use super::*;

/// One admitted capture, waiting for the host to render its window.
pub(super) struct PendingCapture {
    id: u64,
    pub(super) window: u64,
    /// The incarnation admitted for a component or path target; a
    /// replacement at the same path is not it.
    component: Option<u64>,
    request: crate::CaptureRequest,
    /// The error of the flush that admitted it, carried into the reply.
    flush_error: Option<crate::RuntimeError>,
    handle: crate::inspection_service::ReplyHandle<Result<crate::Capture, crate::CaptureError>>,
    /// This request's identity, which every job listed for it shares.
    token: std::sync::Arc<()>,
}

/// Where a capture target lies, resolved from the current state.
struct CaptureRegion {
    window: u64,
    component: Option<u64>,
    /// Logical window coordinates, the visible intersection.
    region: crate::Rect,
    clipped: bool,
    viewport: crate::Viewport,
}

impl Runtime {
    /// Declares whether this host renders capture jobs.
    ///
    /// Off by default, so a headless runtime answers every
    /// [`CaptureRequest`](crate::CaptureRequest) with
    /// [`CaptureError::Unsupported`](crate::CaptureError::Unsupported) at
    /// once. The native `App` turns it on; a custom host that turns it on
    /// commits to completing every job
    /// [`pending_captures`](crate::host::HostIntegration::pending_captures)
    /// lists, with a frame or with an error.
    pub fn set_capture_support(&self, supported: bool) {
        self.shared.capture_support.set(supported);
    }

    // Documented on `crate::host::HostIntegration::captures_supported`.
    #[must_use]
    pub(crate) fn captures_supported(&self) -> bool {
        self.shared.capture_support.get()
    }

    /// Asks for a screenshot from the UI thread; the reply arrives once the
    /// host has rendered the frame, or at once with the error that refused
    /// the request. Never wait for it on the UI thread: the frame is
    /// rendered between this thread's flushes. From another thread, use
    /// [`InspectionClient::capture`](crate::InspectionClient::capture).
    ///
    /// Admission checks that a host renders captures and that the target is
    /// mounted, visible and measured now; the geometry is taken again from
    /// the state the rendered frame shows, and a target that disappeared by
    /// then is reported rather than replaced. An admitted request marks the
    /// target's window for one repaint, which is the frame the host renders
    /// the capture from; nothing polls.
    pub fn request_capture(
        &self,
        request: crate::CaptureRequest,
    ) -> crate::Pending<Result<crate::Capture, crate::CaptureError>> {
        let (handle, pending) = crate::inspection_service::ReplyHandle::new(None);
        self.admit_capture(request, handle, None);
        pending
    }

    /// The most captures that may wait for the host at once; a further
    /// request is refused as [`CaptureError::Busy`](crate::CaptureError::Busy).
    /// Bounds the frames a host is asked to render and the images held.
    pub const MAX_PENDING_CAPTURES: usize = 16;

    pub(crate) fn admit_capture(
        &self,
        request: crate::CaptureRequest,
        handle: crate::inspection_service::ReplyHandle<Result<crate::Capture, crate::CaptureError>>,
        flush_error: Option<&crate::RuntimeError>,
    ) {
        // A closed runtime has no host left to render; the dropped handle
        // reports cancellation, as shutdown does for what was already queued.
        if self.shared.closed.get() {
            return;
        }
        if !self.shared.capture_support.get() {
            handle.answer(Err(crate::CaptureError::Unsupported), flush_error);
            return;
        }
        self.prune_cancelled_captures();
        if self.shared.captures.borrow().len() >= Self::MAX_PENDING_CAPTURES {
            handle.answer(Err(crate::CaptureError::Busy), flush_error);
            return;
        }
        let region = match self.resolve_capture(request.target()) {
            Ok(region) => region,
            Err(error) => {
                handle.answer(Err(error), flush_error);
                return;
            }
        };
        let extent = Self::frame_extent(&region.viewport);
        let bytes = u64::from(extent.width())
            .checked_mul(u64::from(extent.height()))
            .and_then(|pixels| pixels.checked_mul(4));
        if bytes.is_none_or(|bytes| bytes > crate::FrameImage::MAX_BYTES as u64) {
            let limit = crate::LimitError::FrameSize {
                width: extent.width(),
                height: extent.height(),
            };
            handle.answer(Err(crate::CaptureError::Limit(limit)), flush_error);
            return;
        }
        let id = self.shared.next_capture.get();
        self.shared.next_capture.set(id.saturating_add(1));
        for window in self
            .shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
        {
            if window.serial == region.window {
                window.scene_dirty.set(true);
            }
        }
        self.shared.captures.borrow_mut().push(PendingCapture {
            id,
            window: region.window,
            component: region.component,
            request,
            flush_error: flush_error.cloned(),
            handle,
            token: std::sync::Arc::new(()),
        });
    }

    /// The physical size a frame of `viewport` has.
    fn frame_extent(viewport: &crate::Viewport) -> crate::PhysicalExtent {
        viewport.frame_extent()
    }

    /// Drops captures whose service closed; each dropped handle cancels its
    /// waiter.
    fn prune_cancelled_captures(&self) {
        self.shared
            .captures
            .borrow_mut()
            .retain(|capture| !capture.handle.is_cancelled());
    }

    /// Resolves a target against the current state: the window, the
    /// component and its visible logical region.
    fn resolve_capture(
        &self,
        target: &crate::CaptureTarget,
    ) -> Result<CaptureRegion, crate::CaptureError> {
        let mounted: Vec<u64> = self
            .native_windows()
            .iter()
            .map(crate::NativeWindowId::id)
            .collect();
        let windows: Vec<_> = self
            .window_inspections()
            .into_iter()
            .filter(|window| mounted.contains(&window.id))
            .collect();
        let serial = match target {
            crate::CaptureTarget::Window(window) => {
                let window = windows
                    .iter()
                    .find(|candidate| candidate.id == *window)
                    .ok_or(crate::CaptureError::UnknownTarget)?;
                if !window.presentable {
                    return Err(crate::CaptureError::Hidden);
                }
                let viewport = window.viewport;
                let region = crate::Rect::new(0.0, 0.0, viewport.width(), viewport.height())
                    .map_err(|_| crate::CaptureError::Unmeasured)?;
                if region.values()[2] <= 0.0 || region.values()[3] <= 0.0 {
                    return Err(crate::CaptureError::Hidden);
                }
                return Ok(CaptureRegion {
                    window: window.id,
                    component: None,
                    region,
                    clipped: false,
                    viewport,
                });
            }
            crate::CaptureTarget::Region {
                window,
                x,
                y,
                width,
                height,
            } => {
                let window = windows
                    .iter()
                    .find(|candidate| candidate.id == *window)
                    .ok_or(crate::CaptureError::UnknownTarget)?;
                // A malformed request is invalid wherever it points.
                let requested = crate::Rect::new(*x, *y, *width, *height)
                    .map_err(|_| crate::CaptureError::InvalidRegion)?;
                if !window.presentable {
                    return Err(crate::CaptureError::Hidden);
                }
                let viewport = window.viewport;
                let frame = crate::Rect::new(0.0, 0.0, viewport.width(), viewport.height())
                    .map_err(|_| crate::CaptureError::Unmeasured)?;
                let region = requested.intersection(frame);
                let [_, _, shown_width, shown_height] = region.values();
                if shown_width <= 0.0 || shown_height <= 0.0 {
                    return Err(crate::CaptureError::Hidden);
                }
                return Ok(CaptureRegion {
                    window: window.id,
                    component: None,
                    clipped: region != requested,
                    region,
                    viewport,
                });
            }
            crate::CaptureTarget::Component(serial) => *serial,
            crate::CaptureTarget::Path(path) => {
                let mut matches = self.resolve_selector(&crate::Selector::Path(path.clone()), None);
                match (matches.pop(), matches.is_empty()) {
                    (Some(serial), true) => serial,
                    _ => return Err(crate::CaptureError::UnknownTarget),
                }
            }
        };
        let entry = self
            .entries
            .get(&serial)
            .ok_or(crate::CaptureError::UnknownTarget)?;
        let window = entry.node.window();
        let presentable = windows
            .iter()
            .find(|candidate| candidate.id == window.serial)
            .is_some_and(|candidate| candidate.presentable);
        if !presentable || !entry.node.visible.get() {
            return Err(crate::CaptureError::Hidden);
        }
        let bounds = entry
            .node
            .bounds
            .get()
            .ok_or(crate::CaptureError::Unmeasured)?;
        let viewport = window.viewport.get();
        let frame = crate::Rect::new(0.0, 0.0, viewport.width(), viewport.height())
            .map_err(|_| crate::CaptureError::Unmeasured)?;
        let mut region = bounds.intersection(frame);
        if let Some(clip) = entry.node.clip.get() {
            region = region.intersection(clip);
        }
        let [_, _, width, height] = region.values();
        if width <= 0.0 || height <= 0.0 {
            return Err(crate::CaptureError::Hidden);
        }
        Ok(CaptureRegion {
            window: window.serial,
            component: Some(serial),
            region,
            clipped: region != bounds,
            viewport,
        })
    }

    // Documented on `crate::host::HostIntegration::pending_captures`.
    #[must_use]
    pub(crate) fn pending_captures(&self) -> Vec<crate::CaptureJob> {
        self.prune_cancelled_captures();
        self.shared
            .captures
            .borrow()
            .iter()
            .map(|capture| crate::CaptureJob {
                id: capture.id,
                window: capture.window,
                include_overlay: capture.request.overlay_included(),
                request: capture.token.clone(),
            })
            .collect()
    }

    // Documented on `crate::host::HostIntegration::capture_scene`.
    pub(crate) fn capture_scene(
        &mut self,
        job: &crate::CaptureJob,
    ) -> Result<crate::DisplayList, crate::CaptureError> {
        // A job is trusted only as a reference to a pending request, by the
        // request's identity: another runtime's job can repeat the id, the
        // window and the overlay flag.
        let pending = self
            .shared
            .captures
            .borrow()
            .iter()
            .find(|capture| std::sync::Arc::ptr_eq(&capture.token, &job.request))
            .map(|capture| (capture.window, capture.request.overlay_included()));
        let (window, include_overlay) = pending.ok_or(crate::CaptureError::UnknownTarget)?;
        let window = self
            .native_windows()
            .into_iter()
            .find(|candidate| candidate.id() == window)
            .ok_or(crate::CaptureError::UnknownTarget)?;
        self.select_window(&window)
            .map_err(crate::CaptureError::from)?;
        self.paint_scene(include_overlay)
            .map_err(crate::CaptureError::from)
    }

    // Documented on `crate::host::HostIntegration::complete_capture`.
    pub(crate) fn complete_capture(
        &self,
        job: &crate::CaptureJob,
        frame: Result<crate::FrameImage, crate::CaptureError>,
    ) -> bool {
        let position = self
            .shared
            .captures
            .borrow()
            .iter()
            .position(|capture| std::sync::Arc::ptr_eq(&capture.token, &job.request));
        let Some(position) = position else {
            return false;
        };
        let capture = self.shared.captures.borrow_mut().remove(position);
        if capture.handle.is_cancelled() {
            // Its service closed; dropping the handle reports cancellation.
            return true;
        }
        let result = frame.and_then(|frame| self.finish_capture(&capture, frame));
        capture.handle.answer(result, capture.flush_error.as_ref());
        true
    }

    fn finish_capture(
        &self,
        capture: &PendingCapture,
        frame: crate::FrameImage,
    ) -> Result<crate::Capture, crate::CaptureError> {
        let resolved = self.resolve_capture(capture.request.target())?;
        if resolved.window != capture.window || resolved.component != capture.component {
            return Err(crate::CaptureError::UnknownTarget);
        }
        let scale = resolved.viewport.scale();
        // The frame must be the window at its physical extent, or the crop
        // would show other pixels than the region claims.
        let expected = Self::frame_extent(&resolved.viewport);
        if (frame.width(), frame.height()) != (expected.width(), expected.height()) {
            return Err(crate::FrameError::ExtentMismatch {
                frame_width: frame.width(),
                frame_height: frame.height(),
                window_width: expected.width(),
                window_height: expected.height(),
            }
            .into());
        }
        let [x, y, width, height] = resolved.region.values();
        // Logical coordinates map onto the frame as renderers map them: each
        // axis by its physical extent over its logical size, which differs
        // slightly from `scale` when a host supplied the extent.
        let scale_x = f64::from(expected.width()) / f64::from(resolved.viewport.width());
        let scale_y = f64::from(expected.height()) / f64::from(resolved.viewport.height());
        // Outward to the pixel grid, then inside the frame.
        let left = (f64::from(x) * scale_x).floor().max(0.0) as u32;
        let top = (f64::from(y) * scale_y).floor().max(0.0) as u32;
        let right = ((f64::from(x + width) * scale_x).ceil().max(0.0) as u32).min(frame.width());
        let bottom = ((f64::from(y + height) * scale_y).ceil().max(0.0) as u32).min(frame.height());
        let image = frame.crop(
            left.min(frame.width()),
            top.min(frame.height()),
            right.saturating_sub(left),
            bottom.saturating_sub(top),
        )?;
        Ok(crate::Capture {
            target: capture.request.target().clone(),
            window: resolved.window,
            component: resolved.component,
            region: resolved.region.values(),
            clipped: resolved.clipped,
            scale,
            at: self.shared.now.get(),
            overlay: capture.request.overlay_included()
                && cfg!(debug_assertions)
                && self.shared.overlay.get() != crate::DebugOverlay::Off,
            image,
        })
    }
}
