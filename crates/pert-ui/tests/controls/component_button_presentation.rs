//! Redescription separates an icon button's appearance from its bound name.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn button(icon: Option<Icon>, name: ReadSignal<String>) -> Element {
    let button = match icon {
        Some(icon) => Button::icon(icon, name, |_| Ok(())),
        None => Button::new(name, |_| Ok(())),
    };
    Element::new("button", button)
}

#[test]
fn bound_name_does_not_prevent_icon_or_presentation_replacement() -> TestResult {
    for (before, after) in [
        (Some(Icon::Add), Some(Icon::Settings)),
        (None, Some(Icon::Settings)),
        (Some(Icon::Settings), None),
    ] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
        let name = ui.signal("Translated action".to_owned());
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([button(before, name.read_only())]),
        ))?;
        ui.flush(Duration::ZERO)?;
        let id = ui.children(&root)?[0].clone();
        let original = ui.paint()?;
        ui.reconcile_children(&root, vec![button(after, name.read_only())])?;
        ui.flush(Duration::ZERO)?;
        let changed = ui.paint()?;
        assert_eq!(ui.children(&root)?[0], id);
        assert_ne!(
            changed, original,
            "the new presentation must be drawn: {before:?} -> {after:?}"
        );
        let mut reference = Runtime::new();
        reference.set_viewport(Viewport::new(240.0, 80.0, 1.0)?)?;
        let reference_name = reference.signal(name.get()?);
        reference.mount_tree(Element::new(
            "root",
            Column::new([button(after, reference_name.read_only())]),
        ))?;
        reference.flush(Duration::ZERO)?;
        assert_eq!(
            changed,
            reference.paint()?,
            "redescription must draw the newly requested presentation"
        );
        let details = ui
            .inspect()
            .components
            .into_iter()
            .find(|c| c.key == "button")
            .unwrap()
            .details;
        assert_eq!(
            details.semantics.unwrap().name.as_deref(),
            Some("Translated action")
        );
        ui.set_profiling(true)?;
        ui.reconcile_children(&root, vec![button(after, name.read_only())])?;
        ui.flush(Duration::ZERO)?;
        ui.paint()?;
        assert!(
            !ui.profile().stats.iter().any(|stat| matches!(
                stat.operation,
                ProfileOperation::Measure | ProfileOperation::Arrange
            )),
            "unchanged icon and name should retain layout"
        );
    }
    Ok(())
}
