//! A bookmark list: a form that adds bookmarks, a filter by name, a status
//! line and a Remove button per bookmark.
//!
//! [`build`] mounts the whole user interface into a [`Runtime`], so the
//! native window (`main.rs`) and a headless test mount the same content.
use pert_ui::prelude::*;
use std::{cell::Cell, rc::Rc};

/// One saved link. `id` identifies the bookmark for its whole life and keys
/// its row, so two bookmarks with the same name stay distinct.
#[derive(Clone, Debug, PartialEq)]
struct Bookmark {
    id: u64,
    name: String,
    url: String,
}

/// The bookmarks the application starts with.
fn initial() -> Vec<Bookmark> {
    vec![
        Bookmark {
            id: 1,
            name: "Rust".to_owned(),
            url: "https://www.rust-lang.org".to_owned(),
        },
        Bookmark {
            id: 2,
            name: "Crates".to_owned(),
            url: "https://crates.io".to_owned(),
        },
    ]
}

/// Checks a new bookmark's fields, returning the message to show on failure.
fn validate(name: &str, url: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        Err("Enter a name.")
    } else if !(url.starts_with("http://") || url.starts_with("https://")) {
        Err("The URL must start with http:// or https://.")
    } else {
        Ok(())
    }
}

/// Whether `bookmark` passes `filter`: a case-insensitive substring of its
/// name. An empty (or blank) filter passes everything.
fn matches(bookmark: &Bookmark, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty() || bookmark.name.to_lowercase().contains(&filter)
}

/// Mounts the bookmark list into `ui` and registers its Ctrl+L (Cmd+L on
/// macOS) "Clear filter" command. Returns the root component.
///
/// # Errors
/// Returns the runtime's errors from creating bindings, registering the
/// command or mounting the tree.
pub fn build(ui: &mut Runtime) -> Result<WidgetId, RuntimeError> {
    let bookmarks = ui.signal(initial());
    let next_id = Rc::new(Cell::new(3));
    let name = ui.signal(String::new());
    let url = ui.signal(String::new());
    let error = ui.signal(String::new());
    let filter = ui.signal(String::new());

    let visible = bookmarks.combine(&filter, |all, filter| {
        all.iter()
            .filter(|bookmark| matches(bookmark, filter))
            .cloned()
            .collect::<Vec<_>>()
    })?;
    let status = bookmarks.combine(&filter, |all, filter| {
        if filter.trim().is_empty() {
            format!("{} bookmarks", all.len())
        } else {
            let shown = all.iter().filter(|b| matches(b, filter)).count();
            format!("{shown} of {} bookmarks", all.len())
        }
    })?;

    let add = {
        let (list, name, url, error) =
            (bookmarks.clone(), name.clone(), url.clone(), error.clone());
        Button::new("Add", move |_| {
            let (entered_name, entered_url) = (name.get()?, url.get()?);
            if let Err(message) = validate(&entered_name, &entered_url) {
                return error.set(message.to_owned());
            }
            let id = next_id.replace(next_id.get() + 1);
            list.update(|list| {
                list.push(Bookmark {
                    id,
                    name: entered_name.trim().to_owned(),
                    url: entered_url,
                });
            })?;
            name.set(String::new())?;
            url.set(String::new())?;
            error.set(String::new())
        })
    };

    let remove_from = bookmarks.clone();
    let rows = Each::new(
        &visible,
        |bookmark| bookmark.id.to_string(),
        move |_, bookmark| {
            let (list, id) = (remove_from.clone(), bookmark.id);
            Row::new([
                Label::new(bookmark.name.as_str()).key("name"),
                Label::new(bookmark.url.as_str()).key("url"),
                Button::new("Remove", move |_| {
                    list.update(|list| list.retain(|bookmark| bookmark.id != id))
                })
                .key("remove"),
            ])
            .key("row")
        },
    );

    // Registered for the application's lifetime, so its identity is not kept.
    let clear = filter.clone();
    let _ = ui.register_command(
        "Clear filter",
        CommandScope::Application,
        vec![KeyChord::primary('l')],
        move |_, _| clear.set(String::new()),
    )?;

    let content = Column::new([
        Form::new([
            Field::new(TextField::new(&name).key("name"))
                .label("Name")
                .key("name-field"),
            Field::new(TextField::new(&url).placeholder("https://").key("url"))
                .label("URL")
                .key("url-field"),
        ])
        .key("form"),
        add.key("add"),
        Label::new(error.read_only()).key("error"),
        Field::new(
            TextField::new(&filter)
                .placeholder("Filter by name")
                .key("filter"),
        )
        .label("Filter")
        .key("filter-field"),
        Label::new(status).key("status"),
        rows.key("list"),
    ]);
    ui.mount_tree(
        ScrollArea::new(content.key("content"))
            .axes(ScrollAxes::Both)
            .key("bookmarks"),
    )
}
