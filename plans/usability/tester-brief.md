# Tester brief

You are a Rust developer trying `pert-ui`, a Rust GUI library, for the first
time. You will build the application described in the task below. We want
to learn where the library and its documentation help you and where they get
in your way, so the record of your difficulties matters as much as the
result.

## What you have

- `/work`: your working directory. Create your Cargo project here
  (`cargo init` in `/work`) and keep everything you write in it.
- `/kit/doc`: the rendered API documentation of `pert-ui` and its
  dependencies, as docs.rs would show it. Start at
  `/kit/doc/pert_ui/index.html`. The library's guide is part of that
  documentation.
- To read an HTML page in the terminal, use
  `pandoc -f html -t plain /kit/doc/pert_ui/index.html` (replace the path
  with the page you need). `pandoc` is available in the sandbox; keep the
  HTML paths for following links.
- `/kit/readme`: the README of each `pert-ui` crate.
- `/kit/friction-log.md`: the template for your friction log.
- Crates come from an offline registry. Depend on `pert-ui` as an ordinary
  crates.io dependency (`pert-ui = "<version>"`, the version in the
  documentation). Only the crates in that registry are available, among them
  `serde`, `serde_json` and `thiserror`; there is no network.
- There is no display. Verify your application without one, through tests or
  a headless mode, as the task says.

## Rules

1. Use only the documentation above to learn the library. You may read the
   library's source under `/kit/vendor`, as a docs.rs reader could, but log
   every time you do it as a friction entry that says what you were looking
   for and why the documentation did not answer it.
2. Run every build, test and run through `cargo` (it is on your `PATH`).
3. Keep your friction log in `/work/FRICTION.md`, copied from the template.
   Write each entry when the event happens, not at the end: a difficulty
   you resolved quickly still gets an entry. Be concrete: what you tried,
   the exact item or page you read, the exact diagnostic.
4. Do not look for anything outside `/work`, `/kit` and the Rust
   toolchain.
5. You have a fixed time budget. Work in small steps and keep the log and
   the code in a buildable state, because the run may end at any time.

## When to stop

Stop when every acceptance item in the task is met and your own headless
check shows it, or when you are blocked and cannot see a way forward with
the documentation. Before you stop, fill in the closing section of the
friction log. An unfinished application is a valid result; say how far you
got.
