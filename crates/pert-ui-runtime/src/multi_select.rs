//! A chooser for several values of one choice set: the chosen values as chips
//! in a trigger, and a popup list filtered by a search field.
use crate::activation::{ActivationState, activation_key, key_result};
use crate::choice_controls::{focus_ring, rounded};
use crate::layout::{centered, rect};
use crate::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

/// Rows the popup shows before its list scrolls, as `Select` does.
const SHOWN_ROWS: usize = 8;
/// A row's height before the list has measured it.
const ROW_ESTIMATE: f32 = 28.0;

/// When a [`MultiSelect`] writes its bound value.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MultiSelectCommit {
    /// Every toggle in the popup writes the bound value at once. Closing the
    /// popup, by any means, writes nothing more.
    #[default]
    EachToggle,
    /// Toggles in the popup edit a draft, which closing the popup writes in
    /// one change, unless the bound value was written while the popup was
    /// open.
    /// Escape, and closes nobody asked for (the chooser was hidden or removed,
    /// or the popup could not open), discard the draft. See
    /// [`MultiSelect::commit`] for every way the popup closes.
    OnClose,
}

type Filter<T> = Rc<dyn Fn(&str, &Choice<T>) -> bool>;
type ChangeHandler<T> = Box<dyn FnMut(&mut EventContext, &[T]) -> Result<(), RuntimeError>>;

/// What one description of the chooser says: the binding, the options and the
/// presentation. The retained chooser adopts each new description.
struct Description<T> {
    value: Signal<Vec<T>>,
    options: ReadSignal<Vec<Choice<T>>>,
    /// The fixed options behind `options`, so that describing the same fixed
    /// options again keeps the retained source instead of rebuilding the list.
    fixed: Option<Vec<Choice<T>>>,
    /// The trigger's text while no chosen value is shown; by default the
    /// string table's [`StringRole::SelectPlaceholder`](crate::StringRole::SelectPlaceholder).
    placeholder: TextSource,
    /// The search field's placeholder; by default the string table's
    /// [`StringRole::SearchPlaceholder`](crate::StringRole::SearchPlaceholder).
    search_placeholder: TextSource,
    enablement: Permission,
    commit: MultiSelectCommit,
    /// `None` is the default case-insensitive label match.
    filter: Option<Filter<T>>,
}

/// One option shown in the popup, under its stable key.
struct Row<T> {
    key: String,
    choice: Choice<T>,
}

/// The rows matching the query, in option order. Equal when shared, so a new
/// result always reaches the list.
struct Filtered<T> {
    rows: Rc<Vec<Row<T>>>,
    index: Rc<HashMap<String, usize>>,
}
impl<T> Clone for Filtered<T> {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            index: self.index.clone(),
        }
    }
}
impl<T> PartialEq for Filtered<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.rows, &other.rows)
    }
}
impl<T> Filtered<T> {
    /// Keys of the rows that can be toggled, in order.
    fn enabled_keys(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|row| row.choice.is_enabled())
            .map(|row| row.key.as_str())
            .collect()
    }
    fn row(&self, key: &str) -> Option<&Row<T>> {
        self.index.get(key).and_then(|index| self.rows.get(*index))
    }
}

/// The derived bindings the popup is built from, for one option source and
/// filter: the matching rows, the list's source and the highlighted key.
struct Chain<T> {
    options: ReadSignal<Vec<Choice<T>>>,
    filter_generation: u64,
    filtered: ReadSignal<Filtered<T>>,
    source: ReadSignal<ListSource>,
    highlighted: ReadSignal<Option<String>>,
}
impl<T> Clone for Chain<T> {
    fn clone(&self) -> Self {
        Self {
            options: self.options.clone(),
            filter_generation: self.filter_generation,
            filtered: self.filtered.clone(),
            source: self.source.clone(),
            highlighted: self.highlighted.clone(),
        }
    }
}

/// State of one mounted chooser, shared by its owner, trigger, panel and rows.
/// The incarnation keeps its own; describing the chooser again only adopts
/// the new description into it.
struct Shared<T> {
    desc: RefCell<Description<T>>,
    /// Bumped by every adopted change, so each part knows to update again.
    generation: Cell<u64>,
    /// Bumped when the filter is replaced, so the panel rebuilds its chain.
    filter_generation: Cell<u64>,
    open: Signal<bool>,
    /// Wakes retained rows when the permission source changes between descriptions.
    permission_version: Signal<u64>,
    /// Published from the owner's event-phase notification, never reconciliation.
    permission_changed: Cell<bool>,
    query: Signal<String>,
    /// The key the person moved the highlight to; the highlighted row is this
    /// one while it matches, otherwise the first enabled match.
    highlight: Signal<Option<String>>,
    /// The popup's choices under [`MultiSelectCommit::OnClose`].
    draft: Signal<Vec<T>>,
    /// The bound value and its revision when the popup opened, `None`
    /// without an open popup or after the binding was replaced while it was
    /// open. Any write since then, even one that restored the value, wins
    /// over the draft.
    observed: RefCell<Option<(Vec<T>, u64)>>,
    chain: RefCell<Option<Chain<T>>>,
    list: ListController,
    /// Row and chip keys by value, kept for this incarnation's lifetime.
    keys: Rc<RefCell<OptionKeys<T>>>,
    /// The application's report of user writes to the bound value. Only the
    /// owner's reconcile replaces it, because every part adopts the same
    /// incoming description.
    on_change: RefCell<Option<ChangeHandler<T>>>,
}

/// Stable row and chip keys for option values: a deterministic hash of each
/// value, with a suffix for the rare values whose hashes collide. A value's
/// key is assigned the first time the chooser sees it and kept for the
/// chooser's lifetime, including while the value is filtered out or absent
/// from the options, so reordering or replacing the options never moves a key
/// to another value. The table grows with the distinct values the chooser has
/// shown.
struct OptionKeys<T> {
    by_value: HashMap<T, String>,
    used: HashSet<String>,
}
impl<T: Clone + Eq + Hash> OptionKeys<T> {
    fn new() -> Self {
        Self {
            by_value: HashMap::new(),
            used: HashSet::new(),
        }
    }
    fn key(&mut self, value: &T) -> String {
        if let Some(key) = self.by_value.get(value) {
            return key.clone();
        }
        let mut hasher = std::hash::DefaultHasher::new();
        value.hash(&mut hasher);
        let base = format!("option-{:016x}", hasher.finish());
        let mut key = base.clone();
        let mut count = 1;
        while self.used.contains(&key) {
            count += 1;
            key = format!("{base}-{count}");
        }
        self.used.insert(key.clone());
        self.by_value.insert(value.clone(), key.clone());
        key
    }
    fn keys(&mut self, options: &[Choice<T>]) -> Vec<String> {
        options
            .iter()
            .map(|choice| self.key(choice.value()))
            .collect()
    }
}

