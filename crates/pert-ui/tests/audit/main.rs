//! Adverse-path audits of input, notification, layout, presentation and window
//! contracts, and the regressions found by review sweeps.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod audit_activation;
mod audit_collection_permissions;
mod audit_conformance;
mod audit_input;
mod audit_layout;
mod audit_modal_queries;
mod audit_navigation_input;
mod audit_notification_failures;
mod audit_notification_order;
mod audit_numeric;
mod audit_physical_keys;
mod audit_presentation;
mod audit_toasts;
mod audit_tool_stacking;
mod audit_tool_stripe;
mod audit_window_cleanup;
mod audit_workspace_capture;
mod sweep_coverage;
mod sweep_input;
mod sweep_lifecycle;
mod sweep_text;
