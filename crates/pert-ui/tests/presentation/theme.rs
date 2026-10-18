//! Public theme management, role dependencies and retained widget behavior.
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

struct Swatch {
    role: ColorRole,
    source: ReadSignal<ColorRole>,
    paints: Rc<Cell<usize>>,
    foreground: bool,
    fail: Rc<Cell<bool>>,
}
impl Component for Swatch {
    fn update(&mut self, cx: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        self.role = cx.read_paint(&self.source)?;
        Ok(())
    }
    fn paint(&self, painter: &mut Painter) -> std::result::Result<(), DrawError> {
        self.paints.set(self.paints.get() + 1);
        if !self.foreground {
            painter.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0)?, painter.color(self.role));
        }
        if self.fail.get() {
            return Err(DrawError::InvalidColor);
        }
        Ok(())
    }
    fn paint_foreground(&self, painter: &mut Painter) -> std::result::Result<(), DrawError> {
        if self.foreground {
            painter.fill_rect(Rect::new(0.0, 0.0, 10.0, 10.0)?, painter.color(self.role));
        }
        Ok(())
    }
}
type MountedSwatch = (WidgetId, Signal<ColorRole>, Rc<Cell<usize>>, Rc<Cell<bool>>);

fn mount(ui: &mut Runtime, key: &str, role: ColorRole, foreground: bool) -> MountedSwatch {
    let signal = ui.signal(role);
    let paints = Rc::new(Cell::new(0));
    let fail = Rc::new(Cell::new(false));
    let id = ui
        .mount(
            key,
            Swatch {
                role,
                source: signal.read_only(),
                paints: paints.clone(),
                foreground,
                fail: fail.clone(),
            },
        )
        .unwrap();
    (id, signal, paints, fail)
}
fn draw(ui: &mut Runtime) -> DisplayList {
    ui.flush(Duration::ZERO).unwrap();
    ui.paint().unwrap()
}

#[test]
fn overrides_survive_mode_switches_and_inactive_edits_do_no_work() -> Result {
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let (_, _, paints, _) = mount(&mut ui, "primary", ColorRole::Primary, false);
    draw(&mut ui);
    let custom = Color::rgba(0.5, 0.1, 0.4, 1.0)?;
    assert_eq!(theme.mode()?, ThemeMode::Dark);
    assert!(theme.set_color(ThemeMode::Light, ColorRole::Primary, custom)?);
    assert!(!ui.needs_paint());
    assert_eq!(ui.next_deadline(), None);
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(theme.set_mode(ThemeMode::Light)?);
    let light = draw(&mut ui);
    assert!(matches!(light.commands()[0], DrawCommand::FillRect { color, .. } if color == custom));
    assert_eq!(paints.get(), 2);
    assert!(!theme.set_mode(ThemeMode::Light)?);
    assert!(!theme.set_color(ThemeMode::Light, ColorRole::Primary, custom)?);
    assert!(!ui.needs_paint());
    theme.set_mode(ThemeMode::Dark)?;
    draw(&mut ui);
    assert_eq!(theme.palette(ThemeMode::Dark)?, Palette::dark());
    theme.set_mode(ThemeMode::Light)?;
    assert_eq!(draw(&mut ui), light);
    theme.set_palette(ThemeMode::Light, Palette::light())?;
    assert_eq!(theme.palette(ThemeMode::Light)?, Palette::light());
    Ok(())
}

#[test]
fn only_read_roles_invalidate_and_dependencies_are_recollected() -> Result {
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let (_, role, first, _) = mount(&mut ui, "first", ColorRole::Primary, false);
    let (_, _, second, _) = mount(&mut ui, "second", ColorRole::Secondary, true);
    draw(&mut ui);
    theme.set_color(ThemeMode::Dark, ColorRole::Primary, Color::MUTED)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    ui.paint()?;
    assert_eq!((first.get(), second.get()), (2, 1));
    role.set(ColorRole::Success)?;
    draw(&mut ui);
    theme.set_color(ThemeMode::Dark, ColorRole::Primary, Color::TEXT)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(!ui.needs_paint());
    theme.set_color(ThemeMode::Dark, ColorRole::Secondary, Color::MUTED)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    ui.paint()?;
    assert_eq!((first.get(), second.get()), (3, 2));
    Ok(())
}

