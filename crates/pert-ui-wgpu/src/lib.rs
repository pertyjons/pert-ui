#![doc = include_str!("../README.md")]
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
compile_error!(
    "pert-ui-wgpu supports Linux (Vulkan), Windows (DX12) and macOS (Metal) only; \
     there is no fallback backend for other targets"
);

#[cfg(test)]
mod cpu_parity;
#[cfg(test)]
mod native_smoke;
#[cfg(test)]
mod realtime_probe;
mod retained;
mod textures;

use pert_ui_runtime::{
    Color, DisplayList, DrawCommand, ImageSampling, MaskImage, PhysicalExtent, QuarterTurn,
    RasterImage,
};
use retained::{RetainedGeometry, changed_bytes};
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

// What a run of vertices samples: nothing, a mask, or an image with a filter.
// Indices refer to the prepared scene's deduplicated resource tables.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Source {
    Fill,
    Mask(usize),
    Image { index: usize, filter: usize },
}
#[derive(Clone, Debug, PartialEq)]
struct Batch {
    range: std::ops::Range<u32>,
    source: Source,
}

/// Failure in hardware setup, scene preparation, command submission or surface
/// recovery.
///
/// [`RenderError::Limit`] and [`RenderError::Unconfigured`] are fixable by the
/// host or its content and leave the renderer usable; the other variants report
/// missing capabilities or genuine GPU failures.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RenderError {
    /// Vector fallback could not flatten or rasterize the supplied geometry.
    #[error(transparent)]
    Drawing(#[from] pert_ui_runtime::DrawError),
    /// A surface, capture or scene exceeds a documented budget or a device limit.
    ///
    /// The renderer stays usable: resize to a smaller extent or submit a smaller
    /// scene. The next successful scene is prepared from scratch.
    #[error(transparent)]
    Limit(#[from] LimitError),
    /// The renderer has no nonzero surface extent: it was created at zero size
    /// or last resized to zero. A nonzero [`Renderer::resize`] configures it.
    #[error("the surface has no nonzero extent to render into")]
    Unconfigured,
    /// The required hardware adapter, format or capability is unavailable, or a
    /// scene uses a drawing command or sampling mode this adapter does not know.
    #[error("unsupported graphics configuration: {0}")]
    UnsupportedGraphics(String),
    /// Device creation, pipeline validation, read-back or an internal scene
    /// invariant failed.
    #[error("graphics operation failed: {0}")]
    Graphics(String),
    /// Native presentation surface creation or recovery failed.
    #[error("presentation surface failed: {0}")]
    Surface(String),
}

/// Presentation limits are shared with the CPU renderer and the facade, so
/// they are defined in `pert-ui-runtime` and re-exported here unchanged.
pub use pert_ui_runtime::{LimitError, ResourceKind};

/// Asynchronous GPU notification. Marshal to the owning UI/event-loop thread.
/// The callback can run on driver threads; it must not touch live UI handles.
#[derive(Clone, Debug)]
pub enum Notice {
    /// Device loss requiring recreation of the adapter and all GPU resources.
    Lost(String),
    /// Uncaptured GPU validation/allocation/internal error; stop presentation.
    Error(String),
}
/// Surface acquisition/presentation outcome for the host's bounded recovery policy.
///
/// Every variant except `Presented` asks the host to act; discarding it loses
/// that recovery step.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "a frame that was not presented needs the host's recovery step"]
pub enum Frame {
    /// Frame submitted and queued for presentation.
    Presented,
    /// Transient acquisition failure; defer a bounded retry.
    /// An unclassified validation/device-loss result also takes this path so the
    /// host can process the accompanying error/loss notification before retrying.
    Retry,
    /// Pause until a native visibility/exposure notification; do not poll.
    /// Also returned while the renderer has no nonzero extent, without touching
    /// the surface: resume after a nonzero [`Renderer::resize`].
    Occluded,
    /// Recreate the native surface before a bounded retry.
    Recreate,
}

// The pipelines of one surface format, bound to the device-wide texture caches.
#[derive(Clone)]
struct Pipelines {
    fill: wgpu::RenderPipeline,
    masks: textures::Textures<MaskImage>,
    images: textures::Textures<RasterImage>,
}
// Device-wide state every renderer made from one `Renderer::new` shares: the
// texture caches of both resource kinds, whatever their surfaces' formats, and
// the pipelines of each surface format.
struct Shared {
    masks: Rc<textures::Shared<MaskImage>>,
    images: Rc<textures::Shared<RasterImage>>,
    pipelines: RefCell<HashMap<wgpu::TextureFormat, Pipelines>>,
}
impl Shared {
    fn new(device: &wgpu::Device) -> Result<Self, RenderError> {
        let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let masks = Rc::new(textures::Shared::new(device, &textures::MASKS));
        let images = Rc::new(textures::Shared::new(device, &textures::IMAGES));
        if let Some(error) = pollster::block_on(error_scope.pop()) {
            return Err(RenderError::Graphics(error.to_string()));
        }
        Ok(Self {
            masks,
            images,
            pipelines: RefCell::default(),
        })
    }
    // The pipelines of `format`, created on first use.
    fn pipelines(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
    ) -> Result<Pipelines, RenderError> {
        if let Some(cached) = self.pipelines.borrow().get(&format) {
            return Ok(cached.clone());
        }
        let created = create_pipelines(device, format, self)?;
        self.pipelines.borrow_mut().insert(format, created.clone());
        Ok(created)
    }
}

/// Hardware adapter for the backend-independent [`DisplayList`] contract.
///
/// Owns GPU resources and a native surface, but no event loop, UI runtime or timers.
/// The standalone `pert-ui` facade selects this adapter automatically. Advanced
/// hosts may integrate it directly. Only this explicit backend crate exposes wgpu
/// surface interop; the runtime and ordinary facade APIs contain no wgpu types.
/// Vector geometry uses a retained CPU coverage fallback; original paths stay in
/// the public scene. Unchanged command slots reuse encoded vertices. Changed byte
/// ranges upload into a retained buffer, whose capacity grows geometrically.
/// Insertions/reordering may invalidate later slots; viewport changes invalidate
/// all geometry. Composition, comparison and CPU byte assembly remain scene-linear.
/// Empty scenes release obsolete coverage while retaining buffer capacity.
///
/// # Host integration
///
/// A direct host owns the event loop and the recovery policy. It forwards
/// [`Notice`]s from the driver-thread callback to its UI thread, handles them
/// before the next frame, and acts on every [`Frame`] and [`RenderError`]. The
/// sketch below is generic over the host's owned window handle, such as an
/// `Arc` of a winit window:
///
/// ```no_run
/// use pert_ui_runtime::DisplayList;
/// use pert_ui_wgpu::{Frame, Notice, RenderError, Renderer};
/// use std::sync::mpsc::{Receiver, Sender};
///
/// /// What the host's event loop does after one redraw attempt.
/// enum Next {
///     /// Presented; wait for the next change.
///     Idle,
///     /// Schedule a bounded, delayed retry; never spin.
///     RetryLater,
///     /// Wait for a visibility or resize event; do not poll.
///     WaitForExposure,
///     /// Rebuild the renderer and every surface made from it.
///     RebuildDevice,
///     /// Stop presenting and report the failure.
///     Stop(String),
/// }
///
/// fn create<W>(window: W, width: u32, height: u32, notices: Sender<Notice>)
///     -> Result<Renderer, RenderError>
/// where
///     W: Into<wgpu::SurfaceTarget<'static>>,
/// {
///     // Runs on driver threads: only forward. A real host also wakes its
///     // event loop here, for example through an event-loop proxy.
///     Renderer::new(window, width, height, move |notice| {
///         let _ = notices.send(notice);
///     })
/// }
///
/// fn redraw<W>(
///     renderer: &mut Renderer,
///     window: &W,
///     notices: &Receiver<Notice>,
///     scene: DisplayList,
/// ) -> Next
/// where
///     W: Clone + Into<wgpu::SurfaceTarget<'static>>,
/// {
///     for notice in notices.try_iter() {
///         match notice {
///             Notice::Lost(_) => return Next::RebuildDevice,
///             Notice::Error(message) => return Next::Stop(message),
///         }
///     }
///     // A real host passes its compositor hook, such as winit's
///     // `pre_present_notify`, instead of the empty closure.
///     match renderer.render(scene, || {}) {
///         Ok(Frame::Presented) => Next::Idle,
///         Ok(Frame::Retry) => Next::RetryLater,
///         Ok(Frame::Occluded) => Next::WaitForExposure,
///         Ok(Frame::Recreate) => match renderer.recreate(window.clone()) {
///             Ok(()) => Next::RetryLater,
///             Err(error) => Next::Stop(error.to_string()),
///         },
///         // Fixable by the content: keep the renderer and show a smaller scene.
///         Err(RenderError::Limit(limit)) => Next::Stop(format!("scene too large: {limit}")),
///         Err(error) => Next::Stop(error.to_string()),
///     }
/// }
/// ```
pub struct Renderer {
    shared: Rc<Shared>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    // The surface's format and presentation policy; its size is the last
    // configured extent and meaningful only while `extent` is `Some`.
    config: wgpu::SurfaceConfiguration,
    // The configured nonzero extent, `None` until the first nonzero resize and
    // after a resize to zero.
    extent: Option<PhysicalExtent>,
    pipelines: Pipelines,
    vertices: Option<wgpu::Buffer>,
    batches: Vec<Batch>,
    scene: Option<DisplayList>,
    rasterizer: pert_ui_runtime::VectorRasterizer,
    encoded: RetainedGeometry,
}

// The backend each supported platform enables; the crate root rejects other
// targets at compile time.
fn backend() -> wgpu::Backends {
    #[cfg(target_os = "linux")]
    {
        wgpu::Backends::VULKAN
    }
    #[cfg(target_os = "windows")]
    {
        wgpu::Backends::DX12
    }
    #[cfg(target_os = "macos")]
    {
        wgpu::Backends::METAL
    }
}

fn acquired(texture: wgpu::CurrentSurfaceTexture) -> Result<(wgpu::SurfaceTexture, bool), Frame> {
    match texture {
        wgpu::CurrentSurfaceTexture::Success(frame) => Ok((frame, false)),
        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Ok((frame, true)),
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Outdated => {
            Err(Frame::Retry)
        }
        wgpu::CurrentSurfaceTexture::Occluded => Err(Frame::Occluded),
        wgpu::CurrentSurfaceTexture::Lost => Err(Frame::Recreate),
        wgpu::CurrentSurfaceTexture::Validation => {
            // wgpu reports both validation and acquisition-time device loss here,
            // delivering the precise cause through callbacks. Let the host consume
            // Notice before its bounded delayed retry, rather than exiting first.
            Err(Frame::Retry)
        }
    }
}

// Rejects an extent beyond the device's 2D texture dimension.
fn check_extent(device: &wgpu::Device, width: u32, height: u32) -> Result<(), LimitError> {
    let max = device.limits().max_texture_dimension_2d;
    if width > max || height > max {
        return Err(LimitError::SurfaceExtent { width, height, max });
    }
    Ok(())
}

#[cfg(test)]
mod acquisition_tests {
    use super::*;

    #[test]
    fn acquisition_device_error_allows_notification_before_retry() {
        assert!(matches!(
            acquired(wgpu::CurrentSurfaceTexture::Validation),
            Err(Frame::Retry)
        ));
        assert!(matches!(
            acquired(wgpu::CurrentSurfaceTexture::Occluded),
            Err(Frame::Occluded)
        ));
        assert!(matches!(
            acquired(wgpu::CurrentSurfaceTexture::Lost),
            Err(Frame::Recreate)
        ));
    }
}

// The compositor contract applies only to presentation. Captures always retain
// linear-premultiplied RGB before sRGB encoding, independently of the window.
#[derive(Clone, Copy)]
enum Destination {
    Opaque,
    Premultiplied,
}
impl Destination {
    fn clear(self, color: Color) -> wgpu::Color {
        let [r, g, b, a] = color.channels().map(f64::from);
        match self {
            Self::Opaque => wgpu::Color { r, g, b, a: 1.0 },
            Self::Premultiplied => wgpu::Color {
                r: r * a,
                g: g * a,
                b: b * a,
                a,
            },
        }
    }
}
fn surface_alpha(
    modes: &[wgpu::CompositeAlphaMode],
) -> Result<wgpu::CompositeAlphaMode, RenderError> {
    [
        wgpu::CompositeAlphaMode::Opaque,
        wgpu::CompositeAlphaMode::PreMultiplied,
    ]
    .into_iter()
    .find(|mode| modes.contains(mode))
    .ok_or_else(|| {
        RenderError::UnsupportedGraphics(
            "surface supports neither opaque nor premultiplied alpha".into(),
        )
    })
}

impl Renderer {
    /// How long [`Renderer::render_offscreen`] waits for the GPU in total
    /// before it reports a read-back timeout.
    ///
    /// The time starts when the capture's commands are submitted, so it bounds
    /// only the GPU work and the mapping of the read-back buffer. Preparing
    /// the scene on the CPU, which rasterizes vector content and uploads
    /// changed data, happens before that and does not count against it.
    pub const READBACK_TIMEOUT: Duration = Duration::from_secs(5);

    /// Creates a hardware renderer for an owned native surface target.
    ///
    /// Pass an owned target (typically an `Arc` window) so wgpu can retain its
    /// lifetime safely. Use the target platform's main thread. The notification
    /// callback must remain valid until GPU teardown; discard notifications after
    /// the owning event loop exits. No software or legacy backend fallback occurs.
    ///
    /// A zero `width` or `height` leaves the surface unconfigured until a nonzero
    /// [`resize`](Self::resize); meanwhile [`render`](Self::render) returns
    /// [`Frame::Occluded`] and [`render_offscreen`](Self::render_offscreen)
    /// fails with [`RenderError::Unconfigured`].
    ///
    /// The device's maximum texture dimension is the adapter's own, so surfaces
    /// and resources as large as the hardware supports are accepted; other
    /// limits are wgpu's defaults.
    ///
    /// # Errors
    /// [`RenderError::UnsupportedGraphics`] without a compatible hardware
    /// adapter or an sRGB surface format, [`RenderError::Graphics`] for device
    /// or pipeline validation failures, and [`RenderError::Limit`] for a nonzero
    /// size beyond the device's texture dimension. A surface must support opaque
    /// (preferred) or premultiplied compositor alpha; other alpha policies are
    /// rejected. Opaque presentation retains background RGB with alpha one;
    /// premultiplied presentation scales background RGB by its alpha.
    pub fn new(
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
        notify: impl Fn(Notice) + Send + Sync + 'static,
    ) -> Result<Self, RenderError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: backend(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(target)
            .map_err(|e| RenderError::UnsupportedGraphics(e.to_string()))?;
        // Enumerate compatible hardware rather than accepting a CPU fallback selected
        // by a driver. Prefer integrated hardware for this modest GUI workload.
        let mut adapters = pollster::block_on(instance.enumerate_adapters(backend()));
        adapters.retain(|adapter| {
            matches!(
                adapter.get_info().device_type,
                wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
            ) && adapter.is_surface_supported(&surface)
        });
        adapters.sort_by_key(|adapter| {
            adapter.get_info().device_type != wgpu::DeviceType::IntegratedGpu
        });
        let adapter = adapters.into_iter().next().ok_or_else(|| {
            RenderError::UnsupportedGraphics(
                "no compatible integrated/discrete hardware adapter on the selected backend".into(),
            )
        })?;
        let config = surface_configuration(&surface, &adapter, width, height)?;
        let format = config.format;
        // Surfaces spanning several monitors and large images need the hardware's
        // texture dimension, not the portable 8192 default.
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("pert-ui device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
            ..Default::default()
        }))
        .map_err(|e| RenderError::Graphics(e.to_string()))?;
        let notify = Arc::new(notify);
        let errors = notify.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            errors(Notice::Error(error.to_string()))
        }));
        device.set_device_lost_callback(move |reason, message| {
            if reason != wgpu::DeviceLostReason::Destroyed {
                notify(Notice::Lost(message));
            }
        });
        let shared = Rc::new(Shared::new(&device)?);
        let pipelines = shared.pipelines(&device, format)?;
        let mut renderer = Self {
            shared,
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            extent: None,
            pipelines,
            vertices: None,
            batches: Vec::new(),
            scene: None,
            rasterizer: Default::default(),
            encoded: Default::default(),
        };
        renderer.resize(width, height)?;
        Ok(renderer)
    }

    /// The configured surface extent in physical pixels, or `None` while the
    /// renderer has no nonzero extent (created at zero size or last resized to
    /// zero).
    #[must_use]
    pub fn extent(&self) -> Option<PhysicalExtent> {
        self.extent
    }

    /// Configures the surface for a physical-pixel extent.
    ///
    /// A zero `width` or `height` unconfigures the renderer rather than the
    /// surface: until the next nonzero resize, [`render`](Self::render) returns
    /// [`Frame::Occluded`] without acquiring a frame, and
    /// [`render_offscreen`](Self::render_offscreen) fails with
    /// [`RenderError::Unconfigured`].
    ///
    /// # Errors
    /// [`RenderError::Limit`] with [`LimitError::SurfaceExtent`] when a dimension
    /// exceeds the device's texture dimension; the previous configuration stays.
    /// Asynchronous GPU failures are delivered to the startup notification
    /// callback.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), RenderError> {
        if width == 0 || height == 0 {
            self.extent = None;
            return Ok(());
        }
        check_extent(&self.device, width, height)?;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.extent = Some(PhysicalExtent::new(width, height));
        Ok(())
    }

    /// Recreates a lost surface using an owned target, retaining device resources.
    /// The host controls retry bounds and must not call this with a frame in flight.
    /// An unconfigured renderer stays unconfigured until a nonzero resize.
    ///
    /// # Errors
    /// Reports target creation failure or loss of the selected format or alpha
    /// capability. The prior surface and configuration remain owned on rejection.
    pub fn recreate(
        &mut self,
        target: impl Into<wgpu::SurfaceTarget<'static>>,
    ) -> Result<(), RenderError> {
        let surface = self
            .instance
            .create_surface(target)
            .map_err(|e| RenderError::Surface(e.to_string()))?;
        let capabilities = surface.get_capabilities(&self.adapter);
        if !capabilities.formats.contains(&self.config.format) {
            return Err(RenderError::Surface(
                "recreated surface no longer supports the selected format".into(),
            ));
        }
        if !capabilities.alpha_modes.contains(&self.config.alpha_mode) {
            return Err(RenderError::Surface(
                "recreated surface no longer supports the selected alpha policy".into(),
            ));
        }
        if self.extent.is_some() {
            surface.configure(&self.device, &self.config);
        }
        self.surface = surface;
        Ok(())
    }

    fn presentation_destination(&self) -> Destination {
        match self.config.alpha_mode {
            wgpu::CompositeAlphaMode::Opaque => Destination::Opaque,
            _ => Destination::Premultiplied,
        }
    }

    fn encode(
        &self,
        view: &wgpu::TextureView,
        destination: Destination,
    ) -> Result<wgpu::CommandEncoder, RenderError> {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let clear = destination.clear(
            self.scene
                .as_ref()
                .map_or(Color::BACKGROUND, DisplayList::background),
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pert-ui scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            if let Some(vertices) = &self.vertices {
                pass.set_vertex_buffer(0, vertices.slice(..));
                let pipelines = &self.pipelines;
                for batch in &self.batches {
                    match batch.source {
                        Source::Fill => pass.set_pipeline(&pipelines.fill),
                        Source::Mask(index) => {
                            pass.set_pipeline(&pipelines.masks.pipeline);
                            pass.set_bind_group(0, pipelines.masks.bind(index, 0)?, &[]);
                        }
                        Source::Image { index, filter } => {
                            pass.set_pipeline(&pipelines.images.pipeline);
                            pass.set_bind_group(0, pipelines.images.bind(index, filter)?, &[]);
                        }
                    }
                    pass.draw(batch.range.clone(), 0..1);
                }
            }
        }
        Ok(encoder)
    }

    /// Renders ordered primitives and queues presentation of one nonzero surface.
    ///
    /// `before_present` lets a native host notify its compositor immediately before
    /// presentation. The host decides when work is necessary and handles
    /// recovery/occlusion from [`Frame`]; see the host integration example on
    /// [`Renderer`]. While the renderer has no nonzero extent this returns
    /// [`Frame::Occluded`] without acquiring a frame or preparing `scene`.
    /// This adapter clips to the logical viewport and composites straight-alpha
    /// inputs into a premultiplied or opaque destination in linear light.
    ///
    /// # Errors
    /// - [`RenderError::Limit`] when the scene exceeds a
    ///   [`ResourceKind::scene_budget`], a resource exceeds the device texture
    ///   dimension, or its vertices exceed the device buffer or a `u32` draw range.
    /// - [`RenderError::Drawing`] when the vector fallback cannot flatten or
    ///   rasterize a path.
    /// - [`RenderError::UnsupportedGraphics`] for a drawing command or sampling
    ///   mode, added by a newer runtime, that this adapter does not know.
    /// - [`RenderError::Graphics`] when the scene breaks an internal invariant,
    ///   such as a dangling resource reference.
    ///
    /// A failed preparation presents nothing; the next call prepares from scratch.
    /// Acquisition validation and other GPU errors arrive through the
    /// notification callback supplied to [`new`](Self::new).
    pub fn render(
        &mut self,
        scene: DisplayList,
        before_present: impl FnOnce(),
    ) -> Result<Frame, RenderError> {
        if self.extent.is_none() {
            return Ok(Frame::Occluded);
        }
        let texture = self.surface.get_current_texture();
        if matches!(texture, wgpu::CurrentSurfaceTexture::Outdated) {
            self.surface.configure(&self.device, &self.config);
        }
        let (frame, suboptimal) = match acquired(texture) {
            Ok(frame) => frame,
            Err(outcome) => return Ok(outcome),
        };
        self.prepare_scene(scene)?;
        let view = frame.texture.create_view(&Default::default());
        let encoder = self.encode(&view, self.presentation_destination())?;
        self.queue.submit([encoder.finish()]);
        before_present();
        self.queue.present(frame);
        if suboptimal {
            self.surface.configure(&self.device, &self.config);
        }
        Ok(Frame::Presented)
    }
    /// Renders `scene` into an offscreen texture of the configured
    /// [`extent`](Self::extent) and reads the pixels back as an owned 8-bit sRGB
    /// RGBA frame with linear-premultiplied color encoded as sRGB. Background
    /// alpha is preserved even when native presentation uses an opaque surface.
    ///
    /// This is how the default host answers a capture request: the same
    /// prepared scene the surface shows, rendered once more to a texture that
    /// is never presented, so a capture does not depend on the compositor and
    /// does not disturb presentation. After the scene is prepared, the
    /// read-back blocks the calling thread for at most
    /// [`Renderer::READBACK_TIMEOUT`] in total; call it from the UI thread
    /// between frames, never from a time-critical producer callback.
    ///
    /// # Errors
    /// - [`RenderError::Unconfigured`] while the renderer has no nonzero extent.
    /// - [`RenderError::Limit`] when the capture exceeds the frame image size,
    ///   and for the scene limits of [`render`](Self::render). The extent
    ///   itself was checked against the device texture dimension by
    ///   [`resize`](Self::resize).
    /// - [`RenderError::Drawing`] and [`RenderError::UnsupportedGraphics`] as for
    ///   `render`; `UnsupportedGraphics` also for a surface format that is not
    ///   8-bit sRGB.
    /// - [`RenderError::Graphics`] for read-back failures and timeouts.
    pub fn render_offscreen(
        &mut self,
        scene: DisplayList,
    ) -> Result<pert_ui_runtime::FrameImage, RenderError> {
        let extent = self.extent.ok_or(RenderError::Unconfigured)?;
        let (width, height) = (extent.width(), extent.height());
        let swap = match self.config.format {
            wgpu::TextureFormat::Bgra8UnormSrgb => true,
            wgpu::TextureFormat::Rgba8UnormSrgb => false,
            other => {
                return Err(RenderError::UnsupportedGraphics(format!(
                    "capture needs an 8-bit sRGB surface, found {other:?}"
                )));
            }
        };
        // Refuse before allocating anything the limit forbids.
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4));
        if bytes.is_none_or(|bytes| bytes > pert_ui_runtime::FrameImage::MAX_BYTES as u64) {
            return Err(LimitError::CaptureSize { width, height }.into());
        }
        // `resize` checked the extent against this device's texture dimension
        // before configuring it, so it is not checked again here.
        self.prepare_scene(scene)?;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("pert-ui capture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        // Rows are padded to the copy alignment and unpadded after mapping.
        let unpadded = width as usize * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded = unpadded.div_ceil(align) * align;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pert-ui capture read-back"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let view = texture.create_view(&Default::default());
        let mut encoder = self.encode(&view, Destination::Premultiplied)?;
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        // The deadline bounds the wait for the GPU, not the preparation above.
        let deadline = std::time::Instant::now() + Self::READBACK_TIMEOUT;
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = send.send(result);
            });
        // One deadline bounds both the GPU wait and the mapping callback.
        let remaining = || deadline.saturating_duration_since(std::time::Instant::now());
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(remaining()),
            })
            .map_err(|e| RenderError::Graphics(format!("capture read-back: {e}")))?;
        receive
            .recv_timeout(remaining())
            .map_err(|e| RenderError::Graphics(format!("capture read-back: {e}")))?
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Graphics(e.to_string()))?;
        let mut rgba = Vec::with_capacity(unpadded * height as usize);
        for row in mapped.chunks(padded) {
            let row = &row[..unpadded];
            if swap {
                for pixel in row.chunks(4) {
                    rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                }
            } else {
                rgba.extend_from_slice(row);
            }
        }
        drop(mapped);
        buffer.unmap();
        pert_ui_runtime::FrameImage::new(width, height, rgba)
            .map_err(|e| RenderError::Graphics(e.to_string()))
    }
    fn prepare_scene(&mut self, scene: DisplayList) -> Result<(), RenderError> {
        let scene = self.rasterizer.prepare(&scene)?;
        self.prepare_rasterized(scene)
    }
    // Encodes and uploads a scene whose vectors are already coverage masks.
    fn prepare_rasterized(&mut self, scene: DisplayList) -> Result<(), RenderError> {
        if self.scene.as_ref() != Some(&scene) {
            // Every failed preparation must force a full rebuild on the next call.
            self.scene = None;
            let encoded = self.encoded.prepare(&scene)?;
            let bytes = &encoded.bytes;
            let mut upload = changed_bytes(&self.encoded.bytes, bytes);
            self.pipelines
                .masks
                .prepare(&self.device, &self.queue, scene.masks())?;
            self.pipelines
                .images
                .prepare(&self.device, &self.queue, scene.images())?;
            self.batches = encoded.batches.clone();
            let max = self.device.limits().max_buffer_size;
            if bytes.len() as u64 > max {
                return Err(LimitError::VertexBuffer {
                    bytes: bytes.len() as u64,
                    max,
                }
                .into());
            }
            if !bytes.is_empty() {
                let required = bytes.len() as u64;
                if self
                    .vertices
                    .as_ref()
                    .is_none_or(|buffer| buffer.size() < required)
                {
                    let capacity = required
                        .checked_next_power_of_two()
                        .unwrap_or(required)
                        .min(max);
                    upload = 0..bytes.len();
                    self.vertices = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("pert-ui retained vertices"),
                        size: capacity,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }));
                }
                if let Some(buffer) = &self.vertices
                    && !upload.is_empty()
                {
                    self.queue
                        .write_buffer(buffer, upload.start as u64, &bytes[upload]);
                }
            }
            self.encoded = encoded;
            self.scene = Some(scene);
        }
        Ok(())
    }
}

