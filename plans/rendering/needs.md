# Phase 0, item 1: Needs, and an inventory of today's drawing path

Branch `rendering-phase0`, read on 2026-11-05. Read-only survey (rg/reading,
no builds). "Code shows" means read in source; **Inference** marks what I
concluded but did not measure or run. Paths are relative to the repository
root. Line numbers are as of HEAD `78246d1`.

## 1. Today's drawing path

### 1.1 Recording: `Painter` and `DrawCommand`

`DrawCommand` (`crates/pert-ui-runtime/src/drawing.rs:1102`) is
`#[non_exhaustive]` with four variants, every one solid-colored:

| Variant | Fields | Notes |
|---|---|---|
| `FillRect` (:1120) | `rect`, `color` | No clip field; the clip is intersected into `rect` when recorded (`fill_rect`, :1755). |
| `DrawMask` (:1127) | `rect`, `mask: MaskId`, `color`, `turn: QuarterTurn`, `clip: Option<Rect>` | 8-bit linear coverage times a straight-alpha tint. |
| `DrawImage` (:1148) | `rect`, `image`, `tint`, `sampling`, `turn`, `clip` | Premultiplied sRGB RGBA8. |
| `DrawVector` (:1104) | `vector: VectorId`, `origin`, `scale: f32` (uniform), `color`, `clip` | Path + `VectorStyle::Fill(FillRule)` or `Stroke(StrokeStyle)` (`vector_scene.rs`). |

`DisplayList` (`drawing.rs:1289`) holds `background`, `viewport`, the
commands and three content-deduplicated resource arenas (masks, images,
vectors). Its doc says "Arbitrary transforms remain future work" (:1279).

Public `Painter` methods (listed in `api/pert-ui-runtime/default.txt:3450-3477`):

- Primitives in `drawing.rs`: `fill_rect` (:1755), `draw_mask[_turned]`
  (:1589, :1600), `draw_image[_tinted|_turned]` (:1653-1700), `with_clip`
  (:1712, rectangle only, intersected with the outer clip), `text`/`rich_text`
  (:1451, :1477, return a `TextLayout` the caller paints), `icon[_turned]`
  (:1508, :1528), plus `color(role)`, `metric(role)`, `focus_within`,
  `size`, `viewport`, `view_scale`.
- Shapes in `shapes.rs`: `fill_shape` (:330), `stroke_shape` (:366, inward),
  `stroke_arc` (:240), `fill_triangle` (:290). `Shape` (:17) is `Rectangle`,
  `Rounded(Length)` (uniform radius), `CutTopRight(Length)`, `RoundedTop(Length)`.
  No per-corner radii, no ellipse variant.
- Paths in `geometry.rs`: `fill_path` (:675), `stroke_path` (:713, centered),
  `stroke_polyline` (:763), `fill_ellipse` (:780), `stroke_ellipse` (:790).
  All go through `record_vector` (:798), which records `DrawVector` with
  `origin: Point::default()` and `scale: 1.0` (:806): the path keeps
  component-local absolute coordinates.
- Crate-private `focus_ring` (`shapes.rs:340`): `stroke_shape` with
  `FocusRing` at `FocusWidth`.

Every primitive takes a single `Color`; there is no paint, blend mode, layer,
opacity or transform parameter anywhere in the API.

### 1.2 Where coverage is produced

**Shapes (UI thread, at paint time).** `ShapeCache::image`
(`shapes.rs:49`) builds an `R8` `MaskImage` at `ceil(size * scale)` physical
pixels.
- Rounded/cut/triangle silhouettes: 4 sub-rows per pixel with exact horizontal
  span overlap (`SHAPE_ROWS`, :11; `span`, :167).
- Arcs: 4x4 supersampling with `hypot` and `atan2` per sample (`ARC_GRID`, :8;
  loop :68-100).
- A rounded rectangle is split up (`shape`, :383): up to three `FillRect`s
  for the body (or four for a stroke), plus four corner `DrawMask`s. Each
  corner is the same `2*edge` template mask clipped to its quadrant with
  `with_clip` (:465-481). The mask key therefore depends on corner size,
  stroke and scale, not on the rectangle's size. `CutTopRight` fills need
  only one corner mask (:422). Strokes on `Rectangle` are four `FillRect`s
  (:396-406).
