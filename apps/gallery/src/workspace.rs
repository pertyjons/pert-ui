//! Translates addresses from the gallery's original category strips at its import boundary.
use crate::catalogue::CATEGORIES;
use pert_ui::{ViewState, Workspace, WorkspaceError, WorkspacePath, WorkspaceValue};
use std::collections::BTreeMap;

const TABS: &[&str] = &[
    "gallery",
    "command-palette",
    "about-dialog",
    "shortcuts-dialog",
    "guide-dialog",
    "gallery-body",
    "center",
    "gallery-tabs",
];

fn old_home(key: &str) -> Option<&'static str> {
    Some(match key {
        "layout-category" => "layout-nav",
        "controls-category" => "controls-forms",
        "text-category" => "text-graphics",
        "surfaces-category" => "menus-windows",
        "data-category" => "lists-files",
        "tools-category" => "runtime-tools",
        _ => return None,
    })
}

fn original_page(category: &str, page: &str) -> bool {
    matches!(
        (category, page),
        (
            "controls-category",
            "chips"
                | "colors"
                | "controls"
                | "families"
                | "fields"
                | "forms"
                | "input"
                | "progress"
                | "properties"
                | "text"
                | "toolbar"
        ) | (
            "data-category",
            "collections"
                | "dragdrop"
                | "files"
                | "images"
                | "live"
                | "meters"
                | "shapes"
                | "virtual"
        ) | (
            "layout-category",
            "layout"
                | "navigation"
                | "scroll"
                | "sections"
                | "shells"
                | "splits"
                | "state"
                | "tool-windows"
                | "transforms"
        ) | ("surfaces-category", "menus" | "surfaces" | "windows")
            | (
                "text-category",
                "editing" | "fonts" | "numbers" | "text-rendering"
            )
            | (
                "tools-category",
                "gradients" | "overlay" | "profiling" | "theme" | "workspace"
            )
    )
}

fn home(key: &str) -> Option<&'static str> {
    CATEGORIES
        .iter()
        .find(|c| c.pages.iter().any(|p| p.key == key))
        .map(|c| c.key)
}

fn selection(state: &ViewState) -> Option<&str> {
    (state.kind() == "tabs")
        .then(|| state.get("selected")?.as_text())
        .flatten()
}

/// Returns the validated document and whether it used the original catalogue.
pub(crate) fn decode(text: &str) -> Result<(Workspace, bool), WorkspaceError> {
    let document = Workspace::decode(text)?;
    let active = document.entries().find_map(|(path, state)| {
        path.segments()
            .eq(TABS.iter().copied())
            .then(|| selection(state))
            .flatten()
    });
    let mut selected_tabs = BTreeMap::new();
    let mut active_home = active.and_then(old_home);
    let mut entries = Vec::new();
    let mut migrated = active_home.is_some();
    for (path, state) in document.entries() {
        let mut segments: Vec<_> = path.segments().collect();
        let tail = segments.strip_prefix(TABS);
        match tail {
            Some([category, "category-pages"]) if old_home(category).is_some() => {
                migrated = true;
                if state.kind() != "tabs"
                    || state.expect_only(&["selected"]).is_err()
                    || state.get("selected").is_some()
                        && selection(state).is_none_or(|page| !original_page(category, page))
                {
                    return Err(WorkspaceError::Malformed(format!(
                        "invalid previous gallery tab selection at {path}"
                    )));
                }
                let default = old_home(category);
                let destination = selection(state).and_then(home).or(default);
                if let Some(destination) = destination {
                    // Two old strips can converge. Prefer the visible selection,
                    // then the strip whose family still owns the destination.
                    let visible = Some(*category) == active;
                    let priority = (visible, Some(destination) == default);
                    if visible {
                        active_home = Some(destination);
                    }
                    if selected_tabs
                        .get(destination)
                        .is_none_or(|(old, _)| priority > *old)
                    {
                        selected_tabs.insert(destination, (priority, state.clone()));
                    }
                }
                continue;
            }
            Some([category, "category-pages", page, rest @ ..]) if old_home(category).is_some() => {
                migrated = true;
                if !original_page(category, page) {
                    return Err(WorkspaceError::Malformed(format!(
                        "unknown previous gallery page at {path}"
                    )));
                }
                let page = match (*page, rest) {
                    ("files", ["page", "sections", "dialog-section", ..]) => "file-dialog",
                    ("text", ["page", "sections", "buttons", ..]) => "buttons",
                    ("editing", ["page", "sections", "sign-in-example", ..]) => "sign-in",
                    ("chips", ["page", "sections", "breadcrumb-section", ..]) => "breadcrumbs",
                    ("controls", ["page", "sections", "surfaces", ..]) => "popups",
                    ("input", ["page", "sections", "keyboard", ..]) => "keyboard",
                    (key, _) => key,
                };
                if let Some(group) = home(page) {
                    segments[TABS.len()] = group;
                    segments[TABS.len() + 2] = page;
                    if page == "file-dialog" {
                        let at = TABS.len() + 7;
                        if segments.get(at) == Some(&"file-dialog") {
                            segments[at] = "file-dialog-panel";
                        }
                    }
                }
            }
            Some([category, ..]) if old_home(category).is_some() => {
                if let Some(group) = old_home(category) {
                    migrated = true;
                    segments[TABS.len()] = group;
                }
            }
            _ => {}
        }
        entries.push((WorkspacePath::new(segments)?, state.clone()));
    }
    if !migrated {
        return Ok((document, false));
    }
    for (group, (_, state)) in selected_tabs {
        entries.push((
            WorkspacePath::new(TABS.iter().copied().chain([group, "category-pages"]))?,
            state,
        ));
    }
    if let Some(group) = active_home
        && let Some((_, state)) = entries
            .iter_mut()
            .find(|(p, _)| p.segments().eq(TABS.iter().copied()))
    {
        *state = state
            .clone()
            .with("selected", WorkspaceValue::text(group)?)?;
    }
    Ok((Workspace::new(*document.settings(), entries)?, true))
}
