//! The native window smoke: a scripted run through rendering, input, recovery
//! and idle checks that drives `Runner` from inside its own event loop.
//!
//! The smoke opens an event loop, so it runs in its own process with `--exact`;
//! see docs/verification.md §Native evidence.
use super::*;
use crate::{Color, Component, DrawError, Painter, Rect, UpdateContext};
use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

// libtest runs each test on a worker thread. Linux Wayland and Windows accept
// that explicitly; AppKit only accepts the process main thread, so macOS reports
// a typed error instead of letting winit panic.
pub(super) fn native_test_event_loop() -> Result<EventLoop<UserEvent>, AppError> {
    #[cfg(target_os = "linux")]
    let event_loop = {
        use winit::platform::wayland::EventLoopBuilderExtWayland;
        EventLoop::with_user_event()
            .with_wayland()
            .with_any_thread(true)
            .build()
    };
    #[cfg(target_os = "windows")]
    let event_loop = {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        EventLoop::with_user_event().with_any_thread(true).build()
    };
    #[cfg(target_os = "macos")]
    let event_loop = {
        if std::thread::current().name() != Some("main") {
            // `pert-ui-native-checks` matches this text (`MAIN_THREAD_REQUIRED`) to
            // report the check as UNSUPPORTED; its unit test pins the wording.
            return Err(AppError::EventLoop(
                "macOS native tests need the process main thread, which libtest does not provide"
                    .into(),
            ));
        }
        EventLoop::with_user_event().build()
    };
    warn_if_session_locked();
    event_loop.map_err(|error| AppError::EventLoop(error.to_string()))
}

/// The likely cause of a presentation check failing: a stage that requested a
/// frame and saw none can only fail because the compositor withheld it.
pub(super) const WITHHELD: &str =
    "the compositor withheld frames; is the session locked or the window hidden?";

/// The error of a smoke stage whose requested frame never reached the screen.
fn not_presented(what: &str) -> AppError {
    AppError::Graphics(format!("smoke: {what} was not presented; {WITHHELD}"))
}

/// Warns before a native check when logind reports the session locked, since
/// a locked session withholds frames and fails every presentation check. Best
/// effort: without `loginctl` or a logind session nothing is printed. Without
/// `XDG_SESSION_ID`, logind resolves the caller's session as `auto`.
fn warn_if_session_locked() {
    #[cfg(target_os = "linux")]
    {
        let session = std::env::var("XDG_SESSION_ID").unwrap_or_else(|_| "auto".into());
        let locked = std::process::Command::new("loginctl")
            .args(["show-session", &session, "-p", "LockedHint", "--value"])
            .output()
            .is_ok_and(|output| output.stdout.trim_ascii() == b"yes");
        if locked {
            eprintln!(
                "warning: logind reports session {session} locked; \
                 presentation checks fail while the compositor withholds frames"
            );
        }
    }
}

