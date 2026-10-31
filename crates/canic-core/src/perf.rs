//! Cross-cutting performance instrumentation.
//!
//! This module provides instruction-count measurement primitives used
//! across endpoints, ops, timers, and background tasks.
//!
//! It is intentionally crate-level infrastructure, not part of the
//! domain layering.
//! Instrumentation modules are layer-neutral and may be used anywhere.

use canic_contracts::ids::{EndpointCall, EndpointCallKind};
use std::{
    borrow::Borrow,
    cell::RefCell,
    cmp::Ordering,
    collections::BTreeMap,
    future::Future,
    pin::{Pin, pin},
    rc::Rc,
    task::{Context, Poll},
};

// Each checkpoint produces an instruction row and a sample-count row.
const MAX_CHECKPOINT_KEYS: usize = crate::model::public_metrics::MAX_PUBLIC_METRICS / 2;
const CHECKPOINT_NAME_OVERHEAD: usize = "perf.checkpoint.".len() + ".".len() + ".calls".len();

/// Heap-owned counters and the number of distinct admitted checkpoint identities.

struct PerfTable {
    entries: BTreeMap<PerfKey, ic_metrics::MeasurementAggregate>,
    checkpoint_count: usize,
}

thread_local! {
    // Installed only during a synchronous invocation or one future poll.
    static ACTIVE_CONTEXT: RefCell<Option<Rc<RefCell<PerfContext>>>> = const { RefCell::new(None) };

    /// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
    static PERF_TABLE: RefCell<PerfTable> = const { RefCell::new(PerfTable {
        entries: BTreeMap::new(),
        checkpoint_count: 0,
    }) };


}

/// Returns the **call-context instruction counter** for the current execution.
///
/// This value is obtained from `ic0.performance_counter(1)` and represents the
/// total number of WebAssembly instructions executed by *this canister* within
/// the **current call context**.
///
/// Key properties:
/// - Monotonically increasing for the duration of the call context
/// - Accumulates across `await` points and resumptions
/// - Resets only when a new call context begins
/// - Counts *only* instructions executed by this canister (not other canisters)
///
/// This counter is suitable for:
/// - Endpoint-level performance accounting
/// - Async workflows and timers
/// - Regression detection and coarse-grained profiling
///
/// It is **not** a measure of cycle cost. Expensive inter-canister operations
/// (e.g., canister creation) may have low instruction counts here but high cycle
/// charges elsewhere.
///
/// For fine-grained, single-slice profiling (e.g., hot loops), use
/// `ic0.performance_counter(0)` instead.
#[must_use]
#[cfg_attr(not(target_arch = "wasm32"), expect(clippy::missing_const_for_fn))]
pub fn perf_counter() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        ic_platform::cdk::api::call_context_instruction_counter()
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

///
/// PerfKey
/// Splits perf counters by transport surface so metrics rows remain explicit.
///

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PerfKey {
    /// Samples whose checkpoint identity could not be retained within the bounds.
    CheckpointOverflow,
    Endpoint {
        kind: EndpointCallKind,
        name: String,
    },
    Checkpoint {
        scope: String,
        label: String,
    },
}

// The borrowed view preserves the owned key ordering, including transport kind.
// A trait-object Borrow view avoids allocating a compound key on every hit;
// it leaves the public owned key and the single authoritative map unchanged.
impl<'view> Borrow<dyn PerfKeyView + 'view> for PerfKey {
    fn borrow(&self) -> &(dyn PerfKeyView + 'view) {
        self
    }
}

impl PerfKeyView for PerfKey {
    fn key_ref(&self) -> PerfKeyRef<'_> {
        match self {
            Self::CheckpointOverflow => PerfKeyRef::CheckpointOverflow,
            Self::Endpoint { kind, name } => PerfKeyRef::Endpoint { kind: *kind, name },
            Self::Checkpoint { scope, label } => PerfKeyRef::Checkpoint { scope, label },
        }
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum PerfKeyRef<'key> {
    CheckpointOverflow,
    Endpoint {
        kind: EndpointCallKind,
        name: &'key str,
    },
    Checkpoint {
        scope: &'key str,
        label: &'key str,
    },
}

impl PerfKeyRef<'_> {
    fn into_owned(self) -> PerfKey {
        match self {
            Self::CheckpointOverflow => PerfKey::CheckpointOverflow,
            Self::Endpoint { kind, name } => PerfKey::Endpoint {
                kind,
                name: name.to_owned(),
            },
            Self::Checkpoint { scope, label } => PerfKey::Checkpoint {
                scope: scope.to_owned(),
                label: label.to_owned(),
            },
        }
    }
}

