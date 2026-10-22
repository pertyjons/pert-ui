//! Back, forward and history-list controls over a [`NavigationHistory`].

use std::{cell::RefCell, rc::Rc};

use crate::{
    Button, Component, ComponentDetails, Constraints, Element, EventContext, HistoryEntry,
    HistoryEntryId, Icon, InputOutcome, InspectValue, MeasureContext, MetricRole,
    NavigationHistory, PickChoice, PickItem, PointerButton, PointerEvent, PointerKind, QuickPick,
    QuickPickController, Rect, RuntimeError, SemanticRole, Semantics, Size, Tooltip, UpdateContext,
};

/// The application's restoration of a history entry.
type RestoreFn<T> = Box<dyn FnMut(&mut EventContext, &HistoryEntry<T>) -> Result<(), RuntimeError>>;

/// What the parts' callbacks act on.
///
/// Generated parts always capture the mounted copy. A re-description moves
/// its new restore callback here; the history and picker stay mounted.
struct Shared<T> {
    history: NavigationHistory<T>,
    picker: QuickPickController,
    restore: RefCell<RestoreFn<T>>,
    /// Entry identities of the opened list, by row position.
    opened: RefCell<Vec<HistoryEntryId>>,
}
impl<T: Clone + Eq + 'static> Shared<T> {
    fn restore(&self, cx: &mut EventContext, entry: &HistoryEntry<T>) -> Result<(), RuntimeError> {
        (self.restore.borrow_mut())(cx, entry)
    }
}

/// Back and forward buttons and a searchable history list, bound to a
/// [`NavigationHistory`], as at the end of a browser's toolbar.
///
/// The control moves through the history and reports the entry it moved to;
/// it never restores anything itself. The `restore` callback receives the
/// entry, in an event phase, and the application shows that place, for
/// example with [`Runtime::reveal_child`](crate::Runtime::reveal_child) on
/// its tab strips. That call flushes, so queue it from the callback with
/// [`EventContext::defer_detached`]: choosing from the list closes it, and an
/// operation queued with [`EventContext::defer`] from there can be cancelled. Because the history is already at that entry, the
/// application must not record the restoration as a new visit; recording only
/// [`ChangeOrigin::User`](crate::ChangeOrigin::User) selections from
/// [`Tabs::on_select`](crate::Tabs::on_select) does that, as
/// [`NavigationHistory`] describes.
///
/// Parts, in order:
///
/// - **Back** and **Forward** icon buttons, disabled while there is nowhere to
///   go, with a tooltip naming the entry they go to ("Back to Colors").
/// - **History**, an icon button that opens a nonmodal [`QuickPick`] over the
///   entries, newest first, each labelled with the kind "History" and the
///   current one with the detail "Current". Choosing an entry moves the
///   history there ([`NavigationHistory::go_to`]) and restores it; an entry
///   the history dropped while the list was open is refused with a message.
///
/// Each part is a Tab stop. Whether the buttons and the list are offered at
/// all is the application's choice: both are offered by default,
/// [`Self::buttons`] and [`Self::list`] set a constant and the `*_signal`
/// variants follow a binding the application can change at run time. A part that is not offered is not presented and takes no space; a
/// list hidden while it is open is no longer presented.
///
/// Keyboard shortcuts and the mouse's side buttons are the application's:
/// register commands with [`KeyChord::navigate_back`](crate::KeyChord::navigate_back)
/// and [`KeyChord::navigate_forward`](crate::KeyChord::navigate_forward) that
/// call [`NavigationHistory::back`] and [`NavigationHistory::forward`], and
/// wrap the content in [`HistoryButtons`] for the side buttons.
///
/// The history and the list's picker are fixed when the control mounts: a
/// description with another history keeps acting on the mounted one, so give
/// a control with another history a different key. Every part's enablement,
/// tooltip and action follows that mounted history. The control generates its
/// parts during its first update and retains their identities when descriptions
/// or installed strings change. A new description replaces the restore callback, the offered parts
/// and the name. Inspection reports a `Toolbar` with the control's name and
/// `buttons` and `list` properties.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     Element, HistoryNavigator, NavigationHistory, PointerButton, PointerEvent, PointerKind,
///     Point, Runtime, Viewport,
/// };
/// use std::{cell::RefCell, rc::Rc, time::Duration};
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
/// let history = NavigationHistory::new(&ui)?;
/// history.visit("colors", "Colors")?;
/// history.visit("knobs", "Knobs")?;
/// let shown = Rc::new(RefCell::new(None));
/// let record = shown.clone();
/// let root = ui.mount_tree(Element::new(
///     "history",
///     HistoryNavigator::new(&ui, history.clone(), move |_, entry| {
///         *record.borrow_mut() = Some(*entry.value());
///         Ok(())
///     })?,
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// // Click Back: the history moves and the application shows the place.
/// let back = ui.find_descendant(&root, "back")?.ok_or("no back button")?;
/// let [x, y, w, h] = ui.bounds(&back)?.ok_or("not laid out")?.values();
/// let center = Point::new(x + w / 2.0, y + h / 2.0)?;
/// for kind in [PointerKind::Press(PointerButton::Primary), PointerKind::Release(PointerButton::Primary)] {
///     ui.pointer_input(PointerEvent::new(center, kind))?;
/// }
/// assert_eq!(*shown.borrow(), Some("colors"));
/// assert_eq!(history.current().get()?.map(|e| *e.value()), Some("colors"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct HistoryNavigator<T> {
    /// Initial parts, delivered by the first update rather than the descriptor.
    children: Vec<Element>,
    shared: Rc<Shared<T>>,
    buttons: crate::Permission,
    list: crate::Permission,
    /// The application's name, by default the string table's
    /// [`StringRole::HistoryName`](crate::StringRole::HistoryName).
    name: crate::Caption,
    shown_buttons: bool,
    shown_list: bool,
    /// The string table the parts were last described with.
    strings: crate::UiStrings,
}

