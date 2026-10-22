//! A tool window's panel: a header with icon, title, application actions and
//! a hide button, and the content below. A floating panel is also its own
//! frame: its header moves it and its corner grip resizes it.
use super::ToolWindowMode;
use super::model::{self, Link, Window};
use crate::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// The header's arranged pieces, in the panel's local coordinates.
#[derive(Clone, Copy, Default)]
struct Header {
    size: Size,
    height: f32,
    title_x: f32,
    /// The room the title has before the header's controls.
    title_width: f32,
    icon: Option<Rect>,
}

/// A move or resize of a floating panel in progress: the frame it started
/// from, the frame the layout keeps before it, and where the pointer was.
#[derive(Clone, Copy)]
struct FloatDrag {
    frame: Rect,
    original: Option<Rect>,
    original_restored: Option<(Rect, [f64; 4])>,
    pointer: Point,
    resize: bool,
}

pub(super) struct Panel {
    link: Rc<Link>,
    drag: Option<FloatDrag>,
    key: String,
    title: Caption,
    icon: Option<IconSource>,
    children: Vec<Element>,
    toggles: Permission,
    focus_within: bool,
    header: Cell<Header>,
    text: RefCell<Option<TextLayout>>,
    /// What the header shows of the title while it shows less of it.
    title_shown: RefCell<Option<String>>,
}
impl Panel {
    /// Builds the panel of `window`. The hide button exists unless the user
    /// may never toggle the tool window.
    ///
    /// # Errors
    /// Returns the error of deriving the hide button's enabled binding.
    pub(super) fn new(
        link: Rc<Link>,
        window: &Window,
        toggles: Permission,
        options: bool,
        content: Element,
        actions: Option<Element>,
    ) -> Result<Self, RuntimeError> {
        // The content comes first, so it is first in Tab order and receives
        // focus when the panel is entered.
        let mut children = vec![content];
        if let Some(actions) = actions {
            children.push(Element::new("actions", Container::new(actions)).structural());
        }
        if options {
            children.push(
                Element::new(
                    "options",
                    super::options::Options::new(
                        link.clone(),
                        window.key.clone(),
                        window.title_source.clone(),
                    ),
                )
                .structural(),
            );
        }
        if toggles.fixed() != Some(false) {
            let hide = HideButton {
                link: link.clone(),
                key: window.key.clone(),
                title: Caption::new(window.title_source.clone()),
                enabled: toggles.signal()?,
                strings: None,
                children: Vec::new(),
            };
            children.push(Element::new("hide", hide).structural());
        }
        Ok(Self {
            link,
            drag: None,
            key: window.key.clone(),
            title: Caption::new(window.title_source.clone()),
            icon: window.icon.clone(),
            children,
            toggles,
            focus_within: false,
            header: Cell::new(Header::default()),
            text: RefCell::new(None),
            title_shown: RefCell::new(None),
        })
    }
}

/// The hide button, named from the installed string table: it describes its
/// button again when another table is installed or the title changes.
struct HideButton {
    link: Rc<Link>,
    key: String,
    title: Caption,
    enabled: Option<ReadSignal<bool>>,
    /// The table and title the button was last described with.
    strings: Option<(UiStrings, String)>,
    children: Vec<Element>,
}
impl Component for HideButton {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.title.reconcile(next.title);
        self.enabled = next.enabled;
        self.strings = None;
        Some(Invalidation::Layout)
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let strings = cx.strings()?;
        self.title.update_paint(cx)?;
        let described = (strings, self.title.text().to_owned());
        if self.strings.as_ref() == Some(&described) {
            return Ok(());
        }
        let name = described.0.format(UiMessage::ToolWindowHide {
            title: self.title.text(),
        });
        let (action_link, key) = (self.link.clone(), self.key.clone());
        let mut button =
            Button::icon(Icon::Remove, name, move |cx| hide(&action_link, &key, cx)).frameless();
        if let Some(enabled) = &self.enabled {
            button = button.enabled(enabled.clone());
        }
        cx.request_children(vec![Element::new("hide-button", button)]);
        cx.request_layout();
        self.strings = Some(described);
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        cx.measure(0, available)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            cx.place(0, Rect::new(0.0, 0.0, size.width(), size.height())?)?;
        }
        Ok(())
    }
}

