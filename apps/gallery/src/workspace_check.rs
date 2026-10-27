//! Two-process native restart check through the same public API as applications.
use pert_ui::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    io::{self, Write},
    path::Path,
    rc::Rc,
    time::Duration,
};

type Capture = Rc<RefCell<Option<Result<Workspace, WorkspaceError>>>>;

struct Finish {
    capture: Capture,
    report: Rc<RefCell<Option<WorkspaceReport>>>,
    painted: Rc<Cell<bool>>,
    interrupted: Rc<Cell<bool>>,
    until: Option<Duration>,
    notifier: Option<Notifier>,
}
impl Component for Finish {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.notifier.is_none() {
            self.notifier = Some(cx.notifier());
        }
        if self.interrupted.get() {
            self.notifier
                .as_ref()
                .ok_or(RuntimeError::Unmounted)?
                .request();
        } else if let Some(until) = self.until {
            if cx.now() < until {
                cx.wake_after(until - cx.now())?;
            } else {
                self.notifier
                    .as_ref()
                    .ok_or(RuntimeError::Unmounted)?
                    .request();
            }
        } else {
            let delay = Duration::from_secs(3);
            self.until = Some(cx.now().checked_add(delay).ok_or(RuntimeError::Exhausted)?);
            cx.wake_after(delay)?;
        }
        Ok(())
    }
    fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
        self.painted.set(true);
        Ok(())
    }
    // Laid over the gallery as a second root: pointer input passes through to
    // the gallery, and the overlap is intended.
    fn hit_test(&self, _: Point) -> bool {
        false
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.validation.overlap = OverlapPolicy::Allow;
        details
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let (capture, report, interrupted) = (
            self.capture.clone(),
            self.report.clone(),
            self.interrupted.clone(),
        );
        let _task = cx.defer_while_shown(move |ui| {
            if !interrupted.get() {
                *capture.borrow_mut() = Some(ui.capture_workspace());
                *report.borrow_mut() = Some(ui.workspace_report().clone());
            }
            let _close = ui.request_close(&ui.primary_window())?;
            Ok(())
        })?;
        Ok(())
    }
    fn visibility_changed(&mut self, visible: bool) {
        if !visible {
            eprintln!("Native restart interrupted: the window was hidden during the check");
            self.interrupted.set(true);
        }
    }
}

