// `reveal` returning Result<(), RuntimeError> instead of the receipt.
use pert_ui::prelude::*;
struct Lines(Vec<String>);
impl ContentRows for Lines {
    fn len(&self) -> usize { self.0.len() }
    fn key(&self, index: usize) -> Option<String> { Some(index.to_string()) }
    fn text(&self, index: usize) -> Option<String> { self.0.get(index).cloned() }
    fn revision(&self) -> u64 { 0 }
    fn reveal(&self, _key: &str) -> Result<(), RuntimeError> { Ok(()) }
}
fn main() {}
