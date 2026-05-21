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
    config::schema::RoleAttestationConfig,
    dto::{
        auth::{
            CanicInternalCallEnvelopeV1, CanicInternalCallHeaderV1, InternalInvocationProofRequest,
            SignedInternalInvocationProofV1,
        },
        error::Error,
        error::ErrorCode,
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
use std::{borrow::Cow, cell::RefCell, collections::BTreeMap};

const DEFAULT_INTERNAL_CALL_PROOF_TTL_SECS: u64 = 120;
const INTERNAL_CALL_PROOF_REFRESH_MARGIN_MAX_SECS: u64 = 30;

thread_local! {
    static INTERNAL_INVOCATION_PROOF_CACHE:
        RefCell<BTreeMap<InternalInvocationProofCacheKey, SignedInternalInvocationProofV1>> =
        const { RefCell::new(BTreeMap::new()) };
}

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
/// the original Candid arguments in `CanicInternalCallEnvelopeV1`, encodes that
/// envelope as raw ingress bytes, and dispatches through the raw call path.
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
/// ProtectedInternalEndpoint
///
/// Generated metadata for one protected Canic internal endpoint.
///
/// Endpoint macros emit this descriptor next to protected internal endpoints.
/// Callers should pass it to `CanicInternalClient` instead of repeating method
/// names and accepted-role metadata by hand.
///

#[derive(Clone, Debug)]
pub struct ProtectedInternalEndpoint {
    method: &'static str,
    accepted_roles: Vec<CanisterRole>,
}

impl ProtectedInternalEndpoint {
    #[must_use]
    #[track_caller]
    pub fn new(method: &'static str, roles: impl IntoIterator<Item = CanisterRole>) -> Self {
        assert!(
            !method.trim().is_empty(),
            "protected internal endpoint descriptor method must not be empty"
        );
        let accepted_roles = roles.into_iter().collect::<Vec<_>>();
        assert!(
            !accepted_roles.is_empty(),
            "protected internal endpoint descriptor '{method}' must accept at least one caller role"
        );
        for (index, role) in accepted_roles.iter().enumerate() {
            assert!(
                !role.as_str().trim().is_empty(),
                "protected internal endpoint descriptor '{method}' has an empty caller role at index {index}"
            );
            assert!(
                !accepted_roles[..index].iter().any(|prior| prior == role),
                "protected internal endpoint descriptor '{method}' contains duplicate caller role '{role}'"
            );
        }
        Self {
            method,
            accepted_roles,
        }
    }

    #[must_use]
    pub const fn method(&self) -> &'static str {
        self.method
    }

    #[must_use]
    pub fn accepted_roles(&self) -> &[CanisterRole] {
        &self.accepted_roles
    }

    #[must_use]
    pub fn accepts_role(&self, role: &CanisterRole) -> bool {
        self.accepted_roles.iter().any(|accepted| accepted == role)
    }

    #[must_use]
    pub fn single_role(&self) -> Option<&CanisterRole> {
        match self.accepted_roles.as_slice() {
            [role] => Some(role),
            _ => None,
        }
    }

    pub fn required_single_role(&self) -> Result<CanisterRole, Error> {
        self.single_role().cloned().ok_or_else(|| {
            Error::invalid(format!(
                "protected internal endpoint '{}' accepts {} roles; choose a caller role explicitly",
                self.method(),
                self.accepted_roles.len()
            ))
        })
    }
}

///
/// CanicInternalClient
///
/// Generic protected internal client over generated endpoint descriptors.
///

#[derive(Clone, Copy, Debug)]
pub struct CanicInternalClient {
    canister_id: Principal,
    options: CanicInternalCallOptions,
}

impl CanicInternalClient {
    #[must_use]
    pub const fn new(canister_id: Principal) -> Self {
        Self {
            canister_id,
            options: CanicInternalCallOptions::new(),
        }
    }

    #[must_use]
    pub const fn with_options(mut self, options: CanicInternalCallOptions) -> Self {
        self.options = options;
        self
    }

    #[must_use]
    pub const fn with_bounded_wait(mut self) -> Self {
        self.options = self.options.with_bounded_wait();
        self
    }

    #[must_use]
    pub const fn with_unbounded_wait(mut self) -> Self {
        self.options = self.options.with_unbounded_wait();
        self
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.options = self.options.with_cycles(cycles);
        self
    }

