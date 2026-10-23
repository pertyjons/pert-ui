// `.key(..)` on a type that does not implement Component.
use pert_ui::prelude::*;
struct Panel;
fn main() { let _ = Panel.key("panel"); }
