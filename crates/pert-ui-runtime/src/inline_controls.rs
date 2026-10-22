//! Inline family members that are not numeric: latched toggles with their own
//! accessible name and delayed help.
use crate::activation::{ActivationState, activation_key, key_result};
use crate::choice_controls::{focus_ring, rounded};
use crate::control_binding::{ValueBinding, same_binding};
use crate::layout::centered;
use crate::popover::{HelpTimer, arrange_help};
use crate::text_fields::{InlineField, unhandled_text_key};
use crate::*;
use std::cell::RefCell;
use std::rc::Rc;

/// A latched square toggle for toolbars and channel strips, bound to a UI-local
/// `Signal<bool>` or a boolean [`Parameter`], showing an icon or a glyph of at
/// most two characters.
///
/// The control is `ControlHeight` square for icon content; text content adds
/// `ControlPaddingX`. It rests frameless like title-bar actions, fills with
/// `SurfaceHover` on hover, `ButtonPressed` with `OnButtonPressed` content
/// while pressed and, when on, `Primary` with `OnPrimary` content like a
/// checked checkbox. Primary release inside or a
/// matched unmodified Enter/Space release toggles the value once; OS repeats do not.
/// Escape, focus loss, hiding and removal cancel pending activation; an idle
/// toggle leaves Escape to View, Window and Application commands. External
/// writes repaint only subscribed controls. Re-describing the toggle under the
/// same key applies its binding, name, content and enablement, cancelling a pending
/// activation when the binding changes.
///
/// The accessible name given at construction, or by [`Self::name`], feeds
/// inspection and a delayed help surface on hover or keyboard focus, using the
/// shared 500 ms tooltip timing. The name is never drawn inside the control;
/// without an icon or glyph the control shows its first character, which suits
/// a dense strip of one-letter toggles (such as mute and solo) but reads as an
/// unfinished control on its own. In a [`Toolbar`](crate::Toolbar) give it
/// [`Self::icon`], or [`Self::glyph`] for a short abbreviation, and use the
/// standard [`Toggle`], which keeps its switch and visible label, where the name
/// must be read. A momentary action is [`Button::icon`].
/// Inspection reports the name as the `text` property and shown text content
/// as `glyph`.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, Icon, InlineToggle, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let muted = ui.signal(false);
/// let toggle = ui.mount_tree(Element::new("mute",
///     InlineToggle::new(muted.clone(), "Mute").icon(Icon::VolumeOff)))?;
/// ui.focus(&toggle)?;
/// ui.key_input(KeyCode::Space, KeyAction::Press)?;
/// ui.key_input(KeyCode::Space, KeyAction::Release)?;
/// assert!(muted.get()?);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[must_use]
pub struct InlineToggle {
    on_change: Option<crate::choice_controls::BooleanHandler>,
    value: Rc<dyn ValueBinding<bool>>,
    /// The described name, unless it follows the parameter's.
    name_source: TextSource,
    /// The plain name shown by inspection and the help child.
    name: Rc<RefCell<String>>,
    /// The name follows the parameter's name. The glyph follows the shown name
    /// unless an icon or glyph was chosen.
    follows_name: bool,
    explicit_content: bool,
    content: RichText,
    icon_only: bool,
    checked: bool,
    enabled: bool,
    enablement: Permission,
    editable: bool,
    revision: Revision,
    input: ActivationState,
    help: HelpTimer,
    prepared: RefCell<Option<TextLayout>>,
}
description_debug!(InlineToggle);
/// The first `count` grapheme clusters of `text`, so an emoji or a letter
/// with combining marks stays whole.
fn leading_clusters(text: &str, count: usize) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    text.graphemes(true).take(count).collect()
}
fn first_glyph(name: &str) -> RichText {
    RichText::from(leading_clusters(name, 1))
}
impl InlineToggle {
    /// Creates an enabled toggle named `name`, showing the name's first
    /// character (grapheme cluster) until [`Self::icon`] or [`Self::glyph`] replaces it.
    ///
    /// The name is a [`TextSource`]; a binding keeps the name, the help and an
    /// undecorated glyph current, for example for a translated name. Only its
    /// plain text is used.
    pub fn new(value: impl Into<Signal<bool>>, name: impl Into<TextSource>) -> Self {
        let value: Signal<bool> = value.into();
        Self::bound(Rc::new(value), name.into())
    }
    /// Creates a toggle bound to a boolean parameter and named by it.
    ///
    /// The parameter's name feeds inspection, the delayed help and, unless an
    /// icon or glyph is set, the shown character. Activation follows
    /// [`Toggle::for_parameter`]: one begin/commit pair per changed value at the
    /// revision observed at press, disabled while the parameter is read-only,
    /// unavailable, faulted, or has pending/unknown/disconnected remote authority.
    /// [`Self::name`] replaces the parameter's name, and [`Self::icon`] or
    /// [`Self::glyph`] decorates the control without replacing it.
    pub fn for_parameter(parameter: Parameter<bool>) -> Self {
        let mut toggle = Self::bound(Rc::new(parameter), TextSource::default());
        toggle.follows_name = true;
        toggle
    }
    fn bound(value: Rc<dyn ValueBinding<bool>>, name_source: TextSource) -> Self {
        let name = name_source
            .fixed()
            .map(RichText::plain_text)
            .unwrap_or_default();
        Self {
            on_change: None,
            value,
            content: first_glyph(&name),
            name_source,
            name: Rc::new(RefCell::new(name)),
            follows_name: false,
            explicit_content: false,
            icon_only: false,
            checked: false,
            enabled: true,
            enablement: Permission::allowed(),
            editable: true,
            revision: Revision::INITIAL,
            input: ActivationState::default(),
            help: HelpTimer::default(),
            prepared: RefCell::new(None),
        }
    }
    /// Replaces the accessible name with a [`TextSource`]: fixed text or a
    /// binding the name follows; only its plain text is used. On a toggle made
    /// with [`Self::for_parameter`] the name then no longer follows the
    /// parameter's name. Without an icon or glyph the shown character follows
    /// the new name.
    pub fn name(mut self, name: impl Into<TextSource>) -> Self {
        let name = name.into();
        let text = name.fixed().map(RichText::plain_text).unwrap_or_default();
        if !self.explicit_content {
            self.content = first_glyph(&text);
        }
        *self.name.borrow_mut() = text;
        self.name_source = name;
        self.follows_name = false;
        self
    }
    /// Shows an icon instead of text; the control becomes ControlHeight square.
    pub fn icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.content = RichText::new().icon(icon);
        self.icon_only = true;
        self.explicit_content = true;
        self
    }
    /// Shows up to two characters of text; longer input keeps its first two.
    /// A character is a grapheme cluster, so an emoji or an accented letter
    /// built from several code points counts once and is never cut.
    pub fn glyph(mut self, glyph: impl Into<String>) -> Self {
        let glyph = leading_clusters(&glyph.into(), 2);
        self.content = RichText::from(glyph);
        self.icon_only = false;
        self.explicit_content = true;
        self
    }
    /// Disables user edits and automatic focus; the external value remains visible.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts edits and focus with a fixed or reactive permission; allowed by default.
    /// Repeated calls combine restrictions. Revocation cancels armed input at update;
    /// parameter editability remains an additional restriction. Redescription replaces
    /// this description's permission. External values remain visible.
    /// A binding from another runtime fails the update with [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = self.enablement.and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
    /// Reports successful user writes or remote submissions as [`Toggle::on_change`].
    /// Application writes and authority responses do not call it. Failures do not
    /// roll back or replay the write; redescription replaces the handler.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, bool) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
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
impl Component for InlineToggle {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            on_change,
            value,
            name_source,
            name: _,
            follows_name,
            explicit_content,
            content,
            icon_only,
            enabled: _,
            enablement,
            checked: _,
            editable: _,
            revision: _,
            input: _,
            help: _,
            prepared: _,
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
        // The help child reads the retained name cell, so write into it. A name
        // following the parameter or a binding is refreshed by update.
        let name_changed =
            follows_name != self.follows_name || !self.name_source.same_as(&name_source);
        self.name_source = name_source;
        if !follows_name && name_changed {
            // A new binding is empty until the coming update reads it.
            let name = self
                .name_source
                .fixed()
                .map(RichText::plain_text)
                .unwrap_or_default();
            if *self.name.borrow() != name {
                *self.name.borrow_mut() = name;
                self.help.restart();
                invalidation = invalidation.max(Some(Invalidation::Paint));
            }
        }
        // Without explicit content the glyph follows the shown name.
        let content = if explicit_content {
            content
        } else {
            first_glyph(&self.name.borrow())
        };
        if content != self.content || icon_only != self.icon_only {
            self.content = content;
            self.icon_only = icon_only;
            invalidation = Some(Invalidation::Layout);
        }
        self.follows_name = follows_name;
        self.explicit_content = explicit_content;
        invalidation
    }
    fn take_children(&mut self) -> Vec<Element> {
        vec![shared_help_child(self.name.clone())]
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
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.interactive()
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.interactive().then_some(CursorIcon::Default)
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
        if reading.revision != self.revision {
            self.revision = reading.revision;
            if self.input.pressed() {
                self.input.reset();
                cx.cancel_input();
            }
        }
        let name = if self.follows_name {
            reading.name
        } else if self.name_source.is_bound() {
            // The name is not drawn; a changed glyph requests layout below.
            Some(self.name_source.read_paint(cx)?.plain_text())
        } else {
            None
        };
        if let Some(name) = name
            && *self.name.borrow() != name
        {
            if !self.explicit_content {
                self.content = first_glyph(&name);
                cx.request_layout();
            }
            *self.name.borrow_mut() = name;
            self.help.restart();
        }
        self.input.focused = cx.focused();
        let active = cx.hovered() || cx.focus_within();
        self.help.update(cx, active)
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let band = cx.metric(MetricRole::ControlHeight);
        let text = cx.rich_text(&self.content, None, TextStyle::Body)?;
        let width = if self.icon_only {
            band
        } else {
            (text.size().width() + 2.0 * cx.metric(MetricRole::ControlPaddingX)).max(band)
        };
        Ok(Size::new(width, band)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        *self.prepared.borrow_mut() = Some(cx.measurement().rich_text(
            &self.content,
            None,
            TextStyle::Body,
        )?);
        if cx.is_empty() {
            return Ok(());
        }
        arrange_help(cx, size)
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
            || matches!(event.kind(), PointerKind::Scroll(_) | PointerKind::Pinch(_))
        {
            return Ok(InputOutcome::Ignored);
        }
        if self.input.pointer(cx, event)? {
            self.toggle(cx)?;
        }
        Ok(InputOutcome::Handled)
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
        let rect = Rect::from_size(size);
        let shape = rounded(p.metric(MetricRole::CornerRadius))?;
        let pressed = self.input.pressed();
        let enabled = self.interactive();
        let fill = if pressed {
            Some(ColorRole::ButtonPressed)
        } else if self.checked {
            Some(if enabled {
                ColorRole::Primary
            } else {
                ColorRole::Muted
            })
        } else if self.input.hovered && enabled {
            Some(ColorRole::SurfaceHover)
        } else {
            None
        };
        if let Some(role) = fill {
            p.fill_shape(rect, shape, p.color(role))?;
        }
        if let Some(text) = self.prepared.borrow().as_ref() {
            let color = p.color(if !enabled {
                if self.checked {
                    ColorRole::Surface
                } else {
                    ColorRole::Muted
                }
            } else if pressed {
                ColorRole::OnButtonPressed
            } else if self.checked {
                ColorRole::OnPrimary
            } else {
                ColorRole::OnSurface
            });
            text.paint(
                p,
                crate::Point::new(
                    centered(size.width(), text.size().width()),
                    centered(size.height(), text.size().height()),
                )?,
                color,
            )?;
        }
        if self.input.focused {
            focus_ring(p, rect)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        let name = self.name.borrow().clone();
        d.properties
            .insert("text".into(), InspectValue::Text(name.clone()));
        if let Some(glyph) = self.content.as_plain().filter(|_| !self.icon_only) {
            d.properties
                .insert("glyph".into(), InspectValue::Text(glyph.to_owned()));
        }
        d.properties
            .insert("checked".into(), InspectValue::Bool(self.checked));
        d.properties
            .insert("enabled".into(), InspectValue::Bool(self.interactive()));
        d.properties
            .insert("help_open".into(), InspectValue::Bool(self.help.shown()));
        d.validation.interactive = self.interactive();
        d.semantics = Some(
            Semantics::new(SemanticRole::Switch)
                .name(name)
                .value(SemanticValue::Bool(self.checked))
                .disabled(!self.interactive())
                .enabled_actions([SemanticAction::Toggle]),
        );
        d
    }
}

/// Help text read from state shared with its owner, for controls whose
/// accessible name follows the hovered or active part. The owner restarts its
/// help timer when the subject changes, so this label re-reads on presentation.
pub(crate) struct SharedText {
    source: Rc<RefCell<String>>,
    text: String,
    prepared: RefCell<Option<TextLayout>>,
}
impl Component for SharedText {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.text = self.source.borrow().clone();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        Ok(cx.text(&self.text, a.width(), TextStyle::Body)?.size())
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.prepared.borrow_mut() = Some(cx.measurement().text(
            &self.text,
            Some(size.width()),
            TextStyle::Body,
        )?);
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if let Some(text) = self.prepared.borrow().as_ref() {
            text.paint(p, crate::Point::ZERO, p.color(ColorRole::OnSurface))?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties
            .insert("text".into(), InspectValue::Text(self.text.clone()));
        d
    }
}
/// Builds a help child whose text follows shared owner state.
pub(crate) fn shared_help_child(source: Rc<RefCell<String>>) -> Element {
    Element::new(
        "help",
        crate::popover::transient_panel(Element::new(
            "text",
            SharedText {
                source,
                text: String::new(),
                prepared: RefCell::new(None),
            },
        ))
        .padding(MetricRole::ControlPaddingX),
    )
}

type Reader = Box<dyn FnMut(&mut UpdateContext) -> Result<(String, Option<String>), RuntimeError>>;

/// A read-only caption/value/unit row for status bars and toolbars, showing
/// text from a binding or a formatted typed value.
///
/// The row shares the inline numeric presentation: a secondary
/// [`TextStyle::Caption`] label, a [`TextStyle::Monospace`] value and a literal
/// unit in the value style on one baseline, at ControlHeight.
/// The slot reserves the width of [`Self::width_hint`], or of the current text
/// when no hint is given, so live values do not move their neighbors. Clicking
/// the value or pressing Enter/F2 opens read-only text mode for selection and
/// copy, as read-only inline numbers do; nothing else interacts and nothing is
/// written. In text mode [`TextCommand::Copy`] through
/// [`Runtime::text_command`](crate::Runtime::text_command) copies the
/// selection like the chord, and the other commands are unavailable. Keys that
/// type text are claimed in text mode, as a read-only [`TextField`] claims
/// them, so a command bound to a plain character key does not run there. Text
/// mode also stops unmodified Enter, Up/Down and Page keys at the row, while
/// their modified chords reach command bindings. Outside text mode the row is
/// not a text editor. Resting focus schedules no caret work.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Element, InlineReadout, NumberFormat, NumberInputError, Runtime};
/// let mut ui = Runtime::new();
/// let position = ui.signal(String::from("001:01:000"));
/// let load = ui.signal(0.42_f64);
/// let percent = NumberFormat::new("100",
///     |v: f64| format!("{:.0}", v * 100.0),
///     |_| Err(NumberInputError::InvalidNumber)).unit(|_| "%".into());
/// ui.mount_tree(Element::new("status", pert_ui::Row::new([
///     Element::new("position", InlineReadout::new(position.read_only())
///         .label("Pos").width_hint("000:00:000")),
///     Element::new("load", InlineReadout::formatted(load.read_only(), percent).label("CPU")),
/// ])))?;
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
///
/// Re-describing the readout under the same key adopts its reader, hint, style,
/// label and literal unit; the next update reads again, and text mode ends only
/// if the shown text changes.
#[must_use]
pub struct InlineReadout {
    field: InlineField<()>,
    reader: Reader,
    literal_unit: String,
    text: String,
    hint: Option<String>,
    /// The value typography chosen with [`Self::style`], if any.
    style: Option<TextStyle>,
    focused: bool,
}
description_debug!(InlineReadout);
impl InlineReadout {
    fn with_reader(reader: Reader) -> Self {
        let mut field = InlineField::new(true);
        field.set_read_only();
        Self {
            field,
            reader,
            literal_unit: String::new(),
            text: String::new(),
            hint: None,
            style: None,
            focused: false,
        }
    }
    /// Shows a bound string verbatim.
    pub fn new(source: impl Into<ReadSignal<String>>) -> Self {
        let source: ReadSignal<String> = source.into();
        Self::with_reader(Box::new(move |cx| Ok((cx.read_paint(&source)?, None))))
    }
    /// Shows a bound value through a [`NumberFormat`]'s display callback and
    /// optional dynamic unit; the format's width example reserves the slot
    /// unless [`Self::width_hint`] overrides it. Parsing is never invoked.
    pub fn formatted<T: Copy + 'static>(
        source: impl Into<ReadSignal<T>>,
        format: NumberFormat<T>,
    ) -> Self {
        let source: ReadSignal<T> = source.into();
        let hint = format.hint.clone();
        let mut readout = Self::with_reader(Box::new(move |cx| {
            let value = cx.read_paint(&source)?;
            let unit = format.unit.as_ref().map(|unit| unit(value));
            Ok(((format.display)(value), unit))
        }));
        readout.hint = Some(hint);
        readout
    }
    /// Sets the leading caption, empty by default; its plain text is the
    /// readout's accessible name.
    ///
    /// The caption is a [`TextSource`]: fixed plain or rich text, or a
    /// binding the row follows. A binding is read in the row's update and
    /// remeasures the row when it changes; describing the row again with the
    /// same binding keeps the text already read.
    pub fn label(mut self, label: impl Into<TextSource>) -> Self {
        self.field.caption = Caption::new(label);
        self
    }
    /// Sets a literal unit after the value; a formatted dynamic unit overrides
    /// it even when that callback returns an empty suffix.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.literal_unit = unit.into();
        self.field.unit = self.literal_unit.clone();
        self
    }
    /// Reserves the value slot for this example text instead of the current text.
    pub fn width_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
    /// Overrides the value and unit typography, including read-only text mode.
    /// Defaults to [`TextStyle::Monospace`]; the caption keeps
    /// [`TextStyle::Caption`].
    pub fn style(mut self, style: TextStyle) -> Self {
        self.field.set_style(style);
        self.style = Some(style);
        self
    }
    fn hint(&self) -> String {
        self.hint.clone().unwrap_or_else(|| self.text.clone())
    }
    fn begin(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.field.editing {
            let text = self.text.clone();
            self.field.begin_text(cx, (), text)?;
        }
        Ok(())
    }
}
impl Component for InlineReadout {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            field,
            reader,
            literal_unit,
            hint,
            style,
            text: _,
            focused: _,
        } = next;
        // A reader is a callback without identity: adopt it and let update
        // re-read, which leaves text mode only if the shown text changes.
        self.reader = reader;
        let mut invalidation = Some(Invalidation::Paint);
        let caption_changed = self.field.caption.reconcile(field.caption);
        if caption_changed
            || hint != self.hint
            || style != self.style
            || literal_unit != self.literal_unit
        {
            self.hint = hint;
            self.style = style;
            self.field.set_style(style.unwrap_or(TextStyle::Monospace));
            self.literal_unit = literal_unit;
            invalidation = Some(Invalidation::Layout);
        }
        invalidation
    }
    fn focusable(&self) -> bool {
        // In text mode focus belongs to the row's child editor.
        !self.field.drafting()
    }
    fn child_visible(&self, key: &str) -> bool {
        self.field.child_visible(key)
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (!self.field.drafting() && self.field.local_value(point)).then_some(CursorIcon::Text)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let (text, unit) = (self.reader)(cx)?;
        let unit = unit.unwrap_or_else(|| self.literal_unit.clone());
        if unit != self.field.unit {
            self.field.unit = unit;
            cx.request_layout();
        }
        if text != self.text {
            if self.field.editing {
                self.field.stop_text();
            }
            if self.hint.is_none() {
                cx.request_layout();
            }
            self.text = text;
        }
        self.field.display_value((), self.text.clone())?;
        self.focused = cx.focused();
        self.field.update(cx)
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        self.field
            .measure(cx, &self.field.hint(&self.hint()), false)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let hint = self.field.hint(&self.hint());
        self.field.arrange(cx.measurement(), size, &hint, false)?;
        self.field.place(cx)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.field.editing {
            return self.field.text_pointer(cx, e);
        }
        if e.kind() == PointerKind::Press(PointerButton::Primary)
            && self.field.hit_value(cx, e.position())
        {
            self.begin(cx)?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.field.set_hover(cx, hovered);
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(e);
        if control == KeyCode::Escape && e.transition == KeyTransition::Pressed {
            // Only text mode has something to cancel; an idle readout leaves
            // Escape to View, Window and Application commands.
            if !self.field.editing {
                return Ok(InputOutcome::Ignored);
            }
            self.field.stop_text();
            cx.release_pointer();
            cx.request_focus();
            cx.invalidate(Invalidation::Paint);
            return Ok(InputOutcome::Handled);
        }
        if self.field.editing {
            // The read-only editor leaves Enter and the vertical keys here;
            // there is nothing to confirm or step, so the unmodified keys stop
            // at the row and modified chords go on to command bindings.
            return Ok(
                if keys.chord_modifiers(e) == Modifiers::default()
                    && matches!(
                        control,
                        KeyCode::Enter
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                            | KeyCode::PageUp
                            | KeyCode::PageDown
                    )
                {
                    InputOutcome::Handled
                } else {
                    InputOutcome::Ignored
                },
            );
        }
        if e.transition == KeyTransition::Pressed && matches!(control, KeyCode::Enter | KeyCode::F2)
        {
            self.begin(cx)?;
            return Ok(InputOutcome::Handled);
        }
        Ok(InputOutcome::Ignored)
    }
    fn text_key_event(
        &mut self,
        cx: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        keys: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        if self.field.drafting() {
            // The keys the row's editor leaves to it, answered in the text
            // phase so capture bindings never take them.
            let handled = self.keyboard_event(cx, input.editing_event(transition), keys)?
                == InputOutcome::Handled;
            return Ok(if handled {
                TextKeyOutcome::Handled
            } else {
                TextKeyOutcome::Ignored
            });
        }
        if !self.field.editing {
            return Ok(TextKeyOutcome::NotAnEditor);
        }
        if self.keyboard_event(cx, input.editing_event(transition), keys)? == InputOutcome::Handled
        {
            Ok(TextKeyOutcome::Handled)
        } else {
            // Read-only text mode claims typed keys as a read-only TextField does.
            Ok(unhandled_text_key(cx, input, transition))
        }
    }
    fn text_command(
        &mut self,
        cx: &mut EventContext,
        command: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        self.field.command(cx, command)
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if !self.field.drafting() {
            self.field.stop_text();
            cx.invalidate(Invalidation::Paint);
        }
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if !within && self.field.drafting() {
            self.field.stop_text();
            cx.invalidate(Invalidation::Paint);
        }
        Ok(())
    }
    fn input_reset(&mut self) {
        self.field.stop_text();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        self.field.paint(p)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        self.field.inspect(&mut d);
        d.properties
            .insert("text".into(), InspectValue::Text(self.text.clone()));
        d.validation.interactive = true;
        d.semantics = Some(
            Semantics::new(SemanticRole::Readout)
                .name(self.field.label())
                .value(SemanticValue::Text(self.text.clone()))
                .read_only(true),
        );
        d
    }
}