    #[must_use]
    pub const fn with_proof_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.options = self.options.with_proof_ttl_secs(ttl_secs);
        self
    }

    pub async fn call_update<A>(
        &self,
        endpoint: &ProtectedInternalEndpoint,
        caller_role: CanisterRole,
        args: A,
    ) -> Result<CallResult, Error>
    where
        A: ArgumentEncoder,
    {
        if !endpoint.accepts_role(&caller_role) {
            return Err(Error::invalid(format!(
                "caller role '{caller_role}' is not accepted by protected internal endpoint '{}'",
                endpoint.method()
            )));
        }

        let builder = match self.options.wait {
            CanicInternalWaitMode::Bounded => {
                CanicCall::bounded_wait(self.canister_id, endpoint.method())
            }
            CanicInternalWaitMode::Unbounded => {
                CanicCall::unbounded_wait(self.canister_id, endpoint.method())
            }
        };
        let builder = builder
            .with_caller_role(caller_role)
            .with_cycles(self.options.cycles);
        let builder = if let Some(ttl_secs) = self.options.proof_ttl_secs {
            builder.with_proof_ttl_secs(ttl_secs)
        } else {
            builder
        };

        builder.with_args(args)?.execute().await
    }

    pub async fn call_update_with_single_role<A>(
        &self,
        endpoint: &ProtectedInternalEndpoint,
        args: A,
    ) -> Result<CallResult, Error>
    where
        A: ArgumentEncoder,
    {
        let role = endpoint.required_single_role()?;
        self.call_update(endpoint, role, args).await
    }

    pub async fn call_update_result<T, A>(
        &self,
        endpoint: &ProtectedInternalEndpoint,
        caller_role: CanisterRole,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let call = self.call_update(endpoint, caller_role, args).await?;
        let result: Result<T, Error> = call.candid()?;
        result
    }

    pub async fn call_update_result_with_single_role<T, A>(
        &self,
        endpoint: &ProtectedInternalEndpoint,
        args: A,
    ) -> Result<T, Error>
    where
        T: CandidType + DeserializeOwned,
        A: ArgumentEncoder,
    {
        let role = endpoint.required_single_role()?;
        self.call_update_result(endpoint, role, args).await
    }
}

///
/// CanicInternalCallOptions
///
/// Transport options shared by generated protected internal clients.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CanicInternalCallOptions {
    wait: CanicInternalWaitMode,
    cycles: u128,
    proof_ttl_secs: Option<u64>,
}

impl CanicInternalCallOptions {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            wait: CanicInternalWaitMode::Unbounded,
            cycles: 0,
            proof_ttl_secs: None,
        }
    }

    #[must_use]
    pub const fn with_bounded_wait(mut self) -> Self {
        self.wait = CanicInternalWaitMode::Bounded;
        self
    }

    #[must_use]
    pub const fn with_unbounded_wait(mut self) -> Self {
        self.wait = CanicInternalWaitMode::Unbounded;
        self
    }

    #[must_use]
    pub const fn with_cycles(mut self, cycles: u128) -> Self {
        self.cycles = cycles;
        self
    }

    #[must_use]
    pub const fn with_proof_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.proof_ttl_secs = Some(ttl_secs);
        self
    }
}

impl Default for CanicInternalCallOptions {
    fn default() -> Self {
        Self::new()
    }
}

///
/// CanicInternalWaitMode
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanicInternalWaitMode {
    Bounded,
    Unbounded,
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
        validate_internal_call_target_method(&self.method)?;
        let ttl_secs = self.proof_ttl_secs()?;
        let role = self
            .caller_role
            .ok_or_else(|| Error::invalid("CanicCall requires with_caller_role(...)"))?;
        validate_internal_call_caller_role(&role)?;
        let request = InternalInvocationProofRequest {
            subject: IcOps::canister_self(),
            role,
            subnet_id: EnvOps::subnet_pid().ok(),
            audience: self.canister_id,
            audience_method: self.method.clone(),
            ttl_secs,
            metadata: None,
        };
        let args = self.args.into_owned();
        let proof = internal_invocation_proof_for_request(request.clone()).await?;

        let envelope =
            build_internal_call_envelope(self.canister_id, &self.method, proof, args.clone());
        let result = execute_internal_call_once(
            self.wait,
            self.canister_id,
            &self.method,
            self.cycles,
            envelope,
        )
        .await?;
        if !internal_call_result_is_retryable(&result) {
            return Ok(result);
        }

        invalidate_internal_invocation_proof(&request)?;
        let proof = fresh_internal_invocation_proof_for_request(request).await?;
        let envelope = build_internal_call_envelope(self.canister_id, &self.method, proof, args);
        execute_internal_call_once(
            self.wait,
            self.canister_id,
            &self.method,
            self.cycles,
            envelope,
        )
        .await
    }

    fn proof_ttl_secs(&self) -> Result<u64, Error> {
        let requested = self
            .ttl_secs
            .unwrap_or(DEFAULT_INTERNAL_CALL_PROOF_TTL_SECS);
        let max = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .max_ttl_secs;
        effective_internal_call_proof_ttl_secs(requested, max)
    }
}

