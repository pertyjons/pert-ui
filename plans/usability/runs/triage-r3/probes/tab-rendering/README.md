# Literal tab rendering probe

An author-only temporary unit probe in TextService's private test scope, run
with `cargo test --locked -p pert-ui-runtime --lib r3_literal_tab_probe -- --nocapture`
on 0.3.89's working tree on 2026-10-28. The temporary test was removed after
execution; production text code is unchanged. Source and complete output are
retained here. This is a follow-up to independent task-6 acceptance's capture,
not blind-test evidence and not a finished repair.

Confirmed: `A\tB` shapes the literal U+0009 cluster to glyph 0 (advance
8.400001 at the default Body font), rather than whitespace. A standalone tab
also produces a raster part. `A B` uses the space glyph 3 and advance 3.64.
The existing raster preparation draws every glyph, including glyph 0. The
acceptance CPU capture shows that missing-glyph box. Canonical text remains a
literal tab. The presentation contract and a regression test must be settled
before choosing a repair; no tab-stop policy is claimed here.
