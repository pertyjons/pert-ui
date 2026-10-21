# Lint survey

Part of slice 4's audit (rubric, "Surveys: Lints"), run on 2026-10-21 at
commit d4ba108's library source with Rust 1.99.1.

## How it ran

[`run.sh`](run.sh) runs the rubric's seven lints as warnings through
`cargo clippy --locked --message-format=json -- -W <lint>...` in a separate
target directory, once per CI Clippy configuration:
- the workspace with `--all-targets`;
- `pert-ui-core --features serde`;
- `pert-ui-runtime --features serde`;
- each of the facade's eleven CI feature configurations.

Clippy lints every workspace member a configuration builds, so the facade
configurations also lint the runtime and core with unified features.
[`hits.py`](hits.py) reduces the messages to one line per hit. The raw hits
are in `raw/<configuration>.tsv`. Nothing changes the workspace's lint
configuration.

## Result

The configurations add no hit the workspace run lacks: 414 distinct hits in
all. They contain no `return_self_not_must_use` hit and no
`fn_params_excessive_bools` hit.

| Lint | Hits | Real | Exceptions | Noise | Deny? |
|---|---|---|---|---|---|
| `missing_debug_implementations` | 108 | 108 | 0 | 0 | yes, after the fixes |
| `unreachable_pub` | 199 | 0 | 0 | 199 | no |
| `clippy::must_use_candidate` | 79 | 79 (consistency) | 0 | 0 | maintainer's choice |
| `clippy::return_self_not_must_use` | 0 | – | – | – | yes, as a guard |
| `clippy::missing_panics_doc` | 1 | 0 | 1 | 0 | yes, with one `#[expect]` |
| `clippy::fn_params_excessive_bools` | 0 | – | – | – | yes, as a guard |
| `clippy::struct_excessive_bools` | 27 | 0 | 2 | 25 | no |

### `missing_debug_implementations`

Every hit is a public library type without `Debug`. C-DEBUG and rubric
item T1 want `Debug` on every public type.
- **Where:** 105 in `pert-ui-runtime`, 2 in the facade (`App`, `Svg`) and 1 in
  `pert-ui-wgpu` (`Renderer`).
- **What they are:** mostly widget builders and composite controls
  (`Button`, `Chip`, `RadioGroup`, `Table`, `FloatingWindow` and the
  like), plus a few controllers and contexts.
- **The fix:** many of them hold closures, so a derive does not compile.
  They need hand-written `Debug` that prints the configuration and names
  the callbacks without formatting them. The rubric's non-empty-`Debug`
  rule applies.
- **Classification:** all are real. Whether to fix them is part of T1's
  findings. Once fixed, `deny` keeps new types honest. Add it to
  `[workspace.lints.rust]`.

### `unreachable_pub`

The library has no hits. All 199 are in the packaged examples' private
modules (154) and in the facade's shared test support (45), where `pub`
inside a private module is harmless.
- **Classification:** noise for the public API.
- **Deny:** no. Denying it workspace-wide would only force churn in
  examples and tests.

### `clippy::must_use_candidate`

The hits are 78 pure getters and one constructor (`CommandKeyInput::new`,
`runtime/commands.rs:298`) without `#[must_use]`. Examples:
`ParameterId::key`, `Mailbox::pending`, `RemoteCommit::request`, the
`status_binding` methods, `TextSelection::anchor` and `ThemeMetrics::get`.
- **The convention they break:** the public API already marks 296 `&self`
  getters `#[must_use]`, according to the inputs' attribute list.
- **Classification:** real as consistency hits. AGENTS.md's rule ("whose
  discard is a mistake") does not by itself require them. They belong
  with rubric item B2.
- **Deny:** the maintainer's choice. Either decide that pure getters carry
  `#[must_use]` and deny the lint, or leave getters to judgment and drop
  the lint. Clippy keeps the lint in its pedantic group for this reason.

### `clippy::missing_panics_doc`

The one hit is `ThemeMetrics::for_density`
(`runtime/theme.rs:255`). Its `assert!`s are the compile-time
completeness check that AGENTS.md asks for in keyed default tables. The
same `const fn` runs at run time, but only over the fixed table, so the
assertion cannot fail for a caller.
- **Classification:** an intended exception, as
  `#[expect(clippy::missing_panics_doc, reason = "...")]`.
- **Deny:** worth denying, since the rest of the API has no undocumented
  panic.

### `clippy::struct_excessive_bools`

- **Public fields:** only 2 of the 27 structs have public `bool` fields.
  - `Semantics` (`diagnostics.rs:464`) has six inspection flags.
  - `Modifiers` (`input.rs:285`) has the four modifier keys.

  Both are deliberate plain data, so they are exceptions. Slice 3's
  misleading `Modifiers::CONTROL` diagnostic (no hint at the fields)
  concerns discovery, not the field shape.
- **The rest:** 25 are private state or the private fields of builders,
  including one in the gallery. They are internal design, not API.
- **Deny:** no.

### Guards

`return_self_not_must_use` and `fn_params_excessive_bools` have no hits.
Denying them costs nothing and keeps builders `#[must_use]` and
non-builder functions free of bool lists. This matches rubric items B2
and N4.

## Recommendation for the joint checkpoint

- **Deny now:**
  - `clippy::return_self_not_must_use`;
  - `clippy::fn_params_excessive_bools`;
  - `clippy::missing_panics_doc`, with its one `#[expect]`.
- **Deny with the fixes:** `missing_debug_implementations`, after T1's
  `Debug` work.
- **Choose:** `clippy::must_use_candidate`, a policy on pure getters.
- **Leave off:** `unreachable_pub` and `clippy::struct_excessive_bools`.
