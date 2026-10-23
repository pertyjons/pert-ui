//! Collapsible sections and accordions: headers that show and hide keyed
//! content, with the expansion as retained arrangement.
use crate::*;
use std::{cell::RefCell, collections::BTreeSet};

/// One header row of a [`Collapsible`] or an [`Accordion`]: the key of the
/// content it shows and the title it presents.
struct Row {
    key: String,
    title: RichText,
    name: String,
}

/// A header's arrangement: its local rectangle and prepared title.
struct Placed {
    key: String,
    rect: Rect,
    text: TextLayout,
}

/// The input that armed a header, and which header it armed.
#[derive(Clone, PartialEq)]
enum Armed {
    Pointer(String),
    Key(KeyCode, String),
}

/// Theme measurements a header row uses, read during layout.
#[derive(Clone, Copy)]
struct Metrics {
    height: f32,
    padding_x: f32,
    padding_y: f32,
    icon: f32,
    gap: f32,
}
impl Metrics {
    fn read(cx: &MeasureContext<'_>) -> Self {
        Self {
            height: cx.metric(MetricRole::ControlHeight),
            padding_x: cx.metric(MetricRole::ControlPaddingX),
            padding_y: cx.metric(MetricRole::ControlPaddingY),
            icon: cx.metric(MetricRole::IconSize),
            gap: cx.metric(MetricRole::IconGap),
        }
    }
    /// Where the title starts: after the leading padding, chevron and gap.
    fn lead(self) -> f32 {
        self.padding_x + self.icon + self.gap
    }
    /// The title's wrapping width inside a header of `width`, or `None`
    /// when the width is unbounded.
    fn text_width(self, width: Option<f32>) -> Option<f32> {
        width.map(|width| (width - self.lead() - self.padding_x).max(0.0))
    }
    fn header_height(self, text: &TextLayout) -> f32 {
        self.height.max(text.size().height() + 2.0 * self.padding_y)
    }
}

