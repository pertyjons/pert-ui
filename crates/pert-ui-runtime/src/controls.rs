//! Standard text and activation controls with shared typography and interaction.
use crate::activation::{ActivationState, activation_key, key_result, pointer_result};
use crate::layout::centered;
use crate::popover::{HelpTimer, arrange_help, transient_panel};
use crate::*;
use std::cell::{Cell, RefCell};

/// Adds replaceable control adornments without changing the caption itself.
pub(crate) fn adorned_content<'a>(
    content: &'a RichText,
    leading: Option<&IconSource>,
    trailing: Option<&IconSource>,
) -> std::borrow::Cow<'a, RichText> {
    if leading.is_none() && trailing.is_none() {
        return std::borrow::Cow::Borrowed(content);
    }
    let mut result = content.clone();
    if let Some(icon) = leading {
        result = result.leading_icon(icon.clone());
    }
    if let Some(icon) = trailing {
        result = result.trailing_icon(icon.clone());
    }
    std::borrow::Cow::Owned(result)
}

/// A retained text label with automatic width wrapping and default body typography.
/// Its text is a [`TextSource`]: fixed plain or rich content, or a binding the
/// label follows. Shaping/rasterization occurs only for changed
/// content, allocation, scale or style. Font roles default to bundled Noto Sans
/// and Noto Sans Mono; `Runtime::fonts` can override them. Rich labels add spans
/// and shared font icons without creating extra focus targets.
///
/// [`Self::overflow`] keeps the label on one line and replaces what does not
/// fit with an ellipsis; the full text then stays the label's name in
/// inspection and shows as delayed help while the pointer rests on a truncated
/// label.
///
/// Re-describing the label under the same key applies its text or binding,
/// adornments, style, rotation, overflow, color and role; changing the overflow
/// mode resets the truncated-text help.
#[must_use]
pub struct Label {
    caption: Caption,
    style: TextStyle,
    prepared: RefCell<Option<TextLayout>>,
    color: Option<Color>,
    role: ColorRole,
    turn: QuarterTurn,
    painted_color: std::cell::Cell<Option<Color>>,
    overflow: TextOverflow,
    /// The text shown when the last arrangement truncated it.
    shown: RefCell<Option<String>>,
    /// The allocation of the last arrangement, which painting clips to.
    arranged: Cell<Option<Size>>,
    help: HelpTimer,
}
impl Label {
    /// Creates a label showing `text`: fixed plain or rich content, or a
    /// `ReadSignal<String>` or `ReadSignal<RichText>` it follows (see
    /// [`TextSource`]). Font, size and icon changes in rich content measure the
    /// label again. No font or style configuration is required.
    ///
    /// ```
    /// use pert_ui_runtime::{Icon, Label, RichText, Runtime};
    /// let ui = Runtime::new();
    /// let fixed = Label::new("Gain");
    /// let rich = Label::new(RichText::from("Gain").leading_icon(Icon::Info));
    /// let status = ui.signal(String::from("Idle"));
    /// let bound = Label::new(status.read_only());
    /// ```
    pub fn new(text: impl Into<TextSource>) -> Self {
        Self {
            caption: Caption::new(text),
            style: TextStyle::Body,
            color: None,
            role: ColorRole::OnSurface,
            turn: QuarterTurn::None,
            painted_color: std::cell::Cell::new(None),
            prepared: RefCell::new(None),
            overflow: TextOverflow::Wrap,
            shown: RefCell::new(None),
            arranged: Cell::new(None),
            help: HelpTimer::default(),
        }
    }
    /// Chooses how text that does not fit is shown. Default
    /// [`TextOverflow::Wrap`] wraps at the allocated width.
    ///
    /// With [`TextOverflow::Ellipsis`] or [`TextOverflow::MiddleEllipsis`] the
    /// label stays on one line: plain text loses the grapheme clusters that do
    /// not fit, replaced by "…", and rich content with icons or styled spans
    /// is clipped at the label's edge. Measured without a width limit, the
    /// label is as wide as its whole first line. The full text stays the
    /// label's name in inspection, the `truncated` property says whether text
    /// was removed and `shown` holds what the label shows, and while the
    /// pointer rests on a truncated label the full text appears as delayed
    /// help with the [`Tooltip`] timing and appearance. The label never takes
    /// focus, so keyboard users read the full text from the name.
    ///
    /// ```
    /// use pert_ui_runtime::{Element, Label, Runtime, TextOverflow, Viewport, Wrap};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(90.0, 60.0, 1.0)?)?;
    /// ui.mount_tree(Element::new(
    ///     "row",
    ///     Wrap::new([Element::new(
    ///         "path",
    ///         Label::new("/home/user/projects/mix/final.wav")
    ///             .overflow(TextOverflow::MiddleEllipsis),
    ///     )]),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// let label = ui.inspect().components.into_iter().find(|c| c.key == "path").unwrap();
    /// let name = label.details.semantics.unwrap().name.unwrap();
    /// assert_eq!(name, "/home/user/projects/mix/final.wav");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }
    /// Prepares the label's content for a line `line` logical pixels long,
    /// returning the layout and the shown text when it was truncated.
    fn prepare(
        &self,
        cx: &MeasureContext<'_>,
        line: Option<f32>,
    ) -> Result<(TextLayout, Option<String>), RuntimeError> {
        let content = self.caption.display();
        if self.overflow == TextOverflow::Wrap {
            return Ok((cx.rich_text(&content, line, self.style)?, None));
        }
        if let Some(plain) = content.as_plain() {
            let fitted =
                cx.fitted_text(plain, line.unwrap_or(f32::MAX), self.style, self.overflow)?;
            let shown = fitted.shown().map(str::to_owned);
            return Ok((fitted.into_layout(), shown));
        }
        // Rich content stays on one line and is clipped rather than cut.
        let layout = cx.rich_text(&content, None, self.style)?;
        let clipped = line.is_some_and(|line| layout.size().width() > line);
        Ok((layout, clipped.then(|| content.plain_text())))
    }
    /// Turns the label by a quarter turn, for a vertical axis or strip title.
    ///
    /// [`QuarterTurn::Left`] reads upwards and [`QuarterTurn::Right`] downwards.
    /// A label turned either way measures with its width and height exchanged and
    /// wraps against the height it is given. Default is upright.
    pub fn turned(mut self, turn: QuarterTurn) -> Self {
        self.turn = turn;
        self
    }
    /// Sets the leading icon and themed gap. Repeated calls replace this slot;
    /// it stays when the text changes.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.leading = Some(icon.into());
        self
    }
    /// Sets the trailing icon and themed gap. Repeated calls replace this slot;
    /// it stays when the text changes.
    pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.trailing = Some(icon.into());
        self
    }
    /// Uses a fixed text color across theme changes; overrides any selected role.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    /// Uses a theme role for text, replacing a literal override. Default is OnSurface.
    pub fn color_role(mut self, role: ColorRole) -> Self {
        self.role = role;
        self.color = None;
        self
    }
    /// Selects a semantic typography preset, keeping framework spacing and fonts.
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }
}
impl Component for Label {
    fn take_children(&mut self) -> Vec<Element> {
        if self.overflow == TextOverflow::Wrap {
            return Vec::new();
        }
        // The help follows the same source as the label, so a bound label's
        // help changes with it without any state of its own.
        let help = Label::new(self.caption.source().clone());
        vec![Element::new(
            "help",
            transient_panel(Element::new("text", help)).padding(MetricRole::ControlPaddingX),
        )]
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "help" || self.help.shown()
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "help").then_some(SurfaceKind::Tooltip)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.help.dismiss();
        Ok(())
    }
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let layout =
            self.turn != next.turn || self.style != next.style || self.overflow != next.overflow;
        let layout = self.caption.reconcile(next.caption) | layout;
        let paint = self.color != next.color || self.role != next.role;
        self.style = next.style;
        self.turn = next.turn;
        self.color = next.color;
        self.role = next.role;
        if self.overflow != next.overflow {
            self.overflow = next.overflow;
            self.help = HelpTimer::default();
            self.shown.replace(None);
        }
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.caption.update(cx)?;
        if self.overflow != TextOverflow::Wrap {
            // Help is offered only for text that is actually cut, as of the
            // latest arrangement.
            let active = cx.hovered() && self.shown.borrow().is_some();
            self.help.update(cx, active)?;
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        // A label turned on its side wraps against the height it is given.
        let line = if self.turn.swaps_axes() {
            available.height()
        } else {
            available.width()
        };
        let (layout, _) = self.prepare(cx, line)?;
        let mut size = layout.size();
        if self.overflow != TextOverflow::Wrap
            && let Some(line) = line
            && size.width() > line
        {
            size = Size::new(line, size.height())?;
        }
        Ok(self.turn.turned(size))
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let (layout, shown) =
            self.prepare(cx.measurement(), Some(self.turn.turned(size).width()))?;
        *self.prepared.borrow_mut() = Some(layout);
        // Help is offered only while the text is cut; when this arrangement
        // starts or stops cutting it, the update reconsiders the help with
        // the new state inside the same flush.
        let was_cut = self.shown.borrow().is_some();
        if self.overflow != TextOverflow::Wrap && was_cut != shown.is_some() {
            cx.request_update();
        }
        *self.shown.borrow_mut() = shown;
        self.arranged.set(Some(size));
        if cx.is_empty() {
            return Ok(());
        }
        if self.overflow == TextOverflow::Wrap {
            return Err(LayoutError::InvalidChild.into());
        }
        arrange_help(cx, size)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        if let Some(text) = self.prepared.borrow().as_ref() {
            let color = self.color.unwrap_or_else(|| painter.color(self.role));
            self.painted_color.set(Some(color));
            match self.arranged.get() {
                Some(size) if self.overflow != TextOverflow::Wrap => {
                    let bounds = Rect::new(0.0, 0.0, size.width(), size.height())?;
                    painter.with_clip(bounds, |painter| {
                        text.paint_turned(painter, 0.0, 0.0, self.turn, color)
                    })?;
                }
                _ => text.paint_turned(painter, 0.0, 0.0, self.turn, color)?,
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "text".into(),
            InspectValue::Text(self.caption.text().to_owned()),
        );
        if self.overflow != TextOverflow::Wrap {
            let shown = self.shown.borrow();
            details
                .properties
                .insert("truncated".into(), InspectValue::Bool(shown.is_some()));
            if let Some(shown) = shown.as_ref() {
                details
                    .properties
                    .insert("shown".into(), InspectValue::Text(shown.clone()));
            }
            details
                .properties
                .insert("help_open".into(), InspectValue::Bool(self.help.shown()));
        }
        details.box_model.foreground = self.painted_color.get().map(Color::channels);
        details.semantics = Some(Semantics::new(SemanticRole::Label).name(self.caption.text()));
        details
    }
}

