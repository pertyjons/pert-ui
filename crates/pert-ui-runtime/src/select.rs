//! Selection lists use the shared anchored surface machinery. The option list
//! here also presents the rows of the menus in [`crate::menu`].
use crate::activation::{ActivationState, activation_key, key_result};
use crate::choice_controls::{focus_ring, rounded, validate};
use crate::control_binding::{ValueBinding, same_binding};
use crate::layout::{centered, rect};
use crate::menu::{MarkStyle, MenuAction, MenuRows, Shortcut};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Chosen<T> = Box<dyn FnMut(&T, &mut EventContext) -> Result<(), RuntimeError>>;
/// Row positions of an option list. A menu branch shares them to place a
/// submenu beside its row; a separator adds a band before its row.
#[derive(Default)]
pub(crate) struct RowGeometry {
    row: Cell<f32>,
    band: Cell<f32>,
    pub(crate) offset: Cell<f32>,
    /// Indices of the rows preceded by a separator band, ascending and never 0.
    /// A reconciled menu list takes a new description's separators.
    separators: RefCell<Vec<usize>>,
    /// Number of rows, whose total extent bounds the offset.
    count: Cell<usize>,
    /// A row the list asked to scroll into view at the next arrangement.
    reveal: Cell<Option<usize>>,
}
impl RowGeometry {
    /// Scrolls row `index` into a viewport of `height`; returns whether the
    /// offset changed.
    fn reveal_row(&self, index: usize, height: f32) -> bool {
        let before = self.offset.get();
        let top = self.top(index);
        let bottom = top + self.row.get();
        if top < self.offset.get() {
            self.offset.set(top);
        } else if bottom > self.offset.get() + height {
            self.offset.set((bottom - height).max(0.0));
        }
        self.clamp(height);
        self.offset.get() != before
    }
    fn clamp(&self, height: f32) {
        let most = (self.top(self.count.get()) - height).max(0.0);
        self.offset.set(self.offset.get().clamp(0.0, most));
    }
    /// Applies a requested reveal for a viewport of `height`, then keeps the
    /// offset inside the rows. Whichever arrangement comes first applies it: a
    /// menu branch, which places the list at its own size and an open submenu
    /// from the offset, or the list itself. Before the rows were ever measured
    /// the request waits for the list's own arrangement; no submenu can be open
    /// from rows nobody has seen.
    pub(crate) fn settle(&self, height: f32) {
        if self.row.get() > 0.0
            && let Some(index) = self.reveal.take()
        {
            self.reveal_row(index, height);
        }
        self.clamp(height);
    }
    /// Unscrolled top of a row; the top of the row count is the list's height.
    pub(crate) fn top(&self, index: usize) -> f32 {
        let bands = self
            .separators
            .borrow()
            .iter()
            .take_while(|s| **s <= index)
            .count();
        index as f32 * self.row.get() + bands as f32 * self.band.get()
    }
    /// The row under an unscrolled vertical position; none over a separator.
    fn index_at(&self, y: f32, count: usize) -> Option<usize> {
        (0..count).find(|index| {
            let top = self.top(*index);
            y >= top && y < top + self.row.get()
        })
    }
}
/// A menu row's bound text and the decorations drawn around it.
pub(crate) struct LiveLabel {
    pub(crate) index: usize,
    pub(crate) text: TextSource,
    pub(crate) leading: Option<IconSource>,
    pub(crate) submenu: bool,
    /// The text last read from the binding, kept across redescriptions with
    /// the same binding.
    pub(crate) read: Option<RichText>,
}
impl LiveLabel {
    /// The row's content: `text` with its leading icon and submenu chevron.
    fn compose(&self, text: RichText) -> RichText {
        let mut content = text;
        if let Some(icon) = &self.leading {
            content = content.leading_icon(icon.clone());
        }
        if self.submenu {
            content = content.trailing_icon(Icon::ChevronRight);
        }
        content
    }
}

