//! A retained color sample, optionally activatable to open an editor.
use crate::activation::{ActivationState, activation_key, key_result, pointer_result};
use crate::*;

type ActivateFn = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;

// Fixed neutral checkerboard behind translucent colors, as gamma-encoded
// #cccccc and #999999 in linear light. They do not follow the theme, so a
// translucent color looks the same over them in light and dark mode.
const CHECKER_LIGHT: Color = Color::linear_rgb(0.603_8, 0.603_8, 0.603_8);
const CHECKER_DARK: Color = Color::linear_rgb(0.318_5, 0.318_5, 0.318_5);

/// Most checker cells along either axis of one rectangle.
const MAX_CHECKER_CELLS: f32 = 64.0;

/// Fills `rect` with the fixed neutral checkerboard that shows translucency,
/// in cells half `MetricRole::IconSize` wide, aligned to the rectangle's
/// top-left corner. Cells grow so that neither axis holds more than
/// [`MAX_CHECKER_CELLS`], which bounds the work for any valid theme metric or
/// allocation, including a zero icon size.
pub(crate) fn paint_checkerboard(painter: &mut Painter, rect: Rect) -> Result<(), DrawError> {
    painter.fill_rect(rect, CHECKER_DARK);
    let [left, top, width, height] = rect.values();
    let cell =
        (painter.metric(MetricRole::IconSize) / 2.0).max(width.max(height) / MAX_CHECKER_CELLS);
    if !(cell > 0.0 && cell.is_finite()) {
        // An empty rectangle has no cells to draw.
        return Ok(());
    }
    let columns = (width / cell).ceil() as u32;
    let rows = (height / cell).ceil() as u32;
    for row in 0..rows {
        for column in (row % 2..columns).step_by(2) {
            let x = column as f32 * cell;
            let y = row as f32 * cell;
            let cell_rect =
                Rect::new(left + x, top + y, cell.min(width - x), cell.min(height - y))?;
            painter.fill_rect(cell_rect, CHECKER_LIGHT);
        }
    }
    Ok(())
}

