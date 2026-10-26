//! A frameless single-line text editor for a draft its owner commits or discards.
use super::*;
use std::rc::Weak;

/// Where a draft session places the selection when it begins.
///
/// Offsets are UTF-8 byte offsets into the draft text. A range must be ordered
/// and lie on character boundaries inside the text; anything else is rejected
/// with [`TextEditError::InvalidSelection`], never clamped.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DraftSelection {
    /// Select the whole text, as a value opened for editing usually is.
    All,
    /// A collapsed caret after the last character.
    End,
    /// Select `start..end`; a collapsed caret when both are equal.
    Range(Range<usize>),
}

/// Horizontal placement of a draft that is narrower than its editing area.
///
/// Wider text always scrolls to keep the caret visible, whatever the alignment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DraftAlign {
    /// Text starts at the leading edge. The default.
    #[default]
    Start,
    /// Text ends at the trailing edge, as a right-aligned number display does,
    /// so opening a value for editing does not make it jump.
    End,
}

type EditCallback<P> =
    Box<dyn FnMut(&mut EventContext, &str, Option<P>) -> Result<(), RuntimeError>>;

/// Editing state, borrowed by the editor while it handles input and by the
/// controller's session methods.
struct Core<P> {
    editor: Editor<Option<P>>,
    /// Token of the mounted editor that owns this controller.
    owner: Weak<()>,
    /// The owning editor's node, known after its first update.
    node: Weak<crate::runtime::Node>,
}

impl<P: Copy + PartialEq> Core<P> {
    /// Invalidates the editor and, when given, its owner (the logical parent).
    fn invalidate(&self, editor: Invalidation, owner: Option<Invalidation>) {
        if let Some(node) = self.node.upgrade() {
            node.invalidate(editor);
            if let (Some(kind), Some(parent)) = (owner, node.logical_parent()) {
                parent.invalidate(kind);
            }
        }
    }
    fn check_runtime(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        match self.node.upgrade() {
            Some(node) if !Rc::ptr_eq(&node.shared, &cx.node.shared) => {
                Err(RuntimeError::ForeignRuntime)
            }
            _ => Ok(()),
        }
    }
    fn clear(&mut self) {
        let editor = &mut self.editor;
        editor.reset();
        editor.active = false;
        editor.set_owner_lock(false);
        editor.text.clear();
        editor.payload = None;
        editor.undo.clear();
        editor.redo.clear();
        editor.selection.set(TextSelection::default());
        editor.scroll.set([0.0; 2]);
    }
}

/// The draft as of the last completed operation, which the getters read.
struct Published<P> {
    active: bool,
    read_only: bool,
    text: String,
    payload: Option<P>,
}

/// Everything a controller and its editor share.
///
/// `core` is held while the editor runs an operation, which may call into the
/// application's clipboard service. The getters read `published` and the
/// callbacks live in their own cells, and none of those is borrowed while
/// application code runs, so no controller method can find a cell borrowed.
struct Shared<P> {
    core: RefCell<Core<P>>,
    published: RefCell<Published<P>>,
    on_edit: RefCell<Option<EditCallback<P>>>,
    /// `end` was called while `core` was borrowed; applied when it is released.
    end_requested: Cell<bool>,
}

impl<P: Copy + PartialEq> Shared<P> {
    fn publish(&self, core: &Core<P>) {
        let mut published = self.published.borrow_mut();
        published.active = core.editor.active;
        published.read_only = core.editor.read_only;
        if published.text != core.editor.text {
            published.text.clone_from(&core.editor.text);
        }
        published.payload = core.editor.payload;
    }
    /// Ends the session held in `core`, telling the owner.
    fn end_in(&self, core: &mut Core<P>) {
        if core.editor.active {
            core.clear();
            self.publish(core);
            core.invalidate(Invalidation::Paint, Some(Invalidation::Layout));
        }
    }
    /// Applies an `end` that arrived while `core` was borrowed.
    fn settle(&self, core: &mut Core<P>) {
        if self.end_requested.take() {
            self.end_in(core);
        }
    }
}

