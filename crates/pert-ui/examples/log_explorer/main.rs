//! A log explorer over hundreds of thousands of lines.
//!
//! It shows what a large-data application needs from the library, written
//! only against the public API:
//!
//! - every collection given a bounded height (fixed tabs and regions, split
//!   panes), so tables and the tree mount only the rows they show;
//! - row identities (`LineId`, the line number) independent of where a line
//!   is stored, with a reverse index for `TableItems::index_of`;
//! - loading on a worker thread in batches through a `Mailbox`, with typed
//!   failures, skipped malformed lines and stale batches of an earlier load
//!   dropped;
//! - two tables, each with its own sort, a filter field over every column,
//!   level and source filters, and a tree of sources;
//! - appended lines inserted where they belong instead of rebuilding the
//!   orders;
//! - a live tail that starts only once a file has loaded, takes identities
//!   from one allocator shared by every worker, stops by generation, and
//!   counts the lines a full mailbox refused or a stopped worker left queued
//!   as a reported gap; and follow, which reveals the last row the table
//!   shows after every filter, the filter field included, and observes the
//!   reveal's receipt;
//! - an Errors view that can be detached into its own window;
//! - "Copy lines" bound per table view, so the copy chord copies the table
//!   that holds focus, through `Runtime::set_clipboard_text`;
//! - a detail pane whose text can be selected and copied
//!   (`SelectableText`).
//!
//! Without arguments the example runs a measured headless session over a
//! generated log and checks its results, which needs no window:
//! `cargo run -p pert-ui --example log_explorer --no-default-features`.
//! `--lines N` sets the generated log's length. With the `native` feature,
//! `--window` opens it.
mod app;
mod model;
mod session;

fn lines() -> Result<u64, Box<dyn std::error::Error>> {
    match std::env::args()
        .skip_while(|argument| argument != "--lines")
        .nth(1)
    {
        Some(lines) => Ok(lines.parse()?),
        None => Ok(50_000),
    }
}

#[cfg(feature = "native")]
fn window(path: std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    App::new("Log explorer", move |ui| {
        let (explorer, _) = app::build(ui, path.clone()).map_err(|error| {
            pert_ui::ApplicationError::new(std::io::Error::other(error.to_string()))
        })?;
        // The application state keeps it until the application ends.
        Ok(explorer)
    })
    .size(Size::from([1400, 900]))?
    .run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path =
        std::env::temp_dir().join(format!("pert-ui-log-explorer-{}.log", std::process::id()));
    model::generate(&path, lines()?)?;
    let result = if std::env::args().any(|argument| argument == "--window") {
        #[cfg(feature = "native")]
        {
            window(path.clone())
        }
        #[cfg(not(feature = "native"))]
        {
            Err("--window needs the `native` feature".into())
        }
    } else {
        session::run(path.clone(), lines()?)
    };
    std::fs::remove_file(&path)?;
    result
}
