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
        candid::{CandidType, encode_one},
        types::{BoundedString128, Principal},
    },
    dto::{
        auth::{
            CanicInternalCallEnvelopeV1, CanicInternalCallHeaderV1, InternalInvocationProofRequest,
            SignedInternalInvocationProofV1,
        },
        error::Error,
    },
    ids::CanisterRole,
    ops::{config::ConfigOps, ic::IcOps, runtime::env::EnvOps},
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
        IntentSpec as WorkflowIntentSpec,
    },
};
use candid::{
    encode_args,
    utils::{ArgumentDecoder, ArgumentEncoder},
};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

const DEFAULT_INTERNAL_CALL_PROOF_TTL_SECS: u64 = 120;

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
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder<'static> {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

///
/// CanicCall
///
/// Low-level protected Canic internal-call primitive.
///
/// Unlike `Call`, this API is only for Canic-to-Canic protected internal
/// endpoints. It obtains a root-signed method-scoped invocation proof, wraps
/// the original Candid arguments in `CanicInternalCallEnvelopeV1`, and dispatches
/// through the raw call path.
///

pub struct CanicCall;

impl CanicCall {
    #[must_use]
    pub fn bounded_wait(
        canister_id: impl Into<Principal>,
        method: &str,
    ) -> CanicCallBuilder<'static> {
        CanicCallBuilder::new(WaitMode::Bounded, canister_id.into(), method)
    }

    #[must_use]
    pub fn unbounded_wait(
        canister_id: impl Into<Principal>,
        method: &str,
    ) -> CanicCallBuilder<'static> {
        CanicCallBuilder::new(WaitMode::Unbounded, canister_id.into(), method)
    }
}

///
/// CanicCallBuilder
///

pub struct CanicCallBuilder<'a> {
    wait: WaitMode,
    canister_id: Principal,
    method: String,
    caller_role: Option<CanisterRole>,
    ttl_secs: Option<u64>,
    cycles: u128,
    args: Cow<'a, [u8]>,
}

impl CanicCallBuilder<'_> {
    fn new(wait: WaitMode, canister_id: Principal, method: &str) -> Self {
        Self {
            wait,
            canister_id,
            method: method.to_string(),
            caller_role: None,
            ttl_secs: None,
            cycles: 0,
            args: Cow::Owned(encode_args(()).expect("empty Candid tuple must encode")),
        }
    }

    #[must_use]
    pub fn with_caller_role(mut self, role: CanisterRole) -> Self {
        self.caller_role = Some(role);
        self
    }

    #[must_use]
    pub const fn with_proof_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    pub fn with_arg<A>(mut self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        self.args = Cow::Owned(encode_one(arg).map_err(|err| Error::invalid(err.to_string()))?);
        Ok(self)
    }

    pub fn with_args<A>(mut self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        self.args = Cow::Owned(encode_args(args).map_err(|err| Error::invalid(err.to_string()))?);
        Ok(self)
    }

    #[must_use]
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CanicCallBuilder<'b> {
        CanicCallBuilder {
            wait: self.wait,
            canister_id: self.canister_id,
            method: self.method,
            caller_role: self.caller_role,
            ttl_secs: self.ttl_secs,
            cycles: self.cycles,
            args: args.into(),
        }
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    pub async fn execute(self) -> Result<CallResult, Error> {
        let ttl_secs = self.proof_ttl_secs()?;
        let role = self
            .caller_role
            .ok_or_else(|| Error::invalid("CanicCall requires with_caller_role(...)"))?;
        let proof = crate::api::auth::AuthApi::request_internal_invocation_proof(
            InternalInvocationProofRequest {
                subject: IcOps::canister_self(),
                role,
                subnet_id: EnvOps::subnet_pid().ok(),
                audience: self.canister_id,
                audience_method: self.method.clone(),
                ttl_secs,
                metadata: None,
            },
        )
        .await?;

        let envelope = build_internal_call_envelope(
            self.canister_id,
            &self.method,
            proof,
            self.args.into_owned(),
        );
        let call = match self.wait {
            WaitMode::Bounded => Call::bounded_wait(self.canister_id, &self.method),
            WaitMode::Unbounded => Call::unbounded_wait(self.canister_id, &self.method),
        }
        .with_cycles(self.cycles)
        .with_arg(envelope)?;

        call.execute().await
    }

    fn proof_ttl_secs(&self) -> Result<u64, Error> {
        let requested = self
            .ttl_secs
            .unwrap_or(DEFAULT_INTERNAL_CALL_PROOF_TTL_SECS);
        let max = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .max_ttl_secs;
        Ok(requested.min(max))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WaitMode {
    Bounded,
    Unbounded,
}

fn build_internal_call_envelope(
    target_canister: Principal,
    target_method: &str,
    proof: SignedInternalInvocationProofV1,
    args: Vec<u8>,
) -> CanicInternalCallEnvelopeV1 {
    CanicInternalCallEnvelopeV1 {
        version: 1,
        header: CanicInternalCallHeaderV1 {
            target_canister,
            target_method: target_method.to_string(),
        },
        proof,
        args,
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

pub struct CallBuilder<'a> {
    inner: WorkflowCallBuilder<'a>,
}

impl CallBuilder<'_> {
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
    pub fn with_raw_args<'b>(self, args: impl Into<Cow<'b, [u8]>>) -> CallBuilder<'b> {
        CallBuilder {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::auth::{InternalInvocationProofPayloadV1, SignedInternalInvocationProofV1};
    use candid::decode_args;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn proof() -> SignedInternalInvocationProofV1 {
        SignedInternalInvocationProofV1 {
            payload: InternalInvocationProofPayloadV1 {
                subject: p(1),
                role: CanisterRole::new("project_hub"),
                subnet_id: None,
                audience: p(2),
                audience_method: "system_add_project_to_user".to_string(),
                issued_at: 10,
                expires_at: 20,
                epoch: 3,
            },
            signature: vec![1, 2, 3],
            key_id: 1,
        }
    }

    #[test]
    fn canic_call_envelope_binds_target_method_and_original_args() {
        let args = encode_args((7_u64, "project")).expect("args encode");
        let envelope =
            build_internal_call_envelope(p(2), "system_add_project_to_user", proof(), args);

        assert_eq!(envelope.version, 1);
        assert_eq!(envelope.header.target_canister, p(2));
        assert_eq!(envelope.header.target_method, "system_add_project_to_user");
        assert_eq!(
            envelope.proof.payload.audience_method,
            "system_add_project_to_user"
        );

        let decoded: (u64, String) = decode_args(&envelope.args).expect("decode original args");
        assert_eq!(decoded, (7, "project".to_string()));
    }

    #[test]
    fn canic_call_builder_records_role_and_raw_args() {
        let raw = vec![9_u8, 8, 7];
        let builder = CanicCall::unbounded_wait(p(3), "target")
            .with_caller_role(CanisterRole::new("project_hub"))
            .with_proof_ttl_secs(30)
            .with_cycles(10)
            .with_raw_args(raw.clone());

        assert_eq!(builder.wait, WaitMode::Unbounded);
        assert_eq!(builder.canister_id, p(3));
        assert_eq!(builder.method, "target");
        assert_eq!(builder.caller_role, Some(CanisterRole::new("project_hub")));
        assert_eq!(builder.ttl_secs, Some(30));
        assert_eq!(builder.cycles, 10);
        assert_eq!(builder.args.as_ref(), raw.as_slice());
    }
}
