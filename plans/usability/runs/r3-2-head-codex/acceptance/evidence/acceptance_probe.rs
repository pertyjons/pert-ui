// Verifier probe (preserved source; was temporarily at work/src/acceptance_probe.rs
// with `#[cfg(test)] mod acceptance_probe;` appended to main.rs, both removed after
// the run recorded in 04-semantics-probe.log). Dumps list/row semantics.
use super::*;
use pert_ui::testing::*;
#[test]
fn acc_probe_semantics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut app = app().headless()?;
    let (ui, state) = app.runtime_and_state_mut();
    let mut d = Driver::new(ui);
    d.settle()?;
    let root = state.root.clone();
    let list = d.find(&root, &["tasks", "list"])?;
    let field = d.find(&root, &["entry", "new"])?;
    for t in ["Alpha", "Bravo"] { d.replace_text(&field, t)?; d.key(KeyCode::Enter)?; }
    let t1 = d.find(&list, &["1", "title"])?;
    d.click(&t1)?;
    println!("LIST {:#?}", d.semantics(&list)?);
    let row = d.find(&list, &["1"])?;
    println!("ROW1 {:#?}", d.semantics(&row)?);
    println!("ROW1 children {:?}", d.address_children(&row)?.iter().map(|c| c.segment.clone()).collect::<Vec<_>>());
    println!("TITLE1 {:#?}", d.semantics(&t1)?);
    Ok(())
}
