//! Native gallery interactions: named targets on gallery pages driven with
//! injected input, each followed by a page-specific assertion.
//!
//! The scenario is a fixed list of steps. Each step acts (injects input) and
//! then checks what the page shows. The native run (`--interaction-check`)
//! opens the real gallery window through `App`, runs each step's action from a
//! deferred task, waits until a frame has been painted after it and at least
//! one step interval has passed (so debounces and other deadlines elapse in
//! real time), and then runs the step's check. The headless run used by the
//! tests performs the same steps on a `Runtime` without a window, advancing
//! time by the same interval.
//!
//! Input enters at the runtime's native entry points the `App` itself calls:
//! `Runtime::command_key_input` for keys (so command bindings and shortcuts
//! route as on the desktop), `Runtime::pointer_input` for the pointer and
//! `Runtime::text_input` for committed text. The window system's events and
//! the adapter's translation of them are not exercised here; the drag smoke
//! and the input probe cover that path. The run is evidence for exactly the
//! steps in [`steps`], on the platform where it ran.
#[cfg(test)]
use pert_ui::PresentationAction;
use pert_ui::component::*;
use pert_ui::host::HostIntegration;
use pert_ui::prelude::*;
use pert_ui::testing::Driver;
use pert_ui::{AnyParameterSnapshot, ComponentInspection, ParameterId};
use std::{
    cell::{Cell, RefCell},
    io::{self, Write},
    rc::Rc,
    time::{Duration, Instant},
};

/// Time between an action and its check, and between steps. Longer than the
/// gallery's search debounces, so a debounced query has arrived by the check.
const STEP: Duration = Duration::from_millis(400);
/// How long a check may wait for a frame painted after its action.
const FRAME_DEADLINE: Duration = Duration::from_secs(5);
/// Key of the probe root the native run mounts beside the gallery.
const PROBE: &str = "interaction-check";

/// A step failed; the message names what was expected and what was found.
type Outcome = Result<(), String>;

/// The gallery's root and the values one step's action leaves for its check.
pub(crate) struct Scene {
    root: WidgetId,
    /// The knob's parameter base before the knob drag.
    knob_before: f32,
    /// The floating Search panel's origin before its header drag.
    frame_before: Option<(f32, f32)>,
    /// The category and page shown before the run navigates, where Back
    /// returns.
    start: Option<(String, String)>,
}
impl Scene {
    fn new(root: WidgetId) -> Self {
        Self {
            root,
            knob_before: 0.0,
            frame_before: None,
            start: None,
        }
    }
}

/// One interaction: the input it injects and what it then requires.
pub(crate) struct Step {
    /// What the step drives, as recorded in the coverage list.
    pub(crate) name: &'static str,
    act: fn(&mut Runtime, &mut Scene) -> Outcome,
    check: fn(&mut Runtime, &mut Scene) -> Outcome,
}

fn ensure(holds: bool, message: impl FnOnce() -> String) -> Outcome {
    if holds { Ok(()) } else { Err(message()) }
}

fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// The component keyed `key` below (or at) `root`; a missing or repeated key
/// fails the step instead of retargeting it.
fn find(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, String> {
    if ui.key(root).map_err(failed)? == key {
        return Ok(root.clone());
    }
    ui.find_descendant(root, key)
        .map_err(|error| format!("looking up {key}: {error}"))?
        .ok_or_else(|| format!("no component keyed {key}"))
}

/// The mounted gallery page keyed `key`, found through its category's strip.
pub(crate) fn page(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, String> {
    let top = find(ui, root, "gallery-tabs")?;
    for view in ui.children(&top).map_err(failed)? {
        let strip = find(ui, &view, "category-pages")?;
        for page in ui.children(&strip).map_err(failed)? {
            if ui.key(&page).map_err(failed)? == key {
                return Ok(page);
            }
        }
    }
    Err(format!("no gallery page {key}"))
}

/// Shows a page through the reveal contract, as a link or command would.
fn show(ui: &mut Runtime, scene: &Scene, key: &str) -> Outcome {
    let target = page(ui, &scene.root, key)?;
    let outcome = ui.reveal(&target).map_err(failed)?;
    ensure(outcome.is_revealed(), || format!("page {key}: {outcome:?}"))
}

/// Whether a page is the one the gallery shows: its category's strip selects
/// it and the top strip selects that category. Retained bounds are not
/// evidence, since a hidden page keeps its last allocation.
fn shown_page(ui: &Runtime, scene: &Scene, key: &str) -> Result<bool, String> {
    page(ui, &scene.root, key)?;
    Ok(place(ui)?.1 == key)
}

/// The `text` property of every component keyed `key`.
fn texts(ui: &Runtime, key: &str) -> Vec<String> {
    ui.inspect()
        .components
        .into_iter()
        .filter(|component| component.key == key)
        .filter_map(|component| match component.details.properties.get("text") {
            Some(InspectValue::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn expect_texts(ui: &Runtime, key: &str, expected: &str) -> Outcome {
    let found = texts(ui, key);
    ensure(found == [expected], || {
        format!("{key} shows {found:?}, expected {expected:?}")
    })
}

/// The inspected details of the visible component keyed `key`.
fn details(ui: &Runtime, key: &str) -> Result<ComponentDetails, String> {
    ui.inspect()
        .components
        .into_iter()
        .find(|component| component.key == key && component.visible)
        .map(|component| component.details)
        .ok_or_else(|| format!("no visible component keyed {key}"))
}

fn property(ui: &Runtime, key: &str, name: &str) -> Result<Option<InspectValue>, String> {
    Ok(details(ui, key)?.properties.get(name).cloned())
}

fn visible(ui: &Runtime, key: &str) -> bool {
    ui.inspect()
        .components
        .iter()
        .any(|component| component.key == key && component.visible)
}

fn focused_key(ui: &Runtime) -> Result<String, String> {
    let focused = ui.focused().ok_or("nothing has focus")?;
    Ok(ui.key(&focused).map_err(failed)?.to_owned())
}

fn expect_focus(ui: &Runtime, key: &str) -> Outcome {
    let found = focused_key(ui)?;
    ensure(found == key, || {
        format!("{found} has focus, expected {key}")
    })
}

/// The category and page the gallery shows, from the strips' selections.
fn place(ui: &Runtime) -> Result<(String, String), String> {
    let components = ui.inspect().components;
    let selected =
        |component: &ComponentInspection| match component.details.properties.get("selected") {
            Some(InspectValue::Text(key)) => Some(key.clone()),
            _ => None,
        };
    let category = components
        .iter()
        .find(|component| component.key == "gallery-tabs")
        .and_then(selected)
        .ok_or("the top strip selects no category")?;
    let page = components
        .iter()
        .find(|component| {
            component.key == "category-pages"
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.contains(&category))
        })
        .and_then(selected)
        .ok_or("the category's strip selects no page")?;
    Ok((category, page))
}

/// A press and release of `code` on the native keyboard path.
fn key(ui: &mut Runtime, code: KeyCode) -> Outcome {
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(code, action, CommandKey::Named(code)))
            .map_err(failed)?;
    }
    Ok(())
}

/// Presses Escape while tooltip help is shown anywhere in the window, which
/// takes an unmodified Escape before any editor.
fn dismiss_help(ui: &mut Runtime) -> Outcome {
    let shown = |ui: &Runtime| {
        ui.inspect().components.iter().any(|c| {
            c.visible
                && c.details.properties.get("presentation_layer")
                    == Some(&InspectValue::Text("Tooltip".into()))
        })
    };
    for _ in 0..4 {
        if !shown(ui) {
            return Ok(());
        }
        key(ui, KeyCode::Escape)?;
        ui.flush(ui.now()).map_err(failed)?;
    }
    Err("tooltip help stayed shown after Escape".into())
}

/// The modifier state the window system would report while `keys` are held.
fn reported(keys: &[KeyCode]) -> Modifiers {
    let held = |options: &[KeyCode]| keys.iter().any(|key| options.contains(key));
    Modifiers {
        shift: held(&[KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        control: held(&[KeyCode::ControlLeft, KeyCode::ControlRight]),
        alt: held(&[KeyCode::AltLeft, KeyCode::AltRight]),
        super_key: held(&[KeyCode::SuperLeft, KeyCode::SuperRight]),
    }
}

/// `code` pressed while `modifiers` are held, all on the native keyboard path,
/// so command bindings route as they do on the desktop. Like the `App`, the
/// chord also reports the modifier state the window system would send: in a
/// native window the last reported state decides shortcut matching, and no
/// physical key is held there.
fn chord(ui: &mut Runtime, modifiers: &[KeyCode], code: KeyCode, base: CommandKey) -> Outcome {
    for &modifier in modifiers {
        ui.command_key_input(CommandKeyInput::new(
            modifier,
            KeyAction::Press,
            CommandKey::Named(modifier),
        ))
        .map_err(failed)?;
    }
    ui.native_modifiers(reported(modifiers)).map_err(failed)?;
    for action in [KeyAction::Press, KeyAction::Release] {
        ui.command_key_input(CommandKeyInput::new(code, action, base))
            .map_err(failed)?;
    }
    for &modifier in modifiers.iter().rev() {
        ui.command_key_input(CommandKeyInput::new(
            modifier,
            KeyAction::Release,
            CommandKey::Named(modifier),
        ))
        .map_err(failed)?;
    }
    ui.native_modifiers(Modifiers::default()).map_err(failed)
}

fn type_text(ui: &mut Runtime, text: &str) -> Outcome {
    ui.text_input(text).map_err(failed)
}

/// The center of a presented component, in window-logical coordinates.
fn center(ui: &Runtime, id: &WidgetId) -> Result<Point, String> {
    let [x, y, width, height] = ui
        .bounds(id)
        .map_err(failed)?
        .ok_or("the target is not presented")?
        .values();
    Point::new(x + width / 2.0, y + height / 2.0).map_err(failed)
}

fn pointer(ui: &mut Runtime, at: Point, kind: PointerKind) -> Outcome {
    ui.pointer_input(PointerEvent::new(at, kind))
        .map(|_| ())
        .map_err(failed)
}

/// Moves to the target's center, then presses and releases the primary button.
fn click(ui: &mut Runtime, id: &WidgetId) -> Outcome {
    let at = center(ui, id)?;
    pointer(ui, at, PointerKind::Move)?;
    pointer(ui, at, PointerKind::Press(PointerButton::Primary))?;
    pointer(ui, at, PointerKind::Release(PointerButton::Primary))
}

/// A part of the menu bar's history control.
fn history_part(ui: &Runtime, scene: &Scene, key: &str) -> Result<WidgetId, String> {
    let control = find(ui, &scene.root, "gallery-history")?;
    find(ui, &control, key)
}

/// The base of the knob's parameter, the gallery's "amount", found by its
/// identity rather than its place in the inspected list.
fn knob_parameter_base(ui: &Runtime) -> Result<f32, String> {
    let id = ParameterId::new(2, 1, "amount").map_err(failed)?;
    let inspection = ui.inspect();
    let parameter = inspection
        .parameters
        .iter()
        .find(|parameter| parameter.id == id)
        .ok_or("the gallery inspects no amount parameter")?;
    match &parameter.parameter {
        AnyParameterSnapshot::Continuous { state, .. } => Ok(state.base),
        other => Err(format!("the amount parameter is {:?}", other.family())),
    }
}

/// Whether the grid on the Collections page shows the item keyed `key`.
fn grid_item(ui: &Runtime, scene: &Scene, key: &str) -> Result<bool, String> {
    let filter = find(ui, &scene.root, "grid-filter")?;
    let Some(item) = ui.find_descendant(&filter, key).map_err(failed)? else {
        return Ok(false);
    };
    Ok(ui.bounds(&item).map_err(failed)?.is_some()
        && ui
            .inspect()
            .components
            .iter()
            .any(|component| component.key == key && component.visible))
}

/// How many items the Collections grid shows that "item 999" does not match.
fn nonmatching_grid_items(ui: &Runtime) -> usize {
    ui.inspect()
        .components
        .iter()
        .filter(|component| {
            component.visible
                && component
                    .path
                    .as_ref()
                    .is_some_and(|path| path.iter().any(|segment| segment == "grid-filter"))
        })
        .filter_map(|component| component.key.strip_prefix("swatch-")?.parse::<usize>().ok())
        .filter(|index| !(index + 1).to_string().contains("999"))
        .count()
}

fn ok(_: &mut Runtime, _: &mut Scene) -> Outcome {
    Ok(())
}

/// Every step in order. The list is the check's coverage.
pub(crate) fn steps() -> Vec<Step> {
    vec![
        Step {
            name: "brand mark: a pointer click opens About",
            act: |ui, scene| {
                scene.start = Some(place(ui)?);
                let mark = find(ui, &scene.root, "brand-mark")?;
                click(ui, &mark)
            },
            check: |ui, _| ensure(visible(ui, "about"), || "About did not open".into()),
        },
        Step {
            name: "About: Escape closes it",
            act: |ui, _| key(ui, KeyCode::Escape),
            check: |ui, _| ensure(!visible(ui, "about"), || "About stayed open".into()),
        },
        Step {
            name: "command palette: its shortcut opens it",
            act: |ui, _| {
                chord(
                    ui,
                    &[KeyCode::primary(), KeyCode::ShiftLeft],
                    KeyCode::KeyP,
                    CommandKey::Character('p'),
                )
            },
            check: |ui, _| {
                let open = property(ui, "command-palette", "open")?;
                ensure(open == Some(InspectValue::Bool(true)), || {
                    format!("the palette reports open = {open:?}")
                })
            },
        },
        Step {
            name: "command palette: typing filters to the intended page",
            act: |ui, _| type_text(ui, "go to chips"),
            check: |ui, _| {
                let picker = ui
                    .inspect()
                    .components
                    .into_iter()
                    .find(|component| {
                        component.visible
                            && component.key == "content"
                            && component.details.properties.contains_key("query")
                    })
                    .ok_or("the palette's filtered content is missing")?;
                let query = picker.details.properties.get("query");
                let results = picker.details.properties.get("results");
                ensure(
                    query == Some(&InspectValue::Text("go to chips".into()))
                        && results == Some(&InspectValue::Number(1.0)),
                    || format!("the palette has query {query:?} and {results:?} results"),
                )
            },
        },
        Step {
            name: "command palette: Enter goes to the filtered page",
            act: |ui, _| key(ui, KeyCode::Enter),
            check: |ui, scene| {
                ensure(shown_page(ui, scene, "chips")?, || {
                    "the chips page is not shown".into()
                })?;
                expect_texts(ui, "status", "Opened Chips & breadcrumbs.")
            },
        },
        Step {
            name: "chip: Delete removes the focused tag and focus moves on",
            act: |ui, scene| {
                let editor = find(ui, &scene.root, "tag-editor")?;
                let chip = find(ui, &editor, "Review")?;
                ui.focus(&chip).map_err(failed)?;
                key(ui, KeyCode::Delete)
            },
            check: |ui, scene| {
                expect_texts(ui, "status", "Removed tag Review")?;
                expect_focus(ui, "Urgent")?;
                let editor = find(ui, &scene.root, "tag-editor")?;
                ensure(
                    ui.find_descendant(&editor, "Review")
                        .map_err(failed)?
                        .is_none(),
                    || "the removed tag is still mounted".into(),
                )
            },
        },
        Step {
            name: "menu-bar history: Back returns to the first page",
            act: |ui, scene| {
                let back = history_part(ui, scene, "back")?;
                click(ui, &back)
            },
            check: |ui, scene| {
                let shown = place(ui)?;
                ensure(Some(&shown) == scene.start.as_ref(), || {
                    format!("back showed {shown:?}, expected {:?}", scene.start)
                })
            },
        },
        Step {
            name: "menu-bar history: Forward returns to the chips page",
            act: |ui, scene| {
                let forward = history_part(ui, scene, "forward")?;
                click(ui, &forward)
            },
            check: |ui, _| {
                let shown = place(ui)?;
                ensure(
                    shown == ("controls-category".into(), "chips".into()),
                    || format!("forward showed {shown:?}"),
                )
            },
        },
        Step {
            name: "search field: a typed query filters after its debounce",
            act: |ui, scene| {
                show(ui, scene, "editing")?;
                let field = find(ui, &scene.root, "search-field")?;
                ui.focus(&field).map_err(failed)?;
                ui.flush(ui.now()).map_err(failed)?;
                let before = texts(ui, "search-results");
                type_text(ui, "at")?;
                // Before the debounce elapses the list still shows every match.
                ui.flush(ui.now()).map_err(failed)?;
                let early = texts(ui, "search-results");
                ensure(early == before, || {
                    format!("the query applied before its debounce: {early:?}")
                })
            },
            check: |ui, _| expect_texts(ui, "search-results", "Matches: Gate, Saturation"),
        },
        Step {
            name: "search field: Down and Enter choose without moving focus",
            act: |ui, _| {
                key(ui, KeyCode::ArrowDown)?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, _| {
                expect_texts(ui, "search-chosen", "Chosen: Saturation")?;
                expect_focus(ui, "search-field")
            },
        },
        Step {
            name: "search field: Escape clears the query",
            act: |ui, _| {
                // Shown help takes the first Escape, so dismiss any first.
                dismiss_help(ui)?;
                key(ui, KeyCode::Escape)
            },
            check: |ui, _| {
                let found = texts(ui, "search-results");
                ensure(
                    found.len() == 1 && found[0].starts_with("Matches: Compressor"),
                    || format!("search-results shows {found:?}"),
                )
            },
        },
        Step {
            name: "password: typed text stays concealed and IME is withheld",
            act: |ui, scene| {
                let password = find(ui, &scene.root, "password-input")?;
                ui.focus(&password).map_err(failed)?;
                type_text(ui, "correct horse")
            },
            check: |ui, _| {
                let details = details(ui, "password-input")?;
                ensure(!details.properties.contains_key("text"), || {
                    "inspection exposes the password text".into()
                })?;
                // The safe length shows that the typed text arrived.
                let length = details.properties.get("length");
                ensure(length == Some(&InspectValue::Number(13.0)), || {
                    format!("the concealed field holds length {length:?}, expected 13")
                })?;
                let semantics = details.semantics.ok_or("the field reports no semantics")?;
                ensure(semantics.protected && semantics.value.is_none(), || {
                    format!("the field's semantics expose {:?}", semantics.value)
                })?;
                ensure(ui.text_input_state().is_none(), || {
                    "a concealed field asks for IME".into()
                })
            },
        },
        Step {
            name: "password: Alt+F8 reveals it",
            act: |ui, _| {
                chord(
                    ui,
                    &[KeyCode::AltLeft],
                    KeyCode::F8,
                    CommandKey::Named(KeyCode::F8),
                )
            },
            check: |ui, _| {
                let revealed = property(ui, "password-input", "revealed")?;
                ensure(revealed == Some(InspectValue::Bool(true)), || {
                    format!("revealed = {revealed:?}")
                })
            },
        },
        Step {
            name: "password: focus loss conceals it again; an ordinary field asks for IME",
            act: |ui, scene| {
                let user = find(ui, &scene.root, "user-input")?;
                ui.focus(&user).map_err(failed)
            },
            check: |ui, _| {
                let revealed = property(ui, "password-input", "revealed")?;
                ensure(revealed == Some(InspectValue::Bool(false)), || {
                    format!("revealed = {revealed:?} after focus loss")
                })?;
                ensure(ui.text_input_state().is_some(), || {
                    "an ordinary text field does not ask for IME".into()
                })
            },
        },
        Step {
            name: "content search: its shortcut opens the picker",
            act: |ui, _| {
                ui.clear_focus();
                chord(
                    ui,
                    &[KeyCode::primary(), KeyCode::ShiftLeft],
                    KeyCode::KeyF,
                    CommandKey::Character('f'),
                )
            },
            check: |ui, _| {
                let status = property(ui, "gallery-search", "status")?;
                ensure(status == Some(InspectValue::Text("Open".into())), || {
                    format!("the search reports {status:?}")
                })
            },
        },
        Step {
            name: "content search: Enter jumps to a hidden button and focuses it",
            act: |ui, _| {
                type_text(ui, "output level")?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, scene| {
                expect_focus(ui, "output-level")?;
                ensure(shown_page(ui, scene, "navigation")?, || {
                    "the navigation page is not shown".into()
                })
            },
        },
        Step {
            name: "knob: an upward pointer drag raises its parameter",
            act: |ui, scene| {
                show(ui, scene, "input")?;
                let knob = find(ui, &scene.root, "knob")?;
                let outcome = ui.reveal(&knob).map_err(failed)?;
                ensure(outcome.is_revealed(), || format!("knob: {outcome:?}"))?;
                ui.flush(ui.now()).map_err(failed)?;
                scene.knob_before = knob_parameter_base(ui)?;
                let start = center(ui, &knob)?;
                pointer(ui, start, PointerKind::Move)?;
                pointer(ui, start, PointerKind::Press(PointerButton::Primary))?;
                for rise in [10.0, 25.0, 40.0] {
                    let at = Point::new(start.x(), start.y() - rise).map_err(failed)?;
                    pointer(ui, at, PointerKind::Move)?;
                }
                let end = Point::new(start.x(), start.y() - 40.0).map_err(failed)?;
                pointer(ui, end, PointerKind::Release(PointerButton::Primary))
            },
            check: |ui, scene| {
                let after = knob_parameter_base(ui)?;
                ensure(after > scene.knob_before, || {
                    format!(
                        "the drag left the parameter at {after} (was {})",
                        scene.knob_before
                    )
                })
            },
        },
        Step {
            name: "collapsible section: Enter on its header collapses it",
            act: |ui, scene| {
                show(ui, scene, "sections")?;
                let demo = page(ui, &scene.root, "sections")?;
                let header = find(ui, &demo, "remembered")?;
                ui.focus(&header).map_err(failed)?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, _| {
                ensure(!visible(ui, "section-note"), || {
                    "the section stayed open".into()
                })
            },
        },
        Step {
            name: "collapsible section: Space expands it again",
            act: |ui, _| key(ui, KeyCode::Space),
            check: |ui, _| {
                ensure(visible(ui, "section-note"), || {
                    "the section stayed closed".into()
                })
            },
        },
        Step {
            name: "tool windows: a stripe button expands Search and focuses it",
            act: |ui, scene| {
                show(ui, scene, "tool-windows")?;
                let demo = page(ui, &scene.root, "tool-windows")?;
                let stage = find(ui, &demo, "open-stage")?;
                let stripe = find(ui, &stage, "stripe-left")?;
                ui.focus(&stripe).map_err(failed)?;
                key(ui, KeyCode::ArrowDown)?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, _| {
                ensure(visible(ui, "open-search-body"), || {
                    "Search did not expand".into()
                })?;
                expect_focus(ui, "open-search-body")
            },
        },
        Step {
            name: "tool windows: Escape returns focus to the main content",
            act: |ui, _| key(ui, KeyCode::Escape),
            check: |ui, _| expect_focus(ui, "open-main-button"),
        },
        Step {
            name: "tool windows: the options menu floats Search over the layout",
            act: |ui, scene| {
                open_options(ui, scene)?;
                // The last items are Dock, Float and Open in window; Dock is
                // the current mode and so disabled.
                key(ui, KeyCode::End)?;
                key(ui, KeyCode::ArrowUp)?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, _| last_change(ui, "open-search floated"),
        },
        Step {
            name: "tool windows: dragging the floating frame's header moves it",
            act: |ui, scene| {
                let panel = search_panel(ui, scene)?;
                let [x, y, _, _] = ui
                    .bounds(&panel)
                    .map_err(failed)?
                    .ok_or("the floating panel has no bounds")?
                    .values();
                scene.frame_before = Some((x, y));
                // The header is the panel's top strip; its title is a drag area.
                let from = Point::new(x + 24.0, y + 8.0).map_err(failed)?;
                let to = Point::new(x + 64.0, y + 38.0).map_err(failed)?;
                pointer(ui, from, PointerKind::Move)?;
                pointer(ui, from, PointerKind::Press(PointerButton::Primary))?;
                pointer(ui, to, PointerKind::Move)?;
                pointer(ui, to, PointerKind::Release(PointerButton::Primary))
            },
            check: |ui, scene| {
                let panel = search_panel(ui, scene)?;
                let [x, y, _, _] = ui
                    .bounds(&panel)
                    .map_err(failed)?
                    .ok_or("the floating panel has no bounds")?
                    .values();
                let (before_x, before_y) = scene.frame_before.ok_or("no origin recorded")?;
                ensure(x > before_x && y > before_y, || {
                    format!("the frame stayed at {x},{y} from {before_x},{before_y}")
                })
            },
        },
        Step {
            name: "tool windows: dragging the floating frame's corner grip resizes it",
            act: |ui, scene| {
                let panel = search_panel(ui, scene)?;
                let [x, y, w, h] = ui
                    .bounds(&panel)
                    .map_err(failed)?
                    .ok_or("the floating panel has no bounds")?
                    .values();
                scene.frame_before = Some((w, h));
                let from = Point::new(x + w - 3.0, y + h - 3.0).map_err(failed)?;
                let to = Point::new(x + w + 37.0, y + h + 27.0).map_err(failed)?;
                pointer(ui, from, PointerKind::Move)?;
                pointer(ui, from, PointerKind::Press(PointerButton::Primary))?;
                pointer(ui, to, PointerKind::Move)?;
                pointer(ui, to, PointerKind::Release(PointerButton::Primary))
            },
            check: |ui, scene| {
                let panel = search_panel(ui, scene)?;
                let [_, _, w, h] = ui
                    .bounds(&panel)
                    .map_err(failed)?
                    .ok_or("the floating panel has no bounds")?
                    .values();
                let (before_w, before_h) = scene.frame_before.ok_or("no size recorded")?;
                ensure(w > before_w && h > before_h, || {
                    format!("the frame is {w}×{h}, was {before_w}×{before_h}")
                })
            },
        },
        Step {
            name: "tool windows: Open in window moves Search to its own native window",
            act: |ui, scene| {
                open_options(ui, scene)?;
                key(ui, KeyCode::End)?;
                key(ui, KeyCode::Enter)
            },
            check: |ui, _| {
                last_change(ui, "open-search set to open in its own window")?;
                let windows = ui.native_windows().len();
                ensure(windows == 2, || {
                    format!("{windows} native windows are open")
                })
            },
        },
        Step {
            name: "tool windows: closing that window returns Search to its stripe",
            act: |ui, _| {
                let primary = ui.primary_window();
                let window = ui
                    .native_windows()
                    .into_iter()
                    .find(|window| *window != primary)
                    .ok_or("Search has no window of its own")?;
                ui.request_close(&window).map(|_| ()).map_err(failed)
            },
            check: |ui, _| {
                let windows = ui.native_windows().len();
                ensure(windows == 1, || {
                    format!("{windows} native windows are open")
                })?;
                // The window returns to its stripe, collapsed.
                last_change(ui, "open-search collapsed")
            },
        },
        Step {
            name: "collection filter: a typed query hides non-matching items",
            act: |ui, scene| {
                show(ui, scene, "collections")?;
                let filter = find(ui, &scene.root, "grid-filter")?;
                let field = find(ui, &filter, "filter")?;
                ui.focus(&field).map_err(failed)?;
                ensure(grid_item(ui, scene, "swatch-0")?, || {
                    "the first item is not shown before filtering".into()
                })?;
                type_text(ui, "item 999")
            },
            check: |ui, scene| {
                let shown = property(ui, "grid-filter", "shown")?;
                let total = property(ui, "grid-filter", "total")?;
                ensure(shown == Some(InspectValue::Number(11.0)), || {
                    format!("the filter shows {shown:?} of {total:?}")
                })?;
                // "Item 1" does not match, so it is gone from the grid.
                ensure(!grid_item(ui, scene, "swatch-0")?, || {
                    "a non-matching item is still shown".into()
                })
            },
        },
        Step {
            name: "collection filter: Escape clears it and every item returns",
            act: |ui, _| key(ui, KeyCode::Escape),
            check: |ui, _| {
                let shown = property(ui, "grid-filter", "shown")?;
                let total = property(ui, "grid-filter", "total")?;
                ensure(shown.is_some() && shown == total, || {
                    format!("the filter shows {shown:?} of {total:?}")
                })?;
                // Items the filter hid, around "Item 999" that stays in view,
                // are shown again.
                ensure(nonmatching_grid_items(ui) > 0, || {
                    "no non-matching item returned".into()
                })
            },
        },
        Step {
            name: "tab titles: item pointer bounds switch the category",
            act: |ui, scene| {
                show(ui, scene, "tool-windows")?;
                let tabs = find(ui, &scene.root, "gallery-tabs")?;
                let mut driver = Driver::new(ui);
                ensure(
                    !driver.semantics(&tabs).map_err(failed)?.items[1].selected,
                    || "the target category was already selected before the click".into(),
                )?;
                driver.click_item(&tabs, 1).map_err(failed)
            },
            check: |ui, scene| {
                let tabs = find(ui, &scene.root, "gallery-tabs")?;
                ensure(
                    Driver::new(ui).semantics(&tabs).map_err(failed)?.items[1].selected,
                    || "the Controls category title was not selected".into(),
                )
            },
        },
        Step {
            name: "Select: named control and option bounds choose the second value",
            act: |ui, scene| {
                show(ui, scene, "controls")?;
                let page = page(ui, &scene.root, "controls")?;
                let select = find(ui, &page, "dropdown")?;
                let mut driver = Driver::new(ui);
                let control = driver.exposed_part(&select, "control").map_err(failed)?;
                let options = driver.exposed_part(&select, "options").map_err(failed)?;
                driver.click(&control).map_err(failed)?;
                driver.click_item(&options, 2).map_err(failed)
            },
            check: |ui, scene| {
                let page = page(ui, &scene.root, "controls")?;
                let select = find(ui, &page, "dropdown")?;
                let driver = Driver::new(ui);
                let control = driver.exposed_part(&select, "control").map_err(failed)?;
                let semantics = driver.semantics(&control).map_err(failed)?;
                ensure(
                    matches!(
                        semantics.value,
                        Some(SemanticValue::Choice {
                            selected: Some(2),
                            ..
                        })
                    ),
                    || format!("the dropdown has value {:?}", semantics.value),
                )
            },
        },
        Step {
            name: "menu: named Help title and item bounds open About",
            act: |ui, scene| {
                let mut driver = Driver::new(ui);
                let title = driver
                    .exposed_part(&scene.root, "title/help")
                    .map_err(failed)?;
                let items = driver.exposed_part(&title, "items").map_err(failed)?;
                driver.click(&title).map_err(failed)?;
                let about = driver
                    .semantics(&items)
                    .map_err(failed)?
                    .items
                    .iter()
                    .position(|item| item.name.as_deref() == Some("About"))
                    .ok_or("the Help menu has no About item")?;
                driver.click_item(&items, about).map_err(failed)
            },
            check: |ui, _| {
                ensure(visible(ui, "about"), || {
                    "About did not open from the menu row".into()
                })
            },
        },
        Step {
            name: "menu About: Escape closes the dialog",
            act: |ui, _| key(ui, KeyCode::Escape),
            check: |ui, _| ensure(!visible(ui, "about"), || "About stayed open".into()),
        },
        Step {
            name: "menu keyboard: parked title and duplicate motion keep File selected",
            act: |ui, scene| {
                show(ui, scene, "menus")?;
                let page = page(ui, &scene.root, "menus")?;
                let bar = find(ui, &page, "menu-bar")?;
                let document = find(ui, &page, "document-focus")?;
                ui.focus(&document).map_err(failed)?;
                ui.flush(ui.now()).map_err(failed)?;
                let (file, view) = {
                    let driver = Driver::new(ui);
                    (
                        driver.exposed_part(&bar, "title/file").map_err(failed)?,
                        driver.exposed_part(&bar, "title/view").map_err(failed)?,
                    )
                };
                let at = center(ui, &view)?;
                let before = texts(ui, "menu-bar-status");
                pointer(ui, at, PointerKind::Move)?;
                key(ui, KeyCode::F10)?;
                key(ui, KeyCode::ArrowDown)?;
                ui.flush(ui.now()).map_err(failed)?;
                // Native adapters may report an unchanged position again.
                pointer(ui, at, PointerKind::Move)?;
                let driver = Driver::new(ui);
                ensure(
                    driver.semantics(&file).map_err(failed)?.expanded == Some(true)
                        && driver.semantics(&view).map_err(failed)?.expanded == Some(false),
                    || "a stationary View pointer switched the keyboard-opened File menu".into(),
                )?;
                key(ui, KeyCode::Enter)?;
                ui.flush(ui.now()).map_err(failed)?;
                ensure(texts(ui, "menu-bar-status") != before, || {
                    "the keyboard command changed no status".into()
                })
            },
            check: |ui, _| {
                ensure(
                    texts(ui, "menu-bar-status")
                        .iter()
                        .any(|text| text.starts_with("Created sketch ")),
                    || {
                        format!(
                            "wrong command after parked title: {:?}",
                            texts(ui, "menu-bar-status")
                        )
                    },
                )
            },
        },
        Step {
            name: "menu keyboard: parked row keeps the first command through reveal refresh",
            act: |ui, scene| {
                let page = page(ui, &scene.root, "menus")?;
                let bar = find(ui, &page, "menu-bar")?;
                let document = find(ui, &page, "document-focus")?;
                ui.focus(&document).map_err(failed)?;
                // Calibrate from the real open row geometry, then close it.
                key(ui, KeyCode::F10)?;
                key(ui, KeyCode::ArrowDown)?;
                ui.flush(ui.now()).map_err(failed)?;
                let (items, at) = {
                    let driver = Driver::new(ui);
                    let file = driver.exposed_part(&bar, "title/file").map_err(failed)?;
                    let items = driver.exposed_part(&file, "items").map_err(failed)?;
                    let semantics = driver.semantics(&items).map_err(failed)?;
                    let [x, y, width, height] = semantics
                        .items
                        .get(1)
                        .and_then(|item| item.bounds)
                        .ok_or("the second menu row has no visible window bounds")?;
                    let at = Point::new(x + width / 2.0, y + height / 2.0).map_err(failed)?;
                    (items, at)
                };
                key(ui, KeyCode::Escape)?;
                ui.focus(&document).map_err(failed)?;
                ui.flush(ui.now()).map_err(failed)?;
                let before = texts(ui, "menu-bar-status");
                pointer(ui, at, PointerKind::Move)?;
                key(ui, KeyCode::F10)?;
                key(ui, KeyCode::ArrowDown)?;
                ui.flush(ui.now()).map_err(failed)?;
                ensure(
                    Driver::new(ui)
                        .semantics(&items)
                        .map_err(failed)?
                        .items
                        .first()
                        .is_some_and(|item| item.selected),
                    || "the parked row replaced the keyboard's first command".into(),
                )?;
                key(ui, KeyCode::Enter)?;
                ui.flush(ui.now()).map_err(failed)?;
                ensure(texts(ui, "menu-bar-status") != before, || {
                    "the parked-row command changed no status".into()
                })
            },
            check: |ui, _| {
                ensure(
                    texts(ui, "menu-bar-status")
                        .iter()
                        .any(|text| text.starts_with("Created sketch ")),
                    || {
                        format!(
                            "wrong command after parked row: {:?}",
                            texts(ui, "menu-bar-status")
                        )
                    },
                )
            },
        },
        Step {
            name: "settle: with focus and pointer gone nothing stays scheduled",
            act: |ui, _| {
                ui.clear_focus();
                let corner = Point::new(1.0, 1.0).map_err(failed)?;
                pointer(ui, corner, PointerKind::Leave)
            },
            check: |ui, _| {
                // Compose what is pending once, then nothing may ask for more:
                // no deadline and no further composition.
                ui.paint().map_err(failed)?;
                ui.flush(ui.now()).map_err(failed)?;
                let deadline = ui.next_deadline();
                ensure(deadline.is_none(), || {
                    format!("still scheduled: {deadline:?}")
                })?;
                ensure(!ui.needs_paint(), || {
                    "the gallery keeps asking for another frame".into()
                })
            },
        },
        Step {
            name: "end",
            act: ok,
            check: ok,
        },
    ]
}

/// Fails on any validation issue the runtime reports.
fn validated(ui: &Runtime, step: &str) -> Outcome {
    let issues = &ui.validation_report().issues;
    ensure(issues.is_empty(), || {
        format!("{step}: validation issues {issues:?}")
    })
}

/// The Search panel of the open layout.
fn search_panel(ui: &Runtime, scene: &Scene) -> Result<WidgetId, String> {
    let demo = page(ui, &scene.root, "tool-windows")?;
    let stage = find(ui, &demo, "open-stage")?;
    find(ui, &stage, "open-search")
}

/// Focuses the Search panel's options button in the open layout and opens
/// its menu.
fn open_options(ui: &mut Runtime, scene: &Scene) -> Outcome {
    let panel = search_panel(ui, scene)?;
    let options = find(ui, &panel, "options")?;
    // The trigger is the help tooltip around the options button.
    let trigger = find(ui, &options, "trigger")?;
    let button = find(ui, &trigger, "button")?;
    ui.focus(&button).map_err(failed)?;
    key(ui, KeyCode::Enter)?;
    // The menu surface is admitted by the next flush; its keys follow then.
    ui.flush(ui.now()).map(|_| ()).map_err(failed)
}

/// Requires the page's last-change readout to start with `prefix`.
fn last_change(ui: &Runtime, prefix: &str) -> Outcome {
    let shown = texts(ui, "last-change");
    ensure(
        shown
            .iter()
            .any(|text| text.starts_with(&format!("open: {prefix}"))),
        || format!("the last change reads {shown:?}"),
    )
}

/// Serves the presentation requests a headless run has no native host for,
/// as the `App` would: detaching into a new window and closing one.
#[cfg(test)]
fn serve_presentation(ui: &mut Runtime) -> Outcome {
    let primary = ui.primary_window();
    for request in ui.take_presentation_requests().map_err(failed)? {
        let status = match request.action() {
            PresentationAction::Detach { view, options } => {
                let prepared = ui.prepare_detach(view, options.clone()).map_err(failed)?;
                PresentationStatus::Presented(ui.commit_detach(prepared).map_err(failed)?)
            }
            PresentationAction::Close(window) if *window != primary => {
                ui.select_window(window).map_err(failed)?;
                ui.close_detached_window().map_err(failed)?
            }
            _ => continue,
        };
        request.complete(status).map_err(failed)?;
    }
    ui.select_window(&primary).map_err(failed)
}

/// Runs the steps without a window, advancing time by [`STEP`] between each
/// action and its check. Returns the steps that passed.
#[cfg(test)]
pub(crate) fn run_headless() -> Result<Vec<&'static str>, String> {
    let mut ui = Runtime::new();
    let root = crate::gallery::Gallery::new()
        .map_err(failed)?
        .mount(&mut ui)
        .map_err(failed)?;
    ui.set_viewport(Viewport::new(960.0, 640.0, 1.0).map_err(failed)?)
        .map_err(failed)?;
    ui.flush(ui.now()).map_err(failed)?;
    let mut scene = Scene::new(root);
    let mut passed = Vec::new();
    for step in steps() {
        (step.act)(&mut ui, &mut scene).map_err(|error| format!("{}: {error}", step.name))?;
        ui.flush(ui.now())
            .map_err(|error| format!("{}: {error}", step.name))?;
        serve_presentation(&mut ui).map_err(|error| format!("{}: {error}", step.name))?;
        let later = ui.now() + STEP;
        ui.flush(later)
            .map_err(|error| format!("{}: {error}", step.name))?;
        (step.check)(&mut ui, &mut scene).map_err(|error| format!("{}: {error}", step.name))?;
        validated(&ui, step.name)?;
        passed.push(step.name);
    }
    Ok(passed)
}

/// Where the native run is: the next step to act or check.
struct Progress {
    steps: Vec<Step>,
    next: usize,
    /// Set after the step `next` acted: the paint count then and when.
    acted: Option<(u64, Instant)>,
    scene: Option<Scene>,
    passed: Vec<&'static str>,
    failure: Option<String>,
}

/// The native run's probe: a root beside the gallery whose update deadline paces the
/// steps and whose paints prove a frame after each action.
struct Probe {
    progress: Rc<RefCell<Progress>>,
    painted: Rc<Cell<u64>>,
    id: Rc<RefCell<Option<WidgetId>>>,
    /// The notifier requests event-phase delivery when an update deadline arrives.
    notifier: Rc<RefCell<Option<Notifier>>>,
    /// Whether the first tick has been armed; it waits for the first presentation.
    started: bool,
    waiting_until: Option<Duration>,
}
impl Component for Probe {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.notifier.borrow().is_none() {
            *self.notifier.borrow_mut() = Some(cx.notifier());
        }
        if self.progress.borrow().failure.is_some() {
            self.notifier
                .borrow()
                .as_ref()
                .ok_or(RuntimeError::Unmounted)?
                .request();
        } else if let Some(until) = self.waiting_until {
            if cx.now() < until {
                cx.wake_after(until - cx.now())?;
            } else {
                self.waiting_until = None;
                self.notifier
                    .borrow()
                    .as_ref()
                    .ok_or(RuntimeError::Unmounted)?
                    .request();
            }
        } else {
            let delay = if self.started {
                STEP
            } else {
                Duration::from_secs(1)
            };
            self.started = true;
            self.waiting_until = Some(cx.now().checked_add(delay).ok_or(RuntimeError::Exhausted)?);
            cx.wake_after(delay)?;
        }
        Ok(())
    }
    fn paint(&self, _: &mut Painter) -> Result<(), DrawError> {
        self.painted.set(self.painted.get() + 1);
        Ok(())
    }
    // Laid over the gallery as a second root: pointer input passes through to
    // the gallery, and the overlap is intended.
    fn hit_test(&self, _: Point) -> bool {
        false
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.validation.overlap = OverlapPolicy::Allow;
        details
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let (progress, painted, id) =
            (self.progress.clone(), self.painted.clone(), self.id.clone());
        let _task = cx.defer_while_shown(move |ui| advance(ui, &progress, &painted, &id))?;
        Ok(())
    }
    fn visibility_changed(&mut self, visible: bool) {
        if !visible {
            let reason = "the interaction window was hidden during the check";
            eprintln!("Interaction interrupted: {reason}");
            self.progress.borrow_mut().failure = Some(reason.into());
        }
    }
}

/// Acts or checks the next step, then asks the probe to arm its next wake, or
/// closes the window when the run passed or failed.
fn advance(
    ui: &mut Runtime,
    progress: &Rc<RefCell<Progress>>,
    painted: &Rc<Cell<u64>>,
    id: &Rc<RefCell<Option<WidgetId>>>,
) -> Result<(), RuntimeError> {
    let probe = id.borrow().clone().ok_or(RuntimeError::Unmounted)?;
    let finished = {
        let mut state = progress.borrow_mut();
        let state = &mut *state;
        if state.failure.is_none() && state.next < state.steps.len() {
            let result = step_once(ui, state, painted, &probe);
            if let Err(message) = result {
                state.failure = Some(message);
            }
        }
        state.failure.is_some() || state.next >= state.steps.len()
    };
    if finished {
        let _close = ui.request_close(&ui.primary_window())?;
        return Ok(());
    }
    ui.invalidate(&probe, Invalidation::Paint)?;
    Ok(())
}

fn step_once(
    ui: &mut Runtime,
    state: &mut Progress,
    painted: &Cell<u64>,
    probe: &WidgetId,
) -> Outcome {
    let Some(scene) = state.scene.as_mut() else {
        return Err("the gallery root is missing".into());
    };
    let step = &state.steps[state.next];
    match state.acted {
        None => {
            (step.act)(ui, scene).map_err(|error| format!("{}: {error}", step.name))?;
            ui.invalidate(probe, Invalidation::Paint).map_err(failed)?;
            state.acted = Some((painted.get(), Instant::now()));
        }
        Some((mark, at)) => {
            if painted.get() <= mark {
                return ensure(at.elapsed() < FRAME_DEADLINE, || {
                    format!("{}: no frame was painted after the action", step.name)
                });
            }
            (step.check)(ui, scene).map_err(|error| format!("{}: {error}", step.name))?;
            validated(ui, step.name)?;
            state.passed.push(step.name);
            state.acted = None;
            state.next += 1;
        }
    }
    Ok(())
}

/// Opens the gallery natively and runs every step against it.
/// Runs the check in a native window; `software` presents CPU frames instead
/// of using the GPU.
pub(super) fn run(software: bool) -> Result<(), Box<dyn std::error::Error>> {
    let progress = Rc::new(RefCell::new(Progress {
        steps: steps(),
        next: 0,
        acted: None,
        scene: None,
        passed: Vec::new(),
        failure: None,
    }));
    let painted = Rc::new(Cell::new(0));
    let id = Rc::new(RefCell::new(None));
    let probe = Probe {
        progress: progress.clone(),
        painted: painted.clone(),
        id: id.clone(),
        notifier: Rc::new(RefCell::new(None)),
        started: false,
        waiting_until: None,
    };
    let gallery = crate::gallery::Gallery::new()?;
    let producer = gallery.producer();
    let scene = progress.clone();
    let app = App::new("pert-ui — Gallery interaction check", move |ui| {
        let root = gallery.mount(ui)?;
        scene.borrow_mut().scene = Some(Scene::new(root));
        let probe = ui.mount_tree(Element::new(PROBE, probe))?;
        *id.borrow_mut() = Some(probe);
        Ok(())
    })
    .on_error(|_, _, _| ErrorAction::Exit);
    let app = if software {
        app.software_rendering()
    } else {
        app
    };
    let result = app.run();
    let joined = producer.join();
    result?;
    joined?;
    let state = progress.borrow();
    let mut out = io::stdout().lock();
    for name in &state.passed {
        writeln!(out, "Interaction passed: {name}")?;
    }
    if let Some(failure) = &state.failure {
        return Err(io::Error::other(format!("interaction check failed: {failure}")).into());
    }
    if state.next < state.steps.len() {
        return Err(io::Error::other(format!(
            "the window closed after {} of {} steps",
            state.next,
            state.steps.len()
        ))
        .into());
    }
    writeln!(
        out,
        "Native interactions: {} steps passed on the real gallery window; input entered at the runtime's native entry points.",
        state.passed.len()
    )?;
    Ok(())
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn hiding_the_window_reports_interruption_before_any_more_steps() {
        let progress = Rc::new(RefCell::new(Progress {
            steps: Vec::new(),
            next: 0,
            acted: None,
            scene: None,
            passed: Vec::new(),
            failure: None,
        }));
        let id = Rc::new(RefCell::new(None));
        let probe = Probe {
            progress: progress.clone(),
            painted: Rc::new(Cell::new(0)),
            id: id.clone(),
            notifier: Rc::new(RefCell::new(None)),
            started: false,
            waiting_until: None,
        };
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(320.0, 240.0, 1.0).unwrap())
            .unwrap();
        *id.borrow_mut() = Some(ui.mount_tree(Element::new(PROBE, probe)).unwrap());
        ui.flush(Duration::ZERO).unwrap();
        assert!(ui.next_deadline().is_some());
        ui.set_window_visible(false).unwrap();
        assert!(ui.next_deadline().is_none());
        ui.set_window_visible(true).unwrap();
        ui.flush(Duration::ZERO).unwrap();
        assert_eq!(
            progress.borrow().failure.as_deref(),
            Some("the interaction window was hidden during the check")
        );
    }
}
