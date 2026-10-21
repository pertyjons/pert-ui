// `.key(..)` with only the widget imported, not IntoElement.
use pert_ui::Label;
fn main() { let _ = Label::new("x").key("title"); }
