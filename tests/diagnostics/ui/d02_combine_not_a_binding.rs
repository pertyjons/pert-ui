// A fixed value where `ReadSignal::combine` takes a binding.
use pert_ui::prelude::*;
fn main() {
    let ui = Runtime::new();
    let count = ui.signal(1_u32);
    let _ = ReadSignal::combine((&count, 3_u32), |(n, m)| n + m);
}
