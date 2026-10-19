//! The stripe of buttons along an edge that expands and collapses the edge's
//! tool windows.
use super::model::{self, Link};
use super::{StripeGroup, ToolEdge};
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

/// The input that armed a button, and which button it armed.
#[derive(Clone, PartialEq)]
enum Armed {
    /// A pointer press: where it was, in window coordinates, and whether the
    /// tool window was expanded then. A click sets the opposite, so an
    /// overlay panel that hid when the press took focus from it stays hidden.
    Pointer {
        key: String,
        at: Point,
        expanded: bool,
    },
    Key(KeyCode, String),
}
impl Armed {
    fn key(&self) -> &str {
        match self {
            Self::Pointer { key, .. } | Self::Key(_, key) => key,
        }
    }
}

/// What the user may do with one button's tool window, read at update.
#[derive(Clone, Copy, Default)]
struct Rights {
    toggle: bool,
    /// Drag the button: the drag permission and a move or reorder permission.
    drag: bool,
}

/// The drag payload of a stripe button: which layout, by its shared state,
/// and which tool window. Stripes of other layouts reject it.
pub(super) struct ButtonDrag {
    layout: *const RefCell<model::Model>,
    key: String,
}

/// One button as the stripe arranged it, in local coordinates.
struct Placed {
    key: String,
    group: StripeGroup,
    rect: Rect,
    icon: Option<(IconSource, Rect)>,
    name: Option<(TextLayout, Point)>,
}

/// One button's content: the tool window's key, icon and title.
#[derive(Clone, PartialEq)]
struct ButtonSpec {
    key: String,
    title: String,
    icon: Option<IconSource>,
    group: StripeGroup,
}

