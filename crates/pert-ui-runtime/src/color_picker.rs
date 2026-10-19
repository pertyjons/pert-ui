//! A color editor: a saturation/value area, hue and alpha strips and text entry.
use crate::color_swatch::paint_checkerboard;
use crate::layout::rect;
use crate::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Pixels across the generated saturation/value image; linear sampling
/// stretches it smoothly over the area.
const AREA_PIXELS: u32 = 64;
/// Pixels along the generated hue and alpha gradients.
const STRIP_PIXELS: u32 = 256;
/// Keyboard steps as fractions of a part's range: arrows, Shift+arrows and
/// Page Up/Down, as on the slider.
const STEP: f32 = 0.01;
const FINE_STEP: f32 = 0.001;
const PAGE_STEP: f32 = 0.1;
/// An arrow step on the hue strip, in degrees, and its Shift step.
const HUE_STEP: f32 = 1.0;
const HUE_FINE_STEP: f32 = 0.1;

/// Hue in degrees `0..=360`, and saturation, value and alpha in `0..=1`, over
/// gamma-encoded sRGB channels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Hsva {
    hue: f32,
    saturation: f32,
    value: f32,
    alpha: f32,
}
impl Hsva {
    /// The editing state for `color`, keeping `hint`'s hue where the color
    /// does not determine one (no saturation or no value) and its saturation
    /// where the color does not determine that either (no value).
    /// Whether both states produce the same color: equal saturation, value
    /// and alpha, and hues that are equal or the two ends of the circle.
    fn same_color(self, other: Self) -> bool {
        self.saturation == other.saturation
            && self.value == other.value
            && self.alpha == other.alpha
            && self.hue.rem_euclid(360.0) == other.hue.rem_euclid(360.0)
    }
    fn from_color(color: Color, hint: Self) -> Self {
        let [red, green, blue, alpha] = color.to_srgb();
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let delta = max - min;
        if max <= 0.0 {
            return Self {
                value: 0.0,
                alpha,
                ..hint
            };
        }
        if delta <= 0.0 {
            return Self {
                hue: hint.hue,
                saturation: 0.0,
                value: max,
                alpha,
            };
        }
        let sector = if max == red {
            ((green - blue) / delta).rem_euclid(6.0)
        } else if max == green {
            (blue - red) / delta + 2.0
        } else {
            (red - green) / delta + 4.0
        };
        Self {
            hue: (sector * 60.0).clamp(0.0, 360.0),
            saturation: (delta / max).clamp(0.0, 1.0),
            value: max.clamp(0.0, 1.0),
            alpha,
        }
    }
    /// The gamma-encoded sRGB channels of this state.
    fn srgb(self) -> [f32; 3] {
        let chroma = self.value * self.saturation;
        let sector = (self.hue / 60.0).rem_euclid(6.0);
        let second = chroma * (1.0 - ((sector % 2.0) - 1.0).abs());
        let [red, green, blue] = match sector as u8 {
            0 => [chroma, second, 0.0],
            1 => [second, chroma, 0.0],
            2 => [0.0, chroma, second],
            3 => [0.0, second, chroma],
            4 => [second, 0.0, chroma],
            _ => [chroma, 0.0, second],
        };
        let offset = self.value - chroma;
        [red + offset, green + offset, blue + offset].map(|channel| channel.clamp(0.0, 1.0))
    }
    /// The linear color of this state. Channels are clamped into range, so
    /// only a non-finite component can fail, reported as
    /// [`GeometryError::NonFinite`].
    fn color(self) -> Result<Color, RuntimeError> {
        let [red, green, blue] = self.srgb();
        Color::from_srgb(red, green, blue, self.alpha.clamp(0.0, 1.0))
            .map_err(|_| GeometryError::NonFinite.into())
    }
    fn srgb8(self) -> [u8; 3] {
        // Channels are clamped to 0..=1, so the products fit a byte.
        self.srgb().map(|channel| (channel * 255.0).round() as u8)
    }
}
const DEFAULT_HSVA: Hsva = Hsva {
    hue: 0.0,
    saturation: 0.0,
    value: 0.0,
    alpha: 1.0,
};

/// The committed editing state and the color it produced, so the hue survives
/// a color that does not determine one.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Committed {
    hsva: Hsva,
    color: Color,
}

/// The text entry's bindings, replaced when the application writes a color.
struct Entry {
    hex: Signal<String>,
    channels: [Signal<i64>; 4],
}
impl Entry {
    fn new(like: &Signal<Color>, color: Color) -> Self {
        let bytes = color.to_srgb8();
        Self {
            hex: like.sibling(color.to_hex()),
            channels: bytes.map(|byte| like.sibling(i64::from(byte))),
        }
    }
    fn write(&self, color: Color) -> Result<(), RuntimeError> {
        self.hex.set(color.to_hex())?;
        for (signal, byte) in self.channels.iter().zip(color.to_srgb8()) {
            signal.set(i64::from(byte))?;
        }
        Ok(())
    }
}

/// The application's report of a committed user write.
pub(crate) type ChangeHandler =
    Box<dyn FnMut(&mut crate::EventContext, Color) -> Result<(), RuntimeError>>;
/// The handler a picker reports through, shared with the [`Popover`] of
/// [`ColorPicker::into_popover`] so that a new description reaches it.
pub(crate) type ChangeSlot = Rc<HandlerSlot>;

/// `ColorPicker::on_change`, following redescription.
#[derive(Default)]
pub(crate) struct HandlerSlot {
    handler: RefCell<Option<ChangeHandler>>,
    /// Advances whenever a description replaces the handler, so a handler
    /// that is running when that happens is not put back afterwards.
    generation: Cell<u64>,
}
impl HandlerSlot {
    /// Installs the handler of a new description, or none.
    pub(crate) fn replace_from(&self, next: &Self) {
        let handler = next.handler.borrow_mut().take();
        *self.handler.borrow_mut() = handler;
        self.generation.set(self.generation.get().wrapping_add(1));
    }
    /// Calls the handler with a committed color. No borrow is held while it
    /// runs; a handler a new description installed meanwhile is kept, and
    /// otherwise this one is put back, also after an error, so the next
    /// write reports again.
    fn call(&self, cx: &mut crate::EventContext, color: Color) -> Result<(), RuntimeError> {
        let Some(mut handler) = self.handler.borrow_mut().take() else {
            return Ok(());
        };
        let generation = self.generation.get();
        let result = handler(cx, color);
        if self.generation.get() == generation {
            *self.handler.borrow_mut() = Some(handler);
        }
        result
    }
}

