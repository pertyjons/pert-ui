# MultiSelect long-row click author probe

2026-10-29, working 0.3.91 at d479516; later root changes were docs/tests only.
Public facade input/inspection, no native or blind-run claim. The local-path
manifest and lockfile record the exact author consumer, not a supported portable
example or a dependency of the library. No kit/submission was changed.

run4 is final. Six options keep search focus; twenty move focus to list after
clicking O02. The pointer-Leave controls isolate focus from the existing stale-
hover bug: short Down/Enter adds O03 and typing z changes query; long Enter
adds nothing and typing is ignored. Printed values/callback logs/focus/query
establish the bug; passing test status alone does not imply correct behavior.

Every attempt is retained. run1 mounted the chooser as a filling root leaving
no room for its popup; the fixture now places it in a Column. run2 used zero
time after Driver advanced the clock, yielding TimeReversed. run3 uses ui.now;
run4 adds Leave controls. The final trigger is discovered by ComboBox semantics,
not a hardcoded implementation key. Source snapshots preserve the earlier
fixtures. Commands/timestamps/source hashes and statuses are in each log/json.
