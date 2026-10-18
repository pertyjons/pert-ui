//! Public shape, panel boundary and cursor contracts.
use pert_ui::*;
use std::time::Duration;
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}
fn tint(command: &DrawCommand) -> Color {
    match command {
        DrawCommand::FillRect { color, .. } | DrawCommand::DrawMask { color, .. } => *color,
        _ => panic!("unexpected drawing command"),
    }
}
#[test]
fn groups_have_resting_borders_and_focus_within_repaints_only_ancestors() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "groups",
            Row::new([
                Element::new(
                    "group",
                    Group::new(Element::new("action", Button::new("Action", |_| Ok(())))),
                ),
                Element::new("outside", Button::new("Outside", |_| Ok(()))),
            ]),
        ))
        .unwrap();
    let children = ui.children(&root).unwrap();
    let group = &children[0];
    let button = ui.children(group).unwrap()[0].clone();
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        ui.theme().set_mode(mode).unwrap();
        let scene = draw(&mut ui);
        let palette = ui.theme().palette(mode).unwrap();
        assert!(
            scene
                .commands()
                .iter()
                .any(|c| tint(c) == palette.color(ColorRole::PanelBorder))
        );
        ui.focus(&button).unwrap();
        let focused = draw(&mut ui);
        assert!(
            focused
                .commands()
                .iter()
                .any(|c| tint(c) == palette.color(ColorRole::Primary))
        );
        assert!(
            !focused
                .commands()
                .iter()
                .any(|c| tint(c) == palette.color(ColorRole::PanelBorder))
        );
        ui.focus(&children[1]).unwrap();
        let unfocused = draw(&mut ui);
        assert!(
            unfocused
                .commands()
                .iter()
                .any(|c| tint(c) == palette.color(ColorRole::PanelBorder))
        );
        ui.clear_focus();
        draw(&mut ui);
    }
}
#[test]
fn locking_retains_geometry_but_removes_grip_and_resize_cursor() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(308.0, 100.0, 1.0).unwrap())
        .unwrap();
    let permission = ui.signal(true);
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("a", Label::new("A")),
                Element::new("b", Label::new("B")),
            )
            .resizable(permission.read_only()),
        ))
        .unwrap();
    let children = ui.children(&split).unwrap();
    for density in [Density::Compact, Density::Standard, Density::Comfortable] {
        ui.pointer_input(PointerEvent::new(
            Point::new(10.0, 30.0).unwrap(),
            PointerKind::Move,
        ))
        .unwrap();
        permission.set(true).unwrap();
        ui.theme().set_density(density).unwrap();
        let resting = draw(&mut ui);
        assert!(
            resting
                .commands()
                .iter()
                .any(|c| tint(c) == Palette::dark().color(ColorRole::ResizeHandle))
        );
        let bounds = ui.bounds(&children[1]).unwrap();
        ui.pointer_input(PointerEvent::new(
            Point::new(154.0, 30.0).unwrap(),
            PointerKind::Move,
        ))
        .unwrap();
        draw(&mut ui);
        assert_eq!(ui.cursor_icon(), CursorIcon::ResizeHorizontal);
        permission.set(false).unwrap();
        let locked = draw(&mut ui);
        assert_eq!(ui.bounds(&children[1]).unwrap(), bounds);
        assert_eq!(ui.cursor_icon(), CursorIcon::Default);
        assert!(
            locked
                .commands()
                .iter()
                .any(|c| tint(c) == Palette::dark().color(ColorRole::Divider))
        );
        assert!(
            !locked
                .commands()
                .iter()
                .any(|c| tint(c) == Palette::dark().color(ColorRole::ResizeHandle))
        );
    }
}
struct Shapes;
impl Component for Shapes {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_shape(
            Rect::new(0.0, 0.0, 40.0, 30.0)?,
            Shape::CutTopRight(10.into()),
            painter.color(ColorRole::Tab),
        )?;
        painter.stroke_shape(
            Rect::new(50.0, 0.0, 40.0, 30.0)?,
            Shape::Rounded(4.into()),
            1.into(),
            painter.color(ColorRole::PanelBorder),
        )?;
        Ok(())
    }
}
#[test]
fn shape_coverage_cuts_only_top_right_strokes_have_empty_centers_and_colors_reuse_masks() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 50.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("shapes", Shapes)).unwrap();
    let scene = draw(&mut ui);
    assert_eq!(coverage(&scene, 0.5, 0.5), 1.0);
    assert_eq!(coverage(&scene, 39.5, 0.5), 0.0);
    assert_eq!(coverage(&scene, 39.5, 29.5), 1.0);
    assert!(coverage(&scene, 30.5, 0.5) > 0.0 && coverage(&scene, 30.5, 0.5) < 1.0);
    assert_eq!(coverage(&scene, 70.5, 15.5), 0.0);
    assert_eq!(coverage(&scene, 70.5, 0.5), 1.0);
    ui.theme().set_mode(ThemeMode::Light).unwrap();
    let light = draw(&mut ui);
    for (a, b) in scene.masks().iter().zip(light.masks()) {
        assert_eq!(a.pixels().as_ptr(), b.pixels().as_ptr());
    }
    assert_ne!(scene.commands(), light.commands());
}

