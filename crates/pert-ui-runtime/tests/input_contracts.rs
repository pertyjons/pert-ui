//! Public contracts of text commands, key routing, key-path lookup and
//! observable presentation settings.
use pert_ui_runtime::{
    DebugOverlay, InputOutcome, InspectionQuery, KeyAction, KeyCode, KeyEvent, KeyTransition,
    KeyboardState, Modifiers, MotionPreference, RuntimeError, Selector, TextCommand,
    TextCommandOutcome, UiZoom, Viewport, WidgetId, prelude::*,
};
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Counts updates, observed keys and text commands.
#[derive(Default)]
struct Log {
    updates: Cell<usize>,
    keys: Cell<usize>,
    commands: Cell<usize>,
}

/// A focusable leaf that only observes keys through `key_input`.
struct Leaf(Rc<Log>);
impl Component for Leaf {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.updates.set(self.0.updates.get() + 1);
        Ok(())
    }
    fn focusable(&self) -> bool {
        true
    }
    fn key_input(
        &mut self,
        _: &mut EventContext,
        _: KeyEvent,
        _: &KeyboardState,
    ) -> Result<(), RuntimeError> {
        self.0.keys.set(self.0.keys.get() + 1);
        Ok(())
    }
    fn text_command(
        &mut self,
        _: &mut EventContext,
        _: TextCommand,
    ) -> Result<TextCommandOutcome, RuntimeError> {
        self.0.commands.set(self.0.commands.get() + 1);
        Ok(TextCommandOutcome::Handled)
    }
}

/// A container that handles Escape presses and ignores everything else.
struct Parent(Rc<Log>);
impl Component for Parent {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.0.updates.set(self.0.updates.get() + 1);
        Ok(())
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        _: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        self.0.keys.set(self.0.keys.get() + 1);
        Ok(
            if event.code == KeyCode::Escape && event.transition == KeyTransition::Pressed {
                InputOutcome::Handled
            } else {
                InputOutcome::Ignored
            },
        )
    }
}

/// The mounted root with the parent's and the leaf's logs.
type Pair = (WidgetId, Rc<Log>, Rc<Log>);

fn mount_pair(ui: &mut Runtime) -> Result<Pair, Box<dyn std::error::Error>> {
    let parent = Rc::new(Log::default());
    let leaf = Rc::new(Log::default());
    let root = ui.mount_tree(
        Element::new("parent", Parent(parent.clone()))
            .children([Element::new("leaf", Leaf(leaf.clone()))]),
    )?;
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.flush(Duration::ZERO)?;
    Ok((root, parent, leaf))
}

fn leaf_of(ui: &Runtime, root: &WidgetId) -> Result<WidgetId, Box<dyn std::error::Error>> {
    Ok(ui.find(root, &["leaf"])?.ok_or("leaf is mounted")?)
}

#[test]
fn text_command_rejects_a_hidden_target_before_the_editor_sees_it() -> TestResult {
    let mut ui = Runtime::new();
    let (root, _, leaf) = mount_pair(&mut ui)?;
    let target = leaf_of(&ui, &root)?;
    assert_eq!(
        ui.text_command(&target, TextCommand::Cut)?,
        TextCommandOutcome::Handled
    );
    ui.set_visible(&target, false)?;
    assert_eq!(
        ui.text_command(&target, TextCommand::Cut),
        Err(RuntimeError::Hidden)
    );
    // A hidden ancestor hides the target as well.
    ui.set_visible(&target, true)?;
    ui.set_visible(&root, false)?;
    assert_eq!(
        ui.text_command(&target, TextCommand::Paste),
        Err(RuntimeError::Hidden)
    );
    assert_eq!(leaf.commands.get(), 1);
    Ok(())
}

#[test]
fn the_default_keyboard_event_lets_observed_keys_bubble() -> TestResult {
    let mut ui = Runtime::new();
    let (root, parent, leaf) = mount_pair(&mut ui)?;
    ui.focus(&leaf_of(&ui, &root)?)?;
    ui.key_input(KeyCode::Escape, KeyAction::Press)?;
    // The leaf observed Escape through `key_input`, and the key still reached
    // the parent, which a swallowing default would have hidden.
    assert_eq!(leaf.keys.get(), 1);
    assert_eq!(parent.keys.get(), 1);
    Ok(())
}

