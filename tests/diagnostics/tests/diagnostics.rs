//! Compares the compiler messages of the cases under `ui/` with their
//! committed `.stderr` snapshots; see the crate documentation.

#[test]
#[ignore = "compiler wording; run by the diagnostics job on the pinned toolchain"]
fn compiler_messages_match_their_snapshots() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("ui/*.rs");
}
