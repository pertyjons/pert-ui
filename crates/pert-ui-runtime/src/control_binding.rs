//! The one seam between built-in discrete controls and their value: a UI-local
//! signal, or a parameter driven through its public edit protocol.
//!
//! Parameter bindings use only public API (`snapshot`, `spec`, `delivery_status`,
//! `authority_status`, `begin_edit`), so a custom control can reproduce what the built-ins do.
use crate::*;
use std::rc::Rc;

/// What a control reads from its binding during update.
pub(crate) struct Reading<T> {
    /// The value to display: a parameter's preview or base, or the signal value.
    pub(crate) value: T,
    /// Whether user edits may begin: always for signals; for parameters an
    /// available, editable, unfaulted binding.
    pub(crate) editable: bool,
    /// The parameter revision observed, the expected revision of the next edit.
    pub(crate) revision: Revision,
    /// The parameter's display name, for controls whose label follows it.
    pub(crate) name: Option<String>,
    /// Options supplied by a choice parameter's specification.
    pub(crate) options: Option<Vec<Choice<T>>>,
}

pub(crate) trait ValueBinding<T>: std::any::Any {
    /// Reads the current state, subscribing the component for repaint.
    fn read(&self, cx: &mut UpdateContext) -> Result<Reading<T>, RuntimeError>;
    /// The current value without subscribing.
    fn current(&self) -> Result<T, RuntimeError>;
    /// Applies one user choice observed at `observed`. Returns whether the value
    /// changed locally or a remote proposal was admitted for submission.
    /// Choosing the current value is a no-op; protocol refusals (a busy,
    /// stale, read-only, unavailable or faulted parameter) change nothing and
    /// return `false`; runtime failures propagate.
    fn choose(
        &self,
        cx: &mut EventContext,
        value: T,
        observed: Revision,
    ) -> Result<bool, RuntimeError>;
    /// Whether `other` edits the same signal or parameter as this binding.
    /// Reconciliation keeps pending input only while the target is the same.
    fn same_target(&self, other: &dyn ValueBinding<T>) -> bool;
    /// Whether the binding supplies its own options, replacing any fixed list.
    fn supplies_options(&self) -> bool {
        false
    }
}

/// Whether two bindings edit the same target; see [`ValueBinding::same_target`].
pub(crate) fn same_binding<T: 'static>(
    a: &Rc<dyn ValueBinding<T>>,
    b: &Rc<dyn ValueBinding<T>>,
) -> bool {
    a.same_target(&**b)
}

/// Whether `other` is a parameter handle for the same parameter as `parameter`.
fn same_parameter<T: ParameterValue>(parameter: &Parameter<T>, other: &dyn std::any::Any) -> bool {
    other
        .downcast_ref::<Parameter<T>>()
        .is_some_and(|other| other.snapshot().same_source(&parameter.snapshot()))
}

impl<T: Clone + PartialEq + 'static> ValueBinding<T> for Signal<T> {
    fn read(&self, cx: &mut UpdateContext) -> Result<Reading<T>, RuntimeError> {
        Ok(Reading {
            value: cx.read_paint(self)?,
            editable: true,
            revision: Revision::INITIAL,
            name: None,
            options: None,
        })
    }
    fn current(&self) -> Result<T, RuntimeError> {
        self.get()
    }
    fn choose(&self, _: &mut EventContext, value: T, _: Revision) -> Result<bool, RuntimeError> {
        self.set_changed(value)
    }
    fn same_target(&self, other: &dyn ValueBinding<T>) -> bool {
        let other: &dyn std::any::Any = other;
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| other.read_only().same_source(&self.read_only()))
    }
}

/// Local bindings and connected remote bindings with no pending proposal allow edits.
pub(crate) fn authority_allows_edit<T>(status: Option<&AuthorityStatus<T>>) -> bool {
    matches!(
        status,
        None | Some(AuthorityStatus::Ready | AuthorityStatus::Resolved { .. })
    )
}

