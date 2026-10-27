//! A horizontal strip of inline controls, icon actions, separators and groups
//! that collapses whole groups into an overflow surface or wraps when space is
//! short. Built entirely on the public component contract.
use crate::layout::rect;
use crate::*;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

/// How a toolbar responds when its groups do not fit the available width.
///
/// Policies may be added, so a match needs a wildcard arm.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ToolbarOverflow {
    /// Whole groups move into an overflow surface behind a trailing "More"
    /// button, starting with groups marked `collapse_first`, then from the
    /// trailing end, never an `essential` group. This is the default.
    #[default]
    Collapse,
    /// Groups wrap onto additional lines; nothing is hidden and no overflow
    /// surface exists. Suitable for local toolbars where a second line is
    /// acceptable.
    Wrap,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ItemKind {
    Group,
    Separator,
    Spacer,
}

/// One keyed member of a [`Toolbar`]: a group of controls, a separator or a
/// flexible spacer.
///
/// A group's controls are built once and mounted once per key; see
/// [`Toolbar`] for reconciling a changed item set. Collapsing moves the
/// retained group between the strip and the overflow surface with its
/// incarnations, local state and focus intact; only its owner changes. A
/// control whose group moves while it holds a gesture or text draft cancels
/// under the ordinary hiding rules, because the closed overflow surface is
/// hidden. Groups move as units so mutually exclusive controls stay together.
/// Keys are stable sibling keys; `more` and `overflow` are reserved for the
/// toolbar's own children.
#[must_use]
pub struct ToolGroup {
    key: String,
    kind: ItemKind,
    controls: Vec<Element>,
    essential: bool,
    collapse_first: bool,
    shown: Option<ReadSignal<bool>>,
}
description_debug!(ToolGroup, key = |group| group.key.as_str());
impl ToolGroup {
    /// Creates a group from its controls. They follow the inline family
    /// contract: preferred height is ControlHeight and each control owns its own
    /// Tab stop. Fallible control constructors are resolved by the caller before
    /// the list is built.
    pub fn new(key: impl Into<String>, controls: impl IntoIterator<Item = Element>) -> Self {
        Self {
            key: key.into(),
            kind: ItemKind::Group,
            controls: controls.into_iter().collect(),
            essential: false,
            collapse_first: false,
            shown: None,
        }
    }
    /// Creates a vertical separator. It follows the group before it into the
    /// overflow surface, where it becomes a horizontal divider.
    pub fn separator(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            kind: ItemKind::Separator,
            controls: Vec::new(),
            essential: true,
            collapse_first: false,
            shown: None,
        }
    }
    /// Creates a flexible spacer that absorbs remaining width, so content after
    /// it aligns to the trailing edge. Spacers never collapse.
    pub fn spacer(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            kind: ItemKind::Spacer,
            controls: Vec::new(),
            essential: true,
            collapse_first: false,
            shown: None,
        }
    }
    /// Binds this group's presence to UI-local state, for contextual groups such
    /// as the options of the selected tool. A group whose binding reads false
    /// takes no space, never collapses into the overflow surface, cannot be
    /// focused and hides under the ordinary hiding rules; a separator directly
    /// after it hides with it. Changes take effect on the next flush without
    /// rebuilding or moving the group.
    pub fn visible_signal(mut self, shown: impl Into<ReadSignal<bool>>) -> Self {
        let shown: ReadSignal<bool> = shown.into();
        self.shown = Some(shown);
        self
    }
    /// Keeps this group in the strip regardless of available width.
    /// Separators and spacers ignore it.
    pub fn essential(mut self) -> Self {
        if self.kind == ItemKind::Group {
            self.essential = true;
            self.collapse_first = false;
        }
        self
    }
    /// Moves this group into the overflow surface before any other candidate.
    /// Separators and spacers ignore it.
    pub fn collapse_first(mut self) -> Self {
        if self.kind == ItemKind::Group {
            self.essential = false;
            self.collapse_first = true;
        }
        self
    }
}

