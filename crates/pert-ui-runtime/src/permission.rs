//! One shape for a capability the user may or may not exercise.

use crate::{ReadSignal, RuntimeError, UpdateContext};

/// Whether the user may exercise one capability of a component, such as
/// resizing a divider: allowed, denied, or decided at run time by a
/// `ReadSignal<bool>` the application owns.
///
/// A permission only ever restricts. Builders that take one combine it with
/// what the description already holds through [`Self::and`], so the stricter
/// setting wins: a fixed lock cannot be lifted by a later bound permission on
/// the same description, and a bound permission can only deny while the lock is
/// absent. The same rule combines a layout-level permission with an item-level
/// one, as a container that applies its own setting to every child does.
///
/// Builders take `impl Into<Permission>`, so `true`, `false` and a
/// `ReadSignal<bool>` can be passed directly. A component reads a permission
/// in its update with [`Self::read`], which subscribes to every bound signal,
/// so the application can change a capability while the component is mounted.
/// Use [`Self::read_paint`] when capability changes leave geometry unchanged.
/// What a component does when a permission is revoked while the user is
/// exercising it (for example cancelling a held drag) is documented by the
/// component.
///
/// ```
/// use pert_ui_runtime::{Permission, Runtime};
///
/// let ui = Runtime::new();
/// let editing = ui.signal(true);
/// // The layout allows resizing only while editing; one item never resizes.
/// let layout = Permission::from(editing.read_only());
/// let item = Permission::from(false);
/// assert_eq!(layout.clone().and(item).fixed(), Some(false));
/// // Allowed on both levels: the layout's signal decides.
/// let open = layout.and(true);
/// assert_eq!(open.fixed(), None);
/// assert!(open.get()?);
/// editing.set(false)?;
/// assert!(!open.get()?);
/// # Ok::<(), pert_ui_runtime::RuntimeError>(())
/// ```
#[derive(Clone, Debug)]
pub struct Permission {
    /// `false` once any fixed denial was combined in; then `bound` is empty.
    allowed: bool,
    /// Bound conditions, all of which must read true.
    bound: Vec<ReadSignal<bool>>,
}

impl Default for Permission {
    /// Allowed, with no bound condition.
    fn default() -> Self {
        Self::allowed()
    }
}