/// The hide button's action: collapses the tool window when the user may,
/// and returns focus to the center, since focus is in the panel.
fn hide(link: &Link, key: &str, cx: &mut EventContext) -> Result<(), RuntimeError> {
    let model = link.model();
    let permission = model.borrow().toggle_permission(key);
    if !permission.get()? {
        return Ok(());
    }
    model::check_writable(&model)?;
    let center = {
        let mut model = model.borrow_mut();
        model.set_expanded(key, false, ChangeOrigin::User);
        std::mem::take(&mut model.return_focus)
            .then(|| model.center.clone())
            .flatten()
    };
    if let Some(center) = center {
        model.borrow_mut().focused_panel = None;
        cx.request_focus_within(&center)?;
    }
    model::touch(&model)
}

impl Panel {
    fn mode(&self) -> ToolWindowMode {
        self.link
            .model()
            .borrow()
            .window(&self.key)
            .map_or(ToolWindowMode::Docked, |window| window.mode)
    }
    /// Whether the panel is a floating frame the user may move and resize
    /// now.
    fn floating_frame(&self) -> bool {
        let model = self.link.model();
        let model = model.borrow();
        model
            .window(&self.key)
            .is_some_and(|window| window.mode == ToolWindowMode::Floating)
            && model.float_permission(&self.key).get().unwrap_or(false)
    }
    /// The corner grip of a floating frame, in local coordinates.
    fn grip(&self) -> Option<Rect> {
        let header = self.header.get();
        let side = header
            .height
            .min(header.size.width())
            .min(header.size.height())
            / 2.0;
        Rect::new(
            header.size.width() - side,
            header.size.height() - side,
            side,
            side,
        )
        .ok()
    }
    /// Sets the floating frame, for the layout to arrange at once. An
    /// unchanged frame invalidates nothing, so the pointer motion the runtime
    /// replays after a layout settles.
    fn set_frame(&self, cx: &mut EventContext, frame: Option<Rect>) -> bool {
        let model = self.link.model();
        let changed = match model
            .borrow_mut()
            .windows
            .iter_mut()
            .find(|window| window.key == self.key)
        {
            Some(window) if window.floating != frame => {
                window.floating = frame;
                window.restored_floating = None;
                true
            }
            _ => false,
        };
        if changed {
            cx.invalidate(Invalidation::Layout);
        }
        changed
    }
    /// Reports the committed floating frame of this tool window.
    fn report_geometry(&self) {
        self.link.model().borrow_mut().record(
            &self.key,
            crate::ToolWindowChangeKind::Geometry,
            ChangeOrigin::User,
        );
    }
    /// Ends a move or resize, returning the frame from before it.
    fn cancel_drag(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            self.link
                .model()
                .borrow_mut()
                .floating_interactions
                .remove(&self.key);
            let _ = self.set_frame(cx, drag.original);
            if let Some(window) = self
                .link
                .model()
                .borrow_mut()
                .windows
                .iter_mut()
                .find(|window| window.key == self.key)
            {
                window.restored_floating = drag.original_restored;
            }
            cx.release_pointer();
        }
    }
}

