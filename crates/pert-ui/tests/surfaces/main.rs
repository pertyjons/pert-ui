//! Surfaces and arrangement: menus, popups, transients, tool and native
//! windows, splits, tabs, drag and drop and workspaces.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod deferral;
mod detach_permissions;
mod drag_and_drop;
mod geometry_callbacks;
mod menu_changes;
mod menus;
mod multi_window;
mod native_titles;
mod pointer_items;
mod popups;
mod resize_grips;
mod splits;
mod tab_notifications;
mod tab_reorder;
mod tab_selection;
mod tabs;
mod toast_items;
mod tool_window_modes;
mod tool_window_moves;
mod tool_windows;
mod transient_reconcile;
mod transients;
mod views;
mod window_close;
mod window_geometry_callbacks;
mod windows;
mod workspace;