impl PerfKeyView for PerfKeyRef<'_> {
    fn key_ref(&self) -> PerfKeyRef<'_> {
        *self
    }
}

trait PerfKeyView {
    fn key_ref(&self) -> PerfKeyRef<'_>;
}

impl Eq for dyn PerfKeyView + '_ {}

impl Ord for dyn PerfKeyView + '_ {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key_ref().cmp(&other.key_ref())
    }
}

impl PartialEq for dyn PerfKeyView + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.key_ref() == other.key_ref()
    }
}

impl PartialOrd for dyn PerfKeyView + '_ {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

///
/// PerfFrame
/// Tracks an active endpoint scope and accumulated child instructions.
///

struct PerfFrame {
    start: u64,
    child_instructions: u64,
}

///
/// PerfEntry
/// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
///

#[derive(Clone)]
pub struct PerfEntry {
    pub key: PerfKey,
    /// Full source state; reports preserve its width and actual-overflow flags.
    pub aggregate: ic_metrics::MeasurementAggregate,
}

/// Record a counter under the provided key.
pub fn record(key: PerfKey, delta: u64) {
    record_borrowed(key.key_ref(), delta);
}

pub fn record_endpoint_call(call: EndpointCall, delta_instructions: u64) {
    record_borrowed(
        PerfKeyRef::Endpoint {
            kind: call.kind,
            name: call.endpoint.name,
        },
        delta_instructions,
    );
}

/// Record a bounded checkpoint identity, accounting for excess samples as overflow.
pub fn record_checkpoint(scope: &str, label: &str, delta_instructions: u64) {
    record_borrowed(PerfKeyRef::Checkpoint { scope, label }, delta_instructions);
}

fn record_borrowed(key: PerfKeyRef<'_>, delta: u64) {
    PERF_TABLE.with_borrow_mut(|table| {
        let mut key = key;
        if let PerfKeyRef::Checkpoint { scope, label } = key
            && !checkpoint_name_fits(scope, label)
        {
            key = PerfKeyRef::CheckpointOverflow;
        }
        if let Some(slot) = table.entries.get_mut(&key as &dyn PerfKeyView) {
            slot.record(delta);
            return;
        }
        if matches!(key, PerfKeyRef::Checkpoint { .. }) {
            if table.checkpoint_count < MAX_CHECKPOINT_KEYS {
                table.checkpoint_count += 1;
            } else if table
                .entries
                .last_key_value()
                .is_some_and(|(last, _)| key < last.key_ref())
            {
                // Keep the same lexicographic prefix regardless of observation order.
                let (_, evicted) = table
                    .entries
                    .pop_last()
                    .expect("checkpoint capacity has a greatest key");
                table
                    .entries
                    .entry(PerfKey::CheckpointOverflow)
                    .or_default()
                    .merge(evicted);
            } else {
                key = PerfKeyRef::CheckpointOverflow;
            }
        }
        table
            .entries
            .entry(key.into_owned())
            .or_default()
            .record(delta);
    });
}

fn checkpoint_name_fits(scope: &str, label: &str) -> bool {
    scope
        .len()
        .checked_add(label.len())
        .and_then(|bytes| bytes.checked_add(CHECKPOINT_NAME_OVERHEAD))
        .is_some_and(|bytes| bytes <= crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES)
}

/// Invocation-owned accounting state; no frames or checkpoint bases cross futures.
struct PerfContext {
    frames: Vec<PerfFrame>,
    last_checkpoint: u64,
}

impl PerfContext {
    fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            frames: Vec::new(),
            last_checkpoint: perf_counter(),
        }))
    }
}

/// Restore the enclosing poll's context even when nested work unwinds.
struct ContextActivation {
    previous: Option<Rc<RefCell<PerfContext>>>,
}

impl ContextActivation {
    fn enter(context: Rc<RefCell<PerfContext>>) -> Self {
        let previous = ACTIVE_CONTEXT.with_borrow_mut(|active| active.replace(context));
        Self { previous }
    }
}

impl Drop for ContextActivation {
    #[inline(never)]
    fn drop(&mut self) {
        ACTIVE_CONTEXT.with_borrow_mut(|active| *active = self.previous.take());
    }
}

/// A frame remains owned by its invocation, including cancellation and panic cleanup.
struct EndpointScope {
    context: Rc<RefCell<PerfContext>>,
    depth: usize,
}

impl EndpointScope {
    fn start_at(context: Rc<RefCell<PerfContext>>, start: u64) -> Self {
        let depth = {
            let mut state = context.borrow_mut();
            let depth = state.frames.len();
            state.frames.push(PerfFrame {
                start,
                child_instructions: 0,
            });
            depth
        };
        Self { context, depth }
    }

