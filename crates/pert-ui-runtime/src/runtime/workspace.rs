//! Capturing and restoring the workspace arrangement of a mounted tree.
use super::*;
use crate::layout::Addressing;
use crate::{
    IgnoredEntry, ViewState, Workspace, WorkspacePath, WorkspaceReport, WorkspaceSettings,
};

/// The mounted addresses a capture or restore works with.
#[derive(Default)]
struct WorkspaceIndex {
    /// Participants by their validated path, with the state their
    /// classification read; each path is unique. Capture and restore use this
    /// state instead of asking again, so a participant answers once per call.
    participants: BTreeMap<WorkspacePath, (u64, ViewState)>,
    /// Every addressed raw path, `true` when several components share it.
    addressed: BTreeMap<Vec<String>, bool>,
}

impl Runtime {
    /// Captures the restorable arrangement of everything mounted in this runtime.
    ///
    /// The result is an owned document with no live handles: it may outlive the
    /// components it came from and be stored, compared or discarded freely.
    /// [`Workspace::encode`] turns it into the text a host writes wherever it
    /// keeps such things; this library never chooses that place, writes a file or
    /// saves on a timer.
    ///
    /// Capture is explicit and synchronous. It schedules nothing, invalidates
    /// nothing and measures nothing, and signal or parameter writes are rejected
    /// while it runs. Entries are ordered by path, so capturing an unchanged
    /// workspace twice produces identical text.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::Runtime`](crate::WorkspaceError::Runtime) after
    /// shutdown, and a document error when the capture would exceed a documented
    /// limit, when a participant's path is not a valid `WorkspacePath` (for
    /// example a key longer than a segment may be), or when a participant
    /// shares its path with another component, which means a container
    /// declared its slots incorrectly. Other components' keys are not
    /// validated: they are never written. A participant that cannot build its
    /// state fails the whole capture with
    /// [`WorkspaceError::Participant`](crate::WorkspaceError::Participant),
    /// naming its path; a capture never succeeds without a participant that
    /// failed.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Runtime, RuntimeError, SplitAxis, SplitPane, Label};
    /// let mut ui = Runtime::new();
    /// ui.mount_tree(Element::new(
    ///     "main",
    ///     SplitPane::new(
    ///         SplitAxis::LeftRight,
    ///         Element::new("left", Label::new("Left")),
    ///         Element::new("right", Label::new("Right")),
    ///     ),
    /// ))?;
    /// let workspace = ui.capture_workspace()?;
    /// assert_eq!(workspace.len(), 1);
    /// let text = workspace.encode();
    /// assert!(text.starts_with("pert-ui-workspace 1\n"));
    /// # Ok::<(), pert_ui::WorkspaceError>(())
    /// ```
    pub fn capture_workspace(&self) -> Result<Workspace, crate::WorkspaceError> {
        self.shared.readable()?;
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        let theme = self.theme();
        let settings = WorkspaceSettings::default()
            .with_zoom(Some(self.shared.zoom.get()))
            .with_theme_mode(Some(theme.mode()?))
            .with_density(Some(theme.density()?));
        let mounted = self.workspace_index()?;
        let entries: Vec<_> = mounted
            .participants
            .into_iter()
            .map(|(path, (_, state))| (path, state))
            .collect();
        Workspace::new(settings, entries)
    }

