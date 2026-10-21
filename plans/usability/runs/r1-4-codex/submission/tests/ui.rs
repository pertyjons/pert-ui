//! End-to-end tests: only public runtime input and inspection touch the UI.
use pert_ui::{
    CommandKey, CommandKeyInput, KeyAction, KeyCode, Modifiers, Point, PointerButton, PointerEvent,
    PointerKind, Runtime, SemanticRole, SemanticValue, Semantics, Viewport, WidgetId,
};
use std::time::Duration;

struct App {
    ui: Runtime,
    root: WidgetId,
}

impl App {
    fn new() -> Self {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(900.0, 700.0, 1.0).unwrap())
            .unwrap();
        let root = bookmarks::build(&mut ui).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        Self { ui, root }
    }

    fn find(&self, key: &str) -> WidgetId {
        self.ui
            .find_descendant(&self.root, key)
            .unwrap()
            .unwrap_or_else(|| panic!("missing widget {key}"))
    }

    fn field(&self, key: &str) -> WidgetId {
        let wrapper = self.find(&format!("{key}-field"));
        self.ui.find_descendant(&wrapper, key).unwrap().unwrap()
    }

    fn semantics(&self, id: &WidgetId) -> Semantics {
        let incarnation = self.ui.inspection_id(id).unwrap();
        self.ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.id == incarnation)
            .unwrap()
            .details
            .semantics
            .unwrap()
    }

    fn label(&self, key: &str) -> String {
        self.semantics(&self.find(key)).name.unwrap_or_default()
    }

    fn value(&self, key: &str) -> String {
        match self.semantics(&self.field(key)).value {
            Some(SemanticValue::Text(text)) => text,
            other => panic!("{key}: expected text, got {other:?}"),
        }
    }

    fn key(&mut self, code: KeyCode, key: CommandKey) {
        for action in [KeyAction::Press, KeyAction::Release] {
            self.ui
                .command_key_input(CommandKeyInput::new(code, action, key))
                .unwrap();
        }
        self.ui.flush(Duration::ZERO).unwrap();
    }

    fn fill(&mut self, key: &str, text: &str) {
        self.ui.focus(&self.field(key)).unwrap();
        self.ui.native_modifiers(Modifiers::PRIMARY).unwrap();
        self.key(KeyCode::KeyA, CommandKey::Character('a'));
        self.ui.native_modifiers(Modifiers::default()).unwrap();
        // Backspace also clears a selection when the replacement is empty.
        self.key(KeyCode::Backspace, CommandKey::Named(KeyCode::Backspace));
        self.ui.text_input(text).unwrap();
        self.ui.flush(Duration::ZERO).unwrap();
    }

    fn click(&mut self, id: WidgetId) {
        self.ui.focus(&id).unwrap(); // also reveals scrolled descendants
        let bounds = self.ui.bounds(&id).unwrap().unwrap();
        let position = Point::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        )
        .unwrap();
        for kind in [
            PointerKind::Press(PointerButton::Primary),
            PointerKind::Release(PointerButton::Primary),
        ] {
            self.ui
                .pointer_input(PointerEvent::new(position, kind))
                .unwrap();
        }
        self.ui.flush(Duration::ZERO).unwrap();
    }

    fn add(&mut self, name: &str, url: &str) {
        self.fill("name", name);
        self.fill("url", url);
        self.click(self.find("add"));
    }

    fn rows(&self) -> Vec<(String, String, String)> {
        let list = self.find("list");
        let items = self.ui.find_descendant(&list, "items").unwrap().unwrap();
        self.ui
            .children(&items)
            .unwrap()
            .into_iter()
            .map(|row| {
                let read = |key| {
                    let id = self.ui.find_descendant(&row, key).unwrap().unwrap();
                    self.semantics(&id).name.unwrap_or_default()
                };
                (
                    self.ui.key(&row).unwrap().to_owned(),
                    read("name"),
                    read("url"),
                )
            })
            .collect()
    }

    fn ids(&self) -> Vec<String> {
        self.rows().into_iter().map(|r| r.0).collect()
    }

    fn remove(&mut self, row: &str) {
        let row = self.find(row);
        let button = self.ui.find_descendant(&row, "remove").unwrap().unwrap();
        let semantics = self.semantics(&button);
        assert_eq!(semantics.role, SemanticRole::Button);
        assert_eq!(semantics.name.as_deref(), Some("Remove"));
        self.click(button);
    }
}

#[test]
fn add_appends_and_trims_name() {
    let mut app = App::new();
    app.add("  Example  ", "https://example.com");
    app.add("Plain HTTP", "http://example.org");
    assert_eq!(
        app.rows(),
        vec![
            (
                "1".into(),
                "Rust".into(),
                "https://www.rust-lang.org".into()
            ),
            ("2".into(), "Crates".into(), "https://crates.io".into()),
            ("3".into(), "Example".into(), "https://example.com".into()),
            ("4".into(), "Plain HTTP".into(), "http://example.org".into()),
        ]
    );
}

