//! Native window presentation: selecting a window's state, detach, re-dock and
//! activation requests, and close admission.
use super::*;

impl Runner {
    pub(super) fn select_native(&mut self, id: WindowId) -> Result<bool, AppError> {
        if self.current.window.as_ref().is_some_and(|w| w.id() == id) {
            if !self.app.runtime.contains_window(&self.current.root) {
                return Ok(false);
            }
            self.app.runtime.select_window(&self.current.root)?;
            return Ok(true);
        }
        let Some(next) = self.parked.remove(&id) else {
            return Ok(false);
        };
        if !self.app.runtime.contains_window(&next.root) {
            return Ok(false);
        }
        let previous = std::mem::replace(&mut self.current, next);
        if let Some(window) = &previous.window {
            self.parked.insert(window.id(), previous);
        }
        self.app.runtime.select_window(&self.current.root)?;
        Ok(true)
    }
    fn select_root(&mut self, root: &crate::NativeWindowId) -> Result<(), AppError> {
        let native = if self.current.root == *root {
            self.current.window.as_ref().map(|w| w.id())
        } else {
            self.parked
                .iter()
                .find(|(_, w)| w.root == *root)
                .map(|(id, _)| *id)
        }
        .ok_or(RuntimeError::Unmounted)?;
        if !self.select_native(native)? {
            return Err(RuntimeError::Unmounted.into());
        }
        Ok(())
    }
    pub(super) fn retire_windows(&mut self) -> Result<(), AppError> {
        if !self.app.runtime.contains_window(&self.current.root) {
            self.select_root(&self.app.runtime.primary_window())?;
        }
        self.parked
            .retain(|_, window| self.app.runtime.contains_window(&window.root));
        Ok(())
    }
    fn presentation_action(
        &mut self,
        event_loop: &ActiveEventLoop,
        action: &crate::PresentationAction,
    ) -> Result<crate::PresentationStatus, crate::WindowError> {
        use crate::{PresentationAction, PresentationStatus, WindowError};
        match action {
            PresentationAction::Detach { view, options } => {
                let source = self.app.runtime.window_of(view)?;
                self.app.runtime.select_window(&source)?;
                // A release may select a previously inactive tab before requesting
                // native movement. Apply that retained visibility transition first.
                self.app.runtime.flush(self.start.elapsed())?;
                let prepared = self.app.runtime.prepare_detach(view, options.clone())?;
                let size = options.initial_size();
                let window = Arc::new(
                    event_loop
                        .create_window(
                            Window::default_attributes()
                                .with_title(options.title())
                                .with_inner_size(LogicalSize::new(size.width(), size.height())),
                        )
                        .map_err(|error| WindowError::Creation(error.to_string()))?,
                );
                let size = window.inner_size();
                let renderer = self
                    .current
                    .renderer
                    .as_ref()
                    .ok_or_else(|| WindowError::Surface("GPU is suspended".into()))?
                    .new_surface(window.clone(), size.width, size.height)
                    .map_err(|error| WindowError::Surface(error.to_string()))?;
                let scale = window.scale_factor();
                let viewport =
                    Viewport::from_physical(PhysicalExtent::new(size.width, size.height), scale)?;
                let root = self.app.runtime.commit_detach(prepared)?;
                self.app.runtime.set_viewport(viewport)?;
                let mut native = NativeState::new(root.clone());
                native.size = size;
                native.window = Some(window.clone());
                native.renderer = Some(renderer);
                self.parked.insert(window.id(), native);
                Ok(PresentationStatus::Presented(root))
            }
            PresentationAction::Redock(root) => {
                self.app.runtime.redock(root)?;
                Ok(PresentationStatus::Presented(
                    self.app.runtime.current_window(),
                ))
            }
            PresentationAction::Close(_) => Err(RuntimeError::StalePresentation.into()),
            PresentationAction::Retitle { window, title } => {
                let native = if self.current.root == *window {
                    self.current.window.as_ref()
                } else {
                    self.parked
                        .values()
                        .find(|native| native.root == *window)
                        .and_then(|native| native.window.as_ref())
                }
                .ok_or(RuntimeError::Unmounted)?;
                if !self.app.runtime.contains_window(window) {
                    return Err(RuntimeError::Unmounted.into());
                }
                native.set_title(title);
                Ok(PresentationStatus::Presented(window.clone()))
            }
            PresentationAction::Activate(root) => {
                let native = if self.current.root == *root {
                    self.current.window.as_ref()
                } else {
                    self.parked
                        .values()
                        .find(|native| native.root == *root)
                        .and_then(|native| native.window.as_ref())
                }
                .ok_or(RuntimeError::Unmounted)?;
                if !self.app.runtime.contains_window(root) {
                    return Err(RuntimeError::Unmounted.into());
                }
                #[cfg(target_os = "linux")]
                {
                    let _ = native;
                    Ok(PresentationStatus::ActivationUnsupported(root.clone()))
                }
                #[cfg(not(target_os = "linux"))]
                {
                    if native.is_minimized() == Some(true) {
                        native.set_minimized(false);
                    }
                    native.focus_window();
                    Ok(PresentationStatus::ActivationRequested(root.clone()))
                }
            }
        }
    }
    pub(super) fn presentation_requests(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), AppError> {
        for request in self.app.runtime.take_presentation_requests()? {
            if let crate::PresentationAction::Close(root) = request.action() {
                self.begin_close(event_loop, root.clone(), Some(request))?;
                continue;
            }
            let status = self
                .presentation_action(event_loop, request.action())
                .unwrap_or_else(crate::PresentationStatus::Failed);
            if let crate::PresentationStatus::Failed(error) = &status {
                self.app.handle_window_error(error)?;
            }
            request.complete(status)?;
            self.retire_windows()?;
            self.app.runtime.select_window(&self.current.root)?;
        }
        Ok(())
    }
}

