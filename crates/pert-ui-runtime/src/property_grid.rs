//! A property grid: labeled editors in two aligned columns with a resizable
//! label column.
use crate::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
};

/// One labeled row of a [`PropertyGrid`]: an application-supplied editor
/// with its label, optional help text and an optional status.
///
/// The key is the row's identity: it keys the editor among the grid's
/// children, so an editor keeps its focus, draft and other local state while
/// rows are added, removed, reordered or relabeled, as long as the editor
/// element itself keeps its key and component type: the editor is a keyed
/// child of the row, so a new editor key or type mounts a new editor without
/// that state, whatever the row key. The label is
/// presentation and names the editor. The editor is wrapped in a
/// [`Field`] whose label is not drawn ([`Field::hide_label`]), so the help
/// text, status message and status outline appear in the editor column
/// exactly as a field presents them, and inspection names the editor by the
/// label.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// let mut ui = Runtime::new();
/// let name = ui.signal(String::from("Channel 1"));
/// let row = Property::new("name", "Name", Element::new("input", TextField::new(name)))
///     .help("Shown in the mixer");
/// let element = row.into_element();
/// assert_eq!((element.key(), element.label()), ("name", Some("Name")));
/// # Ok::<(), RuntimeError>(())
/// ```
#[must_use]
pub struct Property {
    key: String,
    label: TextSource,
    field: Field,
}
description_debug!(Property, key = |row| row.key.as_str());
impl Property {
    /// Creates a row. The key must not be blank and must be unique among the
    /// grid's rows, and the label must not be blank; [`PropertyGrid`] checks
    /// all three when it is described ([`LayoutError::BlankKey`],
    /// [`LayoutError::DuplicateKey`], [`LayoutError::BlankLabel`]).
    ///
    /// The label is a [`TextSource`]: fixed text, or a binding the row and
    /// its editor's name follow without describing the grid again. A bound
    /// label is checked when it is read: a blank read keeps the last labels
    /// and fails the grid's update with [`LayoutError::BlankLabel`].
    pub fn new(key: impl Into<String>, label: impl Into<TextSource>, editor: Element) -> Self {
        let label: TextSource = label.into();
        Self {
            key: key.into(),
            field: Field::new(editor).label(label.clone()).hide_label(),
            label,
        }
    }
    /// Sets help text below the editor, as [`Field::help`]: fixed or bound
    /// text; empty, the default, shows none.
    pub fn help(mut self, help: impl Into<TextSource>) -> Self {
        self.field = self.field.help(help);
        self
    }
    /// Shows `status` below the editor, as [`Field::status`]: described, it
    /// replaces an earlier status, status binding or form.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let editor = TextField::new(ui.signal(String::from("Ch 1"))).key("input");
    /// let row = Property::new("name", "Name", editor)
    ///     .status(FieldStatus::warning("Shortened in the mixer"));
    /// ui.mount_tree(PropertyGrid::new([row]).key("grid"))?;
    /// ui.flush(Duration::ZERO)?;
    /// let input = ui.inspect().components.into_iter().find(|c| c.key == "input").unwrap();
    /// let semantics = input.details.semantics.unwrap();
    /// assert_eq!(semantics.description.as_deref(), Some("Shortened in the mixer"));
    /// assert!(semantics.warning);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn status(mut self, status: FieldStatus) -> Self {
        self.field = self.field.status(status);
        self
    }
    /// Follows an application-owned status binding, as
    /// [`Field::status_signal`], replacing an earlier status, status binding
    /// or form.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::prelude::*;
    /// use std::time::Duration;
    /// let mut ui = Runtime::new();
    /// let (name, checked) = (ui.signal(String::from("Ch 1")), ui.signal(FieldStatus::Normal));
    /// let editor = || TextField::new(name.clone()).key("input");
    /// let grid = |row: Property| PropertyGrid::new([row]).key("grid");
    /// let row = Property::new("name", "Name", editor()).status(FieldStatus::warning("Check"));
    /// let root = ui.mount_tree(Column::new([grid(row)]).key("root"))?;
    /// ui.flush(Duration::ZERO)?;
    ///
    /// let row = Property::new("name", "Name", editor()).status_signal(checked.read_only());
    /// ui.reconcile_children(&root, vec![grid(row)])?;
    /// checked.set(FieldStatus::error("Required"))?;
    /// ui.flush(Duration::ZERO)?;
    /// let input = ui.inspect().components.into_iter().find(|c| c.key == "input").unwrap();
    /// let semantics = input.details.semantics.unwrap();
    /// assert_eq!(semantics.description.as_deref(), Some("Required"));
    /// assert!(semantics.invalid);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn status_signal(mut self, status: impl Into<ReadSignal<FieldStatus>>) -> Self {
        self.field = self.field.status_signal(status);
        self
    }
    /// Marks the row's editor as required, as [`Field::required`]: " *"
    /// follows the label, the editor's semantics report it and inspection
    /// reports `required`. It decides nothing; the application still publishes
    /// a status for an empty required editor if it wants one. Default: not
    /// required.
    pub fn required(mut self, required: bool) -> Self {
        self.field = self.field.required(required);
        self
    }
    /// Follows a form's field, as [`Field::form`], so
    /// [`FormFields::focus_first_error`] and [`FormFields::discard_drafts`]
    /// reach this row's editor.
    pub fn form(mut self, handle: &FieldHandle) -> Self {
        self.field = self.field.form(handle);
        self
    }
    /// The ordinary labeled description the grid holds as a child, for
    /// [`Runtime::reconcile_children`] on the grid: reusing a key keeps that
    /// row's editor and its state.
    pub fn into_element(self) -> Element {
        Element::new(self.key, self.field).with_label(self.label)
    }
}
impl From<Property> for Element {
    /// Converts as [`Property::into_element`] does, so a list of properties can be
    /// collected into the `Vec<Element>` that
    /// [`Runtime::reconcile_children`](crate::Runtime::reconcile_children)
    /// takes with `.map(Element::from)` or `.map(Into::into)`.
    fn from(property: Property) -> Self {
        property.into_element()
    }
}

