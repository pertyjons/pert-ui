// A borrowing implementation (TreeSource::new needs 'static).
use pert_ui::{TreeItems, TreeSource, prelude::*};
struct Files<'a>(&'a [String]);
impl TreeItems for Files<'_> {
    fn roots(&self) -> Vec<String> { self.0.to_vec() }
    fn children(&self, _key: &str) -> Vec<String> { vec![] }
    fn build(&self, key: &str) -> Option<Element> { Some(Label::new(key.to_owned()).key(key)) }
}
fn source(rows: &[String]) -> TreeSource { TreeSource::new(Files(rows)) }
fn main() { let rows = vec!["a".to_owned()]; let _ = source(&rows); }
