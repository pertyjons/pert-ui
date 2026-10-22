//! The field status contract: a label, help text and an application-owned
//! warning or error tied to one control, and their coordination across a form.
//!
//! The library presents status and never decides it. An application checks its
//! own rules and publishes the outcome; nothing here parses, validates or blocks
//! input. This is unrelated to the development diagnostics in `validation`.
use crate::{
    ArrangeContext, Caption, ColorRole, Component, ComponentDetails, Constraints, DrawError,
    Element, EventContext, InputOutcome, InspectValue, Invalidation, KeyEvent, KeyboardState,
    LayoutError, Length, MeasureContext, MetricRole, Painter, ReadSignal, Rect, RichText,
    RuntimeError, SemanticRole, Semantics, Shape, Signal, Size, SizeLimits, TextLayout, TextSource,
    TextStyle, UpdateContext, WidgetId,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// What an application currently reports about one field's content.
///
/// The default is [`FieldStatus::Normal`]. A message is plain text the
/// application has already localized; an empty message still marks the control
/// but shows no line of its own. Errors outrank warnings wherever the two are
/// compared, and a pending check outranks neither: it is not a verdict.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum FieldStatus {
    /// Nothing to report; the field shows its help text, if any.
    #[default]
    Normal,
    /// The content is accepted but deserves attention.
    Warning(String),
    /// The content is not acceptable to the application.
    Error(String),
    /// The application is still checking the content, for example a name
    /// whose availability is checked asynchronously. [`Field`] shows a
    /// spinner beside the message instead of outlining the control; the
    /// spinner animates only while the field is shown.
    Pending(String),
}
impl FieldStatus {
    /// Creates a warning from any text.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::Warning(message.into())
    }
    /// Creates an error from any text.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error(message.into())
    }
    /// Creates a pending check from any text, such as "Checking the name".
    pub fn pending(message: impl Into<String>) -> Self {
        Self::Pending(message.into())
    }
    /// The warning, error or pending text; `None` for [`FieldStatus::Normal`].
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::Normal => None,
            Self::Warning(message) | Self::Error(message) | Self::Pending(message) => Some(message),
        }
    }
    /// Whether this is an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
    /// Whether this is a warning.
    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self, Self::Warning(_))
    }
    /// Whether the application is still checking the content.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }
    // The theme role that marks the control and colors the message; a
    // pending check marks nothing and shows its message in Muted.
    const fn role(&self) -> Option<ColorRole> {
        match self {
            Self::Normal | Self::Pending(_) => None,
            Self::Warning(_) => Some(ColorRole::Warning),
            Self::Error(_) => Some(ColorRole::Error),
        }
    }
    const fn name(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Warning(_) => "warning",
            Self::Error(_) => "error",
            Self::Pending(_) => "pending",
        }
    }
}

/// The statuses of a form's fields, counted in registration order.
///
/// Read it from [`FormFields::summary`], for example to enable a submit button
/// only while [`FormSummary::errors`] is zero.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FormSummary {
    errors: usize,
    warnings: usize,
    pending: usize,
    modified: usize,
    first_error: Option<String>,
}
impl FormSummary {
    /// How many registered fields currently report an error.
    #[must_use]
    pub const fn errors(&self) -> usize {
        self.errors
    }
    /// How many registered fields currently report a warning.
    #[must_use]
    pub const fn warnings(&self) -> usize {
        self.warnings
    }
    /// How many registered fields are still being checked
    /// ([`FieldStatus::Pending`]), for example to keep a submit button
    /// disabled until every check has finished.
    #[must_use]
    pub const fn pending(&self) -> usize {
        self.pending
    }
    /// How many registered fields hold an unsaved change, as their handles
    /// last reported through [`FieldHandle::set_modified`], for example to
    /// ask before leaving a form with unsaved edits.
    #[must_use]
    pub const fn modified(&self) -> usize {
        self.modified
    }
    /// The key of the first field, in registration order, that reports an error.
    #[must_use]
    pub fn first_error(&self) -> Option<&str> {
        self.first_error.as_deref()
    }
}

