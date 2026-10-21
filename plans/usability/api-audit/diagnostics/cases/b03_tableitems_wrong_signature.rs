// `cell` taking a column index instead of the column key.
use pert_ui::prelude::*;
struct Rows(Vec<[String; 2]>);
impl TableItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { Some(index.to_string()) }
    fn cell(&self, index: usize, column: usize) -> Option<Element> {
        Some(Label::new(self.0.get(index)?.get(column)?.clone()).key("cell"))
    }
}
fn main() {}