/// State shared by a picker and its parts.
struct Model {
    color: Signal<Color>,
    committed: Signal<Committed>,
    /// The running pointer gesture's state, owned by the part that runs it.
    /// Kept outside a signal so that every cancellation path can clear it,
    /// including those that run where writes are refused.
    /// Each preview is stored with the bound color its gesture began from,
    /// so a part that reads it after an application write ignores it even
    /// before the gesture's own part has seen the write and ended it.
    preview: RefCell<Option<(Color, Hsva)>>,
    /// Notifies the parts that `preview` changed.
    tick: Signal<u64>,
    entry: RefCell<Entry>,
    /// The color the text entry was last written with or created from.
    synced: Cell<Option<Color>>,
    hue_image: Option<RasterImage>,
    alpha: Cell<bool>,
    text_entry: Cell<bool>,
    /// The application's restriction of edits; unlike the options it follows
    /// redescription. `enabled` holds the value last read.
    enablement: RefCell<Permission>,
    enabled: Cell<bool>,
    /// The string table the parts' names and the fields' messages come from,
    /// as the picker's update last read it.
    strings: RefCell<crate::UiStrings>,
    /// `ColorPicker::on_change`, following redescription.
    changed: ChangeSlot,
}
impl Model {
    /// The text for `role` in the table the picker last read.
    fn text(&self, role: crate::StringRole) -> String {
        self.strings.borrow().get(role).to_owned()
    }
    /// The state the parts present: the gesture's preview, or the committed
    /// state while it still produces the bound color, or the bound color with
    /// the committed hue and saturation kept where the color leaves them open.
    fn presented(&self, color: Color, committed: Committed) -> Hsva {
        if let Some((before, preview)) = *self.preview.borrow()
            && before == color
        {
            return preview;
        }
        if committed.color == color {
            committed.hsva
        } else {
            Hsva::from_color(color, committed.hsva)
        }
    }
    /// Untracked current state, for event handlers.
    fn current(&self) -> Result<Hsva, RuntimeError> {
        Ok(self.presented(self.color.get()?, self.committed.get()?))
    }
    fn notify(&self) -> Result<(), RuntimeError> {
        let next = self.tick.get()?.wrapping_add(1);
        self.tick.set(next)
    }
    /// Writes a committed state: the editing state, the bound color and the
    /// text entry, and ends any preview. Returns the color when the bound
    /// value changed, which the caller reports once its own bookkeeping is
    /// done.
    fn commit(&self, hsva: Hsva) -> Result<Option<Color>, RuntimeError> {
        // A state that still describes the bound color, such as a new hue
        // for a color without saturation or the other end of the hue
        // circle, keeps that color exactly instead of rounding it through
        // HSV, so the binding is not rewritten.
        let bound = self.color.get()?;
        let color = if Hsva::from_color(bound, hsva).same_color(hsva) {
            bound
        } else {
            hsva.color()?
        };
        self.write(hsva, color)
    }
    /// Commits a color entered as text exactly, keeping the current hue
    /// where the color leaves it open.
    fn commit_color(&self, color: Color) -> Result<Option<Color>, RuntimeError> {
        self.write(Hsva::from_color(color, self.current()?), color)
    }
    /// Writes the editing state and the color it shows.
    fn write(&self, hsva: Hsva, color: Color) -> Result<Option<Color>, RuntimeError> {
        self.preview.borrow_mut().take();
        self.committed.set(Committed { hsva, color })?;
        let changed = self.color.set_changed(color)?;
        self.entry.borrow().write(color)?;
        self.synced.set(Some(color));
        self.notify()?;
        Ok(changed.then_some(color))
    }
    /// Reports a committed user write that changed the bound value.
    fn report(
        &self,
        cx: &mut crate::EventContext,
        color: Option<Color>,
    ) -> Result<(), RuntimeError> {
        match color {
            Some(color) => self.changed.call(cx, color),
            None => Ok(()),
        }
    }
}

/// Which part of the picker a [`ColorPart`] edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Part {
    Area,
    Hue,
    Alpha,
}