pub(super) struct Stripe {
    link: Rc<Link>,
    edge: ToolEdge,
    active: Option<String>,
    armed: Option<Armed>,
    hovered: Option<String>,
    focused: bool,
    /// What the user may do with each button's tool window, read at update.
    rights: BTreeMap<String, Rights>,
    /// The button this stripe is dragging.
    dragging: Option<String>,
    /// Whether this stripe set the layout's `stripe_focus`; it gives it up
    /// when it loses focus, hides or unmounts.
    owns_focus: bool,
    /// Where a drop would insert, along the stripe, while a drag accepted
    /// here is over it.
    indicator: Option<f32>,
    placed: RefCell<Vec<Placed>>,
    size: Cell<Size>,
    offset: Cell<f32>,
    extent: Cell<f32>,
    /// Logical pixels per tick that a drag resting near an edge of an
    /// overflowing stripe scrolls it by: negative towards the start.
    autoscroll: f32,
    reveal_active: Cell<bool>,
    geometry: Option<(Vec<ButtonSpec>, bool)>,
    /// The stripe's name from the installed string table.
    name: String,
}
impl Stripe {
    pub(super) fn new(link: Rc<Link>, edge: ToolEdge) -> Self {
        Self {
            link,
            edge,
            active: None,
            armed: None,
            hovered: None,
            focused: false,
            rights: BTreeMap::new(),
            dragging: None,
            owns_focus: false,
            indicator: None,
            placed: RefCell::new(Vec::new()),
            size: Cell::new(Size::ZERO),
            offset: Cell::new(0.0),
            extent: Cell::new(0.0),
            autoscroll: 0.0,
            reveal_active: Cell::new(true),
            geometry: None,
            name: UiStrings::english().get(edge.stripe_role()).to_owned(),
        }
    }
    /// The edge's buttons, start group first, each in description order.
    fn buttons(&self) -> (Vec<ButtonSpec>, bool) {
        let model = self.link.model();
        let model = model.borrow();
        let mut buttons: Vec<ButtonSpec> = model
            .windows
            .iter()
            .filter(|window| window.edge == self.edge)
            .map(|window| ButtonSpec {
                key: window.key.clone(),
                title: window.title.clone(),
                icon: window.icon.clone(),
                group: window.group,
            })
            .collect();
        buttons.sort_by_key(|button| button.group == StripeGroup::End);
        (buttons, model.names)
    }
    fn keys(&self) -> Vec<String> {
        self.buttons()
            .0
            .into_iter()
            .map(|button| button.key)
            .collect()
    }
    fn is_allowed(&self, key: &str) -> bool {
        self.rights.get(key).is_some_and(|rights| rights.toggle)
    }
    fn may_drag(&self, key: &str) -> bool {
        self.rights.get(key).is_some_and(|rights| rights.drag)
    }
    /// How fast a drag at `point` scrolls an overflowing stripe, with the
    /// lists' zone and speed.
    fn autoscroll_at(&self, point: Point) -> f32 {
        let main = if self.edge.vertical() {
            self.size.get().height()
        } else {
            self.size.get().width()
        };
        if self.extent.get() <= main {
            return 0.0;
        }
        let zone = crate::virtual_list::AUTOSCROLL_ZONE.min(main / 3.0);
        if zone <= 0.0 {
            return 0.0;
        }
        let along = self.along(point);
        let depth = if along < zone {
            -(zone - along.max(0.0)) / zone
        } else if along > main - zone {
            (along.min(main) - (main - zone)) / zone
        } else {
            0.0
        };
        depth * crate::virtual_list::AUTOSCROLL_STEP
    }
    /// Starts, retargets or stops the autoscroll for a drag at `speed`.
    fn set_autoscroll(&mut self, cx: &mut EventContext, speed: f32) -> Result<(), RuntimeError> {
        if speed != 0.0 && self.autoscroll == 0.0 {
            cx.set_timer(crate::virtual_list::AUTOSCROLL_INTERVAL)?;
        } else if speed == 0.0 && self.autoscroll != 0.0 {
            cx.cancel_timer();
        }
        self.autoscroll = speed;
        Ok(())
    }
    /// Along the stripe: the coordinate of a point and the span of a rect.
    fn along(&self, point: Point) -> f32 {
        if self.edge.vertical() {
            point.y()
        } else {
            point.x()
        }
    }
    fn span(&self, rect: Rect) -> (f32, f32) {
        if self.edge.vertical() {
            (rect.y(), rect.y() + rect.height())
        } else {
            (rect.x(), rect.x() + rect.width())
        }
    }
    /// Where dropping `dragged` at `point` (local) would put it: the group,
    /// the position among that group's other buttons, and the coordinate
    /// along the stripe where the insertion line goes.
    fn drop_target(&self, dragged: &str, point: Point) -> (StripeGroup, usize, f32) {
        let placed = self.placed.borrow();
        let others: Vec<&Placed> = placed
            .iter()
            .filter(|placed| placed.key != dragged)
            .collect();
        let total = if self.edge.vertical() {
            self.size.get().height()
        } else {
            self.size.get().width()
        };
        let starts: Vec<&&Placed> = others
            .iter()
            .filter(|p| p.group == StripeGroup::Start)
            .collect();
        let ends: Vec<&&Placed> = others
            .iter()
            .filter(|p| p.group == StripeGroup::End)
            .collect();
        let start_end = starts.last().map_or(0.0, |p| self.span(p.rect).1);
        let end_begin = ends.first().map_or(total, |p| self.span(p.rect).0);
        let along = self.along(point);
        let group = if along >= (start_end + end_begin) / 2.0 {
            StripeGroup::End
        } else {
            StripeGroup::Start
        };
        let members = if group == StripeGroup::Start {
            &starts
        } else {
            &ends
        };
        let index = members
            .iter()
            .filter(|p| {
                let (a, b) = self.span(p.rect);
                (a + b) / 2.0 < along
            })
            .count();
        let line = match members.get(index) {
            Some(p) => self.span(p.rect).0,
            None => match members.last() {
                Some(p) => self.span(p.rect).1,
                None if group == StripeGroup::Start => start_end,
                None => end_begin,
            },
        };
        (group, index, line)
    }
    fn button_at(&self, point: Point) -> Option<String> {
        if !Rect::from_size(self.size.get()).contains(point) {
            return None;
        }
        self.placed
            .borrow()
            .iter()
            .find(|placed| placed.rect.contains(point))
            .map(|placed| placed.key.clone())
    }
    /// Expands or collapses a button's tool window for the user, to the
    /// opposite of `was` when given and of its current state otherwise;
    /// expanding moves focus into its panel.
    fn activate(
        &mut self,
        cx: &mut EventContext,
        key: &str,
        was: Option<bool>,
    ) -> Result<(), RuntimeError> {
        if !self.is_allowed(key) {
            return Ok(());
        }
        let model = self.link.model();
        model::check_writable(&model)?;
        let (expanded, panel) = {
            let mut model = model.borrow_mut();
            let expanded = !was.unwrap_or_else(|| model.is_expanded(key));
            model.set_expanded(key, expanded, ChangeOrigin::User);
            (expanded, model.panels.get(key).cloned())
        };
        if expanded && let Some(panel) = panel {
            cx.request_focus_within(&panel)?;
        }
        cx.invalidate(Invalidation::Paint);
        model::touch(&model)
    }
    /// Moves the active button by an arrow, Home or End key.
    fn step(&mut self, cx: &mut EventContext, code: KeyCode) -> bool {
        let keys = self.keys();
        if keys.is_empty() {
            return false;
        }
        let (back, forward) = if self.edge.vertical() {
            (KeyCode::ArrowUp, KeyCode::ArrowDown)
        } else {
            (KeyCode::ArrowLeft, KeyCode::ArrowRight)
        };
        let current = self
            .active
            .as_ref()
            .and_then(|active| keys.iter().position(|key| key == active))
            .unwrap_or(0);
        let next = match code {
            code if code == back => current.saturating_sub(1),
            code if code == forward => (current + 1).min(keys.len() - 1),
            KeyCode::Home => 0,
            KeyCode::End => keys.len() - 1,
            _ => return false,
        };
        let needs_reveal = self
            .placed
            .borrow()
            .iter()
            .find(|placed| placed.key == keys[next])
            .is_none_or(|placed| {
                let (a, b) = self.span(placed.rect);
                let viewport = if self.edge.vertical() {
                    self.size.get().height()
                } else {
                    self.size.get().width()
                };
                if b - a > viewport {
                    a != 0.0
                } else {
                    a < 0.0 || b > viewport
                }
            });
        if self.active.as_ref() != Some(&keys[next]) || needs_reveal {
            self.active = Some(keys[next].clone());
            self.reveal_active.set(true);
            cx.invalidate(Invalidation::Layout);
        }
        true
    }
    fn reset_input(&mut self) {
        self.armed = None;
        self.hovered = None;
        self.indicator = None;
    }
}

