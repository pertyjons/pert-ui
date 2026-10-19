//! An application menu bar: a row of titles that open menus, with desktop
//! menu-bar pointer and keyboard behavior.
use crate::choice_controls::{focus_ring, rounded};
use crate::layout::{centered, rect};
use crate::menu::{LeafAction, MenuChain, MenuItem, menu_panel, validate_menu};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The command context a bar captured when it was engaged.
type Capture = Rc<RefCell<Option<CommandContext>>>;

/// One title of a bar in the shared state, in display order.
struct TitleState {
    /// The title's item key, stable across descriptions.
    key: String,
    /// Whether the title opens a menu; otherwise it runs an action.
    menu: bool,
    /// Whether the title is currently enabled, maintained by its updates.
    available: bool,
    /// Whether the mounted title is currently presented, maintained by the bar.
    visible: bool,
}

/// State a bar shares with its titles and with the menus they open.
///
/// One core serves a mounted bar for its whole lifetime. A new description
/// builds its own core, which reconciliation folds into the mounted one; see
/// [`BarState`].
struct BarCore {
    /// Key of the title whose menu is open. Keyed rather than positional so an
    /// open menu survives titles being added, removed or reordered.
    open: Signal<Option<String>>,
    /// An open key whose title a description removed. Writes are not permitted
    /// during reconciliation, so the key stays in `open` but counts as closed
    /// until the next write, and a title re-added under it does not reopen.
    closed: RefCell<Option<String>>,
    /// Whether the titles take keyboard focus: from F10 until focus leaves them.
    keyboard: Cell<bool>,
    /// Set by F10 until focus reaches a title, so the pending focus move is not
    /// mistaken for focus leaving the bar.
    arming: Cell<bool>,
    /// The titles in display order.
    titles: RefCell<Vec<TitleState>>,
    /// Command context captured when the bar was engaged.
    capture: Capture,
    /// Capture cells of menus built by later descriptions. Their panels keep
    /// the cell they were built with, so an engagement writes every live one.
    adopted: RefCell<Vec<Weak<RefCell<Option<CommandContext>>>>>,
}
impl BarCore {
    /// Records the command context when the bar goes from idle to engaged, before
    /// anything in it takes focus. An engaged bar keeps its first capture.
    fn engage(&self, cx: &EventContext) -> Result<(), RuntimeError> {
        if !self.keyboard.get() && self.open_index()?.is_none() {
            let context = cx.command_context(CommandSource::Menu);
            for capture in self.adopted.borrow().iter().filter_map(Weak::upgrade) {
                *capture.borrow_mut() = Some(context.clone());
            }
            *self.capture.borrow_mut() = Some(context);
        }
        Ok(())
    }
    /// Whether the bar has a menu open or keyboard-focusable titles.
    fn engaged(&self) -> Result<bool, RuntimeError> {
        Ok(self.keyboard.get() || self.open_index()?.is_some())
    }
    /// Ends menu mode: closes any menu and returns focus to where it was when the
    /// bar was engaged. Without such a place the titles keep focus.
    fn leave(&self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let previous = self
            .capture
            .borrow()
            .as_ref()
            .and_then(|context| context.focused.clone());
        if let Some(previous) = previous {
            // Titles stop being focusable first, so closing a keyboard-opened menu
            // does not return focus to its title and cancel this request.
            self.keyboard.set(false);
            self.arming.set(false);
            cx.request_focus_for(&previous)?;
        }
        self.set_open(None)
    }
    /// Whether `open`, a value of the open-state signal, shows the menu of the
    /// title keyed `key`.
    fn shows(&self, open: Option<&str>, key: &str) -> bool {
        open == Some(key) && self.closed.borrow().as_deref() != Some(key)
    }
    /// Position of the title whose menu is open, if that title is present.
    fn open_index(&self) -> Result<Option<usize>, RuntimeError> {
        let open = self.open.get()?;
        Ok(open
            .filter(|key| self.shows(Some(key), key))
            .and_then(|key| self.titles.borrow().iter().position(|t| t.key == key)))
    }
    /// Opens the menu of the title keyed `key`, or closes the open one.
    fn set_open(&self, key: Option<String>) -> Result<(), RuntimeError> {
        let stale = self.closed.borrow_mut().take();
        if stale.is_some() && stale == key {
            // The signal still holds this key; clear it first so the title is
            // notified that its menu opens.
            self.open.set(None)?;
        }
        self.open.set(key)
    }
    /// The key of the title at `index`.
    fn key(&self, index: usize) -> Option<String> {
        self.titles.borrow().get(index).map(|t| t.key.clone())
    }
    /// Records whether the title keyed `key` is enabled.
    fn set_available(&self, key: &str, available: bool) {
        if let Some(title) = self.titles.borrow_mut().iter_mut().find(|t| t.key == key) {
            title.available = available;
        }
    }
    /// Takes over the titles of `next`, a core built by a new description,
    /// keeping what the mounted titles know: whether each surviving title is
    /// enabled, and which menu is open while its title survives. Returns
    /// whether the titles changed.
    fn adopt(&self, next: &Self) -> bool {
        let mut titles = next.titles.take();
        let changed = {
            let current = self.titles.borrow();
            for title in &mut titles {
                if let Some(previous) = current.iter().find(|t| t.key == title.key) {
                    title.available = previous.available;
                    title.visible = previous.visible;
                }
            }
            current.len() != titles.len()
                || current
                    .iter()
                    .zip(&titles)
                    .any(|(a, b)| a.key != b.key || a.menu != b.menu)
        };
        // The new menus act on the context of an engagement already under way.
        next.capture.borrow_mut().clone_from(&self.capture.borrow());
        let mut adopted = self.adopted.borrow_mut();
        adopted.retain(|capture| capture.strong_count() > 0);
        adopted.push(Rc::downgrade(&next.capture));
        drop(adopted);
        // An open menu whose title is gone, or no longer opens a menu, stays
        // closed until the next write, including when a later description
        // brings the title or its menu back.
        let open = self.open.get().ok().flatten();
        let closed = open.filter(|key| {
            self.closed.borrow().as_deref() == Some(key)
                || !titles.iter().any(|t| &t.key == key && t.menu)
        });
        *self.closed.borrow_mut() = closed;
        *self.titles.borrow_mut() = titles;
        changed
    }
    /// The next enabled title from `from` in direction `step`, wrapping; with
    /// `menus` only titles that open a menu count.
    fn next(&self, from: usize, step: isize, menus: bool) -> Option<usize> {
        let titles = self.titles.borrow();
        let count = titles.len() as isize;
        (1..=count)
            .map(|k| (from as isize + step * k).rem_euclid(count) as usize)
            .find(|index| {
                titles[*index].visible
                    && titles[*index].available
                    && (!menus || titles[*index].menu)
            })
    }
}

