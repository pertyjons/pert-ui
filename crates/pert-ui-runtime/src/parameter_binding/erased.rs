//! Family-tagged wrappers over the typed bindings, for descriptor-driven
//! collections that hold parameters of several families.
use super::*;

impl Runtime {
    /// Creates a parameter from a family-tagged specification, returning tagged
    /// handles of the same family.
    ///
    /// Use it for descriptor-driven collections holding several families. Typed and
    /// erased handles share identical validation, notification and failure
    /// behavior; match the variants for the typed handles.
    ///
    /// # Errors
    /// Same as [`Self::parameter`].
    pub fn parameter_any(
        &mut self,
        id: ParameterId,
        spec: AnySpec,
    ) -> Result<(AnyParameter, AnyParameterHost), RuntimeError> {
        Ok(match spec {
            AnySpec::Continuous(spec) => {
                let (parameter, host) = self.parameter(id, spec)?;
                (
                    AnyParameter::Continuous(parameter),
                    AnyParameterHost::Continuous(host),
                )
            }
            AnySpec::Stepped(spec) => {
                let (parameter, host) = self.parameter(id, spec)?;
                (
                    AnyParameter::Stepped(parameter),
                    AnyParameterHost::Stepped(host),
                )
            }
            AnySpec::Boolean(spec) => {
                let (parameter, host) = self.parameter(id, spec)?;
                (
                    AnyParameter::Boolean(parameter),
                    AnyParameterHost::Boolean(host),
                )
            }
            AnySpec::Choice(spec) => {
                let (parameter, host) = self.parameter(id, spec)?;
                (
                    AnyParameter::Choice(parameter),
                    AnyParameterHost::Choice(host),
                )
            }
        })
    }
}

/// A parameter binding of any family, tagged by its family.
///
/// Match the variants to bind typed controls. Clones share state like typed
/// bindings do. `Debug` shows the variant and identity only.
#[derive(Clone, Debug)]
pub enum AnyParameter {
    /// A continuous binding.
    Continuous(Parameter<f32>),
    /// A stepped binding.
    Stepped(Parameter<i64>),
    /// A boolean binding.
    Boolean(Parameter<bool>),
    /// A choice binding.
    Choice(Parameter<ChoiceId>),
}

macro_rules! each {
    ($value:expr, $binding:ident => $body:expr) => {
        match $value {
            Self::Continuous($binding) => $body,
            Self::Stepped($binding) => $body,
            Self::Boolean($binding) => $body,
            Self::Choice($binding) => $body,
        }
    };
}

impl AnyParameter {
    /// Returns the family tag.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        match self {
            Self::Continuous(_) => ParameterFamily::Continuous,
            Self::Stepped(_) => ParameterFamily::Stepped,
            Self::Boolean(_) => ParameterFamily::Boolean,
            Self::Choice(_) => ParameterFamily::Choice,
        }
    }
    /// Returns the complete identity.
    pub fn id(&self) -> &ParameterId {
        each!(self, parameter => parameter.id())
    }
    /// Captures the specification and state together, tagged.
    ///
    /// The result is an owned copy taken at this call; later edits do not change
    /// it. [`Parameter::snapshot`] differs: it returns a live signal.
    ///
    /// # Errors
    /// Returns `Closed` after runtime drop or `BorrowConflict` on reentrant reads.
    pub fn snapshot(&self) -> Result<AnyParameterSnapshot, RuntimeError> {
        each!(self, parameter => Ok(AnyParameterSnapshot::new(
            parameter.spec()?,
            parameter.snapshot().get()?,
        )))
    }
    /// Returns the read-only delivery health.
    #[must_use]
    pub fn delivery_status(&self) -> ReadSignal<DeliveryStatus> {
        each!(self, parameter => parameter.delivery_status())
    }
}

