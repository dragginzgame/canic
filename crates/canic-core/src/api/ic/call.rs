//! Public IC call façade with optional intent-based concurrency control.
//!
//! This module defines the stable, public API used by application code to make
//! inter-canister calls. It deliberately exposes a *thin* surface:
//!
//! - argument encoding
//! - cycle attachment
//! - optional intent declaration
//!
//! It does NOT:
//! - perform orchestration itself
//! - expose intent internals
//! - leak workflow or storage details
//!
//! If an intent is attached to a call, the actual multi-step behavior
//! (reserve → call → commit/abort) is handled by the workflow layer.
//!
//! This separation keeps application code simple while ensuring correctness
//! under concurrency.
use crate::{
    cdk::{
        candid::CandidType,
        types::{BoundedString128, Principal},
    },
    dto::error::Error,
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
        IntentSpec as WorkflowIntentSpec,
    },
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;

///
/// Call
///
/// Entry point for constructing inter-canister calls.
///
/// `Call` itself has no state; it simply selects the wait semantics
/// (bounded vs unbounded) and produces a `CallBuilder`.
///
/// Think of this as the *verb* (“make a call”), not the call itself.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

///
/// IntentKey
///
/// Stable, bounded identifier for a contended resource.
///
/// An intent key names *what is being reserved*, not how the reservation
/// is enforced. Keys are opaque strings with a fixed maximum length
/// to ensure safe storage and indexing.
///
/// Examples:
/// - "vendor:abc123:inventory"
/// - "collection:xyz:mint"
///

pub struct IntentKey(BoundedString128);

impl IntentKey {
    pub fn try_new(value: impl Into<String>) -> Result<Self, Error> {
        BoundedString128::try_new(value)
            .map(Self)
            .map_err(Error::invalid)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[must_use]
    pub fn into_inner(self) -> BoundedString128 {
        self.0
    }
}

impl AsRef<str> for IntentKey {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<IntentKey> for BoundedString128 {
    fn from(key: IntentKey) -> Self {
        key.0
    }
}

///
/// IntentReservation
///
/// Declarative reservation attached to a call.
///
/// An intent expresses *preconditions* for executing a call, such as:
/// - how much of a resource is required (`quantity`)
/// - how long the reservation may remain pending (`ttl_secs`)
/// - optional concurrency caps (`max_in_flight`)
///
/// Importantly:
/// - An intent is **single-shot**
/// - Failed intents are not reused
/// - Retrying requires creating a new intent
///
/// The reservation itself is enforced by the workflow layer.
///

pub struct IntentReservation {
    key: IntentKey,
    quantity: u64,
    ttl_secs: Option<u64>,
    max_in_flight: Option<u64>,
}

impl IntentReservation {
    #[must_use]
    pub const fn new(key: IntentKey, quantity: u64) -> Self {
        Self {
            key,
            quantity,
            ttl_secs: None,
            max_in_flight: None,
        }
    }

    #[must_use]
    pub const fn with_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    #[must_use]
    pub const fn with_max_in_flight(mut self, max_in_flight: u64) -> Self {
        self.max_in_flight = Some(max_in_flight);
        self
    }

    pub(crate) fn into_spec(self) -> WorkflowIntentSpec {
        WorkflowIntentSpec::new(
            self.key.into(),
            self.quantity,
            self.ttl_secs,
            self.max_in_flight,
        )
    }
}

///
/// CallBuilder (api)
///

pub struct CallBuilder {
    inner: WorkflowCallBuilder,
}

impl CallBuilder {
    // ---------- arguments ----------

    /// Encode a single argument into Candid bytes (fallible).
    pub fn with_arg<A>(self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.with_arg(arg).map_err(Error::from)?,
        })
    }

    /// Encode multiple arguments into Candid bytes (fallible).
    pub fn with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.with_args(args).map_err(Error::from)?,
        })
    }

    /// Use pre-encoded Candid arguments (no validation performed).
    #[must_use]
    pub fn with_raw_args(self, args: Vec<u8>) -> Self {
        Self {
            inner: self.inner.with_raw_args(args),
        }
    }

    // ---------- cycles ----------

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    // ---------- intent ----------

    #[must_use]
    pub fn with_intent(self, intent: IntentReservation) -> Self {
        Self {
            inner: self.inner.with_intent(intent.into_spec()),
        }
    }

    // ---------- execution ----------

    pub async fn execute(self) -> Result<CallResult, Error> {
        Ok(CallResult {
            inner: self.inner.execute().await.map_err(Error::from)?,
        })
    }
}

///
/// CallResult
///
/// Stable wrapper around an inter-canister call response.
///
/// This type exists to:
/// - decouple API consumers from infra response types
/// - provide uniform decoding helpers
/// - allow future extension without breaking callers
///

pub struct CallResult {
    inner: WorkflowCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(Error::from)
    }

    pub fn candid_tuple<R>(&self) -> Result<R, Error>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple().map_err(Error::from)
    }
}
