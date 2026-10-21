// A component that borrows (Component: Any, so 'static).
use pert_ui::prelude::*;
struct Title<'a>(&'a str);
impl<'a> Component for Title<'a> {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
}
fn main() {}
