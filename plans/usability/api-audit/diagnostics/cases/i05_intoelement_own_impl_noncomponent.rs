// An own IntoElement implementation for a type that is not a component.
use pert_ui::prelude::*;
struct Panel;
impl IntoElement for Panel {
    fn key(self, key: impl Into<String>) -> Element { Element::new(key, Label::new("panel")) }
}
fn main() {}
