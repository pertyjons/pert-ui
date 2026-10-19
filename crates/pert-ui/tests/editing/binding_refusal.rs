//! Binding refusal reports through the facade: a field tells the application
//! when it refuses a value the application wrote, and when that ends.
use pert_ui::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Log = Rc<RefCell<Vec<Option<TextEditError>>>>;

fn field(ui: &mut Runtime, value: &Signal<String>, log: &Log) -> Result<WidgetId, RuntimeError> {
    let record = log.clone();
    ui.mount_tree(
        TextField::new(value)
            .normalize(|text: String| {
                if text == "reject" {
                    Err(InputRejection::new("not allowed"))
                } else {
                    Ok(text)
                }
            })
            .on_binding_refusal(move |_, refusal| {
                record.borrow_mut().push(refusal.cloned());
                Ok(())
            })
            .key("field"),
    )
}

#[test]
fn each_refusal_and_its_end_are_reported_once() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal("ok".to_owned());
    let log: Log = Rc::default();
    field(&mut ui, &value, &log)?;
    ui.flush(Duration::ZERO)?;
    assert!(log.borrow().is_empty());
    value.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
    ui.flush(Duration::ZERO)?;
    // Another reason reports again; a single-line field refuses a line break.
    value.set("two\nlines".to_owned())?;
    ui.flush(Duration::ZERO)?;
    value.set("fine".to_owned())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        *log.borrow(),
        [
            Some(TextEditError::TooLarge),
            Some(TextEditError::InvalidText),
            None
        ]
    );
    // A refusal that starts and ends between two flushes reports nothing.
    value.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
    value.set("fine again".to_owned())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(log.borrow().len(), 3);
    Ok(())
}

#[test]
fn refused_typing_and_a_normalizer_rejection_are_not_binding_refusals() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal(String::new());
    let log: Log = Rc::default();
    let id = field(&mut ui, &value, &log)?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    ui.text_input("reject")?;
    ui.key_input(KeyCode::Enter, KeyAction::Press)?;
    ui.key_input(KeyCode::Enter, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    assert!(log.borrow().is_empty());
    Ok(())
}

#[test]
fn a_failing_report_keeps_the_refusal_and_a_new_description_can_stop_reports() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal("ok".to_owned());
    let describe = |report: bool| {
        let mut field = TextArea::new(&value);
        if report {
            field = field.on_binding_refusal(|_, _| Err(RuntimeError::Exhausted));
        }
        vec![field.key("area")]
    };
    let root = ui.mount_tree(Column::new(describe(true)).key("root"))?;
    ui.flush(Duration::ZERO)?;
    value.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
    assert_eq!(
        ui.flush(Duration::ZERO).err(),
        Some(RuntimeError::Exhausted)
    );
    ui.reconcile_children(&root, describe(false))?;
    value.set("short".to_owned())?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(value.get()?, "short");
    Ok(())
}

#[test]
fn a_refusal_and_a_discarded_draft_are_reported_in_order() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal("saved".to_owned());
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (modified, refused) = (log.clone(), log.clone());
    let id = ui.mount_tree(
        TextField::new(&value)
            .normalize(Ok)
            .on_modified(move |_, dirty| {
                modified.borrow_mut().push(format!("modified {dirty}"));
                Ok(())
            })
            .on_binding_refusal(move |_, refusal| {
                refused
                    .borrow_mut()
                    .push(format!("refused {}", refusal.is_some()));
                Ok(())
            })
            .key("field"),
    )?;
    ui.flush(Duration::ZERO)?;
    ui.focus(&id)?;
    ui.text_input("!")?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(*log.borrow(), ["modified true"]);
    // The application writes a refused value: the draft is discarded unsaved.
    value.set("x".repeat(TextEditError::MAX_TEXT_BYTES + 1))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(
        *log.borrow(),
        ["modified true", "refused true", "modified false"]
    );
    Ok(())
}

#[test]
fn a_callback_installed_on_a_refusing_field_reports_the_refusal() -> TestResult {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(300.0, 100.0, 1.0)?)?;
    let value = ui.signal("x".repeat(TextEditError::MAX_TEXT_BYTES + 1));
    let log: Log = Rc::default();
    let record = log.clone();
    let describe = move |report: bool| {
        let mut field = TextField::new(&value).on_modified(|_, _| Ok(()));
        if report {
            let record = record.clone();
            field = field.on_binding_refusal(move |_, refusal| {
                record.borrow_mut().push(refusal.cloned());
                Ok(())
            });
        }
        vec![field.key("field")]
    };
    let root = ui.mount_tree(Column::new(describe(false)).key("root"))?;
    ui.flush(Duration::ZERO)?;
    ui.reconcile_children(&root, describe(true))?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(*log.borrow(), [Some(TextEditError::TooLarge)]);
    Ok(())
}
