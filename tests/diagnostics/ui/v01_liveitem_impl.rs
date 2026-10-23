// Implementing the sealed LiveItem for an own value type.
use pert_ui::LiveItem;
#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct Level(f32);
impl LiveItem for Level {}
fn main() {}
