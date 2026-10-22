//! A compact keyed label, optionally with a remove action the owner handles.
use crate::layout::centered;
use crate::*;
use std::cell::RefCell;

type RemoveFn = Box<dyn FnMut(&mut EventContext) -> Result<(), RuntimeError>>;
/// The label layout, and the remove icon layout with its button area.
struct Prepared {
    label: TextLayout,
    label_clip: Rect,
    close: Option<(TextLayout, Rect)>,
}

/// A compact, pill-shaped label for a tag, a filter or a chosen item, with an
/// optional leading icon and an optional remove action.
///
/// A static chip ([`Chip::new`]) is presentation only: it takes no Tab stop,
/// ignores pointer input so an ancestor can handle it, and reports
/// [`SemanticRole::Label`] with its text as the name.
///
/// A removable chip ([`Chip::removable`]) adds a trailing remove button inside
/// the chip and becomes one Tab stop. A matched primary press and release on
/// the remove button, or a fresh Delete or Backspace press while the chip has
/// focus, runs the remove callback once. OS repeats are ignored, so holding
/// Delete removes one chip per press. An armed pointer removal owns the gesture;
/// interleaved Delete/Backspace presses do not request another removal.
/// Escape or releasing outside the button
/// cancels a press. The callback only reports the request: the owner holds the
/// data, removes the item and describes its chips again without it. The chip
/// never removes itself.
///
/// Chips are identified by their element key, never by their label, so an owner
/// that rebuilds its chips in a [`Wrap`] keeps each chip's state and focus when
/// labels change or items are reordered. When the focused removable chip
/// disappears from the owner's new description, focus moves to the next chip
/// in the previous order, then to the previous chip, then to the nearest
/// focusable ancestor such as the owning control (see
/// [`Component::focus_siblings_on_removal`]).
///
/// Colors are the secondary container roles; size follows body text with the
/// themed control padding, smaller than `ControlHeight`. Inspection reports
/// `text` and `removable`, and a removable, enabled chip lists
/// [`SemanticAction::Dismiss`]. A constrained allocation reserves the trailing
/// removal target before clipping label ink, including its leading icon. Tiny
/// allocations bound the target and glyph to the chip; inspection keeps the
/// complete label and removal name even when the text cannot fit.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Chip, Element, Icon, Runtime, Wrap};
///
/// let mut ui = Runtime::new();
/// let tags = ui.signal(vec!["red".to_owned(), "green".to_owned()]);
/// let mut row: Vec<Element> = tags
///     .get()?
///     .into_iter()
///     .map(|tag| {
///         let (tags, removed) = (tags.clone(), tag.clone());
///         // The key is the item's identity; the label is presentation.
///         Element::new(tag.clone(), Chip::removable(tag, move |_| {
///             // The owner's data loses the item; its next description drops the chip.
///             let mut list = tags.get()?;
///             list.retain(|tag| *tag != removed);
///             tags.set(list)
///         }))
///     })
///     .collect();
/// row.push(Element::new("hint", Chip::new("new").leading_icon(Icon::Add)));
/// ui.mount_tree(Element::new("tags", Wrap::new(row)))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Chip {
    caption: Caption,
    remove: Option<RemoveFn>,
    /// The application's remove name, or `None` for the string table's
    /// message naming the chip.
    remove_name: Option<Caption>,
    /// The string table the default remove name comes from.
    strings: crate::UiStrings,
    enabled: bool,
    enablement: Permission,
    hovered_remove: bool,
    pressed: bool,
    focused: bool,
    prepared: RefCell<Option<Prepared>>,
}
impl Chip {
    /// Creates a static chip: a label with chip styling and no Tab stop.
    ///
    /// The text is a [`TextSource`]: fixed plain or rich content, or a binding
    /// the chip follows, such as a count or a translated tag. Its plain text
    /// names the chip and its default remove name.
    pub fn new(text: impl Into<TextSource>) -> Self {
        Self {
            caption: Caption::new(text),
            remove: None,
            remove_name: None,
            strings: crate::UiStrings::english(),
            enabled: true,
            enablement: Permission::allowed(),
            hovered_remove: false,
            pressed: false,
            focused: false,
            prepared: RefCell::new(None),
        }
    }
    /// Creates a removable chip whose remove button, Delete and Backspace call
    /// `on_remove` on the UI thread.
    ///
    /// The callback runs once per request in the event phase and may write
    /// bindings; an error propagates to the caller of the input method. It
    /// should remove the item from the owner's data, after which the owner's next
    /// description drops this chip.
    pub fn removable(
        text: impl Into<TextSource>,
        on_remove: impl FnMut(&mut EventContext) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        Self {
            remove: Some(Box::new(on_remove)),
            ..Self::new(text)
        }
    }
    /// Sets the leading icon with the themed gap. Repeated calls replace this
    /// slot; it stays when the text changes.
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.caption.leading = Some(icon.into());
        self
    }
    /// Sets the remove button's name for inspection with a [`TextSource`]:
    /// fixed text, a binding the name follows, or a
    /// [`StringRole`](crate::StringRole) of the installed string table; only
    /// its plain text is used, and a bound or role name is read in the chip's
    /// update. By default the installed [`UiStrings`](crate::UiStrings) supply
    /// it through [`StringRole::RemoveItem`](crate::StringRole::RemoveItem),
    /// "Remove" followed by the chip's text in English, and a replaced table
    /// renames it. Re-describing the chip applies a changed name; the same
    /// binding or role keeps the text already read. Ignored by a static chip.
    pub fn remove_name(mut self, name: impl Into<TextSource>) -> Self {
        self.remove_name = Some(Caption::new(name));
        self
    }
    /// Shows a removable chip without its remove action: no Tab stop, no remove
    /// button and muted text. Default is enabled.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts removal with a fixed or reactive permission; allowed by default.
    /// Repeated calls combine restrictions. Revocation cancels an armed removal,
    /// removes the close button and releases focus at update. Static chips remain
    /// presentation only. Same-key redescription replaces the permission.
    /// A binding from another runtime fails the update with [`RuntimeError::ForeignRuntime`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.enablement = self.enablement.and(permission);
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        self
    }
    fn display_content(&self) -> std::borrow::Cow<'_, RichText> {
        self.caption.display()
    }
    fn can_remove(&self) -> bool {
        self.remove.is_some() && self.enabled
    }
    fn padding(&self, metric: impl Fn(MetricRole) -> f32) -> (f32, f32) {
        (
            metric(MetricRole::ControlPaddingX),
            metric(MetricRole::ControlPaddingY) / 2.0,
        )
    }
    /// Width of the trailing remove button area, including its leading gap.
    fn remove_width(&self, metric: impl Fn(MetricRole) -> f32) -> f32 {
        if self.can_remove() {
            metric(MetricRole::IconGap) + metric(MetricRole::IconSize)
        } else {
            0.0
        }
    }
    /// The remove button's square at the trailing end of a chip of `size`.
    fn remove_rect(&self, size: Size, metric: impl Fn(MetricRole) -> f32) -> Option<Rect> {
        if !self.can_remove() {
            return None;
        }
        let icon = metric(MetricRole::IconSize)
            .min(size.height())
            .min(size.width());
        let (pad_x, _) = self.padding(&metric);
        let right = (size.width() - pad_x / 2.0).max(icon);
        Rect::new(right - icon, centered(size.height(), icon), icon, icon).ok()
    }
    fn remove_area(&self) -> Option<Rect> {
        self.prepared
            .borrow()
            .as_ref()
            .and_then(|prepared| prepared.close.as_ref().map(|(_, rect)| *rect))
    }
    fn remove_label(&self) -> String {
        match &self.remove_name {
            Some(name) => name.text().to_owned(),
            None => self.strings.format(crate::UiMessage::RemoveItem {
                item: self.caption.text(),
            }),
        }
    }
    fn request_removal(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        match self.remove.as_mut() {
            Some(remove) if self.enabled => remove(cx),
            _ => Ok(()),
        }
    }
}
impl Component for Chip {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let permission_changed = !self.enablement.same_as(&next.enablement);
        let layout = self.remove.is_some() != next.remove.is_some()
            || (self.remove.is_some() && permission_changed);
        let layout = self.caption.reconcile(next.caption) | layout;
        self.remove = next.remove;
        // The same binding or role keeps the name already read; another
        // source is read again in the update this requests.
        let name_changed = match (&mut self.remove_name, next.remove_name) {
            (Some(mine), Some(theirs)) => mine.reconcile(theirs),
            (None, None) => false,
            (mine, theirs) => {
                *mine = theirs;
                true
            }
        };
        self.enablement = next.enablement;
        if let Some(enabled) = self.enablement.fixed() {
            self.enabled = enabled;
        }
        if !self.can_remove() {
            self.pressed = false;
            self.hovered_remove = false;
        }
        if layout {
            Some(Invalidation::Layout)
        } else {
            (permission_changed || name_changed).then_some(Invalidation::Paint)
        }
    }
    fn focusable(&self) -> bool {
        self.can_remove()
    }
    fn focus_siblings_on_removal(&self) -> bool {
        true
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        self.can_remove().then_some(CursorIcon::Default)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.enablement.read_paint(cx)?;
        if enabled != self.enabled {
            self.enabled = enabled;
            if !enabled {
                self.pressed = false;
                self.hovered_remove = false;
                cx.cancel_input();
            }
            if self.remove.is_some() {
                cx.request_layout();
            }
        }
        self.caption.update(cx)?;
        self.focused = cx.focused();
        if self.remove.is_some() {
            match &mut self.remove_name {
                // The name is not drawn, so it needs no layout.
                Some(name) => {
                    name.update_paint(cx)?;
                }
                // The default remove name follows the installed string table.
                None => self.strings = cx.strings()?,
            }
        }
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.rich_text(&self.display_content(), None, TextStyle::Body)?;
        let metric = |role| cx.metric(role);
        let (pad_x, pad_y) = self.padding(metric);
        let height =
            (text.size().height() + 2.0 * pad_y).max(cx.metric(MetricRole::IconSize) + 2.0 * pad_y);
        Ok(Size::new(
            text.size().width() + 2.0 * pad_x + self.remove_width(metric)
                - if self.can_remove() { pad_x / 2.0 } else { 0.0 },
            height,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        let m = cx.measurement();
        let label = m.rich_text(&self.display_content(), None, TextStyle::Body)?;
        let close = match self.remove_rect(size, |role| m.metric(role)) {
            Some(rect) => Some((
                m.rich_text(&RichText::new().icon(Icon::Close), None, TextStyle::Body)?,
                rect,
            )),
            None => None,
        };
        let left = m.metric(MetricRole::ControlPaddingX).min(size.width());
        let right = close
            .as_ref()
            .map_or(size.width() - left, |(_, rect)| {
                rect.x() - m.metric(MetricRole::IconGap)
            })
            .max(left)
            .min(size.width());
        let label_clip = Rect::new(left, 0.0, right - left, size.height())?;
        *self.prepared.borrow_mut() = Some(Prepared {
            label,
            label_clip,
            close,
        });
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.can_remove() {
            return Ok(InputOutcome::Ignored);
        }
        let over = match (cx.size(), cx.to_local(event.position())) {
            (Some(_), Some(point)) if event.kind() != PointerKind::Leave => {
                self.remove_area().is_some_and(|rect| rect.contains(point))
            }
            _ => false,
        };
        if over != self.hovered_remove {
            self.hovered_remove = over;
            cx.invalidate(Invalidation::Paint);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if over => {
                cx.capture_pointer()?;
                self.pressed = true;
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.pressed => {
                self.pressed = false;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                if over {
                    self.request_removal(cx)?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Press(PointerButton::Primary) | PointerKind::Move | PointerKind::Leave => {
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !self.can_remove() {
            return Ok(InputOutcome::Ignored);
        }
        match control {
            KeyCode::Delete | KeyCode::Backspace => {
                if event.transition == KeyTransition::Pressed && !self.pressed {
                    self.request_removal(cx)?;
                }
                Ok(InputOutcome::Handled)
            }
            KeyCode::Escape if self.pressed => {
                self.pressed = false;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn input_reset(&mut self) {
        self.pressed = false;
        self.hovered_remove = false;
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let rect = Rect::from_size(size);
        let shape = Shape::Rounded(Length::new(size.height() / 2.0)?);
        painter.fill_shape(rect, shape, painter.color(ColorRole::SecondaryContainer))?;
        if self.focused && self.can_remove() {
            painter.focus_ring(rect, shape)?;
        }
        let foreground = if self.remove.is_some() && !self.enabled {
            painter.color(ColorRole::Muted)
        } else {
            painter.color(ColorRole::OnSecondaryContainer)
        };
        let prepared = self.prepared.borrow();
        let Some(prepared) = prepared.as_ref() else {
            return Ok(());
        };
        let label = &prepared.label;
        painter.with_clip(prepared.label_clip, |painter| {
            label.paint(
                painter,
                prepared.label_clip.x(),
                centered(size.height(), label.size().height()),
                foreground,
            )
        })?;
        if let Some((close, button)) = &prepared.close {
            let button = *button;
            if self.hovered_remove || self.pressed {
                let role = if self.pressed {
                    ColorRole::ButtonPressed
                } else {
                    ColorRole::SurfaceHover
                };
                painter.fill_shape(
                    button,
                    Shape::Rounded(Length::new(button.width() / 2.0)?),
                    painter.color(role),
                )?;
            }
            let color = if self.pressed {
                painter.color(ColorRole::OnButtonPressed)
            } else {
                foreground
            };
            painter.with_clip(button, |painter| {
                close.paint(
                    painter,
                    button.x() + centered(button.width(), close.size().width()),
                    button.y() + centered(button.height(), close.size().height()),
                    color,
                )
            })?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "text".into(),
            InspectValue::Text(self.caption.text().to_owned()),
        );
        details.properties.insert(
            "removable".into(),
            InspectValue::Bool(self.remove.is_some()),
        );
        details.validation.interactive = self.can_remove();
        let mut semantics = Semantics::new(SemanticRole::Label).name(self.caption.text());
        if self.remove.is_some() {
            semantics = semantics
                .description(self.remove_label())
                .disabled(!self.enabled);
            if self.enabled {
                semantics = semantics.enabled_actions([SemanticAction::Dismiss]);
            }
        }
        details.semantics = Some(semantics);
        details
    }
}
