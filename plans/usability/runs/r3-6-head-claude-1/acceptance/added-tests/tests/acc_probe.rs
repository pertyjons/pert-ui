//! Verifier probe (diagnostic only): prints how dialogs and list rows appear
//! in the inspection snapshot, to calibrate the acceptance helpers.
use pert_ui::prelude::*;
use pert_ui::testing::*;
use pert_ui::SemanticRole;
use std::path::PathBuf;

fn dump(d: &Driver<'_>, label: &str) {
    eprintln!("== {label}");
    for c in d.runtime().inspect().components {
        if let Some(s) = &c.details.semantics {
            if matches!(s.role, SemanticRole::Dialog | SemanticRole::AlertDialog | SemanticRole::Button | SemanticRole::Notifications) {
                eprintln!("  {:?} key={} path={:?} visible={} name={:?} items={}", s.role, c.key, c.path, c.visible, s.name, s.items.len());
            }
        }
    }
}

#[test]
fn probe() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("acc-probe");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("b.txt"), "x")?;
    std::fs::write(dir.join("a.txt"), "y")?;
    let mut app = notes_editor::app(dir).headless()?;
    let (ui, _) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    dump(&d, "start");
    d.chord(KeyChord::primary('n'), KeyCode::KeyN)?;
    dump(&d, "after Ctrl+N");
    d.key(KeyCode::Escape)?;
    dump(&d, "after Escape");
    Ok(())
}