/// Rejects two options with equal values, in linear time.
fn validate<T: Eq + Hash>(options: &[Choice<T>]) -> Result<(), ChoiceError> {
    let mut values = HashSet::with_capacity(options.len());
    if options.iter().all(|choice| values.insert(choice.value())) {
        Ok(())
    } else {
        Err(ChoiceError::DuplicateValue)
    }
}

impl<T: Clone + Eq + Hash + 'static> Shared<T> {
    /// Adopts a new description, keeping this incarnation's session state.
    fn adopt(&self, next: &Self) {
        if std::ptr::eq(self, next) {
            return;
        }
        let next = next.desc.borrow();
        let mut desc = self.desc.borrow_mut();
        let mut changed = false;
        if !desc.value.read_only().same_source(&next.value.read_only()) {
            desc.value = next.value.clone();
            // A draft taken from the old binding never commits to the new one.
            self.observed.replace(None);
            changed = true;
        }
        let same_options = match (&desc.fixed, &next.fixed) {
            (Some(mine), Some(theirs)) => mine == theirs,
            (None, None) => desc.options.same_source(&next.options),
            _ => false,
        };
        if !same_options {
            desc.options = next.options.clone();
            desc.fixed = next.fixed.clone();
            changed = true;
        }
        if desc.filter.is_some() || next.filter.is_some() {
            // Closures cannot be compared, so a described filter always replaces.
            desc.filter = next.filter.clone();
            self.filter_generation.set(self.filter_generation.get() + 1);
            changed = true;
        }
        if !desc.enablement.same_as(&next.enablement) {
            desc.enablement = next.enablement.clone();
            self.permission_changed.set(true);
            changed = true;
        }
        if !desc.placeholder.same_as(&next.placeholder)
            || !desc.search_placeholder.same_as(&next.search_placeholder)
            || desc.commit != next.commit
        {
            desc.placeholder = next.placeholder.clone();
            desc.search_placeholder = next.search_placeholder.clone();
            desc.commit = next.commit;
            changed = true;
        }
        if changed {
            self.generation.set(self.generation.get() + 1);
        }
    }
    /// The value the popup's rows show as chosen: the binding, or the draft.
    fn shown(&self) -> ReadSignal<Vec<T>> {
        let desc = self.desc.borrow();
        match desc.commit {
            MultiSelectCommit::EachToggle => desc.value.read_only(),
            MultiSelectCommit::OnClose => self.draft.read_only(),
        }
    }
    /// Opens the popup on a fresh session, from an event.
    fn begin(&self) -> Result<(), RuntimeError> {
        if self.open.get()? {
            return Ok(());
        }
        if !self.desc.borrow().enablement.get()? {
            return Ok(());
        }
        let value = self.desc.borrow().value.clone();
        let current = value.get()?;
        self.draft.set(current.clone())?;
        self.query.set(String::new())?;
        self.highlight.set(None)?;
        self.observed.replace(Some((current, value.revision())));
        self.open.set(true)?;
        Ok(())
    }
    /// Ends the session, writing a draft under
    /// [`MultiSelectCommit::OnClose`] when `commit` and nothing has written
    /// the binding since the popup opened.
    fn finish(&self, cx: &mut EventContext, commit: bool) -> Result<(), RuntimeError> {
        let observed = self.observed.take();
        let (mode, binding) = {
            let desc = self.desc.borrow();
            (desc.commit, desc.value.clone())
        };
        // A dropdown disabled while open discards its draft.
        let commit = commit && self.desc.borrow().enablement.get()?;
        if commit
            && mode == MultiSelectCommit::OnClose
            && let Some((observed, revision)) = observed
            && binding.revision() == revision
        {
            let draft = self.draft.get()?;
            if draft != observed {
                binding.set(draft.clone())?;
                self.report(cx, &draft)?;
            }
        }
        Ok(())
    }
    /// Calls the application's handler with the newly written value. The
    /// handler is taken out for the call, so no borrow spans application code,
    /// and put back unless the call installed another.
    fn report(&self, cx: &mut EventContext, values: &[T]) -> Result<(), RuntimeError> {
        let handler = self.on_change.borrow_mut().take();
        if let Some(mut handler) = handler {
            let result = handler(cx, values);
            self.on_change.borrow_mut().get_or_insert(handler);
            result?;
        }
        Ok(())
    }
    /// Adds `value` to the chosen values, or removes it, from an event.
    fn toggle(&self, cx: &mut EventContext, value: &T) -> Result<(), RuntimeError> {
        let (target, commits) = {
            let desc = self.desc.borrow();
            // A popup left open when the dropdown was disabled refuses input.
            if !desc.enablement.get()? {
                return Ok(());
            }
            match desc.commit {
                MultiSelectCommit::EachToggle => (desc.value.clone(), true),
                MultiSelectCommit::OnClose => (self.draft.clone(), false),
            }
        };
        let mut values = target.get()?;
        match values.iter().position(|chosen| chosen == value) {
            Some(index) => {
                values.remove(index);
            }
            None => values.push(value.clone()),
        }
        target.set(values.clone())?;
        if commits {
            self.report(cx, &values)?;
        }
        Ok(())
    }
    /// Removes `value` from the bound value, for a chip's remove request.
    fn remove(&self, cx: &mut EventContext, value: &T) -> Result<(), RuntimeError> {
        if !self.desc.borrow().enablement.get()? {
            return Ok(());
        }
        let binding = self.desc.borrow().value.clone();
        let mut values = binding.get()?;
        let before = values.len();
        values.retain(|chosen| chosen != value);
        if values.len() == before {
            return Ok(());
        }
        binding.set(values.clone())?;
        self.report(cx, &values)
    }
    /// Handles a key the search field leaves to its owner.
    fn key(&self, cx: &mut EventContext, key: SearchKey) -> Result<(), RuntimeError> {
        let Some(chain) = self.chain.borrow().clone() else {
            return Ok(());
        };
        let filtered = chain.filtered.get()?;
        let current = chain.highlighted.get()?;
        if key == SearchKey::Submit {
            if let Some(row) = current.as_deref().and_then(|key| filtered.row(key)) {
                let value = row.choice.value().clone();
                self.toggle(cx, &value)?;
            }
            return Ok(());
        }
        let enabled = filtered.enabled_keys();
        if enabled.is_empty() {
            return Ok(());
        }
        let last = enabled.len() - 1;
        let index = current
            .as_deref()
            .and_then(|current| enabled.iter().position(|key| *key == current))
            .unwrap_or(0);
        let next = match key {
            SearchKey::Up => index.checked_sub(1).unwrap_or(last),
            SearchKey::Down => {
                if index == last {
                    0
                } else {
                    index + 1
                }
            }
            SearchKey::PageUp => index.saturating_sub(SHOWN_ROWS),
            SearchKey::PageDown => (index + SHOWN_ROWS).min(last),
            _ => return Ok(()),
        };
        let target = enabled[next].to_owned();
        self.highlight.set(Some(target.clone()))?;
        // Scrolling the highlight into view is best effort: the highlight
        // is already set, so the receipt's outcome changes nothing here.
        let _ = self.list.reveal(target)?;
        Ok(())
    }
    /// Builds the derived bindings for the current options and filter.
    fn build_chain(self: &Rc<Self>) -> Result<Chain<T>, RuntimeError> {
        let (options, filter) = {
            let desc = self.desc.borrow();
            (desc.options.clone(), desc.filter.clone())
        };
        let registry = self.keys.clone();
        let filtered = self.query.combine(&options, move |query, options| {
            let keys = registry.borrow_mut().keys(options);
            let folded = query.to_lowercase();
            let rows: Vec<Row<T>> = options
                .iter()
                .zip(keys)
                .filter(|(choice, _)| match &filter {
                    Some(filter) => filter(query, choice),
                    None => choice.label().to_lowercase().contains(&folded),
                })
                .map(|(choice, key)| Row {
                    key,
                    choice: choice.clone(),
                })
                .collect();
            let index = rows
                .iter()
                .enumerate()
                .map(|(index, row)| (row.key.clone(), index))
                .collect();
            Filtered {
                rows: Rc::new(rows),
                index: Rc::new(index),
            }
        })?;
        let weak = Rc::downgrade(self);
        let source = filtered.map(move |filtered| {
            ListSource::new(Items {
                filtered: filtered.clone(),
                shared: weak.clone(),
            })
        })?;
        let highlighted = self.highlight.combine(&filtered, |highlight, filtered| {
            let enabled = filtered.enabled_keys();
            highlight
                .as_deref()
                .filter(|key| enabled.contains(key))
                .or_else(|| enabled.first().copied())
                .map(str::to_owned)
        })?;
        Ok(Chain {
            options,
            filter_generation: self.filter_generation.get(),
            filtered,
            source,
            highlighted,
        })
    }
}

