// Modifiers is not in the prelude: rustc suggests Notifier.
use pert_ui::prelude::*;
fn main() { let _alt = Modifiers { alt: true, ..Modifiers::default() }; }