/// Reads a parameter's snapshot, delivery status and specification together.
pub(crate) fn read_parameter<T: ParameterValue>(
    parameter: &Parameter<T>,
    cx: &mut UpdateContext,
) -> Result<(ParameterSnapshot<T>, ParameterSpec<T>, bool), RuntimeError> {
    let snapshot = cx.read_paint(&parameter.snapshot())?;
    let status = cx.read_paint(&parameter.delivery_status())?;
    // Metadata replacement advances the revision, so the snapshot subscription
    // also covers the specification.
    let spec = parameter.spec()?;
    let authority = parameter
        .authority_status()
        .map(|status| cx.read_paint(&status))
        .transpose()?;
    let authority_ready = authority_allows_edit(authority.as_ref());
    let editable = snapshot.available
        && spec.is_editable()
        && status != DeliveryStatus::Faulted
        && authority_ready;
    Ok((snapshot, spec, editable))
}

/// Maps a binding failure to the control contract: runtime errors propagate,
/// protocol refusals are reported as "nothing changed".
fn refused<R>(result: Result<R, RuntimeError>) -> Result<Option<R>, RuntimeError> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(RuntimeError::Binding(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// One begin/commit pair for a changed value, beginning at the observed revision.
pub(crate) fn choose_parameter<T: ParameterValue>(
    parameter: &Parameter<T>,
    cx: &mut EventContext,
    value: T,
    observed: Revision,
) -> Result<bool, RuntimeError> {
    if parameter.snapshot().get()?.base == value {
        return Ok(false);
    }
    let Some(session) = refused(parameter.begin_edit(cx, observed))? else {
        return Ok(false);
    };
    Ok(matches!(
        refused(session.commit(value))?,
        Some(EditOutcome::Committed { .. } | EditOutcome::Submitted { .. })
    ))
}

fn displayed<T: Clone>(snapshot: &ParameterSnapshot<T>) -> T {
    snapshot
        .preview
        .clone()
        .unwrap_or_else(|| snapshot.base.clone())
}

impl ValueBinding<bool> for Parameter<bool> {
    fn read(&self, cx: &mut UpdateContext) -> Result<Reading<bool>, RuntimeError> {
        let (snapshot, spec, editable) = read_parameter(self, cx)?;
        Ok(Reading {
            value: displayed(&snapshot),
            editable,
            revision: snapshot.revision,
            name: Some(spec.name().to_owned()),
            options: None,
        })
    }
    fn current(&self) -> Result<bool, RuntimeError> {
        Ok(displayed(&self.snapshot().get()?))
    }
    fn choose(
        &self,
        cx: &mut EventContext,
        value: bool,
        observed: Revision,
    ) -> Result<bool, RuntimeError> {
        choose_parameter(self, cx, value, observed)
    }
    fn same_target(&self, other: &dyn ValueBinding<bool>) -> bool {
        same_parameter(self, other)
    }
}

impl ValueBinding<ChoiceId> for Parameter<ChoiceId> {
    fn read(&self, cx: &mut UpdateContext) -> Result<Reading<ChoiceId>, RuntimeError> {
        let (snapshot, spec, editable) = read_parameter(self, cx)?;
        Ok(Reading {
            value: displayed(&snapshot),
            editable,
            revision: snapshot.revision,
            name: Some(spec.name().to_owned()),
            options: Some(
                spec.options()
                    .iter()
                    .map(|option| Choice::new(option.id().clone(), option.label()))
                    .collect(),
            ),
        })
    }
    fn current(&self) -> Result<ChoiceId, RuntimeError> {
        Ok(displayed(&self.snapshot().get()?))
    }
    fn choose(
        &self,
        cx: &mut EventContext,
        value: ChoiceId,
        observed: Revision,
    ) -> Result<bool, RuntimeError> {
        choose_parameter(self, cx, value, observed)
    }
    fn same_target(&self, other: &dyn ValueBinding<ChoiceId>) -> bool {
        same_parameter(self, other)
    }
    fn supplies_options(&self) -> bool {
        true
    }
}