#[test]
fn hidden_and_removed_painters_do_not_schedule_theme_work() -> Result {
    let mut ui = Runtime::new();
    let (id, _, paints, _) = mount(&mut ui, "swatch", ColorRole::Primary, false);
    let theme = ui.theme();
    draw(&mut ui);
    ui.set_visible(&id, false)?;
    draw(&mut ui);
    theme.set_color(ThemeMode::Dark, ColorRole::Primary, Color::MUTED)?;
    assert!(!ui.needs_paint());
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert_eq!(ui.next_deadline(), None);
    ui.set_visible(&id, true)?;
    let scene = draw(&mut ui);
    assert_eq!(paints.get(), 2);
    assert!(matches!(
        scene.commands()[0],
        DrawCommand::FillRect {
            color: Color::MUTED,
            ..
        }
    ));
    ui.unmount(&id)?;
    draw(&mut ui);
    theme.set_color(ThemeMode::Dark, ColorRole::Primary, Color::TEXT)?;
    assert!(!ui.needs_paint());
    assert_eq!(ui.next_deadline(), None);
    // Retaining a stale identity cannot keep the subscription active.
    assert_eq!(
        ui.invalidate(&id, Invalidation::Paint),
        Err(RuntimeError::Unmounted)
    );
    Ok(())
}

#[test]
fn failed_paint_keeps_damage_and_retries_with_the_latest_palette() -> Result {
    let mut ui = Runtime::new();
    let (_, role, _, fail) = mount(&mut ui, "swatch", ColorRole::Primary, false);
    draw(&mut ui);
    role.set(ColorRole::Secondary)?;
    fail.set(true);
    ui.flush(Duration::ZERO)?;
    assert_eq!(ui.paint(), Err(DrawError::InvalidColor));
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::Secondary, Color::TEXT)?;
    assert!(ui.needs_paint());
    fail.set(false);
    assert!(matches!(
        draw(&mut ui).commands()[0],
        DrawCommand::FillRect {
            color: Color::TEXT,
            ..
        }
    ));
    assert!(!ui.needs_paint());
    Ok(())
}

#[test]
fn background_is_owned_scene_data_and_changes_without_components() -> Result {
    let mut ui = Runtime::new();
    let old = ui.paint()?;
    assert_eq!(old.background(), Color::BACKGROUND);
    ui.theme().set_mode(ThemeMode::Light)?;
    assert!(ui.needs_paint());
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    let light = ui.paint()?;
    assert_eq!(
        light.background(),
        Palette::light().color(ColorRole::Background)
    );
    assert!(light.is_empty());
    assert!(!ui.needs_paint());
    assert_ne!(old, light);
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0)?)?;
    assert!(ui.needs_paint());
    assert_eq!(ui.paint()?.viewport().width(), 320.0);
    assert!(!ui.needs_paint());
    drop(ui);
    assert_eq!(old.background(), Color::BACKGROUND);
    Ok(())
}

#[test]
fn runtime_isolation_shutdown_and_validation() -> Result {
    let mut first = Runtime::new();
    let mut second = Runtime::new();
    let theme = first.theme();
    theme.set_mode(ThemeMode::Light)?;
    assert_ne!(first.paint()?.background(), second.paint()?.background());
    assert_eq!(
        Color::rgba(f32::NAN, 0.0, 0.0, 1.0),
        Err(DrawError::InvalidColor)
    );
    assert_eq!(
        Color::rgba(2.0, 0.0, 0.0, 1.0),
        Err(DrawError::InvalidColor)
    );
    drop(first);
    assert_eq!(theme.mode(), Err(RuntimeError::Closed));
    assert_eq!(theme.set_mode(ThemeMode::Dark), Err(RuntimeError::Closed));
    assert_eq!(
        theme.set_palette(ThemeMode::Dark, Palette::dark()),
        Err(RuntimeError::Closed)
    );
    Ok(())
}