impl<T: Clone + Eq + 'static> HistoryNavigator<T> {
    /// Creates the control over `history`; `restore` shows the entry the user
    /// moved to. `ui` owns the history list's picker state: a
    /// [`Runtime`](crate::Runtime), or
    /// an update or event context when a component builds the control from
    /// its own update ([`StateOwner`](crate::StateOwner)).
    ///
    /// # Errors
    /// Returns the runtime's errors creating the bindings the parts derive
    /// from the history, such as [`RuntimeError::Closed`] after shutdown.
    pub fn new(
        ui: &impl crate::StateOwner,
        history: NavigationHistory<T>,
        restore: impl FnMut(&mut EventContext, &HistoryEntry<T>) -> Result<(), RuntimeError> + 'static,
    ) -> Result<Self, RuntimeError> {
        let shared = Rc::new(Shared {
            history: history.clone(),
            picker: QuickPickController::new(ui),
            restore: RefCell::new(Box::new(restore)),
            opened: RefCell::new(Vec::new()),
        });
        // Described with the English table; the first update describes the
        // parts again when another table is installed, so construction does
        // not need the runtime's settings and works from any `StateOwner`.
        let strings = crate::UiStrings::english();
        let children = parts(&history, &shared, &strings)?;
        Ok(Self {
            children,
            shared,
            buttons: crate::Permission::allowed(),
            list: crate::Permission::allowed(),
            name: crate::Caption::new(crate::StringRole::HistoryName),
            shown_buttons: true,
            shown_list: true,
            strings,
        })
    }
    /// Restricts whether the back and forward buttons are offered; offered
    /// by default. A constant or a bound signal is a
    /// [`Permission`](crate::Permission), and calls combine as permissions do
    /// ([`Permission::and`](crate::Permission::and)): the buttons are offered
    /// only while every constant and binding given allows them, so a fixed
    /// `false` is never undone by a later binding.
    pub fn buttons(mut self, offered: impl Into<crate::Permission>) -> Self {
        self.buttons = self.buttons.and(offered);
        self
    }
    /// Restricts whether the history list is offered; offered by default.
    /// Combines as [`Self::buttons`] does.
    pub fn list(mut self, offered: impl Into<crate::Permission>) -> Self {
        self.list = self.list.and(offered);
        self
    }
    /// Sets the name inspection and assistive technology report for the
    /// control. By default the installed [`UiStrings`](crate::UiStrings)
    /// supply [`StringRole::HistoryName`](crate::StringRole::HistoryName),
    /// "Navigation history" in English; the parts' names, tooltips and the
    /// list's texts are the `History…` roles and [`StringRole::BackTo`](crate::StringRole::BackTo)
    /// and [`StringRole::ForwardTo`](crate::StringRole::ForwardTo).
    ///
    /// The name is a [`TextSource`](crate::TextSource): fixed text, a binding
    /// the name follows, or another role of the installed table. Only its
    /// plain text is used. Redescription with the same binding or role keeps
    /// the name already read.
    pub fn name(mut self, name: impl Into<crate::TextSource>) -> Self {
        self.name.set_source(name.into());
        self
    }
}

