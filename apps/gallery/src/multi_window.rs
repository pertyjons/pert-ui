//! Facade-only three-window fixture. Synthetic motion is UI data, not an audio tap.
use crate::samples::ReceiptReadout;
use pert_ui::*;
use std::time::Duration;

struct Motion {
    playing: ReadSignal<bool>,
    held: ReadSignal<bool>,
    phase: f32,
    down: bool,
    kind: usize,
}
impl Component for Motion {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        self.down = cx.read_paint(&self.held)?;
        if cx.read_paint(&self.playing)? {
            self.phase = (cx.now().as_secs_f64() % 4.0) as f32 / 4.0;
            cx.animate()?;
        }
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::from([480, 140]))
    }
    fn paint(&self, p: &mut Painter) -> Result<(), DrawError> {
        let Some(size) = p.size() else {
            return Ok(());
        };
        let color = p.color(if self.down {
            ColorRole::Secondary
        } else {
            ColorRole::Primary
        });
        p.fill_rect(Rect::from_size(size), p.color(ColorRole::Surface));
        for i in 0..24 {
            let width = size.width() / 24.0;
            let x = ((i as f32 / 24.0 + self.phase) % 1.0) * size.width();
            let height = if self.kind == 1 {
                (0.15
                    + 0.8
                        * (i as f32 * 0.7 + self.phase * std::f32::consts::TAU)
                            .sin()
                            .abs())
                    * size.height()
            } else {
                size.height() / 8.0
            };
            let y = if self.kind == 1 {
                size.height() - height
            } else {
                (i % 7) as f32 * size.height() / 8.0
            };
            p.fill_rect(Rect::new(x, y, width.min(size.width() - x), height)?, color);
        }
        Ok(())
    }
}
pub(super) fn run() -> Result<(), AppError> {
    App::new("pert-ui — Three native windows", |ui| {
        ui.set_viewport(Viewport::new(960.0, 640.0, 1.0)?)?;
        let playing = ui.signal(true);
        let held = ui.signal(false);
        let message = ui.signal(
            "Shared state across all three windows. Hold A in the pattern window.".to_owned(),
        );
        let receipt = ui.signal(None::<PresentationReceipt>);
        // The latest Play invocation, whose status every panel reports, so a
        // failed Play is shown rather than lost.
        let command_receipt = ui.signal(None::<CommandReceipt>);
        let play_state = playing.clone();
        let play = ui.register_command(
            "Play / pause",
            CommandScope::Application,
            vec![KeyChord::new(
                CommandKey::Named(KeyCode::Space),
                Modifiers::default(),
            )],
            move |_, _| play_state.set(!play_state.get()?),
        )?;
        let target_receipt = receipt.clone();
        ui.register_command(
            "Re-dock",
            CommandScope::Application,
            vec![KeyChord::primary('r')],
            move |ui, context| {
                let next = ui.request_redock(&context.window)?;
                target_receipt.set(Some(next))?;
                Ok(())
            },
        )?;
        let mut panels = Vec::new();
        let mut dialogs = Vec::new();
        for (index, title) in ["Transport", "Editor", "Patterns"].into_iter().enumerate() {
            let state = DialogController::new(ui);
            dialogs.push(state.clone());
            let close = state.clone();
            let input = ui.signal(format!("Editable text in {title}"));
            let play = play.clone();
            let play_receipt = command_receipt.clone();
            let command_receipt = command_receipt.clone();
            let content = Element::new(
                "body",
                Column::new([
                    Element::new("title", Label::new(title)),
                    Element::new(
                        "play",
                        Button::new("Play / pause (Space)", move |cx| {
                            let invocation = cx
                                .invoke_command(&play, cx.command_context(CommandSource::Button))?;
                            command_receipt.set(Some(invocation))
                        }),
                    ),
                    Element::new(
                        "motion",
                        Motion {
                            playing: playing.read_only(),
                            held: held.read_only(),
                            phase: 0.0,
                            down: false,
                            kind: index,
                        },
                    ),
                    Element::new("text", TextField::new(input)),
                    Element::new("message", Label::new(message.read_only())),
                    Element::new(
                        "play-status",
                        ReceiptReadout::new(
                            play_receipt.read_only(),
                            CommandReceipt::status_binding,
                            |status| match status {
                                CommandStatus::Pending => "Play: pending".to_owned(),
                                CommandStatus::Finished(Ok(_)) => "Play: done".to_owned(),
                                CommandStatus::Finished(Err(error)) => {
                                    format!("Play failed: {error}")
                                }
                                CommandStatus::Cancelled => "Play: cancelled".to_owned(),
                            },
                            "Play has not been pressed here.",
                        ),
                    ),
                    Element::new(
                        "status",
                        ReceiptReadout::new(
                            receipt.read_only(),
                            PresentationReceipt::status_binding,
                            |status| format!("Window operation: {status:?}"),
                            "Close a detached window to re-dock it. Primary+R also re-docks.",
                        ),
                    ),
                    Element::new(
                        "help",
                        Label::new(
                            "Primary+O: local dialog · Delete: this editor · Primary+D: detach",
                        ),
                    ),
                ]),
            );
            let panel = Element::new(
                "dialog-owner",
                Dialog::new(
                    state,
                    content,
                    Element::new(
                        "confirmation",
                        Column::new([
                            Element::new(
                                "notice",
                                Label::new(
                                    "Application Open command, placed in the invoking window.",
                                ),
                            ),
                            Element::new(
                                "close",
                                Button::new("Close", move |_| close.cancel().map(|_| ())),
                            ),
                        ]),
                    ),
                ),
            );
            panels.push(Element::new(
                format!("tabs-{index}"),
                Tabs::new([
                    Tab::new("panel", title, panel),
                    Tab::new(
                        "spare",
                        "Spare",
                        Element::new(
                            "spare-text",
                            Label::new(
                                "The detached view keeps updating while this tab is selected.",
                            ),
                        ),
                    ),
                ])
                .detachable(true),
            ));
        }
        let root = ui.mount_tree(Element::new("workspace", Column::new(panels)))?;
        ui.flush(Duration::ZERO)?;
        // Each editor is the view of its strip's "panel" tab, found by key so the
        // lookup does not depend on the strips' child order.
        let mut views = Vec::new();
        for index in 0..dialogs.len() {
            let strip = format!("tabs-{index}");
            let view = ui.find_path(&root, &[&strip, "panel"])?.ok_or_else(|| {
                ApplicationError::from(format!("editor view `{strip}/panel` is not mounted"))
            })?;
            views.push(view);
        }
        for (index, (view, dialog)) in views.iter().zip(&dialogs).enumerate() {
            let dialog = dialog.clone();
            ui.register_command(
                "Open",
                CommandScope::View(view.clone()),
                vec![KeyChord::primary('o')],
                move |_, _| {
                    if matches!(dialog.status().get()?, DialogStatus::Finished(_)) {
                        dialog.take_outcome()?;
                    }
                    dialog.open()
                },
            )?;
            let message = message.clone();
            ui.register_command(
                "Delete",
                CommandScope::View(view.clone()),
                vec![KeyChord::new(
                    CommandKey::Named(KeyCode::Delete),
                    Modifiers::default(),
                )],
                move |_, _| message.set(format!("Delete ran only in editor {index}")),
            )?;
            let view = view.clone();
            let target_receipt = receipt.clone();
            ui.register_command(
                "Detach",
                CommandScope::View(view.clone()),
                vec![KeyChord::primary('d')],
                move |ui, _| {
                    let next =
                        ui.request_detach(&view, NativeWindowOptions::new("Detached editor")?)?;
                    target_receipt.set(Some(next))?;
                    Ok(())
                },
            )?;
        }
        let pressed = held.clone();
        let cancelled = held.clone();
        // The Patterns editor, the last one, holds the A key.
        let patterns = views
            .last()
            .ok_or_else(|| ApplicationError::from("the three-window fixture mounted no editor"))?;
        ui.register_held_input(
            patterns,
            vec![KeyCode::KeyA],
            Modifiers::default(),
            move |_, event, _| {
                pressed.set(event != KeyTransition::Released)?;
                Ok(InputOutcome::Handled)
            },
            move |_, _| {
                if let Err(error) = cancelled.set(false) {
                    debug_assert_eq!(error, RuntimeError::Closed);
                }
            },
        )?;
        for (index, view) in views.iter().enumerate().skip(1) {
            let next = ui.request_detach(
                view,
                NativeWindowOptions::new(format!("pert-ui — Editor {index}"))?,
            )?;
            receipt.set(Some(next))?;
        }
        ui.set_command_target(views.first())?;
        Ok(())
    })
    .run()
}
