//! Conformance helpers own their test roots and compare actual incarnations.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(400.0, 300.0, 1.0).unwrap())
        .unwrap();
    ui
}
fn label(text: &ReadSignal<String>) -> Element {
    Element::new("c", Label::new(text.clone()))
}
fn succeeds_again(ui: &mut Runtime) {
    let item = || Element::new("c", Label::new("Ready"));
    let result = check_redescription(ui, item(), item(), item()).unwrap();
    assert_eq!(ui.key(&result.component).unwrap(), "c");
    // Successful comparison keeps the retained tree, but frees the reference key.
    let fresh = ui
        .mount_tree(Element::new("fresh", Label::new("Free")))
        .unwrap();
    ui.unmount(&fresh).unwrap();
    ui.unmount(&result.root).unwrap();
}
#[test]
fn failed_first_and_second_flush_release_owned_roots() {
    for fail_first in [true, false] {
        let mut ui = runtime();
        let local = ui.signal("Local".to_owned()).read_only();
        let foreign = Runtime::new().signal("Foreign".to_owned()).read_only();
        let (before, after) = if fail_first {
            (label(&foreign), label(&local))
        } else {
            (label(&local), label(&foreign))
        };
        assert!(matches!(
            pert_ui::redescribe(&mut ui, before, after),
            Err(RedescriptionError::Runtime(RuntimeError::ForeignRuntime))
        ));
        succeeds_again(&mut ui);
    }
}
struct Checked(bool);
impl Component for Checked {
    fn check_description(&self) -> Result<(), RuntimeError> {
        if self.0 {
            Err(RuntimeError::EmptyKey)
        } else {
            Ok(())
        }
    }
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
}
#[test]
fn reconcile_and_different_type_rejections_release_owned_roots() {
    let mut ui = runtime();
    assert!(matches!(
        pert_ui::redescribe(
            &mut ui,
            Element::new("c", Checked(false)),
            Element::new("c", Checked(true))
        ),
        Err(RedescriptionError::Runtime(RuntimeError::EmptyKey))
    ));
    succeeds_again(&mut ui);
    assert!(matches!(
        pert_ui::redescribe(
            &mut ui,
            Element::new("c", Label::new("Old")),
            Element::new("c", Button::new("New", |_| Ok(())))
        ),
        Err(RedescriptionError::Runtime(RuntimeError::ChildTypeChanged(key))) if key == "c"
    ));
    succeeds_again(&mut ui);
}
struct Sticky(&'static str);
impl Component for Sticky {
    fn update(&mut self, _: &mut UpdateContext) -> Result<(), RuntimeError> {
        Ok(())
    }
    fn inspect(&self) -> ComponentDetails {
        ComponentDetails {
            properties: [("caption".into(), InspectValue::Text(self.0.into()))].into(),
            ..ComponentDetails::default()
        }
    }
}
#[test]
fn differing_reports_and_reference_failures_release_both_owned_roots() {
    let mut ui = runtime();
    assert!(matches!(
        check_redescription(
            &mut ui,
            Element::new("c", Sticky("Old")),
            Element::new("c", Sticky("New")),
            Element::new("c", Sticky("New"))
        ),
        Err(RedescriptionError::Differs { .. })
    ));
    succeeds_again(&mut ui);
    let item = || Element::new("c", Label::new("Ready"));
    assert!(matches!(
        check_redescription(&mut ui, item(), item(), Element::new("c", Checked(true))),
        Err(RedescriptionError::Runtime(RuntimeError::EmptyKey))
    ));
    succeeds_again(&mut ui);
    let foreign = Runtime::new().signal("Foreign".to_owned()).read_only();
    assert!(matches!(
        check_redescription(&mut ui, item(), item(), label(&foreign)),
        Err(RedescriptionError::Runtime(RuntimeError::ForeignRuntime))
    ));
    succeeds_again(&mut ui);
}
#[test]
fn duplicate_fixed_keys_preserve_unrelated_roots_and_release_only_owned_roots() {
    for key in ["fresh", "redescribe"] {
        let mut ui = runtime();
        let unrelated = ui
            .mount_tree(Element::new(key, Label::new("Unrelated")))
            .unwrap();
        let item = || Element::new("c", Label::new("Ready"));
        assert!(matches!(
            check_redescription(&mut ui, item(), item(), item()),
            Err(RedescriptionError::Runtime(RuntimeError::DuplicateKey(_)))
        ));
        assert_eq!(ui.key(&unrelated).unwrap(), key);
        ui.unmount(&unrelated).unwrap();
        succeeds_again(&mut ui);
    }
}
#[test]
fn comparison_is_independent_of_workspace_addressing() {
    for metadata in 0..4 {
        let mut ui = runtime();
        let describe = |text| {
            let item = Element::new("c", Label::new(text));
            match metadata {
                0 => item,
                1 => item.structural(),
                2 => item.slot("content"),
                _ => item.part("caption"),
            }
        };
        let result =
            check_redescription(&mut ui, describe("Old"), describe("New"), describe("New"))
                .unwrap();
        assert_eq!(ui.key(&result.component).unwrap(), "c");
        ui.unmount(&result.root).unwrap();
    }
}
#[test]
fn nested_duplicate_keys_do_not_replace_the_direct_described_component() {
    let mut ui = runtime();
    let item = || Element::new("c", Column::new([Element::new("c", Label::new("Nested"))]));
    let result = check_redescription(&mut ui, item(), item(), item()).unwrap();
    assert_eq!(
        ui.children(&result.root).unwrap().as_slice(),
        std::slice::from_ref(&result.component)
    );
    assert_eq!(
        ui.key(&ui.children(&result.component).unwrap()[0]).unwrap(),
        "c"
    );
}
#[test]
fn checking_after_elapsed_time_does_not_reverse_the_clock() {
    let mut ui = runtime();
    ui.flush(Duration::from_secs(2)).unwrap();
    succeeds_again(&mut ui);
    assert_eq!(ui.now(), Duration::from_secs(2));
}
struct FreshRemover {
    saved: Rc<RefCell<Option<WidgetId>>>,
    fresh: bool,
    requested: bool,
}
impl Component for FreshRemover {
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if !self.fresh && self.saved.borrow().is_none() {
            *self.saved.borrow_mut() = Some(cx.id());
        }
        if self.fresh && !self.requested {
            self.requested = true;
            cx.notifier().request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let retained = self.saved.borrow().as_ref().unwrap().clone();
        let _task = cx.defer(move |ui| ui.unmount(&retained))?;
        let _reveal = cx.request_reveal(&cx.id())?;
        Ok(())
    }
    fn measure(&self, _: &MeasureContext<'_>, _: Constraints) -> Result<Size, RuntimeError> {
        Ok(Size::new(100.0, 30.0)?)
    }
}
#[test]
fn fresh_flush_revalidates_the_retained_incarnation_and_cleans_up() {
    let mut ui = runtime();
    let saved = Rc::new(RefCell::new(None));
    let item = |fresh| {
        Element::new(
            "c",
            FreshRemover {
                saved: saved.clone(),
                fresh,
                requested: false,
            },
        )
    };
    assert!(matches!(
        check_redescription(&mut ui, item(false), item(false), item(true)),
        Err(RedescriptionError::Remounted { .. })
    ));
    assert!(matches!(
        ui.key(saved.borrow().as_ref().unwrap()),
        Err(RuntimeError::Unmounted)
    ));
    succeeds_again(&mut ui);
}
