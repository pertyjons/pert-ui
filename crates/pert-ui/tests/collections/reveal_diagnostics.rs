//! Historical refusal diagnostics and host-hidden operational failures.
use pert_ui::host::HostIntegration;
use pert_ui::testing::{Driver, TestError};
use pert_ui::*;
use std::time::Duration;

type Check = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy, Debug)]
enum RefusalCase {
    Dialog,
    BoundView,
    ExplicitHide,
}

struct Retained {
    error: TestError,
    cleanup: TestError,
    boxed: Box<dyn std::error::Error>,
    outcome: RevealOutcome,
    receipt: RevealReceipt,
}
impl Retained {
    fn formatted(&self) -> Vec<String> {
        let status = self.receipt.status();
        let boxed = &self.boxed;
        vec![
            self.error.to_string(),
            format!("{:?}", self.error),
            format!("{:#?}", self.error),
            self.cleanup.to_string(),
            format!("{:?}", self.cleanup),
            format!("{:#?}", self.cleanup),
            format!("{:?}", self.outcome),
            format!("{:#?}", self.outcome),
            format!("{status:?}"),
            format!("{status:#?}"),
            format!("{:?}", self.receipt),
            format!("{:#?}", self.receipt),
            boxed.to_string(),
            format!("{boxed:?}"),
            format!("{boxed:#?}"),
        ]
    }
}

fn retain_after_lifecycle(
    case: RefusalCase,
) -> Result<(Retained, Vec<String>), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 240.0, 1.0)?)?;
    let visible = ui.signal(false);
    let dialog = DialogController::new(&ui);
    let content = || Element::new("target", Button::new("Target", |_| Ok(())));
    let element = match case {
        RefusalCase::Dialog => Element::new(
            "owner",
            Dialog::new(
                dialog.clone(),
                Element::new("opener", Button::new("Open", |_| Ok(()))),
                content(),
            ),
        ),
        RefusalCase::BoundView => Element::new(
            "owner",
            View::new(content()).visible_signal(visible.read_only()),
        ),
        RefusalCase::ExplicitHide => Element::new("owner", Container::new(content())),
    };
    let fixture = ui.mount_tree(Element::new(
        "fixture",
        Column::new([
            element,
            Element::new("requester", Button::new("Request", |_| Ok(()))),
        ]),
    ))?;
    let root = ui
        .find_descendant(&fixture, "owner")?
        .ok_or("owner missing")?;
    let requester = ui
        .find_descendant(&fixture, "requester")?
        .ok_or("requester missing")?;
    ui.flush(Duration::ZERO)?;
    let target = ui
        .find_descendant(&root, "target")?
        .ok_or("target missing")?;
    if matches!(case, RefusalCase::ExplicitHide) {
        ui.set_visible(&root, false)?;
    }
    let outcome = ui.reveal(&target)?;
    let RevealOutcome::Refused(ref refusal) = outcome else {
        panic!("expected refusal: {case:?}")
    };
    let owner = match (case, refusal) {
        (RefusalCase::Dialog, RevealRefusal::AncestorRefused { ancestor })
        | (RefusalCase::BoundView, RevealRefusal::ApplicationControlled { ancestor }) => ancestor,
        (RefusalCase::ExplicitHide, RevealRefusal::Hidden { component }) => component,
        _ => panic!("wrong historical reason: {case:?}: {refusal:?}"),
    }
    .clone();
    let serial = ui.inspection_id(&owner)?;
    let expected = match case {
        RefusalCase::Dialog => {
            format!("AncestorRefused {{ ancestor: WidgetId {{ serial: {serial}, .. }} }}")
        }
        RefusalCase::BoundView => {
            format!("ApplicationControlled {{ ancestor: WidgetId {{ serial: {serial}, .. }} }}")
        }
        RefusalCase::ExplicitHide => {
            format!("Hidden {{ component: WidgetId {{ serial: {serial}, .. }} }}")
        }
    };
    assert_eq!(format!("{refusal:?}"), expected);
    let error = Driver::new(&mut ui)
        .click(&target)
        .expect_err("closed path must refuse click");
    assert!(matches!(error, TestError::NotRevealed { .. }));
    let receipt = ui.event(&requester, |cx| cx.request_reveal(&target))??;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        receipt.status(),
        RevealStatus::Finished(Ok(outcome.clone()))
    );
    let cleanup = TestError::Cleanup {
        error: Box::new(error.clone()),
        cleanup: RuntimeError::Closed,
    };
    let retained = Retained {
        boxed: Box::new(error.clone()),
        error,
        cleanup,
        outcome,
        receipt,
    };
    let initial = retained.formatted();
    assert!(
        initial.iter().all(|s| !s.contains("mounted")),
        "{initial:?}"
    );
    assert!(initial[0].contains(&format!("serial: {serial}")));
    assert!(format!("{owner:?}").contains("mounted: true"));
    match case {
        RefusalCase::Dialog => dialog.open()?,
        RefusalCase::BoundView => visible.set(true)?,
        RefusalCase::ExplicitHide => ui.set_visible(&root, true)?,
    }
    ui.flush(Duration::ZERO)?;
    assert!(ui.reveal(&target)?.is_revealed());
    assert_eq!(initial, retained.formatted(), "after opening: {case:?}");
    ui.unmount(&root)?;
    assert!(format!("{owner:?}").contains("mounted: false"));
    assert_eq!(initial, retained.formatted(), "after unmount: {case:?}");
    drop(ui);
    assert_eq!(
        initial,
        retained.formatted(),
        "after runtime drop: {case:?}"
    );
    Ok((retained, initial))
}

