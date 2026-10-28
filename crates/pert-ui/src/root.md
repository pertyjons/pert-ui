The public entry point for pert-ui, an opinionated Rust GUI library built from
scratch for demanding desktop applications. It draws its own widgets through a
retained runtime: a component recomputes only when a binding it read changes,
unchanged widgets keep their layout, text and drawing, and an idle application
sleeps until there is work. Applications choose content and meaning; they supply
no renderer, event loop, repaint timer or cache bookkeeping.

The library is **unreleased**, and its API changes between minor versions.

# A first window

With the default `desktop` feature, [`App`] opens a native window, runs the
event loop and schedules presentation. Its initializer mounts content once:

```no_run
# #[cfg(feature = "native")]
fn main() -> Result<(), pert_ui::AppError> {
    use pert_ui::prelude::*;
    App::new("Hello", |ui| {
        let count = ui.signal(0_u32);
        let caption = count.map(|n| format!("Clicked {n} times"))?;
        let clicks = count.clone();
        ui.mount_tree(Element::new("content", Column::new([
            Element::new("caption", Label::new(caption)),
            Element::new("click", Button::new("Click", move |_| {
                clicks.update(|n| *n += 1)
            })),
        ])))?;
        Ok(())
    })
    .run()
}
# #[cfg(not(feature = "native"))]
# fn main() {}
```

Continue with the [guide](crate::guide), starting at
[getting started](crate::guide::getting_started). Everything the guide and this
page name is exported from the crate root. Application code globs
[`prelude`], which holds what an application's first program writes: the
built-in components with their configuration types and the names an
application uses everywhere. Code that implements [`Component`] also globs
[`component`], a headless test globs [`testing`], and a custom host that
drives the runtime in place of [`App`] imports [`host`]. The thematic modules
gather the rest by need: [`parameter`] for authoritative values, [`commands`]
for commands and the command palette, [`windowing`] for windows, regions,
splits and tool windows, [`inspection`] for inspection, capture and
profiling, and [`live`] for data from other threads. The prelude's page says
where every other name is.

Most items are re-exported from the backend-independent `pert-ui-runtime`
crate, which documents them. Their examples import `pert_ui::…`, as an
application writes them: an application depends on `pert-ui` alone, every such
item is at this crate's root, `pert_ui::prelude` is
`pert_ui_runtime::prelude` plus native startup and the facade's own
feature widgets (`Svg` with `svg`), and `pert_ui::component` and the thematic
modules re-export their `pert_ui_runtime` namesakes.

# Map of the library

| Area | Start with | Guide |
|---|---|---|
| Native application | [`App`], [`Runtime`], [`Element`], [`Component`] | [getting started](crate::guide::getting_started) |
| Local state and derived values | [`Signal`], [`ReadSignal`], [`UpdateContext::read`] | [state](crate::guide::state) |
| Parameters and host edits | [`ParameterSpec`], [`Runtime::parameter`], [`ParameterHost`], [`Runtime::remote_parameter`] | [authoritative values](crate::guide::authoritative_values) |
| Worker results and live data | [`Runtime::mailbox`], [`SeriesSource`], [`Telemetry`], [`series_bridge`], [`LevelMeter`], [`Trace`] | [state](crate::guide::state) |
| Layout and views | [`Column`], [`Row`], [`Element::with_grow`], [`Justify`], [`Align`], [`Grid`], [`Form`], [`View`], [`ScrollArea`], [`RegionLayout`], [`SplitPane`], [`Tabs`] | [layout and views](crate::guide::layout) |
| Windows and workspaces | [`FloatingWindow`], [`Runtime::request_detach`], [`NativeWindowOptions`], [`Workspace`] | [layout and views](crate::guide::layout) |
| Children from values | [`Dynamic`], [`Show`], [`Each`] | [layout and views](crate::guide::layout) |
| Large collections | [`VirtualList`], [`Table`], [`Tree`] | [layout and views](crate::guide::layout) |
| Zoomable content | [`ZoomView`], [`AxisMapping`], [`Component::child_transform`] | [layout and views](crate::guide::layout) |
| Drag and drop | [`EventContext::start_drag`], [`Component::drag_event`], [`DragPayload`] | [layout and views](crate::guide::layout) |
| Controls | [`Button`], [`Checkbox`], [`Toggle`], [`RadioGroup`], [`SegmentedControl`], [`Slider`], [`IntegerInput`], [`DecimalInput`], [`ProgressBar`], [`Spinner`] | [controls](crate::guide::controls) |
| Text editing | [`TextField`], [`TextArea`], [`DraftEditor`] (in-place editing inside a custom control) | [controls](crate::guide::controls) |
| Toolbars and inline controls | [`Toolbar`], [`InlineSlider`], [`InlineToggle`], [`InlineSelect`] | [controls](crate::guide::controls) |
| Menus, dialogs and notifications | [`Menu`], [`MenuBar`], [`ContextMenu`], [`Select`], [`Popover`], [`Tooltip`], [`Dialog`], [`ToastHost`], [`Runtime::register_command`] | [surfaces](crate::guide::surfaces) |
| Themes, typography and graphics | [`ThemeManager`], [`UiSettings`], [`FontManager`], [`RichText`], [`Icon`], [`Painter`], [`Image`], [`RasterImage`] | [appearance](crate::guide::appearance) |
| Image files and SVG | [`images`] (feature `images` or a format feature), [`svg_images`] (feature `svg`) | [appearance](crate::guide::appearance) |
| Keyboard, inspection and profiling | [`Runtime::command_key_input`], [`Runtime::key_input`], [`Runtime::inspect`], [`InspectionService`], [`Runtime::request_capture`], [`Runtime::set_profiling`] | [diagnostics](crate::guide::diagnostics) |
| Headless tests | [`testing`]: [`testing::Driver`] clicks, types, presses keys and settles; [`HeadlessApp`] (feature `native`) runs an `App`'s close policy without a window and hands the test the state its initializer returned | [testing an application](crate::guide::testing) |
| Custom hosts | [`host`]: [`host::HostIntegration`] holds what a host that drives a [`Runtime`] in place of [`App`] calls, with a complete host loop | [diagnostics](crate::guide::diagnostics) |
| Rendering without a GPU | [`cpu`] (feature `cpu`): [`CpuRenderer`] renders frames and serves captures headlessly; `App::software_rendering` presents them in native windows | [diagnostics](crate::guide::diagnostics) |

Custom widgets implement [`Component`] and use the same public measurement,
binding, input and [`Painter`] contracts as the built-in widgets; the guide's
chapters each end with the extension points of their area.

<!-- Targets of the feature table's links in status.md; README.md defines the
same labels with docs.rs addresses. -->
[serde-snapshot]: crate::InspectionSnapshot
[serde-profile]: crate::ProfileSnapshot
[serde-validation]: crate::ValidationReport
[serde-workspace]: crate::Workspace