struct Lifecycle(ThemeManager);
impl Component for Lifecycle {
    fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        assert_eq!(
            self.0.set_mode(ThemeMode::Light),
            Err(RuntimeError::WriteDuringUpdate)
        );
        Ok(())
    }
    fn measure(
        &self,
        _: &MeasureContext<'_>,
        _: Constraints,
    ) -> std::result::Result<Size, RuntimeError> {
        assert_eq!(
            self.0.set_mode(ThemeMode::Light),
            Err(RuntimeError::WriteDuringLayout)
        );
        Ok(Size::from([20, 20]))
    }
    fn paint(&self, _: &mut Painter) -> std::result::Result<(), DrawError> {
        assert_eq!(
            self.0.set_mode(ThemeMode::Light),
            Err(RuntimeError::WriteDuringPaint)
        );
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        assert_eq!(
            self.0.set_mode(ThemeMode::Light),
            Err(RuntimeError::WriteDuringInspection)
        );
        ComponentDetails::default()
    }
}
#[test]
fn theme_mutations_respect_lifecycle_phases() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    ui.mount_tree(Element::new("lifecycle", Lifecycle(ui.theme())))?;
    draw(&mut ui);
    let _ = ui.inspect();
    assert_eq!(ui.theme().mode()?, ThemeMode::Dark);
    Ok(())
}

#[test]
fn standard_widgets_recolor_without_new_layout_or_text_masks() -> Result {
    let mut ui = Runtime::new();
    ui.set_profiling(true)?;
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "group",
        Group::new(Element::new(
            "row",
            Row::new([
                Element::new("label", Label::new("Themed")),
                Element::new("button", Button::new("Action", |_| Ok(()))),
                Element::new("literal", Label::new("Fixed").color(Color::ACCENT)),
            ]),
        )),
    ))?;
    let dark = draw(&mut ui);
    ui.clear_profile()?;
    ui.theme().set_mode(ThemeMode::Light)?;
    let light = draw(&mut ui);
    assert_ne!(dark.commands(), light.commands());
    assert_eq!(dark.masks().len(), light.masks().len());
    for (a, b) in dark.masks().iter().zip(light.masks()) {
        assert_eq!(a.pixels().as_ptr(), b.pixels().as_ptr());
    }
    assert!(light.commands().iter().any(|cmd| matches!(
        cmd,
        DrawCommand::DrawMask {
            color: Color::ACCENT,
            ..
        }
    )));
    assert!(light.commands().iter().any(|cmd| matches!(cmd, DrawCommand::DrawMask { color, .. } if *color == Palette::light().color(ColorRole::Surface))));
    for stat in ui.profile().stats {
        assert!(!matches!(
            stat.operation,
            ProfileOperation::Measure | ProfileOperation::Arrange | ProfileOperation::Text
        ));
    }
    assert_eq!(ui.next_deadline(), None);
    assert!(!ui.needs_paint());
    Ok(())
}

#[test]
fn scrollbars_follow_role_overrides_without_remeasuring_content() -> Result {
    let mut ui = Runtime::new();
    ui.set_profiling(true)?;
    ui.set_viewport(Viewport::new(100.0, 100.0, 1.0)?)?;
    ui.mount_tree(Element::new(
        "scroll",
        ScrollArea::new(Element::new(
            "content",
            Container::default().size_limits(SizeLimits::at_least(Size::from([800, 600]))),
        ))
        .axes(ScrollAxes::Both),
    ))?;
    let before = draw(&mut ui);
    let resting = Palette::dark().color(ColorRole::ScrollbarThumb);
    assert_eq!(
        before
            .commands()
            .iter()
            .filter(|cmd| matches!(
                cmd,
                DrawCommand::FillRect { color, .. } if *color == resting
            ))
            .count(),
        2
    );
    ui.clear_profile()?;
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::ScrollbarThumb, Color::ACCENT)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    let after = ui.paint()?;
    assert_eq!(
        after
            .commands()
            .iter()
            .filter(|cmd| matches!(
                cmd,
                DrawCommand::FillRect {
                    color: Color::ACCENT,
                    ..
                }
            ))
            .count(),
        2
    );
    assert!(ui.profile().stats.iter().all(|stat| !matches!(
        stat.operation,
        ProfileOperation::Measure | ProfileOperation::Arrange
    )));
    Ok(())
}

