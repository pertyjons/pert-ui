//! User-controlled presentation scale, independent of theme and native DPI.
use super::*;

/// Validated application zoom factor in `0.75..=2.0` (75–200%).
/// Default is 1.0. This affects all UI geometry and text; editor/world zoom is
/// independent. Values are preferences, not physical DPI or persisted identities.
///
/// Text is rasterized at no more than
/// [`TextError::MAX_FONT_PIXELS`](crate::TextError::MAX_FONT_PIXELS) physical
/// pixels per font size, the product of logical size, DPI scale and zoom. Larger
/// text is prepared at that size and stretched, so no zoom in range makes a
/// theme's text sizes fail preparation. The raster-side and coverage limits of
/// [`TextLimit`](crate::TextLimit) are not lowered that way: they apply to text
/// laid out at DPI scale times zoom, so a long unwrapped line that fits at 100%
/// can fail at a higher zoom.
///
/// # Serialization
///
/// With the `serde` feature a zoom is its factor, validated through
/// [`UiZoom::new`] when read.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct UiZoom(f64);
impl Default for UiZoom {
    fn default() -> Self {
        Self(1.0)
    }
}
impl UiZoom {
    /// The smallest zoom, 75%.
    pub const MIN: Self = Self(0.75);
    /// The largest zoom, 200%.
    pub const MAX: Self = Self(2.0);
    /// Validates a finite factor in `0.75..=2.0`; rejects invalid input without clamping.
    ///
    /// # Errors
    /// [`GeometryError::NonFinite`](crate::GeometryError::NonFinite) for NaN or an
    /// infinity and [`GeometryError::ZoomOutOfRange`](crate::GeometryError::ZoomOutOfRange)
    /// outside [`Self::MIN`]`..=`[`Self::MAX`].
    pub fn new(factor: f64) -> Result<Self, crate::GeometryError> {
        if !factor.is_finite() {
            return Err(crate::GeometryError::NonFinite);
        }
        if !(Self::MIN.0..=Self::MAX.0).contains(&factor) {
            return Err(crate::GeometryError::ZoomOutOfRange);
        }
        Ok(Self(factor))
    }
    /// Returns the user factor, before multiplying by the current screen DPI scale.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
    /// Next larger preset among 75, 90, 100, 110, 125, 150, 175, 200 percent.
    /// At the upper limit remains 200%; custom values advance to the next preset.
    #[must_use]
    pub fn increased(self) -> Self {
        Self(
            [0.75, 0.90, 1.0, 1.10, 1.25, 1.50, 1.75, 2.0]
                .into_iter()
                .find(|v| *v > self.0)
                .unwrap_or(2.0),
        )
    }
    /// Next smaller preset; at the lower limit remains 75%.
    #[must_use]
    pub fn decreased(self) -> Self {
        Self(
            [2.0, 1.75, 1.50, 1.25, 1.10, 1.0, 0.90, 0.75]
                .into_iter()
                .find(|v| *v < self.0)
                .unwrap_or(0.75),
        )
    }
}
impl TryFrom<f64> for UiZoom {
    type Error = crate::GeometryError;
    /// Validates like [`UiZoom::new`].
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<UiZoom> for f64 {
    fn from(value: UiZoom) -> Self {
        value.0
    }
}

// The serde representation; the public type documents it.
#[cfg(feature = "serde")]
mod serde_impls {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::UiZoom;

    impl Serialize for UiZoom {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_f64(self.0)
        }
    }

    impl<'de> Deserialize<'de> for UiZoom {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Self::new(f64::deserialize(deserializer)?).map_err(D::Error::custom)
        }
    }
}

/// Whether components may run decorative or continuous animation.
///
/// This is a user preference that a host typically takes from its operating
/// system or its own settings; the library neither detects nor persists it, and
/// it is not part of the workspace document. It governs presentation a component
/// chooses, such as an indeterminate sweep. It does not throttle
/// [`UpdateContext::animate`], live data, caret blinking or timed dismissal,
/// which carry information rather than decoration.
///
/// Component authors read it with [`UpdateContext::motion`] and, when it is
/// [`Reduced`](Self::Reduced), present the same state without movement and
/// without requesting frames.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MotionPreference {
    /// Animation is welcome. The default.
    #[default]
    Full,
    /// Present state without decorative movement. Built-in activity indicators
    /// become still and schedule nothing.
    Reduced,
}

