//! The import contract by audience: application code of the representative
//! programs needs only `pert_ui::prelude::*`, their tests add
//! `pert_ui::testing::*`, and a custom component adds `pert_ui::component::*`.
//!
//! A custom component is written in `widget`, which imports nothing else from
//! `pert_ui` than `prelude` and `component`, and driven by a test that imports
//! nothing else than `prelude` and `testing`: their compiling is the evidence.
//! The representative programs' own imports are checked in the pages that
//! carry them.
use pert_ui::{prelude::*, testing::*};
use std::{cell::Cell, rc::Rc, time::Duration};

/// A custom component written from `prelude` and `component` alone.
mod widget {
    use pert_ui::{component::*, prelude::*};
    use std::{cell::Cell, rc::Rc};

    /// A focusable swatch that counts presses and Space keys, overriding the
    /// methods a custom widget commonly implements.
    pub struct Swatch {
        pub presses: Rc<Cell<u32>>,
        pub color: ReadSignal<Color>,
        pub shown: Color,
    }

    impl Component for Swatch {
        fn reconcile(&mut self, next: Self) -> Option<Invalidation> {
            self.color = next.color;
            Some(Invalidation::Paint)
        }
        fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
            self.shown = cx.read(&self.color)?;
            Ok(())
        }
        fn focusable(&self) -> bool {
            true
        }
        fn workspace_state_with_children(
            &self,
            _children: &[ChildInfo],
        ) -> Result<Option<ViewState>, WorkspaceError> {
            self.workspace_state()
        }
        fn content_rows(&self) -> Option<Rc<dyn ContentRows>> {
            None
        }
        fn cursor(&self, _point: Point) -> Option<CursorIcon> {
            Some(CursorIcon::Move)
        }
        fn pointer_input(
            &mut self,
            cx: &mut EventContext,
            event: PointerEvent,
        ) -> Result<InputOutcome, RuntimeError> {
            match event.kind() {
                PointerKind::Press(PointerButton::Primary) => {
                    self.presses.set(self.presses.get() + 1);
                    cx.invalidate(Invalidation::Paint);
                    Ok(InputOutcome::Handled)
                }
                _ => Ok(InputOutcome::Ignored),
            }
        }
        fn keyboard_event(
            &mut self,
            _cx: &mut EventContext,
            event: KeyEvent,
            _keyboard: &KeyboardState,
        ) -> Result<InputOutcome, RuntimeError> {
            if event.code == KeyCode::Space && event.transition == KeyTransition::Pressed {
                self.presses.set(self.presses.get() + 1);
                return Ok(InputOutcome::Handled);
            }
            Ok(InputOutcome::Ignored)
        }
        fn drag_event(
            &mut self,
            _cx: &mut EventContext,
            event: &DragEvent<'_>,
        ) -> Result<DropResponse, RuntimeError> {
            let _: (DragPhase, DragRole, &DragPayload) =
                (event.phase(), event.role(), event.payload());
            let _: (DropOperations, DropOperation) = (event.permitted(), event.preferred());
            Ok(DropResponse::Rejected)
        }
        fn drag_finished(
            &mut self,
            _cx: &mut EventContext,
            _outcome: DragOutcome,
        ) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn text_key_event(
            &mut self,
            _cx: &mut EventContext,
            _input: CommandKeyInput,
            _transition: KeyTransition,
            _keyboard: &KeyboardState,
        ) -> Result<TextKeyOutcome, RuntimeError> {
            Ok(TextKeyOutcome::NotAnEditor)
        }
        fn text_command(
            &mut self,
            _cx: &mut EventContext,
            _command: TextCommand,
        ) -> Result<TextCommandOutcome, RuntimeError> {
            Ok(TextCommandOutcome::NotAnEditor)
        }
        fn text_input_state(&self) -> Option<TextInputState> {
            None
        }
        fn text_preedit(
            &mut self,
            _cx: &mut EventContext,
            _preedit: &TextPreedit,
        ) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn tab_navigation(
            &mut self,
            _cx: &mut EventContext,
            _direction: FocusDirection,
            _transition: KeyTransition,
        ) -> Result<InputOutcome, RuntimeError> {
            Ok(InputOutcome::Ignored)
        }
        fn child_redocked(&mut self, _cx: &mut EventContext, _child: &WidgetId) {}
        fn child_transform(&self) -> ViewTransform {
            ViewTransform::IDENTITY
        }
        fn child_surface(&self, _key: &str) -> Option<SurfaceKind> {
            None
        }
        fn dismiss_surface(
            &mut self,
            _cx: &mut EventContext,
            _reason: SurfaceDismissal,
        ) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn reveal(&mut self, _cx: &mut EventContext, _rect: Rect) -> Result<bool, RuntimeError> {
            Ok(false)
        }
        fn size_limits(&self) -> SizeLimits {
            SizeLimits::default()
        }
        fn measure_limits(&self, _cx: &MeasureContext<'_>) -> Result<SizeLimits, RuntimeError> {
            Ok(self.size_limits())
        }
        fn take_children(&mut self) -> Vec<Element> {
            Vec::new()
        }
        fn measure(
            &self,
            _cx: &MeasureContext<'_>,
            _available: Constraints,
        ) -> Result<Size, RuntimeError> {
            Ok(Size::new(40.0, 20.0)?)
        }
        fn arrange(&self, _cx: &mut ArrangeContext<'_>, _size: Size) -> Result<(), RuntimeError> {
            Ok(())
        }
        fn workspace_state(&self) -> Result<Option<ViewState>, WorkspaceError> {
            Ok(None)
        }
        fn restore_workspace_state(
            &mut self,
            _state: &ViewState,
        ) -> Result<Invalidation, RestoreRejection> {
            Err(RestoreRejection::Unsupported)
        }
        fn inspect(&self) -> ComponentDetails {
            ComponentDetails::default()
        }
        fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
            painter.fill_rect(Rect::new(0.0, 0.0, 40.0, 20.0)?, self.shown);
            Ok(())
        }
    }

    /// Whether the runtime's display list holds a filled rectangle; a
    /// [`DrawError`] does not convert into [`RuntimeError`], so the caller's
    /// boxed error takes it.
    pub fn paints_a_fill(ui: &mut Runtime) -> Result<bool, DrawError> {
        let list: DisplayList = ui.paint()?;
        Ok(list
            .commands()
            .iter()
            .any(|command| matches!(command, DrawCommand::FillRect { .. })))
    }
}

