# Component gallery

Run `cargo run -p pert-ui-gallery` from the repository root. The app depends only
on the public `pert-ui` facade and supplies content, actions and bindings. Visual
spacing, fonts, focus navigation, scrolling and scheduling use library defaults.
Example stages have explicit sizes so several independent layouts can share a page.
The outer `RegionLayout::fixed()`, below the menu bar, pins the tab strips and
the status bar. The top strip holds the categories, and each category has its
own strip of pages below it; switching
category keeps the page last shown there. Each page retains its own scroll
position. Fixed views intentionally clip content below hard size minima;
they do not move the frame to reveal controls outside it.

| Category › page | What to try |
|---|---|
| Layout › Layout | Compare Row, Column and Wrap; resize to reflow buttons. Grid demonstrates content, fixed and weighted tracks. Container demonstrates padding, fill and min/max limits; groups have titled and untitled examples. |
| Layout › Split panes | Three nested splits have two independent resize permissions and a permanently locked inner divider. Try equal shares, ratios, fixed first/second extents, both content-sizing rules and child min/max bounds. Change content width before/after resizing. |
| Layout › Nested regions | Resize or lock each of the top, bottom, left and right regions, or lock them all with the layout-wide switch: an edge resizes only while both its own and the layout-wide permission allow it. The center contains another RegionLayout with a retained counter. A lone-region example has no empty-slot divider. Shrink the window to exercise overflow access. |
| Layout › Scrolling | Compare a fixed View (clipped at local zero) with vertical (default), horizontal and two-axis ScrollArea. Tab to offscreen actions to reveal them, including the bottom-right cell of the large grid. |
| Layout › Sections | Click a section header, or focus it and press Enter or Space, to show or hide its content. Type in the note, collapse and expand it again: the draft survives. The bound section follows the page's switch and is left out of saved workspaces. In the accordions, Up/Down/Home/End move between headers; the first keeps one section open, the second opens them independently. |
| Layout › Tool windows | Compare the full-width open and locked layouts, which share a selected sample file: select it in Files or Search and see the main views and Properties update. In the open layout, use a stripe button (or Tab, arrows and Enter) to expand a panel; Escape returns focus to the main view. Drag a divider to resize, move a stripe button to another edge or position, or use a panel's ⋮ menu to move, overlay, float or open it in its own native window; closing that window returns it to the stripe. Notes starts as an overlay and hides when focus leaves it. The locked layout prevents user toggling, resizing, moving, floating and detaching, while the button below can still open Properties through a controller. The switch shows names on the stripes. Below, a tool window holds a nested layout with its own left stripe. |
| Layout › Transforms | On the `ZoomView` canvas the wheel pans and zooms at the pointer with Control (Command on macOS), dragging the background or with the middle button pans, and once it is focused arrows pan, = and - zoom, 0 shows 100% and Home fits everything; Fit and Reset do the same from application code through a `ZoomController`. The cards are ordinary controls that keep working at every zoom, with text prepared for the size it is shown at; during a continuous zoom it is stretched and sharpens once the zoom rests. A pinch zooms too where the platform delivers one (macOS). Below it, a timeline zooms time with Control and the wheel or a pinch, zooms amplitude with Shift added to either, pans by dragging and shows the whole hour on Home, while rows, labels and the trace's stroke keep their size. Compare quarter-turned labels, which exchange width and height, and one path mapped by rotations and by a per-axis data fit that keeps its stroke width. |
| Layout › State & tabs | Increment a counter, hide/show its View and switch tabs without losing state. Start the scheduled sample; hidden content suspends updates. Detach a single ordinary View with its button or Primary+D. Nested tabs have independent counters and a scrollable title strip. |
| Layout › Navigation | Links reveal a button at the end of a hidden tab, with and without focus (hover a link for its underline and pointing-hand cursor), and one in a closed accordion section; see a view the application hides refuse until its toggle is on; select a tab by key through a command; jump to Data & media › Collections through an always-underlined link, which opens two strips at once, beside a disabled link. A line above the strip reports each selection change with its origin: a click or key is the user's, a link is a reveal. Below, open the history strip's places one after another, then use the back and forward arrows (hover them for the place they lead to) or the clock icon's searchable list to return to one; returning reveals the tab without recording a new visit. A drawer the gallery writes itself opens when a link reveals the setting inside it, through the same public hook the built-in containers use. A nonmodal palette opens beside its button with the gallery's own matcher: every typed word must begin a word of a command's name. The jump links include one with rich content, an icon and a styled span. |
| Controls › Text & buttons | Compare Body, Heading, Caption and Monospace. Noto Sans Mono appears in real Grid cells. Change bound text, increment/reset a counter, try a disabled button, and cancel an armed click. A custom Painter sample shows rectangles, clipping and coverage masks. |
| Controls › Controls | Checkbox and Toggle share a boolean; RadioGroup and Select share a typed choice. Try menus, delayed help and a popover that escapes the page clip. Escape/outside press cancel; focus returns to the opener. |
| Controls › Input & parameters | Focus the key area and hold multiple keys or modifiers. Horizontal and vertical sliders, three knobs (one filling its arc from the default, one with an application-formatted readout) and two button-based editors share one numeric parameter. Try a cancelled preview, an effective-value override, read-only rejection and an authoritative host reset; outcomes appear in the footer. |
| Controls › Parameter families | Continuous, stepped, boolean and choice parameters created from descriptors, each bound to its standard and inline controls. Every committed edit reaches an application undo stack through the host's terminal notification; Undo writes the recorded value back. |
| Controls › Toolbar | A global toolbar with icon actions, latched toggles, a stepped tempo, a snap select, an icon choice, an editable name and read-only readouts; an inline numeric row with slider, integer, decimal, custom millisecond formatting and Reset; a channel strip with inline pan/send sliders, mute/solo and a decibel level; and a status bar attached below. Compare the small muted captions with full-size values, units and action text, including while editing a number. Narrow the window to move groups behind the More button. |
| Controls › Property grid | Two accordion sections hold one property grid each, with text, checkbox, select, stepped number, knob, color and toggle editors. Drag the gap between labels and editors, or Tab to a grid and press Left or Right (Shift for larger steps): both grids share one label width and stay aligned; Escape cancels a resize and Reset label width restores it. The long last label wraps in its column. Clear the name and press Check: the error appears under its editor, which receives focus. |
| Controls › Adaptive forms | Resize the split panel to reflow field columns while retaining text and numeric drafts; small allocations scroll. |
| Controls › Field status | Leave the name empty and press Check: the application's rule publishes an error that outlines the control, replaces the help line and receives focus, while long notes produce a warning. Clear status restores the help text. |
| Controls › Progress | Press Start for activity with no known extent, the only status that animates; Step reports a known fraction, which schedules nothing; Succeed, Fail and Cancel end the task. Switch tabs while it runs to see the animation stop and resume. A spinner beside the caption follows the task, and a second one has its own Busy switch. Below, every status that does not animate. |
| Controls › Colors | Swatches show opaque and translucent colors, the latter over a checkerboard. Type hexadecimal sRGB (`#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa`) and press Enter or move focus away: valid text is saved in canonical lowercase form and recolors the swatch beside it, whose sRGB bytes and linear channels appear below; invalid text is refused with the parser's reason. Tab to the swatch and press Enter or Space, or click it, to activate it as a picker would open. Below, a color picker edits the same color: drag the area or a strip and the swatch changes on release, Escape cancels a drag, arrow keys step a focused part (finer with Shift), and dragging to black and back keeps the hue. The next section opens a second picker in a popover from its swatch. The last one opens a nonmodal quick pick over named colors: type to filter, Up/Down and Enter or a click choose one and recolor the swatch, each row shows its hexadecimal value and whether it is warm, cool or neutral, and the disabled Ultraviolet row is refused with a message while the picker stays open. |
| Controls › Chips & breadcrumbs | Static chips beside removable tags. Focus a tag and press Delete or Backspace, or click its × button: the page removes the tag and focus moves to the next one, then the previous one, then back to the page. Reset tags restores them. Two multi-select choosers pick instruments: open one with Enter, Down or a click, type to filter, move the highlight with Up and Down while focus stays in the search field, and toggle with Enter or a click; Escape clears the query, then closes. The first writes every toggle, the second only when its popup closes by a click outside or Tab, and Escape discards its draft. Two breadcrumbs follow one sample folder: activate a folder to navigate up, and compare the narrow one, which collapses its middle folders behind a … menu; Open mixes returns to the deepest folder. |
| Text › Text input | Single-line body text, a taller single-line field beside a same-height button for alignment, multiline monospace, read-only mirror, disabled field, proportional override and external replacement. Double-click a word; triple-click selects all text. Right-click or press Shift+F10 in any field for Copy, Cut, Paste and the spaced Paste Before/Paste After; inapplicable actions are grayed out. Below the editors, compare `Field` around a text editor in each `FieldStatus`: normal with help text, warning, error, an error with an empty message, a long wrapping message on a multiline editor, a read-only control and a field without a label. The first field is live: press Enter to apply the gallery's own rule. A search field filters a list once typing pauses, highlighting what matched: Up/Down move the highlight while focus stays in the field, Enter chooses and Escape clears the query. Below it, a sign-in form shows the input states: placeholders, required markers, length limits with counters, a concealed password with an eye button (or Alt+F8) and a name check that shows a pending spinner until you finish it. A × button clears a field, a dot in the corner marks an unsaved draft and the summary counts unsaved fields; the delay field shows an icon and a unit around its text. |
| Text › Number input | Edit integer/decimal drafts, save with Enter or focus loss, cancel with Escape, and compare fixed precision with saved-value mirrors. Try external replacement, read-only and disabled fields. A field with a `NumberFormat` shows a fraction as a percentage at rest and edits the plain percentage while focused. |
| Text › Fonts & icons | Compare labels cut to one line in boxes of three widths, with an ellipsis at the end or in the middle of a path; rest the pointer on a cut label for its full text. Combine fonts and colors, try icon buttons with tooltips, change the default text font and select expand icon, restore defaults, and inspect a composed panel header/footer. |
| Text › Text rendering | Switch between the default and phase-aware text masks, edit the field, and scroll the sample. Compare appearance at different zoom levels and themes. The same setting is in Settings › Text rendering. |
| Surfaces › Dialogs & notifications | Open a form and its nested confirmation, compare window/view modality, try safe Cancel focus, right-click or Shift+F10, navigate nested menus, dismiss/restore the banner, and post timed/actionable toasts. Hover/focus pauses the visible toast stack; hidden started toasts expire on return. |
| Surfaces › Floating windows | Move and overlap decorated windows, resize from the bottom-right grip, minimize/restore, use a fixed undecorated palette, and exercise nested tabs, splits and a dialog. Open the local dialog from Properties › General: its panel is centered within the General view and its Close dialog button receives focus. |
| Surfaces › Menu bar | Press File or View, then point at the other title to switch menus. Focus the document and press F10: Left/Right move between titles, Down opens, Left/Right inside a menu move to the neighboring one, and Escape closes one level. File › New sketch (Primary+N) enables Save (Primary+S) through the command's own binding; both shortcuts also work with the menus closed. View shows a check-mark toggle and radio rows, and About is a title that runs an action. The synchronize button at the right of the bar is in its trailing slot: Tab reaches it, F10 and the title arrows skip it. |
| Data & media › Virtual list | Scroll a `VirtualList` of 100,000 rows that mounts only the rows near its viewport, Tab through mounted buttons, and jump to the first, middle or last row by key; the line below reports each jump's outcome, including a key no row has. |
| Data & media › Collections | Three consumers of one virtualized list and one keyed selection. In the grid, arrows move by item and by line, Shift extends a range, Control moves without selecting and Space toggles. In the tree, Right expands and then enters a folder and Left collapses and then leaves it; the hierarchy has over two million nodes that exist only once their parent opens. In the table, click Sample to sort 100,000 rows, drag a column edge to resize, drag the middle column titles or press Alt with Left or Right to reorder them, scroll the overflowing columns, and move the active cell with the arrows before pressing Enter. The last list has rows of their own wrapped height over an estimate. Type in the filter above the grid, the tree or the table to show only matching items: the grid keeps selected items selected while they are hidden and Down moves from the filter into it; the tree shows matches of its first three levels with the folders above them, and Escape in the filter clears it and brings back your own expansion. |
| Data & media › File browser | Browse a synthetic sample library that the page answers through a mailbox, as a host answers from a worker thread; the browser never reads the disk. Click a folder in the tree or expand it, double-click or press Enter on an entry, filter the entries by name and the folder tree from the field above it (it searches only folders already listed, so it never asks for a listing), press Backspace in a list for the parent folder and use the path bar above. Locked fails with an error row, Latest project is a link to a folder, Old link is broken and damaged.bin unreadable. Turn on slow listing to see loading rows and open another folder before one arrives: the late result is counted as stale and dropped. |
| Data & media › Drag & drop | Two reorderable lists exchange tracks, with a multiple selection dragged together and the copy modifier copying; custom chips carry their own payload to one bin that accepts it as a move, a copy or a link, as the modifiers ask, and one that refuses it. The reorderable tree takes a track before or after another, or into a folder, even an empty one, and forbids moving a folder into itself; Alt with an arrow does the same from the keyboard. |
| Data & media › Static & live | Start the synthetic waveform and level beside 1,000 independent checkboxes; consumer components draw them from the library's series sources. Scroll, toggle a checkbox, hide/show the visuals and switch tabs. Reset measurements before a run and inspect the Profiler afterward. |
| Data & media › Level & trace | The library's `Trace` and `LevelMeter` on the same producer thread's two series, each scaled by a `ParameterSpec`. Press Run: the trace sweeps over its grid, the horizontal meter holds its peak marker for a second and the vertical meter reads the same source. Stop it or leave the page and nothing is drained. |
| Data & media › Vector shapes | Paths drawn through the public `Painter` API: a self-overlapping star filled with the nonzero and even-odd rules, butt, square and round caps over a guide at their endpoints, miter, bevel and round joins, a filled and a stroked ellipse, a rounded rectangle and a sampled trace. Switch themes or zoom to see them re-rasterize sharply. |
| Data & media › Images | A translucent PNG, decoded once when the page is built, in the three fits: Contain shows all of it, Cover fills the box and clips, Fill stretches. Its soft edge blends without a dark fringe in both themes. An 8x8 image built in code is magnified with Nearest and Linear sampling. "Decode on a worker" decodes the PNG on a new thread and a mailbox delivers it to the image beside the button. SVG drawings show one monochrome mark at several sizes, each rasterized for the pixels it covers and tinted by a theme color, beside a drawing that keeps its own colors. The last line lists the decoders this build enables; of the raster formats the gallery enables only `png`, and it enables `svg`. |
| Appearance & tools › Themes | Switch dark/light palettes and inspect semantic color overrides. |
| Appearance & tools › Workspace | Drag the sample's divider, pick a tab and scroll the list, then Capture: the document covers the whole gallery's arrangement and settings, not only this page. Rearrange and Restore to put it back, or edit the document text first when it fits the editor. The page writes no file; a host stores the document where it keeps its settings. |
| Appearance & tools › Profiler | Exercise any page, then Refresh snapshot to see calls, total/self/max elapsed UI time. Pause/Enable/Clear are explicit; no polling is scheduled. |
| Appearance & tools › Debug overlay | Choose Settings › Debug overlay › Validation issues, then switch "Arrange the sample wrongly" on: the overlapping and the escaping button are outlined in red and reported once on stderr. Component boxes outlines every border box and padded content box instead. Both need a build with debug assertions and schedule no frames. |