/// Text coverage strategy for this runtime. The default keeps the retained
/// line masks. The phase-aware mode is experimental and may use more UI-thread
/// raster work and cache memory during scrolling.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TextRenderingMode {
    /// Reuse the prepared coverage mask for each text run.
    #[default]
    LineMask,
    /// Rasterize glyphs for their final quarter-pixel physical phase, then
    /// composite adjacent glyphs of one brush into a run mask before drawing.
    /// A phase mask that exceeds raster limits uses the already prepared line
    /// mask for that paint.
    PhaseLine,
}

/// One observable presentation setting: a weakly held signal created on the
/// first request, so an unobserved setting publishes nothing.
pub(crate) struct Observed<T>(RefCell<crate::signal::WeakSignal<T>>);

impl<T> Default for Observed<T> {
    fn default() -> Self {
        Self(RefCell::new(crate::signal::WeakSignal::default()))
    }
}

impl<T: Copy + PartialEq + 'static> Observed<T> {
    /// Returns the live signal, or creates one holding `current`.
    pub(crate) fn read(&self, shared: &Rc<Shared>, current: T) -> ReadSignal<T> {
        let mut weak = self.0.borrow_mut();
        let signal = weak.upgrade().unwrap_or_else(|| {
            let signal = Signal::new(shared.clone(), current);
            *weak = signal.downgrade();
            signal
        });
        signal.read_only()
    }
    /// Publishes a changed setting to its observers, if any.
    pub(crate) fn publish(&self, value: T) {
        let signal = self.0.borrow().upgrade();
        if let Some(signal) = signal {
            signal.publish_copy(value);
        }
    }
}

impl<T: Clone + PartialEq + 'static> Observed<T> {
    /// Returns the live signal, or creates one holding a clone of `current`.
    pub(crate) fn read_cloned(&self, shared: &Rc<Shared>, current: &T) -> ReadSignal<T> {
        let mut weak = self.0.borrow_mut();
        let signal = weak.upgrade().unwrap_or_else(|| {
            let signal = Signal::new(shared.clone(), current.clone());
            *weak = signal.downgrade();
            signal
        });
        signal.read_only()
    }
    /// Publishes a changed value to its observers, if any.
    pub(crate) fn publish_cloned(&self, value: T) -> Result<(), RuntimeError> {
        let signal = self.0.borrow().upgrade();
        if let Some(signal) = signal {
            signal.publish(value)?;
        }
        Ok(())
    }
}

/// The observable views of one runtime's presentation settings.
#[derive(Default)]
pub(crate) struct SettingsSignals {
    pub(crate) theme_mode: Observed<crate::ThemeMode>,
    pub(crate) density: Observed<crate::Density>,
    pub(crate) zoom: Observed<UiZoom>,
    pub(crate) motion: Observed<MotionPreference>,
    pub(crate) text_rendering: Observed<TextRenderingMode>,
    pub(crate) debug_overlay: Observed<crate::DebugOverlay>,
    pub(crate) strings: Observed<crate::UiStrings>,
}

