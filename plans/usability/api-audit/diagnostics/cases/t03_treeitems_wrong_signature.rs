// `children` taking an owned String key.
use pert_ui::{TreeItems, prelude::*};
struct Files;
impl TreeItems for Files {
    fn roots(&self) -> Vec<String> { vec!["/".into()] }
    fn children(&self, _key: String) -> Vec<String> { vec![] }
    fn build(&self, key: &str) -> Option<Element> { Some(Label::new(key.to_owned()).key(key)) }
}
fn main() {}
