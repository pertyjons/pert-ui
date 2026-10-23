// An Rc of an implementation passed to ListSource::new.
use pert_ui::{ListItems, ListSource, prelude::*};
use std::rc::Rc;
struct Rows(Vec<String>);
impl ListItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn build(&self, index: usize) -> Option<Element> { Some(Label::new(self.0.get(index)?.clone()).key("row")) }
}
fn main() { let shared = Rc::new(Rows(vec![])); let _ = ListSource::new(shared.clone()); }
