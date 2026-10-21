// Control: an unimported std type gets an import suggestion.
fn main() { let _ = HashMap::<u8, u8>::new(); let _ = Rc::new(1); }