/// A terminal event of any family, delivered to an erased terminal sink.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyTerminalEvent {
    /// A continuous parameter's event.
    Continuous(TerminalEvent<f32>),
    /// A stepped parameter's event.
    Stepped(TerminalEvent<i64>),
    /// A boolean parameter's event.
    Boolean(TerminalEvent<bool>),
    /// A choice parameter's event.
    Choice(TerminalEvent<ChoiceId>),
}

impl AnyTerminalEvent {
    /// Returns the edit identity.
    pub fn token(&self) -> &EditToken {
        each!(self, event => &event.token)
    }
    /// Returns the base value when the edit began, tagged by its family: the
    /// undo baseline an erased sink records.
    #[must_use]
    pub fn begin_value(&self) -> AnyValue {
        each!(self, event => erase_begin(event))
    }
    /// Returns the terminal outcome with its values tagged by family.
    ///
    /// `EditOutcome::Committed` here is the local store's acceptance, exactly
    /// as in the typed [`TerminalEvent::outcome`].
    ///
    /// An undo stack for parameters of every family, without a match over them:
    ///
    /// ```
    /// # use pert_ui_runtime as pert_ui;
    /// use pert_ui::{
    ///     AnyParameter, AnySpec, AnyValue, EditOutcome, Element, KeyAction, KeyCode,
    ///     ParameterId, ParameterSpec, Runtime, Toggle,
    /// };
    /// use std::{cell::RefCell, rc::Rc};
    ///
    /// let mut ui = Runtime::new();
    /// let spec = AnySpec::Boolean(ParameterSpec::boolean("Bypass", false)?);
    /// let (bypass, host) = ui.parameter_any(ParameterId::new(1, 1, "bypass")?, spec)?;
    /// let undo = Rc::new(RefCell::new(Vec::new()));
    /// let stack = undo.clone();
    /// let _monitor = host.on_outcome(move |event| {
    ///     if let EditOutcome::Committed { .. } = event.outcome() {
    ///         stack.borrow_mut().push(event.begin_value());
    ///     }
    ///     Ok(())
    /// })?;
    /// let AnyParameter::Boolean(typed) = bypass else { unreachable!() };
    /// let toggle = ui.mount_tree(Element::new("bypass", Toggle::for_parameter(typed)))?;
    /// ui.focus(&toggle)?;
    /// ui.key_input(KeyCode::Space, KeyAction::Press)?;
    /// ui.key_input(KeyCode::Space, KeyAction::Release)?;
    /// assert_eq!(*undo.borrow(), [AnyValue::Boolean(false)]);
    ///
    /// // Undo writes the recorded value back through the erased owner.
    /// let before = undo.borrow_mut().pop().ok_or("an undo entry")?;
    /// host.write_base(before)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn outcome(&self) -> EditOutcome<AnyValue> {
        each!(self, event => erase_outcome(&event.outcome))
    }
}

fn erase_begin<T: ParameterValue>(event: &TerminalEvent<T>) -> AnyValue {
    event.begin_value.clone().into_any()
}

fn erase_outcome<T: ParameterValue>(outcome: &EditOutcome<T>) -> EditOutcome<AnyValue> {
    match outcome.clone() {
        EditOutcome::Submitted { value } => EditOutcome::Submitted {
            value: value.into_any(),
        },
        EditOutcome::Committed { value, revision } => EditOutcome::Committed {
            value: value.into_any(),
            revision,
        },
        EditOutcome::Cancelled => EditOutcome::Cancelled,
        EditOutcome::Rejected(error) => EditOutcome::Rejected(error),
        // `EditOutcome` is non-exhaustive; its documented reading of an unknown
        // outcome is a terminal result that did not commit, as `Cancelled` is.
        _ => EditOutcome::Cancelled,
    }
}

/// A preview event of any family, delivered to an erased preview sink.
#[derive(Clone, Debug, PartialEq)]
pub enum AnyPreviewEvent {
    /// A continuous parameter's event.
    Continuous(PreviewEvent<f32>),
    /// A stepped parameter's event.
    Stepped(PreviewEvent<i64>),
    /// A boolean parameter's event.
    Boolean(PreviewEvent<bool>),
    /// A choice parameter's event.
    Choice(PreviewEvent<ChoiceId>),
}

