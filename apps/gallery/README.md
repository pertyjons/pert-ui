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

Use **Help › Search the gallery** to look up a component by API name,
including `MultiSelect` and `FileDialog`. Choose a Page result to open its
example. Page introductions give an action and its expected result. Related
links connect composed examples to their controls. Expand the help in a
sample for details, using Tab and Enter or Space from the keyboard.

| Category › page | Launch key for `--page` | What to try |
|---|---|---|
| Layout & navigation › Rows, columns & grids | `layout` | Resize the window and compare how rows, wrapping items and grid tracks share the available space. |
| Layout & navigation › Split panes | `splits` | Drag a divider, lock it, then change the content width to compare the sizing rules. |
| Layout & navigation › Nested regions | `shells` | Resize an edge and watch the center adjust. Lock the edge and try the same drag again. |
| Layout & navigation › Scrolling | `scroll` | Tab to a control below the viewport and watch the scroll area reveal it. |
| Layout & navigation › Sections | `sections` | Type a note and collapse its section. Expand it again to find the same draft. |
| Layout & navigation › Transforms | `transforms` | Zoom the canvas at the pointer. The controls on the cards remain interactive. |
| Layout & navigation › State & tabs | `state` | Increment a counter, switch tabs and return. The view remembers its value. |
| Layout & navigation › Navigation | `navigation` | Follow a link to a hidden control, then use the history arrows to revisit a page. |
| Layout & navigation › Breadcrumbs | `breadcrumbs` | Open a parent folder in either breadcrumb. Both paths follow the same location. |
| Layout & navigation › Physical keys | `keyboard` | Focus the key area and hold two keys together. Moving focus away clears the held keys. |
| Controls & forms › Buttons | `buttons` | Activate a button by keyboard, then change the bound caption. The disabled example skips focus. |
| Controls & forms › Choices & toggles | `controls` | Change a checkbox or choose a radio option and watch the control beside it follow. |
| Controls & forms › Chips & multiple choice | `chips` | Remove a tag or compare when the two instrument choosers commit their selections. |
| Controls & forms › Text input | `editing` | Edit the text, then try the read-only mirror. A rejected write leaves the previous value visible. |
| Controls & forms › Number entry | `numbers` | Type a number and press Escape, then try Enter. Watch the saved value beside the field. |
| Controls & forms › Knobs & sliders | `input` | Drag Bakelite and compare the readouts. Double-click a knob to restore the shared default. |
| Controls & forms › Parameter families | `families` | Edit a value and use Undo. The host records committed changes across control types. |
| Controls & forms › Field status | `fields` | Leave the name empty and press Check. The editor receives focus and displays the reason. |
| Controls & forms › Adaptive forms | `forms` | Drag the divider while editing a field. The columns reflow and the draft survives. |
| Controls & forms › Sign-in form | `sign-in` | Enter a short user name and submit it. Revert clears the refused draft. |
| Controls & forms › Property grid | `properties` | Resize the label column in either section. Both grids stay aligned. |
| Controls & forms › Color controls | `colors` | Edit the hex value or drag in the picker. Both views show the saved color. |
| Controls & forms › Toolbar | `toolbar` | Narrow the toolbar to move groups into More. The actions and their values remain available. |
| Menus, windows & feedback › Menu bar | `menus` | Activate New sketch in File and observe Save become available. F10 focuses the menu bar. |
| Menus, windows & feedback › Popups & help | `popups` | Open the popover, then press Escape. Focus returns to its opener. |
| Menus, windows & feedback › Dialogs & notifications | `surfaces` | Open the nested confirmation and cancel it. The parent form remains open. |
| Menus, windows & feedback › Floating windows | `windows` | Drag the Out-1 title and resize its frame. Its controls keep their values. |
| Menus, windows & feedback › Tool windows | `tool-windows` | Select a file, then open Properties. The panels share the same selection. |
| Menus, windows & feedback › Progress | `progress` | Start the task, report a step and cancel. The final status retains the fraction reached. |
| Lists, tables & files › Collections | `collections` | Choose a row, then filter the data. Compare selection in a grid, a tree and an editable table. |
| Lists, tables & files › File browser | `files` | Open a folder in the sample library. Slow listing lets you see late results being discarded. |
| Lists, tables & files › File dialog | `file-dialog` | Choose a file and reopen the same request. It remembers the last accepted folder. |
| Lists, tables & files › Drag & drop | `dragdrop` | Drag a track from one list to the other. Use the copy modifier to keep the original. |
| Lists, tables & files › Virtual list | `virtual` | Jump to the last keyed row, then back to the first. Only nearby rows are mounted. |
| Text, graphics & appearance › Typography & drawing | `text` | Resize the window to wrap a label. Compare the bundled type styles and the custom drawing below. |
| Text, graphics & appearance › Fonts & icons | `fonts` | Change the default font, inspect a truncated label and compare icons inside rich text. |
| Text, graphics & appearance › Themes | `theme` | Override the active accent, switch mode and return. Each mode keeps its own palette. |
| Text, graphics & appearance › Images | `images` | Compare the same image fitted, cropped and stretched. Then compare nearest and linear sampling. |
| Text, graphics & appearance › Vector shapes | `shapes` | Compare the joins and caps, then zoom to inspect their edges. |
| Text, graphics & appearance › Gradients | `gradients` | Compare the linear and radial swatches, then move the sun in Horizon to change the composed scene. |
| Text, graphics & appearance › Level & trace | `meters` | Start the producer. Both meters read the same signal and the trace shows the recent samples. |
| Tools & runtime › Workspace | `workspace` | Capture an arrangement, change the divider and restore. The saved layout returns. |
| Tools & runtime › Profiler | `profiling` | Enable profiling, interact with a page and refresh the snapshot to see which work ran. |
| Tools & runtime › Debug overlay | `overlay` | Enable validation issues in Settings and arrange the sample wrongly to see the affected bounds. |
| Tools & runtime › Static & live | `live` | Start the producer beside the static checkboxes. Stop it to let the page go idle. |
| Tools & runtime › Text rendering | `text-rendering` | Change text rendering mode, scroll the sample and compare letter edges at the same zoom. |

