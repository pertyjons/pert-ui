// A plain String passed to set_clipboard.
use pert_ui::host::HostIntegration;
use pert_ui::prelude::*;
fn main() -> Result<(), RuntimeError> { let mut ui = Runtime::new(); ui.set_clipboard(String::new())?; Ok(()) }