/// The handle an owning control uses to drive its [`DraftEditor`].
///
/// A controller holds the **draft**: the text being edited plus an optional
/// payload `P`, the exact value that text stands for (for example the `f64`
/// behind a rounded display). The draft is never written anywhere by the
/// editor; the owner reads it with [`Self::text`] and [`Self::payload`] and
/// decides whether to commit it to its own value or to discard it.
///
/// Clones refer to the same draft. The controller is created before its editor
/// and outlives it; it is UI-thread only (`!Send`). One mounted editor may use
/// a controller at a time: a second mounted [`DraftEditor`] built from the same
/// controller fails its updates with [`RuntimeError::DraftEditorBusy`] until
/// the first one unmounts, and then takes the controller over. Describing the
/// same editor again with the same controller is not a second mount and keeps
/// the session.
///
/// The session methods take the owner's [`EventContext`], because a session
/// begins and changes in response to input: call them from the owner's
/// `keyboard_event`, `pointer_input` or a callback, not from `update`.
///
/// The getters report the draft as of the last completed operation and never
/// fail, including when called from the draft callbacks or from the
/// application's clipboard service while the editor is pasting.
///
/// Plain text uses the default payload `()`.
pub struct DraftEditorController<P = ()> {
    shared: Rc<Shared<P>>,
}

impl<P> Clone for DraftEditorController<P> {
    fn clone(&self) -> Self {
        Self {
            shared: self.shared.clone(),
        }
    }
}

impl<P> std::fmt::Debug for DraftEditorController<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DraftEditorController")
            .finish_non_exhaustive()
    }
}

impl<P: Copy + PartialEq + 'static> Default for DraftEditorController<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Copy + PartialEq + 'static> DraftEditorController<P> {
    /// Creates a controller with no active session.
    #[must_use]
    pub fn new() -> Self {
        let mut editor = Editor::new(None, false);
        editor.embedded = true;
        editor.active = false;
        Self {
            shared: Rc::new(Shared {
                core: RefCell::new(Core {
                    editor,
                    owner: Weak::new(),
                    node: Weak::new(),
                }),
                published: RefCell::new(Published {
                    active: false,
                    read_only: false,
                    text: String::new(),
                    payload: None,
                }),
                on_edit: RefCell::new(None),
                end_requested: Cell::new(false),
            }),
        }
    }

