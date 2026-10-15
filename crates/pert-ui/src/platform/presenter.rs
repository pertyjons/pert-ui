//! The presentation seam: how a native window turns scenes into pixels. The
//! default is the GPU renderer; with the `cpu` feature an application can opt
//! in to frames rendered by `pert_ui_cpu::CpuRenderer` and presented through
//! softbuffer, which creates no GPU device. Backend types stay in this module
//! and in `pert-ui-wgpu`; both paths report `RenderError` so the host's
//! recovery and error policy are the same.
use pert_ui_runtime::{DisplayList, FrameImage};
use pert_ui_wgpu::{Frame, Notice, RenderError, Renderer};
use std::sync::Arc;
use winit::window::Window;

/// One window's presenter.
pub(super) enum Presenter {
    /// The hardware GPU renderer, boxed so a software presenter does not
    /// carry its size.
    Gpu(Box<Renderer>),
    /// CPU frames presented through softbuffer.
    #[cfg(feature = "cpu")]
    Software(Box<software::Software>),
}

impl Presenter {
    /// The first window's presenter: software when the application chose it,
    /// otherwise a new GPU device whose notices go to `notify`.
    pub(super) fn new(
        software: bool,
        window: Arc<Window>,
        width: u32,
        height: u32,
        notify: impl Fn(Notice) + Send + Sync + 'static,
    ) -> Result<Self, RenderError> {
        #[cfg(feature = "cpu")]
        if software {
            return Ok(Self::Software(Box::new(software::Software::new(
                window, width, height,
            )?)));
        }
        #[cfg(not(feature = "cpu"))]
        let _ = software;
        Ok(Self::Gpu(Box::new(Renderer::new(
            window, width, height, notify,
        )?)))
    }

    /// A presenter of the same kind for another window: a surface on the same
    /// GPU device, or another software surface.
    pub(super) fn new_surface(
        &self,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        match self {
            Self::Gpu(renderer) => Ok(Self::Gpu(Box::new(
                renderer.new_surface(window, width, height)?,
            ))),
            #[cfg(feature = "cpu")]
            Self::Software(_) => Ok(Self::Software(Box::new(software::Software::new(
                window, width, height,
            )?))),
        }
    }

    /// Applies a native size; zero in either dimension unconfigures.
    pub(super) fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        match self {
            Self::Gpu(renderer) => renderer.resize(width, height),
            #[cfg(feature = "cpu")]
            Self::Software(software) => software.resize(width, height),
        }
    }

    /// Renders and presents `scene`, calling `before_present` just before.
    pub(super) fn render(
        &mut self,
        scene: DisplayList,
        before_present: impl FnOnce(),
    ) -> Result<Frame, RenderError> {
        match self {
            Self::Gpu(renderer) => renderer.render(scene, before_present),
            #[cfg(feature = "cpu")]
            Self::Software(software) => software.render(&scene, before_present),
        }
    }

    /// Recreates the native surface after the presenter asked for it.
    pub(super) fn recreate(&mut self, window: Arc<Window>) -> Result<(), RenderError> {
        match self {
            Self::Gpu(renderer) => renderer.recreate(window),
            #[cfg(feature = "cpu")]
            Self::Software(software) => software.recreate(window),
        }
    }

    /// Renders `scene` for a capture, keeping the background's alpha.
    pub(super) fn render_offscreen(
        &mut self,
        scene: DisplayList,
    ) -> Result<FrameImage, RenderError> {
        match self {
            Self::Gpu(renderer) => renderer.render_offscreen(scene),
            #[cfg(feature = "cpu")]
            Self::Software(software) => software.render_offscreen(&scene),
        }
    }
}

#[cfg(feature = "cpu")]
mod software {
    use super::*;
    use pert_ui_cpu::{CpuRenderError, CpuRenderer};
    use pert_ui_wgpu::LimitError;
    use std::num::NonZeroU32;

    /// A softbuffer surface fed by the CPU renderer.
    pub(in crate::platform) struct Software {
        surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
        renderer: CpuRenderer,
        /// The configured size, or `None` while either dimension is zero.
        extent: Option<(NonZeroU32, NonZeroU32)>,
    }

    fn surface(
        window: Arc<Window>,
    ) -> Result<softbuffer::Surface<Arc<Window>, Arc<Window>>, RenderError> {
        let context = softbuffer::Context::new(window.clone()).map_err(unsupported)?;
        softbuffer::Surface::new(&context, window).map_err(unsupported)
    }

