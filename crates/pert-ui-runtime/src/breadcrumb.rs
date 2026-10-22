//! A horizontal path of keyed segments whose middle collapses into a menu.
use crate::activation::{ActivationState, activation_key, key_result, pointer_result};
use crate::layout::centered;
use crate::link::{LinkPaint, link_semantics, paint_link};
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type ActivateFn = Box<dyn FnMut(&mut EventContext, &str) -> Result<(), RuntimeError>>;
/// The owner's activation callback, shared by the segments and the menu rows
/// so a new description replaces it without describing the children again.
type SharedActivate = Rc<RefCell<ActivateFn>>;

/// The key of the overflow menu among a breadcrumb's children.
const OVERFLOW: &str = "overflow";

/// One keyed segment of a [`Breadcrumb`].
///
/// The key identifies the segment, for example a path, and is what activation
/// reports; the label is presentation and may change without changing which
/// segment it is.
///
/// Two segments are equal when they have the same key and leading icon and
/// their labels show the same text without reading anything: equal fixed
/// content or the same binding (see [`TextSource::same_as`]).
#[derive(Clone, Debug)]
#[must_use]
pub struct BreadcrumbSegment {
    key: String,
    label: TextSource,
    /// The replaceable leading icon slot, composed before the label.
    leading: Option<IconSource>,
}
impl PartialEq for BreadcrumbSegment {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.leading == other.leading && self.label.same_as(&other.label)
    }
}
impl BreadcrumbSegment {
    /// Creates a segment with an application key independent of its label.
    ///
    /// The label is a [`TextSource`]: fixed plain or rich text, or a binding
    /// the segment follows, such as a translated folder name. The mounted
    /// breadcrumb reads a binding in its update and lays the row out again,
    /// collapsing or expanding segments as the new width requires; the
    /// overflow menu's row follows it too. [`Breadcrumb::new`] rejects a blank
    /// fixed label, including one with an icon, so every segment has
    /// meaningful presentation and semantics; a bound label that reads blank
    /// keeps the segment's last text and fails the breadcrumb's update with
    /// [`LayoutError::BlankLabel`].
    pub fn new(key: impl Into<String>, label: impl Into<TextSource>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            leading: None,
        }
    }
    /// Replaces the leading icon slot: one icon shown before the label with
    /// the themed icon/text gap, independent of the label. A second call
    /// replaces the icon rather than adding one, as for `Tab` and `Choice`.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{BreadcrumbSegment, Icon, RichText};
    /// let segment = BreadcrumbSegment::new("/home", "Home")
    ///     .leading_icon(Icon::Settings)
    ///     .leading_icon(Icon::Folder);
    /// assert_eq!(segment.content(), RichText::from("Home").leading_icon(Icon::Folder));
    /// ```
    pub fn leading_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.leading = Some(icon.into());
        self
    }
    /// The presented label of a fixed label: its text with the leading icon
    /// slot composed before it. A bound label is read by the mounted
    /// breadcrumb, so for it this holds only the icon slot.
    pub fn content(&self) -> RichText {
        self.presented(&self.initial())
    }
    /// The label's text before any read: fixed content, or nothing for a
    /// binding.
    fn initial(&self) -> RichText {
        self.label.fixed().cloned().unwrap_or_default()
    }
    /// `text` with this segment's leading icon slot composed before it.
    fn presented(&self, text: &RichText) -> RichText {
        crate::controls::adorned_content(text, self.leading.as_ref(), None).into_owned()
    }
    /// The segment's key.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