/// A color editor bound to a linear [`Color`]: a saturation/value area, a hue
/// strip, an optional alpha strip and optional text entry for hexadecimal
/// sRGB and 8-bit channels.
///
/// The picker edits in HSV over gamma-encoded sRGB, the space people know
/// from other editors, and writes the bound value as linear [`Color`]. The HSV
/// state is the picker's own, so the hue survives while saturation or value
/// is zero, where the color itself has no hue: dragging the area to black and
/// back keeps the hue it had.
///
/// # Writing the binding
///
/// The bound signal changes only when an edit completes, never during a
/// gesture:
///
/// - A primary press on the area or a strip begins a gesture at the pointer,
///   dragging previews, and the matched release commits the previewed color
///   in one write. The press also focuses the part. Positions beyond the part
///   are clamped to its edge.
/// - Escape during a gesture cancels it, as do focus loss, hiding, removal and
///   any other loss of input context; the parts return to the value from
///   before the gesture, and nothing is written.
/// - An application write to the binding during a gesture ends the gesture
///   without writing: the application's value wins and is shown.
/// - Each key press or repeat on a focused part is one complete edit: Left
///   and Right change the area's saturation, Up and Down its value; on a strip
///   Left/Down decrease and Right/Up increase. A step is 1 % of the range,
///   0.1 % with Shift, except on the hue strip, where it is 1° and 0.1° with
///   Shift; Page Up/Down step 10 % of any range. Home and End move a strip to
///   its ends. Control, Alt and Super chords of these keys are left to
///   command bindings. Keys are ignored while a pointer gesture runs.
/// - The text fields save on Enter or focus loss: the hex field accepts any
///   [`Color::from_hex`] form and shows the canonical lowercase form, the
///   channel fields accept 0–255. A text entry is quantized to 8-bit sRGB.
///   With alpha disabled, hex text with a translucent alpha is refused and the
///   alpha channel field is absent; edits preserve the bound color's exact alpha,
///   including RGB channel saves without rounding alpha to a byte.
///
/// The application can therefore treat every change of the binding as one
/// committed edit, for example to record undo; [`ColorPicker::on_change`]
/// reports each such user write, so the application need not observe the
/// binding. Values the application writes
/// are shown as written. Such a write also gives the text entry fresh
/// bindings, which discards a draft being typed there; the fields and focus
/// stay where they are.
///
/// # Layout and keyboard
///
/// The parts stack vertically with `MetricRole::ItemGap` between them: the
/// area, `MetricRole::ColorAreaSize` high, the hue strip and the alpha strip,
/// each `MetricRole::ColorStripThickness` high, then the entry. Every part
/// fills the picker's width, which is the larger of the area size and the
/// entry's preferred width, limited by the space available. Each part and
/// each field is one Tab stop, in that order.
///
/// # Inspection
///
/// The picker is a [`SemanticRole::Group`] named by [`ColorPicker::name`]
/// ("Color picker" without one) whose value is the bound color's hex text.
/// The area and strips are [`SemanticRole::Slider`]s: the area's value is
/// text naming its saturation and value, a strip's value is a number (hue in
/// degrees, alpha as a fraction). All report `hue`, `saturation`, `value` and
/// `alpha` properties for the state they present.
///
/// Describing the picker again keeps the mounted binding and options; give it
/// a new key to bind it to another signal. Its [`ColorPicker::enabled`]
/// permission is the exception: like every permission it follows the new
/// description.
///
/// ```
/// use pert_ui_runtime::{Color, ColorPicker, Column, Element, KeyAction, KeyCode, Runtime, Viewport};
/// use std::time::Duration;
///
/// let mut ui = Runtime::new();
/// ui.set_viewport(Viewport::new(400.0, 400.0, 1.0)?)?;
/// let color = ui.signal(Color::from_hex("#ff0000")?);
/// let root = ui.mount_tree(Element::new(
///     "page",
///     Column::new([Element::new("picker", ColorPicker::new(color.clone()).alpha(true))]),
/// ))?;
/// ui.flush(Duration::ZERO)?;
/// // Each part is a Tab stop; Shift+Home on the hue strip is one edit.
/// let hue = ui.find_descendant(&root, "hue")?.expect("the hue strip");
/// ui.focus(&hue)?;
/// ui.key_input(KeyCode::End, KeyAction::Press)?;
/// ui.key_input(KeyCode::End, KeyAction::Release)?;
/// // The far end of the strip is red again.
/// assert_eq!(color.get()?.to_hex(), "#ff0000");
/// ui.key_input(KeyCode::ArrowLeft, KeyAction::Press)?;
/// ui.key_input(KeyCode::ArrowLeft, KeyAction::Release)?;
/// assert_ne!(color.get()?.to_hex(), "#ff0000");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[must_use]
pub struct ColorPicker {
    model: Rc<Model>,
    /// The described name; `None` when unset.
    name: Option<TextSource>,
    /// The name as last read, `None` while it is unset or empty.
    shown_name: Option<String>,
    described: bool,
}
impl ColorPicker {
    /// Creates a picker editing `color`, without an alpha strip and with
    /// text entry. The binding belongs to the mounting runtime's UI thread.
    pub fn new(color: impl Into<Signal<Color>>) -> Self {
        let color: Signal<Color> = color.into();
        let initial = color.get().ok();
        let start = initial.unwrap_or(Color::WHITE);
        let hsva = Hsva::from_color(start, DEFAULT_HSVA);
        let committed = color.sibling(Committed { hsva, color: start });
        let tick = color.sibling(0);
        let entry = Entry::new(&color, start);
        let strings = color.shared().strings.borrow().clone();
        Self {
            model: Rc::new(Model {
                committed,
                tick,
                entry: RefCell::new(entry),
                synced: Cell::new(initial),
                preview: RefCell::new(None),
                hue_image: hue_image().ok(),
                alpha: Cell::new(false),
                text_entry: Cell::new(true),
                enablement: RefCell::new(Permission::allowed()),
                enabled: Cell::new(true),
                strings: RefCell::new(strings),
                changed: Rc::default(),
                color,
            }),
            name: None,
            shown_name: None,
            described: false,
        }
    }
    /// Shows the alpha strip and the alpha channel field. Off by default:
    /// edits then keep the bound color's alpha, and translucent hex text is
    /// refused.
    pub fn alpha(self, enabled: bool) -> Self {
        self.model.alpha.set(enabled);
        self
    }
    /// Shows or hides the hex and channel fields. On by default.
    pub fn text_entry(self, enabled: bool) -> Self {
        self.model.text_entry.set(enabled);
        self
    }
    /// Names what the color is for, such as "Track color", for inspection and
    /// for the swatch of [`Self::into_popover`], as a [`TextSource`]: fixed
    /// text, a binding the name follows, or a
    /// [`StringRole`](crate::StringRole) of the installed
    /// [`UiStrings`](crate::UiStrings). Only the plain text is used. Fixed
    /// empty text leaves it unset, and so does a binding or role while it reads
    /// empty; unset, the string table names the picker
    /// ([`StringRole::ColorPickerName`](crate::StringRole::ColorPickerName)).
    /// The swatch receives the same source: unset by fixed empty text, it is
    /// named by [`StringRole::ColorName`](crate::StringRole::ColorName); a
    /// binding or role reading empty leaves it named by its hex value, as
    /// [`ColorSwatch::name`](crate::ColorSwatch::name) describes.
    /// Like the picker's other options, the name stays with the mounted
    /// picker under the same key; a bound or role name keeps following its
    /// source.
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
    /// Disables every part and field: no Tab stops and no edits. The picker
    /// still follows its binding. Equivalent to `enabled(false)`; a later
    /// permission cannot lift the denial.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// Restricts edits and focus with a fixed or reactive permission; allowed
    /// by default. Repeated calls combine restrictions; every condition must
    /// allow. Unlike the picker's options, the permission follows same-key
    /// redescription, which replaces it with the new description's.
    ///
    /// The picker and its parts read a bound permission in their update.
    /// Revoking it ends a pointer gesture without writing the binding, as
    /// Escape does, removes every Tab stop so focus moves away, and disables
    /// the text fields, which keep a draft without saving it. Granting it
    /// again replays nothing. A binding from another runtime fails the update
    /// with [`RuntimeError::ForeignRuntime`].
    pub fn enabled(self, permission: impl Into<Permission>) -> Self {
        let combined = self.model.enablement.borrow().clone().and(permission);
        if let Some(enabled) = combined.fixed() {
            self.model.enabled.set(enabled);
        }
        *self.model.enablement.borrow_mut() = combined;
        self
    }
    /// Reports each committed user write that changed the bound color, with
    /// the color written, for example to record it as one undo step or to
    /// apply it to the application's own document.
    ///
    /// It runs on the UI thread in the event phase of the input that
    /// committed the edit, after the binding and the text fields hold the
    /// color: a pointer release on the area or a strip, a key press or repeat
    /// on a part, or a saved hex or channel field. An edit that leaves the
    /// bound color unchanged, such as moving the hue of a color without
    /// saturation, does not call it, nor do application writes, gestures
    /// ended by Escape, focus loss, hiding, removal or a revoked
    /// [permission](Self::enabled), and refused text. It may write bindings,
    /// including this picker's own, which then shows the application's color
    /// without a further call. Its error propagates from the input call that
    /// committed; the color stays written and the call is not repeated, while
    /// the next edit calls the handler again.
    ///
    /// Unlike the picker's options, the handler follows same-key
    /// redescription: a new description's handler replaces it, and a
    /// description without one removes it. This holds for
    /// [`Self::into_popover`] too, whose retained picker takes the handler
    /// of the popover's new description. Replaces an earlier handler.
    ///
    /// ```
    /// use pert_ui_runtime::{Color, ColorPicker, Element, KeyAction, KeyCode, Runtime, Viewport};
    /// use std::{cell::RefCell, rc::Rc, time::Duration};
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 400.0, 1.0)?)?;
    /// let color = ui.signal(Color::from_hex("#2090a0")?);
    /// let undo: Rc<RefCell<Vec<Color>>> = Rc::default();
    /// let history = undo.clone();
    /// let picker = ColorPicker::new(color.clone()).on_change(move |_, written| {
    ///     history.borrow_mut().push(written);
    ///     Ok(())
    /// });
    /// let root = ui.mount_tree(Element::new("picker", picker))?;
    /// ui.flush(Duration::ZERO)?;
    /// let hue = ui.find_descendant(&root, "hue")?.expect("the hue strip");
    /// ui.focus(&hue)?;
    /// ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    /// ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    /// assert_eq!(*undo.borrow(), [color.get()?]);
    /// // The application's own write is not reported.
    /// color.set(Color::from_hex("#000000")?)?;
    /// ui.flush(Duration::ZERO)?;
    /// assert_eq!(undo.borrow().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn on_change(
        self,
        handler: impl FnMut(&mut crate::EventContext, Color) -> Result<(), RuntimeError> + 'static,
    ) -> Self {
        *self.model.changed.handler.borrow_mut() = Some(Box::new(handler));
        self
    }
    /// Puts the picker in a [`Popover`] opened from a [`ColorSwatch`] of the
    /// bound color.
    ///
    /// The swatch is the popover's anchor, keyed `swatch`, with one Tab stop:
    /// activating it opens the popover, which focuses the first part. The
    /// popover closes on the usual [`Popover`] dismissals, such as Escape or
    /// a press outside, which also cancel a running gesture; edits already
    /// committed stay. The picker is the content, keyed `picker`. The open
    /// state is the popover's own. Compatible same-key redescription retains the
    /// entire composition: open state, anchor, picker binding/options, focus and
    /// active draft or gesture. Use a new key to change its binding/options or
    /// switch to an ordinary externally controlled [`Popover::new`] surface.
    ///
    /// ```
    /// use pert_ui_runtime::{Color, ColorPicker, Column, Element, KeyAction, KeyCode, Runtime, Viewport};
    /// use std::time::Duration;
    ///
    /// let mut ui = Runtime::new();
    /// ui.set_viewport(Viewport::new(400.0, 500.0, 1.0)?)?;
    /// let color = ui.signal(Color::from_hex("#2090a0")?);
    /// let picker = ColorPicker::new(color).name("Track color").into_popover();
    /// let root = ui.mount_tree(Element::new(
    ///     "page",
    ///     Column::new([Element::new("color", picker)]),
    /// ))?;
    /// ui.flush(Duration::ZERO)?;
    /// let swatch = ui.find_descendant(&root, "swatch")?.expect("the anchor");
    /// ui.focus(&swatch)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    /// ui.flush(Duration::ZERO)?;
    /// let area = ui.find_descendant(&root, "area")?.expect("the picker's area");
    /// assert_eq!(ui.focused(), Some(area));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn into_popover(self) -> Popover {
        let color = self.model.color.clone();
        let open = color.sibling(false);
        let opener = open.clone();
        // Without an application name the swatch names itself from the
        // string table, so it follows a table installed later; a bound or
        // role name reaches the swatch as its source.
        let name = self
            .name
            .clone()
            .unwrap_or_else(|| crate::StringRole::ColorName.into());
        // The anchor follows the picker's permission, bound or fixed.
        let swatch = ColorSwatch::bound(color.read_only())
            .name(name)
            .on_activate(move |_| opener.set(true))
            .enabled(self.model.enablement.borrow().clone());
        let changed = self.model.changed.clone();
        Popover::owned(
            open,
            Element::new("swatch", swatch),
            Element::new("picker", self),
        )
        .carrying(changed)
    }
    fn describe(&self) -> Result<Vec<Element>, RuntimeError> {
        let model = &self.model;
        let mut children = vec![
            Element::new("area", ColorPart::new(model.clone(), Part::Area)),
            Element::new("hue", ColorPart::new(model.clone(), Part::Hue)),
        ];
        if model.alpha.get() {
            children.push(Element::new(
                "alpha",
                ColorPart::new(model.clone(), Part::Alpha),
            ));
        }
        if model.text_entry.get() {
            children.push(Element::new("entry", entry_row(model)?));
        }
        Ok(children)
    }
}

