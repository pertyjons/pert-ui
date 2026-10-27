# Task: notes editor

Build a small editor for plain-text notes kept as files in one folder.

## Features

- The application opens a notes folder given on the command line (or a
  folder of your choice when none is given). Its `.txt` files are listed by
  name, sorted alphabetically, in a sidebar beside the editing area, with a
  divider the user can drag to resize them. Other files are not listed.
- Opening a note from the list (pointer, or the arrow keys and Enter) shows
  its text in a tab of its own; several notes can be open at once. Opening
  a note that is already open switches to its tab instead of opening a
  second one.
- Each tab edits its note in a multi-line text editor. A tab whose text
  differs from the file shows a modified marker in its title.
- A menu bar with **File → New note**, **File → Save** and **File → Close
  tab**, also available as the shortcuts Ctrl+N, Ctrl+S and Ctrl+W. Each
  shortcut does exactly what its menu item does.
- **New note** asks for a name in a dialog. A name is 1 to 64 characters,
  must not contain `/` or `\` or start with `.`, and gets `.txt` appended
  when it does not end with it. An invalid name, or the name of an existing
  note, is refused with a visible explanation in the dialog. A valid name
  creates the empty file, adds it to the list and opens it.
- **Save** writes the active tab's text to its file and clears its marker.
  A failed save is reported in a notification and leaves the tab modified.
- **Close tab** closes an unmodified tab at once. For a modified tab it asks
  in a dialog: **Save** (save, then close; a failed save keeps the tab
  open), **Discard** (close without writing) or **Cancel** (keep the tab and
  its text).
- A status line for the active tab: `<name>: L lines, W words`, where lines
  are the number of line breaks plus one and words are runs of
  non-whitespace characters. It follows typing, and is empty when no tab
  is open.

## Constraints

- `pert-ui` with the `cpu` feature in addition to the defaults.
- Verification without a display: tests or a headless mode that drive the
  application through the library's own facilities, not by calling your
  own model functions directly. A test may use a notes folder of its own.

## Acceptance

1. A headless check starts the application on a folder holding `.txt`
   notes and another file, sees only the notes listed in order, opens two
   of them and sees reopening one switch to its tab.
2. A headless check types into a tab and sees the modified marker and the
   status line's counts change, saves with Ctrl+S and with the menu item
   and finds the text in the file with the marker gone, and makes a save
   fail (for example by removing the notes folder) and sees the
   notification and the marker kept.
3. A headless check sees an empty name, a name with `/` and an existing
   name refused in the New note dialog with the explanation shown and no
   file created, and a valid name create, list and open the note.
4. A headless check closes a modified tab with each of Cancel, Discard and
   Save and sees the tab and the file afterwards, and closes an unmodified
   tab without a dialog.
5. A CPU capture (PNG) of the window with two notes open, one of them
   modified.
