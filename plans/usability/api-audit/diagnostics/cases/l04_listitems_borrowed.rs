// A ListItems implementation that borrows the rows (ListSource::new needs 'static).
use pert_ui::{ListItems, ListSource, prelude::*};
struct Rows<'a>(&'a [String]);
impl ListItems for Rows<'_> {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn build(&self, index: usize) -> Option<Element> { Some(Label::new(self.0.get(index)?.clone()).key("row")) }
}
fn source(rows: &[String]) -> ListSource { ListSource::new(Rows(rows)) }
fn main() { let rows = vec!["a".to_owned()]; let _ = source(&rows); }
