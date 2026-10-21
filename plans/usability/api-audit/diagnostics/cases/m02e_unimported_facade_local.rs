// Control: an unimported type defined in the facade crate itself (App, AppError) versus one re-exported from pert_ui_runtime (Label).
fn main() { let _ = App::new; let _: Option<AppError> = None; let _ = Label::new("x"); }