/// The hex and channel fields over the model's current entry bindings.
fn entry_row(model: &Rc<Model>) -> Result<Column, RuntimeError> {
    let entry = model.entry.borrow();
    let alpha = model.alpha.get();
    let enabled = model.enabled.get();
    let hex_model = model.clone();
    let strings = model.strings.borrow().clone();
    let no_alpha = strings.get(crate::StringRole::ColorNoAlpha).to_owned();
    let mut hex = TextField::new(entry.hex.clone())
        .normalize(move |text| {
            let color = Color::from_hex(text.trim()).map_err(|error| {
                InputRejection::new(strings.format(crate::UiMessage::NotHexColor {
                    reason: &error.to_string(),
                }))
            })?;
            if !alpha && color.alpha() < 1.0 {
                return Err(InputRejection::new(no_alpha.clone()));
            }
            Ok(color.to_hex())
        })
        .on_save(move |cx, text| match Color::from_hex(text) {
            Ok(color) => {
                let color = if alpha {
                    color
                } else {
                    color
                        .with_alpha(hex_model.color.get()?.alpha())
                        .map_err(|_| GeometryError::NonFinite)?
                };
                let changed = hex_model.commit_color(color)?;
                hex_model.report(cx, changed)
            }
            // The normalizer admitted only canonical hex text.
            Err(_) => Ok(()),
        });
    if !enabled {
        hex = hex.disabled();
    }
    use crate::StringRole;
    let names = [
        ("r", StringRole::ColorRed),
        ("g", StringRole::ColorGreen),
        ("b", StringRole::ColorBlue),
        ("a", StringRole::ColorAlphaChannel),
    ];
    let count = if alpha { 4 } else { 3 };
    let mut channels = Vec::with_capacity(count);
    for (index, (key, role)) in names.into_iter().enumerate().take(count) {
        let channel_model = model.clone();
        let mut input = IntegerInput::new(entry.channels[index].clone())
            .range(0, 255)?
            .on_save(move |cx, saved| {
                let original = channel_model.color.get()?;
                let mut bytes = original.to_srgb8();
                bytes[index] = u8::try_from(saved).unwrap_or(u8::MAX);
                let [red, green, blue, alpha] = bytes;
                let edited = Color::from_srgb8(red, green, blue, alpha);
                let edited = if channel_model.alpha.get() {
                    edited
                } else {
                    edited
                        .with_alpha(original.alpha())
                        .map_err(|_| GeometryError::NonFinite)?
                };
                let changed = channel_model.commit_color(edited)?;
                channel_model.report(cx, changed)
            });
        if !enabled {
            input = input.disabled();
        }
        channels.push(Element::new(
            key,
            Field::new(Element::new("input", input)).label(model.text(role)),
        ));
    }
    Ok(Column::new([
        Element::new(
            "hex",
            Field::new(Element::new("input", hex)).label(model.text(StringRole::ColorHex)),
        ),
        Element::new("channels", Row::new(channels)),
    ]))
}