/// The back and forward buttons and the history list, named from `strings`.
fn parts<T: Clone + Eq + 'static>(
    history: &NavigationHistory<T>,
    shared: &Rc<Shared<T>>,
    strings: &crate::UiStrings,
) -> Result<Vec<Element>, RuntimeError> {
    Ok(vec![
        step_button(history, shared, true, strings)?,
        step_button(history, shared, false, strings)?,
        history_list(history, shared, strings)?,
    ])
}

/// The back or forward button with a tooltip naming its target.
fn step_button<T: Clone + Eq + 'static>(
    history: &NavigationHistory<T>,
    shared: &Rc<Shared<T>>,
    back: bool,
    strings: &crate::UiStrings,
) -> Result<Element, RuntimeError> {
    let (key, icon, role) = if back {
        ("back", Icon::Back, crate::StringRole::HistoryBack)
    } else {
        ("forward", Icon::Forward, crate::StringRole::HistoryForward)
    };
    let name = strings.text(role).to_owned();
    let acting = shared.clone();
    let button = Button::icon(icon, name.clone(), move |cx| {
        let shared = &acting;
        let entry = if back {
            shared.history.back()?
        } else {
            shared.history.forward()?
        };
        match entry {
            Some(entry) => shared.restore(cx, &entry),
            None => Ok(()),
        }
    })
    .frameless()
    .enabled(if back {
        history.can_go_back()
    } else {
        history.can_go_forward()
    });
    let table = strings.clone();
    let tip = history.state().map(move |state| {
        let target = if back { state.previous() } else { state.next() };
        target.map_or_else(
            || name.clone(),
            |entry| {
                let place = entry.label();
                table.format(if back {
                    crate::UiMessage::BackTo { place }
                } else {
                    crate::UiMessage::ForwardTo { place }
                })
            },
        )
    })?;
    Ok(Element::new(
        format!("{key}-tip"),
        Tooltip::new(Element::new(key, button), tip),
    ))
}

/// The history button and the picker it opens.
fn history_list<T: Clone + Eq + 'static>(
    history: &NavigationHistory<T>,
    shared: &Rc<Shared<T>>,
    strings: &crate::UiStrings,
) -> Result<Element, RuntimeError> {
    let opening = shared.clone();
    let (kind, current_text) = (
        strings.text(crate::StringRole::HistoryKind).to_owned(),
        strings.text(crate::StringRole::HistoryCurrent).to_owned(),
    );
    let title = strings.text(crate::StringRole::HistoryList).to_owned();
    let button = Button::icon(Icon::History, title.clone(), move |_| {
        let opening = &opening;
        let state = opening.history.state().get()?;
        let current = state.current().map(HistoryEntry::id);
        // Rows are keyed by their position in the opened list, and `opened`
        // maps them back to entry identities, which never retarget another
        // entry if the history drops one while the list is open.
        let mut ids = Vec::with_capacity(state.entries().len());
        let items = state
            .entries()
            .iter()
            .rev()
            .enumerate()
            .map(|(index, entry)| {
                ids.push(entry.id());
                let item =
                    PickItem::new(format!("entry-{index}"), entry.label()).kind(kind.clone());
                if Some(entry.id()) == current {
                    item.detail(current_text.clone())
                } else {
                    item
                }
            })
            .collect();
        *opening.opened.borrow_mut() = ids;
        opening.picker.open(items)
    })
    .frameless()
    .enabled(history.state().map(|state| !state.entries().is_empty())?);
    let choosing = shared.clone();
    let gone = strings.text(crate::StringRole::HistoryEntryGone).to_owned();
    let pick = QuickPick::new(
        shared.picker.clone(),
        Element::new(
            "history-tip",
            Tooltip::new(Element::new("history", button), title.clone()),
        ),
        move |cx, item| {
            let choosing = &choosing;
            let id = item
                .key
                .strip_prefix("entry-")
                .and_then(|index| index.parse::<usize>().ok())
                .and_then(|index| choosing.opened.borrow().get(index).copied());
            let entry = match id {
                Some(id) => choosing.history.go_to(id)?,
                None => None,
            };
            match entry {
                Some(entry) => {
                    choosing.restore(cx, &entry)?;
                    Ok(PickChoice::Done)
                }
                None => Ok(PickChoice::Refused(gone.clone())),
            }
        },
    )
    .nonmodal()
    .name(title)
    .list_name(crate::StringRole::HistoryListName)
    .placeholder(crate::StringRole::HistorySearch);
    Ok(Element::new("list", pick))
}