struct TopRounded;
impl Component for TopRounded {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        painter.fill_shape(
            Rect::new(0.0, 0.0, 40.0, 30.0)?,
            Shape::RoundedTop(6.into()),
            painter.color(ColorRole::Tab),
        )?;
        painter.stroke_shape(
            Rect::new(50.0, 0.0, 40.0, 30.0)?,
            Shape::RoundedTop(6.into()),
            2.into(),
            painter.color(ColorRole::Primary),
        )
    }
}
#[test]
fn rounded_top_shapes_round_upper_corners_and_keep_lower_corners_square() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 50.0, 1.0).unwrap())
        .unwrap();
    ui.mount_tree(Element::new("shapes", TopRounded)).unwrap();
    let scene = draw(&mut ui);
    for (x, y) in [(0.5, 0.5), (39.5, 0.5), (50.5, 0.5), (89.5, 0.5)] {
        assert_eq!(coverage(&scene, x, y), 0.0, "upper corner {x},{y}");
    }
    for (x, y) in [(0.5, 29.5), (39.5, 29.5), (50.5, 29.5), (89.5, 29.5)] {
        assert_eq!(coverage(&scene, x, y), 1.0, "lower corner {x},{y}");
    }
    assert_eq!(coverage(&scene, 20.5, 0.5), 1.0);
    assert_eq!(coverage(&scene, 70.5, 15.5), 0.0);
}

#[test]
fn separator_limits_preserve_spacing_in_constrained_parents() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(100.0, 20.0, 1.0).unwrap())
        .unwrap();
    let root = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::TopBottom,
                Element::new("separator", Divider::new()),
                Element::new("label", Label::new("Below")),
            )
            .locked(),
        ))
        .unwrap();
    let divider = ui.children(&root).unwrap()[0].clone();
    for (density, height) in [
        (Density::Compact, 17.0),
        (Density::Standard, 25.0),
        (Density::Comfortable, 33.0),
    ] {
        ui.theme().set_density(density).unwrap();
        draw(&mut ui);
        assert!(ui.bounds(&divider).unwrap().unwrap().height() >= height);
    }
}
#[test]
fn stationary_pointer_tracks_subregions_after_metric_changes_and_drag_cancels() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(408.0, 100.0, 1.0).unwrap())
        .unwrap();
    let split = ui
        .mount_tree(Element::new(
            "split",
            SplitPane::new(
                SplitAxis::LeftRight,
                Element::new("a", Label::new("A")),
                Element::new("b", Label::new("B")),
            )
            .sizing(SplitSizing::first(100.into())),
        ))
        .unwrap();
    draw(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(110.0, 30.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    draw(&mut ui);
    assert_eq!(ui.cursor_icon(), CursorIcon::Default);
    ui.theme().set_density(Density::Comfortable).unwrap();
    let scene = draw(&mut ui);
    assert_eq!(ui.cursor_icon(), CursorIcon::ResizeHorizontal);
    assert!(
        scene
            .commands()
            .iter()
            .any(|c| tint(c) == Palette::dark().color(ColorRole::ResizeHandleHover))
    );
    ui.pointer_input(PointerEvent::new(
        Point::new(105.0, 30.0).unwrap(),
        PointerKind::Press(PointerButton::Primary),
    ))
    .unwrap();
    ui.pointer_input(PointerEvent::new(
        Point::new(145.0, 30.0).unwrap(),
        PointerKind::Move,
    ))
    .unwrap();
    draw(&mut ui);
    ui.theme().set_density(Density::Compact).unwrap();
    draw(&mut ui);
    ui.pointer_input(PointerEvent::new(
        Point::new(150.0, 30.0).unwrap(),
        PointerKind::Release(PointerButton::Primary),
    ))
    .unwrap();
    draw(&mut ui);
    let first = ui.children(&split).unwrap()[0].clone();
    assert_eq!(ui.bounds(&first).unwrap().unwrap().width(), 100.0);
}

fn coverage(scene: &DisplayList, x: f32, y: f32) -> f32 {
    let inside = |rect: Rect| {
        let [a, b, w, h] = rect.values();
        x >= a && y >= b && x < a + w && y < b + h
    };
    scene.commands().iter().fold(0.0, |old, command| {
        let alpha = match command {
            DrawCommand::FillRect { rect, color } if inside(*rect) => color.alpha(),
            DrawCommand::DrawMask {
                rect,
                mask,
                color,
                clip,
                ..
            } if inside(*rect) && clip.is_none_or(inside) => {
                let [a, b, w, h] = rect.values();
                let image = scene.mask(*mask).unwrap();
                let ix = (((x - a) / w) * image.width() as f32).floor() as usize;
                let iy = (((y - b) / h) * image.height() as f32).floor() as usize;
                f32::from(image.pixels()[iy * image.width() as usize + ix]) / 255.0 * color.alpha()
            }
            _ => 0.0,
        };
        alpha + old * (1.0 - alpha)
    })
}
struct LargePanel;
impl Component for LargePanel {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let rect = Rect::new(0.0, 0.0, 6016.0, 3384.0)?;
        painter.fill_shape(
            rect,
            Shape::Rounded(4.into()),
            painter.color(ColorRole::Surface),
        )?;
        painter.stroke_shape(
            rect,
            Shape::Rounded(4.into()),
            1.into(),
            painter.color(ColorRole::PanelBorder),
        )
    }
}
#[test]
fn large_and_clipped_panels_use_small_reusable_corner_masks() {
    for size in [[6016.0, 3384.0], [100.0, 80.0]] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(size[0], size[1], 1.25).unwrap())
            .unwrap();
        ui.mount_tree(Element::new("large", LargePanel)).unwrap();
        let dark = draw(&mut ui);
        assert!(dark.masks().iter().all(|mask| mask.pixels().len() <= 100));
        ui.theme().set_mode(ThemeMode::Light).unwrap();
        let light = draw(&mut ui);
        for (a, b) in dark.masks().iter().zip(light.masks()) {
            assert_eq!(a.pixels().as_ptr(), b.pixels().as_ptr());
        }
    }
}