For alignment checks, narrow **Sample** after sorting it on **Data & media ›
Collections**; its heading should stop before the sort arrow. Resize the window
and post an actionable notification on **Surfaces › Dialogs & notifications** to
see its buttons stack in a narrow viewport. On **Text › Text input**, compare
the tall field with the button beside it. On **Controls › Toolbar**, compare
inline captions, values and units before and during value editing. Hover a
movable tab in the nested strip on **Layout › State & tabs** to compare its title
and detach icon.

For hover feedback, sweep the pointer over sliders and inline values on
**Controls › Input & parameters** and **Controls › Toolbar**, fields on
**Text › Text input**, rows and header grips on **Data & media › Collections**,
tab overflow arrows on **Layout › State & tabs**, notification action and
dismiss buttons on **Surfaces › Dialogs & notifications**, and window title and
corner grips on **Surfaces › Floating windows**. The expander in the tree on
**Data & media › Collections** highlights independently of its row.

Settings › Motion › Reduced stills the Progress page's indeterminate bar and
spinner; they then schedule nothing.

Press Ctrl+Shift+P (Cmd+Shift+P on macOS), or choose Help › Show all commands,
for the command palette: type to filter the gallery's commands, move with
Up/Down and run one with Enter. Its "Go to" commands open pages across
categories; Escape clears the query and then closes the palette.

