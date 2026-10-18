//! Adapter evidence through the public facade only: an application core with its
//! own value types, an adapter that translates descriptors and values, a
//! synchronous UI-thread host with undo, a fault-injection adapter with a
//! bounded downstream queue, and a custom control reproducing the built-in
//! parameter protocol.
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

use pert_ui::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The application's core: its own quantities and descriptors, no UI dependency.
mod app_core {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Gain(pub f64);
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Curve {
        Linear,
        Exponential,
    }
    #[derive(Clone, Debug, PartialEq)]
    pub enum Descriptor {
        Gain {
            key: &'static str,
            name: &'static str,
            range: (f64, f64),
            default: f64,
            curve: Curve,
        },
        Voices {
            key: &'static str,
            max: u8,
        },
        Mode {
            key: &'static str,
            options: Vec<(&'static str, &'static str)>,
        },
        Bypass {
            key: &'static str,
        },
    }
}

/// The adapter layer: wrappers it owns, translations and delivery policy.
#[derive(Debug, PartialEq)]
struct UiGain(app_core::Gain);

/// The adapter's own error. It is written by hand, as an external consumer
/// without an error-derive dependency would write it.
#[derive(Debug, PartialEq)]
enum AdapterError {
    Lossy,
    UnsupportedCurve(app_core::Curve),
    Parameter(ParameterError),
    Id(ParameterIdError),
}
impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lossy => f.write_str("value is not exactly representable"),
            Self::UnsupportedCurve(curve) => write!(f, "curve {curve:?} has no pert-ui equivalent"),
            Self::Parameter(error) => error.fmt(f),
            Self::Id(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for AdapterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // Transparent wrappers: the wrapped error's own source is the cause.
        match self {
            Self::Lossy | Self::UnsupportedCurve(_) => None,
            Self::Parameter(error) => error.source(),
            Self::Id(error) => error.source(),
        }
    }
}
impl From<ParameterError> for AdapterError {
    fn from(error: ParameterError) -> Self {
        Self::Parameter(error)
    }
}
impl From<ParameterIdError> for AdapterError {
    fn from(error: ParameterIdError) -> Self {
        Self::Id(error)
    }
}

impl HostValue for UiGain {
    type Canonical = f32;
    type Error = AdapterError;
    fn to_canonical(&self) -> Result<f32, AdapterError> {
        let value = self.0.0 as f32;
        if f64::from(value) == self.0.0 {
            Ok(value)
        } else {
            Err(AdapterError::Lossy)
        }
    }
    fn from_canonical(value: f32) -> Result<Self, AdapterError> {
        Ok(Self(app_core::Gain(f64::from(value))))
    }
}

fn exact(value: f64) -> Result<f32, AdapterError> {
    UiGain(app_core::Gain(value)).to_canonical()
}

/// Fallible descriptor translation: unsupported curves and lossy numbers are
/// rejected, never approximated.
fn translate(descriptor: &app_core::Descriptor) -> Result<(ParameterId, AnySpec), AdapterError> {
    use app_core::Descriptor;
    Ok(match descriptor {
        Descriptor::Gain {
            key,
            name,
            range,
            default,
            curve,
        } => {
            let spec =
                ParameterSpec::new(*name, exact(range.0)?, exact(range.1)?, exact(*default)?)?;
            let spec = match curve {
                app_core::Curve::Linear => spec,
                other => return Err(AdapterError::UnsupportedCurve(*other)),
            };
            (ParameterId::new(1, 1, *key)?, AnySpec::from(spec))
        }
        Descriptor::Voices { key, max } => (
            ParameterId::new(1, 2, *key)?,
            AnySpec::from(ParameterSpec::stepped("Voices", 1, i64::from(*max), 1)?),
        ),
        Descriptor::Mode { key, options } => {
            // Collect fallible identities before building the specification.
            let options = options
                .iter()
                .map(|(id, label)| Ok(ChoiceOption::new(ChoiceId::new(*id)?, *label)))
                .collect::<Result<Vec<_>, ParameterError>>()?;
            let default = options
                .first()
                .ok_or(ParameterError::EmptyOptions)?
                .id()
                .clone();
            (
                ParameterId::new(1, 3, *key)?,
                AnySpec::from(ParameterSpec::choices("Mode", options, default)?),
            )
        }
        Descriptor::Bypass { key } => (
            ParameterId::new(1, 4, *key)?,
            AnySpec::from(ParameterSpec::boolean("Bypass", false)?),
        ),
    })
}