/// Theme measurements a button uses.
#[derive(Clone, Copy)]
struct Metrics {
    control: f32,
    padding_x: f32,
    padding_y: f32,
    icon: f32,
    gap: f32,
}
impl Metrics {
    fn read(cx: &MeasureContext<'_>) -> Self {
        Self {
            control: cx.metric(MetricRole::ControlHeight),
            padding_x: cx.metric(MetricRole::ControlPaddingX),
            padding_y: cx.metric(MetricRole::ControlPaddingY),
            icon: cx.metric(MetricRole::IconSize),
            gap: cx.metric(MetricRole::IconGap),
        }
    }
}

/// A button's size and content placement relative to its own origin.
struct Laid {
    size: Size,
    icon: Option<Rect>,
    name: Option<(TextLayout, Point)>,
}

fn lay_out(
    cx: &MeasureContext<'_>,
    metrics: Metrics,
    vertical: bool,
    button: &ButtonSpec,
    names: bool,
) -> Result<Laid, RuntimeError> {
    let show_name = names || button.icon.is_none();
    let name_width = metrics.control * 3.0;
    // A vertical stripe gives a name a fixed width and cuts it with an
    // ellipsis; a horizontal stripe gives it the room it needs.
    let name = show_name
        .then(|| {
            if vertical {
                cx.fitted_text(
                    &button.title,
                    name_width,
                    TextStyle::Caption,
                    TextOverflow::Ellipsis,
                )
                .map(FittedText::into_layout)
            } else {
                cx.text(&button.title, None, TextStyle::Body)
            }
        })
        .transpose()?;
    let icon_side = metrics.icon.min(metrics.control);
    let has_icon = button.icon.is_some();
    if vertical {
        let name_size = name.as_ref().map_or(Size::ZERO, |text| text.size());
        let width = metrics
            .control
            .max(name_size.width() + 2.0 * metrics.padding_y);
        let icon_height = if has_icon { metrics.control } else { 0.0 };
        let name_height = if name.is_some() {
            name_size.height() + metrics.padding_y
        } else {
            0.0
        };
        let height = (icon_height + name_height).max(metrics.control);
        let icon = has_icon
            .then(|| {
                Rect::new(
                    (width - icon_side) / 2.0,
                    (metrics.control - icon_side) / 2.0,
                    icon_side,
                    icon_side,
                )
            })
            .transpose()?;
        let name = name
            .map(|text| -> Result<_, RuntimeError> {
                let point = Point::new(
                    (width - text.size().width()) / 2.0,
                    if has_icon {
                        icon_height
                    } else {
                        (height - text.size().height()) / 2.0
                    },
                )?;
                Ok((text, point))
            })
            .transpose()?;
        Ok(Laid {
            size: Size::new(width, height)?,
            icon,
            name,
        })
    } else {
        let name_size = name.as_ref().map_or(Size::ZERO, |text| text.size());
        let icon_width = if has_icon { icon_side } else { 0.0 };
        let between = if has_icon && name.is_some() {
            metrics.gap
        } else {
            0.0
        };
        let content = icon_width + between + name_size.width();
        let width = if name.is_some() {
            content + 2.0 * metrics.padding_x
        } else {
            metrics.control
        };
        let height = metrics
            .control
            .max(name_size.height() + 2.0 * metrics.padding_y);
        let start = (width - content) / 2.0;
        let icon = has_icon
            .then(|| Rect::new(start, (height - icon_side) / 2.0, icon_side, icon_side))
            .transpose()?;
        let name = name
            .map(|text| -> Result<_, RuntimeError> {
                let point = Point::new(
                    start + icon_width + between,
                    (height - text.size().height()) / 2.0,
                )?;
                Ok((text, point))
            })
            .transpose()?;
        Ok(Laid {
            size: Size::new(width, height)?,
            icon,
            name,
        })
    }
}