/// The public WCAG measurement; every role this test compares is opaque in
/// both built-in palettes, so a translucent one fails the test.
fn contrast(a: Color, b: Color) -> std::result::Result<f32, TranslucentColorError> {
    Ok(OpaqueColor::try_from(a)?.contrast_ratio(OpaqueColor::try_from(b)?))
}

#[test]
fn built_in_palettes_keep_readable_foregrounds_and_visible_outlines() -> Result {
    assert_readable("dark", &Palette::dark())?;
    assert_readable("light", &Palette::light())
}

/// The readability every built-in and seeded palette keeps: text and
/// supporting contrast pairs, container and selection separation, and a
/// highlight that differs from body text in hue.
fn assert_readable(mode: &str, palette: &Palette) -> Result {
    use ColorRole::*;
    let text = [
        (Primary, OnPrimary),
        (Secondary, OnSecondary),
        (Error, OnError),
        (Warning, OnWarning),
        (Info, OnInfo),
        (Success, OnSuccess),
        (Background, OnSurface),
        (Surface, OnSurface),
        (SurfaceHover, OnSurface),
        (Button, OnButton),
        (ButtonHover, OnButton),
        (ButtonPressed, OnButtonPressed),
        (Tab, OnTab),
        (TabSelected, OnTab),
        (SurfaceContainer, OnSurfaceContainer),
        (PrimaryContainer, OnPrimaryContainer),
        (SecondaryContainer, OnSecondaryContainer),
        (ErrorContainer, OnErrorContainer),
        (WarningContainer, OnWarningContainer),
        (InfoContainer, OnInfoContainer),
        (SuccessContainer, OnSuccessContainer),
        (TextSelection, OnTextSelection),
        (Surface, Link),
        (Background, Link),
        (Surface, Highlight),
        (SurfaceHover, Highlight),
        (Background, Highlight),
        (Surface, Muted),
        (Background, Muted),
        (SurfaceHover, Muted),
        (Background, Error),
        (Surface, Error),
        (SurfaceContainer, Error),
    ];
    // Only non-text component boundaries and focus indicators use the 3:1
    // minimum. The single-color focus ring borders the surroundings.
    let supporting = [
        (Surface, Outline),
        (Surface, Primary),
        (Surface, Error),
        (Background, FocusRing),
        (Surface, FocusRing),
    ];
    for (minimum, pairs) in [(4.5, &text[..]), (3.0, &supporting[..])] {
        for (fill, foreground) in pairs {
            let ratio = contrast(palette.color(*fill), palette.color(*foreground))?;
            assert!(
                ratio >= minimum,
                "{mode} {foreground:?} on {fill:?}: {ratio:.2} < {minimum}"
            );
        }
    }
    assert!(
        contrast(palette.color(SurfaceContainer), palette.color(Button))? > 1.2,
        "{mode} static containers must not read as buttons"
    );
    assert!(
        contrast(palette.color(SurfaceContainer), palette.color(Surface))? > 1.2,
        "{mode} static containers must stand out from their panel"
    );
    // A selection sits inside ordinary body text, so its foreground must stay
    // close to the unselected text rather than merely clearing the text minimum.
    assert!(
        contrast(palette.color(TextSelection), palette.color(OnTextSelection))? >= 6.0,
        "{mode} selected text must stay as readable as the text around it"
    );
    assert!(
        contrast(palette.color(TextSelection), palette.color(Surface))? > 1.2,
        "{mode} a selection must be visible against the field it sits in"
    );
    // A link stands beside ordinary text and must not read as it.
    assert!(
        contrast(palette.color(Link), palette.color(OnSurface))? > 1.5,
        "{mode} link text must differ from body text"
    );
    // A highlight sits inside body text and differs from it in hue, which
    // luminance contrast alone does not measure.
    let [hr, hg, hb, _] = palette.color(Highlight).channels();
    let [tr, tg, tb, _] = palette.color(OnSurface).channels();
    assert!(
        (hr - tr).abs().max((hg - tg).abs()).max((hb - tb).abs()) > 0.2,
        "{mode} highlighted text must differ from body text"
    );
    Ok(())
}

