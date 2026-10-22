//! A check that a component applies a new description in place.
//!
//! [`Component::reconcile`](crate::Component::reconcile) decides what a re-described component keeps and
//! what it takes from the new description. [`redescribe`] performs the
//! re-description the runtime performs for a surviving keyed child, so a test
//! of a built-in or a custom component can check the outcome through
//! inspection, focus and its bindings.

use crate::{Column, Element, RuntimeError, WidgetId};

/// Why [`redescribe`] could not re-describe a component in place.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RedescriptionError {
    /// The descriptions have different keys, so the runtime would remove the
    /// first component and mount the second instead of re-describing it.
    #[error("both descriptions must use one key, not {before:?} and {after:?}")]
    KeyChanged {
        /// The first description's key.
        before: String,
        /// The second description's key.
        after: String,
    },
    /// The direct retained child disappeared or was replaced during settling.
    /// Component or application work may remove/recreate it between flushes;
    /// ordinary same-key descriptions of different types are rejected through
    /// [`RuntimeError::ChildTypeChanged`] before mutation.
    #[error("re-describing {key:?} did not retain its mounted incarnation")]
    Remounted {
        /// The key of the missing or replaced retained component.
        key: String,
    },
    /// A runtime operation failed while mounting, reconciling, flushing or
    /// resolving retained ownership. This includes a foreign binding rejected
    /// by update, or [`RuntimeError::Unmounted`] if work removed the helper root.
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    /// A described component could not be found under its column after a
    /// flush, for example because an update removed it.
    #[error("no component keyed {key:?} under its column")]
    NotFound {
        /// The missing key.
        key: String,
    },
    /// Removing a helper-owned tree failed. The original cause and every
    /// unreleased owned root are preserved; unrelated roots are never removed.
    /// Teardown hooks that queue errors are reported by subsequent runtime
    /// flushes, rather than this variant.
    #[error("conformance failed and helper-owned roots could not be released")]
    Cleanup {
        /// The original failed operation, preserved as this error's source.
        #[source]
        cause: Box<Self>,
        /// Helper-owned roots whose unmount failed, paired with that typed error.
        /// These exact UI-thread handles allow the caller to retry cleanup. Handles
        /// do not keep mounts alive and never identify a same-key replacement.
        roots: Vec<(WidgetId, RuntimeError)>,
    },
    /// The re-described component reports different semantics or properties
    /// than a fresh mount. Both reports are included even though failed helper
    /// trees are removed before returning.
    #[error("re-describing {key:?} kept state a fresh mount does not have")]
    Differs {
        /// The key of the compared components.
        key: String,
        /// What the re-described component reports.
        redescribed: Box<crate::ComponentDetails>,
        /// What a fresh mount of the second description reports.
        fresh: Box<crate::ComponentDetails>,
    },
}

/// A component mounted and re-described by [`redescribe`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Redescribed {
    /// The column root, keyed `redescribe`, mounted around the component.
    /// Unmount it to remove the test tree, and before re-describing again in
    /// the same runtime: root keys are unique.
    pub root: WidgetId,
    /// The retained component, now configured by the second description.
    pub component: WidgetId,
}