fn validate_internal_call_target_method(method: &str) -> Result<(), Error> {
    if method.trim().is_empty() {
        return Err(Error::invalid(
            "CanicCall requires a non-empty target method",
        ));
    }
    Ok(())
}

fn validate_internal_call_caller_role(role: &CanisterRole) -> Result<(), Error> {
    if role.as_str().trim().is_empty() {
        return Err(Error::invalid("CanicCall requires a non-empty caller role"));
    }
    Ok(())
}

fn effective_internal_call_proof_ttl_secs(requested: u64, max: u64) -> Result<u64, Error> {
    if requested == 0 {
        return Err(Error::invalid(
            "CanicCall proof TTL must be greater than zero",
        ));
    }
    let effective = requested.min(max);
    if effective == 0 {
        return Err(Error::invalid(
            "CanicCall proof TTL maximum must be greater than zero",
        ));
    }
    Ok(effective)
}

async fn execute_internal_call_once(
    wait: WaitMode,
    canister_id: Principal,
    method: &str,
    cycles: u128,
    envelope: CanicInternalCallEnvelopeV1,
) -> Result<CallResult, Error> {
    let call = match wait {
        WaitMode::Bounded => Call::bounded_wait(canister_id, method),
        WaitMode::Unbounded => Call::unbounded_wait(canister_id, method),
    }
    .with_cycles(cycles)
    .with_raw_args(encode_internal_call_envelope_raw(envelope)?);

    call.execute().await
}

///
/// InternalInvocationProofCacheKey
///

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct InternalInvocationProofCacheKey {
    root_pid: Principal,
    attestation_key_name: String,
    subject: Principal,
    role: CanisterRole,
    subnet_id: Option<Principal>,
    audience: Principal,
    audience_method: String,
    ttl_secs: u64,
}

async fn internal_invocation_proof_for_request(
    request: InternalInvocationProofRequest,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let now_secs = IcOps::now_secs();

    if let Some(proof) = cached_internal_invocation_proof(&request, &cfg, root_pid, now_secs) {
        return Ok(proof);
    }

    fresh_internal_invocation_proof_for_request_with_context(request, cfg, root_pid, now_secs).await
}

async fn fresh_internal_invocation_proof_for_request(
    request: InternalInvocationProofRequest,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let now_secs = IcOps::now_secs();
    fresh_internal_invocation_proof_for_request_with_context(request, cfg, root_pid, now_secs).await
}

async fn fresh_internal_invocation_proof_for_request_with_context(
    request: InternalInvocationProofRequest,
    cfg: RoleAttestationConfig,
    root_pid: Principal,
    now_secs: u64,
) -> Result<SignedInternalInvocationProofV1, Error> {
    let proof =
        crate::api::auth::AuthApi::request_internal_invocation_proof(request.clone()).await?;
    cache_internal_invocation_proof(&request, &cfg, root_pid, now_secs, proof.clone());
    Ok(proof)
}

fn internal_invocation_proof_cache_key(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
) -> InternalInvocationProofCacheKey {
    InternalInvocationProofCacheKey {
        root_pid,
        attestation_key_name: cfg.ecdsa_key_name.clone(),
        subject: request.subject,
        role: request.role.clone(),
        subnet_id: request.subnet_id,
        audience: request.audience,
        audience_method: request.audience_method.clone(),
        ttl_secs: request.ttl_secs,
    }
}