impl Component for Panel {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let changed = self.icon != next.icon;
        let changed = self.title.reconcile(next.title) | changed;
        self.icon = next.icon;
        self.toggles = next.toggles;
        changed.then_some(Invalidation::Layout)
    }
    fn unmounted(&mut self) {
        let model = self.link.model();
        let mut model = model.borrow_mut();
        model.floating_interactions.remove(&self.key);
        model.panel_notifiers.remove(&self.key);
        model.panel_pending.borrow_mut().remove(&self.key);
        model.panels.remove(&self.key);
        self.drag = None;
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.title.update(cx)? {
            cx.request_layout();
        }
        let model = self.link.model();
        let revision = model::revision(&model, cx);
        cx.read(&revision)?;
        let permission = model.borrow().float_permission(&self.key);
        let floatable = permission.read(cx)?;
        let mut model = model.borrow_mut();
        model.panels.insert(self.key.clone(), cx.id());
        model
            .panel_notifiers
            .entry(self.key.clone())
            .or_insert_with(|| cx.notifier());
        drop(model);
        // A floating move or resize ends when the panel stops floating.
        if (self.mode() != ToolWindowMode::Floating || !floatable)
            && let Some(drag) = self.drag.take()
        {
            let current = self.link.model();
            let mut model = current.borrow_mut();
            model.floating_interactions.remove(&self.key);
            if let Some(window) = model
                .windows
                .iter_mut()
                .find(|window| window.key == self.key)
            {
                window.floating = drag.original;
                window.restored_floating = drag.original_restored;
            }
            drop(model);
            cx.request_layout();
            cx.cancel_input();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        // A detached panel returns from its own window, whose event context
        // this is, when its tool window collapsed or left window mode.
        let model = self.link.model();
        let must = {
            let model = model.borrow();
            model.panel_pending.borrow_mut().remove(&self.key);
            model.must_return(&self.key) && !model.redocking.contains_key(&self.key)
        };
        if must {
            let receipt = cx.redock_window()?;
            model
                .borrow_mut()
                .redocking
                .insert(self.key.clone(), receipt);
        }
        Ok(())
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        if !self.floating_frame() {
            return None;
        }
        if self.drag.is_some_and(|drag| drag.resize)
            || self.grip().is_some_and(|grip| grip.contains(point))
        {
            Some(CursorIcon::ResizeDiagonal)
        } else if self.drag.is_some() || point.y() < self.header.get().height {
            Some(CursorIcon::Move)
        } else {
            None
        }
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if self.drag.is_some() && !self.floating_frame() {
            self.cancel_drag(cx);
            return Ok(InputOutcome::Handled);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary)
                if self.drag.is_none() && !event.is_outside() && self.floating_frame() =>
            {
                let Some(local) = cx.to_local(event.position()) else {
                    return Ok(InputOutcome::Ignored);
                };
                let resize = self.grip().is_some_and(|grip| grip.contains(local));
                if !resize && local.y() >= self.header.get().height {
                    return Ok(InputOutcome::Ignored);
                }
                let (Some(origin), Some(size)) = (cx.position_in_parent(), cx.size()) else {
                    return Ok(InputOutcome::Ignored);
                };
                let frame = Rect::new(origin.x(), origin.y(), size.width(), size.height())?;
                let original = self
                    .link
                    .model()
                    .borrow()
                    .window(&self.key)
                    .and_then(|window| window.floating);
                let original_restored = self
                    .link
                    .model()
                    .borrow()
                    .window(&self.key)
                    .and_then(|window| window.restored_floating);
                cx.capture_pointer()?;
                // Focus moves into the frame, as a press on a floating
                // window's title focuses it, so Escape reaches the panel.
                if !self.focus_within {
                    cx.request_focus_within(&cx.id())?;
                }
                self.link
                    .model()
                    .borrow_mut()
                    .floating_interactions
                    .insert(self.key.clone());
                self.drag = Some(FloatDrag {
                    frame,
                    original,
                    original_restored,
                    pointer: event.position(),
                    resize,
                });
                Ok(InputOutcome::Handled)
            }
            PointerKind::Move if self.drag.is_some() => {
                let Some(drag) = self.drag else {
                    return Ok(InputOutcome::Ignored);
                };
                let moved = cx.to_local_delta(Point::new(
                    event.position().x() - drag.pointer.x(),
                    event.position().y() - drag.pointer.y(),
                )?);
                let [x, y, width, height] = drag.frame.values();
                let frame = if drag.resize {
                    Rect::new(
                        x,
                        y,
                        (width + moved.x()).max(super::FLOAT_MINIMUM_WIDTH),
                        (height + moved.y()).max(super::FLOAT_MINIMUM_HEIGHT),
                    )?
                } else {
                    Rect::new(x + moved.x(), y + moved.y(), width, height)?
                };
                let _ = self.set_frame(cx, Some(frame));
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.drag.is_some() => {
                // The frame the motion set stays.
                let original = self.drag.take().and_then(|drag| drag.original);
                let committed = {
                    let model = self.link.model();
                    let mut model = model.borrow_mut();
                    model.floating_interactions.remove(&self.key);
                    model.window(&self.key).and_then(|window| window.floating)
                };
                cx.release_pointer();
                if committed != original {
                    self.report_geometry();
                }
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.cancel_drag(cx);
    }
    fn visibility_changed(&mut self, visible: bool) {
        // Hiding sends no focus-within notification, so a hidden panel gives
        // up its focus ownership here; a later collapse of it must not move
        // focus that is elsewhere.
        if !visible {
            self.focus_within = false;
            let model = self.link.model();
            let mut model = model.borrow_mut();
            if model.focused_panel.as_deref() == Some(self.key.as_str()) {
                model.focused_panel = None;
            }
        }
    }
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        if self.focus_within != within {
            self.focus_within = within;
            cx.invalidate(Invalidation::Paint);
        }
        let model = self.link.model();
        let mut model = model.borrow_mut();
        if within {
            model.focused_panel = Some(self.key.clone());
            if let Some(window) = model.windows.iter_mut().find(|w| w.key == self.key) {
                window.focus_entered = true;
            }
        } else {
            if model.focused_panel.as_deref() == Some(self.key.as_str()) {
                model.focused_panel = None;
            }
            // An overlay panel hides at the next delivery when focus is in
            // neither it nor a stripe; focus is already elsewhere, so it does
            // not return to the center.
            model.request();
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let control = cx.metric(MetricRole::ControlHeight);
        let mut width: f32 = 0.0;
        let mut height: f32 = control;
        for index in 0..cx.len() {
            let size = cx.measure(index, Constraints::new(available.width(), None)?)?;
            width = width.max(size.width());
            if !matches!(cx.key(index)?, "actions" | "options" | "hide") {
                height += size.height();
            }
        }
        Ok(Size::new(
            available.width().unwrap_or(width),
            available.height().unwrap_or(height),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let m = cx.measurement();
        let control = m.metric(MetricRole::ControlHeight);
        let padding_x = m.metric(MetricRole::ControlPaddingX);
        let padding_y = m.metric(MetricRole::ControlPaddingY);
        let icon = m.metric(MetricRole::IconSize);
        let gap = m.metric(MetricRole::IconGap);
        let mut trailing = Vec::new();
        let mut content = None;
        for index in 0..m.len() {
            match m.key(index)? {
                "actions" | "options" | "hide" => trailing.push(index),
                _ => content = Some(index),
            }
        }
        // The hide button sits at the end, the actions before it.
        trailing.sort_by_key(|index| m.key(*index).map(|key| key == "hide").unwrap_or(false));
        let sizes: Vec<Size> = trailing
            .iter()
            .map(|index| m.measure(*index, Constraints::new(None, None)?))
            .collect::<Result<_, _>>()?;
        let trailing_width: f32 = sizes.iter().map(|size| size.width() + gap).sum();
        let lead = padding_x + self.icon.as_ref().map_or(0.0, |_| icon + gap);
        let title_width = (size.width() - lead - padding_x - trailing_width).max(0.0);
        // A title too long for its room ends in an ellipsis; one with no room
        // at all is not shown, rather than drawn under the controls.
        let fitted = m.fitted_text(
            self.title.text(),
            title_width,
            TextStyle::Body,
            TextOverflow::Ellipsis,
        )?;
        // With no room at all nothing is shown, which loses text only when
        // there is a title to lose.
        let shown = if title_width > 0.0 {
            fitted.shown().map(str::to_owned)
        } else {
            (!self.title.text().is_empty()).then(String::new)
        };
        self.title_shown.replace(shown);
        let text = fitted.into_layout();
        let height = sizes
            .iter()
            .map(|size| size.height())
            .fold(
                control.max(text.size().height() + 2.0 * padding_y),
                f32::max,
            )
            .min(size.height());
        let mut x = size.width() - padding_x.min(size.width());
        for (index, measured) in trailing.iter().zip(&sizes).rev() {
            x -= measured.width();
            // In a narrow panel the header's start is cut off rather than
            // its controls laid over each other.
            let y = ((height - measured.height()) / 2.0).max(0.0);
            let rect = Rect::new(x, y, measured.width(), measured.height().min(height))?;
            let start = x.max(0.0);
            let end = (x + measured.width()).clamp(start, size.width());
            let clip = Rect::new(start, y, end - start, measured.height().min(height))?;
            cx.place_clipped(*index, rect, clip)?;
            x -= gap;
        }
        // A floating frame keeps a strip below the content for its corner
        // grip, so the grip is never covered.
        let grip = if self.mode() == ToolWindowMode::Floating {
            height / 2.0
        } else {
            0.0
        };
        if let Some(index) = content {
            let rect = Rect::new(
                0.0,
                height,
                size.width(),
                (size.height() - height - grip).max(0.0),
            )?;
            cx.place_clipped(index, rect, rect)?;
        }
        let side = icon.min(height);
        self.header.set(Header {
            size,
            height,
            title_x: lead,
            title_width,
            icon: self
                .icon
                .as_ref()
                .map(|_| Rect::new(padding_x, (height - side) / 2.0, side, side))
                .transpose()?,
        });
        *self.text.borrow_mut() = Some(text);
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let header = self.header.get();
        let [w, h] = [header.size.width(), header.size.height()];
        p.fill_rect(Rect::new(0.0, 0.0, w, h)?, p.color(ColorRole::Surface));
        if self.focus_within {
            p.fill_rect(
                Rect::new(0.0, 0.0, w, header.height)?,
                p.color(ColorRole::SurfaceHover),
            );
        }
        let line = p.metric(MetricRole::BorderWidth).min(header.height);
        p.fill_rect(
            Rect::new(0.0, header.height - line, w, line)?,
            p.color(ColorRole::Divider),
        );
        if self.mode() == ToolWindowMode::Floating {
            p.stroke_shape(
                Rect::new(0.0, 0.0, w, h)?,
                Shape::Rectangle,
                Length::new(p.metric(MetricRole::BorderWidth))?,
                p.color(if self.focus_within {
                    ColorRole::Primary
                } else {
                    ColorRole::PanelBorder
                }),
            )?;
            if self.floating_frame()
                && let Some(grip) = self.grip()
            {
                crate::resize_grip::paint_corner_triangle(p, grip)?;
            }
        }
        if let (Some(icon), Some(rect)) = (&self.icon, header.icon) {
            p.icon(icon.clone(), rect, p.color(ColorRole::OnSurface))?;
        }
        if let Some(text) = self.text.borrow().as_ref()
            && header.title_width > 0.0
        {
            let room = Rect::new(header.title_x, 0.0, header.title_width, header.height)?;
            p.with_clip(room, |p| {
                text.paint(
                    p,
                    crate::Point::new(
                        header.title_x,
                        ((header.height - text.size().height()) / 2.0).max(0.0),
                    )?,
                    p.color(ColorRole::OnSurface),
                )
            })?;
        }
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if control == KeyCode::Escape
            && event.transition == KeyTransition::Pressed
            && self.drag.is_some()
        {
            self.cancel_drag(cx);
            return Ok(InputOutcome::Handled);
        }
        // Alt+arrows move a floating frame by eight logical pixels, and
        // Alt+Shift+arrows resize it, as for a floating window, when the
        // content leaves the keys unhandled.
        if self.drag.is_none()
            && keyboard.modifiers().alt
            // Only Alt, optionally with Shift to resize; other chords belong
            // to command bindings.
            && !keyboard.modifiers().control
            && !keyboard.modifiers().super_key
            && matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            )
            && self.floating_frame()
        {
            let delta = match control {
                KeyCode::ArrowLeft => Some((-8.0, 0.0)),
                KeyCode::ArrowRight => Some((8.0, 0.0)),
                KeyCode::ArrowUp => Some((0.0, -8.0)),
                KeyCode::ArrowDown => Some((0.0, 8.0)),
                _ => None,
            };
            if let (Some((dx, dy)), Some(origin), Some(size)) =
                (delta, cx.position_in_parent(), cx.size())
            {
                let frame = if keyboard.modifiers().shift {
                    Rect::new(
                        origin.x(),
                        origin.y(),
                        (size.width() + dx).max(super::FLOAT_MINIMUM_WIDTH),
                        (size.height() + dy).max(super::FLOAT_MINIMUM_HEIGHT),
                    )?
                } else {
                    Rect::new(
                        origin.x() + dx,
                        origin.y() + dy,
                        size.width(),
                        size.height(),
                    )?
                };
                // Each key step commits, as for a floating window.
                if self.set_frame(cx, Some(frame)) {
                    self.report_geometry();
                }
                return Ok(InputOutcome::Handled);
            }
        }
        if control != KeyCode::Escape || event.transition != KeyTransition::Pressed {
            return Ok(InputOutcome::Ignored);
        }
        let center = self.link.model().borrow().center.clone();
        match center {
            Some(center) => {
                cx.request_focus_within(&center)?;
                Ok(InputOutcome::Handled)
            }
            None => Ok(InputOutcome::Ignored),
        }
    }
    fn inspect(&self) -> ComponentDetails {
        let allowed = self.toggles.get().unwrap_or(false);
        let mut semantics = Semantics::new(SemanticRole::Group)
            .name(self.title.text())
            .actions(allowed.then_some(SemanticAction::Collapse));
        semantics.expanded = Some(true);
        let mut details = ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        };
        details
            .properties
            .insert("tool-window".into(), InspectValue::Text(self.key.clone()));
        details
            .properties
            .insert("active".into(), InspectValue::Bool(self.focus_within));
        // Like a label's: whether the header shows less than the title, and
        // what it shows then (nothing when there is no room at all).
        let shown = self.title_shown.borrow();
        details
            .properties
            .insert("truncated".into(), InspectValue::Bool(shown.is_some()));
        if let Some(shown) = shown.as_ref() {
            details
                .properties
                .insert("shown".into(), InspectValue::Text(shown.clone()));
        }
        let (overlay, mode) = self
            .link
            .model()
            .borrow()
            .window(&self.key)
            .map_or((false, ToolWindowMode::Docked), |window| {
                (window.overlay, window.mode)
            });
        details
            .properties
            .insert("overlay".into(), InspectValue::Bool(overlay));
        details
            .properties
            .insert("mode".into(), InspectValue::Text(mode.name().to_owned()));
        if overlay || mode != ToolWindowMode::Docked {
            // An overlay panel lies over the center, and a floating frame over
            // the whole layout, on purpose.
            details.validation.overlap = OverlapPolicy::Allow;
        }
        if self.floating_frame()
            && let Some(semantics) = details.semantics.as_mut()
        {
            semantics.actions.push(SemanticAction::Move);
            semantics.actions.push(SemanticAction::Resize);
        }
        details
    }
}
