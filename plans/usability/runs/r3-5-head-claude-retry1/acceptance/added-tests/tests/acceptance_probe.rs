//! Verifier probe (acceptance copy only): prints what the library's
//! inspection reports for the monitor, so the acceptance tests are grounded
//! in observed inspection paths. It asserts little; run with --nocapture.

use job_monitor::*;
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::collections::HashMap;
use std::time::Duration;

#[test]
fn dump_inspection() -> Result<(), Box<dyn std::error::Error>> {
    let mut plans: HashMap<u32, JobPlan> = HashMap::new();
    plans.insert(1, JobPlan { total: 100, steps: vec![20; 5], fail_at_tick: Some(2) });
    plans.insert(2, JobPlan { total: 100, steps: vec![10; 10], fail_at_tick: None });
    let clock = ManualClock::new();
    let env = Env {
        clock: clock.clone(),
        planner: Box::new(move |n: u32| plans.remove(&n).expect("plan")),
    };
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(800.0, 360.0, 1.0)?)?;
    let monitor = build(&mut ui, env)?;
    let mut driver = Driver::new(&mut ui);
    driver.settle()?;
    driver.click(&monitor.start)?;
    driver.advance(Duration::from_millis(600))?;
    driver.click(&monitor.start)?;
    driver.advance(Duration::from_millis(600))?;
    for _ in 0..2 {
        clock.advance(TICK);
        std::thread::sleep(Duration::from_millis(100));
        driver.wait_until(Duration::from_millis(50), |_| Ok(true))?;
    }
    driver.settle()?;
    let snapshot = driver.runtime().inspect();
    println!("schema {}", snapshot.schema_version);
    for c in &snapshot.components {
        let s = c.details.semantics.as_ref();
        println!(
            "{:?} key={} role={:?} name={:?} value={:?} items={:?} border={:?}",
            c.path,
            c.key,
            s.map(|s| s.role),
            s.and_then(|s| s.name.clone()),
            s.and_then(|s| s.value.clone()),
            s.map(|s| s
                .items
                .iter()
                .map(|i| (i.name.clone(), i.selected, i.bounds))
                .collect::<Vec<_>>()),
            c.details.box_model.border_box
        );
    }
    Ok(())
}
