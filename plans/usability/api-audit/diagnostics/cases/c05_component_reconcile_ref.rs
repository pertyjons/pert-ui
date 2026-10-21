// `reconcile` taking `&Self` instead of `Self`.
use pert_ui::prelude::*;
struct Panel(String);
impl Component for Panel {
    fn reconcile(&mut self, next: &Self) -> Option<Invalidation> {
        self.0 = next.0.clone();
        Some(Invalidation::Paint)
    }
    fn update(&mut self, _cx: &mut UpdateContext) -> Result<(), RuntimeError> { Ok(()) }
}
fn main() {}