#[test]
fn a_custom_component_is_written_from_component_and_driven_from_testing()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 50.0, 1.0)?)?;
    let presses = Rc::new(Cell::new(0));
    let color = ui.signal(Color::ACCENT);
    let swatch = widget::Swatch {
        presses: Rc::clone(&presses),
        color: color.read_only(),
        shown: Color::SURFACE,
    };
    let root = ui.mount_tree(Element::new("swatch", swatch))?;
    ui.flush(Duration::ZERO)?;

    let press = PointerEvent::new(
        Point::new(5.0, 5.0)?,
        PointerKind::Press(PointerButton::Primary),
    );
    assert_eq!(ui.pointer_input(press)?, InputOutcome::Handled);
    ui.focus(&root)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    assert_eq!(presses.get(), 2);

    ui.flush(Duration::ZERO)?;
    assert!(widget::paints_a_fill(&mut ui)?);
    Ok(())
}

/// The fenced code blocks of the section under `heading` (any level), with
/// rustdoc's hidden lines included.
fn blocks<'a>(page: &'a str, heading: &str) -> Vec<Vec<&'a str>> {
    let mut found = false;
    let mut in_section = false;
    let mut fenced = false;
    let mut blocks: Vec<Vec<&str>> = Vec::new();
    for line in page.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            if fenced && in_section {
                blocks.push(Vec::new());
            }
            continue;
        }
        if fenced {
            if in_section && let Some(block) = blocks.last_mut() {
                block.push(line);
            }
            continue;
        }
        if line.starts_with('#') {
            let title = line.trim_start_matches('#').trim();
            in_section = title == heading;
            found |= in_section;
        }
    }
    assert!(found, "no section headed {heading:?}");
    blocks
}

