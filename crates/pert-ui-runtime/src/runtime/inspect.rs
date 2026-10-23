//! Owned inspection snapshots, queries and selector resolution.
use super::*;

impl Runtime {
    /// Clears retained events and the eviction count; event serials never
    /// restart, and the history generation advances so a poller can tell.
    pub fn clear_event_history(&self) {
        let mut history = self.shared.history.borrow_mut();
        history.records.clear();
        history.dropped = 0;
        history.generation = history.generation.saturating_add(1);
    }

    /// The serials from `start` up through its lifetime owners, for a cheap
    /// "is focus or capture inside this child" test per child.
    pub(super) fn ownership_chain(&self, start: Option<u64>) -> Vec<u64> {
        let mut chain = Vec::new();
        let mut current = start;
        while let Some(serial) = current {
            chain.push(serial);
            current = self
                .entries
                .get(&serial)
                .and_then(|entry| entry.node.logical_parent_serial());
        }
        chain
    }

    /// Captures owned, versioned runtime/component/parameter state on demand.
    ///
    /// May allocate proportional to the inspected state; never runs automatically
    /// on a frame or timer. The result is Send/Sync and may be handed to the
    /// application's tooling thread, with no live UI references or mutation
    /// capability.
    /// Arbitrary host data requires explicit contribution via `inspect_with`.
    /// Dead parameter inspection references are pruned during capture; identity
    /// reservations remain intact for the runtime's lifetime.
    #[must_use]
    pub fn inspect(&self) -> crate::InspectionSnapshot {
        self.inspect_with(BTreeMap::new)
    }

    /// Captures runtime state plus an explicit application-state provider.
    ///
    /// Typed [`Semantics`](crate::Semantics) are resolved here: a component that
    /// names its children, such as a `Field`, gives its name to an immediate child
    /// that has none and records itself in `labelled_by`, and passes its
    /// description and invalid state to every immediate child that reports
    /// semantics.
    ///
    /// The provider runs synchronously on the UI thread with signal/parameter
    /// writes forbidden, and must not mutate other host state or perform I/O.
    /// Report only data intended for the connected inspection tool. This contract
    /// exposes owned observations, not arbitrary Rust heap serialization.
    pub fn inspect_with(
        &self,
        application: impl FnOnce() -> BTreeMap<String, crate::InspectValue>,
    ) -> crate::InspectionSnapshot {
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let application = application();
        let mut components: Vec<_> = self
            .entries
            .values()
            .map(|entry| self.describe_component(entry, crate::Detail::Full))
            .collect();
        name_children(&mut components, &self.structural_serials());
        let parameters = {
            let mut registered = self.parameters.borrow_mut();
            registered.retain(|weak| weak.strong_count() > 0);
            registered
                .iter()
                .filter_map(Weak::upgrade)
                .map(|parameter| parameter.inspect())
                .collect()
        };
        let history = self.shared.history.borrow();
        crate::InspectionSnapshot {
            schema_version: crate::InspectionSnapshot::SCHEMA_VERSION,
            windows: self.window_inspections(),
            sources: self.shared.source_inspections(),
            validation: self.validation_report(),
            mode: history.mode,
            overlay: self.shared.overlay.get(),
            drag: self.drag_inspection(),
            at: self.shared.now.get(),
            next_deadline: self.application_deadline(),
            components,
            parameters,
            pressed_keys: self.keyboard.pressed().collect(),
            application,
            events: history.records.iter().cloned().collect(),
            dropped_events: history.dropped,
            history_generation: history.generation,
        }
    }

    /// Describes one mounted component at the requested detail; only
    /// [`Detail::Semantics`](crate::Detail::Semantics) and above run its
    /// `Component::inspect`.
    /// The incarnations described as internal structure (`Element::structural`).
    fn structural_serials(&self) -> std::collections::BTreeSet<u64> {
        self.entries
            .iter()
            .filter(|(_, entry)| entry.addressing == crate::layout::Addressing::Structural)
            .map(|(serial, _)| *serial)
            .collect()
    }

