//! Collections and navigation: lists, tables, trees, filters, search, palettes,
//! file browser and history.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod collection_filter;
mod command_palette;
mod commands;
mod content_search;
mod file_browser;
mod history_navigator;
mod list_reorder;
mod multi_select;
mod navigation_history;
mod property_grid;
mod quick_pick;
mod reveal;
mod selection_notifications;
mod table;
mod table_columns;
mod tree;
mod tree_reorder;
mod virtual_collections;
mod virtual_list;