struct Meta {
    key: String,
    kind: ItemKind,
    essential: bool,
    collapse_first: bool,
    shown: Option<ReadSignal<bool>>,
}
/// Item metadata the toolbar shares with its overflow surface and "More" button.
///
/// Each description builds its own; reconciliation replaces the mounted one
/// with it, carrying over the open state and the per-item state by key.
struct Shared {
    items: Vec<Meta>,
    desired: RefCell<Vec<bool>>,
    /// Retained identity of every group, recorded by whichever owner holds it
    /// so the toolbar can move it between them, and carried over by key when
    /// the toolbar is described again.
    ids: RefCell<BTreeMap<String, WidgetId>>,
    /// Open state of the overflow surface. Reconciliation points a new
    /// description's metadata at the mounted signal, so its "More" button opens
    /// the surface the toolbar reads.
    open: RefCell<Signal<bool>>,
}
impl Shared {
    fn open(&self) -> Signal<bool> {
        self.open.borrow().clone()
    }
    /// Carries per-item state of `previous` over to this metadata by key:
    /// entries of a key that survives with the same kind keep their value, the
    /// others take `fallback`.
    fn remap<T: Clone>(&self, previous: &Self, values: &[T], fallback: T) -> Vec<T> {
        self.items
            .iter()
            .map(|item| {
                previous
                    .index(&item.key)
                    .filter(|index| previous.items[*index].kind == item.kind)
                    .and_then(|index| values.get(index).cloned())
                    .unwrap_or_else(|| fallback.clone())
            })
            .collect()
    }
    /// Whether the item set differs in anything that decides placement.
    fn differs(&self, other: &Self) -> bool {
        self.items.len() != other.items.len()
            || self.items.iter().zip(&other.items).any(|(a, b)| {
                a.key != b.key
                    || a.kind != b.kind
                    || a.essential != b.essential
                    || a.collapse_first != b.collapse_first
                    || !ReadSignal::same_optional(&a.shown, &b.shown)
            })
    }
    fn index(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|item| item.key == key)
    }
    fn preceding_group(&self, index: usize) -> Option<usize> {
        (0..index)
            .rev()
            .find(|i| self.items[*i].kind == ItemKind::Group)
    }
    /// Reads every group's presence binding for the current update.
    fn read_shown(&self, cx: &mut UpdateContext) -> Result<Vec<bool>, RuntimeError> {
        self.items
            .iter()
            .map(|item| item.shown.as_ref().map_or(Ok(true), |shown| cx.read(shown)))
            .collect()
    }
    /// Whether contextual state removes this item from both presentations.
    fn absent(&self, shown: &[bool], index: usize) -> bool {
        match self.items[index].kind {
            ItemKind::Group => !shown[index],
            ItemKind::Separator => self
                .preceding_group(index)
                .is_some_and(|group| !shown[group]),
            ItemKind::Spacer => false,
        }
    }
    /// Whether the strip hides this item under a collapse set.
    fn hidden(&self, collapsed: &[bool], shown: &[bool], index: usize) -> bool {
        if self.absent(shown, index) {
            return true;
        }
        match self.items[index].kind {
            ItemKind::Group => collapsed[index],
            ItemKind::Separator => self
                .preceding_group(index)
                .is_some_and(|group| collapsed[group]),
            ItemKind::Spacer => false,
        }
    }
    /// Whether the overflow surface shows this item under a collapse set.
    fn overflowed(&self, collapsed: &[bool], shown: &[bool], index: usize) -> bool {
        if self.absent(shown, index) {
            return false;
        }
        match self.items[index].kind {
            ItemKind::Group => collapsed[index],
            ItemKind::Separator => {
                self.preceding_group(index)
                    .is_some_and(|group| collapsed[group])
                    && (index + 1..self.items.len())
                        .any(|i| self.items[i].kind == ItemKind::Group && collapsed[i] && shown[i])
            }
            ItemKind::Spacer => false,
        }
    }
}

/// One visible strip child assigned to a line: its child index, item index and
/// measured size.
#[derive(Clone, Copy)]
struct Placement {
    child: usize,
    item: usize,
    width: f32,
    height: f32,
}

const MORE: &str = "more";
const OVERFLOW: &str = "overflow";

/// The frameless overflow button, named by the application's `configured`
/// label or by the string table's [`StringRole::ToolbarMore`](crate::StringRole::ToolbarMore).
fn more_button(shared: &Rc<Shared>, configured: Option<TextSource>) -> Element {
    let opener = shared.clone();
    let open = shared.open();
    let label = crate::runtime::settings::library_text(
        open.shared(),
        configured,
        crate::StringRole::ToolbarMore,
    );
    let more = Button::icon(Icon::MoreHorizontal, label.clone(), move |_| {
        opener.open().set(true)
    })
    .frameless();
    let button = Element::new("button", more).structural();
    let tooltip = Tooltip::new(button, label);
    Element::new(MORE, tooltip).structural()
}