    fn core(&self) -> Result<std::cell::RefMut<'_, Core<P>>, RuntimeError> {
        self.shared
            .core
            .try_borrow_mut()
            .map_err(|_| RuntimeError::BorrowConflict)
    }

    fn start(
        &self,
        cx: &mut EventContext,
        text: &str,
        payload: Option<P>,
        selection: DraftSelection,
        read_only: bool,
    ) -> Result<(), RuntimeError> {
        let mut core = self.core()?;
        core.check_runtime(cx)?;
        check_text(text, false)?;
        let len = text.len();
        let (anchor, focus) = match selection {
            DraftSelection::All => (0, len),
            DraftSelection::End => (len, len),
            DraftSelection::Range(range) => {
                if range.start > range.end
                    || range.end > len
                    || !text.is_char_boundary(range.start)
                    || !text.is_char_boundary(range.end)
                {
                    return Err(TextEditError::InvalidSelection {
                        anchor_byte: range.start,
                        focus_byte: range.end,
                    }
                    .into());
                }
                (range.start, range.end)
            }
        };
        let layout = cx.text(text, None, core.editor.style)?;
        core.clear();
        let editor = &mut core.editor;
        editor.text = text.to_owned();
        editor.payload = payload;
        editor.initialized = true;
        // A new session is a new IME session: the host resets any native
        // composition left from an earlier one.
        editor.cancel_composition();
        editor
            .selection
            .set(TextSelection::snapped(&layout, anchor, focus));
        editor.reveal.set(true);
        editor.restart_blink = true;
        editor.active = true;
        editor.set_owner_lock(read_only);
        self.shared.end_requested.set(false);
        self.shared.publish(&core);
        if core.node.upgrade().is_some() {
            core.invalidate(Invalidation::Layout, Some(Invalidation::Layout));
        } else {
            // A hidden editor that has never been shown has not recorded its
            // node yet; `cx` is the owner's, which shows it from here.
            cx.invalidate(Invalidation::Layout);
        }
        Ok(())
    }

    /// Starts an editing session on `text`, replacing any active one.
    ///
    /// The previous session's undo history, composition and selection are
    /// discarded; a new session never carries history over. `payload` is the
    /// exact value `text` stands for, or `None`. The editor becomes focusable;
    /// the owner then moves focus to it with
    /// [`EventContext::request_focus_for`] and makes it visible.
    ///
    /// # Errors
    /// Returns [`RuntimeError::TextEdit`] with [`TextEditError::TooLarge`] or
    /// [`TextEditError::InvalidText`] for text a single-line editor does not
    /// accept (see [`TextField::check`]), and with
    /// [`TextEditError::InvalidSelection`] for a range that is not ordered or
    /// not on character boundaries inside `text`; the editor is then unchanged.
    /// Returns a text preparation error when the text cannot be laid out, and
    /// `ForeignRuntime` when the editor, once it has been shown, belongs to
    /// another runtime than `cx`. Returns `BorrowConflict` when called while
    /// the editor is running an operation, which only the application's
    /// clipboard service can do.
    pub fn begin(
        &self,
        cx: &mut EventContext,
        text: &str,
        payload: Option<P>,
        selection: DraftSelection,
    ) -> Result<(), RuntimeError> {
        self.start(cx, text, payload, selection, false)
    }

    /// Starts a session in which the text can be selected and copied but not
    /// edited by the user, replacing any active one. The IME receives no text
    /// input state, and [`Self::on_edit`] never runs during such a session.
    /// The owner may still change the text with [`Self::replace`], for example
    /// to follow a live readout.
    ///
    /// # Errors
    /// As [`Self::begin`].
    pub fn begin_read_only(
        &self,
        cx: &mut EventContext,
        text: &str,
        selection: DraftSelection,
    ) -> Result<(), RuntimeError> {
        self.start(cx, text, None, selection, true)
    }

    /// Replaces the whole draft with `text` and `payload` as one undoable edit,
    /// leaving a collapsed caret at its end. This is how an owner steps a value
    /// while it is being edited. Any composition is cancelled. It also works in
    /// a read-only session.
    ///
    /// Returns `false`, changing nothing, when no session is active. Does not
    /// run [`Self::on_edit`], which reports the user's edits only.
    ///
    /// # Errors
    /// Returns [`RuntimeError::TextEdit`] for text a single-line editor does not
    /// accept, a text preparation error when the text cannot be laid out, and
    /// `ForeignRuntime` and `BorrowConflict` as [`Self::begin`] does. The draft
    /// is unchanged on every error.
    pub fn replace(
        &self,
        cx: &mut EventContext,
        text: &str,
        payload: Option<P>,
    ) -> Result<bool, RuntimeError> {
        let mut core = self.core()?;
        core.check_runtime(cx)?;
        if !core.editor.active {
            return Ok(false);
        }
        check_text(text, false)?;
        let len = core.editor.text.len();
        let replaced = core
            .editor
            .replace_with_payload(cx, 0..len, text, true, payload)?;
        // Only a replacement ends the composition; a refused one keeps it.
        if replaced {
            core.editor.cancel_composition();
        }
        self.shared.publish(&core);
        core.invalidate(Invalidation::Paint, Some(Invalidation::Paint));
        Ok(replaced)
    }

    /// Ends the session, discarding the draft, its undo history, any
    /// composition and pointer gesture. The editor stops being focusable, and
    /// its owner is re-evaluated so it can hide it. Idempotent; the owner
    /// commits the draft first if it wants to keep it.
    ///
    /// Called while the editor is running an operation, which only the
    /// application's clipboard service can do, the session ends as soon as
    /// that operation completes.
    pub fn end(&self) {
        match self.shared.core.try_borrow_mut() {
            Ok(mut core) => self.shared.end_in(&mut core),
            Err(_) => self.shared.end_requested.set(true),
        }
    }

    /// Whether the editor is composing or drag-selecting, so an owner must not
    /// replace the draft under it. A controller whose editor is running an
    /// operation counts as busy.
    pub(crate) fn busy(&self) -> bool {
        self.shared.core.try_borrow().map_or(true, |core| {
            core.editor.preedit.is_some() || core.editor.dragging
        })
    }

    /// Whether a session (editable or read-only) is active.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.shared.published.borrow().active
    }

    /// Whether the active session is read-only; `false` when none is active.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        let published = self.shared.published.borrow();
        published.active && published.read_only
    }

    /// The draft text; empty when no session is active.
    #[must_use]
    pub fn text(&self) -> String {
        self.shared.published.borrow().text.clone()
    }

    /// The draft's payload: the value given to [`Self::begin`] or
    /// [`Self::replace`] while the text is still what they set, `None` after
    /// the user edits the text, and whatever undo or redo restores. `None` when
    /// no session is active.
    #[must_use]
    pub fn payload(&self) -> Option<P> {
        self.shared.published.borrow().payload
    }

    /// Runs `callback` after each change the user makes to the draft: typing,
    /// an IME commit, paste, cut, deletion, undo and redo. It receives the
    /// editor's context, the new draft text and payload. It does not run for
    /// [`Self::begin`], [`Self::replace`] or [`Self::end`]. Replaces an earlier
    /// callback.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    ///
    /// The owner is repainted after each such change, so it can show live
    /// validation, such as an error outline while the draft does not parse.
    /// The callback may call this controller's methods. Its error propagates
    /// from the input call that made the change; the change itself is kept.
    pub fn on_edit(
        &self,
        callback: impl FnMut(&mut EventContext, &str, Option<P>) -> Result<(), RuntimeError> + 'static,
    ) {
        *self.shared.on_edit.borrow_mut() = Some(Box::new(callback));
    }
}