impl Stripe {
    /// Clears the layout's `stripe_focus` this stripe set, and asks for a
    /// delivery so an overlay panel focus left for the stripe hides now if
    /// focus went elsewhere.
    fn give_up_focus(&mut self) {
        if !std::mem::take(&mut self.owns_focus) {
            return;
        }
        let model = self.link.model();
        let mut model = model.borrow_mut();
        model.stripe_focus = false;
        model.request();
    }
}

impl Component for Stripe {
    fn reconcile(&mut self, _: Self) -> Option<Invalidation> {
        // The buttons come from the shared model, which the layout updates.
        self.reset_input();
        Some(Invalidation::Layout)
    }
    fn focusable(&self) -> bool {
        self.rights.values().any(|rights| rights.toggle)
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        self.button_at(point).map(|_| CursorIcon::Default)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hovered.take().is_some() {
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.name = cx.strings()?.get(self.edge.stripe_role()).to_owned();
        let model = self.link.model();
        let edge = self.edge;
        // The buttons compare their titles below, which relayouts on change.
        model::read_titles(&model, cx, |window| window.edge == edge)?;
        type Permissions = (String, Permission, Permission, Permission, Permission);
        let permissions: Vec<Permissions> = {
            let model = model.borrow();
            model
                .windows
                .iter()
                .filter(|window| window.edge == self.edge)
                .map(|window| {
                    let key = &window.key;
                    (
                        key.clone(),
                        model.toggle_permission(key),
                        model.drag_permission(key),
                        model.move_permission(key),
                        model.reorder_permission(key),
                    )
                })
                .collect()
        };
        let revision = model::revision(&model, cx);
        cx.read_paint(&revision)?;
        self.rights.clear();
        for (key, toggle, drag, movable, reorderable) in permissions {
            let toggle = toggle.read_paint(cx)?;
            let drag = drag.read_paint(cx)?;
            let movable = movable.read_paint(cx)?;
            let reorderable = reorderable.read_paint(cx)?;
            self.rights.insert(
                key,
                Rights {
                    toggle,
                    drag: drag && (movable || reorderable),
                },
            );
        }
        let geometry = self.buttons();
        let geometry_changed = self.geometry.as_ref() != Some(&geometry);
        self.geometry = Some(geometry);
        let keys = self.keys();
        let old_active = self.active.clone();
        if !self
            .active
            .as_ref()
            .is_some_and(|active| keys.contains(active))
        {
            self.active = keys.first().cloned();
        }
        if let Some(armed) = &self.armed {
            let key = armed.key();
            let keeps = self.is_allowed(key)
                || matches!(armed, Armed::Pointer { .. }) && self.may_drag(key);
            if !keeps {
                self.armed = None;
                cx.cancel_input();
            }
        }
        let focused = cx.focused();
        if geometry_changed || self.active != old_active {
            self.reveal_active.set(true);
            cx.request_layout();
        }
        self.focused = focused;
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let metrics = Metrics::read(cx);
        let vertical = self.edge.vertical();
        let (buttons, names) = self.buttons();
        let (mut along, mut across): (f32, f32) = (0.0, metrics.control);
        for button in &buttons {
            let laid = lay_out(cx, metrics, vertical, button, names)?;
            if vertical {
                along += laid.size.height();
                across = across.max(laid.size.width());
            } else {
                along += laid.size.width();
                across = across.max(laid.size.height());
            }
        }
        Ok(if vertical {
            Size::new(across, along)?
        } else {
            Size::new(along, across)?
        })
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let m = cx.measurement();
        let metrics = Metrics::read(m);
        let vertical = self.edge.vertical();
        let (buttons, names) = self.buttons();
        let mut placed = Vec::with_capacity(buttons.len());
        let laid: Vec<Laid> = buttons
            .iter()
            .map(|button| lay_out(m, metrics, vertical, button, names))
            .collect::<Result<_, _>>()?;
        let (across, total) = if vertical {
            (size.width(), size.height())
        } else {
            (size.height(), size.width())
        };
        let length = |laid: &Laid| {
            if vertical {
                laid.size.height()
            } else {
                laid.size.width()
            }
        };
        let extent: f32 = laid.iter().map(length).sum();
        let overflow = (extent - total).max(0.0);
        self.extent.set(extent);
        let mut scroll = self.offset.get().clamp(0.0, overflow);
        if (self.reveal_active.replace(false) || self.focused && size != self.size.get())
            && overflow > 0.0
        {
            let mut at = 0.0;
            for (button, laid) in buttons.iter().zip(&laid) {
                let end = at + length(laid);
                if self.active.as_ref() == Some(&button.key) {
                    // A button longer than the viewport reveals its leading edge.
                    if length(laid) > total || at < scroll {
                        scroll = at;
                    } else if end > scroll + total {
                        scroll = end - total;
                    }
                    break;
                }
                at = end;
            }
        }
        self.offset.set(scroll.clamp(0.0, overflow));
        let ends: f32 = buttons
            .iter()
            .zip(&laid)
            .filter(|(button, _)| button.group == StripeGroup::End)
            .map(|(_, laid)| length(laid))
            .sum();
        let mut start = 0.0f32;
        let mut end = if overflow > 0.0 {
            extent - ends
        } else {
            total - ends
        };
        for (button, laid) in buttons.into_iter().zip(laid) {
            let length = if vertical {
                laid.size.height()
            } else {
                laid.size.width()
            };
            let offset = if button.group == StripeGroup::End {
                let offset = end;
                end += length;
                offset
            } else {
                let offset = start;
                start += length;
                offset
            };
            let offset = offset - self.offset.get();
            let (x, y) = if vertical {
                (((across - laid.size.width()) / 2.0).max(0.0), offset)
            } else {
                (offset, ((across - laid.size.height()) / 2.0).max(0.0))
            };
            let rect = Rect::new(x, y, laid.size.width(), laid.size.height())?;
            let shift = |r: Rect| -> Result<Rect, RuntimeError> {
                let [rx, ry, rw, rh] = r.values();
                Ok(Rect::new(rx + rect.x(), ry + rect.y(), rw, rh)?)
            };
            placed.push(Placed {
                icon: match (button.icon, laid.icon) {
                    (Some(icon), Some(r)) => Some((icon, shift(r)?)),
                    _ => None,
                },
                name: laid
                    .name
                    .map(|(text, point)| -> Result<_, RuntimeError> {
                        Ok((
                            text,
                            Point::new(point.x() + rect.x(), point.y() + rect.y())?,
                        ))
                    })
                    .transpose()?,
                group: button.group,
                key: button.key,
                rect,
            });
        }
        *self.placed.borrow_mut() = placed;
        self.size.set(size);
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = self.size.get();
        p.fill_rect(
            Rect::new(0.0, 0.0, size.width(), size.height())?,
            p.color(ColorRole::Background),
        );
        let line = p.metric(MetricRole::BorderWidth);
        let rule = match self.edge {
            ToolEdge::Left => Rect::new((size.width() - line).max(0.0), 0.0, line, size.height())?,
            ToolEdge::Right => Rect::new(0.0, 0.0, line, size.height())?,
            ToolEdge::Top => Rect::new(0.0, (size.height() - line).max(0.0), size.width(), line)?,
            ToolEdge::Bottom => Rect::new(0.0, 0.0, size.width(), line)?,
        };
        p.fill_rect(rule, p.color(ColorRole::Divider));
        let model = self.link.model();
        for placed in self.placed.borrow().iter() {
            if placed.rect.intersection(Rect::from_size(size)).is_empty() {
                continue;
            }
            let expanded = model.borrow().is_expanded(&placed.key);
            let allowed = self.is_allowed(&placed.key);
            let armed = self
                .armed
                .as_ref()
                .is_some_and(|armed| armed.key() == placed.key)
                || self.dragging.as_ref() == Some(&placed.key);
            let radius = Length::new(p.metric(MetricRole::CornerRadius))?;
            if expanded {
                p.fill_shape(
                    placed.rect,
                    Shape::Rounded(radius),
                    p.color(ColorRole::TabSelected),
                )?;
            } else if allowed && (armed || self.hovered.as_ref() == Some(&placed.key)) {
                p.fill_shape(
                    placed.rect,
                    Shape::Rounded(radius),
                    p.color(ColorRole::SurfaceHover),
                )?;
            }
            let color = p.color(if allowed {
                ColorRole::OnSurface
            } else {
                ColorRole::Muted
            });
            if let Some((icon, rect)) = &placed.icon {
                p.icon(icon.clone(), *rect, color)?;
            }
            if let Some((text, point)) = &placed.name {
                text.paint(p, point.x(), point.y(), color)?;
            }
            if self.focused && self.active.as_ref() == Some(&placed.key) {
                p.focus_ring(placed.rect, Shape::Rounded(radius))?;
            }
        }
        // Where a drop would insert the dragged button.
        if let Some(along) = self.indicator {
            let width = p.metric(MetricRole::FocusWidth);
            let line = if self.edge.vertical() {
                Rect::new(0.0, (along - width / 2.0).max(0.0), size.width(), width)?
            } else {
                Rect::new((along - width / 2.0).max(0.0), 0.0, width, size.height())?
            };
            p.fill_rect(line, p.color(ColorRole::Primary));
        }
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if let PointerKind::Scroll(delta) = event.kind() {
            let delta = if self.edge.vertical() || delta.x() == 0.0 {
                delta.y()
            } else {
                delta.x()
            };
            let viewport = if self.edge.vertical() {
                self.size.get().height()
            } else {
                self.size.get().width()
            };
            let next =
                (self.offset.get() + delta).clamp(0.0, (self.extent.get() - viewport).max(0.0));
            if next == self.offset.get() {
                return Ok(InputOutcome::Ignored);
            }
            self.offset.set(next);
            self.reveal_active.set(false);
            self.armed = None;
            self.indicator = None;
            if self.dragging.is_none() {
                cx.release_pointer();
            }
            cx.invalidate(Invalidation::Layout);
            return Ok(InputOutcome::Handled);
        }
        let over = (event.kind() != PointerKind::Leave && cx.pointer_inside())
            .then(|| cx.to_local(event.position()))
            .flatten()
            .and_then(|point| self.button_at(point));
        if over != self.hovered {
            self.hovered = over.clone();
            cx.invalidate(Invalidation::Paint);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                let Some(key) = over.filter(|key| self.is_allowed(key) || self.may_drag(key))
                else {
                    return Ok(InputOutcome::Ignored);
                };
                if self.armed.is_some() || self.dragging.is_some() {
                    return Ok(InputOutcome::Handled);
                }
                cx.capture_pointer()?;
                if self.is_allowed(&key) {
                    cx.request_focus();
                    self.active = Some(key.clone());
                }
                let expanded = self.link.model().borrow().is_expanded(&key);
                self.armed = Some(Armed::Pointer {
                    key,
                    at: event.position(),
                    expanded,
                });
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) => {
                let Some(Armed::Pointer { key, expanded, .. }) = self.armed.clone() else {
                    return Ok(InputOutcome::Ignored);
                };
                self.armed = None;
                cx.release_pointer();
                cx.invalidate(Invalidation::Paint);
                if over.as_ref() == Some(&key) {
                    self.activate(cx, &key, Some(expanded))?;
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Move => {
                let Some(Armed::Pointer { key, at, .. }) = self.armed.clone() else {
                    return Ok(InputOutcome::Ignored);
                };
                let (dx, dy) = (event.position().x() - at.x(), event.position().y() - at.y());
                if self.may_drag(&key) && dx.hypot(dy) > DRAG_THRESHOLD {
                    let (layout, title) = {
                        let model = self.link.model();
                        let title = model
                            .borrow()
                            .window(&key)
                            .map(|window| window.title.clone())
                            .unwrap_or_default();
                        (Rc::as_ptr(&model), title)
                    };
                    cx.start_drag(
                        DragPayload::new(ButtonDrag {
                            layout,
                            key: key.clone(),
                        }),
                        DropOperation::Move,
                        DragPreview::element(Element::new("tool-window-drag", Label::new(title))),
                    )?;
                    self.armed = None;
                    self.dragging = Some(key);
                    cx.invalidate(Invalidation::Paint);
                }
                Ok(InputOutcome::Handled)
            }
            PointerKind::Leave if self.armed.is_some() => Ok(InputOutcome::Handled),
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &DragEvent<'_>,
    ) -> Result<DropResponse, RuntimeError> {
        let model = self.link.model();
        let Some(drag) = event
            .payload()
            .get::<ButtonDrag>()
            .filter(|drag| std::ptr::eq(drag.layout, Rc::as_ptr(&model)))
        else {
            return Ok(DropResponse::Rejected);
        };
        if event.phase() == DragPhase::Leave || event.role() == DragRole::Observer {
            if self.indicator.take().is_some() {
                cx.invalidate(Invalidation::Paint);
            }
            self.set_autoscroll(cx, 0.0)?;
            return Ok(DropResponse::Rejected);
        }
        let Some(point) = cx.to_local(event.position()) else {
            return Ok(DropResponse::Rejected);
        };
        // Resting near an edge of an overflowing stripe scrolls it, as the
        // lists do, so a button can be dropped beyond the shown part.
        let speed = if event.phase() == DragPhase::Drop {
            0.0
        } else {
            self.autoscroll_at(point)
        };
        self.set_autoscroll(cx, speed)?;
        let (group, index, line) = self.drop_target(&drag.key, point);
        // Within its own group a drop reorders; anywhere else it moves. The
        // drag permission is checked again, so revoking it during a drag
        // forbids the drop.
        let permission = {
            let model = model.borrow();
            let Some(window) = model.window(&drag.key) else {
                return Ok(DropResponse::Rejected);
            };
            let placement = if window.edge == self.edge && window.group == group {
                model.reorder_permission(&drag.key)
            } else {
                model.move_permission(&drag.key)
            };
            model.drag_permission(&drag.key).and(placement)
        };
        if !permission.get()? {
            if self.indicator.take().is_some() {
                cx.invalidate(Invalidation::Paint);
            }
            return Ok(DropResponse::Forbidden);
        }
        if event.phase() == DragPhase::Drop {
            self.indicator = None;
            cx.invalidate(Invalidation::Paint);
            model::check_writable(&model)?;
            let changed = model.borrow_mut().move_window(
                &drag.key,
                self.edge,
                group,
                Some(index),
                ChangeOrigin::User,
            );
            if changed {
                model::touch(&model)?;
            }
            return Ok(DropResponse::Accepted(DropOperation::Move));
        }
        if self.indicator != Some(line) {
            self.indicator = Some(line);
            cx.invalidate(Invalidation::Paint);
        }
        Ok(DropResponse::Accepted(DropOperation::Move))
    }
    fn drag_finished(&mut self, cx: &mut EventContext, _: DragOutcome) -> Result<(), RuntimeError> {
        self.dragging = None;
        self.indicator = None;
        self.set_autoscroll(cx, 0.0)?;
        cx.invalidate(Invalidation::Paint);
        Ok(())
    }
    fn timer_elapsed(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if self.autoscroll == 0.0 || !cx.drag_active() {
            self.autoscroll = 0.0;
            return Ok(());
        }
        let viewport = if self.edge.vertical() {
            self.size.get().height()
        } else {
            self.size.get().width()
        };
        let next = (self.offset.get() + self.autoscroll)
            .clamp(0.0, (self.extent.get() - viewport).max(0.0));
        if next == self.offset.get() {
            self.autoscroll = 0.0;
            return Ok(());
        }
        // The buttons move under a still pointer; the runtime resolves the drag
        // again after this layout and the insertion line follows.
        self.offset.set(next);
        self.reveal_active.set(false);
        cx.invalidate(Invalidation::Layout);
        cx.set_timer(crate::virtual_list::AUTOSCROLL_INTERVAL)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keys: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keys.control_key(event);
        if !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(control, KeyCode::Enter | KeyCode::Space)
            && crate::activation::key_result(control, self.armed.is_some(), cx.modifiers())
                == InputOutcome::Ignored
        {
            return Ok(InputOutcome::Ignored);
        }
        match control {
            KeyCode::Escape => {
                if self.armed.take().is_some() {
                    cx.release_pointer();
                    cx.invalidate(Invalidation::Paint);
                    Ok(InputOutcome::Handled)
                } else {
                    Ok(InputOutcome::Ignored)
                }
            }
            KeyCode::Enter | KeyCode::Space => match event.transition {
                KeyTransition::Pressed if self.armed.is_none() => {
                    if let Some(key) = self.active.clone().filter(|key| self.is_allowed(key)) {
                        self.armed = Some(Armed::Key(event.code, key));
                        cx.invalidate(Invalidation::Paint);
                    }
                    Ok(InputOutcome::Handled)
                }
                KeyTransition::Released => {
                    if let Some(Armed::Key(code, key)) = self.armed.clone()
                        && code == event.code
                    {
                        self.armed = None;
                        cx.invalidate(Invalidation::Paint);
                        self.activate(cx, &key, None)?;
                    }
                    Ok(InputOutcome::Handled)
                }
                _ => Ok(InputOutcome::Handled),
            },
            code if matches!(
                event.transition,
                KeyTransition::Pressed | KeyTransition::Repeated
            ) && self.armed.is_none()
                // Modified navigation chords are left to command bindings.
                && keys.chord_modifiers(event) == Modifiers::default()
                && self.step(cx, code) =>
            {
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn input_reset(&mut self) {
        self.reset_input();
    }
    fn focus_gained(&mut self, _: &mut EventContext) {
        self.owns_focus = true;
        self.link.model().borrow_mut().stripe_focus = true;
    }
    fn focus_lost(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        self.cancel_input(cx);
        self.give_up_focus();
        Ok(())
    }
    fn visibility_changed(&mut self, visible: bool) {
        // Hiding and unmounting send no focus-loss notification.
        if !visible {
            self.give_up_focus();
        }
    }
    fn unmounted(&mut self) {
        self.give_up_focus();
    }
    fn inspect(&self) -> ComponentDetails {
        let model = self.link.model();
        let model = model.borrow();
        let items = self
            .buttons_with(&model)
            .into_iter()
            .map(|(key, title)| {
                let expanded = model.is_expanded(&key);
                let allowed = self.is_allowed(&key);
                let item = SemanticItem::new(title)
                    .selected(self.active.as_ref() == Some(&key))
                    .disabled(!allowed)
                    .expanded(expanded);
                if allowed {
                    item.action(if expanded {
                        SemanticAction::Collapse
                    } else {
                        SemanticAction::Expand
                    })
                } else {
                    item
                }
            })
            .collect::<Vec<_>>();
        let mut details = ComponentDetails {
            semantics: Some(
                Semantics::new(SemanticRole::Toolbar)
                    .name(self.name.as_str())
                    .items(items),
            ),
            ..ComponentDetails::default()
        };
        details
            .properties
            .insert("names".into(), InspectValue::Bool(model.names));
        details.properties.insert(
            "scroll_offset".into(),
            InspectValue::Number(f64::from(self.offset.get())),
        );
        details
    }
}
impl Stripe {
    fn buttons_with(&self, model: &model::Model) -> Vec<(String, String)> {
        let mut buttons: Vec<&model::Window> = model
            .windows
            .iter()
            .filter(|window| window.edge == self.edge)
            .collect();
        buttons.sort_by_key(|window| window.group == StripeGroup::End);
        buttons
            .into_iter()
            .map(|window| (window.key.clone(), window.title.clone()))
            .collect()
    }
}
