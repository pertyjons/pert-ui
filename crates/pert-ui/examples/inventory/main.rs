//! An inventory editor: a table edited in place like a spreadsheet.
//!
//! It shows what a data-entry application needs from the library, written
//! only against the public API:
//!
//! - cells edited in place through `Table::cell_editor`: Enter, F2, a
//!   double-click or typing begins an edit, Enter and Escape end it, Tab and
//!   Shift+Tab move to the next or previous editable cell, and a refused
//!   value stays open for correction;
//! - one editor per column, built from a `CellEdit` with its own state owner,
//!   so a `Select` needs no wrapper and the editor's signals live only as long
//!   as the edit;
//! - a checkbox column at rest made with `Checkbox::controlled`, which shows
//!   the item's value from the cell's description, proposes each toggle to
//!   the store and follows undo at the next flush;
//! - the table as one Tab stop (`Table::cell_tab_stops`), with F2 reaching
//!   the checkbox;
//! - an application-owned store that checks a group of edits in order
//!   before any of it applies, with undo and redo, and row copy and paste
//!   as tab-separated text that is refused whole when a field is invalid;
//! - row commands that act on, and are enabled by, the selected rows every
//!   filter leaves shown, the table's search included;
//! - commands registered once and shared by shortcuts and a toolbar of icon
//!   buttons (`Button::command_icon`) whose help names the shortcut
//!   (`Tooltip::command`), and inline fields and filters in the toolbar.
//!
//! Without arguments the example runs a headless session over generated
//! items and checks its results, which needs no window:
//! `cargo run -p pert-ui --example inventory --no-default-features`.
//! `--items N` sets the number of items. With the `native` feature,
//! `--window` opens it.
mod app;
mod model;
mod session;

fn items() -> Result<usize, Box<dyn std::error::Error>> {
    match std::env::args()
        .skip_while(|argument| argument != "--items")
        .nth(1)
    {
        Some(items) => Ok(items.parse()?),
        None => Ok(10_000),
    }
}

#[cfg(feature = "native")]
fn window(items: usize) -> Result<(), Box<dyn std::error::Error>> {
    use pert_ui::prelude::*;
    App::new("Inventory", move |ui| {
        let (inventory, commands, _) = app::build(ui, items).map_err(|error| {
            pert_ui::ApplicationError::new(std::io::Error::other(error.to_string()))
        })?;
        ui.keep_alive((inventory, commands));
        Ok(())
    })
    .size(Size::from([1200, 800]))?
    .run()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let items = items()?;
    if std::env::args().any(|argument| argument == "--window") {
        #[cfg(feature = "native")]
        {
            window(items)
        }
        #[cfg(not(feature = "native"))]
        {
            Err("--window needs the `native` feature".into())
        }
    } else {
        session::run(items)
    }
}
