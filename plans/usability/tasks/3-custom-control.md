# Task: custom point picker

The library has no two-dimensional point picker. Build one as your own
reusable control, and use it in a small application.

## Features

- **The control:** a square area showing a crosshair and a handle at the
  current point, with `x` and `y` each from 0.0 to 1.0.
  - Pointer: pressing inside moves the handle there and starts a drag; the
    drag continues outside the area (clamped to its edges) until release.
    Cancelling the drag (for example with Escape) restores the point it
    had before the press.
  - Keyboard: when focused, the arrow keys move the point by 0.01, and by
    0.1 with Shift.
  - It shows a focus indication that follows the active theme, and uses
    theme colors rather than fixed ones.
  - Its current value is visible to the library's inspection facilities,
    so a test or tool can read it without knowing your types.
  - It reports value changes to its owner, and its owner can set the value.
- **The application:** the picker beside two numeric fields for `x` and
  `y` that stay in sync with it in both directions, and a Reset button.

## Constraints

- `pert-ui` with the `cpu` feature in addition to the defaults.
- Verification without a display: tests or a headless mode that drive the
  application through the library's own facilities.

## Acceptance

1. A headless check drags the handle, including outside the area, and
   reads the clamped value.
2. A headless check cancels a drag and sees the old value restored.
3. A headless check moves the point with the arrow keys, with and without
   Shift.
4. Editing a numeric field moves the handle, and Reset restores (0.5, 0.5).
5. A CPU capture (PNG) of the application, in the dark and the light theme,
   showing the focused picker.
6. The picker's value appears in an inspection snapshot.
