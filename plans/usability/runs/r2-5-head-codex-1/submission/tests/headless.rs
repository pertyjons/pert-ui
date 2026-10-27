use job_monitor::{Monitor, Plan, State, build};
use pert_ui::prelude::*;
use pert_ui::testing::*;
use std::time::Duration;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn wait(d: &mut Driver<'_>, m: &Monitor, id: u64, processed: u64, state: State) -> Result {
    d.wait_until(Duration::from_secs(2), |_| {
        Ok(m.jobs
            .get()?
            .iter()
            .any(|j| j.id == id && j.processed == processed && j.state == state))
    })?;
    Ok(())
}
fn names(d: &Driver<'_>) -> Vec<String> {
    d.runtime()
        .query(
            &InspectionQuery::components(Selector::descendants(["monitor"]))
                .detail(Detail::Semantics),
        )
        .components
        .into_iter()
        .filter_map(|c| c.details.semantics.and_then(|s| s.name))
        .collect()
}
#[test]
fn full_headless_session_and_cpu_capture() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1100.0, 560.0, 1.0)?)?;
    let m = build(
        &mut ui,
        Some(vec![
            Plan {
                total: 50,
                step: 20,
                fail_at: None,
            },
            Plan {
                total: 120,
                step: 15,
                fail_at: None,
            },
            Plan {
                total: 100,
                step: 10,
                fail_at: Some(20),
            },
            Plan {
                total: 500,
                step: 5,
                fail_at: None,
            },
        ]),
    )?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    assert_eq!(d.runtime().next_deadline(), None);
    let start = d.find(&m.root, &["start"])?;
    for _ in 0..4 {
        d.click(&start)?;
    }
    let summary = d.find(&m.root, &["summary"])?;
    assert_eq!(
        d.semantics(&summary)?.name.as_deref(),
        Some("Running: 4   Done: 0   Cancelled: 0   Failed: 0")
    );
    m.tick(1);
    wait(&mut d, &m, 1, 20, State::Running)?;
    assert!(names(&d).contains(&"40%".to_string()));
    m.tick(1);
    wait(&mut d, &m, 1, 40, State::Running)?;
    m.tick(1);
    wait(&mut d, &m, 1, 50, State::Done)?;
    m.tick(2);
    wait(&mut d, &m, 2, 15, State::Running)?;
    // Find the actual Cancel button in the second row through owned inspection paths.
    let cancel = d
        .runtime()
        .query(
            &InspectionQuery::components(Selector::descendants(["monitor", "jobs"]))
                .detail(Detail::Semantics),
        )
        .components
        .into_iter()
        .find(|c| c.path.as_ref().is_some_and(|p| p.iter().any(|s| s == "2")) && c.key == "cancel")
        .expect("Job 2 Cancel button");
    let path = cancel.path.unwrap();
    let target = d.find(
        &m.root,
        &path[1..].iter().map(String::as_str).collect::<Vec<_>>(),
    )?;
    d.click(&target)?;
    wait(&mut d, &m, 2, 15, State::Cancelled)?;
    m.tick(2);
    d.advance(Duration::from_millis(300))?;
    assert_eq!(m.jobs.get()?[1].processed, 15);
    m.tick(3);
    wait(&mut d, &m, 3, 10, State::Running)?;
    m.tick(3);
    wait(&mut d, &m, 3, 20, State::Failed)?;
    m.tick(4);
    wait(&mut d, &m, 4, 5, State::Running)?;
    assert!(names(&d).contains(&"failed: simulated error".into()));
    let notification = d.find(&m.root, &["notification"])?;
    // Banner's message is represented by its child label.
    assert!(names(&d).iter().any(|s| s.contains("Job 3 failed")));
    let _ = notification;
    assert_eq!(
        d.semantics(&summary)?.name.as_deref(),
        Some("Running: 1   Done: 1   Cancelled: 1   Failed: 1")
    );
    // Keyboard sorting traverses the same input path and application callback as headers.
    let table = d.find(&m.root, &["jobs"])?;
    let body = d.runtime().children(&table)?[0].clone();
    for col in ["name", "progress", "state", "elapsed"] {
        for (key, descending) in [(KeyCode::ArrowUp, false), (KeyCode::ArrowDown, true)] {
            m.table.selection().set_active(Some("1".into()))?;
            m.table.set_active_column(Some(col.into()))?;
            d.runtime_mut().focus(&body)?;
            d.runtime_mut()
                .key_input(KeyCode::AltLeft, KeyAction::Press)?;
            d.runtime_mut().key_input(key, KeyAction::Press)?;
            d.runtime_mut().key_input(key, KeyAction::Release)?;
            d.runtime_mut()
                .key_input(KeyCode::AltLeft, KeyAction::Release)?;
            d.settle()?;
            assert_eq!(m.table.sort().get()?.unwrap().column, col);
            let expected = match (col, descending) {
                ("name", false) => vec![1, 2, 3, 4],
                ("name", true) => vec![4, 3, 2, 1],
                ("progress", false) => vec![4, 2, 3, 1],
                ("progress", true) => vec![1, 3, 2, 4],
                ("state", false) => vec![2, 1, 3, 4],
                ("state", true) => vec![4, 3, 1, 2],
                ("elapsed", false) => vec![2, 4, 3, 1],
                ("elapsed", true) => vec![1, 3, 2, 4],
                _ => unreachable!(),
            };
            assert_eq!(
                m.displayed()?.iter().map(|j| j.id).collect::<Vec<_>>(),
                expected,
                "{col} descending={descending}"
            );
            let mut painted_order: Vec<_> = d
                .runtime()
                .query(
                    &InspectionQuery::components(Selector::descendants(["monitor", "jobs"]))
                        .detail(Detail::Semantics),
                )
                .components
                .into_iter()
                .filter_map(|c| {
                    let name = c.details.semantics?.name?;
                    let id = name.strip_prefix("Job ")?.parse::<u64>().ok()?;
                    Some((c.details.box_model.border_box?[1], id))
                })
                .collect();
            painted_order.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert_eq!(
                painted_order.iter().map(|(_, id)| *id).collect::<Vec<_>>(),
                expected,
                "actual visible rows for {col}"
            );
        }
    }
    d.runtime().set_capture_support(true);
    let pending = d
        .runtime()
        .request_capture(CaptureRequest::new(CaptureTarget::window(
            &d.runtime().primary_window(),
        )));
    d.settle()?;
    CpuRenderer::new().serve_captures(d.runtime_mut());
    let capture = pending.wait()?.value?;
    std::fs::write("/work/job-monitor.png", capture.image.encode_png())?;
    // Complete the remaining worker, then demonstrate no animation/timer work remains.
    for n in 2..=100 {
        m.tick(4);
        wait(
            &mut d,
            &m,
            4,
            n * 5,
            if n == 100 {
                State::Done
            } else {
                State::Running
            },
        )?;
    }
    assert_eq!(
        d.semantics(&summary)?.name.as_deref(),
        Some("Running: 0   Done: 2   Cancelled: 1   Failed: 1")
    );
    d.settle()?;
    assert_eq!(d.runtime().next_deadline(), None);
    for _ in 0..3 {
        let report = d.advance(Duration::from_secs(1))?;
        assert_eq!(report.components_updated, 0);
        assert_eq!(report.next_deadline, None);
    }
    Ok(())
}
