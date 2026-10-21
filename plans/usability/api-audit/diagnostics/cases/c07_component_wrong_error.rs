// `update` returning a DrawError (the paint error type) instead of RuntimeError.
use pert_ui::prelude::*;
struct Panel;
impl Component for Panel {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), DrawError> { Ok(()) }
}
fn main() {}
