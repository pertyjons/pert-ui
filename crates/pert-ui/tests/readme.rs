//! The registry README and the crate page share their status, feature and
//! license sections through `src/status.md`; this keeps the copies identical.

#[test]
fn readme_carries_the_crate_page_status_sections() {
    let readme = include_str!("../README.md");
    let status = include_str!("../src/status.md");
    assert!(
        readme.contains(status),
        "README.md must contain src/status.md verbatim; copy it again after editing either"
    );
}
