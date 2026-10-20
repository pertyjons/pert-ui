//! Standard and numeric controls: buttons, links, chips, choices, collapsibles,
//! forms, pickers and parameter controls.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod activation;
mod breadcrumb;
mod chip;
mod choice_controls;
mod choice_permissions;
mod collapsible;
mod color_picker;
mod component_adornments;
mod component_availability;
mod component_bound_choices;
mod component_button_presentation;
mod component_callbacks;
mod component_choice_sources;
mod component_format_retention;
mod component_workflows;
mod control_permissions;
mod control_reconcile;
mod field_status;
mod focus_within;
mod forms;
mod inline_numbers;
mod knob;
mod link;
mod number_field_options;
mod number_inputs;
mod numeric_controls;
mod parameter_controls;
mod readout_formats;
mod segmented_control;
mod tab_traversal;
mod toolbar;