/// The headings of a page, outside its code blocks.
fn headings(page: &str) -> Vec<&str> {
    let mut fenced = false;
    let mut found = Vec::new();
    for line in page.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        } else if !fenced && line.starts_with('#') {
            found.push(line.trim_start_matches('#').trim());
        }
    }
    found
}

/// Which bytes of `code` are code: false inside comments (line comments and
/// nested block comments) and inside Rust's literal forms, so a brace or
/// `fn main(` there is not counted. The literals are character and byte
/// literals with every escape form, strings, byte strings and C strings
/// with escapes, and raw strings `r`, `br` and `cr` with any number of `#`.
/// A `'` that starts no character literal is a lifetime or a label, which
/// is code.
fn code_bytes(code: &str) -> Vec<bool> {
    let bytes = code.as_bytes();
    let mut is_code = vec![true; bytes.len()];
    let mut at = 0;
    while at < bytes.len() {
        match non_code_end(code, at) {
            Some(end) => {
                let end = end.min(bytes.len());
                is_code[at..end].fill(false);
                at = end;
            }
            None => at += 1,
        }
    }
    is_code
}

/// The end of the comment or literal that starts at byte `at` of `code`, or
/// `None` when code continues there.
fn non_code_end(code: &str, at: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    let byte = |offset: usize| bytes.get(at + offset).copied();
    // A byte string `b"…"`, a C string `c"…"` and a byte literal `b'…'` lex
    // as their string or character literal; the prefix letter is code.
    match bytes[at] {
        b'/' if byte(1) == Some(b'/') => {
            Some(code[at..].find('\n').map_or(bytes.len(), |end| at + end))
        }
        b'/' if byte(1) == Some(b'*') => Some(block_comment_end(bytes, at)),
        b'"' => Some(escaped_string_end(bytes, at + 1)),
        b'\'' => char_literal_end(code, at),
        b'b' | b'c' if byte(1) == Some(b'r') => raw_string_end(code, at + 1),
        b'r' => raw_string_end(code, at),
        _ => None,
    }
}

/// The end of the block comment opening at `at`; block comments nest.
fn block_comment_end(bytes: &[u8], mut at: usize) -> usize {
    let mut depth = 0_usize;
    while at < bytes.len() {
        if bytes[at] == b'/' && bytes.get(at + 1) == Some(&b'*') {
            depth += 1;
            at += 2;
        } else if bytes[at] == b'*' && bytes.get(at + 1) == Some(&b'/') {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    at
}

/// The end of a string, byte string or C string whose body starts at
/// `body`: past the closing quote, skipping each escaped character.
fn escaped_string_end(bytes: &[u8], mut at: usize) -> usize {
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return at + 1,
            _ => at += 1,
        }
    }
    at
}

/// The end of the raw string whose `r` is at `at`, past its closing quote
/// and as many `#` as it opened with, or `None` when no quote follows the
/// hashes (a raw identifier such as `r#type`, or an `r` inside an
/// identifier: Rust reserves an identifier directly followed by `"` or
/// `#"`, so such an `r` is never followed by them in valid code).
fn raw_string_end(code: &str, at: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    let hashes = bytes[at + 1..].iter().take_while(|b| **b == b'#').count();
    if bytes.get(at + 1 + hashes) != Some(&b'"') {
        return None;
    }
    let close = format!("\"{}", "#".repeat(hashes));
    let body = at + 2 + hashes;
    Some(
        code[body..]
            .find(&close)
            .map_or(bytes.len(), |end| body + end + close.len()),
    )
}

