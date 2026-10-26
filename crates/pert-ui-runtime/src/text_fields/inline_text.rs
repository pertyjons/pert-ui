//! An editable label: body text at rest and a child `DraftEditor` while
//! editing, with drafts that save on Enter or focus loss and cancel on Escape.
use super::draft::DraftSlot;
use super::*;
use crate::choice_controls::focus_ring;
use crate::layout::{centered, rect};

/// An editable label for names of tracks, patterns and modules, bound to a
/// UI-local `Signal<String>`.
///
/// At rest the control draws its bound text as body typography inside a
/// ControlHeight row, with an optional muted placeholder when the text is empty.
/// Double-click (two primary presses as [`ClickCounter`](crate::ClickCounter)
/// counts them: within 500 ms and four logical pixels), Enter or F2 enters
/// editing with the text selected, and so does [`TextCommand::BeginEdit`] sent
/// from code, which does not depend on the keyboard's state. Editing
/// happens in a [`DraftEditor`] the label mounts as its child under the key
/// `"editor"`, with the validation, 16 KiB limit, selection, clipboard chord,
/// IME and 32-edit undo rules of [`TextField`]; focus moves into it and returns
/// to the label when editing ends by key ([in a row](Self::in_row), to the
/// row's list). While editing,
/// [`Runtime::text_command`](crate::Runtime::text_command) on the label and
/// [`EventContext::request_text_command`] act on the draft as the chords do,
/// and `BeginEdit` answers [`TextCommandOutcome::Unavailable`]. At rest the
/// control answers `BeginEdit` by beginning to edit, with `Handled`, or with
/// `Unavailable` while it is disabled or refuses its binding, and every other
/// command with [`TextCommandOutcome::NotAnEditor`]. The label has no
/// clipboard context menu. Each editing session starts with an empty undo
/// history. Up and Down are consumed while editing. Editing is a draft:
/// Enter and focus leaving the label save it to the binding, Escape restores the
/// last observed bound value, and hiding or removal discard it. An external write that changes
/// the bound text wins over a draft and ends editing. The preferred width is the
/// wider of the bound text and the placeholder plus `ControlPaddingX`; longer
/// drafts scroll inside that allocation instead of moving neighbors.
///
/// Whether an empty name is acceptable is application validation performed on
/// the signal; the control saves any text the editor accepts. With
/// [`Self::normalize`] the application adjusts or refuses a changed draft when
/// it is saved: a refused draft stays open for correction with the error
/// outline, on Enter and on focus loss alike, and an unchanged draft is never
/// normalized.
///
/// A bound value the editor does not accept ([`TextField::check`] fails),
/// written by the application, is refused by the label rather than failing the
/// runtime: it keeps showing its previous text, draws the error outline, reports
/// `binding_error` in inspection and marks its semantics invalid. A refusal
/// ends an editing session without saving, and editing cannot begin while the
/// binding is refused, so the label never overwrites the application's value.
/// The next acceptable binding value ends the refusal. The binding itself is
/// never rewritten.
///
/// **Presses.** By default a single primary press is handled and focuses the
/// label, the second press of a double-click begins editing, and a row
/// around the label never sees the press: a
/// [`VirtualList`](crate::VirtualList) does not select the row whose label was
/// pressed. A label inside a selectable row uses [`Self::in_row`] instead, so
/// that a single press reaches the row.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, InlineText, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let name = ui.signal(String::from("Draft"));
/// let field = ui.mount_tree(Element::new("name",
///     InlineText::new(name.clone()).placeholder("Untitled")))?;
/// ui.focus(&field)?;
/// ui.key_input(KeyCode::F2, KeyAction::Press)?;
/// ui.text_input("Final")?; // replaces the selected text
/// assert_eq!(name.get()?, "Draft"); // still a draft
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(name.get()?, "Final");
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Re-describing the field under the same key applies its binding, enablement,
/// style, placeholder, normalizer, row mode and callbacks; a new binding or
/// disabling discards an open draft.
#[must_use]
pub struct InlineText {
    value: Signal<String>,
    slot: DraftSlot<()>,
    text: String,
    placeholder: String,
    /// Whether the label takes focus and edits now: `enablement` as last read.
    enabled: bool,
    /// The described availability, read in update.
    enablement: Permission,
    /// Why the bound value was last refused, while it is refused.
    rejected: Option<TextEditError>,
    normalizer: Option<Normalizer<String>>,
    /// Counts the user's edits of the draft, so a refusal goes stale on the
    /// next edit.
    edits: Rc<Cell<u64>>,
    /// The edit count of the draft the last save refused, and why.
    rejection: Option<(u64, EntryError)>,
    report: RejectionReport,
    saved: SaveReport<str>,
    /// Focus is on the label or inside its editor.
    focused: bool,
    hovered: bool,
    prepared: RefCell<Option<TextLayout>>,
    area: Cell<Option<Rect>>,
    clicks: crate::ClickCounter,
    /// The label is part of a selectable row; see [`InlineText::in_row`].
    in_row: bool,
    /// Whether the label handled the primary press that the next release
    /// ends, so the release answers alike.
    handled_press: bool,
}
description_debug!(InlineText);
impl InlineText {
    /// Creates an enabled editable label. The binding is validated and
    /// subscribed on the first update; bound text the editor does not accept
    /// is refused as described on [`InlineText`], without rewriting the source.
    pub fn new(value: impl Into<Signal<String>>) -> Self {
        let value: Signal<String> = value.into();
        let slot = DraftSlot::new(TextStyle::Body, DraftAlign::Start);
        let edits = Rc::new(Cell::new(0_u64));
        let counter = edits.clone();
        slot.draft.on_edit(move |_, _, _| {
            counter.set(counter.get().wrapping_add(1));
            Ok(())
        });
        Self {
            value,
            slot,
            edits,
            text: String::new(),
            placeholder: String::new(),
            enabled: true,
            enablement: Permission::allowed(),
            rejected: None,
            normalizer: None,
            rejection: None,
            report: RejectionReport::default(),
            saved: SaveReport::default(),
            focused: false,
            hovered: false,
            prepared: RefCell::new(None),
            area: Cell::new(None),
            clicks: crate::ClickCounter::new(),
            in_row: false,
            handled_press: false,
        }
    }
    /// Shows muted text while the bound value is empty and editing is not active.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }
    /// Overrides the body typography used at rest and while editing.
    pub fn style(mut self, style: TextStyle) -> Self {
        self.slot.set_style(style);
        self
    }
    /// Prevents editing and automatic focus; the bound text remains visible.
    /// There is no separate read-only mode: text that is shown but never
    /// edited is a [`Label`](crate::Label). Equivalent to `enabled(false)`; a
    /// later permission cannot lift it.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts editing and focus with a fixed or reactive permission;
    /// enabled by default. Repeated calls combine restrictions; every
    /// condition must allow, and same-key redescription replaces the
    /// permission with the new description's. Being the editing permission
    /// too, it is the only availability this label has.
    ///
    /// Revoking it, observed at the next update, ends an open draft unsaved,
    /// as disabling by redescription does, and makes the label unfocusable;
    /// it shows the bound text again. Enabling it again replays nothing. A
    /// binding from another runtime fails the update with
    /// [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = std::mem::take(&mut self.enablement).and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
    /// Adjusts a changed draft when it is saved, such as trimming it, or
    /// refuses it with an [`InputRejection`].
    ///
    /// Saving runs the normalizer, checks the result as single-line bound text
    /// ([`TextField::check`]), prepares it and writes it. A refusal writes
    /// nothing and keeps the draft open with the error outline until it
    /// changes. Text the application writes to the binding is never normalized.
    /// The function must be deterministic, bounded, free of side effects,
    /// binding writes and I/O, and idempotent, as [`TextField::normalize`]
    /// describes. Installing another normalizer replaces this one.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, InlineText, InputRejection, KeyAction, KeyCode, Runtime};
    /// let mut ui = Runtime::new();
    /// let name = ui.signal(String::from("Draft"));
    /// let field = ui.mount_tree(Element::new("name", InlineText::new(name.clone())
    ///     .normalize(|text| {
    ///         let text = text.trim().to_owned();
    ///         if text.is_empty() {
    ///             Err(InputRejection::new("Enter a name"))
    ///         } else {
    ///             Ok(text)
    ///         }
    ///     })))?;
    /// ui.focus(&field)?;
    /// ui.key_input(KeyCode::F2, KeyAction::Press)?;
    /// ui.text_input(" Final ")?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// assert_eq!(name.get()?, "Final");
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn normalize(
        mut self,
        normalize: impl Fn(String) -> Result<String, InputRejection> + 'static,
    ) -> Self {
        self.normalizer = Some(Rc::new(normalize));
        self
    }
    /// Makes the label part of a selectable row, such as the title in a
    /// [`VirtualList`](crate::VirtualList) row: a single press reaches the
    /// row, and a double-click edits. Default `false`.
    ///
    /// In this mode the resting label is not focusable, so a press on it
    /// focuses its nearest focusable ancestor, the list, which stays the
    /// collection's keyboard owner, and the label adds no Tab stop at rest;
    /// other row content, such as a checkbox, keeps its own. The label's own
    /// Enter and F2 are therefore out of reach at rest: the keyboard route is
    /// the list's, whose [`on_activate`](crate::VirtualList::on_activate) or an
    /// application command sends [`TextCommand::BeginEdit`] to the active
    /// row's label.
    ///
    /// Presses at rest:
    ///
    /// - A primary press with Shift, Control, Alt or Super held is left to
    ///   the row, which selects with those modifiers, and starts no series.
    /// - An unmodified primary press counts as
    ///   [`ClickCounter`](crate::ClickCounter) counts (within 500 ms and four
    ///   logical pixels). The first press of a series is left to the row,
    ///   which makes it active, selects it and may arm a reorder drag; the
    ///   second begins editing and is handled, so the row does not see it.
    /// - A release answers like the press it ends. A press the label left to
    ///   the row is not captured by the label; the row's own capture rules
    ///   apply.
    ///
    /// A series also ends when the pointer leaves the label (so a press on
    /// another row's label and back is not a double-click), when a reorder
    /// drag starts from it, on a modified press, on cancelled input, on
    /// hiding, on disabling and when an edit begins. A single press therefore
    /// never renames, and a selected row can be dragged by its label.
    ///
    /// While editing, presses follow the default rules. When editing ends by
    /// key, focus falls back to the nearest focusable ancestor, the list, so
    /// its arrows navigate again; focus leaving by other means saves, as by
    /// default. Inspection reports the property `in_row` while the mode is
    /// on. Redescription replaces the mode and never ends a draft; turning it
    /// on while the resting label has focus moves focus to the nearest
    /// focusable ancestor at the end of the flush.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use pert_ui::{ListItems, ListSource, PointerButton, PointerEvent, PointerKind};
    /// use std::time::Duration;
    ///
    /// struct Names(Signal<String>);
    /// impl ListItems for Names {
    ///     fn len(&self) -> usize { 1 }
    ///     fn key(&self, index: usize) -> Option<String> { (index == 0).then(|| "a".to_owned()) }
    ///     fn build(&self, index: usize) -> Option<Element> {
    ///         (index == 0).then(|| Element::new("title", InlineText::new(&self.0).in_row(true)))
    ///     }
    /// }
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    /// let name = ui.signal(String::from("Mix"));
    /// let rows = ui.signal(ListSource::new(Names(name.clone())));
    /// let selection = ListSelection::new(&ui, SelectionMode::Single);
    /// let list = ui.mount_tree(Element::new("list",
    ///     VirtualList::new(rows.read_only(), 32.0)?.selection(selection.clone())))?;
    /// ui.flush(Duration::ZERO)?;
    /// let at = Point::new(20.0, 16.0)?;
    /// let click = |ui: &mut Runtime| -> Result<(), RuntimeError> {
    ///     ui.pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
    ///     ui.pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
    ///     Ok(())
    /// };
    /// click(&mut ui)?; // a single press selects the row
    /// assert_eq!(selection.active().get()?.as_deref(), Some("a"));
    /// assert_eq!(ui.focused(), Some(list.clone()));
    /// click(&mut ui)?; // the second press of a double-click edits
    /// ui.text_input("Master")?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// assert_eq!(name.get()?, "Master");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn in_row(mut self, in_row: bool) -> Self {
        self.in_row = in_row;
        self
    }
    /// Tells the application about the normalizer's refusals, with the rules
    /// of [`TextField::on_rejection`]: `Some` on each refused save, and `None`
    /// once when a reported refusal ends at a later save, at focus loss or at
    /// Escape, including after an application write ended the session.
    /// Escape discards the draft before delivering this callback. A callback
    /// failure does not keep editing active; the clear notification stays pending
    /// for a later retry.
    pub fn on_rejection(
        mut self,
        report: impl FnMut(&mut EventContext, Option<&InputRejection>) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.report.set(Box::new(report));
        self
    }
    /// Tells the application about each save that wrote the binding, with
    /// the saved (normalized) text: Enter or focus loss ending a session
    /// whose draft differs from the bound text. The rules are those of
    /// [`TextField::on_save`]; an unchanged draft writes nothing and is not
    /// reported.
    pub fn on_save(
        mut self,
        saved: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.saved.set(Box::new(saved));
        self
    }
    /// The refusal of the current draft, if the last save refused this draft.
    fn current_rejection(&self) -> Option<&EntryError> {
        let (edits, error) = self.rejection.as_ref()?;
        (self.editing() && *edits == self.edits.get()).then_some(error)
    }
    /// Ends an editing session without saving.
    fn discard(&mut self, cx: Option<&mut EventContext>) {
        self.rejection = None;
        self.slot.end(cx);
    }
    fn editing(&self) -> bool {
        self.slot.is_active()
    }
    fn rest_text(&self) -> (&str, bool) {
        if self.text.is_empty() {
            (&self.placeholder, true)
        } else {
            (&self.text, false)
        }
    }
    fn begin(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.editing() || !self.enabled || self.rejected.is_some() {
            return Ok(());
        }
        self.reset_row_series();
        self.slot.begin(cx, &self.text, None, false)
    }
    /// Ends a row-mode click series; the default mode keeps its own rules.
    fn reset_row_series(&mut self) {
        if self.in_row {
            self.clicks.reset();
        }
    }
    /// Ends a session by key: focus returns to the label, or, in a row,
    /// where the label is not focusable at rest, falls back to its nearest
    /// focusable ancestor.
    fn end_by_key(&mut self, cx: &mut EventContext) {
        if self.in_row {
            self.discard(None);
            cx.invalidate(Invalidation::Layout);
        } else {
            self.discard(Some(cx));
        }
    }
    /// A press or release at rest in row mode.
    fn row_pointer(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                let modifiers = cx.modifiers();
                self.handled_press = false;
                if modifiers.shift || modifiers.control || modifiers.alt || modifiers.super_key {
                    // The row selects with modifiers.
                    self.clicks.reset();
                    return Ok(InputOutcome::Ignored);
                }
                if self.clicks.press_event(cx, e) > Some(1) {
                    self.clicks.reset();
                    self.begin(cx)?;
                    self.handled_press = true;
                    return Ok(InputOutcome::Handled);
                }
                // The first press of a series selects the row.
                Ok(InputOutcome::Ignored)
            }
            PointerKind::Release(PointerButton::Primary) => {
                Ok(if std::mem::take(&mut self.handled_press) {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                })
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    /// Writes a changed draft to the binding, normalized, and ends the
    /// session. Returns `false` when the normalizer or the check refused the
    /// draft, which then stays open.
    fn save(&mut self, cx: &mut EventContext) -> Result<bool, RuntimeError> {
        if !self.editing() {
            // A session an application write ended in an update has no draft
            // to save, but a refusal reported for it ends here.
            self.report.clear(cx)?;
            return Ok(true);
        }
        // An application write since the last update wins over the draft, as
        // it would have at that update, which then shows it.
        if self.value.get()? != self.text {
            self.discard(None);
            cx.invalidate(Invalidation::Layout);
            self.report.clear(cx)?;
            return Ok(true);
        }
        let draft = self.slot.draft.text();
        let saved = match &self.normalizer {
            Some(normalizer) if draft != self.text => {
                match normalize_text(normalizer, draft.clone(), false) {
                    Ok(text) => text,
                    Err(error) => {
                        // The label shows the refusal whether or not the
                        // report reaches the application.
                        cx.invalidate(Invalidation::Paint);
                        self.rejection = Some((self.edits.get(), error.clone()));
                        self.report.outcome(cx, Some(&error))?;
                        return Ok(false);
                    }
                }
            }
            _ => draft,
        };
        if saved != self.text {
            cx.text(&saved, None, self.slot.style())?;
        }
        self.discard(None);
        let wrote = saved != self.text;
        let wrote = wrote && {
            self.text = saved.clone();
            self.value.set_changed(saved.clone())?
        };
        cx.invalidate(Invalidation::Layout);
        // Both notifications are attempted, so a failing rejection report
        // cannot swallow the save; the first error is returned.
        let reported = self.report.clear(cx);
        let reported_save = if wrote {
            self.saved.saved(cx, &saved)
        } else {
            Ok(())
        };
        reported.and(reported_save).map(|()| true)
    }
}
impl Component for InlineText {
    /// Applies a new description: binding, typography, placeholder,
    /// permission, normalizer and callbacks. A different binding ends an open
    /// draft unsaved, as an application write does, and so does disabling;
    /// the next update shows the new binding's text.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if !self.value.read_only().same_source(&next.value.read_only()) {
            self.value = next.value;
            self.rejected = None;
            self.discard(None);
            invalidation = Some(Invalidation::Layout);
        }
        // A bound permission keeps the current state until the next update
        // reads it, so redescription never briefly grants what it denies.
        let enabled = next.enablement.fixed().unwrap_or(self.enabled);
        if !self.enablement.same_as(&next.enablement) {
            self.enablement = next.enablement;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        if self.enabled != enabled {
            self.enabled = enabled;
            if !self.enabled {
                self.discard(None);
                self.reset_row_series();
            }
            invalidation = Some(Invalidation::Layout);
        }
        if self.slot.style() != next.slot.style() {
            self.slot.set_style(next.slot.style());
            invalidation = Some(Invalidation::Layout);
        }
        if self.placeholder != next.placeholder {
            self.placeholder = next.placeholder;
            invalidation = Some(Invalidation::Layout);
        }
        if self.in_row != next.in_row {
            // Focusability changes; an open draft stays.
            self.in_row = next.in_row;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        self.normalizer = next.normalizer;
        self.report.adopt(next.report);
        self.saved.adopt(next.saved);
        invalidation
    }
    fn focusable(&self) -> bool {
        // In a row the resting label leaves focus to the list.
        self.enabled && !self.editing() && !self.in_row
    }
    fn child_visible(&self, key: &str) -> bool {
        self.slot.child_visible(key)
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        (self.enabled && self.editing()).then_some(CursorIcon::Text)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if self.hovered != hovered {
            self.hovered = hovered;
            cx.invalidate(Invalidation::Paint);
        }
        if !hovered {
            // Leaving the label, also when a drag session starts from it,
            // ends a series.
            self.reset_row_series();
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        if enabled != self.enabled {
            self.enabled = enabled;
            if !enabled {
                // A revoked permission ends the draft as disabling does.
                self.discard(None);
                self.reset_row_series();
                cx.cancel_input();
            }
            cx.request_layout();
        }
        self.slot.update(cx);
        self.focused = cx.focus_within();
        let text = cx.read_paint(&self.value)?;
        // Refused text is kept out of the label rather than failing every
        // flush: the label keeps its previous text and marks the refusal, and
        // an open draft ends unsaved so it never overwrites the value.
        if let Err(error) = check_text(&text, false) {
            self.rejected = Some(error);
            self.discard(None);
            return Ok(());
        }
        self.rejected = None;
        if text != self.text {
            self.text = text;
            // External writes win over drafts.
            self.discard(None);
            cx.request_layout();
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let pad = cx.metric(MetricRole::ControlPaddingX) * 0.5;
        let style = self.slot.style();
        let text = cx.text(&self.text, None, style)?.size();
        let placeholder = cx.text(&self.placeholder, None, style)?.size();
        Ok(Size::new(
            text.width()
                .max(placeholder.width())
                .max(cx.metric(MetricRole::IconSize))
                + 2.0 * pad,
            cx.metric(MetricRole::ControlHeight),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let measure = cx.measurement();
        let pad = (measure.metric(MetricRole::ControlPaddingX) * 0.5).min(size.width() / 2.0);
        let (rest, _) = self.rest_text();
        let layout = measure.text(rest, None, self.slot.style())?;
        let line = layout.size().height();
        let area = rect(
            pad,
            centered(size.height(), line),
            (size.width() - 2.0 * pad).max(0.0),
            line.min(size.height()),
        )?;
        self.area.set(Some(area));
        *self.prepared.borrow_mut() = Some(layout);
        self.slot.arrange(cx, area)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled {
            return Ok(InputOutcome::Ignored);
        }
        if self.editing() {
            // The editor takes presses on the text; one on the padding stops
            // at the label while it edits.
            return Ok(if matches!(e.kind(), PointerKind::Scroll(_)) {
                InputOutcome::Ignored
            } else {
                InputOutcome::Handled
            });
        }
        if self.in_row {
            return self.row_pointer(cx, e);
        }
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                if self.clicks.press_event(cx, e) > Some(1) {
                    self.clicks.reset();
                    self.begin(cx)?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) => Ok(InputOutcome::Handled),
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(e);
        if !self.enabled {
            return Ok(InputOutcome::Ignored);
        }
        if !self.editing() {
            if e.transition == KeyTransition::Pressed
                && matches!(control, KeyCode::Enter | KeyCode::F2)
                && self.rejected.is_none()
            {
                self.begin(cx)?;
                return Ok(InputOutcome::Handled);
            }
            if e.transition == KeyTransition::Pressed && control == KeyCode::Escape {
                // Ends a refusal whose session an application write ended;
                // Escape still bubbles.
                self.report.clear(cx)?;
            }
            return Ok(InputOutcome::Ignored);
        }
        // The editor leaves these keys to its owner.
        match control {
            KeyCode::Escape if e.transition == KeyTransition::Pressed => {
                self.end_by_key(cx);
                self.report.clear(cx)?;
                Ok(InputOutcome::Handled)
            }
            KeyCode::Enter if e.transition == KeyTransition::Pressed => {
                if self.save(cx)? && !self.in_row {
                    cx.request_focus();
                }
                Ok(InputOutcome::Handled)
            }
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::PageUp | KeyCode::PageDown => {
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn text_key_event(
        &mut self,
        cx: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        keys: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        if !self.editing() {
            return Ok(TextKeyOutcome::NotAnEditor);
        }
        // The keys the editor leaves to the label, answered in the text phase
        // so capture bindings never take them.
        let handled = self.keyboard_event(cx, input.editing_event(transition), keys)?
            == InputOutcome::Handled;
        Ok(if handled {
            TextKeyOutcome::Handled
        } else {
            TextKeyOutcome::Ignored
        })
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        if command != TextCommand::BeginEdit {
            // While editing the runtime passes the command on to the editor.
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        // What Enter and F2 do at rest; nothing to begin otherwise.
        if self.editing() || !self.enabled || self.rejected.is_some() {
            return Ok(TextCommandOutcome::Unavailable);
        }
        self.begin(cx)?;
        Ok(TextCommandOutcome::Handled)
    }
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        // Focus left the label and its editor: the draft saves.
        if within {
            return Ok(());
        }
        self.clicks.reset();
        let result = self.save(cx).map(|_| ());
        cx.invalidate(Invalidation::Paint);
        result
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.clicks.reset();
        cx.invalidate(Invalidation::Paint);
        // The resting label losing focus is a save with nothing to save.
        self.report.clear(cx)
    }
    fn visibility_changed(&mut self, visible: bool) {
        if !visible {
            // Hiding discards the draft.
            self.discard(None);
            self.reset_row_series();
        }
    }
    fn input_reset(&mut self) {
        self.clicks.reset();
        self.handled_press = false;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let rect = Rect::from_size(size);
        if self.editing() {
            // The editor child draws the draft inside this frame.
            field_frame(
                p,
                rect,
                self.focused,
                self.current_rejection().is_some(),
                self.hovered && self.enabled,
            )?;
            return Ok(());
        }
        if self.hovered && self.enabled {
            p.fill_shape(
                rect,
                crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                p.color(ColorRole::SurfaceHover),
            )?;
        }
        let (_, placeholder) = self.rest_text();
        if let (Some(text), Some(area)) = (self.prepared.borrow().as_ref(), self.area.get()) {
            let [x, y, _, _] = area.values();
            let color = p.color(if !self.enabled || placeholder {
                ColorRole::Muted
            } else {
                ColorRole::OnSurface
            });
            p.with_clip(area, |p| text.paint(p, crate::Point::new(x, y)?, color))?;
        }
        if self.rejected.is_some() {
            p.stroke_shape(
                rect,
                crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                Length::new(p.metric(MetricRole::FocusWidth))?,
                p.color(ColorRole::Error),
            )?;
        } else if self.focused {
            focus_ring(p, rect)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties
            .insert("text".into(), InspectValue::Text(self.text.clone()));
        d.properties
            .insert("editing".into(), InspectValue::Bool(self.editing()));
        if self.editing() {
            d.properties
                .insert("draft".into(), InspectValue::Text(self.slot.draft.text()));
        }
        d.properties.insert(
            "placeholder".into(),
            InspectValue::Text(self.placeholder.clone()),
        );
        if let Some(error) = self.rejected {
            d.properties.insert(
                "binding_error".into(),
                InspectValue::Text(error.to_string()),
            );
        }
        let rejection = self.current_rejection();
        if let Some(error) = rejection {
            d.properties.insert(
                "validation_error".into(),
                InspectValue::Text(error.to_string()),
            );
        }
        d.properties
            .insert("enabled".into(), InspectValue::Bool(self.enabled));
        if self.in_row {
            d.properties
                .insert("in_row".into(), InspectValue::Bool(true));
        }
        d.validation.interactive = self.enabled;
        let editable = self.enabled && self.rejected.is_none();
        d.semantics = Some(
            Semantics::new(SemanticRole::TextInput)
                .description(self.placeholder.as_str())
                .value(SemanticValue::Text(self.text.clone()))
                .disabled(!self.enabled)
                .read_only(self.rejected.is_some())
                .invalid(self.rejected.is_some() || rejection.is_some())
                .enabled_actions(editable.then_some(SemanticAction::SetValue)),
        );
        d
    }
}