struct SmokeControls {
    root: crate::WidgetId,
    enabled: crate::Signal<bool>,
    parameter: crate::Parameter<f32>,
    _host: crate::ParameterHost<f32>,
    open: crate::Signal<bool>,
}
pub(super) struct Smoke {
    original_root: WidgetId,
    controls: Option<SmokeControls>,
    transients: Option<transient_smoke::TransientSmoke>,
    inline: Option<inline_smoke::InlineSmoke>,
    fonts: Option<font_smoke::FontSmoke>,
    windows: Option<window_smoke::WindowSmoke>,
    text_fields: Option<(WidgetId, crate::Signal<String>, crate::Signal<String>)>,
    number_fields: Option<(WidgetId, crate::Signal<i64>, crate::Signal<f64>)>,
    pub(super) frames: usize,
    stage: u8,
    /// The stage in progress, read by the watchdog and the failure report.
    progress: Arc<AtomicU8>,
    settle_attempts: u8,
    next: Duration,
    paints: std::rc::Rc<std::cell::Cell<usize>>,
    baseline_paints: usize,
    baseline_frames: usize,
    original_size: PhysicalSize<u32>,
}
impl Runner {
    /// Moves the pointer across the static tiles through the native motion path.
    fn inject_motion(&mut self) -> Result<(), AppError> {
        for step in 0..8 {
            self.cursor_moved(winit::dpi::PhysicalPosition::new(
                40.0 + f64::from(step) * 12.0,
                20.0,
            ))?;
        }
        Ok(())
    }
    pub(super) fn smoke_step(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<Duration, AppError> {
        let Some(mut smoke) = self.smoke.take() else {
            return Ok(Duration::MAX);
        };
        let now = self.start.elapsed();
        if now >= smoke.next {
            // Native presentation is asynchronous. Require an actual restored
            // frame and a complete quiet interval, with a bounded 2s allowance
            // for compositor delivery; a redraw loop still fails the idle check.
            // Every idle check gets the allowance: a frame the compositor
            // delivers late, or a repaint it asks for, has been seen to land in
            // the quiet interval of more than one of them.
            let idle_stage = matches!(smoke.stage, 5 | 8 | 12 | 28 | 34 | 39 | 46);
            let unsettled = if idle_stage {
                smoke.frames != smoke.baseline_frames || self.app.runtime.needs_paint()
            } else {
                matches!(
                    smoke.stage,
                    2 | 3 | 4 | 6 | 7 | 9 | 18 | 22 | 23 | 24 | 25 | 26
                ) && smoke.frames <= smoke.baseline_frames
            };
            if unsettled && smoke.settle_attempts < 4 {
                smoke.settle_attempts += 1;
                if idle_stage {
                    smoke.baseline_frames = smoke.frames;
                }
                if smoke.stage == 5 {
                    // Keep the pointer moving, so a frame per motion batch
                    // cannot pass as a late compositor delivery.
                    self.inject_motion()?;
                }
                smoke.next = now + Duration::from_millis(500);
                let deadline = smoke.next;
                eprintln!(
                    "Native smoke: waiting for stage {} to settle (attempt {})",
                    smoke.stage, smoke.settle_attempts
                );
                self.smoke = Some(smoke);
                return Ok(deadline);
            }
            match smoke.stage {
                0 => {
                    if smoke.frames == 0 {
                        return Err(AppError::Graphics("smoke: no initial frame".into()));
                    }
                    let snapshot = self.app.runtime.inspect();
                    let tile = snapshot
                        .components
                        .iter()
                        .find(|entry| entry.key == "tile-2")
                        .and_then(|entry| entry.details.box_model.border_box);
                    if tile.is_none_or(|bounds| bounds[1] != 0.0) {
                        return Err(AppError::Graphics(
                            "smoke: initial wrap layout incorrect".into(),
                        ));
                    }
                    // The window opens at the size the App configured.
                    if let Some(window) = &self.current.window {
                        let logical: LogicalSize<f64> =
                            self.current.size.to_logical(window.scale_factor());
                        if (logical.width - 1000.0).abs() > 1.0
                            || (logical.height - 660.0).abs() > 1.0
                        {
                            return Err(AppError::Graphics(format!(
                                "smoke: configured main window size not applied: {logical:?}"
                            )));
                        }
                    }
                    smoke.baseline_paints = smoke.paints.get();
                    smoke.baseline_frames = smoke.frames;
                    smoke.original_size = self.current.size;
                }
                1 => {
                    if smoke.paints.get() != smoke.baseline_paints
                        || self.current.dirty
                        || self.app.runtime.next_deadline().is_some()
                    {
                        return Err(AppError::Graphics(
                            "smoke: static content did not settle".into(),
                        ));
                    }
                    if let Some(window) = &self.current.window
                        && let Some(size) =
                            window.request_inner_size(LogicalSize::new(720.0, 480.0))
                    {
                        self.resize(size)?;
                    }
                }
                2 => {
                    if self.current.size == smoke.original_size {
                        return Err(AppError::Graphics(
                            "smoke: resize did not change the surface size".into(),
                        ));
                    }
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("resize"));
                    }
                    let snapshot = self.app.runtime.inspect();
                    let tile = snapshot
                        .components
                        .iter()
                        .find(|entry| entry.key == "tile-2")
                        .and_then(|entry| entry.details.box_model.border_box);
                    if tile.is_none_or(|bounds| bounds[1] != 48.0) {
                        return Err(AppError::Graphics(
                            "smoke: resized wrap layout incorrect".into(),
                        ));
                    }
                    let physical = self.current.size;
                    self.resize(PhysicalSize::new(0, 0))?;
                    if self.active() {
                        return Err(AppError::Graphics(
                            "smoke: zero surface remained active".into(),
                        ));
                    }
                    self.resize(physical)?;
                    smoke.baseline_frames = smoke.frames;
                }
                3 => {
                    // A restored surface must present, but the zero-size/restore
                    // pair can settle before layout and reuse retained drawing.
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("restored surface"));
                    }
                    smoke.baseline_frames = smoke.frames;
                    smoke.baseline_paints = smoke.paints.get();
                    self.app.runtime.theme().set_mode(crate::ThemeMode::Light)?;
                }
                4 => {
                    // Report the two failures apart: a compositor that withholds
                    // the requested frame (a locked session) is not a reuse defect.
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("background-only theme change"));
                    }
                    if smoke.paints.get() != smoke.baseline_paints {
                        return Err(AppError::Graphics(
                            "smoke: background-only theme change did not reuse component drawing"
                                .into(),
                        ));
                    }
                    smoke.baseline_frames = smoke.frames;
                    // Pointer motion over static content presents nothing;
                    // the idle check that follows sees any frame it causes.
                    self.inject_motion()?;
                }
                5 => {
                    if smoke.frames != smoke.baseline_frames || self.app.runtime.needs_paint() {
                        return Err(AppError::Graphics(
                            "smoke: theme change or pointer motion did not settle".into(),
                        ));
                    }
                    self.inject_motion()?;
                    self.current.motion.flush(&mut self.app.runtime)?;
                    self.app.runtime.flush(self.start.elapsed())?;
                    if self.app.runtime.needs_paint() || self.current.dirty {
                        return Err(AppError::Graphics(
                            "smoke: pointer motion over static content requested a frame".into(),
                        ));
                    }
                    smoke.original_size = self.current.size;
                    self.app
                        .runtime
                        .ui_settings()
                        .set_zoom(crate::UiZoom::new(1.25)?)?;
                }
                6 => {
                    let native_scale = self
                        .current
                        .window
                        .as_ref()
                        .map_or(1.0, |window| window.scale_factor());
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("user zoom"));
                    }
                    if self.current.size != smoke.original_size
                        || self.app.runtime.viewport().scale() != native_scale * 1.25
                    {
                        return Err(AppError::Graphics(
                            "smoke: user zoom did not apply at the unchanged native size".into(),
                        ));
                    }
                    smoke.baseline_frames = smoke.frames;
                    self.app
                        .runtime
                        .ui_settings()
                        .set_zoom(crate::UiZoom::default())?;
                    self.app
                        .runtime
                        .theme()
                        .set_density(crate::Density::Compact)?;
                }
                7 => {
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("density/reset"));
                    }
                    smoke.baseline_frames = smoke.frames;
                }
                8 => {
                    if smoke.frames != smoke.baseline_frames || self.app.runtime.needs_paint() {
                        return Err(AppError::Graphics(
                            "smoke: zoom/density did not settle".into(),
                        ));
                    }
                    let ui = &mut self.app.runtime;
                    ui.unmount(&smoke.original_root)?;
                    let enabled = ui.signal(false);
                    let choice = ui.signal(0);
                    let open = ui.signal(false);
                    let (parameter, host) = ui
                        .parameter(
                            crate::ParameterId::new(99, 1, "smoke")
                                .map_err(|e| AppError::Graphics(e.to_string()))?,
                            crate::ParameterSpec::new("Smoke", 0.0, 1.0, 0.5)
                                .map_err(|e| AppError::Graphics(e.to_string()))?,
                        )
                        .map_err(|e| AppError::Graphics(e.to_string()))?;
                    let options = [crate::Choice::new(0, "One"), crate::Choice::new(1, "Two")];
                    let root = ui.mount_tree(crate::Element::new(
                        "smoke-controls",
                        crate::Container::new(crate::Element::new(
                            "column",
                            crate::Column::new([
                                crate::Element::new(
                                    "checkbox",
                                    crate::Checkbox::new(enabled.clone(), "Checkbox"),
                                ),
                                crate::Element::new(
                                    "toggle",
                                    crate::Toggle::new(enabled.clone(), "Toggle"),
                                ),
                                crate::Element::new(
                                    "radio",
                                    crate::RadioGroup::new(choice.clone(), options.clone())?,
                                ),
                                crate::Element::new(
                                    "slider",
                                    crate::Slider::new(parameter.clone())
                                        .show_value(true)
                                        .unit("%"),
                                ),
                                crate::Element::new(
                                    "dropdown",
                                    crate::Select::new(ui, choice, options)?,
                                ),
                                crate::Element::new(
                                    "menu",
                                    crate::Menu::new(
                                        ui,
                                        "Menu",
                                        vec![crate::MenuItem::new("apply", "Apply", |_| Ok(()))],
                                    )?,
                                ),
                                crate::Element::new(
                                    "help",
                                    crate::Tooltip::new(
                                        crate::Element::new(
                                            "help-trigger",
                                            crate::Button::new("Help", |_| Ok(())),
                                        ),
                                        "Native tooltip",
                                    ),
                                ),
                                crate::Element::new(
                                    "popover",
                                    crate::Popover::new(
                                        open.clone(),
                                        crate::Element::new(
                                            "popup-trigger",
                                            crate::Button::new("Details", |_| Ok(())),
                                        ),
                                        crate::Element::new(
                                            "popup-action",
                                            crate::Button::new("Native popup action", |_| Ok(())),
                                        ),
                                    ),
                                ),
                            ]),
                        ))
                        .background_role(crate::ColorRole::Background),
                    ))?;
                    smoke.controls = Some(SmokeControls {
                        root,
                        enabled,
                        parameter,
                        _host: host,
                        open,
                    });
                    smoke.baseline_frames = smoke.frames;
                }
                9 => {
                    let controls = smoke
                        .controls
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: controls missing".into()))?;
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("controls"));
                    }
                    let column = self.app.runtime.children(&controls.root)?[0].clone();
                    let children = self.app.runtime.children(&column)?;
                    self.app.runtime.focus(&children[0])?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Space, crate::KeyAction::Press)?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Space, crate::KeyAction::Release)?;
                    self.app.runtime.focus(&children[3])?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::ArrowRight, crate::KeyAction::Press)?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::ArrowRight, crate::KeyAction::Release)?;
                    if !controls.enabled.get()? || controls.parameter.snapshot().get()?.base <= 0.5
                    {
                        return Err(AppError::Graphics(
                            "smoke: bound controls did not edit".into(),
                        ));
                    }
                    controls.open.set(true)?;
                    smoke.baseline_frames = smoke.frames;
                }
                10 => {
                    let controls = smoke
                        .controls
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: controls missing".into()))?;
                    let focus =
                        self.app.runtime.focused().ok_or_else(|| {
                            AppError::Graphics("smoke: popup focus missing".into())
                        })?;
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("popup"));
                    }
                    if self.app.runtime.key(&focus)? != "popup-action" {
                        return Err(AppError::Graphics("smoke: popup was not focused".into()));
                    }
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Escape, crate::KeyAction::Press)?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Escape, crate::KeyAction::Release)?;
                    if controls.open.get()? {
                        return Err(AppError::Graphics(
                            "smoke: Escape did not dismiss popup".into(),
                        ));
                    }
                }
                11 => {
                    smoke.baseline_frames = smoke.frames;
                }
                12 => {
                    if smoke.frames != smoke.baseline_frames
                        || self.app.runtime.next_deadline().is_some()
                        || self.app.runtime.needs_paint()
                    {
                        return Err(AppError::Graphics(format!(
                            "smoke: controls did not settle: {} frames since the baseline, \
                             next deadline {:?} at {:?}, needs paint {}, pending components {:?}",
                            smoke.frames - smoke.baseline_frames,
                            self.app.runtime.next_deadline(),
                            self.app.runtime.now(),
                            self.app.runtime.needs_paint(),
                            self.app
                                .runtime
                                .inspect()
                                .components
                                .iter()
                                .filter(|c| c.next_deadline.is_some())
                                .map(|c| (c.key.clone(), c.next_deadline))
                                .collect::<Vec<_>>()
                        )));
                    }
                    if let Some(controls) = &smoke.controls {
                        self.app.runtime.unmount(&controls.root)?;
                    }
                    let name = self.app.runtime.signal("Native name".to_owned());
                    let notes = self.app.runtime.signal(format!(
                        "Native multiline\nMonospace by default\n{}",
                        "Scrollbar line\n".repeat(12)
                    ));
                    let root = self.app.runtime.mount_tree(crate::Element::new(
                        "text-fields",
                        crate::Column::new([
                            crate::Element::new("native-name", crate::TextField::new(name.clone())),
                            crate::Element::new(
                                "native-notes",
                                crate::TextArea::new(notes.clone()),
                            ),
                        ]),
                    ))?;
                    smoke.text_fields = Some((root, name, notes));
                    smoke.baseline_frames = smoke.frames;
                }
                13 => {
                    let (root, _, _) = smoke
                        .text_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: text fields missing".into()))?;
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("text fields"));
                    }
                    let children = self.app.runtime.children(root)?;
                    self.app.runtime.focus(&children[0])?;
                    self.sync_ime();
                    self.native_ime(winit::event::Ime::Enabled)?;
                    self.native_ime(winit::event::Ime::Preedit("åäö".into(), Some((0, 6))))?;
                }
                14 => {
                    self.native_ime(winit::event::Ime::Commit("åäö ".into()))?;
                    let (root, name, _) = smoke
                        .text_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: text fields missing".into()))?;
                    if name.get()? != "åäö Native name" {
                        return Err(AppError::Graphics("smoke: IME commit failed".into()));
                    }
                    let children = self.app.runtime.children(root)?;
                    self.app.runtime.focus(&children[1])?;
                    self.app.runtime.text_input("Edited ")?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Enter, crate::KeyAction::Press)?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::Enter, crate::KeyAction::Release)?;
                }
                15 => {
                    let (_, _, notes) = smoke
                        .text_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: text fields missing".into()))?;
                    if !notes.get()?.starts_with("Edited \n") {
                        return Err(AppError::Graphics("smoke: multiline edit failed".into()));
                    }
                    let text = notes.get()?;
                    let (root, _, _) = smoke
                        .text_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: text fields missing".into()))?;
                    let ui = &mut self.app.runtime;
                    let notes_field = ui.children(root)?[1].clone();
                    let metrics = ui.theme().metrics(ui.theme().density()?)?;
                    let bounds = ui
                        .bounds(&notes_field)?
                        .ok_or(RuntimeError::Unmounted)?
                        .values();
                    // Inside the eight-pixel scrollbar strip at the text's right edge.
                    let lane = bounds[0] + bounds[2]
                        - metrics.get(crate::MetricRole::ControlPaddingX)
                        - 2.0;
                    let top = bounds[1] + metrics.get(crate::MetricRole::ControlPaddingY);
                    let at = |y: f32| -> Result<crate::Point, AppError> {
                        Ok(crate::Point::new(lane, y)?)
                    };
                    for (position, kind) in [
                        (
                            at(top + 4.0)?,
                            crate::PointerKind::Press(crate::PointerButton::Primary),
                        ),
                        (at(top + 48.0)?, crate::PointerKind::Move),
                        (
                            at(top + 48.0)?,
                            crate::PointerKind::Release(crate::PointerButton::Primary),
                        ),
                    ] {
                        ui.pointer_input(crate::PointerEvent::new(position, kind))?;
                        ui.flush(ui.now())?;
                    }
                    let scrolled = ui
                        .inspect()
                        .components
                        .into_iter()
                        .find(|c| c.key == "native-notes")
                        .and_then(|c| c.details.properties.get("scroll_y").cloned());
                    if !matches!(scrolled, Some(crate::InspectValue::Number(offset)) if offset > 0.0)
                        || notes.get()? != text
                    {
                        return Err(AppError::Graphics(
                            "smoke: multiline scrollbar drag did not scroll the text".into(),
                        ));
                    }
                    ui.clear_focus();
                }
                16 => {
                    smoke.baseline_frames = smoke.frames;
                }
                17 => {
                    if smoke.frames != smoke.baseline_frames
                        || self.app.runtime.next_deadline().is_some()
                    {
                        return Err(AppError::Graphics(
                            "smoke: text fields did not settle".into(),
                        ));
                    }
                    if let Some((root, _, _)) = &smoke.text_fields {
                        self.app.runtime.unmount(root)?;
                    }
                    let ui = &mut self.app.runtime;
                    let integer = ui.signal(0_i64);
                    let decimal = ui.signal(0_f64);
                    let root = ui.mount_tree(crate::Element::new(
                        "native-numbers",
                        crate::Column::new([
                            crate::Element::new(
                                "native-integer",
                                crate::IntegerInput::new(integer.clone()),
                            ),
                            crate::Element::new(
                                "native-decimal",
                                crate::DecimalInput::new(decimal.clone()).decimal_places(2)?,
                            ),
                        ]),
                    ))?;
                    smoke.number_fields = Some((root, integer, decimal));
                    smoke.baseline_frames = smoke.frames;
                }
                18 => {
                    let (root, integer, _) = smoke
                        .number_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: number fields missing".into()))?;
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("number fields"));
                    }
                    let ui = &mut self.app.runtime;
                    let children = ui.children(root)?;
                    ui.focus(&children[0])?;
                    ui.text_input("42")?;
                    if integer.get()? != 0 {
                        return Err(AppError::Graphics(
                            "smoke: integer draft saved prematurely".into(),
                        ));
                    }
                    ui.key_input(crate::KeyCode::Enter, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::Enter, crate::KeyAction::Release)?;
                    if integer.get()? != 420 {
                        return Err(AppError::Graphics(
                            "smoke: integer Enter did not save".into(),
                        ));
                    }
                    ui.focus(&children[1])?;
                    // Replace the fixed-precision resting text ("0.00").
                    ui.key_input(crate::KeyCode::Home, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::Home, crate::KeyAction::Release)?;
                    ui.key_input(crate::KeyCode::ShiftLeft, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::End, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::End, crate::KeyAction::Release)?;
                    ui.key_input(crate::KeyCode::ShiftLeft, crate::KeyAction::Release)?;
                    ui.text_input("1,5")?;
                }
                19 => {
                    let (root, _, decimal) = smoke
                        .number_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: number fields missing".into()))?;
                    if decimal.get()? != 0.0 {
                        return Err(AppError::Graphics(
                            "smoke: decimal draft saved prematurely".into(),
                        ));
                    }
                    let ui = &mut self.app.runtime;
                    ui.clear_focus();
                    if decimal.get()? != 1.5 {
                        return Err(AppError::Graphics(
                            "smoke: decimal focus loss did not save".into(),
                        ));
                    }
                    let decimal_field = ui.children(root)?[1].clone();
                    ui.focus(&decimal_field)?;
                    ui.flush(ui.now())?;
                    let bounds = ui
                        .bounds(&decimal_field)?
                        .ok_or(RuntimeError::Unmounted)?
                        .values();
                    let position = crate::Point::new(bounds[0] + 10.0, bounds[1] + 10.0)?;
                    ui.pointer_input(crate::PointerEvent::new(
                        position,
                        crate::PointerKind::Scroll(crate::Point::new(0.0, -40.0)?),
                    ))?;
                    ui.flush(ui.now())?;
                    let text = ui
                        .inspect()
                        .components
                        .into_iter()
                        .find(|c| c.key == "native-decimal")
                        .and_then(|c| c.details.properties.get("text").cloned());
                    if text != Some(crate::InspectValue::Text("1.60".into()))
                        || decimal.get()? != 1.5
                    {
                        return Err(AppError::Graphics(
                            "smoke: decimal wheel precision/draft contract failed".into(),
                        ));
                    }
                    ui.key_input(crate::KeyCode::Escape, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::Escape, crate::KeyAction::Release)?;
                    ui.text_input("9")?;
                    ui.key_input(crate::KeyCode::Escape, crate::KeyAction::Press)?;
                    ui.key_input(crate::KeyCode::Escape, crate::KeyAction::Release)?;
                    ui.clear_focus();
                    if decimal.get()? != 1.5 {
                        return Err(AppError::Graphics(
                            "smoke: decimal Escape did not restore".into(),
                        ));
                    }
                }
                20 => {
                    smoke.baseline_frames = smoke.frames;
                }
                21 => {
                    if smoke.frames != smoke.baseline_frames
                        || self.app.runtime.next_deadline().is_some()
                    {
                        return Err(AppError::Graphics(
                            "smoke: number fields did not settle".into(),
                        ));
                    }
                    // A real native focus-loss event cleans input, and a pending
                    // invalid external number is refused by its field rather than
                    // failing the runtime.
                    let (root, _, decimal) = smoke
                        .number_fields
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: number fields missing".into()))?;
                    self.app
                        .runtime
                        .focus(&self.app.runtime.children(root)?[1])?;
                    self.app
                        .runtime
                        .key_input(crate::KeyCode::ControlLeft, crate::KeyAction::Press)?;
                    decimal.set(f64::NAN)?;
                    let window_id = self
                        .current
                        .window
                        .as_ref()
                        .ok_or_else(|| AppError::Graphics("smoke: native window missing".into()))?
                        .id();
                    self.window_event(event_loop, window_id, WindowEvent::Focused(false));
                    if self.current.component_recovery.error.is_some()
                        || self.app.runtime.focused().is_some()
                        || !self.app.runtime.inspect().pressed_keys.is_empty()
                        || self
                            .app
                            .runtime
                            .inspect()
                            .components
                            .iter()
                            .any(|c| c.next_deadline.is_some())
                    {
                        return Err(AppError::Graphics(
                            "smoke: focus loss did not clean input".into(),
                        ));
                    }
                    smoke.baseline_frames = smoke.frames;
                }
                22 => {
                    // The next presented frame shows the refusal, not an error.
                    let refused = self
                        .app
                        .runtime
                        .inspect()
                        .components
                        .iter()
                        .any(|c| c.details.properties.contains_key("binding_error"));
                    if smoke.frames <= smoke.baseline_frames {
                        return Err(not_presented("invalid number"));
                    }
                    if self.current.component_recovery.error.is_some() || !refused {
                        return Err(AppError::Graphics(
                            "smoke: invalid number was not refused".into(),
                        ));
                    }
                    if let Some((root, _, decimal)) = &smoke.number_fields {
                        decimal.set(1.5)?;
                        self.app.runtime.set_visible(root, false)?;
                    }
                    smoke.transients =
                        Some(transient_smoke::TransientSmoke::new(&mut self.app.runtime)?);
                    smoke.baseline_frames = smoke.frames;
                }
                23..=26 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "transient stage was not presented; {WITHHELD}"
                    );
                    smoke
                        .transients
                        .as_mut()
                        .ok_or(RuntimeError::Unmounted)?
                        .step(&mut self.app.runtime, smoke.stage)?;
                    smoke.baseline_frames = smoke.frames;
                }
                27 => {
                    smoke.baseline_frames = smoke.frames;
                }
                28 => {
                    assert_eq!(
                        smoke.frames, smoke.baseline_frames,
                        "transient content did not settle"
                    );
                    assert!(self.app.runtime.next_deadline().is_none());
                    smoke
                        .transients
                        .as_ref()
                        .ok_or(RuntimeError::Unmounted)?
                        .hide(&mut self.app.runtime)?;
                    smoke.inline = Some(inline_smoke::InlineSmoke::new(&mut self.app.runtime)?);
                    smoke.baseline_frames = smoke.frames;
                }
                29..=32 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "inline stage was not presented; {WITHHELD}"
                    );
                    smoke
                        .inline
                        .as_mut()
                        .ok_or(RuntimeError::Unmounted)?
                        .step(&mut self.app.runtime, smoke.stage)?;
                    smoke.baseline_frames = smoke.frames;
                }
                33 => {
                    smoke.baseline_frames = smoke.frames;
                }
                34 => {
                    assert_eq!(
                        smoke.frames, smoke.baseline_frames,
                        "inline controls did not settle"
                    );
                    assert!(self.app.runtime.next_deadline().is_none());
                    smoke
                        .inline
                        .take()
                        .ok_or(RuntimeError::Unmounted)?
                        .unmount(&mut self.app.runtime)?;
                    smoke.fonts = Some(font_smoke::FontSmoke::new(&mut self.app.runtime)?);
                    smoke.baseline_frames = smoke.frames;
                }
                35..=37 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "font/icon stage was not presented; {WITHHELD}"
                    );
                    let done = smoke
                        .fonts
                        .as_mut()
                        .ok_or(RuntimeError::Unmounted)?
                        .step(&self.app.runtime, smoke.stage)?;
                    if !done {
                        // A capture is on its way; run this stage again.
                        smoke.settle_attempts += 1;
                        if smoke.settle_attempts > 40 {
                            return Err(AppError::Graphics(format!(
                                "smoke: font stage {} capture did not arrive",
                                smoke.stage
                            )));
                        }
                        smoke.next = now + Duration::from_millis(100);
                        let deadline = smoke.next;
                        self.smoke = Some(smoke);
                        return Ok(deadline);
                    }
                    smoke.baseline_frames = smoke.frames;
                }
                38 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "font reset was not presented; {WITHHELD}"
                    );
                    smoke.baseline_frames = smoke.frames;
                }
                39 => {
                    assert_eq!(
                        smoke.frames, smoke.baseline_frames,
                        "font/icon controls did not settle"
                    );
                    assert!(self.app.runtime.next_deadline().is_none());
                    smoke
                        .fonts
                        .take()
                        .ok_or(RuntimeError::Unmounted)?
                        .unmount(&mut self.app.runtime)?;
                    smoke.windows = Some(window_smoke::WindowSmoke::new(&mut self.app.runtime)?);
                    smoke.baseline_frames = smoke.frames;
                }
                40..=44 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "floating window stage was not presented; {WITHHELD}"
                    );
                    smoke
                        .windows
                        .as_ref()
                        .ok_or(RuntimeError::Unmounted)?
                        .step(&mut self.app.runtime, smoke.stage)?;
                    smoke.baseline_frames = smoke.frames;
                }
                45 => {
                    assert!(
                        smoke.frames > smoke.baseline_frames,
                        "window closure was not presented; {WITHHELD}"
                    );
                    smoke.baseline_frames = smoke.frames;
                }
                46 => {
                    assert_eq!(
                        smoke.frames, smoke.baseline_frames,
                        "floating windows did not settle"
                    );
                    assert!(self.app.runtime.next_deadline().is_none());
                    println!(
                        "Native smoke: {} frames; existing rendering/input/recovery checks plus dialog, context menu, submenu, banner, toast, inline numeric editing, rich text, font/icon overrides, floating window move/resize/minimize/restore/close and idle passed",
                        smoke.frames
                    );
                    // Persistent paint errors must exhaust the allowance even
                    // though updates succeed between attempts.
                    struct FailingPaint;
                    impl crate::Component for FailingPaint {
                        fn update(
                            &mut self,
                            _: &mut crate::UpdateContext,
                        ) -> Result<(), RuntimeError> {
                            Ok(())
                        }
                        fn paint(&self, _: &mut crate::Painter) -> Result<(), crate::DrawError> {
                            Err(crate::DrawError::InvalidColor)
                        }
                    }
                    self.app
                        .runtime
                        .mount("persistent-paint-error", FailingPaint)?;
                }
                _ => (),
            }
            smoke.stage += 1;
            smoke.progress.store(smoke.stage, Ordering::Relaxed);
            smoke.settle_attempts = 0;
            smoke.next = now + Duration::from_millis(500);
        }
        let deadline = smoke.next;
        self.smoke = Some(smoke);
        Ok(deadline)
    }
}