- Cache key (`Key`, :36): `width, height, scale, shape/triangle
  vertices/arc start+sweep, stroke`. **Color is not in the key.** Storage is a
  `VecDeque` with linear `find` (:50), capped at 128 entries or 16 MiB
  (:6, :140). A hit does not move the entry, so eviction is FIFO rather than
  LRU (:50-52 against :139-148). The cache is one per runtime
  (`Painter::shapes`, `drawing.rs:1404`).

**Vectors (renderer side, before every frame).** `VectorRasterizer::prepare`
(`vector_scene.rs:170`) lowers each `DrawVector` to a `DrawMask` through
`geometry::rasterize_vector` (`geometry/raster.rs`, flattening at 0.2 px,
4 sub-scanlines, a direct-coverage path for straight strokes per the
`stroke_path` docs, `geometry.rs:695-703`).
- Key (:87): the full `VectorGeometry` (path + style), the raster scale and
  the clip in local units, intersected with the coverage bounds (:216), so a
  window resize that leaves the path whole still hits. Lookups go by the
  path's content hash (`content_key`).
- The cache keeps only the entries used by the current scene
  (`self.cache = next`, :298): one frame without a vector evicts it.
- Because the path is in absolute local coordinates (`geometry.rs:806`), a
  path that moves *within* its component is a new key and is rasterized
  again. Moving the component itself does not cost this, because composition
  applies the component origin to `DrawVector::origin`
  (`runtime/compose.rs:236-243`).
- Both GPU (`pert-ui-wgpu/src/lib.rs:784`) and CPU (`pert-ui-cpu/src/lib.rs:116`)
  renderers own a `VectorRasterizer`; `DisplayList::rasterized`
  (`drawing.rs:1324`) is the one-shot public fallback.

**Text.** Parley shapes the text and swash rasterizes the glyphs
(`text.rs:1316`, `swash::scale::Render`, :1396). Then "each run of
consecutive glyphs with one brush" becomes **one coverage mask**
(`text.rs:1687-1698`): in the default `LineMask` mode a whole multi-line
label is typically one `MaskImage`, recorded as `DrawMask` by
`TextLayout::paint_turned` (`text.rs:306-390`). The experimental
`PhaseLine` mode caches quarter-pixel-phase glyphs and composites run masks
per phase (`PhaseLineKey`, :462, :592). It repaints whenever the component's
fractional physical origin changes (`runtime/compose.rs:91-97`). Icons are
font glyphs rasterized at the next power of two of their physical size
(`drawing.rs:1540-1546`). **There is no glyph atlas**: on the GPU each
distinct text mask is its own texture.

**SVG** (facade `svg` feature) is rasterized once by resvg into a
`MaskImage` or `RasterImage` (`crates/pert-ui/src/svg_images.rs:6-10, :314`);
there is no SVG draw command.

### 1.3 Composition, clipping, opacity, transforms

`Runtime::paint_scene` (`runtime/compose.rs:8`) walks components in
composition order. It repaints only a component with `paint_dirty`
(:99) and otherwise reuses its retained local commands and resources
(`entry.drawing`, :175). Each command is then mapped into window
coordinates with `transform::map_rect(rect, origin, view)` (:212;
`transform.rs:448`): translation plus a **uniform** view scale, and the
ancestor clip (`entry.node.clip`, :192) is intersected.

- **Clipping** is axis-aligned rectangles only: `Option<Rect>` per command,
  or pre-intersected for `FillRect`. No rounded or path clip exists.
- **Opacity** is only per command, through color alpha (`Color::with_alpha`,
  `scale_alpha`, `drawing.rs:546, :571`) and the image tint. There is no
  group opacity, so overlapping translucent commands accumulate.
