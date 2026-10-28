# Notes editor

A plain-text notes editor using pert-ui 0.3 with default features and `cpu`.

```sh
cargo run --offline -- /path/to/notes
```

With no argument it creates and opens `./notes`. The sidebar lists regular
`.txt` files alphabetically. Click a note, or focus the list and use arrows
and Enter. Drag the sidebar divider to resize. Tabs retain their own editor,
selection and undo state. A `*` marks changed text.

File → New note, Save and Close tab share their commands with Ctrl+N,
Ctrl+S and Ctrl+W (the primary modifier on macOS is Command). New note
validates names and exclusively creates files, avoiding overwriting an
existing file. Close offers Save, Discard and Cancel for modified text.
Save errors produce a notification and retain the edit. The status line
counts LF line breaks plus one and whitespace-delimited words.

Headless verification:

```sh
cargo test --offline
```

The acceptance test mounts the same UI as the application, then uses
pert-ui's Driver for pointer clicks, keyboard navigation, typing, menu
selection and shortcuts. It reads widget semantics and filesystem results;
it does not call editor model actions. It covers all five acceptance items,
including failed Save and failed Save-and-close. It writes a 1000×700 CPU
capture to `notes-editor.png` with two tabs open and one modified.

The application is intended for small local notes: file I/O is synchronous,
the sidebar is scanned at startup and after creation, and it does not watch
external edits. CRLF/CR files are converted to LF on opening, and saves use
LF. Files unsupported by TextArea's content limits are refused with a
notification. Native execution could not be checked here because there is
no display.

See FRICTION.md for the development record.
