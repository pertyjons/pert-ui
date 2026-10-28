// An Each whose build returns a String; the message names the two kinds of item.
use pert_ui::prelude::*;
fn main() {
    let ui = Runtime::new();
    let tasks = ui.signal(vec!["write".to_owned()]);
    let _ = Each::new(&tasks, Clone::clone, |_, task| task.clone());
}