/// The header rows, expansion and header input shared by [`Collapsible`] and
/// [`Accordion`]. Content children are identified by key; a row whose key is
/// open shows its content directly under its header.
struct Headers {
    rows: Vec<Row>,
    open: BTreeSet<String>,
    /// The header that keyboard input acts on.
    active: Option<String>,
    armed: Option<Armed>,
    hovered: Option<String>,
    focused: bool,
    /// The application's restriction of header activation; `enabled` holds
    /// its last read value.
    enablement: Permission,
    enabled: bool,
    placed: RefCell<Vec<Placed>>,
}
impl Headers {
    fn new() -> Self {
        Self {
            rows: Vec::new(),
            open: BTreeSet::new(),
            active: None,
            armed: None,
            hovered: None,
            focused: false,
            enablement: Permission::allowed(),
            enabled: true,
            placed: RefCell::new(Vec::new()),
        }
    }
    fn reset_input(&mut self) {
        self.armed = None;
        self.hovered = None;
    }
    /// Combines `permission` into the description's restriction, applying a
    /// fixed denial at once.
    fn restrict(&mut self, permission: impl Into<Permission>) {
        self.enablement = self.enablement.clone().and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
    }
    /// Adopts a new description's permission, returning whether it changed.
    /// A fixed denial cancels an armed press at once; a binding is read at
    /// the next update.
    fn adopt_enablement(&mut self, next: Permission) -> bool {
        if self.enablement.same_as(&next) {
            return false;
        }
        if let Some(enabled) = next.fixed() {
            if self.enabled && !enabled {
                self.reset_input();
            }
            self.enabled = enabled;
        }
        self.enablement = next;
        true
    }
    /// Reads the permission in the owner's update. Revocation cancels an
    /// armed press, which then toggles nothing, and leaves expansion alone.
    fn read_enablement(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        if !enabled && self.armed.is_some() {
            self.reset_input();
            cx.cancel_input();
        }
        self.enabled = enabled;
        Ok(())
    }
    fn header_at(&self, point: Point) -> Option<String> {
        self.placed
            .borrow()
            .iter()
            .find(|placed| placed.rect.contains(point))
            .map(|placed| placed.key.clone())
    }
    /// Keeps the active header on a described row.
    fn settle_active(&mut self) {
        let present = self
            .active
            .as_ref()
            .is_some_and(|key| self.rows.iter().any(|row| &row.key == key));
        if !present {
            self.active = self.rows.first().map(|row| row.key.clone());
        }
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let metrics = Metrics::read(cx);
        let content_space = Constraints::new(available.width(), None)?;
        let mut width: f32 = 0.0;
        let mut height = 0.0;
        for row in &self.rows {
            let text = cx.rich_text(
                &row.title,
                metrics.text_width(available.width()),
                TextStyle::Body,
            )?;
            width = width.max(metrics.lead() + text.size().width() + metrics.padding_x);
            height += metrics.header_height(&text);
            if let Some(index) = visible_index(cx, &row.key)? {
                let content = cx.measure(index, content_space)?;
                width = width.max(content.width());
                height += content.height();
            }
        }
        Ok(Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let metrics = Metrics::read(cx.measurement());
        let mut placed = Vec::with_capacity(self.rows.len());
        let mut y = 0.0;
        for row in &self.rows {
            let text = cx.measurement().rich_text(
                &row.title,
                metrics.text_width(Some(size.width())),
                TextStyle::Body,
            )?;
            let height = metrics.header_height(&text);
            placed.push(Placed {
                key: row.key.clone(),
                rect: Rect::new(0.0, y, size.width(), height)?,
                text,
            });
            y += height;
            if let Some(index) = visible_index(cx.measurement(), &row.key)? {
                let content = cx.measure(index, Constraints::new(Some(size.width()), None)?)?;
                cx.place(index, Rect::new(0.0, y, size.width(), content.height())?)?;
                y += content.height();
            }
        }
        *self.placed.borrow_mut() = placed;
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let metrics = Metrics {
            height: p.metric(MetricRole::ControlHeight),
            padding_x: p.metric(MetricRole::ControlPaddingX),
            padding_y: p.metric(MetricRole::ControlPaddingY),
            icon: p.metric(MetricRole::IconSize),
            gap: p.metric(MetricRole::IconGap),
        };
        let border = p.metric(MetricRole::BorderWidth);
        for placed in self.placed.borrow().iter() {
            let rect = placed.rect;
            let [x, y, w, h] = rect.values();
            let armed = self.armed.as_ref().is_some_and(|armed| match armed {
                Armed::Pointer(key) | Armed::Key(_, key) => key == &placed.key,
            });
            if self.enabled && (armed || self.hovered.as_ref() == Some(&placed.key)) {
                p.fill_rect(rect, p.color(ColorRole::SurfaceHover));
            }
            let line = border.min(h);
            p.fill_rect(
                Rect::new(x, y + h - line, w, line)?,
                p.color(ColorRole::Divider),
            );
            let side = metrics.icon.min(h).min((w - metrics.padding_x).max(0.0));
            if side > 0.0 {
                p.icon(
                    if self.open.contains(&placed.key) {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    },
                    Rect::new(x + metrics.padding_x, y + (h - side) / 2.0, side, side)?,
                    p.color(ColorRole::Muted),
                )?;
            }
            placed.text.paint(
                p,
                crate::Point::new(
                    x + metrics.lead(),
                    y + ((h - placed.text.size().height()) / 2.0).max(0.0),
                )?,
                p.color(if self.enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
            if self.focused && self.active.as_ref() == Some(&placed.key) {
                p.focus_ring(
                    rect,
                    Shape::Rounded(Length::new(p.metric(MetricRole::CornerRadius))?),
                )?;
            }
        }
        Ok(())
    }
    /// Tracks hover and a primary press on a header; returns the header a
    /// matched release activates.
    fn pointer(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<(InputOutcome, Option<String>), RuntimeError> {
        let over = (event.kind() != PointerKind::Leave && cx.pointer_inside())
            .then(|| cx.to_local(event.position()))
            .flatten()
            .and_then(|point| self.header_at(point));
        if over != self.hovered {
            self.hovered = over.clone();
            cx.invalidate(Invalidation::Paint);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                let Some(key) = over else {
                    return Ok((InputOutcome::Ignored, None));
                };
                if self.armed.is_some() {
                    return Ok((InputOutcome::Handled, None));
                }
                cx.capture_pointer()?;
                cx.request_focus();
                self.active = Some(key.clone());
                self.armed = Some(Armed::Pointer(key));
                cx.invalidate(Invalidation::Paint);
                Ok((InputOutcome::Handled, None))
            }
            PointerKind::Release(PointerButton::Primary) => {
                let Some(Armed::Pointer(key)) = self.armed.clone() else {
                    return Ok((InputOutcome::Ignored, None));
                };
                self.armed = None;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                Ok((
                    InputOutcome::Handled,
                    (over == Some(key.clone())).then_some(key),
                ))
            }
            PointerKind::Move | PointerKind::Leave => Ok((
                if self.armed.is_some() {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                },
                None,
            )),
            _ => Ok((InputOutcome::Ignored, None)),
        }
    }
    /// Tracks Enter, Space and Escape on the active header; returns the
    /// header a release of the arming key activates.
    fn activation_key(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        control: KeyCode,
    ) -> (InputOutcome, Option<String>) {
        if crate::activation::key_result(control, self.armed.is_some(), cx.modifiers())
            == InputOutcome::Ignored
        {
            return (InputOutcome::Ignored, None);
        }
        match control {
            KeyCode::Escape => {
                if self.armed.take().is_some() {
                    cx.release_pointer();
                    cx.invalidate(Invalidation::Paint);
                    (InputOutcome::Handled, None)
                } else {
                    (InputOutcome::Ignored, None)
                }
            }
            KeyCode::Enter | KeyCode::Space => match event.transition {
                KeyTransition::Pressed if self.armed.is_none() => {
                    if let Some(key) = self.active.clone() {
                        self.armed = Some(Armed::Key(event.code, key));
                        cx.invalidate(Invalidation::Paint);
                    }
                    (InputOutcome::Handled, None)
                }
                KeyTransition::Released => match self.armed.clone() {
                    Some(Armed::Key(code, key)) if code == event.code => {
                        self.armed = None;
                        cx.invalidate(Invalidation::Paint);
                        (InputOutcome::Handled, Some(key))
                    }
                    _ => (InputOutcome::Handled, None),
                },
                _ => (InputOutcome::Handled, None),
            },
            _ => (InputOutcome::Ignored, None),
        }
    }
    fn items(&self) -> Vec<SemanticItem> {
        self.rows
            .iter()
            .map(|row| {
                let open = self.open.contains(&row.key);
                let item = SemanticItem::new(row.name.clone())
                    .selected(self.active.as_ref() == Some(&row.key))
                    .disabled(!self.enabled)
                    .expanded(open);
                if self.enabled {
                    item.action(if open {
                        SemanticAction::Collapse
                    } else {
                        SemanticAction::Expand
                    })
                } else {
                    item
                }
            })
            .collect()
    }
}

/// Whether two optional bindings are the same signal.
fn same_binding(a: &Option<Signal<bool>>, b: &Option<Signal<bool>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.read_only().same_source(&b.read_only()),
        _ => false,
    }
}

/// The visible child index holding `key`, if that child is shown.
fn visible_index(cx: &MeasureContext<'_>, key: &str) -> Result<Option<usize>, RuntimeError> {
    for index in 0..cx.len() {
        if cx.key(index)? == key {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// A header that shows and hides one content element.
///
/// The header is drawn by the section: a chevron ([`Icon::ChevronRight`]
/// collapsed, [`Icon::ChevronDown`] expanded, both overridable through
/// [`Runtime::icons`]), the title and a divider, at least ControlHeight tall.
/// The content sits directly below the header at the section's full width,
/// measured with unbounded height, and the section is as tall as both; put it
/// in a scrolling container when the content can outgrow the page. The section
/// starts collapsed unless [`Self::expanded`] says otherwise.
///
/// **Input.** The header is the section's one Tab stop. A primary press and
/// release on the header, or unmodified Enter or Space pressed and released while it has
/// focus, toggle the content; Escape cancels an armed press. Hiding, focus
/// loss and unmount cancel an armed press without toggling. A disabled
/// section takes no focus and does not toggle.
///
/// **Collapsed content** is reported not visible through
/// [`Component::child_visible`], so it stops updating, painting and running
/// deadlines exactly as an unselected tab does, while its component state,
/// drafts and scroll positions survive. Focus inside content that collapses
/// moves to the header, by the runtime's ordinary rule for hidden focus.
///
/// **Expansion** is the section's own state by default. [`Self::bind_expanded`] makes
/// it the application's `Signal<bool>` instead: the section shows what the
/// signal holds and writes it when the user toggles. A reveal
/// ([`Runtime::reveal`], [`Runtime::reveal_and_focus`],
/// [`Runtime::reveal_child`], [`EventContext::request_reveal`]) of the content
/// or anything inside it expands the section through the same transition the
/// header runs, so a bound signal is written too; a disabled section refuses.
///
/// **Workspace.** An unbound section takes part in [`Workspace`] persistence
/// as the `collapsible` kind with the single flag `expanded`, so a restored
/// workspace reopens what the user left open. [`Self::persist`]`(false)`
/// leaves it out, for a section that should always start in its initial
/// state. A bound section never takes part: its expansion is application
/// state, which the application persists itself if it wants to.
///
/// **Identity** is the element key of the section and of its content; the
/// title is presentation. Describing the section again (through
/// [`Runtime::reconcile_children`]) with a new title, binding, persistence or
/// disabled state keeps its expansion; [`Self::expanded`] applies only when
/// the section mounts.
///
/// Inspection reports a [`SemanticRole::Group`] named by the title, with
/// `expanded` set, the [`SemanticAction::Expand`] or
/// [`SemanticAction::Collapse`] it offers while enabled, and the `expanded`
/// and `persist` properties.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 240.0, 1.0)?)?;
/// let section = ui.mount_tree(Element::new(
///     "details",
///     Collapsible::new("Details", Element::new("body", Label::new("Hidden until opened"))),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// let shown = |ui: &Runtime, key: &str| {
///     ui.inspect().components.iter().any(|c| c.key == key && c.visible)
/// };
/// assert!(!shown(&ui, "body"));
///
/// // Enter on the focused header expands the section.
/// ui.focus(&section)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(shown(&ui, "body"));
/// # Ok::<(), RuntimeError>(())
/// ```
#[must_use]
pub struct Collapsible {
    headers: Headers,
    content: Option<Element>,
    binding: Option<Signal<bool>>,
    retargeted: bool,
    persist: bool,
    /// The title and its leading icon slot. The header row shows its
    /// composition, which [`Self::sync_title`] keeps current.
    title: Caption,
}
description_debug!(Collapsible);
impl Collapsible {
    /// Creates a collapsed, enabled, persisted section titled `title` around
    /// `content`, whose element key identifies the content.
    ///
    /// The title is a [`TextSource`]: fixed plain or rich text, which may
    /// include icons, or a binding the header follows without the section
    /// being described again, such as a translated title. Its plain text is
    /// the section's name in inspection. The title must not be blank: a blank
    /// fixed title makes the section refuse to mount or change with
    /// [`LayoutError::BlankLabel`], leaving a mounted section as it was, and a
    /// bound title that reads blank keeps the header's last title and fails
    /// the section's update with the same error. A bound title is empty until
    /// its first read.
    pub fn new(title: impl Into<TextSource>, content: Element) -> Self {
        let mut headers = Headers::new();
        headers.rows.push(Row {
            key: content.key().to_owned(),
            name: String::new(),
            title: RichText::default(),
        });
        headers.settle_active();
        let mut section = Self {
            headers,
            content: Some(content),
            binding: None,
            retargeted: false,
            persist: true,
            title: Caption::new(title),
        };
        section.sync_title();
        section
    }
    /// Replaces the leading icon slot: one icon shown before the title with
    /// the theme's icon gap, independent of the title. A second call replaces
    /// the icon rather than adding one, and the slot stays when a bound title
    /// changes; the plain title stays the section's name. Compose several
    /// icons in a rich title when more than one is wanted.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.title.leading = Some(icon.into());
        self.sync_title();
        self
    }
    /// Shows the current title and its icon slot in the header row.
    fn sync_title(&mut self) {
        let row = &mut self.headers.rows[0];
        row.title = self.title.display().into_owned();
        row.name = self.title.text().to_owned();
    }
    /// Starts expanded (`true`) or collapsed (`false`, the default) when the
    /// section mounts. A restored workspace or a bound signal decides after
    /// that; describing the section again does not change it.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.set_open(expanded);
        self
    }
    /// Makes the expansion the application's: the section shows the
    /// signal's value, and the header and reveals write it. The section then
    /// stays out of workspace persistence, since the signal is application
    /// state. [`Self::expanded`] is ignored for a bound section.
    pub fn bind_expanded(mut self, expanded: impl Into<Signal<bool>>) -> Self {
        let expanded: Signal<bool> = expanded.into();
        self.binding = Some(expanded);
        self
    }
    /// Takes part in workspace persistence (`true`, the default) or stays out
    /// of it (`false`). Has no effect on a [bound](Self::bind_expanded) section, which
    /// never takes part.
    pub fn persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }
    /// Disables the header: no focus, no toggling, a muted title, and
    /// reveals refused. The content keeps its current expansion. Equivalent
    /// to `enabled(false)`; a later permission cannot lift the denial.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts header activation with a fixed or reactive permission;
    /// allowed by default. While denied the header takes no focus, pointer and
    /// keyboard toggling and reveals are refused, and the title is muted; the
    /// content keeps its expansion, and a [bound](Self::bind_expanded) expansion still
    /// follows the application. Repeated calls combine restrictions; every
    /// condition must allow. Same-key redescription replaces the permission.
    /// Revoking a bound permission cancels an armed press at the next update
    /// without toggling. A binding from another runtime fails the update with
    /// [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.headers.restrict(permission);
        self
    }
    fn header_key(&self) -> &str {
        &self.headers.rows[0].key
    }
    fn is_open(&self) -> bool {
        self.headers.open.contains(self.header_key())
    }
    fn set_open(&mut self, open: bool) -> bool {
        let key = self.header_key().to_owned();
        if open {
            self.headers.open.insert(key)
        } else {
            self.headers.open.remove(&key)
        }
    }
    /// The one expansion transition, shared by the header and the reveal
    /// hook: changes the local state, writes a bound signal, and invalidates.
    fn set_expanded(&mut self, cx: &mut EventContext, open: bool) -> Result<bool, RuntimeError> {
        if let Some(binding) = &self.binding {
            binding.set(open)?;
        }
        let changed = self.set_open(open);
        if changed {
            cx.invalidate(Invalidation::Layout);
        }
        Ok(changed)
    }
}
impl Component for Collapsible {
    fn check_description(&self) -> Result<(), RuntimeError> {
        // A bound title is checked when it is read.
        if self
            .title
            .source()
            .fixed()
            .is_some_and(|title| title.plain_text().trim().is_empty())
        {
            return Err(LayoutError::BlankLabel.into());
        }
        Ok(())
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        if self.title.reconcile(next.title) {
            self.sync_title();
            invalidation = Some(Invalidation::Layout);
        }
        let next_row = next.headers.rows.into_iter().next();
        if let Some(next_row) = next_row {
            let row = &mut self.headers.rows[0];
            if row.key != next_row.key {
                // The content was described under another key, so it is a new
                // child: expansion follows the section, not the old key.
                let open = self.headers.open.remove(&row.key);
                row.key = next_row.key.clone();
                if open {
                    self.headers.open.insert(next_row.key);
                }
                self.headers.active = Some(row.key.clone());
                // A press armed for the old content must not toggle its
                // replacement on release.
                self.headers.reset_input();
                invalidation = Some(Invalidation::Layout);
            }
        }
        if !same_binding(&self.binding, &next.binding) {
            self.headers.reset_input();
            self.retargeted = true;
            self.binding = next.binding;
            invalidation = Some(Invalidation::Layout);
        }
        self.persist = next.persist;
        if self.headers.adopt_enablement(next.headers.enablement) {
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        invalidation
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.content.take().into_iter().collect()
    }
    fn child_visible(&self, key: &str) -> bool {
        key != self.header_key() || self.is_open()
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        key: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        if key != self.header_key() || !self.headers.enabled {
            return Ok(ChildReveal::Refused);
        }
        if self.is_open() {
            return Ok(ChildReveal::AlreadyVisible);
        }
        self.set_expanded(cx, true)?;
        Ok(ChildReveal::Changed)
    }
    fn focusable(&self) -> bool {
        self.headers.enabled
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (self.headers.enabled && self.headers.header_at(point).is_some())
            .then_some(CursorIcon::Default)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.headers.hovered.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.retargeted) {
            cx.cancel_input();
        }
        self.headers.read_enablement(cx)?;
        if let Some(binding) = &self.binding {
            let open = cx.read(binding)?;
            if self.set_open(open) {
                cx.request_layout();
            }
        }
        self.headers.focused = cx.focused();
        // Last, so a blank title fails the update only after the rest is current.
        if self
            .title
            .update_nonblank(cx, || LayoutError::BlankLabel.into())?
        {
            self.sync_title();
            cx.request_layout();
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.headers.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.headers.arrange(cx, size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.headers.paint(painter)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.headers.enabled {
            return Ok(InputOutcome::Ignored);
        }
        let (outcome, activated) = self.headers.pointer(cx, event)?;
        if activated.is_some() {
            let open = !self.is_open();
            self.set_expanded(cx, open)?;
        }
        Ok(outcome)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.headers.enabled || !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        let (outcome, activated) = self
            .headers
            .activation_key(cx, event, keys.control_key(event));
        if activated.is_some() {
            let open = !self.is_open();
            self.set_expanded(cx, open)?;
        }
        Ok(outcome)
    }
    fn input_reset(&mut self) {
        self.headers.reset_input();
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        if !self.persist || self.binding.is_some() {
            return Ok(None);
        }
        let state = ViewState::new("collapsible")?
            .with("expanded", WorkspaceValue::flag(self.is_open()))?;
        Ok(Some(state))
    }
    fn restore_workspace_state(
        &mut self,
        state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        state.expect_only(&["expanded"])?;
        let open = state.flag("expanded")?;
        if self.headers.armed.is_some() {
            return Err(RestoreRejection::Busy);
        }
        Ok(if self.set_open(open) {
            Invalidation::Layout
        } else {
            Invalidation::Paint
        })
    }
    fn inspect(&self) -> ComponentDetails {
        let open = self.is_open();
        let mut semantics = Semantics::new(SemanticRole::Group)
            .name(self.headers.rows[0].name.clone())
            .actions(self.headers.enabled.then_some(if open {
                SemanticAction::Collapse
            } else {
                SemanticAction::Expand
            }));
        semantics.expanded = Some(open);
        semantics.disabled = !self.headers.enabled;
        let mut details = ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        };
        details
            .properties
            .insert("expanded".into(), InspectValue::Bool(open));
        details.properties.insert(
            "persist".into(),
            InspectValue::Bool(self.persist && self.binding.is_none()),
        );
        details
    }
}

/// How many sections of an [`Accordion`] may be open at once.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ExpandMode {
    /// At most one: opening a section closes the one that was open. All may
    /// be closed.
    Single,
    /// Any number, each toggled on its own. The default.
    #[default]
    Multiple,
}

/// One section of an [`Accordion`]: a stable key, a display title and content.
///
/// The key is the section's identity, for expansion, workspace persistence
/// and reveals; the title is presentation and may change without opening or
/// closing anything.
#[must_use]
pub struct Section {
    element: Element,
    /// The replaceable leading icon slot, composed before the title when the
    /// section becomes an element.
    leading: Option<IconSource>,
}
description_debug!(Section, key = |section| section.element.key.as_str());
impl Section {
    /// Creates a section. The key must not be blank and must be unique in its
    /// accordion, and the title must not be blank; [`Accordion`] checks both
    /// when it is described ([`LayoutError::BlankKey`],
    /// [`LayoutError::DuplicateKey`], [`LayoutError::BlankLabel`]).
    ///
    /// The title is a [`TextSource`], as for [`Tab::new`]: a binding is
    /// followed without describing the accordion again, and a blank read
    /// keeps the last title and fails the accordion's update with
    /// [`LayoutError::BlankLabel`].
    pub fn new(key: impl Into<String>, title: impl Into<TextSource>, content: Element) -> Self {
        Self {
            element: Element::new(key, View::new(content)).with_label(title),
            leading: None,
        }
    }
    /// Replaces the title with styled text and icons, keeping the plain title
    /// as the section's name when the styled one has no text, and keeping the
    /// [`Self::leading_icon`] slot.
    pub fn rich_title(mut self, title: RichText) -> Self {
        self.element = self.element.with_rich_label(title);
        self
    }
    /// Replaces the leading icon slot: one icon shown before the title with
    /// the theme's icon gap, independent of the title. A second call replaces
    /// the icon rather than adding one. Compose several icons in a rich title
    /// when more than one is wanted.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.leading = Some(icon.into());
        self
    }
    /// The ordinary labeled view description the accordion holds as a child,
    /// for [`Runtime::reconcile_children`]: reusing its key keeps the
    /// section's expansion and its content's state.
    pub fn into_element(mut self) -> Element {
        if self.element.label_source().is_some() {
            // A bound title is composed with its icon when it is read.
            self.element = self.element.with_label_leading(self.leading.take());
        }
        if let Some(icon) = &self.leading {
            let title = self
                .element
                .rich_label()
                .cloned()
                .unwrap_or_else(|| RichText::from(self.element.label().unwrap_or_default()));
            self.element = self
                .element
                .with_rich_label(title.leading_icon(icon.clone()));
        }
        self.element
    }
}
impl From<Section> for Element {
    /// Converts as [`Section::into_element`] does, so a list of sections can be
    /// collected into the `Vec<Element>` that
    /// [`Runtime::reconcile_children`](crate::Runtime::reconcile_children)
    /// takes with `.map(Element::from)` or `.map(Into::into)`.
    fn from(section: Section) -> Self {
        section.into_element()
    }
}

