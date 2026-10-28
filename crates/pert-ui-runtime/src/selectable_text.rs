//! Display text a reader can select and copy, without editing it.
use crate::text_fields::{FieldMenu, command_modifier, key_movement, place_menu};
use crate::text_source::Caption;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The actions of the component's clipboard menu, in the order it shows them.
const MENU: [(TextCommand, &str, StringRole); 2] = [
    (TextCommand::Copy, "copy", StringRole::TextCopy),
    (
        TextCommand::SelectAll,
        "select-all",
        StringRole::TextSelectAll,
    ),
];

/// Read-only text that a reader can select with the pointer or the keyboard
/// and copy, such as a log message, an error's details or an identifier.
///
/// It looks and measures like a multiline [`Label`]: no frame, the height of
/// all its lines when [wrapped](Self::wrap) and no scrolling of its own, so
/// long text goes in a [`ScrollArea`]. Unlike a label it takes focus (one Tab
/// stop; empty text is none) and shows a focus ring while focused.
///
/// - **Pointer:** a primary press focuses it and places the caret; dragging
///   extends the selection; Shift+press extends it from its anchor. A second
///   press in a series (as [`ClickCounter`] counts them) selects the word,
///   a third everything, and a fourth starts over. A drag does not scroll an
///   enclosing scroll area, and a selection's end is not revealed beyond the
///   whole component.
/// - **Keyboard:** the text editors' movement keys move or, with Shift,
///   extend the selection: the arrows, the word modifier with the arrows
///   (Option on macOS, Control elsewhere), Home and End, and with the command
///   modifier (Command on macOS, Control elsewhere) the text's ends. The
///   command modifier with A selects all and with C copies. Committed text
///   and IME composition change nothing.
/// - **Escape** first ends a drag in progress (keeping its selection), then
///   collapses a selection, and only then reaches its ancestors, so inside a
///   popover the first Escape collapses the selection and the second closes
///   the popover. It is answered in the text phase on both
///   [`Runtime::key_input`] and [`Runtime::command_key_input`], before capture
///   bindings. On the native path every key is read in the text phase by its
///   layout identity, as the editors read theirs, so the command modifier
///   with the key that types A selects all on any keyboard layout.
/// - **Menu:** a secondary press, the ContextMenu key or Shift+F10 opens a
///   clipboard menu with Copy (available with a selection) and Select All,
///   labeled by [`StringRole::TextCopy`] and [`StringRole::TextSelectAll`].
///   It keeps the selection. Its actions are queued [`TextCommand`]s for this
///   component.
/// - **Commands:** [`TextCommand::Copy`] writes the selected text through the
///   runtime's clipboard service and reports `Unavailable` without a
///   selection; [`TextCommand::SelectAll`] selects everything; every command
///   that would change the text reports `Unavailable`. A clipboard failure is
///   returned as its error.
///
/// The text is a [`TextSource`]: fixed text, a string or rich-text binding,
/// or a string-table role. Rich content is shown and copied as its plain
/// text, without styling, icons or gaps; icon-only content shows nothing.
/// When a binding replaces the text, a selection whose ends both lie inside
/// the new text keeps its offsets, snapped to grapheme boundaries and in its
/// direction; otherwise it collapses to the start. A replacement ends a drag
/// in progress, as does a redescription with another source, even one with
/// the same text; a redescription with the same source keeps everything.
/// Text that becomes empty gives up focus at the end of the flush, and
/// cancelled input ([`HostIntegration::cancel_input`](crate::host::HostIntegration::cancel_input),
/// focus loss, hiding) ends a
/// drag and keeps the selection.
///
/// Text that cannot be prepared, longer than
/// [`TextEditError::MAX_TEXT_BYTES`] or beyond a raster limit at the current
/// width, style or scale ([`TextLimit`]), is refused without failing the
/// flush: the component keeps showing the last text it prepared, with its
/// selection, reports the refusal as the `text_error` inspection property,
/// marks its semantics invalid, and recovers once a later text or size fits.
/// Other errors, such as a binding from another runtime, fail the update.
///
/// Inspection reports `text` (the shown text), `selection` (`anchor..focus`
/// in UTF-8 bytes, so a backward selection shows its direction) and a
/// [`SemanticRole::Label`] named by the whole text and marked read-only. The
/// text is as visible to inspection and content search as a label's, so it is
/// no place for secrets.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::testing::HostIntegration;
/// use pert_ui::prelude::*;
/// use pert_ui::{Clipboard, ClipboardError, TextCommandOutcome};
/// use std::{cell::RefCell, rc::Rc, time::Duration};
/// struct Memory(Rc<RefCell<String>>);
/// impl Clipboard for Memory {
///     fn read_text(&mut self) -> Result<String, ClipboardError> { Ok(self.0.borrow().clone()) }
///     fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> {
///         *self.0.borrow_mut() = text.to_owned();
///         Ok(())
///     }
/// }
/// let mut ui = Runtime::new();
/// let copied = Rc::new(RefCell::new(String::new()));
/// ui.set_clipboard(Memory(copied.clone()))?;
/// let message = ui.signal("connection closed after 3 retries".to_owned());
/// let text = ui.mount_tree(SelectableText::new(message.read_only()).key("message"))?;
/// ui.flush(Duration::ZERO)?;
/// assert_eq!(ui.text_command(&text, TextCommand::SelectAll)?, TextCommandOutcome::Handled);
/// assert_eq!(ui.text_command(&text, TextCommand::Copy)?, TextCommandOutcome::Handled);
/// assert_eq!(*copied.borrow(), "connection closed after 3 retries");
/// // The text itself never changes.
/// assert_eq!(ui.text_command(&text, TextCommand::Paste)?, TextCommandOutcome::Unavailable);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct SelectableText {
    caption: Caption,
    style: TextStyle,
    wrap: bool,
    /// The selection, in the offsets of the shown text.
    selection: Cell<TextSelection>,
    /// The last text that was prepared, with its layout; what is painted,
    /// hit-tested and copied.
    shown: RefCell<Option<Shown>>,
    /// Why the latest arrangement refused the current text.
    refusal: RefCell<Option<String>>,
    arranged: Cell<Option<Size>>,
    /// The caption's text as of the latest update, to notice a replacement.
    seen: Option<String>,
    /// A redescription brought another source, read by the next update.
    rebound: bool,
    dragging: bool,
    clicks: u8,
    click_series: ClickCounter,
    focused: bool,
    /// Built by the first update, which can create its signals.
    menu: Option<FieldMenu>,
}
description_debug!(SelectableText);

