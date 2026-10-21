// `update` without the Result return.
use pert_ui::prelude::*;
struct Panel;
impl Component for Panel {
    fn update(&mut self, _cx: &mut UpdateContext) {}
}
fn main() {}
