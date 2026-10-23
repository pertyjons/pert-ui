// content_rows returning a type that does not implement ContentRows.
use pert_ui::prelude::*;
use std::rc::Rc;
struct Lines(Vec<String>);
struct Log(Rc<Lines>);
impl Component for Log {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn content_rows(&self) -> Option<Rc<dyn ContentRows>> { Some(self.0.clone()) }
}
fn main() {}