#[test]
fn refused_add_preserves_fields_and_recovers() {
    let mut app = App::new();
    let initial = app.rows();
    for (name, url, error) in [
        ("", "invalid", "Enter a name."),
        ("   ", "https://example.com", "Enter a name."),
        (
            "Valid name",
            "ftp://example.com",
            "The URL must start with http:// or https://.",
        ),
        (
            "Valid name",
            "",
            "The URL must start with http:// or https://.",
        ),
    ] {
        app.add(name, url);
        assert_eq!(app.rows(), initial);
        assert_eq!(app.label("error"), error);
        assert_eq!(app.value("name"), name);
        assert_eq!(app.value("url"), url);
    }
    app.add("Valid", "https://example.com");
    assert_eq!(app.label("error"), "");
    assert_eq!(app.ids(), ["1", "2", "3"]);
}

#[test]
fn successful_add_clears_both_fields() {
    let mut app = App::new();
    app.add("Example", "https://example.com");
    assert_eq!(app.ids(), ["1", "2", "3"]);
    assert_eq!(app.value("name"), "");
    assert_eq!(app.value("url"), "");
}

#[test]
fn list_shows_rows_and_remove_deletes_only_its_bookmark() {
    let mut app = App::new();
    assert_eq!(
        app.rows(),
        vec![
            (
                "1".into(),
                "Rust".into(),
                "https://www.rust-lang.org".into()
            ),
            ("2".into(), "Crates".into(), "https://crates.io".into()),
        ]
    );
    // Equal names must not turn deletion into deletion by name.
    app.add("Rust", "https://other.example");
    let before = app.rows();
    app.remove("1");
    assert_eq!(app.rows(), before[1..]);
    app.remove("3");
    assert_eq!(app.ids(), ["2"]);
    app.remove("2");
    assert!(app.rows().is_empty());
}

#[test]
fn filter_matches_substrings_ignoring_case_and_surrounding_spaces() {
    let mut app = App::new();
    for (filter, ids) in [
        ("  uS  ", vec!["1"]),
        ("R", vec!["1", "2"]),
        ("aTe", vec!["2"]),
        ("missing", vec![]),
        ("   ", vec!["1", "2"]),
        ("", vec!["1", "2"]),
    ] {
        app.fill("filter", filter);
        assert_eq!(app.ids(), ids, "filter {filter:?}");
    }
}

#[test]
fn status_counts_visible_and_total_bookmarks() {
    let mut app = App::new();
    assert_eq!(app.label("status"), "2 bookmarks");
    app.fill("filter", "rust");
    assert_eq!(app.label("status"), "1 of 2 bookmarks");
    app.add("Other", "http://example.com");
    assert_eq!(app.label("status"), "1 of 3 bookmarks");
    app.fill("filter", "none");
    assert_eq!(app.label("status"), "0 of 3 bookmarks");
    app.fill("filter", "   ");
    assert_eq!(app.label("status"), "3 bookmarks");
    app.remove("3");
    app.remove("2");
    assert_eq!(app.label("status"), "1 bookmarks");
    app.remove("1");
    assert_eq!(app.label("status"), "0 bookmarks");
}

#[test]
fn clear_filter_shortcut_works_wherever_focus_is() {
    let mut app = App::new();
    let mut targets = vec![
        None,
        Some(app.field("filter")),
        Some(app.field("name")),
        Some(app.field("url")),
        Some(app.find("add")),
    ];
    let row = app.find("1");
    targets.push(Some(
        app.ui.find_descendant(&row, "remove").unwrap().unwrap(),
    ));
    for target in targets {
        app.fill("filter", "rust");
        assert_eq!(app.ids(), ["1"]);
        if let Some(id) = target {
            app.ui.focus(&id).unwrap();
        } else {
            app.ui.clear_focus();
        }
        app.ui.native_modifiers(Modifiers::PRIMARY).unwrap();
        app.key(KeyCode::KeyL, CommandKey::Character('l'));
        app.ui.native_modifiers(Modifiers::default()).unwrap();
        assert_eq!(app.value("filter"), "");
        assert_eq!(app.ids(), ["1", "2"]);
        assert_eq!(app.label("status"), "2 bookmarks");
    }
}

#[test]
fn cpu_renderer_captures_main_window_as_png() {
    use pert_ui::{CaptureRequest, CaptureTarget, cpu::CpuRenderer};
    let mut app = App::new();
    app.ui.set_capture_support(true);
    let pending = app
        .ui
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &app.ui.primary_window(),
        )));
    app.ui.flush(Duration::ZERO).unwrap();
    CpuRenderer::new().serve_captures(&mut app.ui);
    let capture = pending.wait().unwrap().value.unwrap();
    assert_eq!((capture.image.width(), capture.image.height()), (900, 700));
    let pixels = capture.image.pixels();
    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]),
        "the capture must contain more than a flat background"
    );
    let png = capture.image.encode_png();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("bookmarks.png"), png).unwrap();
}