impl Component for ColorPicker {
    fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
        // The retained picker keeps its binding and options; the handler and
        // the permission follow the description.
        self.model.changed.replace_from(&next.model.changed);
        let next = next.model.enablement.borrow().clone();
        let mut mine = self.model.enablement.borrow_mut();
        if mine.same_as(&next) {
            return None;
        }
        if let Some(enabled) = next.fixed() {
            self.model.enabled.set(enabled);
        }
        *mine = next;
        // The parts and fields are described again with the new permission.
        self.described = false;
        Some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let permission = self.model.enablement.borrow().clone();
        let enabled = permission.read_paint(cx)?;
        if self.model.enabled.replace(enabled) != enabled {
            // The text fields take the new state from a new description.
            self.described = false;
        }
        // A bound or role name is not drawn; reading it only repaints.
        if let Some(name) = self.name.as_ref().filter(|name| name.is_bound()) {
            let text = name.read_paint(cx)?.plain_text();
            self.shown_name = (!text.is_empty()).then_some(text);
        }
        let strings = cx.strings()?;
        if *self.model.strings.borrow() != strings {
            // Labels and messages come from the table: describe again.
            *self.model.strings.borrow_mut() = strings;
            self.described = false;
        }
        let color = cx.read(&self.model.color)?;
        if self.model.text_entry.get() && self.model.synced.get() != Some(color) {
            // A value the picker did not write: the fields show it from fresh
            // bindings, since writes are refused here. Re-described with
            // them, the retained fields rebind and drop any draft.
            *self.model.entry.borrow_mut() = Entry::new(&self.model.color, color);
            self.model.synced.set(Some(color));
            self.described = false;
        }
        if !self.described {
            cx.request_children(self.describe()?);
            self.described = true;
        }
        Ok(())
    }
    fn measure(
        &self,
        cx: &MeasureContext<'_>,
        available: Constraints,
    ) -> Result<Size, RuntimeError> {
        let gap = cx.metric(MetricRole::ItemGap);
        let mut width = cx.metric(MetricRole::ColorAreaSize);
        let mut height = 0.0_f32;
        for index in 0..cx.len() {
            let size = cx.measure(index, Constraints::new(available.width(), None)?)?;
            width = width.max(size.width());
            height += size.height() + if index > 0 { gap } else { 0.0 };
        }
        if let Some(limit) = available.width() {
            width = width.min(limit);
        }
        Ok(Size::new(width, height)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let gap = cx.measurement().metric(MetricRole::ItemGap);
        let mut top = 0.0_f32;
        for index in 0..cx.len() {
            let child = cx.measure(index, Constraints::new(Some(size.width()), None)?)?;
            cx.place(index, rect(0.0, top, size.width(), child.height())?)?;
            top += child.height() + gap;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        let color = self.model.color.get().ok();
        let hex = color.map(Color::to_hex);
        if let Some(hex) = &hex {
            details
                .properties
                .insert("color".into(), InspectValue::Text(hex.clone()));
        }
        details.properties.insert(
            "alpha_strip".into(),
            InspectValue::Bool(self.model.alpha.get()),
        );
        let mut semantics = Semantics::new(SemanticRole::Group)
            .name(
                self.shown_name
                    .clone()
                    .unwrap_or_else(|| self.model.text(crate::StringRole::ColorPickerName)),
            )
            .disabled(!self.model.enabled.get());
        if let Some(hex) = hex {
            semantics = semantics.value(SemanticValue::Text(hex));
        }
        details.semantics = Some(semantics);
        details
    }
}

/// The gradient of fully saturated hues from 0° to 360°.
fn hue_image() -> Result<RasterImage, DrawError> {
    let mut pixels = Vec::with_capacity(STRIP_PIXELS as usize * 4);
    for x in 0..STRIP_PIXELS {
        let hue = 360.0 * x as f32 / (STRIP_PIXELS - 1) as f32;
        let [red, green, blue] = Hsva {
            hue,
            saturation: 1.0,
            value: 1.0,
            alpha: 1.0,
        }
        .srgb8();
        pixels.extend_from_slice(&[red, green, blue, 255]);
    }
    RasterImage::from_srgba8(STRIP_PIXELS, 1, pixels)
}

/// Saturation rising to the right and value rising upward, for one hue.
fn area_image(hue: f32) -> Result<RasterImage, DrawError> {
    let last = (AREA_PIXELS - 1) as f32;
    let mut pixels = Vec::with_capacity((AREA_PIXELS * AREA_PIXELS) as usize * 4);
    for y in 0..AREA_PIXELS {
        for x in 0..AREA_PIXELS {
            let [red, green, blue] = Hsva {
                hue,
                saturation: x as f32 / last,
                value: 1.0 - y as f32 / last,
                alpha: 1.0,
            }
            .srgb8();
            pixels.extend_from_slice(&[red, green, blue, 255]);
        }
    }
    RasterImage::from_srgba8(AREA_PIXELS, AREA_PIXELS, pixels)
}

/// One color from transparent on the left to opaque on the right.
fn alpha_image(rgb: [u8; 3]) -> Result<RasterImage, DrawError> {
    let mut pixels = Vec::with_capacity(STRIP_PIXELS as usize * 4);
    for x in 0..STRIP_PIXELS {
        // `x` is below 256, so the alpha is a byte.
        let alpha = (x * 255 / (STRIP_PIXELS - 1)) as u8;
        pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], alpha]);
    }
    RasterImage::from_srgba8(STRIP_PIXELS, 1, pixels)
}

