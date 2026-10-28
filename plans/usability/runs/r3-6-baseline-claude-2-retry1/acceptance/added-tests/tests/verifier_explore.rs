//! Verifier exploration (acceptance copy only): dumps the inspection
//! snapshot around the menu bar and dialogs so the acceptance tests can
//! address them. Prints only; asserts nothing about requirements.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::fs;

fn dump(ui: &Runtime, label: &str) {
    println!("==== {label}");
    for c in ui.inspect().components {
        if !c.visible {
            continue;
        }
        let s = c.details.semantics.as_ref();
        println!(
            "{:>4} parent={:?} key={:?} role={:?} name={:?} value={:?} items={:?}",
            c.id,
            c.parent,
            c.key,
            s.map(|s| s.role),
            s.and_then(|s| s.name.clone()),
            s.and_then(|s| s.value.clone()),
            s.map(|s| s.items.iter().map(|i| (i.name.clone(), i.selected)).collect::<Vec<_>>()),
        );
    }
}

#[test]
fn explore() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("verifier-explore-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("b.txt"), "b")?;
    fs::write(dir.join("a.txt"), "a")?;
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(900.0, 600.0, 1.0)?)?;
    let root = notes::build(&mut ui, dir.clone())?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    let list = d.runtime().find_descendant(&root, "notes")?.ok_or("no list")?;
    println!("list semantics: {:?}", d.semantics(&list)?);
    let bar = d.runtime().find_descendant(&root, "bar")?.ok_or("no bar")?;
    let title = d.runtime().find_descendant(&bar, "title/file")?.ok_or("no title")?;
    println!("title bounds {:?}", d.runtime().bounds(&title)?);
    d.click(&title)?;
    dump(d.runtime(), "after clicking File");
    if let Some(items) = d.runtime().find_descendant(&title, "items")? {
        println!("items bounds {:?}", d.runtime().bounds(&items)?);
    }
    d.key(KeyCode::Escape)?;
    let row = d.find(&list, &["a.txt"])?;
    d.click(&row)?;
    d.type_text("x")?;
    d.chord(KeyChord::primary('w'), KeyCode::KeyW)?;
    dump(d.runtime(), "close dialog open");
    Ok(())
}
