// `paint()?` in a helper returning RuntimeError.
use pert_ui::prelude::*;
fn frame(ui: &mut Runtime) -> Result<DisplayList, RuntimeError> { Ok(ui.paint()?) }
fn main() { let mut ui = Runtime::new(); let _ = frame(&mut ui); }
