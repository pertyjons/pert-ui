//! Content search over mounted semantics and collection rows, and jumping to
//! a hit.
use super::*;
use crate::content_search::RowScan;
use crate::{
    ContentField, ContentHit, ContentJump, ContentQuery, ContentSearch, ContentSearchError,
    ContentStep, ContentTarget, ListRevealStatus, SemanticValue,
};

impl Runtime {
    /// Starts a resumable search of the application's content for `query`.
    ///
    /// The search looks at what components already report through their
    /// [`Semantics`](crate::Semantics): the resolved name (a component's own,
    /// its element's label, or the one a wrapping `Field` gives it) and the
    /// value text (text, a number, a choice's label). Hidden components are
    /// included unless the query excludes them, so content in an unselected
    /// tab or a collapsed section is found; [`Self::jump_to_content`] then
    /// opens the way to it. A collection that offers
    /// [`ContentRows`](crate::ContentRows) through
    /// [`Component::try_content_rows`] is also searched for rows it has not
    /// mounted; `VirtualList`, and so `Table` and `Tree`, do when their source
    /// answers `text`.
    ///
    /// Never found: a subtree excluded with
    /// [`Element::with_searchable`](crate::Element::with_searchable), the value
    /// of a [protected](crate::Semantics::protected) component, components a
    /// modal surface blocks, drawing without semantics, and rows a source does
    /// not describe. Nothing runs until [`ContentSearch::step`] is called; each
    /// step visits components and rows within the query's work budget and
    /// result limit. Beyond that budget a step also reads the revision of each
    /// collection the search has already entered, once, to detect a replaced
    /// source, so its cost grows with the number of collections entered.
    ///
    /// A component reports the values it has read. One that was mounted
    /// hidden and has never updated, such as a field on a page that has not
    /// been shown, reports its initial state until it is shown once; a
    /// collection offers its current bound rows even then and after a hidden
    /// source replacement. Obsolete retained row children are skipped until
    /// presentation catches up; searching does not update or mount them.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::{ContentQuery, ContentTarget, RevealOutcome, ContentJump};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    /// ui.mount_tree(Element::new("tabs", Tabs::new([
    ///     Tab::new("general", "General", Element::new("intro", Label::new("Welcome"))),
    ///     Tab::new("details", "Details", Element::new(
    ///         "report", Button::new("Open report", |_| Ok(())),
    ///     )),
    /// ])))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    ///
    /// // The button sits in the unselected tab and is still found.
    /// let step = ui.content_search(ContentQuery::new("report")?).step(&ui)?;
    /// assert!(step.done);
    /// let hit = &step.hits[0];
    /// assert_eq!(hit.text, "Open report");
    /// assert!(!hit.visible);
    /// let ContentJump::Reveal(RevealOutcome::RevealedAndFocused { .. }) =
    ///     ui.jump_to_content(&hit.target)?
    /// else {
    ///     panic!("the jump did not reach the button");
    /// };
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn content_search(&self, query: ContentQuery) -> ContentSearch {
        ContentSearch::new(query, Rc::as_ptr(&self.shared).cast::<()>() as usize)
    }

