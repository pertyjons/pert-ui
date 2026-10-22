//! A single-line query editor with a search icon, a clear action and keys
//! reported to its owner.
use super::{Editor, unhandled_text_key};
use crate::layout::rect;
use crate::*;
use std::{cell::Cell, time::Duration};

type KeyReport = Box<dyn FnMut(&mut EventContext, SearchKey) -> Result<(), RuntimeError>>;

/// A key a [`SearchField`] reports to its owner instead of editing with it.
///
/// These are the keys a list beside or below the field needs while focus stays
/// in the field: the field never moves its caret or selection for them.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SearchKey {
    /// Arrow Up, pressed or repeated, without Shift, Ctrl, Alt or Cmd: move a
    /// highlight to the previous result.
    Up,
    /// Arrow Down, pressed or repeated, without modifiers: move a highlight to
    /// the next result.
    Down,
    /// Page Up, pressed or repeated, without modifiers: move a highlight one
    /// page back.
    PageUp,
    /// Page Down, pressed or repeated, without modifiers: move a highlight one
    /// page on.
    PageDown,
    /// A fresh Enter press outside IME composition, after any pending
    /// debounced query has been written: act on the query or the highlighted
    /// result.
    Submit,
}

/// A single-line editor for a query, bound to a UI-local `Signal<String>`, with
/// a leading search icon and a trailing clear button.
///
/// It is a [`TextField`] for editing purposes: selection, clipboard, undo, the
/// clipboard context menu, IME, the byte limit and refused bindings behave as
/// documented there. What it adds is query behavior for an owner, such as a
/// command palette or a chooser, that presents results beside the field:
///
/// - **The query is committed text.** The binding receives what the user has
///   typed, pasted or committed through an IME, never composition (preedit)
///   text. By default every committed edit writes the binding at once; with
///   [`Self::debounce`] edits wait until the user pauses.
/// - **Escape clears.** A fresh Escape press with a non-empty query (or a
///   pending debounced edit) clears the text and writes the empty query at
///   once. During IME composition Escape cancels the composition instead,
///   and while the field's clipboard menu is open it closes that menu.
///   With an empty query the field leaves Escape alone, so it reaches the
///   field's owner: an enclosing popup or dialog closes, or an owning
///   component handles it in its own key callback. Clearing never moves focus.
/// - **Keys for the owner.** Unmodified Up, Down, Page Up and Page Down
///   (pressed or repeated) and a fresh Enter press are reported as a
///   [`SearchKey`] to the callback from [`Self::on_key`], and focus and caret
///   stay where they are. Without a callback these keys are not handled by the
///   field, so they bubble to its ancestors' key callbacks instead. With
///   Shift, Ctrl, Alt or Cmd the arrows keep their editing meaning (selection,
///   start and end of the text).
/// - **Clear button.** Shown while the field holds text and accepts edits; a
///   primary press clears exactly as Escape does. It is part of the field's
///   single Tab stop, not a focus target of its own. Its slot is always
///   reserved, so showing it never changes the text's layout.
///
/// The field sees Escape before surfaces do, on the native path
/// ([`Runtime::command_key_input`]) and the headless [`Runtime::key_input`]
/// alike, so Escape inside a popup first clears the query and a second Escape
/// closes the popup.
///
/// Defaults to body typography, a 240-pixel preferred width and no
/// placeholder. Its semantics report [`SemanticRole::SearchInput`] with the
/// visible text as value; inspection adds `query` (the bound text),
/// `pending` (a debounced edit not yet written) and `placeholder`.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, KeyAction, KeyCode, Runtime, SearchField, SearchKey};
/// use std::{cell::RefCell, rc::Rc};
/// let mut ui = Runtime::new();
/// let query = ui.signal(String::new());
/// let keys = Rc::new(RefCell::new(Vec::new()));
/// let seen = keys.clone();
/// let field = ui.mount_tree(Element::new(
///     "search",
///     SearchField::new(query.clone())
///         .placeholder("Search commands")
///         .on_key(move |_, key| {
///             seen.borrow_mut().push(key);
///             Ok(())
///         }),
/// ))?;
/// ui.focus(&field)?;
/// ui.text_input("rev")?;
/// assert_eq!(query.get()?, "rev");
/// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
/// assert_eq!(*keys.borrow(), [SearchKey::Down]);
/// ui.key_input(KeyCode::Escape, KeyAction::Press)?;
/// assert_eq!(query.get()?, "");
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Re-describing the field under the same key applies its binding, editor
/// settings, debounce, key callback and placeholder; a pending debounced query
/// is dropped when the binding changes or the debounce is removed, and rearmed
/// when the debounce changes.
#[must_use]
pub struct SearchField {
    editor: Editor,
    value: Signal<String>,
    debounce: Option<Duration>,
    /// A debounced edit waits for its deadline.
    pending: bool,
    /// The text the running deadline was armed for, so an event that edits
    /// nothing does not postpone the query.
    armed: String,
    report: Option<KeyReport>,
    placeholder: Caption,
    /// A re-description dropped a pending edit; the next update cancels its
    /// deadline, which `reconcile` has no context to do.
    drop_deadline: bool,
    /// A re-description changed the delay of a pending edit; the next
    /// event-phase point restarts its deadline with the new delay, which
    /// neither `reconcile` nor update can schedule.
    rearm: bool,
    /// Asks for `deliver_notifications`, kept from the first update so that
    /// `reconcile` can request it.
    notifier: Option<crate::Notifier>,
    icon: Cell<Option<Rect>>,
    clear: Cell<Option<Rect>>,
    clear_hover: bool,
}
description_debug!(SearchField);

