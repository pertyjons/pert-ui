//! Caption bindings and replaceable adornments through the public facade.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug)]
enum Kind {
    Label,
    Button,
    Link,
    Chip,
}
fn describe(kind: Kind, text: &str, source: Option<ReadSignal<String>>, repeated: bool) -> Element {
    let text: TextSource = source.map_or_else(|| text.into(), Into::into);
    match kind {
        Kind::Label => {
            let mut label = Label::new(text);
            if repeated {
                label = label.leading_icon(Icon::Add).trailing_icon(Icon::Close);
            }
            Element::new(
                "control",
                label.leading_icon(Icon::Settings).trailing_icon(Icon::Info),
            )
        }
        Kind::Button => {
            let mut button = Button::new(text, |_| Ok(()));
            if repeated {
                button = button.leading_icon(Icon::Add).trailing_icon(Icon::Close);
            }
            Element::new(
                "control",
                button
                    .leading_icon(Icon::Settings)
                    .trailing_icon(Icon::Info),
            )
        }
        Kind::Link => {
            let mut link = Link::new(text, |_| Ok(()));
            if repeated {
                link = link.leading_icon(Icon::Add).trailing_icon(Icon::Close);
            }
            Element::new(
                "control",
                link.leading_icon(Icon::Settings).trailing_icon(Icon::Info),
            )
        }
        Kind::Chip => {
            let mut chip = Chip::removable(text, |_| Ok(()));
            if repeated {
                chip = chip.leading_icon(Icon::Add);
            }
            Element::new("control", chip.leading_icon(Icon::Settings))
        }
    }
}
fn runtime() -> Result<Runtime, RuntimeError> {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 90.0, 1.0)?)?;
    Ok(ui)
}
fn reference(kind: Kind, text: &str) -> Result<DisplayList, Box<dyn std::error::Error>> {
    let mut ui = runtime()?;
    ui.mount_tree(Element::new(
        "root",
        Column::new([describe(kind, text, None, false)]),
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(ui.paint()?)
}
#[test]
fn repeated_adornment_builders_replace_the_same_slot() -> TestResult {
    for kind in [Kind::Label, Kind::Button, Kind::Link, Kind::Chip] {
        let mut ui = runtime()?;
        ui.mount_tree(Element::new(
            "root",
            Column::new([describe(kind, "Caption", None, true)]),
        ))?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(ui.paint()?, reference(kind, "Caption")?, "{kind:?}");
    }
    Ok(())
}
#[test]
fn captions_keep_adornments_through_updates_and_binding_replacement() -> TestResult {
    for kind in [Kind::Label, Kind::Button, Kind::Link] {
        let mut ui = runtime()?;
        let caption = ui.signal("Caption".to_owned());
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([describe(kind, "Fallback", Some(caption.read_only()), false)]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let id = ui.children(&root)?[0].clone();
        assert_eq!(
            ui.paint()?,
            reference(kind, "Caption")?,
            "{kind:?} initial binding"
        );
        caption.set("Öppna inställningar".into())?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(
            ui.paint()?,
            reference(kind, "Öppna inställningar")?,
            "{kind:?} caption update"
        );
        let replacement = ui.signal("Replacement".to_owned());
        ui.reconcile_children(
            &root,
            vec![describe(
                kind,
                "Fallback",
                Some(replacement.read_only()),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(ui.children(&root)?[0], id);
        assert_eq!(
            ui.paint()?,
            reference(kind, "Replacement")?,
            "{kind:?} replacement binding"
        );
        caption.set("Obsolete".into())?;
        assert!(
            !ui.needs_paint(),
            "old caption no longer schedules {kind:?}"
        );
        ui.set_profiling(true)?;
        ui.reconcile_children(
            &root,
            vec![describe(
                kind,
                "Fallback",
                Some(replacement.read_only()),
                false,
            )],
        )?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        assert!(
            !ui.profile().stats.iter().any(|s| matches!(
                s.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange
            )),
            "unchanged {kind:?} must retain layout"
        );
    }
    Ok(())
}

fn rich_action(
    kind: Kind,
    source: Option<ReadSignal<String>>,
    leading: Option<Icon>,
    trailing: Option<Icon>,
) -> Element {
    let content: TextSource = source.map_or_else(
        || RichText::from("Caption").leading_icon(Icon::Add).into(),
        Into::into,
    );
    match kind {
        Kind::Button => {
            let mut control = Button::new(content, |_| Ok(()));
            if let Some(icon) = leading {
                control = control.leading_icon(icon);
            }
            if let Some(icon) = trailing {
                control = control.trailing_icon(icon);
            }
            Element::new("control", control)
        }
        Kind::Link => {
            let mut control = Link::new(content, |_| Ok(()));
            if let Some(icon) = leading {
                control = control.leading_icon(icon);
            }
            if let Some(icon) = trailing {
                control = control.trailing_icon(icon);
            }
            Element::new("control", control)
        }
        _ => unreachable!(),
    }
}
#[test]
fn bound_rich_captions_drop_embedded_icons_but_adopt_and_clear_slots() -> TestResult {
    for kind in [Kind::Button, Kind::Link] {
        let mut ui = runtime()?;
        let caption = ui.signal("Caption".to_owned());
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([rich_action(
                kind,
                Some(caption.read_only()),
                Some(Icon::Settings),
                Some(Icon::Info),
            )]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let id = ui.children(&root)?[0].clone();
        assert_eq!(ui.paint()?, reference(kind, "Caption")?);
        caption.set(String::new())?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(
            ui.paint()?,
            reference(kind, "")?,
            "empty captions retain only adornments and their normal spacing"
        );
        for leading in [Some(Icon::Save), None] {
            ui.reconcile_children(
                &root,
                vec![rich_action(kind, Some(caption.read_only()), leading, None)],
            )?;
            ui.flush(Duration::ZERO)?;
            assert_eq!(ui.children(&root)?[0], id);
            let mut fresh = runtime()?;
            let fresh_caption = fresh.signal(String::new());
            fresh.mount_tree(Element::new(
                "root",
                Column::new([rich_action(
                    kind,
                    Some(fresh_caption.read_only()),
                    leading,
                    None,
                )]),
            ))?;
            fresh.flush(Duration::ZERO)?;
            assert_eq!(
                ui.paint()?,
                fresh.paint()?,
                "{kind:?} changed/removed slots"
            );
        }
    }
    Ok(())
}
#[test]
fn adornment_redescription_keeps_armed_activation_and_adopts_the_new_action() -> TestResult {
    use std::{cell::Cell, rc::Rc};
    for link in [false, true] {
        let mut ui = runtime()?;
        let old = Rc::new(Cell::new(0));
        let new = Rc::new(Cell::new(0));
        let describe = |icon, count: Rc<Cell<usize>>| {
            let action = move |_: &mut EventContext| {
                count.set(count.get() + 1);
                Ok(())
            };
            if link {
                Element::new("control", Link::new("Action", action).leading_icon(icon))
            } else {
                Element::new("control", Button::new("Action", action).leading_icon(icon))
            }
        };
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([describe(Icon::Add, old.clone())]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let id = ui.children(&root)?[0].clone();
        ui.focus(&id)?;
        ui.key_input(KeyCode::Space, KeyAction::Press)?;
        ui.reconcile_children(&root, vec![describe(Icon::Settings, new.clone())])?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(ui.children(&root)?[0], id);
        ui.key_input(KeyCode::Space, KeyAction::Release)?;
        ui.key_input(KeyCode::Space, KeyAction::Release)?;
        assert_eq!(old.get(), 0);
        assert_eq!(new.get(), 1);
    }
    Ok(())
}
#[test]
fn icon_buttons_adopt_slots_independently_of_icon_and_bound_name() -> TestResult {
    let mut ui = runtime()?;
    let name = ui.signal("Translated action".to_owned());
    let describe = |icon, trailing, name| {
        Element::new(
            "control",
            Button::icon(icon, name, |_| Ok(())).trailing_icon(trailing),
        )
    };
    let root = ui.mount_tree(Element::new(
        "root",
        Column::new([describe(Icon::Add, Icon::Info, name.read_only())]),
    ))?;
    ui.flush(Duration::ZERO)?;
    let id = ui.children(&root)?[0].clone();
    for (icon, trailing) in [(Icon::Settings, Icon::Info), (Icon::Settings, Icon::Close)] {
        ui.reconcile_children(&root, vec![describe(icon, trailing, name.read_only())])?;
        ui.flush(Duration::ZERO)?;
        let mut fresh = runtime()?;
        let fresh_name = fresh.signal(name.get()?);
        fresh.mount_tree(Element::new(
            "root",
            Column::new([describe(icon, trailing, fresh_name.read_only())]),
        ))?;
        fresh.flush(Duration::ZERO)?;
        assert_eq!(ui.paint()?, fresh.paint()?);
        assert_eq!(ui.children(&root)?[0], id);
        assert_eq!(
            ui.inspect()
                .components
                .into_iter()
                .find(|c| c.key == "control")
                .unwrap()
                .details
                .semantics
                .unwrap()
                .name
                .as_deref(),
            Some("Translated action")
        );
    }
    let before = ui.paint()?;
    ui.icons()
        .set(Icon::Close, ui.fonts().material_icon("save")?)?;
    assert!(
        ui.needs_paint(),
        "the new slot remains subscribed to icon overrides"
    );
    ui.flush(Duration::ZERO)?;
    assert_ne!(before, ui.paint()?, "the new icon mapping must be drawn");
    Ok(())
}