struct Shown {
    text: String,
    layout: TextLayout,
}

impl SelectableText {
    /// Shows `text`, wrapped, in body style, with nothing selected.
    pub fn new(text: impl Into<TextSource>) -> Self {
        Self {
            caption: Caption::new(text),
            style: TextStyle::Body,
            wrap: true,
            selection: Cell::new(TextSelection::default()),
            shown: RefCell::new(None),
            refusal: RefCell::new(None),
            arranged: Cell::new(None),
            seen: None,
            rebound: false,
            dragging: false,
            clicks: 0,
            click_series: ClickCounter::new(),
            focused: false,
            menu: None,
        }
    }
    /// Selects a semantic typography preset. Default is [`TextStyle::Body`].
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
    /// Whether long lines wrap at the allocated width. Default is true; when
    /// false each line keeps its natural width and is clipped at the
    /// allocation, as for a single-line label.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    fn prepare(
        &self,
        cx: &MeasureContext<'_>,
        width: Option<f32>,
    ) -> Result<TextLayout, RuntimeError> {
        let text = self.caption.text();
        if text.len() > TextEditError::MAX_TEXT_BYTES {
            return Err(TextEditError::TooLarge.into());
        }
        cx.text(text, if self.wrap { width } else { None }, self.style)
    }
    fn layout(&self) -> Option<TextLayout> {
        self.shown
            .borrow()
            .as_ref()
            .map(|shown| shown.layout.clone())
    }
    fn select(&self, cx: &mut EventContext, selection: TextSelection) {
        if self.selection.get() != selection {
            self.selection.set(selection);
            cx.invalidate(Invalidation::Paint);
        }
    }
    /// Ends a drag, or else collapses a selection; whether Escape did either.
    fn escape(&mut self, cx: &mut EventContext) -> bool {
        if self.dragging {
            self.dragging = false;
            cx.release_pointer();
            return true;
        }
        let selection = self.selection.get();
        if selection.byte_range().is_empty() {
            return false;
        }
        if let Some(layout) = self.layout() {
            self.select(
                cx,
                TextSelection::snapped(&layout, selection.focus_byte(), selection.focus_byte()),
            );
        }
        true
    }
    /// One key, by the identity the editors read: Escape, the menu keys, the
    /// command chords and the movement keys.
    fn key(&mut self, cx: &mut EventContext, e: KeyEvent) -> Result<InputOutcome, RuntimeError> {
        if !matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(InputOutcome::Ignored);
        }
        let m = cx.modifiers();
        if e.code == KeyCode::Escape {
            let handled = e.transition == KeyTransition::Pressed && self.escape(cx);
            return Ok(if handled {
                InputOutcome::Handled
            } else {
                InputOutcome::Ignored
            });
        }
        if e.requests_context_menu(m) {
            return self.open_menu(cx, None);
        }
        let command = command_modifier(m);
        if command && e.code == KeyCode::KeyA {
            self.text_command(cx, TextCommand::SelectAll)?;
            return Ok(InputOutcome::Handled);
        }
        if command && e.code == KeyCode::KeyC {
            self.text_command(cx, TextCommand::Copy)?;
            return Ok(InputOutcome::Handled);
        }
        if let Some(movement) = key_movement(e.code, m)
            && let Some(layout) = self.layout()
        {
            self.select(
                cx,
                self.selection.get().moved(
                    &layout,
                    movement,
                    crate::SelectionExtension::from_shift(m.shift),
                ),
            );
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn open_menu(
        &mut self,
        cx: &mut EventContext,
        position: Option<Point>,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(menu) = &self.menu else {
            return Ok(InputOutcome::Ignored);
        };
        let selected = !self.selection.get().byte_range().is_empty();
        let text = !self.caption.text().is_empty();
        for (command, available) in &menu.available {
            available.set(match command {
                TextCommand::Copy => selected,
                TextCommand::SelectAll => text,
                _ => false,
            })?;
        }
        menu.position.set(position);
        menu.open.set(true)?;
        cx.invalidate(Invalidation::Layout);
        Ok(InputOutcome::Handled)
    }
}