/// One row as the last update read it.
struct Row {
    key: String,
    label: String,
}

/// A row's arrangement: its prepared label and where it is drawn.
struct Placed {
    text: TextLayout,
    x: f32,
    y: f32,
    /// The row key, when the label was truncated.
    truncated: Option<String>,
}

/// One row's editor rectangle and its placed label.
type Arranged = (Rect, Placed);

/// Layout results the input handlers need.
#[derive(Clone, Copy)]
struct Geometry {
    /// The label column's resolved width.
    label: f32,
    /// The resize zone between the columns, full height.
    divider: Rect,
    /// The narrowest and widest label column the current width permits.
    minimum: f32,
    maximum: f32,
}

/// The input session that owns a tentative label width.
#[derive(Clone, Copy)]
enum Session {
    Pointer { position: f32, start: f32 },
    Keys,
}

/// Rows of a label and an application-supplied editor, in two aligned columns
/// that share one adjustable label column width.
///
/// The host supplies the rows, their editors and their meaning: the grid owns
/// no schema, interprets no units and does not validate or submit anything.
/// Build the rows with [`Property`]; each row's editor is a child keyed by the
/// row key, so describing the grid again through
/// [`Runtime::reconcile_children`] with [`Property::into_element`] adds,
/// removes, reorders and relabels rows while editors keep their state by key.
/// A grid described with its rows is checked before anything mounts: a blank
/// key, two rows with one key or a blank label fail the mount or
/// reconciliation with [`LayoutError::BlankKey`], [`LayoutError::DuplicateKey`]
/// or [`LayoutError::BlankLabel`], and a mounted grid is left as it was. A
/// child described another way must carry a nonblank label
/// ([`Element::with_label`]), which the grid draws in the label column; a
/// blank one returns [`LayoutError::BlankLabel`] from the update that reads it.
///
/// **Layout.** Each row places its label in the label column and its editor
/// in the editor column; the row is as tall as the taller of the two, and rows
/// are separated by the theme ItemGap. By default labels wrap within the label
/// column rather than being cut off, so their full text always shows;
/// [`Self::label_overflow`] keeps them on one line with an ellipsis instead.
/// A single-line label is centered on the first control line (ControlHeight). The editor
/// fills the editor column. Help text, a status message and the status outline
/// come from the row's [`Field`] in the editor column. Hidden rows take no
/// space.
///
/// **Label column.** Without a preference the label column is as wide as the
/// widest label on one line, at most half the grid. The column is never
/// narrower than the theme ControlWidth (64 logical pixels at standard
/// density), and never so wide that the editor column falls below its widest
/// editor's minimum width; when both cannot hold, the minimum wins and the
/// content overflows. Between the columns lies a resize zone ResizeWidth wide
/// (8 logical pixels at standard density): a primary drag there resizes the
/// label column. While the grid itself has keyboard focus, Left and Right
/// resize it by 8 logical pixels (32 with Shift), with native repeats; Control,
/// Alt and Super chords are left to command bindings. A drag
/// previews on this grid and commits on release; keys commit once every held
/// arrow key is up. Escape, focus loss, hiding and removal cancel a tentative
/// resize and restore the width from before it. [`Self::resizable`]`(false)`
/// removes the zone, the Tab stop and keyboard resizing.
///
/// **Shared width and persistence.** [`Self::bind_label_width`] binds the
/// width to an application signal, so several grids, for example one per
/// [`Accordion`] section, keep their columns aligned: a committed resize on
/// any of them writes the signal, and every grid bound to it follows. A bound
/// width is the application's state and does not take part in workspace
/// persistence. An unbound grid takes part in [`Workspace`] persistence as the
/// `property-grid` kind with one optional field, `label-width`: the
/// nonnegative logical-pixel width the user resized to, a restore produced or
/// [`Self::label_width`] set, absent while the column follows its labels.
/// Restoring a document without the field returns the column to its labels'
/// width; restoring refuses unknown fields, a negative or non-number width and
/// a resize in progress. [`Self::persist`]`(false)` leaves it out.
///
/// **Grouping.** Group rows by placing one grid per section of an
/// [`Accordion`] or in [`Collapsible`]s, with one shared width signal; the
/// sections provide expansion, reveal and their own persistence.
///
/// **Scaling.** Every visible row's label and editor are measured when the
/// grid's layout is invalidated, and unchanged rows reuse their retained
/// measurements otherwise. The grid is meant for the tens to low hundreds of
/// rows an inspector shows; place it in a [`ScrollArea`] when rows can exceed
/// the window, and split very long property lists into sections, which stop
/// the presentation work of closed ones. It is not virtualized.
///
/// Inspection reports a [`SemanticRole::Group`] offering
/// [`SemanticAction::Resize`] while resizable, with the `label_width`,
/// `resizable`, `shared` and `persist` properties.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::prelude::*;
/// use pert_ui::{InspectValue, Viewport};
/// use std::time::Duration;
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(480.0, 320.0, 1.0)?)?;
/// let name = ui.signal(String::from("Channel 1"));
/// let enabled = ui.signal(true);
/// let width = ui.signal(Length::new(120.0)?);
/// ui.mount_tree(Element::new(
///     "inspector",
///     PropertyGrid::new([
///         Property::new("name", "Name", Element::new("input", TextField::new(name))),
///         Property::new("enabled", "Enabled", Element::new("check", Checkbox::new(enabled, ""))),
///     ])
///     .bind_label_width(width.clone()),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// let grid = ui.inspect().components.into_iter().find(|c| c.key == "inspector").unwrap();
/// assert_eq!(grid.details.properties["label_width"], InspectValue::Number(120.0));
/// # Ok::<(), RuntimeError>(())
/// ```
#[must_use]
pub struct PropertyGrid {
    children: Vec<Element>,
    rows: Vec<Row>,
    /// The unbound grid's committed width; `None` until resized or restored.
    preference: Option<Length>,
    /// The workspace number a restore set `preference` from, captured again
    /// unchanged so a round trip keeps the document's exact value, which an
    /// `f32` length may not represent.
    restored: Option<f64>,
    shared: Option<Signal<Length>>,
    /// The shared signal's value as the last update read it.
    shared_value: Option<Length>,
    /// A resize in progress, with the width it currently shows.
    tentative: Option<Length>,
    session: Option<Session>,
    keys: BTreeSet<KeyCode>,
    /// Whether the label column may be resized, as last read.
    resizable: bool,
    /// The described restriction; bound conditions are read at each update.
    permission: Permission,
    persist: bool,
    focused: bool,
    hovered: bool,
    label_overflow: TextOverflow,
    geometry: Cell<Option<Geometry>>,
    placed: RefCell<Vec<Placed>>,
    /// Why the described rows are invalid, reported from `check_description`.
    invalid: Option<LayoutError>,
    on_resize: Option<LabelWidthHandler>,
    /// The preference last reported through `on_resize` and a change waiting.
    report: crate::geometry_report::GeometryReport<Option<Length>>,
}
description_debug!(PropertyGrid);