/// The failure a presentation request reports for an error the host met
/// while completing it. Typed runtime and drawing causes stay typed; strings
/// carry only native window-system and GPU causes. An error unrelated to
/// windows (the host's own failure, which ends the application) cancels the
/// request, as shutdown does.
fn window_failure(error: &AppError) -> crate::WindowError {
    use crate::WindowError;
    match error {
        AppError::Runtime(error) => WindowError::Runtime(error.clone()),
        AppError::Drawing(error) => WindowError::from(error.clone()),
        AppError::Window(message) => WindowError::Creation(message.clone()),
        AppError::UnsupportedGraphics(message)
        | AppError::Graphics(message)
        | AppError::Surface(message)
        | AppError::DeviceLost(message) => WindowError::Surface(message.clone()),
        // The runtime cannot name the renderer's limit type; its text is the cause.
        AppError::Limit(limit) => WindowError::Surface(limit.to_string()),
        AppError::InvalidTitle
        | AppError::EventLoop(_)
        | AppError::ErrorReporting { .. }
        | AppError::WindowErrorReporting { .. }
        | AppError::Shutdown(_) => WindowError::Cancelled,
    }
}

pub(super) struct PendingClose {
    window: crate::NativeWindowId,
    decision: crate::ReadSignal<Option<bool>>,
    requests: Vec<crate::PresentationRequest>,
}
impl Runner {
    fn apply_close(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: &crate::NativeWindowId,
        accepted: bool,
    ) -> Result<crate::PresentationStatus, AppError> {
        if !accepted {
            return Ok(crate::PresentationStatus::CloseRejected);
        }
        self.select_root(window)?;
        if *window == self.app.runtime.primary_window() {
            event_loop.exit();
            Ok(crate::PresentationStatus::Closed)
        } else {
            let status = self.app.runtime.close_detached_window()?;
            self.retire_windows()?;
            Ok(status)
        }
    }
    pub(super) fn begin_close(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: crate::NativeWindowId,
        request: Option<crate::PresentationRequest>,
    ) -> Result<(), AppError> {
        if let Some(pending) = self
            .closing
            .iter_mut()
            .find(|pending| pending.window == window)
        {
            if let Some(request) = request {
                if pending.requests.len() >= crate::Runtime::MAX_WINDOW_REQUESTS {
                    request.complete(crate::PresentationStatus::Failed(
                        RuntimeError::WindowRequestFull.into(),
                    ))?;
                } else {
                    pending.requests.push(request);
                }
            }
            return Ok(());
        }
        // A close queued for a window an earlier request already closed is a
        // failed presentation request, reported as other requests' failures
        // are, not a component failure for error recovery.
        if !self.app.runtime.contains_window(&window) {
            if let Some(request) = request {
                let failure = crate::WindowError::Runtime(RuntimeError::Unmounted);
                self.app.handle_window_error(&failure)?;
                request.complete(crate::PresentationStatus::Failed(failure))?;
            }
            return Ok(());
        }
        let admission = self
            .select_root(&window)
            .and_then(|_| self.app.handle_close(&window).map_err(AppError::from));
        let admission = match admission {
            Ok(action) => action,
            Err(error) => {
                return Self::finish_close_requests(request.into_iter().collect(), Err(error));
            }
        };
        let accepted = match admission {
            crate::NativeCloseAction::Close => true,
            crate::NativeCloseAction::KeepOpen => false,
            crate::NativeCloseAction::Defer(decision) => {
                self.app.runtime.read_signal(&decision)?;
                self.closing.push(PendingClose {
                    window,
                    decision,
                    requests: request.into_iter().collect(),
                });
                return Ok(());
            }
        };
        let result = self.apply_close(event_loop, &window, accepted);
        Self::finish_close_requests(request.into_iter().collect(), result)
    }
    fn finish_close_requests(
        requests: Vec<crate::PresentationRequest>,
        result: Result<crate::PresentationStatus, AppError>,
    ) -> Result<(), AppError> {
        match result {
            Ok(status) => {
                for request in requests {
                    request.complete(status.clone())?;
                }
                Ok(())
            }
            Err(error) => {
                let failure = window_failure(&error);
                for request in requests {
                    request.complete(crate::PresentationStatus::Failed(failure.clone()))?;
                }
                Err(error)
            }
        }
    }
    pub(super) fn complete_closes(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        let mut index = 0;
        while index < self.closing.len() {
            if !self
                .app
                .runtime
                .contains_window(&self.closing[index].window)
            {
                self.closing.remove(index);
                continue;
            }
            let Some(accepted) = self
                .app
                .runtime
                .read_signal(&self.closing[index].decision)?
            else {
                index += 1;
                continue;
            };
            let pending = self.closing.remove(index);
            let result = self.apply_close(event_loop, &pending.window, accepted);
            Self::finish_close_requests(pending.requests, result)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_and_detach_failures_keep_their_typed_cause() {
        use crate::WindowError;
        assert_eq!(
            window_failure(&AppError::Runtime(RuntimeError::Unmounted)),
            WindowError::Runtime(RuntimeError::Unmounted)
        );
        assert_eq!(
            window_failure(&AppError::Drawing(crate::DrawError::InvalidColor)),
            WindowError::Drawing(crate::DrawError::InvalidColor)
        );
        assert_eq!(
            window_failure(&AppError::Window("no display".into())),
            WindowError::Creation("no display".into())
        );
        for error in [
            AppError::Surface("lost".into()),
            AppError::Graphics("lost".into()),
            AppError::DeviceLost("lost".into()),
        ] {
            assert_eq!(window_failure(&error), WindowError::Surface("lost".into()));
        }
        assert_eq!(
            window_failure(&AppError::EventLoop("gone".into())),
            WindowError::Cancelled
        );
    }
}