impl AnyPreviewEvent {
    /// Returns the edit identity.
    pub fn token(&self) -> &EditToken {
        each!(self, event => &event.token)
    }
}

/// A delivery monitor of any family, tagged like its parameter.
#[must_use = "a delivery monitor is the only record of failed host deliveries"]
#[derive(Clone, Debug)]
pub enum AnyDeliveryMonitor {
    /// A continuous parameter's monitor.
    Continuous(DeliveryMonitor<f32>),
    /// A stepped parameter's monitor.
    Stepped(DeliveryMonitor<i64>),
    /// A boolean parameter's monitor.
    Boolean(DeliveryMonitor<bool>),
    /// A choice parameter's monitor.
    Choice(DeliveryMonitor<ChoiceId>),
}

impl AnyDeliveryMonitor {
    /// Whether the report currently retains a delivery failure.
    #[must_use]
    pub fn is_faulted(&self) -> bool {
        each!(self, monitor => monitor.snapshot().is_faulted())
    }
    /// Whether the sinks are still registered.
    #[must_use]
    pub fn is_registered(&self) -> bool {
        each!(self, monitor => monitor.snapshot().registered)
    }
}

/// The authoritative owner of a parameter of any family, tagged by its family.
///
/// Erased writes check the family first and return
/// [`RuntimeError::Binding`] with [`BindingError::KindMismatch`] without any
/// mutation, then validate the
/// domain exactly as the typed owner does. Not `Clone`; dropping it drops the
/// typed owner inside. `Debug` shows the variant and identity only.
#[derive(Debug)]
pub enum AnyParameterHost {
    /// A continuous owner.
    Continuous(ParameterHost<f32>),
    /// A stepped owner.
    Stepped(ParameterHost<i64>),
    /// A boolean owner.
    Boolean(ParameterHost<bool>),
    /// A choice owner.
    Choice(ParameterHost<ChoiceId>),
}

fn typed<T: ParameterValue>(value: AnyValue) -> Result<T, RuntimeError> {
    let found = value.family();
    T::from_any(value).map_err(|_| {
        RuntimeError::Binding(BindingError::KindMismatch {
            expected: T::FAMILY,
            found,
        })
    })
}

