// Implementing the sealed ParameterValue for an own type.
use pert_ui::{AnyParameterSnapshot, AnySpec, AnyValue, ParameterFamily, ParameterSnapshot, ParameterSpec, ParameterValue};
#[derive(Clone, PartialEq, Debug)]
struct Mode;
impl ParameterValue for Mode {
    const FAMILY: ParameterFamily = ParameterFamily::Choice;
    fn into_any(self) -> AnyValue { todo!() }
    fn from_any(_: AnyValue) -> Result<Self, AnyValue> { todo!() }
    fn spec_into_any(_: ParameterSpec<Self>) -> AnySpec { todo!() }
    fn spec_from_any(_: AnySpec) -> Result<ParameterSpec<Self>, AnySpec> { todo!() }
    fn snapshot_into_any(_: ParameterSpec<Self>, _: ParameterSnapshot<Self>) -> AnyParameterSnapshot { todo!() }
}
fn main() {}
