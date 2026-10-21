# Platform status

The target platforms are macOS Tahoe on Apple Silicon (Metal), Windows 11 on
x86_64 (DirectX 12) and Linux on x86_64 with Wayland and Vulkan (Ubuntu 26.04
LTS and Fedora Workstation 44, GNOME and KDE sessions). Native execution has so
far been checked only on Fedora 44 in a GNOME Wayland session with Vulkan;
Windows, macOS and the other Linux sessions build and run the headless tests in
continuous integration but have no native evidence yet. The
[native evidence](https://github.com/pertyjons/pert-ui/blob/main/docs/verification.md#native-evidence)
records the support matrix and which checks have run where. Platform
accessibility adapters, X11 sessions and plugin hosting are outside the current
scope.

# Cargo features

| Feature | Default | Adds |
|---|---|---|
| `desktop` | yes | `App`: the native window, event loop, hardware renderer and clipboard (winit, wgpu, arboard). Without it, or `desktop-cpu`, the crate is the backend-independent runtime, for headless use or a custom host. |
| `desktop-cpu` | no | `App` presented by the CPU renderer through softbuffer, without compiling wgpu: smaller programs for tools and machines without a usable GPU. Use it with `default-features = false`; if `desktop` is also enabled, for example by another dependency, the GPU renderer is compiled and is the default. |
| `native`, `gpu` | no | The building blocks of `desktop` (`gpu`) and `desktop-cpu` (`native` with `cpu`); `native` alone has no renderer and does not compile. |
| `png` | no | PNG decoding in the `images` module. |
| `jpeg` | no | JPEG decoding, with its EXIF orientation applied. |
| `gif`, `webp` | no | GIF and WebP decoding, first frame only. |
| `bmp`, `ico`, `tiff`, `pnm`, `qoi` | no | BMP, ICO, TIFF, PBM/PGM/PPM/PAM and QOI decoding. |
| `image-formats` | no | Every format feature above. |
| `images` | no | The `images` module without any decoder; every format feature implies it. |
| `svg` | no | The `svg` module: SVG icons and graphics rasterized with resvg. SVG text is not rendered. |
| `cpu` | no | The `cpu` module: rendering scenes and serving capture requests on the CPU, with no window or GPU; with `desktop`, `App::software_rendering` presents CPU frames in native windows instead of the GPU's. |
| `serde` | no | `Serialize`/`Deserialize` for inspection snapshots, profiles, validation reports and workspace documents, and everything they carry. |

Drawing images needs no feature: `RasterImage::from_srgba8` accepts pixels an
application decoded or generated itself.

# Minimum Rust version and license

Requires Rust 1.99.1 or later, edition 2024. Licensed under either the MIT
license or the Apache License 2.0, at your option; both texts are included in
this package. The bundled fonts keep their own licenses (SIL OFL 1.1 for Noto
Sans and Noto Sans Mono, Apache 2.0 for Material Icons), shipped with the
`pert-ui-runtime` package.