#[cfg(test)]
fn geometry(scene: &DisplayList) -> Result<(Vec<u8>, Vec<Batch>), RenderError> {
    geometry_commands(scene, scene.commands())
}
fn geometry_commands(
    scene: &DisplayList,
    commands: &[DrawCommand],
) -> Result<(Vec<u8>, Vec<Batch>), RenderError> {
    let width = f64::from(scene.viewport().width());
    let height = f64::from(scene.viewport().height());
    let mut bytes = Vec::new();
    let mut batches: Vec<Batch> = Vec::new();
    if width == 0.0 || height == 0.0 {
        return Ok((bytes, batches));
    }
    for command in commands {
        let (rect, color, clip, source, turn) = match *command {
            DrawCommand::FillRect { rect, color } => {
                (rect, color, None, Source::Fill, QuarterTurn::None)
            }
            DrawCommand::DrawMask {
                rect,
                color,
                clip,
                mask,
                turn,
            } => {
                scene
                    .mask(mask)
                    .ok_or_else(|| RenderError::Graphics("invalid scene mask reference".into()))?;
                (rect, color, clip, Source::Mask(mask.index()), turn)
            }
            DrawCommand::DrawImage {
                rect,
                image,
                tint,
                sampling,
                clip,
                turn,
            } => {
                scene
                    .image(image)
                    .ok_or_else(|| RenderError::Graphics("invalid scene image reference".into()))?;
                // Index into the image cache's filters, in `textures::IMAGES` order.
                let filter = match sampling {
                    ImageSampling::Linear => 0,
                    ImageSampling::Nearest => 1,
                    _ => {
                        return Err(RenderError::UnsupportedGraphics(
                            "unknown image sampling mode".into(),
                        ));
                    }
                };
                let source = Source::Image {
                    index: image.index(),
                    filter,
                };
                (rect, tint, clip, source, turn)
            }
            _ => {
                return Err(RenderError::UnsupportedGraphics(
                    "unknown drawing command".into(),
                ));
            }
        };
        let [x, y, w, h] = rect.values().map(f64::from);
        let [cx, cy, cw, ch] = clip.map_or([0.0, 0.0, width, height], |clip| {
            clip.values().map(f64::from)
        });
        let left = x.max(0.0).max(cx);
        let right = (x + w).min(width).min(cx + cw);
        let top = y.max(0.0).max(cy);
        let bottom = (y + h).min(height).min(cy + ch);
        if right <= left || bottom <= top || color.alpha() == 0.0 {
            continue;
        }
        let l = (2.0 * left / width - 1.0) as f32;
        let r = (2.0 * right / width - 1.0) as f32;
        let t = (1.0 - 2.0 * top / height) as f32;
        let b = (1.0 - 2.0 * bottom / height) as f32;
        let u0 = ((left - x) / w) as f32;
        let u1 = ((right - x) / w) as f32;
        let v0 = ((top - y) / h) as f32;
        let v1 = ((bottom - y) / h) as f32;
        let start = u32::try_from(bytes.len() / 32).map_err(|_| LimitError::VertexCount)?;
        let end = start.checked_add(6).ok_or(LimitError::VertexCount)?;
        // `u`/`v` are fractions of the destination; a quarter turn maps them to
        // the upright source, so a clip crops the turned picture consistently.
        for [x, y, u, v] in [
            [l, t, u0, v0],
            [l, b, u0, v1],
            [r, t, u1, v0],
            [r, t, u1, v0],
            [l, b, u0, v1],
            [r, b, u1, v1],
        ] {
            for value in [x, y]
                .into_iter()
                .chain(color.channels())
                .chain(turn.source(u, v))
            {
                bytes.extend_from_slice(&value.to_ne_bytes());
            }
        }
        if let Some(last) = batches.last_mut()
            && last.source == source
        {
            last.range.end = end;
        } else {
            batches.push(Batch {
                range: start..end,
                source,
            });
        }
    }
    Ok((bytes, batches))
}
#[cfg(test)]
fn vertices(scene: &DisplayList) -> Result<Vec<u8>, RenderError> {
    Ok(geometry(&scene.rasterized()?)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_alpha_prefers_opaque_and_rejects_undefined_policies() {
        use wgpu::CompositeAlphaMode::{Auto, Inherit, Opaque, PostMultiplied, PreMultiplied};
        assert_eq!(surface_alpha(&[PreMultiplied, Opaque]), Ok(Opaque));
        assert_eq!(surface_alpha(&[Auto, PreMultiplied]), Ok(PreMultiplied));
        for modes in [&[][..], &[Auto][..], &[Inherit][..], &[PostMultiplied][..]] {
            assert!(matches!(
                surface_alpha(modes),
                Err(RenderError::UnsupportedGraphics(_))
            ));
        }
    }

    #[test]
    fn destination_clear_respects_compositor_alpha_and_capture_color() {
        let color = Color::rgba(0.25, 0.5, 1.0, 0.5).expect("valid color");
        assert_eq!(
            Destination::Opaque.clear(color),
            wgpu::Color {
                r: 0.25,
                g: 0.5,
                b: 1.0,
                a: 1.0
            }
        );
        assert_eq!(
            Destination::Premultiplied.clear(color),
            wgpu::Color {
                r: 0.125,
                g: 0.25,
                b: 0.5,
                a: 0.5
            }
        );
        assert_eq!(
            Destination::Premultiplied.clear(Color::rgba(1.0, 1.0, 1.0, 0.0).expect("valid color")),
            wgpu::Color::TRANSPARENT
        );
    }
    use pert_ui_runtime::{
        Component, DrawError, Painter, Rect, Runtime, RuntimeError, UpdateContext, Viewport,
    };

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_gpu_pixel_readback() -> Result<(), Box<dyn std::error::Error>> {
        native_smoke::pixel_readback()
    }

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_damage_area_probe() -> Result<(), Box<dyn std::error::Error>> {
        native_smoke::damage_area_probe()
    }

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_realtime_workload_probe() -> Result<(), Box<dyn std::error::Error>> {
        realtime_probe::realtime_workload_probe()
    }

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_cpu_gpu_parity() -> Result<(), Box<dyn std::error::Error>> {
        cpu_parity::cpu_gpu_parity()
    }

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_text_rendering_probe() -> Result<(), Box<dyn std::error::Error>> {
        cpu_parity::text_rendering_probe()
    }

    #[test]
    #[ignore = "requires a native desktop session and hardware GPU adapter"]
    fn native_zero_size_surface_is_unconfigured() -> Result<(), Box<dyn std::error::Error>> {
        native_smoke::zero_size()
    }

    #[test]
    fn a_quarter_turn_permutes_the_source_and_a_clip_crops_it_consistently()
    -> Result<(), Box<dyn std::error::Error>> {
        use pert_ui_runtime::QuarterTurn;
        struct Turned;
        impl Component for Turned {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                let mask = MaskImage::new(2, 1, vec![255, 0])?;
                let tall = Rect::new(0.0, 0.0, 50.0, 100.0)?;
                painter.draw_mask_turned(&mask, tall, Color::WHITE, QuarterTurn::Right);
                // The lower half of the same turned picture.
                painter.with_clip(Rect::new(50.0, 50.0, 50.0, 50.0)?, |painter| {
                    let beside = Rect::new(50.0, 0.0, 50.0, 100.0).expect("valid");
                    painter.draw_mask_turned(&mask, beside, Color::WHITE, QuarterTurn::Right);
                });
                Ok(())
            }
        }
        let mut ui = Runtime::new();
        ui.mount("turned", Turned)?;
        ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
        ui.flush(std::time::Duration::ZERO)?;
        let (bytes, _) = geometry(&ui.paint()?)?;
        let vertices = bytes.as_chunks::<32>().0;
        let uv = |vertex: &[u8; 32]| -> Result<[f32; 2], std::array::TryFromSliceError> {
            Ok([
                f32::from_ne_bytes(vertex[24..28].try_into()?),
                f32::from_ne_bytes(vertex[28..32].try_into()?),
            ])
        };
        // Vertex order is top-left, bottom-left, top-right, ... Turned clockwise,
        // the destination's top-left shows the source's bottom-left, and going
        // down the destination walks the source from left to right.
        assert_eq!(uv(&vertices[0])?, [0.0, 1.0]);
        assert_eq!(uv(&vertices[1])?, [1.0, 1.0]);
        assert_eq!(uv(&vertices[2])?, [0.0, 0.0]);
        // Cropped to its lower half, the quad starts halfway along the source.
        assert_eq!(uv(&vertices[6])?, [0.5, 1.0]);
        assert_eq!(uv(&vertices[7])?, [1.0, 1.0]);
        Ok(())
    }

    #[test]
    fn images_batch_by_resource_and_filter_and_crop_their_source()
    -> Result<(), Box<dyn std::error::Error>> {
        use pert_ui_runtime::{ImageSampling, RasterImage};
        struct Images(RasterImage, RasterImage);
        impl Component for Images {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                let full = Rect::new(0.0, 0.0, 100.0, 100.0)?;
                painter.draw_image(&self.0, full, ImageSampling::Linear);
                painter.draw_image(&self.0, full, ImageSampling::Linear);
                painter.draw_image(&self.0, full, ImageSampling::Nearest);
                painter.with_clip(Rect::new(50.0, 0.0, 50.0, 100.0)?, |painter| {
                    painter.draw_image(&self.1, full, ImageSampling::Nearest);
                });
                Ok(())
            }
        }
        let image = |byte| RasterImage::from_srgba8(1, 1, vec![byte, byte, byte, 255]);
        let mut ui = Runtime::new();
        ui.mount("images", Images(image(10)?, image(20)?))?;
        ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
        ui.flush(std::time::Duration::ZERO)?;
        let scene = ui.paint()?;
        assert_eq!(scene.images().len(), 2);
        let (bytes, batches) = geometry(&scene)?;
        let sources: Vec<_> = batches.iter().map(|batch| batch.source).collect();
        assert_eq!(
            sources,
            [
                Source::Image {
                    index: 0,
                    filter: 0
                },
                Source::Image {
                    index: 0,
                    filter: 1
                },
                Source::Image {
                    index: 1,
                    filter: 1
                },
            ]
        );
        assert_eq!(batches[0].range, 0..12, "equal draws share one batch");
        // The clipped image keeps the right half of its source.
        let last = bytes.as_chunks::<32>().0.last().ok_or("no vertices")?;
        let u = f32::from_ne_bytes(last[24..28].try_into()?);
        let x = f32::from_ne_bytes(last[0..4].try_into()?);
        assert_eq!((x, u), (1.0, 1.0));
        let first_clipped = &bytes.as_chunks::<32>().0[18];
        assert_eq!(f32::from_ne_bytes(first_clipped[24..28].try_into()?), 0.5);
        Ok(())
    }

    #[test]
    fn clipped_vertices_are_finite_and_follow_viewport_coordinates()
    -> Result<(), Box<dyn std::error::Error>> {
        struct Huge;
        impl Component for Huge {
            fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
                Ok(())
            }
            fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
                painter.fill_rect(Rect::new(0.0, 0.0, f32::MAX, f32::MAX)?, Color::ACCENT);
                Ok(())
            }
        }
        let mut ui = Runtime::new();
        ui.mount("huge", Huge)?;
        ui.set_viewport(Viewport::new(100.0, 100.0, 1.25)?)?;
        ui.flush(std::time::Duration::ZERO)?;
        let bytes = vertices(&ui.paint()?)?;
        assert_eq!(bytes.len(), 6 * 32);
        for vertex in bytes.as_chunks::<32>().0 {
            for component in vertex[..8].as_chunks::<4>().0 {
                let value = f32::from_ne_bytes(*component);
                assert!(value == -1.0 || value == 1.0);
            }
        }
        Ok(())
    }
}