    /// The name an element label gives a component that reports none: its own
    /// element's label, or, for internal structure a composite built
    /// (`Element::structural`), the label of the nearest described ancestor
    /// when that ancestor reports no semantics of its own, so the label an
    /// application puts on a composite reaches the part that represents it.
    pub(super) fn element_label_name(&self, entry: &Entry) -> Option<String> {
        let named = |entry: &Entry| {
            entry
                .label
                .as_ref()
                .filter(|label| !label.trim().is_empty())
                .cloned()
        };
        if let Some(label) = named(entry) {
            return Some(label);
        }
        let mut current = entry;
        while current.addressing == crate::layout::Addressing::Structural {
            current = self.entries.get(&current.node.logical_parent_serial()?)?;
            if current.addressing != crate::layout::Addressing::Structural {
                return named(current).filter(|_| current.component.inspect().semantics.is_none());
            }
        }
        None
    }

    fn describe_component(
        &self,
        entry: &Entry,
        detail: crate::Detail,
    ) -> crate::ComponentInspection {
        let serial = entry.node.serial;
        let mut details = if detail >= crate::Detail::Semantics {
            entry.component.inspect()
        } else {
            crate::ComponentDetails::default()
        };
        if detail == crate::Detail::Semantics {
            details = crate::ComponentDetails {
                semantics: details.semantics,
                ..Default::default()
            };
        }
        // Public fields can bypass the builders' invariants; a snapshot
        // never exposes a protected value or an unsorted or disabled action.
        if let Some(semantics) = details.semantics.as_mut() {
            semantics.normalize();
        }
        // An element's label names a component that reports no name of its
        // own, so an application can name any control where it describes it.
        if let Some(semantics) = details.semantics.as_mut()
            && semantics.name.is_none()
        {
            semantics.name = self.element_label_name(entry);
        }
        if detail == crate::Detail::Full {
            if let Some(kind) = self.surface_kind(serial) {
                details.validation.overlap = crate::OverlapPolicy::Allow;
                details.properties.insert(
                    "presentation_layer".into(),
                    crate::InspectValue::Text(format!("{kind:?}")),
                );
                if let Some(owner) = entry.node.logical_parent_serial() {
                    details.properties.insert(
                        "anchor_owner".into(),
                        crate::InspectValue::Number(owner as f64),
                    );
                }
            }
            if self.stacked_child(serial) {
                details.validation.overlap = crate::OverlapPolicy::Allow;
            }
        }
        if detail >= crate::Detail::Geometry && entry.composed {
            details.box_model.border_box = entry.node.bounds.get().map(crate::Rect::values);
            details.validation.clip = entry.node.clip.get().map(crate::Rect::values);
        }
        crate::ComponentInspection {
            labelled_by: None,
            id: serial,
            key: entry.key.clone(),
            path: self.workspace_segments(serial),
            parent: entry.node.logical_parent_serial(),
            visible: entry.node.visible.get(),
            focused: entry.node.window().focused.get() == Some(serial),
            window: entry.node.window().serial,
            presentation_parent: entry.node.presentation_parent_serial(),
            view_scale: entry.node.scale.get(),
            dirty: entry.node.dirty.get(),
            subscriptions: entry.subscriptions.len(),
            next_deadline: match (entry.node.deadline.get(), entry.node.timer.get()) {
                (Some(update), Some(timer)) => Some(update.min(timer)),
                (value, None) | (None, value) => value,
            },
            details,
        }
    }

    /// Answers a [query](crate::InspectionQuery) on the UI thread, with no
    /// application properties.
    ///
    /// Three costs, each stated on its own. Selection: an id is one lookup;
    /// a key, focus or window selector compares retained state of every
    /// mounted component; a path selector builds the address of every
    /// mounted component, each a walk up its ancestors. A semantic selector
    /// runs `Component::inspect` on every candidate it searches (all mounted
    /// components unless `within` narrows it), whatever detail or limit the
    /// query has, since role and name are only known that way. Description:
    /// `Component::inspect` runs for the returned components, and the parent
    /// that names one, only at [`Detail::Semantics`](crate::Detail::Semantics)
    /// and above. Nothing is scheduled and no index is kept.
    #[must_use]
    pub fn query(&self, query: &crate::InspectionQuery) -> crate::QueryResult {
        self.query_with(query, BTreeMap::new)
    }