/// The popup list's rows over the current matches.
struct Items<T> {
    filtered: Filtered<T>,
    shared: Weak<Shared<T>>,
}
impl<T: Clone + Eq + Hash + 'static> ListItems for Items<T> {
    fn len(&self) -> usize {
        self.filtered.rows.len()
    }
    fn key(&self, index: usize) -> Option<String> {
        self.filtered.rows.get(index).map(|row| row.key.clone())
    }
    fn build(&self, index: usize) -> Option<Element> {
        let row = self.filtered.rows.get(index)?;
        let shared = self.shared.upgrade()?;
        Some(
            Element::new(
                row.key.clone(),
                ChoiceRow {
                    key: row.key.clone(),
                    value: row.choice.value().clone(),
                    label: row.choice.label().to_owned(),
                    content: row.choice.content().clone(),
                    enabled: row.choice.is_enabled(),
                    allowed: shared.desc.borrow().enablement.fixed().unwrap_or(true),
                    permission_version: 0,
                    shared: shared.clone(),
                    checked: false,
                    highlighted: false,
                    highlight_source: shared
                        .chain
                        .borrow()
                        .as_ref()
                        .map(|chain| chain.highlighted.clone()),
                    pressed: false,
                    text: RefCell::new(None),
                },
            )
            .structural(),
        )
    }
    fn index_of(&self, key: &str) -> Option<usize> {
        self.filtered.index.get(key).copied()
    }
}