impl SearchField {
    /// Creates an enabled search field that writes every committed edit to
    /// `query`. The binding is subscribed on the first update; a
    /// foreign-runtime binding fails that update, and a value the field does
    /// not accept is refused as for [`TextField`].
    pub fn new(query: impl Into<Signal<String>>) -> Self {
        let query: Signal<String> = query.into();
        Self {
            editor: Editor::new(Some(query.clone()), false),
            value: query,
            debounce: None,
            pending: false,
            armed: String::new(),
            report: None,
            placeholder: Caption::new(TextSource::default()),
            drop_deadline: false,
            rearm: false,
            notifier: None,
            icon: Cell::new(None),
            clear: Cell::new(None),
            clear_hover: false,
        }
    }
    /// Overrides the default body typography using a bundled semantic style.
    pub fn style(mut self, style: TextStyle) -> Self {
        self.editor.style = style;
        self
    }
    /// Allows selection and copy but prevents editing, clearing and native IME
    /// activation. The owner keys are still reported. Equivalent to
    /// `editable(false)`; a later permission cannot lift it.
    pub fn read_only(self) -> Self {
        self.editable(false)
    }
    /// Restricts editing and clearing with a fixed or reactive permission;
    /// editable by default. Repeated calls combine restrictions; every
    /// condition must allow, and same-key redescription replaces the
    /// permission with the new description's.
    ///
    /// While editing is denied the field keeps focus, selection and copying,
    /// and still reports the owner keys. Revoking it, observed at the next
    /// update, cancels a composition and a selection drag; a pending
    /// debounced edit is not written while editing is denied and waits for
    /// the next edit, Enter or focus loss once it is allowed again. A binding
    /// from another runtime fails the update with
    /// [`RuntimeError::ForeignRuntime`].
    pub fn editable(mut self, permission: impl Into<Permission>) -> Self {
        self.editor.restrict_editing(permission.into());
        self
    }
    /// Prevents interaction and automatic focus; still follows its binding.
    /// Equivalent to `enabled(false)`; a later permission cannot lift it.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts focus and input with a fixed or reactive permission; enabled
    /// by default. Repeated calls combine restrictions; every condition must
    /// allow, and same-key redescription replaces the permission with the new
    /// description's.
    ///
    /// Revoking it, observed at the next update, makes the field unfocusable
    /// and releases focus, cancels a composition and a selection drag, and
    /// discards a pending debounced edit unwritten, including one whose
    /// deadline is already due; the field shows the bound query again, and
    /// enabling it again replays nothing. A binding from another runtime
    /// fails the update with [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.editor.restrict_enablement(permission.into());
        self
    }
    /// Sets muted hint text shown while the field is empty and nothing is
    /// being composed, such as "Search commands". It is presentation only:
    /// it is never part of the query. Empty by default.
    ///
    /// The hint is a [`TextSource`]: fixed plain or rich text, a binding the
    /// field follows, or a [`StringRole`] such as
    /// [`StringRole::SearchPlaceholder`] that follows the installed string
    /// table. A binding or role is read in the field's update; describing the
    /// field again with the same source keeps the text already read.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Element, Runtime, SearchField, StringRole, UiStrings};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let query = ui.signal(String::new());
    /// ui.mount_tree(Element::new(
    ///     "find",
    ///     SearchField::new(query).placeholder(StringRole::SearchPlaceholder),
    /// ))?;
    /// ui.ui_settings().set_strings(UiStrings::english().with_text(StringRole::SearchPlaceholder, "Sök")?)?;
    /// ui.flush(Duration::ZERO)?;
    /// let details = ui.inspect().components.into_iter()
    ///     .find(|c| c.key == "find").unwrap().details;
    /// assert_eq!(details.properties["placeholder"], pert_ui::InspectValue::Text("Sök".into()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn placeholder(mut self, text: impl Into<TextSource>) -> Self {
        self.placeholder = Caption::new(text);
        self
    }
    /// Delays writing the query until no edit has arrived for `delay`, so an
    /// owner that filters a large set does not filter on every keystroke.
    ///
    /// The delay is one runtime deadline ([`EventContext::set_timer`]) that
    /// each edit restarts; nothing polls. While it runs the field shows the
    /// edited text and inspection reports `pending`. The edit is written at
    /// once, cancelling the deadline, by a fresh Enter press (before
    /// [`SearchKey::Submit`] is reported), by focus transfer or window focus
    /// loss, and by clearing. Hiding the field, or an application write to
    /// the binding, discards a pending edit, and the field shows the bound
    /// text again. A pending edit whose deadline is cancelled by input
    /// cancellation while the field keeps focus waits for the next edit,
    /// Enter or focus loss. Undo and redo are edits like any other.
    ///
    /// `Duration::ZERO`, the default, writes every edit at once.
    pub fn debounce(mut self, delay: Duration) -> Self {
        self.debounce = (!delay.is_zero()).then_some(delay);
        self.editor.deferred = self.debounce.is_some();
        self
    }
    /// Receives the [`SearchKey`]s the field reports to its owner, on the UI
    /// thread in the event phase. A reported key is handled: it does not
    /// bubble further and moves neither focus nor caret. An error from the
    /// callback propagates from the input call that delivered the key.
    /// Replaces an earlier callback.
    pub fn on_key(
        mut self,
        report: impl FnMut(&mut EventContext, SearchKey) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.report = Some(Box::new(report));
        self
    }

    /// Whether Escape or the clear button has something to clear: visible
    /// text, or a bound query a pending edit has emptied but not yet written.
    fn has_query(&self) -> bool {
        !self.editor.text.is_empty() || !self.editor.observed.is_empty()
    }
    fn can_clear(&self) -> bool {
        self.editor.enabled && !self.editor.locked()
    }
    /// Writes a pending debounced edit at once.
    fn flush(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.pending {
            return Ok(());
        }
        // Timers are delivered before updates, so a deadline that is due in
        // the flush that observes a revocation reads the permissions now.
        let (enabled, editable) = self.editor.permits_now()?;
        if !enabled {
            self.pending = false;
            cx.cancel_timer();
            self.discard();
            cx.invalidate(Invalidation::Paint);
            return Ok(());
        }
        if !editable {
            // Kept for the next edit, Enter or focus loss once allowed.
            cx.cancel_timer();
            return Ok(());
        }
        self.pending = false;
        cx.cancel_timer();
        // An application write that this field has not observed yet (timers
        // are delivered before binding updates) wins over the pending edit:
        // nothing is written, and the next update shows the bound text.
        if self.value.get()? != self.editor.observed {
            return Ok(());
        }
        if self.editor.enabled && self.editor.text != self.editor.observed && !self.editor.locked()
        {
            self.value.set(self.editor.text.clone())?;
            self.editor.observed.clone_from(&self.editor.text);
        }
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    /// Starts or stops the debounce deadline after an event that may have
    /// edited the text.
    fn edited(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let Some(delay) = self.debounce else {
            return Ok(());
        };
        if self.editor.text == self.editor.observed {
            if self.pending {
                self.pending = false;
                cx.cancel_timer();
            }
        } else if !self.pending || self.editor.text != self.armed {
            // Only an edit restarts the deadline; caret movement, selection
            // and pointer motion leave it running.
            self.pending = true;
            self.armed.clone_from(&self.editor.text);
            cx.set_timer(delay)?;
        }
        Ok(())
    }
    /// Discards a pending edit, showing the bound text again.
    fn discard(&mut self) {
        self.pending = false;
        if self.editor.text != self.editor.observed {
            self.editor.sync_text(self.editor.observed.clone());
        }
    }
    /// Empties the text and writes the empty query at once.
    fn clear(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.editor.cancel_composition();
        self.editor.dragging = false;
        if !self.editor.text.is_empty() {
            let len = self.editor.text.len();
            self.editor.replace(cx, 0..len, "", true)?;
        }
        self.pending = true;
        self.flush(cx)
    }
    /// The keys this field answers itself before its editor sees them:
    /// `Some(Handled)` when it acted, `Some(Ignored)` for a key it leaves to
    /// its owner, and `None` for ordinary editing keys.
    fn owner_key(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
    ) -> Result<Option<InputOutcome>, RuntimeError> {
        let fresh = e.transition == KeyTransition::Pressed;
        if !self.editor.enabled
            || !(fresh || e.transition == KeyTransition::Repeated)
            || self.editor.preedit.is_some()
        {
            return Ok(None);
        }
        if self.editor.menu_shown() {
            // Focus stays in the field while its clipboard menu is open: Escape
            // belongs to that menu, and the other keys edit as in a TextField.
            return Ok((e.code == KeyCode::Escape).then_some(InputOutcome::Ignored));
        }
        let m = cx.modifiers();
        let plain = !m.shift && !m.control && !m.alt && !m.super_key;
        let key = match e.code {
            KeyCode::Escape => {
                if !fresh {
                    return Ok(None);
                }
                if self.has_query() && self.can_clear() {
                    self.clear(cx)?;
                    cx.release_pointer();
                    return Ok(Some(InputOutcome::Handled));
                }
                return Ok(Some(InputOutcome::Ignored));
            }
            KeyCode::ArrowUp if plain => SearchKey::Up,
            KeyCode::ArrowDown if plain => SearchKey::Down,
            KeyCode::PageUp if plain => SearchKey::PageUp,
            KeyCode::PageDown if plain => SearchKey::PageDown,
            KeyCode::Enter => {
                if !fresh {
                    // A held Enter submits once.
                    return Ok(Some(if self.report.is_some() {
                        InputOutcome::Handled
                    } else {
                        InputOutcome::Ignored
                    }));
                }
                self.flush(cx)?;
                SearchKey::Submit
            }
            _ => return Ok(None),
        };
        Ok(Some(match &mut self.report {
            Some(report) => {
                report(cx, key)?;
                InputOutcome::Handled
            }
            None => InputOutcome::Ignored,
        }))
    }
    /// Lays out the icon, the editing area and the clear button's reserved slot.
    fn place(&self, cx: &MeasureContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let px = cx
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let py = cx
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let gap = cx.metric(MetricRole::IconGap);
        let side = cx
            .metric(MetricRole::IconSize)
            .min(size.height())
            .min(((size.width() - 2.0 * px - 2.0 * gap) / 3.0).max(0.0));
        let y = (size.height() - side) / 2.0;
        let icon = rect(px, y, side, side)?;
        let clear_x = (size.width() - px - side).max(0.0);
        let clear = rect(clear_x, y, side, side)?;
        let start = px + side + gap;
        self.icon.set(Some(icon));
        self.clear.set(Some(clear));
        self.editor.area.set(Some(rect(
            start,
            py,
            (clear_x - gap - start).max(0.0),
            (size.height() - 2.0 * py).max(0.0),
        )?));
        self.editor.arrange(cx, size)
    }
    fn clear_shown(&self) -> bool {
        self.can_clear() && !self.editor.text.is_empty()
    }
    fn over_clear(&self, cx: &EventContext, point: Point) -> bool {
        let local = cx.to_local(point).unwrap_or(point);
        self.clear_shown() && self.clear.get().is_some_and(|r| r.contains(local))
    }
    fn set_clear_hover(&mut self, cx: &mut EventContext, hover: bool) {
        if self.clear_hover != hover {
            self.clear_hover = hover;
            cx.invalidate(Invalidation::Paint);
        }
    }
}

