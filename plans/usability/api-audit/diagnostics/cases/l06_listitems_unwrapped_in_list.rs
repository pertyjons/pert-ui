// An implementation passed straight to VirtualList::new, which wants a bound ListSource.
use pert_ui::{ListItems, VirtualList, prelude::*};
struct Rows(Vec<String>);
impl ListItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn build(&self, index: usize) -> Option<Element> { Some(Label::new(self.0.get(index)?.clone()).key("row")) }
}
fn main() { let _ = VirtualList::new(Rows(vec![]), 24.0); }