pub(crate) struct OptionList<T> {
    options: Vec<Choice<T>>,
    source: Option<ReadSignal<Vec<Choice<T>>>>,
    /// Per-option live enablement, by index into `options`. Menus use it so an
    /// action can gray out while its list stays retained.
    pub(crate) enablement: Vec<(usize, crate::Permission)>,
    /// Per-option bound text, by index into `options`, for menu rows whose
    /// label is a binding.
    pub(crate) labels: Vec<LiveLabel>,
    selected: Option<Rc<dyn ValueBinding<T>>>,
    /// False when the owning control was described as disabled: an open list
    /// then refuses choices like a read-only binding does.
    enabled: bool,
    availability: Permission,
    editable: bool,
    active: Option<T>,
    pressed: Option<T>,
    retargeted: bool,
    input: ActivationState,
    on_choose: Chosen<T>,
    pub(crate) on_right: Option<Chosen<T>>,
    pub(crate) on_left: Option<MenuAction>,
    /// Present when the rows are menu commands rather than options of a value.
    pub(crate) menu: Option<MenuRows>,
    text: RefCell<Vec<TextLayout>>,
    pub(crate) geometry: Rc<RowGeometry>,
    height: Cell<f32>,
}
impl<T: Clone + Eq + 'static> OptionList<T> {
    fn new(
        options: Vec<Choice<T>>,
        selected: Option<Rc<dyn ValueBinding<T>>>,
        on_choose: Chosen<T>,
    ) -> Self {
        Self::with_separators(options, selected, on_choose, Vec::new())
    }
    pub(crate) fn with_separators(
        options: Vec<Choice<T>>,
        selected: Option<Rc<dyn ValueBinding<T>>>,
        on_choose: Chosen<T>,
        separators: Vec<usize>,
    ) -> Self {
        Self {
            options,
            source: None,
            enablement: Vec::new(),
            labels: Vec::new(),
            selected,
            enabled: true,
            availability: Permission::allowed(),
            editable: true,
            active: None,
            pressed: None,
            retargeted: false,
            input: ActivationState::default(),
            on_choose,
            on_right: None,
            on_left: None,
            menu: None,
            text: RefCell::new(Vec::new()),
            geometry: Rc::new(RowGeometry {
                separators: RefCell::new(separators),
                ..RowGeometry::default()
            }),
            height: Cell::new(0.0),
        }
    }
    fn max_offset(&self) -> f32 {
        (self.geometry.top(self.options.len()) - self.height.get()).max(0.0)
    }
    fn active_index(&self) -> Option<usize> {
        self.active
            .as_ref()
            .and_then(|v| self.options.iter().position(|c| c.value() == v))
    }
    /// Scrolls the active option into view now, from an event; returns whether
    /// the offset changed. A menu branch places its open submenu from the
    /// offset, so a caller that changes it invalidates layout rather than paint.
    fn reveal_active(&self) -> bool {
        self.geometry.count.set(self.options.len());
        match self.active_index() {
            Some(index) => self.geometry.reveal_row(index, self.height.get()),
            None => false,
        }
    }
    fn hit(&self, cx: &EventContext, e: PointerEvent) -> Option<T> {
        if !cx.pointer_inside() || self.geometry.row.get() <= 0.0 {
            return None;
        }
        let local = cx.to_local(e.position())?;
        let index = self
            .geometry
            .index_at(local.y() + self.geometry.offset.get(), self.options.len())?;
        self.options
            .get(index)
            .filter(|o| o.is_enabled())
            .map(|o| o.value().clone())
    }
    /// Widths a menu adds around its labels: the mark column before them and the
    /// shortcut column, with its separating gap, after them.
    fn columns(&self, cx: &MeasureContext<'_>) -> Result<(f32, f32), RuntimeError> {
        let Some(menu) = &self.menu else {
            return Ok((0.0, 0.0));
        };
        let lead = if menu.marked() {
            cx.metric(MetricRole::IconSize) + cx.metric(MetricRole::IconGap)
        } else {
            0.0
        };
        let mut widest: f32 = 0.0;
        for shown in menu.extras.iter().filter_map(|extra| extra.shown.as_ref()) {
            widest = widest.max(cx.text(shown, None, TextStyle::Body)?.size().width());
        }
        let trail = if widest > 0.0 {
            cx.metric(MetricRole::SectionGap) + widest
        } else {
            0.0
        };
        Ok((lead, trail))
    }
    /// Takes a new description of a menu level's rows: their options, marks,
    /// shortcuts, enablement, separators and callbacks. The callbacks reach the
    /// mounted level through its shared state, which the level's own
    /// reconciliation points at. The highlight is kept while its row survives,
    /// and so are the scroll offset and the measured row height.
    fn reconcile_menu(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            options,
            enablement,
            labels,
            on_choose,
            on_right,
            on_left,
            menu,
            geometry,
            ..
        } = next;
        let (Some(mine), Some(menu)) = (&mut self.menu, menu) else {
            // A list never changes between presenting options and menu rows.
            return None;
        };
        // A row described with the same bound label keeps the text it read.
        let (mut options, mut labels) = (options, labels);
        for live in &mut labels {
            let retained = self
                .labels
                .iter()
                .find(|old| old.index == live.index && old.text.same_as(&live.text))
                .and_then(|old| old.read.clone());
            if let Some(read) = retained
                && let Some(option) = options.get_mut(live.index)
            {
                option.set_content(live.compose(read.clone()));
                live.read = Some(read);
            }
        }
        let separators = geometry.separators.take();
        let rows_changed = mine.adopt(menu)
            | (*self.geometry.separators.borrow() != separators)
            | (self.options.len() != options.len())
            | self.options.iter().zip(&options).any(|(a, b)| {
                a.value() != b.value()
                    || a.content() != b.content()
                    || a.is_enabled() != b.is_enabled()
            });
        *self.geometry.separators.borrow_mut() = separators;
        self.options = options;
        self.enablement = enablement;
        self.labels = labels;
        self.on_choose = on_choose;
        self.on_right = on_right;
        self.on_left = on_left;
        if !rows_changed {
            // Marks, shortcuts and enablement are read again in the update
            // this schedules.
            return Some(Invalidation::Paint);
        }
        // Pending activation belongs to the old rows.
        self.retargeted = true;
        self.pressed = None;
        self.input.reset();
        self.geometry.count.set(self.options.len());
        Some(Invalidation::Layout)
    }
}
impl<T: Clone + Eq + 'static> Component for OptionList<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if self.menu.is_some() || next.menu.is_some() {
            return self.reconcile_menu(next);
        }
        // A Select list's choice callback commits through the target its
        // anchor retargets, so the retained callback stays valid.
        let Self {
            options,
            source,
            selected,
            enabled: _,
            availability,
            ..
        } = next;
        let mut changed = false;
        let availability_changed = !self.availability.same_as(&availability);
        if availability_changed {
            if let Some(enabled) = availability.fixed() {
                self.enabled = enabled;
                if !enabled {
                    self.editable = false;
                    self.pressed = None;
                    self.input.reset();
                }
            }
            self.availability = availability;
        }
        let same_selected = match (&self.selected, &selected) {
            (Some(a), Some(b)) => same_binding(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same_selected {
            self.selected = selected;
            changed = true;
        }
        if !ReadSignal::same_optional(&self.source, &source) {
            self.source = source;
            changed = true;
        }
        // Fixed options apply unless the binding or a dynamic source supplies them.
        let supplied = self
            .selected
            .as_ref()
            .is_some_and(|binding| binding.supplies_options());
        if self.source.is_none() && !supplied && self.options != options {
            self.options = options;
            changed = true;
        }
        if !changed {
            return availability_changed.then_some(Invalidation::Paint);
        }
        // Pending activation belongs to the old description.
        self.pressed = None;
        self.input.reset();
        self.active = None;
        Some(Invalidation::Layout)
    }
    fn focusable(&self) -> bool {
        self.editable && self.options.iter().any(Choice::is_enabled)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if std::mem::take(&mut self.retargeted) {
            cx.cancel_input();
        }
        let mut reveal = false;
        // Reading here subscribes this list, so a changed decision reevaluates and
        // repaints it without any polling.
        for (index, enabled) in &self.enablement {
            let enabled = enabled.read(cx)?;
            if let Some(option) = self.options.get_mut(*index)
                && option.is_enabled() != enabled
            {
                option.set_enabled(enabled);
                self.pressed = None;
                self.input.reset();
                cx.cancel_input();
            }
        }
        for live in &mut self.labels {
            let read = live.text.read(cx)?;
            let content = live.compose(read.clone());
            live.read = Some(read);
            if let Some(option) = self.options.get_mut(live.index)
                && option.content() != &content
            {
                option.set_content(content);
                cx.request_layout();
            }
        }
        if let Some(menu) = &mut self.menu {
            for extra in &mut menu.extras {
                if let Some(mark) = &extra.mark {
                    extra.checked = (mark.read)(cx)?;
                }
                let shown = match &extra.shortcut {
                    None => None,
                    Some(Shortcut::Fixed(chord)) => Some(chord.to_string()),
                    Some(Shortcut::Live(chords)) => {
                        cx.read(chords)?.first().map(ToString::to_string)
                    }
                };
                if shown != extra.shown {
                    extra.shown = shown;
                    cx.request_layout();
                }
            }
        }
        let reading = self
            .selected
            .as_ref()
            .map(|binding| binding.read(cx))
            .transpose()?;
        let enabled = self.availability.read_paint(cx)?;
        let editable = enabled && reading.as_ref().is_none_or(|reading| reading.editable);
        let (selected, mut options) = match reading {
            Some(reading) => (Some(reading.value), reading.options),
            None => (None, None),
        };
        if let Some(source) = &self.source {
            // Only signal-bound descriptions have an independent option source.
            options = Some(cx.read(source)?);
        }
        if let Some(options) = options {
            validate(&options)?;
            if options != self.options {
                self.options = options;
                reveal = true;
                self.pressed = None;
                self.input.reset();
                cx.cancel_input();
                cx.request_layout();
            }
        }
        self.enabled = enabled;
        if self.editable && !editable {
            self.pressed = None;
            self.input.reset();
            cx.cancel_input();
        }
        self.editable = editable;
        if self.active.as_ref().is_none_or(|v| {
            !self
                .options
                .iter()
                .any(|c| c.value() == v && c.is_enabled())
        }) {
            reveal = true;
            self.active = selected
                .filter(|v| {
                    self.options
                        .iter()
                        .any(|c| c.value() == v && c.is_enabled())
                })
                .or_else(|| {
                    self.options
                        .iter()
                        .find(|c| c.is_enabled())
                        .map(|c| c.value().clone())
                });
        }
        self.input.focused = cx.focused();
        self.geometry.count.set(self.options.len());
        // Revealed at the next arrangement, where the height the list is shown
        // at is known and before a menu branch places an open submenu.
        if reveal && let Some(index) = self.active_index() {
            self.geometry.reveal.set(Some(index));
            cx.request_layout();
        }
        Ok(())
    }
    fn visibility_changed(&mut self, visible: bool) {
        if visible {
            self.active = None;
            self.geometry.offset.set(0.0);
        }
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let mut label: f32 = 0.0;
        let mut h = cx.metric(MetricRole::ControlHeight);
        for c in &self.options {
            let t = cx.rich_text(c.content(), None, TextStyle::Body)?;
            label = label.max(t.size().width());
            h = h.max(t.size().height() + 2.0 * cx.metric(MetricRole::ControlPaddingY));
        }
        let (lead, trail) = self.columns(cx)?;
        let width = cx
            .metric(MetricRole::ControlWidth)
            .max(2.0 * cx.metric(MetricRole::ControlPaddingX) + lead + label + trail);
        // Up to eight rows show before the list scrolls, with their separators.
        let shown = self.options.len().clamp(1, 8);
        let bands = self
            .geometry
            .separators
            .borrow()
            .iter()
            .filter(|index| **index < shown)
            .count();
        Ok(Size::new(
            width,
            h * shown as f32 + bands as f32 * cx.metric(MetricRole::ItemGap),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        let texts: Vec<_> = self
            .options
            .iter()
            .map(|c| {
                cx.measurement()
                    .rich_text(c.content(), None, TextStyle::Body)
            })
            .collect::<Result<_, _>>()?;
        let m = cx.measurement();
        self.geometry.row.set(
            texts
                .iter()
                .fold(m.metric(MetricRole::ControlHeight), |h, t| {
                    h.max(t.size().height() + 2.0 * m.metric(MetricRole::ControlPaddingY))
                }),
        );
        self.geometry.band.set(m.metric(MetricRole::ItemGap));
        self.height.set(s.height());
        *self.text.borrow_mut() = texts;
        if let Some(menu) = &self.menu {
            let shortcuts = menu
                .extras
                .iter()
                .map(|extra| {
                    extra
                        .shown
                        .as_ref()
                        .map(|shown| m.text(shown, None, TextStyle::Body))
                        .transpose()
                })
                .collect::<Result<_, _>>()?;
            *menu.shortcut_text.borrow_mut() = shortcuts;
        }
        self.geometry.count.set(self.options.len());
        self.geometry.settle(s.height());
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.editable {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(e.kind(),PointerKind::Press(b)|PointerKind::Release(b) if b!=PointerButton::Primary)
        {
            return Ok(InputOutcome::Ignored);
        }
        let activate = self.input.pointer(cx, e)?;
        match e.kind() {
            PointerKind::Move => {
                if let Some(value) = self.hit(cx, e) {
                    self.active = Some(value);
                    cx.invalidate(Invalidation::Paint);
                }
            }
            PointerKind::Press(PointerButton::Primary) => self.pressed = self.hit(cx, e),
            PointerKind::Release(PointerButton::Primary) => {
                if let Some(value) = self
                    .pressed
                    .take()
                    .filter(|v| activate && self.hit(cx, e).as_ref() == Some(v))
                {
                    (self.on_choose)(&value, cx)?;
                }
            }
            PointerKind::Scroll(delta) => {
                let offset = &self.geometry.offset;
                let next = (offset.get() + delta.y()).clamp(0.0, self.max_offset());
                // The branch places an open submenu from the offset.
                if offset.replace(next) != next {
                    cx.invalidate(Invalidation::Layout);
                }
            }
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
        if !self.editable {
            return Ok(InputOutcome::Ignored);
        }
        let control = keys.control_key(e);
        // Modified navigation chords are left to command bindings.
        let plain = keys.chord_modifiers(e) == Modifiers::default();
        if !plain
            && matches!(
                control,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Home
                    | KeyCode::End
                    | KeyCode::PageUp
                    | KeyCode::PageDown
            )
        {
            return Ok(InputOutcome::Ignored);
        }
        if control == KeyCode::ArrowRight && self.on_right.is_some() {
            if e.transition == KeyTransition::Pressed
                && let (Some(action), Some(active)) = (&mut self.on_right, &self.active)
            {
                action(active, cx)?;
            }
            return Ok(InputOutcome::Handled);
        }
        if control == KeyCode::ArrowLeft && self.on_left.is_some() {
            if e.transition == KeyTransition::Pressed
                && let Some(action) = &mut self.on_left
            {
                action(cx)?;
            }
            return Ok(InputOutcome::Handled);
        }
        if activation_key(control) {
            let result = key_result(
                control,
                self.input.pressed() || self.pressed.is_some(),
                cx.modifiers(),
            );
            if control == KeyCode::Escape {
                self.pressed = None;
            }
            if self.input.keyboard(cx, e, control)
                && let Some(value) = self.active.clone()
            {
                (self.on_choose)(&value, cx)?;
            }
            return Ok(result);
        }
        if !matches!(
            control,
            KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        ) {
            return Ok(InputOutcome::Ignored);
        }
        if matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            let enabled: Vec<_> = self
                .options
                .iter()
                .filter(|c| c.is_enabled())
                .map(|c| c.value().clone())
                .collect();
            if !enabled.is_empty() {
                let index = self
                    .active
                    .as_ref()
                    .and_then(|v| enabled.iter().position(|c| c == v))
                    .unwrap_or(0);
                let page = (self.height.get() / self.geometry.row.get().max(1.0))
                    .floor()
                    .max(1.0) as usize;
                let next = match control {
                    KeyCode::Home => 0,
                    KeyCode::End => enabled.len() - 1,
                    KeyCode::ArrowUp => (index + enabled.len() - 1) % enabled.len(),
                    KeyCode::PageUp => index.saturating_sub(page),
                    KeyCode::PageDown => (index + page).min(enabled.len() - 1),
                    _ => (index + 1) % enabled.len(),
                };
                self.active = Some(enabled[next].clone());
                cx.invalidate(if self.reveal_active() {
                    Invalidation::Layout
                } else {
                    Invalidation::Paint
                });
            }
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.input.reset();
        self.pressed = None;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let geometry = &self.geometry;
        let h = geometry.row.get();
        let offset = geometry.offset.get();
        let pad = p.metric(MetricRole::ControlPaddingX);
        let band = geometry.band.get();
        for index in geometry.separators.borrow().iter() {
            let top = geometry.top(*index) - band - offset;
            if top + band <= 0.0 || top >= size.height() {
                continue;
            }
            let line = p.metric(MetricRole::BorderWidth).min(band);
            p.fill_rect(
                rect(0.0, top + (band - line) / 2.0, size.width(), line)?,
                p.color(ColorRole::Divider),
            );
        }
        let lead = if self.menu.as_ref().is_some_and(MenuRows::marked) {
            p.metric(MetricRole::IconSize) + p.metric(MetricRole::IconGap)
        } else {
            0.0
        };
        let shortcuts = self.menu.as_ref().map(|menu| menu.shortcut_text.borrow());
        for (i, (c, text)) in self
            .options
            .iter()
            .zip(self.text.borrow().iter())
            .enumerate()
        {
            let top = geometry.top(i) - offset;
            if top + h <= 0.0 || top >= size.height() {
                continue;
            }
            let active = self.active.as_ref() == Some(c.value());
            let rect = rect(0.0, top, size.width(), h)?;
            if active {
                p.fill_rect(rect, p.color(ColorRole::Primary));
            }
            let color = p.color(if !self.editable || !c.is_enabled() {
                ColorRole::Muted
            } else if active {
                ColorRole::OnPrimary
            } else {
                ColorRole::OnSurface
            });
            text.paint(
                p,
                crate::Point::new(pad + lead, top + (h - text.size().height()) / 2.0)?,
                color,
            )?;
            if let Some(extra) = self.menu.as_ref().and_then(|menu| menu.extras.get(i))
                && extra.checked
                && let Some(mark) = &extra.mark
            {
                let side = p.metric(MetricRole::IconSize).min(h);
                match mark.style {
                    MarkStyle::Check => p.icon(
                        Icon::Check,
                        Rect::new(pad, top + (h - side) / 2.0, side, side)?,
                        color,
                    )?,
                    MarkStyle::Radio => {
                        let dot = side * 0.4;
                        p.fill_shape(
                            Rect::new(pad + (side - dot) / 2.0, top + (h - dot) / 2.0, dot, dot)?,
                            rounded(dot / 2.0)?,
                            color,
                        )?;
                    }
                }
            }
            if let Some(Some(shortcut)) = shortcuts.as_ref().and_then(|all| all.get(i)) {
                shortcut.paint(
                    p,
                    crate::Point::new(
                        size.width() - pad - shortcut.size().width(),
                        top + (h - shortcut.size().height()) / 2.0,
                    )?,
                    p.color(if active && self.editable && c.is_enabled() {
                        ColorRole::OnPrimary
                    } else {
                        ColorRole::Muted
                    }),
                )?;
            }
            if active && self.input.focused {
                focus_ring(p, rect)?;
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.validation.interactive = self.focusable();
        d.properties.insert(
            "scroll_offset".into(),
            InspectValue::Number(self.geometry.offset.get().into()),
        );
        d.properties.insert(
            "options".into(),
            InspectValue::Number(self.options.len() as f64),
        );
        if let Some(c) = self
            .options
            .iter()
            .find(|c| self.active.as_ref() == Some(c.value()))
        {
            d.properties
                .insert("active".into(), InspectValue::Text(c.label().into()));
        }
        let active = self
            .options
            .iter()
            .position(|c| self.active.as_ref() == Some(c.value()));
        // A menu's rows are commands that run; a list's rows are options of a
        // value. A row that opens a submenu expands instead of running.
        let open = self.menu.as_ref().and_then(MenuRows::open_key);
        let items = self.options.iter().enumerate().map(|(index, option)| {
            let mut item = SemanticItem::new(option.label())
                .selected(active == Some(index))
                .disabled(!self.editable || !option.is_enabled());
            let Some(menu) = &self.menu else {
                return item.action(SemanticAction::Select);
            };
            if let Some(extra) = menu.extras.get(index) {
                if extra.mark.is_some() {
                    item = item.checked(extra.checked);
                }
                if let Some(shown) = &extra.shown {
                    item = item.shortcut(shown.clone());
                }
            }
            match menu.keys.get(index) {
                Some(key) if menu.branches.contains(key) => {
                    let expanded = open.as_ref() == Some(key);
                    item.expanded(expanded).action(if expanded {
                        SemanticAction::Collapse
                    } else {
                        SemanticAction::Expand
                    })
                }
                _ => item.action(SemanticAction::Activate),
            }
        });
        d.semantics = Some(
            Semantics::new(if self.menu.is_some() {
                SemanticRole::Menu
            } else {
                SemanticRole::ListBox
            })
            .value(SemanticValue::Choice {
                selected: active,
                count: self.options.len(),
                label: active.map(|index| self.options[index].label().to_owned()),
            })
            .items(items)
            .disabled(!self.focusable())
            .enabled_actions([
                if self.menu.is_some() {
                    SemanticAction::Activate
                } else {
                    SemanticAction::Select
                },
                SemanticAction::Dismiss,
            ]),
        );
        d
    }
}
/// What a Select list commits to, shared by the anchor that opens the list and
/// the list's choice callback, so reconciling the anchor retargets both.
type SelectHandler<T> = Box<dyn FnMut(&mut EventContext, &T) -> Result<(), RuntimeError>>;
struct SelectTarget<T> {
    on_change: RefCell<Option<SelectHandler<T>>>,
    /// The host's edit this dropdown reports into ([`Select::session`]).
    session: RefCell<Option<EditorAttachment>>,
    /// The edit attached when the list opened: its choice completes that edit,
    /// not one the host began after reattaching the anchor.
    opened: RefCell<Option<EditorAttachment>>,
    /// A parameter dropdown ignores sessions: a refused submission and an
    /// unchanged choice look alike.
    parameter: Cell<bool>,
    binding: RefCell<Rc<dyn ValueBinding<T>>>,
    /// The revision observed when the list opened. `None` after the anchor was
    /// rebound while the list was open: a choice from that list only closes it.
    observed: Cell<Option<Revision>>,
}
struct SelectAnchor<T> {
    value: Rc<dyn ValueBinding<T>>,
    editable: bool,
    revision: Revision,
    target: Rc<SelectTarget<T>>,
    options: Vec<Choice<T>>,
    source: Option<ReadSignal<Vec<Choice<T>>>>,
    open: Signal<bool>,
    enabled: bool,
    availability: Permission,
    input: ActivationState,
    prepared: RefCell<Option<TextLayout>>,
    text: RichText,
    inline: bool,
    /// An inline anchor's caption, from `InlineSelect::label`.
    label: Caption,
    /// A choice parameter's name, the accessible name without a label.
    parameter_name: Option<String>,
    hint: Option<String>,
    /// Shown for a value that none of the options holds; by default the
    /// installed string table's [`StringRole::SelectPlaceholder`](crate::StringRole::SelectPlaceholder).
    placeholder: TextSource,
    caption: RefCell<Option<TextLayout>>,
}
impl<T: Clone + Eq + 'static> SelectAnchor<T> {
    /// The keys a closed trigger with a session keeps: Escape cancels the
    /// edit and an unmodified Tab confirms it unchanged, each reported once.
    /// `None` leaves the key to ordinary handling, as for an armed press.
    fn session_key(
        &mut self,
        e: KeyEvent,
        control: KeyCode,
        keys: &KeyboardState,
    ) -> Option<InputOutcome> {
        let session = self.target.session.borrow().clone()?;
        if !matches!(
            e.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return None;
        }
        let pressed = e.transition == KeyTransition::Pressed;
        let modifiers = keys.modifiers();
        match control {
            KeyCode::Escape if !self.input.pressed() => {
                if pressed {
                    session.report(EditorTrigger::Escape, EditorOutcome::Cancelled);
                }
                Some(InputOutcome::Handled)
            }
            KeyCode::Tab if !(modifiers.control || modifiers.alt || modifiers.super_key) => {
                if pressed {
                    session.report(
                        EditorTrigger::Tab {
                            direction: crate::FocusDirection::from_shift(modifiers.shift),
                        },
                        EditorOutcome::Unchanged,
                    );
                }
                Some(InputOutcome::Handled)
            }
            _ => None,
        }
    }
    fn open_list(&self) -> Result<(), RuntimeError> {
        self.target.observed.set(Some(self.revision));
        *self.target.opened.borrow_mut() = self.target.session.borrow().clone();
        self.open.set(true)?;
        Ok(())
    }
    /// The widest value presentation the inline anchor must reserve.
    fn reserved_width(&self, cx: &MeasureContext<'_>) -> Result<f32, RuntimeError> {
        if let Some(hint) = &self.hint {
            let text = RichText::from(hint.clone()).trailing_icon(Icon::ChevronDown);
            return Ok(cx.rich_text(&text, None, TextStyle::Body)?.size().width());
        }
        let mut width: f32 = cx
            .rich_text(&self.text, None, TextStyle::Body)?
            .size()
            .width();
        for option in &self.options {
            let text = option.content().clone().trailing_icon(Icon::ChevronDown);
            width = width.max(cx.rich_text(&text, None, TextStyle::Body)?.size().width());
        }
        Ok(width)
    }
    fn caption_width(&self, cx: &MeasureContext<'_>) -> Result<f32, RuntimeError> {
        if self.label.text().is_empty() {
            return Ok(0.0);
        }
        Ok(cx
            .rich_text(&self.label.display(), None, TextStyle::Caption)?
            .size()
            .width()
            + cx.metric(MetricRole::IconGap))
    }
}
impl<T: Clone + Eq + 'static> Component for SelectAnchor<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            value,
            options,
            source,
            enabled: _,
            availability,
            inline,
            label,
            hint,
            placeholder,
            editable: _,
            revision: _,
            // The retained list's callback and popover share these.
            target,
            open: _,
            input: _,
            prepared: _,
            text: _,
            caption: _,
            parameter_name: _,
        } = next;
        *self.target.on_change.borrow_mut() = target.on_change.borrow_mut().take();
        *self.target.session.borrow_mut() = target.session.borrow_mut().take();
        // The wrapper type fixes the presentation; it never changes here.
        debug_assert_eq!(inline, self.inline);
        let mut invalidation = None;
        if !same_binding(&self.value, &value) {
            // Cancel pending activation, and refuse a choice from a list opened
            // on the old target instead of committing it to the new one.
            self.input.reset();
            self.value = value.clone();
            *self.target.binding.borrow_mut() = value;
            self.target.observed.set(None);
            self.revision = Revision::INITIAL;
            invalidation = Some(Invalidation::Layout);
        }
        if !ReadSignal::same_optional(&self.source, &source) {
            self.source = source;
            invalidation = Some(Invalidation::Layout);
        }
        if self.source.is_none() && !self.value.supplies_options() && self.options != options {
            self.options = options;
            invalidation = Some(Invalidation::Layout);
        }
        let caption_changed = self.label.reconcile(label);
        if caption_changed || hint != self.hint {
            self.hint = hint;
            invalidation = Some(Invalidation::Layout);
        }
        if !placeholder.same_as(&self.placeholder) {
            // The shown text is chosen again in the update this schedules.
            self.placeholder = placeholder;
            invalidation = Some(Invalidation::Layout);
        }
        if !self.availability.same_as(&availability) {
            if let Some(enabled) = availability.fixed() {
                self.enabled = enabled;
                if !enabled {
                    self.input.reset();
                }
            }
            self.availability = availability;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        invalidation
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.enabled && self.editable && self.options.iter().any(Choice::is_enabled)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let enabled = self.availability.read_paint(cx)?;
        let reading = self.value.read(cx)?;
        let mut options = reading.options;
        if let Some(source) = &self.source {
            // Only signal-bound descriptions have an independent option source.
            options = Some(cx.read(source)?);
        }
        if let Some(options) = options {
            validate(&options)?;
            if self.options != options {
                self.options = options;
                cx.request_layout();
            }
        }
        self.enabled = enabled;
        self.editable = reading.editable;
        if !self.enabled || !self.editable {
            if self.input.pressed() {
                cx.cancel_input();
            }
            self.input.reset();
        }
        self.revision = reading.revision;
        if self.label.update(cx)? {
            cx.request_layout();
        }
        // The name is not drawn; inspection reads it at the next snapshot.
        self.parameter_name = reading.name;
        let value = reading.value;
        let shown = match self.options.iter().find(|c| c.value() == &value) {
            Some(choice) => choice.content().clone(),
            // A binding or role is read, and so subscribed, only while shown.
            None => self.placeholder.read(cx)?,
        };
        let text = shown.trailing_icon(Icon::ChevronDown);
        if self.text != text {
            self.text = text;
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        Ok(())
    }

    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let _ = a;
        if self.inline {
            let pad = cx.metric(MetricRole::ControlPaddingX) * 0.5;
            return Ok(Size::new(
                2.0 * pad + self.caption_width(cx)? + self.reserved_width(cx)?,
                cx.metric(MetricRole::ControlHeight),
            )?);
        }
        let t = cx.rich_text(&self.text, None, TextStyle::Body)?.size();
        Ok(Size::new(
            (t.width() + 2.0 * cx.metric(MetricRole::ControlPaddingX))
                .max(cx.metric(MetricRole::ControlWidth)),
            (t.height() + 2.0 * cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        let _ = s;
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.prepared.borrow_mut() = Some(cx.measurement().rich_text(
            &self.text,
            None,
            TextStyle::Body,
        )?);
        *self.caption.borrow_mut() = if self.inline && !self.label.text().is_empty() {
            Some(
                cx.measurement()
                    .rich_text(&self.label.display(), None, TextStyle::Caption)?,
            )
        } else {
            None
        };
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.focusable()
            || matches!(e.kind(),PointerKind::Press(b)|PointerKind::Release(b) if b!=PointerButton::Primary)
            || matches!(e.kind(), PointerKind::Scroll(_) | PointerKind::Pinch(_))
        {
            return Ok(InputOutcome::Ignored);
        }
        if self.input.pointer(cx, e)? {
            self.open_list()?;
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
        if let Some(outcome) = self.session_key(e, control, keys) {
            return Ok(outcome);
        }
        if e.requests_context_menu(keys.modifiers()) {
            // The list is this control's menu, so the keyboard context-menu
            // request opens it rather than bubbling to an enclosing ContextMenu.
            if e.transition == KeyTransition::Pressed {
                self.open_list()?;
            }
            Ok(InputOutcome::Handled)
        } else if activation_key(control) {
            let result = key_result(control, self.input.pressed(), cx.modifiers());
            if self.input.keyboard(cx, e, control) {
                self.open_list()?;
            }
            Ok(result)
        } else if matches!(control, KeyCode::ArrowDown | KeyCode::ArrowUp)
            && keys.chord_modifiers(e) == Modifiers::default()
        {
            if e.transition == KeyTransition::Pressed {
                self.open_list()?;
            }
            Ok(InputOutcome::Handled)
        } else {
            Ok(InputOutcome::Ignored)
        }
    }
    fn text_key_event(
        &mut self,
        _: &mut EventContext,
        input: CommandKeyInput,
        transition: KeyTransition,
        keys: &KeyboardState,
    ) -> Result<TextKeyOutcome, RuntimeError> {
        // With a session the closed trigger keeps Escape and Tab ahead of
        // surfaces and capture bindings; otherwise it is no editor.
        let event = input.editing_event(transition);
        if self.focusable()
            && matches!(input.physical, KeyCode::Escape | KeyCode::Tab)
            && self.session_key(event, input.physical, keys) == Some(InputOutcome::Handled)
        {
            return Ok(TextKeyOutcome::Handled);
        }
        Ok(TextKeyOutcome::NotAnEditor)
    }
    fn tab_navigation(
        &mut self,
        _: &mut EventContext,
        direction: crate::FocusDirection,
        transition: crate::KeyTransition,
    ) -> Result<InputOutcome, RuntimeError> {
        let Some(session) = self.target.session.borrow().clone() else {
            return Ok(InputOutcome::Ignored);
        };
        if transition == crate::KeyTransition::Pressed {
            session.report(EditorTrigger::Tab { direction }, EditorOutcome::Unchanged);
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.input.reset();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(s) = p.size() else {
            return Ok(());
        };
        let rect = Rect::from_size(s);
        if self.inline {
            if self.input.hovered && self.focusable() {
                p.fill_shape(
                    rect,
                    rounded(p.metric(MetricRole::CornerRadius))?,
                    p.color(ColorRole::SurfaceHover),
                )?;
            }
            let caption = self.caption.borrow();
            let prepared = self.prepared.borrow();
            let (above, below) = [caption.as_ref(), prepared.as_ref()]
                .into_iter()
                .flatten()
                .filter_map(TextLayout::first_line_box)
                .fold((0.0_f32, 0.0_f32), |(above, below), (a, b)| {
                    (above.max(a), below.max(b))
                });
            let baseline = (s.height() - above - below).max(0.0) / 2.0 + above;
            let mut x = p.metric(MetricRole::ControlPaddingX) * 0.5;
            if let Some(caption) = caption.as_ref() {
                caption.paint(
                    p,
                    crate::Point::new(x, baseline - caption.first_baseline().unwrap_or(0.0))?,
                    p.color(ColorRole::Muted),
                )?;
                x += caption.size().width() + p.metric(MetricRole::IconGap);
            }
            if let Some(text) = prepared.as_ref() {
                let area = Rect::new(x, 0.0, (s.width() - x).max(0.0), s.height())?;
                let y = baseline - text.first_baseline().unwrap_or(0.0);
                let color = p.color(if self.focusable() {
                    ColorRole::OnSurface
                } else {
                    ColorRole::Muted
                });
                p.with_clip(area, |p| text.paint(p, crate::Point::new(x, y)?, color))?;
            }
            if self.input.focused {
                focus_ring(p, rect)?;
            }
            return Ok(());
        }
        p.fill_shape(
            rect,
            rounded(p.metric(MetricRole::CornerRadius))?,
            p.color(if self.input.hovered {
                ColorRole::ButtonHover
            } else {
                ColorRole::Button
            }),
        )?;
        if let Some(text) = self.prepared.borrow().as_ref() {
            text.paint(
                p,
                crate::Point::new(
                    p.metric(MetricRole::ControlPaddingX),
                    centered(s.height(), text.size().height()),
                )?,
                p.color(if self.focusable() {
                    ColorRole::OnButton
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
        d.properties
            .insert("text".into(), InspectValue::Text(self.text.plain_text()));
        if self.inline {
            d.properties.insert(
                "label".into(),
                InspectValue::Text(self.label.text().to_owned()),
            );
        }
        d.validation.interactive = self.focusable();
        d.properties
            .insert("enabled".into(), InspectValue::Bool(self.focusable()));
        let current = self.value.current().ok();
        let chosen = self
            .options
            .iter()
            .position(|c| current.as_ref() == Some(c.value()));
        let open = self.open.get().unwrap_or(false);
        d.semantics = Some(
            Semantics::new(SemanticRole::ComboBox)
                .name(if self.label.text().is_empty() {
                    self.parameter_name.as_deref().unwrap_or_default()
                } else {
                    self.label.text()
                })
                .value(SemanticValue::Choice {
                    selected: chosen,
                    count: self.options.len(),
                    label: chosen.map(|index| self.options[index].label().to_owned()),
                })
                .items(self.options.iter().enumerate().map(|(index, option)| {
                    SemanticItem::new(option.label())
                        .selected(chosen == Some(index))
                        .disabled(!self.focusable() || !option.is_enabled())
                        .action(SemanticAction::Select)
                }))
                .expanded(open)
                .disabled(!self.focusable())
                .enabled_actions([if open {
                    SemanticAction::Collapse
                } else {
                    SemanticAction::Expand
                }]),
        );
        d
    }
}
/// A typed dropdown selector with retained options and standard popup ownership.
///
/// Unmodified Enter/Space, an unmodified Up/Down arrow, the ContextMenu (Menu) key or
/// Shift+F10 opens the list; the last two are the keyboard context-menu request
/// ([`KeyEvent::requests_context_menu`]), which the control consumes instead of
/// letting it bubble to an enclosing `ContextMenu`. Unmodified arrows/Home/End/Page
/// keys move the highlight without changing the value; modified chords of these
/// keys are left to command bindings; matched unmodified Enter/Space release or a primary
/// click commits once and closes. Escape/outside press cancel. Disabled options
/// are skipped. Values are stable application identities, never display indices.
/// The list shows up to eight rows and scrolls; unknown external values show a
/// placeholder without rewriting state. [`Self::new`] binds a local signal;
/// [`Select::for_parameter`] binds a choice parameter, whose specification
/// supplies the options. The tentative highlight is local navigation, never
/// host audition: choosing a different option is one begin/commit pair at the
/// revision observed when the list opened, so a revision change while the list
/// is open refuses the choice instead of retargeting it. Choosing the current
/// option only closes the list. Re-describing the dropdown under the same key
/// applies its options and binding; a list opened before the binding changed
/// only closes when chosen from. Escape is consumed only while it cancels an
/// armed activation.
///
/// The list is an anchored surface and follows the popover placement rule: it
/// needs room above or below the control. Mounted directly as a root the control
/// is allocated the whole viewport, so the list is refused with
/// [`SurfaceDismissal::NoSpace`]; mount it inside a layout container.
#[must_use]
pub struct Select<T> {
    popover: Popover,
    /// Shared with the anchor and the list's choose callback.
    target: Rc<SelectTarget<T>>,
    anchor: Option<SelectAnchor<T>>,
    list: Option<OptionList<T>>,
}
description_debug!(Select<T>);
impl<T: Clone + Eq + 'static> Select<T> {
    /// Creates a closed dropdown. `ui` supplies
    /// internal UI-local open state; the selection must belong to the same runtime.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns [`ChoiceError::DuplicateValue`] when two options have equal values.
    pub fn new(
        ui: &impl crate::StateOwner,
        value: impl Into<Signal<T>>,
        options: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<T> = value.into();
        let options: Vec<_> = options.into_iter().collect();
        validate(&options)?;
        Ok(Self::with_binding(ui, Rc::new(value), options))
    }
    fn with_binding(
        ui: &impl crate::StateOwner,
        value: Rc<dyn ValueBinding<T>>,
        options: Vec<Choice<T>>,
    ) -> Self {
        let open = ui.signal(false);
        let close = open.clone();
        let target = Rc::new(SelectTarget {
            on_change: RefCell::new(None),
            session: RefCell::new(None),
            opened: RefCell::new(None),
            parameter: Cell::new(false),
            binding: RefCell::new(value.clone()),
            observed: Cell::new(Some(Revision::INITIAL)),
        });
        let chooser = target.clone();
        let list = OptionList::new(
            options.clone(),
            Some(value.clone()),
            Box::new(move |value, cx| {
                // Release the borrow before application code can run.
                let binding = chooser.binding.borrow().clone();
                let changed = match chooser.observed.get() {
                    Some(observed) => binding.choose(cx, value.clone(), observed)?,
                    None => false,
                };
                close.set(false)?;
                // Reported before the application's callback runs, so a
                // failing callback cannot lose the completion.
                let opened = chooser.opened.borrow_mut().take();
                if let Some(session) = opened {
                    session.report(
                        EditorTrigger::Choice,
                        if changed {
                            EditorOutcome::Saved
                        } else {
                            EditorOutcome::Unchanged
                        },
                    );
                }
                if changed {
                    let handler = chooser.on_change.borrow_mut().take();
                    if let Some(mut handler) = handler {
                        let result = handler(cx, value);
                        chooser.on_change.borrow_mut().get_or_insert(handler);
                        result?;
                    }
                }
                Ok(())
            }),
        );
        let anchor = SelectAnchor {
            value,
            editable: true,
            revision: Revision::INITIAL,
            target,
            options,
            source: None,
            open: open.clone(),
            enabled: true,
            availability: Permission::allowed(),
            input: ActivationState::default(),
            prepared: RefCell::new(None),
            text: RichText::new(),
            inline: false,
            label: Caption::new(""),
            hint: None,
            placeholder: TextSource::from(crate::StringRole::SelectPlaceholder),
            caption: RefCell::new(None),
            parameter_name: None,
        };
        Self {
            popover: Popover::empty(open),
            target: anchor.target.clone(),
            anchor: Some(anchor),
            list: Some(list),
        }
    }
    /// Reports a successful user write or remote proposal submission immediately
    /// after closing the list. Same-value choices, refused edits, application writes
    /// and authority responses do not call it. Failure propagates without rollback
    /// or replay. Redescription replaces the handler without closing the list.
    /// The context belongs to the closing list: use [`EventContext::defer_detached`]
    /// for deferred application work that must survive the list becoming hidden.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, Select, Runtime};
    /// let ui = Runtime::new();
    /// let chosen = ui.signal("a");
    /// let submitted = ui.signal("a");
    /// let control = Select::new(&ui, chosen, [Choice::new("a", "A"), Choice::new("b", "B")])?
    ///     .on_change(move |_, value| submitted.set(*value));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_change(
        self,
        handler: impl FnMut(&mut EventContext, &T) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        if let Some(anchor) = &self.anchor {
            *anchor.target.on_change.borrow_mut() = Some(Box::new(handler));
        }
        self
    }
    /// Makes this dropdown the editor of a host's
    /// [`EditorSession`](crate::EditorSession), such as a table cell's: it
    /// attaches to the edit open now, so the host calls
    /// [`EditorSession::begin`](crate::EditorSession::begin) first, and
    /// reports each completion once. The edit's start does not apply to a
    /// dropdown.
    ///
    /// - A choice in the open list, by key or click, reports
    ///   [`EditorTrigger::Choice`](crate::EditorTrigger::Choice) with
    ///   `Saved`, or `Unchanged` for the current option, before `on_change`
    ///   runs.
    /// - On the closed trigger Escape reports `Cancelled` and an unmodified
    ///   Tab or Shift+Tab reports `Unchanged`; the dropdown keeps both keys,
    ///   ahead of surfaces and capture bindings. Enter, Space and the arrows
    ///   open the list as without a session.
    /// - In the open list Escape closes the list only, and Tab keeps its
    ///   behavior there.
    /// - Focus leaving the dropdown and its list reports `FocusLeft`,
    ///   `Unchanged`; moving between them is no departure.
    ///
    /// A choice completes the edit that was open when the list opened, so
    /// a list left open across a new edit cannot end that edit.
    ///
    /// A dropdown made with [`Self::for_parameter`] ignores the session and
    /// keeps its ordinary keys: its refused submission cannot be told from
    /// an unchanged choice. Replaces an earlier session.
    pub fn session(self, session: &crate::EditorSession) -> Self {
        if !self.target.parameter.get() {
            *self.target.session.borrow_mut() = Some(session.attach());
        }
        self
    }
    /// Creates a closed dropdown over reactive options; `ui` owns its open state.
    /// The value, options and owner must belong to the mounted runtime. Bindings
    /// are read during update, not construction; foreign bindings fail with
    /// [`RuntimeError::ForeignRuntime`]. Duplicate options fail with
    /// [`RuntimeError::Choice`] holding [`crate::ChoiceError::DuplicateValue`],
    /// preserving the last valid list (initially empty). Changed options cancel
    /// list activation and retain selection by value. Redescription replaces
    /// the value binding and option source, with the usual dropdown lifecycle.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, Element, Runtime, Select};
    /// let mut ui = Runtime::new();
    /// let value = ui.signal(1);
    /// let options = ui.signal(vec![Choice::new(1, "One")]);
    /// ui.mount_tree(Element::new("choice", Select::bound(&ui, value, options.read_only())))?;
    /// ui.flush(std::time::Duration::ZERO)?;
    /// # Ok::<(), pert_ui::RuntimeError>(())
    /// ```
    pub fn bound(
        ui: &impl crate::StateOwner,
        value: impl Into<Signal<T>>,
        options: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<T> = value.into();
        let options: ReadSignal<Vec<Choice<T>>> = options.into();
        let mut select = Self::with_binding(ui, Rc::new(value), Vec::new());
        if let Some(anchor) = &mut select.anchor {
            anchor.source = Some(options.clone());
        }
        if let Some(list) = &mut select.list {
            list.source = Some(options);
        }
        select
    }
    /// Excludes the anchor from automatic focus and user activation. A list
    /// that is open when a redescription disables the dropdown stays open but
    /// refuses choices and shows its rows disabled, as it does for a read-only
    /// binding; Escape and an outside press still close it.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts activation and automatic focus with a fixed or reactive permission.
    /// Repeated calls combine restrictions, and parameter editability remains an
    /// additional restriction. Bound revocation cancels armed input at update;
    /// an open list remains visible but refuses choices until reenabled or dismissed.
    /// Same-key redescription replaces the permission; fixed denial cancels
    /// immediately. Bindings must belong to the mounted runtime.
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        let permission = permission.into();
        if let Some(anchor) = &mut self.anchor {
            anchor.availability = anchor.availability.clone().and(permission.clone());
            if let Some(enabled) = anchor.availability.fixed() {
                anchor.enabled = enabled;
            }
        }
        if let Some(list) = &mut self.list {
            list.availability = list.availability.clone().and(permission);
            if let Some(enabled) = list.availability.fixed() {
                list.enabled = enabled;
                list.editable &= enabled;
            }
        }
        self
    }
    /// Replaces the text shown for a value that none of the options holds,
    /// for example with a prompt of the application's own. Without it the
    /// installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::SelectPlaceholder`](crate::StringRole::SelectPlaceholder),
    /// `"Select…"` in English. The value
    /// itself is never rewritten. Re-describing the dropdown applies a changed
    /// placeholder.
    ///
    /// The placeholder is a [`TextSource`]: fixed plain or rich text, a
    /// binding the anchor follows, or a [`StringRole`](crate::StringRole) that
    /// follows the installed string table, as the default does. It is read in
    /// the anchor's update while it is shown; describing the dropdown again
    /// with the same source keeps the text already read.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, Column, Element, Runtime, Select};
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let unit = ui.signal("none");
    /// let select = Select::new(&ui, unit, [Choice::new("ms", "Millisekunder")])?
    ///     .placeholder("Välj…");
    /// ui.mount_tree(Element::new("form", Column::new([Element::new("unit", select)])))?;
    /// ui.flush(Duration::ZERO)?;
    /// let text = ui.inspect().components.into_iter().find(|c| c.key == "select")
    ///     .and_then(|c| c.details.properties.get("text").cloned());
    /// assert_eq!(text, Some(pert_ui::InspectValue::Text("Välj…".into())));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn placeholder(mut self, placeholder: impl Into<TextSource>) -> Self {
        if let Some(anchor) = &mut self.anchor {
            anchor.placeholder = placeholder.into();
        }
        self
    }
}
impl Select<ChoiceId> {
    /// Creates a closed dropdown bound to a choice parameter; the specification
    /// supplies options, labels and order, and replacing it updates the list.
    /// The anchor is disabled while the parameter is read-only, unavailable or
    /// has a delivery fault, or its remote authority is pending, unknown or
    /// disconnected. `ui` supplies internal UI-local open state and must
    /// own the parameter. The specification's name is the anchor's
    /// accessible name and follows a replaced specification, as for
    /// [`RadioGroup::for_parameter`](crate::RadioGroup::for_parameter).
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn for_parameter(ui: &impl crate::StateOwner, parameter: Parameter<ChoiceId>) -> Self {
        let select = Self::with_binding(ui, Rc::new(parameter), Vec::new());
        select.target.parameter.set(true);
        select
    }
}
impl<T: Clone + Eq + 'static> Component for Select<T> {
    fn reconcile(&mut self, _next: Self) -> Option<Invalidation> {
        // The anchor and the list carry the description and reconcile as
        // children; the retained popover keeps whether the list is open.
        None
    }
    fn take_children(&mut self) -> Vec<Element> {
        if let (Some(anchor), Some(list)) = (self.anchor.take(), self.list.take()) {
            self.popover.set_list_content(
                Element::new("select", anchor).structural(),
                Element::new("options", list).structural(),
            );
        }
        self.popover.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.popover.update(cx)
    }
    fn focus_within_changed(
        &mut self,
        _: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        // Moving between the trigger and its own list stays within.
        if !within && let Some(session) = self.target.session.borrow().as_ref() {
            session.report(EditorTrigger::FocusLeft, EditorOutcome::Unchanged);
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        self.popover.child_visible(key)
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        self.popover.child_surface(key)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        r: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.popover.dismiss_surface(cx, r)
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.popover.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.popover.arrange(cx, s)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn inspect(&self) -> ComponentDetails {
        // The anchor is the combo box; this owner only hosts its surface.
        let mut details = self.popover.inspect();
        details.semantics = None;
        details
    }
}

/// An inline dropdown: a caption, the current choice and a chevron on one
/// ControlHeight row without a field frame, opening the [`Select`] list.
///
/// The secondary caption uses [`TextStyle::Caption`]; the choice uses
/// [`TextStyle::Body`]. They share one text baseline.
///
/// The anchor reserves the widest option, or an explicit [`Self::width_hint`], so
/// changing the value never moves neighbors. Hover shows `SurfaceHover`; rest is
/// frameless like other toolbar content. Unmodified Enter/Space, an arrow, the Menu key
/// or Shift+F10 opens the list, as `Select` does; the closed anchor never
/// changes the committed value, so scrolling past a toolbar cannot change a
/// setting. Tentative highlight
/// versus committed selection, retained list scrolling, dynamic choices,
/// disabled entries and unknown-value placeholders follow [`Select`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, Element, FocusDirection, InlineSelect, KeyAction, KeyCode, Runtime};
/// let mut ui = Runtime::new();
/// let snap = ui.signal("1/4");
/// let select = InlineSelect::new(&ui, snap.clone(),
///     [Choice::new("1/4", "1/4"), Choice::new("1/8", "1/8")])?.label("Snap");
/// ui.mount_tree(Element::new("snap", select))?;
/// ui.focus_next(FocusDirection::Forward)?; // the anchor is the first Tab stop
/// ui.key_input(KeyCode::ArrowDown, KeyAction::Press)?; // opens the list
/// assert_eq!(snap.get()?, "1/4"); // nothing committed until chosen
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Re-describing the dropdown under the same key follows [`Select`].
#[must_use]
pub struct InlineSelect<T>(Select<T>);
description_debug!(InlineSelect<T>);
impl<T: Clone + Eq + 'static> InlineSelect<T> {
    /// Creates a closed inline dropdown.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns [`ChoiceError::DuplicateValue`] when two options have equal values.
    pub fn new(
        ui: &impl crate::StateOwner,
        value: impl Into<Signal<T>>,
        options: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<T> = value.into();
        let mut select = Select::new(ui, value, options)?;
        if let Some(anchor) = &mut select.anchor {
            anchor.inline = true;
        }
        Ok(Self(select))
    }
    /// Sets the leading caption, empty by default. Its plain text is the
    /// anchor's accessible name; without a caption, or with an empty one, a
    /// dropdown made with [`Self::for_parameter`] is named after the parameter.
    ///
    /// The caption is a [`TextSource`]: fixed plain or rich text, or a binding
    /// the row follows. A binding is read in the anchor's update and
    /// remeasures it when it changes; describing the row again with the same
    /// binding keeps the text already read.
    pub fn label(mut self, label: impl Into<TextSource>) -> Self {
        if let Some(anchor) = &mut self.0.anchor {
            anchor.label = Caption::new(label);
        }
        self
    }
    /// Reserves width for this example text instead of the widest option.
    pub fn width_hint(mut self, hint: impl Into<String>) -> Self {
        if let Some(anchor) = &mut self.0.anchor {
            anchor.hint = Some(hint.into());
        }
        self
    }
    /// Creates a closed inline dropdown over reactive options, with the owner,
    /// validation and redescription rules of [`Select::bound`].
    pub fn bound(
        ui: &impl crate::StateOwner,
        value: impl Into<Signal<T>>,
        options: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<T> = value.into();
        let options: ReadSignal<Vec<Choice<T>>> = options.into();
        let mut select = Select::bound(ui, value, options);
        if let Some(anchor) = &mut select.anchor {
            anchor.inline = true;
        }
        Self(select)
    }
    /// Replaces the text shown for an unknown value, as [`Select::placeholder`]:
    /// fixed or rich text, a binding or a string-table role.
    pub fn placeholder(mut self, placeholder: impl Into<TextSource>) -> Self {
        self.0 = self.0.placeholder(placeholder);
        self
    }
    /// Excludes the anchor from automatic focus and user activation, with the
    /// open-list rules of [`Select::disabled`].
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Reports successful user writes or remote submissions as [`Select::on_change`].
    /// Runs after closing; errors do not undo the write. The callback context belongs
    /// to the closing list; use detached deferral for longer-lived application work.
    pub fn on_change(
        mut self,
        handler: impl FnMut(&mut EventContext, &T) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        self.0 = self.0.on_change(handler);
        self
    }
    /// Restricts activation and focus, using the permission composition,
    /// cancellation and open-list rules of [`Select::enabled`].
    pub fn enabled(mut self, permission: impl Into<Permission>) -> Self {
        self.0 = self.0.enabled(permission);
        self
    }
}
impl InlineSelect<ChoiceId> {
    /// Creates a closed inline dropdown bound to a choice parameter, with the
    /// rules of [`Select::for_parameter`]. A caption from [`Self::label`]
    /// replaces the specification's name as the accessible name.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn for_parameter(ui: &impl crate::StateOwner, parameter: Parameter<ChoiceId>) -> Self {
        let mut select = Select::for_parameter(ui, parameter);
        if let Some(anchor) = &mut select.anchor {
            anchor.inline = true;
        }
        Self(select)
    }
}
impl<T: Clone + Eq + 'static> Component for InlineSelect<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.0.reconcile(next.0)
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.0.take_children()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.update(cx)
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
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        self.0.measure(cx, a)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        self.0.arrange(cx, s)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        e: KeyEvent,
        k: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.keyboard_event(cx, e, k)
    }
    fn inspect(&self) -> ComponentDetails {
        self.0.inspect()
    }
}

#[cfg(test)]
mod row_geometry_tests {
    use super::*;

    fn rows(count: usize) -> RowGeometry {
        let geometry = RowGeometry::default();
        geometry.row.set(20.0);
        geometry.count.set(count);
        geometry
    }

    #[test]
    fn the_first_arrangement_applies_a_reveal_and_the_next_keeps_it() {
        let geometry = rows(30);
        geometry.reveal.set(Some(12));
        // The branch arranges first and places a submenu from this offset.
        geometry.settle(100.0);
        let placed = geometry.offset.get();
        assert_eq!(placed, 13.0 * 20.0 - 100.0);
        // The list arranges after it and must not move the rows again.
        geometry.settle(100.0);
        assert_eq!(geometry.offset.get(), placed);
    }

    #[test]
    fn a_reveal_waits_until_the_rows_are_measured() {
        let geometry = rows(30);
        geometry.row.set(0.0);
        geometry.reveal.set(Some(12));
        geometry.settle(100.0);
        assert_eq!(geometry.offset.get(), 0.0);
        geometry.row.set(20.0);
        geometry.settle(100.0);
        assert_eq!(geometry.offset.get(), 160.0);
    }

    #[test]
    fn settling_without_a_request_only_keeps_the_offset_inside_the_rows() {
        let geometry = rows(10);
        geometry.offset.set(500.0);
        geometry.settle(100.0);
        assert_eq!(geometry.offset.get(), 100.0);
        geometry.offset.set(40.0);
        geometry.settle(100.0);
        assert_eq!(geometry.offset.get(), 40.0, "a wheel scroll is kept");
    }
}

#[cfg(test)]
mod inline_typography_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc, time::Duration};

    struct BaselineProbe(Rc<Cell<[f32; 3]>>);
    impl Component for BaselineProbe {
        fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
            Ok(Size::new(200.0, 32.0)?)
        }
        fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
            let measure = cx.measurement();
            let caption = measure.text("Snap", None, TextStyle::Caption)?;
            let value = measure.rich_text(
                &RichText::from("1/16").trailing_icon(Icon::ChevronDown),
                None,
                TextStyle::Body,
            )?;
            self.0.set([
                caption.first_baseline().unwrap(),
                value.first_baseline().unwrap(),
                value.size().height() - caption.size().height(),
            ]);
            Ok(())
        }
        fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
            p.text("Snap", None, TextStyle::Caption)?.paint(
                p,
                crate::Point::ZERO,
                p.color(ColorRole::Warning),
            )?;
            p.rich_text(
                &RichText::from("1/16").trailing_icon(Icon::ChevronDown),
                None,
                TextStyle::Body,
            )?
            .paint(p, crate::Point::new(100.0, 0.0)?, p.color(ColorRole::Error))
        }
    }

    #[test]
    fn inline_choice_has_a_smaller_caption_on_the_value_baseline() {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 80.0, 1.0).unwrap())
            .unwrap();
        let snap = ui.signal("1/16");
        let baselines = Rc::new(Cell::new([0.0; 3]));
        ui.mount_tree(Element::new(
            "root",
            Row::new([
                Element::new("probe", BaselineProbe(baselines.clone())),
                Element::new(
                    "select",
                    InlineSelect::new(&ui, snap, [Choice::new("1/16", "1/16")])
                        .unwrap()
                        .label("Snap"),
                ),
            ]),
        ))
        .unwrap();
        ui.flush(Duration::ZERO).unwrap();
        let palette = ui.theme().palette(ThemeMode::Dark).unwrap();
        let mut caption = None;
        let mut value = None;
        let mut reference_caption = None;
        let mut reference_value = None;
        for command in ui.paint().unwrap().commands() {
            if let DrawCommand::DrawMask { rect, color, .. } = command {
                if *color == palette.color(ColorRole::Muted) {
                    caption = Some(*rect);
                } else if *color == palette.color(ColorRole::OnSurface) {
                    value =
                        Some(value.map_or(
                            *rect,
                            |prior: Rect| {
                                if rect.x() < prior.x() { *rect } else { prior }
                            },
                        ));
                } else if *color == palette.color(ColorRole::Warning) {
                    reference_caption = Some(*rect);
                } else if *color == palette.color(ColorRole::Error) {
                    reference_value = Some(reference_value.map_or(*rect, |prior: Rect| {
                        if rect.x() < prior.x() { *rect } else { prior }
                    }));
                }
            }
        }
        let [caption_offset, value_offset, size_difference] = baselines.get();
        assert!(size_difference > 0.0, "the choice uses larger body text");
        let caption = caption.unwrap();
        let value = value.unwrap();
        let caption_base = caption.y() - reference_caption.unwrap().y() + caption_offset;
        let value_base = value.y() - reference_value.unwrap().y() + value_offset;
        assert!(
            (caption_base - value_base).abs() < 0.01,
            "caption baseline {caption_base}, value baseline {value_base}"
        );
    }
}
