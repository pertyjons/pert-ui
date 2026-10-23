// A type without a Component implementation passed to Element::new.
use pert_ui::prelude::*;
struct Panel;
fn main() { let _ = Element::new("panel", Panel); }