type LabelWidthHandler =
    Box<dyn FnMut(&mut EventContext, Option<Length>, ChangeOrigin) -> Result<(), RuntimeError>>;

impl PropertyGrid {
    /// Creates a grid over `rows`, in order, resizable and persisted, with
    /// the label column sized to its labels.
    pub fn new(rows: impl IntoIterator<Item = Property>) -> Self {
        let children: Vec<Element> = rows.into_iter().map(Property::into_element).collect();
        let invalid = crate::tabs::check_labeled(&children).err();
        Self {
            children,
            rows: Vec::new(),
            preference: None,
            restored: None,
            shared: None,
            shared_value: None,
            tentative: None,
            session: None,
            keys: BTreeSet::new(),
            resizable: true,
            permission: Permission::allowed(),
            persist: true,
            focused: false,
            hovered: false,
            label_overflow: TextOverflow::Wrap,
            geometry: Cell::new(None),
            placed: RefCell::new(Vec::new()),
            invalid,
            on_resize: None,
            report: crate::geometry_report::GeometryReport::new(None),
        }
    }
    /// Starts an unbound grid with this label column width instead of the
    /// labels' width. A resize or a workspace restore decides after that;
    /// describing the grid again does not change it. Ignored while
    /// [`Self::bind_label_width`] binds the width.
    pub fn label_width(mut self, width: Length) -> Self {
        self.preference = Some(width);
        self.report = crate::geometry_report::GeometryReport::new(Some(width));
        self
    }
    /// Reports the committed label column width preference after it
    /// changed, with what changed it: `Some` width in logical pixels, or
    /// `None` while the column follows its labels.
    ///
    /// Timing class: [after settling](https://github.com/pertyjons/pert-ui/blob/main/docs/contracts.md#after-settling).
    ///
    /// The callback runs on the UI thread from an event phase once the
    /// flush's updates and layout have settled
    /// ([`Component::deliver_notifications`]), at most once per flush, with
    /// the last change of the flush. The origin is
    /// [`ChangeOrigin::User`] when a drag ends at its release or held arrow
    /// keys are all released, and [`ChangeOrigin::Restore`] for
    /// [`Runtime::restore_workspace`]. A resize in progress is a preview and
    /// is not reported; Escape, focus loss and revoking [`Self::resizable`]
    /// report nothing. With [`Self::bind_label_width`] the grid reports its
    /// own committed resizes, and a write to the signal by the application or
    /// another grid is the application's own state, not reported here. The
    /// callback may write bindings; its error propagates from the flush after
    /// the other pending deliveries, the width stays and nothing is replayed.
    /// Describing the grid again replaces the callback, including removing
    /// it; a description without one discards a change not yet delivered.
    pub fn on_resize(
        mut self,
        on_resize: impl FnMut(
            &mut EventContext,
            Option<Length>,
            ChangeOrigin,
        ) -> Result<(), RuntimeError>
        + 'static,
    ) -> Self {
        self.on_resize = Some(Box::new(on_resize));
        self
    }
    /// Records a change for `on_resize`, when there is one.
    fn changed(&self, origin: ChangeOrigin) {
        if self.on_resize.is_some() {
            self.report.record(origin);
        }
    }
    /// Binds the label column width to an application signal, replacing an
    /// earlier binding. Bind several grids to one signal to align their
    /// columns. The grid writes the signal once when a resize commits, from
    /// the event phase; the application may write it at any other time. The
    /// width is clamped to what each grid's allocation permits without
    /// writing the clamped value back. A bound grid stays out of workspace
    /// persistence.
    pub fn bind_label_width(mut self, width: impl Into<Signal<Length>>) -> Self {
        let width: Signal<Length> = width.into();
        self.shared = Some(width);
        self
    }
    /// Restricts whether the user may resize the label column: `true` (the
    /// default), `false`, or a bound [`Permission`]. Each call combines with the
    /// permission the description already holds and the stricter wins
    /// ([`Permission::and`]), so `resizable(false)` cannot be lifted by a later
    /// bound permission.
    ///
    /// A bound permission is read at each update. When it turns false, a
    /// resize in progress, by a pointer drag or held arrow keys, is cancelled
    /// and the width from before it restored; when it turns true again, the
    /// next press can resize. Disabling a mounted grid by description cancels
    /// the same way.
    pub fn resizable(mut self, permission: impl Into<Permission>) -> Self {
        self.permission = self.permission.and(permission);
        if let Some(fixed) = self.permission.fixed() {
            self.resizable = fixed;
        }
        self
    }
    /// Chooses how a label wider than the label column is shown. Default
    /// [`TextOverflow::Wrap`] wraps it over several lines. With
    /// [`TextOverflow::Ellipsis`] or [`TextOverflow::MiddleEllipsis`] each
    /// label stays on one line and loses the grapheme clusters that do not
    /// fit, replaced by "…". The full label stays the editor's name, so
    /// inspection and assistive technology read it whole, and inspection
    /// lists the truncated rows under `truncated_rows`. The label column's
    /// natural width still follows the widest whole label.
    pub fn label_overflow(mut self, overflow: TextOverflow) -> Self {
        self.label_overflow = overflow;
        self
    }
    /// Takes part in workspace persistence (`true`, the default) or stays out
    /// of it (`false`). A bound grid never takes part.
    pub fn persist(mut self, persist: bool) -> Self {
        self.persist = persist;
        self
    }
    /// The committed width preference: the bound value, or the unbound
    /// grid's preference, ignoring a resize in progress.
    fn committed(&self) -> Option<Length> {
        if self.shared.is_some() {
            self.shared_value
        } else {
            self.preference
        }
    }
    /// The width preference the layout uses.
    fn current(&self) -> Option<Length> {
        self.tentative.or_else(|| self.committed())
    }
    fn label_of(&self, key: &str) -> &str {
        self.rows
            .iter()
            .find(|row| row.key == key)
            .map_or("", |row| row.label.as_str())
    }
    /// Resolves the label column width for an allocation of `width`, or for
    /// the preferred width when `None`.
    fn resolve(
        &self,
        cx: &MeasureContext<'_>,
        width: Option<f32>,
    ) -> Result<(f32, f32, f32), RuntimeError> {
        let divider = cx.metric(MetricRole::ResizeWidth);
        let floor = cx.metric(MetricRole::ControlWidth);
        let mut natural = 0.0_f32;
        let mut editor_minimum = 0.0_f32;
        for index in 0..cx.len() {
            let label = self.label_of(cx.key(index)?);
            natural = natural.max(cx.text(label, None, TextStyle::Body)?.size().width());
            editor_minimum = editor_minimum.max(cx.limits(index)?.minimum().width());
        }
        let maximum = width.map_or(f32::MAX, |width| {
            (width - divider - editor_minimum).max(floor)
        });
        let preferred = match self.current() {
            Some(width) => width.get(),
            None => width.map_or(natural, |width| natural.min(width / 2.0)),
        };
        Ok((preferred.clamp(floor, maximum.max(floor)), floor, maximum))
    }
    /// Places every visible row for an allocation `width` wide, returning the
    /// total size, the label column width, its bounds and the placements.
    fn layout(
        &self,
        cx: &MeasureContext<'_>,
        width: Option<f32>,
    ) -> Result<(Size, Geometry, Vec<Arranged>), RuntimeError> {
        let (label, minimum, maximum) = self.resolve(cx, width)?;
        let divider = cx.metric(MetricRole::ResizeWidth);
        let gap = cx.metric(MetricRole::ItemGap);
        let line = cx.metric(MetricRole::ControlHeight);
        let editor_x = label + divider;
        let editor_width = width.map(|width| (width - editor_x).max(0.0));
        let mut rows = Vec::with_capacity(cx.len());
        let mut y = 0.0_f32;
        let mut widest = 0.0_f32;
        for index in 0..cx.len() {
            let key = cx.key(index)?;
            let fitted = cx.fitted_text(
                self.label_of(key),
                label,
                TextStyle::Body,
                self.label_overflow,
            )?;
            let truncated = fitted.is_truncated().then(|| key.to_owned());
            let text = fitted.into_layout();
            let editor = cx.measure(index, Constraints::new(editor_width, None)?)?;
            widest = widest.max(editor.width());
            let text_height = text.size().height();
            let offset = if text_height < line && editor.height() >= line {
                (line - text_height) / 2.0
            } else {
                0.0
            };
            let height = editor.height().max(text_height + offset);
            if index > 0 {
                y += gap;
            }
            rows.push((
                Rect::new(
                    editor_x,
                    y,
                    editor_width.unwrap_or(editor.width()),
                    editor.height(),
                )?,
                Placed {
                    text,
                    x: 0.0,
                    y: y + offset,
                    truncated,
                },
            ));
            y += height;
        }
        let total = width.unwrap_or(editor_x + widest);
        let geometry = Geometry {
            label,
            divider: Rect::new(label, 0.0, divider, y)?,
            minimum,
            maximum,
        };
        Ok((Size::new(total, y)?, geometry, rows))
    }
    /// Moves the tentative width, previewing it on this grid only.
    fn move_to(&mut self, cx: &mut EventContext, width: f32) -> Result<(), RuntimeError> {
        let Some(geometry) = self.geometry.get() else {
            return Ok(());
        };
        let width =
            Length::new(width.clamp(geometry.minimum, geometry.maximum.max(geometry.minimum)))?;
        if self.tentative != Some(width) {
            self.tentative = Some(width);
            cx.invalidate(Invalidation::Layout);
        }
        Ok(())
    }
    /// Ends a session, writing its width as the committed preference.
    fn commit(&mut self) -> Result<(), RuntimeError> {
        self.session = None;
        self.keys.clear();
        if let Some(width) = self.tentative.take() {
            self.changed(ChangeOrigin::User);
            match &self.shared {
                Some(shared) => {
                    shared.set(width)?;
                    self.shared_value = Some(width);
                }
                None => {
                    self.preference = Some(width);
                    self.restored = None;
                }
            }
        }
        Ok(())
    }
    /// Abandons a session; the committed width shows again.
    fn cancel(&mut self) -> bool {
        self.session = None;
        self.keys.clear();
        self.tentative.take().is_some()
    }
}
impl Component for PropertyGrid {
    fn reconcile(&mut self, mut next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        // A callback that starts, or a description without one, takes the
        // committed width as known.
        let on_resize = next.on_resize.take();
        if on_resize.is_none() || self.on_resize.is_none() {
            self.report.discard(self.committed());
        }
        self.on_resize = on_resize;
        let same = match (&self.shared, &next.shared) {
            (None, None) => true,
            (Some(a), Some(b)) => a.read_only().same_source(&b.read_only()),
            _ => false,
        };
        if !same {
            self.shared = next.shared;
            self.shared_value = None;
            self.cancel();
            // Another binding's value is the application's, not a change.
            self.report.discard(self.committed());
            invalidation = Some(Invalidation::Layout);
        }
        if !self.permission.same_as(&next.permission) {
            // A fixed permission applies now; a bound one at the next update,
            // which cancels there.
            if let Some(fixed) = next.permission.fixed() {
                self.resizable = fixed;
                if !fixed && self.cancel() {
                    invalidation = Some(Invalidation::Layout);
                }
            }
            self.permission = next.permission;
            invalidation = invalidation.max(Some(Invalidation::Paint));
        }
        self.persist = next.persist;
        if self.label_overflow != next.label_overflow {
            self.label_overflow = next.label_overflow;
            invalidation = Some(Invalidation::Layout);
        }
        invalidation
    }
    fn check_description(&self) -> Result<(), RuntimeError> {
        // The rows were taken when the element was built; the constructor
        // checked them.
        self.invalid
            .clone()
            .map_or(Ok(()), |error| Err(error.into()))
    }
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn focusable(&self) -> bool {
        self.resizable && self.geometry.get().is_some() && !self.rows.is_empty()
    }
    fn cursor(&self, point: Point) -> Option<CursorIcon> {
        (self.resizable
            && self.geometry.get().is_some_and(|geometry| {
                self.session
                    .is_some_and(|session| matches!(session, Session::Pointer { .. }))
                    || geometry.divider.contains(point)
            }))
        .then_some(CursorIcon::ResizeHorizontal)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.hovered {
            self.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        if let Some((width, origin)) = self.report.take(self.committed())
            && let Some(on_resize) = &mut self.on_resize
        {
            on_resize(cx, width, origin)?;
        }
        Ok(())
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.report.set_notifier(|| cx.notifier());
        let rows = cx
            .children()
            .iter()
            .map(|child| {
                let label = child
                    .label()
                    .filter(|label| !label.trim().is_empty())
                    .ok_or(LayoutError::BlankLabel)?
                    .to_owned();
                Ok(Row {
                    key: child.key().to_owned(),
                    label,
                })
            })
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        let changed = rows.len() != self.rows.len()
            || rows
                .iter()
                .zip(&self.rows)
                .any(|(a, b)| a.key != b.key || a.label != b.label);
        self.rows = rows;
        if let Some(shared) = &self.shared {
            let value = cx.read(shared)?;
            if self.shared_value != Some(value) {
                self.shared_value = Some(value);
                // The application's write is its own state, not reported.
                self.report.discard(Some(value));
                // An application write wins over a resize in progress.
                if self.cancel() {
                    cx.cancel_input();
                }
                cx.request_layout();
            }
        }
        if changed {
            cx.request_layout();
        }
        let allowed = self.permission.read(cx)?;
        if allowed != self.resizable {
            if !allowed {
                if self.cancel() {
                    cx.request_layout();
                }
                cx.cancel_input();
            }
            // Reading the permission subscribes this update to layout and
            // paint, so the change is drawn.
            self.resizable = allowed;
        }
        self.focused = cx.focused() && self.resizable;
        Ok(())
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            return Ok(SizeLimits::default());
        }
        let mut editor = 0.0_f32;
        for index in 0..cx.len() {
            editor = editor.max(cx.limits(index)?.minimum().width());
        }
        Ok(SizeLimits::at_least(Size::new(
            cx.metric(MetricRole::ControlWidth) + cx.metric(MetricRole::ResizeWidth) + editor,
            0.0,
        )?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        Ok(self.layout(cx, available.width())?.0)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            self.geometry.set(None);
            self.placed.borrow_mut().clear();
            return Ok(());
        }
        let (_, geometry, rows) = self.layout(cx.measurement(), Some(size.width()))?;
        let mut placed = Vec::with_capacity(rows.len());
        for (index, (rect, label)) in rows.into_iter().enumerate() {
            cx.place(index, rect)?;
            placed.push(label);
        }
        self.geometry.set(Some(geometry));
        *self.placed.borrow_mut() = placed;
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let color = painter.color(ColorRole::OnSurface);
        let column = self.geometry.get().map(|geometry| geometry.label);
        for placed in self.placed.borrow().iter() {
            match column {
                // A label whose ellipsis alone is wider than the column stays
                // inside it.
                Some(width) if placed.truncated.is_some() => {
                    let height = placed.text.size().height();
                    painter.with_clip(
                        Rect::new(placed.x, placed.y, width, height)?,
                        |painter| {
                            placed.text.paint(
                                painter,
                                crate::Point::new(placed.x, placed.y)?,
                                color,
                            )
                        },
                    )?;
                }
                _ => placed
                    .text
                    .paint(painter, crate::Point::new(placed.x, placed.y)?, color)?,
            }
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(geometry) = self.geometry.get() else {
            return Ok(());
        };
        let active = self.session.is_some() || self.focused;
        if !self.resizable || !(active || self.hovered) {
            return Ok(());
        }
        let [x, y, w, h] = geometry.divider.values();
        let thickness = painter.metric(MetricRole::BorderWidth).min(w);
        let role = if self.focused {
            ColorRole::FocusRing
        } else if self.session.is_some() {
            ColorRole::Primary
        } else {
            ColorRole::ResizeHandleHover
        };
        painter.fill_rect(
            Rect::new(x + (w - thickness) / 2.0, y, thickness, h)?,
            painter.color(role),
        );
        Ok(())
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        let point = cx.to_local(event.position());
        let over = !event.is_outside()
            && self.resizable
            && self
                .geometry
                .get()
                .zip(point)
                .is_some_and(|(geometry, point)| geometry.divider.contains(point));
        if self.hovered != over {
            self.hovered = over;
            cx.invalidate(Invalidation::Paint);
        }
        if !self.resizable {
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                if !over || matches!(self.session, Some(Session::Keys)) {
                    return Ok(InputOutcome::Ignored);
                }
                let (Some(geometry), Some(point)) = (self.geometry.get(), point) else {
                    return Ok(InputOutcome::Ignored);
                };
                self.cancel();
                self.session = Some(Session::Pointer {
                    position: point.x(),
                    start: geometry.label,
                });
                cx.capture_pointer()?;
                cx.invalidate(Invalidation::Paint);
            }
            PointerKind::Move => {
                let (Some(Session::Pointer { position, start }), Some(point)) =
                    (self.session, point)
                else {
                    return Ok(InputOutcome::Ignored);
                };
                self.move_to(cx, start + point.x() - position)?;
            }
            PointerKind::Release(PointerButton::Primary)
                if matches!(self.session, Some(Session::Pointer { .. })) =>
            {
                cx.release_pointer();
                self.commit()?;
                cx.invalidate(Invalidation::Layout);
            }
            _ => return Ok(InputOutcome::Ignored),
        }
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        // Escape belongs to the grid only while it owns a tentative resize.
        if control == KeyCode::Escape {
            if self.session.is_none() {
                return Ok(InputOutcome::Ignored);
            }
            self.cancel();
            cx.release_pointer();
            cx.invalidate(Invalidation::Layout);
            return Ok(InputOutcome::Handled);
        }
        if !self.resizable || !cx.focused() {
            return Ok(InputOutcome::Ignored);
        }
        let direction = match control {
            KeyCode::ArrowLeft => -1.0,
            KeyCode::ArrowRight => 1.0,
            _ => return Ok(InputOutcome::Ignored),
        };
        if matches!(self.session, Some(Session::Pointer { .. })) {
            return Ok(InputOutcome::Handled);
        }
        // A new Control, Alt or Super chord is left to command bindings; keys
        // the divider already holds keep resizing.
        if event.transition == KeyTransition::Pressed
            && !self.keys.contains(&event.code)
            && !crate::activation::steps_with(keyboard.modifiers())
        {
            return Ok(InputOutcome::Ignored);
        }
        match event.transition {
            KeyTransition::Pressed => {
                self.session = Some(Session::Keys);
                self.keys.insert(event.code);
            }
            KeyTransition::Repeated if !self.keys.contains(&event.code) => {
                return Ok(InputOutcome::Handled);
            }
            KeyTransition::Released | KeyTransition::UnmatchedRelease => {
                self.keys.remove(&event.code);
                if self.keys.is_empty() && self.session.is_some() {
                    self.commit()?;
                    cx.invalidate(Invalidation::Paint);
                }
                return Ok(InputOutcome::Handled);
            }
            _ => {}
        }
        if let Some(geometry) = self.geometry.get() {
            let step = if keyboard.modifiers().shift {
                32.0
            } else {
                8.0
            };
            self.move_to(cx, geometry.label + direction * step)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        self.input_reset();
        self.focused = false;
        self.hovered = false;
        cx.release_pointer();
        cx.invalidate(Invalidation::Layout);
    }
    fn input_reset(&mut self) {
        self.cancel();
    }
    fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
        if !self.persist || self.shared.is_some() {
            return Ok(None);
        }
        let state = ViewState::new("property-grid")?;
        Ok(Some(match self.preference {
            Some(width) => {
                let number = self.restored.unwrap_or(f64::from(width.get()));
                state.with("label-width", WorkspaceValue::number(number)?)?
            }
            None => state,
        }))
    }
    fn restore_workspace_state(
        &mut self,
        state: &ViewState,
    ) -> Result<Invalidation, RestoreRejection> {
        state.expect_only(&["label-width"])?;
        let number = match state.get("label-width") {
            Some(_) => Some(state.number("label-width")?),
            None => None,
        };
        let width = number
            .map(|number| {
                if number < 0.0 {
                    return Err(RestoreRejection::OutOfRange("label-width".to_owned()));
                }
                Length::new(number as f32)
                    .map_err(|_| RestoreRejection::OutOfRange("label-width".to_owned()))
            })
            .transpose()?;
        if self.session.is_some() {
            return Err(RestoreRejection::Busy);
        }
        self.restored = number;
        if self.preference == width {
            return Ok(Invalidation::Paint);
        }
        self.preference = width;
        self.changed(ChangeOrigin::Restore);
        Ok(Invalidation::Layout)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut semantics = Semantics::new(SemanticRole::Group);
        if self.resizable {
            semantics = semantics.enabled_actions([SemanticAction::Resize]);
        }
        let mut details = ComponentDetails {
            semantics: Some(semantics),
            ..ComponentDetails::default()
        };
        if let Some(geometry) = self.geometry.get() {
            details.properties.insert(
                "label_width".into(),
                InspectValue::Number(f64::from(geometry.label)),
            );
        }
        details
            .properties
            .insert("resizable".into(), InspectValue::Bool(self.resizable));
        details
            .properties
            .insert("shared".into(), InspectValue::Bool(self.shared.is_some()));
        details
            .properties
            .insert("persist".into(), InspectValue::Bool(self.persist));
        if self.label_overflow != TextOverflow::Wrap {
            let rows = self
                .placed
                .borrow()
                .iter()
                .filter_map(|placed| {
                    placed
                        .truncated
                        .as_ref()
                        .map(|key| InspectValue::Text(key.clone()))
                })
                .collect();
            details
                .properties
                .insert("truncated_rows".into(), InspectValue::List(rows));
        }
        details
    }
}