fn cached_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
    now_secs: u64,
) -> Option<SignedInternalInvocationProofV1> {
    let key = internal_invocation_proof_cache_key(request, cfg, root_pid);
    let min_accepted_epoch = cfg
        .min_accepted_epoch_by_role
        .get(request.role.as_str())
        .copied()
        .unwrap_or(0);

    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        let proof = cache.get(&key)?;
        if internal_invocation_proof_is_reusable(proof, request, now_secs, min_accepted_epoch) {
            Some(proof.clone())
        } else {
            cache.remove(&key);
            None
        }
    })
}

fn cache_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
    cfg: &RoleAttestationConfig,
    root_pid: Principal,
    now_secs: u64,
    proof: SignedInternalInvocationProofV1,
) {
    let min_accepted_epoch = cfg
        .min_accepted_epoch_by_role
        .get(request.role.as_str())
        .copied()
        .unwrap_or(0);
    if !internal_invocation_proof_is_reusable(&proof, request, now_secs, min_accepted_epoch) {
        return;
    }

    let key = internal_invocation_proof_cache_key(request, cfg, root_pid);
    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        cache.insert(key, proof);
    });
}

fn invalidate_internal_invocation_proof(
    request: &InternalInvocationProofRequest,
) -> Result<(), Error> {
    let cfg = ConfigOps::role_attestation_config().map_err(Error::from)?;
    let root_pid = EnvOps::root_pid().map_err(Error::from)?;
    let key = internal_invocation_proof_cache_key(request, &cfg, root_pid);
    INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(|cache| {
        cache.remove(&key);
    });
    Ok(())
}

fn internal_invocation_proof_is_reusable(
    proof: &SignedInternalInvocationProofV1,
    request: &InternalInvocationProofRequest,
    now_secs: u64,
    min_accepted_epoch: u64,
) -> bool {
    let payload = &proof.payload;
    if payload.expires_at <= payload.issued_at || now_secs < payload.issued_at {
        return false;
    }

    payload.subject == request.subject
        && payload.role == request.role
        && payload.subnet_id == request.subnet_id
        && payload.audience == request.audience
        && payload.audience_method == request.audience_method
        && payload.epoch >= min_accepted_epoch
        && now_secs.saturating_add(internal_invocation_proof_refresh_margin_secs(proof))
            < payload.expires_at
}

fn internal_invocation_proof_refresh_margin_secs(proof: &SignedInternalInvocationProofV1) -> u64 {
    proof
        .payload
        .expires_at
        .saturating_sub(proof.payload.issued_at)
        .saturating_div(5)
        .clamp(1, INTERNAL_CALL_PROOF_REFRESH_MARGIN_MAX_SECS)
}

fn internal_call_result_is_retryable(result: &CallResult) -> bool {
    let Ok(Err(err)) = result.candid::<Result<candid::Reserved, Error>>() else {
        return false;
    };
    internal_call_error_is_retryable(&err)
}