impl Component for SearchField {
    /// Applies a new description: binding, typography, permissions,
    /// placeholder, debounce and key callback. A different binding ends the
    /// edit in progress as [`TextField`] does, and a pending debounced edit
    /// is dropped unwritten, as for hiding; so is a pending edit when the new
    /// description has no debounce or disables the field. Reenabling shows
    /// the bound query, never the discarded pending text. A new delay for a pending edit restarts
    /// its deadline with that delay within the same flush.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let rebound = !self.value.read_only().same_source(&next.value.read_only());
        let mut invalidation = self.editor.adopt(next.editor);
        if (rebound || !self.editor.enabled || next.debounce.is_none()) && self.pending {
            self.pending = false;
            self.drop_deadline = true;
            if !rebound {
                // The bound text is still current: show it again.
                self.discard();
            }
            invalidation =
                Some(invalidation.map_or(Invalidation::Paint, |i| i.max(Invalidation::Paint)));
        }
        if self.pending && next.debounce.is_some() && next.debounce != self.debounce {
            self.rearm = true;
            // A pending edit implies an earlier update, which kept this.
            if let Some(notifier) = &self.notifier {
                notifier.request();
            }
        }
        self.value = next.value;
        self.debounce = next.debounce;
        self.editor.deferred = self.debounce.is_some();
        self.report = next.report;
        if self.placeholder.reconcile(next.placeholder) {
            invalidation =
                Some(invalidation.map_or(Invalidation::Paint, |i| i.max(Invalidation::Paint)));
        }
        invalidation
    }
    fn focusable(&self) -> bool {
        self.editor.enabled
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        if self.clear_shown() && self.clear.get().is_some_and(|r| r.contains(point)) {
            return Some(CursorIcon::Default);
        }
        self.editor.cursor(point)
    }
    fn text_input_state(&self) -> Option<TextInputState> {
        self.editor.input_state()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.drop_deadline) && !self.pending {
            cx.cancel_timer();
        }
        if self.notifier.is_none() {
            self.notifier = Some(cx.notifier());
        }
        let observed = self.editor.observed.clone();
        self.editor.update(cx)?;
        // The hint is painted, never measured.
        self.placeholder.update_paint(cx)?;
        if self.editor.take_revoked() && self.pending {
            // A revoked field discards its pending edit as disabling it by
            // redescription does, before the deadline can write it.
            self.pending = false;
            self.discard();
            cx.cancel_timer();
        }
        if self.editor.observed != observed {
            // An application write wins over a pending edit, which the editor
            // has replaced with the bound text; its deadline is cancelled so
            // no wake remains scheduled for it.
            if self.pending {
                self.pending = false;
                cx.cancel_timer();
            }
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.editor.measure(cx, available)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.editor.menu_children()
    }
    fn child_visible(&self, key: &str) -> bool {
        key == "menu" && self.editor.menu_shown()
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "menu").then_some(SurfaceKind::ContextMenu)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.editor.close_menu()
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.place(cx.measurement(), size)?;
        self.editor.arrange_menu(cx, size)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.editor.paint(p)?;
        let muted = p.color(ColorRole::Muted);
        if let Some(icon) = self.icon.get() {
            p.icon(Icon::Search, icon, muted)?;
        }
        if self.editor.text.is_empty()
            && self.editor.preedit.is_none()
            && !self.placeholder.text().is_empty()
            && let Some(area) = self.editor.area.get()
        {
            let layout = p.rich_text(&self.placeholder.display(), None, self.editor.style)?;
            let [x, y, _, h] = area.values();
            p.with_clip(area, |p| {
                layout.paint(
                    p,
                    crate::Point::new(x, y + (h - layout.size().height()).max(0.0) / 2.0)?,
                    muted,
                )
            })?;
        }
        if self.clear_shown()
            && let Some(clear) = self.clear.get()
        {
            if self.clear_hover {
                p.fill_shape(
                    clear,
                    crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
                    p.color(ColorRole::ButtonHover),
                )?;
            }
            p.icon(Icon::Close, clear, p.color(ColorRole::OnSurface))?;
        }
        Ok(())
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.editor.set_hover(cx, hovered);
        if !hovered {
            self.set_clear_hover(cx, false);
        }
    }
    fn text_key_event(
        &mut self,
        cx: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        _: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        let event = input.editing_event(transition);
        match self.owner_key(cx, event)? {
            Some(InputOutcome::Handled) => return Ok(TextKeyOutcome::Handled),
            Some(InputOutcome::Ignored) => return Ok(TextKeyOutcome::Ignored),
            None => {}
        }
        let outcome = self.editor.key(cx, event)?;
        self.edited(cx)?;
        if outcome == InputOutcome::Handled {
            return Ok(TextKeyOutcome::Handled);
        }
        Ok(unhandled_text_key(cx, input, transition))
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let e = KeyEvent {
            code: keys.control_key(e),
            ..e
        };
        if let Some(outcome) = self.owner_key(cx, e)? {
            return Ok(outcome);
        }
        let outcome = self.editor.key(cx, e)?;
        self.edited(cx)?;
        Ok(outcome)
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        let outcome = self.editor.command(cx, command)?;
        self.edited(cx)?;
        Ok(outcome)
    }
    fn text_input(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        self.editor.insert(cx, text)?;
        self.edited(cx)
    }
    fn text_preedit(
        &mut self,
        cx: &mut EventContext,
        text: &TextPreedit,
    ) -> Result<(), RuntimeError> {
        // Composition never changes the committed text or the query.
        self.editor.compose(cx, text)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.editor.enabled {
            return Ok(InputOutcome::Ignored);
        }
        match e.kind() {
            PointerKind::Move => {
                let over = !e.is_outside() && self.over_clear(cx, e.position());
                self.set_clear_hover(cx, over);
            }
            PointerKind::Leave => self.set_clear_hover(cx, false),
            PointerKind::Press(PointerButton::Primary)
                if !e.is_outside() && self.over_clear(cx, e.position()) =>
            {
                self.clear(cx)?;
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        let outcome = self.editor.pointer(cx, e)?;
        self.edited(cx)?;
        Ok(outcome)
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        // Release local input first, so an uncommitted composition is dropped
        // rather than written; then a pending query is written.
        self.editor.reset();
        self.clear_hover = false;
        cx.invalidate(Invalidation::Paint);
        self.flush(cx)
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.flush(cx)
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.rearm)
            && self.pending
            && let Some(delay) = self.debounce
        {
            cx.set_timer(delay)?;
        }
        Ok(())
    }
    fn visibility_changed(&mut self, visible: bool) {
        if !visible {
            self.discard();
        }
    }
    fn input_reset(&mut self) {
        self.editor.reset();
        self.clear_hover = false;
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.input_reset();
        // Cancellation never writes. A field that keeps focus keeps its pending
        // edit for the next edit, Enter or focus loss; otherwise it is dropped.
        if !cx.focused() {
            self.discard();
        }
        cx.invalidate(Invalidation::Paint);
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = self.editor.inspect();
        details.properties.insert(
            "query".into(),
            InspectValue::Text(self.editor.observed.clone()),
        );
        details
            .properties
            .insert("pending".into(), InspectValue::Bool(self.pending));
        details.properties.insert(
            "placeholder".into(),
            InspectValue::Text(self.placeholder.text().to_owned()),
        );
        if let Some(semantics) = &mut details.semantics {
            semantics.role = SemanticRole::SearchInput;
        }
        details
    }
}