impl Permission {
    /// A permission that always allows.
    #[must_use]
    pub const fn allowed() -> Self {
        Self {
            allowed: true,
            bound: Vec::new(),
        }
    }
    /// A permission that always denies.
    #[must_use]
    pub const fn denied() -> Self {
        Self {
            allowed: false,
            bound: Vec::new(),
        }
    }
    /// Combines two permissions so that the stricter wins: the result allows
    /// only when both allow. A fixed denial on either side makes the result a
    /// fixed denial; otherwise every bound condition of both sides must read
    /// true.
    #[must_use]
    pub fn and(mut self, other: impl Into<Self>) -> Self {
        let other = other.into();
        if !self.allowed || !other.allowed {
            return Self::denied();
        }
        for signal in other.bound {
            if !self.bound.iter().any(|known| known.same_source(&signal)) {
                self.bound.push(signal);
            }
        }
        self
    }
    /// The permission's value when it does not depend on a signal: `Some(true)`
    /// for allowed, `Some(false)` for denied, `None` when bound conditions
    /// decide.
    #[must_use]
    pub fn fixed(&self) -> Option<bool> {
        if !self.allowed {
            Some(false)
        } else if self.bound.is_empty() {
            Some(true)
        } else {
            None
        }
    }
    /// Reads the current value in a component's update and subscribes the
    /// component to every bound condition, so a change updates it again.
    ///
    /// # Errors
    /// Returns the runtime error of reading a bound signal, such as a closed
    /// runtime or a signal from another runtime.
    pub fn read(&self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        self.read_using(|signal| cx.read(signal))
    }
    /// Reads a permission whose changes affect presentation and input admission
    /// inside unchanged geometry, subscribing to every bound condition.
    ///
    /// Called on the UI thread during component update. Changes schedule Paint
    /// rather than clearing measurements. If a capability changes measurements
    /// or placement, use [`Self::read`] or explicitly request layout on those
    /// updates through [`UpdateContext::request_layout`]. Fixed denial remains
    /// final, and every bound condition is observed even while another denies.
    ///
    /// # Errors
    /// Returns the runtime error of reading a bound signal, such as a closed
    /// runtime or a signal from another runtime.
    pub fn read_paint(&self, cx: &mut UpdateContext) -> Result<bool, RuntimeError> {
        self.read_using(|signal| cx.read_paint(signal))
    }
    fn read_using(
        &self,
        mut read: impl FnMut(&ReadSignal<bool>) -> Result<bool, RuntimeError>,
    ) -> Result<bool, RuntimeError> {
        if !self.allowed {
            return Ok(false);
        }
        let mut allowed = true;
        // Observe every condition so denial never drops another subscription.
        for signal in &self.bound {
            allowed &= read(signal)?;
        }
        Ok(allowed)
    }
    /// Reads the current value without subscribing, for code outside a
    /// component's update.
    ///
    /// # Errors
    /// Returns the runtime error of reading a bound signal.
    pub fn get(&self) -> Result<bool, RuntimeError> {
        if !self.allowed {
            return Ok(false);
        }
        for signal in &self.bound {
            if !signal.get()? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// Reads the current value without subscribing, refusing a bound
    /// condition from another runtime than `shared`.
    pub(crate) fn get_in(
        &self,
        shared: &std::rc::Rc<crate::runtime::Shared>,
    ) -> Result<bool, RuntimeError> {
        if self
            .bound
            .iter()
            .any(|signal| !std::rc::Rc::ptr_eq(signal.shared(), shared))
        {
            return Err(RuntimeError::ForeignRuntime);
        }
        self.get()
    }
    /// The permission as one derived binding when bound conditions decide it,
    /// for the builders that take a `ReadSignal<bool>`, such as
    /// [`Runtime::command_enabled_signal`](crate::Runtime::command_enabled_signal); `None` when
    /// it is fixed, which [`Self::fixed`] then reports. The binding reads true
    /// while every bound condition does and follows them without polling.
    ///
    /// ```
    /// use pert_ui_runtime::{Permission, Runtime};
    ///
    /// let ui = Runtime::new();
    /// let (a, b) = (ui.signal(true), ui.signal(true));
    /// let both = Permission::from(a.read_only()).and(b.read_only());
    /// let enabled = both.signal()?.expect("bound conditions decide");
    /// assert!(enabled.get()?);
    /// b.set(false)?;
    /// assert!(!enabled.get()?);
    /// assert!(Permission::from(false).signal()?.is_none());
    /// # Ok::<(), pert_ui_runtime::RuntimeError>(())
    /// ```
    ///
    /// # Errors
    /// Returns `ForeignRuntime` when the conditions belong to different
    /// runtimes, and the read errors of deriving a binding, such as a closed
    /// runtime or a call during a layout or paint phase.
    pub fn signal(&self) -> Result<Option<ReadSignal<bool>>, RuntimeError> {
        if !self.allowed {
            return Ok(None);
        }
        let mut conditions = self.bound.iter();
        let Some(first) = conditions.next() else {
            return Ok(None);
        };
        let mut combined = first.clone();
        for condition in conditions {
            combined = combined.combine(condition, |a, b| *a && *b)?;
        }
        Ok(Some(combined))
    }
    /// Whether two permissions are the same description: equal fixed values,
    /// or the same bound signals in any order. A component compares the old and
    /// new permission with this when it is described again.
    #[must_use]
    pub fn same_as(&self, other: &Self) -> bool {
        self.allowed == other.allowed
            && self.bound.len() == other.bound.len()
            && self
                .bound
                .iter()
                .all(|signal| other.bound.iter().any(|known| known.same_source(signal)))
    }
}

impl From<bool> for Permission {
    /// `true` allows and `false` denies, independent of any signal.
    fn from(allowed: bool) -> Self {
        if allowed {
            Self::allowed()
        } else {
            Self::denied()
        }
    }
}

impl From<ReadSignal<bool>> for Permission {
    /// Allows while the signal reads true.
    fn from(signal: ReadSignal<bool>) -> Self {
        Self {
            allowed: true,
            bound: vec![signal],
        }
    }
}

impl From<&ReadSignal<bool>> for Permission {
    /// Allows while the signal reads true; the signal is shared, not copied.
    fn from(signal: &ReadSignal<bool>) -> Self {
        signal.clone().into()
    }
}

impl From<crate::Signal<bool>> for Permission {
    /// Allows while the signal reads true; the permission only reads it.
    fn from(signal: crate::Signal<bool>) -> Self {
        signal.read_only().into()
    }
}

impl From<&crate::Signal<bool>> for Permission {
    /// Allows while the signal reads true; the signal is shared, and the
    /// permission only reads it.
    fn from(signal: &crate::Signal<bool>) -> Self {
        signal.read_only().into()
    }
}