/// Shared UI-thread presentation settings for one runtime, obtained from
/// [`Runtime::ui_settings`]. Clones share the preference, with no global state,
/// file persistence or worker/callback-safe endpoint. Hosts own persistence if desired.
/// Setters follow the same lifecycle restrictions as theme and signal writes.
#[derive(Clone)]
#[must_use]
pub struct UiSettings {
    shared: Rc<Shared>,
}
impl UiSettings {
    /// Returns the current text coverage strategy, initially
    /// [`TextRenderingMode::LineMask`]. The choice belongs to this runtime and
    /// is not persisted by the library.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn text_rendering(&self) -> Result<TextRenderingMode, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.text_rendering.get())
    }

    /// Selects a text coverage strategy. Returns false when unchanged. The
    /// next paint refreshes text commands without remeasuring text or changing
    /// caret and selection geometry. The experimental mode may rerasterize when
    /// text changes its fractional physical-pixel position, including during
    /// scrolling.
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{Runtime, TextRenderingMode};
    /// # fn main() -> Result<(), pert_ui::RuntimeError> {
    /// let ui = Runtime::new();
    /// let settings = ui.ui_settings();
    /// assert_eq!(settings.text_rendering()?, TextRenderingMode::LineMask);
    /// assert!(settings.set_text_rendering(TextRenderingMode::PhaseLine)?);
    /// assert!(!settings.set_text_rendering(TextRenderingMode::PhaseLine)?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Rejects writes in update, layout, paint, inspection or delivery, and
    /// after shutdown, leaving the current mode unchanged.
    pub fn set_text_rendering(&self, mode: TextRenderingMode) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        if self.shared.text_rendering.replace(mode) == mode {
            return Ok(false);
        }
        self.shared.damage_scenes();
        self.shared.settings_signals.text_rendering.publish(mode);
        Ok(true)
    }

    /// Observes the runtime's text strategy for settings controls. No polling
    /// or repaint is scheduled while the value is unchanged.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn text_rendering_binding(&self) -> Result<ReadSignal<TextRenderingMode>, RuntimeError> {
        self.shared.readable()?;
        Ok(self
            .shared
            .settings_signals
            .text_rendering
            .read(&self.shared, self.shared.text_rendering.get()))
    }
    /// Current user zoom, independent of DPI.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn zoom(&self) -> Result<UiZoom, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.zoom.get())
    }
    /// Changes zoom without resizing the native window. Returns false for a no-op.
    /// The effective viewport changes immediately; the next flush cancels active
    /// component gestures before layout, retaining physical key state and focus.
    /// Visible components refresh once; hidden components retain pending dirtiness.
    /// No timer or implicit zoom shortcut is installed.
    /// Zooming never makes a font size too large to prepare, but the raster-side
    /// and coverage limits of [`TextLimit`](crate::TextLimit) scale with zoom: text
    /// that fits at the old zoom, such as a long unwrapped label, can fail the next
    /// flush with [`TextError::TooLarge`](crate::TextError::TooLarge) after this
    /// returns `Ok`. See [`UiZoom`].
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` when called from inside that phase; and
    /// [`RuntimeError::Geometry`] when a window's zoomed viewport is not valid geometry.
    /// Either leaves the preference and viewport unchanged.
    pub fn set_zoom(&self, zoom: UiZoom) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        if self.shared.zoom.get() == zoom {
            return Ok(false);
        }
        let windows = self
            .shared
            .windows
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .map(|window| {
                effective(window.native_viewport.get(), zoom).map(|viewport| (window, viewport))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.shared.zoom.set(zoom);
        for (window, viewport) in windows {
            window.change_viewport(viewport);
        }
        self.shared.settings_signals.zoom.publish(zoom);
        Ok(true)
    }
    /// Observes the user zoom: a read-only signal that every change through
    /// [`Self::set_zoom`], including one made by
    /// [`Runtime::restore_workspace`], updates.
    ///
    /// A component reading it in `update` is invalidated when the zoom
    /// changes, like any signal; a menu row can mark the current preset
    /// without polling. Clones and later calls share one signal while any of
    /// them is alive, and the runtime publishes nothing while none is.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn zoom_binding(&self) -> Result<ReadSignal<UiZoom>, RuntimeError> {
        self.shared.readable()?;
        Ok(self
            .shared
            .settings_signals
            .zoom
            .read(&self.shared, self.shared.zoom.get()))
    }
    /// Current motion preference, [`MotionPreference::Full`] unless set.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn motion(&self) -> Result<MotionPreference, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.motion.get())
    }
    /// Changes the motion preference. Returns false for a no-op.
    ///
    /// Only components whose last update read [`UpdateContext::motion`] are
    /// invalidated, for paint; hidden ones keep the dirtiness until shown. An
    /// animation that a component stops requesting leaves no deadline behind,
    /// and one it resumes starts from the runtime's clock. No timer is installed.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` from inside that phase.
    pub fn set_motion(&self, motion: MotionPreference) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        if self.shared.motion.replace(motion) == motion {
            return Ok(false);
        }
        let mut subscribers = self.shared.motion_subscribers.borrow_mut();
        // A component whose last update stopped reading the preference leaves
        // the list here; reading it again later lists it anew.
        subscribers.retain(|node| {
            node.upgrade().is_some_and(|node| {
                let reads = node.motion_reader.get();
                node.motion_listed.set(reads);
                reads
            })
        });
        for node in subscribers.iter().filter_map(Weak::upgrade) {
            node.invalidate(Invalidation::Paint);
        }
        drop(subscribers);
        self.shared.settings_signals.motion.publish(motion);
        Ok(true)
    }
    /// Observes the motion preference as a read-only signal that
    /// [`Self::set_motion`] updates, for application chrome such as a settings
    /// row. Components that adapt their own animation read
    /// [`UpdateContext::motion`] instead. Sharing and cost follow
    /// [`Self::zoom_binding`].
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn motion_binding(&self) -> Result<ReadSignal<MotionPreference>, RuntimeError> {
        self.shared.readable()?;
        Ok(self
            .shared
            .settings_signals
            .motion
            .read(&self.shared, self.shared.motion.get()))
    }
    /// The installed table of library strings, [`UiStrings::english`](crate::UiStrings::english)
    /// unless replaced.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn strings(&self) -> Result<crate::UiStrings, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.strings.borrow().clone())
    }
    /// Installs a table of library strings. Returns false when it equals the
    /// installed one.
    ///
    /// Built-in components read the table in their update through
    /// [`UpdateContext::strings`], so exactly the components that show one of
    /// its strings are invalidated for layout; hidden ones keep the dirtiness
    /// until shown. Nothing polls, and the table is not part of the workspace
    /// document. Text an application gave a component itself, such as a
    /// dialog's cancel label, is not replaced.
    ///
    /// # Errors
    /// Rejects writes during update, layout, paint, inspection or delivery, or
    /// after shutdown, leaving the installed table unchanged.
    pub fn set_strings(&self, strings: crate::UiStrings) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        if *self.shared.strings.borrow() == strings {
            return Ok(false);
        }
        *self.shared.strings.borrow_mut() = strings.clone();
        self.shared
            .settings_signals
            .strings
            .publish_cloned(strings)?;
        Ok(true)
    }
    /// Observes the table of library strings as a read-only signal that
    /// [`Self::set_strings`] updates, for application chrome such as a language
    /// menu. Components read [`UpdateContext::strings`] instead. Sharing and
    /// cost follow [`Self::zoom_binding`].
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn strings_binding(&self) -> Result<ReadSignal<crate::UiStrings>, RuntimeError> {
        self.shared.readable()?;
        let current = self.shared.strings.borrow().clone();
        Ok(self
            .shared
            .settings_signals
            .strings
            .read_cloned(&self.shared, &current))
    }
    /// Current development overlay.
    ///
    /// # Errors
    /// Returns [`RuntimeError::Closed`] after shutdown.
    pub fn debug_overlay(&self) -> Result<crate::DebugOverlay, RuntimeError> {
        self.shared.readable()?;
        Ok(self.shared.overlay.get())
    }
    /// Selects the development overlay from an event callback, with the contract
    /// of [`Runtime::set_debug_overlay`]. Returns false for a no-op.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown, and `WriteDuringUpdate`,
    /// `WriteDuringLayout`, `WriteDuringPaint`, `WriteDuringInspection` or
    /// `WriteDuringDelivery` from inside that phase.
    pub fn set_debug_overlay(&self, overlay: crate::DebugOverlay) -> Result<bool, RuntimeError> {
        self.shared.writable()?;
        if self.shared.overlay.replace(overlay) == overlay {
            return Ok(false);
        }
        self.shared.damage_scenes();
        self.shared.settings_signals.debug_overlay.publish(overlay);
        Ok(true)
    }
    /// Observes the development overlay as a read-only signal that this
    /// handle's [`Self::set_debug_overlay`] and
    /// [`Runtime::set_debug_overlay`] update. Sharing and cost follow
    /// [`Self::zoom_binding`].
    ///
    /// # Errors
    /// Returns `Closed` after shutdown.
    pub fn debug_overlay_binding(&self) -> Result<ReadSignal<crate::DebugOverlay>, RuntimeError> {
        self.shared.readable()?;
        Ok(self
            .shared
            .settings_signals
            .debug_overlay
            .read(&self.shared, self.shared.overlay.get()))
    }
}
pub(super) fn effective(
    native: crate::Viewport,
    zoom: UiZoom,
) -> Result<crate::Viewport, RuntimeError> {
    crate::Viewport::new(
        (f64::from(native.width()) / zoom.0) as f32,
        (f64::from(native.height()) / zoom.0) as f32,
        native.scale() * zoom.0,
    )
    .map(|viewport| viewport.with_physical(native.physical()))
    .map_err(RuntimeError::from)
}
impl native_windows::WindowShared {
    pub(super) fn change_viewport(&self, viewport: crate::Viewport) {
        let previous = self.viewport.replace(viewport);
        if previous == viewport {
            return;
        }
        if self.geometry_change.get().is_none() {
            self.geometry_change.set(Some(previous.scale()));
        }
        self.scene_dirty.set(true);
        self.layout_pending.set(true);
    }
}
impl Runtime {
    /// Shared presentation settings. Default zoom is 100%, independent of theme/DPI.
    pub fn ui_settings(&self) -> UiSettings {
        UiSettings {
            shared: self.shared.clone(),
        }
    }
    /// Effective logical UI extent and physical pixels per logical UI pixel,
    /// including user zoom. Custom hosts use its scale for pointer/pixel-wheel
    /// conversion, and supply unzoomed native geometry to `set_viewport`.
    #[must_use]
    pub fn viewport(&self) -> crate::Viewport {
        self.window.shared.viewport.get()
    }
    pub(super) fn native_viewport(&self, native: crate::Viewport) -> Result<(), RuntimeError> {
        let viewport = effective(native, self.shared.zoom.get())?;
        self.window.shared.native_viewport.set(native);
        self.window.shared.change_viewport(viewport);
        Ok(())
    }
    pub(super) fn geometry_pending(&self) -> bool {
        self.window.shared.geometry_change.get().is_some()
            || self.window.shared.theme_geometry_change.get()
    }
    pub(super) fn apply_geometry_change(&mut self) -> Result<(), RuntimeError> {
        let previous_scale = self.window.shared.geometry_change.take();
        let theme_changed = self.window.shared.theme_geometry_change.replace(false);
        if previous_scale.is_none() && !theme_changed {
            return Ok(());
        }
        let mut targets = std::collections::BTreeSet::new();
        if let Some(serial) = self.window.shared.capture.get() {
            targets.insert(serial);
        }
        if self.keyboard.pressed().next().is_some()
            && let Some(serial) = self.window.shared.focused.get()
        {
            targets.insert(serial);
        }
        {
            for (&serial, entry) in &self.entries {
                if !Rc::ptr_eq(&entry.node.window(), &self.window.shared) {
                    continue;
                }
                if entry
                    .node
                    .gestures
                    .borrow()
                    .iter()
                    .any(|gesture| gesture.strong_count() > 0)
                {
                    targets.insert(serial);
                }
            }
        }
        // Cancel actual owners once, even when zoom and density coalesce. Idle
        // focused or hovered components keep their local state and keyboard focus.
        for serial in targets {
            if let Some(entry) = self.entries.get_mut(&serial) {
                entry.node.input_cancelled.set(true);
                entry.node.lose_context();
                entry.component.cancel_input(&mut EventContext {
                    node: entry.node.clone(),
                });
                entry.node.invalidate(Invalidation::Paint);
            }
        }
        // Cancellation callbacks may legally adjust presentation settings. Fold
        // those writes into this transition using its earliest coordinate scale.
        let callback_scale = self.window.shared.geometry_change.take();
        let previous_scale = previous_scale.or(callback_scale);
        self.window.shared.theme_geometry_change.set(false);
        if let Some(previous_scale) = previous_scale {
            let old_point = self.window.shared.pointer_position.get();
            let leave = if let Some(old) = self.window.pointer_target.take()
                && let Some(point) = old_point
            {
                self.dispatch_pointer(
                    old.0.serial,
                    crate::PointerEvent::new(point, crate::PointerKind::Leave),
                )
            } else {
                Ok(crate::InputOutcome::Ignored)
            };
            // Finish coordinate cleanup even if Leave failed, and coalesce any
            // settings changed by that callback before the single reprojection.
            self.window.shared.geometry_change.set(None);
            self.window.shared.theme_geometry_change.set(false);
            let scale = self.window.shared.viewport.get().scale();
            self.window
                .shared
                .pointer_position
                .set(old_point.and_then(|point| {
                    crate::Point::new(
                        (f64::from(point.x()) * previous_scale / scale) as f32,
                        (f64::from(point.y()) * previous_scale / scale) as f32,
                    )
                    .ok()
                }));
            for entry in self
                .entries
                .values()
                .filter(|entry| Rc::ptr_eq(&entry.node.window(), &self.window.shared))
            {
                entry.node.invalidate(Invalidation::Layout);
            }
            leave?;
        }

        Ok(())
    }
}
impl UpdateContext {
    /// Shared UI-thread presentation settings; constructing this handle
    /// subscribes nothing. Use [`Self::strings`] to track texts during update.
    pub fn ui_settings(&self) -> UiSettings {
        UiSettings {
            shared: self.node.shared.clone(),
        }
    }

