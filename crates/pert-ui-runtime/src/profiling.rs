//! Optional UI-local elapsed-time profiling with bounded owned snapshots.
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    time::{Duration, Instant},
};

/// Measured UI work. Durations include preemption/waiting; they are not CPU usage
/// percentages or GPU execution times. Nested work is included in total time and
/// excluded from self time. Cache hits do not count as component callback work.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ProfileOperation {
    /// Pointer position delivery, including hit testing and pending layout.
    PointerMove,
    /// Pointer button press or release delivery.
    PointerButton,
    /// Pointer leave delivery.
    PointerLeave,
    /// Wheel/trackpad scrolling and pinch delivery.
    Scroll,
    /// Physical key delivery.
    KeyInput,
    /// Committed text delivery, without retaining its contents.
    TextInput,
    /// Application event closure dispatch.
    Application,
    /// Focus transfer, reveal or cancellation.
    Focus,
    /// Explicit visibility change.
    Visibility,
    /// Component/tree mounting.
    Mount,
    /// Component/tree removal.
    Unmount,
    /// Retained child reconciliation.
    Reconcile,
    /// Native viewport size or scale change admission.
    Viewport,
    /// Parameter edit admission.
    EditBegin,
    /// Parameter preview update.
    EditPreview,
    /// Parameter commit, including receipt replay or rejection.
    EditCommit,
    /// Explicit or lifecycle cancellation.
    EditCancel,
    /// Authoritative host parameter write.
    HostWrite,
    /// Effective-value overlay change.
    Effective,
    /// Parameter metadata replacement.
    Metadata,
    /// Parameter removal or host shutdown.
    ParameterRemoved,
    /// A scheduled update/layout flush.
    Flush,
    /// A layout pass that had pending work.
    Layout,
    /// Uncached size-limit or size measurement, including descendant work.
    Measure,
    /// Uncached local component arrangement, including descendant work.
    /// Translating or clipping an unchanged leaf reuses its local arrangement.
    Arrange,
    /// A component update callback.
    Update,
    /// An elapsed event-phase component timer callback.
    Timer,
    /// A component's drawing callbacks, including its foreground.
    Paint,
    /// Display-list composition, including changed drawing callbacks.
    Compose,
    /// Text cache lookup, shaping and coverage preparation.
    Text,
    /// Vector resource registration and command recording. Backend fallback
    /// flattening/rasterization is outside this UI-runtime profiler.
    Geometry,
}

