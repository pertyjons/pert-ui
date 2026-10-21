// Passing a non-owner where `&impl StateOwner` is required.
use pert_ui::{ListController, prelude::*};
struct Model;
fn main() { let _ = ListController::new(&Model); }
