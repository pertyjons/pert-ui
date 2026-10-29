# Round 3 triage follow-up input probes: P12–P14

These are author-written probes against the frozen kits, written from fresh context and driven by the leads in `r3-menu-diagnostics-design-review1/result.md`.

They are not blind testing, independent review, acceptance or native/human evidence. The primary repository, the kits, the submissions and the earlier `r3-followup-probes-evidence` were not written. That directory's `source/lib.rs` still has sha256 `c1aeefed…`, and no file in it is newer than its README.

## Execution

- **Source.** One file, `source/lib.rs` (final, sha256 `c7eae42f…`), compiled unchanged on both kits. It imports only the public facade: `pert_ui::*`, `pert_ui::host::HostIntegration` and `pert_ui::testing::Driver`.
  - Manifests: `source/Cargo.head.toml` (`pert-ui = "=0.3.88"`, `["cpu"]`) and `source/Cargo.baseline.toml` (`"=0.3.75"`, `["cpu"]`).
  - The exact compiled inputs, including lockfiles, are in `source/as-run-{head,baseline}/`.
- **Harness.** Every cargo command ran as `plans/usability/run.sh exec <kit> <probe-dir> …` through `scripts/run-probe.sh`. The wrapper refuses to overwrite a log and records the command, `kit.env`, input sha256s, full output and `exit_status`.
  - Head: `kit-r3-head`, commit `32dd9c4f`, dirty=0, 0.3.88, rustc 1.99.1. Probe dir: `r3-extra-input-probes-head`.
  - Baseline: `kit-r3-baseline`, commit `26e6b5a3`, **dirty=1** as prepared, 0.3.75. Probe dir: `r3-extra-input-probes-baseline`.
- **Iterations.** All are kept; all exited 0. Only my own probe changed between them.
  - iter1 (`source/iterations/lib.iter1.rs`): builds and tests on both kits. The P14 scroll fixture was wrong on my side. The ScrollArea sat in a `Column` that gave it its content height, so it never overflowed, `on_scroll` never fired and the queued work never ran. iter1's scroll lines therefore show nothing.
  - iter2 (`lib.iter2.rs`): the ScrollArea is mounted as the root, so it fills the viewport, and logs are printed after one extra settle.
  - iter3 (final): adds the Tabs-based P14 experiment.
- **Comparisons.** `runs/*-iterN-observations.txt` hold the extracted `P1x|` lines.
  - Head and baseline are **byte-identical** in every iteration.
  - P12 and P13 lines are identical across all iterations.
- **Assertions** check only scope: F10 focused `title/file`, a menu or popup opened.
- **Vendor sources.** `vendor-source-checksums.txt` covers the cited files. Identical across the kits: `menu_bar.rs`, `select.rs`, `runtime/reveal.rs`, `runtime/pointer.rs` and the facade's `platform/motion.rs`. `multi_select.rs` differs only in signal-combine syntax (lines 408 and 440–448). The other cited files differ elsewhere; the cited text is the same.

## Verdicts

| Lead | Verdict | Scope |
|---|---|---|
| P12 MenuTitle switch | **Verified on both kits.** The actual open menu and the command run both change. | Inside the menu repair scope |
| P13 MultiSelect rows | **Verified on both kits.** Worse than menus: keyboard navigation is stuck. | Outside scope; TODO suggestion |
| P14 `Hidden{target}` fallback | **Not reached by any legitimate path tried.** Reached only by an out-of-contract host call. A related legitimate case returns `Err(RuntimeError::Hidden)`. | No TODO for the fallback; one documentation lead, verified |

## P12: a refreshed Move switches the open menu bar menu

**Fixture.** File = [Save (a command row), Beta], View = [Zoom], over content buttons. The title bounds are File `[0,0,47,32]` and View `[47,0,55,32]`; the View middle is (74.5, 16).