/// Aggregate for an operation and runtime-local component incarnation.
/// Keys are diagnostic labels truncated to [`Self::MAX_KEY_CHARS`] characters, not target identities.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ProfileStats {
    /// Kind of measured work.
    pub operation: ProfileOperation,
    /// Component incarnation, or `None` for runtime-wide work.
    pub component: Option<u64>,
    /// Caller-supplied component key, truncated to [`Self::MAX_KEY_CHARS`]
    /// Unicode characters; empty for runtime-wide work.
    pub key: String,
    /// Number of completed scopes, including returned errors and unwinding scopes.
    pub calls: u64,
    /// Accumulated inclusive elapsed time; nested totals must not be added together.
    pub total: Duration,
    /// Accumulated time excluding directly nested measured scopes.
    pub self_time: Duration,
    /// Largest inclusive duration observed for this aggregate.
    pub maximum: Duration,
}
/// One recently completed timing scope; separate from semantic event history.
/// Completion does not imply success. Event history retains operation outcomes.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ProfileSample {
    /// Operation category.
    pub operation: ProfileOperation,
    /// Component incarnation, when applicable.
    pub component: Option<u64>,
    /// Inclusive elapsed duration.
    pub elapsed: Duration,
    /// Duration excluding measured descendants.
    pub self_time: Duration,
}
/// Owned profiling data captured on the UI thread; safe to pass to other threads.
/// Profiling is off by default, adds no polling, and never retains live UI handles.
/// Statistics retain at most [`Self::MAX_STATS`] operation/component pairs; recent
/// samples retain [`Self::MAX_RECENT`] scopes in completion order (inner scopes
/// precede outer scopes), and scopes nested deeper than [`Self::MAX_NESTING`] are
/// omitted. Aggregates survive hiding/removal without retargeting IDs. Clear
/// explicitly between runs.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(deny_unknown_fields)
)]
pub struct ProfileSnapshot {
    /// Whether new timing scopes are being recorded.
    pub enabled: bool,
    /// Aggregates sorted by descending self time, then operation and incarnation.
    pub stats: Vec<ProfileStats>,
    /// Up to [`Self::MAX_RECENT`] recent scopes in completion order.
    pub recent: Vec<ProfileSample>,
    /// Evicted recent samples; saturates instead of wrapping.
    pub dropped_samples: u64,
    /// Scopes omitted because the [`Self::MAX_STATS`] aggregate or
    /// [`Self::MAX_NESTING`] nesting limit was reached.
    pub dropped_scopes: u64,
}
impl ProfileSnapshot {
    /// The most operation/component aggregates retained. When full, a scope
    /// with a new identity is omitted and counted in
    /// [`Self::dropped_scopes`]; existing aggregates keep accumulating.
    pub const MAX_STATS: usize = 4096;
    /// The most recent completed scopes retained; recording one more evicts
    /// the oldest and counts it in [`Self::dropped_samples`].
    pub const MAX_RECENT: usize = 256;
    /// The deepest scope nesting measured; deeper scopes are omitted and
    /// counted in [`Self::dropped_scopes`].
    pub const MAX_NESTING: usize = 256;
}
impl ProfileStats {
    /// The most Unicode characters of a component key an aggregate keeps.
    pub const MAX_KEY_CHARS: usize = 128;
}
#[derive(Clone, Default)]
pub(crate) struct Profiler(Rc<RefCell<State>>);
#[derive(Default)]
struct State {
    enabled: bool,
    epoch: Rc<()>,
    stack: Vec<Duration>,
    stats: BTreeMap<(ProfileOperation, Option<u64>), ProfileStats>,
    recent: VecDeque<ProfileSample>,
    dropped_samples: u64,
    dropped_scopes: u64,
}
impl Profiler {
    pub(crate) fn enable(&self, enabled: bool) {
        let mut state = self.0.borrow_mut();
        if state.enabled != enabled {
            state.enabled = enabled;
            state.epoch = Rc::new(());
            state.stack.clear();
        }
    }
    pub(crate) fn clear(&self) {
        let enabled = self.0.borrow().enabled;
        *self.0.borrow_mut() = State {
            enabled,
            ..Default::default()
        };
    }
    pub(crate) fn snapshot(&self) -> ProfileSnapshot {
        let state = self.0.borrow();
        let mut stats: Vec<_> = state.stats.values().cloned().collect();
        stats.sort_by_key(|s| (std::cmp::Reverse(s.self_time), s.operation, s.component));
        ProfileSnapshot {
            enabled: state.enabled,
            stats,
            recent: state.recent.iter().cloned().collect(),
            dropped_samples: state.dropped_samples,
            dropped_scopes: state.dropped_scopes,
        }
    }
    pub(crate) fn span(
        &self,
        operation: ProfileOperation,
        component: Option<u64>,
        key: &str,
    ) -> Option<Span> {
        let mut state = self.0.borrow_mut();
        if !state.enabled {
            return None;
        }
        if state.stack.len() >= ProfileSnapshot::MAX_NESTING {
            state.dropped_scopes = state.dropped_scopes.saturating_add(1);
            return None;
        }
        state.stack.push(Duration::ZERO);
        Some(Span {
            profiler: self.clone(),
            epoch: state.epoch.clone(),
            start: Instant::now(),
            operation,
            component,
            key: key.chars().take(ProfileStats::MAX_KEY_CHARS).collect(),
        })
    }
}
pub(crate) struct Span {
    profiler: Profiler,
    epoch: Rc<()>,
    start: Instant,
    operation: ProfileOperation,
    component: Option<u64>,
    key: String,
}
impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let mut state = self.profiler.0.borrow_mut();
        // Clear/pause inside a callback invalidates every older in-flight scope.
        if !Rc::ptr_eq(&state.epoch, &self.epoch) {
            return;
        }
        let children = state.stack.pop().unwrap_or_default();
        let self_time = elapsed.saturating_sub(children);
        if let Some(parent) = state.stack.last_mut() {
            *parent = parent.saturating_add(elapsed);
        }
        let key = (self.operation, self.component);
        if !state.stats.contains_key(&key) && state.stats.len() >= ProfileSnapshot::MAX_STATS {
            state.dropped_scopes = state.dropped_scopes.saturating_add(1);
            return;
        }
        let stat = state.stats.entry(key).or_insert_with(|| ProfileStats {
            operation: self.operation,
            component: self.component,
            key: self.key.clone(),
            calls: 0,
            total: Duration::ZERO,
            self_time: Duration::ZERO,
            maximum: Duration::ZERO,
        });
        stat.calls = stat.calls.saturating_add(1);
        stat.total = stat.total.saturating_add(elapsed);
        stat.self_time = stat.self_time.saturating_add(self_time);
        stat.maximum = stat.maximum.max(elapsed);
        if state.recent.len() == ProfileSnapshot::MAX_RECENT {
            state.recent.pop_front();
            state.dropped_samples = state.dropped_samples.saturating_add(1);
        }
        state.recent.push_back(ProfileSample {
            operation: self.operation,
            component: self.component,
            elapsed,
            self_time,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nesting_pause_clear_and_unwinding_are_accounted_without_stale_scopes() {
        let profiler = Profiler::default();
        assert!(
            profiler
                .span(ProfileOperation::Update, Some(1), "off")
                .is_none()
        );
        profiler.enable(true);
        {
            let _parent = profiler.span(ProfileOperation::Flush, None, "");
            let _child = profiler.span(ProfileOperation::Update, Some(1), "child");
        }
        let snapshot = profiler.snapshot();
        let parent = snapshot
            .stats
            .iter()
            .find(|s| s.operation == ProfileOperation::Flush)
            .unwrap();
        let child = snapshot
            .stats
            .iter()
            .find(|s| s.operation == ProfileOperation::Update)
            .unwrap();
        assert_eq!(parent.total, parent.self_time + child.total);
        assert_eq!(child.total, child.self_time);
        let stale = profiler.span(ProfileOperation::Paint, Some(2), "stale");
        profiler.clear();
        drop(stale);
        assert!(profiler.snapshot().stats.is_empty());
        let stale = profiler.span(ProfileOperation::Paint, Some(2), "stale");
        profiler.enable(false);
        profiler.enable(true);
        drop(stale);
        assert!(profiler.snapshot().stats.is_empty());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _scope = profiler.span(ProfileOperation::Application, Some(3), "panic");
            panic!("test unwind");
        }));
        assert!(result.is_err());
        assert_eq!(profiler.snapshot().stats[0].calls, 1);
        assert!(profiler.0.borrow().stack.is_empty());
    }
    #[test]
    fn retained_samples_aggregates_and_labels_are_bounded() {
        let profiler = Profiler::default();
        profiler.enable(true);
        for id in 0..4100 {
            let _scope = profiler.span(ProfileOperation::Update, Some(id), &"å".repeat(200));
        }
        let snapshot = profiler.snapshot();
        assert_eq!(snapshot.stats.len(), 4096);
        assert_eq!(snapshot.recent.len(), 256);
        assert_eq!(snapshot.dropped_scopes, 4);
        assert_eq!(snapshot.dropped_samples, 4096 - 256);
        assert!(snapshot.stats.iter().all(|s| s.key.chars().count() == 128));
        let scopes: Vec<_> = (0..260)
            .map(|_| profiler.span(ProfileOperation::Flush, None, ""))
            .collect();
        assert_eq!(profiler.0.borrow().stack.len(), 256);
        // Guards in real instrumentation use lexical nesting and drop in reverse.
        for scope in scopes.into_iter().rev() {
            drop(scope);
        }
        assert!(profiler.0.borrow().stack.is_empty());
        assert!(profiler.snapshot().dropped_scopes >= 8);
    }
}