/// The presentation policy every surface shares, the first window's and each added
/// window's: an SDR sRGB format, FIFO presentation and at most two frames queued.
fn surface_configuration(
    surface: &wgpu::Surface<'_>,
    adapter: &wgpu::Adapter,
    width: u32,
    height: u32,
) -> Result<wgpu::SurfaceConfiguration, RenderError> {
    let capabilities = surface.get_capabilities(adapter);
    let alpha_mode = surface_alpha(&capabilities.alpha_modes)?;
    let format = capabilities
        .formats
        .into_iter()
        .find(|format| format.is_srgb())
        .ok_or_else(|| RenderError::UnsupportedGraphics("surface has no SDR sRGB format".into()))?;
    let mut config = surface
        .get_default_config(adapter, width.max(1), height.max(1))
        .ok_or_else(|| {
            RenderError::UnsupportedGraphics(
                "surface has no supported presentation configuration".into(),
            )
        })?;
    config.format = format;
    config.alpha_mode = alpha_mode;
    config.present_mode = wgpu::PresentMode::Fifo;
    config.desired_maximum_frame_latency = 2;
    Ok(config)
}
fn create_pipelines(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shared: &Shared,
) -> Result<Pipelines, RenderError> {
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pert-ui rectangles"),
        source: wgpu::ShaderSource::Wgsl(include_str!("rectangles.wgsl").into()),
    });
    let fill = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pert-ui solid fills"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4, 2 => Float32x2],
            })],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    let masks = textures::Textures::new(device, format, &shared.masks);
    let images = textures::Textures::new(device, format, &shared.images);
    if let Some(error) = pollster::block_on(error_scope.pop()) {
        return Err(RenderError::Graphics(error.to_string()));
    }
    Ok(Pipelines {
        fill,
        masks,
        images,
    })
}