| Pointer | After F10 | After Down | Enter ran |
|---|---|---|---|
| never reported | focus `title/file` | File open, View closed, focus in File's rows (`items#6`), active Save | `command:Save` |
| host Move to the View middle while the bar is closed | focus `title/file` | **File `open=false`; View `open=true expanded=Some(true)`, rows visible, focus in View's rows (`items#10`), active Zoom** | **`row:Zoom`** |
| the same Move sent twice (duplicate host event) | as above | as above | `row:Zoom` |
| Move, then Leave | | File open | `command:Save` |
| Move to the File middle | | File open | `command:Save` |
| never; host Move to View *after* the keyboard opened File | | File, then View, on that genuine Move (documented pointer switching) | `row:Zoom` |
| …a second identical Move | | no further change | `row:Zoom` |
| Move, Leave, keyboard-open File, then a host Move at the **same** coordinates as the earlier park | | switches to View | `row:Zoom` |

**Mechanism.** `MenuTitle::pointer_input` calls `core.set_open(self.key)` on any `PointerKind::Move` inside the title while another menu of the bar is open (`menu_bar.rs:386-390`, identical in both kits). Opening File changes layout, so the runtime's refreshed Move at the parked position reaches the View title.
- As in P8, the switch is already visible right after the raw `key_input` Down, before the probe's own settle.
- The guide's F10/Down/Enter recipe (`guide/surfaces.md:160`) thus runs another menu's row.

**Disposition: in scope for the menu repair** (not implemented).
- A refreshed Move must never switch titles; hover feedback may still update.
- The last two rows matter for the repair. A host Move at an unchanged position also switches. The native adapter queues and dispatches every motion without comparing positions (`pert-ui/src/platform/motion.rs:25-37`; read from source only, native not exercised).
- So gating on `is_refresh()` alone leaves a native same-position Move able to switch. Gate on the pointer having moved since the bar last saw it (position change), or record why that is unnecessary per platform.
- Regression tests: each row of the table, including Leave and parked-over-File, a genuine switch after opening, and the duplicate same-position Move.

## P13: MultiSelect rows (separate control family)

**Fixture.** `MultiSelect::new` over 4 options (Alpha–Delta) or 20 (O01–O20), with `on_change` logging. The trigger is focused, Down opens the popup (focus moves to `search`), then further Downs, Enter and Escape. Rows were located through inspection `border_box`; calibration confirmed that a host Move at each row's middle highlights that row.

| Pointer (before opening) | Highlight after opening | Further Downs | Enter: value / `on_change` |
|---|---|---|---|
| never | Alpha | Down → Beta | `["alpha"]`; with one Down `["beta"]` |
| parked over Gamma | **Gamma** | **Down → still Gamma** | **`["gamma"]`** (also with one Down) |
| Gamma, then Leave | Alpha | | `["alpha"]` |
| outside the popup | Alpha | | `["alpha"]` |
| long list, never | O01 | 12 Downs → O13; the list scrolls from O09 | `["o13"]` |
| long list, parked over the O03 row | **O03** | **12 Downs → O03 every time**; the list never scrolls | **`["o03"]`** |
| long list, parked then Leave | O01 | → O13 | `["o13"]` |

**Mechanism (source).** `Row::pointer_input` writes the shared highlight on every `Move` inside a row that is not highlighted, with no `is_refresh()` check (head `multi_select.rs:592`, baseline `:591`).
- In this control, keyboard navigation is **blocked**, not just reset, even in the four-row list that cannot scroll. Each Down's change is undone by the row under the stationary pointer.
- I did not trace the exact invalidation chain that produces the refresh.
- The documented behaviour is "Up and Down move the highlight … and Enter toggles the highlighted option" (head `multi_select.rs:1156`). Enter instead toggles the row under a pointer the user never moved, and `on_change` reports it as the user's write.

**Calibration note.** Inspection marks overscan rows as `visible`. Rows O09–O12 have boxes below the eight-row popup, and Moves there land outside it and change nothing. The `shown=` ranges count mounted rows, not painted ones. The parked point, O03's row, is inside the painted area.

**Disposition: outside the current menu repair scope; suggested TODO.md entry** (not written; the repository was out of bounds):

> `- [ ] 2026-10-28 crates/pert-ui-runtime/src/multi_select.rs Row::pointer_input — a refreshed or stationary pointer Move over a popup row overrides keyboard navigation: with the pointer resting on a row, the popup opens on that row, Up/Down cannot move off it, and Enter toggles it (frozen 0.3.88 and 0.3.75 probes). Apply the same pointer-ownership rule as the menu OptionList repair and add parked-pointer, Leave and long-list regression tests.`

