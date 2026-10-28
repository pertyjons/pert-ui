# Frozen-head author probes

Prepared from the initial independent triage. **Compiled and executed against kit-r3-head: all eight tests passed, 2026-10-28.**
These are verification inputs, not advertised application examples. They
import only the public facade and test the packaged 0.3.88 head, not later
repairs. The negative item-bounds assertions intentionally describe the
observed missing capability.

After the timed blind queues finish, initialize a fresh outside-checkout
acceptance directory with `run.sh exec`, copy Cargo.toml.txt as Cargo.toml
and lib.rs as src/lib.rs into its work directory, and run `cargo test --
--nocapture` through the same harness with kit-r3-head. Preserve the exact
commands, output, any verifier-source corrections and final source here.
Do not change a library or submission to make a probe pass.

P1 compares tab addressing before/after settlement; P2 exercises a Dynamic
write and reveal_child inside the same deferred operation with retained
editor identity; P3 establishes missing menu/tab/context-menu item bounds;
P4 checks Dialog and AlertDialog address segments; P5 checks MenuBar title
addressability. P6 establishes the Select owner/anchor semantics and missing address route
reported by the head settings-editor tester. Passing a probe is evidence only for what its assertions cover.

Execution: `plans/usability/run.sh exec <kit-r3-head> <author-probes-r3-head>
cargo test -- --nocapture`, fresh work directory, offline vendored
pert-ui 0.3.88. [Output](output.txt) preserves compilation and per-probe results.
There were no verifier-source corrections after the first execution.

P1's tab lookup/enumeration agree before and after settlement; P2's Dynamic
write followed by reveal_child inside a deferred operation returns Changed
and retains the old editor identity. The alleged timing limitation and
missing-address-children bug are not established. P3 confirms missing item
bounds on tabs, menus and context menus; P4 confirms Dialog/AlertDialog's
dialog segment; P5 confirms structural MenuBar titles; P6 confirms the Select
owner's NoSemantics and unavailable anchor address. The probes support only
these assertions, not independent application acceptance or native behavior.
