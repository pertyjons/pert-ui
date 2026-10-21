## Your assignment: group G5, discovery (rubric items D1, D2)

Inputs beyond the common ones:
- the round 1 testers' code and friction logs:
  plans/usability/runs/r1-*/submission/ (their src/ and tests/) and
  plans/usability/runs/r1-*/FRICTION.md, plus the triage reports in
  plans/usability/runs/triage/;
- the probe applications: crates/pert-ui/examples/ (live_mixer,
  dynamic_documents, diagram_editor, log_explorer, inventory,
  getting_started) and apps/gallery;
- the prelude's membership rule in its rustdoc
  (crates/pert-ui-runtime/src/lib.rs, near the `pub mod prelude`
  declaration) and the facade's prelude (crates/pert-ui/src/lib.rs).
For D2, list every name the probes and round 1 imported from a crate root
or module path rather than the prelude, check each against the rule with
its feature conditions, and check the prelude's members against the rule in
the other direction as well.
