
// ---------------------------------------------------------------------------
// Phase 1 probe adapter, appended by plans/rendering/phase1-probe/run.sh to a
// scratch copy of `live_mixer.rs` whose `main` was removed. It lives in the
// example's module so it can reach the private `build`, `Command` and
// `level`; the repository's example does not change.

/// The mixer with its engine's clock stopped, stepped one tick at a time.
pub(crate) struct ProbeMixer {
    mixer: Rc<Mixer>,
    /// The mixer's root component.
    pub(crate) root: WidgetId,
}

/// Builds the mixer and stops the engine's own clock with `Run(false)`, as
/// the example's headless session does.
pub(crate) fn probe_build(ui: &mut Runtime) -> Result<ProbeMixer, Box<dyn std::error::Error>> {
    let (mixer, root) = build(ui)?;
    mixer.engine.send(Command::Run(false))?;
    Ok(ProbeMixer { mixer, root })
}

impl ProbeMixer {
    /// Asks the engine to publish one level per channel.
    pub(crate) fn tick(&self) -> Result<(), SinkError> {
        self.mixer.engine.send(Command::Tick)
    }

    /// The level every meter shows, read through inspection.
    pub(crate) fn levels(ui: &Runtime) -> Vec<f64> {
        (0..CHANNELS).map(|channel| level(ui, channel)).collect()
    }
}