Press Ctrl+Shift+F (Cmd+Shift+F on macOS), or choose Help › Search the gallery,
to search the gallery's content and commands in one list, each row labelled
Content or Command: type a button's text, a label or a field's value from any
page, including pages and sections that are not shown, and press Enter to open
the page and focus it.

Use Tab/Shift+Tab to traverse controls and Enter/Space to activate buttons. Focused
tab strips use Left/Right/Home/End; Ctrl+Tab or Ctrl+Shift+Tab switches from content.
Tab-title overflow has arrow buttons and wheel scrolling. Every tab strip in the
gallery is detachable; see "Detaching tabs" below. Dividers support dragging
or arrow keys (Shift makes larger steps); release commits and Escape cancels.
Scroll viewports support wheels, arrows, Page Up/Down and Home/End as applicable.

The scheduled sample is stopped initially. Starting it requests one shared-runtime
wake every 250 ms while visible; a highlighted cell advances on presentation
updates, including visibility changes, rather than counting wall-clock intervals. Hiding
its View or switching the outer tab suspends work. Stop it to return to a settled
UI without scheduled sample wakeups. This is a lifecycle example, not a meter,
oscilloscope or performance benchmark.

`cargo run -p pert-ui-gallery -- --headless` runs the parameter, key and inspection
protocol example without opening a window and exits with status 1 when one of its
contracts does not hold; CI runs it that way. `--help` lists every mode and wins over any other argument; an
unknown or second mode argument is otherwise a usage error with status 2. `cargo test -p pert-ui-gallery` mounts
the same tree as the GUI and exercises all pages, nested state, lock controls,
scroll-to-focus, scheduler suspension and parameter actions through public APIs.
Readouts of a value are `Label`s bound to the value mapped to text. The receipt
readout, which follows a status held inside the latest receipt, the key area,
scheduled sample, Painter canvas and the Static & live visuals are local consumer
examples, not new library widgets.

