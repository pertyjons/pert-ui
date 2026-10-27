//! Text entry and draft editing: text and search fields, input normalization,
//! adornments and inline editors.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod begin_edit;
mod binding_refusal;
mod draft_editor;
mod draft_policy;
mod editor_permissions;
mod editor_sessions;
mod inline_controls;
mod inline_draft_editing;
mod input_adornments;
mod input_controls;
mod input_normalization;
mod input_notifications;
mod input_states;
mod open_selected;
mod search_field;
mod selectable_text;
mod text_fields;