/// The running pointer gesture of one part.
#[derive(Clone, Copy)]
struct Gesture {
    /// The bound color when the gesture began; any other value read during
    /// the gesture is an application write that ends it.
    before: Color,
}

/// The area or one strip of a [`ColorPicker`].
struct ColorPart {
    model: Rc<Model>,
    part: Part,
    color: Color,
    hsva: Hsva,
    gesture: Option<Gesture>,
    /// The permission this part last read and its value.
    permission: Permission,
    allowed: bool,
    focused: bool,
    image: Option<RasterImage>,
    /// What the generated image was built for: the hue of the area, the
    /// channels of the alpha strip.
    image_key: Option<[u32; 3]>,
}
impl ColorPart {
    fn new(model: Rc<Model>, part: Part) -> Self {
        Self {
            model: model.clone(),
            part,
            color: Color::WHITE,
            hsva: DEFAULT_HSVA,
            gesture: None,
            permission: model.enablement.borrow().clone(),
            allowed: model.enabled.get(),
            focused: false,
            image: None,
            image_key: None,
        }
    }
    fn enabled(&self) -> bool {
        self.allowed
    }
    /// The state with this part's coordinates taken from a local position.
    fn at(&self, cx: &EventContext, base: Hsva, position: Point) -> Option<Hsva> {
        let local = cx.to_local(position)?;
        let size = cx.size()?;
        let x = (local.x() / size.width().max(1.0)).clamp(0.0, 1.0);
        let y = (local.y() / size.height().max(1.0)).clamp(0.0, 1.0);
        Some(match self.part {
            Part::Area => Hsva {
                saturation: x,
                value: 1.0 - y,
                ..base
            },
            Part::Hue => Hsva {
                hue: 360.0 * x,
                ..base
            },
            Part::Alpha => Hsva { alpha: x, ..base },
        })
    }
    fn preview(&mut self, hsva: Hsva) -> Result<(), RuntimeError> {
        let Some(gesture) = self.gesture else {
            return Ok(());
        };
        *self.model.preview.borrow_mut() = Some((gesture.before, hsva));
        self.hsva = hsva;
        self.model.notify()
    }
    /// Ends a running gesture without writing the binding.
    fn abandon(&mut self) {
        if self.gesture.take().is_some() {
            self.model.preview.borrow_mut().take();
        }
    }
    /// One keyboard edit of this part.
    fn stepped(&self, base: Hsva, code: KeyCode, shift: bool) -> Option<Hsva> {
        let step = if shift { FINE_STEP } else { STEP };
        let (delta, edge) = match code {
            KeyCode::ArrowLeft | KeyCode::ArrowDown => (-step, None),
            KeyCode::ArrowRight | KeyCode::ArrowUp => (step, None),
            KeyCode::PageDown => (-PAGE_STEP, None),
            KeyCode::PageUp => (PAGE_STEP, None),
            KeyCode::Home => (0.0, Some(0.0)),
            KeyCode::End => (0.0, Some(1.0)),
            _ => return None,
        };
        let moved = |current: f32| edge.unwrap_or(current + delta).clamp(0.0, 1.0);
        Some(match self.part {
            Part::Area => match code {
                KeyCode::ArrowLeft | KeyCode::ArrowRight => Hsva {
                    saturation: moved(base.saturation),
                    ..base
                },
                KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::PageUp | KeyCode::PageDown => {
                    Hsva {
                        value: moved(base.value),
                        ..base
                    }
                }
                _ => return None,
            },
            Part::Hue => {
                // Arrows step whole degrees; pages and the ends use the
                // shared fraction of the range.
                let degrees = match code {
                    KeyCode::ArrowLeft | KeyCode::ArrowDown => {
                        -(if shift { HUE_FINE_STEP } else { HUE_STEP })
                    }
                    KeyCode::ArrowRight | KeyCode::ArrowUp => {
                        if shift {
                            HUE_FINE_STEP
                        } else {
                            HUE_STEP
                        }
                    }
                    _ => delta * 360.0,
                };
                let hue = match edge {
                    Some(edge) => edge * 360.0,
                    None => (base.hue + degrees).clamp(0.0, 360.0),
                };
                Hsva { hue, ..base }
            }
            Part::Alpha => Hsva {
                alpha: moved(base.alpha),
                ..base
            },
        })
    }
    fn refresh_image(&mut self) {
        let key = match self.part {
            Part::Area => [self.hsva.hue.to_bits(), 0, 0],
            Part::Alpha => self.hsva.srgb8().map(u32::from),
            Part::Hue => return,
        };
        if self.image_key != Some(key) {
            // The images have constant, nonzero dimensions well within the
            // raster limit, which the constructor cannot refuse; were it to,
            // the part would draw its frame and thumb without a gradient.
            self.image = match self.part {
                Part::Area => area_image(self.hsva.hue).ok(),
                _ => alpha_image(self.hsva.srgb8()).ok(),
            };
            self.image_key = Some(key);
        }
    }
    fn label(&self) -> String {
        self.model.text(match self.part {
            Part::Area => crate::StringRole::ColorArea,
            Part::Hue => crate::StringRole::ColorHue,
            Part::Alpha => crate::StringRole::ColorAlpha,
        })
    }
}
impl Component for ColorPart {
    fn reconcile(&mut self, _next: Self) -> Option<Invalidation> {
        // A part shares the picker's model; it reads a replaced permission
        // again.
        (!self.permission.same_as(&self.model.enablement.borrow())).then_some(Invalidation::Paint)
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        // Each part follows the permission itself, so a revocation ends its
        // gesture without waiting for the picker.
        self.permission = self.model.enablement.borrow().clone();
        self.allowed = self.permission.read_paint(cx)?;
        if !self.allowed && self.gesture.is_some() {
            self.abandon();
            cx.cancel_input();
        }
        let color = cx.read_paint(&self.model.color)?;
        let committed = cx.read_paint(&self.model.committed)?;
        cx.read_paint(&self.model.tick)?;
        if let Some(gesture) = self.gesture
            && gesture.before != color
        {
            // The application's write wins over the gesture.
            self.abandon();
            cx.cancel_input();
        }
        self.color = color;
        self.hsva = self.model.presented(color, committed);
        self.focused = cx.focused();
        self.refresh_image();
        Ok(())
    }
    fn focusable(&self) -> bool {
        self.enabled()
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let width = cx.metric(MetricRole::ColorAreaSize);
        let height = match self.part {
            Part::Area => width,
            Part::Hue | Part::Alpha => cx.metric(MetricRole::ColorStripThickness),
        };
        Ok(Size::new(width, height)?)
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
        if !self.enabled() {
            return Ok(InputOutcome::Ignored);
        }
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) if self.gesture.is_none() => {
                if event.is_outside() {
                    return Ok(InputOutcome::Ignored);
                }
                let base = self.model.current()?;
                let Some(next) = self.at(cx, base, event.position()) else {
                    return Ok(InputOutcome::Ignored);
                };
                cx.capture_pointer()?;
                self.gesture = Some(Gesture {
                    before: self.model.color.get()?,
                });
                self.preview(next)?;
            }
            PointerKind::Move if self.gesture.is_some() => {
                if let Some(next) = self.at(cx, self.hsva, event.position()) {
                    self.preview(next)?;
                }
            }
            PointerKind::Release(PointerButton::Primary) if self.gesture.is_some() => {
                let next = self
                    .at(cx, self.hsva, event.position())
                    .unwrap_or(self.hsva);
                self.gesture = None;
                cx.release_pointer();
                let changed = self.model.commit(next)?;
                cx.invalidate(Invalidation::Paint);
                self.model.report(cx, changed)?;
                return Ok(InputOutcome::Handled);
            }
            PointerKind::Press(_) | PointerKind::Release(_) | PointerKind::Move
                if self.gesture.is_some() => {}
            _ => return Ok(InputOutcome::Ignored),
        }
        cx.invalidate(Invalidation::Paint);
        Ok(InputOutcome::Handled)
    }
    fn keyboard_event(
        &mut self,
        cx: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let control = keyboard.control_key(event);
        if !self.enabled() {
            return Ok(InputOutcome::Ignored);
        }
        if self.gesture.is_some() {
            if control == KeyCode::Escape && event.transition == KeyTransition::Pressed {
                self.abandon();
                cx.release_pointer();
                self.hsva = self.model.current()?;
                self.model.notify()?;
                cx.invalidate(Invalidation::Paint);
            }
            // A pointer gesture owns the part until it ends.
            return Ok(InputOutcome::Handled);
        }
        if !matches!(
            event.transition,
            KeyTransition::Pressed | KeyTransition::Repeated
        ) {
            return Ok(if self.stepped(self.hsva, control, false).is_some() {
                InputOutcome::Handled
            } else {
                InputOutcome::Ignored
            });
        }
        // Control, Alt and Super chords are left to command bindings.
        if !crate::activation::steps_with(keyboard.chord_modifiers(event)) {
            return Ok(InputOutcome::Ignored);
        }
        let base = self.model.current()?;
        let Some(next) = self.stepped(base, control, keyboard.modifiers().shift) else {
            return Ok(InputOutcome::Ignored);
        };
        if next != base {
            let changed = self.model.commit(next)?;
            cx.invalidate(Invalidation::Paint);
            self.model.report(cx, changed)?;
        }
        Ok(InputOutcome::Handled)
    }
    fn cancel_input(&mut self, cx: &mut EventContext) {
        let previewing = self.gesture.is_some();
        self.input_reset();
        if previewing {
            // Where writes are refused (hiding, removal) the notification
            // fails; the parts then read the cleared preview when they are
            // shown again, which updates them.
            let _ = self.model.notify();
        }
        cx.invalidate(Invalidation::Paint);
    }
    fn input_reset(&mut self) {
        self.abandon();
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = painter.size() else {
            return Ok(());
        };
        let bounds = Rect::from_size(size);
        let [_, _, width, height] = bounds.values();
        if self.part == Part::Alpha {
            paint_checkerboard(painter, bounds)?;
        }
        let image = match self.part {
            Part::Hue => self.model.hue_image.as_ref(),
            Part::Area | Part::Alpha => self.image.as_ref(),
        };
        if let Some(image) = image {
            painter.draw_image(image, bounds, ImageSampling::Linear);
        }
        let ring = painter.metric(MetricRole::FocusWidth);
        let edge = painter.metric(MetricRole::BorderWidth);
        let light = Color::WHITE;
        let dark = Color::rgba(0.0, 0.0, 0.0, 1.0)?;
        match self.part {
            Part::Area => {
                let radius = painter.metric(MetricRole::ColorStripThickness) / 2.0;
                let x = self.hsva.saturation * width;
                let y = (1.0 - self.hsva.value) * height;
                let thumb = Rect::new(x - radius, y - radius, radius * 2.0, radius * 2.0)?;
                painter.with_clip(bounds, |painter| -> Result<(), DrawError> {
                    painter.stroke_ellipse(
                        thumb,
                        StrokeStyle::new(Length::new(ring + edge * 2.0)?),
                        dark,
                    )?;
                    painter.stroke_ellipse(thumb, StrokeStyle::new(Length::new(ring)?), light)
                })?;
            }
            Part::Hue | Part::Alpha => {
                let fraction = match self.part {
                    Part::Hue => self.hsva.hue / 360.0,
                    _ => self.hsva.alpha,
                };
                let thumb_width = ring * 2.0 + edge * 2.0;
                let x = (fraction * width - thumb_width / 2.0)
                    .clamp(0.0, (width - thumb_width).max(0.0));
                let thumb = Rect::new(x, 0.0, thumb_width.min(width), height)?;
                painter.stroke_shape(thumb, Shape::Rectangle, Length::new(edge)?, dark)?;
                let [left, top, across, tall] = thumb.values();
                if across > edge * 2.0 && tall > edge * 2.0 {
                    let inner = Rect::new(
                        left + edge,
                        top + edge,
                        across - edge * 2.0,
                        tall - edge * 2.0,
                    )?;
                    painter.stroke_shape(inner, Shape::Rectangle, Length::new(ring)?, light)?;
                }
            }
        }
        if self.focused && self.enabled() {
            return painter.focus_ring(bounds, Shape::Rectangle);
        }
        let role = if self.enabled() {
            ColorRole::Outline
        } else {
            ColorRole::Muted
        };
        painter.stroke_shape(
            bounds,
            Shape::Rectangle,
            Length::new(painter.metric(MetricRole::BorderWidth))?,
            painter.color(role),
        )
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        for (key, value) in [
            ("hue", self.hsva.hue),
            ("saturation", self.hsva.saturation),
            ("value", self.hsva.value),
            ("alpha", self.hsva.alpha),
        ] {
            details
                .properties
                .insert(key.into(), InspectValue::Number(f64::from(value)));
        }
        details.properties.insert(
            "dragging".into(),
            InspectValue::Bool(self.gesture.is_some()),
        );
        details.validation.interactive = self.enabled();
        let value = match self.part {
            Part::Area => SemanticValue::Text(self.model.strings.borrow().format(
                crate::UiMessage::ColorAreaValue {
                    saturation: percent(self.hsva.saturation),
                    value: percent(self.hsva.value),
                },
            )),
            Part::Hue => SemanticValue::Number {
                value: f64::from(self.hsva.hue),
                minimum: Some(0.0),
                maximum: Some(360.0),
            },
            Part::Alpha => SemanticValue::Number {
                value: f64::from(self.hsva.alpha),
                minimum: Some(0.0),
                maximum: Some(1.0),
            },
        };
        let mut semantics = Semantics::new(SemanticRole::Slider)
            .name(self.label())
            .value(value)
            .disabled(!self.enabled());
        if self.enabled() {
            semantics = semantics.enabled_actions([
                SemanticAction::SetValue,
                SemanticAction::Increment,
                SemanticAction::Decrement,
            ]);
        }
        details.semantics = Some(semantics);
        details
    }
}