/// Several collapsible sections under one policy: one open at a time or any
/// number, with the expansion owned by the accordion.
///
/// Each section's header is drawn by the accordion, as [`Collapsible`] draws
/// its one header, and each section's content sits below its header. The
/// sections' contents are the accordion's children, keyed by section key, so
/// [`Runtime::reconcile_children`] with [`Section::into_element`] adds,
/// removes, reorders and renames sections; expansion and content state follow
/// the keys, never the titles or positions. A removed section's expansion is
/// forgotten, so a section added again under the same key starts closed.
///
/// **Input.** The accordion is one Tab stop with an active header, which
/// starts at the first section and is outlined while the accordion has focus.
/// Up and Down move the active header to the previous or next section, Home
/// and End to the first or last, and unmodified Enter or Space toggle it (on release, as
/// a button does). A primary press and release on any header toggles that
/// section and makes it the active header. Escape cancels an armed press, and
/// hiding, focus loss and unmount cancel it without toggling. Tab from the
/// accordion moves into the open sections' content in order.
///
/// **Policy.** With [`ExpandMode::Single`] opening a section closes the open
/// one, and all may be closed; with [`ExpandMode::Multiple`], the default,
/// each section toggles on its own. Closed sections' content stops its
/// presentation work and keeps its state, as for [`Collapsible`], and focus
/// inside a closing section moves to the accordion.
///
/// **Reveals** of a closed section's content, or of anything inside it, open
/// that section through the same transition the header runs, so the policy
/// applies; a disabled accordion refuses.
///
/// **Workspace.** The accordion takes part in [`Workspace`] persistence as the
/// `accordion` kind with one map, `open`, of the open section keys, each
/// mapped to the flag `true`. Restoring refuses a key that is not a described
/// section as unresolved, any value but a flag, and more than one open section
/// in single mode, leaving the accordion unchanged. Sections' contents keep
/// their own addresses below the section keys. [`Self::persist`]`(false)`
/// leaves the accordion out. A persisted accordion records its open section
/// keys, so they must be storable workspace text (at most
/// [`Workspace::MAX_STRING_BYTES`] bytes and no control characters); while
/// such a key is open, capturing the workspace fails with
/// [`WorkspaceError::Participant`] naming the accordion, so give such sections
/// other keys or turn persistence off.
///
/// Inspection reports a [`SemanticRole::Group`] with one
/// [`SemanticItem`] per section in order, naming it by its title, `selected`
/// for the active header, with `expanded` and the
/// [`SemanticAction::Expand`] or [`SemanticAction::Collapse`] it offers while
/// enabled, and the `mode`, `persist` and `active` properties.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::*;
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(320.0, 400.0, 1.0)?)?;
/// let accordion = ui.mount_tree(Element::new(
///     "settings",
///     Accordion::new([
///         Section::new("general", "General", Element::new("a", Label::new("Name and size"))),
///         Section::new("advanced", "Advanced", Element::new("b", Label::new("Expert options"))),
///     ])
///     .mode(ExpandMode::Single)
///     .expanded(["general"]),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// let shown = |ui: &Runtime, key: &str| {
///     ui.inspect().components.iter().any(|c| c.key == key && c.visible)
/// };
/// assert!(shown(&ui, "general") && !shown(&ui, "advanced"));
///
/// // Down moves to the Advanced header; Space opens it and closes General.
/// ui.focus(&accordion)?;
/// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
/// ui.key_input(KeyCode::Space, KeyAction::Press)?;
/// ui.key_input(KeyCode::Space, KeyAction::Release)?;
/// ui.flush(Duration::ZERO)?;
/// assert!(!shown(&ui, "general") && shown(&ui, "advanced"));
/// # Ok::<(), RuntimeError>(())
/// ```
#[must_use]
pub struct Accordion {
    headers: Headers,
    children: Vec<Element>,
    /// Described or restore-admitted section keys, retained before layout updates.
    keys: Vec<String>,
    mode: ExpandMode,
    persist: bool,
    on_change: Option<AccordionHandler>,
    pending: std::collections::VecDeque<Option<ExpansionChange>>,
    notifier: Option<crate::Notifier>,
    /// Why the described sections are invalid, reported from `check_description`.
    invalid: Option<LayoutError>,
}
description_debug!(Accordion);
/// Called with the open section keys, in display order, after they change.
type AccordionHandler =
    Box<dyn FnMut(&mut EventContext, &ExpansionChange) -> Result<(), RuntimeError>>;