/// A synchronous UI-thread application store with undo keyed by edit identity,
/// so a repeated delivery never records undo twice.
#[derive(Default)]
struct LocalStore {
    gains: Vec<app_core::Gain>,
    undo: Vec<(EditToken, AnyValue)>,
    applied: Vec<EditToken>,
}
impl LocalStore {
    fn apply_once(&mut self, event: &AnyTerminalEvent) -> Result<(), SinkError> {
        let token = event.token().clone();
        if self.applied.contains(&token) {
            return Ok(());
        }
        let before = match event {
            AnyTerminalEvent::Continuous(event) => {
                let EditOutcome::Committed { value, .. } = event.outcome else {
                    return Ok(()); // cancelled or rejected: nothing to record
                };
                // Translate before mutating: a failure changes nothing.
                let gain = UiGain::from_canonical(value).map_err(|error| {
                    SinkError::new(SinkErrorKind::InvalidValue, error.to_string())
                })?;
                self.gains.push(gain.0);
                AnyValue::Continuous(event.begin_value)
            }
            AnyTerminalEvent::Stepped(event) => match event.outcome {
                EditOutcome::Committed { .. } => AnyValue::Stepped(event.begin_value),
                _ => return Ok(()),
            },
            AnyTerminalEvent::Boolean(event) => match event.outcome {
                EditOutcome::Committed { .. } => AnyValue::Boolean(event.begin_value),
                _ => return Ok(()),
            },
            AnyTerminalEvent::Choice(event) => match event.outcome {
                EditOutcome::Committed { .. } => AnyValue::Choice(event.begin_value.clone()),
                _ => return Ok(()),
            },
        };
        self.undo.push((token.clone(), before));
        self.applied.push(token);
        Ok(())
    }
}

/// A custom control built only on the public parameter API: each primary
/// press nudges a stepped parameter one step up as one begin/update/commit.
struct Nudge(Parameter<i64>);
impl Component for Nudge {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read_paint(&self.0.snapshot())?;
        cx.read_paint(&self.0.delivery_status())?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(40.0, 20.0)?)
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        if event.kind() != PointerKind::Press(PointerButton::Primary) {
            return Ok(InputOutcome::Ignored);
        }
        let snapshot = self.0.snapshot().get()?;
        let spec = self.0.spec()?;
        let next = spec.offset(snapshot.base, 1)?;
        let result = (|| -> Result<EditOutcome<i64>, RuntimeError> {
            let session = self.0.begin_edit(cx, snapshot.revision)?;
            session.update(next)?;
            session.commit(next)
        })();
        // A refused edit is the parameter's answer, not a component failure.
        match result {
            Ok(_) | Err(RuntimeError::Binding(_)) => Ok(InputOutcome::Handled),
            Err(error) => Err(error),
        }
    }
}

#[test]
fn a_synchronous_host_translates_descriptors_and_records_undo_once() -> TestResult {
    use app_core::{Curve, Descriptor};
    let descriptors = vec![
        Descriptor::Gain {
            key: "gain",
            name: "Gain",
            range: (-60.0, 12.0),
            default: 0.0,
            curve: Curve::Linear,
        },
        Descriptor::Voices {
            key: "voices",
            max: 8,
        },
        Descriptor::Mode {
            key: "mode",
            options: vec![("poly", "Poly"), ("mono", "Mono")],
        },
        Descriptor::Bypass { key: "bypass" },
    ];
    // Unsupported and lossy descriptors are rejected before anything is published.
    assert_eq!(
        translate(&Descriptor::Gain {
            key: "drive",
            name: "Drive",
            range: (0.0, 1.0),
            default: 0.0,
            curve: Curve::Exponential,
        })
        .err(),
        Some(AdapterError::UnsupportedCurve(Curve::Exponential))
    );
    assert_eq!(
        translate(&Descriptor::Gain {
            key: "trim",
            name: "Trim",
            range: (0.0, 0.1),
            default: 0.0,
            curve: Curve::Linear,
        })
        .err(),
        Some(AdapterError::Lossy)
    );

    let mut ui = Runtime::new();
    let store = Rc::new(RefCell::new(LocalStore::default()));
    let mut panel = BTreeMap::new();
    for descriptor in &descriptors {
        let (id, spec) = translate(descriptor)?;
        let key = id.key().to_owned();
        let (parameter, host) = ui.parameter_any(id, spec)?;
        let sink_store = store.clone();
        let monitor = host.on_outcome(move |event| {
            // Borrow only inside the sink; nothing else holds the store here.
            sink_store
                .try_borrow_mut()
                .map_err(|_| SinkError::new(SinkErrorKind::Other, "store busy"))?
                .apply_once(event)
        })?;
        panel.insert(key, (parameter, host, monitor));
    }
    // Initialize from a coherent host snapshot, not from defaults.
    let (_, gain_host, _) = &panel["gain"];
    gain_host.write_base(AnyValue::Continuous(
        UiGain(app_core::Gain(-6.0)).to_canonical()?,
    ))?;

    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let AnyParameter::Stepped(voices) = panel["voices"].0.clone() else {
        return Err("voices is stepped".into());
    };
    let nudge = ui.mount_tree(Element::new("nudge", Nudge(voices.clone())))?;
    ui.flush(std::time::Duration::ZERO)?;
    let [x, y, _, _] = ui.bounds(&nudge)?.ok_or("arranged")?.values();
    let at = Point::new(x + 1.0, y + 1.0)?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Press(PointerButton::Primary),
    ))?;
    ui.pointer_input(PointerEvent::new(
        at,
        PointerKind::Release(PointerButton::Primary),
    ))?;
    assert_eq!(voices.snapshot().get()?.base, 2);

    let AnyParameter::Continuous(gain) = panel["gain"].0.clone() else {
        return Err("gain is continuous".into());
    };
    let edit = ui.event(&nudge, |cx| {
        gain.begin_edit(cx, gain.snapshot().get()?.revision)
    })??;
    edit.update(-3.0)?;
    edit.commit(-3.0)?;
    drop(edit);

    let store = store.borrow();
    assert_eq!(store.gains, [app_core::Gain(-3.0)]);
    assert_eq!(
        store
            .undo
            .iter()
            .map(|entry| entry.1.clone())
            .collect::<Vec<_>>(),
        [AnyValue::Stepped(1), AnyValue::Continuous(-6.0)]
    );
    assert!(panel.values().all(|(_, _, monitor)| !monitor.is_faulted()));
    Ok(())
}

