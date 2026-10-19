//! Development validation of geometry waits for a layout with a viewport that
//! has area: content mounted before the host supplies its first viewport, and
//! a window whose viewport has no area, report no geometry issue, while a
//! component that is empty after a layout with area still does.
#![cfg(debug_assertions)]

use pert_ui::*;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn codes(ui: &Runtime) -> Vec<ValidationCode> {
    ui.validation_report()
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect()
}

/// The root key of each issue's component, in issue order.
fn issue_roots(ui: &Runtime) -> Vec<String> {
    let inspection = ui.inspect();
    ui.validation_report()
        .issues
        .iter()
        .filter_map(|issue| {
            let component = inspection
                .components
                .iter()
                .find(|component| component.id == issue.components[0])?;
            Some(component.path.as_ref()?.first()?.clone())
        })
        .collect()
}

fn grid(ui: &Runtime) -> Element {
    Element::new(
        "grid",
        PropertyGrid::new([Property::new(
            "name",
            "Name",
            Element::new("editor", TextField::new(ui.signal(String::new()))),
        )]),
    )
}

/// An interactive editor in a pane that a split gives no width.
fn collapsed_pane(ui: &Runtime) -> Result<Element, GeometryError> {
    Ok(Element::new(
        "split",
        SplitPane::new(
            SplitAxis::LeftRight,
            Element::new("left", Label::new("Left")),
            Element::new("right", TextField::new(ui.signal(String::new()))),
        )
        .sizing(SplitSizing::second(Length::new(0.0)?)),
    ))
}

/// A leaf that reports an invalid foreground color: a rule that does not
/// depend on geometry.
struct InvalidColor;
impl Component for InvalidColor {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(10.0, 10.0)?)
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.box_model.foreground = Some([2.0, 0.0, 0.0, 1.0]);
        details
    }
}

#[test]
fn content_flushed_before_the_first_viewport_reports_no_geometry_issue() -> TestResult {
    let mut ui = Runtime::new();
    let root = ui.mount_tree(grid(&ui))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(codes(&ui), []);
    // Layout ran against the zero-sized viewport, and inspection shows it.
    let editor = ui.find_descendant(&root, "editor")?.ok_or("editor")?;
    assert_eq!(ui.bounds(&editor)?.ok_or("laid out")?.width(), 0.0);
    // The first viewport with area lays the content out, and it is valid.
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    ui.flush(Duration::from_millis(1))?;
    assert!(ui.bounds(&editor)?.ok_or("laid out")?.width() > 0.0);
    assert_eq!(codes(&ui), []);
    Ok(())
}

#[test]
fn a_viewport_without_area_withholds_geometry_issues_until_its_layout() -> TestResult {
    for empty in [
        Viewport::new(0.0, 300.0, 1.0)?,
        Viewport::new(400.0, 0.0, 1.0)?,
    ] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        let text = ui.signal(String::new());
        let root = ui.mount_tree(Element::new(
            "root",
            Column::new([
                collapsed_pane(&ui)?,
                Element::new("bound", TextField::new(text.clone())),
            ]),
        ))?;
        ui.flush(Duration::ZERO)?;
        assert_eq!(codes(&ui), [ValidationCode::EmptyHitArea]);
        // Until a layout runs at the new viewport, the last validation stands
        // and the report says that it is not current.
        ui.set_viewport(empty)?;
        assert_eq!(codes(&ui), [ValidationCode::EmptyHitArea]);
        assert!(ui.validation_report().pending);
        ui.flush(Duration::from_millis(1))?;
        assert_eq!(codes(&ui), []);
        assert!(!ui.validation_report().pending);
        // Updates that copy retained bounds into validation, without a layout
        // at another viewport, report nothing either.
        for value in ["a", "ab", "abc"] {
            text.set(value.to_owned())?;
            ui.flush(Duration::from_millis(2))?;
            assert_eq!(codes(&ui), []);
        }
        // A size with area validates the next layout again.
        ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
        ui.flush(Duration::from_millis(3))?;
        assert_eq!(codes(&ui), [ValidationCode::EmptyHitArea]);
        assert!(
            ui.validation_report().issues[0]
                .message
                .contains("\"right\"")
        );
        assert!(ui.find_descendant(&root, "right")?.is_some());
    }
    Ok(())
}

#[test]
fn an_empty_pane_in_a_floating_window_is_still_reported() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let window = FloatingWindow::new(&ui, "panel", "Panel", collapsed_pane(&ui)?)
        .initial_size(Size::from([400, 280]))?;
    ui.mount_tree(Element::new(
        "view",
        View::new(Element::new("windows", WindowLayer::new([window]))),
    ))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(codes(&ui), [ValidationCode::EmptyHitArea]);
    Ok(())
}

#[test]
fn rules_that_do_not_depend_on_geometry_apply_without_a_viewport() -> TestResult {
    let mut ui = Runtime::new();
    ui.mount_tree(Element::new("invalid", InvalidColor))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(codes(&ui), [ValidationCode::InvalidColor]);
    Ok(())
}

#[test]
fn each_native_window_is_validated_by_its_own_viewport() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let primary = ui.primary_window();
    ui.mount_tree(collapsed_pane(&ui)?)?;
    let other_root = ui.mount_tree(Element::new("other", Column::new([collapsed_pane(&ui)?])))?;
    let prepared = ui.prepare_detach(&other_root, NativeWindowOptions::new("Other")?)?;
    let other = ui.commit_detach(prepared)?;
    let issue_windows = |ui: &mut Runtime| -> Result<Vec<String>, RuntimeError> {
        ui.flush_windows(Duration::ZERO)?;
        Ok(issue_roots(ui))
    };
    // The detached window starts with its requested size as its viewport.
    assert_eq!(issue_windows(&mut ui)?, ["split", "other"]);
    // The primary losing its area leaves the other window's issue.
    ui.select_window(&primary)?;
    ui.set_viewport(Viewport::new(0.0, 0.0, 1.0)?)?;
    assert_eq!(issue_windows(&mut ui)?, ["other"]);
    // Validating one window while another's layout is pending leaves the
    // report pending, since that window's geometry was not checked.
    ui.select_window(&other)?;
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
    ui.select_window(&primary)?;
    ui.flush(Duration::ZERO)?;
    assert!(ui.validation_report().pending);
    ui.flush_windows(Duration::ZERO)?;
    assert!(!ui.validation_report().pending);
    // And the other way round.
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    ui.select_window(&other)?;
    ui.set_viewport(Viewport::new(0.0, 200.0, 1.0)?)?;
    assert_eq!(issue_windows(&mut ui)?, ["split"]);
    Ok(())
}
