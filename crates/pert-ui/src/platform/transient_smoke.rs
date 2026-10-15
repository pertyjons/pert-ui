//! Native presentation evidence for the public transient components.
use crate::*;

pub(super) struct TransientSmoke {
    root: WidgetId,
    dialog: DialogController,
    queue: ToastQueue,
    count: Signal<u32>,
    banner: Signal<bool>,
    receipt: Option<ToastReceipt>,
}
/// The unique widget keyed `key` below `root`; several matches are
/// `RuntimeError::AmbiguousKey`, none is `RuntimeError::Unmounted`.
fn find(ui: &Runtime, root: &WidgetId, key: &str) -> Result<WidgetId, RuntimeError> {
    ui.find_descendant(root, key)?
        .ok_or(RuntimeError::Unmounted)
}
fn key(ui: &mut Runtime, key: KeyCode) -> Result<(), RuntimeError> {
    ui.key_input(key, KeyAction::Press)?;
    ui.key_input(key, KeyAction::Release)?;
    ui.flush(ui.inspect().at)?;
    Ok(())
}
impl TransientSmoke {
    pub(super) fn hide(&self, ui: &mut Runtime) -> Result<(), RuntimeError> {
        ui.set_visible(&self.root, false)
    }
    pub(super) fn new(ui: &mut Runtime) -> Result<Self, RuntimeError> {
        let dialog = DialogController::new(ui);
        let queue = ToastQueue::new(ui);
        let count = ui.signal(0);
        let c = count.clone();
        let banner = ui.signal(true);
        let root = ui.mount_tree(Element::new(
            "transient-smoke",
            ToastHost::new(
                queue.clone(),
                Element::new(
                    "content",
                    Column::new([
                        Element::new(
                            "alert",
                            AlertDialog::new(
                                dialog.clone(),
                                Element::new("opener", Button::new("Confirmation", |_| Ok(()))),
                                "Confirm task",
                                "Cancel receives safe initial focus.",
                                "Accept",
                            ),
                        ),
                        Element::new(
                            "context",
                            ContextMenu::new(
                                ui,
                                Element::new(
                                    "context-target",
                                    Button::new("Context menu", |_| Ok(())),
                                ),
                                vec![MenuItem::submenu(
                                    "nested",
                                    "Nested",
                                    vec![MenuItem::new("apply", "Apply", move |_| {
                                        c.set(c.get()? + 1).map(|_| ())
                                    })],
                                )],
                            )?,
                        ),
                        Element::new(
                            "banner",
                            Banner::new("Persistent feedback", [])
                                .visible_signal(banner.read_only()),
                        ),
                    ]),
                ),
            ),
        ))?;
        ui.flush(ui.inspect().at)?;
        ui.focus(&find(ui, &root, "opener")?)?;
        dialog.open()?;
        Ok(Self {
            root,
            dialog,
            queue,
            count,
            banner,
            receipt: None,
        })
    }
    pub(super) fn step(&mut self, ui: &mut Runtime, stage: u8) -> Result<(), RuntimeError> {
        match stage {
            23 => {
                assert_eq!(
                    ui.key(&ui.focused().ok_or(RuntimeError::Unmounted)?)?,
                    "cancel"
                );
                assert_eq!(
                    ui.focus(&find(ui, &self.root, "context-target")?),
                    Err(RuntimeError::ModalBlocked)
                );
                key(ui, KeyCode::Escape)?;
                assert_eq!(self.dialog.take_outcome()?, Some(DialogOutcome::Cancelled));
                ui.focus(&find(ui, &self.root, "context-target")?)?;
                ui.key_input(KeyCode::ShiftLeft, KeyAction::Press)?;
                key(ui, KeyCode::F10)?;
                ui.key_input(KeyCode::ShiftLeft, KeyAction::Release)?;
            }
            24 => {
                key(ui, KeyCode::ArrowRight)?;
            }
            25 => {
                key(ui, KeyCode::Enter)?;
                assert_eq!(self.count.get()?, 1);
                self.receipt = Some(
                    self.queue
                        .push(Toast::new("Native notification").action("Done", |_| Ok(())))?,
                );
            }
            26 => {
                assert_eq!(
                    self.receipt.as_ref().map(ToastReceipt::status),
                    Some(ToastStatus::Visible)
                );
                ui.focus(&find(ui, &self.root, "toasts")?)?;
                key(ui, KeyCode::Enter)?;
                assert_eq!(
                    self.receipt.as_ref().map(ToastReceipt::status),
                    Some(ToastStatus::ActionSucceeded)
                );
                self.banner.set(false)?;
            }
            _ => {}
        }
        Ok(())
    }
}