/// A bounded downstream queue standing in for an engine connection.
struct EngineQueue {
    capacity: usize,
    commands: VecDeque<(EditToken, i64)>,
    /// Bounded dedupe of identities the adapter accepted responsibility for.
    accepted: VecDeque<EditToken>,
}
impl EngineQueue {
    const DEDUPE: usize = 16;
    fn send(&mut self, event: &TerminalEvent<i64>) -> Result<(), SinkError> {
        let EditOutcome::Committed { value, .. } = event.outcome else {
            return Ok(());
        };
        if self.accepted.contains(&event.token) {
            return Ok(()); // a retry of an accepted command: never resend blindly
        }
        if self.commands.len() >= self.capacity {
            return Err(SinkError::new(SinkErrorKind::Full, "engine queue full"));
        }
        self.commands.push_back((event.token.clone(), value));
        if self.accepted.len() == Self::DEDUPE {
            self.accepted.pop_front();
        }
        self.accepted.push_back(event.token.clone());
        Ok(())
    }
}

#[test]
fn a_full_downstream_queue_is_retained_retried_and_never_duplicated() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let (steps, host) = ui.parameter(
        ParameterId::new(2, 1, "steps")?,
        ParameterSpec::stepped("Steps", 0, 64, 16)?,
    )?;
    let engine = Rc::new(RefCell::new(EngineQueue {
        capacity: 1,
        commands: VecDeque::new(),
        accepted: VecDeque::new(),
    }));
    let queue = engine.clone();
    let monitor = host.on_outcome(move |event| {
        queue
            .try_borrow_mut()
            .map_err(|_| SinkError::new(SinkErrorKind::Other, "queue busy"))?
            .send(event)
    })?;
    let control = ui.mount_tree(Element::new(
        "steps",
        IntegerInput::for_parameter(steps.clone()),
    ))?;
    ui.flush(std::time::Duration::ZERO)?;
    let issues = ui.validation_report().issues;
    assert!(issues.is_empty(), "{issues:?}");

    // First commit fills the queue; the second finds it full.
    for value in [20, 24] {
        let edit = ui.event(&control, |cx| {
            steps.begin_edit(cx, steps.snapshot().get()?.revision)
        })??;
        edit.commit(value)?;
    }
    let report = monitor.snapshot();
    assert!(report.is_faulted());
    assert_eq!(
        report.last_error.as_ref().map(SinkError::kind),
        Some(SinkErrorKind::Full)
    );
    let retained = report.terminal.ok_or("retained terminal event")?;
    assert!(matches!(
        retained.outcome,
        EditOutcome::Committed { value: 24, .. }
    ));
    // Edits are refused until recovery; the local value is already committed.
    assert_eq!(steps.snapshot().get()?.base, 24);
    assert!(matches!(
        ui.event(&control, |cx| steps
            .begin_edit(cx, steps.snapshot().get()?.revision))?,
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::Faulted
        )))
    ));

    // The engine drains, then the adapter retries the same immutable event.
    engine.borrow_mut().commands.pop_front();
    host.retry_outcome(&retained.token)?;
    assert!(!monitor.snapshot().is_faulted());
    assert_eq!(
        engine
            .borrow()
            .commands
            .iter()
            .map(|command| command.1)
            .collect::<Vec<_>>(),
        [24]
    );
    // A second retry is refused rather than duplicated.
    assert_eq!(
        host.retry_outcome(&retained.token),
        Err(RuntimeError::Binding(BindingError::Delivery(
            DeliveryError::NotFaulted
        )))
    );

    // An unknown engine result: the adapter reconciles by identity instead of
    // resending, then resolves the fault explicitly.
    engine.borrow_mut().capacity = 0;
    let edit = ui.event(&control, |cx| {
        steps.begin_edit(cx, steps.snapshot().get()?.revision)
    })??;
    edit.commit(32)?;
    let token = edit.token().clone();
    drop(edit);
    assert!(monitor.snapshot().is_faulted());
    let engine_state = 32; // the adapter asked the engine and learned it applied 32
    host.write_base(engine_state)?; // an authoritative, corrected engine value
    host.resolve_delivery(&token, DeliveryResolution::Reconciled)?;
    assert!(!monitor.snapshot().is_faulted());
    assert_eq!(steps.delivery_status().get()?, DeliveryStatus::Ready);
    Ok(())
}