type ActivateFn = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;
/// A standard action button with pointer capture, visible focus and keyboard activation.
///
/// Default is enabled, at least 64×32 logical pixels at Standard density, with body text and themed
/// padding and an opaque blue-gray action surface distinct from panels/tabs.
/// Matching primary-button release inside activates once; release outside,
/// Escape or context loss cancels. Unmodified Enter/Space activate on matched release, ignoring
/// OS repeats. Whichever of the pointer and a key arms first owns the gesture and
/// the other is ignored until it ends, so one gesture activates at most once, as
/// for the choice controls. The UI-thread callback may write bindings; failures
/// propagate.
///
/// ```
/// use pert_ui_runtime::{Button, Element, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let saved = ui.signal(0);
/// let count = saved.clone();
/// let button = ui.mount_tree(Element::new(
///     "save",
///     Button::new("Save", move |_| count.set(count.get()? + 1)),
/// ))?;
/// ui.focus(&button)?;
/// // Enter activates on its matched release.
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// assert_eq!(saved.get()?, 0);
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// assert_eq!(saved.get()?, 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the button under the same key applies its text or text
/// binding, leading/trailing adornments, icon-only and frameless presentation,
/// enablement and action; hover
/// and an armed activation are retained while it stays enabled.
#[must_use]
pub struct Button {
    /// The caption, or an icon-only button's action name.
    caption: Caption,
    /// The icon of an icon-only button, shown instead of the caption.
    icon: Option<IconSource>,
    action: ActivateFn,
    enabled: bool,
    enablement: Permission,
    input: ActivationState,
    prepared: RefCell<Option<TextLayout>>,
    frameless: bool,
}
impl Button {
    /// Creates an enabled button showing `text`; the callback expresses
    /// application intent.
    ///
    /// The text is a [`TextSource`]: fixed plain or rich content, or a
    /// binding the caption follows, for example a translated caption. Explicit
    /// [`Self::leading_icon`] and [`Self::trailing_icon`] slots stay when the
    /// caption changes; icons inside rich content are part of the caption.
    ///
    /// ```
    /// use pert_ui_runtime::{Button, Icon, RichText, Runtime};
    /// let ui = Runtime::new();
    /// let caption = ui.signal(String::from("Save"));
    /// let save = Button::new(caption.read_only(), |_| Ok(())).leading_icon(Icon::Check);
    /// let rich = Button::new(RichText::from("Undo").trailing_icon(Icon::Undo), |_| Ok(()));
    /// ```
    pub fn new(
        text: impl Into<TextSource>,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self {
            caption: Caption::new(text),
            icon: None,
            action: Box::new(action),
            enabled: true,
            enablement: Permission::allowed(),
            input: ActivationState::default(),
            prepared: RefCell::new(None),
            frameless: false,
        }
    }
    /// Creates an icon-only button with a human-readable action name for inspection.
    /// The minimum width equals ControlHeight. Wrap in `Tooltip` to also show the
    /// action name on hover/focus; the name is independent of the icon's appearance.
    ///
    /// The name is a [`TextSource`]; a binding keeps the name current, for
    /// example when the application installs translated strings, and only its
    /// plain text is used. Re-describing the button can change its icon while
    /// keeping the same name binding.
    pub fn icon(
        icon: impl Into<IconSource>,
        name: impl Into<TextSource>,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self {
            icon: Some(icon.into()),
            ..Self::new(name, action)
        }
    }
    /// Creates a button that invokes a registered command, showing the command's
    /// name, as [`MenuItem::command`](crate::MenuItem::command) does for a menu
    /// row.
    ///
    /// A command whose availability is a simple binding from
    /// [`Runtime::command_enabled_signal`](crate::Runtime::command_enabled_signal)
    /// disables the button with it, and a later [`Self::enabled`] combines with
    /// that binding through [`Permission::and`]. The binding is the one the
    /// command has when the button is built: describe the button again after
    /// replacing it. A contextual predicate from
    /// [`Runtime::command_enabled`](crate::Runtime::command_enabled) depends on
    /// the target, so such a button stays enabled and a refusal arrives in the
    /// receipt as [`CommandOutcome::Disabled`](crate::CommandOutcome::Disabled).
    ///
    /// Activation, by pointer release or keyboard, captures
    /// [`EventContext::command_context`] with [`CommandSource::Button`](crate::CommandSource::Button)
    /// at that moment and queues the command through
    /// [`EventContext::invoke_command`]; the next flush revalidates the target,
    /// scope, modality and enabled state before the command runs once.
    /// `on_receipt` receives each invocation's [`CommandReceipt`](crate::CommandReceipt) on the
    /// UI thread, so a refusal or failure is reported rather than lost; a full
    /// command queue is the activation's error. Neither the command nor the
    /// handler is replayed after a failure. Same-key redescription replaces the
    /// command, the handler and this description's restrictions, as for any
    /// button. For an icon-only command button use [`Self::icon`] with an
    /// action that calls `invoke_command`.
    ///
    /// # Errors
    /// `StaleCommand` if the command is no longer registered.
    ///
    /// ```
    /// use pert_ui_runtime::{Button, CommandScope, Element, KeyChord, KeyCode, Runtime};
    /// let mut ui = Runtime::new();
    /// let play = ui.register_command("Play", CommandScope::Application,
    ///     vec![KeyChord::named(KeyCode::Space)], |_, _| Ok(()))?;
    /// let button = Button::command(&ui, &play, |receipt, _| {
    ///     let _status = receipt.status(); // pending until the next flush
    ///     Ok(())
    /// })?;
    /// ui.mount_tree(Element::new("play", button))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn command(
        ui: &crate::Runtime,
        command: &crate::CommandId,
        mut on_receipt: impl FnMut(crate::CommandReceipt, &mut EventContext) -> Result<(), RuntimeError>
        + 'static,
    ) -> Result<Self, RuntimeError> {
        let id = command.clone();
        let mut button = Self::new(ui.command_name(command)?, move |cx| {
            let context = cx.command_context(crate::CommandSource::Button);
            let receipt = cx.invoke_command(&id, context)?;
            on_receipt(receipt, cx)
        });
        if let Some(enabled) = ui.command_enabled_binding(command)? {
            button = button.enabled(enabled);
        }
        Ok(button)
    }
    /// Removes the resting fill so this button sits directly on its parent surface.
    /// Text/icon padding, minimum size, input and focus behavior remain unchanged.
    /// Hover and press use the normal transient fills; keyboard focus remains
    /// outlined. Useful for icon actions in title bars and toolbars. Default false.
    pub fn frameless(mut self) -> Self {
        self.frameless = true;
        self
    }
    /// Sets the leading icon with a themed gap, preserved across caption updates.
    /// Repeated calls replace this slot; compose multiple icons in [`RichText`].
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.leading = Some(icon.into());
        self
    }
    /// Sets the trailing icon with a themed gap, preserved across caption updates.
    /// Repeated calls replace this slot; compose multiple icons in [`RichText`].
    pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.trailing = Some(icon.into());
        self
    }
    fn display_content(&self) -> std::borrow::Cow<'_, RichText> {
        match &self.icon {
            Some(icon) => std::borrow::Cow::Owned(
                adorned_content(
                    &RichText::new().icon(icon.clone()),
                    self.caption.leading.as_ref(),
                    self.caption.trailing.as_ref(),
                )
                .into_owned(),
            ),
            None => self.caption.display(),
        }
    }
    fn minimum_width(&self, cx: &MeasureContext<'_>) -> f32 {
        cx.metric(if self.icon.is_some() {
            MetricRole::ControlHeight
        } else {
            MetricRole::ControlWidth
        })
    }
    /// Disables activation and automatic focus for this description.
    /// Equivalent to `enabled(false)`; later permissions cannot lift the denial.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation with a fixed or reactive permission. Enabled by default.
    /// Repeated calls combine restrictions; every condition must allow.
    /// Revocation cancels armed input and releases focus at the next update.
    /// Same-key redescription replaces the permission with the new description's.
    /// A binding from another runtime fails the update with [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = self.enablement.and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
}
impl Component for Button {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let drawn_changed = self.icon != next.icon
            || self.caption.leading != next.caption.leading
            || self.caption.trailing != next.caption.trailing;
        let caption_changed = self.caption.reconcile(next.caption);
        // An icon button's caption is its name, which is not drawn.
        let layout = drawn_changed || (caption_changed && next.icon.is_none());
        let paint = self.frameless != next.frameless
            || !self.enablement.same_as(&next.enablement)
            || caption_changed;
        self.icon = next.icon;
        self.action = next.action;
        self.enablement = next.enablement;
        if let Some(enabled) = self.enablement.fixed() {
            if self.enabled && !enabled {
                self.input.reset();
            }
            self.enabled = enabled;
        }
        self.frameless = next.frameless;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        // Enabled buttons own activation; disabled ones allow ancestor dragging.
        self.enabled.then_some(CursorIcon::Default)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.enabled
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        Ok(SizeLimits::at_least(Size::new(
            self.minimum_width(cx),
            cx.metric(MetricRole::ControlHeight),
        )?))
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.enabled = self.enablement.read_paint(cx)?;
        if !self.enabled && self.input.pressed() {
            self.input.reset();
            cx.cancel_input();
        }
        if self.icon.is_some() {
            // An icon button's binding names it and keeps its icon.
            self.caption.update_paint(cx)?;
        } else if self.caption.update_paint(cx)? {
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.rich_text(&self.display_content(), None, TextStyle::Body)?;
        Ok(Size::new(
            (text.size().width() + 2.0 * cx.metric(MetricRole::ControlPaddingX))
                .max(self.minimum_width(cx)),
            (text.size().height() + 2.0 * cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.prepared.borrow_mut() = Some(cx.measurement().rich_text(
            &self.display_content(),
            None,
            TextStyle::Body,
        )?);
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled {
            return Ok(InputOutcome::Ignored);
        }
        // Every event updates hover; only the primary button and motion are handled.
        if self.input.pointer(cx, event)? {
            (self.action)(cx)?;
        }
        Ok(pointer_result(event.kind()))
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !self.enabled || !activation_key(control) {
            return Ok(InputOutcome::Ignored);
        }
        let result = key_result(control, self.input.pressed(), cx.modifiers());
        if self.input.keyboard(cx, event, control) {
            (self.action)(cx)?;
        }
        Ok(result)
    }
    fn input_reset(&mut self) {
        self.input.reset();
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let background = if self.input.pressed() {
            painter.color(ColorRole::ButtonPressed)
        } else if self.input.hovered && self.enabled {
            painter.color(ColorRole::ButtonHover)
        } else {
            painter.color(ColorRole::Button)
        };
        let rect = Rect::from_size(size);
        let shape = Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?);
        if !self.frameless || (self.input.hovered && self.enabled) || self.input.pressed() {
            painter.fill_shape(rect, shape, background)?;
        }
        if self.input.focused {
            painter.focus_ring(rect, shape)?;
        }
        if let Some(text) = self.prepared.borrow().as_ref() {
            text.paint(
                painter,
                centered(size.width(), text.size().width()),
                centered(size.height(), text.size().height()),
                if self.enabled {
                    if self.input.pressed() {
                        painter.color(ColorRole::OnButtonPressed)
                    } else {
                        painter.color(ColorRole::OnButton)
                    }
                } else {
                    painter.color(ColorRole::Muted)
                },
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "text".into(),
            InspectValue::Text(self.caption.text().to_owned()),
        );
        details
            .properties
            .insert("enabled".into(), InspectValue::Bool(self.enabled));
        details.validation.interactive = self.enabled;
        details.semantics = Some(
            Semantics::new(SemanticRole::Button)
                .name(self.caption.text())
                .disabled(!self.enabled)
                .enabled_actions([SemanticAction::Activate]),
        );
        details
    }
}
