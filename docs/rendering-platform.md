# Rendering and platform contract

This document describes coordinates, presentation and supported native
boundaries. [Architecture](architecture.md) explains package ownership;
[verification](verification.md) lists what has actually run on native systems.

## Coordinates and drawing

Component layout and `Painter` geometry use logical pixels. A `Viewport`
connects logical size and physical scale; UI zoom and monitor DPI scale text,
icons, spacing and hit regions together. Explicit transforms map content while
input and clipping follow the same spaces. Fractional scale may place boundaries
between device pixels. The renderer preserves ordered clipping and alpha
composition rather than silently rounding application coordinates.
Positive `Painter::stroke_shape` outlines have a minimum rendered width of one
physical pixel. This keeps straight sides of thin control borders visible when
zoom falls below 100%, without moving layout or snapping the whole interface.
Built-in focus rings use one accent-colored band at every zoom level. Their
former dark inner contrast band could appear on only some sides at fractional
zoom.
Filled rectangles, vector strokes, text and images retain their own sampling
rules.

Color inputs are straight linear RGBA; drawing blends premultiplied color in
linear light and stores RGB through the sRGB attachment encoding. Native surfaces
prefer opaque compositor alpha: the background retains its RGB with alpha one.
If opaque is unavailable, premultiplied compositor alpha is required and the
background is premultiplied before clear. Other compositor alpha policies are
rejected rather than inferred. Surface recreation retains and validates the
selected alpha policy. Offscreen captures always preserve background alpha with
linear-premultiplied RGB encoded as sRGB, independently of native presentation.
PNG encoding converts that linear-premultiplied representation to straight color;
it does not divide the encoded sRGB bytes directly.

## CPU rendering

`pert-ui-cpu` interprets the same `DisplayList` as the GPU renderer, through its
public accessors, and produces a `FrameImage`. It serves the runtime's capture
requests headlessly, so window, region, component and path screenshots work
without a display or GPU. Browser rendering is not implemented.

### Software presentation

With the `cpu` feature an application can call `App::software_rendering`.
Every native window then presents frames rendered by `CpuRenderer` through
softbuffer, and no GPU adapter or device is created. It is opt-in and chosen
once before `run`: an unsupported GPU still fails explicitly, and nothing
switches renderers during a session. Each presented frame is rendered whole on
the UI thread, as an opaque surface: the clear uses the background's RGB at
full alpha, as an opaque GPU surface does, and the sRGB bytes are packed into
softbuffer's `0x00RRGGBB` pixels. Transparent native windows are not offered.
Captures render separately with the background's alpha kept, so a capture
follows the same contract on either path.

Input, IME, focus, scale, captures and native windows go through the same
host as with the GPU. Only the presenter differs, behind an internal seam in
the platform layer; no backend type reaches the public API. A window whose
frame would exceed `FrameImage::MAX_BYTES` stays unpresented and reports
`LimitError::FrameSize` through the error policy, as an oversized GPU surface
reports `SurfaceExtent`. There is no device loss to recover from. On Linux the
presentation uses Wayland shared memory; X11 is not supported here either.

Its model is the GPU's, written down so that both can be checked against it:

- **Storage.** Each command blends in linear-premultiplied `f32` and stores the
  result as 8-bit sRGB-encoded premultiplied RGBA with linear alpha, so
  translucent overlaps are quantized after every primitive, as the GPU's sRGB
  attachment does. The frame is cleared with the background's RGB multiplied by
  its alpha.
- **Coverage.** Rectangles are not antialiased. A physical pixel is covered when
  its center lies in the half-open span of the logical rectangle times the
  scale, which is the top-left rule. Clips intersect before iteration.
- **Sampling.** Masks and linear images are filtered bilinearly with
  clamp-to-edge addressing at the pixel center, mapped through the command's
  quarter turn. Image texels are decoded from sRGB to linear before filtering.
  Nearest images take the texel under the center.
- **Blending.** Source-over: `out = src + dst × (1 − src.a)`, with fills and
  mask tints premultiplied from straight `Color`, and image texels multiplied
  by the premultiplied tint.