struct Rectangle(Rc<Cell<usize>>);
impl Component for Rectangle {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn measure(
        &self,
        _: &crate::MeasureContext<'_>,
        _: crate::Constraints,
    ) -> Result<crate::Size, RuntimeError> {
        Ok(crate::Size::new(300.0, 40.0)?)
    }
    fn paint(&self, painter: &mut Painter) -> Result<(), DrawError> {
        self.0.set(self.0.get() + 1);
        if let Some(size) = painter.size() {
            painter.fill_rect(
                Rect::new(-10.0, -10.0, size.width() + 20.0, size.height() + 20.0)?,
                Color::ACCENT,
            );
        }
        Ok(())
    }
}
/// Runs the smoke's `Runner` with the desk's input left out.
///
/// The smoke injects every key, pointer move and focus change it checks by
/// calling the runner directly. Input the operating system delivers, from a
/// hand on the mouse, a pointer resting where the window opens or a
/// notification taking focus, would interleave with that script: a focus loss
/// closes the context menu between two scripted keys. This handler drops that
/// input and gives the window focus once, as a desktop does when it maps a
/// new window. Presentation events (resize, scale, occlusion, redraw, close)
/// still come from the system, because they are what the smoke checks.
struct Scripted {
    runner: Runner,
    focused: bool,
}
impl Scripted {
    /// Input from the desk rather than from the script.
    fn from_desk(event: &WindowEvent) -> bool {
        matches!(
            event,
            WindowEvent::Focused(_)
                | WindowEvent::KeyboardInput { .. }
                | WindowEvent::ModifiersChanged(_)
                | WindowEvent::Ime(_)
                | WindowEvent::CursorMoved { .. }
                | WindowEvent::CursorEntered { .. }
                | WindowEvent::CursorLeft { .. }
                | WindowEvent::MouseInput { .. }
                | WindowEvent::MouseWheel { .. }
                | WindowEvent::PinchGesture { .. }
                | WindowEvent::PanGesture { .. }
                | WindowEvent::RotationGesture { .. }
                | WindowEvent::DoubleTapGesture { .. }
                | WindowEvent::TouchpadPressure { .. }
                | WindowEvent::AxisMotion { .. }
                | WindowEvent::Touch(_)
        )
    }
}
impl ApplicationHandler<UserEvent> for Scripted {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.resumed(event_loop);
        if !self.focused
            && let Some(id) = self
                .runner
                .current
                .window
                .as_ref()
                .map(|window| window.id())
        {
            self.focused = true;
            self.runner
                .window_event(event_loop, id, WindowEvent::Focused(true));
        }
    }
    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.suspended(event_loop);
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if !Self::from_desk(&event) {
            self.runner.window_event(event_loop, id, event);
        }
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        self.runner.user_event(event_loop, event);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.exiting(event_loop);
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.runner.about_to_wait(event_loop);
    }
}

