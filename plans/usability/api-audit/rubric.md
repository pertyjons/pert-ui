# API consistency audit: rubric

Slice 4 of [the developer-usability plan](../../archive/developer-usability.md). The
rubric says what the audit checks, where each rule comes from, how its
candidates are found and what counts as a finding. Codex reviews it and the
maintainer approves it before the audit starts.

## Sources and kinds of rule

Each rubric item says which kind of rule it applies:

- **Project rule.** AGENTS.md's "Component API conventions" and "Rust and
  data invariants", and [contracts](../../../docs/contracts.md): "Public
  naming rules", "Error policy", "Public compatibility policy", "User-write
  callbacks", "Component notifications", "Choice sources", "Permissions and
  availability", "Editor sessions", "Geometry notifications" and the family
  exceptions these sections name. A departure is a finding.
- **Guideline.** An item of the
  [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html),
  cited by its code and applied with the guideline's own qualifications. A
  departure is a finding unless a project rule decides otherwise.
- **Consistency proposal.** A convention neither source states, proposed
  because the API is inconsistent without it. Such findings are labelled as
  proposals; adopting one is a policy decision for the maintainer.

Where a project rule and a guideline differ, the project rule applies, and
the audit records the difference only if it misleads a consumer.

## Scope

- **Packages:** the library packages intended for publication
  (`pert-ui-core`, `pert-ui-runtime`, `pert-ui-cpu`, `pert-ui-wgpu`,
  `pert-ui`), in the configurations `api/generate.sh` lists, which are
  those CI checks.
- **In scope:** each rubric item's rule, including the behavior a rule
  states. Replacement against accumulation, permission composition,
  callback timing, validation, cancellation and notification order need
  the source and its tests, not only signatures.
  - A finding records which it established: **signature** (the shape
    departs) or **behavior** (the source or a test shows the behavior
    departs).
  - Behavior that no rubric item states is out of scope.
- **Out of scope:**
  - a general functional review;
  - performance;
  - documentation completeness. The `missing_docs` lint and slice 3 cover
    it, except `# Errors` coverage under T4.

## Method

### Inputs, frozen

`plans/usability/api-audit/inventory.py`, an unpackaged script, builds the
inputs. Both auditors get the same inputs, frozen at one commit.

**Per configuration,** the script:
1. builds the rustdoc JSON with the same command and flags as
   `api/generate.sh`, into its own directory per package and configuration
   (`target/api-audit/<package>/<configuration>/`), so nothing is
   overwritten;
2. records the provenance: commit (and whether the tree was clean),
   toolchain, JSON format version, target and feature arguments;
3. checks that the public paths it finds match that configuration's `api/`
   listing, and stops on a mismatch.

**Two kinds of output, kept apart:**
- **Mechanical candidate lists,** derived from the JSON, resolving
  re-exports, type aliases, concrete constructor return types, generic
  bounds and conditional trait implementations. They include, among others:
  - every public type with its traits;
  - every associated function returning `Self`, `Result<Self, _>`,
    `Option<Self>` or the concrete type;
  - every method taking `self` and returning `Self`;
  - every `on_*` method with its closure bounds and parameter names;
  - every function with a `bool` or bare numeric parameter or return;
  - every `#[must_use]` attribute with its reason;
  - every type appearing as the error of a public `Result`;
  - every `std::error::Error` implementation;
  - every `From` and `TryFrom` implementation;
  - every public field;
  - the `#[non_exhaustive]` attributes.
- **Semantic review lists,** seeded with **all** public types and their
  methods, not with what the script guesses. The auditor classifies each
  entry by role (controller, handle, receipt, value with an invariant,
  error carrier and so on), with a source reference, or marks it
  **unresolved**. The script never claims a role.

Some things need the source rather than the JSON: function bodies,
callback timing, replacement and composition, `#[error]` or `Display`
text, and private representation. The auditor reads them and cites
file:line.

### Two auditors

- **Who:** Codex, and a Claude agent in a fresh context.
- **Independence:** each works from the frozen inputs, without access to
  the other's report until both are submitted.