/// The end of the character or byte literal whose opening quote is at `at`,
/// past its closing quote, or `None` for a lifetime or label. After the
/// quote, an escape always means a literal (`\'`, `\\`, `\n`, `\x7d`,
/// `\u{7d}` and the rest); otherwise it is a literal exactly when the
/// character after the next one is a quote.
fn char_literal_end(code: &str, at: usize) -> Option<usize> {
    let bytes = code.as_bytes();
    let mut next = at + 1;
    if bytes.get(next) == Some(&b'\\') {
        next = match bytes.get(next + 1)? {
            b'x' => next + 4,
            b'u' => next + code[next..].find('}')? + 1,
            _ => next + 2,
        };
    } else {
        let c = code[next..].chars().next()?;
        next += c.len_utf8();
    }
    (bytes.get(next) == Some(&b'\'')).then_some(next + 1)
}

/// The byte range of `fn main`'s body in `code`, from its opening brace to
/// its matching closing brace, counting only braces in code: not in
/// comments, nested block comments included, or in literals.
fn main_body(code: &str) -> Option<std::ops::Range<usize>> {
    let is_code = code_bytes(code);
    let start = code
        .match_indices("fn main(")
        .map(|(at, _)| at)
        .find(|at| is_code[*at])?;
    let mut depth = 0_usize;
    let mut open = None;
    for (at, byte) in code.bytes().enumerate().skip(start) {
        if !is_code[at] {
            continue;
        }
        match byte {
            b'{' => {
                depth += 1;
                open.get_or_insert(at);
            }
            b'}' if open.is_some() => {
                depth -= 1;
                if depth == 0 {
                    return open.map(|open| open..at);
                }
            }
            _ => {}
        }
    }
    None
}

/// The `pert_ui::…` paths a piece of code names, in order, with whitespace
/// removed, each with whether it is inside `fn main`'s body. A path inside a
/// comment or a literal is not named.
fn paths(code: &str) -> Vec<(String, bool)> {
    let main = main_body(code);
    let is_code = code_bytes(code);
    let mut found = Vec::new();
    for (at, _) in code.match_indices("pert_ui::") {
        if !is_code[at] {
            continue;
        }
        let rest = &code[at..];
        let len = if rest["pert_ui::".len()..].starts_with('{') {
            rest.find('}').map_or(rest.len(), |end| end + 1)
        } else {
            rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':' || c == '*'))
                .unwrap_or(rest.len())
        };
        let path: String = rest[..len].chars().filter(|c| !c.is_whitespace()).collect();
        found.push((path, main.as_ref().is_some_and(|body| body.contains(&at))));
    }
    found
}

/// Asserts that `code`, the representative `name`, names exactly `expected`
/// and, where `test_in_main` is set, imports `testing` inside `fn main` only.
fn check(name: &str, code: &str, expected: &[&str], test_in_main: bool) {
    let found = paths(code);
    let names: Vec<&str> = found.iter().map(|(path, _)| path.as_str()).collect();
    assert_eq!(
        names, expected,
        "{name} must import only its audience's globs"
    );
    if test_in_main {
        for (path, in_main) in &found {
            assert_eq!(
                *in_main,
                path == "pert_ui::testing::*",
                "{name}: the application part imports the prelude, the test in `fn main` adds testing",
            );
        }
    }
}

