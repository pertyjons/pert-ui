// The required `build` is missing.
use pert_ui::TreeItems;
struct Files;
impl TreeItems for Files {
    fn roots(&self) -> Vec<String> { vec!["/".into()] }
    fn children(&self, _key: &str) -> Vec<String> { vec![] }
}
fn main() {}
