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
/// `App` and the types it reports. They follow one membership rule: the
/// widgets with their configuration types; state, parameters and layout; and
/// the [`Component`] trait with every type its method signatures name, so a
/// custom widget that lays out, draws and handles input needs no other import.
/// It includes errors returned by [`Component`] method signatures:
/// [`RuntimeError`], [`DrawError`], [`WorkspaceError`] and
/// [`RestoreRejection`]. Widget configuration also brings [`HexColorError`],
/// [`ListingError`] and [`InputRejection`] into the prelude, alongside
/// `AppError` and its shutdown payload `AppShutdownError` with `native`. Other
/// errors, and the inspection, capture, live-data and host-integration types,
/// stay at the crate root. Asset-loading errors such as `svg::SvgError` stay
/// in their feature modules.
///
/// The re-exports are `#[doc(no_inline)]`: each item is documented once, at
/// the crate root.
///
/// # Root names to add
///
/// rustc suggests no import for a name outside the prelude, so these are the
/// crate-root names code most often adds to the glob:
///
/// - keyboard shortcuts and commands: [`Modifiers`], [`KeyChord`],
///   [`CommandKey`], [`CommandScope`], [`CommandId`] and [`CommandContext`];
///   held keys: [`HeldInputId`] and [`HeldCancellation`];
/// - reordering a list, tabs or a tree: [`ListDrop`], [`DropPosition`],
///   [`ListDropPoint`], [`ListDropPlacement`] and [`ListDropMark`];
/// - live data: [`LevelMeter`], [`Trace`], [`MeterAxis`], [`SeriesSource`],
///   [`series_bridge`], [`SeriesConfig`] and [`Timestamp`]; a parameter edit's
///   end: [`EditOutcome`];
/// - a custom component that describes itself for inspection: [`Semantics`],
///   [`SemanticRole`], [`SemanticValue`], [`SemanticAction`], [`SemanticItem`],
///   [`InspectValue`], [`ValidationHints`] and [`BoxModelInspection`]; one that
///   starts a drag: [`DragPreview`] and [`DRAG_THRESHOLD`]; one that presents
///   in its own native window: [`NativeWindowOptions`], [`NativeWindowId`] and
///   [`PresentationReceipt`].
pub mod prelude {
    #[cfg(feature = "svg")]
    #[doc(no_inline)]
    pub use crate::Svg;
    #[cfg(feature = "native")]
    #[doc(no_inline)]
    pub use crate::{App, AppError, AppShutdownError, ErrorAction, NativeCloseAction};
    #[doc(no_inline)]
    pub use pert_ui_runtime::prelude::*;
}