    pub(crate) fn content_step(
        &self,
        search: &mut ContentSearch,
    ) -> Result<ContentStep, ContentSearchError> {
        if search.owner != Rc::as_ptr(&self.shared).cast::<()>() as usize {
            return Err(ContentSearchError::ForeignRuntime);
        }
        if search.done {
            return Ok(ContentStep {
                hits: Vec::new(),
                done: true,
                work: 0,
            });
        }
        if !search.started {
            search.discover = Some(0);
            search.generation = self.structure_generation;
            search.started = true;
        } else if search.generation != self.structure_generation {
            return Err(ContentSearchError::Stale);
        }
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let limit = search.query.result_limit();
        let budget = search.query.work_budget();
        let mut hits = Vec::new();
        let mut work = 0;
        // A collection the search has started on must still offer the rows
        // it saw, or hits already returned could name rows of another source.
        // This check reads each such collection's revision once per step and
        // is not charged to the budget, so every step still makes progress.
        for visited in &search.visited {
            let Some(entry) = self.entries.get(&visited.collection) else {
                return Err(ContentSearchError::Stale);
            };
            let unchanged = entry
                .component
                .try_content_rows()?
                .is_some_and(|rows| rows.revision() == visited.revision);
            if !unchanged {
                return Err(ContentSearchError::Stale);
            }
        }
        while work < budget && hits.len() < limit {
            if let Some(next) = search.discover {
                // Window roots are found by walking the entry table in serial
                // (mount) order, a unit per entry, resumable across steps.
                let mut cursor = None;
                for (serial, entry) in self.entries.range(next..) {
                    if work >= budget {
                        cursor = Some(*serial);
                        break;
                    }
                    work += 1;
                    if entry.home.is_none() {
                        search.roots.push(*serial);
                    }
                }
                search.discover = cursor;
                if cursor.is_none() {
                    let mut roots = std::mem::take(&mut search.roots);
                    roots.reverse();
                    search.pending = roots;
                }
                continue;
            }
            if let Some(scan) = search.rows.as_mut() {
                let Some(entry) = self.entries.get(&scan.collection) else {
                    return Err(ContentSearchError::Stale);
                };
                // A modal surface opened over the collection since the scan
                // began blocks its rows as it blocks its components.
                if !self.modal_allows_in_own_window(scan.collection) {
                    search.rows = None;
                    continue;
                }
                while scan.children_next < entry.children.len() && work < budget {
                    let child = entry.children[scan.children_next];
                    scan.children_next += 1;
                    work += 1;
                    if let Some(child) = self.entries.get(&child) {
                        scan.mounted.insert(child.key.clone());
                    }
                }
                if scan.children_next < entry.children.len() {
                    continue;
                }
                let Some(rows) = entry.component.try_content_rows()? else {
                    return Err(ContentSearchError::Stale);
                };
                if rows.revision() != scan.revision {
                    return Err(ContentSearchError::Stale);
                }
                let count = rows.len();
                while scan.next < count && work < budget && hits.len() < limit {
                    let index = scan.next;
                    scan.next += 1;
                    work += 1;
                    let Some(key) = rows.key(index) else { continue };
                    if scan.mounted.contains(&key) {
                        continue;
                    }
                    if let Some(text) = rows.text(index)
                        && search.query.matches(&text)
                    {
                        hits.push(ContentHit {
                            target: ContentTarget::Row {
                                collection: WidgetId(entry.node.clone()),
                                key,
                            },
                            role: None,
                            text,
                            field: ContentField::Row,
                            other: None,
                            path: scan.path.clone(),
                            visible: false,
                        });
                    }
                }
                if scan.next >= count {
                    search.rows = None;
                }
                continue;
            }
            let Some(serial) = search.pending.pop() else {
                search.done = true;
                break;
            };
            let Some(entry) = self.entries.get(&serial) else {
                return Err(ContentSearchError::Stale);
            };
            work += 1;
            if !entry.searchable {
                continue;
            }
            let visible = entry.node.visible.get();
            if !visible && !search.query.includes_hidden() {
                continue;
            }
            let rows = if self.modal_allows_in_own_window(serial) {
                entry.component.try_content_rows()?
            } else {
                None
            };
            let mounted_current = rows.as_ref().is_none_or(|rows| {
                entry.node.children_current.get() && rows.mounted_rows_current()
            });
            if mounted_current {
                search.pending.extend(entry.children.iter().rev().copied());
            }
            if let Some(hit) = self.content_hit(entry, &search.query, visible) {
                hits.push(hit);
            }
            if let Some(rows) = rows {
                search.visited.push(crate::content_search::VisitedRows {
                    collection: serial,
                    revision: rows.revision(),
                    _rows: rows.clone(),
                });
                if !search.query.includes_rows() {
                    continue;
                }
                search.rows = Some(RowScan {
                    collection: serial,
                    revision: rows.revision(),
                    children_next: if mounted_current {
                        0
                    } else {
                        entry.children.len()
                    },
                    next: 0,
                    mounted: std::collections::BTreeSet::new(),
                    path: self.content_path(serial),
                });
            }
        }
        if search.discover.is_none() && search.pending.is_empty() && search.rows.is_none() {
            search.done = true;
        }
        Ok(ContentStep {
            hits,
            done: search.done,
            work,
        })
    }

    /// The hit `entry` makes for `query`, if its name or value matches.
    fn content_hit(
        &self,
        entry: &Entry,
        query: &ContentQuery,
        visible: bool,
    ) -> Option<ContentHit> {
        let semantics = entry.component.inspect().semantics?;
        // A component that names its child, such as a `Field`, is found
        // through that child, which carries its name.
        if semantics.names_children || !self.modal_allows_in_own_window(entry.node.serial) {
            return None;
        }
        let name = semantics
            .name
            .clone()
            .or_else(|| self.element_label_name(entry))
            .or_else(|| self.naming_parent(entry))
            .filter(|name| !name.trim().is_empty());
        let value = if semantics.protected {
            None
        } else {
            semantics.value.as_ref().and_then(|value| match value {
                SemanticValue::Text(text) => Some(text.clone()),
                SemanticValue::Number { value, .. } => Some(value.to_string()),
                SemanticValue::Choice { label, .. } => label.clone(),
                SemanticValue::Bool(_) => None,
            })
        };
        let value = value.filter(|value| !value.trim().is_empty());
        let (text, field, other) = match (name, value) {
            (Some(name), value) if query.matches(&name) => (name, ContentField::Name, value),
            (name, Some(value)) if query.matches(&value) => (value, ContentField::Value, name),
            _ => return None,
        };
        Some(ContentHit {
            target: ContentTarget::Widget(WidgetId(entry.node.clone())),
            role: Some(semantics.role),
            text,
            field,
            other,
            path: self.content_path(entry.node.serial),
            visible,
        })
    }