impl<T: Clone + Eq + 'static> Component for HistoryNavigator<T> {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        // The mounted parts share this callback slot, so replace only the
        // callback and keep their history, picker and opened list in use.
        let callback = std::mem::replace(
            &mut *next.shared.restore.borrow_mut(),
            Box::new(|_, _| Ok(())),
        );
        *self.shared.restore.borrow_mut() = callback;
        let changed = !self.buttons.same_as(&next.buttons) || !self.list.same_as(&next.list);
        let renamed = self.name.reconcile(next.name);
        self.buttons = next.buttons;
        self.list = next.list;
        let changed = changed || renamed;
        // Incoming parts are never mounted: the existing parts already follow
        // the retained history and shared callback. Only changed offers/name
        // need another update; string replacement is tracked independently.
        changed.then_some(crate::Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let strings = cx.strings()?;
        if !self.children.is_empty() || strings != self.strings {
            let shared = &self.shared;
            let children = if strings == self.strings && !self.children.is_empty() {
                std::mem::take(&mut self.children)
            } else {
                parts(&shared.history, shared, &strings)?
            };
            cx.request_children(children);
            self.children.clear();
            self.strings = strings;
            cx.request_layout();
        }
        // The name is not drawn; a change only repaints.
        self.name.update_paint(cx)?;
        let buttons = self.buttons.read(cx)?;
        let list = self.list.read(cx)?;
        if buttons != self.shown_buttons || list != self.shown_list {
            self.shown_buttons = buttons;
            self.shown_list = list;
            cx.request_layout();
        }
        Ok(())
    }
    fn clips_children(&self) -> bool {
        // The history list's popup extends beyond the control.
        false
    }
    fn child_visible(&self, key: &str) -> bool {
        match key {
            "back-tip" | "forward-tip" => self.shown_buttons,
            "list" => self.shown_list,
            _ => true,
        }
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let gap = cx.metric(MetricRole::IconGap);
        let child = Constraints::new(None, available.height())?;
        let (mut width, mut height) = (0.0f32, 0.0f32);
        for index in 0..cx.len() {
            let size = cx.measure(index, child)?;
            width += size.width();
            height = height.max(size.height());
        }
        width += gap * cx.len().saturating_sub(1) as f32;
        Ok(Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut crate::ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let gap = cx.measurement().metric(MetricRole::IconGap);
        let child = Constraints::new(None, Some(size.height()))?;
        let mut x = 0.0;
        for index in 0..cx.len() {
            let measured = cx.measure(index, child)?;
            let y = (size.height() - measured.height()) / 2.0;
            cx.place(
                index,
                Rect::new(x, y.max(0.0), measured.width(), measured.height())?,
            )?;
            x += measured.width() + gap;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("buttons".into(), InspectValue::Bool(self.shown_buttons));
        details
            .properties
            .insert("list".into(), InspectValue::Bool(self.shown_list));
        details.semantics = Some(Semantics::new(SemanticRole::Toolbar).name(self.name.text()));
        details
    }
}

/// Goes back and forward in a [`NavigationHistory`] with the mouse's side
/// buttons ([`PointerButton::Back`] and [`PointerButton::Forward`]) pressed
/// anywhere over its content, as browsers do.
///
/// Wrap the content area the side buttons should work over, usually the
/// whole window body. A side-button press that no descendant handled moves
/// the history and calls `restore` with the entry, exactly as
/// [`HistoryNavigator`]'s buttons do; the control captures the pointer for
/// that press where the capture is free, and consumes the matching release
/// wherever the pointer ends. A release whose press it did not handle passes
/// through. A second side-button press while the first is held does not move
/// history or replace the first press; its release passes through. Permission
/// revocation, focus loss, hiding and removal forget a pending press. Other
/// pointer input passes through. The reaction is a permission: on by default,
/// [`Self::enabled`] adds a constant or a binding, and it is on only while all
/// of them allow it. The control adds no Tab stop and no presentation of its
/// own. A new description replaces the callback, the permission and the
/// history.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     Element, HistoryButtons, Label, NavigationHistory, Point, PointerButton, PointerEvent,
///     PointerKind, Runtime, Viewport,
/// };
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
/// let history = NavigationHistory::new(&ui)?;
/// history.visit(1, "One")?;
/// history.visit(2, "Two")?;
/// ui.mount_tree(Element::new(
///     "body",
///     HistoryButtons::new(history.clone(), Element::new("text", Label::new("Content")), |_, _| Ok(())),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// let point = Point::new(10.0, 10.0)?;
/// ui.pointer_input(PointerEvent::new(point, PointerKind::Press(PointerButton::Back)))?;
/// ui.pointer_input(PointerEvent::new(point, PointerKind::Release(PointerButton::Back)))?;
/// assert_eq!(history.current().get()?.map(|entry| *entry.value()), Some(1));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the buttons under the same key applies the restore callback,
/// the enabled permission and the history: the side buttons move the history
/// given last. A press held while another history arrives is forgotten, so its
/// release passes through, and the next press moves the new history. Unlike
/// [`HistoryNavigator`], this control generates no parts from its history, so
/// nothing retains the first one.
#[must_use]
pub struct HistoryButtons<T> {
    children: Vec<Element>,
    history: NavigationHistory<T>,
    restore: RestoreFn<T>,
    enabled: crate::Permission,
    active: bool,
    /// The side button whose press this control handled, until its release
    /// or a loss of input context.
    armed: Option<PointerButton>,
    /// Whether a new description replaced the history since the last update.
    handoff: bool,
}

impl<T: Clone + Eq + 'static> HistoryButtons<T> {
    /// Wraps `content`; side-button presses over it move `history` and call
    /// `restore` with the entry.
    pub fn new(
        history: NavigationHistory<T>,
        content: Element,
        restore: impl FnMut(&mut EventContext, &HistoryEntry<T>) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self {
            children: vec![content],
            history,
            restore: Box::new(restore),
            enabled: crate::Permission::allowed(),
            active: true,
            armed: None,
            handoff: false,
        }
    }
    /// Restricts the reaction to the side buttons; on by default. A constant
    /// or a bound signal is a [`Permission`](crate::Permission), and calls
    /// combine restrictively ([`Permission::and`](crate::Permission::and)).
    pub fn enabled(mut self, enabled: impl Into<crate::Permission>) -> Self {
        self.enabled = self.enabled.and(enabled);
        self
    }
}

