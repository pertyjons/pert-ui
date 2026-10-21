// An already keyed Element passed where a component is expected.
use pert_ui::prelude::*;
fn main() { let _ = Element::new("outer", Label::new("x").key("inner")); }