- **Transforms** (`transform.rs:1-6`, `docs/rendering-platform.md:109-119`):
  "deliberately no transform stack on Painter". Three tools exist:
  `Transform2D` maps **path geometry** before recording (`Path::transformed`,
  `geometry.rs:346`), so a rotated path is new geometry and new coverage.
  `ViewTransform` through `Component::child_transform`/`ZoomView` gives a
  subtree pan and uniform zoom. `QuarterTurn` turns masks and images. During a
  continuous zoom the runtime stretches prepared drawing and re-prepares it
  when the scale rests for 150 ms (`rendering-platform.md:121-131`;
  `Painter::view_scale` docs, `drawing.rs:1736-1747`). Raster scale is capped
  at 8 physical px per logical px (`runtime.rs:809, :825-831`). Text and images
  turn in quarter turns (`TextLayout::paint_turned`, `Painter::draw_image_turned`);
  arbitrary-angle text and image rotation and rotated subtrees are not
  possible.

### 1.4 GPU renderer (`pert-ui-wgpu`)

- Three pipelines, one per kind: `rectangles.wgsl` (solid), `masks.wgsl`
  (`R8Unorm` texture × color), `images.wgsl` (`Rgba8UnormSrgb`,
  premultiplied tint) (`textures.rs:66-83`). Blending is `ALPHA_BLENDING` for
  fills and masks and `PREMULTIPLIED_ALPHA_BLENDING` for images
  (`lib.rs:1244`, `textures.rs:71, :80`). The pass clears to the background
  (`lib.rs:568`).
- `geometry_commands` (`lib.rs:847`) emits **six 32-byte vertices per
  command** (NDC position, RGBA color, UV), with positions already in NDC.
  Clips are applied **on the CPU by cropping the quad and its UVs**
  (:905-925); no scissor or stencil is used. Unknown variants return
  `UnsupportedGraphics` (:902-905).
- Batching (`lib.rs:27-36`, `retained.rs:38-52`): consecutive commands with
  the same `Source` (`Fill`, `Mask(index)`, `Image{index, filter}`) merge
  into one draw. Each different mask or image means a new bind group and a
  new draw (`lib.rs:576-589`).
- Retention (`retained.rs:14-58`): a command slot reuses its encoded quad only
  if the viewport and the command at **the same index** are unchanged (:25).
  An insertion re-encodes every later slot (test, :165-172). The changed
  byte range is uploaded (`changed_bytes`, :62).
- Textures (`textures.rs`): **one texture plus bind group per mask or image**
  (`upload`, :226-260). They are pooled device-wide by content key, with weak
  entries released once no bound scene table refers to them (`Pool::bind`,
  :137-162). A mask absent from the current scene is therefore freed and
  uploaded again when it returns. There are per-kind scene byte budgets
  (`check_table`, :88).
- Presentation is `PresentMode::Fifo` with
  `desired_maximum_frame_latency = 2` (`lib.rs:1211-1212`). Every presented
  frame is a full pass (`rendering-platform.md:149-154`).

### 1.5 CPU renderer (`pert-ui-cpu/src/lib.rs`)

It validates commands (`check`, :281), prepares vectors with its own
`VectorRasterizer`, then interprets each command per pixel on one thread
(`Frame::draw`, :334-430). Rectangles are not antialiased (top-left
pixel-center rule, :378). Masks and images are sampled bilinearly or nearest,
and blending is source-over in linear premultiplied f32, stored as sRGB8
after every command (`blended`, :458). The documented model and the
GPU-parity tiers (exact and ±1 per channel) are in
`docs/rendering-platform.md:72-107`. `serve_captures` (:266) renders capture
jobs. Its measured cost is 41 ms at 1280×800 with 384 commands
(`docs/verification.md:421-440`).

### 1.6 Capture and inspection today

- Capture: `capture_scene` (`runtime/capture.rs:291`) re-runs `paint_scene`
  for the window, the host renders it with the real backend (CPU
  `serve_captures` or GPU offscreen `render_offscreen`, `lib.rs:670`), and
  `finish_capture` crops it. Any new primitive is therefore captured exactly
  when both renderers implement it.
- Inspection: `BoxModelInspection` (`diagnostics.rs:109`) reports
  `foreground`/`background` as one `[f32; 4]` and `border:
  BorderInspection { width, color }` (:161). `Container::inspect` fills them
  from what it painted (`containers.rs:364-379`). Snapshots carry a
  `SCHEMA_VERSION` that advances when a field is added
  (`docs/contracts.md:1594-1601`).

## 2. What the applications draw