    /// Answers a query, with `application` providing the exposed application
    /// properties when the query asks for them; the provider runs under the
    /// same restrictions as in [`inspect_with`](Self::inspect_with).
    pub fn query_with(
        &self,
        query: &crate::InspectionQuery,
        application: impl FnOnce() -> BTreeMap<String, crate::InspectValue>,
    ) -> crate::QueryResult {
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let application = query.wants_application().then(application);
        let selected: Vec<u64> = query
            .selector()
            .map(|selector| self.resolve_selector(selector, None))
            .unwrap_or_default();
        let selected_set: std::collections::BTreeSet<u64> = selected.iter().copied().collect();
        let mut returned: Vec<u64> = match query.depth_value() {
            crate::Depth::Matches => selected,
            crate::Depth::Children => self
                .entries
                .values()
                .filter(|entry| {
                    selected_set.contains(&entry.node.serial)
                        || entry
                            .node
                            .logical_parent_serial()
                            .is_some_and(|parent| selected_set.contains(&parent))
                })
                .map(|entry| entry.node.serial)
                .collect(),
            crate::Depth::Subtree => self
                .entries
                .keys()
                .copied()
                .filter(|serial| {
                    self.ownership_chain(Some(*serial))
                        .iter()
                        .any(|ancestor| selected_set.contains(ancestor))
                })
                .collect(),
        };
        let matched = returned.len();
        let truncated = query.limit_value().is_some_and(|limit| limit < matched);
        if let Some(limit) = query.limit_value() {
            returned.truncate(limit);
        }
        let components = self.describe_many(&returned, query.detail_value());
        let history = self.shared.history.borrow();
        crate::QueryResult {
            schema_version: crate::InspectionSnapshot::SCHEMA_VERSION,
            at: self.shared.now.get(),
            components,
            matched,
            truncated,
            parameters: query.wants_parameters().then(|| {
                let mut registered = self.parameters.borrow_mut();
                registered.retain(|weak| weak.strong_count() > 0);
                registered
                    .iter()
                    .filter_map(Weak::upgrade)
                    .map(|parameter| parameter.inspect())
                    .collect()
            }),
            events: query
                .events_after_value()
                .map(|cursor| crate::EventHistory {
                    events: history
                        .records
                        .iter()
                        .filter(|record| record.sequence > cursor)
                        .cloned()
                        .collect(),
                    dropped_events: history.dropped,
                    generation: history.generation,
                }),
            windows: query.wants_windows().then(|| self.window_inspections()),
            sources: query
                .wants_sources()
                .then(|| self.shared.source_inspections()),
            validation: query.wants_validation().then(|| self.validation_report()),
            application,
        }
    }

    /// Describes `serials` at `detail`, resolving the names a `Field` gives
    /// its control by describing each returned component's parent too.
    fn describe_many(
        &self,
        serials: &[u64],
        detail: crate::Detail,
    ) -> Vec<crate::ComponentInspection> {
        if detail < crate::Detail::Semantics {
            return serials
                .iter()
                .filter_map(|serial| self.entries.get(serial))
                .map(|entry| self.describe_component(entry, detail))
                .collect();
        }
        let wanted: std::collections::BTreeSet<u64> = serials.iter().copied().collect();
        let mut with_parents: std::collections::BTreeSet<u64> = wanted.clone();
        // A naming parent may sit above a chain of internal structure.
        for serial in serials {
            let mut current = self.entries.get(serial);
            while let Some(entry) = current {
                let Some(parent) = entry.node.logical_parent_serial() else {
                    break;
                };
                with_parents.insert(parent);
                if entry.addressing != crate::layout::Addressing::Structural {
                    break;
                }
                current = self.entries.get(&parent);
            }
        }
        let mut components: Vec<_> = with_parents
            .iter()
            .filter_map(|serial| self.entries.get(serial))
            .map(|entry| self.describe_component(entry, detail))
            .collect();
        // Only the described chain's structure matters, so a query costs its
        // returned components' chains rather than a walk of every entry.
        let structural = with_parents
            .iter()
            .copied()
            .filter(|serial| {
                self.entries
                    .get(serial)
                    .is_some_and(|entry| entry.addressing == crate::layout::Addressing::Structural)
            })
            .collect();
        name_children(&mut components, &structural);
        components.retain(|component| wanted.contains(&component.id));
        components
    }

