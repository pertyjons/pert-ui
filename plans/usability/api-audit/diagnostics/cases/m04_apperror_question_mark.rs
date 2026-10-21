// F11: `?` of an io::Error in a function returning AppError.
use pert_ui::prelude::*;
fn load() -> Result<String, AppError> { Ok(std::fs::read_to_string("settings.toml")?) }
fn main() { let _ = load(); }
