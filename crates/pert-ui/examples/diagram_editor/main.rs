//! A diagram editor built around a custom canvas widget.
//!
//! The canvas (`canvas.rs`) is a custom container written only against the
//! public extension points: one child component per node, arranged by the
//! canvas and presented through `Component::child_transform` for pan and
//! zoom, stacked children whose priority keeps document order, edges painted
//! below them and selection and gesture feedback above. It shows the
//! mechanics a widget that is dragged and clicked needs:
//!
//! - move handling that converges on the pointer move the runtime repeats
//!   after each layout (`PointerEvent::is_refresh`);
//! - double clicks counted by a `ClickCounter` the canvas resets itself;
//! - a pan owned by the button that started it;
//! - Space held for panning through `Runtime::register_held_input`, taken
//!   only while the canvas has focus;
//! - renaming in place with a retained `DraftEditor` that opens with its
//!   text selected (`DraftSelection::All`);
//! - typed `Semantics` for the canvas and its nodes.
//!
//! Around it (`app.rs`, `editor.rs`): an application-owned document and undo
//! history (`model.rs`), one `ListSelection` shared by the canvas and the
//! outline, an inspector whose fields stay bound to a view model synchronized
//! in event phases so a draft survives unrelated changes, a `ColorPicker`
//! whose `on_change` records each picked fill as one undo step, commands in a
//! menu bar, a context menu, a toolbar and a command palette, files opened
//! through a `FileBrowser` dialog and saved on background threads, work a
//! closing dialog starts queued with `EventContext::defer`, a saved
//! tool-window layout, close admission that also counts the drafts the
//! inspector's fields keep, through a `FormFields` they report to, and, with
//! the `cpu` feature, PNG export through a second runtime in the light theme.
//!
//! Without arguments the example runs a deterministic headless session and
//! checks its results, which needs no window:
//! `cargo run -p pert-ui --example diagram_editor --no-default-features --features cpu`.
//! Its close checks start the application with `App::headless`, so they need
//! the `native` feature, which the default features include.
//! With the `native` feature, `--window` opens it; files go to the folder
//! given with `--dir`, by default `pert-ui-diagram-editor` in the system's
//! temporary folder.
mod app;
mod canvas;
mod editor;
mod model;
mod session;

#[cfg(feature = "native")]
fn folder() -> std::path::PathBuf {
    std::env::args()
        .skip_while(|argument| argument != "--dir")
        .nth(1)
        .map_or_else(
            || std::env::temp_dir().join("pert-ui-diagram-editor"),
            std::path::PathBuf::from,
        )
}

/// The application, saving its files in `dir`: the window runs it, and the
/// session's close checks start it headless.
#[cfg(feature = "native")]
fn application(
    dir: std::path::PathBuf,
) -> Result<pert_ui::App<std::rc::Rc<app::DiagramApp>>, Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    std::fs::create_dir_all(&dir)?;
    // The application the initializer builds is the application state: close
    // admission and error handling receive it, and it lives until the
    // application ends. A failing initializer reaches neither; its error
    // returns from `run`.
    Ok(App::new("Diagram editor", move |ui| {
        Ok(app::build(ui, dir.clone()).map_err(|error| {
            pert_ui::ApplicationError::new(std::io::Error::other(error.to_string()))
        })?)
    })
    .size(Size::from([1200, 800]))?
    // A component failure is shown on the status line and the window keeps
    // running.
    .on_error(|_, diagram, error| {
        match diagram
            .editor
            .status
            .set(format!("Something went wrong: {error}"))
        {
            Ok(()) => ErrorAction::Retry,
            Err(_) => ErrorAction::Exit,
        }
    })
    .on_close(|ui, diagram, _| {
        Ok(match app::close_requested(diagram, ui)? {
            None => NativeCloseAction::Close,
            Some(decision) => NativeCloseAction::Defer(decision),
        })
    }))
}

#[cfg(feature = "native")]
fn window() -> Result<(), Box<dyn std::error::Error>> {
    application(folder())?.run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|argument| argument == "--window") {
        #[cfg(feature = "native")]
        return window();
        #[cfg(not(feature = "native"))]
        return Err("--window needs the `native` feature".into());
    }
    session::run(
        std::env::temp_dir().join(format!("pert-ui-diagram-session-{}", std::process::id())),
    )
}
