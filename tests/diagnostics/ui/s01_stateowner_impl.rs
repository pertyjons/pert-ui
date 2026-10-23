// Implementing the sealed StateOwner for an own context.
use pert_ui::prelude::*;
struct MyOwner(Runtime);
impl StateOwner for MyOwner {
    fn signal<T>(&self, value: T) -> Signal<T> { self.0.signal(value) }
    fn ui_settings(&self) -> UiSettings { self.0.ui_settings() }
}
fn main() {}
