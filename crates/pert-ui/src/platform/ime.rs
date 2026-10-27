//! Focus/session correlation for untagged native IME event streams.
use crate::host::HostIntegration;
use crate::{Runtime, RuntimeError, TextEditError, TextField, TextPreedit, WidgetId};
use winit::event::Ime;

#[derive(Default)]
pub(super) struct NativeIme {
    target: Option<(WidgetId, u64)>,
    enabled: bool,
    pub(super) composing: bool,
}
impl NativeIme {
    fn target(ui: &Runtime) -> Option<(WidgetId, u64)> {
        ui.text_input_state().map(|(id, state)| (id, state.session))
    }
    pub(super) fn reconcile(&mut self, ui: &Runtime) -> bool {
        let target = Self::target(ui);
        if target == self.target {
            return false;
        }
        self.target = target;
        self.enabled = false;
        self.composing = false;
        true
    }
    pub(super) fn wanted(&self) -> bool {
        self.target.is_some()
    }
    pub(super) fn receive(&mut self, ui: &mut Runtime, event: Ime) -> Result<(), RuntimeError> {
        ui.flush(ui.now())?;
        // Never accept a commit using the previous recipient/session, even if
        // redraw/about_to_wait has not yet reconciled platform state.
        if self.target.is_none() || self.target != Self::target(ui) {
            return Ok(());
        }
        match event {
            Ime::Enabled => self.enabled = true,
            Ime::Disabled => {
                self.enabled = false;
                self.composing = false;
                ui.text_preedit(&TextPreedit::new("", None)?)?;
            }
            Ime::Preedit(text, cursor) if self.enabled => {
                // The IME composes whether or not its preedit can be shown,
                // so key text stays with it until it commits.
                self.composing = !text.is_empty();
                // winit reports the composition selection as a `(start, end)` pair.
                let cursor = cursor.map(|(start, end)| start..end);
                ui.text_preedit(&preedit(text, cursor)?)?;
            }
            Ime::Commit(text) if self.enabled => {
                self.composing = false;
                if let Some(text) = commit(&text) {
                    ui.text_input(&text)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// The preedit shown for OS-supplied composition text. A malformed one, with
/// control characters or unsupported Unicode line separators, over
/// [`TextEditError::MAX_TEXT_BYTES`] or with a cursor off its
/// character boundaries, is not shown: the field shows no composition until
/// a valid preedit or the commit arrives, rather than failing the component.
fn preedit(
    text: String,
    cursor: Option<std::ops::Range<usize>>,
) -> Result<TextPreedit, RuntimeError> {
    if TextField::check(&text).is_ok()
        && let Ok(preedit) = TextPreedit::new(text, cursor)
    {
        return Ok(preedit);
    }
    Ok(TextPreedit::new("", None)?)
}

/// The text inserted for an OS-supplied commit, or `None`. Control characters
/// are removed, as for key text; a commit over [`TextEditError::MAX_TEXT_BYTES`] is dropped
/// whole, because no field accepts it and a partial insertion would be
/// wrong text.
fn commit(text: &str) -> Option<String> {
    let text: String = text.chars().filter(|c| !c.is_control()).collect();
    (!text.is_empty() && text.len() <= TextEditError::MAX_TEXT_BYTES).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::time::Duration;
    fn setup() -> (Runtime, NativeIme, Signal<String>) {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 100.0, 1.0).unwrap())
            .unwrap();
        let value = ui.signal("original".to_owned());
        let id = ui
            .mount_tree(Element::new("field", TextField::new(value.clone())))
            .unwrap();
        ui.focus(&id).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        ui.paint().unwrap();
        let mut ime = NativeIme::default();
        assert!(ime.reconcile(&ui));
        ime.receive(&mut ui, Ime::Enabled).unwrap();
        (ui, ime, value)
    }
    #[test]
    fn discarded_composition_cannot_commit_before_or_after_reconciliation() {
        for cancel in [0, 1, 2] {
            let (mut ui, mut ime, value) = setup();
            ime.receive(&mut ui, Ime::Preedit("pending".into(), Some((7, 7))))
                .unwrap();
            match cancel {
                0 => {
                    value.set("external".into()).unwrap();
                }
                1 => {
                    ui.key_input(KeyCode::Escape, KeyAction::Press).unwrap();
                }
                _ => {
                    ui.pointer_input(PointerEvent::new(
                        Point::new(30.0, 15.0).unwrap(),
                        PointerKind::Press(PointerButton::Primary),
                    ))
                    .unwrap();
                }
            }
            let expected = value.get().unwrap();
            ime.receive(&mut ui, Ime::Commit("stale".into())).unwrap();
            assert_eq!(value.get().unwrap(), expected);
            assert!(ime.reconcile(&ui));
            assert!(!ime.composing);
            ime.receive(&mut ui, Ime::Commit("also stale".into()))
                .unwrap();
            assert_eq!(value.get().unwrap(), expected);
            ime.receive(&mut ui, Ime::Enabled).unwrap();
            ime.receive(&mut ui, Ime::Commit("fresh".into())).unwrap();
            assert!(value.get().unwrap().contains("fresh"));
        }
    }
    #[test]
    fn enabled_composition_commits_and_enter_during_preedit_only_confirms_ime() {
        let (mut ui, mut ime, value) = setup();
        ime.receive(&mut ui, Ime::Preedit("å".into(), Some((2, 2))))
            .unwrap();
        ui.key_input(KeyCode::Enter, KeyAction::Press).unwrap();
        assert_eq!(value.get().unwrap(), "original");
        ime.receive(&mut ui, Ime::Preedit("".into(), None)).unwrap();
        ime.receive(&mut ui, Ime::Commit("å".into())).unwrap();
        assert_eq!(value.get().unwrap(), "åoriginal");
    }
    /// OS-supplied payloads the field would reject never reach it as a
    /// component failure: controls are filtered from commits, an oversized
    /// commit is dropped, and a malformed preedit is not shown.
    #[test]
    fn invalid_ime_payloads_are_filtered_at_the_adapter() {
        let (mut ui, mut ime, value) = setup();
        for malformed in [
            Ime::Preedit("a\u{0}b".into(), None),
            Ime::Preedit("a\u{2028}b".into(), None),
            Ime::Preedit("a\u{2029}b".into(), None),
            Ime::Preedit("x".repeat(TextEditError::MAX_TEXT_BYTES + 1), None),
            Ime::Preedit("å".into(), Some((1, 1))),
        ] {
            ime.receive(&mut ui, Ime::Preedit("pending".into(), None))
                .unwrap();
            let composing = |ui: &Runtime| {
                ui.inspect().components.into_iter().any(|component| {
                    component.details.properties.get("composing") == Some(&InspectValue::Bool(true))
                })
            };
            assert!(composing(&ui));
            ime.receive(&mut ui, malformed).unwrap();
            assert!(
                !composing(&ui),
                "malformed preedit clears the shown composition"
            );
            assert!(ime.composing, "the IME still composes");
            assert_eq!(value.get().unwrap(), "original");
        }
        for text in ["a\u{0}b", "a\u{2028}b", "a\u{2029}b"] {
            assert_eq!(
                preedit(text.into(), None).unwrap(),
                TextPreedit::new("", None).unwrap()
            );
        }
        assert_eq!(
            preedit("åä".into(), Some(2..4)).unwrap(),
            TextPreedit::new("åä", Some(2..4)).unwrap()
        );
        ime.receive(&mut ui, Ime::Commit("a\u{85}b\u{0}".into()))
            .unwrap();
        assert!(!ime.composing);
        assert_eq!(value.get().unwrap(), "aboriginal");
        ime.receive(
            &mut ui,
            Ime::Commit("x".repeat(TextEditError::MAX_TEXT_BYTES + 1)),
        )
        .unwrap();
        ime.receive(&mut ui, Ime::Commit("\u{85}".into())).unwrap();
        assert_eq!(value.get().unwrap(), "aboriginal");
    }
}