- **Vectors.** They are prepared into coverage masks by the runtime's
  `VectorRasterizer`, with its limits, as for the GPU.
- **Budget.** The frame must fit `FrameImage::MAX_BYTES`, checked before
  allocation; vector preparation keeps its own limits. A refused scene leaves
  the renderer usable.

Byte-identical output across backends is not promised. The CPU renderer is
tested against hand-computed pixels. On hardware it is compared with the GPU in
two tiers. The exact tier (opaque integer-aligned fills in exact sRGB colors,
nearest images and their quarter turns, the opaque clear) must match byte for
byte. The tolerance tier (translucency, bilinear filtering, text, vectors,
fractional edges and scales, arbitrary linear colors) allows one step per
channel. One exception is outside both tiers: a rectangle edge within the GPU's
subpixel precision of a pixel center (typically 1/256 pixel) can be snapped
across it by the hardware. That pixel is then covered by one renderer and not
the other, a full color difference. The CPU renderer tests the unsnapped edge.
No shared snapped grid exists yet, so the corpus keeps its fractional edges
away from pixel centers. [Verification](verification.md#cpu-rendering-cost) records where the
comparison ran and what rendering costs.

## Three transform tools

There is no arbitrary transform stack on `Painter`. Rectangular clipping,
half-open hit regions and coverage caches depend on axis-aligned rectangles.
The implemented tools cover distinct needs without changing that invariant:

| Need | Public tool |
|---|---|
| Pan and uniform zoom of ordinary child components | `ViewTransform` through `Component::child_transform`, or `ZoomView` with `ZoomController`. Input is mapped back into content space. |
| Rotate, mirror or stretch drawn path geometry | `Transform2D` applied to the path; child layout does not rotate. |
| Turn a text mask or image sideways | `QuarterTurn`, preserving axis-aligned destination bounds. |
| Map independent data axes to pixels | One `AxisMapping` per axis, in the component; text and stroke widths remain undistorted. |

A pan repositions retained child drawing without changing text measurement.
A settled zoom prepares descendants at the shown scale. During consecutive
scale changes within 150 ms, the runtime stretches prepared drawing and
prepares it once when the scale rests. This rule came from a local release-mode
headless canvas measurement on Fedora 44: repeated full preparation cost
10–18 ms in the median for the measured directions, while the retained path
was under 1.1 ms at the 95th percentile after the optimization. The first
change and the final preparation still cost substantially more; this is not a
GPU or cross-platform performance guarantee. View magnification caps coverage
preparation to eight physical pixels per local pixel, and very large text is
prepared within its font-pixel limit then stretched. Surfaces opened by a
transformed child are placed at the window's scale. Vector coverage is bounded
per primitive: besides the flattening budget and the mask size limit, a vector
whose edges or stroke pieces would cross more than
`VectorRasterizer::MAX_COVERAGE_WORK` sub-scanlines is rejected with
`DrawError::TooComplex` before any coverage work.

Vector fallback preparation retains the documented 0.2 physical-pixel chord
tolerance within its generated-point and outline budgets. If that tolerance
requires more work, it returns a typed path budget failure instead of reducing
accuracy. Cropped coverage uses bounded local indices for extreme offsets and
retains the original grid phase and visible endpoint geometry. Path bounds round
outward when needed to contain every endpoint/control point; a path without a
finite conservative extent is rejected. The mask and active-coverage budgets remain distinct; intermediate row-index memory admission
remains a separate limitation.

Widgets record solid fills, text coverage masks, raster images and vector
geometry through `Painter`. `DisplayList` is an owned, readable scene of drawing
commands and resources. Runtime contains no GPU type. The GPU adapter validates
surface and device limits, prepares buffers and textures, and renders the scene.
The current adapter caches command encodings and uploads changed byte ranges;
presentation still draws a full render pass when a frame is due. Damage
rectangles and partial presentation remain measurement-led proposals, not a
generic runtime guarantee.

## Text, icons and images

Bundled fonts and icon assets provide a usable default without system font
discovery. Text shaping, layout and coverage preparation are retained and
bounded. Font and text admission limits are reported as typed errors. Rich text
can combine spans and icons; the same text service supports labels and editable
fields. Consumers may register font and icon replacements through the public
manager APIs. Asset attribution and licenses ship with the relevant packages.

`UiSettings::set_text_rendering` chooses between the retained line mask
(`LineMask`, the default) and quarter-pixel phase glyphs composited into a run
mask (`PhaseLine`). The latter is experimental. It uses Swash grayscale
coverage and hinting, then composites glyphs of one brush before they enter
the display list. A mode change repaints text without changing layout,
caret or selection geometry. For scrolling text the phase mode repaints
when its physical origin changes, even when the source content is unchanged.
Glyph and line caches each admit at most 16 MiB of coverage data per runtime;
cache keys, metadata and retained display-list resources add memory. Glyph
coverage is shared while a phase-specific run mask is built. Rotated text
and text stretched during a live zoom use the default path until their stable
geometry is prepared. Both CPU and GPU consume the same display list, so this
setting does not select a backend. The gallery's Text rendering page offers
live controls for visual comparison.
On a phase-cache miss, inline icons also use an unphased reference raster to
keep their vertical alignment when fractional coverage changes the ink bounds.
If phase coverage exceeds a raster limit, that paint uses its already prepared
line mask.

`RasterImage` and `Image` separate owned pixels from layout and sampling.
Decoding belongs to optional facade features. Vector paths preserve geometry
through the display list; the backend prepares their coverage. Optional SVG
loading parses supported drawing content and reports unsupported or hostile
input instead of inventing a fallback. SVG text is outside the current parser
contract. Text, image and vector resources are shared by content where the
scene's ownership permits it.

## Scheduling and visibility

The event loop presents on demand. Changed bindings invalidate their readers;
live sources and animations request shared deadlines only while their content
is visible and active. Missed frames skip ahead. Hidden, unmounted and fully
clipped components stop unnecessary presentation work; reveal refreshes their
stale presentation. One accepted exception: a native window whose surface is
blocked before its first presented frame, as macOS reports until the window
server first shows a window, pauses its own presentation, updates and timers
but stays visible to the runtime, so the focus and timers set up while it was
created survive. Live sources are shared, so while another window is active
that window's subscribers keep their sources draining until it presents or
reports occlusion. This was accepted on 2026-10-12 until a bug report shows a
window that stays blocked without an occlusion event; the fix would be a
runtime state that pauses a window's sources without ending its input. A
reported monitor refresh rate is a scheduling ceiling,
not a promise that every frame is delivered. Static content can retain its
layout, text and drawing beside a changing source.

Worker computation may deliver results through the UI mailbox. It does not run
component callbacks on a background thread. The GPU is never used from a
time-critical producer callback. Selective parallel preparation and stable visual layer identities are
possible future optimizations after workload measurements and ownership rules
justify them.

## Native platform boundary

The default facade owns windows and translates native pointer, keyboard, IME,
scale and monitor events into runtime events. The runtime remains headless.
The selected native stack uses current window and GPU dependencies with Metal
on macOS, DirectX 12 on Windows and Vulkan with Wayland on Linux. Compatible
integrated hardware is preferred; CPU or virtual adapters are rejected. The
renderer requests a conservative capability profile, an SDR sRGB surface and
FIFO presentation. Surface, adapter, device-loss and size failures have typed
outcomes and defined recovery behavior. Colors in the public drawing API are linear; the
surface applies the sRGB encoding on output, and `Color`'s sRGB conversions use
the same IEC 61966-2-1 transfer, so a color given as sRGB bytes is presented
with those bytes.

Native multi-window support can move a retained view into another OS window
and return it without assigning a new component identity. Each window has its
own scale, presentation state and focus; application-scoped commands remain
shared. The feature is experimental pending broader native evidence. In-view
floating windows, dialogs, popovers and tooltips are runtime surfaces within an
existing OS window and follow their own focus and dismissal contracts.

The initial platform matrix and the evidence for each tested contract are in
[verification](verification.md#native-evidence). Headless compilation and tests
do not establish native GUI or GPU behavior on an untested platform.
