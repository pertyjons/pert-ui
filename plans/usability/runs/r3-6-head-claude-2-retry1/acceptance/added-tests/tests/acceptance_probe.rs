//! Acceptance verifier exploration (not a pass/fail check): prints the
//! inspection records of the menu bar before and after a pointer click on
//! its File title, to find how a test can click its rows.
use notes::Notes;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::path::PathBuf;

fn dump(driver: &Driver<'_>, label: &str) {
    println!("==== {label}");
    for c in driver.runtime().inspect().components {
        if !c.visible {
            continue;
        }
        let Some(s) = c.details.semantics.as_ref() else { continue };
        println!(
            "{:?} key={:?} role={:?} name={:?} value={:?} border={:?}",
            c.path, c.key, s.role, s.name, s.value, c.details.box_model.border_box
        );
        for item in &s.items {
            println!("    item name={:?} bounds={:?} shortcut={:?}", item.name, item.bounds, item.shortcut);
        }
    }
}

#[test]
fn probe_menu_bar() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("acceptance-probe");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("a.txt"), "x")?;
    let mut app = notes::app(dir).headless()?;
    let (rt, notes) = app.runtime_and_state_mut();
    let root = notes.root.clone();
    let mut driver = Driver::new(rt);
    driver.settle()?;
    let bar = driver.find(&root, &Notes::content_path())?;
    for child in driver.address_children(&bar)? {
        println!("bar child segment={:?}", child.segment);
    }
    dump(&driver, "before");
    let title = driver.runtime().inspect().components.into_iter()
        .find(|c| c.visible && c.details.semantics.as_ref().is_some_and(|s| s.role == SemanticRole::Menu && s.name.as_deref() == Some("File")))
        .ok_or("no File title")?;
    let [x, y, w, h] = title.details.box_model.border_box.ok_or("no bounds")?;
    let at = Point::new(x + w / 2.0, y + h / 2.0)?;
    driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Press(PointerButton::Primary)))?;
    driver.runtime_mut().pointer_input(PointerEvent::new(at, PointerKind::Release(PointerButton::Primary)))?;
    driver.settle()?;
    dump(&driver, "after clicking File");
    Ok(())
}
