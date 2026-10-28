# Notes editor

A small file-backed notes editor using pert-ui 0.3 with its default features and `cpu`.

Run on a desktop with:

```sh
cargo run --offline -- /path/to/notes
```

Without a folder argument it uses `./notes`, creating it when needed.
The sidebar lists regular `.txt` files in alphabetical order. Click a name
or focus the list and use arrows and Enter. Drag the divider to resize.
Use the File menu or Ctrl+N, Ctrl+S and Ctrl+W to create, save and close.
An asterisk marks text that differs from the last loaded/saved contents.

Run the complete acceptance check without a display:

```sh
cargo test --offline
```

The integration test starts the same application with `App::headless`,
drives it through pert-ui's `Driver` (pointer, text and keyboard events),
reads widget semantics, checks files, and writes `notes.png` using the CPU renderer.
Its temporary notes are in `target/headless-notes`. No model functions are called
by the check. `FRICTION.md` records documentation and API difficulties.

File operations are synchronous, appropriate for small notes. External file
changes are not monitored; save replaces the file with the tab's text.
Save errors appear as a persistent notification at the bottom of the window.
