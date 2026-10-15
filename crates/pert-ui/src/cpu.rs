//! Rendering on the CPU, without a window or GPU (feature `cpu`).
//!
//! [`CpuRenderer`] turns a [`DisplayList`](crate::DisplayList) from
//! [`Runtime::paint`](crate::Runtime::paint) into an owned
//! [`FrameImage`](crate::FrameImage) with the same rendering model as the
//! native hardware renderer. As a capture host it answers screenshot requests
//! of a window, part of one ([`CaptureTarget::Region`](crate::CaptureTarget::Region)),
//! a component or a key path, including those an
//! [`InspectionClient`](crate::InspectionClient) sends from another thread, so a
//! headless tool or test can see what the application shows.
//!
//! With the default `desktop` feature, `App::software_rendering` also presents
//! these frames in native windows instead of using the GPU.
//!
//! ```
//! use pert_ui::cpu::CpuRenderer;
//! use pert_ui::{CaptureRequest, CaptureTarget, Element, Label, Runtime, Viewport};
//! use std::time::Duration;
//!
//! let mut ui = Runtime::new();
//! ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
//! ui.mount_tree(Element::new("greeting", Label::new("Hello")))?;
//! ui.flush(Duration::ZERO)?;
//! ui.set_capture_support(true);
//! let window = ui.primary_window().id();
//! let pending = ui.request_capture(CaptureRequest::new(CaptureTarget::Region {
//!     window,
//!     x: 0.0,
//!     y: 0.0,
//!     width: 120.0,
//!     height: 40.0,
//! }));
//! ui.flush(Duration::ZERO)?;
//! CpuRenderer::new().serve_captures(&mut ui);
//! let png = pending.wait()?.value?.image.encode_png();
//! assert!(png.starts_with(b"\x89PNG"));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#[doc(inline)]
pub use pert_ui_cpu::{CpuRenderError, CpuRenderer};
