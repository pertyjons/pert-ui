// A borrowing implementation (TableSource::new needs 'static).
use pert_ui::prelude::*;
struct Rows<'a>(&'a [String]);
impl TableItems for Rows<'_> {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn cell(&self, index: usize, _column: &str) -> Option<Element> { Some(Label::new(self.0.get(index)?.clone()).key("c")) }
}
fn source(rows: &[String]) -> TableSource { TableSource::new(Rows(rows)) }
fn main() { let rows = vec!["a".to_owned()]; let _ = source(&rows); }