#[test]
fn key_events_do_not_repaint_the_focus_chain_by_themselves() -> TestResult {
    let mut ui = Runtime::new();
    let (root, parent, leaf) = mount_pair(&mut ui)?;
    ui.focus(&leaf_of(&ui, &root)?)?;
    ui.flush(Duration::ZERO)?;
    let (parent_before, leaf_before) = (parent.updates.get(), leaf.updates.get());
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
    for _ in 0..10 {
        ui.key_input(KeyCode::ShiftLeft, KeyAction::Repeat)?;
    }
    ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(leaf.keys.get(), 12);
    assert_eq!(parent.keys.get(), 12);
    assert_eq!(parent.updates.get(), parent_before);
    assert_eq!(leaf.updates.get(), leaf_before);
    Ok(())
}

#[test]
fn an_orphan_repeat_never_establishes_a_press() -> TestResult {
    let mut ui = Runtime::new();
    let (root, parent, leaf) = mount_pair(&mut ui)?;
    ui.focus(&leaf_of(&ui, &root)?)?;
    let repeat = ui.key_input(KeyCode::KeyA, KeyAction::Repeat)?;
    assert_eq!(repeat.transition, KeyTransition::Repeated);
    assert!(!ui.keyboard().is_pressed(KeyCode::KeyA));
    assert_eq!((leaf.keys.get(), parent.keys.get()), (0, 0));
    let release = ui.key_input(KeyCode::KeyA, KeyAction::Release)?;
    assert_eq!(release.transition, KeyTransition::UnmatchedRelease);
    // A repeat of a key that is held is delivered as before.
    ui.key_input(KeyCode::KeyA, KeyAction::Press)?;
    let held = ui.key_input(KeyCode::KeyA, KeyAction::Repeat)?;
    assert_eq!(held.transition, KeyTransition::Repeated);
    assert!(ui.keyboard().is_pressed(KeyCode::KeyA));
    Ok(())
}

#[test]
fn modifiers_follow_the_pressed_set_until_the_host_reports_a_snapshot() -> TestResult {
    let mut ui = Runtime::new();
    ui.key_input(KeyCode::primary(), KeyAction::Press)?;
    assert!(ui.keyboard().modifiers().primary());
    ui.native_modifiers(Modifiers::default())?;
    // The operating system's snapshot wins over the pressed set.
    assert!(!ui.keyboard().modifiers().primary());
    Ok(())
}

#[test]
fn find_resolves_key_paths_under_a_root() -> TestResult {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([Element::new(
            "group",
            Row::new([Element::new("first", Label::new("First"))]),
        )]),
    ))?;
    let first = ui.find(&root, &["group", "first"])?.ok_or("mounted")?;
    assert_eq!(ui.key(&first)?, "first");
    assert_eq!(ui.find(&root, &[])?, Some(root.clone()));
    assert_eq!(ui.find(&root, &["first"])?, None);
    assert_eq!(ui.find(&root, &["group", "first", "deeper"])?, None);
    // A key path is relative to its root.
    let group = ui.find(&root, &["group"])?.ok_or("mounted")?;
    assert_eq!(ui.find(&group, &["first"])?, Some(first.clone()));
    // Hidden components are still found.
    ui.set_visible(&first, false)?;
    assert_eq!(ui.find(&root, &["group", "first"])?, Some(first));

    let other = Runtime::new();
    assert_eq!(
        other.find(&root, &["group"]),
        Err(RuntimeError::ForeignRuntime)
    );
    ui.unmount(&root)?;
    assert_eq!(ui.find(&root, &[]), Err(RuntimeError::Unmounted));
    Ok(())
}