/// How long the smoke may run before its watchdog reports the stage it is
/// stuck in. A normal run takes about 26 s; this stays below the native-check
/// runner's default 120 s limit, which would kill the process without naming
/// the stage.
const WATCHDOG: Duration = Duration::from_secs(100);

/// Names the smoke's stage when the smoke ends abnormally: a panic in a stage
/// reports it on unwinding, and a smoke still running at [`WATCHDOG`] reports
/// it and ends the process with a failure.
struct StageReport {
    progress: Arc<AtomicU8>,
    done: Arc<AtomicBool>,
}
impl StageReport {
    fn start(progress: Arc<AtomicU8>) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let (stage, finished) = (progress.clone(), done.clone());
        std::thread::spawn(move || {
            let start = Instant::now();
            while start.elapsed() < WATCHDOG {
                if finished.load(Ordering::Relaxed) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            eprintln!(
                "Native smoke: stuck in stage {} after {} s",
                stage.load(Ordering::Relaxed),
                WATCHDOG.as_secs()
            );
            std::process::exit(1);
        });
        Self { progress, done }
    }
    fn stage(&self) -> u8 {
        self.progress.load(Ordering::Relaxed)
    }
}
impl Drop for StageReport {
    fn drop(&mut self) {
        self.done.store(true, Ordering::Relaxed);
        if std::thread::panicking() {
            eprintln!("Native smoke: failed in stage {}", self.stage());
        }
    }
}