#[test]
fn representative_programs_import_only_their_globs() {
    const APPLICATION: &[&str] = &["pert_ui::prelude::*"];
    const TEST: &[&str] = &["pert_ui::prelude::*", "pert_ui::testing::*"];
    const COMPONENT: &[&str] = &["pert_ui::{component::*,prelude::*}"];
    let controls = include_str!("../../src/guide/controls.md");
    let layout = include_str!("../../src/guide/layout.md");
    let surfaces = include_str!("../../src/guide/surfaces.md");
    let state = include_str!("../../src/guide/state.md");
    let testing = include_str!("../../src/guide/testing.md");
    let continued = include_str!("../../src/guide/getting-started-continued.md");
    let custom = include_str!("../../src/guide/custom-widgets.md");
    let block = |page, heading, index: usize| -> String {
        blocks(page, heading)
            .get(index)
            .unwrap_or_else(|| panic!("no block {index} under {heading:?}"))
            .join("\n")
    };

    check(
        "R1, first window",
        include_str!("../../examples/getting_started.rs"),
        APPLICATION,
        false,
    );
    let settings = block(controls, "Boolean and discrete controls", 4);
    assert!(settings.contains("\"settings\""), "R2 is the settings form");
    check("R2, settings form", &settings, APPLICATION, false);
    let records = block(layout, "An editable list of records", 0);
    check("R3, editable list of records", &records, TEST, true);
    check(
        "R4, alert dialog",
        &block(surfaces, "Dialogs", 0),
        TEST,
        true,
    );
    check(
        "R4, form dialog",
        &block(surfaces, "Dialogs", 1),
        TEST,
        true,
    );
    let menu = block(surfaces, "Menus", 1);
    assert!(menu.contains("MenuBar::new"), "R5 is the menu bar");
    check("R5, menu bar with a command", &menu, TEST, true);
    let work = block(state, "Worker results and live data", 0);
    assert!(work.contains("cx.spawn"), "R7 starts work from a button");
    check("R7, background work", &work, TEST, true);

    let mut chapter = 0;
    for heading in headings(testing) {
        for (index, code) in blocks(testing, heading).iter().enumerate() {
            let name = format!("testing chapter, {heading}, block {index}");
            check(&name, &code.join("\n"), TEST, false);
            chapter += 1;
        }
    }
    // The opening block sits before the first heading.
    let opening = testing.split("```").nth(1).unwrap_or_default();
    check("testing chapter, opening", opening, TEST, false);
    assert!(chapter > 0, "the testing chapter has examples");

    check(
        "the getting-started indicator",
        &block(continued, "Write a custom widget", 0),
        COMPONENT,
        false,
    );
    check(
        "the custom-widgets Marker",
        &block(custom, "Imports", 0),
        COMPONENT,
        false,
    );
    check(
        "the diagram editor's canvas",
        include_str!("../../examples/diagram_editor/canvas.rs"),
        &["pert_ui::component::*"],
        false,
    );
}

/// A `testing` import after `fn main`'s closing brace is at module level,
/// in the application part, and fails the check.
#[test]
#[should_panic(expected = "the test in `fn main` adds testing")]
fn a_testing_import_after_main_is_not_inside_main() {
    let code = "use pert_ui::prelude::*;\n\
                fn build() {}\n\
                fn main() {\n    let _ = \"{\";\n    let _ = '}';\n    build();\n}\n\
                use pert_ui::testing::*;\n";
    check("a module-level testing import", code, TEST_GLOBS, true);
}

/// The same import inside `fn main`, after nested blocks and braces in
/// literals, passes.
#[test]
fn a_testing_import_inside_main_passes() {
    let code = "use pert_ui::prelude::*;\n\
                fn main() {\n    if true { let _ = \"}\"; }\n    let _ = '{';\n\
                use pert_ui::testing::*;\n}\n";
    check("a testing import in main", code, TEST_GLOBS, true);
}

/// A brace inside a block comment in `fn main` does not open a block, so a
/// module-level `testing` import after `main`, followed by a comment that
/// closes the brace, is still outside `main` and fails the check.
#[test]
#[should_panic(expected = "the test in `fn main` adds testing")]
fn a_brace_in_a_block_comment_does_not_extend_main() {
    let code = "use pert_ui::prelude::*;\n\
                fn main() { /* { */ }\n\
                use pert_ui::testing::*;\n\
                /* } */\n";
    check("a module-level testing import", code, TEST_GLOBS, true);
}

