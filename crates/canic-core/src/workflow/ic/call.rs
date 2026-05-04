use crate::{
    InternalError, InternalErrorOrigin,
    ids::IntentResourceKey,
    ops::{
        ic::{
            IcOps,
            call::{CallBuilder as OpsCallBuilder, CallOps, CallResult as OpsCallResult},
        },
        runtime::metrics::intent::{
            IntentMetricOperation, IntentMetricOutcome, IntentMetricReason, IntentMetricSurface,
            IntentMetrics,
        },
        storage::intent::IntentStoreOps,
    },
    workflow::{prelude::*, runtime::intent::IntentCleanupWorkflow},
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

///
/// CallWorkflow
///

pub struct CallWorkflow;

impl CallWorkflow {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::bounded_wait(canister_id, method),
            intent: None,
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallOps::unbounded_wait(canister_id, method),
            intent: None,
        }
    }
}

///
/// IntentSpec
/// Internal intent spec for call orchestration.
///

pub struct IntentSpec {
    key: IntentResourceKey,
    quantity: u64,
    ttl_secs: Option<u64>,
    max_in_flight: Option<u64>,
}

impl IntentSpec {
    pub const fn new(
        key: IntentResourceKey,
        quantity: u64,
        ttl_secs: Option<u64>,
        max_in_flight: Option<u64>,
    ) -> Self {
        Self {
            key,
            quantity,
            ttl_secs,
            max_in_flight,
        }
    }
}

///
/// CallBuilder (workflow)
///

pub struct CallBuilder<'a> {
    inner: OpsCallBuilder<'a>,
    intent: Option<IntentSpec>,
}

impl CallBuilder<'_> {
    // ---------- arguments ----------

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        let Self { inner, intent } = self;
        let inner = inner.with_arg(arg)?;

        Ok(Self { inner, intent })
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        let Self { inner, intent } = self;
        let inner = inner.with_args(args)?;

        Ok(Self { inner, intent })
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        let Self { inner, intent } = self;
        let inner = inner.with_raw_args(args);

        CallBuilder { inner, intent }
    }

    // ---------- cycles ----------

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        let Self { inner, intent } = self;

        Self {
            inner: inner.with_cycles(cycles),
            intent,
        }
    }

    // ---------- intent ----------

    #[must_use]
    pub fn with_intent(mut self, intent: IntentSpec) -> Self {
        self.intent = Some(intent);
        self
    }

    // ---------- execution ----------

    pub async fn execute(self) -> Result<CallResult, InternalError> {
        // Intent semantics:
        // - reserve before executing the call
        // - commit on success; commit errors are logged, call result still returned
        // - abort on failure; abort errors are attached to the call error
        let Self { inner, intent } = self;
        let now = IcOps::now_secs();

        let Some(intent) = intent else {
            return Ok(CallResult {
                inner: inner.execute().await?,
            });
        };

        let resource_key = IntentResourceKey::try_new(intent.key.clone()).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("intent key invalid: {err}"),
            )
        })?;

        IntentCleanupWorkflow::ensure_started();

        enforce_call_intent_capacity(&resource_key, &intent, now)?;
        let intent_id = reserve_call_intent(&resource_key, &intent, now)?;

        match inner.execute().await {
            Ok(inner) => {
                commit_call_intent(intent_id, &resource_key, now);

                Ok(CallResult { inner })
            }
            Err(call_err) => abort_call_intent(intent_id, call_err),
        }
    }
}

// Enforce the optional in-flight limit before reserving an intent.
fn enforce_call_intent_capacity(
    resource_key: &IntentResourceKey,
    intent: &IntentSpec,
    now: u64,
) -> Result<(), InternalError> {
    let Some(max_in_flight) = intent.max_in_flight else {
        return Ok(());
    };

    let totals = IntentStoreOps::totals_at(resource_key, now);
    let in_flight = totals.reserved_qty;
    let next = match next_in_flight_quantity(in_flight, intent.quantity) {
        Ok(next) => next,
        Err(err) => {
            record_call_intent(
                IntentMetricOperation::CapacityCheck,
                IntentMetricOutcome::Failed,
                IntentMetricReason::Overflow,
            );
            return Err(err);
        }
    };

    if next > max_in_flight {
        record_call_intent(
            IntentMetricOperation::CapacityCheck,
            IntentMetricOutcome::Failed,
            IntentMetricReason::Capacity,
        );
        return Err(InternalError::domain(
            InternalErrorOrigin::Domain,
            format!(
                "intent capacity exceeded key={resource_key} in_flight={in_flight} \
requested={} max_in_flight={max_in_flight}",
                intent.quantity
            ),
        ));
    }

    record_call_intent(
        IntentMetricOperation::CapacityCheck,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );

    Ok(())
}

