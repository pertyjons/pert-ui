# Notes editor

A plain-text notes editor using pert-ui 0.3.88 with its default features and `cpu`.

Run on a machine with a display:

```sh
cargo run --offline -- /path/to/notes
```

Without a folder argument, the application creates/opens `./notes`.
The sidebar shows regular `.txt` files in alphabetical order. Click a note,
or focus the sidebar and use arrows and Enter. Drag the divider to resize the
sidebar. Click tab titles or use Ctrl+Tab to switch documents.

File → New note, Save, and Close tab share their actions with Ctrl+N, Ctrl+S,
and Ctrl+W. New note validates names in a dialog. Closing a modified note offers
Save, Discard, and Cancel. Save errors appear above the status line.

Verify without a display:

```sh
cargo test --offline
```

The acceptance test starts the same application using `App::headless`, drives
widgets, menus, dialogs, text input, shortcuts, list navigation, and divider
pointer input through pert-ui, and checks semantics and disk contents. It uses
`/work/test-notes` and `/work/test-notes-moved` as disposable fixture directories.
It writes a CPU screenshot to `/work/notes-editor.png`.

See `FRICTION.md` for the library evaluation and documentation difficulties.