/// A brace inside each of Rust's comment and literal forms, and a lifetime
/// or label beside one, never ends `fn main`: the testing import after it is
/// still inside `main` and passes. Every form is checked, and the failure
/// names the forms whose brace ended `main`.
#[test]
fn a_brace_in_any_comment_or_literal_does_not_end_main() {
    // Literals are written without spaces after commas, so a literal read
    // wrongly would end at the next quote and expose the brace after it.
    let forms: &[(&str, &str)] = &[
        ("a nested block comment", "/* outer /* } */ } */"),
        ("a line comment", "// }\n"),
        ("a character literal", "let _ = '}';"),
        ("an escaped quote", "let _ = ['\\'','}'];"),
        ("an escaped backslash", "let _ = ['\\\\','}'];"),
        ("a newline escape", "let _ = ['\\n','}'];"),
        ("a hex escape", "let _ = ['\\x7d','}'];"),
        ("a unicode escape", "let _ = ['\\u{7d}','}'];"),
        ("a non-ASCII character", "let _ = ['é','}'];"),
        ("a lifetime", "let _ = |x: &'static str| x; let _ = '}';"),
        ("a label", "'outer: loop { break 'outer; } let _ = '}';"),
        ("a byte literal", "let _ = b'}';"),
        ("an escaped byte literal", "let _ = (b'\\'','}');"),
        ("a string", "let _ = \"}\";"),
        ("a string with an escaped quote", "let _ = \"\\\"}\";"),
        (
            "a string ending in a backslash",
            "let _ = (\"}\\\\\", '}');",
        ),
        ("a byte string", "let _ = b\"\\\"}\";"),
        ("a C string", "let _ = c\"\\\"}\";"),
        ("a raw string", "let _ = (r\"\\\", '}');"),
        ("a raw string with hashes", "let _ = (r##\"\"#\\\"##, '}');"),
        ("a raw byte string", "let _ = (br#\"\\\"#, '}');"),
        ("a raw C string", "let _ = (cr#\"\\\"#, '}');"),
        ("a raw identifier", "let r#type = '}'; let _ = r#type;"),
    ];
    let failed: Vec<&str> = forms
        .iter()
        .filter(|(_, form)| {
            let code = format!(
                "use pert_ui::prelude::*;\nfn main() {{\n    {form}\n    use pert_ui::testing::*;\n}}\n"
            );
            std::panic::catch_unwind(|| check("a testing import in main", &code, TEST_GLOBS, true))
                .is_err()
        })
        .map(|(name, _)| *name)
        .collect();
    assert!(failed.is_empty(), "a brace ended `main` in: {failed:?}");
}

/// Codex's fixtures: an escaped quote before a brace literal, and a raw C
/// string holding a brace, each followed by the testing import in `main`.
#[test]
fn an_escaped_quote_and_a_raw_c_string_keep_the_import_in_main() {
    for code in [
        "use pert_ui::prelude::*;\nfn main() { let _ = ['\\'','}']; use pert_ui::testing::*; }\n",
        "use pert_ui::prelude::*;\nfn main() { let _ = cr#\"}\"#; use pert_ui::testing::*; }\n",
    ] {
        check("a testing import in main", code, TEST_GLOBS, true);
    }
}

/// A path quoted in a string is not an import: a program whose only
/// `testing` path is a string literal fails the check.
#[test]
#[should_panic(expected = "must import only its audience's globs")]
fn a_quoted_path_is_not_an_import() {
    let code = "use pert_ui::prelude::*;\nfn main() { let _ = \"pert_ui::testing::*\"; }\n";
    check("a quoted testing path", code, TEST_GLOBS, true);
}

/// A path mentioned in comments is not counted: a program with the right
/// imports and comments naming them, inside and outside `fn main`, passes.
#[test]
fn a_path_in_a_comment_is_not_counted() {
    let code = "use pert_ui::prelude::*;\n\
                // The test adds pert_ui::testing::* in main.\n\
                fn main() {\n    /* pert_ui::prelude::* */\n    use pert_ui::testing::*; // pert_ui::testing::*\n}\n";
    check("an import with commented paths", code, TEST_GLOBS, true);
}

/// The globs a representative test imports.
const TEST_GLOBS: &[&str] = &["pert_ui::prelude::*", "pert_ui::testing::*"];

/// Names a type, so that this file compiles only while the prelude holds it.
#[cfg(feature = "native")]
fn named<T: ?Sized>() {}

/// `App` and everything it reports, errors included.
#[cfg(feature = "native")]
#[test]
fn the_native_prelude_holds_what_app_reports() {
    named::<App>();
    named::<AppError>();
    named::<AppShutdownError>();
    named::<NativeCloseAction>();
    named::<NativeWindowId>();
    named::<WindowError>();
}