/// The handle titles and menu chains hold to their bar's [`BarCore`].
///
/// A description's titles and menus are built around the core of that
/// description, and those that mount keep it. Reconciliation points the
/// handle of a new description at the mounted bar's core, so one core serves
/// every title and menu of the bar, whichever description built them.
struct BarState {
    core: RefCell<Rc<BarCore>>,
}
impl BarState {
    /// The core this handle currently refers to.
    fn core(&self) -> Rc<BarCore> {
        self.core.borrow().clone()
    }
}

/// One title of a bar: the anchor and owner of its menu, or a direct action.
struct MenuTitle {
    /// The item key, which the element key `title/{key}` carries.
    key: String,
    state: Rc<BarState>,
    /// The title's text and leading icon, following a bound label.
    caption: Caption,
    enabled: bool,
    live: Option<crate::Permission>,
    available: bool,
    popup: Option<Element>,
    action: Option<LeafAction>,
    shown: bool,
    hovered: bool,
    pressed: bool,
    held: Option<KeyCode>,
    /// A press or held key ended by a change of title kind, whose input
    /// context the next update cancels.
    retargeted: bool,
    focused: bool,
    text: RefCell<Option<TextLayout>>,
}
impl MenuTitle {
    /// Opens this title's menu, or runs its action, which ends menu mode as a
    /// command chosen from a menu does.
    fn activate(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.available {
            return Ok(());
        }
        let core = self.state.core();
        core.engage(cx)?;
        match &mut self.action {
            Some(action) => {
                core.leave(cx)?;
                action.run(&core.capture, cx)
            }
            None => core.set_open(Some(self.key.clone())),
        }
    }
    /// Whether the bar's open menu is this title's.
    fn is_open(&self) -> Result<bool, RuntimeError> {
        let core = self.state.core();
        Ok(core.shows(core.open.get()?.as_deref(), &self.key))
    }
}
impl Component for MenuTitle {
    fn take_children(&mut self) -> Vec<Element> {
        self.popup.take().into_iter().collect()
    }
    /// Takes the new caption, enablement and action. The menu itself is the
    /// reconciled child; hover, press and open state are retained, except that
    /// a change between a direct action and a menu ends an armed press or held
    /// key, which belonged to the previous kind.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            key: _,
            state: _,
            caption,
            enabled,
            live,
            available: _,
            popup: _,
            action,
            shown: _,
            hovered: _,
            pressed: _,
            held: _,
            retargeted: _,
            focused: _,
            text: _,
        } = next;
        if (!enabled || self.action.is_some() != action.is_some())
            && (self.pressed || self.held.is_some())
        {
            self.pressed = false;
            self.held = None;
            self.retargeted = true;
        }
        // The key is the element key, and both handles refer to the bar's core.
        let layout = self.caption.reconcile(caption);
        let paint = self.enabled != enabled
            || !match (&self.live, &live) {
                (Some(a), Some(b)) => a.same_as(b),
                (None, None) => true,
                _ => false,
            }
            || self.action.is_some() != action.is_some();
        if layout {
            self.text.take();
        }
        self.enabled = enabled;
        self.live = live;
        self.action = action;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.retargeted) {
            cx.cancel_input();
        }
        if self.caption.update(cx)? {
            self.text.take();
        }
        let core = self.state.core();
        let open = cx.read(&core.open)?;
        let live = match &self.live {
            Some(enabled) => enabled.read(cx)?,
            None => true,
        };
        let was_available = self.available;
        self.available = self.enabled && live;
        if was_available && !self.available {
            // Revocation ends this title's interaction: an armed press or
            // held key cannot complete later, and its open menu stays closed
            // when the title becomes available again.
            if open.as_deref() == Some(self.key.as_str()) {
                core.closed.replace(Some(self.key.clone()));
            }
            self.pressed = false;
            self.held = None;
            cx.cancel_input();
        }
        self.shown = core.shows(open.as_deref(), &self.key);
        core.set_available(&self.key, self.available);
        self.focused = cx.focused();
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.available && self.state.core().keyboard.get()
    }
    fn tab_stop(&self) -> bool {
        // Arrow keys move between titles; Tab leaves the title row for the
        // next tab stop, which ends menu mode.
        false
    }
    fn child_visible(&self, _: &str) -> bool {
        self.shown && self.available
    }
    fn child_surface(&self, _: &str) -> Option<SurfaceKind> {
        Some(SurfaceKind::Popover)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        // Switching titles replaces this menu with another; only a dismissal of
        // the menu that is still open closes the bar's menu.
        if self.is_open()? {
            self.state.core().set_open(None)?;
        }
        self.shown = false;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx
            .rich_text(&self.caption.display(), None, TextStyle::Body)?
            .size();
        Ok(Size::new(
            text.width() + 2.0 * cx.metric(MetricRole::ControlPaddingX),
            (text.height() + 2.0 * cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        *self.text.borrow_mut() = Some(cx.measurement().rich_text(
            &self.caption.display(),
            None,
            TextStyle::Body,
        )?);
        if cx.len() == 1 {
            // The runtime places a popover below or above its owner; this only
            // supplies the size.
            let viewport = cx.measurement().viewport();
            let menu = cx.measure(
                0,
                Constraints::new(Some(viewport.width()), Some(viewport.height()))?,
            )?;
            cx.place(0, rect(0.0, size.height(), menu.width(), menu.height())?)?;
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let inside = e.kind() != PointerKind::Leave && cx.pointer_inside();
        if inside != self.hovered {
            self.hovered = inside;
            cx.invalidate(Invalidation::Paint);
        }
        match e.kind() {
            // While any menu of the bar is open, pointing at another title opens
            // its menu instead.
            PointerKind::Move if inside && self.available && self.action.is_none() => {
                let core = self.state.core();
                if core.open_index()?.is_some() && !self.is_open()? {
                    core.set_open(Some(self.key.clone()))?;
                }
            }
            PointerKind::Press(PointerButton::Primary) if self.available => {
                if self.action.is_some() {
                    self.pressed = true;
                    cx.capture_pointer()?;
                    cx.invalidate(Invalidation::Paint);
                } else if !self.shown {
                    self.activate(cx)?;
                }
            }
            PointerKind::Release(PointerButton::Primary) if self.pressed => {
                self.pressed = false;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                if inside {
                    self.activate(cx)?;
                }
            }
            PointerKind::Move | PointerKind::Leave => {}
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(e);
        if matches!(control, KeyCode::Enter | KeyCode::Space) {
            let result = crate::activation::key_result(
                control,
                self.pressed || self.held.is_some(),
                cx.modifiers(),
            );
            if self.held.is_some() {
                if e.transition == KeyTransition::Released && self.held == Some(e.code) {
                    self.held = None;
                }
                return Ok(result);
            }
            if self.pressed || result == InputOutcome::Ignored {
                return Ok(result);
            }
            if e.transition == KeyTransition::Pressed {
                self.held = Some(e.code);
            }
        }
        let opens = match control {
            KeyCode::Enter | KeyCode::Space => true,
            // Modified arrows are left to command bindings.
            KeyCode::ArrowDown | KeyCode::ArrowUp => {
                self.action.is_none() && keys.chord_modifiers(e) == Modifiers::default()
            }
            _ => false,
        };
        if !opens {
            return Ok(InputOutcome::Ignored);
        }
        if e.transition == KeyTransition::Pressed {
            self.activate(cx)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.pressed = false;
        self.held = None;
        self.hovered = false;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let bounds = Rect::from_size(size);
        let fill = if self.shown {
            Some(ColorRole::Primary)
        } else if (self.hovered || self.pressed) && self.available {
            Some(ColorRole::SurfaceHover)
        } else {
            None
        };
        if let Some(fill) = fill {
            p.fill_shape(
                bounds,
                rounded(p.metric(MetricRole::CornerRadius))?,
                p.color(fill),
            )?;
        }
        if let Some(text) = self.text.borrow().as_ref() {
            text.paint(
                p,
                p.metric(MetricRole::ControlPaddingX),
                centered(size.height(), text.size().height()),
                p.color(if !self.available {
                    ColorRole::Muted
                } else if self.shown {
                    ColorRole::OnPrimary
                } else {
                    ColorRole::OnSurface
                }),
            )?;
        }
        if self.focused {
            focus_ring(p, bounds)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.validation.interactive = self.available;
        d.properties
            .insert("open".into(), InspectValue::Bool(self.shown));
        let semantics = if self.action.is_some() {
            Semantics::new(SemanticRole::Button).enabled_actions([SemanticAction::Activate])
        } else {
            Semantics::new(SemanticRole::Menu)
                .expanded(self.shown)
                .enabled_actions([if self.shown {
                    SemanticAction::Collapse
                } else {
                    SemanticAction::Expand
                }])
        };
        d.semantics = Some(
            semantics
                .name(self.caption.text())
                .disabled(!self.available),
        );
        d
    }
}

/// An application menu bar: a row of titles such as File, Edit and Help, each
/// opening a menu, with desktop menu-bar pointer and keyboard behavior.
///
/// Each top-level [`MenuItem::submenu`] becomes a title that opens its items;
/// any other top-level item becomes a title that runs its action directly, and
/// top-level separators are ignored. Rows keep every [`MenuItem`] feature:
/// nested submenus, separators, shortcuts, check and radio marks and
/// [`MenuItem::command`] rows. Keys follow the [`Menu`] rules at every level.
///
/// Pointer: pressing a title opens its menu; while a menu is open, pointing at
/// another title switches to its menu, and pressing outside the menu, including
/// its own title, closes it.
///
/// Keyboard: F10 moves focus to the first enabled title. The titles are not Tab
/// stops otherwise, so the bar does not lengthen ordinary Tab traversal. On a
/// focused title, unmodified Left and Right move between titles, Home and End go
/// to the first and last, and unmodified Enter/Space or Down/Up open its menu.
/// Inside an open menu, unmodified Left and Right move to the neighboring menu
/// when the row does not open a submenu. Modified chords of these keys are left
/// to command bindings. Escape closes one level at a time and, on a title, leaves the bar;
/// F10 again leaves it from anywhere. Choosing an action also leaves the bar.
/// Leaving returns focus to what was focused when the bar was engaged.
///
/// F10 reaches the bar through ordinary key bubbling, like the Shift+F10 of a
/// [`ContextMenu`]: from the bar itself, or from focused content inside it. Give
/// the application's content to [`Self::content`] so the bar is its ancestor; F10
/// then works wherever focus is in that content. With no focused component in
/// the window, the first visible bar receives F10 directly (see
/// [`Component::receives_unfocused_keys`]), so it works before anything has been
/// focused. Menu mnemonics such as Alt+F are not provided.
///
/// A bar captures one [`CommandContext`] when it is engaged by pointer or F10,
/// before anything in it is focused, so [`MenuItem::command`] rows act on the
/// content target and focus from before the bar was used. Every menu is an
/// anchored popover below its title, following the [`Popover`] placement rules.
/// The bar spans the width it is given and clips titles that do not fit. No
/// native OS menu bar is created, including on macOS.
///
/// [`Self::leading`] and [`Self::trailing`] add application content to the
/// title row: before the first title, such as an application mark, and at the
/// row's right edge, such as navigation or window buttons. Slot content is
/// ordinary content: its controls are Tab stops in visual order (leading, then
/// trailing, then [`Self::content`]), and F10, Left, Right, Home and End move
/// between titles only, never into a slot. The row is as tall as the tallest
/// of the titles, the slots and [`MetricRole::ControlHeight`]; slot content
/// keeps its measured size and is centered vertically. When the bar is too
/// narrow, the titles keep their place after the leading slot and the
/// trailing slot moves right of the last title instead of overlapping it, so
/// whatever does not fit is clipped at the bar's right edge.
///
/// Reconciling a new description, for example one with an added "Recent files"
/// menu, takes its titles in their new order. Titles are matched by item key: a
/// surviving title keeps its identity, keyboard focus and open menu, a new one
/// mounts and a removed one unmounts, closing its menu. The captured command
/// context and keyboard menu mode survive as well. Hidden titles and optional
/// slots are omitted from layout; later titles compact while slots retain their
/// roles. F10 and title navigation skip hidden titles without changing their
/// retained identities, and showing them again restores their ordinary position.
///
/// ```
/// use pert_ui_runtime::{
///     Button, Column, CommandScope, Element, KeyAction, KeyChord, KeyCode, MenuBar,
///     MenuItem, Runtime, Viewport,
/// };
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
/// let save = ui.register_command("Save", CommandScope::Application,
///     vec![KeyChord::primary('s')], |_, _| Ok(()))?;
/// let wrap = ui.signal(true);
/// let bar = MenuBar::new(&ui, vec![
///     MenuItem::submenu("file", "File", vec![
///         MenuItem::command(&ui, "save", &save, |_, _| Ok(()))?,
///         MenuItem::separator(),
///         MenuItem::new("quit", "Quit", |_| Ok(())),
///     ]),
///     MenuItem::submenu("view", "View", vec![
///         MenuItem::toggle("wrap", "Wrap lines", wrap.clone()),
///     ]),
/// ])?
/// .content(Element::new("document", Button::new("Document", |_| Ok(()))));
/// ui.mount_tree(Element::new("window", Column::new([Element::new("bar", bar)])))?;
/// ui.flush(Duration::ZERO)?;
///
/// ui.focus_next(false)?; // the document: titles are not Tab stops
/// let document = ui.focused();
/// assert!(document.is_some());
/// let press = |ui: &mut Runtime, key| -> Result<(), Box<dyn std::error::Error>> {
///     ui.key_input(key, KeyAction::Press)?;
///     ui.key_input(key, KeyAction::Release)?;
///     ui.flush(Duration::ZERO)?;
///     Ok(())
/// };
/// press(&mut ui, KeyCode::F10)?;
/// assert_ne!(ui.focused(), document); // the File title
/// press(&mut ui, KeyCode::Escape)?;
/// assert_eq!(ui.focused(), document);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct MenuBar {
    /// The titles, in display order; the slots and content follow them as
    /// children in [`Component::take_children`].
    children: Vec<Element>,
    leading_element: Option<Element>,
    trailing_element: Option<Element>,
    content_element: Option<Element>,
    leading: bool,
    trailing: bool,
    state: Rc<BarState>,
    /// Title identities by position, from the latest update.
    titles: Vec<Option<WidgetId>>,
    /// Position of the title that holds focus or whose menu does.
    focused: Option<usize>,
    content: bool,
    row: Cell<f32>,
}
/// Phase-local indices of the bar's currently visible generated children.
#[derive(Default)]
struct BarLayout {
    titles: Vec<usize>,
    leading: Option<usize>,
    trailing: Option<usize>,
    content: Option<usize>,
}
impl MenuBar {
    /// Creates a bar from top-level items, usually one [`MenuItem::submenu`] per
    /// title. `ui` owns the bar's internal state and all callbacks run on its
    /// thread.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Rejects empty or whitespace keys and duplicate sibling keys at every level.
    pub fn new(ui: &impl crate::StateOwner, menus: Vec<MenuItem>) -> Result<Self, ChoiceError> {
        validate_menu(&menus)?;
        let menus: Vec<_> = menus.into_iter().filter(|item| !item.separator).collect();
        let core = Rc::new(BarCore {
            open: ui.signal(None),
            closed: RefCell::new(None),
            keyboard: Cell::new(false),
            arming: Cell::new(false),
            titles: RefCell::new(
                menus
                    .iter()
                    .map(|item| TitleState {
                        key: item.key.clone(),
                        menu: item.submenu.is_some(),
                        available: false,
                        visible: false,
                    })
                    .collect(),
            ),
            capture: Rc::default(),
            adopted: RefCell::new(Vec::new()),
        });
        let state = Rc::new(BarState {
            core: RefCell::new(core.clone()),
        });
        let mut children = Vec::new();
        for mut item in menus {
            let popup = item
                .submenu
                .take()
                .map(|items| menu_panel(&core.open, items, chain(&state, &core)))
                .transpose()?
                .map(|panel| Element::new("menu", panel).structural());
            let action = popup.is_none().then_some(item.action);
            children.push(
                Element::new(
                    format!("title/{}", item.key),
                    MenuTitle {
                        key: item.key,
                        state: state.clone(),
                        caption: item.caption,
                        enabled: item.enabled,
                        live: item.live,
                        available: false,
                        popup,
                        action,
                        shown: false,
                        hovered: false,
                        pressed: false,
                        held: None,
                        retargeted: false,
                        focused: false,
                        text: RefCell::new(None),
                    },
                )
                .structural(),
            );
        }
        Ok(Self {
            children,
            leading_element: None,
            trailing_element: None,
            content_element: None,
            leading: false,
            trailing: false,
            state,
            titles: Vec::new(),
            focused: None,
            content: false,
            row: Cell::new(0.0),
        })
    }
    /// Places `content` below the bar, filling the rest of the space it is given,
    /// and makes the bar its ancestor so F10 reaches the bar from focused content.
    /// Calling it again replaces the content.
    pub fn content(mut self, content: Element) -> Self {
        self.content_element = Some(Element::new("content", Container::new(content)).structural());
        self.content = true;
        self
    }
    /// Places `content` on the title row before the first title, for example an
    /// application mark. It keeps its measured size and is centered vertically
    /// in the row; its controls are ordinary Tab stops that F10 and the title
    /// arrow keys do not reach. Calling it again replaces the content, and
    /// reconciliation keeps its state by key like any child.
    ///
    /// ```
    /// use pert_ui_runtime::{Button, Element, MenuBar, MenuItem, Runtime, Viewport};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(640.0, 480.0, 1.0)?)?;
    /// let bar = MenuBar::new(&ui, vec![
    ///     MenuItem::submenu("file", "File", vec![MenuItem::new("quit", "Quit", |_| Ok(()))]),
    /// ])?
    /// .leading(Element::new("mark", Button::new("App", |_| Ok(()))))
    /// .trailing(Element::new("back", Button::new("Back", |_| Ok(()))));
    /// let root = ui.mount_tree(Element::new("bar", bar))?;
    /// ui.flush(Duration::ZERO)?;
    /// // Slots are Tab stops in visual order; the titles are not.
    /// ui.focus_next(false)?;
    /// assert_eq!(ui.focused(), ui.find_descendant(&root, "mark")?);
    /// ui.focus_next(false)?;
    /// assert_eq!(ui.focused(), ui.find_descendant(&root, "back")?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn leading(mut self, content: Element) -> Self {
        self.leading_element = Some(Element::new("leading", Container::new(content)).structural());
        self.leading = true;
        self
    }
    /// Places `content` on the title row at its right edge, for example
    /// navigation or window buttons. It keeps its measured size and is centered
    /// vertically; in a bar too narrow for it, it moves right of the last title
    /// instead of overlapping it and is clipped at the bar's edge. Its controls
    /// are ordinary Tab stops after the leading slot's. Calling it again
    /// replaces the content.
    pub fn trailing(mut self, content: Element) -> Self {
        self.trailing_element =
            Some(Element::new("trailing", Container::new(content)).structural());
        self.trailing = true;
        self
    }
    /// Resolve stable generated keys to this callback's visible child indices.
    fn slots(&self, cx: &MeasureContext<'_>) -> Result<BarLayout, RuntimeError> {
        let mut slots = BarLayout::default();
        for index in 0..cx.len() {
            match cx.key(index)? {
                "leading" if self.leading => slots.leading = Some(index),
                "trailing" if self.trailing => slots.trailing = Some(index),
                "content" if self.content => slots.content = Some(index),
                key if key.starts_with("title/") => slots.titles.push(index),
                _ => {}
            }
        }
        Ok(slots)
    }
    /// Starts keyboard menu mode on the first enabled title.
    fn activate(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let core = self.state.core();
        let first = core
            .titles
            .borrow()
            .iter()
            .position(|t| t.visible && t.available);
        let Some(title) = first
            .and_then(|index| self.titles.get(index))
            .and_then(Option::as_ref)
        else {
            return Ok(());
        };
        core.engage(cx)?;
        core.keyboard.set(true);
        core.arming.set(true);
        cx.request_focus_for(title)
    }
}
/// The chain of each menu in a bar: closing leaves the bar, and Left or Right at
/// the edge of a menu opens the neighboring one. The closures go through the
/// handle, so they reach the mounted bar's core after reconciliation; the
/// capture cell is the description's own, which [`BarCore::adopt`] registers.
fn chain(state: &Rc<BarState>, core: &BarCore) -> MenuChain {
    let leave = state.clone();
    let side = state.clone();
    MenuChain {
        close: Rc::new(move |cx| leave.core().leave(cx)),
        capture: core.capture.clone(),
        sideways: Some(Rc::new(move |step| {
            let core = side.core();
            if let Some(open) = core.open_index()?
                && let Some(next) = core.next(open, step, true)
            {
                core.set_open(core.key(next))?;
            }
            Ok(())
        })),
    }
}
impl Component for MenuBar {
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        // Focus left the bar by a real transfer or native window focus loss;
        // hiding a focused menu is a cancellation path that tells nothing.
        // Either way menu mode ends, so the next F10 opens the bar again.
        if !within {
            let core = self.state.core();
            core.keyboard.set(false);
            core.arming.set(false);
        }
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        let mut children = std::mem::take(&mut self.children);
        children.extend(self.leading_element.take());
        children.extend(self.trailing_element.take());
        children.extend(self.content_element.take());
        children
    }
    /// Takes the new titles, slots and content. The titles are reconciled children;
    /// this folds the new description's shared state into the mounted one, so
    /// titles and menus from either description drive one bar.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            leading_element: _,
            trailing_element: _,
            content_element: _,
            leading,
            trailing,
            state,
            titles: _,
            focused: _,
            content,
            row: _,
        } = next;
        let core = self.state.core();
        let changed = core.adopt(&state.core());
        *state.core.borrow_mut() = core;
        let content_changed =
            self.content != content || self.leading != leading || self.trailing != trailing;
        self.content = content;
        self.leading = leading;
        self.trailing = trailing;
        // Title identities and the focused position refresh in the next update.
        (changed || content_changed).then_some(Invalidation::Layout)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Reading subscribes to focus moving. Focus inside the bar but outside
        // its titles is in the content, which ends keyboard mode. Focus that is
        // nowhere does not: hiding a focused menu leaves it there briefly until
        // the runtime restores it to the menu's opener.
        let within = cx.focus_within();
        let core = self.state.core();
        let mut titles = core.titles.borrow_mut();
        self.focused = None;
        self.titles = titles
            .iter_mut()
            .enumerate()
            .map(|(index, title)| {
                let key = format!("title/{}", title.key);
                let child = cx.children().iter().find(|child| child.key() == key);
                title.visible = child.is_some_and(ChildInfo::visible);
                if title.visible && child.is_some_and(ChildInfo::focus_within) {
                    self.focused = Some(index);
                }
                child.map(|child| child.id().clone())
            })
            .collect();
        drop(titles);
        if self.focused.is_some() {
            core.arming.set(false);
        } else if !core.arming.get() && within {
            core.keyboard.set(false);
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let BarLayout {
            titles,
            leading,
            trailing,
            content,
        } = self.slots(cx)?;
        let mut width = 0.0;
        let mut row = cx.metric(MetricRole::ControlHeight);
        for index in titles.into_iter().chain(leading).chain(trailing) {
            let child = cx.measure(index, Constraints::new(None, None)?)?;
            width += child.width();
            row = row.max(child.height());
        }
        let mut height = row;
        if let Some(content) = content {
            let rest = available.height().map(|height| (height - row).max(0.0));
            let content = cx.measure(content, Constraints::new(available.width(), rest)?)?;
            width = width.max(content.width());
            height = available.height().unwrap_or(row + content.height());
        }
        Ok(Size::new(available.width().unwrap_or(width), height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let BarLayout {
            titles,
            leading,
            trailing,
            content,
        } = self.slots(cx.measurement())?;
        let mut row = cx.measurement().metric(MetricRole::ControlHeight);
        let mut widths = Vec::with_capacity(titles.len());
        for index in titles {
            let title = cx.measure(index, Constraints::new(None, None)?)?;
            widths.push((index, title.width()));
            row = row.max(title.height());
        }
        let slot_size = |index: Option<usize>| -> Result<Option<(usize, Size)>, RuntimeError> {
            index
                .map(|index| Ok((index, cx.measure(index, Constraints::new(None, None)?)?)))
                .transpose()
        };
        let leading = slot_size(leading)?;
        let trailing = slot_size(trailing)?;
        for (_, measured) in leading.iter().chain(trailing.iter()) {
            row = row.max(measured.height());
        }
        // Titles keep their full height in a shorter bar and are clipped, so a
        // bar given no space yet has no empty hit areas.
        let mut x = 0.0;
        if let Some((index, measured)) = leading {
            let y = (row - measured.height()) / 2.0;
            cx.place(index, rect(x, y, measured.width(), measured.height())?)?;
            x += measured.width();
        }
        for (index, width) in widths {
            cx.place(index, rect(x, 0.0, width, row)?)?;
            x += width;
        }
        if let Some((index, measured)) = trailing {
            // Right-aligned, but never over a title: a narrow bar clips it.
            let left = (size.width() - measured.width()).max(x);
            let y = (row - measured.height()) / 2.0;
            cx.place(index, rect(left, y, measured.width(), measured.height())?)?;
        }
        let row = row.min(size.height());
        if let Some(content) = content {
            cx.place(
                content,
                rect(0.0, row, size.width(), (size.height() - row).max(0.0))?,
            )?;
        }
        self.row.set(row);
        Ok(())
    }
    fn clips_children(&self) -> bool {
        true
    }
    fn receives_unfocused_keys(&self) -> bool {
        true
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(e);
        let pressed = matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        );
        let core = self.state.core();
        if control == KeyCode::F10 && keys.modifiers() == Modifiers::default() {
            if e.transition == KeyTransition::Pressed {
                if core.engaged()? {
                    core.leave(cx)?;
                } else {
                    self.activate(cx)?;
                }
            }
            return Ok(InputOutcome::Handled);
        }
        // Keys that reach the bar from its content are not the bar's.
        let Some(current) = self.focused else {
            return Ok(InputOutcome::Ignored);
        };
        let count = core.titles.borrow().len();
        // Modified navigation chords are left to command bindings.
        if control != KeyCode::Escape && keys.chord_modifiers(e) != Modifiers::default() {
            return Ok(InputOutcome::Ignored);
        }
        let target = match control {
            KeyCode::ArrowLeft => core.next(current, -1, false),
            KeyCode::ArrowRight => core.next(current, 1, false),
            KeyCode::Home => core.next(count.saturating_sub(1), 1, false),
            KeyCode::End => core.next(0, -1, false),
            KeyCode::Escape => {
                if e.transition == KeyTransition::Pressed {
                    core.leave(cx)?;
                }
                return Ok(InputOutcome::Handled);
            }
            _ => return Ok(InputOutcome::Ignored),
        };
        if pressed
            && let Some(title) = target
                .and_then(|index| self.titles.get(index))
                .and_then(Option::as_ref)
        {
            cx.request_focus_for(title)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let row = self.row.get().min(size.height());
        p.fill_rect(
            rect(0.0, 0.0, size.width(), row)?,
            p.color(ColorRole::Surface),
        );
        let line = p.metric(MetricRole::BorderWidth).min(row);
        p.fill_rect(
            rect(0.0, row - line, size.width(), line)?,
            p.color(ColorRole::Divider),
        );
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties.insert(
            "keyboard".into(),
            InspectValue::Bool(self.state.core().keyboard.get()),
        );
        // The titles are components and describe themselves.
        d.semantics = Some(Semantics::new(SemanticRole::MenuBar));
        d
    }
}