/// Whether a preparation error refuses the text rather than failing: the
/// size limits, which a later text, width, style or scale can meet.
pub(crate) fn refused(error: &RuntimeError) -> bool {
    matches!(
        error,
        RuntimeError::Text(TextError::TooLarge(_))
            | RuntimeError::TextEdit(TextEditError::TooLarge)
    )
}

/// `index` moved back to a character boundary of `text`, within it.
fn boundary(text: &str, index: usize) -> usize {
    let mut index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

impl Component for SelectableText {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.rebound |= !self.caption.source().same_as(next.caption.source());
        let layout = self.style != next.style || self.wrap != next.wrap;
        let layout = self.caption.reconcile(next.caption) | layout;
        self.style = next.style;
        self.wrap = next.wrap;
        layout.then_some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.caption.update(cx)?;
        let replaced = self.seen.as_deref() != Some(self.caption.text());
        if (replaced || std::mem::take(&mut self.rebound)) && self.seen.is_some() && self.dragging {
            // The text under the drag was replaced.
            self.dragging = false;
            cx.cancel_input();
        }
        if replaced {
            self.seen = Some(self.caption.text().to_owned());
        }
        let focused = cx.focused();
        if !focused {
            self.dragging = false;
        }
        self.focused = focused;
        if self.menu.is_none() {
            let strings = cx.strings()?;
            let mut menu = FieldMenu::with_actions(&MENU, &strings, |value| cx.signal(value))?;
            *menu.target.borrow_mut() = Some(Rc::downgrade(&cx.id().0));
            cx.request_children(menu.items.take().into_iter().collect());
            self.menu = Some(menu);
        }
        if let Some(menu) = &mut self.menu {
            // Subscribing here is what hides the menu again once an action,
            // Escape or an outside press closes it.
            menu.shown = cx.read(&menu.open)?;
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        match self.prepare(cx, available.width()) {
            Ok(layout) => Ok(layout.size()),
            Err(error) if refused(&error) => Ok(self
                .shown
                .borrow()
                .as_ref()
                .map_or(Size::ZERO, |shown| shown.layout.size())),
            Err(error) => Err(error),
        }
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.arranged.set(Some(size));
        match self.prepare(cx.measurement(), Some(size.width())) {
            Ok(layout) => {
                let text = self.caption.text();
                let old = self.selection.get();
                let replaced = self
                    .shown
                    .borrow()
                    .as_ref()
                    .is_none_or(|shown| shown.text != text);
                // Offsets that still lie inside the text are kept, snapped to
                // its graphemes; the selection is recreated on every new
                // layout so its geometry follows width and scale.
                let kept = !replaced
                    || (old.anchor_byte() <= text.len() && old.focus_byte() <= text.len());
                self.selection.set(if kept {
                    TextSelection::snapped(
                        &layout,
                        boundary(text, old.anchor_byte()),
                        boundary(text, old.focus_byte()),
                    )
                } else {
                    TextSelection::snapped(&layout, 0, 0)
                });
                *self.shown.borrow_mut() = Some(Shown {
                    text: text.to_owned(),
                    layout,
                });
                *self.refusal.borrow_mut() = None;
            }
            Err(error) if refused(&error) => {
                *self.refusal.borrow_mut() = Some(error.to_string());
            }
            Err(error) => return Err(error),
        }
        place_menu(cx, size, self.menu.as_ref())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let (Some(layout), Some(size)) = (self.layout(), self.arranged.get()) else {
            return Ok(());
        };
        let bounds = Rect::from_size(size);
        p.with_clip(bounds, |p| -> Result<(), DrawError> {
            let rectangles = self.selection.get().rectangles(&layout)?;
            for r in &rectangles {
                p.fill_rect(*r, p.color(ColorRole::TextSelection));
            }
            layout.paint(p, crate::Point::ZERO, p.color(ColorRole::OnSurface))?;
            for r in rectangles {
                p.with_clip(r, |p| {
                    layout.paint(p, crate::Point::ZERO, p.color(ColorRole::OnTextSelection))
                })?;
            }
            Ok(())
        })?;
        if self.focused {
            crate::choice_controls::focus_ring(p, bounds)?;
        }
        Ok(())
    }
    fn focusable(&self) -> bool {
        !self.caption.text().is_empty()
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Text)
    }
    fn child_visible(&self, key: &str) -> bool {
        key == "menu" && self.menu.as_ref().is_some_and(|menu| menu.shown)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "menu").then_some(SurfaceKind::ContextMenu)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        if let Some(menu) = &self.menu {
            menu.open.set(false)?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match e.kind() {
            PointerKind::Press(PointerButton::Secondary) => {
                let local = cx.to_local(e.position());
                self.open_menu(cx, local)
            }
            PointerKind::Press(PointerButton::Primary) if !e.is_outside() => {
                let Some(layout) = self.layout() else {
                    return Ok(InputOutcome::Ignored);
                };
                let point = cx.to_local(e.position()).unwrap_or(e.position());
                let consecutive =
                    self.click_series
                        .press(cx.now(), e.position(), PointerButton::Primary)
                        > 1;
                self.clicks = if consecutive { self.clicks % 3 + 1 } else { 1 };
                let selected = match self.clicks {
                    2 => TextSelection::word_at_point(&layout, point),
                    3 => TextSelection::snapped(&layout, 0, usize::MAX),
                    _ => self.selection.get().at_point(
                        &layout,
                        point,
                        crate::SelectionExtension::from_shift(cx.modifiers().shift),
                    ),
                };
                self.select(cx, selected);
                self.dragging = true;
                cx.capture_pointer()?;
                Ok(InputOutcome::Handled)
            }
            PointerKind::Move if self.dragging => {
                if self.clicks != 3
                    && let Some(layout) = self.layout()
                {
                    let point = cx.to_local(e.position()).unwrap_or(e.position());
                    self.select(
                        cx,
                        self.selection.get().at_point(
                            &layout,
                            point,
                            crate::SelectionExtension::Extend,
                        ),
                    );
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.dragging => {
                self.dragging = false;
                cx.release_pointer();
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
        _: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        // Keys are read by their layout identity here, before capture
        // bindings; what is not handled bubbles from the parent, so the
        // ordinary phase never reads the same key again by its physical code.
        Ok(match self.key(cx, input.editing_event(transition))? {
            InputOutcome::Handled => TextKeyOutcome::Handled,
            _ => TextKeyOutcome::Ignored,
        })
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.key(
            cx,
            KeyEvent {
                code: keys.control_key(e),
                ..e
            },
        )
    }
    fn input_reset(&mut self) {
        self.dragging = false;
        self.click_series.reset();
        self.clicks = 0;
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        match command {
            TextCommand::Copy => {
                let range = self.selection.get().byte_range();
                let selected = self
                    .shown
                    .borrow()
                    .as_ref()
                    .and_then(|shown| shown.text.get(range).map(str::to_owned))
                    .filter(|selected| !selected.is_empty());
                let Some(selected) = selected else {
                    return Ok(TextCommandOutcome::Unavailable);
                };
                cx.set_clipboard_text(&selected)?;
                Ok(TextCommandOutcome::Handled)
            }
            TextCommand::SelectAll => {
                let Some(layout) = self.layout().filter(|_| !self.caption.text().is_empty()) else {
                    return Ok(TextCommandOutcome::Unavailable);
                };
                self.select(cx, TextSelection::snapped(&layout, 0, usize::MAX));
                Ok(TextCommandOutcome::Handled)
            }
            // The text never changes.
            _ => Ok(TextCommandOutcome::Unavailable),
        }
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let shown = self.shown.borrow();
        let text = shown.as_ref().map_or("", |shown| shown.text.as_str());
        details
            .properties
            .insert("text".into(), InspectValue::Text(text.to_owned()));
        let selection = self.selection.get();
        details.properties.insert(
            "selection".into(),
            InspectValue::Text(format!(
                "{}..{}",
                selection.anchor_byte(),
                selection.focus_byte()
            )),
        );
        let refusal = self.refusal.borrow();
        if let Some(refusal) = refusal.as_ref() {
            details
                .properties
                .insert("text_error".into(), InspectValue::Text(refusal.clone()));
        }
        details.semantics = Some(
            Semantics::new(SemanticRole::Label)
                .name(text)
                .read_only(true)
                .invalid(refusal.is_some()),
        );
        details
    }
}
