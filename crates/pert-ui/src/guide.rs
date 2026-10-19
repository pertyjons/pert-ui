//! A practical introduction to pert-ui, one chapter per area of the library.
//!
//! Start with [getting started](getting_started): a complete window, the data
//! contracts, a saved workspace and a custom widget. The other chapters can be
//! read in any order:
//!
//! - [State and parameters](state): signals, retained components, parameter
//!   gestures and host notifications, worker results, live data and recovery.
//! - [Layout and views](layout): region layouts, splits, tabs, floating and
//!   native windows, workspaces, large collections, zoomable content and drag
//!   and drop.
//! - [Controls](controls): buttons, boolean and choice controls, sliders, text
//!   and numeric editors, progress, inline controls and toolbars.
//! - [Surfaces](surfaces): selects, popovers, tooltips, menus, dialogs and
//!   notifications.
//! - [Appearance](appearance): themes, density, zoom, shapes, fonts, icons,
//!   images and SVG.
//! - [Diagnostics](diagnostics): keyboard input, inspection, screenshots,
//!   serialization and profiling.
//!
//! Every example is a doctest that imports only the public API. Each item's own
//! reference page holds its complete contract: defaults, limits, errors and
//! lifecycle. The repository's design notes explain decisions and also contain
//! proposals; they are not a list of available functions.

// Every chapter imports the crate root so that its prose links items by their
// plain names; the import is for rustdoc's link resolution only.
//
// Every chapter also appends the same link definitions, so its prose can name items
// behind optional features: an intra-doc link when the feature is enabled, and a
// docs.rs link (whose build enables them all) when it is not.

pub mod getting_started {
    //! Getting started: a first window, composition, data contracts, a saved
    //! workspace and a custom widget.
    //!
    #![doc = include_str!("guide/getting-started.md")]
    #![cfg_attr(feature = "native", doc = "```no_run")]
    #![cfg_attr(not(feature = "native"), doc = "```text")]
    #![doc = include_str!("../examples/getting_started.rs")]
    #![doc = "```"]
    #![doc = include_str!("guide/getting-started-continued.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod state {
    //! State and parameters: signals, retained components, parameter edits, worker
    //! results, live data and recovery.
    //!
    #![doc = include_str!("guide/state.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod layout {
    //! Layout and views: containers, region layouts, splits, tabs, windows,
    //! workspaces, collections, zoomable content and drag and drop.
    //!
    #![doc = include_str!("guide/layout.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod controls {
    //! Controls: actions, boolean and choice controls, sliders, text and numeric
    //! editors, progress, inline controls and toolbars.
    //!
    #![doc = include_str!("guide/controls.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod surfaces {
    //! Surfaces: selects, popovers, tooltips, menus, dialogs and notifications.
    //!
    #![doc = include_str!("guide/surfaces.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod appearance {
    //! Appearance: themes, density, zoom, shapes, fonts, icons, images and SVG.
    //!
    #![doc = include_str!("guide/appearance.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}

pub mod diagnostics {
    //! Diagnostics: keyboard input, inspection, screenshots, serialization and
    //! profiling.
    //!
    #![doc = include_str!("guide/diagnostics.md")]
    #![cfg_attr(feature = "native", doc = include_str!("guide/links/desktop.md"))]
    #![cfg_attr(not(feature = "native"), doc = include_str!("guide/links/no-desktop.md"))]
    #![cfg_attr(feature = "images", doc = include_str!("guide/links/images.md"))]
    #![cfg_attr(not(feature = "images"), doc = include_str!("guide/links/no-images.md"))]
    #![cfg_attr(feature = "svg", doc = include_str!("guide/links/svg.md"))]
    #![cfg_attr(not(feature = "svg"), doc = include_str!("guide/links/no-svg.md"))]
    #![cfg_attr(feature = "cpu", doc = include_str!("guide/links/cpu.md"))]
    #![cfg_attr(not(feature = "cpu"), doc = include_str!("guide/links/no-cpu.md"))]
    #[allow(unused_imports)]
    use crate::*;
}