struct Entry {
    key: String,
    status: Signal<FieldStatus>,
    modified: Rc<Cell<bool>>,
    control: Rc<RefCell<Option<WidgetId>>>,
}
struct FormInner {
    entries: RefCell<Vec<Entry>>,
    summary: Signal<FormSummary>,
}
impl FormInner {
    /// Recounts every registered field; a refused write leaves the previous
    /// summary until the next successful one.
    fn refresh(&self) -> Result<(), RuntimeError> {
        let mut summary = FormSummary::default();
        for entry in self.entries.borrow().iter() {
            if entry.modified.get() {
                summary.modified += 1;
            }
            match entry.status.get()? {
                FieldStatus::Normal => {}
                FieldStatus::Pending(_) => summary.pending += 1,
                FieldStatus::Warning(_) => summary.warnings += 1,
                FieldStatus::Error(_) => {
                    summary.errors += 1;
                    summary.first_error.get_or_insert_with(|| entry.key.clone());
                }
            }
        }
        self.summary.set(summary)
    }
}

/// Form-level coordination of field statuses, owned by the application.
///
/// A form registers its fields by key, in the order a reader meets them. Each
/// registration returns a [`FieldHandle`] the application writes to and hands to
/// the matching [`Field`]. The form keeps one [`FormSummary`] binding current
/// with every write, and can move focus to the first field in error.
///
/// Clones share the same form. Keys identify fields within one form only; they
/// are not element keys and need not match them. A key stays taken while its
/// field is registered: a form with repeatable rows calls
/// [`FieldHandle::remove`] when it removes a row, which drops that field from
/// the summary and frees its key for a later registration. The form holds no
/// component alive and schedules nothing: it changes only when a handle is
/// written or removed.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Column, Element, Field, FieldStatus, FormFields, Runtime, TextField};
/// let mut ui = Runtime::new();
/// let form = FormFields::new(&ui);
/// let name = ui.signal(String::new());
/// let name_status = form.field("name")?;
/// ui.mount_tree(Element::new(
///     "form",
///     Column::new([Element::new(
///         "name-field",
///         Field::new(Element::new("name", TextField::new(name.clone())))
///             .label("Name")
///             .help("Shown on the title page")
///             .form(&name_status),
///     )]),
/// ))?;
/// // The application applies its own rule and publishes the outcome.
/// if name.get()?.is_empty() {
///     name_status.set(FieldStatus::error("A name is required"))?;
/// }
/// assert_eq!(form.summary().get()?.errors(), 1);
/// assert_eq!(form.summary().get()?.first_error(), Some("name"));
/// name_status.clear()?;
/// assert_eq!(form.summary().get()?.errors(), 0);
/// # Ok::<(), pert_ui::RuntimeError>(())
/// ```
#[derive(Clone)]
#[must_use]
pub struct FormFields(Rc<FormInner>);

