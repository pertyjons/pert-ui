# pert-ui-core

Backend-independent building blocks for the pert-ui GUI framework. This initial,
unreleased package provides normalized values, parameter specifications for
four closed families (continuous `f32`, exact stepped `i64`, `bool` and stable
`ChoiceId` choices), their family-tagged erased forms, the `HostValue` adapter
trait, host-scoped parameter identities and one synchronous local edit state
machine for every family with bounded terminal receipt storage, plus bounded, lossy,
freshness-aware live-data bridges: a single-producer ring of ordered items whose
endpoint publishes from a producer thread with bounded work and no allocation,
and whose consumer drains into a bounded retained window, marks where continuity
broke and counts exactly what never arrived. The ring protocol and the wake flag
are model-checked with `loom`. Reducing a window, such as holding a peak between
two frames, is the consuming widget's own policy.
It has no window, GPU or operating-system service dependency.
Linear mapping is the default for continuous values; logarithmic mapping is
available for positive domains. No application unit or meaning is assigned to values. `ParameterState`
separates base/preview/effective values and validates begin/update/commit/cancel,
revision conflicts, target removal and replay. It owns no background processor or undo
stack. Observable UI bindings, component lifecycles and numeric formatting/parsing
through `NumberFormat` belong to the runtime and public `pert-ui` facade.
Unit presets may later compose these general APIs; application meaning and
transport integrations remain the host's responsibility.

Applications normally import these types through `pert-ui`.

```rust
use pert_ui_core::{NormalizedValue, NormalizedValueError};

let position = NormalizedValue::try_from(0.25)?;
assert_eq!(position.get(), 0.25);
assert_eq!(NormalizedValue::try_from(1.5), Err(NormalizedValueError::OutOfRange));
# Ok::<(), NormalizedValueError>(())
```

Requires Rust 1.99.1 or later, edition 2024. Licensed under either MIT or
Apache-2.0, at your option; both license texts are included in this package.