/// The documented default sources: each follower and the role it follows.
const FOLLOWERS: [(ColorRole, ColorRole); 13] = {
    use ColorRole::*;
    [
        (OnButton, OnSurface),
        (OnTab, OnSurface),
        (OnTextSelection, OnSurface),
        (OnSurfaceContainer, OnSurface),
        (OnPrimaryContainer, OnSurface),
        (OnSecondaryContainer, OnSurface),
        (OnErrorContainer, OnSurface),
        (OnWarningContainer, OnSurface),
        (OnInfoContainer, OnSurface),
        (OnSuccessContainer, OnSurface),
        (PanelBorder, Divider),
        (ButtonPressed, Primary),
        (OnButtonPressed, OnPrimary),
    ]
};

#[test]
fn followers_name_their_sources_and_match_them_in_built_in_palettes() {
    use ColorRole::*;
    for (follower, source) in FOLLOWERS {
        assert_eq!(follower.follows(), Some(source), "{follower:?}");
        assert_eq!(source.follows(), None, "{source:?} has its own value");
        for palette in [Palette::dark(), Palette::light()] {
            assert_eq!(palette.color(follower), palette.color(source));
        }
    }
    // Roles whose fills vary independently of the accent keep their own values.
    for role in [
        FocusRing,
        FocusRingContrast,
        OnSecondary,
        OnError,
        OnWarning,
        OnInfo,
        OnSuccess,
    ] {
        assert_eq!(role.follows(), None, "{role:?}");
    }
}

#[test]
fn followers_track_source_overrides_until_overridden_themselves() -> Result {
    use ColorRole::*;
    let custom = Color::rgba(0.5, 0.1, 0.4, 1.0)?;
    for defaults in [Palette::dark(), Palette::light()] {
        for (follower, source) in FOLLOWERS {
            let palette = defaults.with_color(source, custom);
            assert_eq!(palette.color(follower), custom, "{follower:?}");
        }
        // Overriding OnPrimary leaves foregrounds on independent fills alone.
        let palette = defaults.with_color(OnPrimary, custom);
        for role in [FocusRingContrast, OnSecondary, OnError, OnInfo, OnSuccess] {
            assert_eq!(palette.color(role), defaults.color(role), "{role:?}");
        }
        // A follower override equal to its source's value at the time is kept.
        let pressed = defaults.color(Primary);
        let palette = defaults
            .with_color(ButtonPressed, pressed)
            .with_color(Primary, custom);
        assert_eq!(palette.color(ButtonPressed), pressed);
        assert_ne!(defaults.with_color(ButtonPressed, pressed), defaults);
        // Removing it resumes following.
        assert_eq!(
            palette
                .with_default_color(ButtonPressed)
                .color(ButtonPressed),
            custom
        );
        // Removing an own-value override restores the palette mode's value.
        let palette = defaults.with_color(Primary, custom);
        assert_eq!(palette.with_default_color(Primary), defaults);
        assert_eq!(defaults.with_default_color(Primary), defaults);
    }
    // A light-based palette keeps light defaults in the dark slot.
    let ui = Runtime::new();
    let theme = ui.theme();
    theme.set_palette(
        ThemeMode::Dark,
        Palette::light()
            .with_color(Primary, custom)
            .with_default_color(Primary),
    )?;
    assert_eq!(
        theme.palette(ThemeMode::Dark)?.color(Primary),
        Palette::light().color(Primary)
    );
    Ok(())
}