- **Completeness:** each auditor also checks the inputs. Both rely on one
  inventory, so a shared omission would otherwise go unseen. An auditor
  that finds an item the candidate lists miss reports it as an
  **inventory gap**, and the script is fixed and rerun before the
  cross-verification.
- **Disposition:** each auditor records a disposition for every rubric
  item: findings, no findings, or not assessable with the reason.

### Cross-verification and disputes

- **One-auditor findings:** a finding only one auditor reports is verified
  by the other, against the source.
- **Facts:** a disputed fact is settled by the author against the source.
  The record keeps the evidence and the resolution.
- **Policy:** a dispute about policy is not settled by the author. This
  covers whether a rule should hold, a new exception, or adopting a
  consistency proposal. It goes to the maintainer at the joint checkpoint.
- **Exceptions:** an exception cites the exact contract sentence and the
  behavior it covers. A family exception does not excuse an unrelated
  departure in the same family.

### Finding record

Each finding records:
- the rubric item and the kind of rule;
- the item path and the affected configurations;
- the observation with file:line, and whether it is signature or behavior;
- the rule it departs from;
- severity: how much it misleads or costs a consumer;
- the auditors' confidence;
- a suggested fix, and whether it is breaking;
- the destination if the checkpoint does not choose it: TODO.md,
  follow-ups or app-author slice 9, by the plan's rules.

## Rubric

### C. Construction

