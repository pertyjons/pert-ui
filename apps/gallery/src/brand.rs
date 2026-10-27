//! The pert-ui mark at the left of the gallery's menu bar, a consumer-authored
//! button built on the public component contract.
use pert_ui::*;

/// The compact mark for light backgrounds, drawn in the light theme.
const ICON_LIGHT: &[u8] = include_bytes!("../../../assets/brand/pert-ui-icon.svg");
/// The compact mark for dark backgrounds, drawn in the dark theme.
const ICON_DARK: &[u8] = include_bytes!("../../../assets/brand/pert-ui-icon-dark.svg");

/// The pert-ui mark as an activatable button that opens the About dialog.
///
/// Both theme variants are mounted once as children; the theme mode decides
/// which one is visible, so a mode change repaints the mark without
/// describing the menu bar again. The mark is `IconSize` inside a
/// `ControlHeight` square, like an icon button. A primary click released over
/// the mark, or Enter or Space while it has focus, opens the dialog;
/// releasing elsewhere, focus loss and hiding cancel a press.
pub(crate) struct BrandMark {
    children: Vec<Element>,
    mode: ReadSignal<ThemeMode>,
    dark: bool,
    about: DialogController,
    pressed: bool,
    hovered: bool,
    focused: bool,
}
impl BrandMark {
    /// The mark following `ui`'s theme mode, opening `about` when activated.
    pub(crate) fn new(ui: &Runtime, about: DialogController) -> Result<Self, RuntimeError> {
        let mark = |key: &str, source: &[u8]| -> Result<Element, RuntimeError> {
            let svg = Svg::new(source)
                .map_err(|error| RuntimeError::from(ApplicationError::new(error)))?
                .description("pert-ui");
            Ok(Element::new(key, svg))
        };
        let mode = ui.theme().mode_binding()?;
        Ok(Self {
            children: vec![mark("light", ICON_LIGHT)?, mark("dark", ICON_DARK)?],
            dark: mode.get()? == ThemeMode::Dark,
            mode,
            about,
            pressed: false,
            hovered: false,
            focused: false,
        })
    }
    /// Opens About as the menu's About title does: the previous session's
    /// outcome is only acknowledged, and the next open needs it gone.
    fn open_about(&self) -> Result<(), RuntimeError> {
        self.about.take_outcome()?;
        self.about.open()
    }
    fn release(&mut self, cx: &mut EventContext) {
        if self.pressed {
            self.pressed = false;
            cx.release_pointer();
            cx.invalidate(Invalidation::Paint);
        }
    }
}
impl Component for BrandMark {
    fn take_children(&mut self) -> Vec<Element> {
        std::mem::take(&mut self.children)
    }
    fn child_visible(&self, key: &str) -> bool {
        (key == "dark") == self.dark
    }
    fn focusable(&self) -> bool {
        true
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        let dark = cx.read(&self.mode)? == ThemeMode::Dark;
        if dark != self.dark {
            self.dark = dark;
            cx.request_layout();
        }
        self.focused = cx.focused();
        Ok(())
    }
    fn measure(&self, cx: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        let side = cx.metric(MetricRole::ControlHeight);
        Ok(Size::new(side, side)?)
    }
    fn arrange(&self, cx: &mut ArrangeContext<'_>, size: Size) -> Result<(), RuntimeError> {
        let icon = cx.measurement().metric(MetricRole::IconSize);
        let x = (size.width() - icon) / 2.0;
        let y = (size.height() - icon) / 2.0;
        // Layout sees only the visible variant.
        for index in 0..cx.measurement().len() {
            cx.place(index, Rect::new(x, y, icon, icon)?)?;
        }
        Ok(())
    }
    fn hover_changed(&mut self, cx: &mut EventContext, hovered: bool) {
        self.hovered = hovered;
        cx.invalidate(Invalidation::Paint);
    }
    fn pointer_input(
        &mut self,
        cx: &mut EventContext,
        event: PointerEvent,
    ) -> Result<InputOutcome, RuntimeError> {
        match event.kind() {
            PointerKind::Press(PointerButton::Primary) => {
                self.pressed = true;
                cx.capture_pointer()?;
                cx.invalidate(Invalidation::Paint);
                Ok(InputOutcome::Handled)
            }
            PointerKind::Release(PointerButton::Primary) if self.pressed => {
                self.release(cx);
                let inside = cx.size().is_some_and(|size| {
                    let point = event.position();
                    (0.0..=size.width()).contains(&point.x())
                        && (0.0..=size.height()).contains(&point.y())
                });
                if inside && !event.is_outside() {
                    self.open_about()?;
                }
                Ok(InputOutcome::Handled)
            }
            _ => Ok(InputOutcome::Ignored),
        }
    }
    fn keyboard_event(
        &mut self,
        _: &mut EventContext,
        event: KeyEvent,
        keyboard: &KeyboardState,
    ) -> Result<InputOutcome, RuntimeError> {
        let activates = matches!(event.code, KeyCode::Enter | KeyCode::Space)
            && keyboard.modifiers() == Modifiers::default();
        if !activates {
            return Ok(InputOutcome::Ignored);
        }
        if event.transition == KeyTransition::Pressed {
            self.open_about()?;
        }
        Ok(InputOutcome::Handled)
    }
    fn input_reset(&mut self) {
        self.pressed = false;
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        let size = painter.size();
        let rect = Rect::new(0.0, 0.0, size.width(), size.height())?;
        let shape = Shape::Rounded(Length::new(painter.metric(MetricRole::CornerRadius))?);
        if self.pressed || self.hovered {
            painter.fill_shape(rect, shape, painter.color(ColorRole::SurfaceHover))?;
        }
        if self.focused {
            painter.stroke_shape(
                rect,
                shape,
                Length::new(painter.metric(MetricRole::FocusWidth))?,
                painter.color(ColorRole::FocusRing),
            )?;
        }
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        let mut details = ComponentDetails::default();
        details.properties.insert(
            "variant".into(),
            InspectValue::Text(if self.dark { "dark" } else { "light" }.into()),
        );
        details.semantics = Some(
            Semantics::new(SemanticRole::Button)
                .name("pert-ui")
                .description("Opens About pert-ui")
                .action(SemanticAction::Activate),
        );
        details
    }
}
