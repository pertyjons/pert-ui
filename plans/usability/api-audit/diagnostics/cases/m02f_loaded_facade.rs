// Control: as m02e, but the facade crate is loaded by a `use pert_ui::Runtime;`.
use pert_ui::Runtime;
fn main() { let _ = Runtime::new(); let _ = App::new; let _: Option<AppError> = None; let _ = Label::new("x"); }