#[test]
fn find_descendant_skips_built_in_parts_and_refuses_ambiguity() -> TestResult {
    let mut ui = Runtime::new();
    // A group puts its content below parts it keys itself, which `find`
    // would have to name.
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            Element::new(
                "left",
                Group::titled(
                    "Settings",
                    Element::new(
                        "form",
                        Row::new([
                            Element::new("apply", Label::new("Apply")),
                            Element::new("root", Label::new("Same key as the ancestor")),
                        ]),
                    ),
                ),
            ),
            Element::new(
                "right",
                Row::new([Element::new("apply", Label::new("Apply"))]),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let left = ui.find(&root, &["left"])?.ok_or("mounted")?;
    let form = ui.find_descendant(&root, "form")?.ok_or("mounted")?;
    assert_eq!(ui.find(&left, &["form"])?, None, "the group adds levels");
    assert_eq!(ui.key(&form)?, "form");
    // Keys repeat across siblings' subtrees: the lookup refuses to choose.
    assert_eq!(
        ui.find_descendant(&root, "apply"),
        Err(RuntimeError::AmbiguousKey)
    );
    let apply = ui.find_descendant(&left, "apply")?.ok_or("mounted")?;
    assert_eq!(ui.find(&form, &["apply"])?, Some(apply.clone()));
    // The ancestor is not its own descendant.
    let same = ui.find_descendant(&root, "root")?.ok_or("mounted")?;
    assert_ne!(same, root);
    assert_eq!(ui.find_descendant(&apply, "apply")?, None);
    assert_eq!(ui.find_descendant(&root, "missing")?, None);
    // Hidden components are still found.
    ui.set_visible(&form, false)?;
    assert_eq!(ui.find_descendant(&left, "apply")?, Some(apply));

    let other = Runtime::new();
    assert_eq!(
        other.find_descendant(&root, "form"),
        Err(RuntimeError::ForeignRuntime)
    );
    ui.unmount(&root)?;
    assert_eq!(
        ui.find_descendant(&root, "form"),
        Err(RuntimeError::Unmounted)
    );
    Ok(())
}

#[test]
fn find_address_names_addressable_segments_and_refuses_ambiguity() -> TestResult {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([
            // A titled group wraps its content in structural frame and body
            // levels, which the address skips.
            Element::new(
                "left",
                Group::titled(
                    "Settings",
                    Element::new(
                        "form",
                        Row::new([Element::new("apply", Label::new("Apply"))]),
                    ),
                ),
            ),
            // A slot replaces the key for itself and its descendants; a part
            // names only itself.
            Element::new(
                "right",
                Column::new([
                    Element::new(
                        "internal",
                        Row::new([Element::new("apply", Label::new("Apply"))]),
                    )
                    .slot("slotted"),
                    Element::new("divider", Label::new("Divider")).part("handle"),
                ]),
            ),
            // Two structural wrappers, each with a child keyed alike, give two
            // components one address.
            Element::new(
                "twins",
                Column::new([
                    Element::new("a", Row::new([Element::new("item", Label::new("A"))]))
                        .structural(),
                    Element::new("b", Row::new([Element::new("item", Label::new("B"))]))
                        .structural(),
                ]),
            ),
        ]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let left = ui.find(&root, &["left"])?.ok_or("mounted")?;
    let form = ui.find_descendant(&left, "form")?.ok_or("mounted")?;
    assert_eq!(
        ui.find_address(&root, &["left", "form"])?,
        Some(form.clone())
    );
    assert_eq!(ui.find_address(&left, &["form"])?, Some(form.clone()));
    let apply = ui
        .find_address(&root, &["left", "form", "apply"])?
        .ok_or("mounted")?;
    assert_eq!(ui.find(&form, &["apply"])?, Some(apply.clone()));
    // The address agrees with inspection's path selector.
    let query = ui.query(&InspectionQuery::components(Selector::path([
        "root", "left", "form", "apply",
    ])));
    assert_eq!(query.matched, 1);
    assert_eq!(query.components[0].key, ui.key(&apply)?);

    let slotted = ui
        .find_address(&root, &["right", "slotted", "apply"])?
        .ok_or("mounted")?;
    assert_ne!(slotted, apply);
    assert_eq!(
        ui.find_address(&root, &["right", "internal", "apply"])?,
        None
    );
    let handle = ui
        .find_address(&root, &["right", "handle"])?
        .ok_or("mounted")?;
    assert_eq!(ui.key(&handle)?, "divider");
    assert_eq!(ui.find_address(&root, &["right", "divider"])?, None);

    assert_eq!(
        ui.find_address(&root, &["twins", "item"]),
        Err(RuntimeError::AmbiguousKey)
    );
    assert_eq!(ui.find_address(&root, &["twins", "a", "item"])?, None);
    assert_eq!(ui.find_address(&root, &["left", "missing"])?, None);
    assert_eq!(
        ui.find_address(&root, &["root", "left"])?,
        None,
        "relative to root"
    );
    assert_eq!(ui.find_address(&root, &[])?, Some(root.clone()));
    // Hidden components are still found.
    ui.set_visible(&form, false)?;
    assert_eq!(
        ui.find_address(&root, &["left", "form", "apply"])?,
        Some(apply)
    );

    let other = Runtime::new();
    assert_eq!(
        other.find_address(&root, &["left"]),
        Err(RuntimeError::ForeignRuntime)
    );
    ui.unmount(&root)?;
    assert_eq!(
        ui.find_address(&root, &["left"]),
        Err(RuntimeError::Unmounted)
    );
    Ok(())
}

/// Reads the motion preference only while `reads` is set.
struct MotionReader {
    reads: Rc<Cell<bool>>,
    updates: Rc<Cell<usize>>,
}
impl Component for MotionReader {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.updates.set(self.updates.get() + 1);
        if self.reads.get() {
            let _ = cx.motion();
        }
        Ok(())
    }
}

#[test]
fn the_motion_subscription_ends_with_the_last_update_that_read_it() -> TestResult {
    let mut ui = Runtime::new();
    let reads = Rc::new(Cell::new(true));
    let updates = Rc::new(Cell::new(0));
    let id = ui.mount_tree(Element::new(
        "reader",
        MotionReader {
            reads: reads.clone(),
            updates: updates.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    let settings = ui.ui_settings();
    settings.set_motion(MotionPreference::Reduced)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(updates.get(), 2);

    // An update that no longer reads the preference ends the subscription.
    reads.set(false);
    ui.invalidate(&id, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(updates.get(), 3);
    settings.set_motion(MotionPreference::Full)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(updates.get(), 3);

    // Reading it again subscribes again.
    reads.set(true);
    ui.invalidate(&id, Invalidation::Paint)?;
    ui.flush(Duration::ZERO)?;
    settings.set_motion(MotionPreference::Reduced)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(updates.get(), 5);
    Ok(())
}

/// Records the theme mode it reads from a settings signal.
struct ModeReader {
    mode: ReadSignal<ThemeMode>,
    seen: Rc<Cell<Option<ThemeMode>>>,
}
impl Component for ModeReader {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.seen.set(Some(cx.read(&self.mode)?));
        Ok(())
    }
}

#[test]
fn presentation_settings_are_observable_through_read_signals() -> TestResult {
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let settings = ui.ui_settings();
    let mode = theme.mode_signal()?;
    let density = theme.density_signal()?;
    let zoom = settings.zoom_signal()?;
    let motion = settings.motion_signal()?;
    let overlay = settings.debug_overlay_signal()?;
    assert_eq!(mode.get()?, ThemeMode::Dark);
    assert_eq!(density.get()?, Density::Standard);
    assert_eq!(zoom.get()?, UiZoom::default());
    assert_eq!(motion.get()?, MotionPreference::Full);
    assert_eq!(overlay.get()?, DebugOverlay::Off);

    let seen = Rc::new(Cell::new(None));
    ui.mount_tree(Element::new(
        "reader",
        ModeReader {
            mode: mode.clone(),
            seen: seen.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(seen.get(), Some(ThemeMode::Dark));

    let saved = ui.capture_workspace()?;
    theme.set_mode(ThemeMode::Light)?;
    theme.set_density(Density::Compact)?;
    settings.set_zoom(UiZoom::new(1.5)?)?;
    settings.set_motion(MotionPreference::Reduced)?;
    settings.set_debug_overlay(DebugOverlay::Boxes)?;
    assert_eq!(mode.get()?, ThemeMode::Light);
    assert_eq!(density.get()?, Density::Compact);
    assert_eq!(zoom.get()?.get(), 1.5);
    assert_eq!(motion.get()?, MotionPreference::Reduced);
    assert_eq!(overlay.get()?, DebugOverlay::Boxes);
    ui.flush(Duration::ZERO)?;
    assert_eq!(seen.get(), Some(ThemeMode::Light));

    // A restored workspace changes the settings through the same setters.
    let _report = ui.restore_workspace(&saved)?;
    assert_eq!(mode.get()?, ThemeMode::Dark);
    assert_eq!(density.get()?, Density::Standard);
    assert_eq!(zoom.get()?, UiZoom::default());
    ui.flush(Duration::ZERO)?;
    assert_eq!(seen.get(), Some(ThemeMode::Dark));

    // The runtime-level overlay setter publishes too, and a signal requested
    // after every earlier one was dropped starts from the current value.
    ui.set_debug_overlay(DebugOverlay::Issues);
    assert_eq!(overlay.get()?, DebugOverlay::Issues);
    drop(overlay);
    ui.set_debug_overlay(DebugOverlay::Off);
    assert_eq!(settings.debug_overlay_signal()?.get()?, DebugOverlay::Off);
    Ok(())
}