Rough counts of call sites (rg, including docs) for `fill_rect` / `fill_shape`
/ `stroke_shape`: runtime 73/46/27, facade examples 11/1/3, apps 19/1/3.
`icon(` appears about 600 times in the runtime, mostly docs and `RichText`.
Vector calls are rare: runtime widgets use them only in the knob, the color
picker thumb and the resize grip.

| Site | Primitives (code) | Change rate |
|---|---|---|
| **Buttons, fields, chips, choice controls** (`controls.rs:530-535`, `choice_controls.rs:156-172, 399-472`) | `fill_shape` + `stroke_shape` `Rounded(CornerRadius)`; Toggle track/thumb `rounded(side/2)` (:418-420); checkmark via `icon` | Hover/press repaint only; corner masks shared across all controls with the same radius and scale. |
| **Tabs** (`tabs.rs:1671-1814`) | `CutTopRight` fill/stroke, underline `fill_rect`, icon, focus ring | On selection/hover. |
| **Focus** (`shapes.rs:340`, 38 call sites) | inward `stroke_shape` in `FocusRing`; single band (contrast band removed, test `shapes.rs:576-600`) | On focus change. |
| **Popovers, menus, tooltips** (`popover.rs:107-117` `transient_panel`) | `Container` Surface `fill_shape` + PanelBorder `stroke_shape` at `BorderWidth`, `Rounded(CornerRadius)` (`containers.rs:337-362`) | Static frame; content scrolls inside a rectangular clip. **Separation is only a 1 px border; no shadow, no scrim.** |
| **Modal dialog, modal quick pick / command palette** (`dialog.rs:466-472`, `quick_pick.rs:1366-1373`) | full-window `fill_rect` with **literal** `Color::from_linear(0,0,0,0.35)` scrim + transient panel | Static while open. |
| **Floating windows** (`windows.rs:1182-1229`) | `Rounded` Surface body, `RoundedTop` SurfaceContainer title fill, border `stroke_shape` (Primary when focused), grip `fill_path` triangle (`resize_grip.rs:6-21`) | Move: retained (component origin). Resize: grip path in absolute local coords, so **inference**: new vector coverage per resize step. |
| **Floating tool windows** (`tool_windows/panel.rs:640-665`) | `fill_rect` header, Divider line, `Rectangle` border | As above. |
| **Toasts** (`notifications.rs:1267-1290`) | rounded Surface + PanelBorder + text | On show/expiry. |
| **Spinner / progress** (`progress.rs:700-722`) | dots as `fill_shape(Rounded(dot/2))` with per-dot `scale_alpha` | Every animation frame; masks reused (same key), only colors change. |
| **LevelMeter** (`live_widgets.rs:323-385`) | 2–3 `fill_rect`: track, bar in one `ColorRole`, peak marker | Every drained frame; cheap (quads only). |
| **Trace** (`live_widgets.rs:574-620`) | one `fill_rect` per physical-pixel column (min/max envelope) | Every frame; up to width×scale quads; vertical edges not antialiased. |
| **Knob** (`numeric_controls.rs:425-520`) | `fill_ellipse` (hover disc), two `stroke_arc` (track, value), `stroke_polyline` pointer, readout text, focus ring | **Every value change**: new value-arc mask (key holds start+sweep, `shapes.rs:262-273`), new pointer vector (new endpoints), new readout text mask. Each is a new GPU texture. |
| **Color picker** (`color_picker.rs:825-868, 1193-1244`) | CPU-generated `RasterImage` gradients (64×64 SV area, 256×1 hue and alpha strips) stretched with linear sampling; `stroke_ellipse` thumb rings | SV image rebuilt when hue changes. |
| **ColorSwatch** (`color_swatch.rs:21-40`) | checkerboard of `fill_rect` cells (≤64 per axis) in fixed constant colors | Static. |
| **Table, VirtualList, Tree** (`table.rs:2659-2750, 3004-3027`) | header `fill_rect`, `with_clip` per column, text, icons, rect focus ring; cells are application `Element`s (usually `Label`) (`table.rs:937-955`) | Scroll: rows enter/leave and each new cell is a new text layout and mask. |
| **Labels** (`controls.rs:376-390`) | one `TextLayout::paint_turned` → one mask per brush run | On text change. |