impl Renderer {
    /// Creates an independent surface sharing this renderer's GPU context.
    ///
    /// Reuses the instance, selected adapter, device, queue, format-keyed pipelines
    /// and the device-wide mask and image texture caches, so equal content drawn
    /// in several windows is uploaded once whatever their surface formats.
    /// Surface configuration, extent, scene and vertex buffer remain local.
    /// A zero size leaves the new surface unconfigured, as for
    /// [`new`](Self::new). Any renderer can outlive the one that created
    /// it. The original notification callback reports shared device errors/loss;
    /// the host must invalidate all related renderers on device loss.
    ///
    /// Call on the owning UI thread after creating the native target. This checks
    /// adapter compatibility and chooses each surface's own sRGB format. Failure
    /// releases provisional resources without changing existing surfaces. Hosts
    /// must prepare this before committing runtime presentation movement.
    ///
    /// # Errors
    /// [`RenderError::Surface`] when the target cannot become a surface,
    /// [`RenderError::UnsupportedGraphics`] when the selected adapter cannot
    /// present to it, it has no sRGB format, or it supports neither opaque nor
    /// premultiplied compositor alpha, [`RenderError::Graphics`] for
    /// pipeline validation failures, and [`RenderError::Limit`] for a size beyond
    /// the device's texture dimension.
    pub fn new_surface(
        &self,
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let surface = self
            .instance
            .create_surface(target)
            .map_err(|e| RenderError::Surface(e.to_string()))?;
        if !self.adapter.is_surface_supported(&surface) {
            return Err(RenderError::UnsupportedGraphics(
                "selected adapter does not support the new surface".into(),
            ));
        }
        let config = surface_configuration(&surface, &self.adapter, width, height)?;
        let format = config.format;
        let pipelines = self.shared.pipelines(&self.device, format)?;
        let mut renderer = Self {
            shared: self.shared.clone(),
            instance: self.instance.clone(),
            surface,
            adapter: self.adapter.clone(),
            device: self.device.clone(),
            queue: self.queue.clone(),
            config,
            extent: None,
            pipelines,
            vertices: None,
            batches: Vec::new(),
            scene: None,
            rasterizer: Default::default(),
            encoded: Default::default(),
        };
        renderer.resize(width, height)?;
        Ok(renderer)
    }
}
