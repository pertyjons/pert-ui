// F11: AppError::from on an unsupported error names DrawError.
use pert_ui::prelude::*;
fn main() { let _ = AppError::from(std::io::Error::other("x")); }
