# Architecture and direction

This document records the project's scope, package boundaries and design decisions.
[Contracts](contracts.md) defines state and interaction rules,
[rendering and platform](rendering-platform.md) defines display behavior,
[verification](verification.md) records evidence, and the [manual](manual.md)
shows how to build applications. The public Rust API and its examples are
authoritative for exact constructors and errors.

## Product scope

pert-ui is an opinionated desktop GUI library with general widgets. Applications
provide content, stable identities, bindings and actions. The library provides
layout, standard interactions, visual defaults, invalidation, scheduling and
rendering. Application meaning stays in the host: the library does not own an
application data model, background processor or undo history. Unit-specific
formatting can be composed over general parameter APIs; base controls do not
encode application units.

The default facade starts a native application with bundled typography and a
coherent dark or light theme. Normal applications need no renderer selection,
repaint timer, executor, theme file or cache key. A custom host can use the
runtime without native startup through the `host` module, whose
`HostIntegration` trait holds the methods only a host calls. Accessibility adapters and plugin hosting are
outside the current delivery. Native support has been exercised on one Linux
desktop configuration; the rest of the [platform matrix](verification.md#native-evidence)
still needs execution evidence.

## Package boundaries

| Package | Responsibility |
|---|---|
| `pert-ui-core` | Validated values and identities, parameter specifications, edit transitions and bounded live-data primitives. |
| `pert-ui-runtime` | UI-local state, retained components, layout, input, scheduling, text, inspection and an owned backend-independent `DisplayList`. It has no window or GPU dependency. |
| `pert-ui-wgpu` | GPU interpretation of the public display list and an explicit backend interop boundary. |
| `pert-ui-cpu` | CPU interpretation of the public display list into `FrameImage` pixels, for rendering and serving captures without a window or GPU. The facade's optional `cpu` feature exposes it and, with `App::software_rendering`, presents its frames in native windows through softbuffer; the `desktop-cpu` feature builds the native application that way without wgpu. |
| `pert-ui` | Public facade, native startup, platform event translation and optional image loaders. |

Dependencies point toward the core. Renderer and platform types stay behind
their boundaries; widgets draw through `Painter`. Public facade APIs use
pert-ui-owned types except the renderer crate's explicit interop boundary.
The gallery consumes the facade and includes a custom widget to exercise public
extension points.

![The pert-ui packages: applications depend on the pert-ui facade, which depends
on pert-ui-runtime and, through optional features, on the pert-ui-wgpu and
pert-ui-cpu renderers; the renderers consume the runtime's DisplayList, and the
runtime depends on pert-ui-core.](diagrams/packages.svg)

The diagrams in this document are hand-written SVG in [diagrams/](diagrams/).
Update them in the same change as the boundaries or flows they show.

## Retained component model

An `Element` describes a component with a sibling-scoped stable key. Mounting
creates a retained instance; a compatible keyed update reconciles its properties
without discarding local state. Changing structure is explicit work, not a
per-frame rebuild. Runtime owns identity, input and subscriptions. Unmounting
releases subscriptions and held actions. Hidden and fully clipped views suspend
presentation work; reveal refreshes stale dependencies.

Signals record reads during component work. A changed binding invalidates only
its readers. `Invalidation::Paint` re-records drawing without remeasuring;
`Invalidation::Layout` reruns measure and arrangement. The runtime retains text,
layout and drawing data that did not change. Animation and live sources request
shared deadlines only while visible and active. A settled application waits for
an input, state or operating-system event.

Components use a two-phase layout contract: `measure` reports a size within
constraints, and `arrange` places children. The renderer receives an ordered
display list with explicit clipping, not component callbacks. A custom widget
uses the same public lifecycle, `Painter` and event contexts as built-in widgets.

![One UI thread turns an event into a frame: the App translates the event, a
component writes state that invalidates its readers, a flush updates, measures
and arranges, paint records a DisplayList, the chosen presenter draws it, and the
event loop waits for the next event or deadline.](diagrams/frame-pipeline.svg)

## Ownership and failure rules

The UI runtime is owned by its UI thread. Worker results enter through bounded
or explicit UI delivery contracts. Local state, authoritative edits and lossy
telemetry are separate paths. A telemetry gap is observable; a committed edit or
terminal gesture outcome must not silently disappear. Callback-safe producer
endpoints have narrower guarantees than ordinary senders. Consult
[contracts](contracts.md#data-and-edit-ownership) before connecting a producer.

![Data paths and threads: a real-time producer publishes into a lossy series
drained on the UI thread, workers send owned values to a Mailbox, tooling queries
an InspectionService, and on the UI thread components read Signals and edit
Parameters whose application-owned host accepts or rejects the
edits.](diagrams/data-paths.svg)

Invalid external numbers and identities are rejected. Control-limit clamping is
used only where documented. Persisted workspace state has its own schema version
and remains separate from application project data. Failures are typed and
propagated; a fallback has defined behavior. Public examples compile without a
sibling checkout or application-specific dependency.

## Direction and limits

The implemented foundation includes native startup, standard controls,
virtualized collections, typed parameters, workspace capture and restore,
inspection, images, vector drawing and experimental retained native windows.
Further work is driven by measured responsibilities: stable visual identities,
more incremental renderer updates, selective parallel preparation and broader
native platform evidence. These are proposals until implemented and verified.
The [verification record](verification.md) separates current evidence from
outstanding checks; [TODO.md](../TODO.md) holds verified open deviations.

Before publication, package and isolated-consumer checks must show that the
facade, assets and documented examples work outside the workspace. An actual
registry publication requires a separate instruction.
