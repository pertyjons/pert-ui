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

/// The byte range of `fn main`'s body in `code`, from its opening brace to
/// its matching closing brace, skipping braces in comments and in string and
/// character literals.
fn main_body(code: &str) -> Option<std::ops::Range<usize>> {
    let start = code.find("fn main(")?;
    let bytes = code.as_bytes();
    let mut depth = 0_usize;
    let mut open = None;
    let mut at = start;
    while at < bytes.len() {
        match bytes[at] {
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at = code[at..].find('\n').map_or(bytes.len(), |end| at + end);
            }
            b'"' => {
                at += 1;
                while at < bytes.len() && bytes[at] != b'"' {
                    at += if bytes[at] == b'\\' { 2 } else { 1 };
                }
            }
            // A character literal: 'x', '\n' or '{'; a lifetime has no
            // closing quote two or three bytes on.
            b'\'' if bytes.get(at + 2) == Some(&b'\'') => at += 2,
            b'\'' if bytes.get(at + 1) == Some(&b'\\') && bytes.get(at + 3) == Some(&b'\'') => {
                at += 3;
            }
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
        at += 1;
    }
    None
}

/// The `pert_ui::…` paths a piece of code names, in order, with whitespace
/// removed, each with whether it is inside `fn main`'s body.
fn paths(code: &str) -> Vec<(String, bool)> {
    let main = main_body(code);
    let mut found = Vec::new();
    for (at, _) in code.match_indices("pert_ui::") {
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