/// A change of an [`Accordion`]'s open sections, reported through
/// [`Accordion::on_change`] after it took effect.
///
/// Keys identify sections, never positions or titles.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ExpansionChange {
    /// The keys of the open sections after the change, in display order;
    /// empty when every section is closed.
    pub expanded: Vec<String>,
    /// What caused the change.
    pub origin: crate::ChangeOrigin,
}
impl ExpansionChange {
    /// Describes a change that left the sections `expanded` open.
    #[must_use]
    pub fn new(expanded: Vec<String>, origin: crate::ChangeOrigin) -> Self {
        Self { expanded, origin }
    }
}
impl Accordion {
    /// Creates an accordion over `sections`, in order, all closed, in
    /// [`ExpandMode::Multiple`], enabled and persisted.
    pub fn new(sections: impl IntoIterator<Item = Section>) -> Self {
        let children: Vec<Element> = sections.into_iter().map(Section::into_element).collect();
        let invalid = crate::tabs::check_labeled(&children).err();
        Self {
            headers: Headers::new(),
            keys: children
                .iter()
                .map(|child| child.key().to_owned())
                .collect(),
            children,
            mode: ExpandMode::default(),
            persist: true,
            on_change: None,
            pending: std::collections::VecDeque::new(),
            notifier: None,
            invalid,
        }
    }
    /// Sets how many sections may be open at once. Default
    /// [`ExpandMode::Multiple`]. Switching a mounted accordion to
    /// [`ExpandMode::Single`] keeps only the first open section in display
    /// order.
    pub fn mode(mut self, mode: ExpandMode) -> Self {
        self.mode = mode;
        self
    }
    /// Opens these sections when the accordion mounts. Keys that name no
    /// section are ignored, and in [`ExpandMode::Single`] only the first open
    /// section in display order stays open. A restored workspace decides after
    /// that; describing the accordion again does not change it.
    pub fn expanded(mut self, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.headers.open = keys.into_iter().map(Into::into).collect();
        self
    }
    /// Takes part in workspace persistence (`true`, the default) or stays out
    /// of it (`false`).
    pub fn persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }
    /// Disables the headers: no focus, no toggling, muted titles, and reveals
    /// refused. Open sections stay open. Equivalent to `enabled(false)`; a
    /// later permission cannot lift the denial.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts header activation with a fixed or reactive permission;
    /// allowed by default. While denied the headers take no focus, pointer and
    /// keyboard toggling and reveals are refused, and titles are muted; open
    /// sections stay open. Repeated calls combine restrictions; every
    /// condition must allow. Same-key redescription replaces the permission.
    /// Revoking a bound permission cancels an armed press at the next update
    /// without toggling. A binding from another runtime fails the update with
    /// [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.headers.restrict(permission);
        self
    }
    /// Calls a UI-thread handler with an [`ExpansionChange`] after each change:
    /// the open section keys, in display order, and its
    /// [`ChangeOrigin`](crate::ChangeOrigin).
    /// Changes from input, reveal, workspace restore and redescription are
    /// queued in order and delivered after layout settles, in the event phase.
    /// Initial expansion and restoration before the first successful update are
    /// not reported. The callback may write signals.
    /// Failure is returned without undoing the change or retrying the callback.
    /// Redescription replaces the handler; removing it discards queued changes.
    /// Removing the component prevents delivery. By default there is no handler.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Accordion, Element, Label, Runtime, Section};
    /// let mut ui = Runtime::new();
    /// let open = ui.signal(Vec::<String>::new());
    /// let seen = open.clone();
    /// let accordion = Accordion::new([
    ///     Section::new("a", "First", Element::new("a-body", Label::new("A"))),
    ///     Section::new("b", "Second", Element::new("b-body", Label::new("B"))),
    /// ])
    /// .on_change(move |_, change| seen.set(change.expanded.clone()));
    /// let root = ui.mount_tree(Element::new("accordion", accordion))?;
    /// ui.reveal_child(&root, "b")?;
    /// assert_eq!(open.get()?, ["b"]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, &ExpansionChange) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
    /// Removes open keys that name no section, and in single mode all but
    /// the first open section in `order`. Returns whether anything closed.
    fn enforce(&mut self, order: &[String]) -> bool {
        let before = self.headers.open.len();
        self.headers.open.retain(|key| order.contains(key));
        if self.mode == ExpandMode::Single && self.headers.open.len() > 1 {
            let first = order
                .iter()
                .find(|key| self.headers.open.contains(*key))
                .cloned();
            self.headers.open = first.into_iter().collect();
        }
        self.headers.open.len() != before
    }
    fn order(&self) -> Vec<String> {
        if self.headers.rows.is_empty() {
            self.keys.clone()
        } else {
            self.headers
                .rows
                .iter()
                .map(|row| row.key.clone())
                .collect()
        }
    }
    /// The one expansion transition, shared by the headers and the reveal
    /// hook, under the accordion's policy.
    fn set_open(&mut self, cx: &mut EventContext, key: &str, open: bool) -> bool {
        let changed = if open {
            if self.mode == ExpandMode::Single {
                let was = std::mem::take(&mut self.headers.open);
                self.headers.open.insert(key.to_owned());
                was.len() != 1 || !was.contains(key)
            } else {
                self.headers.open.insert(key.to_owned())
            }
        } else {
            self.headers.open.remove(key)
        };
        if changed {
            cx.invalidate(Invalidation::Layout);
        }
        changed
    }
    fn toggle(&mut self, cx: &mut EventContext, key: &str) -> Result<(), RuntimeError> {
        let open = !self.headers.open.contains(key);
        if self.set_open(cx, key, open) {
            self.record(crate::ChangeOrigin::User);
        }
        Ok(())
    }
    fn record(&mut self, origin: crate::ChangeOrigin) {
        if self.on_change.is_none() {
            return;
        }
        if let Some(notifier) = &self.notifier {
            let open = self
                .keys
                .iter()
                .filter(|key| self.headers.open.contains(*key))
                .cloned()
                .collect();
            self.pending
                .push_back(Some(ExpansionChange::new(open, origin)));
            notifier.request();
        }
    }
    fn step(&mut self, cx: &mut EventContext, code: KeyCode) -> bool {
        let rows = &self.headers.rows;
        if rows.is_empty() {
            return false;
        }
        let current = self
            .headers
            .active
            .as_ref()
            .and_then(|key| rows.iter().position(|row| &row.key == key))
            .unwrap_or(0);
        let next = match code {
            KeyCode::ArrowUp => current.saturating_sub(1),
            KeyCode::ArrowDown => (current + 1).min(rows.len() - 1),
            KeyCode::Home => 0,
            KeyCode::End => rows.len() - 1,
            _ => return false,
        };
        if next != current {
            self.headers.active = Some(rows[next].key.clone());
            // A key armed on the old header must not toggle the new one.
            self.headers.armed = None;
            cx.invalidate(Invalidation::Paint);
        }
        true
    }
}
impl Accordion {
    fn capture_open(
        &self,
        contains: impl Fn(&str) -> bool,
    ) -> Result<Option<ViewState>, WorkspaceError> {
        if !self.persist {
            return Ok(None);
        }
        let open = WorkspaceValue::map(
            self.headers
                .open
                .iter()
                .filter(|key| contains(key))
                .map(|key| (key.clone(), WorkspaceValue::flag(true))),
        )?;
        Ok(Some(ViewState::new("accordion")?.with("open", open)?))
    }
    fn restore_open(
        &mut self,
        state: &ViewState,
        order: &[String],
    ) -> Result<Invalidation, RestoreRejection> {
        state.expect_only(&["open"])?;
        let entries = state.map("open")?;
        let mut open = BTreeSet::new();
        for (key, value) in entries {
            let flag = value
                .as_flag()
                .ok_or_else(|| RestoreRejection::Value("open".to_owned()))?;
            if !order.contains(key) {
                return Err(RestoreRejection::Unresolved(key.clone()));
            }
            if flag {
                open.insert(key.clone());
            }
        }
        if self.mode == ExpandMode::Single && open.len() > 1 {
            return Err(RestoreRejection::Value("open".to_owned()));
        }
        if self.headers.armed.is_some() {
            return Err(RestoreRejection::Busy);
        }
        // Capture must retain newly admitted keys before rows are refreshed.
        self.keys = order.to_vec();
        if open == self.headers.open {
            return Ok(Invalidation::Paint);
        }
        self.headers.open = open;
        self.record(crate::ChangeOrigin::Restore);
        Ok(Invalidation::Layout)
    }
}

