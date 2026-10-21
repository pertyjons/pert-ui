// `key` returning a borrowed &str.
use pert_ui::{ListItems, prelude::*};
struct Rows(Vec<String>);
impl ListItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<&str> { self.0.get(index).map(String::as_str) }
    fn build(&self, index: usize) -> Option<Element> { Some(Label::new(self.0.get(index)?.clone()).key("row")) }
}
fn main() {}
