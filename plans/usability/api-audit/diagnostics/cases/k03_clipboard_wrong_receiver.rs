// `read_text(&self)` instead of `&mut self`.
use pert_ui::{Clipboard, ClipboardError};
struct Memory(String);
impl Clipboard for Memory {
    fn read_text(&self) -> Result<String, ClipboardError> { Ok(self.0.clone()) }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> { self.0 = text.to_owned(); Ok(()) }
}
fn main() {}
