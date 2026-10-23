// F11: AppError::from on an error it does not accept; the message names AppError.
use pert_ui::prelude::*;
fn main() { let _ = AppError::from(std::io::Error::other("x")); }
