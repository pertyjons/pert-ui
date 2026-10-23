use super::*;
use crate::layout::rect;
use crate::{ArrangeContext, Constraints, Element, LayoutError, MeasureContext, Rect, Size};

/// One arranged component, applied to its node only when the whole pass succeeded.
struct Staged {
    serial: u64,
    /// Allocation in the window.
    bounds: Rect,
    /// The same allocation in the component's own local pixels.
    size: Size,
    clip: Rect,
    children_clip: Rect,
    /// The component's `child_transform` offset used for this arrangement.
    view_offset: crate::Point,
    /// The visible children the component's `arrange` placed in this pass, with
    /// their placements and local clips; `None` when it was not called. Kept
    /// only once the whole pass succeeds, like the geometry above.
    arranged: Option<(Vec<u64>, Vec<Placement>)>,
}

/// Where a parent placed a child in its content space, with the local clip it
/// gave.
type Placement = (Rect, Option<Rect>);
/// A reconciled child's new metadata, which a re-description refreshes
/// without remounting: its serial, label, rich label, label source and
/// leading label icon, detach permission and searchability.
struct MetadataChange {
    serial: u64,
    label: Option<String>,
    rich_label: Option<crate::RichText>,
    label_source: Option<crate::TextSource>,
    label_leading: Option<crate::IconSource>,
    detachable: crate::Permission,
    searchable: bool,
    grow: u16,
}

/// Whether two optional label sources are the same binding, whose read text
/// the mounted child keeps.
fn same_bound_label(mine: Option<&crate::TextSource>, theirs: Option<&crate::TextSource>) -> bool {
    matches!((mine, theirs), (Some(a), Some(b)) if a.is_bound() && a.same_as(b))
}

fn validate(elements: &[Element]) -> Result<u64, RuntimeError> {
    validate_traced(elements, &mut Vec::new())
}

/// [`validate`], recording in `trail`, innermost first, the keys down to the
/// element an error names: the duplicated key, the element whose description
/// failed, or the parent of an empty key.
fn validate_traced(elements: &[Element], trail: &mut Vec<String>) -> Result<u64, RuntimeError> {
    let mut keys = std::collections::BTreeSet::new();
    let mut count = 0u64;
    for element in elements {
        if element.key.trim().is_empty() {
            return Err(RuntimeError::EmptyKey);
        }
        if !keys.insert(&element.key) {
            trail.push(element.key.clone());
            return Err(RuntimeError::DuplicateKey(element.key.clone()));
        }
        element
            .component
            .check_description()
            .inspect_err(|_| trail.push(element.key.clone()))?;
        let descendants = validate_traced(&element.children, trail)
            .inspect_err(|_| trail.push(element.key.clone()))?;
        count = count
            .checked_add(1)
            .and_then(|n| n.checked_add(descendants))
            .ok_or(RuntimeError::Exhausted)?;
    }
    Ok(count)
}

