//! The protocol through which a host, such as a table, runs one editor at a
//! time and learns how each edit ended.
use crate::Notifier;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

/// How an editor starts an edit, applied once the editor has seen the edit
/// begin. See [`EditorSession::begin`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditorStart {
    /// The value is selected, so typing replaces it.
    SelectAll,
    /// The caret is after the value, nothing selected.
    CaretAtEnd,
    /// The value is replaced by this committed text with the caret after it,
    /// as when the user started the edit by typing. A single-line editor
    /// refuses text with a line break with its ordinary text error and starts
    /// as [`Self::SelectAll`] does.
    Typed(String),
}

/// What ended or tried to end an edit. See [`EditorCompletion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditorTrigger {
    /// Enter confirmed the edit.
    Enter,
    /// Tab or Shift+Tab confirmed the edit; a host moves to the next or
    /// previous editable place.
    Tab {
        /// [`FocusDirection::Forward`](crate::FocusDirection::Forward) for Tab,
        /// [`FocusDirection::Backward`](crate::FocusDirection::Backward) for
        /// Shift+Tab.
        direction: crate::FocusDirection,
    },
    /// Escape cancelled the edit.
    Escape,
    /// A choice was made in a choice editor's list, by key or click.
    Choice,
    /// Focus left the editor by other means: a press elsewhere, the window
    /// losing focus. The host should not take focus back.
    FocusLeft,
}

/// The result of an edit's completion. See [`EditorCompletion`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditorOutcome {
    /// The editor wrote a changed value to its binding.
    Saved,
    /// There was nothing to write: the draft equaled the value, normalized to
    /// it, or an application write had replaced the draft.
    Unchanged,
    /// The editor refused the draft, kept it for correction and wrote
    /// nothing. The edit stays active; a later completion may follow.
    Refused,
    /// The edit was cancelled and the draft discarded.
    Cancelled,
}
impl EditorOutcome {
    /// Whether this outcome ends the edit: every outcome but
    /// [`Self::Refused`].
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self != Self::Refused
    }
}

/// How one edit ended, or tried to end, as its editor reported it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct EditorCompletion {
    /// What ended or tried to end the edit.
    pub trigger: EditorTrigger,
    /// The result.
    pub outcome: EditorOutcome,
}

struct SessionInner {
    notifier: Notifier,
    /// The current edit's number; 0 before the first `begin`.
    edit: Cell<u64>,
    /// Whether the current edit is active.
    active: Cell<bool>,
    start: RefCell<Option<EditorStart>>,
    completions: RefCell<VecDeque<EditorCompletion>>,
}