/// A frameless, single-line **text** editor for an uncommitted draft that its
/// owner commits or discards.
///
/// This is a complete, mounted component, not a preliminary one: "draft" names
/// the text it edits. It is the embeddable part a custom control composes to
/// get a full text mode (selection, word movement, undo and redo, IME,
/// clipboard, caret blink) inside its own row, the kind of text mode the
/// built-in inline controls have.
///
/// # `DraftEditor` or `TextField`
///
/// A [`TextField`] is a standalone field: it writes its bound signal on every
/// edit, handles Enter itself and draws a frame and a clipboard menu. A
/// `DraftEditor` writes nothing. Its [`DraftEditorController`] holds the draft,
/// and the owning control reads it and decides. Use a `TextField` for a form
/// field and a `DraftEditor` for editing a value in place inside another
/// control.
///
/// # Contract for the owner
///
/// - **Visibility.** Mount the editor as a child and report it visible (see
///   [`Component::child_visible`]) only while a session is active. A hidden
///   editor takes no input and schedules no caret blink.
/// - **Focus.** The editor is focusable exactly while a session is active.
///   After [`DraftEditorController::begin`], move focus to it with
///   [`EventContext::request_focus_for`]. Report the owner itself as not
///   focusable while editing, so Shift+Tab leaves the row instead of landing
///   on it, and paint the focus outline while
///   [`UpdateContext::focus_within`] is true. The editor draws no frame and no
///   focus outline.
/// - **Keys.** The editor handles caret movement, selection, deletion,
///   select-all, clipboard chords and undo/redo. It leaves Enter, Up, Down,
///   Page Up and Page Down to the owner, where they arrive by ordinary
///   bubbling: commit on Enter, step on Up/Down. An unmodified Tab or
///   Shift+Tab never reaches the owner's `keyboard_event`. Before ordinary
///   bubbling the runtime offers it to the focused component's ancestors
///   through [`Component::tab_navigation`], where an owner that moves its
///   edit itself, as a table moves between cells, claims it. Unclaimed, it
///   moves focus to the next Tab stop, and the owner learns of it as a
///   departure (see **Ending**). Escape reaches the owner too,
///   unless it cancels the editor's own composition or drag-selection; the
///   owner cancels the session on it. While a composition is active the editor
///   keeps every key.
/// - **Ending.** To commit, read the draft, write your value, call
///   [`DraftEditorController::end`] and request focus for the owner. When focus
///   leaves the owner's subtree by other means (Tab, a press elsewhere, native
///   window focus loss), the owner receives
///   [`Component::focus_within_changed`] with `false` and applies its own
///   policy there (save or cancel) with its own `&mut self` and context; it
///   must not take focus back. Hiding and removal cancel without that
///   notification. Beginning a session runs the owner's
///   [`Component::cancel_input`], because the owner stops being focusable, so
///   the owner must not end the draft there: a session that loses focus ends
///   in `focus_within_changed(false)`.
/// - **Redescription.** An owner its parent describes again, such as a row of
///   a list whose source is replaced, keeps the controller and the mounted
///   editor paired: it returns `true` from
///   [`Component::retain_initial_children`], so the incoming editor is
///   discarded and the mounted one stays, or it reconciles the controller and
///   the editor child together. Otherwise ordinary reconciliation adopts the
///   incoming editor, and the owner and the editor it shows can end up on
///   different controllers.
/// - **Naming.** The editor reports `TextInput` semantics without a name. An
///   owner that names it declares [`Semantics::names_children`].
///
/// The owner is re-evaluated whenever a session begins or ends and after each
/// edit the user makes, so its `child_visible`, `focusable` and painting can
/// follow [`DraftEditorController::is_active`] and the draft without reading
/// a signal of their own.
///
/// Hiding ends a composition and a pointer gesture but not the session;
/// unmounting the editor ends the session.
///
/// # Presentation
///
/// Defaults to [`TextStyle::Body`] and [`DraftAlign::Start`]. The preferred
/// size is one line of the style high and up to `MetricRole::ControlWidth`
/// wide; the owner arranges the editor into its own value area, and wider text
/// scrolls horizontally. There is no scrollbar and no context menu; an owner
/// that wants one composes a [`ContextMenu`] whose items call
/// [`EventContext::request_text_command`] on the editor.
///
/// # Example
///
/// A row that opens its value for editing on Enter, commits on Enter and
/// cancels on Escape:
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
///
/// struct Row {
///     value: Signal<String>,
///     draft: DraftEditorController,
///     editor: Option<WidgetId>,
///     children: Vec<Element>,
/// }
///
/// impl Component for Row {
///     fn take_children(&mut self) -> Vec<Element> {
///         std::mem::take(&mut self.children)
///     }
///     fn focusable(&self) -> bool {
///         !self.draft.is_active()
///     }
///     fn child_visible(&self, _key: &str) -> bool {
///         self.draft.is_active()
///     }
///     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
///         self.editor = cx.children().first().map(|child| child.id().clone());
///         Ok(())
///     }
///     fn keyboard_event(
///         &mut self,
///         cx: &mut EventContext,
///         event: KeyEvent,
///         _: &KeyboardState,
///     ) -> Result<InputOutcome, RuntimeError> {
///         if event.transition != KeyTransition::Pressed {
///             return Ok(InputOutcome::Ignored);
///         }
///         match event.code {
///             KeyCode::Enter if !self.draft.is_active() => {
///                 let text = self.value.get()?;
///                 self.draft.begin(cx, &text, None, DraftSelection::All)?;
///                 if let Some(editor) = &self.editor {
///                     cx.request_focus_for(editor)?;
///                 }
///             }
///             KeyCode::Enter => {
///                 self.value.set(self.draft.text())?;
///                 self.draft.end();
///                 cx.request_focus();
///             }
///             KeyCode::Escape if self.draft.is_active() => {
///                 self.draft.end();
///                 cx.request_focus();
///             }
///             _ => return Ok(InputOutcome::Ignored),
///         }
///         cx.invalidate(Invalidation::Layout);
///         Ok(InputOutcome::Handled)
///     }
/// }
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 100.0, 1.0)?)?;
/// let value = ui.signal("120".to_owned());
/// let draft = DraftEditorController::new();
/// let row = ui.mount_tree(Element::new(
///     "row",
///     Row {
///         value: value.clone(),
///         draft: draft.clone(),
///         editor: None,
///         children: vec![Element::new("editor", DraftEditor::new(draft.clone()))],
///     },
/// ))?;
/// ui.flush(ui.now())?;
/// ui.focus(&row)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// ui.flush(ui.now())?;
/// ui.text_input("128")?; // replaces the selected draft
/// assert_eq!(draft.text(), "128");
/// assert_eq!(value.get()?, "120", "nothing is written while editing");
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.flush(ui.now())?;
/// assert_eq!(value.get()?, "128");
/// assert!(!draft.is_active());
/// # Ok::<(), RuntimeError>(())
/// ```
///
/// Re-describing the editor under the same key applies its style and alignment;
/// a different controller releases the old one and takes the new one.
pub struct DraftEditor<P = ()> {
    shared: Rc<Shared<P>>,
    /// Identifies this mount as the controller's owner.
    token: Rc<()>,
    style: TextStyle,
    align: DraftAlign,
}

