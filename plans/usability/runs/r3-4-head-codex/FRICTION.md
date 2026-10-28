# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

## Entries

### 17:18 — runtime — annoyance

- **Category:** runtime (deliberate regression experiment; not a library defect).
- **Severity:** annoyance; this entry records the required negative check, not unexpected library friction.
- **Tried:** Temporarily changed `bookmarks/src/lib.rs` from `name: entered_name.trim().to_owned()` to `name: entered_name.to_owned()`, then ran `cargo test --manifest-path /work/bookmarks/Cargo.toml --offline --test ui add_appends_and_trims_name -- --exact`.
- **Expected:** The public-input add test should detect the lost name trimming.
- **Resolved by:** The test failed as expected (exit 101). Restoring the original source next, then rerunning all tests. Full failure output: `/work/mutation-test.log`.
- **Read:** `/work/bookmarks/README.md`, behavior 1; `/kit/doc/pert_ui/guide/testing/index.html`, “Typing and replacing text” and “Reading what the runtime shows”.
- **Diagnostic:**

  ```text
  test add_appends_and_trims_name ... FAILED
  thread 'add_appends_and_trims_name' (291) panicked at tests/ui.rs:92:9:
  assertion `left == right` failed
    left: "  Docs  "
   right: "Docs"
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out
  ```

## Closing

- **Result:** All acceptance items met. Seven integration tests in `/work/bookmarks/tests/ui.rs` cover the seven README behaviors through `Driver` input and semantic/address inspection; an eighth captures the main window with `CpuRenderer` and writes `/work/bookmarks/captures/bookmarks.png` (900 × 700). `cargo test --manifest-path /work/bookmarks/Cargo.toml --offline` passes all eight without a display. The deliberate mutation failed as recorded above; original application source was restored byte-for-byte and the complete suite passed again. Evidence: `/work/first-test.log`, `/work/mutation-test.log`, `/work/final-test.log`. `cargo fmt --check` also passes.
- **Application changes:** None retained. No testability changes were necessary; the existing public `build` function suffices. The temporary mutation changed behavior only for the required negative check and was reverted. The existing dependency already had default features plus `cpu` at version 0.3.88.
- **Concepts I had to learn:** Headless `Runtime` and `Viewport`; mounting the existing builder; `Driver::settle`; key paths relative to the mounted root; semantic names versus editable `SemanticValue::Text` (empty label name is `None`); simulated text replacement and pointer clicks; `address_children` for ordered `Each` rows; platform-primary keyboard chords; capture support, queued capture requests and CPU service before `try_reply`; PNG encoding.
- **What worked well:** The crate landing page links directly to the testing chapter. Its examples answered every library question needed here. `testing::*` plus `prelude::*` provided all test types. The chapter explicitly explains empty label semantics, transparent `Each` address structure, command routing and capture ordering, avoiding trial and error. All eight tests compiled and passed on the first attempt. No vendor source was read, and no unexpected library difficulty arose.
- **The three changes I want most:** No blocking documentation or API changes were identified in this exercise. Optional improvements: (1) a reusable example test fixture for an existing app builder, (2) small semantic text assertion helpers to reduce repeated value matching, (3) a convenience capture-to-PNG helper bundling support declaration, request, settling and CPU serving. These are ergonomic suggestions, not observed defects.