/// A square sample of one [`Color`], drawn over a checkerboard when the color
/// is translucent.
///
/// The swatch presents a value the application owns: a constant through
/// [`ColorSwatch::new`] or a binding through [`ColorSwatch::bound`], whose
/// changes repaint it without layout. It never writes the color.
///
/// By default it is static: not focusable, with no Tab stop, reported to
/// inspection as an [`SemanticRole::Image`]. [`ColorSwatch::on_activate`] makes
/// it an action with [`Button`]'s activation rules, for example to open a color
/// editor in a [`Popover`]: a primary press and release inside, or unmodified Enter or
/// Space released on the focused swatch, activates once; release outside,
/// Escape, focus loss, hiding and removal cancel. It is then reported as a
/// [`SemanticRole::Button`].
///
/// Either way the inspection name is the [`ColorSwatch::name`] text, or the
/// color's [`Color::to_hex`] text when none is set, and the semantic value is
/// the hex text. The `color` property holds the same text and the box model's
/// `background` the linear channels.
///
/// Default size is a square of the theme's `MetricRole::ControlHeight`; a
/// parent may allocate more or less. Translucent colors show the fixed neutral
/// checkerboard beneath them, in cells half `MetricRole::IconSize` wide; opaque
/// colors draw no checkerboard. The color itself is drawn unchanged when the
/// swatch is disabled, and only its outline turns muted.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{KeyAction, KeyCode};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
/// let color = ui.signal(Color::from_hex("#2090a0cc")?);
/// let opened = ui.signal(false);
/// let open = opened.clone();
/// let swatch = ColorSwatch::bound(color.read_only())
///     .name("Track color")
///     .on_activate(move |_| open.set(true));
/// let id = ui.mount_tree(Element::new("swatch", swatch))?;
/// ui.flush(Duration::ZERO)?;
/// ui.focus(&id)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
/// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
/// assert!(opened.get()?);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the swatch under the same key applies its color or binding,
/// name, enablement and action; adding or removing the action cancels a pending
/// activation.
#[must_use]
pub struct ColorSwatch {
    color: Color,
    source: Option<ReadSignal<Color>>,
    /// The described name; `None` when unset.
    name: Option<TextSource>,
    /// The name as last read, `None` while it is unset or empty.
    shown_name: Option<String>,
    action: Option<ActivateFn>,
    /// The application's restriction of activation; `enabled` holds its last
    /// read value.
    enablement: Permission,
    enabled: bool,
    input: ActivationState,
}
description_debug!(ColorSwatch);
impl ColorSwatch {
    /// Creates a static swatch of a constant color.
    pub fn new(color: Color) -> Self {
        Self {
            color,
            source: None,
            name: None,
            shown_name: None,
            action: None,
            enablement: Permission::allowed(),
            enabled: true,
            input: ActivationState::default(),
        }
    }
    /// Creates a static swatch that follows a color binding. Changes repaint
    /// the swatch without measuring it again.
    pub fn bound(color: impl Into<ReadSignal<Color>>) -> Self {
        let color: ReadSignal<Color> = color.into();
        Self {
            source: Some(color),
            ..Self::new(Color::WHITE)
        }
    }
    /// Names what the color is for, such as "Track color", for inspection and
    /// tools, as a [`TextSource`]: fixed text, a binding the name follows, or
    /// a [`StringRole`](crate::StringRole) of the installed
    /// [`UiStrings`](crate::UiStrings), which the swatch follows when another
    /// table is installed. Only the plain text is used. Without a name, and
    /// while the name is empty (fixed empty text, or a binding or role that
    /// reads empty), the hex text of the color is used; fixed empty text
    /// therefore leaves the name unset. Redescription with the same binding or
    /// role keeps the name already read; another source is read at the next
    /// update.
    pub fn name(mut self, name: impl Into<TextSource>) -> Self {
        let name: TextSource = name.into();
        match name.fixed().map(RichText::plain_text) {
            Some(text) if text.is_empty() => {
                self.name = None;
                self.shown_name = None;
            }
            fixed => {
                self.shown_name = fixed;
                self.name = Some(name);
            }
        }
        self
    }
    /// Makes the swatch an action: focusable, with one Tab stop, and running
    /// `action` on the UI thread once per completed activation. The callback
    /// may write bindings, such as one that opens a picker; its error
    /// propagates out of the input call.
    ///
    /// Timing class: [in the input](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#in-the-input).
    pub fn on_activate(
        mut self,
        action: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.action = Some(Box::new(action));
        self
    }
    /// Disables activation and removes the Tab stop. A static swatch is
    /// unaffected apart from its muted outline. Equivalent to
    /// `enabled(false)`; a later permission cannot lift the denial.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation with a fixed or reactive permission; allowed by
    /// default. Repeated calls combine restrictions; every condition must
    /// allow. Revocation cancels armed input and removes the Tab stop at the
    /// next update, so focus moves away; same-key redescription replaces the
    /// permission with the new description's. A static swatch only shows the
    /// muted outline. A binding from another runtime fails the update with
    /// [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = self.enablement.and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
    fn interactive(&self) -> bool {
        self.action.is_some() && self.enabled
    }
    fn label(&self) -> String {
        self.shown_name
            .clone()
            .unwrap_or_else(|| self.color.to_hex())
    }
}
impl Component for ColorSwatch {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let same_source = ReadSignal::same_optional(&self.source, &next.source);
        let name_changed = match (&self.name, &next.name) {
            (Some(mine), Some(theirs)) => !mine.same_as(theirs),
            (None, None) => false,
            _ => true,
        };
        let paint = !same_source
            || name_changed
            || (next.source.is_none() && self.color != next.color)
            || !self.enablement.same_as(&next.enablement)
            || self.action.is_some() != next.action.is_some();
        if next.source.is_none() || !same_source {
            self.color = next.color;
        }
        self.source = next.source;
        if name_changed {
            self.name = next.name;
            self.shown_name = next.shown_name;
        }
        if self.action.is_some() != next.action.is_some() {
            self.input.reset();
        }
        self.action = next.action;
        self.enablement = next.enablement;
        if let Some(enabled) = self.enablement.fixed() {
            if self.enabled && !enabled {
                self.input.reset();
            }
            self.enabled = enabled;
        }
        paint.then_some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        if !enabled && self.input.pressed() {
            // Revocation cancels an armed activation.
            self.input.reset();
            cx.cancel_input();
        }
        self.enabled = enabled;
        if let Some(source) = &self.source {
            self.color = cx.read_paint(source)?;
        }
        // A bound name or one from the string table follows its binding or a
        // replaced table; reading subscribes only such a swatch. The name is
        // not drawn, so a change only repaints.
        if let Some(name) = self.name.as_ref().filter(|name| name.is_bound()) {
            let text = name.read_paint(cx)?.plain_text();
            self.shown_name = (!text.is_empty()).then_some(text);
        }
        self.input.focused = cx.focused();
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.interactive()
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.interactive().then_some(CursorIcon::Default)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let side = cx.metric(MetricRole::ControlHeight);
        Ok(Size::new(side, side)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
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
        if self.input.pointer(cx, event)?
            && let Some(action) = &mut self.action
        {
            action(cx)?;
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
        if !self.interactive() || !activation_key(control) {
            return Ok(InputOutcome::Ignored);
        }
        let result = key_result(control, self.input.pressed(), cx.modifiers());
        if self.input.keyboard(cx, event, control)
            && let Some(action) = &mut self.action
        {
            action(cx)?;
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
        let rect = Rect::from_size(size);
        if self.color.alpha() < 1.0 {
            paint_checkerboard(painter, rect)?;
        }
        if self.color.alpha() > 0.0 {
            painter.fill_rect(rect, self.color);
        }
        if self.input.focused && self.interactive() {
            // Focus replaces the resting outline over any application color.
            return painter.focus_ring(rect, Shape::Rectangle);
        }
        let role = if self.input.pressed() {
            ColorRole::ButtonPressed
        } else if !self.enabled {
            ColorRole::Muted
        } else if self.input.hovered && self.interactive() {
            ColorRole::OnSurface
        } else {
            ColorRole::Outline
        };
        painter.stroke_shape(
            rect,
            Shape::Rectangle,
            Length::new(painter.metric(MetricRole::BorderWidth))?,
            painter.color(role),
        )?;
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let hex = self.color.to_hex();
        details
            .properties
            .insert("color".into(), InspectValue::Text(hex.clone()));
        details.box_model.background = Some(self.color.channels());
        let semantics = if self.action.is_some() {
            details
                .properties
                .insert("enabled".into(), InspectValue::Bool(self.enabled));
            details.validation.interactive = self.enabled;
            Semantics::new(SemanticRole::Button)
                .disabled(!self.enabled)
                .enabled_actions([SemanticAction::Activate])
        } else {
            Semantics::new(SemanticRole::Image)
        };
        details.semantics = Some(semantics.name(self.label()).value(SemanticValue::Text(hex)));
        details
    }
}