struct Triangles;
impl Component for Triangles {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let vertices = [
            Point::new(0.0, 0.0).unwrap(),
            Point::new(20.0, 10.0).unwrap(),
            Point::new(0.0, 20.0).unwrap(),
        ];
        painter.fill_triangle(vertices, Color::ACCENT)?;
        painter.fill_triangle([vertices[2], vertices[1], vertices[0]], Color::ACCENT)?;
        painter.with_clip(Rect::new(30.0, 0.0, 10.0, 20.0)?, |p| {
            p.fill_triangle(
                vertices.map(|v| Point::new(v.x() + 30.0, v.y()).unwrap()),
                Color::ACCENT,
            )
        })?;
        painter.fill_triangle([vertices[0]; 3], Color::ACCENT)?;
        painter.fill_triangle(
            [
                Point::new(0.0, 0.0).unwrap(),
                Point::new(1.0, 1.0).unwrap(),
                Point::new(2.0, 2.0).unwrap(),
            ],
            Color::ACCENT,
        )
    }
}
#[test]
fn public_triangles_preserve_winding_coverage_clips_and_translation_reuse() {
    for scale in [1.0, 1.25, 2.0] {
        let mut ui = Runtime::new();
        ui.set_viewport(Viewport::new(60.0, 30.0, scale).unwrap())
            .unwrap();
        ui.mount_tree(Element::new("triangles", Triangles)).unwrap();
        let scene = draw(&mut ui);
        assert_eq!(scene.commands().len(), 3);
        // Both windings and the translated copy produce equal coverage, which
        // the scene stores once and every command references.
        assert_eq!(scene.masks().len(), 1);
        assert!(scene.commands().iter().all(|command| matches!(
            command,
            DrawCommand::DrawMask { mask, .. } if mask.index() == 0
        )));
        assert!(coverage(&scene, 2.0, 10.0) > 0.99);
        assert_eq!(coverage(&scene, 19.0, 1.0), 0.0);
        assert!(coverage(&scene, 32.0, 10.0) > 0.99);
        assert_eq!(coverage(&scene, 42.0, 10.0), 0.0);
        assert!(scene.masks()[0].pixels().iter().any(|a| *a > 0 && *a < 255));
    }
}

