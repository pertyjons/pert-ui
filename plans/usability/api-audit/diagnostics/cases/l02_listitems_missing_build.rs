// The required `build` is missing.
use pert_ui::{ListItems, prelude::*};
struct Rows(Vec<String>);
impl ListItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
}
fn main() {}