    /// The addressable path of `serial`, or of its nearest addressable
    /// ancestor for internal structure, such as a table's body list.
    fn content_path(&self, serial: u64) -> Option<Vec<String>> {
        let mut current = serial;
        loop {
            if let Some(path) = self.workspace_segments(current) {
                return Some(path);
            }
            current = self.entries.get(&current)?.node.logical_parent_serial()?;
        }
    }

    /// The name a `Field` above `entry`, past internal structure, gives it.
    fn naming_parent(&self, entry: &Entry) -> Option<String> {
        let mut current = self.entries.get(&entry.node.logical_parent_serial()?)?;
        loop {
            if let Some(semantics) = current.component.inspect().semantics {
                return semantics.names_children.then_some(semantics.name).flatten();
            }
            if current.addressing != crate::layout::Addressing::Structural {
                return None;
            }
            current = self.entries.get(&current.node.logical_parent_serial()?)?;
        }
    }

    /// Opens the way to a content hit and focuses it.
    ///
    /// A mounted component is revealed and focused with
    /// [`Self::reveal_and_focus`] (a component that cannot take focus is only
    /// revealed). A row that is not mounted is reached in three steps: the
    /// collection is revealed, the row is requested by key through the
    /// collection's [`ContentRows`](crate::ContentRows) and the runtime
    /// flushes until it is mounted, and then the row is revealed and focused.
    /// Like the reveal operations it flushes, so it may be called where those
    /// may: from application code, a command action or a deferred task
    /// (`EventContext::defer`), never from a component callback.
    ///
    /// The outcome is typed: a refused reveal is
    /// `ContentJump::Reveal(RevealOutcome::Refused(..))`, a row the
    /// collection could not mount is [`ContentJump::Row`] with its status, and
    /// a row the collection has not mounted after two flushes is
    /// [`ContentJump::Pending`], which [`Self::resume_content_jump`] finishes.
    ///
    /// # Errors
    /// The errors of [`Self::reveal_and_focus`] and [`Self::flush`] for a
    /// stale, foreign or unmounted target, and the collection's request
    /// failure for a row.
    pub fn jump_to_content(&mut self, target: &ContentTarget) -> Result<ContentJump, RuntimeError> {
        match target {
            ContentTarget::Widget(id) => self.reveal_and_focus(id).map(ContentJump::Reveal),
            ContentTarget::Row { collection, key } => {
                self.check(collection)?;
                let outcome = self.reveal(collection)?;
                if !matches!(outcome, crate::RevealOutcome::Revealed { .. }) {
                    return Ok(ContentJump::Reveal(outcome));
                }
                let Some(rows) = self.entries[&collection.0.serial]
                    .component
                    .try_content_rows()?
                else {
                    return Ok(ContentJump::Row(ListRevealStatus::NotFound));
                };
                let receipt = rows.reveal(key)?;
                // Mounting usually completes within one flush; a second
                // settles a row whose mounting scheduled another update.
                for _ in 0..2 {
                    if receipt.status() != ListRevealStatus::Pending {
                        break;
                    }
                    self.flush(self.shared.now.get())?;
                }
                self.resume_content_jump(&crate::PendingJump { receipt })
            }
        }
    }

    /// Finishes a [`ContentJump::Pending`] jump: once the collection has
    /// mounted the row it reveals and focuses it, as
    /// [`Self::jump_to_content`] would have; a row the collection could not
    /// mount ends as [`ContentJump::Row`] with its status; a row still being
    /// mounted returns the pending jump again. It flushes only to reveal, so
    /// it may be called where the reveal operations may.
    ///
    /// # Errors
    /// The errors of [`Self::reveal_and_focus`] for a stale or foreign row.
    pub fn resume_content_jump(
        &mut self,
        pending: &crate::PendingJump,
    ) -> Result<ContentJump, RuntimeError> {
        match pending.status() {
            ListRevealStatus::Mounted(row) => self.reveal_and_focus(&row).map(ContentJump::Reveal),
            ListRevealStatus::Pending => Ok(ContentJump::Pending(pending.clone())),
            status => Ok(ContentJump::Row(status)),
        }
    }
}