Applications:

- **live_mixer** (`crates/pert-ui/examples/live_mixer.rs`) uses `Slider`,
  `Toggle::for_parameter`, `LevelMeter` (:460), buttons, labels and a
  `SplitPane`. It has **no knob and no trace**: those are in the gallery
  (`apps/gallery/src/pages/controls.rs:828-841`,
  `pages/properties.rs:84`, `live.rs:385-399`). Per frame only the meters
  change.
- **Gallery live page** (`apps/gallery/src/live.rs:491-525, 617-672`): a
  custom `PlotBackground` grid of `fill_rect` lines, kept as a separate
  retained component "so trace drains cannot invalidate these axes", plus a
  custom min/max column trace and meter, all `fill_rect`.
- **Gallery paths page** (`apps/gallery/src/samples.rs:282-378`): stars
  (both fill rules), caps, joins, ellipses, a rounded rectangle and a trace,
  "each one antialiased coverage mask, rasterized when the page paints"
  (`pages/data.rs:796`). Transforms page (`pages/layout/transforms.rs:310-340`):
  arrows rotated by `Path::transformed`, a sine fitted per axis, and a
  `ZoomView` of cards (:387).
- **diagram_editor** (`examples/diagram_editor/canvas.rs:794-880, 942-975`):
  - Background: `fill_rect`, plus a dot grid as **one 1×1 `fill_rect` per
    dot**, at least 8 px apart (:799-812).
  - Edges: `stroke_polyline` plus a `fill_triangle` arrowhead each, in view
    coordinates (:815-848).
  - Selection: rounded `stroke_shape` (:858). Rubber band: `fill_rect` with
    `scale_alpha(0.15)` and a rect stroke (:869-870). Connecting line:
    `stroke_path` (:879).
  - Nodes: `NodeView` children under `child_transform` (:593), drawn as
    `Rounded(6)` fill/stroke or `fill/stroke_ellipse`, with a clipped label
    in a literal ink color (:946-975).
  - Panning sets `editor.view` (:405-418). **Inference**: the canvas
    repaints on every pan step, re-records all dots and edges, and every edge
    polyline and arrowhead gets new coverage, because its points are in view
    coordinates. Nodes are only repositioned. A zoom stretches nodes and then
    re-rasterizes them at rest.
- **log_explorer / inventory**: no custom painting. They are `Table`,
  `Tree`, `Tabs`, `SelectableText`, `SplitPane`, `RegionLayout`,
  `ProgressBar` and toolbars (`log_explorer/app.rs:799, :903`;
  `inventory/app.rs:631`). Dense text means many small `Label` masks:
  **inference**, about rows × columns textures and draw calls per frame, with
  new uploads while scrolling.
- **dynamic_documents**: ordinary controls over dynamic lists; nothing custom.

Themes (`runtime/theme.rs`, `crates/pert-ui/src/guide/appearance.md`):

- `ColorRole` (:390): about 60 **solid** RGBA roles per mode, with follower
  defaults (:670-680) and seed palettes (`theme/seed.rs`).
- `MetricRole` (:78): lengths such as `CornerRadius`, `BorderWidth`,
  `FocusWidth`, `KnobStroke`, `MeterThickness`, `ScrollbarThumb`; density
  presets; zoom 75–200%.
- Fonts and icons are replaceable.
- Appearance.md:121-124 says "subtree theme providers, persistence,
  typography and spacing overrides are outside this color-theme API"
  (metrics do exist).

## 3. Workarounds and gaps (code shows; benefit is inference)

1. **Gradients faked as images.** The color picker builds and uploads
   gradient `RasterImage`s on the CPU and relies on bilinear stretching
   (`color_picker.rs:8-12, 825-868`). Nothing else in `crates/`, `apps/`,
   `docs/` or `TODO.md` mentions gradients. Meters use one flat role
   (`live_widgets.rs:356-362`); the gallery meter is a flat `Secondary` bar
   (`live.rs:621-631`).