impl<P> std::fmt::Debug for DraftEditor<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DraftEditor")
            .field("style", &self.style)
            .field("align", &self.align)
            .finish_non_exhaustive()
    }
}

impl<P: Copy + PartialEq + 'static> DraftEditor<P> {
    /// Creates an editor for `controller`'s draft. Ownership of the controller
    /// is claimed when the editor mounts, not here, so the owner can describe
    /// it again with the same controller.
    #[must_use]
    pub fn new(controller: DraftEditorController<P>) -> Self {
        Self {
            shared: controller.shared,
            token: Rc::new(()),
            style: TextStyle::Body,
            align: DraftAlign::Start,
        }
    }
    /// Sets the typography of the draft. Default [`TextStyle::Body`]; the
    /// built-in inline numeric rows use [`TextStyle::Monospace`].
    #[must_use]
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
    /// Sets where a draft narrower than the editing area sits. Default
    /// [`DraftAlign::Start`].
    #[must_use]
    pub fn align(mut self, align: DraftAlign) -> Self {
        self.align = align;
        self
    }

    fn owns(&self) -> bool {
        self.shared
            .core
            .borrow()
            .owner
            .upgrade()
            .is_some_and(|owner| Rc::ptr_eq(&owner, &self.token))
    }
    /// Takes the controller over unless another live mount owns it, and
    /// applies this editor's presentation. Returns whether this mount owns it.
    fn claim(&self) -> bool {
        let mut core = self.shared.core.borrow_mut();
        match core.owner.upgrade() {
            Some(owner) if !Rc::ptr_eq(&owner, &self.token) => false,
            _ => {
                core.owner = Rc::downgrade(&self.token);
                core.editor.style = self.style;
                core.editor.align_end = self.align == DraftAlign::End;
                true
            }
        }
    }
    /// Ends the session and gives the controller up, if this mount owns it.
    fn release(&self) {
        if !self.owns() {
            return;
        }
        let mut core = self.shared.core.borrow_mut();
        self.shared.end_in(&mut core);
        core.owner = Weak::new();
        core.node = Weak::new();
    }
    /// Whether this editor owns an active session and may take input.
    fn live(&self) -> bool {
        self.owns() && self.shared.core.borrow().editor.active
    }
    /// Keys the editor leaves to its owner.
    fn passes(editor: &Editor<Option<P>>, code: KeyCode) -> bool {
        if editor.preedit.is_some() {
            return false;
        }
        match code {
            KeyCode::Enter
            | KeyCode::Tab
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::PageUp
            | KeyCode::PageDown => true,
            KeyCode::Escape => !editor.dragging,
            _ => false,
        }
    }
    /// Runs an input operation on the editor, publishes the result, applies an
    /// `end` requested meanwhile, and reports a changed draft to the owner's
    /// callback with no borrow held while it runs.
    fn edit<R>(
        &mut self,
        cx: &mut EventContext,
        operation: impl FnOnce(&mut Editor<Option<P>>, &mut EventContext) -> Result<R, RuntimeError>,
    ) -> Result<R, RuntimeError> {
        let mut core = self.shared.core.borrow_mut();
        let before = (core.editor.text.clone(), core.editor.payload);
        let result = operation(&mut core.editor, cx);
        self.shared.publish(&core);
        if self.shared.end_requested.get() {
            self.shared.settle(&mut core);
            return result;
        }
        let result = result?;
        let changed = before.0 != core.editor.text || before.1 != core.editor.payload;
        if !changed || core.editor.read_only {
            return Ok(result);
        }
        core.invalidate(Invalidation::Paint, Some(Invalidation::Paint));
        let (text, payload) = (core.editor.text.clone(), core.editor.payload);
        drop(core);
        let Some(mut callback) = self.shared.on_edit.borrow_mut().take() else {
            return Ok(result);
        };
        let outcome = callback(cx, &text, payload);
        let mut slot = self.shared.on_edit.borrow_mut();
        if slot.is_none() {
            *slot = Some(callback);
        }
        outcome.map(|()| result)
    }
}

