# Task: tests for an existing application

`/work/bookmarks` holds a small application built with `pert-ui`. Write
tests for it there. Do not change the application's behavior; small changes
that make it testable are allowed and must be listed in your friction log
with the reason.

## Behaviors to test

The application's README lists them. Each test drives the application
through the library's own facilities (simulated input, inspection), not by
calling the application's internal functions.

## Constraints

- `pert-ui` with the `cpu` feature in addition to the defaults.
- Tests run with `cargo test` without a display.

## Acceptance

1. One test per listed behavior, all passing.
2. One test that renders the main window with the CPU renderer and writes a
   PNG capture.
3. A test that fails when a behavior is broken: show it by temporarily
   breaking one behavior and recording the failure in your friction log.
