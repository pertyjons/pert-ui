# Implementation notes

This document is for maintainers. It records the implementation rationale and
internal ordering behind some rules of the
[state, composition and interaction contracts](contracts.md). Every guarantee
an application, a custom component or a custom host relies on is stated in
contracts.md; what is here explains how the runtime keeps those guarantees
and may change with the implementation. When a detail here becomes something
others rely on, state it in contracts.md instead.

## Contents

- [Surface admission and command queries](#surface-admission-and-command-queries)
- [Notification delivery](#notification-delivery)
- [Failure provenance](#failure-provenance)

## Surface admission and command queries

These notes belong to [menus, pickers and
dialogs](contracts.md#menus-pickers-and-dialogs).

Error recovery records the current kind and retries incomplete admission; no-flush
command queries read the current kind rather than a cached admission kind. A
command listing shares one pass over all mounted nodes across all rows. Command
invocation and native key routing, after flushing all presentation roots before
admission, inspect the synchronized surface frontier without repeating the full
declaration walk; command queries add newly shown roots missing from the last
synchronized frontier.

## Notification delivery

These notes belong to [delivery order and hidden
windows](contracts.md#delivery-order-and-hidden-windows).

Preparation follows the mounted incarnation's current owner and repeats visible
presentation settlement after callback writes, without replaying the mailbox,
task or timer delivery frontiers.

ToolWindows associates each reported change with its own delivery slot,
separate from housekeeping, so interleaved layout changes retain request order.
Detached-panel housekeeping requests coalesce while one wakeup is pending,
including hidden roots.

## Failure provenance

These notes belong to [failure cleanup in
components](contracts.md#failure-cleanup-in-components).

Pending notification requests hold weak input provenance, which is why a request
cannot retarget a later event. Nested input calls share
request lineage, and each call classifies errors against its own part of it.
Queued reveals carry their request's weak provenance while they invoke container
reveal hooks.
