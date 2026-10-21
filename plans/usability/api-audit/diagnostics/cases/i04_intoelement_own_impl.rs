// An own IntoElement implementation for a component.
use pert_ui::prelude::*;
struct Panel;
impl Component for Panel {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
}
impl IntoElement for Panel {}
fn main() {}
