//! Exploration (to be replaced): ordering of a deferred refocus against input.
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;

#[derive(Clone)]
struct Note {
    key: String,
    text: Signal<String>,
}
impl PartialEq for Note {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.text.read_only().same_source(&other.text.read_only())
    }
}

fn editor_key(key: &str) -> String {
    format!("editor-{key}")
}

#[test]
fn explore_race() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 300.0, 1.0)?)?;
    let notes = ui.signal(
        ["a", "b", "c"]
            .map(|key| Note { key: key.to_owned(), text: ui.signal(format!("text {key}")) })
            .to_vec(),
    );
    let selected = ui.signal(Some("a".to_owned()));
    let observed = selected.clone();
    let root = ui.mount_tree(
        Column::new([
            TextField::new(ui.signal(String::new())).key("sidebar"),
            Dynamic::new(&notes, move |_, notes| {
                let observed = observed.clone();
                Tabs::new(notes.iter().map(|note| {
                    Tab::new(note.key.clone(), note.key.clone(), TextArea::new(&note.text).key(editor_key(&note.key)))
                }))
                .on_select(move |cx, change| {
                    println!("on_select {:?} -> {:?} {:?} focused={}", change.previous, change.selected, change.origin, cx.focused());
                    observed.set(change.selected.clone())?;
                    if change.origin == ChangeOrigin::Redescription && cx.focused()
                        && let Some(key) = change.selected.clone()
                    {
                        let (tabs, current) = (cx.id(), observed.clone());
                        cx.defer(move |ui| {
                            if std::env::var_os("NESTED").is_some() { ui.flush(ui.now())?; }
                            let editor = ui.find_descendant(&tabs, &editor_key(&key))?;
                            println!(
                                "task: focused_is_tabs={} observed={:?} key={key} can_focus={:?}",
                                ui.focused().as_ref() == Some(&tabs),
                                current.get()?,
                                editor.as_ref().map(|e| ui.can_focus(e)),
                            );
                            if let Some(editor) = editor { println!("focus -> {:?}", ui.focus(&editor)); }
                            Ok(())
                        })?;
                    }
                    Ok(())
                })
                .key("tabs")
            })
            .key("notes")
            .with_grow(1),
        ])
        .key("app"),
    )?;
    ui.flush(Duration::ZERO)?;
    let tabs = ui.find_descendant(&root, "tabs")?.ok_or("tabs")?;
    let a = ui.find_descendant(&tabs, "editor-a")?.ok_or("a")?;
    ui.focus(&a)?;
    notes.update(|n| n.retain(|n| n.key != "a"))?;
    ui.flush(Duration::ZERO)?;
    let fk = |ui: &Runtime| ui.focused().map(|f| ui.key(&f).map(str::to_owned).unwrap_or_default());
    println!("after removal flush: focus={:?} selected={:?}", fk(&ui), selected.get()?);
    // Input before the next flush: ArrowRight on the focused strip.
    ui.key_input(KeyCode::ArrowRight, KeyAction::Press)?;
    println!("after ArrowRight press: focus={:?} selected={:?}", fk(&ui), selected.get()?);
    ui.key_input(KeyCode::ArrowRight, KeyAction::Release)?;
    ui.flush(Duration::ZERO)?;
    println!("end: focus={:?} selected={:?} sem={:?}", fk(&ui), selected.get()?, Driver::new(&mut ui).semantics(&tabs)?.items.iter().map(|i| i.selected).collect::<Vec<_>>());
    Ok(())
}
