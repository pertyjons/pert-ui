//! A headless driver for the bookmarks window: it mounts `bookmarks::build`
//! into a `Runtime` with a viewport, and acts on it only through the inputs a
//! native host delivers (pointer, committed text, keys) and reads it only
//! through inspection.
#![allow(dead_code)]

use pert_ui::prelude::*;
use pert_ui::{
    CommandKey, CommandKeyInput, KeyAction, KeyCode, Modifiers, PointerButton, PointerEvent,
    PointerKind, SemanticValue, Semantics,
};
use std::time::Duration;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Bookmarks {
    pub ui: Runtime,
    pub root: WidgetId,
}

impl Bookmarks {
    /// The main window at 800 × 900 logical pixels, laid out once.
    pub fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(800.0, 900.0, 1.0)?)?;
        let root = bookmarks::build(&mut ui)?;
        ui.flush(Duration::ZERO)?;
        Ok(Self { ui, root })
    }

    /// The component at an addressable path below the `bookmarks` root,
    /// as inspection reports it, e.g. `["content", "add"]`.
    pub fn id(&self, path: &[&str]) -> WidgetId {
        self.ui
            .find(&self.root, path)
            .expect("lookup")
            .unwrap_or_else(|| panic!("nothing mounted at {path:?}"))
    }

    pub fn exists(&self, path: &[&str]) -> bool {
        self.ui.find(&self.root, path).expect("lookup").is_some()
    }

    pub fn semantics(&self, path: &[&str]) -> Semantics {
        let id = self.ui.inspection_id(&self.id(path)).expect("inspection id");
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == id)
            .and_then(|c| c.details.semantics)
            .unwrap_or_else(|| panic!("{path:?} reports no semantics"))
    }

    /// The text a label shows (its semantic name; absent when empty).
    pub fn label(&self, path: &[&str]) -> String {
        self.semantics(path).name.unwrap_or_default()
    }

    /// The text a text field holds.
    pub fn field(&self, path: &[&str]) -> String {
        match self.semantics(path).value {
            Some(SemanticValue::Text(text)) => text,
            other => panic!("{path:?} has no text value: {other:?}"),
        }
    }

    /// Ids of the rows the list shows, in order.
    pub fn rows(&self) -> Vec<String> {
        let items = self.id(&["content", "list", "items"]);
        self.ui
            .children(&items)
            .expect("children")
            .iter()
            .map(|row| self.ui.key(row).expect("key").to_owned())
            .collect()
    }

    /// (name, url) of every shown row, in order.
    pub fn shown(&self) -> Vec<(String, String)> {
        self.rows()
            .iter()
            .map(|row| {
                let at = |part| self.label(&["content", "list", "items", row, part]);
                (at("name"), at("url"))
            })
            .collect()
    }

    /// A primary-button click at the center of the component, as a mouse does.
    pub fn click(&mut self, path: &[&str]) {
        let id = self.id(path);
        self.ui.reveal(&id).expect("reveal");
        self.ui.flush(Duration::ZERO).expect("flush");
        let bounds = self.ui.bounds(&id).expect("bounds").expect("laid out");
        let center = Point::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .expect("point");
        for kind in [
            PointerKind::Move,
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(center, kind))
                .expect("pointer input");
        }
        self.ui.flush(Duration::ZERO).expect("flush");
    }

    /// One key press and release through the native routing, with `modifiers` held.
    pub fn chord(&mut self, modifiers: Modifiers, physical: KeyCode, key: char) {
        self.press(modifiers, physical, CommandKey::Character(key));
    }

    fn press(&mut self, modifiers: Modifiers, physical: KeyCode, key: CommandKey) {
        self.ui.native_modifiers(modifiers).expect("modifiers");
        for action in [KeyAction::Press, KeyAction::Release] {
            self.ui
                .command_key_input(CommandKeyInput::new(physical, action, key))
                .expect("key input");
        }
        self.ui.native_modifiers(Modifiers::default()).expect("modifiers");
        self.ui.flush(Duration::ZERO).expect("flush");
    }

    /// Clicks into a text field, selects and deletes all its text, and types
    /// `text`.
    pub fn type_into(&mut self, path: &[&str], text: &str) {
        self.click(path);
        self.chord(Modifiers::PRIMARY, KeyCode::KeyA, 'a');
        let backspace = KeyCode::Backspace;
        self.press(Modifiers::default(), backspace, CommandKey::Named(backspace));
        assert_eq!(self.field(path), "", "{path:?} not emptied");
        self.ui.text_input(text).expect("text input");
        self.ui.flush(Duration::ZERO).expect("flush");
    }

    pub fn add(&mut self, name: &str, url: &str) {
        self.type_into(NAME, name);
        self.type_into(URL, url);
        self.click(ADD);
    }
}

pub const NAME: &[&str] = &["content", "form", "name-field", "name"];
pub const URL: &[&str] = &["content", "form", "url-field", "url"];
pub const ADD: &[&str] = &["content", "add"];
pub const ERROR: &[&str] = &["content", "error"];
pub const FILTER: &[&str] = &["content", "filter-field", "filter"];
pub const STATUS: &[&str] = &["content", "status"];

pub fn pair(name: &str, url: &str) -> (String, String) {
    (name.to_owned(), url.to_owned())
}