impl Component for Accordion {
    fn check_description(&self) -> Result<(), RuntimeError> {
        // The sections were taken when the element was built; the constructor
        // checked them.
        self.invalid
            .clone()
            .map_or(Ok(()), |error| Err(error.into()))
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        self.keys = next.keys;
        // Closures cannot be compared, so the described handler replaces.
        self.on_change = next.on_change;
        if self.on_change.is_none() {
            // Preserve request positions so a stale request cannot deliver a
            // newer change ahead of another component's notification.
            self.pending.iter_mut().for_each(|slot| *slot = None);
        }
        if self.persist != next.persist {
            // Persistence only changes what capture reports; capture checks
            // that the open section keys are storable workspace text.
            self.persist = next.persist;
            invalidation = Some(Invalidation::Paint);
        }
        if self.mode != next.mode {
            self.mode = next.mode;
            // Enforce against the new description's order, not the rows of
            // the previous one, so single mode keeps the first open section
            // as the sections are now shown.
            let order = self.keys.clone();
            if self.enforce(&order) {
                self.record(crate::ChangeOrigin::Redescription);
                invalidation = Some(Invalidation::Layout);
            }
        }
        if self.headers.adopt_enablement(next.headers.enablement) {
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        invalidation
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some(Some(change)) = self.pending.pop_front()
            && let Some(handler) = &mut self.on_change
        {
            handler(cx, &change)?;
        }
        Ok(())
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, key: &str) -> bool {
        self.headers.open.contains(key)
    }
    fn reveal_child(
        &mut self,
        cx: &mut EventContext,
        key: &str,
    ) -> Result<ChildReveal, RuntimeError> {
        if !self.headers.enabled || !self.order().iter().any(|candidate| candidate == key) {
            return Ok(ChildReveal::Refused);
        }
        if self.headers.open.contains(key) {
            return Ok(ChildReveal::AlreadyVisible);
        }
        if self.set_open(cx, key, true) {
            self.record(crate::ChangeOrigin::Reveal);
        }
        Ok(ChildReveal::Changed)
    }
    fn focusable(&self) -> bool {
        self.headers.enabled && !self.headers.rows.is_empty()
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (self.headers.enabled && self.headers.header_at(point).is_some())
            .then_some(CursorIcon::Default)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.headers.hovered.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.headers.read_enablement(cx)?;
        let rows = cx
            .children()
            .iter()
            .map(|child| {
                let name = child
                    .label()
                    .filter(|label| !label.trim().is_empty())
                    .ok_or(LayoutError::BlankLabel)?
                    .to_owned();
                Ok(Row {
                    key: child.key().to_owned(),
                    title: child
                        .rich_label()
                        .cloned()
                        .unwrap_or_else(|| RichText::from(name.as_str())),
                    name,
                })
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        let order: Vec<String> = rows.iter().map(|row| row.key.clone()).collect();
        let changed = rows.len() != self.headers.rows.len()
            || rows
                .iter()
                .zip(&self.headers.rows)
                .any(|(a, b)| a.key != b.key || a.title != b.title);
        self.headers.rows = rows;
        self.keys = order.clone();
        let closed = self.enforce(&order);
        if closed {
            self.record(crate::ChangeOrigin::Redescription);
        }
        self.notifier.get_or_insert_with(|| cx.notifier());
        if closed || changed {
            cx.request_layout();
        }
        self.headers.settle_active();
        if let Some(Armed::Pointer(key) | Armed::Key(_, key)) = &self.headers.armed
            && !order.contains(key)
        {
            self.headers.armed = None;
            cx.cancel_input();
        }
        self.headers.focused = cx.focused();
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.headers.measure(cx, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        self.headers.arrange(cx, size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.headers.paint(painter)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.headers.enabled {
            return Ok(InputOutcome::Ignored);
        }
        let (outcome, activated) = self.headers.pointer(cx, event)?;
        if let Some(key) = activated {
            self.toggle(cx, &key)?;
        }
        Ok(outcome)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if !self.headers.enabled || !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(
            control,
            KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::Home | KeyCode::End
        ) {
            if event.transition == KeyTransition::Released
                || keyboard.chord_modifiers(event) != Modifiers::default()
            {
                return Ok(InputOutcome::Ignored);
            }
            return Ok(if self.step(cx, control) {
                InputOutcome::Handled
            } else {
                InputOutcome::Ignored
            });
        }
        let (outcome, activated) = self.headers.activation_key(cx, event, control);
        if let Some(key) = activated {
            self.toggle(cx, &key)?;
        }
        Ok(outcome)
    }
    fn input_reset(&mut self) {
        self.headers.reset_input();
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        self.capture_open(|key| {
            self.keys.iter().any(|candidate| candidate == key)
                || self.headers.rows.iter().any(|row| row.key == key)
        })
    }
    fn workspace_state_with_children(
        &self,
        children: &[ChildInfo],
    ) -> Result<Option<ViewState>, WorkspaceError> {
        self.capture_open(|key| children.iter().any(|child| child.key() == key))
    }
    fn restore_workspace_state(
        &mut self,
        state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        self.restore_open(state, &self.order())
    }
    fn restore_workspace_state_with_children(
        &mut self,
        state: &ViewState,
        children: &[ChildInfo],
    ) -> Result<Invalidation, RestoreRejection> {
        let order = children
            .iter()
            .map(|child| child.key().to_owned())
            .collect::<Vec<_>>();
        self.restore_open(state, &order)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut semantics = Semantics::new(SemanticRole::Group).items(self.headers.items());
        semantics.disabled = !self.headers.enabled;
        let mut details = ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        };
        details.properties.insert(
            "mode".into(),
            InspectValue::Text(
                match self.mode {
                    ExpandMode::Single => "single",
                    ExpandMode::Multiple => "multiple",
                }
                .into(),
            ),
        );
        details
            .properties
            .insert("persist".into(), InspectValue::Bool(self.persist));
        if let Some(active) = &self.headers.active {
            details
                .properties
                .insert("active".into(), InspectValue::Text(active.clone()));
        }
        details
    }
}

/// The open keys an accordion of this description starts with, for tests.
#[cfg(test)]
fn initial_open(accordion: &Accordion) -> std::collections::BTreeMap<String, bool> {
    accordion
        .keys
        .iter()
        .map(|key| (key.clone(), accordion.headers.open.contains(key)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_mode_keeps_the_first_open_section_in_order() {
        let mut accordion = Accordion::new([
            Section::new("a", "A", Element::new("x", Label::new("x"))),
            Section::new("b", "B", Element::new("y", Label::new("y"))),
        ])
        .mode(ExpandMode::Single)
        .expanded(["b", "a", "missing"]);
        let order = accordion.order();
        assert!(accordion.enforce(&order));
        assert_eq!(
            initial_open(&accordion),
            std::collections::BTreeMap::from([("a".to_owned(), true), ("b".to_owned(), false)])
        );
    }
}
