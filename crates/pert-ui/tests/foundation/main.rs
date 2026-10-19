//! Core facade contracts: values, permissions, phases, composition, host
//! adapters, inspection, capture and persistence.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod application_services;
mod arc_geometry;
mod async_source;
mod callback_conveniences;
mod capture;
mod click_counter;
mod composition;
mod coordinates;
mod described;
mod failure_sites;
mod flex_layout;
mod foundation;
mod host_adapter;
mod inspection_service;
mod key_repaint;
mod live_widgets;
mod owned_children;
mod permission;
mod phase_rules;
mod pointer_refresh;
mod prelude;
mod profiling;
mod public_api;
mod readme;
mod receipts;
mod recovery;
mod redescription;
mod remote_authority;
mod row_stretch;
mod semantic_names;
mod semantics;
mod serialization;
mod signal_writes;
mod sink_follow_ups;
mod startup_viewport;
mod state_owner;
mod transforms;
mod vector_geometry;