impl AnyParameterHost {
    /// Returns the family tag.
    #[must_use]
    pub fn family(&self) -> ParameterFamily {
        match self {
            Self::Continuous(_) => ParameterFamily::Continuous,
            Self::Stepped(_) => ParameterFamily::Stepped,
            Self::Boolean(_) => ParameterFamily::Boolean,
            Self::Choice(_) => ParameterFamily::Choice,
        }
    }
    /// Returns the complete identity; see [`ParameterHost::id`].
    pub fn id(&self) -> &ParameterId {
        each!(self, host => host.id())
    }
    /// Performs an authoritative write of a value of the same family.
    ///
    /// # Errors
    /// [`RuntimeError::Binding`] with [`BindingError::KindMismatch`] without
    /// mutation, then the typed errors.
    pub fn write_base(&self, value: AnyValue) -> Result<(), RuntimeError> {
        each!(self, host => host.write_base(typed(value)?))
    }
    /// Replaces metadata with a specification of the same family.
    ///
    /// # Errors
    /// [`RuntimeError::Binding`] with [`BindingError::KindMismatch`] without
    /// mutation, then the typed errors.
    pub fn replace_spec(&self, spec: AnySpec) -> Result<(), RuntimeError> {
        let found = spec.family();
        let expected = self.family();
        let mismatch = RuntimeError::Binding(BindingError::KindMismatch { expected, found });
        match (self, spec) {
            (Self::Continuous(host), AnySpec::Continuous(spec)) => host.replace_spec(spec),
            (Self::Stepped(host), AnySpec::Stepped(spec)) => host.replace_spec(spec),
            (Self::Boolean(host), AnySpec::Boolean(spec)) => host.replace_spec(spec),
            (Self::Choice(host), AnySpec::Choice(spec)) => host.replace_spec(spec),
            _ => Err(mismatch),
        }
    }
    /// Supplies or clears an effective overlay of the same family.
    ///
    /// # Errors
    /// [`RuntimeError::Binding`] with [`BindingError::KindMismatch`] without
    /// mutation, then the typed errors.
    pub fn set_effective(&self, value: Option<AnyValue>) -> Result<(), RuntimeError> {
        each!(self, host => host.set_effective(value.map(typed).transpose()?))
    }
    /// Removes the incarnation; see [`ParameterHost::remove`].
    ///
    /// # Errors
    /// Returns only a runtime phase error (see [`Parameter`](crate::Parameter)),
    /// as [`ParameterHost::remove`] does.
    pub fn remove(&self) -> Result<(), RuntimeError> {
        each!(self, host => host.remove())
    }
    /// Registers an erased terminal sink; see [`ParameterHost::on_outcome`].
    ///
    /// # Errors
    /// Same as the typed registration.
    pub fn on_outcome(
        &self,
        mut sink: impl FnMut(&AnyTerminalEvent) -> Result<(), SinkError> + 'static,
    ) -> Result<AnyDeliveryMonitor, RuntimeError> {
        Ok(match self {
            Self::Continuous(host) => AnyDeliveryMonitor::Continuous(
                host.on_outcome(move |event| sink(&AnyTerminalEvent::Continuous(event.clone())))?,
            ),
            Self::Stepped(host) => AnyDeliveryMonitor::Stepped(
                host.on_outcome(move |event| sink(&AnyTerminalEvent::Stepped(event.clone())))?,
            ),
            Self::Boolean(host) => AnyDeliveryMonitor::Boolean(
                host.on_outcome(move |event| sink(&AnyTerminalEvent::Boolean(event.clone())))?,
            ),
            Self::Choice(host) => AnyDeliveryMonitor::Choice(
                host.on_outcome(move |event| sink(&AnyTerminalEvent::Choice(event.clone())))?,
            ),
        })
    }
    /// Registers an erased preview sink; see [`ParameterHost::on_preview`].
    ///
    /// # Errors
    /// Same as the typed registration.
    pub fn on_preview(
        &self,
        mut sink: impl FnMut(&AnyPreviewEvent) -> Result<(), SinkError> + 'static,
    ) -> Result<(), RuntimeError> {
        match self {
            Self::Continuous(host) => {
                host.on_preview(move |event| sink(&AnyPreviewEvent::Continuous(event.clone())))
            }
            Self::Stepped(host) => {
                host.on_preview(move |event| sink(&AnyPreviewEvent::Stepped(event.clone())))
            }
            Self::Boolean(host) => {
                host.on_preview(move |event| sink(&AnyPreviewEvent::Boolean(event.clone())))
            }
            Self::Choice(host) => {
                host.on_preview(move |event| sink(&AnyPreviewEvent::Choice(event.clone())))
            }
        }
    }
    /// Detaches both sinks; see [`ParameterHost::clear_sinks`].
    ///
    /// # Errors
    /// Same as the typed operation.
    pub fn clear_sinks(&self) -> Result<(), RuntimeError> {
        each!(self, host => host.clear_sinks())
    }
    /// Retries the retained terminal delivery; see [`ParameterHost::retry_outcome`].
    ///
    /// # Errors
    /// Same as the typed operation.
    pub fn retry_outcome(&self, token: &EditToken) -> Result<(), RuntimeError> {
        each!(self, host => host.retry_outcome(token))
    }
    /// Resolves a retained fault; see [`ParameterHost::resolve_delivery`].
    ///
    /// # Errors
    /// Same as the typed operation.
    pub fn resolve_delivery(
        &self,
        token: &EditToken,
        resolution: DeliveryResolution,
    ) -> Result<(), RuntimeError> {
        each!(self, host => host.resolve_delivery(token, resolution))
    }
}