## P14: `RevealRefusal::Hidden { component: target }` fallback

**Source.** `hiding_refusal` (`runtime/reveal.rs:655-681`, identical in both kits) runs only after `reveal_focus` finds the target hidden again (`:629-632`). Its first two branches cover any node with `local_visible == false` (explicit `set_visible`) and any ancestor whose `child_visible` is false. The fallback therefore needs `visible == false` with neither cause.
- In the runtime that leaves native window visibility (`set_window_visible` → `refresh_visibility(root, visible)`, `native_windows.rs:362-375`) and detach/redock. Both are host operations.
- If such a state already exists *before* the reveal, the walk ends with `Err(RuntimeError::Hidden)` (`reveal.rs:614-623`), not the fallback.
- **This is a source analysis of the frozen code, not proof that no other route exists.**

**Probes, identical on both kits:**

| Case | Result |
|---|---|
| explicit `set_visible(group, false)` (control) | `Refused(Hidden { component: group(2) })`; `Driver::click` → `NotRevealed … Hidden { … serial: 2 … }` |
| the host reports the window hidden (`HostIntegration::set_window_visible(false)`), a legitimate minimized window | `reveal` and `reveal_and_focus` → **`Err(Hidden)`**; `Driver::click` → `Runtime(Hidden)`, "component is hidden". After `set_window_visible(true)`: `Revealed` |
| ScrollArea target: `on_scroll` (`Reveal`, y=672) defers `set_visible(area, false)`, `unmount(area)` or a host hide | `reveal` → `Revealed` every time. The deferred work runs only at the next flush, after the reveal returned (`tasks.rs:516-518`: tasks queued during delivery wait for the next flush) |
| Tabs: the target is in unselected tab b inside a ScrollArea; `on_select` (Reveal, a→b) defers work into the same reveal | `reveal_child(&tabs, "a")` → `Refused(AncestorRefused { tabs(1) })`; `set_visible(area, false)` → `Refused(Hidden { area(7) })`; control → `Revealed` |
| **BOUNDARY, outside the host contract:** the same, the deferred work calls `HostIntegration::set_window_visible(false)` | **`Refused(Hidden { component: target(39) })`** |

**Reading the boundary row.**
- This is the fallback: neither the target nor any ancestor is explicitly hidden, and no container refuses.
- It required application work to report a native visibility that no platform reported. `set_window_visible` "supplies **known** native presentation visibility for the selected root" (head `host.rs:238`, baseline `:239`).
- It is not a legitimate path, and I make no claim that it is a defect.
- In this Tabs fixture `on_scroll` did not report, so I don't claim from evidence which flush inside the reveal ran the work. The source places the fallback after `reveal_focus`.

**Disposition.**
- The fallback was **not reached by any legitimate path tried**. No TODO is proposed for it. Diagnostic tests should keep using explicit `set_visible` for `Hidden`.
- **Separate verified observation (both kits):** a reveal into a window the host reports hidden returns `Err(RuntimeError::Hidden)`, not a `RevealOutcome::Refused`.
  - That error is not among `Runtime::reveal`'s documented errors ("errors of the flushes, of `reveal_child` and of `reveal`").
  - `RuntimeError::Hidden` is documented as "The component is hidden and cannot receive an editing event" (head `runtime.rs:325`, baseline `:322`).
  - The code comment at `reveal.rs:615-616` assumes visibility is then "only waiting to be applied", which a minimized window contradicts.
  - Suggested follow-up for the maintainer to decide: document this error for `reveal`, `reveal_and_focus` and `Driver::click`, or make it a typed refusal. Do not reuse `Hidden { component: target }` for it, since that variant means an explicit `set_visible`.

## Paths

- Evidence: `/home/per/pert-ui-usability/r3-extra-input-probes-evidence/`
  - `README.md`
  - `source/`: final source, both manifests, `iterations/`, `as-run-*`
  - `runs/`: per kit and iteration build and test logs plus observations
  - `scripts/run-probe.sh`
  - `vendor-source-checksums.txt`
- Probe dirs: `/home/per/pert-ui-usability/r3-extra-input-probes-{head,baseline}/`.

No CLI configuration or transcripts are archived.
