# pert-ui

The public entry point for pert-ui, an opinionated Rust GUI library built from
scratch for demanding desktop applications. It draws its own widgets through a
retained runtime: a component recomputes only when a binding it read changes,
unchanged widgets keep their layout, text and drawing, and an idle application
sleeps until there is work. Applications choose content and meaning; they supply
no renderer, event loop, repaint timer or cache bookkeeping.

The library is **unreleased**, and its API changes between minor versions.

```rust,no_run
use pert_ui::prelude::*;

fn main() -> Result<(), AppError> {
    App::new("Hello", |ui| {
        let count = ui.signal(0_u32);
        let caption = count.read_only().map(|n| format!("Clicked {n} times"))?;
        let clicks = count.clone();
        ui.mount_tree(Element::new("content", Column::new([
            Element::new("caption", Label::new(caption)),
            Element::new("click", Button::new("Click", move |_| {
                clicks.set(clicks.get()? + 1).map(|_| ())
            })),
        ])))?;
        Ok(())
    })
    .run()
}
```

The library offers:

- retained layout with flow containers, adaptive forms, region layouts, splits,
  tabs, floating in-view windows and experimental native multi-window support;
- standard, inline and toolbar controls, text and numeric editors, menus,
  dialogs, notifications and progress indicators;
- UI-local signals, host-owned parameters with preview, commit and cancel, and
  lossy live-data sources for continuously changing visualizations;
- virtualized lists, tables and trees, zoomable views and drag and drop;
- themes, density, application zoom, bundled fonts and icons, images and SVG;
- a versioned workspace document, and owned inspection snapshots, screenshots
  and profiles for the application's own tooling.

Start with the [guide](https://docs.rs/pert-ui/latest/pert_ui/guide/index.html)
and its
[getting started](https://docs.rs/pert-ui/latest/pert_ui/guide/getting_started/index.html)
chapter; the [crate documentation](https://docs.rs/pert-ui) maps every area to
its types. The [repository](https://github.com/pertyjons/pert-ui) holds the
gallery, the changelog and the design notes.

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
| `desktop` | yes | `App`: the native window, event loop, hardware renderer and clipboard (winit, wgpu, arboard). Without it the crate is the backend-independent runtime, for headless use or a custom host. |
| `png` | no | PNG decoding in the `images` module. |
| `jpeg` | no | JPEG decoding, with its EXIF orientation applied. |
| `gif`, `webp` | no | GIF and WebP decoding, first frame only. |
| `bmp`, `ico`, `tiff`, `pnm`, `qoi` | no | BMP, ICO, TIFF, PBM/PGM/PPM/PAM and QOI decoding. |
| `image-formats` | no | Every format feature above. |
| `images` | no | The `images` module without any decoder; every format feature implies it. |
| `svg` | no | The `svg` module: SVG icons and graphics rasterized with resvg. SVG text is not rendered. |
| `cpu` | no | The `cpu` module: rendering scenes and serving capture requests on the CPU, with no window or GPU; with `desktop`, `App::software_rendering` presents CPU frames in native windows. |
| `serde` | no | `Serialize`/`Deserialize` for inspection snapshots, profiles, validation reports and workspace documents, and everything they carry. |

Drawing images needs no feature: `RasterImage::from_srgba8` accepts pixels an
application decoded or generated itself.

# Minimum Rust version and license

Requires Rust 1.99.0 or later, edition 2024. Licensed under either the MIT
license or the Apache License 2.0, at your option; both texts are included in
this package. The bundled fonts keep their own licenses (SIL OFL 1.1 for Noto
Sans and Noto Sans Mono, Apache 2.0 for Material Icons), shipped with the
`pert-ui-runtime` package.