// Reserve a call intent and record the storage outcome.
fn reserve_call_intent(
    resource_key: &IntentResourceKey,
    intent: &IntentSpec,
    now: u64,
) -> Result<crate::storage::stable::intent::IntentId, InternalError> {
    let intent_id = match IntentStoreOps::allocate_intent_id() {
        Ok(intent_id) => intent_id,
        Err(err) => {
            record_call_intent(
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
            return Err(err);
        }
    };
    let _ = match IntentStoreOps::try_reserve(
        intent_id,
        resource_key.clone(),
        intent.quantity,
        now,
        intent.ttl_secs,
        now,
    ) {
        Ok(record) => {
            record_call_intent(
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Completed,
                IntentMetricReason::Ok,
            );
            record
        }
        Err(err) => {
            record_call_intent(
                IntentMetricOperation::Reserve,
                IntentMetricOutcome::Failed,
                IntentMetricReason::StorageFailed,
            );
            return Err(err);
        }
    };

    Ok(intent_id)
}

// Commit a call intent after successful execution; call results remain authoritative.
fn commit_call_intent(
    intent_id: crate::storage::stable::intent::IntentId,
    resource_key: &IntentResourceKey,
    now: u64,
) {
    if let Err(err) = IntentStoreOps::commit_at(intent_id, now) {
        record_call_intent(
            IntentMetricOperation::Commit,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        crate::log!(
            Error,
            "intent commit failed id={intent_id} key={resource_key}: {err}"
        );
    } else {
        record_call_intent(
            IntentMetricOperation::Commit,
            IntentMetricOutcome::Completed,
            IntentMetricReason::Ok,
        );
    }
}

// Abort a call intent after failed execution and attach abort errors to the result.
fn abort_call_intent(
    intent_id: crate::storage::stable::intent::IntentId,
    call_err: InternalError,
) -> Result<CallResult, InternalError> {
    if let Err(abort_err) = IntentStoreOps::abort(intent_id) {
        record_call_intent(
            IntentMetricOperation::Abort,
            IntentMetricOutcome::Failed,
            IntentMetricReason::StorageFailed,
        );
        let message = format!("{call_err}; intent abort failed: {abort_err}");
        return Err(InternalError::new(
            call_err.class(),
            call_err.origin(),
            message,
        ));
    }

    record_call_intent(
        IntentMetricOperation::Abort,
        IntentMetricOutcome::Completed,
        IntentMetricReason::Ok,
    );
    Err(call_err)
}

// Record a call-surface intent metric with fixed labels only.
fn record_call_intent(
    operation: IntentMetricOperation,
    outcome: IntentMetricOutcome,
    reason: IntentMetricReason,
) {
    IntentMetrics::record(IntentMetricSurface::Call, operation, outcome, reason);
}

// Compute the next in-flight quantity after applying a reservation request.
// Returns an invariant error if arithmetic overflows.
fn next_in_flight_quantity(in_flight: u64, requested: u64) -> Result<u64, InternalError> {
    in_flight.checked_add(requested).ok_or_else(|| {
        InternalError::invariant(InternalErrorOrigin::Workflow, "intent reservation overflow")
    })
}

///
/// CallResult (workflow)
///

pub struct CallResult {
    inner: OpsCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, InternalError>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid()
    }

    pub fn candid_tuple<R>(&self) -> Result<R, InternalError>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Guard against arithmetic wraparound when adding a new reservation.
    fn next_in_flight_rejects_overflow() {
        let err = next_in_flight_quantity(u64::MAX, 1).expect_err("overflow must fail");
        assert!(err.to_string().contains("intent reservation overflow"));
    }
}