#[test]
fn a_source_override_repaints_only_the_readers_of_changed_followers() -> Result {
    use ColorRole::*;
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let (_, _, pressed, _) = mount(&mut ui, "pressed", ButtonPressed, false);
    let (_, _, focus, _) = mount(&mut ui, "focus", FocusRing, false);
    let (_, _, secondary, _) = mount(&mut ui, "secondary", Secondary, true);
    draw(&mut ui);
    let accent = Color::rgba(0.5, 0.1, 0.4, 1.0)?;
    theme.set_color(ThemeMode::Dark, Primary, accent)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    ui.paint()?;
    assert_eq!((pressed.get(), focus.get(), secondary.get()), (2, 1, 1));
    // An override equal to the resolved color changes stored state only.
    assert!(theme.set_color(ThemeMode::Dark, ButtonPressed, accent)?);
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(!ui.needs_paint());
    // The explicit follower no longer tracks Primary.
    theme.set_color(ThemeMode::Dark, Primary, Color::MUTED)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 0);
    assert!(!ui.needs_paint());
    // Resuming following repaints its reader with the current source.
    let palette = theme
        .palette(ThemeMode::Dark)?
        .with_default_color(ButtonPressed);
    theme.set_palette(ThemeMode::Dark, palette)?;
    assert_eq!(ui.flush(Duration::ZERO)?.components_updated, 1);
    let scene = ui.paint()?;
    assert!(
        matches!(scene.commands()[0], DrawCommand::FillRect { color, .. } if color == Color::MUTED)
    );
    assert_eq!((pressed.get(), focus.get(), secondary.get()), (3, 1, 1));
    Ok(())
}

#[test]
fn ordinary_text_follows_an_on_surface_override() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(640.0, 400.0, 1.0)?)?;
    ui.mount_tree(Element::new("button", Button::new("Action", |_| Ok(()))))?;
    draw(&mut ui);
    let text = Color::rgba(0.9, 0.6, 0.2, 1.0)?;
    ui.theme()
        .set_color(ThemeMode::Dark, ColorRole::OnSurface, text)?;
    let scene = draw(&mut ui);
    assert!(scene.commands().iter().any(|command| matches!(
        command,
        DrawCommand::DrawMask { color, .. } if *color == text
    )));
    Ok(())
}