    /// Applies a decoded document to the mounted tree, reporting every entry.
    ///
    /// Settings are applied first, then each entry in document order. An entry
    /// whose view is not mounted yet is retained and applied when a matching path
    /// mounts, at most once; [`Runtime::workspace_report`] reflects those later
    /// applications. A second restore replaces what the first one retained.
    ///
    /// While a drag session is active ([`Runtime::drag_active`]) no entry is
    /// applied: a mounted participant's entry is reported as
    /// [`RestoreRejection::Busy`](crate::RestoreRejection::Busy), and so is a
    /// retained entry whose view mounts during the drag. Either way the entry
    /// is spent; restore again after the drag to apply it. The document's
    /// settings are not held back: zoom, theme mode and density apply during a
    /// drag like at any other time. A changed zoom or geometry-affecting density
    /// cancels captured gestures, focused held-key actions and registered edits
    /// when geometry next settles. An active drag ends with
    /// [`DragOutcome::Cancelled`](crate::DragOutcome::Cancelled) to its source.
    /// Density affects geometry when a visible layout reader uses a changed
    /// metric. Unchanged settings keep gestures active. Hosts defer changed
    /// settings when their interaction policy requires preserving gestures.
    ///
    /// Saved extents, proportions and offsets are preferences, not measured
    /// results: they are written back as they were captured, and the next layout
    /// clamps them against the viewport it then has. Restoring state for a hidden
    /// view schedules no work for it; the view records the change when revealed.
    /// Focus, pointer capture, gestures and drafts are never restored, because
    /// they were never captured.
    ///
    /// # Errors
    /// Returns [`WorkspaceError::Runtime`](crate::WorkspaceError::Runtime) when
    /// the runtime is closed, when called during update, layout, paint,
    /// inspection or delivery, or when a setting is refused. A refused setting
    /// leaves the entries unapplied. A participant with an invalid or shared
    /// mounted path, or a document entry whose path several mounted components
    /// share, is rejected before settings or entries change, and so is a
    /// mounted participant that cannot build its own state
    /// ([`WorkspaceError::Participant`](crate::WorkspaceError::Participant)).
    pub fn restore_workspace(
        &mut self,
        workspace: &Workspace,
    ) -> Result<WorkspaceReport, crate::WorkspaceError> {
        self.shared.writable()?;
        let mounted = self.workspace_index()?;
        for (path, _) in workspace.entries() {
            let segments: Vec<_> = path.segments().map(str::to_owned).collect();
            if mounted.addressed.get(&segments) == Some(&true) {
                return Err(crate::WorkspaceError::DuplicatePath(path.to_string()));
            }
        }
        let settings = workspace.settings();
        if let Some(zoom) = settings.zoom() {
            self.ui_settings().set_zoom(zoom)?;
        }
        let theme = self.theme();
        if let Some(mode) = settings.theme_mode() {
            theme.set_mode(mode)?;
        }
        if let Some(density) = settings.density() {
            theme.set_density(density)?;
        }
        let mut report = WorkspaceReport::default();
        let mut pending = Vec::new();
        for (path, state) in workspace.entries() {
            let segments: Vec<_> = path.segments().map(str::to_owned).collect();
            if mounted.addressed.contains_key(&segments) {
                let Some((serial, mounted_state)) = mounted.participants.get(path) else {
                    report
                        .ignored
                        .push((path.clone(), IgnoredEntry::NotAParticipant));
                    continue;
                };
                // Restoring under a drag would move what the pointer is over.
                // Settings above were applied regardless, as documented.
                if self.drag_active() {
                    report
                        .rejected
                        .push((path.clone(), crate::RestoreRejection::Busy));
                    continue;
                }
                self.apply_workspace_entry(
                    *serial,
                    path,
                    state,
                    Ok(Some(mounted_state.clone())),
                    &mut report,
                );
            } else {
                report.pending.push(path.clone());
                pending.push((path.clone(), state.clone()));
            }
        }
        self.workspace_pending = pending;
        self.workspace_report = report.clone();
        Ok(report)
    }

    /// What the last [`Runtime::restore_workspace`] did, including entries that
    /// applied later because their view mounted after the restore.
    ///
    /// Empty before the first restore.
    pub fn workspace_report(&self) -> &WorkspaceReport {
        &self.workspace_report
    }

    /// Every addressable mounted component by its raw path segments.
    ///
    /// Nonparticipants are included, so that one can never silently shadow a
    /// saved target, but only participants must have a valid `WorkspacePath`:
    /// an ordinary component keyed with text a document cannot hold, such as a
    /// file path or a very long row key, does not make the workspace unusable.
    /// A collision is an error when a participant takes part in it; a collision
    /// among nonparticipants only matters to a document entry addressing it.
    fn workspace_index(&self) -> Result<WorkspaceIndex, crate::WorkspaceError> {
        let mut mounted: BTreeMap<Vec<String>, Vec<u64>> = BTreeMap::new();
        for serial in self.entries.keys() {
            if let Some(segments) = self.workspace_segments(*serial) {
                mounted.entry(segments).or_default().push(*serial);
            }
        }
        let mut index = WorkspaceIndex::default();
        for (segments, serials) in mounted {
            let mut participant = None;
            let mut participants = 0;
            for serial in &serials {
                match self.read_workspace_state(*serial) {
                    Ok(Some(state)) => {
                        participants += 1;
                        participant.get_or_insert((*serial, state));
                    }
                    Ok(None) => {}
                    Err(source) => return Err(participant_error(&display(&segments), source)),
                }
            }
            if let Some(participant) = participant {
                let path = WorkspacePath::new(segments.iter().map(String::as_str))?;
                if serials.len() > 1 || participants > 1 {
                    return Err(crate::WorkspaceError::DuplicatePath(path.to_string()));
                }
                index.participants.insert(path, participant);
            }
            index.addressed.insert(segments, serials.len() > 1);
        }
        Ok(index)
    }

    /// Asks one mounted component for its workspace state in the inspection
    /// phase, so that, as [`Component::workspace_state`](crate::Component::workspace_state)
    /// documents, a write it attempts is refused whichever operation asks.
    fn read_workspace_state(
        &self,
        serial: u64,
    ) -> Result<Option<ViewState>, crate::WorkspaceError> {
        let Some(entry) = self.entries.get(&serial) else {
            return Ok(None);
        };
        let previous = self.shared.phase.replace(Phase::Inspect);
        let _guard = InspectionGuard {
            shared: self.shared.clone(),
            previous,
        };
        entry
            .component
            .workspace_state_with_children(&self.child_snapshot(serial))
    }

