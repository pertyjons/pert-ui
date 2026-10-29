# Post-review command record

Commands executed from /home/per/github/pert-ui on 2026-10-28. Logs were
redirected externally and copied here after completion. These exact command
strings are recorded retrospectively by the author; the logs do not themselves
contain a command header. All final commands exited zero. Earlier failed
attempts are preserved separately and are not substituted for final results.

| Final output | Command |
| --- | --- |
| r3-review-final-clippy.log | cargo clippy --locked -p pert-ui -p pert-ui-gallery --all-targets -- -D warnings |
| r3-review-facade-doctests.log | cargo test --locked -p pert-ui --doc |
| r3-review-runtime-doctests.log | cargo test --locked -p pert-ui-runtime --doc |
| r3-review-routing-doctests.log | cargo test --locked -p pert-ui-runtime --doc (after the register_command cross-link) |
| r3-review-rustdoc.log | RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps |
| r3-review-routing-rustdoc.log | RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps (after the routing cross-link) |
| r3-review-parts-recheck2.log | cargo test --locked -p pert-ui --test foundation exposed_parts |
| r3-review-pointer-recheck3.log | cargo test --locked -p pert-ui --test surfaces pointer_items |
| r3-native-pointer-checks-review2.log | cargo run --locked -p pert-ui-native-checks -- interaction_check |

The negative-control commands and exit statuses are machine-recorded in
negative-controls/outcomes.json. They intentionally fail and restore source.
The early narrow-strip extent check is intentionally failing author evidence;
the final pointer log above covers the corrected allocated-width fixture.
