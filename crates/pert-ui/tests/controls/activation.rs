//! Simultaneous pointer and key activation: Button, an activatable
//! ColorSwatch and the choice controls follow one ownership rule, so no
//! interleaving activates twice.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Clone, Copy, Debug)]
enum Step {
    KeyDown,
    KeyUp,
    PointerDown,
    PointerUp,
    Escape,
    FocusLoss,
}
use Step::*;

/// Builds the control under test and a probe whose value changes once per
/// activation.
type Build = fn(&Runtime) -> (Element, Box<dyn Fn() -> u32>);

fn button(_: &Runtime) -> (Element, Box<dyn Fn() -> u32>) {
    let count = Rc::new(Cell::new(0));
    let action = count.clone();
    let control = Button::new("Apply", move |_| {
        action.set(action.get() + 1);
        Ok(())
    });
    (
        Element::new("control", control),
        Box::new(move || count.get()),
    )
}
fn checkbox(ui: &Runtime) -> (Element, Box<dyn Fn() -> u32>) {
    let value = ui.signal(false);
    let control = Checkbox::new(value.clone(), "Enabled");
    (
        Element::new("control", control),
        Box::new(move || u32::from(value.get().unwrap())),
    )
}
fn toggle(ui: &Runtime) -> (Element, Box<dyn Fn() -> u32>) {
    let value = ui.signal(false);
    let control = Toggle::new(value.clone(), "Enabled");
    (
        Element::new("control", control),
        Box::new(move || u32::from(value.get().unwrap())),
    )
}

