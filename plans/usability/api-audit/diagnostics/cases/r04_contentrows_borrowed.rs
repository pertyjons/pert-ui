// content_rows returning rows that borrow from the component (Rc<dyn ContentRows> is 'static).
use pert_ui::{ListRevealReceipt, prelude::*};
use std::rc::Rc;
struct Lines<'a>(&'a [String]);
impl ContentRows for Lines<'_> {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { Some(index.to_string()) }
    fn text(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn revision(&self) -> u64 { 0 }
    fn reveal(&self, _key: &str) -> Result<ListRevealReceipt, RuntimeError> { unimplemented!() }
}
struct Log(Vec<String>);
impl Component for Log {
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
    fn content_rows(&self) -> Option<Rc<dyn ContentRows>> { Some(Rc::new(Lines(&self.0))) }
}
fn main() {}
