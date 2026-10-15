//! Floating windows exercised through runtime input and real native presentation.
use crate::*;
pub(super) struct WindowSmoke {
    root: WidgetId,
    window: WidgetId,
    state: WindowController,
}
impl WindowSmoke {
    pub(super) fn new(ui: &mut Runtime) -> Result<Self, AppError> {
        let window = FloatingWindow::new(
            ui,
            "movable",
            "Inspector",
            Element::new(
                "editor",
                TextField::new(ui.signal("Retained editor".to_owned())),
            ),
        )
        .initial_size(Size::from([300, 200]))?
        .minimizable(true)
        .title_action(Element::new(
            "info",
            Button::icon(Icon::Info, "Information", |_| Ok(())).frameless(),
        ))
        .title_action(Element::new(
            "menu",
            Menu::icon(
                ui,
                Icon::MoreHorizontal,
                "More actions",
                vec![MenuItem::new("inspect", "Inspect", |_| Ok(()))],
            )?,
        ));
        let state = window.controller();
        let fixed = FloatingWindow::new(
            ui,
            "fixed",
            "Fixed palette",
            Element::new("controls", Button::new("Action", |_| Ok(()))),
        )
        .decorated(false)
        .drag_area(Element::new("drag", Label::new("Drag palette")))
        .position(Point::new(220.0, 120.0)?)?
        .fixed_size(Size::from([240, 100]))?;
        let root = ui.mount_tree(Element::new(
            "window-smoke",
            WindowLayer::new([window, fixed]),
        ))?;
        let window = ui
            .children(&root)?
            .into_iter()
            .next()
            .ok_or(RuntimeError::Unmounted)?;
        Ok(Self {
            root,
            window,
            state,
        })
    }
    fn pointer(ui: &mut Runtime, x: f32, y: f32, kind: PointerKind) -> Result<(), RuntimeError> {
        ui.pointer_input(PointerEvent::new(Point::new(x, y)?, kind))?;
        ui.flush(ui.inspect().at)?;
        Ok(())
    }
    pub(super) fn step(&self, ui: &mut Runtime, stage: u8) -> Result<(), RuntimeError> {
        match stage {
            40 => {
                Self::pointer(ui, 20.0, 15.0, PointerKind::Press(PointerButton::Primary))?;
                Self::pointer(
                    ui,
                    100.0,
                    75.0,
                    PointerKind::Release(PointerButton::Primary),
                )?;
                assert_eq!(
                    ui.bounds(&self.window)?
                        .ok_or(LayoutError::InvalidChild)?
                        .values(),
                    [80.0, 60.0, 300.0, 200.0]
                );
            }
            41 => {
                Self::pointer(ui, 374.0, 254.0, PointerKind::Press(PointerButton::Primary))?;
                Self::pointer(
                    ui,
                    424.0,
                    294.0,
                    PointerKind::Release(PointerButton::Primary),
                )?;
                assert_eq!(
                    ui.bounds(&self.window)?
                        .ok_or(LayoutError::InvalidChild)?
                        .values(),
                    [80.0, 60.0, 350.0, 240.0]
                );
            }
            42 => {
                self.state.minimize()?;
            }
            43 => {
                self.state.restore()?;
            }
            44 => {
                self.state.close()?;
            }
            _ => {}
        }
        ui.flush(ui.inspect().at)?;
        if stage == 44 {
            assert_eq!(ui.children(&self.root)?.len(), 1);
        }
        Ok(())
    }
}