    /// The addressable path of one mounted component, or `None` when structural.
    pub(super) fn workspace_segments(&self, serial: u64) -> Option<Vec<String>> {
        // A drag preview is transient presentation, never an arrangement.
        if self.in_drag_preview(serial) {
            return None;
        }
        let entry = self.entries.get(&serial)?;
        let mut segments = vec![match &entry.addressing {
            Addressing::Own => entry.key.clone(),
            Addressing::Slot(name) | Addressing::Part(name) => name.clone(),
            Addressing::Structural => return None,
        }];
        let mut current = entry.node.logical_parent_serial();
        while let Some(serial) = current {
            let entry = self.entries.get(&serial)?;
            // Preview descendants mount before the drag session is registered.
            // Their transient ancestor is already known at that point.
            let component: &dyn std::any::Any = &*entry.component;
            if component.is::<super::drag::PreviewRoot>() {
                return None;
            }
            match &entry.addressing {
                Addressing::Own => segments.push(entry.key.clone()),
                Addressing::Slot(name) => segments.push(name.clone()),
                Addressing::Part(_) | Addressing::Structural => {}
            }
            current = entry.node.logical_parent_serial();
        }
        segments.reverse();
        Some(segments)
    }

    /// Offers one entry to the component mounted at its path, given the state
    /// that component reported when it was classified.
    fn apply_workspace_entry(
        &mut self,
        serial: u64,
        path: &WorkspacePath,
        state: &ViewState,
        mounted: Result<Option<ViewState>, crate::WorkspaceError>,
        report: &mut WorkspaceReport,
    ) {
        if !self.entries.contains_key(&serial) {
            report
                .ignored
                .push((path.clone(), IgnoredEntry::NotAParticipant));
            return;
        }
        let mounted = match mounted {
            Ok(Some(mounted)) => mounted,
            Ok(None) => {
                report
                    .ignored
                    .push((path.clone(), IgnoredEntry::NotAParticipant));
                return;
            }
            Err(error) => {
                report
                    .rejected
                    .push((path.clone(), crate::RestoreRejection::State(error)));
                return;
            }
        };
        if mounted.kind() != state.kind() {
            report.ignored.push((
                path.clone(),
                IgnoredEntry::KindMismatch {
                    found: state.kind().to_owned(),
                    mounted: mounted.kind().to_owned(),
                },
            ));
            return;
        }
        let children = self.child_snapshot(serial);
        let Some(entry) = self.entries.get_mut(&serial) else {
            return;
        };
        match entry
            .component
            .restore_workspace_state_with_children(state, &children)
        {
            Ok(invalidation) => {
                entry.node.clone().invalidate(invalidation);
                report.applied.push(path.clone());
            }
            Err(rejection) => report.rejected.push((path.clone(), rejection)),
        }
    }

    /// Applies pending entries only after the whole mount/reconcile has committed.
    pub(super) fn apply_pending_workspace(&mut self) {
        if self.workspace_pending.is_empty() {
            return;
        }
        // Address strings are enough to match already validated pending paths.
        // Include nonparticipants and retain collisions explicitly.
        let mut mounted = BTreeMap::new();
        for serial in self.entries.keys() {
            if let Some(segments) = self.workspace_segments(*serial) {
                mounted
                    .entry(segments)
                    .and_modify(|value| *value = None)
                    .or_insert(Some(*serial));
            }
        }
        let pending = std::mem::take(&mut self.workspace_pending);
        let mut report = std::mem::take(&mut self.workspace_report);
        for (path, state) in pending {
            let segments: Vec<_> = path.segments().map(str::to_owned).collect();
            match mounted.get(&segments) {
                Some(Some(serial)) => {
                    report.pending.retain(|pending| *pending != path);
                    // A failure is reported by `apply_workspace_entry` below.
                    let mounted_state = self.read_workspace_state(*serial);
                    let participant = matches!(mounted_state, Ok(Some(_)));
                    // As in `restore_workspace`: applying under a drag would
                    // move what the pointer is over, so a view that mounts
                    // during one, such as lazily built tab content, has its
                    // entry reported busy rather than applied.
                    if participant && self.drag_active() {
                        report.rejected.push((path, crate::RestoreRejection::Busy));
                    } else {
                        self.apply_workspace_entry(
                            *serial,
                            &path,
                            &state,
                            mounted_state,
                            &mut report,
                        );
                    }
                }
                Some(None) => {
                    report.pending.retain(|pending| *pending != path);
                    report.ignored.push((path, IgnoredEntry::AmbiguousPath));
                }
                None => self.workspace_pending.push((path, state)),
            }
        }
        self.workspace_report = report;
    }
}

/// The participant failure for the component at `path`.
fn participant_error(path: &str, source: crate::WorkspaceError) -> crate::WorkspaceError {
    crate::WorkspaceError::Participant {
        path: path.to_owned(),
        source: Box::new(source),
    }
}

/// Writes raw path segments as [`WorkspacePath`] displays them, falling back to
/// the segments joined by `/` when they do not form a valid path.
fn display(segments: &[String]) -> String {
    WorkspacePath::new(segments.iter().map(String::as_str))
        .map_or_else(|_| segments.join("/"), |path| path.to_string())
}