impl Runtime {
    /// Mounts an owned component tree with sibling-scoped stable keys.
    ///
    /// Keys/identity capacity are validated before callbacks. Mount callbacks run
    /// parent first; teardown runs children first in logical sibling order. If a mount
    /// callback panics, completed mounts are cleaned up before unwinding resumes.
    /// New handles never reuse a removed incarnation. No callbacks may panic during
    /// teardown. First update/layout is scheduled automatically.
    ///
    /// # Errors
    /// Nothing is mounted after an error: a runtime phase error (see
    /// [`Runtime`]); [`RuntimeError::EmptyKey`] for a key without a
    /// non-whitespace character anywhere in the tree;
    /// [`RuntimeError::DuplicateKey`] for siblings sharing a key or a root key
    /// already in use; the error a component's
    /// [`Component::check_description`](crate::Component::check_description)
    /// returns, conventionally [`RuntimeError::Layout`] with
    /// [`LayoutError::InvalidChild`](crate::LayoutError::InvalidChild); or
    /// [`RuntimeError::Exhausted`] when the tree needs more identities than
    /// remain.
    pub fn mount_tree(&mut self, element: Element) -> Result<WidgetId, RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Mount, None, "");
        self.shared.writable()?;
        let count = validate(std::slice::from_ref(&element))?;
        self.serial
            .checked_add(count)
            .ok_or(RuntimeError::Exhausted)?;
        let start = self.serial;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.mount_description(element, None, true)
        }));
        match result {
            Ok(Ok(id)) => {
                self.apply_pending_workspace();
                Ok(id)
            }
            Ok(Err(error)) => {
                self.rollback_new(start);
                Err(error)
            }
            Err(panic) => {
                self.rollback_new(start);
                std::panic::resume_unwind(panic)
            }
        }
    }

    /// Reads a mounted component's stable sibling key.
    /// The key is for logical structure discovery; retain `WidgetId` for event targets.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime and `Unmounted` for an
    /// unmounted one.
    pub fn key(&self, id: &WidgetId) -> Result<&str, RuntimeError> {
        self.check(id)?;
        Ok(&self.entries[&id.0.serial].key)
    }
    /// Returns ordered immediate children, including locally or ancestrally hidden ones.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ForeignRuntime`] for a parent of another runtime
    /// and [`RuntimeError::Unmounted`] for an unmounted one.
    pub fn children(&self, parent: &WidgetId) -> Result<Vec<WidgetId>, RuntimeError> {
        self.check(parent)?;
        Ok(self.entries[&parent.0.serial]
            .children
            .iter()
            .map(|serial| WidgetId(self.entries[serial].node.clone()))
            .collect())
    }

    /// Finds the descendant of `root` reached by the full internal path of
    /// sibling keys, one key per level, including the keys built-in composites
    /// give the parts they create; an empty path returns `root` itself.
    ///
    /// This is the lookup for code that knows how the built-ins it passes
    /// through are composed, such as a test of a composite's internal parts.
    /// Ordinary lookups use [`Self::find`], which names only the addressable
    /// segments that inspection reports, or [`Self::find_descendant`] for one
    /// application key. It follows logical children, including hidden ones and those presented
    /// in a detached window, exactly as `children` lists them. Each level scans
    /// that parent's children, so the cost grows with the path length times the
    /// sibling counts; retain the returned `WidgetId` rather than looking it up
    /// for every event.
    ///
    /// # Errors
    /// Rejects a foreign or unmounted `root`. A path that does not resolve is
    /// `Ok(None)`, not an error.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    ///
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([Element::new(
    ///     "settings",
    ///     Row::new([Element::new("apply", Label::new("Apply"))]),
    /// )])))?;
    /// let apply = ui.find_path(&root, &["settings", "apply"])?.ok_or("not mounted")?;
    /// assert_eq!(ui.key(&apply)?, "apply");
    /// assert!(ui.find_path(&root, &["settings", "missing"])?.is_none());
    /// assert_eq!(ui.find_path(&root, &[])?, Some(root.clone()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn find_path(
        &self,
        root: &WidgetId,
        path: &[&str],
    ) -> Result<Option<WidgetId>, RuntimeError> {
        self.check(root)?;
        let mut serial = root.0.serial;
        for key in path {
            let Some(child) = self.entries[&serial]
                .children
                .iter()
                .copied()
                .find(|child| self.entries[child].key == *key)
            else {
                return Ok(None);
            };
            serial = child;
        }
        Ok(Some(WidgetId(self.entries[&serial].node.clone())))
    }

    /// Finds the unique descendant of `ancestor` whose key is `key`, at any
    /// depth below it.
    ///
    /// [`Self::find_path`] needs every key on the way down, including the keys that
    /// built-in composites give the parts they create, such as a dialog's
    /// anchor, a scroll area's viewport or a group's content. This lookup names
    /// only the key the application chose, so a host can find its own
    /// component without knowing how the built-ins in between are composed.
    /// Keys are unique only among siblings, so the lookup never picks one of
    /// several matches: if more than one descendant carries `key`, it fails
    /// with [`RuntimeError::AmbiguousKey`]. Narrow the search by first finding
    /// a uniquely keyed ancestor and searching below it, name the addressable
    /// path with [`Self::find`], or use [`Self::find_path`] with a full path.
    /// A common cause is a container keyed like the control it holds, such as
    /// a [`ToolGroup`](crate::ToolGroup) named after its only control; give
    /// the container a key of its own. `ancestor` itself is not a candidate.
    ///
    /// It follows logical children, including hidden ones and those presented
    /// in a detached window, exactly as [`Self::children`] lists them. It
    /// always visits the whole subtree below `ancestor` to rule out a second
    /// match, so the cost grows with the subtree's size; retain the returned
    /// `WidgetId` rather than looking it up for every event.
    ///
    /// # Errors
    /// Rejects a foreign or unmounted `ancestor` and returns
    /// [`RuntimeError::AmbiguousKey`] when several descendants carry `key`. No
    /// matching descendant is `Ok(None)`, not an error.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    ///
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("left", Row::new([Element::new("apply", Label::new("Apply"))])),
    ///     Element::new("right", Row::new([
    ///         Element::new("apply", Label::new("Apply")),
    ///         Element::new("reset", Label::new("Reset")),
    ///     ])),
    /// ])))?;
    /// let reset = ui.find_descendant(&root, "reset")?.ok_or("not mounted")?;
    /// assert_eq!(ui.key(&reset)?, "reset");
    /// assert!(ui.find_descendant(&root, "missing")?.is_none());
    /// // Two descendants are keyed "apply": the lookup refuses to choose.
    /// assert_eq!(
    ///     ui.find_descendant(&root, "apply"),
    ///     Err(RuntimeError::AmbiguousKey("apply".into()))
    /// );
    /// let right = ui.find_descendant(&root, "right")?.ok_or("not mounted")?;
    /// assert!(ui.find_descendant(&right, "apply")?.is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn find_descendant(
        &self,
        ancestor: &WidgetId,
        key: &str,
    ) -> Result<Option<WidgetId>, RuntimeError> {
        self.check(ancestor)?;
        let mut found = None;
        let mut pending: Vec<u64> = self.entries[&ancestor.0.serial].children.clone();
        while let Some(serial) = pending.pop() {
            let entry = &self.entries[&serial];
            if entry.key == key {
                if found.is_some() {
                    return Err(RuntimeError::AmbiguousKey(key.to_owned()));
                }
                found = Some(serial);
            }
            pending.extend(entry.children.iter().copied());
        }
        Ok(found.map(|serial| WidgetId(self.entries[&serial].node.clone())))
    }

    /// Finds the descendant of `root` at an addressable path below it, skipping
    /// the structure built-in composites create; an empty path returns `root`
    /// itself.
    ///
    /// This is the lookup for a component a description mounted, such as the
    /// target of [`Self::focus`], [`Self::request_detach`] or a view-scoped
    /// command, without walking [`Self::children`] and [`Self::key`] by hand,
    /// and the path it takes is the one inspection reports
    /// ([`ComponentInspection::path`](crate::ComponentInspection::path)) and
    /// [`Selector::path`](crate::Selector::path) matches. An inspection path
    /// starts with the root's own key, so pass the rest of it:
    /// `find(&root, &path[1..])`. Built-in containers add a segment only where
    /// they document one, such as [`RegionLayout`](crate::RegionLayout)'s
    /// region names `top`, `bottom`, `left`, `right` and `center`.
    ///
    /// The segments follow the rules of
    /// [workspace persistence](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#workspace-persistence)
    /// and inspection's `Selector::path`: an element contributes its key, a
    /// slot contributes its slot name, a part contributes its part name for
    /// itself only, and structural elements contribute nothing. `segments` is
    /// the address relative to `root`: the segments contributed by the
    /// elements strictly below `root` on the way down to the target, so a
    /// titled group's content is `["form"]` below the group's key, not the
    /// frame and body levels [`Self::find_path`] would have to name. The match is
    /// the element that introduces the last segment; structural wrappers
    /// beneath it share its address but are not matches. A drag preview
    /// never matches.
    ///
    /// Unlike [`Self::find_descendant`], every segment on the way down is
    /// named, so two application keys that repeat in different containers
    /// are told apart by their containers' keys. Structural levels can still
    /// make one address reach several elements, such as two structural
    /// wrappers each holding a child keyed alike; the lookup then fails with
    /// [`RuntimeError::AmbiguousKey`] rather than picking one.
    ///
    /// It follows logical children, including hidden ones and those presented
    /// in a detached window, exactly as [`Self::children`] lists them. It
    /// visits every descendant reachable through structural levels and
    /// matching segments, so the cost grows with that part of the subtree;
    /// retain the returned `WidgetId` rather than looking it up for every
    /// event.
    ///
    /// # Errors
    /// Returns [`RuntimeError::ForeignRuntime`] for a `root` of another
    /// runtime, [`RuntimeError::Unmounted`] for an unmounted one and
    /// [`RuntimeError::AmbiguousKey`] when several descendants have the
    /// address. An address that does not resolve is `Ok(None)`, not an error.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    ///
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("left", Group::titled("Left", Element::new(
    ///         "form",
    ///         Row::new([Element::new("apply", Label::new("Apply"))]),
    ///     ))),
    ///     Element::new("right", Row::new([Element::new("apply", Label::new("Apply"))])),
    /// ])))?;
    /// // The group's own frame and body levels are not part of the address.
    /// let apply = ui.find(&root, &["left", "form", "apply"])?.ok_or("not mounted")?;
    /// assert_eq!(ui.key(&apply)?, "apply");
    /// // `find_descendant(&root, "apply")` would be ambiguous; an address is not.
    /// assert!(ui.find(&root, &["right", "apply"])?.is_some());
    /// assert!(ui.find(&root, &["left", "apply"])?.is_none());
    /// assert_eq!(ui.find(&root, &[])?, Some(root.clone()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn find(
        &self,
        root: &WidgetId,
        segments: &[&str],
    ) -> Result<Option<WidgetId>, RuntimeError> {
        use crate::layout::Addressing;
        self.check(root)?;
        let Some(last) = segments.len().checked_sub(1) else {
            return Ok(Some(root.clone()));
        };
        let preview = self.drag_preview_root();
        let mut found = None;
        // Each pending child carries how many segments its ancestors below
        // `root` have matched; that count is always below `segments.len()`.
        let mut pending: Vec<(u64, usize)> = self.entries[&root.0.serial]
            .children
            .iter()
            .map(|child| (*child, 0))
            .collect();
        while let Some((serial, matched)) = pending.pop() {
            if Some(serial) == preview {
                continue;
            }
            let entry = &self.entries[&serial];
            let (own, inherited) = match &entry.addressing {
                Addressing::Own => (Some(entry.key.as_str()), true),
                Addressing::Slot(name) => (Some(name.as_str()), true),
                Addressing::Part(name) => (Some(name.as_str()), false),
                Addressing::Structural => (None, false),
            };
            let hit = own.is_some_and(|segment| segment == segments[matched]);
            if hit && matched == last {
                if found.is_some() {
                    return Err(RuntimeError::AmbiguousKey(segments.join("/")));
                }
                found = Some(serial);
            }
            let next = match (own, inherited) {
                // Anything below has a longer address, or shares this one
                // through structural wrappers without being a match.
                (Some(_), true) if hit && matched == last => continue,
                (Some(_), true) if hit => matched + 1,
                // The descendants' addresses contain this mismatched segment.
                (Some(_), true) => continue,
                // A part's descendants address as if it were structural.
                _ => matched,
            };
            pending.extend(entry.children.iter().map(|child| (*child, next)));
        }
        Ok(found.map(|serial| WidgetId(self.entries[&serial].node.clone())))
    }

    /// Returns the most recent allocated window-coordinate bounds, if laid out.
    ///
    /// Bounds may describe the last successful layout while a new layout is pending.
    /// Legacy roots have no allocation. Hidden components retain their last bounds.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime and `Unmounted` for an
    /// unmounted one.
    pub fn bounds(&self, id: &WidgetId) -> Result<Option<Rect>, RuntimeError> {
        self.check(id)?;
        Ok(id.0.bounds.get())
    }

    /// Returns the [`ComponentInspection::id`](crate::ComponentInspection::id)
    /// of a mounted component, so a test or tool finds that component's entry
    /// in a snapshot of this runtime without matching keys.
    ///
    /// The value names this incarnation only: it is unique within the runtime,
    /// is never reused for another component, and a component mounted again
    /// under the same key gets a new one. It means nothing in another runtime's
    /// snapshot.
    ///
    /// # Errors
    /// Returns `ForeignRuntime` for an ID of another runtime and `Unmounted` for an
    /// unmounted one.
    ///
    /// One component's entry is one [`Self::query`] with
    /// [`Selector::id`](crate::Selector::id), a single lookup, rather than a
    /// search of a whole [`Self::inspect`] snapshot:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Button, Column, Detail, Element, InspectionQuery, Runtime, Selector};
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("save", Button::new("Save", |_| Ok(()))),
    /// ])))?;
    /// let save = ui.find(&root, &["save"])?.ok_or("not mounted")?;
    /// let id = ui.inspection_id(&save)?;
    /// let result = ui.query(
    ///     &InspectionQuery::components(Selector::id(id)).detail(Detail::Semantics),
    /// );
    /// let entry = result.components.first().ok_or("no entry")?;
    /// assert_eq!(entry.key, "save");
    /// let semantics = entry.details.semantics.as_ref().ok_or("no semantics")?;
    /// assert_eq!(semantics.name.as_deref(), Some("Save"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn inspection_id(&self, id: &WidgetId) -> Result<u64, RuntimeError> {
        self.check(id)?;
        Ok(id.incarnation())
    }

    /// Reconciles a composed parent's children by key and concrete component type.
    ///
    /// Surviving components retain their state, subscriptions and identity. Their
    /// new descriptions are applied through [`Component::reconcile`], which
    /// separates replaceable properties from retained local state. Descendants are
    /// reconciled recursively, except for components opting into
    /// [`Component::retain_initial_children`]: their incoming descendants are
    /// discarded before admission while their mounted children stay.
    /// Display labels update independently of retained
    /// component values. New mounts complete before removals/order changes;
    /// a mounting panic cleans new mounts without changing surviving topology.
    /// Removing children cancels edits and subscriptions recursively. A
    /// surviving component that has requested its own children through
    /// [`UpdateContext::request_children`], such as a `VirtualList`, keeps them:
    /// only its own description is applied.
    ///
    /// A child that was moved deeper into the subtree of the parent describing
    /// it, as a container moves its content into its own overflow surface, is
    /// matched by key where it is now (see [`Self::reparent`]): it keeps its
    /// owner, position and state, its new description is applied there, and a
    /// description that omits it unmounts it. The owner holding it keeps it
    /// through that owner's own descriptions, which do not list it.
    ///
    /// # Errors
    /// Rejects invalid IDs, duplicate/empty keys (including a key that names a
    /// child moved here from a parent that still describes it), changed
    /// surviving component types,
    /// identity exhaustion, legacy parents, children described for a surviving
    /// component that manages its own (`LayoutError::InvalidChild`, except for
    /// [`Component::retain_initial_children`]) or forbidden phases before mutation.
    pub fn reconcile_children(
        &mut self,
        parent: &WidgetId,
        children: Vec<Element>,
    ) -> Result<(), RuntimeError> {
        self.reconcile_children_traced(parent, children, &mut Vec::new())
    }

    /// [`Self::reconcile_children`], recording in `trail`, innermost first, the
    /// keys from `children` down to the description an error names. It stays
    /// empty for an error that names no element, such as a parent that cannot
    /// take children.
    pub(crate) fn reconcile_children_traced(
        &mut self,
        parent: &WidgetId,
        mut children: Vec<Element>,
        trail: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Reconcile, None, "");
        self.shared.writable()?;
        self.check(parent)?;
        if !self.entries[&parent.0.serial].composed {
            return Err(LayoutError::InvalidChild.into());
        }
        self.discard_owned_descriptions(parent.0.serial, &mut children, trail)?;
        let count = validate_traced(&children, trail)?;
        self.serial
            .checked_add(count)
            .ok_or(RuntimeError::Exhausted)?;
        self.validate_types(parent.0.serial, &children, trail)?;
        let mut labels = Vec::new();
        self.metadata_changes(parent.0.serial, &children, &mut labels);
        let start = self.serial;
        let mut patches = Vec::new();
        let mut properties = Vec::new();
        let mut kept = std::collections::BTreeSet::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.prepare_children(
                parent.0.serial,
                children,
                &mut patches,
                &mut properties,
                &mut kept,
                trail,
            )
        }));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                self.rollback_new(start);
                return Err(error);
            }
            Err(panic) => {
                self.rollback_new(start);
                std::panic::resume_unwind(panic);
            }
        }
        let mut searchability_changed = false;
        for change in labels {
            let MetadataChange {
                serial,
                label,
                rich_label,
                label_source,
                label_leading,
                detachable,
                searchable,
                grow,
            } = change;
            let entry = self
                .entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?;
            // The same binding keeps the text the parent read; anything else
            // takes the description's text, empty for a binding until read.
            let keep = same_bound_label(entry.label_source.as_ref(), label_source.as_ref())
                && entry.label_leading == label_leading;
            if !keep {
                entry.label = label;
                entry.rich_label = rich_label;
            }
            entry.label_source = label_source;
            entry.label_leading = label_leading;
            entry.node.detachable.replace(detachable);
            searchability_changed |= entry.searchable != searchable;
            entry.searchable = searchable;
            entry.grow = grow;
            if let Some(parent) = entry.node.logical_parent() {
                parent.invalidate(Invalidation::Layout);
            }
        }
        // A content search may have queued the children of an element that
        // has just opted out; it must not search them.
        if searchability_changed {
            self.structure_generation = self.structure_generation.wrapping_add(1);
        }
        // Moved children whose describing parent was reconciled without them.
        let described: std::collections::BTreeSet<u64> =
            patches.iter().map(|(serial, _)| *serial).collect();
        let dropped: Vec<u64> = self
            .homes
            .iter()
            .filter(|((home, _), serial)| described.contains(home) && !kept.contains(*serial))
            .map(|(_, serial)| *serial)
            .collect();
        let mut removed = Vec::new();
        let focused = self.window.focus.as_ref().map(|id| id.0.serial);
        let mut focus_successors = Vec::new();
        for (serial, mut order) in patches {
            // Children another parent describes are not listed here; they keep
            // their positions among the described ones.
            let current = &self.entries[&serial].children;
            for (index, child) in current.iter().enumerate() {
                if self.entries[child].home != Some(serial) && !dropped.contains(child) {
                    order.insert(index.min(order.len()), *child);
                }
            }
            if let Some(focused) = focused
                && !order.contains(&focused)
                && let Some(position) = current.iter().position(|id| *id == focused)
                && self.entries[&focused].component.focus_siblings_on_removal()
            {
                // Following siblings nearest first, then preceding ones.
                let kind = self.entries[&focused].kind;
                focus_successors = current[position + 1..]
                    .iter()
                    .chain(current[..position].iter().rev())
                    .filter(|id| order.contains(id) && self.entries[*id].kind == kind)
                    .map(|id| Rc::downgrade(&self.entries[id].node))
                    .collect();
            }
            let entry = self
                .entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?;
            removed.extend(
                entry
                    .children
                    .iter()
                    .filter(|id| !order.contains(id))
                    .copied(),
            );
            if entry.children != order {
                entry.raised_children.retain(|id| order.contains(id));
                entry.children = order;
                entry.node.invalidate(Invalidation::Layout);
            }
        }
        for serial in dropped {
            let owner = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.logical_parent_serial());
            if let Some(entry) = owner.and_then(|owner| self.entries.get_mut(&owner))
                && entry.children.contains(&serial)
            {
                entry.children.retain(|id| *id != serial);
                entry.raised_children.retain(|id| *id != serial);
                entry.node.invalidate(Invalidation::Layout);
                removed.push(serial);
            }
        }
        for serial in removed {
            self.remove_subtree(serial);
        }
        if !focus_successors.is_empty() && self.window.focus.is_none() {
            focus_successors.append(&mut self.window.focus_fallback);
            self.window.focus_fallback = focus_successors;
        }
        self.apply_pending_workspace();
        // All fallible mounting/validation is complete before property callbacks.
        let previous = self.shared.phase.replace(super::Phase::Update);
        let _guard = super::InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        for (serial, apply, next) in properties {
            if let Some(entry) = self.entries.get_mut(&serial)
                && let Some(kind) = apply(entry.component.as_mut(), next)
            {
                entry.node.invalidate(kind);
            }
        }
        #[cfg(debug_assertions)]
        self.refresh_validation();
        Ok(())
    }

    /// Moves a mounted subtree to another owner, or to another position under its
    /// current owner, without remounting it.
    ///
    /// The subtree keeps its incarnations, keys, component values, local state,
    /// subscriptions, parameter edits, pointer capture and keyboard focus: this is
    /// a change of ownership, not a rebuild. `index` is the insertion position
    /// among `parent`'s current children and is clamped to their count; it decides
    /// layout and keyboard navigation order exactly like a mounted position.
    /// Both owners and the moved subtree are scheduled for layout.
    ///
    /// Visibility is reevaluated against the new owner: a parent whose
    /// `Component::child_visible` rejects the key, or that is itself hidden, hides
    /// the subtree with the ordinary hiding rules, which cancel its gestures,
    /// drafts and held actions. Moving into a visible owner shows it again with
    /// its retained state. The new owner's `Component::child_surface` decides
    /// promotion as it does for a mounted child.
    ///
    /// The move also decides which description lists the subtree. Moved to an
    /// owner inside the subtree of the parent whose description lists it, such
    /// as a container's own overflow surface, it stays described by that
    /// parent: [`Self::reconcile_children`] on the parent matches it by key
    /// where it is now, applies its new description there and unmounts it when
    /// the description omits it, while descriptions of the owner holding it
    /// neither list nor remove it. Moved anywhere else, `parent` describes it
    /// from then on, so a later description of `parent` must list it to keep it
    /// and one of its former parent no longer does. Moving it back into the
    /// describing parent's subtree keeps that parent. An owner that is itself
    /// unmounted takes the moved children it holds with it.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Column, Element, Label, Runtime};
    /// let mut ui = Runtime::new();
    /// let root = ui.mount_tree(Element::new("root", Column::new([
    ///     Element::new("strip", Column::new([Element::new("tools", Label::new("Tools"))])),
    ///     Element::new("overflow", Column::new([])),
    /// ])))?;
    /// let slots = ui.children(&root)?;
    /// let (strip, overflow) = (slots[0].clone(), slots[1].clone());
    /// let tools = ui.children(&strip)?[0].clone();
    /// ui.reparent(&tools, &overflow, 0)?;
    /// assert_eq!(ui.children(&overflow)?, [tools.clone()]);
    /// assert!(ui.children(&strip)?.is_empty());
    /// ui.reparent(&tools, &strip, 0)?; // the same subtree returns
    /// assert_eq!(ui.children(&strip)?, [tools]);
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Rejects foreign or unmounted handles, a legacy (non-composed) mount on
    /// either side, a parent inside the moved subtree, and a key already used by
    /// another child of `parent` or by a child `parent` describes elsewhere
    /// (`DuplicateKey`). Returns `NotMovable` when the move would leave the
    /// subtree's native presentation root, when the subtree contains a detached
    /// view, when the moved component is itself a promoted surface root, whose
    /// presentation is anchored to its current owner, or when the subtree holds
    /// a component moved into it that would leave the subtree of the parent
    /// describing it.
    pub fn reparent(
        &mut self,
        child: &WidgetId,
        parent: &WidgetId,
        index: usize,
    ) -> Result<(), RuntimeError> {
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Reconcile, None, "");
        self.shared.writable()?;
        self.check(child)?;
        self.check(parent)?;
        let serial = child.0.serial;
        let target = parent.0.serial;
        if serial == target
            || !child.0.composed
            || !self.entries[&target].composed
            || self.owns(serial, target)
        {
            return Err(LayoutError::InvalidChild.into());
        }
        if !Rc::ptr_eq(&child.0.window(), &parent.0.window())
            || self.surface_kind(serial).is_some()
            || self.contains_detached(serial)
        {
            return Err(RuntimeError::NotMovable);
        }
        let key = self.entries[&serial].key.clone();
        let previous = child.0.logical_parent_serial();
        // A move deeper into the describing parent's subtree keeps that parent
        // as the one describing the child; any other move hands it to `target`.
        let home = self.entries[&serial]
            .home
            .filter(|home| self.owns(*home, target))
            .unwrap_or(target);
        // Moved descendants must stay inside the parents that describe them.
        if self.homes.iter().any(|((described_by, _), moved)| {
            *moved != serial
                && self.owns(serial, *moved)
                && !self.owns(serial, *described_by)
                && !self.owns(*described_by, target)
        }) {
            return Err(RuntimeError::NotMovable);
        }
        let taken = (previous != Some(target)
            && self.keys.contains_key(&(Some(target), key.clone())))
            || (home == target
                && self
                    .homes
                    .get(&(target, key.clone()))
                    .is_some_and(|moved| *moved != serial));
        if taken {
            let error = RuntimeError::DuplicateKey(key.clone());
            self.shared
                .record(crate::EventKind::Reparent, Some(serial), None, || {
                    crate::EventStatus::Failed(error.to_string())
                });
            return Err(error);
        }
        // Focus/hover readers on the old path must repaint before the move, and
        // readers on the new path after it; both paths are otherwise unchanged.
        child.0.invalidate_focus_path();
        if let Some(previous) = previous {
            if let Some(entry) = self.entries.get_mut(&previous) {
                entry.children.retain(|id| *id != serial);
                entry.raised_children.retain(|id| *id != serial);
                entry.node.invalidate(Invalidation::Layout);
            }
            self.keys.remove(&(Some(previous), key.clone()));
        } else {
            self.keys.remove(&(None, key.clone()));
        }
        *child.0.parent.borrow_mut() = Some(Rc::downgrade(&parent.0));
        child.0.parent_serial.set(Some(target));
        self.keys.insert((Some(target), key.clone()), serial);
        if let Some(entry) = self.entries.get_mut(&serial)
            && let Some(old) = entry.home.replace(home)
            && self.homes.get(&(old, key.clone())) == Some(&serial)
        {
            self.homes.remove(&(old, key.clone()));
        }
        if home != target {
            self.homes.insert((home, key.clone()), serial);
        }
        let entry = self
            .entries
            .get_mut(&target)
            .ok_or(RuntimeError::Unmounted)?;
        let index = index.min(entry.children.len());
        entry.children.insert(index, serial);
        entry.node.invalidate(Invalidation::Layout);
        let allowed = entry.component.child_visible(&key);
        self.apply_view_scale(serial, self.inherited_scale(serial));
        child.0.invalidate(Invalidation::Layout);
        child.0.invalidate_focus_path();
        self.window.shared.stack_hover_pending.set(true);
        self.window.shared.scene_dirty.set(true);
        self.refresh_visibility(serial, parent.0.visible.get() && allowed);
        // Focus that stays inside the moved subtree changes which ancestors it
        // is within: the old path's lose it, the new path's gain it.
        if let Some(focus) = self.window.focus.clone()
            && (focus.0.serial == serial || Self::focus_ancestors(&focus.0).contains(&serial))
        {
            let (leaving, entering) = self.focus_within_transfer(Some(&focus), &focus);
            let mut errors = self.notify_focus_within(leaving, false);
            errors.extend(self.notify_focus_within(entering, true));
            self.window.focus_errors.extend(errors);
        }
        #[cfg(debug_assertions)]
        {
            let window = child.0.window().serial;
            self.validator.relocate(serial, window, Some(target));
            // A component's requested move runs inside a flush, before the moved
            // subtree and its new siblings are arranged; this defers to the flush.
            self.refresh_validation();
        }
        self.shared
            .record(crate::EventKind::Reparent, Some(serial), None, || {
                crate::EventStatus::Succeeded
            });
        Ok(())
    }

    /// The accumulated view scale `serial` receives from its presentation parent:
    /// 1 for a root, a detached view or a promoted surface, which leave every
    /// view transform.
    pub(super) fn inherited_scale(&self, serial: u64) -> f32 {
        let node = &self.entries[&serial].node;
        if node.detached.get() || self.surface_kind(serial).is_some() {
            return 1.0;
        }
        node.presentation_parent_serial()
            .and_then(|parent| self.entries.get(&parent))
            .map_or(1.0, |parent| parent.node.content_scale.get())
    }
    /// Gives `serial` an accumulated view scale and carries the consequence down
    /// its subtree. A component whose scale changes prepares its text for another
    /// size, so its layout is invalidated; an unchanged one ends the walk.
    pub(super) fn apply_view_scale(&mut self, serial: u64, scale: f32) {
        let Some(entry) = self.entries.get(&serial) else {
            return;
        };
        let content = scale * entry.component.child_transform().scale().get();
        let changed = entry.node.scale.replace(scale) != scale;
        let content_changed = entry.node.content_scale.replace(content) != content;
        if changed {
            let node = entry.node.clone();
            self.prepare_view_scale(&node);
        }
        if content_changed {
            for child in entry.children.clone() {
                let inherited = self.inherited_scale(child);
                self.apply_view_scale(child, inherited);
            }
        }
    }
    /// Prepares `node` for its changed view scale, or stretches its drawing when
    /// the change continues a zoom; see [`super::VIEW_SCALE_SETTLE`].
    fn prepare_view_scale(&self, node: &Rc<Node>) {
        let now = self.shared.now.get();
        let last = node.scale_changed.replace(Some(now));
        let continuing =
            last.is_some_and(|last| now > last && now - last < super::VIEW_SCALE_SETTLE);
        if !continuing {
            node.prepared_scale.set(node.scale.get());
            node.invalidate(Invalidation::Layout);
            return;
        }
        // Back at the scale it was prepared at, nothing is stretched; the list
        // may still hold it from an earlier step, which settling tolerates.
        if node.prepared_scale.get() == node.scale.get() {
            return;
        }
        let shared = &self.shared;
        if !node.stretched.replace(true) {
            shared.stretched.borrow_mut().push(Rc::downgrade(node));
        }
        let settle = now.saturating_add(super::VIEW_SCALE_SETTLE);
        let next = shared
            .scale_settle
            .get()
            .map_or(settle, |due| due.min(settle));
        shared.scale_settle.set(Some(next));
    }
    /// Prepares the components a continuous zoom stretched whose scale has
    /// rested for [`super::VIEW_SCALE_SETTLE`], and schedules the rest.
    pub(super) fn settle_view_scale(&self, now: Duration) {
        let shared = &self.shared;
        if !shared.scale_settle.get().is_some_and(|due| due <= now) {
            return;
        }
        let stretched = std::mem::take(&mut *shared.stretched.borrow_mut());
        let mut waiting = Vec::new();
        let mut next: Option<Duration> = None;
        for node in stretched.iter().filter_map(Weak::upgrade) {
            if !node.mounted.get() || node.prepared_scale.get() == node.scale.get() {
                node.stretched.set(false);
                continue;
            }
            let due = node.scale_changed.get().map_or(now, |changed| {
                changed.saturating_add(super::VIEW_SCALE_SETTLE)
            });
            if due <= now {
                node.stretched.set(false);
                node.prepared_scale.set(node.scale.get());
                node.invalidate(Invalidation::Layout);
            } else {
                next = Some(next.map_or(due, |next| next.min(due)));
                waiting.push(Rc::downgrade(&node));
            }
        }
        *shared.stretched.borrow_mut() = waiting;
        shared.scale_settle.set(next);
    }
    /// Notices a changed `Component::child_transform` scale after an update,
    /// and a child that its parent started or stopped promoting to a surface,
    /// which enters or leaves the view transform without any scale changing.
    pub(super) fn sync_view_scale(&mut self, serial: u64) {
        let Some(entry) = self.entries.get(&serial) else {
            return;
        };
        self.apply_view_scale(serial, entry.node.scale.get());
        // Outside every view there is nothing a promotion could change.
        if self.entries[&serial].node.content_scale.get() == 1.0 {
            return;
        }
        for child in self.entries[&serial].children.clone() {
            let inherited = self.inherited_scale(child);
            self.apply_view_scale(child, inherited);
        }
    }

    /// Whether `ancestor` owns `serial` through lifetime parents, including itself.
    fn owns(&self, ancestor: u64, mut serial: u64) -> bool {
        loop {
            if serial == ancestor {
                return true;
            }
            let Some(parent) = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.logical_parent_serial())
            else {
                return false;
            };
            serial = parent;
        }
    }

    fn contains_detached(&self, serial: u64) -> bool {
        self.entries.get(&serial).is_some_and(|entry| {
            entry.node.detached.get()
                || entry
                    .children
                    .iter()
                    .any(|child| self.contains_detached(*child))
        })
    }

    /// Container metadata that reconciliation refreshes without remounting state.
    fn metadata_changes(
        &self,
        parent: u64,
        children: &[Element],
        changes: &mut Vec<MetadataChange>,
    ) {
        for child in children {
            if let Ok(Some(serial)) = self.described_child(parent, &child.key) {
                let entry = &self.entries[&serial];
                let same_label = if child.label_source.as_ref().is_some_and(|s| s.is_bound()) {
                    same_bound_label(entry.label_source.as_ref(), child.label_source.as_ref())
                        && entry.label_leading == child.label_leading
                } else {
                    entry.label == child.label
                        && entry.rich_label == child.rich_label
                        && entry.label_source.as_ref().is_none_or(|s| !s.is_bound())
                };
                if !same_label
                    || !entry.node.detachable.borrow().same_as(&child.detachable)
                    || entry.searchable != child.searchable
                    || entry.grow != child.grow
                {
                    changes.push(MetadataChange {
                        serial,
                        label: child.label.clone(),
                        rich_label: child.rich_label.clone(),
                        label_source: child.label_source.clone(),
                        label_leading: child.label_leading.clone(),
                        detachable: child.detachable.clone(),
                        searchable: child.searchable,
                        grow: child.grow,
                    });
                }
                self.metadata_changes(serial, &child.children, changes);
            }
        }
    }
    /// The mounted child that `parent`'s description names `key`: a direct
    /// child it describes, or one moved deeper into its subtree.
    ///
    /// # Errors
    /// `DuplicateKey` when the direct child with that key was moved here from
    /// another describing parent, which still describes it.
    fn described_child(&self, parent: u64, key: &str) -> Result<Option<u64>, RuntimeError> {
        match self.keys.get(&(Some(parent), key.to_owned())) {
            Some(serial) if self.entries[serial].home == Some(parent) => Ok(Some(*serial)),
            Some(_) => Err(RuntimeError::DuplicateKey(key.to_owned())),
            None => Ok(self.homes.get(&(parent, key.to_owned())).copied()),
        }
    }
    /// Ignore constructor parts only for compatible mounted owners. Newly
    /// mounted descriptions still receive recursive admission and validation.
    fn discard_owned_descriptions(
        &self,
        parent: u64,
        children: &mut [Element],
        trail: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        for element in children {
            let described = self
                .described_child(parent, &element.key)
                .inspect_err(|_| trail.push(element.key.clone()))?;
            if let Some(serial) = described {
                let entry = &self.entries[&serial];
                if entry.kind == element.kind && entry.node.initial_children_owned {
                    element.children.clear();
                } else {
                    self.discard_owned_descriptions(serial, &mut element.children, trail)
                        .inspect_err(|_| trail.push(element.key.clone()))?;
                }
            }
        }
        Ok(())
    }

    fn validate_types(
        &self,
        parent: u64,
        children: &[Element],
        trail: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        for element in children {
            let described = self
                .described_child(parent, &element.key)
                .inspect_err(|_| trail.push(element.key.clone()))?;
            if let Some(serial) = described {
                if self.entries[&serial].kind != element.kind {
                    trail.push(element.key.clone());
                    return Err(RuntimeError::ChildTypeChanged(element.key.clone()));
                }
                self.validate_types(serial, &element.children, trail)
                    .inspect_err(|_| trail.push(element.key.clone()))?;
            }
        }
        Ok(())
    }

    fn prepare_children(
        &mut self,
        parent: u64,
        children: Vec<Element>,
        patches: &mut Vec<(u64, Vec<u64>)>,
        properties: &mut Vec<(u64, crate::layout::Reconcile, Box<dyn Component>)>,
        kept: &mut std::collections::BTreeSet<u64>,
        trail: &mut Vec<String>,
    ) -> Result<(), RuntimeError> {
        let mut order = Vec::new();
        for mut element in children {
            let descendants = std::mem::take(&mut element.children);
            let key = element.key.clone();
            let existing = self
                .described_child(parent, &key)
                .inspect_err(|_| trail.push(key.clone()))?;
            let serial = if let Some(serial) = existing {
                properties.push((serial, element.reconcile, element.component));
                serial
            } else {
                self.mount_description(element, Some(parent), true)
                    .inspect_err(|_| trail.push(key.clone()))?
                    .0
                    .serial
            };
            // A child moved deeper into this parent's subtree stays with its
            // current owner, in its position there.
            if self.entries[&serial].node.logical_parent_serial() == Some(parent) {
                order.push(serial);
            } else {
                kept.insert(serial);
            }
            // A component that requested its own children owns them: a new
            // description of it must not unmount the rows it decided to show.
            if existing.is_some()
                && (self.entries[&serial].node.self_managed.get()
                    || self.entries[&serial].node.initial_children_owned)
            {
                if !descendants.is_empty() {
                    trail.push(key);
                    return Err(LayoutError::InvalidChild.into());
                }
                continue;
            }
            self.prepare_children(serial, descendants, patches, properties, kept, trail)
                .inspect_err(|_| trail.push(key))?;
        }
        patches.push((parent, order));
        Ok(())
    }

    fn rollback_new(&mut self, after: u64) {
        // Failed recursive mounts and prepared reconciliation entries may not yet
        // be linked into children. Reconstruct only new ownership before teardown.
        let serials: Vec<_> = self
            .entries
            .range((std::ops::Bound::Excluded(after), std::ops::Bound::Unbounded))
            .map(|(id, _)| *id)
            .collect();
        for serial in &serials {
            if let Some(parent) = self.entries[serial]
                .node
                .logical_parent_serial()
                .filter(|id| *id > after)
                && let Some(entry) = self.entries.get_mut(&parent)
                && !entry.children.contains(serial)
            {
                entry.children.push(*serial);
            }
        }
        for serial in serials {
            self.remove_subtree(serial);
        }
    }

    pub(super) fn mount_description(
        &mut self,
        element: Element,
        parent: Option<u64>,
        composed: bool,
    ) -> Result<WidgetId, RuntimeError> {
        self.shared.writable()?;
        if element.key.trim().is_empty() {
            return Err(RuntimeError::EmptyKey);
        }
        let scoped_key = (parent, element.key.clone());
        let moved = parent.and_then(|parent| self.homes.get(&(parent, element.key.clone())));
        if let Some(_existing) = self.keys.get(&scoped_key).or(moved) {
            #[cfg(debug_assertions)]
            {
                self.validator.duplicate(*_existing, &element.key);
                self.refresh_validation();
            }
            let error = RuntimeError::DuplicateKey(element.key.clone());
            self.shared.record(crate::EventKind::Mount, None, None, || {
                crate::EventStatus::Failed(error.to_string())
            });
            return Err(error);
        }
        let serial = self.serial.checked_add(1).ok_or(RuntimeError::Exhausted)?;
        let parent_node = parent.map(|id| self.entries[&id].node.clone());
        let visible = parent_node.as_ref().is_none_or(|n| n.visible.get())
            && parent
                .is_none_or(|serial| self.entries[&serial].component.child_visible(&element.key));
        let node = Rc::new(Node {
            window: RefCell::new(
                parent_node
                    .as_ref()
                    .map_or_else(|| self.window.shared.clone(), |n| n.window()),
            ),
            detached: Cell::new(false),
            font_layout: std::array::from_fn(|_| Cell::new(0)),
            font_paint: Cell::new(0),
            metric_layout: std::array::from_fn(|_| Cell::new(0)),
            metric_paint: Cell::new(0),
            focus_reader: Cell::new(false),
            motion_reader: Cell::new(false),
            motion_listed: Cell::new(false),
            focus_update_reader: Cell::new(false),
            self_managed: Cell::new(false),
            initial_children_owned: element.component.retain_initial_children(),
            children_current: Cell::new(true),
            hover_within: Cell::new(false),
            hover_reader: Cell::new(false),
            input_cancelled: Cell::new(false),
            serial,
            shared: self.shared.clone(),
            mounted: Cell::new(true),
            visible: Cell::new(visible),
            detachable: RefCell::new(element.detachable),
            local_visible: Cell::new(true),
            parent: RefCell::new(parent_node.as_ref().map(Rc::downgrade)),
            parent_serial: Cell::new(parent),
            composed,
            layout_dirty: Cell::new(true),
            measured: Cell::new(None),
            limits: Cell::new(None),
            bounds: Cell::new(None),
            size: Cell::new(None),
            scale: Cell::new(1.0),
            prepared_scale: Cell::new(1.0),
            scale_changed: Cell::new(None),
            stretched: Cell::new(false),
            content_scale: Cell::new(1.0),
            view_offset: Cell::new(crate::Point::ZERO),
            placement: Cell::new(None),
            arranged_children: RefCell::new(Vec::new()),
            clip: Cell::new(None),
            children_clip: Cell::new(None),
            dirty: Cell::new(None),
            deadline: Cell::new(None),
            presentation_suspended: Cell::new(false),
            animated: Cell::new(false),
            timer: Cell::new(None),
            gestures: RefCell::new(Vec::new()),
        });
        let mut entry = Entry {
            key: element.key,
            addressing: element.addressing,
            label: element.label,
            rich_label: element.rich_label,
            label_source: element.label_source,
            label_leading: element.label_leading,
            searchable: element.searchable,
            grow: element.grow,
            node: node.clone(),
            component: element.component,
            drawing: Vec::new(),
            masks: Vec::new(),
            images: Vec::new(),
            vectors: Vec::new(),
            foreground: 0,
            paint_dirty: Cell::new(true),
            text_drawn: false,
            text_origin: None,
            text_mode: self.shared.text_rendering.get(),
            subscriptions: Vec::new(),
            children: Vec::new(),
            raised_children: Vec::new(),
            kind: element.kind,
            composed,
            home: parent,
        };
        self.serial = serial;
        entry.component.mounted();
        self.entries.insert(serial, entry);
        self.structure_generation = self.structure_generation.wrapping_add(1);
        self.keys.insert(scoped_key, serial);
        let scale = self.inherited_scale(serial);
        node.scale.set(scale);
        node.prepared_scale.set(scale);
        node.content_scale.set(
            scale
                * self.entries[&serial]
                    .component
                    .child_transform()
                    .scale()
                    .get(),
        );
        if !visible {
            self.entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?
                .component
                .visibility_changed(false);
        }
        #[cfg(debug_assertions)]
        self.validator.mounted();
        node.invalidate(Invalidation::Layout);
        self.shared
            .record(crate::EventKind::Mount, Some(serial), None, || {
                crate::EventStatus::Succeeded
            });
        for child in element.children {
            let id = self.mount_description(child, Some(serial), true)?;
            self.entries
                .get_mut(&serial)
                .ok_or(RuntimeError::Unmounted)?
                .children
                .push(id.0.serial);
        }
        Ok(WidgetId(node))
    }

    pub(super) fn root_order(&self) -> Vec<u64> {
        self.entries
            .iter()
            .filter(|(_, e)| {
                e.node.presentation_parent_serial().is_none()
                    && Rc::ptr_eq(&e.node.window(), &self.window.shared)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    pub(super) fn paint_order(&self) -> Vec<u64> {
        self.drawing_order()
            .into_iter()
            .filter_map(|(id, foreground)| (!foreground).then_some(id))
            .collect()
    }
    pub(super) fn stacked_child(&self, serial: u64) -> bool {
        self.entries
            .get(&serial)
            .and_then(|e| e.node.presentation_parent_serial())
            .is_some_and(|parent| self.entries[&parent].component.stacks_children())
    }

    pub(super) fn raise_ancestors(&mut self, mut serial: u64) {
        while let Some(parent) = self.entries[&serial].node.presentation_parent_serial() {
            if let Some(entry) = self.entries.get_mut(&parent)
                && entry.component.stacks_children()
                && entry.raised_children.last().copied() != Some(serial)
            {
                entry.raised_children.retain(|id| *id != serial);
                entry.raised_children.push(serial);
                self.note_drag_layout_change();
                self.window.shared.stack_hover_pending.set(true);
                self.window.shared.scene_dirty.set(true);
            }
            serial = parent;
        }
    }

    pub(super) fn drawing_order(&self) -> Vec<(u64, bool)> {
        self.ordered_tree(true, false)
    }
    /// Drawing order without the components an empty clip removes entirely.
    ///
    /// Composition would translate and clip every command of a scrolled-out
    /// component only to discard it, so a dense collection costs a scene
    /// proportional to what is on screen instead of to what is mounted. A
    /// component that still owes a recording stays in the order: skipping it
    /// would leave it dirty, and `needs_paint` would then keep asking the host
    /// for a frame that changes nothing.
    pub(super) fn composition_order(&self) -> Vec<(u64, bool)> {
        self.ordered_tree(true, true)
    }
    pub(super) fn navigation_order(&self) -> Vec<u64> {
        self.ordered_tree(false, false)
            .into_iter()
            .filter_map(|(id, foreground)| (!foreground).then_some(id))
            .collect()
    }
    fn ordered_tree(&self, stacking: bool, composing: bool) -> Vec<(u64, bool)> {
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        fn visit(
            ui: &Runtime,
            serial: u64,
            entry: &Entry,
            order: &mut Vec<(u64, bool)>,
            deferred: &mut Vec<u64>,
            stacking: bool,
            composing: bool,
        ) {
            if !entry.node.visible.get() {
                return;
            }
            // An empty clip is the component's own allocation intersected with
            // its ancestors', so it removes that component's drawing without
            // saying anything about a child that its parent does not clip.
            let drawn = !composing || entry.node.is_presented();
            if drawn {
                order.push((serial, false));
            }
            // Ordinary parents use their logical child slice without allocating.
            // Stacking parents retain raised recency within each priority tier.
            let mut sorted = Vec::new();
            let children = if stacking && entry.component.stacks_children() {
                sorted.extend(
                    entry
                        .children
                        .iter()
                        .copied()
                        .filter(|id| !entry.raised_children.contains(id))
                        .chain(entry.raised_children.iter().copied()),
                );
                sorted
                    .sort_by_key(|id| entry.component.child_stacking_priority(&ui.entries[id].key));
                sorted.as_slice()
            } else {
                entry.children.as_slice()
            };
            for &child in children {
                let child_entry = &ui.entries[&child];
                if child_entry.node.detached.get() {
                    continue;
                }
                // This parent owns the child, so its own answer is the child's
                // surface kind; `surface_kind` would look both entries up again.
                if entry.component.child_surface(&child_entry.key).is_some() {
                    deferred.push(child);
                } else {
                    visit(ui, child, child_entry, order, deferred, stacking, composing);
                }
            }
            if drawn {
                order.push((serial, true));
            }
        }
        let mut order = Vec::with_capacity(self.entries.len() * 2);
        let mut deferred = Vec::new();
        // A drag preview draws above every view and surface, and is absent from
        // keyboard navigation.
        let preview = self.drag_preview_root();
        for root in self.root_order() {
            if Some(root) == preview {
                continue;
            }
            visit(
                self,
                root,
                &self.entries[&root],
                &mut order,
                &mut deferred,
                stacking,
                composing,
            );
        }
        while !deferred.is_empty() {
            let index = deferred
                .iter()
                .position(|id| !self.surface_kind(*id).is_some_and(|kind| kind.modal()))
                .unwrap_or(0);
            let root = deferred.remove(index);
            visit(
                self,
                root,
                &self.entries[&root],
                &mut order,
                &mut deferred,
                stacking,
                composing,
            );
        }
        if stacking
            && let Some(root) = preview
            && let Some(entry) = self.entries.get(&root)
            && Rc::ptr_eq(&entry.node.window(), &self.window.shared)
        {
            visit(
                self,
                root,
                entry,
                &mut order,
                &mut deferred,
                stacking,
                composing,
            );
        }
        order
    }
    pub(super) fn refresh_child_visibility(&mut self, serial: u64) {
        let children = self.entries[&serial].children.clone();
        let parent_visible = self.entries[&serial].node.visible.get();
        for child in children {
            if self.entries[&child].node.detached.get() {
                continue;
            }
            let allowed = self.entries[&serial]
                .component
                .child_visible(&self.entries[&child].key);
            self.refresh_visibility(child, parent_visible && allowed);
        }
    }
    pub(super) fn refresh_visibility(&mut self, serial: u64, parent_visible: bool) {
        if self.entries.get(&serial).is_some_and(|e| {
            e.node.visible.get() && (!parent_visible || !e.node.local_visible.get())
        }) {
            self.cancel_held_subtree(serial, crate::HeldCancellation::Unavailable);
            // A natively hidden (minimized or occluded) root only suspends its
            // presentation: an open modal session survives and is admitted
            // again when the root is shown.
            let suspended_modal = self.native_hiding
                && self.entries[&serial]
                    .children
                    .iter()
                    .any(|child| self.surface_kind(*child).is_some_and(|kind| kind.modal()));
            if !suspended_modal {
                self.defer_surface_dismissal(serial, crate::SurfaceDismissal::OwnerHidden);
            }
        }
        let Some(entry) = self.entries.get_mut(&serial) else {
            return;
        };
        let node = entry.node.clone();
        let visible = parent_visible && node.local_visible.get();
        if node.visible.replace(visible) != visible {
            self.shared.source_visibility_dirty.set(true);
            node.invalidate_layout();
            if visible {
                node.invalidate(Invalidation::Layout);
            } else {
                // Hiding cancels: a hidden ancestor is not told later that
                // focus left it.
                self.window.focus_within.retain(|id| *id != serial);
                if self
                    .window
                    .pointer_target
                    .as_ref()
                    .is_some_and(|id| id.0.serial == serial)
                {
                    self.window.pointer_target = None;
                }
                node.hover_within.set(false);
                node.stop_deadline();
                node.lose_context();
                self.window.shared.pending.borrow_mut().remove(&serial);
                if self
                    .window
                    .focus
                    .as_ref()
                    .is_some_and(|id| id.0.serial == serial)
                {
                    self.window.restore_focus |= node.composed;
                    self.window.focus_fallback = Self::focus_fallback_of(&node);
                    node.invalidate_focus_path();
                    self.window.focus = None;
                    self.window.shared.focused.set(None);
                    self.window.shared.focus_node.borrow_mut().take();
                }
                node.input_cancelled.set(true);
                entry.component.cancel_input(&mut EventContext {
                    node: entry.node.clone(),
                });
            }
            entry.component.visibility_changed(visible);
            // A shown or hidden component changes what a still pointer is over.
            self.note_drag_layout_change();
            #[cfg(debug_assertions)]
            {
                if visible && node.composed {
                    self.validator.unmeasured(serial);
                }
                self.validator.flags(serial, visible, false);
            }
            self.window.shared.scene_dirty.set(true);
            self.shared
                .record(crate::EventKind::Visibility, Some(serial), None, || {
                    crate::EventStatus::Succeeded
                });
        }
        self.refresh_child_visibility(serial);
    }

    pub(super) fn remove_subtree(&mut self, serial: u64) {
        let Some(node) = self.entries.get(&serial).map(|e| e.node.clone()) else {
            return;
        };
        let previous = self.current_window();
        let window = native_windows::NativeWindowId(node.window());
        // The root is retained until its last subtree cleanup completes.
        if let Err(error) = self.select_window(&window) {
            self.window.focus_errors.push_back(error);
            return;
        }
        // A session in a window that is about to be retired ends first, while
        // its preview root can still be reached and unmounted.
        if node.detached.get() && self.drag_in_window() {
            self.cancel_drag_infallibly();
        }
        self.remove_subtree_in_window(serial);
        if node.detached.get() {
            let primary = self.primary_window();
            if let Err(error) = self.select_window(&primary) {
                self.window.focus_errors.push_back(error);
            }
            self.retire_window(&window);
        }
        if self.contains_window(&previous)
            && let Err(error) = self.select_window(&previous)
        {
            self.window.focus_errors.push_back(error);
        }
    }
    fn remove_subtree_in_window(&mut self, serial: u64) {
        self.window.focus_within.retain(|id| *id != serial);
        self.defer_surface_dismissal(serial, crate::SurfaceDismissal::OwnerRemoved);
        self.structure_generation = self.structure_generation.wrapping_add(1);
        let Some(mut entry) = self.entries.remove(&serial) else {
            return;
        };
        for child in entry.children {
            self.remove_subtree(child);
        }
        self.remove_held_registration(serial);
        self.remove_scoped_commands(serial);
        self.shared.theme.borrow_mut().subscribers.remove(&serial);
        self.shared
            .theme
            .borrow_mut()
            .metric_subscribers
            .remove(&serial);
        self.window.shared.scene_dirty.set(true);
        self.note_drag_layout_change();
        entry.node.mounted.set(false);
        entry.node.stop_deadline();
        self.shared
            .font_subscribers
            .borrow_mut()
            .0
            .remove(&entry.node.serial);
        entry.node.lose_context();
        self.window.shared.pending.borrow_mut().remove(&serial);
        if let Some(home) = entry.home
            && self.homes.get(&(home, entry.key.clone())) == Some(&serial)
        {
            self.homes.remove(&(home, entry.key.clone()));
        }
        self.keys
            .remove(&(entry.node.logical_parent_serial(), entry.key));
        if self
            .window
            .focus
            .as_ref()
            .is_some_and(|id| id.0.serial == serial)
        {
            self.window.restore_focus |= entry.composed;
            self.window.focus_fallback = Self::focus_fallback_of(&entry.node);
            entry.node.invalidate_focus_path();
            self.window.focus = None;
            self.window.shared.focused.set(None);
            self.window.shared.focus_node.borrow_mut().take();
        }
        #[cfg(debug_assertions)]
        self.validator.remove(serial);
        entry.subscriptions.clear();
        entry.component.cancel_input(&mut EventContext {
            node: entry.node.clone(),
        });
        entry.component.unmounted();
        self.shared
            .record(crate::EventKind::Unmount, Some(serial), None, || {
                crate::EventStatus::Succeeded
            });
    }

    fn visible_children(&self, serial: u64) -> Vec<u64> {
        self.entries[&serial]
            .children
            .iter()
            .copied()
            .filter(|id| {
                self.entries[id].node.visible.get() && !self.entries[id].node.detached.get()
            })
            .collect()
    }

    pub(crate) fn child_fills_row_height(&self, serial: u64) -> bool {
        self.entries[&serial].component.fills_row_height()
    }
    pub(crate) fn child_grow(&self, serial: u64) -> u16 {
        self.entries[&serial].grow
    }
    pub(crate) fn child_position(&self, serial: u64) -> crate::Point {
        self.entries[&serial].component.layout_position()
    }
    pub(crate) fn child_key(&self, serial: u64) -> &str {
        &self.entries[&serial].key
    }
    pub(crate) fn child_limits(&self, serial: u64) -> Result<crate::SizeLimits, RuntimeError> {
        let entry = &self.entries[&serial];
        if let Some(limits) = entry.node.limits.get() {
            return Ok(limits);
        }
        let _timing =
            self.shared
                .profiler
                .span(crate::ProfileOperation::Measure, Some(serial), &entry.key);
        let children = self.visible_children(serial);
        entry.node.metric_layout[0].set(0);
        entry.node.font_layout[0].set(0);
        let limits = entry
            .component
            .measure_limits(&MeasureContext {
                node: &entry.node,
                reads: &entry.node.metric_layout[0],
                font_reads: &entry.node.font_layout[0],
                runtime: self,
                children: &children,
            })
            .inspect_err(|_| self.note_failure(serial, crate::FailurePhase::Measure))?;
        entry.node.limits.set(Some(limits));
        Ok(limits)
    }
    pub(crate) fn measure_node(
        &self,
        serial: u64,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let entry = &self.entries[&serial];
        if let Some((cached, size)) = entry.node.measured.get()
            && cached == available
        {
            return Ok(size);
        }
        let _timing =
            self.shared
                .profiler
                .span(crate::ProfileOperation::Measure, Some(serial), &entry.key);
        let children = self.visible_children(serial);
        let limits = self.child_limits(serial)?;
        entry.node.metric_layout[1].set(0);
        entry.node.font_layout[1].set(0);
        let size = entry
            .component
            .measure(
                &MeasureContext {
                    node: &entry.node,
                    reads: &entry.node.metric_layout[1],
                    font_reads: &entry.node.font_layout[1],
                    runtime: self,
                    children: &children,
                },
                limits.available(available),
            )
            .inspect_err(|_| self.note_failure(serial, crate::FailurePhase::Measure))?;
        let size = limits.constrain(size);
        entry.node.measured.set(Some((available, size)));
        Ok(size)
    }

    /// Arranges `serial` at a window origin with a size in its own local pixels,
    /// both carried by `placement`. The two spaces differ only under a view
    /// transform, where the window rectangle is the local size times the
    /// component's accumulated scale.
    fn arrange_node(
        &self,
        serial: u64,
        placement: Rect,
        clip: Rect,
        staged: &mut Vec<Staged>,
    ) -> Result<(), RuntimeError> {
        let entry = &self.entries[&serial];
        let [x, y, w, h] = placement.values();
        let size = self.child_limits(serial)?.constrain(Size::new(w, h)?);
        let scale = entry.node.scale.get();
        let bounds = rect(x, y, size.width() * scale, size.height() * scale)?;
        let inherited_clip = clip;
        let clip = clip.intersection(bounds);
        let children_clip = if entry.component.clips_children() {
            clip
        } else {
            inherited_clip
        };
        let changed =
            entry.node.bounds.get() != Some(bounds) || entry.node.size.get() != Some(size);
        if !changed
            && !entry.node.layout_dirty.get()
            && entry.node.clip.get() == Some(clip)
            && entry.node.children_clip.get() == Some(children_clip)
        {
            return Ok(());
        }
        // A leaf's arrangement is local to its size. Translation and ancestor
        // clipping only change composition/hit geometry; retain prepared text
        // and the metric/font reads that made that local arrangement valid.
        if entry.children.is_empty()
            && !entry.node.layout_dirty.get()
            && entry.node.size.get() == Some(size)
        {
            staged.push(Staged {
                serial,
                bounds,
                size,
                clip,
                children_clip,
                view_offset: entry.node.view_offset.get(),
                arranged: None,
            });
            return Ok(());
        }
        let children = self.visible_children(serial);
        // A composite component that is only moved, as scrolling or a pan moves
        // it, keeps the placements its last `arrange` gave the same children:
        // they are in its own space, which a translation does not change. Only
        // their window geometry is derived again below, without calling it.
        let retained = (!entry.node.layout_dirty.get()
            && entry.node.size.get() == Some(size)
            && *entry.node.arranged_children.borrow() == children)
            .then(|| {
                children
                    .iter()
                    .map(|child| self.entries[child].node.placement.get())
                    .collect::<Option<Vec<_>>>()
            })
            .flatten();
        // Timed only when the component arranges, and like before over the
        // children it places too, so inclusive and self times keep their meaning.
        let _timing = retained.is_none().then(|| {
            self.shared
                .profiler
                .span(crate::ProfileOperation::Arrange, Some(serial), &entry.key)
        });
        let fresh = retained.is_none();
        let placements = match retained {
            Some(placements) => placements,
            None => {
                entry.node.metric_layout[2].set(0);
                entry.node.font_layout[2].set(0);
                let mut context = ArrangeContext {
                    measure: MeasureContext {
                        node: &entry.node,
                        reads: &entry.node.metric_layout[2],
                        font_reads: &entry.node.font_layout[2],
                        runtime: self,
                        children: &children,
                    },
                    placements: vec![None; children.len()],
                    clips: vec![None; children.len()],
                };
                entry
                    .component
                    .arrange(&mut context, size)
                    .inspect_err(|_| self.note_failure(serial, crate::FailurePhase::Arrange))?;
                context
                    .placements
                    .into_iter()
                    .zip(context.clips)
                    .map(|(placement, clip)| placement.map(|placement| (placement, clip)))
                    .collect::<Option<Vec<_>>>()
                    .ok_or(LayoutError::InvalidChild)
                    // A child the container left unplaced is its failure.
                    .inspect_err(|_| self.note_failure(serial, crate::FailurePhase::Arrange))?
            }
        };
        let arranged = fresh.then(|| (children.clone(), placements.clone()));
        for (child, (placement, local_clip)) in children.iter().zip(placements) {
            let [_, _, w, h] = placement.values();
            let [bx, by, _, _] = bounds.values();
            // Content space reaches the window through this component's origin,
            // its view offset (in its own local pixels) and the accumulated scale
            // its children inherit. All three are the identity outside a view.
            let view = entry.component.child_transform();
            let content_scale = entry.node.content_scale.get();
            let content_origin = (
                f64::from(bx) + f64::from(scale) * f64::from(view.offset().x()),
                f64::from(by) + f64::from(scale) * f64::from(view.offset().y()),
            );
            let shown = crate::transform::map_rect(placement, content_origin, content_scale)
                .ok_or(crate::GeometryError::NonFinite)?;
            // Window origin with the child's local size, as `arrange_node` takes it.
            let absolute = rect(shown.x(), shown.y(), w, h)?;
            let child_clip = if let Some(local_clip) = local_clip {
                crate::transform::map_rect(local_clip, content_origin, content_scale)
                    .ok_or(crate::GeometryError::NonFinite)?
                    .intersection(children_clip)
            } else {
                children_clip
            };
            if let Some(kind) = self.surface_kind(*child) {
                let viewport = self.window.shared.viewport.get();
                let window = rect(0.0, 0.0, viewport.width(), viewport.height())?;
                if kind.modal() {
                    let scope = if kind == crate::SurfaceKind::ViewDialog {
                        bounds.intersection(children_clip)
                    } else {
                        window
                    };
                    self.arrange_node(*child, scope, scope, staged)?;
                    continue;
                }
                if kind == crate::SurfaceKind::Notification {
                    let width = w.min(viewport.width());
                    let height = h.min(viewport.height());
                    let surface = rect(
                        viewport.width() - width,
                        viewport.height() - height,
                        width,
                        height,
                    )?;
                    self.arrange_node(*child, surface, window, staged)?;
                    continue;
                }
                if matches!(
                    kind,
                    crate::SurfaceKind::Submenu | crate::SurfaceKind::ContextMenu
                ) {
                    let width = w.min(viewport.width());
                    let height = h.min(viewport.height());
                    let [px, py, _, _] = absolute.values();
                    let left =
                        if kind == crate::SurfaceKind::Submenu && px + width > viewport.width() {
                            (bx - width).max(0.0)
                        } else {
                            px.clamp(0.0, (viewport.width() - width).max(0.0))
                        };
                    let top = py.clamp(0.0, (viewport.height() - height).max(0.0));
                    self.arrange_node(*child, rect(left, top, width, height)?, window, staged)?;
                    continue;
                }
                let [ax, ay, _, ah] = bounds.values();
                let width = w.min(viewport.width());
                let height = h.min(viewport.height());
                let left = ax.clamp(0.0, (viewport.width() - width).max(0.0));
                let below = (viewport.height() - ay - ah).max(0.0);
                let above = ay.max(0.0);
                let top = if height <= below || below >= above {
                    (ay + ah).min(viewport.height())
                } else {
                    (ay - height).max(0.0)
                };
                let height = height.min(if top >= ay + ah { below } else { above });
                let surface = rect(left, top, width, height)?;
                let window = rect(0.0, 0.0, viewport.width(), viewport.height())?;
                self.arrange_node(*child, surface, window, staged)?;
            } else {
                self.arrange_node(*child, absolute, child_clip, staged)?;
            }
        }
        staged.push(Staged {
            serial,
            bounds,
            size,
            clip,
            children_clip,
            view_offset: entry.component.child_transform().offset(),
            arranged,
        });
        Ok(())
    }

    pub(super) fn layout(&mut self) -> Result<(), RuntimeError> {
        if !self.window.shared.layout_pending.get() {
            return Ok(());
        }
        let _timing = self
            .shared
            .profiler
            .span(crate::ProfileOperation::Layout, None, "");
        let previous = self.shared.phase.replace(Phase::Layout);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let viewport = self.window.shared.viewport.get();
        let size = Size::new(viewport.width(), viewport.height())?;
        let root = Rect::from_size(size);
        let mut staged = Vec::new();
        let preview = self.drag_preview_root();
        for serial in self.root_order() {
            let entry = &self.entries[&serial];
            if !entry.composed || !entry.node.visible.get() {
                continue;
            }
            if Some(serial) == preview {
                // The preview keeps its preferred size and follows the pointer,
                // clipped by the window rather than kept inside it.
                let preferred = self.measure_node(serial, Constraints::default())?;
                let origin = self.drag_preview_origin().unwrap_or_default();
                let placement = rect(
                    origin.x(),
                    origin.y(),
                    preferred.width(),
                    preferred.height(),
                )?;
                self.arrange_node(serial, placement, root, &mut staged)?;
                continue;
            }
            self.measure_node(serial, Constraints::bounded(size))?;
            self.arrange_node(serial, root, root, &mut staged)?;
        }
        let mut moved = false;
        for Staged {
            serial,
            bounds,
            size,
            clip,
            children_clip,
            view_offset,
            arranged,
        } in staged
        {
            let entry = &self.entries[&serial];
            if let Some((children, placements)) = arranged {
                for (child, placement) in children.iter().zip(placements) {
                    if let Some(child) = self.entries.get(child) {
                        child.node.placement.set(Some(placement));
                    }
                }
                *entry.node.arranged_children.borrow_mut() = children;
            }
            if self.drag_active()
                && (entry.node.bounds.get() != Some(bounds) || entry.node.clip.get() != Some(clip))
                && !self.in_drag_preview(serial)
            {
                moved = true;
            }
            entry.node.view_offset.set(view_offset);
            // Local drawing depends on the local allocation size, not its window
            // origin or the inherited clip (which is applied during composition).
            // A pure translation therefore reuses recorded content geometry and
            // text. A changed view scale arrives as a layout invalidation.
            if entry.node.size.get() != Some(size) || entry.node.layout_dirty.get() {
                entry.paint_dirty.set(true);
            }
            entry.node.bounds.set(Some(bounds));
            entry.node.size.set(Some(size));
            if entry.node.clip.get().is_some_and(crate::Rect::is_empty) != clip.is_empty() {
                self.shared.source_visibility_dirty.set(true);
            }
            entry.node.clip.set(Some(clip));
            entry.node.children_clip.set(Some(children_clip));
            entry.node.layout_dirty.set(false);
            if !entry.node.is_presented() {
                if entry.node.deadline.get().is_some() {
                    entry.node.presentation_suspended.set(true);
                    entry.node.stop_deadline();
                }
            } else if entry.node.presentation_suspended.replace(false) {
                entry.node.invalidate(Invalidation::Paint);
            }
            #[cfg(debug_assertions)]
            self.validator.geometry(
                serial,
                bounds.values(),
                clip.values(),
                (entry.component.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<crate::VirtualList>()
                    .map(crate::VirtualList::extent_for_validation),
            );
        }
        self.window.shared.layout_pending.set(false);
        if moved {
            self.note_drag_layout_change();
        }
        Ok(())
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use crate::*;

    fn resize(ui: &mut Runtime, width: f32) -> Result<(), Box<dyn std::error::Error>> {
        ui.set_viewport(Viewport::new(width, 400.0, 1.0)?)?;
        ui.flush(ui.now())?;
        Ok(())
    }

    #[test]
    fn a_move_requested_during_a_flush_is_validated_once_the_flush_settles()
    -> Result<(), Box<dyn std::error::Error>> {
        // A narrowing toolbar moves groups into its overflow surface from its
        // update, and shows its More button before the next arrangement places
        // it. Validating at the move saw the button's stale wider position.
        let mut ui = Runtime::new();
        let button = |key: &str| Element::new(key, Button::new(key, |_| Ok(())));
        let toolbar = Toolbar::new(
            &ui,
            [
                ToolGroup::new("first", [button("a"), button("b")]).essential(),
                ToolGroup::spacer("spacer"),
                ToolGroup::new("second", [button("c"), button("d")]),
                ToolGroup::new("third", [button("e"), button("f")]).collapse_first(),
            ],
        )?;
        resize(&mut ui, 900.0)?;
        ui.mount_tree(Element::new(
            "root",
            Column::new([Element::new("toolbar", toolbar)]),
        ))?;
        ui.flush(ui.now())?;
        for width in [200.0, 900.0, 200.0] {
            resize(&mut ui, width)?;
        }
        assert!(ui.validation_report().issues.is_empty());
        assert_eq!(
            ui.validator.logged_lines, 0,
            "no transient issue was logged"
        );
        Ok(())
    }

    /// Replaces its children when `changed` turns true.
    struct Restructuring {
        changed: ReadSignal<bool>,
    }
    impl Component for Restructuring {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            let changed = cx.read(&self.changed)?;
            let last = if changed {
                Element::new("added", Label::new("added"))
            } else {
                Element::new(
                    "doomed",
                    Doomed {
                        changed: self.changed.clone(),
                    },
                )
            };
            cx.request_children(vec![Element::new("kept", Label::new("kept")), last]);
            Ok(())
        }
        fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            let mut height = 0.0;
            for index in 0..cx.len() {
                height += cx.measure(index, Constraints::default())?.height();
            }
            Ok(Size::new(200.0, height)?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
            let mut y = 0.0;
            for index in 0..cx.len() {
                let height = cx.measure(index, Constraints::default())?.height();
                cx.place(index, Rect::new(0.0, y, size.width(), height)?)?;
                y += height;
            }
            Ok(())
        }
    }
    /// Requests its own removal when `changed` turns true.
    struct Doomed {
        changed: ReadSignal<bool>,
    }
    impl Component for Doomed {
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            if cx.read(&self.changed)? {
                cx.request_unmount();
            }
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(200.0, 20.0)?)
        }
    }

    #[test]
    fn requested_children_and_removals_are_validated_once_the_flush_settles()
    -> Result<(), Box<dyn std::error::Error>> {
        // Requested children and removals apply inside a flush, before the
        // changed tree is arranged; each used to run a validation pass on the
        // previous arrangement's bounds.
        let mut ui = Runtime::new();
        resize(&mut ui, 400.0)?;
        let changed = ui.signal(false);
        let root = ui.mount_tree(Element::new(
            "root",
            Restructuring {
                changed: changed.read_only(),
            },
        ))?;
        ui.flush(ui.now())?;
        let passes = ui.validator.passes;
        changed.set(true)?;
        ui.flush(ui.now())?;
        let keys = ui
            .children(&root)?
            .iter()
            .map(|child| ui.key(child).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(keys, ["kept", "added"]);
        assert_eq!(
            ui.validator.passes,
            passes + 1,
            "only the settled flush validated"
        );
        assert!(ui.validation_report().issues.is_empty());
        assert_eq!(ui.validator.logged_lines, 0);
        Ok(())
    }
}
