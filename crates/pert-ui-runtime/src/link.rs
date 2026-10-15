//! A text-sized navigation control that activates like a button.
use crate::activation::{ActivationState, activation_key, key_result, pointer_result};
use crate::*;
use std::cell::RefCell;

type ActivateFn = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;

/// When a [`Link`] draws its underline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum LinkUnderline {
    /// Underline only while the pointer is over the enabled link. The default,
    /// for links that stand alone in a layout and are recognizable by their
    /// color and placement.
    #[default]
    Hover,
    /// Underline whenever the link is enabled, for links that sit among
    /// ordinary text, where color alone may not set them apart.
    Always,
}

/// A navigation control presented as link text, such as "Show in mixer" or
/// "Go to settings", whose destination is the application's.
///
/// A link activates exactly like [`Button`]: a matching primary press and
/// release inside activates once, release outside, Escape or context loss
/// cancels, unmodified Enter and Space activate on the matched release and ignore OS
/// repeats, and whichever of the pointer and a key arms first owns the gesture.
/// The callback runs on the UI thread in the event phase, may write bindings
/// and propagates its error. A typical destination is a reveal through
/// [`EventContext::request_reveal`].
///
/// Only the presentation differs from a button. A link is text-sized, in body
/// typography by default, with no button fill, padding or minimum width: its
/// size is the text's extent plus [`MetricRole::FocusWidth`] on every side,
/// which keeps the focus ring off the glyphs. The text uses
/// [`ColorRole::Link`] and is underlined while hovered, or always with
/// [`LinkUnderline::Always`]. A press shows a [`ColorRole::SurfaceHover`]
/// fill, keyboard focus the usual [`ColorRole::FocusRing`] ring, and an enabled
/// link shows [`CursorIcon::Pointer`]. A disabled link is drawn in
/// [`ColorRole::Muted`] without an underline, is not a Tab stop and leaves
/// pointer input to its ancestors.
///
/// The text is laid out on one line, as a button's caption is. A link inside a
/// paragraph of rich text, wrapping with it, is not provided.
///
/// Inspection reports [`SemanticRole::Link`] named by the text, with
/// [`SemanticAction::Activate`] while enabled.
///
/// ```
/// use pert_ui_runtime::{Element, Link, LinkUnderline, Runtime, SemanticRole};
///
/// let mut ui = Runtime::new();
/// let opened = ui.signal(false);
/// let target = opened.clone();
/// ui.mount_tree(Element::new(
///     "settings-link",
///     Link::new("Go to settings", move |_| target.set(true).map(|_| ()))
///         .underline(LinkUnderline::Always),
/// ))?;
/// let snapshot = ui.inspect();
/// let link = snapshot
///     .components
///     .iter()
///     .find(|item| item.key == "settings-link")
///     .and_then(|item| item.details.semantics.as_ref())
///     .expect("a link reports semantics");
/// assert_eq!(link.role, SemanticRole::Link);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the link under the same key applies its text or text binding,
/// adornments, style, underline, enablement and action; hover and an armed activation are
/// retained while it stays enabled.
#[must_use]
pub struct Link {
    caption: Caption,
    action: ActivateFn,
    enabled: bool,
    enablement: Permission,
    underline: LinkUnderline,
    style: TextStyle,
    input: ActivationState,
    prepared: RefCell<Option<TextLayout>>,
}
impl Link {
    /// Creates an enabled link showing `text`, whose callback expresses the
    /// navigation.
    ///
    /// The text is a [`TextSource`]: fixed plain or rich content, such as text
    /// with an icon, or a binding the link follows. Its plain text names it in
    /// inspection; the underline spans the whole content. Explicit icon slots
    /// stay when the text changes.
    pub fn new(
        text: impl Into<TextSource>,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self {
            caption: Caption::new(text),
            action: Box::new(action),
            enabled: true,
            enablement: Permission::allowed(),
            underline: LinkUnderline::default(),
            style: TextStyle::Body,
            input: ActivationState::default(),
            prepared: RefCell::new(None),
        }
    }
    /// Sets the leading icon with a themed gap, preserved across caption updates.
    /// Repeated calls replace this slot; compose multiple icons in [`RichText`].
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.leading = Some(icon.into());
        self
    }
    /// Sets the trailing icon with a themed gap, such as an arrow for a link.
    /// It survives caption updates. Repeated calls replace this slot; compose
    /// multiple icons in [`RichText`].
    pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.trailing = Some(icon.into());
        self
    }
    fn display_content(&self) -> std::borrow::Cow<'_, RichText> {
        self.caption.display()
    }
    /// Sets when the underline is drawn. Default [`LinkUnderline::Hover`].
    pub fn underline(mut self, underline: LinkUnderline) -> Self {
        self.underline = underline;
        self
    }
    /// Sets the typography, for example [`TextStyle::Caption`] for a link in
    /// supporting text. Default [`TextStyle::Body`].
    pub fn style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
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
impl Component for Link {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let layout = self.style != next.style;
        let layout = self.caption.reconcile(next.caption) | layout;
        let paint = self.underline != next.underline || !self.enablement.same_as(&next.enablement);
        self.action = next.action;
        self.enablement = next.enablement;
        if let Some(enabled) = self.enablement.fixed() {
            if self.enabled && !enabled {
                self.input.reset();
            }
            self.enabled = enabled;
        }
        self.underline = next.underline;
        self.style = next.style;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        // Enabled links own activation; disabled ones leave the pointer alone.
        self.enabled.then_some(CursorIcon::Pointer)
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
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.enabled = self.enablement.read_paint(cx)?;
        if !self.enabled {
            if self.input.pressed() {
                cx.cancel_input();
            }
            self.input.reset();
        }
        if self.caption.update_paint(cx)? {
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.rich_text(&self.display_content(), None, self.style)?;
        let inset = 2.0 * cx.metric(MetricRole::FocusWidth);
        Ok(Size::new(
            text.size().width() + inset,
            text.size().height() + inset,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.prepared.borrow_mut() = Some(cx.measurement().rich_text(
            &self.display_content(),
            None,
            self.style,
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
        let Some(text) = self.prepared.borrow().clone() else {
            return Ok(());
        };
        let state = if !self.enabled {
            LinkPaint::Disabled
        } else {
            LinkPaint::Enabled {
                pressed: self.input.pressed(),
                focused: self.input.focused,
                underline: self.input.hovered || self.underline == LinkUnderline::Always,
            }
        };
        paint_link(painter, size, &text, state)
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
        details.properties.insert(
            "underline".into(),
            InspectValue::Text(
                match self.underline {
                    LinkUnderline::Hover => "hover",
                    LinkUnderline::Always => "always",
                }
                .into(),
            ),
        );
        details.validation.interactive = self.enabled;
        details.semantics = Some(link_semantics(self.caption.text(), self.enabled));
        details
    }
}

/// The semantics of a link-like control: [`SemanticRole::Link`] named by its
/// text, activatable while enabled. Shared with breadcrumb segments.
pub(crate) fn link_semantics(text: &str, enabled: bool) -> Semantics {
    let semantics = Semantics::new(SemanticRole::Link)
        .name(text)
        .disabled(!enabled);
    if enabled {
        semantics.enabled_actions([SemanticAction::Activate])
    } else {
        semantics
    }
}

/// How [`paint_link`] presents a link-like control.
#[derive(Clone, Copy)]
pub(crate) enum LinkPaint {
    /// An activatable link in its current input state.
    Enabled {
        pressed: bool,
        focused: bool,
        underline: bool,
    },
    /// A link that cannot be activated: muted, without an underline.
    Disabled,
    /// Plain text in the surface color, such as a breadcrumb's current
    /// location, which is not a link.
    Plain,
}

/// Paints link text centered in `size` with the press fill, focus ring and
/// underline that [`Link`] documents. Shared with breadcrumb segments so the
/// two link presentations cannot drift apart.
pub(crate) fn paint_link(
    painter: &mut Painter,
    size: Size,
    text: &TextLayout,
    state: LinkPaint,
) -> Result<(), DrawError> {
    let rect = Rect::from_size(size);
    let shape = Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?);
    if let LinkPaint::Enabled { pressed: true, .. } = state {
        painter.fill_shape(rect, shape, painter.color(ColorRole::SurfaceHover))?;
    }
    if let LinkPaint::Enabled { focused: true, .. } = state {
        painter.focus_ring(rect, shape)?;
    }
    let color = painter.color(match state {
        LinkPaint::Enabled { .. } => ColorRole::Link,
        LinkPaint::Disabled => ColorRole::Muted,
        LinkPaint::Plain => ColorRole::OnSurface,
    });
    let x = crate::layout::centered(size.width(), text.size().width());
    let y = crate::layout::centered(size.height(), text.size().height());
    text.paint(painter, x, y, color)?;
    if let LinkPaint::Enabled {
        underline: true, ..
    } = state
    {
        let thickness = painter.metric(MetricRole::BorderWidth);
        // One stroke width below the baseline, kept inside the allocation.
        let below = text
            .first_baseline()
            .map_or(text.size().height(), |baseline| baseline + thickness);
        let line = Rect::new(
            x,
            (y + below).min(size.height() - thickness).max(0.0),
            text.size().width(),
            thickness,
        )
        .map_err(DrawError::from)?;
        painter.fill_rect(line, color);
    }
    Ok(())
}
