// A borrowing clipboard (set_clipboard needs 'static).
use pert_ui::{Clipboard, ClipboardError, prelude::*};
struct Shared<'a>(&'a mut String);
impl Clipboard for Shared<'_> {
    fn read_text(&mut self) -> Result<String, ClipboardError> { Ok(self.0.clone()) }
    fn write_text(&mut self, text: &str) -> Result<(), ClipboardError> { *self.0 = text.to_owned(); Ok(()) }
}
fn main() -> Result<(), RuntimeError> { let mut held = String::new(); let mut ui = Runtime::new(); ui.set_clipboard(Shared(&mut held))?; Ok(()) }
