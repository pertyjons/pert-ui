# pert-ui-wgpu

Hardware graphics adapter for the owned `pert-ui-runtime::DisplayList` contract.
The normal application entry point is `pert-ui::App`; it selects and drives this
adapter automatically. UI components and drawing commands do not depend on wgpu.

This advanced backend crate exposes wgpu surface-target interop explicitly for
native hosts. It owns devices, surfaces and vertex buffers, but no event loop,
component state or application data. Linux uses Vulkan, Windows DX12 and
macOS Metal. Hardware integrated/discrete adapters and an sRGB surface are required;
there is no software, GLES or X11 fallback. This is not a callback-safe API.

Solid rectangles, retained 8-bit coverage masks for CPU-shaped text and shapes, and
premultiplied sRGB color images are drawn with straight-alpha linear colors and
tints, clipped UV coordinates and retained geometry. Mask and image textures live
in device-wide caches shared by every surface made from one renderer, whatever
their surface formats; they are found by the resources' content keys, confirmed by
equality, so equal content from different allocations or windows on one device is
uploaded once. A scene's unique coverage is limited to 64 MiB, its unique image
data to 256 MiB (`ResourceKind::scene_budget`), and each resource and surface to
the hardware adapter's own texture dimension. Exceeding a budget or device limit
is a typed `RenderError::Limit`, distinct from capability and GPU failures, and
leaves the renderer usable. Images sample linearly or by nearest pixel,
clamped at their edges, from an sRGB texture that decodes to linear light before
filtering. Failed preparation invalidates
the scene cache so a previous scene can be retried safely. A renderer without a
nonzero extent reports `Frame::Occluded` instead of touching its surface. Native
window management and bounded recovery scheduling belong to the host; the
`Renderer` rustdoc shows the `Frame`/`Notice` recovery loop of a direct host.
Shader source and MIT/Apache-2.0 licenses are included in the package. See
`Renderer` rustdoc for owned-target lifetime, zero-size and error callback
requirements.

Vector commands preserve curves and stroke styles until backend preparation. This
adapter currently uses the shared `VectorRasterizer` coverage fallback, retaining
only geometry used by the last successful scene. CPU fallback limits and failures
are explicit; no GPU-native path speedup is claimed.

Unchanged command slots reuse encoded quads. Only changed byte ranges are uploaded
into a retained vertex buffer; insertions and reordering may invalidate later slots,
and viewport changes invalidate all geometry. Capacity grows geometrically and is
retained for smaller/empty scenes. Comparison, composition and byte assembly remain
linear in the scene size; this is not a retained GPU subtree/layer compositor.
