//! Opening an editor with its value selected: focus and `SelectAll` requested
//! in one event, in either order, before or after the editor first lays out.
use crate::support;
use pert_ui::*;
use support::frame;

/// Focuses its only child and selects its text from its first delivery.
struct Opener {
    child: Option<Element>,
    target: Option<WidgetId>,
    focus_first: bool,
}
impl Component for Opener {
    fn take_children(&mut self) -> Vec<Element> {
        self.child.take().into_iter().collect()
    }
    fn update(&mut self, cx: &mut UpdateContext) -> Result<(), RuntimeError> {
        if self.target.is_none() {
            self.target = cx.children().first().map(|child| child.id().clone());
            cx.notifier().request();
        }
        Ok(())
    }
    fn deliver_notifications(&mut self, cx: &mut EventContext) -> Result<(), RuntimeError> {
        let Some(target) = self.target.clone() else {
            return Ok(());
        };
        if self.focus_first {
            cx.request_focus_within(&target)?;
            cx.request_text_command(&target, TextCommand::SelectAll)?;
        } else {
            cx.request_text_command(&target, TextCommand::SelectAll)?;
            cx.request_focus_within(&target)?;
        }
        Ok(())
    }
}

type Build = fn(&Runtime) -> Element;

/// Each built-in editor over a value that typing "12" must replace.
fn editors() -> [(&'static str, Build); 5] {
    [
        ("text field", |ui| {
            Element::new("editor", TextField::new(ui.signal("27".to_owned())))
        }),
        ("text area", |ui| {
            Element::new("editor", TextArea::new(ui.signal("27".to_owned())))
        }),
        ("search field", |ui| {
            Element::new("editor", SearchField::new(ui.signal("27".to_owned())))
        }),
        ("integer", |ui| {
            Element::new("editor", IntegerInput::new(ui.signal(27_i64)))
        }),
        ("decimal", |ui| {
            Element::new("editor", DecimalInput::new(ui.signal(27.0_f64)))
        }),
    ]
}

fn text(ui: &Runtime) -> InspectValue {
    ui.inspect()
        .components
        .iter()
        .find(|c| c.key == "editor")
        .expect("editor mounted")
        .details
        .properties["text"]
        .clone()
}

fn opener(editor: Element, focus_first: bool) -> Element {
    Element::new(
        "opener",
        Opener {
            child: Some(editor),
            target: None,
            focus_first,
        },
    )
}

fn runtime() -> Runtime {
    let ui = Runtime::new();
    ui.set_viewport(Viewport::new(320.0, 200.0, 1.0).unwrap())
        .unwrap();
    ui
}

#[test]
fn editors_open_selected_whichever_request_comes_first() {
    for (name, build) in editors() {
        for focus_first in [true, false] {
            let mut ui = runtime();
            let editor = build(&ui);
            ui.mount_tree(opener(editor, focus_first)).unwrap();
            frame(&mut ui);
            ui.text_input("12").unwrap();
            frame(&mut ui);
            assert_eq!(
                text(&ui),
                InspectValue::Text("12".into()),
                "{name}, {focus_first}"
            );
        }
    }
}

/// As a table opens a cell editor: the draft binding is set and the editor
/// mounted in the same step, then focus and select-all requested.
#[test]
fn integer_input_opens_selected_over_a_draft_set_while_mounting() {
    for focus_first in [true, false] {
        let mut ui = runtime();
        let draft = ui.signal(5_i64);
        let open = ui.signal(false);
        let bound = draft.clone();
        ui.mount_tree(Element::new(
            "cell",
            Show::new(&open, move |_| {
                opener(
                    Element::new("editor", IntegerInput::new(bound.clone())),
                    focus_first,
                )
            }),
        ))
        .unwrap();
        frame(&mut ui);
        draft.set(27).unwrap();
        open.set(true).unwrap();
        frame(&mut ui);
        ui.text_input("12").unwrap();
        frame(&mut ui);
        assert_eq!(text(&ui), InspectValue::Text("12".into()), "{focus_first}");
    }
}