2. **No shadows or elevation.** rg finds no shadow, elevation or glow in the
   code or docs. Transient surfaces are separated from what lies behind by a
   1 px `PanelBorder` alone (`popover.rs:107-117`), modals by a hard-coded
   35% black scrim (`dialog.rs:470`, `quick_pick.rs:1371`). The scrim is a
   literal, not a `ColorRole`, so a theme cannot change it.
3. **No rounded clip.** Rounded panels clip their content rectangularly
   (`ArrangeContext::place_clipped`, `windows.rs:1175`). The floating window
   paints its title bar as a separate `RoundedTop` fill to match the outer
   corners instead of clipping (`windows.rs:1189-1193`). **Inference**:
   popover padding (`popover.rs:95`) is what keeps hover rows off the corners.
4. **Rotation computed on the CPU.** The knob pointer is recomputed as a
   polyline every value (`numeric_controls.rs:476-492`), and its value arc is
   a new 4×4-supersampled `atan2` mask per value (`shapes.rs:68-100`). The
   diagram arrowhead is a new triangle mask per orientation (`fill_triangle`
   key holds the local vertices, `shapes.rs:300-312`). The gallery arrows use
   `Path::transformed`.
5. **Zoom by re-rasterization.** `ViewTransform` stretches during a gesture
   and re-prepares everything at rest (`rendering-platform.md:121-131`).
   Shape keys include the scale (`shapes.rs:39`), so each resting zoom level
   is new coverage for every shape, text and vector under the view.
6. **Live vector data as rectangles.** The built-in `Trace` and the gallery
   trace draw a min/max column envelope of `fill_rect`s (`live_widgets.rs:574-620`).
   The realtime probe measured this at about 1.1 ms in total, against
   2.3–5.1 ms for an antialiased polyline whose coverage is rasterized every
   frame (`verification.md:383-388`). The envelope trades antialiasing for
   cost.
7. **Opacity per command only.** The rubber band, the spinner and hover veils
   use `scale_alpha` on each color (`canvas.rs:869`, `progress.rs:719`,
   `appearance.md:146-163`). A drag preview (`runtime/drag.rs:46-60`) or a
   panel cannot be faded as a group. No fade animations exist: only progress
   and field status animate (rg `animation`).
8. **Dense small rectangles.** The diagram dot grid draws one quad per dot
   (`canvas.rs:806-812`) and the checkerboard draws one quad per cell
   (`color_swatch.rs:21-40`). **Inference**: a pattern or tiled paint would
   replace them.
9. **Shape-cache churn.** A FIFO cache of 128 entries (`shapes.rs:50, :140`)
   with a new key per knob value means **inference**: dragging a knob through
   more than about 128 values evicts the shared rounded-corner masks, which
   are then rasterized again.
10. **Text masks per layout, no atlas.** One texture and one draw per text
    run (`text.rs:1687`, `textures.rs:226`). Batches merge only for an equal
    source (`retained.rs:38-52`), so a rounded control costs at least a fill
    batch and a mask batch.
11. **Rectangles are not antialiased** on either renderer
    (`rendering-platform.md:79`). Fractional edges snap to pixel centers.
12. Doc inaccuracy found: `stroke_path`'s rustdoc says `stroke_shape` strokes
    a "rectangle, rounded rectangle or ellipse inward"
    (`geometry.rs:689-690`), but `Shape` has no ellipse variant
    (`shapes.rs:17-28`). Outside this task's scope: report it and add it to
    TODO.md if the maintainer agrees.

**What a theme author can express today:** solid colors (with alpha) per
role and mode, seeds, corner radius, border and focus widths, control,
knob and meter sizes, density, fonts and icons.

**What they cannot:**

- Gradients on any role (meters, buttons, surfaces).
- Shadows, elevation, glows or blur.
- Per-corner radii, per-widget shape choice (tabs are fixed `CutTopRight`,
  `tabs.rs`) or dashed and double borders.
- The scrim color, the checkerboard colors (constants,
  `color_swatch.rs:10-11`) and meter color zones (one role per meter).
- Opacity of disabled content as a group (disabled states swap roles,
  `appearance.md:20-27`).
- Typography (appearance.md:121-124).

Real-time needs named by the plan (waveforms, spectra, scrolling traces):
today only the column-rect envelope or the per-frame polyline mask
(`verification.md:367-410`), and a "waterfall" `RasterImage` rebuilt per
frame (0.45–0.79 ms rebuild, :397-401). There is no scrolling-buffer
primitive.

