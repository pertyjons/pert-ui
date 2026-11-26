# Component gallery review

Implemented on 2026-11-26. The [gallery guide](../apps/gallery/README.md)
lists the current destinations, launch keys and interactions.
The findings and delivery sequence below record the original design review.

## Findings before implementation

The review covered the registered pages in [gallery.rs](../apps/gallery/src/gallery.rs),
their first viewport rendered with the headless CPU screenshot command at
1280 × 800, the page builders, guide, navigation and gallery tests. The
captures show the initial state. Interaction evidence comes separately from
the gallery tests. This review did not exercise a native desktop session.

The gallery already has good comparisons. Images presents the same asset
with different fits and sampling. Multi-select offers immediate and deferred
commits. The tool-window examples share a selection, so changing one view
demonstrates what happens to the others. Keep those relationships. On the
other hand, finding a knob used to require scrolling past a physical keyboard
demo and two host-side parameter editors. Finding the file dialog requires
looking under File browser. Buttons appear under Text & buttons, Fonts &
icons, Toolbar and Gradients for different reasons. The Controls page contains
both choices and popup surfaces. The general Help dialog explains the shell.

Many pages already explain interactions. The File dialog introduction also
tries to teach selection modes, persistence, places, folder creation, filters
and replacement confirmation before the first action. On the Menu bar page
a long paragraph describes the entire keyboard sequence above a small
interactive menu. Keep the instructions beside the relevant action and let
people open the rest when they need it.

## Proposed navigation

Organize the primary catalogue by what a visitor wants to build. The table
maps the existing page keys to their proposed homes. A split names the
specific section that moves. Keep complete application examples reachable
as examples in their own right.

| Destination | Existing page keys and proposed organization |
|---|---|
| Layout & navigation | Keep `layout`, `splits`, `shells`, `scroll`, `sections`, `transforms`, `state` and `navigation`. Name the entry page “Rows, columns & grids”. Move the breadcrumb example here. Give the physical-key example from `input` its own input section. |
| Controls & forms | Make Buttons a destination using the buttons from `text` and icon adornments from `fonts`. Put single-choice and Boolean examples from `controls` with the chips and multi-select examples from `chips`. Put `editing`, `numbers`, `input`, `families`, `fields`, `forms`, `properties`, `colors` and `toolbar` here. Present standard controls before their inline and host-integration examples. Separate Number entry (draft, parse, save) from Knobs & sliders (gesture, preview, reset). |
| Menus, windows & feedback | Bring together `menus`, `surfaces`, `windows`, `tool-windows` and `progress`. Make menus/popovers from `controls` findable here. Link the small menu-bar sample to the working shell. Within feedback, distinguish reporting a task, showing a status and asking for a modal decision. |
| Lists, tables & files | Keep `virtual`, `collections`, `files` and `dragdrop`. Give Grid, Tree and Table direct destinations inside Collections. Give File dialog its own destination with Open, Select folder and Save examples. Link back to the shared host-backed File browser. |
| Text, graphics & appearance | Put typography from `text` with `fonts`, then `theme`, `images`, `shapes`, `gradients` and `meters`. Move the small custom Painter example here. Show a basic shape or swatch first, with Horizon available as a composed example. Keep the interactive scene intact. |
| Tools & runtime | Put `workspace`, `profiling`, `overlay`, `live` and `text-rendering` here. Link the live workload to Level & trace and the rendering-mode comparison to typography. Keep the workload paused on entry. |

Use the same page metadata for navigation, page search results and the
page's help title. A page destination should be discoverable under its
public component names, e.g. “knob”, “MultiSelect” and “FileDialog”.
Preserve the existing content search for values, controls and collection
items. Start with the existing detachable strips, history and shortcuts.
Evaluate a sidebar only if finding a named component still takes too long
in the narrow layout. The Controls strip already overflows at desktop width.

A page key is only one segment of a workspace address. Moving the page to
another category changes that address even if its key survives. Before
moving mounted views, inventory path users in history, reveal links, launch
arguments, screenshots, workspace save/restore and interaction checks.
Keep old launch keys working. Test restoring a real pre-change gallery
workspace. Where a path needs translation, implement and test that
translation at the gallery boundary before moving it. Preserve local
drafts, active tabs, focus and detached views when navigating during a
session. Leave the library workspace schema under its own contract.

## What an example should teach

Give each page a purpose and each comparison an observable outcome. Show a
working default and vary one salient property at a time where a user needs
to choose between alternatives. Label visible differences next to their
controls. Use the same state when comparing presentation; use distinct
state when comparing save, cancel or refusal policies. Show the saved
value separately only when it teaches that distinction.