/// A horizontal path of keyed segments, such as a folder path or a location in
/// a document outline. Activating a segment reports its key to the owner.
///
/// Segments are in path order, outermost first. The last segment is the
/// current location: it is shown in the surface text color and is neither a
/// Tab stop nor activatable. Every other segment is a link with its own Tab
/// stop that activates on a matched primary press and release or on unmodified Enter or
/// Space, the same rule as [`Button`], and is drawn like a [`Link`] whose
/// underline shows on hover. A themed chevron separates the segments.
///
/// When the segments do not fit the allocated width, the middle segments
/// collapse into a menu behind a "…" button placed after the first segment,
/// starting with the one after the first, so the root and the current location
/// stay visible as long as possible. The menu is an ordinary [`Menu`]; choosing
/// a row reports that segment's key like activating it. The decision depends
/// only on the width and the labels and is applied within the same flush, so a
/// frame never shows the overflowing row. If even the first segment, the
/// button and the last segment do not fit, the row is clipped.
///
/// Identity is by key: an owner that describes the breadcrumb again with a new
/// path keeps the state and focus of every segment whose key survives, and a
/// focused segment that collapses into the menu loses focus under the ordinary
/// hiding rules. The overflow menu's child key is `overflow`, which segments
/// may not use.
///
/// Inspection reports the breadcrumb as a [`SemanticRole::Group`] named
/// "Breadcrumb" with the number of collapsed segments as `collapsed`;
/// activatable segments report [`SemanticRole::Link`] with
/// [`SemanticAction::Activate`] and the current one [`SemanticRole::Label`].
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{Breadcrumb, BreadcrumbSegment, Element, Runtime};
///
/// let mut ui = Runtime::new();
/// let location = ui.signal("/home/projects".to_owned());
/// let target = location.clone();
/// let path = Breadcrumb::new(
///     [
///         BreadcrumbSegment::new("/", "Root"),
///         BreadcrumbSegment::new("/home", "home"),
///         BreadcrumbSegment::new("/home/projects", "projects"),
///     ],
///     move |_, key| target.set(key.to_owned()),
/// )?;
/// ui.mount_tree(Element::new("path", path))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct Breadcrumb {
    segments: Vec<BreadcrumbSegment>,
    /// Each segment's label text as last read, without its icon slot.
    texts: Vec<RichText>,
    activate: SharedActivate,
    /// Whether the children must be described again on the next update.
    stale: bool,
    /// Whether that description needs a new, closed overflow menu. A change
    /// of label text alone keeps the menu's open state.
    new_menu: bool,
    /// The overflow menu's open state in the current description.
    open: Option<Signal<bool>>,
    /// Collapsed middle segments the children currently show.
    applied: usize,
    /// Collapsed middle segments the last layout asked for.
    desired: Cell<usize>,
    separators: RefCell<Vec<f32>>,
    chevron: RefCell<Option<TextLayout>>,
    /// The string table the overflow menu and name were last taken from.
    strings: crate::UiStrings,
}
description_debug!(Breadcrumb);
impl Breadcrumb {
    /// Creates a breadcrumb over `segments`, outermost first, that reports the
    /// key of an activated segment or chosen overflow row to `on_activate`.
    ///
    /// The breadcrumb needs no `Runtime`, so an owner can describe it again
    /// from its own update, for example after navigating. The callback runs on
    /// the UI thread in the event phase and may write bindings; an error
    /// propagates to the caller of the input method. An empty list shows
    /// nothing.
    ///
    /// # Errors
    /// Returns [`LayoutError::BlankKey`] for a blank segment key,
    /// [`LayoutError::BlankLabel`] for a blank fixed label (a bound label is
    /// checked when it is read), [`LayoutError::ReservedKey`]
    /// for the key `overflow`, which the menu uses, and
    /// [`LayoutError::DuplicateKey`] for two segments with the same key.
    pub fn new(
        segments: impl IntoIterator<Item = BreadcrumbSegment>,
        on_activate: impl FnMut(&mut EventContext, &str) -> Result<(), RuntimeError> + 'static,
    ) -> Result<Self, LayoutError> {
        let segments: Vec<BreadcrumbSegment> = segments.into_iter().collect();
        for (index, segment) in segments.iter().enumerate() {
            if segment.key.trim().is_empty() {
                return Err(LayoutError::BlankKey);
            }
            if segment
                .label
                .fixed()
                .is_some_and(|text| text.plain_text().trim().is_empty())
            {
                return Err(LayoutError::BlankLabel);
            }
            if segment.key == OVERFLOW {
                return Err(LayoutError::ReservedKey);
            }
            if segments[..index].iter().any(|old| old.key == segment.key) {
                return Err(LayoutError::DuplicateKey);
            }
        }
        Ok(Self {
            texts: segments.iter().map(BreadcrumbSegment::initial).collect(),
            segments,
            activate: Rc::new(RefCell::new(Box::new(on_activate))),
            stale: true,
            new_menu: true,
            open: None,
            applied: 0,
            desired: Cell::new(0),
            separators: RefCell::new(Vec::new()),
            chevron: RefCell::new(None),
            strings: crate::UiStrings::english(),
        })
    }
    /// The most segments that can collapse: all but the first and the last.
    fn collapsible(&self) -> usize {
        self.segments.len().saturating_sub(2)
    }
    fn collapsed(&self, index: usize) -> bool {
        (1..=self.applied).contains(&index) && index + 1 < self.segments.len()
    }
    fn describe(&self, open: Signal<bool>) -> Result<Vec<Element>, RuntimeError> {
        let last = self.segments.len().saturating_sub(1);
        let mut children = Vec::with_capacity(self.segments.len() + 1);
        for (index, segment) in self.segments.iter().enumerate() {
            children.push(Element::new(
                segment.key.clone(),
                Crumb {
                    key: segment.key.clone(),
                    text: self.texts[index].plain_text(),
                    content: segment.presented(&self.texts[index]),
                    current: index == last,
                    activate: self.activate.clone(),
                    input: ActivationState::default(),
                    prepared: RefCell::new(None),
                },
            ));
            if index == 0 {
                children.push(Element::new(OVERFLOW, self.overflow_menu(open.clone())?));
            }
        }
        Ok(children)
    }
    fn overflow_menu(&self, open: Signal<bool>) -> Result<Menu, RuntimeError> {
        let rows = self
            .segments
            .iter()
            .enumerate()
            .filter(|(index, _)| self.collapsed(*index))
            .map(|(_, segment)| {
                let (activate, key) = (self.activate.clone(), segment.key.clone());
                // The row follows a bound label itself, so a new label never
                // needs a new menu.
                let row = MenuItem::new(segment.key.clone(), segment.label.clone(), move |cx| {
                    (activate.borrow_mut())(cx, &key)
                });
                match &segment.leading {
                    Some(icon) => row.leading_icon(icon.clone()),
                    None => row,
                }
            })
            .collect();
        let name = self.strings.text(crate::StringRole::BreadcrumbOverflow);
        Menu::icon_with_state(open, Icon::MoreHorizontal, name, rows)
            .map_err(|_| LayoutError::InvalidChild.into())
    }
    /// Segment widths in path order and the overflow button width, from the
    /// labels alone, so hidden segments count as well.
    fn widths(&self, cx: &MeasureContext<'_>) -> Result<(Vec<f32>, f32), RuntimeError> {
        let widths = self
            .segments
            .iter()
            .zip(&self.texts)
            .map(|(segment, text)| Ok(crumb_size(cx, &segment.presented(text))?.width()))
            .collect::<Result<Vec<_>, RuntimeError>>()?;
        let icon = cx.rich_text(
            &RichText::new().icon(Icon::MoreHorizontal),
            None,
            TextStyle::Body,
        )?;
        let more = (icon.size().width() + 2.0 * cx.metric(MetricRole::ControlPaddingX))
            .max(cx.metric(MetricRole::ControlHeight));
        Ok((widths, more))
    }
    fn separator_width(cx: &MeasureContext<'_>) -> Result<f32, RuntimeError> {
        Ok(cx
            .rich_text(
                &RichText::new().icon(Icon::ChevronRight),
                None,
                TextStyle::Body,
            )?
            .size()
            .width())
    }
    /// The width of the row with `collapsed` middle segments behind the button.
    fn row_width(widths: &[f32], more: f32, separator: f32, collapsed: usize) -> f32 {
        if widths.is_empty() {
            return 0.0;
        }
        let shown: f32 = widths[0] + widths[1 + collapsed..].iter().sum::<f32>();
        let items = widths.len() - collapsed + usize::from(collapsed > 0);
        shown + if collapsed > 0 { more } else { 0.0 } + (items - 1) as f32 * separator
    }
}
impl Component for Breadcrumb {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        *self.activate.borrow_mut() = next.activate.replace(Box::new(|_, _| Ok(())));
        if self.segments != next.segments {
            // A segment described with the same key and label source keeps
            // the text it read.
            self.texts = next
                .segments
                .iter()
                .map(|segment| {
                    self.segments
                        .iter()
                        .position(|old| old.key == segment.key && old.label.same_as(&segment.label))
                        .map_or_else(|| segment.initial(), |index| self.texts[index].clone())
                })
                .collect();
            self.segments = next.segments;
            self.applied = self.applied.min(self.collapsible());
            self.stale = true;
            self.new_menu = true;
            Some(Invalidation::Layout)
        } else {
            None
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Bound labels first, so the description below uses their text. A
        // blank read keeps the segment's last text and fails this update
        // once everything else is current.
        let mut blank = false;
        for (segment, text) in self.segments.iter().zip(self.texts.iter_mut()) {
            if !segment.label.is_bound() {
                continue;
            }
            let read = segment.label.read(cx)?;
            if read.plain_text().trim().is_empty() {
                blank = true;
            } else if *text != read {
                *text = read;
                self.stale = true;
            }
        }
        let strings = cx.strings()?;
        if strings != self.strings {
            self.strings = strings;
            self.stale = true;
            self.new_menu = true;
        }
        let desired = self.desired.get().min(self.collapsible());
        if desired != self.applied {
            self.applied = desired;
            self.stale = true;
            self.new_menu = true;
        }
        if self.stale {
            self.stale = false;
            // A new path or collapse starts with a closed overflow menu, so
            // its rows are never shown for a path that has since changed; a
            // changed label alone keeps it as it is.
            let open = match (&self.open, std::mem::take(&mut self.new_menu)) {
                (Some(open), false) => open.clone(),
                _ => {
                    let open = cx.signal(false);
                    self.open = Some(open.clone());
                    open
                }
            };
            cx.request_children(self.describe(open)?);
            cx.request_layout();
        }
        if blank {
            return Err(LayoutError::BlankLabel.into());
        }
        Ok(())
    }
    fn child_visible(&self, key: &str) -> bool {
        if key == OVERFLOW {
            return self.applied > 0;
        }
        self.segments
            .iter()
            .position(|segment| segment.key == key)
            .is_none_or(|index| !self.collapsed(index))
    }
    fn clips_children(&self) -> bool {
        true
    }
    fn measure_limits(&self, cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
        let (widths, more) = self.widths(cx)?;
        let separator = Self::separator_width(cx)?;
        let narrowest = Self::row_width(&widths, more, separator, self.collapsible());
        Ok(SizeLimits::at_least(Size::new(
            narrowest.min(Self::row_width(&widths, more, separator, 0)),
            cx.metric(MetricRole::ControlHeight),
        )?))
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let (widths, more) = self.widths(cx)?;
        let full = Self::row_width(&widths, more, Self::separator_width(cx)?, 0);
        let width = available.width().map_or(full, |limit| limit.min(full));
        Ok(Size::new(width, cx.metric(MetricRole::ControlHeight))?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let m = cx.measurement();
        let (widths, more) = self.widths(m)?;
        let separator = Self::separator_width(m)?;
        let desired = (0..=self.collapsible())
            .find(|collapsed| {
                Self::row_width(&widths, more, separator, *collapsed) <= size.width() + 0.5
            })
            .unwrap_or(self.collapsible());
        self.desired.set(desired);
        if desired != self.applied {
            cx.request_update();
        }
        *self.chevron.borrow_mut() = Some(m.rich_text(
            &RichText::new().icon(Icon::ChevronRight),
            None,
            TextStyle::Body,
        )?);
        let mut separators = Vec::new();
        let mut x = 0.0;
        for index in 0..cx.len() {
            if index > 0 {
                separators.push(x);
                x += separator;
            }
            let child = cx.measure(index, Constraints::new(None, Some(size.height()))?)?;
            let height = child.height().min(size.height());
            cx.place(
                index,
                Rect::new(x, centered(size.height(), height), child.width(), height)?,
            )?;
            x += child.width();
        }
        *self.separators.borrow_mut() = separators;
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        if let Some(chevron) = self.chevron.borrow().as_ref() {
            let color = painter.color(ColorRole::Muted);
            for x in self.separators.borrow().iter() {
                chevron.paint(
                    painter,
                    crate::Point::new(*x, centered(size.height(), chevron.size().height()))?,
                    color,
                )?;
            }
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "collapsed".into(),
            InspectValue::Number(self.applied as f64),
        );
        details.semantics = Some(
            Semantics::new(SemanticRole::Group)
                .name(self.strings.text(crate::StringRole::BreadcrumbName)),
        );
        details
    }
}

/// The size of one segment with `content`, shared by the segment's own
/// measurement and the breadcrumb's collapse decision.
fn crumb_size(cx: &MeasureContext<'_>, content: &RichText) -> Result<Size, RuntimeError> {
    let text = cx.rich_text(content, None, TextStyle::Body)?;
    Ok(Size::new(
        text.size().width() + cx.metric(MetricRole::ControlPaddingX),
        cx.metric(MetricRole::ControlHeight),
    )?)
}

/// One segment of a breadcrumb: a link-like activatable label, or the current
/// location as plain text.
struct Crumb {
    key: String,
    text: String,
    content: RichText,
    current: bool,
    activate: SharedActivate,
    input: ActivationState,
    prepared: RefCell<Option<TextLayout>>,
}
impl Component for Crumb {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        let layout = self.content != next.content;
        let paint = self.current != next.current;
        self.key = next.key;
        self.text = next.text;
        self.content = next.content;
        self.current = next.current;
        self.activate = next.activate;
        if self.current {
            self.input.reset();
        }
        if layout {
            Some(Invalidation::Layout)
        } else {
            paint.then_some(Invalidation::Paint)
        }
    }
    fn focusable(&self) -> bool {
        !self.current
    }
    fn cursor(&self, _: Point) -> Option<CursorIcon> {
        (!self.current).then_some(CursorIcon::Pointer)
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        if !hovered && self.input.hovered {
            self.input.hovered = false;
            cx.invalidate(Invalidation::Paint);
        }
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.input.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        crumb_size(cx, &self.content)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, _: Size) -> Result<(), RuntimeError> {
        if !cx.is_empty() {
            return Err(LayoutError::InvalidChild.into());
        }
        *self.prepared.borrow_mut() = Some(cx.measurement().rich_text(
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
        if self.current {
            return Ok(InputOutcome::Ignored);
        }
        if self.input.pointer(cx, event)? {
            (self.activate.borrow_mut())(cx, &self.key)?;
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
        if self.current || !activation_key(control) {
            return Ok(InputOutcome::Ignored);
        }
        let result = key_result(control, self.input.pressed(), cx.modifiers());
        if self.input.keyboard(cx, event, control) {
            (self.activate.borrow_mut())(cx, &self.key)?;
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
        let Some(text) = self.prepared.borrow().clone() else {
            return Ok(());
        };
        let state = if self.current {
            LinkPaint::Plain
        } else {
            LinkPaint::Enabled {
                pressed: self.input.pressed(),
                focused: self.input.focused,
                underline: self.input.hovered,
            }
        };
        paint_link(painter, size, &text, state)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details
            .properties
            .insert("text".into(), InspectValue::Text(self.text.clone()));
        details
            .properties
            .insert("current".into(), InspectValue::Bool(self.current));
        details.validation.interactive = !self.current;
        details.semantics = Some(if self.current {
            Semantics::new(SemanticRole::Label).name(self.text.as_str())
        } else {
            link_semantics(&self.text, true)
        });
        details
    }
}