## 4. Candidates A–I: who benefits, and what each touches

Columns for every candidate: **Capture** means both renderers must draw it
because capture uses the real backend (§1.6). **Inspection** is the DTO in
`diagnostics.rs:109-170`. **CPU** is `pert-ui-cpu`. **API** is the `api/`
listings (`DrawCommand` is `#[non_exhaustive]`, so a new variant is additive,
but its variants have public fields, so a new field on an existing variant
is a breaking listing change).

**A. Paint model with gradients**
- Benefits: color picker strips and area (`color_picker.rs:825-868`, would
  drop three image generators); LevelMeter and gallery meter zones
  (`live_widgets.rs:356`, `live.rs:621`); button and surface theme gradients
  (`controls.rs:530`, `containers.rs:337`); the checkerboard as a pattern
  paint (`color_swatch.rs:21`); diagram node fills (`canvas.rs:950`).
- Capture: no change beyond parity.
- Inspection: `background: Option<[f32;4]>` cannot describe a gradient. It
  needs a paint DTO and a schema bump, or a documented "representative
  color".
- CPU: per-pixel gradient evaluation in `Frame::draw`, plus dithering
  parity.
- API: `Paint` in every `Painter` fill and stroke signature (about 15
  methods, `default.txt:3450-3477`), a `color` field becomes `paint` on all
  four variants (breaking), and theme roles or `Palette` need paint values.

**B. Analytic shapes**
- Benefits: every rounded control and panel (`shapes.rs:383-483`: 3–4 fills
  plus 4 corner masks become one primitive); knob arcs per value
  (`numeric_controls.rs:451-475`); spinner dots (`progress.rs:711`);
  ellipses in the diagram, color picker and knob; focus rings (38 sites);
  zoom re-preparation for shapes (§3.5). Removes `ShapeCache` and its FIFO
  churn (§3.9).
- Capture: none.
- Inspection: none (`Shape` is already painting-only).
- CPU: SDF coverage per pixel in `Frame::draw`, with a stated tolerance
  against today's 4-row coverage.
- API: new `DrawCommand` variant(s) (additive). `Painter` signatures can stay.
  Per-corner radii would extend `Shape` (additive if `Shape` gains
  `#[non_exhaustive]`; today it does not, so a new variant breaks exhaustive
  matches: `shapes.rs:16`).

**C. Instanced rendering**
- Benefits: the trace column envelope (up to width×scale quads per frame,
  `live_widgets.rs:613`); the diagram dot grid (`canvas.rs:809`); tables
  with many cells. **Inference**: the gains are vertex bytes (192 → about 48
  per command) and simpler retention (`retained.rs`'s index-aligned slots
  re-encode after an insertion).
- Capture, inspection, CPU, API: none (internal to `pert-ui-wgpu`).

**D. Shadows and blur**
- Benefits: popovers, menus, tooltips and toasts (`popover.rs:107`,
  `notifications.rs:1270`); floating windows (`windows.rs:1189`) and tool
  windows; dialogs (`dialog.rs:466`); the drag preview. Theme elevation and
  glow roles.
- Capture: none for an analytic shadow. A backdrop blur reads the frame
  below, which needs an extra pass and a CPU readback of its own output.
- Inspection: shadows are paint-only. **Inference**: the box model's
  `border_box` stays the layout box and needs a note that the shadow extends
  beyond it.
- CPU: an erf box shadow is cheap per pixel; blur is a convolution over the
  frame, costly at 41 ms per frame baseline.
- API: new variant plus a `Painter` method; theme roles (shadow color,
  elevation metrics); probably a `Container` builder.

**E. Glyph atlas**
- Benefits: log_explorer and inventory tables, Tree, `SelectableText`
  (`log_explorer/app.rs:799-903`); knob readouts that change per value; all
  labels. It removes one texture and one draw per text run (§3.10).
- Capture: none.
- Inspection: none.
- CPU: either keep run masks for the CPU (two paths) or blit glyphs from the
  same atlas. `PhaseLine` already caches per-phase glyphs (`text.rs:462-592`)
  and is the natural feed.