impl<T: Clone + Eq + 'static> Component for HistoryButtons<T> {
    fn reconcile(&mut self, next: Self) -> Option<crate::Invalidation> {
        self.restore = next.restore;
        self.enabled = next.enabled;
        if !self.history.same_as(&next.history) {
            // The held press moved the old history; its release must not be
            // consumed on behalf of the new one. The update cancels the input.
            self.history = next.history;
            self.handoff = true;
        }
        // Re-read the permission at the next update.
        Some(crate::Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.active = self.enabled.read(cx)?;
        let handoff = std::mem::take(&mut self.handoff);
        if (!self.active || handoff) && self.armed.take().is_some() {
            cx.cancel_input();
        }
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.active {
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Press(button @ (PointerButton::Back | PointerButton::Forward)) => {
                if self.armed.is_some() {
                    return Ok(InputOutcome::Handled);
                }
                // The press is this control's: its release is consumed wherever
                // the pointer ends up, through the capture where it is free.
                self.armed = Some(button);
                let _ = cx.capture_pointer();
                let entry = if button == PointerButton::Back {
                    self.history.back()?
                } else {
                    self.history.forward()?
                };
                if let Some(entry) = entry {
                    (self.restore)(cx, &entry)?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(button @ (PointerButton::Back | PointerButton::Forward))
                if self.armed == Some(button) =>
            {
                self.armed = None;
                cx.release_pointer();
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn input_reset(&mut self) {
        self.armed = None;
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("enabled".into(), InspectValue::Bool(self.active));
        details
    }
}
