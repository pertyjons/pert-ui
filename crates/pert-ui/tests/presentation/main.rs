//! Visual presentation: theme, colors, density, typography, fonts and icons,
//! images, SVG, text sources and zoom.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod appearance;
mod body_hit_regions;
mod bound_background;
mod colors;
mod density;
mod display_text_limits;
mod focus_ring;
mod fonts_icons;
mod hover_feedback;
mod images;
mod images_bomb;
mod library_label_sources;
mod name_sources;
mod placeholder_sources;
mod progress;
mod scrollbar_emphasis;
mod svg;
mod text_overflow;
mod text_source;
mod theme;
mod title_sources_collapsible;
mod title_sources_elements;
mod title_sources_fields;
mod title_sources_windows;
mod typography;
mod ui_strings;
mod zoom;
mod zoom_view;
