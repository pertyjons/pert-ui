//! The gallery's search: one picker over its content and its commands, each
//! row labelled by kind.
//!
//! The command that opens it lists the content with
//! [`Runtime::content_search`] and the commands with
//! [`Runtime::commands_for`], then opens a [`QuickPick`] the user filters by
//! typing. Choosing content jumps to it with [`Runtime::jump_to_content`] in a
//! task queued with [`EventContext::defer`], since a jump flushes and the
//! choose callback runs inside an event; the task runs after answering
//! `Pending` has closed the picker. Choosing a command invokes it.
use crate::{history::GalleryHistory, pages::layout::describe_outcome};
use pert_ui::commands::*;
use pert_ui::prelude::*;
use pert_ui::{ContentField, ContentJump, ContentQuery, ContentTarget};
use std::{cell::RefCell, rc::Rc};

/// The most content rows the search lists. The picker filters them as the
/// user types; content beyond them is not offered. Collections' rows that are
/// not mounted are left out: a 100,000-row table would fill the list.
const CONTENT_LIMIT: usize = 5_000;

/// What a row of the search leads to.
#[derive(Clone)]
pub(crate) enum SearchTarget {
    Content(ContentTarget),
    Command(CommandId, CommandContext),
}

/// The targets of the open search, by row position in its keys.
pub(crate) type SearchTargets = Rc<RefCell<Vec<SearchTarget>>>;

/// Lists the gallery's content and commands for the search, except the
/// command `own` that opens it. A content row's detail is the page it is on,
/// named by `places`' titles rather than by its key path.
pub(crate) fn search_items(
    ui: &mut Runtime,
    context: &CommandContext,
    own: Option<&CommandId>,
    places: &GalleryHistory,
    destinations: &std::collections::HashMap<CommandId, (&str, &crate::catalogue::Entry)>,
) -> Result<(Vec<PickItem>, Vec<SearchTarget>), RuntimeError> {
    let mut items = Vec::new();
    let mut targets = Vec::new();
    let query = ContentQuery::everything()
        .rows(false)
        .limit(CONTENT_LIMIT)
        .map_err(ApplicationError::new)?;
    let mut search = ui.content_search(query);
    while targets.len() < CONTENT_LIMIT {
        let step = search.step(ui).map_err(ApplicationError::new)?;
        for hit in step.hits {
            if targets.len() == CONTENT_LIMIT {
                break;
            }
            // A component with both a name and a value is listed as
            // "name: value", so typing either finds it.
            let label = match (hit.field, &hit.other) {
                (ContentField::Name, Some(value)) => format!("{}: {value}", hit.text),
                (ContentField::Value, Some(name)) => format!("{name}: {}", hit.text),
                // An item, such as a menu command, is listed under its owner.
                (ContentField::Item { .. }, Some(name)) => format!("{name}: {}", hit.text),
                _ => hit.text.clone(),
            };
            let mut item =
                PickItem::new(format!("content-{}", targets.len()), label).kind("Content");
            if let Some(location) = hit.path.as_deref().and_then(|path| places.location(path)) {
                item = item.detail(location);
            }
            items.push(item);
            targets.push(SearchTarget::Content(hit.target));
        }
        if step.done {
            break;
        }
    }
    for entry in ui.commands_for(context)? {
        if Some(&entry.id) == own {
            continue;
        }
        if let Some((category, page)) = destinations.get(&entry.id) {
            items.push(
                PickItem::new(format!("page-{}", targets.len()), page.title)
                    .detail(format!("{} · {}", category, page.components))
                    .kind("Page"),
            );
            targets.push(SearchTarget::Command(entry.id.clone(), context.clone()));
        }
        let item = PickItem::new(format!("command-{}", targets.len()), entry.name.clone())
            .kind("Command")
            .disabled(!matches!(entry.enabled, Ok(true)));
        items.push(item);
        targets.push(SearchTarget::Command(entry.id, context.clone()));
    }
    Ok((items, targets))
}

/// Wraps `body` in the search picker that `controller` opens.
pub(crate) fn search_picker(
    controller: QuickPickController,
    targets: SearchTargets,
    body: Element,
) -> Element {
    let picker = QuickPick::new(controller, body, move |cx, item| {
        let index = item
            .key
            .rsplit('-')
            .next()
            .and_then(|index| index.parse::<usize>().ok());
        let target = index.and_then(|index| targets.borrow().get(index).cloned());
        let Some(target) = target else {
            return Ok(PickChoice::Refused(
                "That result is no longer available.".to_owned(),
            ));
        };
        // Release picker modality before revealing content or dispatching a
        // command in the context in which the visitor opened the search.
        let outcome = cx.signal(None::<Result<(), String>>);
        let writer = outcome.clone();
        cx.defer(move |ui| {
            let result = match target {
                SearchTarget::Command(id, context) => match ui.invoke_command(&id, context) {
                    Ok(CommandOutcome::Invoked) => Ok(()),
                    Ok(other) => Err(format!("The command could not run: {other:?}")),
                    Err(error) => Err(error.to_string()),
                },
                SearchTarget::Content(target) => match ui.jump_to_content(&target) {
                    Ok(jump) => describe_jump(&jump),
                    Err(error) => Err(error.to_string()),
                },
            };
            writer.set(Some(result))
        })?;
        Ok(PickChoice::Pending(PickPending::new(
            outcome.read_only(),
            Clone::clone,
        )))
    })
    .matcher(PickMatcher::new(|query, item| {
        if item.kind.as_deref() == Some("Page") {
            let detail = item.detail.as_deref().unwrap_or_default();
            if item.label.eq_ignore_ascii_case(query)
                || detail
                    .split_whitespace()
                    .any(|alias| alias.eq_ignore_ascii_case(query))
            {
                return Some(u32::MAX);
            }
            return PickMatcher::fuzzy_score(query, &item.label)
                .or_else(|| PickMatcher::fuzzy_score(query, detail));
        }
        PickMatcher::fuzzy_score(query, &item.label)
    }))
    .name("Search the gallery")
    .list_name("Results")
    .placeholder("Search content and commands")
    // Its results are content search hits: a later search must not list
    // them as content again.
    .searchable(false);
    // Structural, so wrapping the gallery adds no segment to its workspace
    // paths and saved workspaces stay valid.
    Element::new("gallery-search", picker).structural()
}

/// A jump that revealed its target succeeds; anything else is the message
/// the picker shows.
fn describe_jump(jump: &ContentJump) -> Result<(), String> {
    match jump {
        ContentJump::Reveal(outcome) if outcome.is_revealed() => Ok(()),
        ContentJump::Reveal(outcome) => Err(describe_outcome(outcome)),
        ContentJump::Row(status) => Err(format!("The row could not be shown: {status:?}")),
        ContentJump::Pending(_) => Err("The row is still being shown; choose it again.".to_owned()),
        _ => Err("The result could not be shown.".to_owned()),
    }
}