    /// The user's motion preference, subscribing this component to its changes.
    ///
    /// A change invalidates the component for paint, so its next update sees the
    /// new value and can start or stop requesting [`animate`](Self::animate).
    /// The subscription lasts until an update that does not call this: a
    /// component whose last update did not read the preference is not
    /// invalidated by it.
    #[must_use]
    pub fn motion(&self) -> MotionPreference {
        self.node.motion_reader.set(true);
        if !self.node.motion_listed.replace(true) {
            let mut subscribers = self.node.shared.motion_subscribers.borrow_mut();
            // Unmounted readers are dropped when the list would otherwise grow.
            if subscribers.len() == subscribers.capacity() {
                subscribers.retain(|node| node.strong_count() > 0);
            }
            subscribers.push(Rc::downgrade(&self.node));
        }
        self.node.shared.motion.get()
    }
    /// The installed table of library strings, subscribing this component to
    /// its replacement.
    ///
    /// [`UiSettings::set_strings`] invalidates the component for layout, so its
    /// next update reads the new texts; the subscription lasts until an update
    /// that does not call this. A built-in component reads the table here and
    /// keeps the texts it needs for layout, paint, inspection and events. A
    /// custom component can do the same for its own use of library words.
    ///
    /// # Errors
    /// Returns `Closed` after shutdown or `BorrowConflict` for a read during the
    /// table's publication.
    pub fn strings(&mut self) -> Result<crate::UiStrings, RuntimeError> {
        let shared = self.node.shared.clone();
        let current = shared.strings.borrow().clone();
        let signal = shared
            .settings_signals
            .strings
            .read_cloned(&shared, &current);
        let strings = self.read(&signal)?;
        // The setting's signal lives only while someone holds it; this
        // component holds it until its next update releases its subscriptions.
        self.subscriptions
            .push(crate::signal::Subscription::keep_alive(signal));
        Ok(strings)
    }
    /// Current application zoom. Every zoom change invalidates visible components,
    /// so readouts refresh without polling. This does not include native DPI.
    #[must_use]
    pub fn ui_zoom(&self) -> UiZoom {
        self.node.shared.zoom.get()
    }
}
impl EventContext {
    /// Shared UI-thread presentation settings for constructing initial parts
    /// or changing preferences from an event. Clones share the same runtime.
    pub fn ui_settings(&self) -> UiSettings {
        UiSettings {
            shared: self.node.shared.clone(),
        }
    }