/// One edit at a time, shared by a host and the editor it mounts, so the host
/// learns synchronously how the user ended it.
///
/// A component cannot call another component while it handles an event, so a
/// host cannot ask an arbitrary editor to save and tell it the result. With a
/// session the editor does the asking itself: it keeps Enter, Tab and Escape,
/// performs its own save, normalization and refusal, and reports an
/// [`EditorCompletion`] here, which requests the host's
/// [`Component::deliver_notifications`](crate::Component::deliver_notifications)
/// through the host's [`Notifier`]. The host reads it there with
/// [`Self::take_completion`] and ends the edit, moves to the next place on
/// Tab, or keeps the editor open after [`EditorOutcome::Refused`].
///
/// The host calls [`Self::begin`] before it builds the editor, which binds the
/// editor to that edit: [`TextField::session`](crate::TextField::session),
/// [`IntegerInput::session`](crate::IntegerInput::session),
/// [`DecimalInput::session`](crate::DecimalInput::session) and
/// [`Select::session`](crate::Select::session) take an [`EditorAttachment`]
/// with [`Self::attach`], and a custom editor does the same. Reports from an
/// attachment of an earlier edit, or after a terminal report or
/// [`Self::end`], are dropped, so an editor that loses focus because the host
/// moved on cannot end the next edit. Completions queue in order until taken.
///
/// The report is recorded before the editor runs a fallible application
/// callback such as `on_save`. When that callback fails, input dispatch
/// returns its error and delivers no notification, so the host receives the
/// completion at the next flush; nothing is replayed.
///
/// UI-thread only; clones share the session.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{EditorOutcome, EditorSession, EditorStart, EditorTrigger};
/// # use pert_ui::*;
/// # use std::{cell::RefCell, rc::Rc, time::Duration};
/// # struct Host(Option<EditorSession>, Rc<RefCell<Vec<EditorTrigger>>>);
/// # impl Component for Host {
/// #     fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
/// #         if self.0.is_none() {
/// #             let session = EditorSession::new(cx.notifier());
/// #             session.begin(EditorStart::SelectAll);
/// #             let attachment = session.attach();
/// #             attachment.report(EditorTrigger::Enter, EditorOutcome::Saved);
/// #             self.0 = Some(session);
/// #         }
/// #         Ok(())
/// #     }
/// #     fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
/// #         while let Some(done) = self.0.as_ref().and_then(EditorSession::take_completion) {
/// #             self.1.borrow_mut().push(done.trigger);
/// #         }
/// #         Ok(())
/// #     }
/// # }
/// # let mut ui = Runtime::new();
/// # let seen = Rc::new(RefCell::new(Vec::new()));
/// # ui.mount_tree(Element::new("host", Host(None, seen.clone())))?;
/// # ui.flush(Duration::ZERO)?;
/// # assert_eq!(*seen.borrow(), [EditorTrigger::Enter]);
/// # Ok::<(), RuntimeError>(())
/// ```
#[derive(Clone)]
pub struct EditorSession(Rc<SessionInner>);
impl EditorSession {
    /// A session reporting to the host that owns `notifier`, obtained from
    /// [`UpdateContext::notifier`](crate::UpdateContext::notifier) or
    /// [`EventContext::notifier`](crate::EventContext::notifier). No edit is
    /// active until [`Self::begin`].
    #[must_use]
    pub fn new(notifier: Notifier) -> Self {
        Self(Rc::new(SessionInner {
            notifier,
            edit: Cell::new(0),
            active: Cell::new(false),
            start: RefCell::new(None),
            completions: RefCell::new(VecDeque::new()),
        }))
    }
    /// Begins a new edit that starts as `start`, ending any active one: reports
    /// from editors attached to an earlier edit are dropped from now on.
    /// Completions of earlier edits not yet taken are discarded too. Call it
    /// before building the editor, which attaches to the edit active at that
    /// moment.
    pub fn begin(&self, start: EditorStart) {
        self.0.edit.set(self.0.edit.get() + 1);
        self.0.active.set(true);
        *self.0.start.borrow_mut() = Some(start);
        self.0.completions.borrow_mut().clear();
    }
    /// Ends the active edit without a completion, for example when the host
    /// itself closes the editor; later reports for it are dropped.
    pub fn end(&self) {
        self.0.active.set(false);
        self.0.start.borrow_mut().take();
    }
    /// Whether an edit is active: begun and not yet ended by a terminal report
    /// or [`Self::end`].
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.0.active.get()
    }
    /// Binds an editor to the edit active now. Built-in editors call this from
    /// their `session` builder; a custom editor calls it when it is built.
    #[must_use]
    pub fn attach(&self) -> EditorAttachment {
        EditorAttachment {
            session: self.clone(),
            edit: self.0.edit.get(),
        }
    }
    /// The oldest completion reported and not yet taken.
    #[must_use]
    pub fn take_completion(&self) -> Option<EditorCompletion> {
        self.0.completions.borrow_mut().pop_front()
    }
}
impl core::fmt::Debug for EditorSession {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EditorSession")
            .field("edit", &self.0.edit.get())
            .field("active", &self.0.active.get())
            .finish_non_exhaustive()
    }
}

/// An editor's binding to one edit of an [`EditorSession`], from
/// [`EditorSession::attach`]. The editor takes its start from it and reports
/// how the user ended the edit.
#[derive(Clone)]
pub struct EditorAttachment {
    session: EditorSession,
    edit: u64,
}
impl EditorAttachment {
    /// Whether this attachment's edit is still active, so reports are kept.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.session.0.active.get() && self.session.0.edit.get() == self.edit
    }
    /// Takes the edit's pending start, once, while the edit is current.
    #[must_use]
    pub fn take_start(&self) -> Option<EditorStart> {
        if !self.is_current() {
            return None;
        }
        self.session.0.start.borrow_mut().take()
    }
    /// Whether a start is waiting, without taking it.
    #[must_use]
    pub fn has_start(&self) -> bool {
        self.is_current() && self.session.0.start.borrow().is_some()
    }
    /// Reports how the user ended, or tried to end, the edit, and requests the
    /// host's delivery. A terminal outcome ([`EditorOutcome::is_terminal`])
    /// closes the edit. Dropped when the edit is no longer current.
    pub fn report(&self, trigger: EditorTrigger, outcome: EditorOutcome) {
        if !self.is_current() {
            return;
        }
        if outcome.is_terminal() {
            self.session.0.active.set(false);
            self.session.0.start.borrow_mut().take();
        }
        self.session
            .0
            .completions
            .borrow_mut()
            .push_back(EditorCompletion { trigger, outcome });
        self.session.0.notifier.request();
    }
}
impl core::fmt::Debug for EditorAttachment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EditorAttachment")
            .field("edit", &self.edit)
            .field("current", &self.is_current())
            .finish_non_exhaustive()
    }
}