The gallery covers implemented visual components; it does not make unimplemented
widgets available. The synthetic static/live workload and its evidence are described
in [the workload record](../../docs/verification.md). Its visuals are
consumer components over the library's series bridges and sources; the Level &
trace page shows the library's own `Trace` and `LevelMeter` on the same sources.
Host-only operations such as keyed tree reconciliation and owned inspection are
also covered by public library tests; the native gallery does not supply a tree
editor or a remote inspection service.

The manual CPU probe covers 120 split moves followed by clicking the fixed
**Controls** category header. It includes the demo's enabled profiling overhead:

```sh
cargo test -p pert-ui-gallery split_drag_timings -- --ignored --nocapture
cargo test -p pert-ui-gallery --release split_drag_timings -- --ignored --nocapture
```

It times direct runtime input, layout and display-list construction separately;
it excludes the native motion coalescer, GPU work and presentation. Timing values
are diagnostic, not test thresholds. A regular regression also nearly collapses
each content-sized pane, switches tabs by pointer and checks retained sizing and
parameter interaction.

The gallery's own `MenuBar` holds its settings and information, with the
pert-ui mark leading its title row: the mark follows the theme mode and, like
**Help › About**, opens the About dialog when clicked or activated with Enter or
Space (it is the bar's first Tab stop). At the right end of the bar, back and
forward arrows move through the pages visited, as in a browser, and the clock
icon lists them with a search field; Alt+Left and Alt+Right (Cmd+[ and Cmd+]
on macOS) and the mouse's side buttons do the same. Opening a page yourself,
or through a palette's Go to command, is a visit; returning to one is not, so
forward still leads on afterwards. **Settings ›
Theme** switches Dark/Light, **Settings › Density** switches
Compact/Standard/Comfortable, **Settings › Motion** chooses full or reduced
motion, **Settings › Text rendering** switches between the original and
phase-aware line mask, **Settings › Debug overlay** outlines validation issues or component
boxes, and **Settings › Library language** switches the library's own words
(menu items, placeholders, names, messages) between English and a small
Swedish table while the gallery's own texts stay English. Each current choice
carries a radio dot that follows its setting, including settings changed by a
Workspace restore. Text rendering is runtime-local and is not saved in the
workspace document. **Help › About** opens a dialog with the version, license
and repository, and **Help** also opens
dialogs on keyboard shortcuts and on using the gallery. F10 reaches the bar from
focused content. Zoom controls (−, +, reset to 100%, and the current percentage)
sit at the right end of the status bar; zoom ranges from 75 to 200%. The Themes
tab displays semantic accent roles, lets you override the active
primary/foreground pair, and resets the current palette. Switching mode preserves
overrides separately for each mode.

The Themes page also compares nested/untitled/frameless groups, a content Divider,
and fixed versus draggable boundaries. Page margins and section gaps follow the
active theme. Tests exercise every color/density combination at 75, 125 and
200% zoom, plus retained state and the actual menu and zoom actions.


## Detaching tabs

Every tab strip in the gallery uses `Tabs::detachable(true)`: the category strip,
whose tabs move a whole category of pages, each category's strip of pages, the
nested retained views on **State & tabs**, and the General/Editor strip inside the
Properties floating window. Hover a title: the pointer becomes a
move cursor and a dimmed "open in new window" marker appears in the slot every
title already reserves, so nothing shifts. Press and drag off the strip, more than
eight logical pixels if downward: a drag session starts and the title follows the
pointer.
Release anywhere nothing takes the drop, over other content or outside the
window, and the native host opens an independent decorated window with that
retained view. While the drag is outside the strip the source title already reads
as an emptied outlined slot, so you can see what the release will do. Back over
the strip the cursor shows that nothing can be dropped there and a release keeps
the tab; Escape, hiding or focus loss cancel.

A detached tab keeps its place in the strip as that emptied slot with a dimmed
title and an accented marker, so detached and docked tabs never look alike. The
detached view keeps its incarnation, local state, bindings and scheduling; no
content is duplicated and the strip selects a remaining tab. Its title matches the
tab title. Closing the detached window re-docks the view, the default close policy.
Pressing an already detached title activates its window instead. The OS places the
new window, and dragging it back onto the strip is not implemented. Detaching
inside a floating window moves the view to a native window, not to another
in-view window.

Nothing is detachable by accident. A strip offers the gesture only because the
gallery calls `Tabs::detachable(true)`, and one nested retained view on
**State & tabs** calls `Tab::detachable(false)`: its title keeps a marker-free slot
and never leaves the window, while its neighbors can.

Tabs are not the only detachable unit. **State & tabs** has a "Detach a single
view" section whose button, or Primary+D, moves an ordinary `View` into its own
window through the same public request; its counter keeps counting there, the
readout reports the window operation, and closing the window docks it again.
Asking a second time reports the existing window instead of duplicating content.
The protected `View` beside it sets `Element::with_detachable(false)`, so "Try the
protected view" makes the identical request and the footer reports the refusal.

`cargo run -p pert-ui-gallery -- --multi-window` runs the separate three-window
fixture with application commands, per-view commands and held input.

## Reordering tabs

The nested retained views on **State & tabs** are also `Tabs::reorderable`. Drag a
title along the strip: a Primary line marks where it would go, and resting over
an overflow arrow scrolls the titles. With the strip focused, Alt with Left or
Right moves the selected view one place. The gallery owns the order, edits it in
the callback and describes the strip again; each view keeps its key and its
count. Dragging a title off the strip still detaches it, except for the one that
forbids it.

## Floating windows

The Floating windows tab contains three overlapping windows: Out-1 controls, Properties
with nested tabs/split/editor/dialog, and a fixed undecorated tool palette with a
custom drag bar. Drag titles to move and use bottom-right grips to resize. The
Out-1 resize permission can change live. Minimize/restore preserves content state;
Restore minimized acts on surviving windows. Close removes that incarnation;
restart the gallery to recreate closed windows. Title buttons/menus remain ordinary
widgets. Focus a frame and use Alt+arrows to move or Alt+Shift+arrows to resize.


## Static/live acceptance workload

The Static & live tab starts paused. Its 1,000 standard checkboxes each own an
independent boolean binding; the scroll viewport keeps normal control hit sizes.
Only a subset fits on screen at once. A producer thread publishes a repeating
synthetic signal into two public series bridges, a waveform and a level, while
the Run toggle of this page or of Level & trace is on, and is parked otherwise,
so stopped pages wake nothing; the gallery joins the thread when it exits. The
trace and meter subscribe to the runtime's sources over them, and the runtime
drains at the 60 Hz presentation schedule only while a visible subscriber
exists. The meter holds its peak by folding the level items it has not folded
yet, which is the widget's own policy rather than a service of the bridge. Stop
unsubscribes, so nothing is drained; hiding the visuals or the tab does the same,
and returning discards what arrived unseen and continues from current items,
reporting the loss. The checkbox states are preserved throughout.

A retained background draws the scope grid separately. The trace copies the
retained window, at most 512 items, and drawing reduces it to at most 512
physical-width columns with min/max preservation, connecting neighboring columns
except across a reported break. No audio input is involved: the producer is a
synthetic clock on an ordinary thread.

```sh
cargo run --release -p pert-ui-gallery -- --live-benchmark
cargo run --release -p pert-ui-gallery -- --live-smoke
```

The first command measures 300 deterministic frames through the same public tree,
reports update, reconciliation, layout, measure/arrange, text, drawing and shared
composition separately, then checks 30 real seconds of headless quiescence.
The native smoke opens that tree, runs the visuals for five seconds and then
stops their deadlines; leave its controls untouched to measure native idle and
close the window afterward. The five-second limit applies only to this smoke
mode. The normal gallery remains interactive with explicit Run and Show controls.
See the workload record for native tracing instructions, results and limitations.

## Native workspace restart check

Run these as separate processes in a supported desktop session. The first opens
an actual gallery with changed theme, density, zoom, tab selection and split
preferences, waits for presentation, writes the document and closes. The second
mounts a fresh gallery, restores it, presents and compares a new capture with the
saved document. Each fails on incomplete restoration or component errors.
Choose a new path: save refuses to overwrite an existing file.

```sh
cargo run --locked -p pert-ui-gallery -- --workspace-check-save /tmp/pert-workspace.txt
cargo run --locked -p pert-ui-gallery -- --workspace-check-restore /tmp/pert-workspace.txt
```

Use a writable native path on Windows. Let the test close its own window and do
not interact during the check: input context loss can cancel its three-second
completion timer. An early manual close reports an incomplete check. This is a
scripted restore/presentation check, not a test of physical IME, monitor changes
or all manual input. Platform evidence lives in the workspace persistence note.

`--interaction-check` opens the gallery in a native window and drives named
targets on its pages one step at a time: the menu bar's brand mark and history
arrows, the command palette and content search shortcuts, a chip removal, the
search field, the concealed password field and its reveal, a knob drag, a
collapsible section, a tool window (expanded, floated through its options
menu, moved by its header, resized by its corner grip, opened in its own native
window and returned to its stripe by closing that window) and a collection
filter. Each step injects
its input, waits for a painted frame and one step interval, then checks the
page; the run fails on an error, a validation issue or anything still
scheduled at the end. Input enters at the runtime's native entry points, not as
window-system events. `cargo run --locked -p pert-ui-native-checks` runs it with
the other native checks; do not interact with the window while it runs.

```sh
cargo run --locked -p pert-ui-gallery -- --interaction-check
```

`--cpu` opens the native gallery with software rendering: every frame is drawn
by the CPU renderer and presented without a GPU device, and the window title
says so. Everything else behaves as in the GPU gallery, more slowly.

```sh
cargo run --locked --release -p pert-ui-gallery -- --cpu
```

`--screenshot [--scale N] [--palette violet] PAGE TARGET PATH` writes a PNG of
the gallery without a display or GPU. It mounts the gallery headlessly at
1280×800 logical pixels and scale `N` (default 1). It shows `PAGE` (a page key
such as `tool-windows`, or `-` for the first page) and asks for a capture through
the public capture contract, which the CPU renderer answers. `TARGET` is `window`,
`region:X,Y,WIDTH,HEIGHT` in logical pixels, `path:KEY/KEY/…` for an
addressable key path, or `key:KEY` for the one shown component with that key,
found through inspection. The optional Violet palette is the Themes page's
dark Violet preset; the default is the built-in dark palette. The printed line
names the captured region and whether it was clipped. An AI assistant or a
script can use it to look at a page or a single component:

```sh
cargo run --locked -p pert-ui-gallery -- --screenshot tool-windows window window.png
cargo run --locked -p pert-ui-gallery -- --screenshot --palette violet toolbar window toolbar-violet.png
cargo run --locked -p pert-ui-gallery -- --screenshot --scale 2 tool-windows key:open-stage stage.png
cargo run --locked -p pert-ui-gallery -- --screenshot - region:0,0,640,200 top.png
```