/// A unit fraction in whole percent, clamped to 0–100.
fn percent(fraction: f32) -> u8 {
    // The clamp keeps the rounded value inside u8.
    (fraction.clamp(0.0, 1.0) * 100.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_round_trips_every_primary_and_keeps_open_components() {
        for hex in [
            "#ff0000", "#00ff00", "#0000ff", "#ffff00", "#00ffff", "#ff00ff", "#808080",
        ] {
            let color = Color::from_hex(hex).unwrap();
            let hsva = Hsva::from_color(color, DEFAULT_HSVA);
            assert_eq!(hsva.color().unwrap().to_hex(), hex);
        }
        let hint = Hsva {
            hue: 200.0,
            saturation: 0.7,
            value: 0.5,
            alpha: 1.0,
        };
        let black = Hsva::from_color(Color::from_hex("#000000").unwrap(), hint);
        assert_eq!(
            (black.hue, black.saturation, black.value),
            (200.0, 0.7, 0.0)
        );
        let gray = Hsva::from_color(Color::from_hex("#808080").unwrap(), hint);
        assert_eq!((gray.hue, gray.saturation), (200.0, 0.0));
    }

    #[test]
    fn images_have_their_documented_extents() {
        assert_eq!(hue_image().unwrap().width(), STRIP_PIXELS);
        assert_eq!(area_image(120.0).unwrap().height(), AREA_PIXELS);
        let alpha = alpha_image([255, 0, 0]).unwrap();
        assert_eq!(alpha.pixels()[3], 0);
        assert_eq!(alpha.pixels()[alpha.pixels().len() - 1], 255);
    }
}
