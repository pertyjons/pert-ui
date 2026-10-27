# Distinct diagnostics

```text
error[E0599]: no method named `y` found for array `[f32; 4]` in the current scope
   --> tests/monitor.rs:119:68
    |
119 |                 let top = c.details.box_model.border_box.map(|b| b.y()).unwrap_or(f32::MAX);
    |                                                                    ^ method not found in `[f32; 4]`
```

```text
warning: unused import: `pert_ui::prelude::*`
  --> tests/app.rs:10:5
   |
10 | use pert_ui::prelude::*;
   |     ^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: variable does not need to be mutable
   --> tests/monitor.rs:353:9
    |
353 |     let mut click = |h: &mut Harness| -> Result {
    |         ----^^^^^
    |         |
    |         help: remove this `mut`
    |
    = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default
```

```text
warning: this `if` statement can be collapsed
   --> src/lib.rs:134:9
    |
134 | /         if let Some(job) = self.jobs.borrow_mut().iter_mut().find(|job| job.id == id) {
135 | |             if job.state == JobState::Running {
136 | |                 job.cancel.store(true, Ordering::Release);
137 | |                 job.state = JobState::Cancelled;
...   |
141 | |         }
    | |_________^
    |
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.99.0/index.html#collapsible_if
    = note: `#[warn(clippy::collapsible_if)]` on by default
help: collapse nested if block
    |
134 ~         if let Some(job) = self.jobs.borrow_mut().iter_mut().find(|job| job.id == id)
135 ~             && job.state == JobState::Running {
136 |                 job.cancel.store(true, Ordering::Release);
...
139 |                 changed = true;
140 ~             }
    |
```
