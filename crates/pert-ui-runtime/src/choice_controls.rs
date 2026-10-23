//! Boolean and typed discrete selection controls.
use crate::activation::{ActivationState, activation_key, key_result};
use crate::control_binding::{ValueBinding, same_binding};
use crate::inline_controls::shared_help_child;
use crate::layout::centered;
use crate::popover::{HelpTimer, arrange_help};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// One stable typed choice, independent of its label or presentation order.
/// Values must be unique within a control. Disabled choices remain visible and
/// may represent an external selection, but cannot be chosen through input.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct Choice<T> {
    value: T,
    label: String,
    content: RichText,
    caption: RichText,
    leading: Option<IconSource>,
    trailing: Option<IconSource>,
    enabled: bool,
}
impl<T> Choice<T> {
    /// Creates an enabled option. Use durable values, not display indices.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self::from_rich(value, RichText::from(label.into()))
    }
    /// Creates an icon-only option with an accessible name that is not drawn.
    pub fn icon(value: T, icon: impl Into<IconSource>, name: impl Into<String>) -> Self {
        let mut choice = Self::from_rich(value, RichText::new().icon(icon));
        choice.label = name.into();
        choice
    }
    pub(crate) fn from_rich(value: T, content: RichText) -> Self {
        Self {
            value,
            label: content.plain_text(),
            caption: content.clone(),
            content,
            leading: None,
            trailing: None,
            enabled: true,
        }
    }
    fn compose(&mut self) {
        self.content = crate::controls::adorned_content(
            &self.caption,
            self.leading.as_ref(),
            self.trailing.as_ref(),
        )
        .into_owned();
    }
    /// Replaces the caption and inspection label, preserving icon slots and value.
    /// Compose multiple icons in rich content when more than one per side is needed.
    pub fn rich_label(mut self, content: RichText) -> Self {
        self.set_content(content);
        self
    }
    /// Replaces the leading icon slot using the shared mapping and themed gap.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.leading = Some(icon.into());
        self.compose();
        self
    }
    /// Replaces the trailing icon slot using the shared mapping and themed gap.
    pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.trailing = Some(icon.into());
        self.compose();
        self
    }
    /// Styled display content, independent of the plain inspection label and value.
    pub fn content(&self) -> &RichText {
        &self.content
    }
    /// Excludes this option from pointer and keyboard selection.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
    /// Stable application value associated with this option.
    pub fn value(&self) -> &T {
        &self.value
    }
    /// User-facing label; not an identity.
    pub fn label(&self) -> &str {
        &self.label
    }
    /// Whether user input may select this option.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub(crate) fn set_content(&mut self, content: RichText) {
        self.label = content.plain_text();
        self.caption = content;
        self.compose();
    }
    pub(crate) fn set_enabled(&mut self, enabled: bool) -> bool {
        let changed = self.enabled != enabled;
        self.enabled = enabled;
        changed
    }
}

/// Invalid discrete selection configuration.
///
/// Constructors return it directly; a dynamic option list that becomes invalid
/// fails the control's update with [`RuntimeError::Choice`](crate::RuntimeError::Choice)
/// holding it, and the last valid list stays in place.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ChoiceError {
    /// A menu action key is empty or contains only whitespace.
    #[error("empty menu action key")]
    EmptyValue,
    /// Two options have equal application values, or two sibling menu items
    /// share an action key. The positions are zero-based indices into the
    /// list, or the sibling items, in their given order, so an application
    /// value need not be printable to name the pair.
    #[error("choices {first} and {second} have equal values")]
    DuplicateValue {
        /// The earlier option's position.
        first: usize,
        /// The later option's position, which repeats the earlier value.
        second: usize,
    },
}
pub(crate) fn validate<T: Eq>(choices: &[Choice<T>]) -> Result<(), ChoiceError> {
    for (second, choice) in choices.iter().enumerate() {
        if let Some(first) = choices[..second]
            .iter()
            .position(|old| old.value == choice.value)
        {
            return Err(ChoiceError::DuplicateValue { first, second });
        }
    }
    Ok(())
}

pub(crate) fn rounded(radius: f32) -> Result<Shape, DrawError> {
    Ok(Shape::Rounded(Length::new(radius)?))
}
/// Shared editable-field frame: a Surface fill, or SurfaceHover on an enabled
/// hovered field, with a rounded outline: the border width in Outline at rest,
/// the focus ring when focused, and the focus width in Error when
/// invalid, which takes precedence.
pub(crate) fn field_frame(
    p: &mut Painter,
    rect: Rect,
    focused: bool,
    invalid: bool,
    hovered: bool,
) -> Result<(), DrawError> {
    let shape = rounded(p.metric(MetricRole::CornerRadius))?;
    p.fill_shape(
        rect,
        shape,
        p.color(if hovered {
            ColorRole::SurfaceHover
        } else {
            ColorRole::Surface
        }),
    )?;
    let (role, width) = if invalid {
        (ColorRole::Error, MetricRole::FocusWidth)
    } else if focused {
        return p.focus_ring(rect, shape);
    } else {
        (ColorRole::Outline, MetricRole::BorderWidth)
    };
    p.stroke_shape(rect, shape, Length::new(p.metric(width))?, p.color(role))
}
/// The focus ring on the standard rounded control silhouette.
pub(crate) fn focus_ring(p: &mut Painter, rect: Rect) -> Result<(), DrawError> {
    p.focus_ring(rect, rounded(p.metric(MetricRole::CornerRadius))?)
}

pub(crate) type BooleanHandler =
    Box<dyn FnMut(&mut EventContext, bool) -> Result<(), RuntimeError>>;

