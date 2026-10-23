// The crate page is rustdoc-specific (intra-doc links, hidden doctest lines);
// README.md is the registry page. Both carry status.md, which tests/foundation/readme.rs
// keeps identical.
#![doc = include_str!("root.md")]
#![doc = include_str!("status.md")]
#![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
#![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
#![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
#![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
#![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
#![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
#![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
#![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]

pub mod guide;

// Runs the registry README's example as a doctest; it opens a window, so it
// needs `desktop`.
#[cfg(all(doctest, feature = "native"))]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(all(feature = "native", not(any(feature = "gpu", feature = "cpu"))))]
compile_error!(
    "the `native` feature needs a renderer: enable `desktop` (GPU) or `desktop-cpu` (CPU)"
);

#[doc(inline)]
pub use pert_ui_runtime::*;
#[cfg(feature = "native")]
mod app;
#[cfg(feature = "cpu")]
pub mod cpu;
#[cfg(feature = "images")]
pub mod images;
#[cfg(feature = "native")]
mod platform;
#[cfg(feature = "svg")]
pub mod svg;
// Placement: every widget, including a feature-gated one, is at the crate root;
// a feature module holds its loaders, rasterizers and their errors.
#[cfg(feature = "native")]
pub use app::{App, AppError, AppShutdownError, ErrorAction, NativeCloseAction};
#[cfg(feature = "svg")]
pub use svg::component::Svg;

/// The names ordinary application code and a custom component write, for
/// `use pert_ui::prelude::*`.
///
/// This includes the runtime prelude and enabled feature widgets. With the
/// `native` feature, which `desktop` and `desktop-cpu` enable, it also includes
/// `App` and everything it reports, errors included. They follow one
/// membership rule: the widgets with their configuration types; state,
/// parameters, local and remote hosts, and layout; and the [`Component`]
/// trait with every type its method signatures name, so a custom widget that
/// lays out, draws and handles input needs no other import. It includes errors returned by
/// [`Component`] method signatures: [`RuntimeError`], [`DrawError`],
/// [`WorkspaceError`] and [`RestoreRejection`]. Widget configuration also
/// brings [`HexColorError`], [`ListingError`] and [`InputRejection`] into the
/// prelude, and so do the errors an application constructs to report its own
/// failure, [`SinkError`] and [`ApplicationError`]; with `native` it holds
/// `AppError`, its shutdown payload `AppShutdownError` and the [`WindowError`]
/// that `App::on_window_error` reports. Other errors, and the inspection
/// queries and records, capture, live-data plumbing and host transport, stay at
/// the crate root; a live widget's [`SeriesSource`], and the bridge that feeds
/// it, stay with the live-data plumbing. Asset-loading errors such as
/// `svg::SvgError` stay in their feature modules.
///
/// A custom component that describes itself for inspection, starts a drag,
/// keeps a receipt, presents in its own native window or hosts an editor
/// session also globs [`component`], which holds every name implementing
/// [`Component`] writes. The re-exports are `#[doc(no_inline)]`: each item is
/// documented once, at the crate root.
///
/// # Root names to add
///
/// These are the crate-root names code most often adds to the glob:
///
/// - live data: [`SeriesSource`], [`series_bridge`], [`SeriesConfig`] and
///   [`Timestamp`];
/// - a custom component's inspection description, drag preview, receipts and
///   native windows: `use pert_ui::component::*` (see [`component`]).
pub mod prelude {
    #[cfg(feature = "svg")]
    #[doc(no_inline)]
    pub use crate::Svg;
    #[cfg(feature = "native")]
    #[doc(no_inline)]
    pub use crate::{
        App, AppError, AppShutdownError, ErrorAction, NativeCloseAction, NativeWindowId,
        WindowError,
    };
    #[doc(no_inline)]
    pub use pert_ui_runtime::prelude::*;
}

/// The names implementing [`Component`] writes, for
/// `use pert_ui::component::*`.
///
/// Applications glob [`prelude`]; code that implements [`Component`] also
/// globs this module, which re-exports `pert_ui_runtime::component`. Two globs
/// that re-export one item do not conflict, so the names both hold, such as
/// [`Component`] and [`Painter`], can be imported through either.
///
/// It holds [`Component`], its contexts and every name an implementation has
/// to write: an argument the component constructs, such as a [`DragPreview`];
/// an enum it matches, including the status of a receipt it keeps; a field or
/// payload it builds or destructures for a [`Component`] method, such as the
/// [`Semantics`] and [`InspectValue`] parts of the [`ComponentDetails`] that
/// [`Component::inspect`] returns; and a value it keeps where the rustdoc says
/// to keep it, such as a [`TaskReceipt`] or an [`EditorAttachment`]. A value
/// only passed on, compared or converted in the call that returned it, such as
/// [`FittedText`], is not required. Profiling, capture, inspection queries and
/// records, live-data plumbing and host transport stay at the crate root. The
/// re-exports are `#[doc(no_inline)]`: each item is documented once, at the
/// crate root.
///
/// ```
/// use pert_ui::{component::*, prelude::*};
///
/// /// A marker that names itself for inspection.
/// struct Marker;
///
/// impl Component for Marker {
///     fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         Ok(())
///     }
///     fn inspect(&self) -> ComponentDetails {
///         ComponentDetails {
///             semantics: Some(Semantics::new(SemanticRole::Image).name("marker")),
///             ..ComponentDetails::default()
///         }
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.mount_tree(Element::new("marker", Marker))?;
/// # Ok::<(), RuntimeError>(())
/// ```
pub mod component {
    #[doc(no_inline)]
    pub use pert_ui_runtime::component::*;
}