    /// The incarnations `selector` matches, in incarnation order, among
    /// `scope` when given.
    pub(super) fn resolve_selector(
        &self,
        selector: &crate::Selector,
        scope: Option<&std::collections::BTreeSet<u64>>,
    ) -> Vec<u64> {
        let candidates = || {
            self.entries
                .values()
                .filter(move |entry| scope.is_none_or(|scope| scope.contains(&entry.node.serial)))
        };
        let by_path = |accept: &dyn Fn(&[String]) -> bool| -> Vec<u64> {
            candidates()
                .filter(|entry| {
                    self.workspace_segments(entry.node.serial)
                        .is_some_and(|segments| accept(&segments))
                })
                .map(|entry| entry.node.serial)
                .collect()
        };
        match selector {
            crate::Selector::Path(path) => by_path(&|segments| segments == path.as_slice()),
            crate::Selector::Children(path) => {
                by_path(&|segments| segments.len() == path.len() + 1 && segments.starts_with(path))
            }
            crate::Selector::Descendants(path) => {
                by_path(&|segments| segments.len() > path.len() && segments.starts_with(path))
            }
            crate::Selector::Id(id) => self
                .entries
                .get(id)
                .filter(|_| scope.is_none_or(|scope| scope.contains(id)))
                .map(|entry| vec![entry.node.serial])
                .unwrap_or_default(),
            crate::Selector::Key(key) => candidates()
                .filter(|entry| entry.key == *key)
                .map(|entry| entry.node.serial)
                .collect(),
            crate::Selector::Focused => candidates()
                .filter(|entry| entry.node.window().focused.get() == Some(entry.node.serial))
                .map(|entry| entry.node.serial)
                .collect(),
            crate::Selector::Window(window) => candidates()
                .filter(|entry| entry.node.window().serial == *window)
                .map(|entry| entry.node.serial)
                .collect(),
            crate::Selector::Semantic { role, name } => {
                let serials: Vec<u64> = candidates().map(|entry| entry.node.serial).collect();
                self.describe_many(&serials, crate::Detail::Semantics)
                    .into_iter()
                    .filter(|component| {
                        component
                            .details
                            .semantics
                            .as_ref()
                            .is_some_and(|semantics| {
                                semantics.role == *role
                                    && name
                                        .as_ref()
                                        .is_none_or(|name| semantics.name.as_deref() == Some(name))
                            })
                    })
                    .map(|component| component.id)
                    .collect()
            }
            crate::Selector::Within(ancestor, inner) => {
                let ancestors: std::collections::BTreeSet<u64> =
                    self.resolve_selector(ancestor, scope).into_iter().collect();
                let inside: std::collections::BTreeSet<u64> = candidates()
                    .map(|entry| entry.node.serial)
                    .filter(|serial| {
                        self.ownership_chain(Some(*serial))
                            .iter()
                            .skip(1)
                            .any(|chain| ancestors.contains(chain))
                    })
                    .collect();
                self.resolve_selector(inner, Some(&inside))
            }
        }
    }
}

// Resolves the association a naming parent declares: a control inside a `Field`
// has no label of its own, so its name comes from the field that wraps it. The
// association passes through a child that reports no semantics to the internal
// structure it built (`structural` incarnations), so a field names the part of
// a composite, such as a `Select`'s anchor, that represents it.
fn name_children(
    components: &mut [crate::ComponentInspection],
    structural: &std::collections::BTreeSet<u64>,
) {
    let naming: BTreeMap<u64, (Option<String>, Option<String>, bool)> = components
        .iter()
        .filter_map(|component| {
            let semantics = component.details.semantics.as_ref()?;
            semantics.names_children.then(|| {
                (
                    component.id,
                    (
                        semantics.name.clone(),
                        semantics.description.clone(),
                        semantics.invalid,
                    ),
                )
            })
        })
        .collect();
    if naming.is_empty() {
        return;
    }
    let tree: BTreeMap<u64, (Option<u64>, bool)> = components
        .iter()
        .map(|component| {
            (
                component.id,
                (component.parent, component.details.semantics.is_some()),
            )
        })
        .collect();
    for component in components {
        let mut child = component.id;
        let mut parent = component.parent;
        let found = loop {
            let Some(serial) = parent else { break None };
            if let Some(found) = naming.get(&serial) {
                break Some((serial, found));
            }
            match tree.get(&serial) {
                Some(&(above, false)) if structural.contains(&child) => {
                    child = serial;
                    parent = above;
                }
                _ => break None,
            }
        };
        let Some((naming_parent, (name, description, invalid))) = found else {
            continue;
        };
        let Some(semantics) = component.details.semantics.as_mut() else {
            continue;
        };
        if semantics.name.is_none() && name.is_some() {
            semantics.name.clone_from(name);
            component.labelled_by = Some(naming_parent);
        }
        // An invalid naming parent describes its error, which outranks a
        // child's own description of its state.
        if semantics.description.is_none() || (*invalid && description.is_some()) {
            semantics.description.clone_from(description);
        }
        semantics.invalid |= invalid;
    }
}