    /// The installed table of library strings, for text a component produces
    /// in an event, such as a refusal message. It subscribes nothing; text a
    /// component shows should be read in update through
    /// [`UpdateContext::strings`].
    pub fn strings(&self) -> crate::UiStrings {
        self.node.shared.strings.borrow().clone()
    }
}

/// A binding to one role's text in the installed table, for retained rows
/// such as menu items that take their label from a signal. `None` when it
/// cannot be derived in the current phase, such as during layout; the caller
/// then keeps its constructor text.
pub(crate) fn string_signal(
    shared: &Rc<Shared>,
    role: crate::StringRole,
) -> Option<ReadSignal<String>> {
    let current = shared.strings.borrow().clone();
    let strings = shared
        .settings_signals
        .strings
        .read_cloned(shared, &current);
    strings.map(move |table| table.text(role).to_owned()).ok()
}

/// The application's `configured` text, or the library text for `role`,
/// following the installed table and showing its current text until the
/// first read.
pub(crate) fn library_text(
    shared: &Rc<Shared>,
    configured: Option<crate::TextSource>,
    role: crate::StringRole,
) -> crate::TextSource {
    configured.unwrap_or_else(|| {
        crate::TextSource::library(role, shared.strings.borrow().text(role).to_owned())
    })
}
