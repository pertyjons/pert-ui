//! `ReadSignal::combine` over a tuple of bindings: one evaluation per batch of
//! writes, equal results, retention and release, runtime and phase checks, and
//! the input forms, through the public facade.
use pert_ui::{component::*, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Counts a function's calls.
fn counter() -> (Rc<Cell<usize>>, Rc<Cell<usize>>) {
    let count = Rc::new(Cell::new(0));
    (count.clone(), count)
}

/// A component that reads one binding in its update and counts its updates,
/// with no derived binding between the binding and the component.
struct Observer<T> {
    binding: ReadSignal<T>,
    updates: Rc<Cell<usize>>,
}

impl<T: Clone + 'static> Component for Observer<T> {
    fn update(&mut self, cx: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        cx.read(&self.binding)?;
        self.updates.set(self.updates.get() + 1);
        Ok(())
    }
}

fn observe<T: Clone + 'static>(
    ui: &mut Runtime,
    binding: &ReadSignal<T>,
) -> Result<Rc<Cell<usize>>> {
    let updates = Rc::new(Cell::new(0));
    ui.mount_tree(Element::new(
        "observer",
        Observer {
            binding: binding.clone(),
            updates: updates.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    Ok(updates)
}

#[test]
fn five_inputs_evaluate_once_per_batch_and_an_equal_result_updates_no_component() -> Result {
    let mut ui = Runtime::new();
    let prefix = ui.signal("img".to_owned());
    let format = ui.signal(1_u8);
    let quality = ui.signal(80_u8);
    let width = ui.signal(640_u32);
    let open = ui.signal(false);
    let applied = ("img".to_owned(), 1_u8, 80_u8, 640_u32, false);
    let (calls, count) = counter();
    let dirty = ReadSignal::combine((&prefix, &format, &quality, &width, &open), move |values| {
        count.set(count.get() + 1);
        *values != applied
    })?;
    let updates = observe(&mut ui, &dirty)?;
    // (a) Evaluated once at creation; the flush that mounted the observer
    // found nothing stale.
    assert_eq!(calls.get(), 1, "one evaluation at creation");
    let observed = updates.get();

    // (b) Writing every input runs nothing; the flush evaluates once.
    prefix.set("png".to_owned())?;
    format.set(2)?;
    quality.set(90)?;
    width.set(1280)?;
    open.set(true)?;
    assert_eq!(calls.get(), 1, "writes run no evaluation");
    ui.flush(Duration::ZERO)?;
    assert_eq!(calls.get(), 2, "five writes, one evaluation");
    assert_eq!(updates.get(), observed + 1, "the changed result updates");
    assert!(dirty.get()?);

    // (c) The last position alone written back: evaluated, result equal.
    open.set(false)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(calls.get(), 3, "the last input is observed");
    assert_eq!(
        updates.get(),
        observed + 1,
        "an equal result updates no component"
    );

    // (d) The other four written back: one evaluation, a changed result.
    prefix.set("img".to_owned())?;
    format.set(1)?;
    quality.set(80)?;
    width.set(640)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(calls.get(), 4, "four writes, one evaluation");
    assert_eq!(updates.get(), observed + 2);
    assert!(!dirty.get()?);
    Ok(())
}

#[test]
fn inputs_and_the_function_are_retained_and_released_with_the_last_result() -> Result {
    let mut ui = Runtime::new();
    let a = ui.signal(1_u32);
    let b = ui.signal(3_u32);
    let input_token = Rc::new(());
    let input_alive: Weak<()> = Rc::downgrade(&input_token);
    let held = ui.signal(input_token);
    let mapper_token = Rc::new(());
    let mapper_alive: Weak<()> = Rc::downgrade(&mapper_token);
    // The derived input and the held signal are passed by value: the test
    // keeps no strong handle to either.
    let sum = ReadSignal::combine((a.map(|v| v * 2)?, held, &b), move |(doubled, _, b)| {
        let _ = &mapper_token;
        doubled + b
    })?;
    a.set(5)?;
    assert_eq!(sum.get()?, 13);
    let copy = sum.clone();
    drop(sum);
    assert!(input_alive.upgrade().is_some(), "the input is retained");
    assert!(mapper_alive.upgrade().is_some(), "the function is retained");
    a.set(6)?;
    assert_eq!(
        copy.get()?,
        15,
        "a derived input by value stays current after the original drops"
    );
    drop(copy);
    assert!(
        input_alive.upgrade().is_none(),
        "the last result releases its inputs"
    );
    assert!(
        mapper_alive.upgrade().is_none(),
        "the last result releases its function"
    );
    a.set(7)?;
    b.set(4)?;
    ui.flush(Duration::ZERO)?;
    Ok(())
}

#[test]
fn a_binding_of_another_runtime_is_refused_before_anything_runs() -> Result {
    let ui = Runtime::new();
    let other = Runtime::new();
    let (a, b) = (ui.signal(1_u32), ui.signal(2_u32));
    let foreign = other.signal(3_u32);
    let (calls, count) = counter();
    let last = count.clone();
    let refused = ReadSignal::combine((&a, &b, &foreign), move |(a, b, c)| {
        last.set(last.get() + 1);
        a + b + c
    });
    assert!(matches!(refused, Err(RuntimeError::ForeignRuntime)));
    let refused = ReadSignal::combine((&foreign, &a, &b), move |(a, b, c)| {
        count.set(count.get() + 1);
        a + b + c
    });
    assert!(matches!(refused, Err(RuntimeError::ForeignRuntime)));
    assert_eq!(calls.get(), 0, "the function never ran");

    let (doubled_calls, doubled_count) = counter();
    let doubled = a.map(move |v| {
        doubled_count.set(doubled_count.get() + 1);
        v * 2
    })?;
    a.set(9)?;
    let refused = ReadSignal::combine((&doubled, &foreign), |(d, f)| d + f);
    assert!(matches!(refused, Err(RuntimeError::ForeignRuntime)));
    assert_eq!(
        doubled_calls.get(),
        1,
        "the refusal read no input, so the dirty input stayed stale"
    );
    assert_eq!(doubled.get()?, 18);
    Ok(())
}

#[test]
fn a_write_inside_the_function_is_refused_at_every_evaluation() -> Result {
    let ui = Runtime::new();
    let (a, b) = (ui.signal(1_u32), ui.signal(2_u32));
    let writer = a.clone();
    let (calls, count) = counter();
    let guarded = ReadSignal::combine((&a, &b), move |_| {
        count.set(count.get() + 1);
        writer.set(9)
    })?;
    assert_eq!(guarded.get()?, Err(RuntimeError::WriteDuringUpdate));
    assert_eq!(a.get()?, 1, "the initial evaluation wrote nothing");
    b.set(5)?;
    assert_eq!(guarded.get()?, Err(RuntimeError::WriteDuringUpdate));
    assert_eq!(calls.get(), 2, "the later evaluation ran");
    assert_eq!(a.get()?, 1, "the later evaluation wrote nothing");
    Ok(())
}

type Outcomes = Rc<RefCell<Vec<(&'static str, std::result::Result<(), RuntimeError>)>>>;

/// Derives from two signals in its update and in its paint, recording each
/// call's outcome.
struct Deriver {
    a: Signal<u32>,
    b: Signal<u32>,
    outcomes: Outcomes,
}

impl Deriver {
    fn derive(&self, phase: &'static str) {
        let outcome = ReadSignal::combine((&self.a, &self.b), |(a, b)| a + b).map(|_| ());
        self.outcomes.borrow_mut().push((phase, outcome));
    }
}

impl Component for Deriver {
    fn update(&mut self, _: &mut UpdateContext) -> std::result::Result<(), RuntimeError> {
        self.derive("update");
        Ok(())
    }

    fn measure(
        &self,
        _: &MeasureContext<'_>,
        _: Constraints,
    ) -> std::result::Result<Size, RuntimeError> {
        Ok(Size::new(32.0, 32.0)?)
    }

    fn paint(&self, _: &mut Painter) -> std::result::Result<(), DrawError> {
        self.derive("paint");
        Ok(())
    }
}

#[test]
fn an_update_may_combine_and_a_paint_may_not() -> Result {
    let mut ui = Runtime::new();
    ui.set_viewport(Viewport::new(200.0, 100.0, 1.0)?)?;
    let outcomes = Outcomes::default();
    ui.mount_tree(Element::new(
        "deriver",
        Deriver {
            a: ui.signal(1),
            b: ui.signal(2),
            outcomes: outcomes.clone(),
        },
    ))?;
    ui.flush(Duration::ZERO)?;
    ui.paint()?;
    let outcomes = outcomes.borrow();
    let of = |phase| {
        outcomes
            .iter()
            .filter(|(at, _)| *at == phase)
            .map(|(_, outcome)| outcome.clone())
            .collect::<Vec<_>>()
    };
    let (updates, paints) = (of("update"), of("paint"));
    assert!(!updates.is_empty() && !paints.is_empty());
    assert!(
        updates.iter().all(std::result::Result::is_ok),
        "{updates:?}"
    );
    assert!(
        paints
            .iter()
            .all(|outcome| *outcome == Err(RuntimeError::WriteDuringPaint)),
        "{paints:?}"
    );
    Ok(())
}

#[test]
fn a_foreign_runtime_is_reported_before_a_closed_one() -> Result {
    let mut ui = Runtime::new();
    let (a, b) = (ui.signal(1_u32), ui.signal(2_u32));
    ui.shutdown();
    let open = Runtime::new();
    let foreign = open.signal(3_u32);
    assert!(matches!(
        ReadSignal::combine((&a, &foreign), |(a, f)| a + f),
        Err(RuntimeError::ForeignRuntime)
    ));
    assert!(matches!(
        ReadSignal::combine((&a, &b), |(a, b)| a + b),
        Err(RuntimeError::Closed)
    ));
    Ok(())
}

#[test]
fn an_equal_result_stops_components_but_not_a_derived_binding() -> Result {
    let mut ui = Runtime::new();
    let (a, b) = (ui.signal(1_u32), ui.signal(2_u32));
    let flag = ReadSignal::combine((&a, &b), |(a, b)| a + b > 10)?;
    let (mapped_calls, count) = counter();
    let mapped = flag.map(move |flag| {
        count.set(count.get() + 1);
        !flag
    })?;
    let updates = observe(&mut ui, &flag)?;
    let (before, observed) = (mapped_calls.get(), updates.get());
    a.set(2)?;
    b.set(3)?;
    ui.flush(Duration::ZERO)?;
    assert!(!flag.get()?);
    assert_eq!(
        mapped_calls.get(),
        before + 1,
        "a binding derived from the result still evaluates"
    );
    assert_eq!(
        updates.get(),
        observed,
        "a component reading the result is not updated"
    );
    assert!(mapped.get()?);
    Ok(())
}

#[test]
fn every_input_form_and_a_repeated_input_are_followed() -> Result {
    let mut ui = Runtime::new();
    let count = ui.signal(2_u32);
    let read = ui.signal(10_u32).read_only();
    let base = ui.signal(100_u32);
    let owned = ui.signal(1000_u32);
    let total = ReadSignal::combine(
        (&count, &read, base.map(|v| v + 1)?, owned.clone()),
        |(count, read, base, owned)| count + read + base + owned,
    )?;
    assert_eq!(total.get()?, 2 + 10 + 101 + 1000);
    count.set(3)?;
    assert_eq!(total.get()?, 3 + 10 + 101 + 1000);
    base.set(200)?;
    assert_eq!(total.get()?, 3 + 10 + 201 + 1000);
    owned.set(2000)?;
    assert_eq!(total.get()?, 3 + 10 + 201 + 2000);

    let twice = ReadSignal::combine((&count, &count), |(a, b)| a + b)?;
    count.set(4)?;
    assert_eq!(twice.get()?, 8);
    drop(twice);
    count.set(5)?;
    ui.flush(Duration::ZERO)?;
    assert_eq!(total.get()?, 5 + 10 + 201 + 2000);
    Ok(())
}

/// An application derives from several bindings with the prelude alone.
mod prelude_only {
    use pert_ui::prelude::*;

    #[test]
    fn the_prelude_alone_combines() -> Result<(), RuntimeError> {
        let ui = Runtime::new();
        let (a, b) = (ui.signal(1_u32), ui.signal(2_u32));
        let sum = ReadSignal::combine((&a, &b), |(a, b)| a + b)?;
        b.set(5)?;
        assert_eq!(sum.get()?, 6);
        Ok(())
    }
}