fn opaque(
    red: f32,
    green: f32,
    blue: f32,
) -> std::result::Result<OpaqueColor, Box<dyn std::error::Error>> {
    Ok(OpaqueColor::try_from(Color::rgba(red, green, blue, 1.0)?)?)
}
/// Seeds across the linear RGB cube, from black and white through grays to
/// every saturated corner, with and without a secondary seed, keep every
/// readability check of the built-in palettes in both modes.
#[test]
fn every_seeded_palette_keeps_the_built_in_readability() -> Result {
    const LEVELS: [f32; 6] = [0.0, 0.02, 0.1, 0.3, 0.6, 1.0];
    for red in LEVELS {
        for green in LEVELS {
            for blue in LEVELS {
                let primary = opaque(red, green, blue)?;
                let secondary = opaque(blue, red, green)?;
                for mode in [ThemeMode::Dark, ThemeMode::Light] {
                    for secondary in [None, Some(secondary)] {
                        let palette = Palette::from_seeds(mode, primary, secondary);
                        let name = format!("{mode:?} seed ({red}, {green}, {blue}) {secondary:?}");
                        assert_readable(&name, &palette)?;
                        // Fixed roles stay built in, and translucent roles keep their alpha.
                        let built_in = match mode {
                            ThemeMode::Dark => Palette::dark(),
                            ThemeMode::Light => Palette::light(),
                        };
                        for role in [ColorRole::Error, ColorRole::OnPrimary, ColorRole::Highlight] {
                            assert_eq!(
                                palette.color(role),
                                built_in.color(role),
                                "{name} {role:?}"
                            );
                        }
                        assert_eq!(
                            palette.color(ColorRole::ScrollbarThumb).alpha(),
                            built_in.color(ColorRole::ScrollbarThumb).alpha()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn a_seed_sets_the_accent_hue_and_a_secondary_seed_its_own() -> Result {
    use ColorRole::*;
    let red = opaque(0.8, 0.05, 0.05)?;
    let green = opaque(0.05, 0.6, 0.1)?;
    let palette = Palette::from_seeds(ThemeMode::Dark, red, Some(green));
    let [r, g, b, _] = palette.color(Primary).channels();
    assert!(r > g && r > b, "a red seed gives a red accent: {r} {g} {b}");
    let [r, g, b, _] = palette.color(Secondary).channels();
    assert!(
        g > r && g > b,
        "a green secondary seed gives a green secondary: {r} {g} {b}"
    );
    // Neutrals take the seed hue, and a gray seed leaves them gray.
    let [r, _, b, _] = palette.color(Background).channels();
    assert!(r > b, "the background leans toward the seed");
    let gray = Palette::from_seeds(ThemeMode::Dark, opaque(0.3, 0.3, 0.3)?, None);
    let [r, g, b, _] = gray.color(Surface).channels();
    assert!((r - g).abs() < 1e-3 && (g - b).abs() < 1e-3);
    Ok(())
}

#[test]
fn overrides_and_followers_work_on_a_seeded_palette() -> Result {
    use ColorRole::*;
    let seeded = Palette::from_seeds(ThemeMode::Light, opaque(0.05, 0.5, 0.2)?, None);
    assert_eq!(seeded.color(ButtonPressed), seeded.color(Primary));
    assert_eq!(seeded.color(OnSurfaceContainer), seeded.color(OnSurface));
    let accent = Color::rgba(0.4, 0.1, 0.5, 1.0)?;
    let overridden = seeded.with_color(Primary, accent);
    assert_eq!(overridden.color(ButtonPressed), accent);
    assert_ne!(overridden, seeded);
    assert_eq!(overridden.with_default_color(Primary), seeded);
    assert_eq!(
        overridden.with_default_color(Primary).color(Primary),
        seeded.color(Primary)
    );
    // A different seed is a different palette even before any override.
    let other = Palette::from_seeds(ThemeMode::Light, opaque(0.5, 0.05, 0.2)?, None);
    assert_ne!(other, seeded);
    Ok(())
}

#[test]
fn seeded_palettes_in_both_modes_survive_switches_and_reset_repaints() -> Result {
    use ColorRole::*;
    let mut ui = Runtime::new();
    let theme = ui.theme();
    let (_, _, paints, _) = mount(&mut ui, "primary", Primary, false);
    draw(&mut ui);
    let seed = opaque(0.6, 0.1, 0.5)?;
    let dark = Palette::from_seeds(ThemeMode::Dark, seed, None);
    let light = Palette::from_seeds(ThemeMode::Light, seed, None);
    assert!(theme.set_palette(ThemeMode::Dark, dark)?);
    assert!(theme.set_palette(ThemeMode::Light, light)?);
    draw(&mut ui);
    assert_eq!(paints.get(), 2, "only the active mode repaints");
    theme.set_mode(ThemeMode::Light)?;
    let scene = draw(&mut ui);
    assert!(
        matches!(scene.commands()[0], DrawCommand::FillRect { color, .. } if color == light.color(Primary))
    );
    theme.set_mode(ThemeMode::Dark)?;
    assert_eq!(theme.palette(ThemeMode::Dark)?, dark);
    assert_eq!(theme.palette(ThemeMode::Light)?, light);
    draw(&mut ui);
    // An override and its removal each repaint the reader; removal restores
    // the seeded color, not the built-in one.
    theme.set_color(ThemeMode::Dark, Primary, Color::MUTED)?;
    draw(&mut ui);
    let before = paints.get();
    theme.set_palette(
        ThemeMode::Dark,
        theme.palette(ThemeMode::Dark)?.with_default_color(Primary),
    )?;
    let scene = draw(&mut ui);
    assert_eq!(paints.get(), before + 1);
    assert!(
        matches!(scene.commands()[0], DrawCommand::FillRect { color, .. } if color == dark.color(Primary))
    );
    Ok(())
}