fn error(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

/// Changes settings, tab selections and split preferences from gallery defaults.
fn sample(ui: &Runtime) -> Result<Workspace, WorkspaceError> {
    let initial = ui.capture_workspace()?;
    let entries = initial
        .entries()
        .map(|(path, state)| {
            let segments: Vec<_> = path.segments().collect();
            let state = if segments.last() == Some(&"gallery-tabs") {
                state
                    .clone()
                    .with("selected", WorkspaceValue::text("controls-category")?)?
            } else if segments.last() == Some(&"category-pages")
                && segments.contains(&"controls-category")
            {
                state
                    .clone()
                    .with("selected", WorkspaceValue::text("forms")?)?
            } else if state.kind() == "split" {
                // A split keeps its extent in single precision, so the sample
                // uses a ratio that survives that exactly and restores equal.
                ViewState::new("split")?
                    .with("rule", WorkspaceValue::text("ratio")?)?
                    .with("extent", WorkspaceValue::number(0.625).unwrap())?
            } else {
                state.clone()
            };
            Ok((path.clone(), state))
        })
        .collect::<Result<Vec<_>, WorkspaceError>>()?;
    Workspace::new(
        WorkspaceSettings::default()
            .with_theme_mode(Some(ThemeMode::Light))
            .with_density(Some(Density::Compact))
            .with_zoom(Some(UiZoom::new(1.25)?)),
        entries,
    )
}

/// Describes how `actual` differs from `expected`: the paths missing from
/// `actual`, the extra ones, the ones whose state changed, and the settings.
/// `None` when the documents are equal.
fn difference(expected: &Workspace, actual: &Workspace) -> Option<String> {
    if expected == actual {
        return None;
    }
    let index = |workspace: &Workspace| -> BTreeMap<String, ViewState> {
        workspace
            .entries()
            .map(|(path, state)| (path.to_string(), state.clone()))
            .collect()
    };
    let (expected_entries, actual_entries) = (index(expected), index(actual));
    let mut parts = Vec::new();
    let missing: Vec<_> = expected_entries
        .keys()
        .filter(|path| !actual_entries.contains_key(*path))
        .collect();
    if !missing.is_empty() {
        parts.push(format!("missing {missing:?}"));
    }
    let extra: Vec<_> = actual_entries
        .keys()
        .filter(|path| !expected_entries.contains_key(*path))
        .collect();
    if !extra.is_empty() {
        parts.push(format!("extra {extra:?}"));
    }
    let changed: Vec<_> = expected_entries
        .iter()
        .filter_map(|(path, state)| {
            let found = actual_entries.get(path)?;
            (found != state).then(|| format!("{path}: {found:?} instead of {state:?}"))
        })
        .collect();
    if !changed.is_empty() {
        parts.push(format!("changed [{}]", changed.join("; ")));
    }
    if expected.settings() != actual.settings() {
        parts.push(format!(
            "settings {:?} instead of {:?}",
            actual.settings(),
            expected.settings()
        ));
    }
    Some(parts.join(", "))
}

pub(super) fn run(save: bool, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let expected = if save {
        None
    } else {
        Some(Workspace::decode(&std::fs::read_to_string(path)?)?)
    };
    let restore = expected.clone();
    let capture = Capture::default();
    let report = Rc::new(RefCell::new(None));
    let painted = Rc::new(Cell::new(false));
    let interrupted = Rc::new(Cell::new(false));
    let probe = Finish {
        capture: capture.clone(),
        report: report.clone(),
        painted: painted.clone(),
        interrupted: interrupted.clone(),
        until: None,
        notifier: None,
    };
    let gallery = crate::gallery::Gallery::new()?;
    let producer = gallery.producer();
    let result = App::new("pert-ui — Workspace restart check", move |ui| {
        gallery.mount(ui)?;
        let workspace = restore
            .map_or_else(|| sample(ui), Ok)
            .map_err(ApplicationError::new)?;
        let _report = ui
            .restore_workspace(&workspace)
            .map_err(ApplicationError::new)?;
        let _finish = ui.mount_tree(Element::new("restart-check", probe))?;
        Ok(())
    })
    .on_error(|_, _| ErrorAction::Exit)
    .run();
    let joined = producer.join();
    result?;
    joined?;
    if interrupted.get() {
        return Err(error("native restart check window was hidden during the check").into());
    }
    if !painted.get() {
        return Err(error("native check never reached painting").into());
    }
    let actual = capture
        .borrow_mut()
        .take()
        .ok_or_else(|| error("check closed before capture"))??;
    let report = report
        .borrow_mut()
        .take()
        .ok_or_else(|| error("missing restore report"))?;
    if !report.is_complete() {
        return Err(error(format!("incomplete restore: {report:?}")).into());
    }
    if let Some(expected) = expected {
        if let Some(difference) = difference(&expected, &actual) {
            return Err(error(format!("restored capture differs: {difference}")).into());
        }
        writeln!(
            io::stdout().lock(),
            "Native restart: restored document matches the saved capture; all entries applied."
        )?;
    } else {
        // The check never overwrites an existing user file.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(actual.encode().as_bytes())?;
        file.sync_all()?;
        writeln!(
            io::stdout().lock(),
            "Native restart: saved {} after presentation.",
            path.display()
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pert_ui::testing::HostIntegration;

    #[test]
    fn a_hidden_restart_probe_reports_interruption_after_resume() {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 240.0, 1.0).unwrap())
            .unwrap();
        let capture = Capture::default();
        let interrupted = Rc::new(Cell::new(false));
        ui.mount_tree(Element::new(
            "restart-check",
            Finish {
                capture: capture.clone(),
                report: Rc::new(RefCell::new(None)),
                painted: Rc::new(Cell::new(false)),
                interrupted: interrupted.clone(),
                until: None,
                notifier: None,
            },
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(ui.next_deadline().is_some());
        ui.set_window_visible(false).unwrap();
        assert!(interrupted.get());
        assert!(ui.next_deadline().is_none());
        ui.set_window_visible(true).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert!(capture.borrow().is_none());
    }

    fn mounted() -> Runtime {
        let mut ui = Runtime::new();
        crate::gallery::Gallery::new()
            .unwrap()
            .mount(&mut ui)
            .unwrap();
        ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).unwrap())
            .unwrap();
        ui.flush(ui.now()).unwrap();
        ui
    }

    /// The headless half of the native restart check: a changed arrangement
    /// survives encoding and a restore into a fresh gallery, entry by entry
    /// and setting by setting.
    #[test]
    fn a_changed_sample_restores_into_a_fresh_gallery() {
        let ui = mounted();
        let initial = ui.capture_workspace().unwrap();
        let sample = sample(&ui).unwrap();
        let changes = difference(&initial, &sample).expect("the sample changes the defaults");
        assert!(changes.contains("changed"), "{changes}");
        assert!(changes.contains("settings"), "{changes}");

        let mut fresh = mounted();
        let decoded = Workspace::decode(&sample.encode()).unwrap();
        let report = fresh.restore_workspace(&decoded).unwrap();
        assert!(report.is_complete(), "{report:?}");
        fresh.flush(fresh.now()).unwrap();
        let restored = fresh.capture_workspace().unwrap();
        assert_eq!(difference(&sample, &restored), None);
        assert_eq!(fresh.theme().mode().unwrap(), ThemeMode::Light);
        assert_eq!(fresh.theme().density().unwrap(), Density::Compact);
        assert_eq!(
            fresh.ui_settings().zoom().unwrap(),
            UiZoom::new(1.25).unwrap()
        );
    }

    #[test]
    fn a_mismatch_names_missing_extra_and_changed_paths_and_settings() {
        let ui = mounted();
        let initial = ui.capture_workspace().unwrap();
        let entries: Vec<_> = initial
            .entries()
            .map(|(path, state)| (path.clone(), state.clone()))
            .collect();
        let (dropped, kept) = entries.split_first().unwrap();
        let without_first = Workspace::new(*initial.settings(), kept.to_vec()).unwrap();
        let missing = difference(&initial, &without_first).unwrap();
        assert!(missing.starts_with("missing") && missing.contains(&dropped.0.to_string()));
        let extra = difference(&without_first, &initial).unwrap();
        assert!(extra.starts_with("extra") && extra.contains(&dropped.0.to_string()));

        let (path, state) = entries
            .iter()
            .find(|(_, state)| state.kind() == "split")
            .unwrap();
        let moved = ViewState::new("split")
            .unwrap()
            .with("rule", WorkspaceValue::text("ratio").unwrap())
            .unwrap()
            .with("extent", WorkspaceValue::number(0.3).unwrap())
            .unwrap();
        assert_ne!(&moved, state);
        let changed = Workspace::new(
            *initial.settings(),
            entries.iter().map(|(other, value)| {
                (
                    other.clone(),
                    if other == path {
                        moved.clone()
                    } else {
                        value.clone()
                    },
                )
            }),
        )
        .unwrap();
        let report = difference(&initial, &changed).unwrap();
        assert!(report.starts_with("changed") && report.contains(&path.to_string()));

        let mut relit = initial.clone();
        relit
            .set_settings((*initial.settings()).with_theme_mode(Some(ThemeMode::Light)))
            .unwrap();
        let mut darkened = initial.clone();
        darkened
            .set_settings((*initial.settings()).with_theme_mode(Some(ThemeMode::Dark)))
            .unwrap();
        let settings = difference(&relit, &darkened).unwrap();
        assert!(settings.starts_with("settings"), "{settings}");
        assert_eq!(difference(&initial, &initial.clone()), None);
    }
}
