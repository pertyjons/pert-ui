# codex verification: G5

Source prefixes below: **R** = `crates/pert-ui-runtime/src/`, **C** = `crates/pert-ui-core/src/`, **F** = `crates/pert-ui/src/`, **U** = `plans/usability/runs/`.

Repository unchanged. Verification was read-only; no compilation or compiler experiments were run.

## The other auditor's findings

| Other ID | Matched by | Verdict | Evidence | Remarks on severity, kind or fix |
|---|---|---|---|---|
| claude-D1-1 | none | **partly confirmed** | `U/r1-2-claude/diagnostics.md:4` records the misleading `Notifier` suggestion; `FRICTION.md:92` records resolving `Modifiers` through documentation. `R/lib.rs:448` omits it, while `F/lib.rs:29` exposes runtime root items. The observed discovery friction holds. The claim that **no re-export layout can produce an import suggestion** is broader than this evidence; I did not independently reproduce the reported toy experiments. | Low severity and D1 project-rule classification are appropriate. Prelude additions or documentation pointers address the demonstrated problem. Treat the universal compiler claim as unverified here. The related F20 aside agrees with `codex-D1-1`, but is a different departure. |
| claude-D2-1 | partly: `codex-D2-1` matches every listed command/shortcut type; `HeldInputId` and `HeldCancellation` are unmatched | **confirmed** | `R/runtime/held.rs:51` describes UI-owned physical-key capture; `:71` takes a cancellation callback over `HeldCancellation` and returns `HeldInputId`. Neither appears in the prelude beginning at `R/lib.rs:328`; both are root exports at `:479`. They belong to the keyboard configuration family included by `:306`. | Low severity, project rule, additive exports are appropriate. I correct my earlier classification of `HeldInputId` as host plumbing. The source supports adding both held-input names. |
| claude-D2-2 | `codex-D2-1`: all four names | **matched** | `R/virtual_list.rs:970` names `ListDrop` in a builder callback; `:1046` names `ListDropPoint` and `ListDropPlacement`; `R/list_drag.rs:81` returns `DropPosition`. | Agree: low severity, project rule, additive exports. |
| claude-D2-3 | partly: `codex-D2-1` matches `LevelMeter`, `Trace`, `MeterAxis`; the `SeriesSource` policy claim is unmatched | **partly confirmed** | The widgets and builder argument are present at `R/live_widgets.rs:131`, `:163`, `:481`. Their constructors take `SeriesSource` at `:148` and `:493`. That source manages bridge draining and subscriptions (`R/live.rs:600`), covered by the explicit live-data-plumbing exclusion at `R/lib.rs:320`. | Agree on the three omissions, low severity and additive fix. The existing exclusion explains keeping `SeriesSource` at the root; changing that exception is a maintainer policy choice, not another established membership violation. |
| claude-D2-4 | partly: `codex-D2-1` matches `WorkspaceValue`, `CommandKey`, `Modifiers`; the inspection/helper names and documentation claim are unmatched | **partly confirmed** | `R/diagnostics.rs:646` exposes the four named `ComponentDetails` field types; semantics depend on the further enums (`:464`). `R/validation.rs:89` uses `OverlapPolicy`. `R/runtime/drag.rs:132` requires `DragPreview`; `R/drag.rs:15` exposes the recommended threshold. These support the overbroad wording at `F/lib.rs:54` and `F/root.md:40`. However, inspection is expressly excluded (`R/lib.rs:320`), and the rule does not require every `EventContext` helper or free constant. | Low severity is appropriate. Confirm a documentation qualification problem and a **policy question** about expanding closure. Do not classify all listed absent names as established project-rule omissions. Narrowing the promise is justified; adding excluded inspection types requires the checkpoint decision. |
| claude-D2-5 | partly: `codex-D2-1` matches `PowerExponent`, `EditOutcome`, `ParameterSnapshot`, `PreviewOutcome`; the host-family uncertainty is unmatched | **partly confirmed** | The four omissions match. `ParameterValue` also directly bounds `Parameter` (`R/parameter_binding.rs:283`), and `EditSession::token` returns `EditToken` (`:730`): neither is exclusively delivery plumbing. Conversely, sink errors describe application-adapter failures (`R/parameter_binding/delivery.rs:48`), and delivery monitors retain sink reports (`:238`), fitting the host-integration exclusion at `R/lib.rs:320`. | Agree on low severity and adding the four matched names. Maintain my inclusion finding for `EditToken` and `ParameterValue`. Keep the eight delivery-specific types under the existing exclusion; revising that boundary is policy, not a proven omission. |
| claude-D2-6 | `codex-D2-1`: every listed name | **matched** | Representative direct obligations: `Label::turned` takes `QuarterTurn` (`R/controls.rs:186`); `Painter::draw_mask` takes `MaskImage` (`R/drawing.rs:1487`); `TextField::session` takes `EditorSession` (`R/text_fields.rs:805`). The remaining instances also occur in my omission table. | Agree: low severity and project rule. Additive exports are appropriate; the direct builder/controller/drawing clauses already justify these instances. |
| claude-D2-7 | none | **partly confirmed** | `F/app.rs:465` reports `NativeWindowId`; `:480` reports `WindowError`. Both are absent from `F/lib.rs:71`. The “App and the types it reports” promise at `:51` supports the ID omission. `AppShutdownError` is exported at `:71`, despite the incomplete error list at `:59`. `WindowError` encounters that same “other errors” exclusion. | Low severity is appropriate. Confirm adding `NativeWindowId` under `native` and correcting the error documentation. `WindowError`’s absence is factual, but its required membership needs resolution of the conflicting clauses. Retain `AppShutdownError`; it is an App-reported failure (`F/app.rs:98`). |
| claude-D2-8 | none; my report records the same exclusions without a finding | **partly confirmed** | The exclusions are explicit at `R/lib.rs:318` and `F/lib.rs:59`; CPU rendering is correctly exposed through its feature module (`F/cpu.rs:1`). Actual capture/CPU references occur in **six** round 1 submissions. The seventh root-glob importer, `U/r1-1-codex/submission/src/main.rs:1`, makes root names available without using capture or CPU rendering. | Correctly a policy question, with no required fix or established departure. Qualify the usage counts: unused wildcard availability is not demonstrated use. A testing prelude remains a slice 9 proposal. |