For a custom in-place editor, see the public
[DraftEditor example](../../crates/pert-ui-runtime/src/text_fields/draft.rs).
The Text input and Sign-in form pages demonstrate the ready-made editors.

For alignment checks, narrow **Sample** after sorting it on **Lists, tables & files ›
Collections**; its heading should stop before the sort arrow. Resize the window
and post an actionable notification on **Menus, windows & feedback › Dialogs & notifications** to
see its buttons stack in a narrow viewport. On **Controls & forms › Text input**, compare
the tall field with the button beside it. On **Controls & forms › Toolbar**, compare
inline captions, values and units before and during value editing. Hover a
movable tab in the nested strip on **Layout & navigation › State & tabs** to compare its title
and detach icon.

For hover feedback, sweep the pointer over sliders and inline values on
**Controls & forms › Knobs & sliders** and **Controls & forms › Toolbar**, fields on
**Controls & forms › Text input**, rows and header grips on **Lists, tables & files › Collections**,
tab overflow arrows on **Layout & navigation › State & tabs**, notification action and
dismiss buttons on **Menus, windows & feedback › Dialogs & notifications**, and window title and
corner grips on **Menus, windows & feedback › Floating windows**. The expander in the tree on
**Lists, tables & files › Collections** highlights independently of its row.

Settings › Motion › Reduced stills the Progress page's indeterminate bar and
spinner; they then schedule nothing.

Press Ctrl+Shift+P (Cmd+Shift+P on macOS), or choose Help › Show all commands,
for the command palette: type to filter the gallery's commands, move with
Up/Down and run one with Enter. Its "Go to" commands open pages across
categories; Escape clears the query and then closes the palette.

Press Ctrl+Shift+F (Cmd+Shift+F on macOS), or choose Help › Search the gallery,
to search the gallery's content and commands in one list, each row labelled
Page, Content or Command: type a button's text, a label or a field's value from any
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
**Controls & forms** category header. It includes the demo's enabled profiling overhead:

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

Older gallery workspaces are translated by page key on import. When two formerly
separate strips now share a category, the visible selection takes precedence,
followed by the selection of the original category that still owns that family.
New pages use their defaults.

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

## Gradients

Open **Text, graphics & appearance → Gradients** or start directly on that page:

```sh
cargo run --locked -p pert-ui-gallery -- --page gradients
```

In **Horizon**, drag the spectrum slider to move the sun and its reflection.
Select **Change sky** to cycle through Afterglow, Arctic fjord and Moonrise.
The enamel button layers a standard interactive button above a gradient.
**Reset sun** restores the initial position and shows a gradient border
around a solid button face. Tab to it to see the fill shift from teal through
blue to plum. Tab away to restore the solid fill. Moving the pointer over the button adds an amber-to-turquoise wash. Moving it
away restores the focused or resting fill. The focus ring stays visible.
The material samples use reflected stops for brushed silver and repeating
hard stops for a two-ink print. The introductory swatches show a linear gradient, a concentric radial gradient and a
40-logical-pixel transition. Under **Menus, windows & feedback → Floating windows**, the
Out-1 title gradient ends at the center of its Information icon as the window resizes.