/// Mounts the control beside a spare button, focuses it, runs `steps` with the
/// pointer at its center and Space as the key, and counts activations: the
/// steps after which the probe changed. Each activation completes in its own
/// step, so a double activation cannot cancel out.
fn activations(build: Build, steps: &[Step]) -> u32 {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let (control, probe) = build(&ui);
    let root = ui
        .mount_tree(Element::new(
            "row",
            Row::new([
                control,
                Element::new("other", Button::new("Other", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let children = ui.children(&root).unwrap();
    let [x, y, w, h] = ui.bounds(&children[0]).unwrap().unwrap().values();
    let center = Point::new(x + w / 2.0, y + h / 2.0).unwrap();
    let pointer = |ui: &mut Runtime, kind| {
        ui.pointer_input(PointerEvent::new(center, kind)).unwrap();
    };
    let key = |ui: &mut Runtime, code, action| {
        ui.key_input(code, action).unwrap();
    };
    pointer(&mut ui, PointerKind::Move);
    ui.focus(&children[0]).unwrap();
    ui.flush(Duration::ZERO).unwrap();
    let mut last = probe();
    let mut count = 0;
    for step in steps {
        match step {
            KeyDown => key(&mut ui, KeyCode::Space, KeyAction::Press),
            KeyUp => key(&mut ui, KeyCode::Space, KeyAction::Release),
            Escape => {
                key(&mut ui, KeyCode::Escape, KeyAction::Press);
                key(&mut ui, KeyCode::Escape, KeyAction::Release);
            }
            PointerDown => pointer(&mut ui, PointerKind::Press(PointerButton::Primary)),
            PointerUp => pointer(&mut ui, PointerKind::Release(PointerButton::Primary)),
            FocusLoss => ui.focus(&children[1]).unwrap(),
        }
        ui.flush(Duration::ZERO).unwrap();
        if probe() != last {
            count += 1;
            last = probe();
        }
    }
    count
}

fn swatch(_: &Runtime) -> (Element, Box<dyn Fn() -> u32>) {
    let count = Rc::new(Cell::new(0));
    let action = count.clone();
    let control = ColorSwatch::new(Color::WHITE).on_activate(move |_| {
        action.set(action.get() + 1);
        Ok(())
    });
    (
        Element::new("control", control),
        Box::new(move || count.get()),
    )
}

const CONTROLS: [(&str, Build); 4] = [
    ("Button", button),
    ("Checkbox", checkbox),
    ("Toggle", toggle),
    ("ColorSwatch", swatch),
];

/// Every control gives each interleaving the same number of activations.
fn expect(steps: &[Step], expected: u32) {
    for (name, build) in CONTROLS {
        assert_eq!(activations(build, steps), expected, "{name}: {steps:?}");
    }
}

#[test]
fn a_key_held_first_owns_the_activation_and_a_pointer_press_is_ignored() {
    expect(&[KeyDown, PointerDown, PointerUp, KeyUp], 1);
    expect(&[KeyDown, PointerDown, KeyUp, PointerUp], 1);
}
#[test]
fn a_pointer_held_first_owns_the_activation_and_a_key_press_is_ignored() {
    expect(&[PointerDown, KeyDown, KeyUp, PointerUp], 1);
    expect(&[PointerDown, KeyDown, PointerUp, KeyUp], 1);
}
#[test]
fn escape_cancels_whichever_input_is_held() {
    expect(&[KeyDown, PointerDown, Escape, PointerUp, KeyUp], 0);
    expect(&[PointerDown, KeyDown, Escape, KeyUp, PointerUp], 0);
}
#[test]
fn focus_loss_cancels_whichever_input_is_held() {
    expect(&[KeyDown, PointerDown, FocusLoss, PointerUp, KeyUp], 0);
    expect(&[PointerDown, KeyDown, FocusLoss, KeyUp, PointerUp], 0);
}
#[test]
fn a_single_input_still_activates_once() {
    expect(&[KeyDown, KeyUp], 1);
    expect(&[PointerDown, PointerUp], 1);
}

type TestError = Box<dyn std::error::Error>;
/// Keeps parameter hosts alive while their controls are edited.
type Hosts = Vec<ParameterHost<f32>>;
/// Builds a focusable control under test.
type Make = fn(&mut Runtime, &mut Hosts) -> Result<Element, TestError>;
fn level(ui: &mut Runtime, hosts: &mut Hosts) -> Result<Parameter<f32>, TestError> {
    let (parameter, host) = ui.parameter(
        ParameterId::new(1, 1, "level")?,
        ParameterSpec::new("Level", 0.0, 1.0, 0.5)?,
    )?;
    hosts.push(host);
    Ok(parameter)
}
fn two(ui: &Runtime) -> (Signal<u8>, [Choice<u8>; 2]) {
    (ui.signal(1), [Choice::new(1, "One"), Choice::new(2, "Two")])
}
const ESCAPE_CASES: [(&str, Make, KeyCode, CommandKey); 10] = [
    (
        "Button",
        |_, _| Ok(Element::new("control", Button::new("Apply", |_| Ok(())))),
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "Checkbox",
        |ui, _| {
            Ok(Element::new(
                "control",
                Checkbox::new(ui.signal(false), "On"),
            ))
        },
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "Toggle",
        |ui, _| Ok(Element::new("control", Toggle::new(ui.signal(false), "On"))),
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "InlineToggle",
        |ui, _| {
            Ok(Element::new(
                "control",
                InlineToggle::new(ui.signal(false), "Mute"),
            ))
        },
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "RadioGroup",
        |ui, _| {
            let (value, choices) = two(ui);
            Ok(Element::new("control", RadioGroup::new(value, choices)?))
        },
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "SegmentedControl",
        |ui, _| {
            let (value, choices) = two(ui);
            Ok(Element::new(
                "control",
                SegmentedControl::new(value, choices)?,
            ))
        },
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "Select",
        |ui, _| {
            let (value, choices) = two(ui);
            Ok(Element::new("control", Select::new(ui, value, choices)?))
        },
        KeyCode::Space,
        CommandKey::Character(' '),
    ),
    (
        "Slider",
        |ui, hosts| Ok(Element::new("control", Slider::new(level(ui, hosts)?))),
        KeyCode::ArrowRight,
        CommandKey::Named(KeyCode::ArrowRight),
    ),
    (
        "InlineSlider",
        |ui, hosts| {
            let level = level(ui, hosts)?;
            Ok(Element::new(
                "control",
                InlineSlider::new(level).label("Level"),
            ))
        },
        KeyCode::ArrowRight,
        CommandKey::Named(KeyCode::ArrowRight),
    ),
    (
        "InlineReadout",
        |ui, _| {
            let text = ui.signal(String::from("1.0"));
            Ok(Element::new(
                "control",
                InlineReadout::new(text.read_only()).label("Load"),
            ))
        },
        KeyCode::Enter,
        CommandKey::Named(KeyCode::Enter),
    ),
];

/// A focused control with nothing to cancel leaves Escape to View, Window and
/// Application commands; an armed or editing control consumes it.
#[test]
fn escape_reaches_commands_unless_a_focused_control_cancels_something() -> Result<(), TestError> {
    fn key(
        ui: &mut Runtime,
        physical: KeyCode,
        logical: CommandKey,
        action: KeyAction,
    ) -> Result<(), RuntimeError> {
        ui.command_key_input(CommandKeyInput::new(physical, action, logical))?;
        Ok(())
    }
    fn escape(ui: &mut Runtime) -> Result<(), RuntimeError> {
        let logical = CommandKey::Named(KeyCode::Escape);
        key(ui, KeyCode::Escape, logical, KeyAction::Press)?;
        key(ui, KeyCode::Escape, logical, KeyAction::Release)?;
        ui.flush(Duration::ZERO)?;
        Ok(())
    }
    for (name, make, arm, logical) in ESCAPE_CASES {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
        let hits = Rc::new(Cell::new(0));
        let counter = hits.clone();
        let _leave = ui.register_command(
            "Leave",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::Escape),
                Modifiers::default(),
            )],
            move |_, _| {
                counter.set(counter.get() + 1);
                Ok(())
            },
        )?;
        let mut hosts = Hosts::new();
        let control = make(&mut ui, &mut hosts)?;
        ui.mount_tree(Element::new("column", Column::new([control])))?;
        ui.flush(Duration::ZERO)?;
        escape(&mut ui)?;
        assert_eq!(hits.get(), 1, "{name}: nothing focused");

        ui.focus_next(FocusDirection::Forward)?;
        assert!(ui.focused().is_some(), "{name} takes focus");
        escape(&mut ui)?;
        assert_eq!(hits.get(), 2, "{name}: an idle control lets Escape through");

        key(&mut ui, arm, logical, KeyAction::Press)?;
        ui.flush(Duration::ZERO)?;
        escape(&mut ui)?;
        key(&mut ui, arm, logical, KeyAction::Release)?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(hits.get(), 2, "{name}: Escape cancelled the armed input");
    }
    Ok(())
}
