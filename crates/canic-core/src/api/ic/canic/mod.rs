//! Protected Canic-to-Canic internal call API.
//!
//! This module owns the native Canic internal RPC path: method-scoped root
//! proofs, protected endpoint descriptors, generated-client transport options,
//! envelope construction, proof caching, and the narrow auth-material repair
//! retry.

mod endpoint;
mod envelope;
mod proof_cache;

pub use endpoint::ProtectedInternalEndpoint;

use super::call::{Call, CallResult};
use crate::{
    cdk::{
        candid::{CandidType, encode_one},
        types::Principal,
    },
    dto::{
        auth::{CanicInternalCallEnvelopeV1, InternalInvocationProofRequest},
        error::{Error, ErrorCode},
    },
    ids::CanisterRole,
    ops::{config::ConfigOps, ic::IcOps, runtime::env::EnvOps},
};
use candid::{encode_args, utils::ArgumentEncoder};
use envelope::{build_internal_call_envelope, encode_internal_call_envelope_raw};
use proof_cache::{
    fresh_internal_invocation_proof_for_request, internal_invocation_proof_for_request,
    invalidate_internal_invocation_proof,
};
use serde::de::DeserializeOwned;
use std::borrow::Cow;

const DEFAULT_INTERNAL_CALL_PROOF_TTL_SECS: u64 = 120;

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
                "caller role '{caller_role}' is not accepted by protected internal endpoint '{}'; accepted caller roles: [{}]. Use the generated endpoint descriptor with call_update(..., accepted_role, args).",
                endpoint.method(),
                endpoint.accepted_roles_label()
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

#[cfg(test)]
mod tests;