    fn finish_at(self, call: EndpointCall, end: u64) {
        let exclusive = {
            let mut state = self.context.borrow_mut();
            let frame = state.frames.pop().expect("owned endpoint frame");
            let total = end.saturating_sub(frame.start);
            if let Some(parent) = state.frames.last_mut() {
                parent.child_instructions = parent.child_instructions.saturating_add(total);
            }
            total.saturating_sub(frame.child_instructions)
        };
        record_endpoint_call(call, exclusive);
    }
}

impl Drop for EndpointScope {
    fn drop(&mut self) {
        self.context.borrow_mut().frames.truncate(self.depth);
    }
}

/// Run a background future with its own checkpoint base across awaits.
///
/// The context is installed only during each poll; spawned futures need their own scope.
/// The future must remain within one IC call context; migratory tasks need a new
/// scope for each method context rather than retaining a counter from another method.
#[expect(
    clippy::future_not_send,
    reason = "IC futures and invocation contexts stay on one canister thread"
)]
pub async fn with_async_context<F: Future>(future: F) -> F::Output {
    PerfFuture {
        future: pin!(future),
        measurement: AsyncMeasurement::new(None),
    }
    .await
}

/// Shared accounting across concrete endpoint and background future types.
struct AsyncMeasurement {
    context: Rc<RefCell<PerfContext>>,
    call: Option<EndpointCall>,
    scope: Option<EndpointScope>,
}

impl AsyncMeasurement {
    // Keep frame management out of each handler's monomorphized poll body.
    #[inline(never)]
    fn new(call: Option<EndpointCall>) -> Self {
        Self {
            context: PerfContext::new(),
            call,
            scope: None,
        }
    }

    #[inline(never)]
    fn enter(&mut self) -> ContextActivation {
        let activation = ContextActivation::enter(Rc::clone(&self.context));
        if self.call.is_some() && self.scope.is_none() {
            self.scope = Some(EndpointScope::start_at(
                Rc::clone(&self.context),
                perf_counter(),
            ));
        }
        activation
    }

    #[inline(never)]
    fn finish(&mut self) {
        if let Some(scope) = self.scope.take() {
            scope.finish_at(self.call.expect("measured endpoint call"), perf_counter());
        }
    }
}

/// The handler is pinned by its async owner; only polling depends on its type.
struct PerfFuture<F> {
    future: F,
    measurement: AsyncMeasurement,
}

impl<F: Future + Unpin> Future for PerfFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let _activation = this.measurement.enter();
        let result = Pin::new(&mut this.future).poll(task);
        if result.is_ready() {
            this.measurement.finish();
        }
        result
    }
}

pub(crate) fn measure_endpoint<T>(call: EndpointCall, invoke: impl FnOnce() -> T) -> T {
    let context = ACTIVE_CONTEXT
        .with_borrow(Clone::clone)
        .unwrap_or_else(PerfContext::new);
    let _activation = ContextActivation::enter(Rc::clone(&context));
    let scope = EndpointScope::start_at(context, perf_counter());
    let result = invoke();
    scope.finish_at(call, perf_counter());
    result
}

#[expect(
    clippy::future_not_send,
    reason = "IC endpoint futures retain single-threaded invocation state"
)]
#[expect(
    clippy::redundant_async_block,
    reason = "the owned await drops the handler inside its active measurement scope"
)]
pub(crate) async fn measure_endpoint_async<F: Future>(call: EndpointCall, future: F) -> F::Output {
    PerfFuture {
        // Own the await so the handler is dropped inside the active poll,
        // before its final instruction count is recorded.
        future: pin!(async { future.await }),
        measurement: AsyncMeasurement::new(Some(call)),
    }
    .await
}

/// A checkpoint measured against the preceding observation in this invocation.
pub struct CheckpointSample {
    pub instructions: u64,
    pub call_context_instructions: u64,
}

/// Record a checkpoint only when an invocation owns its baseline.
/// Unscoped calls have no interval evidence and produce no sample.
#[must_use]
pub fn checkpoint(scope: &str, label: &str) -> Option<CheckpointSample> {
    checkpoint_at(scope, label, perf_counter())
}

fn checkpoint_at(scope: &str, label: &str, now: u64) -> Option<CheckpointSample> {
    let instructions = ACTIVE_CONTEXT.with_borrow(|active| {
        let mut context = active.as_ref()?.borrow_mut();
        let instructions = now.saturating_sub(context.last_checkpoint);
        context.last_checkpoint = now;
        Some(instructions)
    })?;
    record_checkpoint(scope, label, instructions);
    Some(CheckpointSample {
        instructions,
        call_context_instructions: now,
    })
}

