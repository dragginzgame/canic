use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenClaims, DelegationAudience,
            DelegationCert, DelegationProof, DelegationProvisionResponse,
            DelegationProvisionStatus, DelegationProvisionTargetKind, DelegationRequest,
            RoleAttestationRequest, SignedRoleAttestation,
        },
        error::{Error, ErrorCode},
        rpc::{Request as RootRequest, Response as RootCapabilityResponse},
    },
    error::InternalErrorClass,
    ids::cap,
    log,
    log::Topic,
    ops::{
        auth::{DelegatedTokenOps, audience},
        config::ConfigOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_refresh_failed, record_delegation_provision_complete,
            record_delegation_verifier_target_count, record_delegation_verifier_target_failed,
            record_delegation_verifier_target_missing, record_signer_issue_without_proof,
        },
        storage::auth::DelegationStateOps,
    },
    protocol,
    workflow::rpc::request::handler::RootResponseWorkflow,
};

#[cfg(test)]
use crate::ids::CanisterRole;

// Internal auth pipeline:
// - `session` owns delegated-session ingress and replay/session state handling.
// - `admin` owns explicit root-driven fanout preparation and routing.
// - `proof_store` owns proof-install validation and storage/cache side effects.
//
// Keep these modules free of lateral calls to each other. Coordination stays here,
// and shared invariants should live in dedicated seams like `ops::auth::audience`.
mod admin;
mod metadata;
mod proof_store;
mod session;
mod verify_flow;

///
/// DelegationApi
///
/// Requires auth.delegated_tokens.enabled = true in config.
///

pub struct DelegationApi;

impl DelegationApi {
    const DELEGATED_TOKENS_DISABLED: &str =
        "delegated token auth disabled; set auth.delegated_tokens.enabled=true in canic.toml";
    const MAX_DELEGATED_SESSION_TTL_SECS: u64 = 24 * 60 * 60;
    const SESSION_BOOTSTRAP_TOKEN_FINGERPRINT_DOMAIN: &[u8] =
        b"canic-session-bootstrap-token-fingerprint:v1";

    fn map_delegation_error(err: crate::InternalError) -> Error {
        match err.class() {
            InternalErrorClass::Infra | InternalErrorClass::Ops | InternalErrorClass::Workflow => {
                Error::internal(err.to_string())
            }
            _ => Error::from(err),
        }
    }

