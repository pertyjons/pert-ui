//! Acceptance verifier test entry point (added in the acceptance copy only).
//!
//! The submission is a binary crate, so its private `app()` and `Editor`
//! are not reachable from an integration test. This entry point compiles the
//! unchanged `src/main.rs` as a module with `include!` and puts the
//! verifier's checks inside it as a child module. No application source is
//! edited and application behavior is unchanged; `main` is simply unused.
#![allow(dead_code)]

mod notes {
    include!("../src/main.rs");

    mod verifier {
        include!("../acceptance_tests/verifier.rs");
    }
}