#[test]
fn refusal_text_is_historical_through_open_unmount_drop_and_return() -> Check {
    for case in [
        RefusalCase::Dialog,
        RefusalCase::BoundView,
        RefusalCase::ExplicitHide,
    ] {
        let (retained, before) = retain_after_lifecycle(case)?;
        assert_eq!(
            before,
            retained.formatted(),
            "after function return: {case:?}"
        );
    }
    assert_eq!(format!("{:?}", RevealRefusal::ModalBlocked), "ModalBlocked");
    assert_eq!(
        format!("{:#?}", RevealRefusal::ModalBlocked),
        "ModalBlocked"
    );
    Ok(())
}

#[test]
fn host_hidden_window_is_an_operational_reveal_error_and_can_be_retried() -> Check {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 240.0, 1.0)?)?;
    let fixture = ui.mount_tree(Element::new(
        "fixture",
        Column::new([
            Element::new("target", Button::new("Target", |_| Ok(()))),
            Element::new(
                "tabs",
                Tabs::new([Tab::new("a", "A", Element::new("body", Label::new("Body")))]),
            ),
        ]),
    ))?;
    let target = ui
        .find_descendant(&fixture, "target")?
        .ok_or("target missing")?;
    let tabs = ui
        .find_descendant(&fixture, "tabs")?
        .ok_or("tabs missing")?;
    ui.flush(Duration::ZERO)?;
    let receipt = ui.event(&target, |cx| cx.request_reveal_and_focus(&target))??;
    let reveal_only = ui.event(&target, |cx| cx.request_reveal(&target))??;
    ui.set_window_visible(false)?;
    assert_eq!(ui.reveal(&target), Err(RuntimeError::Hidden));
    assert_eq!(ui.reveal_and_focus(&target), Err(RuntimeError::Hidden));
    assert_eq!(
        ui.jump_to_content(&ContentTarget::Widget(target.clone())),
        Err(RuntimeError::Hidden)
    );
    assert_eq!(
        Driver::new(&mut ui).click(&target),
        Err(TestError::Runtime(RuntimeError::Hidden))
    );
    assert_eq!(
        Driver::new(&mut ui).click_item(&tabs, 0),
        Err(TestError::Runtime(RuntimeError::Hidden))
    );
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        receipt.status(),
        RevealStatus::Finished(Err(RuntimeError::Hidden))
    );
    assert_eq!(
        reveal_only.status(),
        RevealStatus::Finished(Err(RuntimeError::Hidden))
    );
    ui.set_window_visible(true)?;
    assert!(ui.reveal_and_focus(&target)?.is_revealed());
    Driver::new(&mut ui).click(&target)?;
    Driver::new(&mut ui).click_item(&tabs, 0)?;
    Ok(())
}
