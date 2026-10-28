# Friction log

Copy this file to `/work/FRICTION.md`. Add one entry per event, in time
order, as it happens. When something costs you time, run `date +%H:%M` at
that moment and write the entry before you continue; do not estimate a
time. If you do write an entry later, give it the time you write it and add
`(written later)` after the time.

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

### 16:55 — understand — annoyance

- **Tried:** Followed the settings-form recipe to compute dirty state from edited values.
- **Expected:** Comparing values alone would cover unsaved edits and Revert.
- **Resolved by:** The recipe explains that normalized text and integer editors retain refused drafts even when the bound value is unchanged. Track `on_modified` with `FormFields`, include its summary in unsaved state, and explicitly discard drafts on Revert.
- **Read:** `/kit/doc/pert_ui/guide/state/index.html#a-settings-form`, `/kit/doc/pert_ui/guide/testing/index.html#typing-and-replacing-text`, `/kit/doc/pert_ui/struct.Field.html`.

### 16:57 — understand — annoyance

- **Tried:** Added a text normalizer using a shared `fn valid_prefix(&str)`, and used a `PartialEq` enum for `SegmentedControl` as in the controls guide.
- **Expected:** The closure argument to behave like a borrowed string; a signal-compatible enum to suffice for choices.
- **Resolved by:** Borrowed the normalizer's owned `String` and derived `Eq` for the enum. Compiler suggestions gave both fixes. No source was opened.
- **Read:** `/kit/doc/pert_ui/guide/state/index.html#a-settings-form`, `/kit/doc/pert_ui/guide/controls/index.html#joined-exclusive-buttons`.
- **Diagnostic:** `E0308: mismatched types` (`expected &str, found String`); `E0277: the trait bound Format: Eq is not satisfied`; cascading `E0599: pert_ui::SegmentedControl<Format> is not a pert-ui component`.

### 16:57 — compile — annoyance

- **Tried:** Derived the marker label with `dirty.map(|d| if *d { "Unsaved changes".into() } else { "All changes saved".into() })`.
- **Expected:** Passing the result to `Label::new` to infer a string.
- **Resolved by:** Annotated the result as `ReadSignal<String>`. `TextSource` accepts both string and rich-text signals, so `.into()` is ambiguous.
- **Read:** `/kit/doc/pert_ui/guide/controls/index.html#text-and-actions`.
- **Diagnostic:** `E0283: type annotations needed for pert_ui::ReadSignal<_>`; `multiple impls satisfying TextSource: From<pert_ui::ReadSignal<_>> found`.

## Closing

- **Result:** All five acceptance items met. `cargo test --offline -- --nocapture` passes two tests using `App::headless`, `Driver` input/semantics and `HeadlessApp::admit_close`. They show all loaded fields and group titles; invalid prefix and quality refused with visible descriptions and previous valid bindings; button availability and Revert; JSON save, deterministic failure and retry; and dirty-close refusal with an apply/revert message. `cargo check --offline` also passed. No display was used. No library source was opened.
- **Concepts I had to learn:** Runtime and keyed Elements; Signal and derived ReadSignal; draft commits on Enter/focus loss; InputRejection and FieldStatus; FormFields and on_modified; background spawn/delivery; shared App state and close admission; Driver and semantic inspection.
- **What worked well:** The settings-form recipe closely matches this task and explicitly covers the difficult equal-value draft/Revert case. The testing chapter supplies runnable examples for real input, worker waits and close policy. Field propagates error text and invalid state to the editor, making both presentation and headless assertions straightforward. Compiler suggestions fixed the initial type mismatches without source inspection. The default-features application runs headlessly without a display or GPU.
- **The three changes I want most:**
  1. Show the normalizer argument type (`String`) explicitly in the guide example, since untyped closures obscure ownership when extracting validation helpers.
  2. State `Clone + Eq` beside the guide's generic choice example, with a small user-defined enum example.
  3. Include a derived conditional text-label example using an explicit `ReadSignal<String>` or `to_owned()`, avoiding the `.into()` inference trap between string and rich-text sources.
