//! The application as `main` starts it, through `App`, without a window,
//! and the sidebar divider resizing.

use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::{cell::RefCell, fs, rc::Rc};

#[test]
fn starts_as_an_app_and_the_divider_resizes() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("notes-startup-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("b.txt"), "b")?;
    fs::write(dir.join("a.txt"), "a")?;
    fs::write(dir.join("c.log"), "c")?;

    let built: Rc<RefCell<Option<WidgetId>>> = Rc::default();
    let slot = built.clone();
    let mut app = App::new("Notes", move |ui| {
        *slot.borrow_mut() = Some(notes::build(ui, dir)?);
        Ok(())
    })
    .headless()?;
    let root = built.borrow().clone().ok_or("not built")?;
    let ui = app.runtime_mut();
    let list = ui.find_descendant(&root, "notes")?.ok_or("no list")?;
    let keys: Vec<String> = ui.children(&list)?.iter()
        .map(|row| ui.key(row).map(str::to_owned)).collect::<Result<_, _>>()?;
    assert_eq!(keys, ["a.txt", "b.txt"]);

    // The divider between the sidebar and the tabs: focused, the arrows move it.
    let split = ui.find_descendant(&root, "split")?.ok_or("no split")?;
    let width = |ui: &Runtime| ui.bounds(&list).ok().flatten().map(|b| b.width());
    let before = width(ui).ok_or("no bounds")?;
    let mut driver = Driver::new(ui);
    driver.runtime_mut().focus(&split)?;
    driver.key(KeyCode::ArrowRight)?;
    driver.key(KeyCode::ArrowRight)?;
    let after = width(driver.runtime()).ok_or("no bounds")?;
    assert_eq!(after, before + 16.0);

    // Dragging the divider with the pointer resizes the sidebar too.
    let [x, _, w, _] = driver.runtime().bounds(&list)?.ok_or("no bounds")?.values();
    let start = Point::new(x + w + 4.0, 300.0)?;
    let end = Point::new(x + w + 64.0, 300.0)?;
    for (point, kind) in [
        (start, PointerKind::Move),
        (start, PointerKind::Press(PointerButton::Primary)),
        (end, PointerKind::Move),
        (end, PointerKind::Release(PointerButton::Primary)),
    ] {
        driver.runtime_mut().pointer_input(PointerEvent::new(point, kind))?;
        driver.settle()?;
    }
    assert_eq!(width(driver.runtime()).ok_or("no bounds")?, after + 60.0);
    app.shutdown()?;
    Ok(())
}
