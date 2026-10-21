// The required `cell` is missing.
use pert_ui::prelude::*;
struct Rows(Vec<[String; 2]>);
impl TableItems for Rows {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { Some(index.to_string()) }
}
fn main() {}