/// One option in the popup: a check mark when chosen, the highlight when the
/// keyboard is on it. A click toggles it; it never takes focus.
struct ChoiceRow<T> {
    key: String,
    value: T,
    label: String,
    content: RichText,
    enabled: bool,
    allowed: bool,
    permission_version: u64,
    shared: Rc<Shared<T>>,
    checked: bool,
    highlighted: bool,
    highlight_source: Option<ReadSignal<Option<String>>>,
    pressed: bool,
    text: RefCell<Option<TextLayout>>,
}
impl<T> ChoiceRow<T> {
    /// Whether the option is enabled and the dropdown is too.
    fn active(&self) -> bool {
        self.enabled && self.allowed && self.shared.desc.borrow().enablement.fixed() != Some(false)
    }
}
impl<T: Clone + Eq + Hash + 'static> Component for ChoiceRow<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let layout = self.content != next.content || self.enabled != next.enabled;
        let highlight_changed =
            !ReadSignal::same_optional(&self.highlight_source, &next.highlight_source);
        self.highlight_source = next.highlight_source;
        self.value = next.value;
        self.label = next.label;
        self.content = next.content;
        self.enabled = next.enabled;
        if !self.enabled {
            self.pressed = false;
        }
        if layout {
            Some(Invalidation::Layout)
        } else {
            highlight_changed.then_some(Invalidation::Paint)
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let version = cx.read_paint(&self.shared.permission_version)?;
        self.allowed = self.shared.desc.borrow().enablement.read_paint(cx)?;
        if (!self.active() || version != self.permission_version) && self.pressed {
            self.pressed = false;
            cx.cancel_input();
        }
        self.permission_version = version;
        let shown = self.shared.shown();
        self.checked = cx.read_paint(&shown)?.contains(&self.value);
        self.highlighted = match &self.highlight_source {
            Some(highlighted) => cx.read_paint(highlighted)?.as_deref() == Some(self.key.as_str()),
            None => false,
        };
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let text = cx.rich_text(&self.content, None, TextStyle::Body)?.size();
        let mark = cx.metric(MetricRole::IconSize) + cx.metric(MetricRole::IconGap);
        Ok(Size::new(
            2.0 * cx.metric(MetricRole::ControlPaddingX) + mark + text.width(),
            (text.height() + 2.0 * cx.metric(MetricRole::ControlPaddingY))
                .max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.text.borrow_mut() = Some(cx.measurement().rich_text(
            &self.content,
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
        if !self.active() {
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Move if cx.pointer_inside() && !self.highlighted => {
                self.shared.highlight.set(Some(self.key.clone()))?;
                Ok(InputOutcome::Handled)
            }
            PointerKind::Press(PointerButton::Primary) => {
                cx.capture_pointer()?;
                self.pressed = true;
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.pressed => {
                self.pressed = false;
                cx.release_pointer();
                if cx.pointer_inside() {
                    self.shared.highlight.set(Some(self.key.clone()))?;
                    self.shared.toggle(cx, &self.value)?;
                }
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn input_reset(&mut self) {
        self.pressed = false;
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let highlighted = self.highlighted && self.active();
        if highlighted {
            p.fill_rect(Rect::from_size(size), p.color(ColorRole::Primary));
        }
        let color = p.color(if !self.active() {
            ColorRole::Muted
        } else if highlighted {
            ColorRole::OnPrimary
        } else {
            ColorRole::OnSurface
        });
        let pad = p.metric(MetricRole::ControlPaddingX);
        let icon = p.metric(MetricRole::IconSize).min(size.height());
        if self.checked {
            p.icon(
                Icon::Check,
                Rect::new(pad, centered(size.height(), icon), icon, icon)?,
                color,
            )?;
        }
        if let Some(text) = self.text.borrow().as_ref() {
            text.paint(
                p,
                pad + p.metric(MetricRole::IconSize) + p.metric(MetricRole::IconGap),
                centered(size.height(), text.size().height()),
                color,
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("text".into(), InspectValue::Text(self.label.clone()));
        details
            .properties
            .insert("checked".into(), InspectValue::Bool(self.checked));
        details
            .properties
            .insert("highlighted".into(), InspectValue::Bool(self.highlighted));
        details.semantics = Some(
            Semantics::new(SemanticRole::Checkbox)
                .name(self.label.as_str())
                .value(SemanticValue::Bool(self.checked))
                .disabled(!self.active())
                .enabled_actions(self.active().then_some(SemanticAction::Toggle)),
        );
        details
    }
}

/// The popup's content: the search field above the list of matches, or a
/// "No matches" line in place of an empty list.
struct Panel<T> {
    shared: Rc<Shared<T>>,
    seen: u64,
    /// The search placeholder the field was last described with.
    described: Option<TextSource>,
    /// Stable binding owned by this panel and consumed by the retained VirtualList.
    rows: Option<Signal<ListSource>>,
    pending_rows: Option<ListSource>,
    rows_notification_pending: bool,
    count: usize,
    /// The text shown when no option matches, from the string table.
    no_matches: String,
    empty: RefCell<Option<TextLayout>>,
}
impl<T: Clone + Eq + Hash + 'static> Panel<T> {
    fn list_height(&self, cx: &MeasureContext<'_>) -> f32 {
        cx.metric(MetricRole::ControlHeight) * self.count.clamp(1, SHOWN_ROWS) as f32
    }
}
impl<T: Clone + Eq + Hash + 'static> Component for Panel<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.shared.adopt(&next.shared);
        (self.seen != self.shared.generation.get()).then(|| {
            self.seen = self.shared.generation.get();
            Invalidation::Layout
        })
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let (options, search_placeholder) = {
            let desc = self.shared.desc.borrow();
            (desc.options.clone(), desc.search_placeholder.clone())
        };
        let strings = cx.strings()?;
        let no_matches = strings.get(crate::StringRole::NoMatches);
        if self.no_matches != no_matches {
            self.no_matches = no_matches.to_owned();
            cx.request_layout();
        }
        validate(&cx.read(&options)?)?;
        let current = self.shared.chain.borrow().clone();
        let chain = match current {
            Some(chain)
                if chain.options.same_source(&options)
                    && chain.filter_generation == self.shared.filter_generation.get() =>
            {
                chain
            }
            _ => {
                let chain = self.shared.build_chain()?;
                self.shared.chain.replace(Some(chain.clone()));
                self.described = None;
                chain
            }
        };
        let count = cx.read(&chain.filtered)?.rows.len();
        let source = cx.read(&chain.source)?;
        let rows = self
            .rows
            .get_or_insert_with(|| cx.signal(source.clone()))
            .clone();
        let pending_rows = (rows.get()? != source).then_some(source);
        if !self
            .described
            .as_ref()
            .is_some_and(|described| described.same_as(&search_placeholder))
        {
            // The search field reads a bound or role placeholder itself.
            let shared = self.shared.clone();
            let search = SearchField::new(self.shared.query.clone())
                .placeholder(search_placeholder.clone())
                .on_key(move |cx, key| shared.key(cx, key));
            let list = VirtualList::estimated(rows.read_only(), ROW_ESTIMATE)?
                .controller(self.shared.list.clone());
            cx.request_children(vec![
                Element::new("search", search).structural(),
                Element::new("list", list).structural(),
            ]);
            self.described = Some(search_placeholder);
        }
        self.pending_rows = pending_rows;
        if self.pending_rows.is_some() && !self.rows_notification_pending {
            cx.notifier().request();
            self.rows_notification_pending = true;
        }
        if count != self.count {
            self.count = count;
            cx.request_layout();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.rows_notification_pending = false;
        if let (Some(rows), Some(pending)) = (&self.rows, &self.pending_rows) {
            rows.set(pending.clone())?;
            self.pending_rows = None;
        }
        Ok(())
    }
    fn focus_within_changed(
        &mut self,
        cx: &mut EventContext,
        within: bool,
    ) -> Result<(), RuntimeError> {
        // Tab leaves the popup for the trigger or its chips, which belong to
        // the owner. The surface rules dismiss the popup for that too; this
        // also finishes the draft when focus leaves the panel by any path.
        if within || !self.shared.open.get()? {
            return Ok(());
        }
        let finished = self.shared.finish(cx, true);
        self.shared.open.set(false)?;
        finished
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let search = if cx.is_empty() {
            Size::ZERO
        } else {
            cx.measure(0, Constraints::new(a.width(), None)?)?
        };
        Ok(Size::new(
            search.width().max(cx.metric(MetricRole::ControlWidth)),
            search.height() + cx.metric(MetricRole::ItemGap) + self.list_height(cx),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        *self.empty.borrow_mut() = Some(cx.measurement().text(
            &self.no_matches,
            None,
            TextStyle::Body,
        )?);
        if cx.len() != 2 {
            return Ok(());
        }
        let search = cx.measure(0, Constraints::new(Some(s.width()), None)?)?;
        let height = search.height().min(s.height());
        cx.place(0, rect(0.0, 0.0, s.width(), height)?)?;
        let top = (height + cx.measurement().metric(MetricRole::ItemGap)).min(s.height());
        cx.place(1, rect(0.0, top, s.width(), s.height() - top)?)
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        if self.count > 0 {
            return Ok(());
        }
        let (Some(size), Some(text)) = (p.size(), self.empty.borrow().clone()) else {
            return Ok(());
        };
        let row = p.metric(MetricRole::ControlHeight);
        text.paint(
            p,
            p.metric(MetricRole::ControlPaddingX),
            (size.height() - row + centered(row, text.size().height())).max(0.0),
            p.color(ColorRole::Muted),
        )
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("matches".into(), InspectValue::Number(self.count as f64));
        details
    }
}

/// One chip the trigger shows: its value, key, label and whether it can be
/// removed.
#[derive(Clone, PartialEq)]
struct ChipSpec<T> {
    value: T,
    key: String,
    label: String,
    removable: bool,
}

/// The always-visible part: a field holding the chosen values as chips, a
/// placeholder while none is shown, and a chevron. It is the chooser's Tab
/// stop and opens the popup.
struct Trigger<T> {
    shared: Rc<Shared<T>>,
    seen: u64,
    input: ActivationState,
    focusable: bool,
    open: bool,
    chips: Option<Vec<ChipSpec<T>>>,
    options: Vec<Choice<T>>,
    chosen: Vec<T>,
    /// The placeholder as last read.
    placeholder: RichText,
    prepared: RefCell<Option<(TextLayout, TextLayout)>>,
}
impl<T: Clone + Eq + Hash + 'static> Trigger<T> {
    /// Leading padding, trailing padding with the chevron, and vertical padding.
    fn padding(metric: impl Fn(MetricRole) -> f32) -> (f32, f32, f32) {
        (
            metric(MetricRole::ControlPaddingY),
            metric(MetricRole::ControlPaddingX)
                + metric(MetricRole::IconSize)
                + metric(MetricRole::IconGap),
            metric(MetricRole::ControlPaddingY) / 2.0,
        )
    }
    fn shows_chips(&self) -> bool {
        self.chips.as_ref().is_some_and(|chips| !chips.is_empty())
    }
    fn describe(&self, specs: &[ChipSpec<T>]) -> Element {
        let chips = specs.iter().map(|spec| {
            let chip = if spec.removable {
                let (shared, value) = (self.shared.clone(), spec.value.clone());
                Chip::removable(spec.label.clone(), move |cx| shared.remove(cx, &value))
            } else {
                Chip::removable(spec.label.clone(), |_| Ok(())).disabled()
            };
            Element::new(spec.key.clone(), chip).structural()
        });
        Element::new("chips", Wrap::new(chips)).structural()
    }
}
impl<T: Clone + Eq + Hash + 'static> Component for Trigger<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        self.shared.adopt(&next.shared);
        if self.seen == self.shared.generation.get() {
            return None;
        }
        self.seen = self.shared.generation.get();
        self.input.reset();
        Some(Invalidation::Layout)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn focusable(&self) -> bool {
        self.focusable
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let (value, options, enabled, placeholder) = {
            let desc = self.shared.desc.borrow();
            (
                desc.value.read_only(),
                desc.options.clone(),
                desc.enablement.clone(),
                desc.placeholder.clone(),
            )
        };
        let placeholder = placeholder.read(cx)?;
        let chosen = cx.read(&value)?;
        let options = cx.read(&options)?;
        validate(&options)?;
        let enabled = enabled.read_paint(cx)?;
        self.open = cx.read_paint(&self.shared.open)?;
        let keys = self.shared.keys.borrow_mut().keys(&options);
        let positions: HashMap<&T, usize> = options
            .iter()
            .enumerate()
            .map(|(index, choice)| (choice.value(), index))
            .collect();
        let mut shown = HashSet::new();
        let specs: Vec<ChipSpec<T>> = chosen
            .iter()
            .filter_map(|value| positions.get(value).copied())
            .filter(|index| shown.insert(*index))
            .map(|index| ChipSpec {
                value: options[index].value().clone(),
                key: keys[index].clone(),
                label: options[index].label().to_owned(),
                removable: enabled && options[index].is_enabled(),
            })
            .collect();
        self.focusable = enabled && options.iter().any(Choice::is_enabled);
        if !self.focusable {
            self.input.reset();
        }
        self.options = options;
        self.chosen = chosen;
        if self.chips.as_ref() != Some(&specs) {
            cx.request_children(vec![self.describe(&specs)]);
            self.chips = Some(specs);
        }
        if placeholder != self.placeholder {
            self.placeholder = placeholder;
            cx.request_layout();
        }
        self.input.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        let (lead, trail, vertical) = Self::padding(|role| cx.metric(role));
        let content = if self.shows_chips() && !cx.is_empty() {
            let inner = a.width().map(|width| (width - lead - trail).max(0.0));
            cx.measure(0, Constraints::new(inner, None)?)?
        } else {
            cx.rich_text(&self.placeholder, None, TextStyle::Body)?
                .size()
        };
        let natural = (content.width() + lead + trail).max(cx.metric(MetricRole::ControlWidth));
        Ok(Size::new(
            a.width().map_or(natural, |width| natural.min(width)),
            (content.height() + 2.0 * vertical).max(cx.metric(MetricRole::ControlHeight)),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, s: Size) -> Result<(), RuntimeError> {
        let m = cx.measurement();
        let (lead, trail, _) = Self::padding(|role| m.metric(role));
        *self.prepared.borrow_mut() = Some((
            m.rich_text(&self.placeholder, None, TextStyle::Body)?,
            m.rich_text(
                &RichText::new().icon(Icon::ChevronDown),
                None,
                TextStyle::Body,
            )?,
        ));
        if cx.is_empty() {
            return Ok(());
        }
        let inner = (s.width() - lead - trail).max(0.0);
        let content = cx.measure(0, Constraints::new(Some(inner), None)?)?;
        let height = content.height().min(s.height());
        cx.place(0, rect(lead, centered(s.height(), height), inner, height)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        e: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if !self.focusable
            || matches!(e.kind(), PointerKind::Press(b) | PointerKind::Release(b) if b != PointerButton::Primary)
            || matches!(e.kind(), PointerKind::Scroll(_) | PointerKind::Pinch(_))
        {
            return Ok(InputOutcome::Ignored);
        }
        if self.input.pointer(cx, e)? {
            self.shared.begin()?;
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
        if !self.focusable {
            return Ok(InputOutcome::Ignored);
        }
        // Modified arrows are left to command bindings.
        if e.requests_context_menu(keys.modifiers())
            || (matches!(control, KeyCode::ArrowDown | KeyCode::ArrowUp)
                && keys.chord_modifiers(e) == Modifiers::default())
        {
            if e.transition == KeyTransition::Pressed {
                self.shared.begin()?;
            }
            return Ok(InputOutcome::Handled);
        }
        if activation_key(control) {
            let result = key_result(control, self.input.pressed(), cx.modifiers());
            if self.input.keyboard(cx, e, control) {
                self.shared.begin()?;
            }
            return Ok(result);
        }
        Ok(InputOutcome::Ignored)
    }
    fn input_reset(&mut self) {
        self.input.reset();
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(s) = p.size() else {
            return Ok(());
        };
        let frame = Rect::from_size(s);
        p.fill_shape(
            frame,
            rounded(p.metric(MetricRole::CornerRadius))?,
            p.color(if self.input.hovered && self.focusable {
                ColorRole::ButtonHover
            } else {
                ColorRole::Button
            }),
        )?;
        let text = p.color(if self.focusable {
            ColorRole::OnButton
        } else {
            ColorRole::Muted
        });
        if let Some((placeholder, chevron)) = self.prepared.borrow().as_ref() {
            if !self.shows_chips() {
                let (lead, _, _) = Self::padding(|role| p.metric(role));
                placeholder.paint(
                    p,
                    lead,
                    centered(s.height(), placeholder.size().height()),
                    p.color(ColorRole::Muted),
                )?;
            }
            chevron.paint(
                p,
                s.width() - p.metric(MetricRole::ControlPaddingX) - chevron.size().width(),
                centered(s.height(), chevron.size().height()),
                text,
            )?;
        }
        if self.input.focused {
            focus_ring(p, frame)?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let labels: Vec<&str> = self
            .chips
            .iter()
            .flatten()
            .map(|chip| chip.label.as_str())
            .collect();
        let text = labels.join(", ");
        details
            .properties
            .insert("text".into(), InspectValue::Text(text.clone()));
        details
            .properties
            .insert("chosen".into(), InspectValue::Number(labels.len() as f64));
        details.properties.insert(
            "placeholder".into(),
            InspectValue::Text(self.placeholder.plain_text()),
        );
        details
            .properties
            .insert("enabled".into(), InspectValue::Bool(self.focusable));
        details.validation.interactive = self.focusable;
        details.semantics = Some(
            Semantics::new(SemanticRole::ComboBox)
                .value(SemanticValue::Text(text))
                .items(self.options.iter().map(|option| {
                    SemanticItem::new(option.label())
                        .selected(self.chosen.contains(option.value()))
                        .disabled(!self.focusable || !option.is_enabled())
                        .action(SemanticAction::Toggle)
                }))
                .expanded(self.open)
                .disabled(!self.focusable)
                .enabled_actions([if self.open {
                    SemanticAction::Collapse
                } else {
                    SemanticAction::Expand
                }]),
        );
        details
    }
}

/// A chooser for any number of values from one set of options: the chosen
/// values appear as removable [`Chip`]s in a trigger, and the trigger opens a
/// popup with a [`SearchField`] above a [`VirtualList`] of the options.
///
/// **Value and identity.** The bound `Signal<Vec<T>>` holds the chosen values
/// in the order they were chosen; a newly chosen value is appended. Options
/// are identified by their value, as in [`Select`], never by label or
/// position: reordering, relabeling or filtering the options keeps each
/// chosen value, its chip and its row. `T` must be [`Hash`] because rows and
/// chips are keyed by a hash of the value and duplicates are found in linear
/// time, so a large option set stays cheap. A chosen value that no option
/// holds stays in the binding but shows no chip; only the application removes
/// it. A value listed twice shows one chip. Two options with equal values are
/// rejected with [`ChoiceError::DuplicateValue`].
///
/// **Trigger.** One Tab stop that shows the chips wrapped onto lines, the
/// placeholder while no chip is shown, and a chevron. Unmodified Enter/Space, Up, Down,
/// the Menu key or Shift+F10 opens the popup (modified arrows are left to
/// command bindings), as does a primary click on the
/// trigger outside its chips. Each chip of an enabled option is its own Tab
/// stop after the trigger: its × button, Delete or Backspace removes that
/// value from the binding at once, whatever [`MultiSelect::commit`] says, and
/// focus moves to the next chip, then the previous one, then the trigger
/// (see [`Chip`]). Chips of disabled options cannot be removed.
///
/// **Popup.** Opening starts with an empty query and moves focus to the search
/// field; the list shows the options whose label contains the query, ignoring
/// case, or as [`MultiSelect::filter`] decides. Focus stays in the field:
/// Up and Down move the highlight over the enabled matches (wrapping), Page Up
/// and Page Down move it by eight, and Enter toggles the highlighted option.
/// The highlight stays on its option while that option matches, and is
/// otherwise the first enabled match. A primary click on a row toggles it
/// without moving focus. Disabled options are shown muted and never toggle.
/// The popup stays open across toggles. Escape with a non-empty query clears
/// the query (see [`SearchField`]); with an empty query it closes the popup.
/// Tab moves from the search field to the list while the list scrolls (where
/// the arrows scroll it), and out of the popup after that; focus leaving the
/// popup closes it, even when it lands on the trigger or a chip. Closing
/// returns focus to the trigger unless focus moved elsewhere, and the chosen
/// values are never lost by moving focus between the trigger and the popup.
///
/// **Committing.** [`MultiSelectCommit::EachToggle`], the default, writes the
/// binding on every toggle, so Escape only closes. With
/// [`MultiSelectCommit::OnClose`] the popup edits a draft that starts from the
/// bound value when it opens; see [`MultiSelect::commit`] for when the draft is
/// written. Values the application writes are never changed by the chooser,
/// except by the person's own toggles and removals.
///
/// The popup is an anchored surface with the [`Popover`] placement rule: it
/// needs room above or below the trigger, so mount the chooser inside a layout
/// container rather than as a root. Hiding or removing the chooser closes the
/// popup. There is no polling or timer. Permission-source redescription uses an
/// event-phase notification to refresh retained row subscriptions in the same flush.
/// Replacing an option source or filter similarly publishes the validated rows
/// through the retained list's stable binding, preserving surviving row identities.
///
/// Inspection reports the trigger as a [`SemanticRole::ComboBox`] whose value
/// is the chosen labels joined by ", ", with one item per option, and each
/// popup row as a [`SemanticRole::Checkbox`]. The trigger's `placeholder`
/// property is its placeholder text as last read.
///
/// Describing the chooser again under the same key applies its binding,
/// options, availability, placeholders, commit timing and filter, and keeps the popup's
/// query, highlight and draft. A draft taken from a replaced binding is never
/// written to the new one.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Choice, Column, Element, MultiSelect, Runtime};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// let tags = ui.signal(vec!["mix"]);
/// let chooser = MultiSelect::new(&ui, tags.clone(), [
///     Choice::new("draft", "Draft"),
///     Choice::new("mix", "Mix"),
///     Choice::new("master", "Master"),
/// ])?;
/// ui.mount_tree(Element::new("form", Column::new([Element::new("tags", chooser)])))?;
/// ui.flush(Duration::ZERO)?;
/// let trigger = ui.inspect().components.into_iter().find(|c| c.key == "anchor")
///     .and_then(|c| c.details.properties.get("text").cloned());
/// assert_eq!(trigger, Some(pert_ui::InspectValue::Text("Mix".into())));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct MultiSelect<T> {
    shared: Rc<Shared<T>>,
    seen: u64,
    shown: bool,
    permission_notification_pending: bool,
}
impl<T: Clone + Eq + Hash + 'static> MultiSelect<T> {
    /// Creates a closed chooser over fixed options. `ui` supplies the
    /// chooser's UI-local state; `value` must belong to the same runtime.
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
        value: impl Into<Signal<Vec<T>>>,
        options: impl IntoIterator<Item = Choice<T>>,
    ) -> Result<Self, ChoiceError> {
        let value: Signal<Vec<T>> = value.into();
        let options: Vec<_> = options.into_iter().collect();
        validate(&options)?;
        let source = ui.signal(options.clone()).read_only();
        Ok(Self::with_options(ui, value, source, Some(options)))
    }
    fn with_options(
        ui: &impl crate::StateOwner,
        value: Signal<Vec<T>>,
        options: ReadSignal<Vec<Choice<T>>>,
        fixed: Option<Vec<Choice<T>>>,
    ) -> Self {
        let desc = Description {
            value,
            options,
            fixed,
            placeholder: TextSource::from(crate::StringRole::SelectPlaceholder),
            search_placeholder: TextSource::from(crate::StringRole::SearchPlaceholder),
            enablement: Permission::allowed(),
            commit: MultiSelectCommit::default(),
            filter: None,
        };
        Self {
            shared: Rc::new(Shared {
                desc: RefCell::new(desc),
                generation: Cell::new(0),
                filter_generation: Cell::new(0),
                open: ui.signal(false),
                permission_version: ui.signal(0),
                permission_changed: Cell::new(false),
                query: ui.signal(String::new()),
                highlight: ui.signal(None),
                draft: ui.signal(Vec::new()),
                observed: RefCell::new(None),
                chain: RefCell::new(None),
                list: ListController::new(ui),
                keys: Rc::new(RefCell::new(OptionKeys::new())),
                on_change: RefCell::new(None),
            }),
            seen: 0,
            shown: false,
            permission_notification_pending: false,
        }
    }
    fn describe(self, change: impl FnOnce(&mut Description<T>)) -> Self {
        change(&mut self.shared.desc.borrow_mut());
        self
    }
    /// Creates a closed chooser over reactive options. `ui` owns its local state;
    /// the value and options must belong to that runtime. The source is read at
    /// update, not construction. Duplicate options fail with [`RuntimeError::Choice`]
    /// holding [`ChoiceError::DuplicateValue`]; foreign bindings fail with
    /// [`RuntimeError::ForeignRuntime`]. An invalid update preserves the last
    /// valid list (initially empty). Valid changes retain chosen values, the draft
    /// and highlight by value. Redescription replaces these bindings with the
    /// existing draft-conflict rules of [`Self::commit`].
    pub fn bound(
        ui: &impl crate::StateOwner,
        value: impl Into<Signal<Vec<T>>>,
        options: impl Into<ReadSignal<Vec<Choice<T>>>>,
    ) -> Self {
        let value: Signal<Vec<T>> = value.into();
        let options: ReadSignal<Vec<Choice<T>>> = options.into();
        Self::with_options(ui, value, options, None)
    }
    /// Sets when the bound value is written; [`MultiSelectCommit::EachToggle`]
    /// by default.
    ///
    /// With [`MultiSelectCommit::OnClose`] the draft is written in one change
    /// when the popup closes by an outside press (including a press on the
    /// trigger), by focus leaving the popup (Tab), by the window losing focus,
    /// by its anchor being clipped or by another popup replacing it, and only
    /// if nothing wrote the bound value while the popup was open (see
    /// [`Signal::revision`]): an application write wins and the draft is
    /// dropped, even one that later restored the value the popup opened on,
    /// and so does a chip removal made while the popup was open. Escape discards the draft, and so do hiding or removing the
    /// chooser and a popup that could not open. Unchanged drafts write
    /// nothing.
    pub fn commit(self, commit: MultiSelectCommit) -> Self {
        self.describe(|desc| desc.commit = commit)
    }
    /// Reports each user write of the bound value, synchronously in the input
    /// event that made it, with the values just written in binding order.
    ///
    /// It is called after a toggle under [`MultiSelectCommit::EachToggle`],
    /// after a chip removal under either commit timing, and after an
    /// [`MultiSelectCommit::OnClose`] draft is written by one of the closes
    /// [`Self::commit`] lists. Toggles that only edit a draft, closes that
    /// discard it or write nothing (Escape, a hidden or removed chooser, a
    /// popup that could not open, an unchanged draft, a draft that lost to
    /// another write) and refusals while [`Self::enabled`] denies never call
    /// it. Application writes, option changes and redescription do not call
    /// it either; observe the bound signal for those.
    ///
    /// A failing handler's error propagates through the runtime's input
    /// handling: the write stays, the popup still closes when the write was
    /// its close, and nothing is replayed. Redescribing the chooser replaces
    /// the handler, including removing it, without closing an open popup or
    /// discarding its draft.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, MultiSelect, Runtime};
    /// let ui = Runtime::new();
    /// let tags = ui.signal(vec!["mix"]);
    /// let submitted = ui.signal(Vec::new());
    /// let options = [Choice::new("mix", "Mix"), Choice::new("send", "Send")];
    /// let chooser = MultiSelect::new(&ui, tags, options)?
    ///     .on_change(move |_, values| submitted.set(values.to_vec()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_change(
        self,
        handler: impl FnMut(&mut EventContext, &[T]) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        *self.shared.on_change.borrow_mut() = Some(Box::new(handler));
        self
    }
    /// Replaces the text the trigger shows while no chosen value is shown.
    /// Without it the installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::SelectPlaceholder`](crate::StringRole::SelectPlaceholder),
    /// `"Select…"` in English.
    ///
    /// The placeholder is a [`TextSource`]: fixed plain or rich text, a
    /// binding the trigger follows, or a [`StringRole`](crate::StringRole)
    /// that follows the installed string table, as the default does. The
    /// trigger reads it in its update; describing the chooser again with the
    /// same source keeps the text already read.
    pub fn placeholder(self, placeholder: impl Into<TextSource>) -> Self {
        let placeholder = placeholder.into();
        self.describe(|desc| desc.placeholder = placeholder)
    }
    /// Replaces the search field's placeholder. Without it the installed
    /// [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::SearchPlaceholder`](crate::StringRole::SearchPlaceholder),
    /// `"Search"` in English. The text shown when no option matches is
    /// [`StringRole::NoMatches`](crate::StringRole::NoMatches).
    ///
    /// The placeholder is a [`TextSource`], read by the popup's search field
    /// as [`SearchField::placeholder`](crate::SearchField::placeholder) does;
    /// describing the chooser again with the same source keeps the field.
    pub fn search_placeholder(self, placeholder: impl Into<TextSource>) -> Self {
        let placeholder = placeholder.into();
        self.describe(|desc| desc.search_placeholder = placeholder)
    }
    /// Replaces the match rule. `filter` receives the query exactly as typed
    /// and an option, and returns whether the option is listed; an empty query
    /// is passed like any other. It runs for every option whenever the query
    /// or the options change, during the runtime's settling of derived values,
    /// so it must be deterministic, cheap and free of side effects. The
    /// default lists the options whose label contains the query, ignoring case.
    ///
    /// Unlike the collection filters, which trim a query before their
    /// [`FilterMatcher`](crate::FilterMatcher) sees it, this rule receives
    /// surrounding whitespace too, so a rule that should ignore it trims the
    /// query itself. The matcher presets apply to an option's label like this,
    /// and the rule can also read the option's value:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Choice, FilterMatcher, MultiSelect, Runtime};
    /// let ui = Runtime::new();
    /// let effects = ui.signal(vec!["reverb"]);
    /// let options = [Choice::new("reverb", "Reverb"), Choice::new("delay", "Delay")];
    /// let matcher = FilterMatcher::fuzzy();
    /// assert!(matcher.matches("rvb", "Reverb"));
    /// let chooser = MultiSelect::new(&ui, effects, options)?
    ///     .filter(move |query, choice| matcher.matches(query.trim(), choice.label()));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn filter(self, filter: impl Fn(&str, &Choice<T>) -> bool + 'static) -> Self {
        let filter: Filter<T> = Rc::new(filter);
        self.describe(|desc| desc.filter = Some(filter))
    }
    /// Excludes the trigger and its chips from focus and user input; chips are
    /// shown muted and cannot be removed. A popup that is open when a
    /// redescription disables the dropdown stays open but shows its rows
    /// disabled and refuses toggles, and closing it writes no
    /// [`MultiSelectCommit::OnClose`] draft; Escape and an outside press still
    /// close it.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts opening, option toggles and chip removal with a fixed or reactive
    /// permission. Repeated calls combine restrictions; redescription replaces
    /// the permission. Bindings must belong to this chooser's runtime. Mounted
    /// rows and chips observe revocation during a successful flush and cancel
    /// armed input. Unlike a standalone Chip's fixed denial, a chip inside this
    /// composite sees the latest description at update; intermediate descriptions
    /// replaced before that update are not individually applied to its input state.
    /// An open popup stays visible and can be dismissed while denied. Closing
    /// while denied discards an `OnClose` draft; reenabling before closing allows
    /// that draft to commit under the usual conflict rules of [`Self::commit`].
    pub fn enabled(self, permission: impl Into<Permission>) -> Self {
        self.describe(|desc| desc.enablement = desc.enablement.clone().and(permission))
    }
}
impl<T: Clone + Eq + Hash + 'static> Component for MultiSelect<T> {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        if !Rc::ptr_eq(&self.shared, &next.shared) {
            // Replaced unconditionally, including removal; an open popup,
            // its draft and armed input are unaffected.
            let handler = next.shared.on_change.borrow_mut().take();
            *self.shared.on_change.borrow_mut() = handler;
        }
        self.shared.adopt(&next.shared);
        (self.seen != self.shared.generation.get()).then(|| {
            self.seen = self.shared.generation.get();
            Invalidation::Layout
        })
    }
    fn take_children(&mut self) -> Vec<Element> {
        // The trigger's update resolves the shown text, from the string table
        // when the application gave none.
        // The trigger reads its placeholder in its update.
        let placeholder = RichText::default();
        let trigger = Trigger {
            shared: self.shared.clone(),
            seen: 0,
            input: ActivationState::default(),
            focusable: false,
            open: false,
            chips: None,
            options: Vec::new(),
            chosen: Vec::new(),
            placeholder,
            prepared: RefCell::new(None),
        };
        let panel = Panel {
            shared: self.shared.clone(),
            seen: 0,
            described: None,
            rows: None,
            pending_rows: None,
            rows_notification_pending: false,
            count: 0,
            no_matches: String::new(),
            empty: RefCell::new(None),
        };
        vec![
            Element::new("anchor", trigger).structural(),
            Element::new(
                "popup",
                Container::new(Element::new("panel", panel).structural())
                    .background_role(ColorRole::Surface)
                    .border_role(ColorRole::PanelBorder)
                    .cursor(CursorIcon::Default)
                    .padding(MetricRole::ControlPaddingX),
            )
            .structural(),
        ]
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.shared.permission_changed.get() && !self.permission_notification_pending {
            cx.notifier().request();
            self.permission_notification_pending = true;
        }
        self.shown = cx.read(&self.shared.open)?;
        Ok(())
    }
    fn deliver_notifications(&mut self, _: &mut EventContext) -> Result<(), RuntimeError> {
        self.permission_notification_pending = false;
        if self.shared.permission_changed.get() {
            self.shared
                .permission_version
                .set(self.shared.generation.get())?;
            self.shared.permission_changed.set(false);
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        key != "popup" || self.shown
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == "popup").then_some(SurfaceKind::Popover)
    }
    fn dismiss_surface(
        &mut self,
        cx: &mut EventContext,
        reason: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        let commit = match reason {
            SurfaceDismissal::Escape
            | SurfaceDismissal::OwnerHidden
            | SurfaceDismissal::OwnerRemoved
            | SurfaceDismissal::NoSpace
            | SurfaceDismissal::ModalConflict => false,
            SurfaceDismissal::OutsidePress
            | SurfaceDismissal::FocusLeft
            | SurfaceDismissal::WindowFocusLost
            | SurfaceDismissal::AnchorClipped
            | SurfaceDismissal::Replaced => true,
        };
        self.shown = false;
        let finished = self.shared.finish(cx, commit);
        self.shared.open.set(false)?;
        finished
    }
    fn measure(&self, cx: &MeasureContext<'_>, a: Constraints) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            Ok(Size::ZERO)
        } else {
            cx.measure(0, a)
        }
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.len() > 2 {
            return Err(LayoutError::InvalidChild.into());
        }
        if !cx.is_empty() {
            cx.place(0, Rect::from_size(size))?;
        }
        if cx.len() == 2 {
            let viewport = cx.measurement().viewport();
            let available = Constraints::new(Some(viewport.width()), Some(viewport.height()))?;
            let popup = cx.measure(1, available)?;
            cx.place(
                1,
                rect(
                    0.0,
                    size.height(),
                    popup.width().max(size.width()).min(viewport.width()),
                    popup.height().min(viewport.height()),
                )?,
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("open".into(), InspectValue::Bool(self.shown));
        details
    }
}
