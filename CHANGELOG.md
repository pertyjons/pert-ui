# Changelog

## 0.3.1 (2026-10-18)

- The facade's integration tests are grouped by area into a few test binaries
  under `crates/pert-ui/tests/<area>/`, so each feature configuration links a
  few binaries instead of one per test file. The tests themselves are
  unchanged.
- CI builds the dev and test profiles without debuginfo or incremental caches,
  which kept filling the Linux runner disk and exceeded the Windows PDB limit.

## 0.3.0 — first prerelease (2026-10-15)

First public preview of pert-ui, a Rust desktop GUI library with retained
components, general controls, typed parameters, reactive bindings, inspection,
workspace persistence and a native gallery. The public facade supports GPU
rendering and optional CPU rendering and software presentation.

The API may change before 1.0. The libraries have not been published to a
registry.