- **C1, choice sources** (project rule: AGENTS.md, contracts "Choice
  sources").
  - Every choice family offers its option sources as exclusive
    constructors: `new` for a fixed list, `bound` for a reactive list,
    `for_parameter` for a parameter specification.
  - No builder adds or replaces a source.
  - A fixed list is validated before adoption, where its validity is known
    locally.
  - A reactive list is validated during update.
  - An invalid reactive list keeps the last valid presentation.
  - Same-key redescription replaces value and option sources together.
  - Candidates: every choice family (all its constructors, including
    families with one) and every builder that takes options, a list or a
    specification.
- **C2, constructor names** (guideline: C-CTOR; consistency proposal for
  the prefixes).
  - Constructors are static inherent methods (C-CTOR).
  - Proposal: `try_*` means the fallible form of a constructor, `from_*`
    and `with_*` each keep one meaning across the API.
  - Legitimate named conversion constructors (`from_rgb` and the like) are
    not findings.
  - Candidates: the constructor list grouped by prefix.
- **C3, required state owners lead** (project rule: AGENTS.md).
  - A constructor that needs a state owner (`&impl StateOwner`, such as
    the `Runtime`) takes it first.
  - The state it binds (a `Signal`, `ReadSignal`, `ListSelection` or
    controller) follows the owner, if any, and comes before the other
    parameters.
  - Candidates: constructors whose parameters include such a type, with
    order and names from the JSON.
- **C4, validated values** (project rule: contracts "Public naming rules",
  AGENTS.md "Rust and data invariants"; guideline: C-NEWTYPE, C-VALIDATE).
  - A domain value with an invariant keeps it private and is built through
    a fallible constructor, with conventional `TryFrom` **where
    appropriate**; one entry point is enough.
  - Its raw value is read with `get()` unless the unit itself is the point
    of the type.
  - Documented limits are associated `MIN_*` or `MAX_*` constants **of the
    type they bound**, where the type has such a limit.
  - Non-finite and out-of-range inputs are rejected, not clamped, except
    for documented control-limit clamping.
  - Candidates: the semantic list, classified by the auditor as
    invariant-bearing or not.

### B. Builders and callbacks

- **B1, builders and permissions** (project rule: AGENTS.md, contracts
  "Public naming rules", "Permissions and availability").
  - Every ordinary property builder replaces its earlier value; only
    documented family exceptions accumulate. This is a behavior check.
  - A capability restriction takes `impl Into<Permission>` and composes
    with `Permission::and`.
  - A same-key redescription replaces the description's permission rather
    than accumulating old restrictions.
  - Revocation cancels an active interaction as the component documents.
  - Parameter editability stays separate from application enablement.
  - A boolean property takes `bool`, and its binding variant adds `_signal`
    and takes `ReadSignal<bool>`.
  - `disabled()` and `read_only()` are no-argument shortcuts.
  - A composite control documents when its retained descendants observe a
    redescription (for example, MultiSelect's rows and chips observe a
    permission change within the successful flush).
  - Candidates: every method taking `self` and returning `Self`, and every
    composite control.
- **B2, `#[must_use]`** (project rule: AGENTS.md "Use `#[must_use]` for
  results/handles/builders whose discard is a mistake").
  - Builders, handles, receipts and results whose discard is a mistake
    carry it, with a reason where the discard is a likely slip.
  - A value whose discard is normal does not. `CommandId` is slice 3's F19.
  - Candidates: every builder and every type the semantic list classifies
    as a handle or receipt, with its attribute.
- **B3, callbacks** (project rule: contracts "User-write callbacks",
  "Component notifications", "Editor sessions", "Geometry notifications";
  AGENTS.md "Component API conventions"). Checked per callback family:
  - **Timing of user-write callbacks (`on_change` and kin):**
    - they report a successful local write or a submitted proposal, never
      authority acceptance;
    - they are not called for application writes, unchanged values or
      refused edits;
    - a failure propagates without rollback or replay;
    - a redescribed handler replaces the old one, including removal.
  - **Component-owned selection and expansion notifications:**
    - they report their origin, after layout settles, through the public
      notification contract;
    - they are not confused with user-write callbacks.
  - **Editor drafts and geometry observation:** these follow their own
    documented contracts. Differences they explain are not findings.
  - **No signal is published from reconciliation, and no refused write is
    discarded silently.**
  - **Names:** within a family, one name means one timing. Across
    families a name may carry different timings where the applicable
    contract says so; for example, `Accordion::on_change` reports a settled
    event-phase notification while a boolean control's `on_change` reports
    a successful write. A difference no contract explains and that misleads
    a consumer is a finding. Further naming uniformity is a consistency
    proposal.
  - **Signature shapes,** per family: the closure's parameters (borrowed
    or owned value, origin), its return type, and how it receives the
    `EventContext`. A shape that differs from its family without a
    documented reason is a finding.
  - Candidates: every `on_*` method and every public notification type.
- **B4, role suffixes** (project rule: contracts "Public naming rules").
  - Cloneable UI-thread handles that drive a widget are `*Controller`.
  - Passive captured data are `*State`.
  - A queued request's report is a `*Receipt`, with `status()` and
    `status_binding()`.
  - The way an interaction ended is an `*Outcome`.
  - Candidates: the semantic list, classified by role; a type in a role
    under another suffix, or a suffix on a type in another role.

### N. Names

- **N1, getters** (guideline: C-GETTER; project rule for `get`).
  - No `get_` prefix.
  - `get` is legitimate for:
    - a validated value's raw content;
    - a cell-like read (`Signal::get`);
    - a single obvious value;
    - a keyed or indexed lookup.
  - Candidates: methods named `get_*` or `get`, classified by these uses.
- **N2, one word per concept** (consistency proposal; AGENTS.md "Do not
  change authority, callback timing or cancellation merely for spelling
  symmetry" limits the fixes).
  - The same concept has the same word across families.
  - Synonym sets to check, at least:
    - enabled / disabled / active;
    - visible / shown / hidden;
    - remove / delete / clear;
    - key / id;
    - value / current / selected;
    - change / changed / edit / modified;
    - open / show / reveal;
    - close / dismiss / hide;
    - begin / start, and end / finish / stop.
  - A finding: two words for one concept where a consumer could not tell
    that they mean the same, or one word for two concepts.
- **N3, units** (project rule: AGENTS.md "Give domain quantities …
  appropriate validated types").
  - A numeric parameter or return value with a unit says so, through its
    type (`Duration`, a length or angle type) or its name.
  - Candidates: functions with bare numeric parameters or returns, with
    parameter names from the JSON.
- **N4, boolean parameters** (guideline: C-CUSTOM-TYPE).
  - A non-builder `bool` parameter whose meaning the call site cannot show
    is an enum.
  - Candidates: non-builder functions with a `bool` parameter, and every
    function with two or more.
- **N5, Rust conventions** (guidelines: C-CASE, C-CONV, C-ITER, C-ITER-TY,
  C-WORD-ORDER, each with its own scope).
  - Case conventions (C-CASE).
  - `as_`, `to_` and `into_` by cost and ownership (C-CONV).
  - The iteration methods a homogeneous collection supports follow the
    `iter`, `iter_mut` and `into_iter` names and item ownership (C-ITER).
    `iter_mut` is not required where mutation would break the
    collection's invariants. Their iterator types match the methods'
    names (C-ITER-TY).
  - Words in a consistent order across related names, such as error types
    (C-WORD-ORDER). The verb-object-error order is the guideline's
    example, not a mandate.

### T. Types

- **T1, common traits** (guidelines: C-COMMON-TRAITS, C-DEBUG,
  C-DEBUG-NONEMPTY).
  - Every public type implements `Debug`, and a hand-written `Debug` is
    never empty; the auditor checks custom implementations.
  - Value types implement `Clone`, `Copy`, `PartialEq`, `Eq`,
    `PartialOrd`, `Ord`, `Hash` and `Default` where their meaning permits.
  - User-facing values implement `Display`.
  - A finding: a missing trait the type's meaning supports and a consumer
    would plausibly need, for example a key or identity type without
    `Hash`. Conditional implementations count with their bounds.
- **T2, `Send` and `Sync`** (guideline: C-SEND-SYNC; project rule:
  AGENTS.md "Scheduling and real-time boundaries").
  - `Send` and `Sync` are assessed separately against each type's
    ownership and rustdoc.
  - UI-thread types are neither.
  - Data that moves between threads is `Send`.
  - `Sync` is required only where shared references cross threads.
  - Candidates: the auto-trait lines of the listings.
- **T3, `#[non_exhaustive]`** (project rule: contracts "Public
  compatibility policy" for error enums; consistency proposal beyond them).
  - A growing public error enum is `#[non_exhaustive]`, and a deliberately
    closed enum says so in its rustdoc (rule).
  - Other enums and structs with public fields that are expected to grow
    are reported as proposals.
  - Candidates: public enums, and structs with public fields, with their
    attribute and any "closed" statement.
- **T4, errors** (project rule: contracts "Error policy"; guideline:
  C-GOOD-ERR). Every type in an error position of a public `Result` is
  checked, whatever its name:
  - **Type and traits:** it is typed and implements `std::error::Error`.
    Library-owned errors derive `Clone`, `Debug`, `PartialEq` and `Eq`
    where their data permits; one retaining an I/O source derives only
    `Debug`.
  - **`Display`:** it begins lowercase, has no trailing period, and says
    what failed, with which value and, where the caller can act, what to
    do.
  - **Causes:**
    - a wrapped error is transparent or exposed through `source()`, so a
      chain prints each cause once;
    - foreign backend failures may become diagnostic strings, while
      library causes stay typed.
  - **Application failures:** they go through `RuntimeError::Application`,
    which keeps the source for downcasting, and library variants do not
    stand in for host failures.
  - **Conversions:**
    - they point upward, with one normalized representation at each
      level;
    - the callback-boundary exceptions are as contracts lists them;
    - a shared limit has one reporting variant.
  - **Documentation:** every public `Result` has an `# Errors` section
    naming its variants and conditions, including forwarded failures.
  - Slice 3's F11, F23 and F30 belong here.
- **T5, public fields** (guideline: C-STRUCT-PRIVATE; project rule:
  AGENTS.md).
  - Public fields only on plain data without an invariant.
  - Candidates: structs with public fields.
- **T6, conversion traits** (guideline: C-CONV-TRAITS; consistency
  proposal for the last point).
  - Conversions implement `From`, `TryFrom`, `AsRef` and `AsMut` where they
    apply.
  - Named conversion methods remain legitimate.
  - Proposal: an inherent method that duplicates a trait implementation
    under a confusable name (an inherent `from_str` beside `FromStr`) is
    reported.

### D. Discovery

- **D1, paths** (project rule: AGENTS.md "Ordinary application code uses
  the public facade").
  - The facade's public modules (`cpu`, `images`, `svg` and the others)
    satisfy the rule. A module path is not a finding by itself.
  - A finding is demonstrated friction: an item that the probes or round 1
    needed, found late or only through a deep path.
  - The fix may be a re-export, a link or a guide pointer; layout
    proposals go to app-author slice 9.
- **D2, prelude** (project rule: the prelude's own membership rule in its
  rustdoc).
  - The prelude holds:
    - the widgets, containers and surfaces, with the types their builders
      take and what their controllers and status values return;
    - state and parameters;
    - the layout, theme, typography, keyboard and shape types an
      application configures;
    - `Component` with every type its signatures name, and the event enums
      and drawing types a component uses;
    - the errors its rustdoc lists.
  - Inspection, profiling, capture, live-data plumbing and other
    host-integration types stay at the root, by the same rule.
  - Each name the probes and round 1 imported from the root is checked
    against this rule, with feature conditions.
  - A finding is a name the rule includes but the prelude lacks, or the
    reverse. The prelude's size and shape are app-author slice 9's decision.
  - Slice 3's F3 belongs here.

## Surveys

### Lints

- **Minimum set:**
  - `missing_debug_implementations`;
  - `unreachable_pub`;
  - `clippy::must_use_candidate`;
  - `clippy::return_self_not_must_use`;
  - `clippy::missing_panics_doc`;
  - `clippy::fn_params_excessive_bools`;
  - `clippy::struct_excessive_bools`.

  Others may be added with a reason.
- **How each runs:** as a warning in a throwaway configuration (`cargo
  clippy --locked --workspace --all-targets` with `-W <lint>`, or
  `RUSTFLAGS` for rustc lints; never committed), in each CI feature
  configuration where its hits can differ.
- **What is kept:** the commands, the configurations and the raw output.
- **Classifying hits:** each hit is read and classed as real, intended
  exception or noise. Duplicate hits across configurations are counted
  once.
- **Per lint, the survey records:**
  - the real hits;
  - the exceptions it would need, each later an
    `#[expect(..., reason = ...)]`;
  - its noise;
  - whether it is worth denying.

### Diagnostics

- **Minimum matrix:**
  - **Traits consumers implement:** the common mistakes for each
    (`Component`, `ListItems`, `TreeItems`, `TableItems`, `ContentRows`,
    `Clipboard`).
  - **The sealed `StateOwner`:** trying to implement it.
  - **`IntoElement`:** a missing import, a missing `Component`
    implementation, and an attempted own implementation.
  - **Slice 3's misleading diagnostics.**
- **Baseline:** each case is reproduced in a throwaway consumer against the
  facade, and its exact message is recorded.
- **Prototype:** where `#[diagnostic::on_unimplemented]` can apply, which
  is only for unsatisfied trait bounds and not for import, coherence or
  sealed-trait errors, a prototype in a scratch copy (never committed)
  records the new message.
- **Outcome:** for each case, whether the attribute helps, and the cases it
  cannot help.
- **Testing:** `compile_fail` doctests check that a mistake is rejected;
  message wording needs snapshot tests such as `trybuild`, run only on the
  pinned toolchain. The survey recommends one for each case.

## Outputs

- `plans/usability/api-audit/`:
  - the script;
  - the frozen inputs, with their provenance;
  - each auditor's report, with a disposition for every rubric item;
  - the cross-verification record;
  - the two surveys.
- A merged audit findings table in the plan beside slice 3's, with the
  finding record's fields, for the joint checkpoint with the maintainer.