- API: probably a new glyph-run `DrawCommand` (additive). `MaskImage` text
  resources would change, so `DisplayList::masks` would no longer hold text.
  That breaks consumers reading text as masks, such as the diagram session
  test that finds labels by ink color (`label_ink`, `canvas.rs:906`).

**F. Lines and live vector data on the GPU**
- Benefits: `Trace` (`live_widgets.rs:574`) and the gallery live trace
  (`live.rs:634-672`), which would become antialiased polylines without the
  2–4 ms per-frame raster (`verification.md:383-388`); diagram edges and the
  connecting line during pan (`canvas.rs:830, 879`); the knob pointer
  (`numeric_controls.rs:485`); waveform and spectrum needs.
- Capture: none.
- Inspection: none.
- CPU: needs the same segment coverage; the direct-coverage stroker in
  `geometry/raster.rs` is the reference.
- API: a polyline/buffer variant, or `DrawVector` consumed natively.
  `VectorRasterizer` (public) would remain the CPU path.

**G. Shaped clips, opacity groups, transforms**
- Benefits:
  - Rounded clips: rounded panel content and the floating window title
    (`windows.rs:1189-1193`).
  - Group opacity: fading panels, toasts and the drag preview; disabled
    subtrees; it would replace per-command `scale_alpha` (§3.7).
  - Transforms: knob pointer rotation without new geometry; diagram zoom
    without re-preparation; rotated text and images (impossible today,
    `rendering-platform.md:111-119`).
- Capture: an opacity group needs an offscreen layer in both renderers.
- Inspection: affine transforms break the box model's axis-aligned
  `border_box` and the "half-open hit regions" invariant
  (`rendering-platform.md:111-113`). Clip and opacity would need new fields
  or a documented absence.
- CPU: layers mean an allocation per group; non-axis-aligned sampling for
  rotated masks and images.
- API: push/pop or closure groups on `Painter` beside `with_clip`, new
  variants, and possibly `ViewTransform` generalized; `transform::map_rect`
  stops being exact.

**H. Layers and partial redraw**
- Benefits: the gallery `PlotBackground` beside a live trace (`live.rs:491`,
  which is the damage-probe scenario, `verification.md:320-366`); the diagram
  canvas grid under moving edges; static panels beside meters in the mixer.
  The baseline (item 4) decides.
- Capture: a capture must still compose the whole frame from layers.
- Inspection: a stable layer id per component could be reported.
- CPU: retained layer images and a composite step; the CPU renderer could
  gain the most, since it re-renders the whole frame on the UI thread
  (`rendering-platform.md:57-60`).
- API: `DisplayList` gains layer identity and revision (`verification.md:323-330`
  lists the needs). `WidgetId` is not in commands today.

**I. Latency**
- Benefits: the mixer (meters, slider audition) and any playing or recording
  use. Today `Fifo` with latency 2 (`lib.rs:1211-1212`); redraws are
  scheduled by `crates/pert-ui/src/platform/schedule.rs:167, :236` from the
  monitor refresh rate; acquisition waits about 8 ms (`verification.md:357-360`).
- Capture: none.
- Inspection: optional frame-timing diagnostics through the profiler.
- CPU: softbuffer presentation has its own pacing.
- API: possibly an `App` or host setting for the present mode (facade
  listing).

## What I could not determine

- Actual per-frame costs (masks rasterized, textures created, draw calls) of
  the gallery, the mixer or the diagram editor: no runs were allowed, so
  item 4 must measure them. Every "inference" above about churn and per-pan
  re-rasterization is unmeasured.
- Whether the diagram canvas really repaints on every pan step. I read the
  `editor.view` signal write (`canvas.rs:418`) but not where `Canvas::update`
  reads it.
- Whether menu and popover rows can reach the rounded corners, which would
  make a missing rounded clip visible.
- The exact batch and draw counts for a typical gallery page, and how often
  the weak texture pool re-uploads masks that disappear for one frame.
- Whether `Shape` or the theme roles are matched exhaustively by external
  code. That affects whether B's and A's additions are breaking beyond the
  listing.
- Native-platform behavior (Windows, macOS): nothing here was observed
  outside source reading.
