//! Headless screenshots of the gallery, rendered on the CPU.
//!
//! `--screenshot [--scale N] [--palette violet] PAGE TARGET PATH` mounts the gallery in a headless runtime,
//! shows `PAGE` (a page key such as `tool-windows`, or `-` for the first page),
//! at scale `N` (physical pixels per logical pixel, default 1), asks for a capture of `TARGET` through the public capture contract and writes
//! the PNG to `PATH`. The `cpu` renderer answers the request, so this needs no
//! display or GPU. Targets:
//!
//! - `window`: the whole window;
//! - `region:X,Y,WIDTH,HEIGHT`: a rectangle of it in logical pixels;
//! - `path:KEY/KEY/…`: the component at that addressable key path;
//! - `key:KEY`: the one shown component with that key, found through inspection.
use pert_ui::cpu::CpuRenderer;
use pert_ui::{
    CaptureRequest, CaptureTarget, Color, InspectionQuery, OpaqueColor, Palette, Runtime, Selector,
    ThemeMode, Viewport,
};
use std::{error::Error, path::Path, time::Duration};

/// The logical window size of a screenshot run.
const WINDOW: (f32, f32) = (1280.0, 800.0);

/// Palette chosen for a screenshot run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScreenshotPalette {
    BuiltIn,
    Violet,
}

/// Parses `TARGET` against the mounted gallery.
pub(crate) fn target(ui: &Runtime, text: &str) -> Result<CaptureTarget, String> {
    let window = ui.primary_window().id();
    if text == "window" {
        return Ok(CaptureTarget::Window(window));
    }
    if let Some(rect) = text.strip_prefix("region:") {
        let values: Vec<f32> = rect
            .split(',')
            .map(|value| value.trim().parse::<f32>())
            .collect::<Result<_, _>>()
            .map_err(|error| format!("region {rect:?}: {error}"))?;
        let [x, y, width, height] = values[..] else {
            return Err(format!("region {rect:?} needs X,Y,WIDTH,HEIGHT"));
        };
        return Ok(CaptureTarget::Region {
            window,
            x,
            y,
            width,
            height,
        });
    }
    if let Some(path) = text.strip_prefix("path:") {
        return Ok(CaptureTarget::Path(
            path.split('/').map(str::to_owned).collect(),
        ));
    }
    if let Some(key) = text.strip_prefix("key:") {
        // Inspection names the component; a key shown more than once is
        // ambiguous rather than resolved to the first match.
        let shown: Vec<u64> = ui
            .query(&InspectionQuery::components(Selector::key(key)))
            .components
            .into_iter()
            .filter(|component| component.visible)
            .map(|component| component.id)
            .collect();
        return match shown[..] {
            [id] => Ok(CaptureTarget::Component(id)),
            [] => Err(format!("no shown component is keyed {key}")),
            _ => Err(format!(
                "{} shown components are keyed {key}; use path:",
                shown.len()
            )),
        };
    }
    Err(format!(
        "unknown target {text:?}: use window, region:X,Y,W,H, path:A/B or key:KEY"
    ))
}

/// Runs `--screenshot`: returns the line describing what was written.
pub(crate) fn run(
    page: &str,
    target_text: &str,
    path: &Path,
    scale: f64,
    palette: ScreenshotPalette,
) -> Result<String, Box<dyn Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(WINDOW.0, WINDOW.1, scale)?)?;
    if palette == ScreenshotPalette::Violet {
        let [red, green, blue] = crate::pages::tools::VIOLET_SEED;
        let seed = OpaqueColor::try_from(Color::rgba(red, green, blue, 1.0)?)?;
        ui.theme().set_palette(
            ThemeMode::Dark,
            Palette::from_seeds(ThemeMode::Dark, seed, None),
        )?;
    }
    let root = crate::gallery::Gallery::new()?.mount(&mut ui)?;
    ui.flush(Duration::ZERO)?;
    if page != "-" {
        let target = crate::interaction_check::page(&ui, &root, page)?;
        let outcome = ui.reveal(&target)?;
        if !outcome.is_revealed() {
            return Err(format!("page {page} could not be shown: {outcome:?}").into());
        }
    }
    // Let layout, text and deferred work settle before the frame is taken.
    for step in 1..=4 {
        ui.flush(Duration::from_millis(100 * step))?;
    }
    ui.set_capture_support(true);
    let target = target(&ui, target_text)?;
    let pending = ui.request_capture(CaptureRequest::new(target));
    ui.flush(ui.now())?;
    CpuRenderer::new().serve_captures(&mut ui);
    let capture = pending.wait()?.value?;
    std::fs::write(path, capture.image.encode_png())?;
    let [x, y, width, height] = capture.region;
    Ok(format!(
        "wrote {} ({}×{} pixels) of region {x},{y} {width}×{height}{}",
        path.display(),
        capture.image.width(),
        capture.image.height(),
        if capture.clipped { ", clipped" } else { "" }
    ))
}