#[test]
fn slider_thumbs_and_split_grips_are_slim_and_rounded_and_tabs_join_the_baseline() {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0).unwrap())
        .unwrap();
    let (parameter, _host) = ui
        .parameter(
            ParameterId::new(1, 9, "value").unwrap(),
            ParameterSpec::new("Value", 0.0, 1.0, 0.5).unwrap(),
        )
        .unwrap();
    let slider = ui
        .mount_tree(Element::new("slider", Slider::new(parameter)))
        .unwrap();
    let scene = draw(&mut ui);
    let [x, y, right, bottom] = mask_bounds(&scene, Color::ACCENT);
    assert!((bottom - y - 16.0 * 0.65).abs() < 0.001);
    assert!(right - x > bottom - y);
    assert!(scene.masks().iter().any(|m| m.pixels().contains(&0)));
    ui.unmount(&slider).unwrap();
    for axis in [SplitAxis::LeftRight, SplitAxis::TopBottom] {
        let split = ui
            .mount_tree(Element::new(
                "split",
                SplitPane::new(
                    axis,
                    Element::new("a", Container::default()),
                    Element::new("b", Container::default()),
                ),
            ))
            .unwrap();
        let scene = draw(&mut ui);
        let [x, y, right, bottom] =
            mask_bounds(&scene, Palette::dark().color(ColorRole::ResizeHandle));
        let (w, h) = (right - x, bottom - y);
        assert_eq!(
            (w, h),
            if axis == SplitAxis::LeftRight {
                (3.0, 24.0)
            } else {
                (24.0, 3.0)
            }
        );
        assert!(
            scene
                .masks()
                .iter()
                .any(|m| m.pixels().iter().any(|a| *a > 0 && *a < 255))
        );
        ui.unmount(&split).unwrap();
    }
    ui.mount_tree(Element::new(
        "tabs",
        Tabs::new([
            Tab::new("a", "A", Element::new("a", Container::default())),
            Tab::new("b", "B", Element::new("b", Container::default())),
        ]),
    ))
    .unwrap();
    let scene = draw(&mut ui);
    assert!(scene.commands().iter().any(|c| matches!(c, DrawCommand::FillRect { rect, color } if *color == Palette::dark().color(ColorRole::Divider) && rect.values() == [0.0, 35.0, 300.0, 1.0])));
}

fn mask_bounds(scene: &DisplayList, tint: Color) -> [f32; 4] {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            DrawCommand::DrawMask {
                rect, color, clip, ..
            } if *color == tint => Some(clip.map_or(*rect, |clip| rect.intersection(clip))),
            _ => None,
        })
        .fold(
            [
                f32::INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NEG_INFINITY,
            ],
            |b, rect| {
                let [x, y, w, h] = rect.values();
                [b[0].min(x), b[1].min(y), b[2].max(x + w), b[3].max(y + h)]
            },
        )
}

#[test]
fn split_grips_extend_equally_in_physical_pixels_beyond_both_line_edges() {
    for axis in [SplitAxis::LeftRight, SplitAxis::TopBottom] {
        for scale in [0.75, 1.0, 1.25, 1.5, 2.0] {
            for width in [300.0, 301.0] {
                let mut ui = Runtime::new();
                ui.set_viewport(Viewport::new(width, 101.0, scale).unwrap())
                    .unwrap();
                ui.mount_tree(Element::new(
                    "split",
                    SplitPane::new(
                        axis,
                        Element::new("a", Container::default()),
                        Element::new("b", Container::default()),
                    ),
                ))
                .unwrap();
                let scene = draw(&mut ui);
                let color = Palette::dark().color(ColorRole::ResizeHandle);
                let grip = mask_bounds(&scene, color);
                let line = scene
                    .commands()
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, color: tint }
                            if *tint == color
                                && (if axis == SplitAxis::LeftRight {
                                    rect.height() == 101.0
                                } else {
                                    rect.width() == width
                                }) =>
                        {
                            Some(rect.values())
                        }
                        _ => None,
                    })
                    .unwrap();
                let dim = usize::from(axis == SplitAxis::TopBottom);
                let before = f64::from(line[dim] - grip[dim]) * scale;
                let after = f64::from(grip[dim + 2] - (line[dim] + line[dim + 2])) * scale;
                assert!((before - after).abs() < 0.0001);
                assert!((before - before.round()).abs() < 0.0001);
                assert!(before >= 0.9999);
            }
        }
    }
}