/// A horizontal strip hosting inline controls, icon buttons, separators and
/// groups at ControlHeight, with a SurfaceContainer fill and a PanelBorder line
/// on the edge facing the content.
///
/// Children keep their own Tab stops; the toolbar takes no arrow keys. Groups use
/// IconGap between their controls and ItemGap between groups; separators are
/// vertical dividers with IconGap spacing; a spacer absorbs remaining width.
/// Every child is vertically centered in the ControlHeight band; a taller child
/// is clipped and counted in the `oversized` inspection property rather than
/// growing the strip.
///
/// With [`ToolbarOverflow::Collapse`], groups that do not fit move into an
/// overflow popover behind a frameless "More" button at the trailing edge, whose
/// name and tooltip [`Toolbar::more_label`] can replace. The
/// decision is made from the final allocation during layout and applied in the
/// following update of the same flush, so a resize never presents a half-moved
/// strip. A collapsing group is the same retained subtree moved to the surface,
/// keeping its incarnations, local state and position among its peers; it is
/// hidden while the surface is closed, so an active gesture or text draft in it
/// cancels under the ordinary hiding rules.
/// With [`ToolbarOverflow::Wrap`], groups wrap onto further lines instead.
/// Contextual groups bound with [`ToolGroup::visible_signal`] leave the strip entirely
/// while their binding is false, so one toolbar can present the options of the
/// selected tool without nesting toolbars.
///
/// Reconciling a new description, for example one with a group added for the
/// current tool, takes its groups, separators and spacers in their new order,
/// along with the overflow policy and edge. Items are matched by key: a
/// surviving group in the strip keeps its controls' identity, local state and
/// focus, a new one mounts and a removed one unmounts. A group collapsed into
/// the overflow surface is matched there and keeps the same state, including a
/// text draft while the surface is open, because the toolbar moved it inside
/// its own subtree (see `Runtime::reparent`). Which groups are collapsed and
/// whether the overflow surface is open are kept by key and then follow the
/// new width.
///
/// The toolbar owns no commands: buttons carry their own activation and toggles
/// their own signals. All elements and the internal open-state signal belong to
/// the creating runtime's UI thread.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Button, Element, Icon, Runtime, ToolGroup, Toolbar};
/// let mut ui = Runtime::new();
/// let toolbar = Toolbar::new(&ui, [
///     ToolGroup::new("transport", [
///         Element::new("play", Button::icon(Icon::PlayArrow, "Play", |_| Ok(())).frameless()),
///         Element::new("stop", Button::icon(Icon::Stop, "Stop", |_| Ok(())).frameless()),
///     ]).essential(),
///     ToolGroup::separator("divider"),
///     ToolGroup::new("edit", [
///         Element::new("undo", Button::icon(Icon::Undo, "Undo", |_| Ok(())).frameless()),
///     ]),
///     ToolGroup::spacer("spacer"),
///     ToolGroup::new("session", [
///         Element::new("save", Button::icon(Icon::Save, "Save", |_| Ok(())).frameless()),
///     ]).collapse_first(),
/// ])?;
/// ui.mount_tree(Element::new("toolbar", toolbar))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Toolbar {
    children: Vec<Element>,
    shared: Rc<Shared>,
    policy: ToolbarOverflow,
    below: bool,
    applied: Vec<bool>,
    present: Vec<bool>,
    shown: bool,
    widths: Vec<Cell<f32>>,
    heights: Vec<Cell<f32>>,
    more_width: Cell<Option<f32>>,
    oversized: Cell<usize>,
    lines: Cell<usize>,
}
description_debug!(Toolbar);
impl Toolbar {
    /// Creates a toolbar from ordered groups. `ui` supplies the internal
    /// open-state signal of the overflow surface.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    ///
    /// # Errors
    /// Returns `LayoutError::InvalidChild` for an empty group, a blank key or
    /// the reserved keys `more` and `overflow`. Duplicate keys are rejected when
    /// the element is mounted, like any sibling keys.
    pub fn new(
        ui: &impl crate::StateOwner,
        groups: impl IntoIterator<Item = ToolGroup>,
    ) -> Result<Self, LayoutError> {
        let mut items = Vec::new();
        let mut strip = Vec::new();
        let mut overflow = Vec::new();
        for group in groups {
            if group.key.trim().is_empty() || group.key == MORE || group.key == OVERFLOW {
                return Err(LayoutError::InvalidChild);
            }
            match group.kind {
                ItemKind::Group => {
                    if group.controls.is_empty() {
                        return Err(LayoutError::InvalidChild);
                    }
                    strip.push(Element::new(
                        group.key.clone(),
                        Row::new(group.controls).gap(MetricRole::IconGap),
                    ));
                }
                ItemKind::Separator => {
                    strip.push(
                        Element::new(
                            group.key.clone(),
                            Divider::new().vertical().spacing(MetricRole::IconGap),
                        )
                        .structural(),
                    );
                    // Separators are stateless dividers with a different
                    // orientation in each presentation, so each keeps its own
                    // retained copy instead of moving.
                    overflow.push(
                        Element::new(
                            group.key.clone(),
                            Divider::new().spacing(MetricRole::IconGap),
                        )
                        .structural(),
                    );
                }
                ItemKind::Spacer => {
                    strip.push(Element::new(group.key.clone(), Container::default()));
                }
            }
            items.push(Meta {
                key: group.key,
                kind: group.kind,
                essential: group.essential,
                collapse_first: group.collapse_first,
                shown: group.shown,
            });
        }
        let count = items.len();
        let shared = Rc::new(Shared {
            items,
            desired: RefCell::new(vec![false; count]),
            ids: RefCell::new(BTreeMap::new()),
            open: RefCell::new(ui.signal(false)),
        });
        strip.push(more_button(&shared, None));
        strip.push(
            Element::new(
                OVERFLOW,
                OverflowColumn {
                    children: overflow,
                    shared: shared.clone(),
                    applied: vec![false; count],
                    present: vec![true; count],
                    painted_background: Cell::new(None),
                    painted_border: Cell::new(None),
                },
            )
            .structural(),
        );
        Ok(Self {
            children: strip,
            shared,
            policy: ToolbarOverflow::Collapse,
            below: false,
            applied: vec![false; count],
            present: vec![true; count],
            shown: false,
            widths: (0..count).map(|_| Cell::new(0.0)).collect(),
            heights: (0..count).map(|_| Cell::new(0.0)).collect(),
            more_width: Cell::new(None),
            oversized: Cell::new(0),
            lines: Cell::new(1),
        })
    }
    /// Selects how groups behave when the strip is narrower than its content.
    /// Default is [`ToolbarOverflow::Collapse`].
    pub fn overflow(mut self, policy: ToolbarOverflow) -> Self {
        self.policy = policy;
        self
    }
    /// Replaces the overflow button's accessible name and tooltip with a
    /// [`TextSource`]: fixed text, a binding the name follows, or a
    /// [`StringRole`](crate::StringRole) of the installed string table. By
    /// default the installed [`UiStrings`](crate::UiStrings) supply
    /// [`StringRole::ToolbarMore`](crate::StringRole::ToolbarMore), `"More"` in
    /// English, and a replaced table relabels it. Only the plain text is used.
    /// Re-describing the toolbar applies a changed label; the same binding or
    /// role keeps the text already read.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Button, Element, Icon, Runtime, ToolGroup, Toolbar};
    /// let mut ui = Runtime::new();
    /// let toolbar = Toolbar::new(&ui, [ToolGroup::new("edit", [Element::new(
    ///     "undo",
    ///     Button::icon(Icon::Undo, "Ångra", |_| Ok(())).frameless(),
    /// )])])?
    /// .more_label("Mer");
    /// ui.mount_tree(Element::new("toolbar", toolbar))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn more_label(mut self, label: impl Into<TextSource>) -> Self {
        let button = more_button(&self.shared, Some(label.into()));
        if let Some(slot) = self.children.iter_mut().find(|child| child.key == MORE) {
            *slot = button;
        }
        self
    }
    /// Marks the strip as sitting below its content, such as a status bar, so
    /// the PanelBorder line is drawn on its top edge instead of its bottom edge.
    pub fn attached_below(mut self) -> Self {
        self.below = true;
        self
    }
    fn width(&self, index: usize) -> f32 {
        match self.shared.items[index].kind {
            ItemKind::Spacer => 0.0,
            _ => self.widths[index].get(),
        }
    }
    fn gap_between(&self, previous: ItemKind, next: ItemKind, gap: f32) -> f32 {
        if previous == ItemKind::Separator || next == ItemKind::Separator {
            0.0
        } else {
            gap
        }
    }
    /// Remeasures every visible child inside the ControlHeight band, retaining
    /// widths so hidden groups still take part in overflow decisions.
    fn refresh(&self, cx: &MeasureContext<'_>) -> Result<(), RuntimeError> {
        let band = cx.metric(MetricRole::ControlHeight);
        for i in 0..cx.len() {
            let key = cx.key(i)?;
            if key == MORE {
                let size = cx.measure(i, Constraints::new(None, Some(band))?)?;
                self.more_width.set(Some(size.width()));
            } else if key != OVERFLOW
                && let Some(index) = self.shared.index(key)
            {
                let size = cx.measure(i, Constraints::new(None, Some(band))?)?;
                self.widths[index].set(size.width());
                self.heights[index].set(size.height());
            }
        }
        Ok(())
    }
    fn more_width(&self, cx: &MeasureContext<'_>) -> f32 {
        self.more_width
            .get()
            .unwrap_or_else(|| cx.metric(MetricRole::ControlHeight))
    }
    /// Total strip content width under a collapse set, including the trailing
    /// "More" button when anything is collapsed.
    fn total(&self, collapsed: &[bool], gap: f32, more: f32) -> f32 {
        let mut total = 0.0;
        let mut previous = None;
        for (index, item) in self.shared.items.iter().enumerate() {
            if self.shared.hidden(collapsed, &self.present, index) {
                continue;
            }
            if let Some(previous) = previous {
                total += self.gap_between(previous, item.kind, gap);
            }
            total += self.width(index);
            previous = Some(item.kind);
        }
        if collapsed.iter().any(|c| *c) {
            total += more + if previous.is_some() { gap } else { 0.0 };
        }
        total
    }
    fn desired(&self, inner: f32, gap: f32, more: f32) -> Vec<bool> {
        let items = &self.shared.items;
        let mut collapsed = vec![false; items.len()];
        if self.policy == ToolbarOverflow::Wrap || self.total(&collapsed, gap, more) <= inner {
            return collapsed;
        }
        let candidate =
            |index: usize, item: &Meta| item.kind == ItemKind::Group && self.present[index];
        let first: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(index, item)| candidate(*index, item) && item.collapse_first)
            .map(|(index, _)| index)
            .collect();
        let rest = items
            .iter()
            .enumerate()
            .rev()
            .filter(|(index, item)| {
                candidate(*index, item) && !item.essential && !first.contains(index)
            })
            .map(|(index, _)| index);
        let order: Vec<usize> = first.iter().copied().chain(rest).collect();
        for index in order {
            collapsed[index] = true;
            if self.total(&collapsed, gap, more) <= inner {
                break;
            }
        }
        collapsed
    }
    /// Records the identity of every group currently owned by the strip, and
    /// moves groups whose collapse state no longer matches their owner.
    ///
    /// Items are visited in order, so each move's insertion index is the number
    /// of preceding items that the receiving owner already holds: earlier items
    /// have reached their final owner before the later ones move.
    fn move_groups(&mut self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        let mut strip: Vec<String> = Vec::new();
        let mut surface = None;
        for child in cx.children() {
            if child.key() == OVERFLOW {
                surface = Some(child.id().clone());
            } else if self.shared.index(child.key()).is_some() {
                strip.push(child.key().to_owned());
                self.shared
                    .ids
                    .borrow_mut()
                    .insert(child.key().to_owned(), child.id().clone());
            }
        }
        let Some(surface) = surface else {
            return Ok(false);
        };
        let toolbar = cx.id();
        let mut moved = false;
        let mut in_strip = 0;
        let mut in_surface = 0;
        for index in 0..self.shared.items.len() {
            let item = &self.shared.items[index];
            let collapsed = item.kind == ItemKind::Group && self.applied[index];
            match item.kind {
                // A separator keeps a retained copy in both presentations.
                ItemKind::Separator => {
                    in_strip += 1;
                    in_surface += 1;
                    continue;
                }
                ItemKind::Spacer => {
                    in_strip += 1;
                    continue;
                }
                ItemKind::Group => {}
            }
            // A group belongs to the strip exactly while it is not collapsed.
            let misplaced = collapsed == strip.contains(&item.key);
            let id = self.shared.ids.borrow().get(&item.key).cloned();
            if let Some(id) = id
                && misplaced
            {
                let (parent, position) = if collapsed {
                    (&surface, in_surface)
                } else {
                    (&toolbar, in_strip)
                };
                cx.request_reparent(&id, parent, position)?;
                moved = true;
            }
            if collapsed {
                in_surface += 1;
            } else {
                in_strip += 1;
            }
        }
        Ok(moved)
    }

    /// Assigns visible strip children to lines.
    fn lines(
        &self,
        cx: &MeasureContext<'_>,
        inner: Option<f32>,
        gap: f32,
    ) -> Result<Vec<Vec<Placement>>, RuntimeError> {
        let mut lines: Vec<Vec<Placement>> = vec![Vec::new()];
        let mut x = 0.0;
        let mut previous = None;
        for i in 0..cx.len() {
            let key = cx.key(i)?;
            let Some(index) = self.shared.index(key) else {
                continue;
            };
            let kind = self.shared.items[index].kind;
            let width = self.width(index);
            let space = previous.map_or(0.0, |p| self.gap_between(p, kind, gap));
            if self.policy == ToolbarOverflow::Wrap
                && x > 0.0
                && inner.is_some_and(|inner| x + space + width > inner)
            {
                lines.push(Vec::new());
                x = 0.0;
            } else {
                x += space;
            }
            x += width;
            previous = Some(kind);
            if let Some(line) = lines.last_mut() {
                line.push(Placement {
                    child: i,
                    item: index,
                    width,
                    height: self.heights[index].get(),
                });
            }
        }
        Ok(lines)
    }
}
impl Component for Toolbar {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    /// Takes the new item set, policy and edge. The groups are reconciled
    /// children; this replaces the metadata that places them and keeps, by key,
    /// which groups are collapsed or absent, their measured sizes and whether
    /// the overflow surface is open.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            shared,
            policy,
            below,
            applied: _,
            present: _,
            shown: _,
            widths: _,
            heights: _,
            more_width: _,
            oversized: _,
            lines: _,
        } = next;
        let previous = &self.shared;
        let layout = shared.differs(previous) || policy != self.policy;
        let paint = below != self.below;
        let desired = shared.remap(previous, &previous.desired.borrow(), false);
        *shared.desired.borrow_mut() = desired;
        *shared.open.borrow_mut() = previous.open();
        // A surviving group keeps its identity wherever it is, a collapsed one
        // in the overflow surface included.
        *shared.ids.borrow_mut() = previous
            .ids
            .borrow()
            .iter()
            .filter(|(key, _)| {
                shared.index(key).is_some_and(|index| {
                    shared.items[index].kind == ItemKind::Group
                        && previous
                            .index(key)
                            .is_some_and(|old| previous.items[old].kind == ItemKind::Group)
                })
            })
            .map(|(key, id)| (key.clone(), id.clone()))
            .collect();
        self.applied = shared.remap(previous, &self.applied, false);
        self.present = shared.remap(previous, &self.present, true);
        let sizes = |cells: &[Cell<f32>]| -> Vec<Cell<f32>> {
            let values: Vec<f32> = cells.iter().map(Cell::get).collect();
            shared
                .remap(previous, &values, 0.0)
                .into_iter()
                .map(Cell::new)
                .collect()
        };
        self.widths = sizes(&self.widths);
        self.heights = sizes(&self.heights);
        self.shared = shared;
        self.policy = policy;
        self.below = below;
        if layout {
            Some(Invalidation::Layout)
        } else if paint {
            Some(Invalidation::Paint)
        } else {
            None
        }
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let open = cx.read(&self.shared.open())?;
        let present = self.shared.read_shown(cx)?;
        let desired = self.shared.desired.borrow().clone();
        let mut changed = false;
        if present != self.present {
            self.present = present;
            changed = true;
        }
        if desired != self.applied {
            self.applied = desired;
            changed = true;
        }
        let shown =
            open && self.policy == ToolbarOverflow::Collapse && self.applied.iter().any(|c| *c);
        if shown != self.shown {
            self.shown = shown;
            changed = true;
        }
        if self.move_groups(cx)? || changed {
            cx.request_layout();
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        if key == MORE {
            return self.policy == ToolbarOverflow::Collapse && self.applied.iter().any(|c| *c);
        }
        if key == OVERFLOW {
            return self.shown;
        }
        self.shared
            .index(key)
            .is_none_or(|index| !self.shared.hidden(&self.applied, &self.present, index))
    }
    fn child_surface(&self, key: &str) -> Option<SurfaceKind> {
        (key == OVERFLOW).then_some(SurfaceKind::Popover)
    }
    fn dismiss_surface(
        &mut self,
        _: &mut EventContext,
        _: SurfaceDismissal,
    ) -> Result<(), RuntimeError> {
        self.shared.open().set(false)?;
        self.shown = false;
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        self.refresh(cx)?;
        let pad_x = cx.metric(MetricRole::ControlPaddingX);
        let pad_y = cx.metric(MetricRole::ControlPaddingY);
        let band = cx.metric(MetricRole::ControlHeight);
        let gap = cx.metric(MetricRole::ItemGap);
        let width = match self.policy {
            ToolbarOverflow::Collapse => {
                let all: Vec<bool> = self
                    .shared
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        item.kind == ItemKind::Group && !item.essential && self.present[index]
                    })
                    .collect();
                self.total(&all, gap, self.more_width(cx))
            }
            ToolbarOverflow::Wrap => (0..self.shared.items.len())
                .map(|index| self.width(index))
                .fold(0.0, f32::max),
        };
        Ok(SizeLimits::at_least(Size::new(
            width + 2.0 * pad_x,
            band + 2.0 * pad_y,
        )?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        self.refresh(cx)?;
        let pad_x = cx.metric(MetricRole::ControlPaddingX);
        let pad_y = cx.metric(MetricRole::ControlPaddingY);
        let band = cx.metric(MetricRole::ControlHeight);
        let gap = cx.metric(MetricRole::ItemGap);
        let full = self.total(&vec![false; self.shared.items.len()], gap, 0.0) + 2.0 * pad_x;
        let width = available.width().map_or(full, |w| w.min(full));
        let lines = match self.policy {
            ToolbarOverflow::Collapse => 1,
            ToolbarOverflow::Wrap => self
                .lines(
                    cx,
                    available.width().map(|w| (w - 2.0 * pad_x).max(0.0)),
                    gap,
                )?
                .len()
                .max(1),
        };
        Ok(Size::new(
            width,
            lines as f32 * band + (lines - 1) as f32 * gap + 2.0 * pad_y,
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let m = cx.measurement();
        self.refresh(m)?;
        let pad_x = m
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0);
        let pad_y = m
            .metric(MetricRole::ControlPaddingY)
            .min(size.height() / 2.0);
        let band = m.metric(MetricRole::ControlHeight);
        let gap = m.metric(MetricRole::ItemGap);
        let more = self.more_width(m);
        let inner = (size.width() - 2.0 * pad_x).max(0.0);
        let desired = self.desired(inner, gap, more);
        if *self.shared.desired.borrow() != desired {
            *self.shared.desired.borrow_mut() = desired.clone();
        }
        if desired != self.applied {
            cx.request_update();
        }
        let more_visible =
            self.policy == ToolbarOverflow::Collapse && self.applied.iter().any(|c| *c);
        let lines = self.lines(m, Some(inner), gap)?;
        self.lines.set(lines.len().max(1));
        let mut oversized = 0;
        let mut placements: Vec<(usize, Rect)> = Vec::new();
        for (line, entries) in lines.iter().enumerate() {
            let fixed: f32 = entries.iter().map(|e| e.width).sum::<f32>()
                + entries
                    .windows(2)
                    .map(|pair| {
                        self.gap_between(
                            self.shared.items[pair[0].item].kind,
                            self.shared.items[pair[1].item].kind,
                            gap,
                        )
                    })
                    .sum::<f32>()
                + if more_visible && line + 1 == lines.len() {
                    more + if entries.is_empty() { 0.0 } else { gap }
                } else {
                    0.0
                };
            let spacers = entries
                .iter()
                .filter(|e| self.shared.items[e.item].kind == ItemKind::Spacer)
                .count();
            let spare = if spacers > 0 {
                (inner - fixed).max(0.0) / spacers as f32
            } else {
                0.0
            };
            let top = pad_y + line as f32 * (band + gap);
            let mut x = pad_x;
            let mut previous = None;
            for &Placement {
                child,
                item: index,
                width,
                height,
            } in entries
            {
                let kind = self.shared.items[index].kind;
                if let Some(previous) = previous {
                    x += self.gap_between(previous, kind, gap);
                }
                let width = if kind == ItemKind::Spacer {
                    spare
                } else {
                    width
                };
                if height > band {
                    oversized += 1;
                }
                let y = top + (band - height.min(band)) / 2.0;
                placements.push((child, rect(x, y, width, height.min(band))?));
                x += width;
                previous = Some(kind);
            }
        }
        self.oversized.set(oversized);
        for i in 0..cx.len() {
            let key = m.key(i)?;
            if key == MORE {
                let size_more = m.measure(i, Constraints::new(None, Some(band))?)?;
                let x = (size.width() - pad_x - size_more.width()).max(pad_x);
                let y = pad_y + (band - size_more.height().min(band)) / 2.0;
                placements.push((
                    i,
                    rect(x, y, size_more.width(), size_more.height().min(band))?,
                ));
            } else if key == OVERFLOW {
                let viewport = m.viewport();
                let popup = m.measure(
                    i,
                    Constraints::new(Some(viewport.width()), Some(viewport.height()))?,
                )?;
                let width = popup.width().min(viewport.width());
                placements.push((
                    i,
                    rect(
                        (size.width() - width).max(0.0),
                        size.height(),
                        width,
                        popup.height().min(viewport.height()),
                    )?,
                ));
            }
        }
        for (child, bounds) in placements {
            cx.place(child, bounds)?;
        }
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let size = p.size();
        p.fill_rect(Rect::from_size(size), p.color(ColorRole::SurfaceContainer));
        let thickness = p.metric(MetricRole::BorderWidth).min(size.height());
        let y = if self.below {
            0.0
        } else {
            size.height() - thickness
        };
        p.fill_rect(
            Rect::new(0.0, y, size.width(), thickness)?,
            p.color(ColorRole::PanelBorder),
        );
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.properties.insert(
            "collapsed".into(),
            InspectValue::Number(self.applied.iter().filter(|c| **c).count() as f64),
        );
        d.properties.insert(
            "hidden".into(),
            InspectValue::Number(self.present.iter().filter(|p| !**p).count() as f64),
        );
        d.properties.insert(
            "collapsed_keys".into(),
            InspectValue::List(
                self.shared
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(index, item)| item.kind == ItemKind::Group && self.applied[*index])
                    .map(|(_, item)| InspectValue::Text(item.key.clone()))
                    .collect(),
            ),
        );
        d.properties
            .insert("overflow_open".into(), InspectValue::Bool(self.shown));
        d.properties.insert(
            "policy".into(),
            InspectValue::Text(format!("{:?}", self.policy)),
        );
        d.properties.insert(
            "lines".into(),
            InspectValue::Number(self.lines.get() as f64),
        );
        d.properties.insert(
            "oversized".into(),
            InspectValue::Number(self.oversized.get() as f64),
        );
        d.semantics = Some(Semantics::new(SemanticRole::Toolbar));
        d
    }
}

