# Task: settings editor

Build a settings editor for a fictional photo-export tool.

## Features

- Three groups of fields, each visibly titled:
  - **Output:** a file name prefix (text, 1 to 32 characters, letters,
    digits, `-` and `_` only) and a format choice (`PNG`, `JPEG`, `WebP`).
  - **Quality:** a JPEG quality number from 1 to 100 (integer) and a
    maximum width in pixels from 16 to 16384, shown with its unit.
  - **Behavior:** a toggle "Open folder after export".
- Invalid input is refused with a visible explanation next to the field,
  and the last valid value stays in effect.
- **Apply** and **Revert** buttons. They are enabled only when the edited
  values differ from the applied ones (dirty state), and a visible marker
  shows unsaved changes.
- Apply saves the settings to a JSON file through background work (not on
  the UI thread). A failed save is reported visibly and leaves the
  settings dirty; a successful one clears the dirty state.
- On startup the settings are loaded from that file if it exists.
- Closing the window with unsaved changes is refused and asks the user to
  apply or revert first.

## Constraints

- `pert-ui` with its default features, plus `cpu` if you want captures.
- Verification without a display: tests or a headless mode that drive the
  application through the library's own facilities, not by calling your
  own model functions directly.

## Acceptance

1. All fields, groups and buttons exist and show the values loaded from the
   file.
2. A headless check shows an invalid prefix and an out-of-range quality
   refused, with the explanation shown and the old value kept.
3. A headless check shows Apply and Revert disabled when clean and enabled
   when dirty, and Revert restoring the applied values.
4. A headless check shows a successful save writing the file, and a failed
   save (for example to an unwritable path) reported and leaving the state
   dirty.
5. A headless check shows closing refused while dirty.
