//! Compiler-message snapshots for pert-ui and the `testing` module's closure
//! check; this crate exports nothing.
//!
//! `tests/diagnostics.rs` compiles each case under `ui/` against the facade
//! and compares rustc's output with the committed `.stderr` beside it. The
//! cases cover the messages pert-ui shapes through
//! `#[diagnostic::on_unimplemented]` and `#[diagnostic::do_not_recommend]`.
//! rustc's wording changes between releases, so the test is ignored in
//! ordinary runs and a separate CI job runs it with the toolchain
//! `rust-toolchain.toml` pins:
//!
//! ```sh
//! cargo test --locked -p pert-ui-diagnostics -- --ignored
//! ```
//!
//! A toolchain bump or a changed message regenerates the snapshots by
//! running the same command with `TRYBUILD=overwrite` set, and the `.stderr`
//! diff is reviewed like the `api/` listing.
//!
//! `tests/testing_closure.rs` checks from the committed API listings under
//! `api/` that every type the names of `pert_ui::testing` and
//! `pert_ui_runtime::testing` hand a test is in `testing` or the prelude,
//! except a reviewed list of records a test reads by field or passes on. It
//! is not ignored and runs in `cargo test --locked --workspace`; the API
//! listing gate keeps its input current.