/// Snapshot all recorded perf counters, sorted by key.
/// Entries are sorted by (kind, label).
#[must_use]
pub fn entries() -> Vec<PerfEntry> {
    PERF_TABLE.with(|table| {
        let table = table.borrow();

        table
            .entries
            .iter()
            .map(|(key, slot)| PerfEntry {
                key: key.clone(),
                aggregate: *slot,
            })
            .collect()
    })
}

/// Read admission-loss accounting without narrowing the instruction total.
#[cfg(test)]
pub(crate) fn checkpoint_overflow_entry() -> Option<PerfEntry> {
    PERF_TABLE.with_borrow(|table| {
        table
            .entries
            .get(&PerfKey::CheckpointOverflow)
            .map(|slot| PerfEntry {
                key: PerfKey::CheckpointOverflow,
                aggregate: *slot,
            })
    })
}

/// Read a key-ordered prefix without cloning arbitrarily large checkpoint labels.
pub(crate) fn bounded_entries(limit: usize) -> Result<Vec<PerfEntry>, crate::InternalError> {
    PERF_TABLE.with_borrow(|table| {
        table
            .entries
            .iter()
            .take(limit)
            .map(|(key, slot)| {
                let valid = match key {
                    PerfKey::CheckpointOverflow => true,
                    PerfKey::Endpoint { name, .. } => {
                        name.len() <= crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES
                    }
                    PerfKey::Checkpoint { scope, label } => {
                        scope.len() <= crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES
                            && label.len()
                                <= crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES
                    }
                };
                if !valid {
                    return Err(crate::InternalError::invalid_input());
                }
                Ok(PerfEntry {
                    key: key.clone(),
                    aggregate: *slot,
                })
            })
            .collect()
    })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
pub fn reset() {
    PERF_TABLE.with_borrow_mut(|table| {
        table.entries.clear();
        table.checkpoint_count = 0;
    });
    ACTIVE_CONTEXT.with_borrow_mut(|active| *active = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, future::poll_fn};

    /// Observes the context when an owned, completed future is destroyed.
    struct DropObservedFuture {
        context: Option<Rc<RefCell<PerfContext>>>,
        dropped_in_scope: Rc<Cell<bool>>,
    }

    impl Future for DropObservedFuture {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> {
            self.context = ACTIVE_CONTEXT.with_borrow(Clone::clone);
            Poll::Ready(())
        }
    }

    impl Drop for DropObservedFuture {
        fn drop(&mut self) {
            let in_scope = ACTIVE_CONTEXT.with_borrow(|active| {
                active
                    .as_ref()
                    .zip(self.context.as_ref())
                    .is_some_and(|(active, owned)| {
                        Rc::ptr_eq(active, owned) && owned.as_ref().borrow().frames.len() == 1
                    })
            });
            self.dropped_in_scope.set(in_scope);
        }
    }

    #[test]
    fn completed_endpoint_future_drops_before_accounting_finishes() {
        reset();
        let dropped_in_scope = Rc::new(Cell::new(false));
        let mut future = Box::pin(measure_endpoint_async(
            call("drop_work", EndpointCallKind::Update),
            DropObservedFuture {
                context: None,
                dropped_in_scope: Rc::clone(&dropped_in_scope),
            },
        ));
        let mut task = Context::from_waker(futures::task::noop_waker_ref());
        assert!(future.as_mut().poll(&mut task).is_ready());
        assert!(dropped_in_scope.get());
        assert_eq!(
            entry_for(EndpointCallKind::Update, "drop_work")
                .aggregate
                .samples(),
            1
        );
        assert!(ACTIVE_CONTEXT.with_borrow(Option::is_none));
    }

    fn call(name: &'static str, kind: EndpointCallKind) -> EndpointCall {
        EndpointCall {
            endpoint: crate::ids::EndpointId::new(name),
            kind,
        }
    }

    fn entry_for(kind: EndpointCallKind, label: &str) -> PerfEntry {
        entries()
            .into_iter()
            .find(|entry| {
                matches!(
                    &entry.key,
                    PerfKey::Endpoint {
                        kind: entry_kind,
                        name
                    } if *entry_kind == kind && name == label
                )
            })
            .expect("expected perf entry to exist")
    }

    fn checkpoint_entry_for(scope: &str, label: &str) -> PerfEntry {
        entries()
            .into_iter()
            .find(|entry| {
                matches!(
                    &entry.key,
                    PerfKey::Checkpoint {
                        scope: entry_scope,
                        label: entry_label,
                    } if entry_scope == scope && entry_label == label
                )
            })
            .expect("expected checkpoint perf entry to exist")
    }

    #[test]
    fn checkpoint_names_admit_the_complete_public_identity_in_bytes() {
        reset();
        let budget =
            crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES - CHECKPOINT_NAME_OVERHEAD;
        let label = "x".repeat(budget - 1);
        record_checkpoint("s", &label, 0);
        record_checkpoint("s", &"é".repeat((budget - 1) / 2), 5);
        record_checkpoint("s", &format!("{label}x"), 7);
        record_checkpoint(&"m".repeat(129), "short", 11);
        record(
            PerfKey::Checkpoint {
                scope: "s".into(),
                label: "é".repeat(budget.div_ceil(2)),
            },
            13,
        );
        let valid = checkpoint_entry_for("s", &label);
        assert_eq!(valid.aggregate.samples(), 1);
        assert_eq!(valid.aggregate.total(), 0);
        let rows = bounded_entries(MAX_CHECKPOINT_KEYS + 1).unwrap();
        assert_eq!(rows.len(), 3);
        let overflow = &rows[0];
        assert!(matches!(overflow.key, PerfKey::CheckpointOverflow));
        assert_eq!(overflow.aggregate.samples(), 3);
        assert_eq!(overflow.aggregate.total(), 31);
        for row in rows {
            if let PerfKey::Checkpoint { scope, label } = row.key {
                assert!(
                    format!("perf.checkpoint.{scope}.{label}.calls").len()
                        <= crate::model::public_metrics::MAX_PUBLIC_METRIC_TEXT_BYTES
                );
            }
        }
    }

    #[test]
    fn checkpoint_capacity_preserves_key_order_and_all_observations() {
        let count = MAX_CHECKPOINT_KEYS * 4;
        let snapshot = |reverse| {
            reset();
            for position in 0..count {
                let index = if reverse {
                    count - position - 1
                } else {
                    position
                };
                let label = format!("sample_{index:04}");
                let delta = u64::try_from(index).unwrap();
                if index % 2 == 0 {
                    record(
                        PerfKey::Checkpoint {
                            scope: "bounded".into(),
                            label,
                        },
                        delta,
                    );
                } else {
                    record_checkpoint("bounded", &label, delta);
                }
            }
            record_checkpoint("bounded", "sample_0000", 25);
            record_endpoint_call(call("unrelated", EndpointCallKind::Query), 3);
            assert_eq!(
                checkpoint_entry_for("bounded", "sample_0000")
                    .aggregate
                    .samples(),
                2
            );
            assert_eq!(
                entry_for(EndpointCallKind::Query, "unrelated")
                    .aggregate
                    .samples(),
                1
            );
            PERF_TABLE.with_borrow(|table| {
                assert_eq!(table.checkpoint_count, MAX_CHECKPOINT_KEYS);
                assert_eq!(table.entries.len(), MAX_CHECKPOINT_KEYS + 2);
            });
            entries()
                .into_iter()
                .map(|row| (row.key, row.aggregate.samples(), row.aggregate.total()))
                .collect::<Vec<_>>()
        };
        let forward = snapshot(false);
        let reverse = snapshot(true);
        assert!(forward == reverse);
        assert!(matches!(forward[0].0, PerfKey::CheckpointOverflow));
        assert_eq!(
            forward[0].1,
            u64::try_from(count - MAX_CHECKPOINT_KEYS).unwrap()
        );
        assert_eq!(
            forward.iter().map(|row| row.1).sum::<u64>(),
            u64::try_from(count + 2).unwrap()
        );
        assert_eq!(
            forward.iter().map(|row| row.2).sum::<u128>(),
            (0..count).map(|value| value as u128).sum::<u128>() + 28
        );
        assert!(matches!(
            bounded_entries(1).unwrap()[0].key,
            PerfKey::CheckpointOverflow
        ));
    }

    #[test]
    fn checkpoint_eviction_moves_prior_samples_to_overflow_once() {
        reset();
        for index in 0..MAX_CHECKPOINT_KEYS {
            record_checkpoint("z", &format!("sample_{index:04}"), 4);
        }
        let last = format!("sample_{:04}", MAX_CHECKPOINT_KEYS - 1);
        record_checkpoint("z", &last, 6);
        record_checkpoint("a", "first", 3);
        record_checkpoint("z", &last, 8);
        let rows = entries();
        assert!(matches!(rows[0].key, PerfKey::CheckpointOverflow));
        assert_eq!(rows[0].aggregate.samples(), 3);
        assert_eq!(rows[0].aggregate.total(), 18);
        assert_eq!(checkpoint_entry_for("a", "first").aggregate.total(), 3);
        assert!(!rows.iter().any(|row| matches!(&row.key, PerfKey::Checkpoint { scope, label } if scope == "z" && label == &last)));
    }

    #[test]
    fn checkpoint_overflow_preserves_wide_totals_and_resets_with_the_table() {
        reset();
        assert!(checkpoint_overflow_entry().is_none());
        let scope = "x".repeat(129);
        record_checkpoint(&scope, "wide", u64::MAX);
        record_checkpoint(&scope, "wide", u64::MAX);
        let overflow = checkpoint_overflow_entry().unwrap();
        assert_eq!(overflow.aggregate.samples(), 2);
        assert_eq!(overflow.aggregate.total(), u128::from(u64::MAX) * 2);
        reset();
        assert!(checkpoint_overflow_entry().is_none());
    }

    #[test]
    fn borrowed_recording_preserves_mixed_key_order_and_independent_totals() {
        reset();
        // Insert owned keys first, then look them up through borrowed names.
        record(
            PerfKey::Checkpoint {
                scope: "a".into(),
                label: "é".into(),
            },
            3,
        );
        record_checkpoint("a", "é", 0);
        record_checkpoint("a", "e", 8);
        record_checkpoint("aa", "e", 9);
        record_endpoint_call(call("aa", EndpointCallKind::Query), 10);
        record_endpoint_call(call("a", EndpointCallKind::Update), 20);
        record_endpoint_call(call("a", EndpointCallKind::QueryComposite), 30);
        record_endpoint_call(call("a", EndpointCallKind::Query), u64::MAX);
        record_endpoint_call(call("a", EndpointCallKind::Query), 1);
        let observed = entries();
        let expected = vec![
            PerfKey::Endpoint {
                kind: EndpointCallKind::Query,
                name: "a".into(),
            },
            PerfKey::Endpoint {
                kind: EndpointCallKind::Query,
                name: "aa".into(),
            },
            PerfKey::Endpoint {
                kind: EndpointCallKind::QueryComposite,
                name: "a".into(),
            },
            PerfKey::Endpoint {
                kind: EndpointCallKind::Update,
                name: "a".into(),
            },
            PerfKey::Checkpoint {
                scope: "a".into(),
                label: "e".into(),
            },
            PerfKey::Checkpoint {
                scope: "a".into(),
                label: "é".into(),
            },
            PerfKey::Checkpoint {
                scope: "aa".into(),
                label: "e".into(),
            },
        ];
        assert!(
            observed
                .iter()
                .map(|entry| entry.key.clone())
                .collect::<Vec<_>>()
                == expected
        );
        assert_eq!(observed[0].aggregate.samples(), 2);
        assert_eq!(observed[0].aggregate.total(), u128::from(u64::MAX) + 1);
        assert_eq!(observed[5].aggregate.samples(), 2);
        assert_eq!(observed[5].aggregate.total(), 3);
        assert_eq!(observed[6].aggregate.samples(), 1);
        assert_eq!(observed[6].aggregate.total(), 9);
        reset();
        record_checkpoint("a", "é", 4);
        let after_reset = entries();
        assert_eq!(after_reset.len(), 1);
        assert_eq!(after_reset[0].aggregate.samples(), 1);
        assert_eq!(after_reset[0].aggregate.total(), 4);
    }

    #[test]
    fn zero_sample_is_distinct_from_missing_endpoint() {
        reset();
        assert!(entries().is_empty());
        record_endpoint_call(call("measured", EndpointCallKind::Update), 0);
        let observed = entry_for(EndpointCallKind::Update, "measured");
        assert_eq!(observed.aggregate.samples(), 1);
        assert_eq!(observed.aggregate.total(), 0);
    }

    #[test]
    fn nested_endpoints_record_exclusive_totals() {
        reset();

        let context = PerfContext::new();
        let _activation = ContextActivation::enter(Rc::clone(&context));
        let parent = EndpointScope::start_at(Rc::clone(&context), 100);
        let child = EndpointScope::start_at(context, 200);
        child.finish_at(call("child", EndpointCallKind::Query), 260);
        parent.finish_at(call("parent", EndpointCallKind::Update), 300);

        let parent = entry_for(EndpointCallKind::Update, "parent");
        let child = entry_for(EndpointCallKind::Query, "child");

        assert_eq!(child.aggregate.samples(), 1);
        assert_eq!(child.aggregate.total(), 60);
        assert_eq!(parent.aggregate.samples(), 1);
        assert_eq!(parent.aggregate.total(), 140);
    }

    #[test]
    fn interleaved_invocations_own_frames_and_checkpoint_baselines() {
        reset();
        let a = PerfContext::new();
        let b = PerfContext::new();
        a.borrow_mut().last_checkpoint = 40_000;
        b.borrow_mut().last_checkpoint = 45_000;
        let a_scope = EndpointScope::start_at(Rc::clone(&a), 40_000);
        let b_scope = EndpointScope::start_at(Rc::clone(&b), 45_000);
        {
            let _active = ContextActivation::enter(Rc::clone(&a));
            assert_eq!(
                checkpoint_at("a", "first", 60_000).unwrap().instructions,
                20_000
            );
        }
        {
            let _active = ContextActivation::enter(Rc::clone(&b));
            assert_eq!(
                checkpoint_at("b", "first", 70_000).unwrap().instructions,
                25_000
            );
        }
        {
            let _active = ContextActivation::enter(a);
            assert_eq!(
                checkpoint_at("a", "resume", 2_000_000)
                    .unwrap()
                    .instructions,
                1_940_000
            );
            a_scope.finish_at(call("a", EndpointCallKind::Update), 2_000_000);
        }
        {
            let _active = ContextActivation::enter(b);
            assert_eq!(
                checkpoint_at("b", "resume", 300_000).unwrap().instructions,
                230_000
            );
            b_scope.finish_at(call("b", EndpointCallKind::Update), 300_000);
        }
        assert_eq!(
            entry_for(EndpointCallKind::Update, "a").aggregate.total(),
            1_960_000
        );
        assert_eq!(
            entry_for(EndpointCallKind::Update, "b").aggregate.total(),
            255_000
        );
        assert!(checkpoint_at("unowned", "missing", 3_000_000).is_none());
    }

    #[test]
    fn pending_futures_restore_context_and_cancellation_discards_owned_state() {
        use std::task::{Context, Poll};
        reset();
        let make_future = |scope: &'static str, first, resumed| {
            let mut polled = false;
            with_async_context(poll_fn(move |_| {
                if polled {
                    checkpoint_at(scope, "resume", resumed).unwrap();
                    Poll::Ready(())
                } else {
                    polled = true;
                    checkpoint_at(scope, "first", first).unwrap();
                    Poll::Pending
                }
            }))
        };
        let mut a = Box::pin(make_future("a", 100, 300));
        let mut b = Box::pin(make_future("b", 500, 800));
        let mut cancelled = Box::pin(make_future("cancelled", 900, 1_000));
        let mut task = Context::from_waker(futures::task::noop_waker_ref());
        assert!(a.as_mut().poll(&mut task).is_pending());
        assert!(b.as_mut().poll(&mut task).is_pending());
        assert!(cancelled.as_mut().poll(&mut task).is_pending());
        drop(cancelled);
        assert!(checkpoint_at("outside", "no_base", 1_100).is_none());
        assert!(a.as_mut().poll(&mut task).is_ready());
        assert!(b.as_mut().poll(&mut task).is_ready());
        assert_eq!(checkpoint_entry_for("a", "resume").aggregate.total(), 200);
        assert_eq!(checkpoint_entry_for("b", "resume").aggregate.total(), 300);
        assert!(!entries().iter().any(|entry| matches!(&entry.key,
            PerfKey::Checkpoint { scope, label } if scope == "cancelled" && label == "resume")));
        assert!(ACTIVE_CONTEXT.with_borrow(Option::is_none));
    }

    #[test]
    fn async_endpoint_completion_and_cancellation_preserve_the_enclosing_context() {
        reset();
        let parent = PerfContext::new();
        let _active = ContextActivation::enter(Rc::clone(&parent));
        let owned = Rc::new(RefCell::new(None));
        let observed = Rc::clone(&owned);
        let mut pending = true;
        let mut completed = Box::pin(measure_endpoint_async(
            call("completed", EndpointCallKind::Update),
            poll_fn(move |_| {
                ACTIVE_CONTEXT.with_borrow(|active| {
                    let context = active.as_ref().unwrap();
                    assert_eq!(context.as_ref().borrow().frames.len(), 1);
                    *observed.borrow_mut() = Some(Rc::clone(context));
                });
                measure_endpoint(call("nested", EndpointCallKind::Query), || {});
                if std::mem::take(&mut pending) {
                    Poll::Pending
                } else {
                    Poll::Ready(String::from("result"))
                }
            }),
        ));
        let cancelled_context = Rc::new(RefCell::new(None));
        let observed = Rc::clone(&cancelled_context);
        let mut cancelled = Box::pin(measure_endpoint_async(
            call("cancelled", EndpointCallKind::Update),
            poll_fn(move |_| {
                *observed.borrow_mut() = ACTIVE_CONTEXT.with_borrow(Clone::clone);
                Poll::<()>::Pending
            }),
        ));
        let mut task = Context::from_waker(futures::task::noop_waker_ref());
        // Construction is lazy, and pending/cancelled invocations have no sample.
        assert!(owned.as_ref().borrow().is_none());
        assert!(completed.as_mut().poll(&mut task).is_pending());
        assert!(cancelled.as_mut().poll(&mut task).is_pending());
        assert!(ACTIVE_CONTEXT.with_borrow(|active| Rc::ptr_eq(active.as_ref().unwrap(), &parent)));
        drop(cancelled);
        assert!(
            cancelled_context
                .as_ref()
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .borrow()
                .frames
                .is_empty()
        );
        assert_eq!(
            completed.as_mut().poll(&mut task),
            Poll::Ready(String::from("result"))
        );
        assert!(ACTIVE_CONTEXT.with_borrow(|active| Rc::ptr_eq(active.as_ref().unwrap(), &parent)));
        assert!(
            owned
                .as_ref()
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .borrow()
                .frames
                .is_empty()
        );
        assert_eq!(
            entry_for(EndpointCallKind::Update, "completed")
                .aggregate
                .samples(),
            1
        );
        assert_eq!(
            entry_for(EndpointCallKind::Query, "nested")
                .aggregate
                .samples(),
            2
        );
        assert!(!entries().iter().any(|entry| matches!(&entry.key,
            PerfKey::Endpoint { name, .. } if name == "cancelled")));
    }

    #[test]
    fn async_endpoint_unwind_restores_parent_and_discards_its_frame() {
        reset();
        let parent = PerfContext::new();
        let _active = ContextActivation::enter(Rc::clone(&parent));
        let owned = Rc::new(RefCell::new(None));
        let observed = Rc::clone(&owned);
        let mut future = Box::pin(measure_endpoint_async(
            call("panicked", EndpointCallKind::Update),
            poll_fn(move |_| -> Poll<()> {
                *observed.borrow_mut() = ACTIVE_CONTEXT.with_borrow(Clone::clone);
                panic!("fixture unwind");
            }),
        ));
        let mut task = Context::from_waker(futures::task::noop_waker_ref());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            future.as_mut().poll(&mut task)
        }));
        assert!(result.is_err());
        assert!(ACTIVE_CONTEXT.with_borrow(|active| Rc::ptr_eq(active.as_ref().unwrap(), &parent)));
        drop(future);
        assert!(
            owned
                .as_ref()
                .borrow()
                .as_ref()
                .unwrap()
                .as_ref()
                .borrow()
                .frames
                .is_empty()
        );
        assert!(entries().is_empty());
    }

    #[test]
    fn synchronous_unwind_restores_parent_frames_and_checkpoint_owner() {
        reset();
        let parent = PerfContext::new();
        let _active = ContextActivation::enter(Rc::clone(&parent));
        let parent_scope = EndpointScope::start_at(Rc::clone(&parent), 100);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            measure_endpoint(call("trapped_child", EndpointCallKind::Query), || {
                panic!("fixture unwind");
            });
        }));
        assert!(result.is_err());
        assert_eq!(parent.as_ref().borrow().frames.len(), 1);
        assert!(ACTIVE_CONTEXT.with_borrow(|active| Rc::ptr_eq(active.as_ref().unwrap(), &parent)));
        parent_scope.finish_at(call("parent", EndpointCallKind::Update), 300);
        assert_eq!(
            entry_for(EndpointCallKind::Update, "parent")
                .aggregate
                .total(),
            200
        );
    }

    #[test]
    fn endpoint_perf_keys_preserve_call_kind() {
        reset();

        record_endpoint_call(call("same_name", EndpointCallKind::Query), 10);
        record_endpoint_call(call("same_name", EndpointCallKind::QueryComposite), 20);
        record_endpoint_call(call("same_name", EndpointCallKind::Update), 30);

        assert_eq!(
            entry_for(EndpointCallKind::Query, "same_name")
                .aggregate
                .total(),
            10
        );
        assert_eq!(
            entry_for(EndpointCallKind::QueryComposite, "same_name")
                .aggregate
                .total(),
            20
        );
        assert_eq!(
            entry_for(EndpointCallKind::Update, "same_name")
                .aggregate
                .total(),
            30
        );
    }

    #[test]
    fn checkpoints_record_scope_and_label() {
        reset();

        record_checkpoint("workflow::bootstrap", "load_cfg", 120);
        record_checkpoint("workflow::bootstrap", "load_cfg", 80);

        let checkpoint = checkpoint_entry_for("workflow::bootstrap", "load_cfg");

        assert_eq!(checkpoint.aggregate.samples(), 2);
        assert_eq!(checkpoint.aggregate.total(), 200);
    }
}
