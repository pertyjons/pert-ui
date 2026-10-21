// `read_text` returning io::Error instead of ClipboardError.
use pert_ui::{Clipboard, ClipboardError};
struct Memory(String);
impl Clipboard for Memory {
    fn read_text(&mut self) -> Result<String, std::io::Error> { Ok(self.0.clone()) }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> { self.0 = text.to_owned(); Ok(()) }
}
fn main() {}
