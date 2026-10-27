// Implementing the sealed HostIntegration for an own type.
use pert_ui::host::HostIntegration;
struct MyRuntime;
impl HostIntegration for MyRuntime {}
fn main() {}
