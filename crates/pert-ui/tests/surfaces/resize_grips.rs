//! The two floating frame families use the same corner resize mark.
use pert_ui::*;
use std::time::Duration;

fn triangle(scene: &DisplayList, color: Color) -> Rect {
    let marks: Vec<_> = scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawVector {
                vector,
                color: tint,
                ..
            } if *tint == color => scene.vector(*vector),
            _ => None,
        })
        .collect();
    assert_eq!(marks.len(), 1, "one resize mark per floating frame");
    let path = marks[0].path();
    let [
        PathElement::Move(left),
        PathElement::Line(top),
        PathElement::Line(corner),
        PathElement::Close,
    ] = path.elements()
    else {
        panic!("the resize mark is a closed triangle");
    };
    assert_eq!(left.y(), corner.y());
    assert_eq!(top.x(), corner.x());
    assert!(left.x() < corner.x() && top.y() < corner.y());
    assert!(path.bounds().width() <= 8.0);
    path.bounds()
}

#[test]
fn floating_window_and_tool_panel_share_the_inset_triangle()
-> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let window = FloatingWindow::new(
        &ui,
        "window",
        "Window",
        Element::new("body", Label::new("Content")),
    )
    .initial_size(Size::from([300, 200]))?;
    ui.mount_tree(Element::new("layer", WindowLayer::new([window])))?;
    ui.flush(Duration::ZERO)?;
    let color = ui.theme().palette(ThemeMode::Dark)?.color(ColorRole::Muted);
    let window_triangle = triangle(&ui.paint()?, color);

    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 600.0, 1.0)?)?;
    let layout = ToolWindowLayout::new(Element::new("center", Label::new("Center")))
        .edge(ToolEdge::Left, EdgeMode::Stripe)
        .window(
            ToolWindow::new("tool", "Tool", Element::new("body", Label::new("Content")))
                .expanded(true)
                .mode(ToolWindowMode::Floating),
        );
    ui.mount_tree(Element::new("layout", layout))?;
    ui.flush(Duration::ZERO)?;
    let tool_triangle = triangle(&ui.paint()?, color);
    assert_eq!(window_triangle.width(), tool_triangle.width());
    assert_eq!(window_triangle.height(), tool_triangle.height());
    Ok(())
}