struct BooleanControl {
    on_change: Option<BooleanHandler>,
    /// The label and its icon slots.
    caption: Caption,
    /// The label shows the parameter's name until replaced explicitly.
    follows_name: bool,
    value: Rc<dyn ValueBinding<bool>>,
    checked: bool,
    enabled: bool,
    enablement: Permission,
    editable: bool,
    revision: Revision,
    switch: bool,
    input: ActivationState,
    text: RefCell<Option<TextLayout>>,
}
impl BooleanControl {
    fn new(label: impl Into<TextSource>, value: Rc<dyn ValueBinding<bool>>, switch: bool) -> Self {
        Self {
            on_change: None,
            caption: Caption::new(label),
            follows_name: false,
            value,
            checked: false,
            enabled: true,
            enablement: Permission::allowed(),
            editable: true,
            revision: Revision::INITIAL,
            switch,
            input: ActivationState::default(),
            text: RefCell::new(None),
        }
    }
    fn interactive(&self) -> bool {
        self.enabled && self.editable
    }
    fn toggle(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let next = !self.value.current()?;
        if self.value.choose(cx, next, self.revision)?
            && let Some(handler) = &mut self.on_change
        {
            handler(cx, next)?;
        }
        Ok(())
    }
}
impl Component for BooleanControl {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            on_change,
            caption,
            follows_name,
            value,
            enabled: _,
            enablement,
            switch: _,
            checked: _,
            editable: _,
            revision: _,
            input: _,
            text: _,
        } = next;
        self.on_change = on_change;
        let mut invalidation = None;
        if !same_binding(&self.value, &value) {
            // A pending activation belongs to the old target: cancel it rather
            // than let its release toggle the new one.
            self.input.reset();
            self.value = value;
            self.revision = Revision::INITIAL;
            invalidation = Some(Invalidation::Paint);
        }
        if !self.enablement.same_as(&enablement) {
            if let Some(enabled) = enablement.fixed() {
                self.enabled = enabled;
                if !enabled {
                    self.input.reset();
                }
            }
            self.enablement = enablement;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        let caption_changed = if follows_name && self.follows_name {
            // A label following the parameter's name is refreshed by update;
            // only the icon slots come from the description.
            let slots = self.caption.leading != caption.leading
                || self.caption.trailing != caption.trailing;
            self.caption.leading = caption.leading;
            self.caption.trailing = caption.trailing;
            slots
        } else if follows_name != self.follows_name {
            self.caption = caption;
            true
        } else {
            self.caption.reconcile(caption)
        };
        if caption_changed {
            invalidation = Some(Invalidation::Layout);
        }
        self.follows_name = follows_name;
        invalidation
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.interactive()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        let reading = self.value.read(cx)?;
        self.enabled = enabled;
        self.checked = reading.value;
        self.editable = reading.editable;
        if !self.interactive() && self.input.pressed() {
            self.input.reset();
            cx.cancel_input();
        }
        // A changed revision cancels a pending activation instead of retargeting it.
        if reading.revision != self.revision {
            self.revision = reading.revision;
            if self.input.pressed() {
                self.input.reset();
                cx.cancel_input();
            }
        }
        let relabeled = if self.follows_name {
            reading
                .name
                .is_some_and(|name| self.caption.set_content(name.into()))
        } else {
            self.caption.update(cx)?
        };
        if relabeled {
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.rich_text(&self.caption.display(), None, TextStyle::Body)?;
        let icon = cx.metric(MetricRole::IconSize) * if self.switch { 2.0 } else { 1.0 };
        Ok(Size::new(
            icon + cx.metric(MetricRole::IconGap)
                + text.size().width()
                + 2.0 * cx.metric(MetricRole::ControlPaddingX),
            cx.metric(MetricRole::ControlHeight)
                .max(text.size().height() + 2.0 * cx.metric(MetricRole::ControlPaddingY)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.text.borrow_mut() = Some(cx.measurement().rich_text(
            &self.caption.display(),
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
        if !self.interactive() {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(event.kind(), PointerKind::Press(b) | PointerKind::Release(b) if b != PointerButton::Primary)
        {
            return Ok(InputOutcome::Ignored);
        }
        if self.input.pointer(cx, event)? {
            self.toggle(cx)?;
        }
        Ok(
            if matches!(
                event.kind(),
                PointerKind::Scroll(_)
                    | PointerKind::Pinch(_)
                    | PointerKind::Press(PointerButton::Secondary)
                    | PointerKind::Release(PointerButton::Secondary)
            ) {
                InputOutcome::Ignored
            } else {
                InputOutcome::Handled
            },
        )
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !self.interactive() || !activation_key(control) {
            return Ok(InputOutcome::Ignored);
        }
        let result = key_result(control, self.input.pressed(), cx.modifiers());
        if self.input.keyboard(cx, event, control) {
            self.toggle(cx)?;
        }
        Ok(result)
    }
    fn input_reset(&mut self) {
        self.input.reset();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let enabled = self.interactive();
        let rect = Rect::from_size(size);
        if self.input.hovered && enabled {
            p.fill_shape(
                rect,
                rounded(p.metric(MetricRole::CornerRadius))?,
                p.color(ColorRole::SurfaceHover),
            )?;
        }
        let side = p.metric(MetricRole::IconSize).min(size.height());
        let x = p.metric(MetricRole::ControlPaddingX);
        let y = (size.height() - side) / 2.0;
        let width = side * if self.switch { 2.0 } else { 1.0 };
        let color = p.color(if !enabled {
            ColorRole::Muted
        } else if self.checked {
            ColorRole::Primary
        } else {
            ColorRole::Outline
        });
        let stroke = (p.metric(MetricRole::BorderWidth) * 2.0).min(side / 2.0);
        if self.switch {
            p.fill_shape(Rect::new(x, y, width, side)?, rounded(side / 2.0)?, color)?;
            // The thumb reads as a raised knob on the accent and as a hole on the outline.
            p.fill_shape(
                Rect::new(
                    x + if self.checked { side + 2.0 } else { 2.0 },
                    y + 2.0,
                    (side - 4.0).max(0.0),
                    (side - 4.0).max(0.0),
                )?,
                rounded(side / 2.0)?,
                p.color(if self.checked && enabled {
                    ColorRole::OnPrimary
                } else {
                    ColorRole::Surface
                }),
            )?;
        } else {
            let square = Rect::new(x, y, side, side)?;
            let shape = rounded(p.metric(MetricRole::CornerRadius).min(side / 4.0))?;
            if self.checked {
                p.fill_shape(square, shape, color)?;
                p.icon(
                    Icon::Check,
                    Rect::new(
                        x + 1.0,
                        y + 1.0,
                        (side - 2.0).max(0.0),
                        (side - 2.0).max(0.0),
                    )?,
                    p.color(if enabled {
                        ColorRole::OnPrimary
                    } else {
                        ColorRole::Surface
                    }),
                )?;
            } else {
                p.stroke_shape(square, shape, Length::new(stroke)?, color)?;
            }
        }
        if let Some(text) = self.text.borrow().as_ref() {
            text.paint(
                p,
                crate::Point::new(
                    x + width + p.metric(MetricRole::IconGap),
                    centered(size.height(), text.size().height()),
                )?,
                p.color(if enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
        }
        if self.input.focused {
            focus_ring(p, rect)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties.insert(
            "text".into(),
            InspectValue::Text(self.caption.text().to_owned()),
        );
        d.properties
            .insert("checked".into(), InspectValue::Bool(self.checked));
        d.properties
            .insert("enabled".into(), InspectValue::Bool(self.interactive()));
        d.validation.interactive = self.interactive();
        d.semantics = Some(
            Semantics::new(if self.switch {
                SemanticRole::Switch
            } else {
                SemanticRole::Checkbox
            })
            .name(self.caption.text())
            .value(SemanticValue::Bool(self.checked))
            .disabled(!self.interactive())
            .enabled_actions([SemanticAction::Toggle]),
        );
        d
    }
}
macro_rules! boolean_widget {
    ($name:ident, $switch:expr, $doc:literal) => {
        #[doc = $doc]
        ///
        /// Binds either a shared UI-local `Signal<bool>` ([`Self::new`]) or a
        /// boolean [`Parameter`] ([`Self::for_parameter`]). Enabled by default. The
        /// entire label activates on primary release inside or matched unmodified Enter/Space
        /// release; OS repeats do not toggle. Escape, focus loss, hiding and removal
        /// cancel pending activation; an idle control leaves Escape to View,
        /// Window and Application commands. External changes repaint only
        /// subscribed controls. Re-describing the control under the same key
        /// applies its label, binding and enabled state, cancelling a pending
        /// activation when the binding changes.
        ///
        #[doc = concat!(
            "```\n",
            "# use pert_ui_runtime as pert_ui;\n",
            "use pert_ui::{", stringify!($name), ", Element, KeyAction, KeyCode, Runtime};\n",
            "let mut ui = Runtime::new();\n",
            "let enabled = ui.signal(false);\n",
            "let control = ui.mount_tree(Element::new(\"enabled\", ",
            stringify!($name), "::new(enabled.clone(), \"Enabled\")))?;\n",
            "ui.focus(&control)?;\n",
            "ui.key_input(KeyCode::Space, KeyAction::Press)?;\n",
            "ui.key_input(KeyCode::Space, KeyAction::Release)?;\n",
            "assert!(enabled.get()?);\n",
            "# Ok::<(), Box<dyn std::error::Error>>(())\n",
            "```",
        )]
        #[must_use]
        pub struct $name(BooleanControl);
        description_debug!($name);
        impl $name {
            /// Creates a labeled control sharing the supplied local value.
            ///
            /// The label is a [`TextSource`]: fixed plain or rich content, or a
            /// binding the label follows, such as a translation.
            pub fn new(value: impl Into<Signal<bool>>, label: impl Into<TextSource>) -> Self {
                let value: Signal<bool> = value.into();
                Self(BooleanControl::new(label, Rc::new(value), $switch))
            }
            /// Creates a control bound to a boolean parameter, labeled with the
            /// parameter's name until [`Self::label`] replaces it. The label
            /// follows a changed specification name; icon slots from
            /// [`Self::leading_icon`] and [`Self::trailing_icon`] decorate it
            /// without replacing it.
            ///
            /// Each activation that changes the value is one begin/commit pair at
            /// the revision observed when the press started; a revision change in
            /// between cancels the activation instead of retargeting it. The
            /// control is disabled while the parameter is read-only, unavailable
            /// or has a delivery fault, or while remote authority is pending, unknown
            /// or disconnected. Keep the `ParameterHost` alive for as
            /// long as editing is available.
            pub fn for_parameter(parameter: Parameter<bool>) -> Self {
                let mut control = BooleanControl::new("", Rc::new(parameter), $switch);
                control.follows_name = true;
                Self(control)
            }
            /// Replaces the label with a [`TextSource`]: fixed plain or rich
            /// content, or a binding the label follows. On a control made with
            /// [`Self::for_parameter`] the label then no longer follows the
            /// parameter's name; a bound replacement, such as an application
            /// translation, follows its binding instead. Icon slots are kept.
            pub fn label(mut self, label: impl Into<TextSource>) -> Self {
                self.0.caption.set_source(label.into());
                self.0.follows_name = false;
                self
            }
            /// Sets the leading icon with a themed gap between the box and the
            /// label. Repeated calls replace this slot; it stays when the label
            /// changes, including a parameter's name.
            pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
                self.0.caption.leading = Some(icon.into());
                self
            }
            /// Sets the trailing icon with a themed gap after the label.
            /// Repeated calls replace this slot; it stays when the label
            /// changes, including a parameter's name.
            pub fn trailing_icon(mut self, icon: impl Into<IconSource>) -> Self {
                self.0.caption.trailing = Some(icon.into());
                self
            }
            /// Reports each successful user write or remote proposal submission,
            /// synchronously after it occurs. Application writes, refused edits and
            /// authority responses do not call it. Failure propagates without rollback
            /// or replay. Redescription replaces the handler, including removing it.
            pub fn on_change(mut self, handler: impl FnMut(&mut EventContext, bool)
                -> Result<(), RuntimeError> + 'static) -> Self {
                self.0.on_change = Some(Box::new(handler));
                self
            }
            /// Permanently disables user edits and automatic focus; external values remain visible.
            pub fn disabled(self) -> Self {
                self.enabled(false)
            }
            /// Restricts edits and focus with a fixed or reactive permission; allowed by default.
            /// Repeated calls combine restrictions. Revocation cancels armed input at update;
            /// parameter editability remains an additional restriction. Redescription replaces
            /// this description's permission. External values remain visible.
            /// A binding from another runtime fails the update with [`RuntimeError::ForeignRuntime`].
            pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
                self.0.enablement = self.0.enablement.and(permission);
                if let Some(enabled) = self.0.enablement.fixed() {
                    self.0.enabled = enabled;
                }
                self
            }
        }
        impl Component for $name {
            fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
                self.0.reconcile(next.0)
            }
            fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
                self.0.hover_changed(cx, hovered);
            }
            fn focusable(&self) -> bool {
                self.0.focusable()
            }
            fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
                self.0.update(cx)
            }
            fn measure(
                &self,
                cx: &MeasureContext<'_>,
                a: Constraints,
            ) -> Result<Size, RuntimeError> {
                self.0.measure(cx, a)
            }
            fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
                self.0.arrange(cx, s)
            }
            fn pointer_input(
                &mut self,
                cx: &mut EventContext,
                e: PointerEvent,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.pointer_input(cx, e)
            }
            fn keyboard_event(
                &mut self,
                cx: &mut EventContext,
                e: KeyEvent,
                k: &KeyboardState,
            ) -> Result<InputOutcome, RuntimeError> {
                self.0.keyboard_event(cx, e, k)
            }
            fn input_reset(&mut self) {
                self.0.input_reset();
            }
            fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
                self.0.paint(p)
            }
            fn inspect(&self) -> ComponentDetails {
                self.0.inspect()
            }
        }
    };
}
boolean_widget!(
    Checkbox,
    false,
    "A two-state checkbox with a clickable label and visible keyboard focus."
);
boolean_widget!(
    Toggle,
    true,
    "An on/off switch with a clickable label and visible keyboard focus."
);