const fn internal_call_error_is_retryable(err: &Error) -> bool {
    matches!(
        err.code,
        ErrorCode::AuthKeyUnknown | ErrorCode::AuthMaterialStale
    )
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

fn encode_internal_call_envelope_raw(
    envelope: CanicInternalCallEnvelopeV1,
) -> Result<Vec<u8>, Error> {
    encode_one(envelope).map_err(|err| Error::invalid(err.to_string()))
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
    use crate::config::schema::RoleAttestationConfig;
    use crate::dto::auth::{InternalInvocationProofPayloadV1, SignedInternalInvocationProofV1};
    use candid::{decode_args, decode_one};
    use std::collections::BTreeMap;

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

    fn request() -> InternalInvocationProofRequest {
        InternalInvocationProofRequest {
            subject: p(1),
            role: CanisterRole::new("project_hub"),
            subnet_id: Some(p(9)),
            audience: p(2),
            audience_method: "system_add_project_to_user".to_string(),
            ttl_secs: 120,
            metadata: None,
        }
    }

    fn cfg(min_epoch: u64) -> RoleAttestationConfig {
        let mut min_accepted_epoch_by_role = BTreeMap::new();
        min_accepted_epoch_by_role.insert("project_hub".to_string(), min_epoch);
        RoleAttestationConfig {
            ecdsa_key_name: "key_1".to_string(),
            max_ttl_secs: 900,
            min_accepted_epoch_by_role,
        }
    }

    fn clear_internal_invocation_proof_cache() {
        INTERNAL_INVOCATION_PROOF_CACHE.with_borrow_mut(BTreeMap::clear);
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
    fn canic_call_encodes_envelope_as_raw_ingress_bytes() {
        let args = encode_args((7_u64, "project")).expect("args encode");
        let envelope =
            build_internal_call_envelope(p(2), "system_add_project_to_user", proof(), args);
        let raw =
            encode_internal_call_envelope_raw(envelope.clone()).expect("envelope should encode");

        let decoded: CanicInternalCallEnvelopeV1 =
            decode_one(&raw).expect("raw ingress bytes decode as envelope");

        assert_eq!(decoded, envelope);
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

    #[test]
    fn canic_call_rejects_empty_target_method_locally() {
        let err = validate_internal_call_target_method("   ")
            .expect_err("empty protected call method should fail locally");

        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn canic_call_rejects_empty_caller_role_locally() {
        let err = validate_internal_call_caller_role(&CanisterRole::new("   "))
            .expect_err("empty protected call role should fail locally");

        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn canic_call_rejects_zero_effective_proof_ttl_locally() {
        let zero_requested = effective_internal_call_proof_ttl_secs(0, 900)
            .expect_err("zero requested proof ttl should fail locally");
        assert_eq!(zero_requested.code, ErrorCode::InvalidInput);

        let zero_max = effective_internal_call_proof_ttl_secs(120, 0)
            .expect_err("zero configured max proof ttl should fail locally");
        assert_eq!(zero_max.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn canic_call_clamps_requested_proof_ttl_to_config_max() {
        assert_eq!(
            effective_internal_call_proof_ttl_secs(120, 900).expect("ttl"),
            120
        );
        assert_eq!(
            effective_internal_call_proof_ttl_secs(1200, 900).expect("ttl"),
            900
        );
    }

    #[test]
    fn protected_internal_endpoint_descriptor_matches_roles() {
        let endpoint = ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [
                CanisterRole::new("project_hub"),
                CanisterRole::new("admin_hub"),
            ],
        );

        assert_eq!(endpoint.method(), "system_add_project_to_user");
        assert!(endpoint.accepts_role(&CanisterRole::new("project_hub")));
        assert!(endpoint.accepts_role(&CanisterRole::new("admin_hub")));
        assert!(!endpoint.accepts_role(&CanisterRole::new("user_hub")));
        assert!(endpoint.single_role().is_none());
    }

    #[test]
    fn protected_internal_endpoint_single_role_is_available_to_generated_clients() {
        let endpoint = ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [CanisterRole::new("project_hub")],
        );

        assert_eq!(
            endpoint.single_role(),
            Some(&CanisterRole::new("project_hub"))
        );
        assert_eq!(
            endpoint.required_single_role().expect("single role"),
            CanisterRole::new("project_hub")
        );
    }

    #[test]
    fn protected_internal_endpoint_requires_explicit_role_when_ambiguous() {
        let endpoint = ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [
                CanisterRole::new("project_hub"),
                CanisterRole::new("admin_hub"),
            ],
        );

        let err = endpoint
            .required_single_role()
            .expect_err("multi-role endpoint should require explicit caller role");
        assert_eq!(err.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_missing_method() {
        let result =
            std::panic::catch_unwind(|| ProtectedInternalEndpoint::new("", [CanisterRole::ROOT]));

        assert!(result.is_err());
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_blank_method() {
        let result = std::panic::catch_unwind(|| {
            ProtectedInternalEndpoint::new("   ", [CanisterRole::ROOT])
        });

        assert!(result.is_err());
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_missing_roles() {
        let result = std::panic::catch_unwind(|| {
            ProtectedInternalEndpoint::new("system_add_project_to_user", [])
        });

        assert!(result.is_err());
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_empty_role() {
        let result = std::panic::catch_unwind(|| {
            ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("")])
        });

        assert!(result.is_err());
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_blank_role() {
        let result = std::panic::catch_unwind(|| {
            ProtectedInternalEndpoint::new("system_add_project_to_user", [CanisterRole::new("   ")])
        });

        assert!(result.is_err());
    }

    #[test]
    fn protected_internal_endpoint_descriptor_rejects_duplicate_roles() {
        let result = std::panic::catch_unwind(|| {
            ProtectedInternalEndpoint::new(
                "system_add_project_to_user",
                [
                    CanisterRole::new("project_hub"),
                    CanisterRole::new("project_hub"),
                ],
            )
        });

        assert!(result.is_err());
    }

    #[test]
    fn internal_client_options_are_chainable() {
        let client = CanicInternalClient::new(p(3))
            .with_bounded_wait()
            .with_cycles(10)
            .with_proof_ttl_secs(30);

        assert_eq!(client.canister_id, p(3));
        assert_eq!(client.options.wait, CanicInternalWaitMode::Bounded);
        assert_eq!(client.options.cycles, 10);
        assert_eq!(client.options.proof_ttl_secs, Some(30));
    }

    #[test]
    fn internal_client_rejects_unaccepted_explicit_role_locally() {
        let client = CanicInternalClient::new(p(3));
        let endpoint = ProtectedInternalEndpoint::new(
            "system_add_project_to_user",
            [CanisterRole::new("project_hub")],
        );
        let result = futures::executor::block_on(client.call_update(
            &endpoint,
            CanisterRole::new("admin_hub"),
            (),
        ));

        match result {
            Err(err) => assert_eq!(err.code, ErrorCode::InvalidInput),
            Ok(_) => panic!("unaccepted caller role should fail before transport"),
        }
    }

    #[test]
    fn internal_invocation_proof_cache_reuses_exact_fresh_edge() {
        clear_internal_invocation_proof_cache();
        let request = request();
        let mut proof = proof();
        proof.payload.subnet_id = request.subnet_id;
        cache_internal_invocation_proof(&request, &cfg(0), p(7), 12, proof.clone());

        let cached = cached_internal_invocation_proof(&request, &cfg(0), p(7), 12)
            .expect("fresh matching proof should cache-hit");

        assert_eq!(cached, proof);
    }

    #[test]
    fn internal_invocation_proof_cache_rejects_near_expiry_entry() {
        clear_internal_invocation_proof_cache();
        let request = request();
        let mut proof = proof();
        proof.payload.subnet_id = request.subnet_id;
        proof.payload.issued_at = 10;
        proof.payload.expires_at = 20;
        cache_internal_invocation_proof(&request, &cfg(0), p(7), 18, proof);

        assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 18).is_none());
    }

    #[test]
    fn internal_invocation_proof_cache_rejects_future_issued_at_entry() {
        clear_internal_invocation_proof_cache();
        let request = request();
        let mut proof = proof();
        proof.payload.subnet_id = request.subnet_id;
        proof.payload.issued_at = 20;
        proof.payload.expires_at = 40;
        cache_internal_invocation_proof(&request, &cfg(0), p(7), 12, proof);

        assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 12).is_none());
    }

    #[test]
    fn internal_invocation_proof_cache_rejects_invalid_time_window() {
        clear_internal_invocation_proof_cache();
        let request = request();
        let mut proof = proof();
        proof.payload.subnet_id = request.subnet_id;
        proof.payload.issued_at = 20;
        proof.payload.expires_at = 20;
        cache_internal_invocation_proof(&request, &cfg(0), p(7), 20, proof);

        assert!(cached_internal_invocation_proof(&request, &cfg(0), p(7), 20).is_none());
    }

    #[test]
    fn internal_invocation_proof_cache_rejects_epoch_below_local_floor() {
        clear_internal_invocation_proof_cache();
        let request = request();
        let mut proof = proof();
        proof.payload.subnet_id = request.subnet_id;
        proof.payload.epoch = 3;
        cache_internal_invocation_proof(&request, &cfg(0), p(7), 12, proof);

        assert!(cached_internal_invocation_proof(&request, &cfg(4), p(7), 12).is_none());
    }

    #[test]
    fn internal_call_retry_classifier_is_limited_to_repairable_auth_material() {
        assert!(internal_call_error_is_retryable(&Error::new(
            ErrorCode::AuthKeyUnknown,
            "unknown key".to_string(),
        )));
        assert!(internal_call_error_is_retryable(&Error::new(
            ErrorCode::AuthMaterialStale,
            "stale epoch".to_string(),
        )));
        assert!(!internal_call_error_is_retryable(&Error::new(
            ErrorCode::AuthProofExpired,
            "expired".to_string(),
        )));
        assert!(!internal_call_error_is_retryable(&Error::unauthorized(
            "role mismatch"
        )));
    }
}
