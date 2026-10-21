// `write_text` is missing.
use pert_ui::{Clipboard, ClipboardError};
struct Memory(String);
impl Clipboard for Memory {
    fn read_text(&mut self) -> Result<String, ClipboardError> { Ok(self.0.clone()) }
}
fn main() {}
