//! Direct dynamic-choice constructors preserve validation and source ownership.
use pert_ui::*;
use std::time::Duration;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[derive(Clone, Copy, Debug)]
enum Kind {
    Radio,
    Segmented,
    InlineChoice,
    Select,
    InlineSelect,
    MultiSelect,
}
const KINDS: [Kind; 6] = [
    Kind::Radio,
    Kind::Segmented,
    Kind::InlineChoice,
    Kind::Select,
    Kind::InlineSelect,
    Kind::MultiSelect,
];
fn bound(kind: Kind, ui: &Runtime, options: ReadSignal<Vec<Choice<u8>>>) -> Element {
    let value = ui.signal(1);
    match kind {
        Kind::Radio => Element::new("control", RadioGroup::bound(value, options)),
        Kind::Segmented => Element::new("control", SegmentedControl::bound(value, options)),
        Kind::InlineChoice => Element::new("control", InlineChoice::bound(value, options)),
        Kind::Select => Element::new("control", Select::bound(ui, value, options)),
        Kind::InlineSelect => Element::new("control", InlineSelect::bound(ui, value, options)),
        Kind::MultiSelect => Element::new(
            "control",
            MultiSelect::bound(ui, ui.signal(vec![1]), options),
        ),
    }
}
#[test]
fn dynamic_construction_defers_validation_and_recovers_from_duplicate_lists() -> TestResult {
    for kind in KINDS {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 300.0, 1.0)?)?;
        let options = ui.signal(vec![Choice::new(1, "One"), Choice::new(1, "Duplicate")]);
        let id = ui.mount_tree(bound(kind, &ui, options.read_only()))?;
        assert_eq!(
            ui.flush(Duration::ZERO),
            Err(RuntimeError::Choice(ChoiceError::DuplicateValue {
                first: 0,
                second: 1
            })),
            "{kind:?}"
        );
        options.set(vec![Choice::new(1, "One"), Choice::new(2, "Two")])?;
        ui.flush(Duration::ZERO)?;
        let valid = ui.paint()?;
        let presented = |ui: &Runtime| {
            ui.inspect()
                .components
                .into_iter()
                .map(|component| (component.key, component.details.semantics))
                .collect::<Vec<_>>()
        };
        let last_valid = presented(&ui);
        options.set(vec![Choice::new(2, "Two"), Choice::new(2, "Duplicate")])?;
        assert_eq!(
            ui.flush(Duration::ZERO),
            Err(RuntimeError::Choice(ChoiceError::DuplicateValue {
                first: 0,
                second: 1
            })),
            "{kind:?}"
        );
        assert_eq!(
            presented(&ui),
            last_valid,
            "{kind:?} preserves valid semantics"
        );
        options.set(vec![Choice::new(1, "One"), Choice::new(2, "Two")])?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(
            ui.paint()?,
            valid,
            "{kind:?} recovers the prior presentation"
        );
        assert!(ui.bounds(&id)?.is_some());
    }
    Ok(())
}
#[test]
fn dynamic_constructors_report_foreign_sources_during_update() -> TestResult {
    let foreign = Runtime::new();
    let options = foreign.signal(vec![Choice::new(1, "One")]);
    for kind in KINDS {
        let mut ui = Runtime::new();
        ui.mount_tree(bound(kind, &ui, options.read_only()))?;
        assert_eq!(
            ui.flush(Duration::ZERO),
            Err(RuntimeError::ForeignRuntime),
            "{kind:?}"
        );
    }
    Ok(())
}
#[test]
fn same_key_can_switch_parameter_dynamic_and_fixed_choice_sources() -> TestResult {
    for kind in KINDS
        .into_iter()
        .filter(|kind| !matches!(kind, Kind::MultiSelect))
    {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 300.0, 1.0)?)?;
        let one = ChoiceId::new("one")?;
        let (parameter, _host) = ui.parameter(
            ParameterId::new(1, 1, "choice")?,
            ParameterSpec::choices(
                "Choice",
                [ChoiceOption::new(one.clone(), "Parameter")],
                one.clone(),
            )?,
        )?;
        let value = ui.signal(one.clone());
        let source = ui.signal(vec![Choice::new(one.clone(), "Dynamic")]);
        let describe = |ui: &Runtime, mode| -> Result<Element, ChoiceError> {
            let fixed = || [Choice::new(one.clone(), "Fixed")];
            macro_rules! group {
                ($ty:ident) => {
                    Element::new(
                        "control",
                        match mode {
                            0 => $ty::for_parameter(parameter.clone()),
                            1 => $ty::bound(value.clone(), source.read_only()),
                            _ => $ty::new(value.clone(), fixed())?,
                        },
                    )
                };
            }
            macro_rules! dropdown {
                ($ty:ident) => {
                    Element::new(
                        "control",
                        match mode {
                            0 => $ty::for_parameter(ui, parameter.clone()),
                            1 => $ty::bound(ui, value.clone(), source.read_only()),
                            _ => $ty::new(ui, value.clone(), fixed())?,
                        },
                    )
                };
            }
            Ok(match kind {
                Kind::Radio => group!(RadioGroup),
                Kind::Segmented => group!(SegmentedControl),
                Kind::InlineChoice => group!(InlineChoice),
                Kind::Select => dropdown!(Select),
                Kind::InlineSelect => dropdown!(InlineSelect),
                Kind::MultiSelect => unreachable!(),
            })
        };
        let first = describe(&ui, 0)?;
        let root = ui.mount_tree(Element::new("root", Column::new([first])))?;
        ui.flush(Duration::ZERO)?;
        let id = ui.children(&root)?[0].clone();
        for (mode, label) in [(1, "Dynamic"), (2, "Fixed"), (0, "Parameter")] {
            // Build before borrowing the runtime mutably for reconciliation.
            let next = describe(&ui, mode)?;
            ui.reconcile_children(&root, vec![next])?;
            ui.flush(Duration::ZERO)?;
            assert_eq!(ui.children(&root)?[0], id);
            let key = if matches!(kind, Kind::Select | Kind::InlineSelect) {
                "select"
            } else {
                "control"
            };
            let semantics = ui
                .inspect()
                .components
                .into_iter()
                .find(|c| c.key == key)
                .unwrap()
                .details
                .semantics
                .unwrap();
            assert_eq!(
                semantics
                    .items
                    .iter()
                    .map(|item| item.name.as_deref().unwrap())
                    .collect::<Vec<_>>(),
                vec![label],
                "{kind:?}"
            );
        }
    }
    Ok(())
}
#[test]
fn a_rebinding_with_an_invalid_list_keeps_the_last_valid_presentation() -> TestResult {
    for kind in [Kind::Radio, Kind::Segmented, Kind::InlineChoice] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(500.0, 300.0, 1.0)?)?;
        let describe = |value: Signal<u8>, options: ReadSignal<Vec<Choice<u8>>>| match kind {
            Kind::Radio => Element::new("control", RadioGroup::bound(value, options)),
            Kind::Segmented => Element::new("control", SegmentedControl::bound(value, options)),
            _ => Element::new("control", InlineChoice::bound(value, options)),
        };
        let presented = |ui: &Runtime| {
            ui.inspect()
                .components
                .into_iter()
                .find(|component| component.key == "control")
                .map(|component| component.details)
        };
        let first = ui.signal(vec![Choice::new(1, "One"), Choice::new(2, "Two")]);
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([describe(ui.signal(2), first.read_only())]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let last_valid = presented(&ui);
        // A new value binding and an invalid list in the same flush.
        let second = ui.signal(vec![Choice::new(3, "Three"), Choice::new(3, "Duplicate")]);
        ui.reconcile_children(&root, vec![describe(ui.signal(3), second.read_only())])?;
        assert_eq!(
            ui.flush(Duration::ZERO),
            Err(RuntimeError::Choice(ChoiceError::DuplicateValue {
                first: 0,
                second: 1
            })),
            "{kind:?}"
        );
        assert_eq!(
            presented(&ui),
            last_valid,
            "{kind:?} keeps the last valid presentation"
        );
        // A valid list then presents the new binding.
        second.set(vec![Choice::new(3, "Three"), Choice::new(4, "Four")])?;
        ui.flush(Duration::ZERO)?;
        assert_ne!(
            presented(&ui),
            last_valid,
            "{kind:?} presents the new binding"
        );
    }
    Ok(())
}