## My findings disputed

- **`codex-D2-1`, held-input disposition — correct.** Withdraw the classification of `HeldInputId` as host plumbing. `Runtime::register_held_input` configures UI keyboard ownership and names both held-input types (`R/runtime/held.rs:51`, `:71`). Include `HeldInputId` and `HeldCancellation` among the omissions.

- **`codex-D2-1`, inspection/helper exclusions — maintain, with a documentation correction.** Inspection remains explicitly excluded (`R/lib.rs:320`). Neither every `EventContext` argument nor `DRAG_THRESHOLD` is expressly required by the membership clauses. Nevertheless, the unconditional “needs no other import” wording (`F/lib.rs:55`) overpromises for supported custom-component work. Accept that documentation finding; leave broader closure to the maintainer.

- **`codex-D2-1`, parameter classification — maintain.** `ParameterValue` bounds the ordinary binding (`R/parameter_binding.rs:283`); `EditToken` is returned by its local edit session (`:730`). Their delivery uses do not erase these local parameter roles. Delivery-specific sink/report types remain covered by the host-integration exception.

- **Native facade disposition — correct in part.** My blanket exclusion of `NativeWindowId` overlooked the facade’s additional “App and the types it reports” clause (`F/lib.rs:51`, `F/app.rs:465`). Correct it for configurations enabling `native`. Maintain `AppShutdownError`’s membership, but acknowledge the incomplete error documentation. Leave `WindowError`’s conflicting inclusion/exclusion wording for the checkpoint.

## Inventory delta

Processed the version 2 additions and changes relevant to G5:

| Inventory material | Entries processed |
|---|---:|
| Type membership annotations | 534 |
| New free-item list | 30 |
| Added semantic fields and variants | 1,540 across 251 types |
| Added/changed semantic method rows | 295 |
| Newly listed fallible builders | 49 |
| Newly listed closure-taking functions | 64 |

Counts overlap across lists. `App::software_rendering` now has its conditional semantic entry; it reveals no further discovery departure. The free items reveal no new D1/D2 finding. One additional parameter-state omission emerged.

### codex-D2-d1 `EditSource` is absent from the parameter prelude

- **Rubric/rule:** D2, **project rule**.
- **Item:** `pert_ui_core::EditSource`, re-exported as `pert_ui_runtime::EditSource` and `pert_ui::EditSource`.
- **Configurations:** core/runtime default and serde; all eleven inventoried facade configurations.
- **Observation — signature:** the newly listed field `ParameterSnapshot::edit_source: Option<EditSource>` appears in `inputs/semantic/pert-ui-core.txt:163`; source confirms it at `C/edit.rs:118`. Ordinary bindings return this snapshot (`R/parameter_binding.rs:316`), and `EditToken::source` also returns `EditSource` (`C/edit.rs:96`). Runtime exports it at `R/lib.rs:286`, but its prelude omits it.
- **Rule:** `R/lib.rs:304` includes state, parameters and bindings. `EditSource` identifies the origin of a local edit (`C/edit.rs:47`); it is not exclusively transport or inspection plumbing.
- **Severity:** **low** — additional imports when reading or comparing edit ownership.
- **Confidence:** **high**.
- **Suggested fix:** add `EditSource` to the runtime prelude’s core exports. **Additive**; check downstream glob-import ambiguity.
- **Destination if not chosen:** **TODO.md** for the membership deviation; broader prelude organization belongs to **app-author slice 9**.