# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. Take each entry's time from the shell (`date +%H:%M`);
do not estimate it.

## Entries

### <time> — <category> — <severity>

- **Category:** one of
  - *discover*: could not find how to do it;
  - *understand*: found it, but it was unclear how it behaves;
  - *compile*: a compiler diagnostic that did not point to the fix;
  - *runtime*: behavior differed from the documentation or your expectation;
  - *docs-wrong*: the documentation is incorrect;
  - *missing*: there seems to be no way to do it;
  - *workaround*: you did it in a way you think the library did not intend;
  - *source*: you read the library's source (say why).
- **Severity:** *blocker* (you could not continue without it), *slow* (it
  cost noticeable time), *annoyance*.
- **Tried:** what you did.
- **Expected:** what you expected to happen or to find.
- **Resolved by:** what resolved it, or "unresolved".
- **Read:** the documentation pages or items you read for it.
- **Diagnostic:** for *compile*, the exact error code and the first lines.

### 15:53 — discover — annoyance

- **Tried:** Looked for a testing chapter and guessed `struct.PointerInput.html` and `enum.PointerEvent.html` while learning input simulation.
- **Expected:** A single headless testing walkthrough with input, lookup and assertions.
- **Resolved by:** The guide index points to `guide/diagnostics/index.html`; `Runtime::pointer_input` links to `struct.PointerEvent.html`. The diagnostics examples cover keyboard and captures, but I assembled pointer input and semantic assertions from separate reference pages.
- **Read:** `/kit/doc/pert_ui/index.html`, `guide/index.html`, `guide/diagnostics/index.html`, `struct.Runtime.html`, `struct.PointerEvent.html`, `struct.Semantics.html`.

### 15:53 — understand — annoyance

- **Tried:** Compared the existing Cargo dependency (`0.3.51`, already enabling `cpu`) with the documentation version (`0.3.52`).
- **Expected:** The supplied app and docs to name the same version.
- **Resolved by:** Updating the manifest requirement to documented `0.3.52`, retaining defaults and `cpu`; no application behavior or testability changes.
- **Read:** `/work/bookmarks/Cargo.toml`, `/kit/doc/pert_ui/index.html`.

### 15:54 — understand — annoyance

- **Tried:** Planned row assertions from the README's `list` description and considered using only widget keys to identify fields.
- **Expected:** Rows directly under `list`, and unique keys to be sufficient.
- **Resolved by:** `Each` documents its default `items` column; tests enumerate that column to check row order. `name` repeats in rows, so tests first narrow lookup to `name-field`, and match inspection entries with `Runtime::inspection_id`.
- **Read:** `/kit/doc/pert_ui/struct.Each.html`, `struct.Runtime.html#method.find_descendant`, `struct.Runtime.html#method.inspection_id`, `struct.ComponentInspection.html`.

### 15:55 — runtime — annoyance

- **Tried:** Required mutation check after the first `cargo test --offline --manifest-path /work/bookmarks/Cargo.toml` passed all eight integration tests without a display.
- **Expected:** `add_appends_and_trims_name` must fail if trimming is broken.
- **Resolved by:** Temporarily replacing `name: entered_name.trim().to_owned()` with `name: entered_name.to_owned()` in the application; running the exact test next. This deliberate behavior change will be restored immediately after recording the result.
- **Read:** `/work/bookmarks/README.md` behavior 1 and `/work/bookmarks/src/lib.rs`; no library source read.

### 15:55 — runtime — annoyance

- **Tried:** `cargo test --offline --manifest-path /work/bookmarks/Cargo.toml --test ui add_appends_and_trims_name -- --exact` with name trimming deliberately removed.
- **Expected:** The test detects untrimmed displayed names through runtime inspection.
- **Resolved by:** The test failed (exit 101), showing row 3's name as `"  Example  "` instead of `"Example"`. Restored `src/lib.rs` byte-for-byte from a pre-mutation copy. Full output retained at `/work/bookmarks/mutation-failure.txt`.
- **Read:** `/work/bookmarks/tests/ui.rs` and `/work/bookmarks/src/lib.rs`.
- **Diagnostic:**

```text
Compiling bookmarks v0.1.0 (/work/bookmarks)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 26.95s
     Running tests/ui.rs (bookmarks/target/debug/deps/ui-54557f0af9941109)

running 1 test
test add_appends_and_trims_name ... FAILED

failures:

---- add_appends_and_trims_name stdout ----

thread 'add_appends_and_trims_name' (218) panicked at tests/ui.rs:117:5:
assertion `left == right` failed
  left: [("1", "Rust", "https://www.rust-lang.org"), ("2", "Crates", "https://crates.io"), ("3", "  Example  ", "https://example.com"), ("4", "Plain HTTP", "http://example.org")]
 right: [("1", "Rust", "https://www.rust-lang.org"), ("2", "Crates", "https://crates.io"), ("3", "Example", "https://example.com"), ("4", "Plain HTTP", "http://example.org")]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    add_appends_and_trims_name

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.06s

error: test failed, to rerun pass `--test ui`
```

## Closing

- **Result:** All acceptance items met. Seven behavior tests and one CPU screenshot test pass under `cargo test --offline --manifest-path /work/bookmarks/Cargo.toml` (8 passed, 0 failed). The screenshot is `/work/bookmarks/captures/bookmarks.png` (900×700); visually inspected and contains the initial main-window form, status and both rows. The required temporary removal of name trimming produced the recorded assertion failure, exit 101; source was restored and the complete suite passed again. No persistent application source or behavior changes and no testability changes were needed. Only the dependency version requirement changed to the documented 0.3.52, keeping defaults plus `cpu`. No library source was read.
- **Concepts I had to learn:** Mount a shared tree in `Runtime`; set a headless `Viewport` and flush; distinguish native command routing from component-only keys; report primary modifiers; focus and commit text; inspect semantic names and values by incarnation id; scope repeated widget keys; enumerate `Each`'s default `items` container; simulate pointer press/release using allocated bounds; declare capture support, request a window capture, serve it after flushing, encode PNG.
- **What worked well:** The diagnostics guide's native keyboard and CPU capture examples worked directly. Bundled fonts made headless rendering straightforward. Typed semantics provided clear assertions without reading private signals or invoking app callbacks. `find_descendant` avoided internal wrapper paths; reference pages explicitly explained ambiguity and `Each`'s container. The first compiled test suite passed without source-reading workarounds.
- **The three changes I want most:**
  1. Add a dedicated end-to-end testing walkthrough combining field replacement, button clicks, ordered list assertions and screenshots.
  2. Provide documented reusable input helpers for replacing text and clicking a revealed widget, including modifier/key cleanup.
  3. Keep fixture dependency requirements aligned with the supplied documentation version, and include a key-tree example for composites such as `Each` and `Field` in testing guidance.