    /// A platform softbuffer cannot drive is an unsupported configuration;
    /// other failures are surface failures.
    fn unsupported(error: softbuffer::SoftBufferError) -> RenderError {
        match error {
            softbuffer::SoftBufferError::UnsupportedDisplayPlatform { .. }
            | softbuffer::SoftBufferError::UnsupportedWindowPlatform { .. } => {
                RenderError::UnsupportedGraphics(format!("software presentation: {error}"))
            }
            error => RenderError::Surface(format!("software presentation: {error}")),
        }
    }

    /// The CPU renderer's refusals in the GPU renderer's terms, so the host
    /// treats both alike; a too large frame is the presenting kind of limit.
    fn presenting(error: CpuRenderError) -> RenderError {
        match error {
            CpuRenderError::Drawing(error) => RenderError::Drawing(error),
            CpuRenderError::TooLarge { width, height } => {
                RenderError::Limit(LimitError::FrameSize { width, height })
            }
            CpuRenderError::Unsupported => RenderError::UnsupportedGraphics(error.to_string()),
            error => RenderError::Graphics(error.to_string()),
        }
    }

    impl Software {
        pub(in crate::platform) fn new(
            window: Arc<Window>,
            width: u32,
            height: u32,
        ) -> Result<Self, RenderError> {
            let mut software = Self {
                surface: surface(window)?,
                renderer: CpuRenderer::new(),
                extent: None,
            };
            // A window already beyond the frame budget starts unconfigured: the
            // host resizes every new or resumed presenter next and reports the
            // limit through its error policy there, as for any later resize,
            // rather than failing the whole application here.
            match software.resize(width, height) {
                Ok(()) | Err(RenderError::Limit(LimitError::FrameSize { .. })) => Ok(software),
                Err(error) => Err(error),
            }
        }

        pub(in crate::platform) fn resize(
            &mut self,
            width: u32,
            height: u32,
        ) -> Result<(), RenderError> {
            let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) else {
                self.extent = None;
                return Ok(());
            };
            let fits = u64::from(width)
                .checked_mul(u64::from(height))
                .and_then(|pixels| pixels.checked_mul(4))
                .is_some_and(|bytes| bytes <= FrameImage::MAX_BYTES as u64);
            if !fits {
                // The previous configuration stays, as a GPU surface keeps
                // its own past the device limit.
                return Err(RenderError::Limit(LimitError::FrameSize { width, height }));
            }
            self.surface.resize(w, h).map_err(unsupported)?;
            self.extent = Some((w, h));
            Ok(())
        }

        pub(in crate::platform) fn render(
            &mut self,
            scene: &DisplayList,
            before_present: impl FnOnce(),
        ) -> Result<Frame, RenderError> {
            if self.extent.is_none() {
                return Ok(Frame::Occluded);
            }
            let frame = match self.renderer.render_opaque(scene) {
                Ok(frame) => frame,
                Err(CpuRenderError::EmptyViewport) => return Ok(Frame::Occluded),
                Err(error) => return Err(presenting(error)),
            };
            // The frame has the viewport's physical extent, which the host set
            // from the same native size; present it at its own size if a
            // resize is still in flight.
            let (Some(width), Some(height)) = (
                NonZeroU32::new(frame.width()),
                NonZeroU32::new(frame.height()),
            ) else {
                return Ok(Frame::Occluded);
            };
            if self.extent != Some((width, height)) {
                self.surface.resize(width, height).map_err(unsupported)?;
                self.extent = Some((width, height));
            }
            let mut buffer = self.surface.buffer_mut().map_err(unsupported)?;
            // Opaque sRGB bytes packed as softbuffer's 0x00RRGGBB.
            let (pixels, _) = frame.pixels().as_chunks::<4>();
            for (target, [red, green, blue, _]) in buffer.iter_mut().zip(pixels) {
                *target = u32::from(*red) << 16 | u32::from(*green) << 8 | u32::from(*blue);
            }
            before_present();
            buffer.present().map_err(unsupported)?;
            Ok(Frame::Presented)
        }

        pub(in crate::platform) fn recreate(
            &mut self,
            window: Arc<Window>,
        ) -> Result<(), RenderError> {
            self.surface = surface(window)?;
            if let Some((width, height)) = self.extent {
                self.surface.resize(width, height).map_err(unsupported)?;
            }
            Ok(())
        }

        pub(in crate::platform) fn render_offscreen(
            &mut self,
            scene: &DisplayList,
        ) -> Result<FrameImage, RenderError> {
            self.renderer.render(scene).map_err(|error| match error {
                CpuRenderError::TooLarge { width, height } => {
                    RenderError::Limit(LimitError::CaptureSize { width, height })
                }
                CpuRenderError::EmptyViewport => RenderError::Unconfigured,
                error => presenting(error),
            })
        }
    }
}
