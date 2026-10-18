//! Core facade contracts: values, permissions, phases, composition, host
//! adapters, inspection, capture and persistence.
//!
//! Each area links one test binary per feature configuration. Add a new test
//! file as a module of the area it covers rather than as a top-level file in
//! `tests/`, which Cargo would link as a binary of its own.

#[path = "../support/mod.rs"]
mod support;

mod arc_geometry;
mod async_source;
mod capture;
mod composition;
mod coordinates;
mod foundation;
mod host_adapter;
mod inspection_service;
mod key_repaint;
mod live_widgets;
mod owned_children;
mod permission;
mod phase_rules;
mod prelude;
mod profiling;
mod public_api;
mod readme;
mod receipts;
mod recovery;
mod redescription;
mod remote_authority;
mod semantic_names;
mod semantics;
mod serialization;
mod state_owner;
mod transforms;
mod vector_geometry;
