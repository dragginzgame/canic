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
    future::{Future, poll_fn},
    pin::pin,
    rc::Rc,
};

thread_local! {
    // Installed only during a synchronous invocation or one future poll.
    static ACTIVE_CONTEXT: RefCell<Option<Rc<RefCell<PerfContext>>>> = const { RefCell::new(None) };

    /// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
    static PERF_TABLE: RefCell<BTreeMap<PerfKey, PerfSlot>> = const { RefCell::new(BTreeMap::new()) };


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
        ic_cdk::api::call_context_instruction_counter()
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
            Self::Endpoint { kind, name } => PerfKeyRef::Endpoint { kind: *kind, name },
            Self::Checkpoint { scope, label } => PerfKeyRef::Checkpoint { scope, label },
        }
    }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum PerfKeyRef<'key> {
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
/// PerfSlot
///

#[derive(Default)]
struct PerfSlot {
    count: u64,
    total_instructions: u64,
}

///
/// PerfEntry
/// Aggregated perf counters keyed by kind (endpoint vs timer) and label.
///

#[derive(Clone)]
pub struct PerfEntry {
    pub key: PerfKey,
    pub count: u64,
    pub total_instructions: u64,
}

/// Record a counter under the provided key.
pub fn record(key: PerfKey, delta: u64) {
    PERF_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let slot = table.entry(key).or_default();
        ic_metrics::record_sample(&mut slot.count, &mut slot.total_instructions, delta);
    });
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

pub fn record_checkpoint(scope: &str, label: &str, delta_instructions: u64) {
    record_borrowed(PerfKeyRef::Checkpoint { scope, label }, delta_instructions);
}

fn record_borrowed(key: PerfKeyRef<'_>, delta: u64) {
    PERF_TABLE.with_borrow_mut(|table| {
        let slot = if let Some(slot) = table.get_mut(&key as &dyn PerfKeyView) {
            slot
        } else {
            table.entry(key.into_owned()).or_default()
        };
        ic_metrics::record_sample(&mut slot.count, &mut slot.total_instructions, delta);
    });
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
    let context = PerfContext::new();
    let mut future = pin!(future);
    poll_fn(|task| {
        let _activation = ContextActivation::enter(Rc::clone(&context));
        future.as_mut().poll(task)
    })
    .await
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
pub(crate) async fn measure_endpoint_async<F: Future>(call: EndpointCall, future: F) -> F::Output {
    with_async_context(async {
        let context =
            ACTIVE_CONTEXT.with_borrow(|active| active.clone().expect("active future poll"));
        let scope = EndpointScope::start_at(context, perf_counter());
        let result = future.await;
        scope.finish_at(call, perf_counter());
        result
    })
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
            .iter()
            .map(|(key, slot)| PerfEntry {
                key: key.clone(),
                count: slot.count,
                total_instructions: slot.total_instructions,
            })
            .collect()
    })
}

/// Read a key-ordered prefix without cloning arbitrarily large checkpoint labels.
pub(crate) fn bounded_entries(limit: usize) -> Result<Vec<PerfEntry>, crate::InternalError> {
    PERF_TABLE.with_borrow(|table| {
        table
            .iter()
            .take(limit)
            .map(|(key, slot)| {
                let valid = match key {
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
                    count: slot.count,
                    total_instructions: slot.total_instructions,
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
    PERF_TABLE.with(|t| t.borrow_mut().clear());
    ACTIVE_CONTEXT.with_borrow_mut(|active| *active = None);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(observed[0].count, 2);
        assert_eq!(observed[0].total_instructions, u64::MAX);
        assert_eq!(observed[5].count, 2);
        assert_eq!(observed[5].total_instructions, 3);
        assert_eq!(observed[6].count, 1);
        assert_eq!(observed[6].total_instructions, 9);
        reset();
        record_checkpoint("a", "é", 4);
        let after_reset = entries();
        assert_eq!(after_reset.len(), 1);
        assert_eq!(after_reset[0].count, 1);
        assert_eq!(after_reset[0].total_instructions, 4);
    }

    #[test]
    fn zero_sample_is_distinct_from_missing_endpoint() {
        reset();
        assert!(entries().is_empty());
        record_endpoint_call(call("measured", EndpointCallKind::Update), 0);
        let observed = entry_for(EndpointCallKind::Update, "measured");
        assert_eq!(observed.count, 1);
        assert_eq!(observed.total_instructions, 0);
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

        assert_eq!(child.count, 1);
        assert_eq!(child.total_instructions, 60);
        assert_eq!(parent.count, 1);
        assert_eq!(parent.total_instructions, 140);
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
            entry_for(EndpointCallKind::Update, "a").total_instructions,
            1_960_000
        );
        assert_eq!(
            entry_for(EndpointCallKind::Update, "b").total_instructions,
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
        assert_eq!(checkpoint_entry_for("a", "resume").total_instructions, 200);
        assert_eq!(checkpoint_entry_for("b", "resume").total_instructions, 300);
        assert!(!entries().iter().any(|entry| matches!(&entry.key,
            PerfKey::Checkpoint { scope, label } if scope == "cancelled" && label == "resume")));
        assert!(ACTIVE_CONTEXT.with_borrow(Option::is_none));
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
            entry_for(EndpointCallKind::Update, "parent").total_instructions,
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
            entry_for(EndpointCallKind::Query, "same_name").total_instructions,
            10
        );
        assert_eq!(
            entry_for(EndpointCallKind::QueryComposite, "same_name").total_instructions,
            20
        );
        assert_eq!(
            entry_for(EndpointCallKind::Update, "same_name").total_instructions,
            30
        );
    }

    #[test]
    fn checkpoints_record_scope_and_label() {
        reset();

        record_checkpoint("workflow::bootstrap", "load_cfg", 120);
        record_checkpoint("workflow::bootstrap", "load_cfg", 80);

        let checkpoint = checkpoint_entry_for("workflow::bootstrap", "load_cfg");

        assert_eq!(checkpoint.count, 2);
        assert_eq!(checkpoint.total_instructions, 200);
    }
}