Put one short action beside the example. On Knobs and sliders it is
“Drag a knob up. Both sliders and the other knobs follow the same value.”
Follow it with the real controls. A keyboard-accessible Collapsible can
explain differences, keyboard actions and the pertinent API names. Use
Field help for entry rules, choice descriptions for individual options
and tooltips for compact icon actions. Keep essential instructions available
without hovering. Composed examples should name who owns the shared state:
on File dialog, the application supplies the directory listing, and
accepting a file updates the dialog's memory for its next invocation.

## Public API and variants

Use the facade and its supported feature configuration as the inventory.
Compare the public runtime Component implementations, their facade exports
and the gallery builders. A mention or a successful build establishes
neither a visible comparison nor an explanation. Macro-generated and
composed controls also need source inspection. The gallery package already
depends directly on the facade alone.

| Family | Evidence and next change |
|---|---|
| Numeric controls | [Controls](../apps/gallery/src/pages/controls.rs) already uses Bakelite, Orbit, Diode, arc origin, custom formatting and exact reset. The revised page puts the actual controls first, demonstrates the readout style and exposes keyboard details on demand. Add a same-face comparison if the visitor's question is specifically about readout sizing. |
| Choices | The toolbar uses fixed `Choice::with_description`. The waveform descriptor now also demonstrates `ChoiceOption::with_description` through a parameter-backed Select. Add a `Select::bound` example that changes the option list while it is visible and shows what happens to selection. |
| State ownership | The shared checkbox and toggle demonstrate immediate signal writes. Add `Checkbox::controlled` alongside them, with the proposed value and an explicit application acceptance action. Explain why the presentation waits for a new description. |
| Text and input | [Text](../apps/gallery/src/pages/text.rs) already has drafts, external writes, IME-capable editors, search, password and field status. Move the sign-in form into composed examples and link back to each input. A small rich-label comparison can demonstrate `TextSpan::style` and the Micro roles. Ordinary text and numeric examples should lead. The public `DraftEditor` is an advanced extension point covered indirectly through controls; give it a source link or a custom-editor example. |
| Files and collections | [Files](../apps/gallery/src/pages/data/files.rs) uses FileDialog, FileDialogMemory, places and the host's new-folder operation. Expose that existing example directly. In Collections lead with a small selectable list, then editable cells, filtered trees and the large virtualized datasets. |
| Drawing and theme | [Gradients](../apps/gallery/src/pages/gradients.rs) and the [floating title](../apps/gallery/src/pages/surfaces.rs) demonstrate available drawing APIs. Distinguish the custom title drawing from the title paint builder. Theme-role gradients remain proposed. Keep literal drawing and theme palette examples separately labelled. |
| Layout, windows and diagnostics | Keep comparisons of fixed/scrollable views, nested splits, locked/open tool windows, view/window modality and the opt-in invalid layout. Add a short expected result to the first action in each composed view. The profiler and the stress workload belong together. |

## Delivery order and verification

1. Use Input & parameters as the pilot. Preserve separate examples of the
   controls, physical keys and host editing. Check the first viewport,
   narrow layout, keyboard operation, reset and shared value.
2. Build the catalogue from the page declarations. Test that every mounted
   destination can be found and opened, including aliases and return through
   history. Use the same destinations in the README index.
3. Split or move the mixed pages by family. Verify the workspace migration
   and navigation contracts above with an older gallery document and the
   actual launch, reveal and restore entry points.
4. Bring the remaining examples to the pilot's help pattern in family-sized
   changes. Add the missing API comparisons with interaction tests that
   check the visible outcome. Review images of the first viewport in light
   and dark themes, desktop and narrow widths; open help using only the
   keyboard. Preserve focus, scrolling and idle scheduling. Run the existing
   gallery package and native interaction checks on available platforms.

For future component changes, add the gallery page/section or the reason a
manual example is more appropriate to the change review. Compiling the old
example catches removed APIs; only a comparison with the changelog and
facade reveals a new, easier path the old example still bypasses.

## Implementation evidence

The catalogue supplies the mounted pages, search aliases, page commands,
introductions and related destinations. Gallery tests exercise the public
controls, navigation, compact layouts, zoom, semantic descriptions and idle
scheduling. The CPU-rendered first viewports were reviewed at desktop and
narrow widths in light and dark themes. The
[screenshot guide](../docs/screenshots/README.md) shows selected examples.

The native interaction check ran on Fedora 45 GNOME/Wayland. Separate gallery
processes saved and restored a new workspace and restored the original
catalogue fixture. A fresh-context review of the migration led to validation
of original category ownership and explicit coverage of the visible selection
when old strips converge. Native Windows and macOS evidence remains open in
the [platform tracking list](../TODO.md).
