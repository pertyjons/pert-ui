//! The registry README and the crate page share their status, feature and
//! license sections through `src/status.md`; this keeps the copies identical,
//! and keeps the guide's dependency lines and Rust version in step with the
//! package.

#[test]
fn readme_carries_the_crate_page_status_sections() {
    let readme = include_str!("../../README.md");
    let status = include_str!("../../src/status.md");
    assert!(
        readme.contains(status),
        "README.md must contain src/status.md verbatim; copy it again after editing either"
    );
}

/// The getting-started chapter names the dependency and the minimum Rust
/// version a registry user needs; this keeps both in step with the package.
#[test]
fn getting_started_names_the_current_release_and_rust_version() {
    let guide = include_str!("../../src/guide/getting-started.md");
    let version = env!("CARGO_PKG_VERSION");
    let release = release();
    let dependency = format!("pert-ui = {{ version = \"{release}\" }}");
    assert!(
        guide.contains(&dependency),
        "getting-started.md must depend on `{dependency}` for version {version}"
    );
    let rust = format!("requires Rust {} or later", env!("CARGO_PKG_RUST_VERSION"));
    assert!(
        guide.contains(&rust),
        "getting-started.md must say that pert-ui {rust}"
    );
    let status = include_str!("../../src/status.md");
    assert!(
        status.contains(&format!(
            "Requires Rust {} or later",
            env!("CARGO_PKG_RUST_VERSION")
        )),
        "status.md must name the package's rust-version"
    );
}

/// The version requirement a registry user writes: the package's major and
/// minor version.
fn release() -> String {
    env!("CARGO_PKG_VERSION")
        .splitn(3, '.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".")
}

/// Every dependency line the guide gives names the same requirement as
/// getting started, with the features its page needs.
#[test]
fn guide_dependency_lines_name_the_current_release() {
    let release = release();
    let pages = [
        (
            "getting-started.md",
            include_str!("../../src/guide/getting-started.md"),
        ),
        (
            "appearance.md",
            include_str!("../../src/guide/appearance.md"),
        ),
        (
            "diagnostics.md",
            include_str!("../../src/guide/diagnostics.md"),
        ),
        ("state.md", include_str!("../../src/guide/state.md")),
        (
            "authoritative-values.md",
            include_str!("../../src/guide/authoritative-values.md"),
        ),
        ("layout.md", include_str!("../../src/guide/layout.md")),
        ("controls.md", include_str!("../../src/guide/controls.md")),
        ("surfaces.md", include_str!("../../src/guide/surfaces.md")),
        (
            "custom-widgets.md",
            include_str!("../../src/guide/custom-widgets.md"),
        ),
        (
            "getting-started-continued.md",
            include_str!("../../src/guide/getting-started-continued.md"),
        ),
    ];
    let prefix = "pert-ui = { version = \"";
    for (name, page) in pages {
        for (at, _) in page.match_indices(prefix) {
            let requirement = &page[at + prefix.len()..];
            assert!(
                requirement.starts_with(&format!("{release}\"")),
                "{name} must require version \"{release}\": {}",
                requirement.lines().next().unwrap_or_default()
            );
        }
    }
    for (name, page, line) in [
        (
            "appearance.md",
            pages[1].1,
            format!("pert-ui = {{ version = \"{release}\", features = [\"png\", \"jpeg\"] }}"),
        ),
        (
            "diagnostics.md",
            pages[2].1,
            format!("pert-ui = {{ version = \"{release}\", features = [\"serde\"] }}"),
        ),
    ] {
        assert!(page.contains(&line), "{name} must give `{line}`");
    }
}
