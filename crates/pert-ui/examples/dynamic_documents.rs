//! Dynamic resources and external acceptance through the public facade.
use pert_ui::parameter::*;
use pert_ui::prelude::*;
use pert_ui::{Capacity, KeyAction, TaskStatus, Viewport};
use std::{cell::RefCell, rc::Rc, time::Duration};

type Document = (WidgetId, Parameter<f32>, Rc<RemoteHost<f32>>);
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 200.0, 1.0)?)?;
    let open = ui.signal(false);
    let caption = open.map(|open| if *open { "Opened" } else { "Open document" }.to_owned())?;
    let enabled = open.map(|open| !open)?;
    let document: Rc<RefCell<Option<Document>>> = Rc::default();
    let slot: Rc<RefCell<Option<WidgetId>>> = Rc::default();
    let target = slot.clone();
    let owned = document.clone();
    let opened = open.clone();
    let id = ParameterId::new(1, 1, "amount")?;
    let spec = ParameterSpec::new("Amount", 0.0, 1.0, 0.5)?;
    let button = Element::new(
        "open",
        Button::new(caption, move |cx| {
            let target = target.clone();
            let (owned, opened, id, spec) =
                (owned.clone(), opened.clone(), id.clone(), spec.clone());
            // Retain this receipt when the application needs to inspect cancellation.
            let _operation = cx.defer(move |ui| {
                let (parameter, host) = ui.remote_parameter(id, spec)?;
                let parent = target.borrow().clone().ok_or(RuntimeError::Unmounted)?;
                ui.reconcile_children(
                    &parent,
                    vec![Element::new("document", Slider::new(parameter.clone()))],
                )?;
                let root = ui.children(&parent)?[0].clone();
                *owned.borrow_mut() = Some((root, parameter, Rc::new(host)));
                opened.set(true)?;
                Ok(())
            })?;
            Ok(())
        })
        .enabled(enabled),
    );
    let root = ui.mount_tree(Element::new(
        "application",
        Column::new([button, Element::new("documents", Container::default())]),
    ))?;
    let children = ui.children(&root)?;
    let button = children[0].clone();
    *slot.borrow_mut() = Some(children[1].clone());
    ui.flush(Duration::ZERO)?;
    // Headless input drives the same action as a native button.
    ui.focus(&button)?;
    ui.key_input(KeyCode::Space, KeyAction::Press)?;
    ui.key_input(KeyCode::Space, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    let (root, parameter, host) = document
        .borrow_mut()
        .take()
        .ok_or("document did not open")?;
    let observed = host.clone();
    let mailbox = ui.mailbox(Capacity::new(8)?, move |(request, response)| {
        observed.respond(&request, response)?;
        Ok(())
    })?;
    let (outbound, requests) = std::sync::mpsc::sync_channel(8);
    host.on_submit(move |request| {
        outbound
            .try_send(request.clone())
            .map_err(|error| match error {
                std::sync::mpsc::TrySendError::Full(_) => {
                    SinkError::new(SinkErrorKind::Full, "authority queue full")
                }
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    SinkError::new(SinkErrorKind::Closed, "authority stopped")
                }
            })
    })?;
    let sender = mailbox.sender();
    // Real authorities validate expected_revision and deduplicate token and
    // generation before applying. This demonstrates the owned transport only.
    let worker = std::thread::spawn(move || -> Result<(), &'static str> {
        // Bounded, so a missing proposal fails the example instead of
        // hanging it and the join below.
        let proposal = requests
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "no proposal arrived")?;
        sender
            .send((proposal, AuthorityResponse::Accepted(0.75)))
            .map_err(|_| "response queue unavailable")
    });
    ui.focus(&root)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    assert_eq!(parameter.snapshot().get()?.base, 0.5);
    worker.join().map_err(|_| "worker panicked")??;
    ui.flush(Duration::ZERO)?;
    assert_eq!(parameter.snapshot().get()?.base, 0.75);
    let closed = ui.defer(move |ui| {
        ui.unmount(&root)?;
        drop(host);
        Ok(())
    })?;
    // The response mailbox also owns a host reference; release it on close.
    drop(mailbox);
    ui.flush(Duration::ZERO)?;
    assert_eq!(closed.status(), TaskStatus::Succeeded);
    assert!(!parameter.snapshot().get()?.available);
    Ok(())
}
