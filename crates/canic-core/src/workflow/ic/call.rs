use crate::{
    InternalError, InternalErrorOrigin,
    ids::IntentResourceKey,
    ops::{
        ic::{
            IcOps,
            call::{CallBuilder as OpsCallBuilder, CallOps, CallResult as OpsCallResult},
        },
        storage::intent::IntentStoreOps,
    },
    workflow::prelude::*,
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;

///
/// CallWorkflow
///

pub struct CallWorkflow;

impl CallWorkflow {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallOps::bounded_wait(canister_id, method),
            intent: None,
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallOps::unbounded_wait(canister_id, method),
            intent: None,
        }
    }
}

// Internal intent spec for call orchestration.
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

pub struct CallBuilder {
    inner: OpsCallBuilder,
    intent: Option<IntentSpec>,
}

impl CallBuilder {
    // ---------- arguments ----------

    #[must_use]
    pub fn with_arg<A>(self, arg: A) -> Self
    where
        A: CandidType,
    {
        let Self { inner, intent } = self;

        Self {
            inner: inner.with_arg(arg),
            intent,
        }
    }

    #[must_use]
    pub fn with_args<A>(self, args: A) -> Self
    where
        A: ArgumentEncoder,
    {
        let Self { inner, intent } = self;

        Self {
            inner: inner.with_args(args),
            intent,
        }
    }

    pub fn try_with_arg<A>(self, arg: A) -> Result<Self, InternalError>
    where
        A: CandidType,
    {
        let Self { inner, intent } = self;
        let inner = inner.try_with_arg(arg)?;

        Ok(Self { inner, intent })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, InternalError>
    where
        A: ArgumentEncoder,
    {
        let Self { inner, intent } = self;
        let inner = inner.try_with_args(args)?;

        Ok(Self { inner, intent })
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

        let resource_key = IntentResourceKey::try_new(intent.key).map_err(|err| {
            InternalError::invariant(
                InternalErrorOrigin::Workflow,
                format!("intent key invalid: {err}"),
            )
        })?;

        if let Some(max_in_flight) = intent.max_in_flight {
            let totals = IntentStoreOps::totals_at(&resource_key, now);
            let in_flight = totals
                .reserved_qty
                .checked_add(totals.committed_qty)
                .ok_or_else(|| {
                    InternalError::invariant(
                        InternalErrorOrigin::Workflow,
                        "intent in-flight overflow",
                    )
                })?;

            let next = in_flight.checked_add(intent.quantity).ok_or_else(|| {
                InternalError::invariant(
                    InternalErrorOrigin::Workflow,
                    "intent reservation overflow",
                )
            })?;

            if next > max_in_flight {
                return Err(InternalError::domain(
                    InternalErrorOrigin::Domain,
                    format!(
                        "intent capacity exceeded key={resource_key} in_flight={in_flight} \
requested={} max_in_flight={max_in_flight}",
                        intent.quantity
                    ),
                ));
            }
        }

        let intent_id = IntentStoreOps::allocate_intent_id()?;
        let created_at = IcOps::now_secs();
        let _ = IntentStoreOps::try_reserve(
            intent_id,
            resource_key.clone(),
            intent.quantity,
            created_at,
            intent.ttl_secs,
        )?;

        match inner.execute().await {
            Ok(inner) => {
                if let Err(err) = IntentStoreOps::commit_at(intent_id, now) {
                    crate::log!(
                        Error,
                        "intent commit failed id={intent_id} key={resource_key}: {err}"
                    );
                }

                Ok(CallResult { inner })
            }
            Err(call_err) => {
                if let Err(abort_err) = IntentStoreOps::abort(intent_id) {
                    let message = format!("{call_err}; intent abort failed: {abort_err}");
                    return Err(InternalError::new(
                        call_err.class(),
                        call_err.origin(),
                        message,
                    ));
                }

                Err(call_err)
            }
        }
    }
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