impl<P: Copy + PartialEq + 'static> Component for DraftEditor<P> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if !Rc::ptr_eq(&self.shared, &next.shared) {
            self.release();
            self.shared = next.shared;
            self.claim();
            invalidation = Some(Invalidation::Layout);
        }
        if self.style != next.style || self.align != next.align {
            self.style = next.style;
            self.align = next.align;
            invalidation = Some(Invalidation::Layout);
        }
        if self.owns() {
            self.claim();
        }
        invalidation
    }
    fn mounted(&mut self) {
        self.claim();
    }
    fn unmounted(&mut self) {
        self.release();
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Claimed again on every update, so a mount that found the controller
        // taken takes it over once the earlier owner has unmounted.
        if !self.claim() {
            return Err(RuntimeError::DraftEditorBusy);
        }
        let mut core = self.shared.core.borrow_mut();
        core.node = Rc::downgrade(&cx.id().0);
        self.shared.settle(&mut core);
        core.editor.update(cx)
    }
    fn focusable(&self) -> bool {
        self.live()
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        if self.live() {
            self.shared.core.borrow().editor.cursor(point)
        } else {
            None
        }
    }
    fn text_input_state(&self) -> Option<TextInputState> {
        if !self.owns() {
            return None;
        }
        self.shared.core.borrow().editor.input_state()
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let line = cx.text("Mg", None, self.style)?.size().height();
        let width = cx.metric(MetricRole::ControlWidth);
        Ok(Size::new(
            available.width().map_or(width, |w| w.min(width)),
            line,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !self.owns() {
            return Ok(());
        }
        let measure = cx.measurement();
        let area = Rect::from_size(size);
        let core = self.shared.core.borrow();
        core.editor.area.set(Some(area));
        core.editor.arrange(measure, size)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if !self.live() {
            return Ok(());
        }
        self.shared.core.borrow().editor.paint(p)
    }
    fn text_key_event(
        &mut self,
        cx: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        _: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        if !self.live() {
            return Ok(TextKeyOutcome::NotAnEditor);
        }
        let event = input.editing_event(transition);
        if Self::passes(&self.shared.core.borrow().editor, event.code) {
            return Ok(TextKeyOutcome::Ignored);
        }
        if self.edit(cx, |editor, cx| editor.key(cx, event))? == InputOutcome::Handled {
            return Ok(TextKeyOutcome::Handled);
        }
        Ok(unhandled_text_key(cx, input, transition))
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !self.live() || Self::passes(&self.shared.core.borrow().editor, control) {
            return Ok(InputOutcome::Ignored);
        }
        self.edit(cx, |editor, cx| {
            editor.key(
                cx,
                KeyEvent {
                    code: control,
                    ..event
                },
            )
        })
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        if !self.live() {
            return Ok(TextCommandOutcome::NotAnEditor);
        }
        self.edit(cx, |editor, cx| editor.command(cx, command))
    }
    fn text_input(&mut self, cx: &mut EventContext, text: &str) -> Result<(), RuntimeError> {
        if !self.live() {
            return Ok(());
        }
        self.edit(cx, |editor, cx| editor.insert(cx, text))
            .map(|_| ())
    }
    fn text_preedit(
        &mut self,
        cx: &mut EventContext,
        text: &TextPreedit,
    ) -> Result<(), RuntimeError> {
        if !self.live() {
            return Ok(());
        }
        self.shared.core.borrow_mut().editor.compose(cx, text)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.live() {
            return Ok(InputOutcome::Ignored);
        }
        self.shared.core.borrow_mut().editor.pointer(cx, e)
    }
    fn focus_lost(&mut self, _cx: &mut EventContext) -> Result<(), RuntimeError> {
        // The session stays: the owner decides in `focus_within_changed`.
        if self.owns() {
            self.shared.core.borrow_mut().editor.reset();
        }
        Ok(())
    }
    fn input_reset(&mut self) {
        if self.owns() {
            self.shared.core.borrow_mut().editor.reset();
        }
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        let busy = !self.owns() && self.shared.core.borrow().owner.upgrade().is_some();
        d.properties.insert("busy".into(), InspectValue::Bool(busy));
        if busy {
            return d;
        }
        let core = self.shared.core.borrow();
        let editor = &core.editor;
        let selection = editor.selection.get();
        for (name, value) in [
            ("active", editor.active),
            ("read_only", editor.read_only),
            ("composing", editor.preedit.is_some()),
            ("payload_present", editor.payload.is_some()),
        ] {
            d.properties.insert(name.into(), InspectValue::Bool(value));
        }
        d.properties
            .insert("text".into(), InspectValue::Text(editor.text.clone()));
        d.properties.insert(
            "selection".into(),
            InspectValue::Text(format!(
                "{}..{}",
                selection.anchor_byte(),
                selection.focus_byte()
            )),
        );
        d.properties.insert(
            "typography".into(),
            InspectValue::Text(format!("{:?}", editor.style)),
        );
        d.validation.interactive = editor.active;
        if editor.active {
            d.semantics = Some(
                Semantics::new(SemanticRole::TextInput)
                    .value(SemanticValue::Text(editor.text.clone()))
                    .read_only(editor.read_only)
                    .enabled_actions((!editor.read_only).then_some(SemanticAction::SetValue)),
            );
        }
        d
    }
}

/// The key under which [`DraftSlot`] mounts its editor.
pub(crate) const DRAFT_KEY: &str = "editor";

/// How a built-in inline control holds its [`DraftEditor`]: through the same
/// public contract a consumer uses. The editor is mounted with
/// [`UpdateContext::request_children`], so the owner keeps its controller, and
/// with it an open session, when an application describes the owner again.
pub(crate) struct DraftSlot<P> {
    pub(crate) draft: DraftEditorController<P>,
    id: Option<WidgetId>,
    style: TextStyle,
    align: DraftAlign,
    mounted: Option<(TextStyle, DraftAlign)>,
}

impl<P: Copy + PartialEq + 'static> DraftSlot<P> {
    pub(crate) fn new(style: TextStyle, align: DraftAlign) -> Self {
        Self {
            draft: DraftEditorController::new(),
            id: None,
            style,
            align,
            mounted: None,
        }
    }
    pub(crate) fn set_style(&mut self, style: TextStyle) {
        self.style = style;
    }
    pub(crate) fn style(&self) -> TextStyle {
        self.style
    }
    pub(crate) fn is_active(&self) -> bool {
        self.draft.is_active()
    }
    /// Mounts the editor, or describes it again when its presentation
    /// changed, and learns its identity.
    pub(crate) fn update(&mut self, cx: &mut UpdateContext) {
        if self.mounted != Some((self.style, self.align)) {
            cx.request_children(vec![Element::new(
                DRAFT_KEY,
                DraftEditor::new(self.draft.clone())
                    .style(self.style)
                    .align(self.align),
            )]);
            self.mounted = Some((self.style, self.align));
        }
        self.id = cx
            .children()
            .iter()
            .find(|child| child.key() == DRAFT_KEY)
            .map(|child| child.id().clone());
    }
    /// Starts a session and moves focus into the editor.
    pub(crate) fn begin(
        &self,
        cx: &mut EventContext,
        text: &str,
        payload: Option<P>,
        read_only: bool,
    ) -> Result<(), RuntimeError> {
        if read_only {
            self.draft.begin_read_only(cx, text, DraftSelection::All)?;
        } else {
            self.draft.begin(cx, text, payload, DraftSelection::All)?;
        }
        if let Some(id) = &self.id {
            cx.request_focus_for(id)?;
        }
        cx.invalidate(Invalidation::Layout);
        Ok(())
    }
    /// Ends the session; `refocus` returns focus to the owner, as Enter and
    /// Escape do, but not a departure of focus.
    pub(crate) fn end(&self, cx: Option<&mut EventContext>) {
        self.draft.end();
        if let Some(cx) = cx {
            cx.request_focus();
            cx.invalidate(Invalidation::Layout);
        }
    }
    pub(crate) fn child_visible(&self, key: &str) -> bool {
        key != DRAFT_KEY || self.is_active()
    }
    /// Places the editor, the owner's only child, in `area`.
    pub(crate) fn arrange(
        &self,
        cx: &mut ArrangeContext<'_>,
        area: Rect,
    ) -> Result<(), RuntimeError> {
        for index in 0..cx.len() {
            cx.place(index, area)?;
        }
        Ok(())
    }
}
