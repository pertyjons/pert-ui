// A one-element tuple where `ReadSignal::combine` takes 2 to 12 bindings.
use pert_ui::prelude::*;
fn main() {
    let ui = Runtime::new();
    let count = ui.signal(1_u32);
    let _ = ReadSignal::combine((&count,), |(n,)| *n);
}
