#![doc = include_str!("../README.md")]

mod choice;
mod edit;
mod erased;
mod family;
mod live;
mod normalized;
mod parameter;
mod parameter_id;

pub use choice::{ChoiceId, ChoiceOption};
pub use edit::{
    EditError, EditOutcome, EditSequence, EditSource, EditToken, ParameterSnapshot, ParameterState,
    PreviewOutcome, Revision,
};
pub use erased::{AnyParameterSnapshot, AnySpec, AnyValue, HostValue};
pub use family::{ParameterFamily, ParameterValue};
pub use live::{
    Capacity, DrainReport, Incarnation, LiveError, LiveItem, PublishOutcome, Sample, SampleWindow,
    SampleWindowEntry, Sequence, SeriesConfig, SeriesConsumer, SeriesEndpoint, Timestamp, WakeFlag,
    series_bridge,
};
pub use normalized::{NormalizedValue, NormalizedValueError};
pub use parameter::{ParameterError, ParameterMapping, ParameterSpec};
pub use parameter_id::{ParameterId, ParameterIdError};
