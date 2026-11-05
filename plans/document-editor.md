# Plan: large-document text editing (TEXT-01)

Status: design plan, accepted by the maintainer as direction (2026-11-04);
not scheduled; TEXT-01 stays open. The raster admission and memory
guarantees are unresolved open items (see "Open design items (from the
fourth review)"), so the plan makes no guarantee there yet.

Completed plans, design checkpoints, review reports and run records under
`plans/` were removed on 2026-11-05. A `plans/` path cited in backticks that
no longer exists is in Git history up to commit f5fde76 (`git show
f5fde76:<path>`).

Revision 4, 2026-11-04. Written for slice 7 of the
TODO repair plan (`plans/todo-repairs.md`) under its decision D6: TEXT-01 gets this
design plan now and nothing is implemented. It continues the deferred
large-buffer branch of
[app-author plan slice 6](app-author-improvements.md#6-large-documents-deferred).
Codex reviewed revision 1 (`plans/todo-repairs-review/design-s7.md`)
and rechecked revisions 2 (`plans/todo-repairs-review/design-s7-r2.md`),
3 (`plans/todo-repairs-review/design-s7-r3.md`) and 4
(`plans/todo-repairs-review/design-s7-r4.md`), judging each "needs
rework". This revision addresses every point of the third recheck. The
maintainer then stopped the iteration: the fourth recheck's findings are
open design items settled by prototypes in slices 1 and 2. The "Review
record" gives the dispositions. The design checkpoint (slice 5) is still
outstanding.

Every type, method, constant and variant below that the code does not have
is a **proposal**, and is marked so. Line references are to commit 9aaacca
(0.3.95); Parley references are to the pinned parley 0.12.0 and
parley_engine 0.12.0 sources.

## Background

[TODO.md](../TODO.md) TEXT-01 says a `TextArea` cannot hold a real document:
its text is limited to `TextEditError::MAX_TEXT_BYTES` and prepared as one
`TextLayout`, which `TextLimit::MAX_RASTER_SIDE` also bounds. It asks for:
- a decision between extending `TextArea` and a separate document editor;
- per-paragraph preparation with visible-only rasterization;
- selection, caret, IME and movement across paragraphs;
- measurements on a defined corpus before a capacity is chosen;
- an editor capacity separate from the other text limits.

### How the current path works

All bound editors (`TextField`, `TextArea`, the numeric inputs, `SearchField`,
`InlineText`, `DraftEditor`) share one crate-internal `Editor<M>`
(`crates/pert-ui-runtime/src/text_fields.rs:1330`). `TextArea` is
`pub struct TextArea(Editor)` (`text_fields.rs:327`). For the large-text
question:

- **One string, one layout.** The editor holds the whole text as one
  `String`. Every edit:
  1. clones it into a candidate (`text_fields.rs:3187`);
  2. refuses it above `MAX_TEXT_BYTES` (`:3189`);
  3. prepares the whole candidate as one `TextLayout` (`:3195`);
  4. passes it to `publish_text` (`:1732`), which writes the whole string to
     the `Signal<String>` binding unless the editor is in draft mode (a
     normalizer or length limit) or deferred (`:1733-1738`; those write on
     save instead);
  5. pushes a snapshot of the whole previous text onto a 32-deep undo queue
     (`:3198-3225`).

  Arrangement (`:2643`) and paint (`:3012`) prepare the displayed text again;
  for an unchanged text those are cache hits.
- **Preparation is shaping plus full rasterization.** `TextService` shapes
  the whole text as one Parley layout (`text.rs:1246`), aligned with
  `Alignment::Start` (`:1307`). It checks the whole canvas against
  `MAX_RASTER_SIDE` (8192 physical pixels, `text.rs:116`, checked at `:1144`
  and during glyph placement at `:1500`) and the coverage budgets
  (`:1147`, `:1437-1441`, `:1503`). It then rasterizes every glyph and
  composites the coverage (`text.rs:1127-1180`). The resulting `TextLayout`
  (`text.rs:212`) owns both the shaped Parley layout and the coverage
  masks.
- **One base direction per text.** Parley stores one layout-wide base level
  (parley `builder.rs:368`) and aligns every line by it
  (`layout/alignment.rs:76-121`). Its analyzer resolves the whole supplied
  text in one call (parley_engine `analysis.rs:811-822`). pert-ui does not
  set a base direction, so Parley's default `BaseDirection::Auto` applies.
- **One shared cache.** Prepared texts go into one runtime-wide cache of at
  most 128 entries and a 16 MiB *admission estimate* (`text.rs:28`,
  `:1100-1123`), shared with labels and every other text.
- **Paint draws the whole layout** clipped to the text area. It paints the
  layout once, then once more clipped to each selection rectangle in the
  selected-text color (`text_fields.rs:3040-3063`).
- **Selection is tied to one layout.** The public `TextSelection`
  (`text_selection.rs:61`) wraps `parley::editing::Selection`; `new`,
  `moved`, `at_point`, `word_at_point`, `caret` and `rectangles`
  (`:93-215`) all take one `&TextLayout`. `TextMovement` has no page
  movement. Without Shift, Left or Right on a non-collapsed selection
  collapses to the end that comes visually first or last, compared by line
  top and then x (parley `editing/selection.rs:171-200`). Vertical movement
  keeps a preferred x, which the other movements reset.
- **IME.** `compose` (`text_fields.rs:3241`) splices the preedit into a copy
  of the whole text and prepares it (`:3266`). A composition that would pass
  `MAX_TEXT_BYTES` is not shown (`:3256`). `TextPreedit::new` refuses
  preedit over the same limit (`text_input.rs:104`). `TextInputState`
  (`text_input.rs:395`) gives a caret rectangle in component coordinates and
  a session number. The session must increase after every cancellation and
  disables IME on exhaustion (`text_input.rs:396-399`). `cancel_composition`
  does this with `checked_add` (`text_fields.rs:2254`).
- **Binding check and external writes.** `check_text`
  (`text_fields.rs:3814`) refuses a bound value over `MAX_TEXT_BYTES` or with
  unsupported control characters; `TextArea::check` and the binding refusal
  use it. An accepted external write replaces the text, resets the
  selection, cancels composition and clears undo and redo (`sync_text`,
  `text_fields.rs:2497-2505`).

### What shares which limit

`TextEditError::MAX_TEXT_BYTES` (16 KiB, `text_input.rs:57`) is one constant
for unrelated paths:

| Path | Where |
|---|---|
| Bound editor value (`check_text`, `TextField::check`, `TextArea::check`) | `text_fields.rs:3814` |
| Typed, pasted and composed edits | `text_fields.rs:3189`, `:3256` |
| IME preedit | `text_input.rs:104` |
| One prepared plain or rich text | `text.rs:831` |
| Ellipsis fitting cuts a longer first line first | `text/fit.rs:149-152` |
| `Label` help tooltip eligibility | `controls.rs:292` |
| Toast text | `notifications.rs:332`, `:613` |
| Parley style-run bound for tab hiding: `2 * MAX_TEXT_BYTES + RichText::MAX_SEGMENTS + 4 <= u16::MAX`, checked at compile time | `text/tabs.rs:41-43` |

Because of the last row, the shared constant cannot be raised past 32,253
bytes without changing how tab hiding uses Parley style runs.

`UiStrings::MAX_TEXT_BYTES` (1024, `strings.rs:603`) is a different constant:
the longest replacement text one string-table role accepts. It shares nothing
with editors, labels or toasts, and this plan does not affect it.

## Baseline

### Method

A throwaway program measured the current path headlessly through the public
`pert-ui-runtime` API: `Runtime`, with no window and no GPU. Its source is
in [document-editor-bench](document-editor-bench/). Its raw output is in
[document-editor-bench/results](document-editor-bench/results/), and the
appendix gives the procedure.

- **Code measured:** commit 9aaacca (0.3.95), exported with `git archive`.
  The working tree held another session's uncommitted changes, so it was
  not measured.
- **Machine:**
  - CPU: Intel Core i9-12900, with 8 performance cores of two threads each
    (CPUs 0-15) and 8 efficiency cores (CPUs 16-23);
  - memory: 62 GiB;
  - system: Fedora 44, kernel 7.2.15-200.fc44.x86_64, CPU frequency
    governor `performance`.
- **Build:** rustc 1.99.1, release profile.
- **Instrumentation:** a counting global allocator is part of the measured
  configuration. It performs atomic counter updates on every allocation and
  deallocation, and it was active during all timings. It counts live and
  peak heap bytes allocated through Rust's global allocator, not resident
  set size.
- **Corpus:** deterministic generated ASCII English prose. Sentences have 6
  to 18 words, and paragraphs have 3 to 7 sentences separated by a blank
  line (two LFs). There are no tabs, no non-ASCII or right-to-left text and
  no long unbroken lines. Each corpus is 138 bytes short of 4, 8 or 16 KiB
  (3958, 8054 and 16,246 bytes), so the 65 one-byte edits of a series fit
  under the limit. A series ends 65 bytes longer.
- **Setup:** a `TextArea` bound to the corpus fills an 800 x 600 logical
  pixel viewport at scale 1, as the center of a `RegionLayout`, with the
  default typography (bundled Noto Sans Mono, 14 logical pixels). It is
  focused, and ArrowDown presses move the caret to the middle of the text:
  bytes 2036, 4050 and 8165.
- **Operations:**
  - *insert:* `Runtime::text_input("x")`;
  - *Enter:* an Enter press and release through `Runtime::key_input`.

  Each is followed by `Runtime::flush` and `Runtime::paint`. A series has 5
  warm-up edits and 60 timed ones. Every edit produces a new text.
- **Latency:** wall time from the start of the input call to the end of
  paint, with the input call, flush and paint also timed separately. The
  tables give the median and 95th percentile of the 60 timed edits.
- **Rounds:**
  - *Round 2* (18:36-18:37, load average 2.8-2.9; another session was
    compiling earlier) is the source of the tables below. Its complete raw
    output is preserved.
  - *Round 1* (about 18:17-18:18, load average 5.4-6.1) gave consistent
    figures. Only its pinned latency lines were kept, transcribed in
    `results/r1-pinned-transcribed.txt`.

### Edit latency (round 2)

Pinned to one performance-core thread (`taskset -c 2`), three runs. Each
cell gives the range over the three runs, in milliseconds:

| Corpus | Insert, median | Insert, p95 | Enter, median | Enter, p95 |
|---|---|---|---|---|
| 3958 B | 5.73-5.85 | 5.95-6.11 | 6.11-6.22 | 6.25-6.47 |
| 8054 B | 11.60-11.69 | 11.96-12.30 | 12.40-12.79 | 13.07-13.17 |
| 16,246 B | 23.18-23.38 | 23.69-25.75 | 25.11-25.34 | 25.73-26.01 |

One run pinned to an efficiency core (`taskset -c 20`) gave these medians:

| Corpus | Insert | Enter |
|---|---|---|
| 3958 B | 9.31 ms | 9.79 ms |
| 8054 B | 18.57 ms | 20.39 ms |
| 16,246 B | 37.11 ms | 40.42 ms |

Round 1 gave these medians:
- on the performance core, 5.84-5.90 / 6.26-6.49 ms at 4 KiB,
  11.88-13.60 / 12.59-14.09 ms at 8 KiB and 24.17-25.41 / 25.88-27.28 ms
  at 16 KiB (insert / Enter);
- on the efficiency core, up to 46.6 / 64.0 ms at 16 KiB, under heavier
  load.

Unpinned runs spread between and around the two core types. That the
scheduler moving the process between core types causes this spread is a
hypothesis, not measured.

What was measured:
- The input call (`text_input` or `key_input`) took over 99% of the
  measured time.
- Flush and paint medians were at most 0.04 ms each; both reuse the
  prepared text the input call created.
- At the three sizes, the input call's duration was roughly proportional
  to the text size: about 1.4-1.5 ms per KiB on the performance core.
  Three points show the trend at these sizes, not a general scaling law.
- An Enter in the middle of 8 KiB took most of a 60 Hz frame (16.7 ms); an
  edit at 16 KiB took more than one frame.

**Hypothesis, not measured:** whole-text preparation (shaping and
rasterization) dominates the input call. The call also includes the
candidate clone, validation, binding publication with `PartialEq`
comparison, and the undo snapshot (`text_fields.rs:3187-3225`). The harness
does not separate these costs.

### Memory (round 2)

These figures were identical in every run of both rounds:

| Corpus | Heap after mount and first frame | Peak during a series, above the settled heap before it (insert / Enter) | Live heap after a series, as an increment over the settled heap before it (insert / Enter) |
|---|---|---|---|
| 3958 B | 1435.6 KiB | 13,764.5 / 17,836.8 KiB | 11,019.9 / 11,691.5 KiB |
| 8054 B | 2810.8 KiB | 15,563.3 / 18,792.5 KiB | 10,090.4 / 11,483.2 KiB |
| 16,246 B | 5664.8 KiB | 17,153.1 / 21,000.8 KiB | 6047.9 / 8002.1 KiB |

"Settled" is the live heap after mounting, the first frame and moving the
caret, just before the timed series; the bench prints the last column as
"retained after series". For comparison, an editor that refused its binding
and shows empty text took 106.1 KiB after mount.

**Hypotheses, not measured as a breakdown:**
- The mounted heap grows by about 350 bytes per byte of text, which
  suggests that the coverage of the single prepared layout dominates it.
- The increment after a series is mostly the shared text cache holding
  layouts of superseded intermediate texts, plus the 32 whole-text undo
  snapshots. The cache's 16 MiB figure bounds only its admission estimate
  (`text.rs:1100-1123`), not the actual heap, undo, scratch allocations or
  other retained resources.
- The transient peak is cache admission and eviction churn plus
  preparation scratch memory.

### The raster limit is reached below the byte limit

The same outcomes in every run:

| Case | Outcome |
|---|---|
| 16,384 B in a 320 x 600 editor, scale 1 | The first `flush` fails: `text preparation exceeds its limit of 8192 physical pixels per side` |
| 8192 B in the same editor | Mounts and paints |
| 16,384 B in an 800 x 600 editor, scale 2 | The first `flush` fails with the same error |
| 8192 B in the same editor | Mounts and paints |
| Enter repeatedly in the middle of 8192 B, 320 x 600, scale 1 | Presses 1-135 succeed. Press 136 makes `key_input` return the same error; the binding stays at 8327 B and the next frame succeeds |

At 14 logical pixels and line spacing 1.4, 8192 physical pixels hold about
418 lines at scale 1 and 209 at scale 2, consistent with TEXT-01's "about
400 lines". The line count at the failure was not measured. These
thresholds are reported experimental evidence for this corpus, width and
font, not statically verified facts.

### Rejection above 16 KiB

| Case | Outcome |
|---|---|
| A 16,385-byte binding at mount | `flush` succeeds. Inspection reports `binding_error` = "text exceeds the 16384 byte limit", and the shown text is empty because no valid text had been adopted. Typing returns `Ok` and leaves the binding unchanged |
| A 16,384-byte binding, typing one byte | `text_input` returns `Ok`; the binding stays at 16,384 B and no `binding_error` is reported. This is the documented silent refusal at the limit (`TextField` rustdoc, "Bound strings must be at most …") |

### Not measured

The current path rejects the larger corpora at the binding, so they have no
current-path figures:
- 1 MiB of prose;
- 50,000 log lines;
- a long unbroken line;
- right-to-left or mixed scripts;
- IME composition;
- a 100 KiB paste;
- replacing a large external value.

The paint figures cover recording the display list only, not rasterizing
it on a GPU or CPU backend.

## Decision: a separate document editor

**Recommendation:** a separate component, proposed as `DocumentEditor`, over
a library-defined, application-owned paragraph buffer, proposed as
`TextDocument`. `TextArea` stays the multiline form field. Both editors
build on one new paragraph-layout engine:
- it starts crate-internal;
- it becomes public extension points before any public editor uses it
  (see "Extension points");
- `TextArea` adopts it in slice 6.

Reasons:

1. **`TextArea` is a form field over a whole `String`.** In its default
   mode it writes a `Signal<String>` on every edit. Its form features are
   built on that string:
   - draft mode through normalization or length limits;
   - `required`, `min_length`, `max_length` and the counter;
   - the modified indicator;
   - editor sessions through the shared `Editor`;
   - `check`.

   For a document, writing the whole `String` costs O(n) per edit:
   `Signal::set` compares old and new values with `T`'s `PartialEq`
   (`signal.rs:554-569`), and every reader rereads the string.
   Paragraph-partitioned layout does not remove that cost.
2. **The shared core serves small editors.** `Editor<M>` also drives the
   single-line, numeric, search and in-place editors. A paragraph store
   inside it would burden all of them, or split the core anyway behind one
   public name.
3. **One type with two sources would carry dead builders.** A
   `TextArea::for_document` constructor path would leave the form builders
   meaningless on that path.
4. **Undo and inspection need different shapes.** Document undo must store
   deltas: 32 snapshots of 1 MiB would be 32 MiB. Inspection must not copy
   the whole text by default, as `TextArea::inspect` does with its `text`
   property and semantic value.

**What it makes advanced or retires** (the AGENTS.md rule for a new way to do
something already possible): `DocumentEditor` takes over long multiline
text. `TextArea`'s rustdoc will say that its capacity is for form-sized text
and point to `DocumentEditor`. Nothing is retired.

### Alternatives considered

| Alternative | Why not recommended |
|---|---|
| **A. Extend `TextArea` in place**, keeping `Signal<String>` and switching to paragraph layout internally | Addresses preparation cost and, with tiling, the raster failure. It keeps O(n) binding writes and comparisons per keystroke, whole-text undo snapshots and whole-text inspection. Still worth doing for `TextArea`'s own capacity, in slice 6. |
| **B. `TextArea` with a second constructor taking a `TextDocument`** | One public name, but every form builder is meaningless on one path, and `Editor<M>` must still be split. |
| **C. Generalize `TextSelection` and `TextLayout` so one editor works over either** | A large breaking change to a public type used by labels, `SelectableText` and custom editors, for a capability only the document editor needs. |
| **D. `VirtualList` (as in `log_explorer`) for large read-only text, and no editor** | Right for read-only logs, and the editor's rustdoc will say so. It does not edit, select across rows as text, or compose with an IME. |

## Design

### Intended text contract

Revision 1 claimed that preparing paragraphs separately gives "the same
result" as today's single layout. That claim is withdrawn. UAX #9 treats
paragraphs independently, but today's implementation does not: it resolves
one base level and aligns every line by it (see "How the current path
works"). The document editor therefore states its own contract and does not
claim parity:

- **Paragraphs.** The text between LFs. N LFs make N+1 paragraphs (see
  "Boundary mapping").
- **Own direction.** Each paragraph's own direction comes from UAX #9 rules
  P2-P3 (its first strong character), or is absent when it has no strong
  character (empty, digits or punctuation only).
- **Resolved direction.** A paragraph's own direction, if it has one.
  Otherwise the own direction of the nearest preceding paragraph that has
  one, or left-to-right when there is none; UAX #9 HL1 permits such a
  higher-level rule. Each paragraph is built with its resolved direction
  through Parley's `set_base_direction` (parley `builder.rs:82`).
- **Alignment.** Each paragraph aligns to the start edge of its resolved
  direction.
- **Line breaking, graphemes and words.** Analyzed per paragraph. LF is a
  mandatory break (UAX #14), a grapheme cluster boundary (UAX #29 GB4/GB5)
  and a word boundary (WB3a/WB3b), so no cluster or word spans a paragraph
  boundary.

The resolved direction depends on preceding paragraphs. It is therefore part
of a paragraph's layout identity, and its invalidation is specified under
"Direction context".

**Differences from today's `TextArea`.** In mixed-direction text, a
paragraph whose direction differs from the whole text's single resolved
level can be laid out, aligned and navigated differently. This plan treats
those differences as **corrections** following UAX #9, not as compatibility
requirements. The maintainer decides this at the checkpoint, before slice 6
moves `TextArea` to the engine (open question 3). Slice 3 first records
today's behavior on these cases in characterization tests.

Required cases, as tests in slice 3 and part of the contract:
- **Across paragraphs:**
  - an LTR paragraph followed by an RTL one, and the reverse;
  - a neutral paragraph between paragraphs of either direction;
  - a run of neutral paragraphs whose preceding strong paragraph changes
    direction through an edit;
  - an empty paragraph at the start, in the middle and at the end;
  - Left, Right, Up and Down across each of these boundaries.
- **Within one line:**
  - an LTR paragraph containing an RTL run, and the reverse;
  - a paragraph that ends in an opposite-direction run;
  - for each, the caret at the run boundaries with both affinities;
  - Left and Right across the boundaries;
  - hit tests on either side of a boundary.

### Boundary mapping (normative)

Paragraph `i` has the content byte range `s_i..e_i`, half-open. Every
paragraph except the last is followed by its LF terminator, the byte
`e_i..e_i+1`, and `s_(i+1) = e_i + 1`. The terminator belongs to no
paragraph's prepared text. The caret stops are the grapheme boundaries
inside each paragraph, including `s_i` and `e_i`. Each document offset maps
to exactly one (paragraph, local byte) pair: `e_i` is the end of paragraph
`i`, and `e_i + 1` is the start of paragraph `i+1`.

| Text | Bytes | Paragraphs | Caret stop → (paragraph, local) |
|---|---|---|---|
| `""` | 0 | P0 = `0..0` | 0 → (0, 0) |
| `"\n"` | 1 | P0 = `0..0`, P1 = `1..1` | 0 → (0, 0); 1 → (1, 0) |
| `"a\n"` | 2 | P0 = `0..1`, P1 = `2..2` | 0 → (0, 0); 1 → (0, 1); 2 → (1, 0) |
| `"a\n\nb"` | 4 | P0 = `0..1`, P1 = `2..2`, P2 = `3..4` | 0 → (0, 0); 1 → (0, 1); 2 → (1, 0); 3 → (2, 0); 4 → (2, 1) |

Rules:

- **Empty paragraphs** are real lines. They have a positive line height from
  the paragraph font's metrics, draw no ink, and take the caret at local 0,
  at the start edge of their resolved direction.
- **Affinity inside a paragraph.** A caret offset has two logically adjacent
  clusters, upstream and downstream. Affinity selects which one the caret
  attaches to wherever they are visually separated. That happens at a soft
  wrap and also at a bidi level change within one line. Parley uses
  affinity this way to choose the bounding clusters (parley
  `editing/cursor.rs:80-85`, `:331` and following). The editor keeps the
  affinity at every such position and uses it for drawing, movement and
  hit testing, as Parley does.
- **Affinity at paragraph endpoints.** At `e_i` there is no downstream
  cluster in paragraph `i`, and at `s_i` no upstream one. Constructing a
  caret there converts the affinity explicitly:
  - at `e_i` it becomes Upstream, attaching to the logically last cluster's
    trailing edge;
  - at `s_i` it becomes Downstream, attaching to the logically first
    cluster's leading edge;
  - in an empty paragraph the caret is at its start edge.

  The logical end of a paragraph need not be at its visual edge. An LTR
  paragraph that ends in an RTL run has its logical end inside the line, at
  the trailing edge of that run's logically last cluster.
- **Backward selections** are valid. A selection covers
  `min(anchor, focus)..max(anchor, focus)`. In the four-byte `"a\n\nb"`,
  anchor 4 and focus 0 select all four bytes.
- **Boundary deletion:**
  - Backspace at `s_(i+1)` deletes the terminator byte `e_i..e_i+1`, which
    merges paragraphs `i` and `i+1`, with the caret at `e_i`.
  - Delete at `e_i`, when it is not the last paragraph, deletes the same
    byte; the caret stays at `e_i`.
  - In `"a\n\nb"`, Backspace at 3 gives `"a\nb"` with the caret at 2, and
    Delete at 1 gives `"a\nb"` with the caret at 1.
  - Backspace at 0 and Delete at the document end change nothing.
  - A merge can join graphemes across the former boundary only if the text
    joins: LF never joins. Deletions inside a paragraph can join graphemes,
    for example by removing a ZWJ's neighbor; see "Offsets after a change".
- **Copy** of a selection is exactly `text[min..max]`, terminators
  included.
- **Selected terminators.** When a selection includes paragraph `i`'s
  terminator, the last line of paragraph `i` gets an end-of-line fill as wide
  as a space in the font at that position. A selected empty paragraph shows
  only that fill. This is the intended contract, not a reproduction of
  today's geometry.

### Buffer, revisions and edits: `TextDocument` (proposal)

- **Contents.** A value type holding the paragraphs. LF and tab are the only
  control characters accepted. Paste converts CR, CRLF, NEL and the Unicode
  separators to LF, as `TextArea` does (`text_fields.rs:2119-2147`). An
  explicit value is validated and refused, not cleaned.
- **Storage.** Paragraph contents are shared (`Arc<str>`) in a persistent
  balanced tree. Each subtree summary holds:
  - the byte and paragraph counts;
  - the index of its first and its last paragraph with an own direction,
    and those directions.

  Clone is O(1). An edit replaces only the touched paragraphs and the tree
  path to them. An offset maps to (paragraph, local) in O(log n), and a
  paragraph's resolved direction is found in O(log n).
- **Equality.** `PartialEq` is content equality, with a pointer-equality
  fast path for shared subtrees. It ignores state identity.
- **State identity (proposal).** `TextDocument::revision()` returns a
  `DocumentRevision`: an opaque token naming one document *state*, not a
  position in a counter sequence.
  - It is an `Arc`-allocated token, compared and hashed by pointer.
  - `new`, deserialization and every successful non-empty `replace` create
    a fresh token. Two replacements applied to the same document therefore
    produce two different revisions. Sibling branches never collide.
  - Clones share the token. `TextChange` values and undo entries hold their
    tokens, so a token's address cannot be reused while anything names it.
  - There is no counter to exhaust and no global mutable state. The cost is
    one small allocation per state.
  - `Debug` prints an opaque form, not the address.
  - `Eq` and `Hash` on `DocumentRevision` are both by identity. If
    `TextDocument` ever implements `Hash`, it hashes content, consistent
    with its content `PartialEq`.
  - Revisions and changes are **not portable**. They live only in this
    process, are never serialized, and are not durable identifiers. An
    application that persists history needs its own versioned envelope and
    its own durable identifiers.
- **No-op replacement.** A `replace` whose inserted text equals the removed
  text leaves the content unchanged. Such a replacement:
  - returns the same document, with the same revision;
  - returns no change: the result type is
    `Result<(TextDocument, Option<TextChange>), TextDocumentError>`.

  Any other successful replacement changes the content, because the result
  is the prefix, the inserted text and the suffix, and the base is the
  prefix, the removed text and the suffix. Its result is therefore never
  equal to its base.
- **`replace(range, text)` (proposal)** fails, with no clamping and no
  panic, with:
  - `InvalidRange { start, end }` unless `start <= end <= len()` and both
    are character boundaries;
  - `InvalidText` for an unsupported character;
  - `TooLarge` above the document capacity;
  - `ParagraphTooLong` when a resulting paragraph exceeds the per-paragraph
    limit.
- **`slice(range)` (proposal)** returns `Result<String, TextDocumentError>`
  and validates the range the same way.
- **`TextChange` (proposal)** has five fields:
  - `base: DocumentRevision`, the state the change applies to;
  - `result: DocumentRevision`, the state it produces;
  - `removed: Range<usize>`, in the base state's byte offsets;
  - `inserted: Range<usize>`, in the result state's byte offsets;
  - `kind: ChangeKind { Edit, Undo, Redo }`.

  The inserted text is `result_document.slice(change.inserted)`. The
  removed text is the base document's `slice(change.removed)` while the
  base is available.
- **How identity fits the binding.**
  - **Editor writes.** The editor applies an edit to the document it last
    observed (revision `r`).
    - At publication it first checks that the binding still holds `r`, by
      revision. If an application write arrived in between, that write
      wins: the edit is refused without effect and the write is adopted
      at the next update, as the draft policy has external writes win.
    - Otherwise the editor calls `Signal::set` with the result. The result's
      content differs from `r`'s, so `Signal::set`'s equal-value suppression
      (`signal.rs:554-569`) never drops an editor write.
    - The editor then observes the result's revision.
  - **Equal-content application writes.** An application write with content
    equal to the binding's is suppressed by `Signal::set`, whatever its
    revision. The binding and the editor keep the earlier state and its
    revision. A later write derived from the suppressed value carries a
    base the editor never observed, and is therefore an unrelated external
    write.
  - **External-write detection** compares revisions, not content: a binding
    value whose revision differs from the editor's observed revision is an
    external write.
  - **Callbacks.** `on_change` reports exactly the states the binding
    adopted: `base` is the observed revision before the write, and `result`
    is the revision after it.
- **Publication order for a user edit:**
  1. validate the edit;
  2. if it is a no-op, update only the selection, with no write, history
     entry or callback;
  3. prepare the touched paragraphs and mark the direction context. A
     preparation failure refuses the edit and leaves everything unchanged;
  4. check the binding revision, then `Signal::set`. A write error
     propagates, and nothing else is recorded;
  5. push the history entry;
  6. call `on_change(&TextChange, &TextDocument)`
     [in the input](../docs/contracts.md#in-the-input).

  A callback error propagates from the input call. The write and the
  history entry stay, and nothing is replayed (AGENTS.md: failures do not
  undo accepted state). Application writes do not call `on_change`. The
  first version has no draft or deferred mode.
- **External replacement.** The first version handles it like `TextArea`'s
  `sync_text` (`text_fields.rs:2497-2505`):
  - it cancels composition and any drag;
  - it clears undo and redo;
  - it resets the selection to the document start.

  The scroll anchor is kept if its offset is still a caret stop in the new
  document, and otherwise resolves as in "Offsets after a change", against
  the new document's length. Change-aware adoption, which would transform
  the selection instead, is open question 6.

### Local undo and redo

- **Entries.** An entry stores the document before the edit (a structurally
  shared clone, so it keeps the earlier revision alive), the selection
  before, the result revision, the selection after, and the two ranges.
- **Undo** restores the stored earlier document, with its original revision,
  and the selection before. **Redo** restores the result document, with its
  revision, and the selection after. Undo and redo therefore move between
  exact earlier states. Consumers keyed by revision see them recur (A, B,
  A), which is correct because they are the same states.
- **Applicability.** An entry applies only while the binding holds the
  entry's result revision (for undo) or its base revision (for redo). An
  external write clears both stacks, so this holds except for an
  application write that arrived since the last update. In that case the
  undo or redo is refused without effect, and the write is adopted at the
  next update.
- **Order:**
  1. check applicability;
  2. prepare the touched paragraphs;
  3. `Signal::set` the restored state;
  4. move the entry to the other stack;
  5. set the selection;
  6. call `on_change` with `kind` Undo or Redo, `base` the replaced state,
     `result` the restored state, and ranges derived from the entry.

  Failures follow the same rules as an edit.
- **Accounting: retained memory, charged conservatively.** One budget covers
  undo and redo together (proposal: 100 entries or 8 MiB, whichever comes
  first). The 8 MiB bounds the memory history keeps alive, not a logical
  payload. Each entry is charged, when it is pushed:
  - the **full length** of every paragraph allocation that the entry's
    stored document holds and the document after the edit does not, which
    are the superseded versions of the touched paragraphs. A one-byte edit
    in a 16 KiB paragraph is charged 16 KiB, because the entry keeps that
    whole `Arc<str>` alive;
  - for redo, the same for the paragraph allocations of the restored side
    that the current document does not hold;
  - the **tree nodes** the edit copied, counted from the actual path at that
    moment (tree height times the node size, plus split or merge siblings),
    not a constant.

  Allocations that later states share again are not refunded, so the charge
  is meant as an upper bound on what history alone keeps alive. That bound
  is *provisional*: removed subtrees, empty paragraphs and allocation
  overhead are not yet charged (open item OD4). A new edit clears
  redo. When the budget is exceeded, the oldest undo entries are dropped
  first. Slice 2 checks the charge against heap counts.
- **An edit larger than the whole budget** is applied, and both stacks are
  then cleared. The edit cannot be undone, and the rustdoc says so. Refusing
  a valid edit for history reasons would be worse.
- **Granularity** is the same as `TextArea`'s: one entry per edit,
  composition commit or paste.
- **Host-owned undo** is not in the first version. A builder proposed as
  `local_undo(false)` would keep no history and leave the undo and redo
  chords (Ctrl/Cmd+Z, Shift+Ctrl/Cmd+Z, Ctrl+Y outside macOS) unclaimed, so
  they reach the application's commands. Without change-aware adoption,
  every host undo write would be an external replacement that resets the
  selection. Host-owned undo therefore waits for open question 6 and gets
  its own slice.

### Long paragraphs: refused in the first version

The first version **refuses** a paragraph longer than the per-paragraph
limit:
- `TextDocument::new` and `replace` return `ParagraphTooLong`;
- the editor refuses such a binding through binding refusal, and refuses
  such an edit or paste silently, like the byte limit.

The per-paragraph limit is the prepared-text limit, today 16 KiB, which
keeps every paragraph within the Parley style-run assertion
(`text/tabs.rs:41-43`). The unbroken 64 KiB line is a refusal test, not a
performance corpus.

Chunked preparation of longer paragraphs is deferred. A later design must
specify:
- fallback split rules where no word boundary exists, and graphemes larger
  than a chunk;
- paragraph-wide bidi resolution;
- continuous line packing across chunks;
- coordinate mapping;
- worst-case work per edit.

It must be tested with Arabic joining across joins, mixed directions,
enormous graphemes and edits that move chunk boundaries.

### Direction context

- **Layout identity.** A shaped paragraph is keyed by its content (hash and
  length, compared on a hit), its **resolved direction** and the layout
  generation `G` (see "Caches"). The same neutral text after an LTR and
  after an RTL paragraph has two different keys.
- **Lookup.** A paragraph's resolved direction is its own, or the last own
  direction among the preceding paragraphs. The tree summaries find that
  in O(log n).
- **Invalidation.**
  - An edit can change a paragraph's own direction, or insert or remove a
    paragraph that has one. The resolved direction of the following run of
    paragraphs without an own direction, up to the next paragraph with one,
    may then change.
  - The run's end is found in O(log n). The run is marked direction-stale
    lazily, by a range tag on the height index in O(log n). Nothing in it
    is reshaped eagerly.
  - Stale entries keep their heights as estimates. A stale paragraph is
    reshaped under its new resolved direction when the visible fill
    reaches it, or when caret movement or hit testing needs it.
- **Edit cost.** An edit costs:
  - shaping the touched paragraphs, each at most the per-paragraph limit;
  - O(log n) for the document tree, the index and the direction lookup and
    tagging;
  - reshaping the stale paragraphs the visible fill covers, bounded by the
    visible-fill bound below.

  It never reshapes a whole run. An edit inside a maximal paragraph can
  still take time comparable to today's whole-text preparation of 16 KiB;
  slice 2 measures this.
- **IME projection** follows the same rules. Projected paragraphs get
  resolved directions from the projected context, and the following
  neutral run is marked stale within the projection overlay only (see
  "IME").

### Preparation and tiled rasterization

**Shaping is separate from coverage.** A shaped paragraph holds the Parley
layout, its size, line metrics and horizontal overflow, the per-glyph raster
outlines and bounds below, and the paragraph's admission decisions. It holds
no coverage and has no raster-side check. `TextLayout` and every existing
path keep today's checks.

#### Raster descriptor

Bounds are computed from exactly the outline that is later rendered. Each
glyph has one **raster descriptor**:
- the font face, after fallback, because Parley assigns fonts per run;
- the glyph id;
- the size in physical pixels at the effective scale;
- the normalized variation coordinates;
- hinting, which is on as today (`.hint(true)`, `text.rs:1361`).

The glyph's outline is produced once per descriptor with swash's
`Scaler::scale_outline` (swash `scale/mod.rs:508`). That is the same
hinted, variation-instantiated outline step that swash's `Render` uses
before rasterizing (`scale/mod.rs:850-860`). The outline is cached under
the descriptor.

**Synthesis is excluded.** Swash's `Render` can embolden or transform an
outline after scaling (`scale/mod.rs:858-863`). pert-ui configures neither,
and does not apply Parley's synthesis today. The tile path does not support
synthetic emboldening or skew. A future synthesis must join the descriptor
and the cache identity before it is used.

**Bounds and rendering agree by construction.**
- The glyph's raster bounds are the cached outline's `Outline::bounds`
  (`scale/outline.rs:86`) at the glyph's position, widened by one physical
  pixel on every side.
- The widening covers only the subpixel render offset, which is below one
  pixel, and antialiasing. The bounds therefore do not depend on phase.
- Rendering uses the same cached outline object (`Outline::path`,
  `scale/outline.rs:81`); it is never scaled again.

Glyphs without an outline draw nothing, as on today's path, which renders
`Source::Outline` only (`text.rs:1396`). Color and bitmap glyphs are out of
scope.

#### Overhang limit

A glyph's ink may extend beyond its line box. The plan bounds how far:
- the **overhang limit** `M_h` is twice the paragraph style's line height in
  physical pixels (proposal);
- a glyph whose widened bounds extend more than `M_h` above or below its
  line box, or are wider or taller than `MAX_RASTER_SIDE`, is
  **excessive**. It is never rasterized and is drawn as a marker (see
  "Markers").

Every rasterized ink pixel therefore lies within `M_h` of its paragraph's
line boxes vertically. Paragraphs outside the visible region plus a halo of
`M_h` contribute no visible ink. The visible fill uses that halo (see
"Visible fill").

Pathologically tall stacks of combining marks therefore show markers for
the marks beyond the limit. This is documented, and it is a deliberate
trade for a bounded discovery region.

#### Admission: per paragraph, whole glyphs, independent of the viewport

Admission is decided per paragraph, before any rasterization. It depends on
nothing but the paragraph's shaping inputs, and is cached with the shaped
paragraph.

1. **Cells.** The paragraph is divided into a grid of `T` x `T` cells of
   paragraph-local physical pixels (proposal: T = 1024), anchored at the
   paragraph's own integer origin. The grid is not anchored on screen, and
   it is never cropped for admission. Scrolling, phase and the viewport
   therefore do not change cells.
2. **Charge per glyph and cell.** For each glyph, and each cell its widened
   bounds touch, the charge is computed from the outline without
   rasterizing:
   - the outline is flattened by pert-ui's own flattener, with a fixed
     tolerance of a quarter pixel and a fixed maximum subdivision depth,
     into line segments clipped to the cell. The depth-exhaustion outcome
     and winding-preserving clipping are open (items OD5 and OD1);
   - the charge is the sum over those segments of the pixel rows and
     columns each crosses, plus one per segment, plus the window area: the
     glyph's widened bounds clipped to the cell.

   This counting pass allocates nothing. Its work is proportional to the
   number of flattened segments, which the depth limit bounds per outline
   curve. *Provisional:* whether this charge bounds the rasterizer's actual
   work is open item OD1.
3. **Excessive glyphs.** Besides the bounds rules above, a glyph whose
   charge in any one cell exceeds the cell budget `W` (proposal: 16 Mi
   units, the same order as today's per-text temporary budget, `RASTER_BYTES`,
   `text.rs:26`) is excessive.
4. **Aggregate admission.** In each cell, glyphs are admitted in logical
   order while the cell's running charge stays within `W`. A glyph is
   **replaced as a whole**, in every cell it touches, if it fails in any
   one of them. Admission is never partial within a glyph.

Because cells, bounds and charges are fixed by the content, the resolved
direction and `G`, admission is the same for every viewport, crop, scroll
position and phase. It changes only when the paragraph is shaped again.

#### Rasterizing admitted glyphs

- **Tiles.** A tile is a cell's coverage, allocated cropped to the
  intersection of the cell, the visible region (with halo) and the
  paragraph's admitted ink bounds, rounded out to whole pixels. A short
  paragraph allocates only its ink. Per-paragraph anchoring means that an
  edit which moves later paragraphs moves their tiles without rasterizing
  them again. A document-wide shared grid was rejected, because one Enter
  would invalidate every visible tile below the caret.
- **Bands.** Each admitted glyph is rendered through zeno's `Mask` with
  `size`, `offset` and `render_into` (zeno 0.3.3 `mask.rs:136-191`).
  - It is rendered one band at a time, a band being at most `B` rows of
    the tile (proposal: B = 64), into a coverage scratch of at most
    `T x B` bytes.
  - Each band is composited into the tile immediately, so no glyph raster
    accumulates.
  - The path fed to zeno is the flattened, clipped segment sequence from
    the counting pass, emitted again without storing it. zeno therefore
    rasterizes line segments only. *Provisional:* that its work is linear
    in the admitted charge does not yet hold, because of zeno's row
    searches and the repeated passes per band (open item OD1).
- **Rasterizer storage.** zeno keeps raster cells (16 bytes each: x, area,
  cover and a link) and one row index per band row (`raster.rs:569-625`,
  and the adaptive storage that spills to the heap at `:627-740`).
  - The tile path uses one reused scratch. zeno's `HeapStorage` is
    internal, so the integration goes through its public `Scratch`;
    how is open item OD2.
  - Cells are unique per pixel within a row of the band, so their count is
    at most `(T + 2)·B`, and the row indices are at most `B`. Slice 1
    verifies that bound against zeno's clipping at the window edges.
  - *Provisional:* rasterizer storage is then at most
    `16·(T + 2)·B + 4·B` bytes, about 1 MiB at the proposed sizes,
    allocated once and reused. This counts occupied cells, not vector
    capacity or growth peaks (open item OD2).
- **Small glyphs.** A small glyph (proposal: widened bounds within
  256 x 256) may still be rendered whole and kept in the existing
  phase-glyph cache, under that cache's budget. Its charge is counted the
  same way. *Provisional:* this path needs the same bounded geometry, or an
  admission proof of its own (open item OD1).
- **Seams.** Every tile renders a glyph at the same phase, so seams match at
  the prepared scale. Text stretched during a live zoom
  ([rendering contract](../docs/rendering-platform.md#text-icons-and-images))
  can resample at seams, and slice 1 tests that.
- **Wrapped overflow.** An unbreakable word wider than the text area, which
  the current contract permits (`docs/contracts.md:1283`), gets horizontal
  tiles over the visible x-range.

#### What the guarantee covers

- **Covered:** tile coverage, the coverage scratch, zeno's cell and index
  storage, and the rasterization work for admitted glyphs. These are meant
  to be bounded before allocation, as above. *Provisional:* the work and
  storage bounds depend on open items OD1 and OD2.
- **Outside the guarantee:** allocations that derive from the font and
  happen before admission can be decided. They are bounded only by the font
  data and by Parley's, swash's and skrifa's own limits:
  - font parsing;
  - Parley shaping;
  - hinting-program execution;
  - `scale_outline`'s outline buffers, whose size follows the glyph's point
    and contour counts (TrueType glyph data addresses points with 16-bit
    indices);
  - composite-glyph expansion.

  These are the allocations today's path also makes. Fonts are bundled or
  registered by the application, and are trusted to the same degree as
  today.

#### Markers

Whole-glyph replacement is the chosen policy. Omission was rejected
because it hides text silently. Refusal was rejected because whether a glyph
is excessive depends on font, size and scale, not on the document.

- **Geometry.** A marker is an outlined rectangle drawn as vector fills,
  not coverage. Its box:
  - is the glyph's widened bounds intersected with its line box;
  - grows, if needed, to a positive minimum extent of
    `max(1 physical pixel, 0.25 em)` in each dimension, centered on the
    glyph's pen position plus half its advance. A zero-advance combining
    mark therefore still gets a visible marker.
- **Discovery.** A marker's box is meant to lie inside its line box.
  *Provisional:* growth to the minimum extent, an empty intersection or a
  zero-width line box can break that (open item OD3). The glyph's
  candidate bounds for tiles and for the fill are the union of its widened
  bounds and its marker box, so discovery and drawing agree.
- **Clipping and order.** Markers are clipped by the text area like text.
  For each paragraph, they are painted after that paragraph's tiles, in
  logical glyph order, in the text color.
- **Count.** Inspection reports `raster_markers: Number(count)` (proposal):
  the number of distinct glyphs drawn as markers in the paragraphs of the
  last painted frame, deduplicated by paragraph and glyph index, not
  counted per tile.
- **Lifetime.** Admission is fixed per shaped paragraph, so a paragraph's
  markers change only when it is shaped again, after a content, direction
  or `G` change. The count also changes when paragraphs scroll into or out
  of the painted frame. It does not flicker with phase, crop or scroll
  position for a paragraph that stays visible.
- **Notification.** Whether hosts also get an after-settling notification
  is open question 9.
- **Other failures.** Font read failures and invalid geometry still fail as
  today.

#### Memory bound, in physical pixels

One coverage byte per pixel, so scale is counted once. For one frame `F`,
let:
- `A_F` be the area of the visible region plus the halo `M_h` above and
  below;
- `w_F` its width;
- `N_F` the number of paragraphs intersecting it, halo paragraphs included;
- `M_h` bound each paragraph's ink beyond its line boxes.

Then:
- **Frame coverage** `C_F` is at most `A_F + 2·N_F·M_h·w_F + R_F`, where the
  round-off `R_F` is at most one pixel row and column per tile of `F`.
  *Provisional:* outward rounding of both edges can exceed that (open item
  OD6).
- **While a new frame is recorded,** the previous display list may still
  hold tiles the new one does not. Total coverage is at most
  `C_old + C_new`, each computed with its own frame's dimensions, scale,
  halo and round-off. A viewport or scale change can make `C_old` the
  larger term.
- **Rasterizer storage** adds one coverage scratch of at most `T·B` bytes
  and zeno storage of at most `16·(T + 2)·B + 4·B` bytes. *Provisional:*
  that term excludes vector capacity (open item OD2).
- **Retained tiles** beyond the current frame are bounded by the tile cache
  budget (proposal: 32 MiB).
- **Font-derived allocations** fall outside this bound, as listed under
  "What the guarantee covers".

Slice 1 verifies the accounting with heap counts.

**No capacity guarantee yet.** Nothing in this plan claims that a value
within capacity cannot fail until slice 1 settles open items OD1, OD2, OD3,
OD5 and OD6, implements admission, and its tests pass. After that, raster limits produce markers, not errors, within the
coverage of "What the guarantee covers"; the remaining failures are those
listed under "Markers".

### Caches

- **Generation.** The layout generation `G` changes when any shaping input
  that applies to every paragraph changes:
  - the wrap width in physical pixels;
  - the effective scale;
  - the style;
  - the font registry, for example a font replacement.
- **Shaped-paragraph cache.** Keyed by content, resolved direction and `G`
  (see "Direction context"). It is keyed by content, not position, so after
  a split or merge unchanged paragraphs keep their entries. It is
  editor-owned with a byte budget (proposal: 16 MiB), independent of the
  shared text cache (`text.rs:1110`).
- **Tile cache.** Keyed by:
  - the shaped key;
  - the tile cell;
  - the crop;
  - the text rendering mode;
  - in `PhaseLine` mode, the quarter-pixel physical phase of the
    paragraph's origin, as `TextLayout::paint_turned` selects phase
    coverage today (`text.rs:330-352`).

  Fractional scrolling in `PhaseLine` mode therefore rasterizes new tiles.
  Colors are resolved at paint time and are not part of the key.
- **Height index.**
  - Each entry is *measured at G with direction d*, or *estimated*.
  - A `G` change turns every measured entry into an estimate that keeps its
    old height.
  - A direction-stale tag does the same for its range.
  - An edit re-measures the touched paragraphs synchronously.
  - Enter inserts an entry, and a merge removes one, in O(log n).
- **Budgets beyond residency.** Accounting includes the transient
  allocations of each edit (shaping the touched paragraphs and the visible
  fill, rasterizing tiles) and the display-list resources that hold tile
  coverage, as in "Memory bound".
- **Visibility and lifetime.**
  - Hiding releases all tiles and their display resources, and keeps shaped
    paragraphs within budget.
  - Unmount releases everything the editor owns.
  - Nothing runs while the editor is hidden. There is no background
    executor and no idle polling (AGENTS.md scheduling rules).

### Visible fill, anchors and offsets

**The visible fill** is convergent by construction and replaces revision 2's
repeated passes. Positions inside the filled range come only from measured
heights, counted from the anchor; estimates are used only outside it.

- **The visible region** is the text area's visible rectangle in physical
  pixels, widened by an overscan `O` of half its height above and below
  (proposal). A logical viewport larger than 8192 physical pixels, or
  larger than any coverage budget, just has more tiles; no viewport limit
  exists.
- **The filled range** is the visible region plus the overhang halo `M_h`
  above and below (see "Overhang limit"). Ink from a paragraph outside the
  filled range cannot reach the visible region, because rasterized ink
  stays within `M_h` of its line boxes and markers stay inside them.
  *Provisional:* marker confinement is open item OD3.

1. **Resolve the anchor** (see "Anchors") to a paragraph and a line in it.
   Shape that paragraph at the current `G` and its resolved direction.
2. **Fill downward.** Shape each following paragraph in turn, placing it
   below the previous one at its measured height, until the bottom of the
   filled range is covered or the document ends.
3. **Fill upward.** Shape each preceding paragraph in turn, placing it above
   at its measured height, until the top of the filled range is covered or
   the document starts.
4. **At the document start**, if space remains above, the anchor's screen
   position is lowered (clamped to the top), and step 2 continues downward.
5. **Write back.** Write the measured heights into the index, then update
   the scroll offset from the index's prefix sum above the first filled
   paragraph, which keeps the scrollbar consistent.

Every step shapes one paragraph and advances by a positive height, at least
the minimum line height `h`. The fill therefore ends after at most
`(H_v + 2·O + 2·M_h)/h + 2` paragraphs, for the text area's visible height
`H_v`, the overscan `O` and the halo `M_h`, each paragraph at most the
per-paragraph limit. That count includes the halo paragraphs, which are
shaped (and admitted) so that their incoming ink is drawn, and it is the
`N_F` of the memory bound. There is no iteration cap to exceed and no
continuation to schedule.

- **Paint** uses the filled set.
- **Layout** runs the fill in the editor's `arrange`, during the flush.
- **Input** that needs geometry runs the same bounded fill first if `G` or
  the document changed since the last one. Event contexts can prepare text
  synchronously, as `TextArea`'s edits already do (`text_fields.rs:3195`).
  Hit testing and caret geometry therefore use only paragraphs measured at
  the current `G` and direction.
- **A hidden editor** does not fill.
- **A jump** (Ctrl+End, a scrollbar drag, page movement) resolves its target
  y to a paragraph through the estimated index, uses that as the anchor,
  and fills from it.

**Anchors.**

- **Identity.** An anchor is a caret-stop document offset plus a pixel
  distance from the top of the visual line containing that offset, taken
  with Downstream affinity.
- **Priority**, applied at each fill:
  1. a pending caret reveal, after caret movement or an edit;
  2. otherwise the caret's line, when the caret is visible;
  3. otherwise the first visible line.

  An offscreen caret never pulls the view.
- **After reflow** (a width, scale or font change), the offset is usually
  no longer a line start. The anchor line is the visual line that now
  contains the offset, and the pixel distance is kept relative to that
  line's top.

**Offsets after a change.** The anchor and any retained offset move through
each `TextChange`:
- an offset before `removed.start` is unchanged;
- an offset at or after `removed.end` is shifted by the change's length
  difference;
- an offset inside the removed range moves to `inserted.end`.

If the moved offset is no longer a grapheme boundary, because the change
joined graphemes, it snaps to the start of the containing grapheme. This is
an internal presentation mapping. Explicit offsets passed to public APIs are
still rejected, never snapped.

**Tests:**
- scale change, width change and font replacement;
- a rendering-mode change;
- fractional scrolling in `PhaseLine` mode;
- estimate replacement under each anchor priority, with estimates far off
  in both directions;
- fill bounds at a narrow width with many short paragraphs;
- the anchor after reflow and after joining deletions;
- cleanup after hiding and unmounting, by heap counts in an ignored release
  test.

**Coordinates.** Document positions are f64, as `VirtualAxis` offsets are.
Paint subtracts the scroll offset in f64 and converts to f32 relative to the
viewport. A document of 50,000 lines at 19.6 pixels is about 980,000 pixels
tall, where f32 resolves only about 0.06 pixels.

### Selection, caret and movement

Positions are document byte offsets with the validation rules of
`TextSelection`: explicit offsets must be grapheme boundaries and are
rejected, not snapped. A `DocumentSelection` (proposal) holds an anchor and a
focus, each an offset with a pert-ui-owned affinity (proposal:
`CaretAffinity { Upstream, Downstream }`; Parley's type is not exposed), plus
the preferred x.

- **Left and Right** move visually within a paragraph by Parley's visual
  movement, which keeps and updates affinity at soft wraps and bidi
  boundaries.
  - **Crossing.** When visual movement in a direction cannot go further,
    that is, Parley returns the same offset and affinity, the caret leaves
    the paragraph. Right leaves an LTR paragraph forward and an RTL
    paragraph backward; Left mirrors that.
  - **Entering.** Moving forward enters the next paragraph at its logical
    start `s` (Downstream). Moving backward enters the previous paragraph
    at its logical end `e` (Upstream).
  - The entry point is a logical endpoint, which may lie inside the line
    rather than at a visual edge. For example, moving backward into an LTR
    paragraph that ends in an RTL run places the caret at the trailing edge
    of that run's logically last cluster.
- **Collapsing.** Without Shift, Left or Right on a non-collapsed selection
  collapses to the end that comes visually first or last. Ends are compared
  by document y (paragraph top plus line top), then x: Parley's rule
  (`editing/selection.rs:171-200`) in document coordinates. Up, Down and
  page movement without Shift move from the focus and collapse.
- **Preferred x:**
  - set from the focus's x at the first vertical or page movement;
  - kept across consecutive vertical and page movements, including across
    paragraphs;
  - reset by any horizontal, word or line-edge movement, click, edit or
    external replacement;
  - kept across a width or scale change, and used in the new geometry.
- **Up and Down** move into the neighboring paragraph's last or first line
  at the preferred x, shaping that paragraph first if needed.
- **Page movement** (proposal) moves by the visible height through the
  index.
- **Word movement** crosses to the adjacent paragraph's nearest word
  boundary.
- **Line start and end** stay on the displayed line. **Document start and
  end** shape only the first or last paragraph.
- **Hit testing** finds the paragraph by binary search of y over the filled
  set, then uses Parley's hit test in paragraph coordinates. The hit test
  supplies the affinity on either side of a bidi boundary. A drag past the
  viewport autoscrolls and keeps extending.
- **Double-click** selects a word within its paragraph. **Triple-click**
  selects the paragraph (proposal; open question 5).

### IME through the paragraph engine

The composition is shown as a temporary projection of the document through
the same engine. There is no separate large prepared unit.

- **State.**
  - `R = r0..r1`, half-open, in committed offsets: the selection range when
    composition starts, as `compose` takes it today
    (`text_fields.rs:3249-3253`).
  - The preedit text `P`, of length `p`, which may contain LF.
  - The preedit cursor, as given by `TextPreedit`.

  `R` is kept whether or not the preedit is displayed. The committed
  selection is hidden while composing.
- **Projection.** The displayed document is the committed one with `R`
  replaced by `P`:
  - paragraphs entirely outside `R` keep their content;
  - the paragraphs `R` touches are replaced by the paragraphs of the touched
    prefix, then `P`, then the touched suffix, split at LFs.

  Projected paragraphs get resolved directions from the projected context.
  The following neutral run is direction-stale within the projection only
  (see "Direction context"). A height overlay replaces the touched index
  entries and carries the stale tag. The committed index and caches are not
  modified.
- **Forward mapping**, from a committed offset `c` to a display offset. It
  is total, and its rules are tried in order:
  1. `c < r0` maps to `c`;
  2. `c == r0` maps to `r0`, the preedit's start, both for an insertion
     (`r0 == r1`) and for a nonempty replacement;
  3. `r0 < c < r1`, which occurs only for a nonempty replacement, maps to
     `r0`. These committed positions are hidden while composing, and the
     mapping collapses them onto the preedit's start;
  4. `c >= r1` with `c > r0` maps to `c - (r1 - r0) + p`.

  For an insertion, the insertion point itself takes rule 2, and every later
  offset takes rule 4. For a nonempty replacement, `r1` takes rule 4 and maps
  to `r0 + p`.
- **Endpoint ownership.** Display `r0` belongs to the preedit, as its start
  before the first byte. Display `r0 + p` is the preedit's end; for a
  nonempty replacement it is also where committed `r1` maps, so the suffix
  owns it. The preedit occupies the display range `r0..r0+p`.
- **Reverse mapping**, from a display offset `d` to a committed offset, used
  by hit tests during composition:
  - `d < r0` maps to `d`;
  - `d >= r0 + p` maps to `d + (r1 - r0) - p`;
  - `r0 <= d < r0 + p` lies in the preedit.

  A pointer press anywhere repositions the caret. Today's contract cancels
  the composition then (`TextInputState` session rules), so the press first
  cancels, then hit tests against the committed layout. The reverse
  mapping is used only for hover and cursor shape over the projection.
- **Refused display.** If a projected paragraph would exceed the
  per-paragraph limit, or the projected document the capacity, the preedit
  is not shown, matching today's rule (`text_fields.rs:3256`). The
  composition continues, with `R` and the session kept. The commit goes
  through ordinary edit validation and is refused silently if it does not
  fit.
- **Caret geometry.**
  - While the projection is displayed, `TextInputState.caret` is the
    composition caret at display `r0` plus the preedit cursor's end.
  - While display is refused, it is the committed layout's caret at `r0`,
    with Downstream affinity. The committed layout exists, because the
    committed document is shown unchanged.
  - Either way the rectangle is clamped into the text area, as
    `input_state` does today (`text_fields.rs:3147`).
- **Anchors when the projection appears or disappears.** The anchor stays
  in committed offsets and is shown through the forward mapping:
  - an anchor at or before `r0` shows at its own offset, or at `r0`;
  - one inside a nonempty `R` shows at `r0`;
  - one at or after `r1` shows shifted while the projection is displayed.

  The committed anchor itself does not move. Only an external write or the
  commit's `TextChange` moves it (see "Offsets after a change").
- **Grapheme joins across the preedit's ends.** The projected paragraph is
  shaped as one text, so the prefix, the preedit and the suffix may join
  into one grapheme at either end. An example is a preedit that starts with
  a combining mark after an `a`.
  - The display shows the joined cluster, as shaping produces it.
  - Display offsets are internal and are never validated as caret stops.
    The selection-validation rule applies only to committed offsets and to
    explicit offsets passed to public APIs.
  - The composition caret is drawn at the nearest display grapheme boundary
    at or before `r0` plus the preedit cursor, as `TextSelection::snapped`
    places carets today (`text_selection.rs:111`).
  - The committed range `R` is kept unchanged whatever joins.
  - At commit, the inserted text joins the same way in committed text, and
    the caret after the commit snaps to the start of the containing
    grapheme, as in "Offsets after a change" and as `TextArea`'s edits
    place it today (`text_fields.rs:3213-3217`).

  The caret reveal follows the composition caret when the projection is
  displayed, and the committed caret at `r0` when it is refused.
- **Commit** replaces `R` with the committed text as one edit: one
  `TextChange` and one undo entry, published in the order above.
- **Cancellation** (Escape, an empty preedit, an external write, caret
  repositioning, focus loss or hiding) drops the projection and its
  overlay. The session increases on every cancellation, as the
  `TextInputState` contract requires. On counter exhaustion the editor
  returns no input state, which disables IME (`text_input.rs:396-399`).
  Today's editors already do this with `checked_add`
  (`text_fields.rs:2254`).
- **Tests:**
  - preedit that inserts an LF, and one that removes LFs by replacing a
    selection containing them;
  - a cross-paragraph selection replaced by a commit;
  - preedit that changes the direction context of following neutral
    paragraphs;
  - a refused display followed by a commit that fits, and by one that does
    not, checking the fallback caret rectangle;
  - cancellation by each cause;
  - session exhaustion, using a test hook to start the counter near
    `u64::MAX`;
  - an external write and hiding during composition;
  - anchors before, at, inside and after `R`, for an insertion and for a
    nonempty replacement;
  - a preedit that joins a grapheme with the prefix, with the suffix, and
    with both.

### Extension points: no privileged built-in

AGENTS.md requires that a consumer can reproduce built-in widgets through
documented public extension points, with no exemption for experimental ones.

- **Internal until public.** The engine and the editor stay crate-internal
  until the extension points they use are public. No Parley, Swash, zeno or
  other dependency type appears in those signatures. The extension points
  are:
  - `TextDocument` and its revisions, changes and errors;
  - a pert-ui-owned shaped-paragraph type;
  - tile painting through `Painter`;
  - `DocumentSelection` with `CaretAffinity` and its geometry operations;
  - the height index, or the equivalent part of `VirtualAxis`.
- **The prototype is never exported.** The internal prototype (slice 4) is
  not public, and it is not hidden behind `#[doc(hidden)]` either.
- **Acceptance requirement for the public editor (slice 8).** A custom
  document editor written with only `pert_ui` facade imports must do all of
  the following. It lives in the facade's editing tests and in the isolated
  consumer check. If any part needs a private API, the public editor waits.
  - shape and tile paragraphs;
  - place and move a cross-paragraph selection;
  - hit test;
  - provide `TextInputState`;
  - paint, scroll and edit.

`TextSelection` stays as it is: a selection in one prepared text, used by
`TextArea`, `TextField`, `SelectableText`, `DraftEditor` and custom editors.
Its rustdoc gains a sentence pointing to `DocumentSelection` for
paragraph-partitioned text. No breaking change is proposed.

### Capacity separation and the raster limit

Proposals, with the values decided at the checkpoint:

- **One prepared text.** The shared constant becomes the limit of one
  prepared text (labels, rich text, toasts, ellipsis fitting, preedit) and
  of one document paragraph. It may move next to `TextLimit` (open
  question 8). The Parley style-run assertion stays tied to it.
- **Editor capacities.** `TextField::MAX_BYTES` and `TextArea::MAX_BYTES`
  (proposals), checked by `check` and by binding refusal. `TextField` stays
  at 16 KiB. `TextArea` may rise after slice 6, with each of its paragraphs
  within the prepared-text limit.
- **Document capacity.** `TextDocument::MAX_BYTES` (proposal).
- **Errors.** The error policy says "One limit has one reporting variant
  across paths that share the limit" (`docs/contracts.md`). Separate limits
  therefore get separate variants:
  - `TextEditError::TooLarge` keeps the prepared-text limit;
  - `TextDocumentError::TooLarge` and `ParagraphTooLong` report the
    document's limits.

  `TextEditError::TooLarge`'s documentation (`text_input.rs:4-8`) changes
  accordingly.
- **The raster limit.** With per-paragraph admission, `MAX_RASTER_SIDE` no
  longer bounds the viewport, a paragraph or the document. It survives as
  one of the individual-glyph checks, beside the overhang limit and the cell
  charge, and tiles are far smaller. Raster limits then produce markers, not
  errors, for the allocations admission covers. Font-derived allocations
  stay outside that guarantee (see "What the guarantee covers"). This holds
  only once slice 1 settles the open raster items and is implemented and
  tested; until then the plan makes no such claim.

## Open design items (from the fourth review)

Codex's fourth recheck (`plans/todo-repairs-review/design-s7-r4.md`)
found that the raster admission and memory guarantees still depend on
unresolved specifications. The maintainer decided on 2026-11-04 to accept
the plan as direction and to settle these items by prototype in the first
implementation slices, not by further plan revisions. The claims they
affect are marked *provisional* in place.

- **OD1. Work charge and winding-preserving clipping (blocker; slice 1).**
  - zeno's `HeapStorage::set` searches a row's cell list from its head on
    every update (`raster.rs:595`), so distinct x cells can cost quadratic
    traversal. Replaying the path for each band repeats flattening and
    traversal of clipped-away segments.
  - Clipping individual segments loses the winding of a contour that
    encloses a cell without crossing it. zeno itself keeps left-side
    winding through a sentinel cell (`raster.rs:250-262`). Clipping before
    the subpixel phase is applied can also drop geometry.

  To decide: a winding-preserving clip with contour closure and phase
  handling; a charge covering all emitted and traversed work, band replay
  and row searches, or a storage with directly bounded updates. The
  whole-glyph small-glyph path needs the same geometry or its own proof.
- **OD2. Vector capacities and zeno's public `Scratch` (slice 1).** The
  storage formula counts occupied cells. zeno's `HeapStorage` grows vectors
  by `push` and keeps their capacity after `clear` (`raster.rs:569-625`),
  so capacity can exceed the live length. `HeapStorage` is also internal:
  only `Scratch` is public (`scratch.rs:14-19`, re-exported in `lib.rs:302`).
  To decide: account for capacities and growth peaks, or use storage with
  controlled preallocation. Name the actual integration through `Scratch`,
  and update the frame's scratch term.
- **OD3. Marker confinement (slice 1).** Growing a marker to its minimum
  extent and recentering it can escape the line box. An excessive,
  displaced glyph can have an empty intersection with the line box, and a
  positive-width marker cannot fit a zero-width line box. The halo argument
  assumes markers stay inside line boxes. To decide: marker geometry for
  empty intersections and zero-advance lines, with vertical confinement
  enforced after the size adjustment, or a bounded marker overhang that the
  halo includes.
- **OD4. Undo retained allocations (slice 2).** Deleting a large range can
  leave history holding a whole removed subtree, including nodes that were
  never copied. Empty paragraphs are charged zero although their
  allocations stay alive, and `Arc` headers, revision tokens and history
  bookkeeping are not charged. To decide: charge every allocation that
  history retains exclusively, with overhead. Reserve both restoration
  directions when recording an entry, with an explicit policy for moving an
  entry between undo and redo. The exact-state semantics stay.
- **OD5. Flattener depth exhaustion and coverage tolerance (slice 1).** The
  plan gives both a quarter-pixel tolerance and a maximum depth, but not
  what happens when the depth runs out before the tolerance is met.
  Geometric deviation also does not bound coverage-byte differences. To
  decide:
  - the flatness criterion and the depth-exhaustion outcome, with a marker
    as the preferred terminal fallback;
  - rendered-coverage acceptance, kept separate from the geometric
    tolerance;
  - tests that compare tiled with whole rendering of the same flattened
    path, so seams are tested apart from the curve approximation.
- **OD6. Frame round-off on both edges (slice 1).** Rounding a rectangle
  outward on both edges can add more than one pixel per axis: `[0.9, 1.1]`
  becomes two pixels from a width of 0.2. To decide: derive `R_F` from both
  rounded edges, including the corner area, or state the formula entirely
  in already-rounded integer rectangles. The `C_old + C_new` structure
  stands.

## Public API proposals

All are proposals, and the names are open (open question 1). Slice 7
publishes the data and extension-point types; slice 8 publishes the
component.

- **Slice 7, data types.** Each is usable on its own, with a doctest that
  needs no editor:
  - `TextDocument`: `new(&str)` (fallible), `len`, `is_empty`,
    `paragraph_count`, `paragraph(index)`, a paragraph iterator,
    `slice(range)` (fallible), `replace(range, text)` (fallible,
    persistent, returning an optional change), `revision`, `Display`, and
    serde under the `serde` feature;
  - `DocumentRevision`, `TextChange` and `ChangeKind`;
  - `TextDocumentError` (`InvalidRange`, `InvalidText`, `TooLarge`,
    `ParagraphTooLong`).
- **Slice 7, extension points**, built on the data types and usable from a
  custom component, with doctests:
  - a document layout view, made of shaped paragraphs and the height index,
    over a `TextDocument`;
  - tile painting through `Painter`;
  - `DocumentSelection` and `CaretAffinity`, whose movement, hit-test and
    geometry operations take that layout view.
- **Slice 8, `DocumentEditor`**, a component:
  - `DocumentEditor::new(document: Signal<TextDocument>)`, with the required
    state owner leading;
  - builders: `style`, `editable(Permission)`, `read_only`,
    `enabled(Permission)` and `disabled`, composing as the editors' do;
    `wrap(bool)`; `on_binding_refusal`; `on_change` (a user-write callback
    taking `&TextChange` and `&TextDocument`); later `local_undo(bool)`;
  - it owns a clipboard menu, local undo and a scrollbar;
  - inspection reports the length, paragraph count, selection, scroll, the
    visible paragraph range and `raster_markers`. It does not include the
    text by default (open question 7).
- **Page movement**, a `TextMovement` variant or a document movement (open
  question 5).
- **`TextField::MAX_BYTES` and `TextArea::MAX_BYTES`** (slice 6), and the
  relocated prepared-text limit if chosen, each with its replacement usage
  recorded.

Each public slice runs the API listing gate, names every listed change in
the CHANGELOG and documents every item.

## Persistence and compatibility

- **No persisted format changes.** The library persists no document content;
  saving is the application's, possibly from `TextChange` deltas.
  `TextArea` state is not in the workspace document today, and this plan
  does not add the document editor to it.
- **Workspace hooks, if added later.** Capturing scroll or selection needs
  its own decision:
  - a workspace schema version;
  - offsets validated against the current document on restore;
  - rejection, not clamping, of out-of-range or non-boundary offsets;
  - round-trip and rejection tests
    ([workspace persistence](../docs/contracts.md#workspace-persistence)).
- **Serde.** `TextDocument` serializes as one string and validates on
  deserialization like `new`, documented in a `# Serialization` section.
  Revisions are not serialized: a deserialized document is a fresh state
  with a fresh revision.
- **Breaking changes.** While the library is unpublished, breaking changes
  are welcome when they improve the API. Moving the prepared-text limit and
  adding the capacity constants are such changes. Each records its
  replacement usage and migrates the repository's tests, examples, guide and
  gallery, with no compatibility aliases.
- **`TextArea` behavior changes**, only in slice 6 and only as the
  checkpoint decides (open questions 3 and 10). The CHANGELOG records each
  one adopted as a behavior change, with migration notes:
  - raster limits produce markers instead of flush and input errors, once
    slice 1's admission algorithm exists;
  - mixed-direction layout follows the intended text contract, if accepted;
  - its paragraph limit.

## Measurements and targets

**Provisional targets, confirmed by the maintainer before any slice 0 or
later results are examined (open question 4).**

The measurement conditions are frozen with the targets:

| Condition | Value |
|---|---|
| Machine | The reference machine of "Baseline", pinned to a performance core, release profile, with the counting allocator active |
| Viewport | The editor filling 800 x 600 logical pixels, at scale 1 and at scale 2. Both are measured, and the targets apply to each |
| Rendering mode | `LineMask`, the default. `PhaseLine` is recorded for information and has no target |
| Typography | The editor's default, bundled Noto Sans Mono at 14 logical pixels |
| Warm state | After mount, the first frame, placing the caret in the target paragraph by direct selection and 5 warm-up operations of the measured kind |
| History | Empty before the warm-up, so a series runs with the warm-up entries and its own. One extra memory run starts with a full history (100 one-byte entries) |
| Steady state | The live heap after the warm state, before the timed series |
| Timed span | From the input call through flush and paint, which includes display-list recording. Backend presentation (GPU or CPU rasterization of the display list) is excluded and has no target; native slices record it separately |
| Included allocations | Everything through the global allocator during the timed span. A headless clipboard stub's allocations are reported separately |

| Measure | Provisional target |
|---|---|
| Insert, Enter and Backspace merge in paragraphs up to 2 KiB, at 1 MiB of prose and at 50,000 log lines | p95 at most 8 ms |
| The same edits inside a maximal (16 KiB) paragraph | p95 at most 33 ms (two 60 Hz frames) |
| Page movement and scrolling one page | p95 at most 8 ms |
| Steady heap increment over an empty editor, for a 1 MiB document | at most 3 times the document size, plus the documented cache budgets |
| Peak above steady during one insert, Enter or Backspace merge, in paragraphs up to 2 KiB | at most 8 MiB |
| Peak above steady during a paste of `k` bytes | at most 8 MiB + 3·`k` |
| Peak above steady during select-all and copy | at most 8 MiB + 2 times the document size |

**Two groups of measurements:**

- **Pre-checkpoint primitive and prototype measurements** (slices 1-4).
  These run as ignored release-profile tests inside `pert-ui-runtime`, which
  print figures and do not gate CI. They measure:
  - tile admission and rasterization time and memory per tile, including
    an exhausted cell and the flattening charge;
  - shaping time for paragraphs of 1, 2, 8 and 16 KiB, with and without
    outline bounds;
  - `TextDocument` `replace`, `clone` and `eq` at 1 MiB and at 50,000
    paragraphs;
  - height-index insert, remove, lookup and range tagging;
  - the visible fill at the bound;
  - the internal prototype's edit, paste, copy, external replacement and
    scripted composition on the target corpora, under the frozen
    conditions.
- **Acceptance measurements** (slice 8), through the public facade, with
  the [bench](document-editor-bench/) extended:
  - *corpus:* 1 MiB of prose; 50,000 log lines of 60 to 200 bytes;
    mixed-direction paragraphs with combining marks, emoji ZWJ sequences and
    tabs; the 64 KiB line as a refusal case;
  - *operations:* insert, Enter and Backspace merge; a 100 KiB paste;
    select-all and copy; Down across a boundary; page movement; Ctrl+End;
    replacing the whole external value; a scripted composition;
  - *statistics:* median, p95 and maximum, with raw output preserved.

Figures are labeled with the slice and path they come from. None is
presented as a current-path figure.

## Slices

Each slice is one branch with its own version bump and CHANGELOG entry.
Gates follow the AGENTS.md table:
- slices 1-4 change the text service's coverage partition, which the
  renderers consume through the display list, and add scheduling-relevant
  layout behavior. They run the **workspace gate with focused contract
  tests and semantic review**, not only package-local checks;
- public slices (6-8) add the API listing gate and a Codex review.

0. **Targets and conditions.** The maintainer confirms the provisional
   targets and the frozen conditions. The current-path baseline in this
   plan is done.
1. **Tiled rasterization of shaped text** (internal):
   - raster descriptors and cached outlines;
   - the overhang limit;
   - per-paragraph admission with the flattening charge;
   - cropped per-paragraph tiles with banded rendering;
   - markers;
   - the tile cache and the memory accounting.

   It is the raster prototype that settles open items OD1, OD2, OD3, OD5
   and OD6 first. Tests compare tiled coverage with whole-layout coverage
   on small texts, with a tolerance that OD5 defines. They also cover:
   - a stacked combining-mark sequence that exhausts a cell, with visible
     zero-advance markers;
   - a complex outline in a tiny window, which the charge counts;
   - an excessive glyph from a test font, by size, by overhang and by
     charge;
   - fallback and variable fonts, checking bounds against the rendered
     coverage;
   - admission and the marker count staying the same across viewport crops,
     fractional scrolls and phases;
   - ink from a paragraph in the halo, just outside the visible region;
   - zeno storage staying within its bound, by heap counts;
   - the old and new frame terms after a viewport shrink and a scale
     change;
   - seams under live zoom;
   - both rendering modes with phases.
2. **Document store, revisions, edit and undo contract, direction context,
   height index, caches, visible fill and anchors** (internal). It settles
   open item OD4 first. Tests:
   - sibling branches from one document have distinct revisions;
   - a same-text replacement is a no-op, with the same revision, no
     history entry and no callback;
   - undo and redo restore exact revisions in the specified notification
     order;
   - an equal-content write from another lineage is suppressed and leaves
     the editor unchanged;
   - an application write between the editor's update and its publication
     wins;
   - an edit larger than the whole undo budget clears history;
   - the undo charge for one-byte edits in a large paragraph, against heap
     counts;
   - direction-run invalidation;
   - the fill bound;
   - the anchor and offset rules.
3. **Selection, movement, hit testing and boundary mapping** (internal):
   - the normative table as tests;
   - the required mixed-direction, neutral and empty-paragraph cases, both
     across paragraphs and within one line;
   - first, characterization tests that record today's `TextArea` behavior
     on the same cases.
4. **Internal end-to-end prototype** (not exported): editing, local undo,
   clipboard, IME projection with its tests, scrollbar and inspection. The
   primitive and prototype measurements run under the frozen conditions.
5. **Design checkpoint** (maintainer and Codex), using slices 1-4 and their
   measurements. It decides:
   - the capacities;
   - the direction contract and whether `TextArea` adopts it;
   - the marker policy for `TextArea` (open question 10);
   - the extension-point shapes;
   - the names;
   - change-aware adoption;
   - whether to continue.

   The plan is revised before slice 6.
6. **Capacity separation and `TextArea` on the engine:**
   - `TextField::MAX_BYTES` and `TextArea::MAX_BYTES`, and the relocated
     prepared-text limit if chosen;
   - `TextArea` prepared per paragraph with admitted tiles, so that raster
     limits no longer surface as flush or input errors (the TODO.md entry
     of 2026-11-04);
   - the behavior changes the checkpoint adopted;
   - consumer migration.

   It merges only together with slice 7, or after it, so no release has a
   built-in editor on non-public internals.
7. **Public data types and extension points:** `TextDocument`,
   `DocumentRevision`, `TextChange`, `ChangeKind` and `TextDocumentError`,
   then the layout view, tile painting, `DocumentSelection` and
   `CaretAffinity`. Each comes with examples, run as doctests, that use
   only what this slice publishes.
8. **Public `DocumentEditor`**, built only on slice 7:
   - the facade-only custom-editor acceptance test and the isolated
     consumer check;
   - headless contract tests for refusal, external replacement during
     composition, hiding, unmount and focus loss;
   - the acceptance measurements;
   - a gallery page and a guide section.
9. **Native evidence** on Fedora GNOME/Wayland:
   - IME composition across a paragraph boundary;
   - pointer selection with drag autoscroll;
   - scrolling a 1 MiB document;
   - HiDPI tiles and markers.

   Windows and macOS stay open, as for the other native records.

Host-owned undo with change-aware adoption follows as its own slice after
the checkpoint decides open question 6.

## Risks

- **Divergence between two editors.** Mitigated by shared crate-internal
  key mapping (`key_movement`, `text_fields.rs:1280`), clipboard command
  handling and menu code. The mixed-direction differences from `TextArea`
  are deliberate and recorded.
- **Outline and charge cost.** Scaling outlines and counting the flattening
  charge add work per glyph before rasterization. Slice 1 measures it, with
  outline caching as the mitigation.
- **Markers for tall stacks.** The overhang limit turns marks stacked beyond
  twice the line height into markers. Heavily stacked diacritics show
  markers rather than ink.
- **Scroll offset drift** while estimates dominate the regions outside the
  visible fill. The fill keeps the view itself stable, but the scrollbar
  thumb moves as estimates are replaced.
- **Tile seams and phase coverage** under live zoom and in `PhaseLine` mode.
- **Memory.** The frame bound grows with the overhang term for many short
  paragraphs. While a new frame is recorded, the old frame's coverage adds
  to the new one, and the old frame can be the larger one after a resize or
  a scale change.
- **Long neutral runs.** Lazy invalidation keeps edits cheap, but scrolling
  through a stale run reshapes it paragraph by paragraph as it becomes
  visible.
- **Extension points take time.** Publishing a pert-ui-owned paragraph and
  tile API is a substantial design of its own, and the public editor waits
  for it.
- **Binding cost.** `Signal<TextDocument>` writes are cheap only if clone and
  equality are structural, so the persistent tree is required.
- **External replacement resets the selection** in the first version, which
  serves programmatic appends poorly until change-aware adoption exists.
- **IME platform differences** for compositions over cross-paragraph
  selections need native evidence.
- **Cost.** The plan spans the text service, a data structure, a component,
  public extension points and migrations. The checkpoint may stop it.

## Open questions for the maintainer

1. **Names:** `DocumentEditor`, `TextDocument`, `TextChange`,
   `DocumentRevision`, `ChangeKind`, `DocumentSelection` and
   `CaretAffinity`, or others?
2. **Extension points:** what shape should the public layout view and tile
   API take? Is the height index public on its own, or through a
   `VirtualAxis` extension?
3. **Direction contract:** are the per-paragraph resolved direction, the
   inheritance rule for neutral paragraphs and per-paragraph alignment
   corrections that `TextArea` also adopts in slice 6? Or must `TextArea`
   keep today's single-level behavior?
4. **Targets:** are the provisional targets and frozen conditions right?
   Which document capacity follows from them?
5. **Behavior:** should triple-click select a paragraph, and should page
   movement be added, also to `TextArea`?
6. **Change-aware adoption:** should an external write whose
   `last_change().base` (a proposal) is the editor's observed revision
   transform the selection and anchor instead of resetting them? It is the
   prerequisite for host-owned undo.
7. **Inspection:** no text, a bounded excerpt around the caret, or an
   opt-in for tests?
8. **The shared limit:** move it to `TextLimit` in slice 6, or keep it on
   `TextEditError` and only document the separation? Should
   `TextArea::MAX_BYTES` stay at 16 KiB or rise?
9. **Marker notification:** is the inspection count enough, or should hosts
   also get an after-settling notification when markers appear or
   disappear? And is twice the line height the right overhang limit?
10. **`TextArea` and markers:** when `TextArea` moves to the engine in slice
    6, should raster limits draw markers as in the document editor? Today
    they make the flush fail or the input call return an error. Or should
    `TextArea` refuse such a binding through binding refusal instead,
    keeping its form-field strictness?

## Findings outside this plan's scope

Revision 1 reported four findings, verified against 9aaacca. Their
disposition in the current working tree:

1. **A value that passes `TextArea::check` can fail the flush**, and
2. **an edit that crosses the raster limit returns an error from the input
   call.** Both are recorded by the author in TODO.md (the 2026-11-04 entry
   on `TextField`/`TextArea` binding refusal and raster limits). The entry
   points to slice 6 of this plan, which keeps that number and scope. Its
   reproduction figures (16,380 bytes, step 165) are the author's own probe
   and differ from this plan's (16,384 bytes, press 136). Both are reported
   experimental evidence.
3. **TEXT-01's list of what shares `MAX_TEXT_BYTES`:** expanded with the
   full list and the 32 KiB style-run ceiling. Done.
4. **The repair plan's `text_input.rs:104` reference:** corrected in
   `plans/todo-repairs.md` slice 7. Done.

What remains is the current-contract fix the TODO.md entry describes. It
belongs to that entry, not to this plan.

## Review record

- **2026-11-04, revision 1:** Codex (`codex exec -s read-only`): "needs
  rework", with three blockers, seven should-fix items and two nits
  (`plans/todo-repairs-review/design-s7.md`).
- **2026-11-04, revision 2:** addressed all twelve findings, which were
  verified against 9aaacca and Parley 0.12, and reran the bench with raw
  output preserved.
- **2026-11-04, Codex recheck of revision 2:** still "needs rework"
  (`plans/todo-repairs-review/design-s7-r2.md`).
  - Resolved: revision-1 findings 1, 3, 4, 9, 10, 11 and 12.
  - Partly resolved: 2, 5, 6, 7 and 8.
  - New: two blockers and ten further findings.
- **2026-11-04, revision 3 (this one).** Each point was checked against the
  code and Parley 0.12 before adoption:
  1. *State identity (blocker).* Verified: revision 2's counter collides
     across sibling branches, and `Signal::set` suppresses equal values
     (`signal.rs:554-569`). Adopted:
     - `Arc`-token revisions that are fresh per state, with no counter and
       so no exhaustion;
     - `TextChange.result`;
     - no-op replacements returning no change;
     - revision-based detection of external writes, with the publication
       check;
     - how equal-content writes, callbacks and undo agree;
     - the listed tests (slice 2).
  2. *Raster admission (blocker).* Verified: today's path renders the glyph
     before counting it (`text.rs:1396`, `:1437-1441`). Adopted:
     - conservative, phase-independent outline bounds through swash
       (`scale/mod.rs:508`, `outline.rs:86`);
     - an aggregate per-tile check before allocation;
     - preallocated tile and scratch buffers;
     - windowed streaming through zeno's `Mask` (`mask.rs:136-191`).

     The renewed capacity statements in the compatibility section and in
     slice 6 are now conditional on slice 1.
  3. *Direction identity.* Adopted:
     - resolved direction in the layout key;
     - O(log n) direction lookup through tree summaries;
     - lazy range invalidation through the next paragraph with an own
       direction;
     - qualified edit cost;
     - the same rules for the IME projection.
  4. *Affinity.* Verified at parley `editing/cursor.rs:80-85`, `:331` and
     following. Adopted:
     - affinity at bidi boundaries within a line;
     - explicit conversion at paragraph endpoints;
     - crossing after visual movement reaches its end, with entry at the
       logical endpoint;
     - same-line mixed-direction tests.
  5. *Visible fill.* Adopted: an anchor-first fill from measured heights
     only, with an explicit bound and no pass cap or continuation, covering
     paint, layout, input and hidden state. Also anchor resolution after
     reflow, and snapping after grapheme joins.
  6. *Memory bound.* Adopted:
     - cropped per-paragraph tiles; a shared document grid was rejected,
       with the reason given;
     - a physical-pixel bound with overhang, round-off, temporary and
       display-list terms.
  7. *IME geometry.* Adopted: half-open forward and reverse mappings,
     endpoint ownership, the committed-layout caret fallback when display is
     refused, anchor behavior, and direction effects beyond `R`.
  8. *Glyph policy.* Adopted: markers, with omission and refusal rejected
     for the reasons given. Also the `raster_markers` inspection count with
     its lifetime and phase independence, open question 9, and the
     `TextArea` choice as open question 10 and a checkpoint item.
  9. *Undo semantics.* Adopted: exact-state undo and redo, applicability,
     notification order, combined accounting, oversize edits and restored
     selections.
  10. *Frozen conditions.* Adopted: the condition table, steady state,
      included allocations, operation domains for peak targets, and
      presentation excluded.
  11. *Publication order.* Adopted: the data types move to slice 7 with
      independent doctests, and the extension points are built on them.
  12. *Byte count.* Fixed: `"a\n\nb"` is four bytes, and the table gains a
      byte column.

  No point was rejected. One choice where the review offered alternatives:
  for finding 2, the plan chose windowed streaming with an aggregate
  pre-check rather than only conservative candidate bounds, because the
  windows also bound the work for a single large glyph.

  The maintainer checkpoint is still outstanding.
- **2026-11-04, Codex recheck of revision 3:** still "needs rework"
  (`plans/todo-repairs-review/design-s7-r3.md`).
  - Resolved: revision-2 points 1, 3, 4 and 9-12.
  - Partly resolved: 2 and 5-8.
  - Remaining: one blocker and six findings.

  The recheck also confirmed that pointer-identity revisions are sound,
  provided `Eq` and `Hash` are identity-based and revisions stay
  nonportable. Revision 4 states both.
- **2026-11-04, revision 4 (this one).** Each point was verified against
  zeno 0.3.3, swash 0.2.10, Parley 0.12 and 9aaacca before adoption:
  1. *Rasterizer storage and work (blocker).* Verified: zeno's cell and row
     index storage spills to the heap (`raster.rs:627-740`), and curve
     subdivision depends on the path, not on the window area
     (`raster.rs:383-470`). Adopted:
     - a separate charge counted from pert-ui's own bounded flattening,
       with rows and columns crossed per segment plus the window area;
     - excessive glyphs by charge as the terminal fallback;
     - banded rendering, with a bound of `16·(T + 2)·B + 4·B` bytes on
       zeno's storage;
     - an explicit list of the font-derived allocations outside the
       guarantee.
  2. *Raster descriptor.* Verified: hinting is on (`text.rs:1361`), and
     swash applies emboldening and transforms after scaling
     (`scale/mod.rs:858-863`). Adopted: one descriptor (face, glyph, size,
     coordinates, hinting) and one cached outline, used both for bounds and
     for rendering (`Outline::path`, `outline.rs:81`). Synthesis is
     explicitly excluded, since pert-ui configures none today.
  3. *Markers.* Adopted:
     - whole-glyph replacement, decided per paragraph on a fixed,
       paragraph-anchored cell grid, so admission is the same for every
       crop, scroll position and phase, and the no-flicker claim holds;
     - a positive minimum extent of `max(1 px, 0.25 em)`;
     - discovery bounds as the union with the marker box;
     - clipping, painting after the paragraph's tiles in logical order, and
       a deduplicated count.
  4. *Incoming overhang.* Adopted: an overhang limit `M_h` makes glyphs
     reaching further excessive. The fill covers a halo of `M_h`, and its
     bound and the memory bound's `N_F` include the halo paragraphs.
  5. *IME mapping.* Adopted:
     - a total forward mapping, with `c == r0` explicit for insertion and
       for nonempty replacement;
     - endpoint ownership restated;
     - anchors shown through the mapping without moving;
     - presentation of grapheme joins at both ends of the preedit, with
       display offsets never validated and `R` preserved;
     - the matching tests.
  6. *Frame memory.* Adopted: `C_old + C_new`, each with its own
     dimensions, scale, halo and round-off, plus the coverage scratch and
     zeno storage terms.
  7. *Undo accounting.* Adopted the conservative choice: each entry is
     charged the full length of superseded paragraph allocations and the
     tree nodes actually copied, so 8 MiB bounds retained memory. The
     exact-state semantics are unchanged.

  No point was rejected. Two choices where the review offered alternatives:
  - for point 1, a work admission with a terminal fallback rather than
    withdrawing the work bound;
  - for point 4, an overhang limit with a halo rather than indexed extents.
    Indexed extents would need shaping for unvisited paragraphs, or font
    bounding boxes that variable fonts need not honor.

- **2026-11-04, Codex recheck of revision 4:** "needs rework"
  (`plans/todo-repairs-review/design-s7-r4.md`).
  - Resolved: revision-3 points 2 (raster descriptor) and 5 (IME mapping).
  - Partly resolved: points 1, 3, 4, 6 and 7.
  - Remaining: one blocker and five findings. The recheck judged the plan
    not ready for the design checkpoint while the admission and memory
    guarantees depend on them.
- **2026-11-04, maintainer decision.** Iteration on the plan stops, and
  revision 4 is committed as the design plan, accepted as direction. The
  fourth recheck's findings become open design items OD1-OD6:
  - slice 1, the raster prototype, settles OD1, OD2, OD3, OD5 and OD6;
  - slice 2, the document and undo slice, settles OD4.

  The claims they affect are marked provisional in place, and the status
  line says the plan gives no raster admission or memory guarantee yet.
  The design checkpoint (slice 5) still comes after those slices. TEXT-01
  stays open.

## Appendix: repeating the measurement

The program is [document-editor-bench/src/main.rs](document-editor-bench/src/main.rs),
with [its manifest](document-editor-bench/Cargo.toml). Raw outputs are in
[document-editor-bench/results](document-editor-bench/results/): complete
for round 2, and transcribed latency lines for round 1. The program is not a
workspace member, and it contains the one `unsafe impl GlobalAlloc` its
counting allocator needs, which the workspace lints would deny. Build it
outside the repository, never in `/tmp` (a RAM file system here) and never
in the repository's target directory:

```sh
REPO=$PWD                            # the pert-ui checkout
BENCH=$HOME/pert-ui-s7-bench         # any directory outside the repository
mkdir -p "$BENCH/head" "$BENCH/bench"
git archive HEAD apps crates tests Cargo.toml Cargo.lock rust-toolchain.toml \
  | tar -x -C "$BENCH/head"          # the measured commit, without local edits
cp -r "$REPO/plans/document-editor-bench/." "$BENCH/bench/"
cp "$REPO/Cargo.lock" "$BENCH/bench/" # the workspace's dependency versions
cd "$BENCH/bench"
CARGO_TARGET_DIR="$BENCH/target" cargo build --release
taskset -c 2 "$BENCH/target/release/document-editor-bench"   # a performance core here
rm -rf "$BENCH/target"
```

Notes:
- `head/` and `bench/` are siblings, because a path dependency inside the
  bench's directory would become a member of the bench's workspace.
- On a hybrid CPU, pin the program to a core of known type.
- The program prints the latency rows, the heap figures and the limit cases
  above, and asserts that every timed edit landed.
- Its "retained after series" line is the live-heap increment over the
  settled heap before the series.