    /// Full delegation proof verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_delegation_proof(
        proof: &DelegationProof,
        authority_pid: Principal,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_delegation_proof(proof, authority_pid)
            .map_err(Self::map_delegation_error)
    }

    #[cfg(canic_test_delegation_material)]
    #[must_use]
    pub fn current_signing_proof_for_test() -> Option<DelegationProof> {
        DelegationStateOps::latest_proof_dto()
    }

    /// Return whether this canister currently has a local signing proof.
    #[must_use]
    pub fn has_signing_proof() -> bool {
        DelegationStateOps::latest_proof_dto().is_some()
    }

    async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(claims, proof)
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Resolve the local shard public key in SEC1 encoding.
    pub async fn local_shard_public_key_sec1() -> Result<Vec<u8>, Error> {
        DelegatedTokenOps::local_shard_public_key_sec1(IcOps::canister_self())
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Issue a delegated token using a reusable local proof when possible.
    ///
    /// If the proof is missing or no longer valid for the requested claims, this
    /// performs canonical shard-initiated setup and retries with the refreshed proof.
    pub async fn issue_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
        let proof = Self::ensure_signing_proof(&claims).await?;
        let claims = Self::canonicalize_claims_for_proof(claims, &proof);
        Self::sign_token(claims, proof).await
    }

    /// Full delegated token verification (structure + signature).
    ///
    /// Purely local verification; does not read certified data or require a
    /// query context.
    pub fn verify_token(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs, IcOps::canister_self())
            .map(|_| ())
            .map_err(Self::map_delegation_error)
    }

    /// Verify a delegated token and return verified contents.
    ///
    /// This is intended for application-layer session construction.
    /// It performs full verification and returns verified claims and cert.
    pub fn verify_token_verified(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(DelegatedTokenClaims, DelegationCert), Error> {
        DelegatedTokenOps::verify_token(token, authority_pid, now_secs, IcOps::canister_self())
            .map(crate::ops::auth::VerifiedDelegatedToken::into_parts)
            .map_err(Self::map_delegation_error)
    }

    /// Verify a delegated token and require its subject to match `msg_caller()`.
    ///
    /// This issuer-side helper does not require the old token audience to
    /// include the local signer, which allows stale-audience reissue flows.
    pub fn verify_token_for_caller(
        token: &DelegatedToken,
        authority_pid: Principal,
        now_secs: u64,
    ) -> Result<(DelegatedTokenClaims, DelegationCert), Error> {
        let verified = DelegatedTokenOps::verify_token_for_reissue(token, authority_pid, now_secs)
            .map_err(Self::map_delegation_error)?;
        Self::ensure_claims_bound_to_caller(&verified.claims.to_dto(), IcOps::msg_caller())?;
        Ok(verified.into_parts())
    }

    /// Reissue a caller-bound token for a new audience without extending expiry.
    ///
    /// Scopes and `ext` are preserved. The replacement expiry is capped at the
    /// old token expiry, so this refreshes audience only and does not renew the
    /// session.
    pub async fn reissue_token(
        token: DelegatedToken,
        aud: DelegationAudience,
    ) -> Result<DelegatedToken, Error> {
        let aud = Self::normalize_audience(aud)?;
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let now_secs = IcOps::now_secs();
        let (old_claims, _) = Self::verify_token_for_caller(&token, root_pid, now_secs)?;
        let replacement_claims = DelegatedTokenClaims {
            aud,
            iat: now_secs,
            ..old_claims.clone()
        };

        Self::reissue_token_from_verified(old_claims, replacement_claims).await
    }

    /// Ensure the caller has a valid delegated token for the requested audience.
    ///
    /// With no token, this mints a default `verify`-scoped token for
    /// `msg_caller()`. With a caller-bound token, this returns it unchanged when
    /// it already covers the audience or reissues it without extending expiry.
    pub async fn ensure_token(
        token: Option<DelegatedToken>,
        aud: DelegationAudience,
    ) -> Result<DelegatedToken, Error> {
        let requested_aud = Self::normalize_audience(aud)?;
        match token {
            Some(token) => Self::ensure_existing_token_for_audience(token, requested_aud).await,
            None => Self::issue_token_for_caller_audience(requested_aud).await,
        }
    }

    /// Reissue a token from previously verified claims and proposed claims.
    ///
    /// CANIC enforces same `sub`, same `shard_pid`, no expiry extension, and a
    /// default scope-subset rule.
    pub async fn reissue_token_from_verified(
        old_claims: DelegatedTokenClaims,
        replacement_claims: DelegatedTokenClaims,
    ) -> Result<DelegatedToken, Error> {
        Self::ensure_reissue_claims_allowed(&old_claims, &replacement_claims)?;
        let proof = Self::ensure_signing_proof(&replacement_claims).await?;
        let replacement_claims = Self::canonicalize_reissue_claims_for_proof(
            replacement_claims,
            &proof,
            old_claims.exp,
        )?;
        Self::sign_token(replacement_claims, proof).await
    }

    // Return an existing caller-bound token or reissue it to cover missing audience entries.
    async fn ensure_existing_token_for_audience(
        token: DelegatedToken,
        requested_aud: DelegationAudience,
    ) -> Result<DelegatedToken, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let now_secs = IcOps::now_secs();
        let (old_claims, _) = Self::verify_token_for_caller(&token, root_pid, now_secs)?;
        if audience::roles_subset(&requested_aud, &old_claims.aud) {
            return Ok(token);
        }

        let aud = Self::merge_audience_for_reissue(old_claims.aud.clone(), requested_aud);
        let replacement_claims = DelegatedTokenClaims {
            aud,
            iat: now_secs,
            ..old_claims.clone()
        };

        Self::reissue_token_from_verified(old_claims, replacement_claims).await
    }

    // Issue the initial caller-bound token for an authenticated wallet/session principal.
    async fn issue_token_for_caller_audience(
        aud: DelegationAudience,
    ) -> Result<DelegatedToken, Error> {
        let caller = IcOps::msg_caller();
        if let Err(reason) = crate::access::auth::validate_delegated_session_subject(caller) {
            return Err(Error::forbidden(format!(
                "delegated token caller rejected: {reason}"
            )));
        }

        let now_secs = IcOps::now_secs();
        let ttl_secs = ConfigOps::delegated_tokens_config()
            .map_err(Error::from)?
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS);
        let claims = DelegatedTokenClaims {
            sub: caller,
            shard_pid: IcOps::canister_self(),
            scopes: vec![cap::VERIFY.to_string()],
            aud,
            iat: now_secs,
            exp: now_secs.saturating_add(ttl_secs),
            ext: None,
        };

        Self::issue_token(claims).await
    }

    /// Canonical shard-initiated delegation request (user_shard -> root).
    ///
    /// Caller must match shard_pid and be registered to the subnet.
    pub async fn request_delegation(
        request: DelegationRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let request = metadata::with_root_request_metadata(request);
        Self::request_delegation_remote(request).await
    }

    pub async fn request_role_attestation(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        let response = Self::request_role_attestation_remote(request).await?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    pub async fn attestation_key_set() -> Result<AttestationKeySet, Error> {
        DelegatedTokenOps::attestation_key_set()
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Warm the root delegation and attestation key caches once.
    pub async fn prewarm_root_key_material() -> Result<(), Error> {
        EnvOps::require_root().map_err(Error::from)?;
        DelegatedTokenOps::prewarm_root_key_material()
            .await
            .map_err(|err| {
                log!(Topic::Auth, Warn, "root auth key prewarm failed: {err}");
                Self::map_delegation_error(err)
            })
    }

    pub fn replace_attestation_key_set(key_set: AttestationKeySet) {
        DelegatedTokenOps::replace_attestation_key_set(key_set);
    }

    pub async fn verify_role_attestation(
        attestation: &SignedRoleAttestation,
        min_accepted_epoch: u64,
    ) -> Result<(), Error> {
        let configured_min_accepted_epoch = ConfigOps::role_attestation_config()
            .map_err(Error::from)?
            .min_accepted_epoch_by_role
            .get(attestation.payload.role.as_str())
            .copied();
        let min_accepted_epoch = verify_flow::resolve_min_accepted_epoch(
            min_accepted_epoch,
            configured_min_accepted_epoch,
        );

        let caller = IcOps::msg_caller();
        let self_pid = IcOps::canister_self();
        let now_secs = IcOps::now_secs();
        let verifier_subnet = Some(EnvOps::subnet_pid().map_err(Error::from)?);
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;

        let verify = || {
            DelegatedTokenOps::verify_role_attestation_cached(
                attestation,
                caller,
                self_pid,
                verifier_subnet,
                now_secs,
                min_accepted_epoch,
            )
            .map(|_| ())
        };
        let refresh = || async {
            let key_set: AttestationKeySet =
                RpcOps::call_rpc_result(root_pid, protocol::CANIC_ATTESTATION_KEY_SET, ()).await?;
            DelegatedTokenOps::replace_attestation_key_set(key_set);
            Ok(())
        };

        match verify_flow::verify_role_attestation_with_single_refresh(verify, refresh).await {
            Ok(()) => Ok(()),
            Err(verify_flow::RoleAttestationVerifyFlowError::Initial(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                verify_flow::log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "cached",
                );
                Err(Self::map_delegation_error(err.into()))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::Refresh { trigger, source }) => {
                verify_flow::record_attestation_verifier_rejection(&trigger);
                verify_flow::log_attestation_verifier_rejection(
                    &trigger,
                    attestation,
                    caller,
                    self_pid,
                    "cache_miss_refresh",
                );
                record_attestation_refresh_failed();
                log!(
                    Topic::Auth,
                    Warn,
                    "role attestation refresh failed local={} caller={} key_id={} error={}",
                    self_pid,
                    caller,
                    attestation.key_id,
                    source
                );
                Err(Self::map_delegation_error(source))
            }
            Err(verify_flow::RoleAttestationVerifyFlowError::PostRefresh(err)) => {
                verify_flow::record_attestation_verifier_rejection(&err);
                verify_flow::log_attestation_verifier_rejection(
                    &err,
                    attestation,
                    caller,
                    self_pid,
                    "post_refresh",
                );
                Err(Self::map_delegation_error(err.into()))
            }
        }
    }

    fn require_proof() -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        DelegationStateOps::latest_proof_dto().ok_or_else(|| {
            record_signer_issue_without_proof();
            Error::not_found("delegation proof not installed")
        })
    }

    // Resolve a proof that is currently usable for token issuance.
    async fn ensure_signing_proof(claims: &DelegatedTokenClaims) -> Result<DelegationProof, Error> {
        let now_secs = IcOps::now_secs();

        match Self::require_proof() {
            Ok(proof)
                if !DelegatedTokenOps::proof_reusable_for_claims(&proof, claims, now_secs) =>
            {
                Self::setup_delegation(claims).await
            }
            Ok(proof) => Ok(proof),
            Err(err) if err.code == ErrorCode::NotFound => Self::setup_delegation(claims).await,
            Err(err) => Err(err),
        }
    }

    // Provision a fresh delegation from root, then resolve the latest locally stored proof.
    async fn setup_delegation(claims: &DelegatedTokenClaims) -> Result<DelegationProof, Error> {
        let shard_public_key_sec1 =
            DelegatedTokenOps::local_shard_public_key_sec1(claims.shard_pid)
                .await
                .map_err(Self::map_delegation_error)?;
        let request = Self::delegation_request_from_claims(claims, shard_public_key_sec1)?;
        let required_verifier_targets = request.verifier_targets.clone();
        let response = Self::request_delegation_remote(request).await?;
        Self::ensure_required_verifier_targets_provisioned(&required_verifier_targets, &response)?;
        let proof = response.proof;
        Self::store_local_signer_proof(proof.clone()).await?;
        Ok(proof)
    }

    // Rebase claims onto a freshly issued proof window when delegation setup
    // completed after the original token timestamps were chosen.
    fn canonicalize_claims_for_proof(
        claims: DelegatedTokenClaims,
        proof: &DelegationProof,
    ) -> DelegatedTokenClaims {
        if claims.iat >= proof.cert.issued_at && claims.exp <= proof.cert.expires_at {
            return claims;
        }

        DelegatedTokenClaims {
            iat: proof.cert.issued_at,
            exp: proof.cert.expires_at,
            ..claims
        }
    }

    // Bind verified token claims to the current IC caller.
    fn ensure_claims_bound_to_caller(
        claims: &DelegatedTokenClaims,
        caller: Principal,
    ) -> Result<(), Error> {
        if claims.sub == caller {
            Ok(())
        } else {
            Err(Error::forbidden(format!(
                "delegated token subject '{}' does not match caller '{}'",
                claims.sub, caller
            )))
        }
    }

    // Normalize caller-supplied audience roles with set semantics.
    fn normalize_audience(audience: DelegationAudience) -> Result<DelegationAudience, Error> {
        let DelegationAudience::Roles(roles) = audience else {
            return Ok(DelegationAudience::Any);
        };

        let mut out = Vec::new();
        for role in roles {
            if !out.contains(&role) {
                out.push(role);
            }
        }

        if out.is_empty() {
            return Err(Error::invalid("token audience role list must not be empty"));
        }

        Ok(DelegationAudience::Roles(out))
    }

    // Merge role-scoped audiences while preserving wildcard broadening semantics.
    fn merge_audience_for_reissue(
        current: DelegationAudience,
        requested: DelegationAudience,
    ) -> DelegationAudience {
        match (current, requested) {
            (DelegationAudience::Any, _) | (_, DelegationAudience::Any) => DelegationAudience::Any,
            (DelegationAudience::Roles(mut current), DelegationAudience::Roles(requested)) => {
                for role in requested {
                    if !current.contains(&role) {
                        current.push(role);
                    }
                }
                DelegationAudience::Roles(current)
            }
        }
    }

    // Enforce same-session reissue invariants before resolving signing material.
    fn ensure_reissue_claims_allowed(
        old_claims: &DelegatedTokenClaims,
        replacement_claims: &DelegatedTokenClaims,
    ) -> Result<(), Error> {
        if audience::has_empty_roles(&replacement_claims.aud) {
            return Err(Error::invalid(
                "replacement token audience role list must not be empty",
            ));
        }

        if replacement_claims.sub != old_claims.sub {
            return Err(Error::forbidden(format!(
                "replacement token subject '{}' must match old subject '{}'",
                replacement_claims.sub, old_claims.sub
            )));
        }

        if replacement_claims.shard_pid != old_claims.shard_pid {
            return Err(Error::forbidden(format!(
                "replacement token shard '{}' must match old shard '{}'",
                replacement_claims.shard_pid, old_claims.shard_pid
            )));
        }

        if replacement_claims.exp > old_claims.exp {
            return Err(Error::forbidden(
                "replacement token expiry must not exceed old token expiry",
            ));
        }

        if replacement_claims.exp < replacement_claims.iat {
            return Err(Error::invalid(
                "replacement token expiry must not precede issued_at",
            ));
        }

        if !audience::strings_subset(&replacement_claims.scopes, &old_claims.scopes) {
            return Err(Error::forbidden(
                "replacement token scopes must be a subset of old token scopes",
            ));
        }

        Ok(())
    }

    // Rebase reissue timing onto the resolved proof while preserving the old-expiry cap.
    fn canonicalize_reissue_claims_for_proof(
        claims: DelegatedTokenClaims,
        proof: &DelegationProof,
        old_exp: u64,
    ) -> Result<DelegatedTokenClaims, Error> {
        let iat = claims.iat.max(proof.cert.issued_at);
        let exp = claims.exp.min(old_exp).min(proof.cert.expires_at);

        if exp < iat {
            return Err(Error::invalid(
                "replacement token expiry is outside the current signing proof window",
            ));
        }

        Ok(DelegatedTokenClaims { iat, exp, ..claims })
    }

    // Build a canonical delegation request from token claims.
    fn delegation_request_from_claims(
        claims: &DelegatedTokenClaims,
        shard_public_key_sec1: Vec<u8>,
    ) -> Result<DelegationRequest, Error> {
        let ttl_secs = claims.exp.saturating_sub(claims.iat);
        if ttl_secs == 0 {
            return Err(Error::invalid(
                "delegation ttl_secs must be greater than zero",
            ));
        }

        let signer_pid = IcOps::canister_self();
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let verifier_targets = DelegatedTokenOps::required_verifier_targets_from_audience(
            &claims.aud,
            signer_pid,
            root_pid,
        )
        .map_err(|role| {
            Error::invalid(format!(
                "delegation audience role '{role}' is invalid for canonical verifier provisioning"
            ))
        })?;

        Ok(DelegationRequest {
            shard_pid: signer_pid,
            scopes: claims.scopes.clone(),
            aud: claims.aud.clone(),
            ttl_secs,
            verifier_targets,
            include_root_verifier: true,
            shard_public_key_sec1,
            metadata: None,
        })
    }

    // Validate required verifier fanout and fail closed when any required target is missing/failing.
    fn ensure_required_verifier_targets_provisioned(
        required_targets: &[Principal],
        response: &DelegationProvisionResponse,
    ) -> Result<(), Error> {
        let mut checked = Vec::new();
        for target in required_targets {
            if checked.contains(target) {
                continue;
            }
            checked.push(*target);
        }
        record_delegation_verifier_target_count(checked.len());

        for target in &checked {
            let Some(result) = response.results.iter().find(|entry| {
                entry.kind == DelegationProvisionTargetKind::Verifier && entry.target == *target
            }) else {
                record_delegation_verifier_target_missing();
                return Err(Error::internal(format!(
                    "delegation provisioning missing verifier target result for '{target}'"
                )));
            };

            if result.status != DelegationProvisionStatus::Ok {
                record_delegation_verifier_target_failed();
                let detail = result
                    .error
                    .as_ref()
                    .map_or_else(|| "unknown error".to_string(), ToString::to_string);
                return Err(Error::internal(format!(
                    "delegation provisioning failed for required verifier target '{target}': {detail}"
                )));
            }
        }

        record_delegation_provision_complete();
        Ok(())
    }

    // Derive required verifier targets from audience with strict filtering/validation.
    #[cfg(test)]
    fn derive_required_verifier_targets_from_aud(
        audience: &DelegationAudience,
        signer_pid: Principal,
        root_pid: Principal,
        mut resolve_role: impl FnMut(&CanisterRole) -> Result<Vec<Principal>, ()>,
    ) -> Result<Vec<Principal>, Error> {
        let mut verifier_targets = Vec::new();
        let DelegationAudience::Roles(roles) = audience else {
            return Ok(verifier_targets);
        };
        if roles.is_empty() {
            return Err(Error::invalid(
                "delegation audience role list must not be empty",
            ));
        }

        for role in roles {
            let pids = resolve_role(role).map_err(|()| {
                Error::invalid(format!(
                    "delegation audience role '{role}' is invalid for canonical verifier provisioning"
                ))
            })?;
            for pid in pids {
                if pid == signer_pid || pid == root_pid || verifier_targets.contains(&pid) {
                    continue;
                }
                verifier_targets.push(pid);
            }
        }
        Ok(verifier_targets)
    }

    // Delegated audience invariants:
    // 1. Some(empty) audiences are invalid; None means any registered verifier.
    // 2. claims.aud must stay within proof.cert.aud.
    // 3. proof installation on target T requires T's role to be allowed by proof.cert.aud.
    // 4. token acceptance on canister C requires C's role to be allowed by claims.aud.
    //
    // Keep ingress, fanout, install, and runtime checks aligned to this block.
}