/// Shows neither the fields nor the summary: formatting must not read a signal
/// inside a runtime phase. Read the summary with [`FormFields::summary`].
impl std::fmt::Debug for FormFields {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormFields").finish_non_exhaustive()
    }
}
impl FormFields {
    /// Creates an empty form on this runtime; its summary starts clear.
    ///
    /// `ui` is any [`StateOwner`](crate::StateOwner): the
    /// [`Runtime`](crate::Runtime) while the application builds its tree, or
    /// the [`UpdateContext`](crate::UpdateContext) or
    /// [`EventContext`](crate::EventContext) of a component that builds this
    /// while it describes its children.
    pub fn new(ui: &impl crate::StateOwner) -> Self {
        Self(Rc::new(FormInner {
            entries: RefCell::new(Vec::new()),
            summary: ui.signal(FormSummary::default()),
        }))
    }
    /// Registers a field after those already registered, starting as
    /// [`FieldStatus::Normal`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::EmptyKey`] for an empty key and
    /// [`RuntimeError::DuplicateKey`] for one registered in this form and not
    /// yet removed with [`FieldHandle::remove`].
    pub fn field(&self, key: impl Into<String>) -> Result<FieldHandle, RuntimeError> {
        let key = key.into();
        let mut entries = self.0.entries.borrow_mut();
        if key.is_empty() {
            return Err(RuntimeError::EmptyKey);
        }
        if entries.iter().any(|entry| entry.key == key) {
            return Err(RuntimeError::DuplicateKey);
        }
        let status = self.0.summary.sibling(FieldStatus::Normal);
        let control = Rc::new(RefCell::new(None));
        let modified = Rc::new(Cell::new(false));
        entries.push(Entry {
            key: key.clone(),
            status: status.clone(),
            modified: modified.clone(),
            control: control.clone(),
        });
        Ok(FieldHandle {
            form: self.0.clone(),
            key,
            status,
            modified,
            control,
        })
    }
    /// The current counts and first error, updated by every handle write.
    #[must_use]
    pub fn summary(&self) -> ReadSignal<FormSummary> {
        self.0.summary.read_only()
    }
    /// Requests keyboard focus for the control of the first field in error, in
    /// registration order, skipping fields whose [`Field`] is not mounted and
    /// fields whose control is hidden, for example on an inactive tab.
    ///
    /// Call it from an input callback, typically a submit button's, after
    /// publishing the statuses. Focus moves after the pending updates and layout
    /// of the same flush, under the rules of
    /// [`EventContext::request_focus_for`], and an enclosing scroll area reveals
    /// the control as for any focus change. Returns whether a request was made;
    /// `false` means no mounted, visible field is in error. A request is still
    /// ignored if the control cannot take focus when it applies, for example
    /// because it is disabled or outside an open modal surface.
    ///
    /// # Errors
    /// Propagates status read failures and the target's `ForeignRuntime`.
    pub fn focus_first_error(&self, cx: &EventContext) -> Result<bool, RuntimeError> {
        for entry in self.0.entries.borrow().iter() {
            if entry.status.get()?.is_error()
                && let Some(control) = entry.control.borrow().as_ref()
                && control.0.is_visible()
            {
                cx.request_focus_for(control)?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

/// The application's write access to one registered field of a [`FormFields`].
///
/// Clones share the same field. Give it to [`Field::form`] so the field follows
/// the status and the form can focus its control.
#[derive(Clone)]
#[must_use]
pub struct FieldHandle {
    form: Rc<FormInner>,
    key: String,
    status: Signal<FieldStatus>,
    modified: Rc<Cell<bool>>,
    control: Rc<RefCell<Option<WidgetId>>>,
}

/// Shows the key, not the status: formatting must not read a signal inside a
/// runtime phase.
impl std::fmt::Debug for FieldHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldHandle")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}
impl FieldHandle {
    /// The key this field was registered under.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    /// Publishes a status and brings the form's summary up to date. An equal
    /// status changes nothing; [`Self::set_changed`] reports whether it changed.
    ///
    /// # Errors
    /// The errors of [`Signal::set`]: `Closed` after the runtime shut down;
    /// `WriteDuringUpdate`, `WriteDuringLayout`, `WriteDuringPaint`,
    /// `WriteDuringInspection` or `WriteDuringDelivery` from inside that phase;
    /// or `BorrowConflict`.
    pub fn set(&self, status: FieldStatus) -> Result<(), RuntimeError> {
        self.set_changed(status).map(|_| ())
    }
    /// Publishes a status like [`Self::set`] and returns whether it changed.
    ///
    /// # Errors
    /// The errors of [`Self::set`].
    pub fn set_changed(&self, status: FieldStatus) -> Result<bool, RuntimeError> {
        let changed = self.status.set_changed(status)?;
        if changed {
            self.form.refresh()?;
        }
        Ok(changed)
    }
    /// Records whether this field holds an unsaved change and brings the
    /// form's [`FormSummary::modified`] count up to date. Returns whether the
    /// state changed. Typically called from an editor's
    /// [`TextField::on_modified`](crate::TextField::on_modified). The state is
    /// separate from the status: a field can be modified and valid.
    ///
    /// # Errors
    /// Same as [`FieldHandle::set`]; on a refused summary write the state is
    /// recorded and the summary catches up with the next successful write.
    pub fn set_modified(&self, modified: bool) -> Result<bool, RuntimeError> {
        if self.modified.replace(modified) == modified {
            return Ok(false);
        }
        self.form.refresh()?;
        Ok(true)
    }
    /// Publishes [`FieldStatus::Normal`].
    ///
    /// # Errors
    /// Same as [`FieldHandle::set`].
    pub fn clear(&self) -> Result<bool, RuntimeError> {
        self.set_changed(FieldStatus::Normal)
    }
    /// A read-only binding of the status, for components other than [`Field`].
    #[must_use]
    pub fn status(&self) -> ReadSignal<FieldStatus> {
        self.status.read_only()
    }
    /// Removes this field from its form, for a form that removes a repeatable
    /// row. The field no longer counts in the summary, which is brought up to
    /// date, [`FormFields::focus_first_error`] no longer reaches its control,
    /// and its key can be registered again; a new registration is a different
    /// field, which this handle never reaches. Returns whether the field was
    /// still registered: removing it again, or through a clone, does nothing.
    ///
    /// The handle stays usable afterwards: a [`Field`] bound to it keeps
    /// presenting its status, and writes change only that presentation.
    ///
    /// # Errors
    /// The summary write's errors, those of [`Self::set`]; the field is removed
    /// even then, and the summary catches up with the next successful write.
    pub fn remove(&self) -> Result<bool, RuntimeError> {
        let removed = {
            let mut entries = self.form.entries.borrow_mut();
            let before = entries.len();
            entries.retain(|entry| !Rc::ptr_eq(&entry.control, &self.control));
            entries.len() != before
        };
        if removed {
            self.form.refresh()?;
        }
        Ok(removed)
    }
}

#[derive(Default)]
struct Prepared {
    label: Option<TextLayout>,
    message: Option<TextLayout>,
    control: Option<Rect>,
    /// Where a pending check's spinner sits, beside the message.
    spinner: Option<Rect>,
}

/// One control with its label above, and help text or a status message below.
///
/// `Field` wraps any single control, built in or custom, so every kind of field
/// presents label, help and status the same way without each control
/// implementing them. The label uses body text in OnSurface. The line below uses
/// caption text: the help text in Muted, replaced while the status is a warning
/// or an error by its message in the Warning or Error role. A warning or error
/// also outlines the control at BorderWidth in that role, above the control's own
/// drawing; the control's focus indication stays visible beneath it. Blocks are
/// separated by half the theme ItemGap. A block with no text takes no space, so
/// a field with help text does not reflow when a one-line message replaces it.
///
/// The control alone decides the field's preferred width, and the label, help
/// and message wrap to it: text never widens the field, so a status change never
/// resizes the control. The control fills the allocated width and keeps its own
/// measured height, limited by what the label and message leave. Focus, input, cursor and the control's own disabled or
/// read-only state are untouched: the field is not focusable and handles no
/// input. Hiding the control with [`Runtime::set_visible`](crate::Runtime::set_visible) hides the label and
/// the line below with it: the field then takes no space and paints nothing. The status is read from the application's binding on update and
/// invalidates this field's layout only.
///
/// Describing a mounted field again (through [`Runtime::reconcile_children`](crate::Runtime::reconcile_children))
/// applies its new label, help text and status binding; the control keeps its
/// identity and state.
///
/// A [`FieldStatus::Pending`] check shows its message in Muted after a small
/// spinner (IconSize, Primary dots) and outlines nothing. The spinner steps
/// every 100 ms through one deadline while the field is shown, stands still
/// under reduced motion, and schedules nothing otherwise or while the field
/// is hidden. A [required](Field::required) field shows " *" after its label.
///
/// Bind the status with [`Field::status`], or with [`Field::form`] when a
/// [`FormFields`] coordinates several fields; see [`FormFields`] for a complete
/// example. Inspection reports `label`, `help`, `status`, `message` and
/// `required`.
#[must_use]
pub struct Field {
    control: Option<Element>,
    label: Caption,
    label_hidden: bool,
    help: Caption,
    source: Option<ReadSignal<FieldStatus>>,
    target: Option<Rc<RefCell<Option<WidgetId>>>>,
    status: FieldStatus,
    required: bool,
    /// The pending spinner's current step and whether motion is reduced.
    step: u32,
    still: bool,
    prepared: RefCell<Prepared>,
    painted: Cell<bool>,
}
description_debug!(Field);
use crate::progress::{DOTS as SPINNER_DOTS, STEP as SPINNER_STEP};
impl Field {
    /// Wraps one control, with no label, no help text and a normal status.
    pub fn new(control: Element) -> Self {
        Self {
            control: Some(control),
            label: Caption::new(""),
            label_hidden: false,
            help: Caption::new(""),
            source: None,
            target: None,
            status: FieldStatus::Normal,
            required: false,
            step: 0,
            still: false,
            prepared: RefCell::default(),
            painted: Cell::new(false),
        }
    }
    /// Marks the field as required: " *" follows the label, the field's
    /// semantics report [`Semantics::required`], and inspection reports
    /// `required`. It decides nothing: the application still publishes an
    /// error for an empty required field if it wants one. The control keeps
    /// its own state; set `required` on a text editor too when its length
    /// limits should apply to an empty value. Default: not required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
    /// Sets the label above the control; empty, the default, shows none.
    ///
    /// The label is a [`TextSource`]: fixed plain or rich content, or a
    /// binding the field follows, such as a translated label. A binding is
    /// read in the field's update and relayouts it when it changes; describing
    /// the field again with the same binding keeps the text already read. The
    /// label's plain text names the control.
    pub fn label(mut self, label: impl Into<TextSource>) -> Self {
        self.label = Caption::new(label);
        self
    }
    /// Keeps the label as the control's name for inspection and accessibility
    /// without drawing it or reserving space for it, for layouts that present
    /// the label elsewhere, such as [`crate::PropertyGrid`]'s label column. Help
    /// text, the status message and the outline are unchanged. Default: the
    /// label is drawn above the control.
    pub fn hide_label(mut self) -> Self {
        self.label_hidden = true;
        self
    }
    /// Sets the help text below the control; empty, the default, shows none.
    /// A status message replaces it while one is shown.
    ///
    /// The help text is a [`TextSource`] with the same rules as
    /// [`Self::label`]; its plain text describes the control.
    pub fn help(mut self, help: impl Into<TextSource>) -> Self {
        self.help = Caption::new(help);
        self
    }
    /// Follows an application-owned status binding, replacing an earlier one.
    pub fn status(mut self, status: impl Into<ReadSignal<FieldStatus>>) -> Self {
        let status: ReadSignal<FieldStatus> = status.into();
        self.source = Some(status);
        self.target = None;
        self
    }
    /// Follows a form's field, and lets [`FormFields::focus_first_error`] reach
    /// this field's control while it is mounted. Replaces an earlier binding.
    pub fn form(mut self, handle: &FieldHandle) -> Self {
        self.source = Some(handle.status());
        self.target = Some(handle.control.clone());
        self
    }
    fn gap(cx: &MeasureContext<'_>) -> f32 {
        cx.metric(MetricRole::ItemGap) / 2.0
    }
    // The line below the control: the status message outranks the help text.
    fn message(&self) -> &str {
        match self.status.message() {
            Some(message) if !message.is_empty() => message,
            _ => self.help.text(),
        }
    }
    /// How far a pending check's message is indented past its spinner.
    fn spinner_indent(&self, cx: &MeasureContext<'_>) -> f32 {
        if self.status.is_pending() {
            cx.metric(MetricRole::IconSize) + cx.metric(MetricRole::IconGap)
        } else {
            0.0
        }
    }
    fn texts(
        &self,
        cx: &MeasureContext<'_>,
        width: Option<f32>,
    ) -> Result<(Option<TextLayout>, Option<TextLayout>), RuntimeError> {
        let indent = self.spinner_indent(cx);
        let prepare = |text: Option<RichText>, style, indent: f32| {
            text.filter(|text| !text.plain_text().is_empty())
                .map(|text| cx.rich_text(&text, width.map(|w| (w - indent).max(1.0)), style))
                .transpose()
        };
        let label = (!self.label_hidden).then(|| {
            let label = self.label.display().into_owned();
            if self.required && !self.label.text().is_empty() {
                label.text(" *")
            } else {
                label
            }
        });
        // A status message is plain text; the help text keeps its styling.
        let message = match self.status.message() {
            Some(message) if !message.is_empty() => RichText::from(message),
            _ => self.help.display().into_owned(),
        };
        Ok((
            prepare(label, TextStyle::Body, 0.0)?,
            prepare(Some(message), TextStyle::Caption, indent)?,
        ))
    }
    fn block(text: &Option<TextLayout>, gap: f32) -> f32 {
        text.as_ref().map_or(0.0, |text| text.size().height() + gap)
    }
}
impl Component for Field {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let mut invalidation = None;
        let label = self.label.reconcile(next.label);
        let help = self.help.reconcile(next.help);
        if label || help || self.label_hidden != next.label_hidden || self.required != next.required
        {
            self.label_hidden = next.label_hidden;
            self.required = next.required;
            invalidation = Some(Invalidation::Layout);
        }
        let same_target = match (&self.target, &next.target) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !ReadSignal::same_optional(&self.source, &next.source) || !same_target {
            if !same_target && let Some(target) = &self.target {
                target.borrow_mut().take();
            }
            self.source = next.source;
            self.target = next.target;
            if self.source.is_none() {
                self.status = FieldStatus::Normal;
            }
            invalidation = Some(Invalidation::Layout);
        }
        invalidation
    }
    fn take_children(&mut self) -> Vec<Element> {
        self.control.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.label.update(cx)?;
        self.help.update(cx)?;
        if let Some(source) = &self.source {
            self.status = cx.read(source)?;
        }
        self.still = cx.motion() == crate::MotionPreference::Reduced;
        if self.status.is_pending() && !self.still {
            // One deadline per step, aligned to the clock as the spinner's is;
            // the runtime withdraws it while the field is hidden.
            let (now, step) = (cx.now().as_nanos(), SPINNER_STEP.as_nanos());
            // An elapsed wake repaints this field, as it does a spinner.
            self.step = ((now / step) % u128::from(SPINNER_DOTS)) as u32;
            let remaining = step - now % step;
            cx.wake_after(std::time::Duration::from_nanos(remaining as u64))?;
        }
        if let Some(target) = &self.target {
            *target.borrow_mut() = cx.children().first().map(|child| child.id().clone());
        }
        Ok(())
    }
    fn unmounted(&mut self) {
        if let Some(target) = &self.target {
            target.borrow_mut().take();
        }
    }
    fn clips_children(&self) -> bool {
        false
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        if cx.is_empty() {
            return Ok(SizeLimits::default());
        }
        let (label, message) = self.texts(cx, None)?;
        let gap = Self::gap(cx);
        let minimum = cx.limits(0)?.minimum();
        Ok(SizeLimits::at_least(Size::new(
            minimum.width(),
            minimum.height() + Self::block(&label, gap) + Self::block(&message, gap),
        )?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        // A hidden control takes its label and line below with it: the field
        // then takes no space, as a hidden child of any container does.
        if cx.is_empty() {
            return Ok(Size::ZERO);
        }
        if cx.len() != 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        let control = cx.measure(0, Constraints::new(available.width(), None)?)?;
        // The control alone decides the width and the text wraps to it. Were the
        // text allowed to widen the field, a shorter message replacing a longer
        // help line would narrow the field and resize the control with it.
        let (label, message) = self.texts(cx, Some(control.width()))?;
        let gap = Self::gap(cx);
        Ok(Size::new(
            control.width(),
            control.height() + Self::block(&label, gap) + Self::block(&message, gap),
        )?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        if cx.is_empty() {
            // A zero-size field is not painted, so clear what painting reports.
            *self.prepared.borrow_mut() = Prepared::default();
            self.painted.set(false);
            return Ok(());
        }
        if cx.len() != 1 {
            return Err(LayoutError::InvalidChild.into());
        }
        let (label, message) = self.texts(cx.measurement(), Some(size.width()))?;
        let gap = Self::gap(cx.measurement());
        let top = Self::block(&label, gap);
        // The control keeps its own measured height, limited by what is left, so
        // text that wraps differently in a wider allocation never resizes it.
        let height = cx
            .measure(0, Constraints::new(Some(size.width()), None)?)?
            .height()
            .min((size.height() - top - Self::block(&message, gap)).max(0.0));
        let control = Rect::new(0.0, top, size.width(), height)?;
        cx.place(0, control)?;
        let spinner = match (&message, self.status.is_pending()) {
            (Some(message), true) => {
                let side = cx
                    .measurement()
                    .metric(MetricRole::IconSize)
                    .min(message.size().height().max(1.0));
                Some(Rect::new(
                    0.0,
                    control.y() + control.height() + gap,
                    side,
                    side,
                )?)
            }
            _ => None,
        };
        *self.prepared.borrow_mut() = Prepared {
            label,
            message,
            control: Some(control),
            spinner,
        };
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let prepared = self.prepared.borrow();
        if let Some(label) = &prepared.label {
            label.paint(
                painter,
                crate::Point::ZERO,
                painter.color(ColorRole::OnSurface),
            )?;
        }
        if let (Some(message), Some(control)) = (&prepared.message, prepared.control) {
            let role = self
                .status
                .role()
                .filter(|_| self.status.message().is_some_and(|text| !text.is_empty()))
                .unwrap_or(ColorRole::Muted);
            let gap = painter.metric(MetricRole::ItemGap) / 2.0;
            let indent = prepared.spinner.map_or(0.0, |spinner| {
                spinner.width() + painter.metric(MetricRole::IconGap)
            });
            message.paint(
                painter,
                crate::Point::new(indent, control.y() + control.height() + gap)?,
                painter.color(role),
            )?;
        }
        if let Some(spinner) = prepared.spinner {
            crate::progress::paint_dots(painter, spinner, self.step, self.still)?;
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let outlined = self.status.role().zip(self.prepared.borrow().control);
        self.painted.set(outlined.is_some());
        if let Some((role, control)) = outlined
            && !control.is_empty()
        {
            let radius = Length::new(painter.metric(MetricRole::CornerRadius))?;
            let width = Length::new(painter.metric(MetricRole::BorderWidth))?;
            painter.stroke_shape(control, Shape::Rounded(radius), width, painter.color(role))?;
        }
        Ok(())
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
        let mut details = ComponentDetails::default();
        let text = |value: &str| InspectValue::Text(value.to_owned());
        details
            .properties
            .insert("label".into(), text(self.label.text()));
        details
            .properties
            .insert("help".into(), text(self.help.text()));
        details
            .properties
            .insert("status".into(), text(self.status.name()));
        details.properties.insert(
            "message".into(),
            text(self.status.message().unwrap_or_default()),
        );
        details
            .properties
            .insert("outlined".into(), InspectValue::Bool(self.painted.get()));
        details
            .properties
            .insert("required".into(), InspectValue::Bool(self.required));
        // The label names the wrapped control, and the line below it, the status
        // message or else the help text, describes it; an error marks it invalid.
        details.semantics = Some(
            Semantics::new(SemanticRole::Field)
                .name(self.label.text())
                .description(self.message())
                .invalid(self.status.is_error())
                .required(self.required)
                .names_children(),
        );
        details
    }
}
