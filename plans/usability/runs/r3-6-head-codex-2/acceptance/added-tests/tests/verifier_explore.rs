//! Verifier exploration (acceptance copy only): prints inspection records of
//! the menu bar, an open menu, the dialogs and the tabs, to learn their
//! public shapes before writing the acceptance checks. Asserts nothing about
//! the application.
use notes_editor::app;
use pert_ui::{prelude::*, testing::*};
use std::{fs, path::PathBuf};

fn dump(d: &Driver<'_>, label: &str, filter: impl Fn(&ComponentInspection) -> bool) {
    println!("==== {label}");
    for c in d.runtime().inspect().components {
        if filter(&c) {
            let sem = c.details.semantics.as_ref();
            println!(
                "id={} key={:?} path={:?} visible={} focused={} props={:?}\n    role={:?} name={:?} value={:?} items={:?}",
                c.id,
                c.key,
                c.path,
                c.visible,
                c.focused,
                c.details.properties,
                sem.map(|s| &s.role),
                sem.and_then(|s| s.name.clone()),
                sem.and_then(|s| s.value.clone()),
                sem.map(|s| s
                    .items
                    .iter()
                    .map(|i| (i.name.clone(), i.selected, i.shortcut.clone(), i.bounds))
                    .collect::<Vec<_>>()),
            );
        }
    }
}

#[test]
fn explore() -> Result<(), Box<dyn std::error::Error>> {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/verifier-explore");
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder)?;
    fs::write(folder.join("b.txt"), "b")?;
    fs::write(folder.join("a.txt"), "a")?;
    let mut app = app(folder.clone()).headless()?;
    let root = app.state().root.clone();
    let mut d = Driver::new(app.runtime_mut());
    d.settle()?;
    dump(&d, "all at start", |_| true);
    let bar = d.runtime().find_descendant(&root, "bar")?.unwrap();
    let title = d
        .runtime()
        .children(&bar)?
        .into_iter()
        .find(|c| d.runtime().key(c).is_ok_and(|k| k == "title/file"))
        .unwrap();
    d.click(&title)?;
    dump(&d, "after clicking File", |c| c.key == "items" || c.key == "title/file" || c.key == "menu");
    // find the visible menu list under the title
    fn walk(rt: &Runtime, at: &WidgetId, out: &mut Vec<WidgetId>) {
        for c in rt.children(at).unwrap() {
            out.push(c.clone());
            walk(rt, &c, out);
        }
    }
    let mut all = Vec::new();
    walk(d.runtime(), &title, &mut all);
    let items = all
        .into_iter()
        .find(|c| d.runtime().key(c).is_ok_and(|k| k == "items"))
        .unwrap();
    let r = d.runtime().bounds(&items)?.unwrap().values();
    println!("items bounds {r:?}");
    let mut y = r[1] + 1.0;
    while y < r[1] + r[3] {
        d.runtime_mut().pointer_input(PointerEvent::new(
            Point::new(r[0] + r[2] / 2.0, y)?,
            PointerKind::Move,
        ))?;
        d.settle()?;
        let sel: Vec<_> = d.semantics(&items)?.items.iter().map(|i| i.selected).collect();
        println!("y={y} selected={sel:?} value={:?}", d.semantics(&items)?.value);
        y += 4.0;
    }
    d.key(KeyCode::Escape)?;
    let a = d.find(&root, &["close-dialog", "bar", "body", "split", "files", "a.txt", "open"])?;
    d.click(&a)?;
    dump(&d, "after open a", |c| c.path.as_ref().is_some_and(|p| p.iter().any(|s| s == "tabs")));
    let editor = d.runtime().find_descendant(&root, "editor")?.unwrap();
    d.click(&editor)?;
    println!("focused editor: {}", d.runtime().focused() == Some(editor.clone()));
    d.type_text("X")?;
    d.key(KeyCode::Enter)?;
    d.type_text("Y Z")?;
    println!("editor value {:?}", d.semantics(&editor)?.value);
    let tabs = d.runtime().find_descendant(&root, "tabs")?.unwrap();
    println!("tabs {:?}", d.semantics(&tabs)?);
    Ok(())
}
