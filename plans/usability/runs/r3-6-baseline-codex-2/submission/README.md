# Notes editor

A plain-text folder editor built with pert-ui 0.3.75 (default features plus `cpu`).

```sh
cargo run --offline -- /path/to/notes
```

Without an argument, it creates and opens `./notes`. Only regular `.txt` files
are listed. Click a note, or focus the sidebar and use arrows and Enter.
Drag the divider to resize the sidebar. File commands are available in the
menu and through Ctrl+N, Ctrl+S and Ctrl+W.

```sh
cargo test --offline
```

The headless acceptance test mounts the same UI as the desktop application,
uses pert-ui's Driver for pointer, keyboard, menu and editor input, and reads
UI inspection plus files on disk. It exercises all acceptance scenarios,
including a missing-folder save failure and failed Save on close. It writes
`notes-editor.png` with the CPU renderer: two notes open, one modified.

`FRICTION.md` records documentation and API difficulties. No library source
was read. File operations are synchronous, suitable for small local notes.