/// A vertical, single-selection group using stable application values.
///
/// Binds a UI-local `Signal<T>` ([`Self::new`] or [`Self::bound`]) or a choice [`Parameter`]
/// ([`RadioGroup::for_parameter`]), whose specification supplies the options,
/// labels and order. One Tab stop covers the group. Arrows wrap through enabled options; Home/End
/// choose the first/last enabled option. Unmodified Space/Enter select the focused option on
/// matched release. Pointer activation requires release over the pressed value.
/// An external value absent from the choices leaves every option unchecked; the
/// framework never silently rewrites application state. Empty groups cannot focus.
/// [`Self::new`] supplies fixed options; [`Self::bound`] follows a dynamic list;
/// [`RadioGroup::for_parameter`] uses the parameter specification exclusively.
/// Dynamic list changes cancel pending activation, so removal/recreation cannot retarget input.
/// Re-describing the group under the same key applies its options, binding,
/// [`on_change`](Self::on_change) handler and enabled state the same way.
/// Escape is consumed only while it cancels an armed activation.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, Element, KeyAction, KeyCode, RadioGroup, Runtime};
/// use std::{cell::Cell, rc::Rc};
/// let mut ui = Runtime::new();
/// let size = ui.signal("m");
/// let changes = Rc::new(Cell::new(0));
/// let counted = changes.clone();
/// let group = RadioGroup::new(
///     size.clone(),
///     [Choice::new("s", "Small"), Choice::new("m", "Medium"), Choice::new("l", "Large")],
/// )?
/// .on_change(move |_, _| {
///     counted.set(counted.get() + 1);
///     Ok(())
/// });
/// let group = ui.mount_tree(Element::new("size", group))?;
/// ui.focus(&group)?;
/// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?;
/// assert_eq!(size.get()?, "l");
/// assert_eq!(changes.get(), 1);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct RadioGroup<T> {
    segmented: bool,
    inline: bool,
    segments: RefCell<Vec<(f32, f32)>>,
    segment_height: Cell<f32>,
    help: HelpTimer,
    help_text: Rc<RefCell<String>>,
    hovered: Option<T>,
    on_change: Option<SelectionHandler<T>>,
    choices: Vec<Choice<T>>,
    source: Option<ReadSignal<Vec<Choice<T>>>>,
    value: Rc<dyn ValueBinding<T>>,
    editable: bool,
    revision: Revision,
    name: Option<String>,
    selected: Option<T>,
    active: Option<T>,
    pressed: Option<T>,
    enabled: bool,
    enablement: Permission,
    input: ActivationState,
    row_height: Cell<f32>,
    texts: RefCell<Vec<TextLayout>>,
}
description_debug!(RadioGroup<T>);
impl<T: Clone + Eq + 'static> RadioGroup<T> {
    /// Creates an enabled group.
    ///
    /// # Errors
    /// Returns [`ChoiceError::DuplicateValue`] when two choices have equal values,
    /// including disabled ones.
    pub fn new(
        value: impl Into<Signal<T>>,
        choices: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<T> = value.into();
        let choices: Vec<_> = choices.into_iter().collect();
        validate(&choices)?;
        Ok(Self::with_binding(Rc::new(value), choices))
    }
    fn with_binding(value: Rc<dyn ValueBinding<T>>, choices: Vec<Choice<T>>) -> Self {
        Self {
            segmented: false,
            inline: false,
            segments: RefCell::new(Vec::new()),
            segment_height: Cell::new(0.0),
            help: HelpTimer::default(),
            help_text: Rc::new(RefCell::new(String::new())),
            hovered: None,
            on_change: None,
            choices,
            source: None,
            value,
            editable: true,
            revision: Revision::INITIAL,
            name: None,
            selected: None,
            active: None,
            pressed: None,
            enabled: true,
            enablement: Permission::allowed(),
            input: ActivationState::default(),
            row_height: Cell::new(0.0),
            texts: RefCell::new(Vec::new()),
        }
    }
    /// Creates a group over a reactive choice list, read at update rather than construction.
    /// The value and list must belong to the mounted runtime; a foreign binding
    /// fails update with [`RuntimeError::ForeignRuntime`]. Duplicate values fail
    /// with [`RuntimeError::Choice`] holding [`ChoiceError::DuplicateValue`],
    /// preserving the last valid list (empty before the first valid update).
    /// Valid label/order changes retain selection by value and cancel armed input.
    /// Re-describing the same key replaces the binding and choice source.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, Element, RadioGroup, Runtime};
    /// let mut ui = Runtime::new();
    /// let value = ui.signal(1);
    /// let choices = ui.signal(vec![Choice::new(1, "One")]);
    /// ui.mount_tree(Element::new("choice", RadioGroup::bound(value, choices.read_only())))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn bound(
        value: impl Into<Signal<T>>,
        choices: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<T> = value.into();
        let choices: ReadSignal<Vec<Choice<T>>> = choices.into();
        let mut group = Self::with_binding(Rc::new(value), Vec::new());
        group.source = Some(choices);
        group
    }
    /// Disables editing and automatic focus for the entire group.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation and automatic focus with a fixed or reactive permission.
    /// Repeated calls combine restrictions; a fixed denial cannot be lifted on
    /// this description. Parameter editability and each option's availability
    /// remain additional restrictions. Revocation at update cancels armed input;
    /// same-key redescription replaces the permission and fixed denial cancels
    /// immediately. Bindings must belong to the mounted runtime.
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = self.enablement.and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
    /// Calls a UI-thread handler after a local write or remote submission,
    /// with the rules of [`SegmentedControl::on_change`]. By default there is
    /// no handler.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, &T) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
    fn select_value(&mut self, cx: &mut EventContext, value: T) -> Result<(), RuntimeError> {
        if self.value.choose(cx, value.clone(), self.revision)?
            && let Some(handler) = &mut self.on_change
        {
            handler(cx, &value)?;
        }
        // A refused or pending choice leaves the value, and so the update that
        // would repaint, unchanged; the focused option still moves.
        if self.active.as_ref() != Some(&value) {
            self.active = Some(value);
            cx.invalidate(Invalidation::Paint);
        }
        Ok(())
    }
    fn enabled_value(&self, value: &T) -> bool {
        self.choices.iter().any(|c| &c.value == value && c.enabled)
    }
    /// Preferred segment widths: inline squares for icon-only choices, padded
    /// text otherwise; ordinary segmented controls share the widest segment.
    fn segment_widths(&self, cx: &MeasureContext<'_>) -> Result<Vec<f32>, RuntimeError> {
        let band = cx.metric(MetricRole::ControlHeight);
        let pad = cx.metric(MetricRole::ControlPaddingX);
        let mut widths = Vec::with_capacity(self.choices.len());
        let mut widest: f32 = 0.0;
        for c in &self.choices {
            let t = cx.rich_text(c.content(), None, TextStyle::Body)?;
            let width = if self.inline && c.content().plain_text().trim().is_empty() {
                band
            } else {
                t.size().width() + 2.0 * pad
            };
            widest = widest.max(width);
            widths.push(width);
        }
        if !self.inline {
            widths.iter_mut().for_each(|w| *w = widest);
        }
        Ok(widths)
    }
    fn help_subject(&self) -> String {
        let value = self.hovered.as_ref().or(self.active.as_ref());
        value
            .and_then(|v| self.choices.iter().find(|c| &c.value == v))
            .map(|c| c.label.clone())
            .unwrap_or_default()
    }
    fn hit(&self, cx: &EventContext, e: PointerEvent) -> Option<T> {
        if !cx.pointer_inside() || self.row_height.get() <= 0.0 {
            return None;
        }
        let local = cx.to_local(e.position())?;
        let index = if self.segmented {
            self.segments
                .borrow()
                .iter()
                .position(|(start, width)| local.x() >= *start && local.x() < start + width)?
        } else {
            (local.y() / self.row_height.get()).floor() as usize
        };
        self.choices
            .get(index)
            .filter(|c| c.enabled)
            .map(|c| c.value.clone())
    }
}
impl RadioGroup<ChoiceId> {
    /// Creates a group bound to a choice parameter. The specification supplies
    /// the options, labels and order, and replacing it updates them, cancelling
    /// pending input; selection follows the identity, never an index.
    ///
    /// Choosing a different option is one begin/commit pair at the revision
    /// observed when the interaction started; a revision change in between
    /// cancels the activation instead of retargeting it. Choosing the selected
    /// option is a no-op. The group is disabled while the parameter is read-only,
    /// unavailable, faulted, or has pending/unknown/disconnected remote authority.
    /// The specification is the only option source. A parameter-bound control
    /// cannot be given a separate list; use [`Self::bound`] with a local signal
    /// for an application-owned dynamic list.
    ///
    /// ```compile_fail
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, ChoiceId, Parameter, RadioGroup, ReadSignal};
    /// fn conflicting(parameter: Parameter<ChoiceId>, options: ReadSignal<Vec<Choice<ChoiceId>>>) {
    ///     let _ = RadioGroup::for_parameter(parameter).choices(options);
    /// }
    /// ```
    pub fn for_parameter(parameter: Parameter<ChoiceId>) -> Self {
        Self::with_binding(Rc::new(parameter), Vec::new())
    }
}
impl<T: Clone + Eq + 'static> Component for RadioGroup<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            segmented,
            inline,
            on_change,
            choices,
            source,
            value,
            enabled: _,
            enablement,
            segments: _,
            segment_height: _,
            help: _,
            help_text: _,
            hovered: _,
            editable: _,
            revision: _,
            name: _,
            selected: _,
            active: _,
            pressed: _,
            input: _,
            row_height: _,
            texts: _,
        } = next;
        // The wrapper type fixes the presentation; it never changes here.
        debug_assert!(segmented == self.segmented && inline == self.inline);
        self.on_change = on_change;
        let mut invalidation = None;
        let mut cancel = false;
        if !same_binding(&self.value, &value) {
            // Pending input belongs to the old target; its option list and
            // name come from the new binding at the next update.
            self.value = value;
            self.revision = Revision::INITIAL;
            self.name = None;
            self.selected = None;
            cancel = true;
            invalidation = Some(Invalidation::Layout);
        }
        if !ReadSignal::same_optional(&self.source, &source) {
            self.source = source;
            cancel = true;
            invalidation = Some(Invalidation::Layout);
        }
        // Fixed choices apply unless a binding or a dynamic source supplies them.
        if self.source.is_none() && !self.value.supplies_options() && self.choices != choices {
            self.choices = choices;
            cancel = true;
            invalidation = Some(Invalidation::Layout);
        }
        if !self.enablement.same_as(&enablement) {
            if let Some(enabled) = enablement.fixed() {
                self.enabled = enabled;
                cancel |= !enabled;
            }
            self.enablement = enablement;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        if cancel {
            self.input.reset();
            self.pressed = None;
            self.hovered = None;
        }
        invalidation
    }
    fn take_children(&mut self) -> Vec<Element> {
        if self.inline {
            vec![shared_help_child(self.help_text.clone())]
        } else {
            Vec::new()
        }
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
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hovered.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.enabled && self.editable && self.choices.iter().any(|c| c.enabled)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        let reading = self.value.read(cx)?;
        let mut choices = reading.options;
        if let Some(source) = &self.source {
            // Only signal-bound descriptions have an independent choice source.
            choices = Some(cx.read(source)?);
        }
        if let Some(choices) = choices {
            validate(&choices)?;
            if self.choices != choices {
                self.choices = choices;
                self.hovered = None;
                cx.cancel_input();
                self.input.reset();
                self.pressed = None;
                cx.request_layout();
            }
        }
        // A changed revision cancels pending activation instead of retargeting it.
        if reading.revision != self.revision {
            self.revision = reading.revision;
            if self.pressed.take().is_some() || self.input.pressed() {
                self.input.reset();
                cx.cancel_input();
            }
        }
        self.enabled = enabled;
        self.editable = reading.editable;
        if !self.enabled || !self.editable {
            if self.input.pressed() || self.pressed.is_some() {
                cx.cancel_input();
            }
            self.input.reset();
            self.pressed = None;
            self.hovered = None;
        }
        self.name = reading.name;
        let selected = reading.value;
        if self.selected.as_ref() != Some(&selected) {
            self.active = Some(selected.clone());
        }
        self.selected = Some(selected);
        if self.active.as_ref().is_none_or(|v| !self.enabled_value(v)) {
            self.active = self
                .choices
                .iter()
                .find(|c| c.enabled)
                .map(|c| c.value.clone());
        }
        self.input.focused = cx.focused();
        if self.inline {
            let subject = self.help_subject();
            if *self.help_text.borrow() != subject {
                *self.help_text.borrow_mut() = subject;
                self.help.restart();
            }
            let active = cx.hovered() || cx.focus_within();
            self.help.update(cx, active)?;
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let mut width: f32 = 0.0;
        let mut height = cx.metric(MetricRole::ControlHeight);
        for c in &self.choices {
            let t = cx.rich_text(c.content(), None, TextStyle::Body)?;
            width = width.max(t.size().width());
            height = height.max(t.size().height() + 2.0 * cx.metric(MetricRole::ControlPaddingY));
        }
        if self.segmented {
            let widths = self.segment_widths(cx)?;
            return Ok(Size::new(
                widths.iter().sum::<f32>(),
                if self.inline {
                    cx.metric(MetricRole::ControlHeight)
                } else {
                    height
                },
            )?);
        }
        Ok(Size::new(
            width
                + cx.metric(MetricRole::IconSize)
                + cx.metric(MetricRole::IconGap)
                + 2.0 * cx.metric(MetricRole::ControlPaddingX),
            height * self.choices.len() as f32,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() && !self.inline {
            return Err(LayoutError::InvalidChild.into());
        }
        let text: Vec<_> = self
            .choices
            .iter()
            .map(|c| {
                cx.measurement()
                    .rich_text(c.content(), None, TextStyle::Body)
            })
            .collect::<Result<_, _>>()?;
        self.row_height.set(text.iter().fold(
            cx.measurement().metric(MetricRole::ControlHeight),
            |h, t| {
                h.max(
                    t.size().height() + 2.0 * cx.measurement().metric(MetricRole::ControlPaddingY),
                )
            },
        ));
        let widths = if self.inline {
            self.segment_widths(cx.measurement())?
        } else {
            vec![size.width() / self.choices.len().max(1) as f32; self.choices.len()]
        };
        let mut x = 0.0;
        *self.segments.borrow_mut() = widths
            .into_iter()
            .map(|width| {
                let start = x;
                x += width;
                (start, width)
            })
            .collect();
        self.segment_height.set(size.height());
        *self.texts.borrow_mut() = text;
        if cx.is_empty() {
            return Ok(());
        }
        arrange_help(cx, size)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.enabled || !self.editable {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(e.kind(), PointerKind::Press(b) | PointerKind::Release(b) if b != PointerButton::Primary)
        {
            return Ok(InputOutcome::Ignored);
        }
        if self.segmented {
            let hovered = if e.kind() == PointerKind::Leave {
                None
            } else {
                self.hit(cx, e)
            };
            if self.hovered != hovered {
                self.hovered = hovered;
                cx.invalidate(Invalidation::Paint);
            }
        }
        let activate = self.input.pointer(cx, e)?;
        match e.kind() {
            PointerKind::Press(PointerButton::Primary) => self.pressed = self.hit(cx, e),
            PointerKind::Release(PointerButton::Primary) => {
                if let Some(value) = self
                    .pressed
                    .take()
                    .filter(|v| activate && self.hit(cx, e).as_ref() == Some(v))
                {
                    self.select_value(cx, value)?;
                }
            }
            PointerKind::Scroll(_) | PointerKind::Pinch(_) => return Ok(InputOutcome::Ignored),
            _ => {}
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
        if !self.focusable() {
            return Ok(InputOutcome::Ignored);
        }
        if activation_key(control) {
            let result = key_result(
                control,
                self.input.pressed() || self.pressed.is_some(),
                cx.modifiers(),
            );
            if self.input.keyboard(cx, e, control)
                && let Some(value) = self.active.clone()
            {
                self.select_value(cx, value)?;
            }
            if control == KeyCode::Escape {
                self.pressed = None;
            }
            return Ok(result);
        }
        // Modified navigation keys are left to command bindings.
        if !matches!(
            control,
            KeyCode::ArrowUp
                | KeyCode::ArrowLeft
                | KeyCode::ArrowDown
                | KeyCode::ArrowRight
                | KeyCode::Home
                | KeyCode::End
        ) || keys.chord_modifiers(e) != Modifiers::default()
        {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            let enabled: Vec<_> = self
                .choices
                .iter()
                .filter(|c| c.enabled)
                .map(|c| c.value.clone())
                .collect();
            if enabled.is_empty() {
                return Ok(InputOutcome::Handled);
            }
            let index = self
                .active
                .as_ref()
                .and_then(|v| enabled.iter().position(|x| x == v))
                .unwrap_or(0);
            let next = match control {
                KeyCode::Home => 0,
                KeyCode::End => enabled.len() - 1,
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    (index + enabled.len() - 1) % enabled.len()
                }
                _ => (index + 1) % enabled.len(),
            };
            self.select_value(cx, enabled[next].clone())?;
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.input.reset();
        self.pressed = None;
        self.hovered = None;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        if self.segmented {
            return self.paint_segments(p);
        }
        let h = self.row_height.get();
        let side = p.metric(MetricRole::IconSize).min(h);
        let x = p.metric(MetricRole::ControlPaddingX);
        for (i, (c, text)) in self
            .choices
            .iter()
            .zip(self.texts.borrow().iter())
            .enumerate()
        {
            let top = i as f32 * h;
            let enabled = self.enabled && self.editable && c.enabled;
            let checked = self.selected.as_ref() == Some(&c.value);
            let rect = Rect::new(x, top + (h - side) / 2.0, side, side)?;
            let color = p.color(if !enabled {
                ColorRole::Muted
            } else if checked {
                ColorRole::Primary
            } else {
                ColorRole::Outline
            });
            let stroke = (p.metric(MetricRole::BorderWidth) * 2.0).min(side / 2.0);
            p.stroke_shape(rect, rounded(side / 2.0)?, Length::new(stroke)?, color)?;
            if checked {
                let dot = side / 2.0;
                p.fill_shape(
                    Rect::new(x + (side - dot) / 2.0, top + (h - dot) / 2.0, dot, dot)?,
                    rounded(dot / 2.0)?,
                    color,
                )?;
            }
            text.paint(
                p,
                crate::Point::new(
                    x + side + p.metric(MetricRole::IconGap),
                    top + centered(h, text.size().height()),
                )?,
                p.color(if enabled {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                }),
            )?;
            if self.input.focused && self.active.as_ref() == Some(&c.value) {
                focus_ring(p, Rect::new(0.0, top, size.width(), h)?)?;
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.validation.interactive = self.focusable();
        d.properties.insert(
            "options".into(),
            InspectValue::Number(self.choices.len() as f64),
        );
        let chosen = self
            .choices
            .iter()
            .position(|c| self.selected.as_ref() == Some(&c.value));
        if let Some(c) = chosen.map(|index| &self.choices[index]) {
            d.properties
                .insert("selected".into(), InspectValue::Text(c.label.clone()));
        }
        d.semantics = Some(
            Semantics::new(if self.segmented {
                SemanticRole::SegmentedChoice
            } else {
                SemanticRole::RadioGroup
            })
            .name(self.name.as_deref().unwrap_or_default())
            .value(SemanticValue::Choice {
                selected: chosen,
                count: self.choices.len(),
                label: chosen.map(|index| self.choices[index].label.clone()),
            })
            .items(self.choices.iter().enumerate().map(|(index, choice)| {
                SemanticItem::new(choice.label.as_str())
                    .selected(chosen == Some(index))
                    .disabled(!self.enabled || !self.editable || !choice.enabled)
                    .action(SemanticAction::Select)
            }))
            .disabled(!self.focusable())
            .enabled_actions([SemanticAction::Select]),
        );
        if self.inline {
            d.properties
                .insert("help_open".into(), InspectValue::Bool(self.help.shown()));
            d.properties.insert(
                "help_text".into(),
                InspectValue::Text(self.help_text.borrow().clone()),
            );
        }
        d
    }
}

type SelectionHandler<T> = Box<dyn FnMut(&mut EventContext, &T) -> Result<(), RuntimeError>>;

impl<T: Clone + Eq + 'static> RadioGroup<T> {
    fn paint_segments(&self, p: &mut Painter) -> Result<(), DrawError> {
        let segments = self.segments.borrow();
        let height = self.segment_height.get();
        let total = segments.last().map_or(0.0, |(x, w)| x + w);
        let group = Rect::new(0.0, 0.0, total, height)?;
        let shape = rounded(p.metric(MetricRole::CornerRadius))?;
        for (index, (choice, text)) in self
            .choices
            .iter()
            .zip(self.texts.borrow().iter())
            .enumerate()
        {
            let Some(&(x, width)) = segments.get(index) else {
                break;
            };
            let border = p.metric(MetricRole::BorderWidth).min(width).min(height);
            let rect = Rect::new(x, 0.0, width, height)?;
            let selected = self.selected.as_ref() == Some(&choice.value);
            let enabled = self.enabled && self.editable && choice.enabled;
            // Clip one shared rounded silhouette per segment: exterior corners
            // follow buttons while internal edges remain straight and joined.
            p.with_clip(rect, |p| -> Result<(), DrawError> {
                p.fill_shape(
                    group,
                    shape,
                    p.color(if selected {
                        ColorRole::TabSelected
                    } else if enabled && self.hovered.as_ref() == Some(&choice.value) {
                        ColorRole::SurfaceHover
                    } else {
                        ColorRole::Tab
                    }),
                )?;
                if index > 0 {
                    p.with_clip(Rect::new(x, 0.0, border, height)?, |p| {
                        p.fill_shape(group, shape, p.color(ColorRole::Outline))
                    })?;
                }
                text.paint(
                    p,
                    crate::Point::new(
                        x + centered(width, text.size().width()),
                        centered(height, text.size().height()),
                    )?,
                    p.color(if enabled {
                        ColorRole::OnTab
                    } else {
                        ColorRole::Muted
                    }),
                )?;
                if self.input.focused && self.active.as_ref() == Some(&choice.value) {
                    p.focus_ring(group, shape)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}

/// A horizontal group of joined buttons with one selected application value.
///
/// Equal-width segments share straight internal edges with no gap. The group's
/// exterior corners use the button theme's radius. Selection is the tab
/// palette's selected fill alone; the group carries no underline or baseline,
/// unlike [`Tabs`](crate::Tabs), whose selected title does. Preferred width
/// fits the widest label in every segment; constrained labels clip within their
/// segment. The entire group is one Tab stop. Arrows wrap over enabled choices;
/// Home/End select endpoints. Pointer selection requires release over the pressed
/// value; unmodified Space/Enter activate on matched release. Escape/focus loss/hiding/removal
/// cancel pending activation. [`Self::new`] binds UI-local selection;
/// [`SegmentedControl::for_parameter`] binds a choice parameter's edit protocol.
///
/// Values must be unique. An absent external value leaves all segments unselected;
/// disabled choices may remain selected externally. Empty/all-disabled groups do
/// not focus. Dynamic choice changes cancel pending input before replacing labels
/// or order. Like other retained controls, bindings update only their dependents;
/// no polling or timers are added. Handles and callbacks stay on the UI thread.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, Element, KeyAction, KeyCode, Runtime, SegmentedControl};
/// let mut ui = Runtime::new();
/// let mode = ui.signal("dark");
/// let group = ui.mount_tree(Element::new("mode", SegmentedControl::new(
///     mode.clone(), [Choice::new("dark", "Dark"), Choice::new("light", "Light")],
/// )?))?;
/// ui.focus(&group)?;
/// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
/// assert_eq!(mode.get()?, "light");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the control under the same key follows [`RadioGroup`].
#[must_use]
pub struct SegmentedControl<T>(RadioGroup<T>);
description_debug!(SegmentedControl<T>);
impl<T: Clone + Eq + 'static> SegmentedControl<T> {
    /// Creates an enabled group.
    ///
    /// # Errors
    /// Returns [`ChoiceError::DuplicateValue`] when two choices have equal values,
    /// including disabled ones.
    pub fn new(
        value: impl Into<Signal<T>>,
        choices: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<T> = value.into();
        let mut group = RadioGroup::new(value, choices)?;
        group.segmented = true;
        Ok(Self(group))
    }
    /// Creates joined segments over reactive choices, with the validation,
    /// ownership, cancellation and redescription rules of [`RadioGroup::bound`].
    pub fn bound(
        value: impl Into<Signal<T>>,
        choices: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<T> = value.into();
        let choices: ReadSignal<Vec<Choice<T>>> = choices.into();
        let mut group = RadioGroup::bound(value, choices);
        group.segmented = true;
        Self(group)
    }
    /// Disables input/focus for the group while still displaying external selection.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation and automatic focus with the composition,
    /// cancellation and redescription rules of [`RadioGroup::enabled`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.0 = self.0.enabled(permission);
        self
    }
    /// Calls a UI-thread handler after a local write or remote submission.
    /// A remote submission keeps the proposed selection visible and disables
    /// editing until its outcome. Its base stays unchanged; submission does not
    /// prove external acceptance. The handler runs once with the proposal, not
    /// again for authority correction or rejection. External writes and reselecting
    /// the same value do not call it. A failing handler cannot roll back an already
    /// written value or retained proposal; its error propagates through normal
    /// input recovery, without rollback or event replay. The borrowed value lives
    /// for this call only. The callback must return promptly and may update local
    /// application settings; it is not safe for time-critical producers. By
    /// default there is no handler.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, &T) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.0.on_change = Some(Box::new(handler));
        self
    }
}
impl SegmentedControl<ChoiceId> {
    /// Creates joined segments bound to a choice parameter, with the rules of
    /// [`RadioGroup::for_parameter`]. `on_change` follows
    /// [`Self::on_change`]: local commit or remote submission.
    pub fn for_parameter(parameter: Parameter<ChoiceId>) -> Self {
        let mut group = RadioGroup::for_parameter(parameter);
        group.segmented = true;
        Self(group)
    }
}
impl<T: Clone + Eq + 'static> Component for SegmentedControl<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.0.reconcile(next.0)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.0.hover_changed(cx, hovered);
    }
    fn focusable(&self) -> bool {
        self.0.focusable()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.update(cx)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.0.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.0.arrange(cx, s)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.pointer_input(cx, e)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        k: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.keyboard_event(cx, e, k)
    }
    fn input_reset(&mut self) {
        self.0.input_reset();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.0.paint(p)
    }
    fn inspect(&self) -> ComponentDetails {
        self.0.inspect()
    }
}

/// An inline exclusive selection with square icon segments for tool modes and
/// editor views, sharing the [`SegmentedControl`] contract.
///
/// Each choice without text, such as one created with [`Choice::icon`], becomes
/// a ControlHeight square; choices with text keep their padded width. Segments
/// are joined without gaps and exterior corners only, the whole group is one Tab
/// stop, arrows wrap over enabled choices, Home/End select endpoints and pointer
/// selection requires release over the pressed value. Every choice carries a
/// name: the hovered choice's name, or the active one's under keyboard focus, is
/// presented as delayed help with the shared tooltip timing, so icon-only
/// segments remain nameable. Values, uniqueness, dynamic choices, `on_change`
/// and external-write rules follow [`SegmentedControl`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, Element, Icon, InlineChoice, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let tool = ui.signal("select");
/// let group = ui.mount_tree(Element::new("tools", InlineChoice::new(
///     tool.clone(),
///     [
///         Choice::icon("select", Icon::Edit, "Select"),
///         Choice::icon("erase", Icon::Delete, "Erase"),
///     ],
/// )?))?;
/// ui.focus(&group)?;
/// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
/// assert_eq!(tool.get()?, "erase");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the group under the same key follows [`RadioGroup`].
#[must_use]
pub struct InlineChoice<T>(RadioGroup<T>);
description_debug!(InlineChoice<T>);
impl<T: Clone + Eq + 'static> InlineChoice<T> {
    /// Creates an enabled inline group.
    ///
    /// # Errors
    /// Returns [`ChoiceError::DuplicateValue`] when two choices have equal values,
    /// including disabled ones.
    pub fn new(
        value: impl Into<Signal<T>>,
        choices: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<T> = value.into();
        let mut group = RadioGroup::new(value, choices)?;
        group.segmented = true;
        group.inline = true;
        Ok(Self(group))
    }
    /// Creates an inline group over reactive choices, with the validation,
    /// ownership, cancellation and redescription rules of [`RadioGroup::bound`].
    pub fn bound(
        value: impl Into<Signal<T>>,
        choices: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<T> = value.into();
        let choices: ReadSignal<Vec<Choice<T>>> = choices.into();
        let mut group = RadioGroup::bound(value, choices);
        group.segmented = true;
        group.inline = true;
        Self(group)
    }
    /// Disables input/focus for the group while still displaying external selection.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation and automatic focus with the composition,
    /// cancellation and redescription rules of [`RadioGroup::enabled`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.0 = self.0.enabled(permission);
        self
    }
    /// Calls a UI-thread handler after a local write or remote submission,
    /// with the rules of [`SegmentedControl::on_change`].
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, &T) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.0.on_change = Some(Box::new(handler));
        self
    }
}
impl InlineChoice<ChoiceId> {
    /// Creates an inline group bound to a choice parameter, with the rules of
    /// [`RadioGroup::for_parameter`]. Option labels name the segments and their
    /// help; icon-only presentation needs the signal-bound constructor.
    pub fn for_parameter(parameter: Parameter<ChoiceId>) -> Self {
        let mut group = RadioGroup::for_parameter(parameter);
        group.segmented = true;
        group.inline = true;
        Self(group)
    }
}
impl<T: Clone + Eq + 'static> Component for InlineChoice<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.0.reconcile(next.0)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take_children()
    }
    fn child_visible(&self, key: &str) -> bool {
        self.0.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.0.child_surface(key)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        r: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.0.dismiss_surface(cx, r)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.0.hover_changed(cx, hovered);
    }
    fn focusable(&self) -> bool {
        self.0.focusable()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.update(cx)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.0.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.0.arrange(cx, s)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.pointer_input(cx, e)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        k: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.keyboard_event(cx, e, k)
    }
    fn input_reset(&mut self) {
        self.0.input_reset();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.0.paint(p)
    }
    fn inspect(&self) -> ComponentDetails {
        self.0.inspect()
    }
}