/// The overflow surface: the collapsed groups themselves, moved here from the
/// strip and stacked vertically, with the separators' horizontal dividers
/// between them. It draws the shared rounded Surface/PanelBorder frame with
/// ControlPaddingX inset and a default cursor, because a promoted surface root
/// cannot move to another owner and the groups must attach directly to it.
struct OverflowColumn {
    children: Vec<Element>,
    shared: Rc<Shared>,
    applied: Vec<bool>,
    present: Vec<bool>,
    painted_background: Cell<Option<Color>>,
    painted_border: Cell<Option<(f32, Color)>>,
}
impl Component for OverflowColumn {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    /// Follows the toolbar to the new description's metadata.
    ///
    /// The description lists only the separators' dividers here; collapsed
    /// groups stay, because the toolbar's own description still lists them,
    /// and the next update restores item order if the description changed it.
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let Self {
            children: _,
            shared,
            applied: _,
            present: _,
            painted_background: _,
            painted_border: _,
        } = next;
        self.applied = shared.remap(&self.shared, &self.applied, false);
        self.present = shared.remap(&self.shared, &self.present, true);
        let changed = shared.differs(&self.shared);
        self.shared = shared;
        changed.then_some(Invalidation::Layout)
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Default)
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        Ok(InputOutcome::Ignored)
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let present = self.shared.read_shown(cx)?;
        let desired = self.shared.desired.borrow().clone();
        // Record the groups held here, and keep children in item order: a new
        // description may reorder groups that stay collapsed here.
        let mut current: Vec<(usize, WidgetId)> = Vec::new();
        for child in cx.children() {
            let Some(index) = self.shared.index(child.key()) else {
                continue;
            };
            if self.shared.items[index].kind == ItemKind::Group {
                self.shared
                    .ids
                    .borrow_mut()
                    .insert(child.key().to_owned(), child.id().clone());
            }
            current.push((index, child.id().clone()));
        }
        let mut sorted = current.clone();
        sorted.sort_by_key(|(index, _)| *index);
        let owner = cx.id();
        for (position, (index, id)) in sorted.into_iter().enumerate() {
            if current.get(position).is_some_and(|(at, _)| *at == index) {
                continue;
            }
            cx.request_reparent(&id, &owner, position)?;
            if let Some(from) = current.iter().position(|(at, _)| *at == index) {
                let moved = current.remove(from);
                current.insert(position.min(current.len()), moved);
            }
        }
        if desired != self.applied || present != self.present {
            self.applied = desired;
            self.present = present;
            cx.request_layout();
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        self.shared
            .index(key)
            .is_some_and(|index| self.shared.overflowed(&self.applied, &self.present, index))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let gap = cx.metric(MetricRole::ItemGap);
        let pad = cx.metric(MetricRole::ControlPaddingX);
        let inner = Constraints::new(available.width().map(|w| (w - 2.0 * pad).max(0.0)), None)?;
        let mut width: f32 = 0.0;
        let mut height = 0.0;
        for i in 0..cx.len() {
            let size = cx.measure(i, inner)?;
            width = width.max(size.width());
            height += size.height() + if i > 0 { gap } else { 0.0 };
        }
        Size::new(width + 2.0 * pad, height + 2.0 * pad).map_err(Into::into)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if *self.shared.desired.borrow() != self.applied {
            cx.request_update();
        }
        let gap = cx.measurement().metric(MetricRole::ItemGap);
        let pad = cx
            .measurement()
            .metric(MetricRole::ControlPaddingX)
            .min(size.width() / 2.0)
            .min(size.height() / 2.0);
        let inner = (size.width() - 2.0 * pad).max(0.0);
        let mut y = pad;
        for i in 0..cx.len() {
            let child = cx.measure(i, Constraints::new(Some(inner), None)?)?;
            let width = child.width().max(inner);
            cx.place(i, rect(pad, y, width, child.height())?)?;
            y += child.height() + gap;
        }
        Ok(())
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let color = p.color(ColorRole::Surface);
        p.fill_shape(
            Rect::from_size(p.size()),
            crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
            color,
        )?;
        self.painted_background.set(Some(color));
        Ok(())
    }
    fn paint_foreground(&self, p: &mut Painter) -> Result<(), DrawError> {
        let width = Length::new(p.metric(MetricRole::BorderWidth))?;
        let color = p.color(ColorRole::PanelBorder);
        p.stroke_shape(
            Rect::from_size(p.size()),
            crate::choice_controls::rounded(p.metric(MetricRole::CornerRadius))?,
            width,
            color,
        )?;
        self.painted_border.set(Some((width.get(), color)));
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut d = ComponentDetails::default();
        d.box_model.background = self.painted_background.get().map(Color::channels);
        d.box_model.border = self
            .painted_border
            .get()
            .map(|(width, color)| BorderInspection {
                width,
                color: color.channels(),
            });
        d.properties.insert(
            "groups".into(),
            InspectValue::Number(self.applied.iter().filter(|c| **c).count() as f64),
        );
        d
    }
}