/// Mounts `before` as the only child of a new column root in `ui`, flushes,
/// describes it again as `after` (through
/// [`Runtime::reconcile_children`](crate::Runtime::reconcile_children), as a
/// parent re-describing a surviving child does) and flushes again.
///
/// It returns the retained component, so the caller can check that the second
/// description took effect: its texts, bindings, ranges, permissions and
/// callbacks. A component that keeps a property on purpose should say so in
/// its rustdoc; [`Component::reconcile`](crate::Component::reconcile) defaults to keeping everything. The
/// flushes use the runtime's current elapsed timestamp without advancing time;
/// the column is an additional root of `ui`
/// keyed `redescribe`, laid out in its current viewport, so unmount
/// [`Redescribed::root`] before checking another component in the same
/// runtime.
///
/// # Errors
/// [`RedescriptionError::KeyChanged`] when the keys differ,
/// [`RedescriptionError::Remounted`] when settling removed or replaced the
/// retained component, [`RedescriptionError::NotFound`] when it is missing
/// after the first flush, and [`RedescriptionError::Runtime`] for a failed mount,
/// reconciliation, flush or retained ownership lookup. Removing the helper root
/// during settling reports `Runtime(Unmounted)`. On any failure, the helper
/// removes its own tree.
/// [`RedescriptionError::Cleanup`] preserves the original cause and exact owned
/// handles when removal itself fails. Existing unrelated roots are preserved;
/// application side effects from callbacks are not rolled back.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{
///     redescribe, Component, Element, Invalidation, MeasureContext, Constraints, Painter,
///     DrawError, Runtime, RuntimeError, Size, UpdateContext,
/// };
/// // A custom component that copies its caption from a new description.
/// struct Caption(String);
/// impl Component for Caption {
///     fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
///         (self.0 != next.0).then(|| {
///             self.0 = next.0;
///             Invalidation::Layout
///         })
///     }
///     fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
///         Ok(Size::new(10.0, 10.0)?)
///     }
///     fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
///         Ok(())
///     }
///     fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
///         Ok(())
///     }
/// }
/// let mut ui = Runtime::new();
/// let result = redescribe(
///     &mut ui,
///     Element::new("caption", Caption("Before".into())),
///     Element::new("caption", Caption("After".into())),
/// )?;
/// assert_eq!(ui.key(&result.component)?, "caption");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn redescribe(
    ui: &mut crate::Runtime,
    before: Element,
    after: Element,
) -> Result<Redescribed, RedescriptionError> {
    if before.key != after.key {
        return Err(RedescriptionError::KeyChanged {
            before: before.key,
            after: after.key,
        });
    }
    let key = before.key.clone();
    let root = ui.mount_tree(Element::new("redescribe", Column::new([before])))?;
    let result = (|| {
        ui.flush(ui.now())?;
        let first = direct_child(ui, &root, &key)?;
        ui.reconcile_children(&root, vec![after])?;
        ui.flush(ui.now())?;
        let component = retained_child(ui, &root, &first, &key)?;
        Ok(Redescribed {
            root: root.clone(),
            component,
        })
    })();
    result.map_err(|cause| cleanup_failed(ui, cause, [&root]))
}

/// Re-describes `before` as `after` with [`redescribe`], mounts `fresh`, a
/// second copy of `after`, as another root, and checks that both report the
/// same semantics and inspection properties.
///
/// It checks what inspection shows: whatever the second description
/// configures that reaches the component's semantics or inspection
/// properties, such as its texts, bound values, ranges and enabled state,
/// must present exactly as a component built from it would. Build `after`
/// and `fresh` from one function, so they share bindings and values. Layout
/// data (the box model) is not compared.
///
/// It cannot see what inspection does not report. A callback the component
/// kept from the first description, or an edit in progress that a
/// re-description should end or keep, passes this check unnoticed; test those
/// by behavior after [`redescribe`]: trigger the callback and observe which
/// one runs, or start an edit, re-describe, and check the draft and the
/// binding.
///
/// # Errors
/// As [`redescribe`], [`RedescriptionError::NotFound`] when either component
/// cannot be found after the flush, and [`RedescriptionError::Differs`] when
/// the reports differ. Success removes the reference tree and returns the
/// retained tree under [`Redescribed::root`]. Every failure removes all roots
/// this call mounted; [`RedescriptionError::Cleanup`] preserves the original
/// cause and exact handles when removal itself fails. Comparison uses mounted
/// incarnation identity, independently of workspace addressing metadata.
///
/// ```
/// # use pert_ui_runtime as pert_ui;
/// use pert_ui::{check_redescription, Element, Label, Runtime};
/// let mut ui = Runtime::new();
/// let (before, after) = (ui.signal("Before".to_owned()), ui.signal("After".to_owned()));
/// let label = |text: &pert_ui::Signal<String>| {
///     Element::new("caption", Label::new(text.read_only()))
/// };
/// check_redescription(&mut ui, label(&before), label(&after), label(&after))?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn check_redescription(
    ui: &mut crate::Runtime,
    before: Element,
    after: Element,
    fresh: Element,
) -> Result<Redescribed, RedescriptionError> {
    let key = after.key.clone();
    let result = redescribe(ui, before, after)?;
    let reference = match ui.mount_tree(Element::new("fresh", Column::new([fresh]))) {
        Ok(reference) => reference,
        Err(error) => return Err(cleanup_failed(ui, error.into(), [&result.root])),
    };
    let compared = (|| {
        let (redescribed, fresh) = compare_reports(ui, &result, &reference, &key)?;
        if redescribed.semantics != fresh.semantics || redescribed.properties != fresh.properties {
            return Err(RedescriptionError::Differs {
                key,
                redescribed: Box::new(redescribed),
                fresh: Box::new(fresh),
            });
        }
        ui.unmount(&reference)?;
        Ok(())
    })();
    match compared {
        Ok(()) => Ok(result),
        Err(cause) => Err(cleanup_failed(ui, cause, [&reference, &result.root])),
    }
}