// Native window checks each need their own process: winit allows one event
// loop per process, so run them one at a time with `--exact`, as listed in
// docs/verification.md §Native evidence.
#[test]
#[ignore = "requires a native desktop session and hardware GPU adapter"]
fn native_window_resize_and_idle() -> Result<(), AppError> {
    window_resize_and_idle(false)
}

/// The same scripted run with software presentation: resize, minimize and
/// restore, input, recovery, floating windows and settled idle, presenting
/// CPU frames without a GPU device.
#[cfg(feature = "cpu")]
#[test]
#[ignore = "requires a native desktop session"]
fn software_window_resize_and_idle() -> Result<(), AppError> {
    window_resize_and_idle(true)
}

fn window_resize_and_idle(software: bool) -> Result<(), AppError> {
    let paints = Rc::new(Cell::new(0));
    let widget_paints = paints.clone();
    let failures = Rc::new(Cell::new(0));
    let observed = failures.clone();
    let mut app = App::new("pert-ui native verification", |_| Ok(()))
        .size(crate::Size::from([1000, 660]))?
        .min_size(crate::Size::from([480, 320]))?
        .on_error(move |_, _| {
            observed.set(observed.get() + 1);
            crate::ErrorAction::Retry
        });
    app.software = software;
    let original_root = app.runtime.mount_tree(crate::Element::new(
        "flow",
        crate::Wrap::new((0..4).map(|index| {
            crate::Element::new(format!("tile-{index}"), Rectangle(widget_paints.clone()))
        })),
    ))?;
    let event_loop = native_test_event_loop()?;
    let progress = Arc::new(AtomicU8::new(0));
    let report = StageReport::start(progress.clone());
    let mut runner = Runner::new(app, event_loop.create_proxy());
    runner.smoke = Some(Smoke {
        original_root,
        controls: None,
        transients: None,
        inline: None,
        fonts: None,
        windows: None,
        text_fields: None,
        number_fields: None,
        frames: 0,
        stage: 0,
        progress,
        settle_attempts: 0,
        next: Duration::from_secs(2),
        paints,
        baseline_paints: 0,
        baseline_frames: 0,
        original_size: PhysicalSize::new(0, 0),
    });
    let mut scripted = Scripted {
        runner,
        focused: false,
    };
    event_loop
        .run_app(&mut scripted)
        .map_err(|e| AppError::EventLoop(e.to_string()))?;
    let runner = scripted.runner;
    assert!(
        matches!(
            runner.error,
            Some(AppError::Drawing(crate::DrawError::InvalidColor))
        ),
        "unexpected native failure in stage {} before the final recovery check: {:?}",
        report.stage(),
        runner.error
    );
    assert_eq!(
        failures.get(),
        4,
        "four paint failures; the invalid number is refused by its field, not reported"
    );
    Ok(())
}
