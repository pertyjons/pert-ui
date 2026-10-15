# TODO

Verified deviations from [AGENTS.md](AGENTS.md) or the design notes, and
concrete improvements, found outside the scope of the task that noticed them.
See "Reporting deviations and improvements" in AGENTS.md for the entry format
and lifecycle. Entries are suggestions for the maintainer to prioritize.

- [ ] **NATIVE-01** 2026-09-20 `docs/verification.md` — Verification and review:
  native gallery save/restart/restore passed on Fedora GNOME/Wayland, but Windows,
  macOS and the remaining Linux combinations in
  [native evidence](docs/verification.md#native-evidence) still lacks restart checks.
  Run the documented two-process check and record the results before shipping
  support for those platforms; headless checks do not replace native execution.
- [ ] **NATIVE-02** 2026-09-21 `docs/verification.md` — Verification and review: Fedora 44
  GNOME/Wayland has the injected-pointer native check and a hand check
  (2026-09-26), but Windows and macOS have no native evidence for the drag
  session or the tab drag-out: run `platform::drag_smoke` and the hand check on
  the gallery's Drag & drop page there and record the results in the evidence record.
- [ ] **NATIVE-03** 2026-10-05 `docs/verification.md` native evidence — Verification and review: floating tool-window frames and own native windows now have Linux evidence (Fedora 44 GNOME/Wayland, 2026-10-05) through the native gallery interaction check, but Windows and macOS have none. Run `cargo run --locked -p pert-ui-native-checks -- interaction` and the hand check listed in the evidence table there and record the results.
- [ ] **NATIVE-04** 2026-10-08 `apps/gallery/src/interaction_check.rs` and
  `apps/gallery/src/workspace_check.rs` — Verification and review: both probes
  now pace with update deadlines, report a hidden window immediately, and the
  native runner ends that process with the reported cause. Headless hide and
  resume tests and normal Fedora 44 GNOME/Wayland GPU and CPU runs pass. The
  actual Fedora minimize path still needs a native interruption run to verify
  that the compositor reports hiding and the runner returns the cause rather
  than an unrelated assertion or timeout. On macOS 26.7, activating Finder
  during `--interaction-check` previously stalled for an unidentified cause;
  that platform also needs a separate interruption check.