/// Resolves only the described direct child, independently of nested keys.
fn direct_child(
    ui: &crate::Runtime,
    root: &WidgetId,
    key: &str,
) -> Result<WidgetId, RedescriptionError> {
    let children = ui.children(root)?;
    if let [child] = children.as_slice()
        && ui.key(child)? == key
    {
        return Ok(child.clone());
    }
    Err(RedescriptionError::NotFound {
        key: key.to_owned(),
    })
}

fn retained_child(
    ui: &crate::Runtime,
    root: &WidgetId,
    first: &WidgetId,
    key: &str,
) -> Result<WidgetId, RedescriptionError> {
    match direct_child(ui, root, key) {
        Ok(child) if child == *first => Ok(child),
        Ok(_) | Err(RedescriptionError::NotFound { .. }) => Err(RedescriptionError::Remounted {
            key: key.to_owned(),
        }),
        Err(error) => Err(error),
    }
}

/// Releases exact helper-owned roots, never a replacement found by key.
fn cleanup_failed<'a>(
    ui: &mut crate::Runtime,
    cause: RedescriptionError,
    roots: impl IntoIterator<Item = &'a WidgetId>,
) -> RedescriptionError {
    let mut remaining = Vec::new();
    for root in roots {
        // A component can remove its own root or shut down the runtime during
        // flushing. A retired identity needs no cleanup and cannot retarget.
        if matches!(ui.key(root), Err(RuntimeError::Unmounted)) {
            continue;
        }
        if let Err(error) = ui.unmount(root) {
            remaining.push((root.clone(), error));
        }
    }
    if remaining.is_empty() {
        cause
    } else {
        RedescriptionError::Cleanup {
            cause: Box::new(cause),
            roots: remaining,
        }
    }
}

/// Flushes, revalidates retained ownership and compares actual incarnations.
fn compare_reports(
    ui: &mut crate::Runtime,
    result: &Redescribed,
    reference: &WidgetId,
    key: &str,
) -> Result<(crate::ComponentDetails, crate::ComponentDetails), RedescriptionError> {
    ui.flush(ui.now())?;
    let retained = retained_child(ui, &result.root, &result.component, key)?;
    let fresh = direct_child(ui, reference, key)?;
    let snapshot = ui.inspect();
    let details = |id: &WidgetId| {
        snapshot
            .components
            .iter()
            .find(|component| component.id == id.incarnation())
            .map(|component| component.details.clone())
    };
    match (details(&retained), details(&fresh)) {
        (Some(retained), Some(fresh)) => Ok((retained, fresh)),
        _ => Err(RedescriptionError::NotFound {
            key: key.to_owned(),
        }),
    }
}
