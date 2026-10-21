// `revision` and `reveal` are missing.
use pert_ui::prelude::*;
struct Lines(Vec<String>);
impl ContentRows for Lines {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { Some(index.to_string()) }
    fn text(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
}
fn main() {}
