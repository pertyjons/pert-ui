//! Numeric boundary regressions discovered by the framework audit.
use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn label(key: &str) -> Element {
    Element::new(key, Label::new(key))
}
fn rejected_unchanged(
    component: &mut impl Component,
    state: &ViewState,
    field: &str,
) -> TestResult {
    let before = component.workspace_state()?;
    assert_eq!(
        component.restore_workspace_state(state),
        Err(RestoreRejection::OutOfRange(field.to_owned()))
    );
    assert_eq!(component.workspace_state()?, before);
    Ok(())
}
#[test]
fn workspace_numbers_are_validated_before_narrowing() -> TestResult {
    let mut scroll = ScrollArea::new(label("body"));
    for (x, y, field) in [(-1e-300, 0.0, "x"), (12.0, -1e-300, "y")] {
        let state = ViewState::new("scroll")?
            .with("x", WorkspaceValue::number(x)?)?
            .with("y", WorkspaceValue::number(y)?)?;
        rejected_unchanged(&mut scroll, &state, field)?;
    }
    let mut grid = PropertyGrid::new([]);
    rejected_unchanged(
        &mut grid,
        &ViewState::new("property-grid")?.with("label-width", WorkspaceValue::number(-1e-300)?)?,
        "label-width",
    )?;
    let mut split = SplitPane::new(SplitAxis::LeftRight, label("a"), label("b"));
    for (rule, extent) in [
        ("ratio", 1.00000001),
        ("ratio", -1e-300),
        ("first", -1e-300),
        ("second", -1e-300),
    ] {
        let state = ViewState::new("split")?
            .with("rule", WorkspaceValue::text(rule)?)?
            .with("extent", WorkspaceValue::number(extent)?)?;
        rejected_unchanged(&mut split, &state, "extent")?;
    }
    let mut zoom = ZoomView::new([(Point::ZERO, label("content"))]);
    for scale in [32.00000001, f64::from(ViewScale::MIN.get()) - 1e-10] {
        let state = ViewState::new("zoom")?
            .with("scale", WorkspaceValue::number(scale)?)?
            .with("x", WorkspaceValue::number(-10.0)?)?
            .with("y", WorkspaceValue::number(20.0)?)?;
        rejected_unchanged(&mut zoom, &state, "scale")?;
    }
    Ok(())
}
#[test]
fn content_splits_reject_extraneous_extent_without_changing_state() -> TestResult {
    let mut split = SplitPane::new(SplitAxis::LeftRight, label("a"), label("b"));
    let before = split.workspace_state()?;
    for rule in ["content-first", "content-second"] {
        let state = ViewState::new("split")?
            .with("rule", WorkspaceValue::text(rule)?)?
            .with("extent", WorkspaceValue::flag(true))?;
        assert!(split.restore_workspace_state(&state).is_err());
        assert_eq!(split.workspace_state()?, before);
    }
    Ok(())
}
#[test]
fn window_extents_reject_negative_underflow_transactionally() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let window = FloatingWindow::new(&ui, "window", "Window", label("body"));
    ui.mount_tree(Element::new("layer", WindowLayer::new([window])))?;
    ui.flush(Duration::ZERO)?;
    let before = ui.capture_workspace()?;
    for (width, height, field) in [(-1e-300, 120.0, "width"), (160.0, -1e-300, "height")] {
        let state = ViewState::new("floating-window")?
            .with("x", WorkspaceValue::number(-20.0)?)?
            .with("y", WorkspaceValue::number(30.0)?)?
            .with("width", WorkspaceValue::number(width)?)?
            .with("height", WorkspaceValue::number(height)?)?
            .with("minimized", WorkspaceValue::flag(false))?;
        let document = Workspace::new(
            WorkspaceSettings::default(),
            [(WorkspacePath::new(["layer", "window"])?, state)],
        )?;
        let report = ui.restore_workspace(&document)?;
        assert!(
            report
                .rejected()
                .any(|(_, reason)| *reason == RestoreRejection::OutOfRange(field.into()))
        );
        assert_eq!(ui.capture_workspace()?, before);
    }
    Ok(())
}
