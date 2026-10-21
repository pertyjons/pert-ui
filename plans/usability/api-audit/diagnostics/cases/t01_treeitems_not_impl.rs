// A map of children passed to TreeSource::new.
use pert_ui::TreeSource;
use std::collections::BTreeMap;
fn main() { let tree: BTreeMap<String, Vec<String>> = BTreeMap::new(); let _ = TreeSource::new(tree); }
