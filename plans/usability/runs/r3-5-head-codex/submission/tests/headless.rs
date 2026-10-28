use job_monitor::{Plan, State, mount};
use pert_ui::{prelude::*, testing::*};
use std::time::Duration;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn text(d: &Driver<'_>, root: &WidgetId, path: &[&str]) -> Result<String> {
    Ok(d.semantics(&d.find(root, path)?)?.name.unwrap_or_default())
}
#[test]
fn monitor_acceptance_and_cpu_capture() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(1000.0, 460.0, 1.0)?)?;
    let monitor = mount(
        &mut ui,
        vec![
            Plan {
                total: 50,
                step: Some(20),
                fail_at: None,
            },
            Plan {
                total: 500,
                step: Some(5),
                fail_at: None,
            },
            Plan {
                total: 500,
                step: Some(5),
                fail_at: None,
            },
            Plan {
                total: 70,
                step: Some(20),
                fail_at: None,
            },
            Plan {
                total: 100,
                step: Some(20),
                fail_at: Some(35),
            },
        ],
    )?;
    let mut d = Driver::new(&mut ui);
    d.settle()?;
    assert!(d.quiet_for(Duration::from_secs(1))?.is_idle());
    let start = d.find(&monitor.root, &["start"])?;
    for _ in 0..5 {
        d.click(&start)?;
    }
    assert_eq!(
        text(&d, &monitor.root, &["summary"])?,
        "Running: 5   Done: 0   Cancelled: 0   Failed: 0"
    );
    d.wait_until(Duration::from_secs(3), |_| {
        Ok(monitor.jobs().iter().all(|j| j.processed > 0))
    })?;
    let table = d.find(&monitor.root, &["jobs"])?;
    assert_ne!(
        text(&d, &table, &["1", "progress", "value", "percent"])?,
        "0%"
    );
    let cancel = d.find(&table, &["2", "action", "cancel"])?;
    let before_cancel = std::time::Instant::now();
    d.click(&cancel)?;
    d.wait_until(Duration::from_millis(250), |_| {
        Ok(monitor.jobs()[1].state == State::Cancelled)
    })?;
    assert!(before_cancel.elapsed() < Duration::from_millis(250));
    let stopped = monitor.jobs()[1].processed;
    d.wait_until(Duration::from_secs(3), |_| {
        let jobs = monitor.jobs();
        Ok(jobs[0].state == State::Done
            && jobs[3].state == State::Done
            && jobs[4].state == State::Failed)
    })?;
    assert_eq!(monitor.jobs()[1].processed, stopped);
    assert_eq!(text(&d, &table, &["2", "state", "value"])?, "cancelled");
    assert_eq!(
        text(&d, &table, &["1", "progress", "value", "percent"])?,
        "100%"
    );
    assert_eq!(
        text(&d, &table, &["5", "state", "value"])?,
        "failed: simulated error"
    );
    assert!(text(&d, &monitor.root, &["notification"])?.contains("Job 5 failed"));
    assert_eq!(
        text(&d, &monitor.root, &["summary"])?,
        "Running: 1   Done: 2   Cancelled: 1   Failed: 1"
    );

    // Every sortable header, in both directions, through actual pointer input.
    for (column, key) in ["name", "progress", "state", "elapsed"]
        .into_iter()
        .enumerate()
    {
        for descending in [false, true] {
            d.click_item(&table, column)?;
            let jobs = monitor.jobs();
            let mut expected = jobs.clone();
            expected.sort_by(|a, b| {
                let cmp = match key {
                    "name" => a.name().cmp(&b.name()),
                    "progress" => a.percent().total_cmp(&b.percent()),
                    "state" => a.state.text().cmp(b.state.text()),
                    "elapsed" => a.elapsed.cmp(&b.elapsed),
                    _ => unreachable!(),
                }
                .then(a.id.cmp(&b.id));
                if descending { cmp.reverse() } else { cmp }
            });
            let actual: Vec<String> = d
                .address_children(&table)?
                .into_iter()
                .map(|r| r.segment)
                .collect();
            assert_eq!(
                actual,
                expected
                    .iter()
                    .map(|j| j.id.to_string())
                    .collect::<Vec<_>>(),
                "{key}, descending={descending}"
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
    let capture = pending.try_reply()?.value?;
    std::fs::write("/work/jobs.png", capture.image.encode_png())?;
    assert_eq!(capture.image.width(), 1000);

    let cancel = d.find(&table, &["3", "action", "cancel"])?;
    d.click(&cancel)?;
    d.wait_until(Duration::from_millis(250), |_| {
        Ok(monitor.jobs()[2].state == State::Cancelled)
    })?;
    assert_eq!(
        text(&d, &monitor.root, &["summary"])?,
        "Running: 0   Done: 2   Cancelled: 2   Failed: 1"
    );
    d.clear_focus()?;
    assert!(d.quiet_for(Duration::from_secs(2))?.is_idle());
    #[cfg(debug_assertions)]
    assert!(
        d.runtime().validation_report().issues.is_empty(),
        "{:?}",
        d.runtime().validation_report().issues
    );
    Ok(())
}