impl DelegationApi {
    // Execute one local root delegation provisioning request.
    pub async fn request_delegation_root(
        request: DelegationRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let request = metadata::with_root_request_metadata(request);
        let response = RootResponseWorkflow::response(RootRequest::issue_delegation(request))
            .await
            .map_err(Self::map_delegation_error)?;

        match response {
            RootCapabilityResponse::DelegationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for delegation request",
            )),
        }
    }

    // Route a canonical delegation provisioning request over RPC to root.
    async fn request_delegation_remote(
        request: DelegationRequest,
    ) -> Result<DelegationProvisionResponse, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RpcOps::call_rpc_result(root_pid, protocol::CANIC_REQUEST_DELEGATION, request)
            .await
            .map_err(Self::map_delegation_error)
    }

    // Execute one local root role-attestation request.
    pub async fn request_role_attestation_root(
        request: RoleAttestationRequest,
    ) -> Result<SignedRoleAttestation, Error> {
        let request = metadata::with_root_attestation_request_metadata(request);
        let response = RootResponseWorkflow::response(RootRequest::issue_role_attestation(request))
            .await
            .map_err(Self::map_delegation_error)?;

        match response {
            RootCapabilityResponse::RoleAttestationIssued(response) => Ok(response),
            _ => Err(Error::internal(
                "invalid root response type for role attestation request",
            )),
        }
    }

    // Route a canonical role-attestation request over RPC to root.
    async fn request_role_attestation_remote(
        request: RoleAttestationRequest,
    ) -> Result<RootCapabilityResponse, Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        RpcOps::call_rpc_result(root_pid, protocol::CANIC_REQUEST_ROLE_ATTESTATION, request)
            .await
            .map_err(Self::map_delegation_error)
    }
}

#[cfg(test)]
mod tests;
