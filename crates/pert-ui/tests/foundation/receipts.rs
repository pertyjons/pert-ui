//! The one receipt contract: an owned `status()` that survives shutdown and a
//! `status_binding()` that invalidates its readers when the status changes.
use pert_ui::testing::HostIntegration;
use pert_ui::*;
use std::{cell::Cell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Counts its updates, which a change of the watched binding causes.
struct Watch<S> {
    binding: ReadSignal<S>,
    updates: Rc<Cell<usize>>,
}
impl<S: Clone + 'static> Component for Watch<S> {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        cx.read(&self.binding)?;
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
}
fn watch<S: Clone + 'static>(
    ui: &mut Runtime,
    key: &str,
    binding: ReadSignal<S>,
) -> Result<Rc<Cell<usize>>, RuntimeError> {
    let updates = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        key,
        Watch {
            binding,
            updates: updates.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(updates)
}

#[test]
fn task_receipt_binding_follows_delivery_and_status_survives_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    let done = ui.defer(|_| Ok(()))?;
    let pending = ui.defer(|_| Ok(()))?;
    assert_eq!(done, done.clone());
    assert_ne!(done, pending);
    assert_eq!(done.status_binding().get()?, TaskStatus::Pending);
    let updates = watch(&mut ui, "watch", done.status_binding())?;
    assert_eq!(done.status(), TaskStatus::Succeeded);
    assert_eq!(done.status_binding().get()?, TaskStatus::Succeeded);
    let late = ui.defer(|_| Ok(()))?;
    let seen = updates.get();
    ui.shutdown();
    assert_eq!(updates.get(), seen);
    assert_eq!(done.status(), TaskStatus::Succeeded);
    assert_eq!(late.status(), TaskStatus::Cancelled);
    assert_eq!(late.status_binding().get(), Err(RuntimeError::Closed));
    Ok(())
}

#[test]
fn presentation_receipt_binding_follows_completion_and_status_survives_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    let primary = ui.primary_window();
    let receipt = ui.request_close(&primary)?;
    let updates = watch(&mut ui, "watch", receipt.status_binding())?;
    let before = updates.get();
    for request in ui.take_presentation_requests()? {
        request.complete(PresentationStatus::CloseRejected)?;
    }
    ui.flush(Duration::ZERO)?;
    assert!(updates.get() > before, "completion invalidates the reader");
    assert_eq!(receipt.status(), PresentationStatus::CloseRejected);
    assert_eq!(
        receipt.status_binding().get()?,
        PresentationStatus::CloseRejected
    );
    let discarded = ui.request_close(&primary)?;
    ui.shutdown();
    assert_eq!(
        discarded.status(),
        PresentationStatus::Failed(WindowError::Cancelled)
    );
    assert_eq!(receipt.status(), PresentationStatus::CloseRejected);
    assert_eq!(receipt.status_binding().get(), Err(RuntimeError::Closed));
    Ok(())
}

#[test]
fn toast_receipt_binding_follows_presentation_and_status_survives_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0)?)?;
    let queue = ToastController::new(&ui);
    ui.mount_tree(Element::new(
        "host",
        ToastHost::new(
            queue.clone(),
            Element::new("content", Label::new("Content")),
        ),
    ))?;
    let receipt = queue.push(Toast::new("Saved"))?;
    assert_eq!(receipt, receipt.clone());
    let updates = watch(&mut ui, "watch", receipt.status_binding())?;
    assert_eq!(receipt.status(), ToastStatus::Visible);
    assert_eq!(receipt.status_binding().get()?, ToastStatus::Visible);
    let before = updates.get();
    ui.flush(Duration::from_secs(5))?;
    assert_eq!(receipt.status(), ToastStatus::Expired);
    assert_eq!(receipt.status_binding().get()?, ToastStatus::Expired);
    ui.flush(Duration::from_secs(5))?;
    assert!(updates.get() > before, "expiry invalidates the reader");
    let pending = queue.push(Toast::new("Late"))?;
    ui.shutdown();
    assert_eq!(receipt.status(), ToastStatus::Expired);
    assert_eq!(pending.status(), ToastStatus::OwnerRemoved);
    assert_eq!(pending.status_binding().get(), Err(RuntimeError::Closed));
    Ok(())
}

struct Owner;
impl Component for Owner {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}

#[test]
fn authority_receipt_binding_follows_the_response_and_status_survives_shutdown() -> TestResult {
    let mut ui = Runtime::new();
    let (parameter, host) = ui.remote_parameter(
        ParameterId::new(1, 1, "amount")?,
        ParameterSpec::new("Amount", 0.0, 1.0, 0.25)?,
    )?;
    let id = ui.mount_tree(Element::new("owner", Owner))?;
    let edit = ui.event(&id, |cx| {
        parameter.begin_edit(cx, parameter.snapshot().get()?.revision)
    })??;
    edit.commit(0.6)?;
    let request = host.request().ok_or("a submitted proposal")?;
    let receipt = host.receipt().ok_or("a retained receipt")?;
    assert_eq!(receipt, receipt.clone());
    assert!(matches!(
        receipt.status_binding().get()?,
        AuthorityStatus::Pending(_)
    ));
    let updates = watch(&mut ui, "watch", receipt.status_binding())?;
    let before = updates.get();
    host.respond(&request, AuthorityResponse::Accepted(0.6))?;
    ui.flush(Duration::ZERO)?;
    assert!(
        updates.get() > before,
        "the response invalidates the reader"
    );
    assert!(matches!(
        receipt.status_binding().get()?,
        AuthorityStatus::Resolved { .. }
    ));
    drop(edit);
    ui.shutdown();
    assert!(matches!(receipt.status(), AuthorityStatus::Resolved { .. }));
    assert!(matches!(
        receipt.status_binding().get(),
        Err(RuntimeError::Closed)
    ));
    Ok(())
}
