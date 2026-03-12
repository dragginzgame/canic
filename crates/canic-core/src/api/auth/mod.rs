use crate::{
    access::auth::validate_delegated_session_subject,
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenClaims, DelegationCert,
            DelegationProof, DelegationProvisionResponse, DelegationProvisionTargetKind,
            DelegationRequest, RoleAttestationRequest, SignedRoleAttestation,
        },
        error::{Error, ErrorCode},
        rpc::{Request as RootRequest, Response as RootCapabilityResponse},
    },
    error::InternalErrorClass,
    log,
    log::Topic,
    ops::{
        auth::DelegatedTokenOps,
        config::ConfigOps,
        ic::IcOps,
        rpc::RpcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            record_attestation_refresh_failed, record_signer_issue_without_proof,
        },
        storage::auth::{DelegatedSession, DelegationStateOps},
    },
    protocol,
    workflow::rpc::request::handler::RootResponseWorkflow,
};

mod metadata;
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

    async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(claims, proof)
            .await
            .map_err(Self::map_delegation_error)
    }

    /// Issue a delegated token using a reusable local proof when possible.
    ///
    /// If the proof is missing or no longer valid for the requested claims, this
    /// performs canonical shard-initiated setup and retries with the refreshed proof.
    pub async fn issue_token(claims: DelegatedTokenClaims) -> Result<DelegatedToken, Error> {
        let proof = Self::ensure_signing_proof(&claims).await?;
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
            .map(|verified| (verified.claims, verified.cert))
            .map_err(Self::map_delegation_error)
    }

    /// Canonical shard-initiated delegation request (user_shard -> root).
    ///
    /// Caller must match shard_pid and be registered to the subnet.
    pub async fn request_delegation(
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

    pub async fn request_role_attestation(
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

    pub async fn attestation_key_set() -> Result<AttestationKeySet, Error> {
        DelegatedTokenOps::attestation_key_set()
            .await
            .map_err(Self::map_delegation_error)
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

    /// Persist a temporary delegated session subject for the caller wallet.
    pub fn set_delegated_session_subject(
        delegated_subject: Principal,
        bootstrap_token: DelegatedToken,
        requested_ttl_secs: Option<u64>,
    ) -> Result<(), Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let wallet_caller = IcOps::msg_caller();
        if let Err(reason) = validate_delegated_session_subject(wallet_caller) {
            return Err(Error::forbidden(format!(
                "delegated session wallet caller rejected: {reason}"
            )));
        }

        if let Err(reason) = validate_delegated_session_subject(delegated_subject) {
            return Err(Error::forbidden(format!(
                "delegated session subject rejected: {reason}"
            )));
        }

        let issued_at = IcOps::now_secs();
        let authority_pid = EnvOps::root_pid().map_err(Error::from)?;
        let self_pid = IcOps::canister_self();
        let verified =
            DelegatedTokenOps::verify_token(&bootstrap_token, authority_pid, issued_at, self_pid)
                .map_err(Self::map_delegation_error)?;

        if verified.claims.sub != delegated_subject {
            return Err(Error::forbidden(format!(
                "delegated session subject mismatch: requested={} token_subject={}",
                delegated_subject, verified.claims.sub
            )));
        }

        let configured_max_ttl_secs = cfg
            .max_ttl_secs
            .unwrap_or(Self::MAX_DELEGATED_SESSION_TTL_SECS);
        let expires_at = Self::clamp_delegated_session_expires_at(
            issued_at,
            verified.claims.exp,
            configured_max_ttl_secs,
            requested_ttl_secs,
        )?;

        DelegationStateOps::upsert_delegated_session(
            DelegatedSession {
                wallet_pid: wallet_caller,
                delegated_pid: delegated_subject,
                issued_at,
                expires_at,
            },
            issued_at,
        );

        Ok(())
    }

    /// Remove the caller's delegated session subject.
    pub fn clear_delegated_session() {
        let wallet_caller = IcOps::msg_caller();
        DelegationStateOps::clear_delegated_session(wallet_caller);
    }

    /// Read the caller's active delegated session subject, if configured.
    #[must_use]
    pub fn delegated_session_subject() -> Option<Principal> {
        let wallet_caller = IcOps::msg_caller();
        DelegationStateOps::delegated_session_subject(wallet_caller, IcOps::now_secs())
    }

    /// Prune all currently expired delegated sessions.
    #[must_use]
    pub fn prune_expired_delegated_sessions() -> usize {
        DelegationStateOps::prune_expired_delegated_sessions(IcOps::now_secs())
    }

    /// Compatibility helper for the legacy delegated-caller API.
    pub fn set_delegated_caller(
        delegated_caller: Principal,
        bootstrap_token: DelegatedToken,
        requested_ttl_secs: Option<u64>,
    ) -> Result<(), Error> {
        Self::set_delegated_session_subject(delegated_caller, bootstrap_token, requested_ttl_secs)
    }

    /// Compatibility helper for the legacy delegated-caller API.
    pub fn clear_delegated_caller() {
        Self::clear_delegated_session();
    }

    /// Compatibility helper for the legacy delegated-caller API.
    #[must_use]
    pub fn delegated_caller() -> Option<Principal> {
        Self::delegated_session_subject()
    }

    pub async fn store_proof(
        proof: DelegationProof,
        kind: DelegationProvisionTargetKind,
    ) -> Result<(), Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        if caller != root_pid {
            return Err(Error::forbidden(
                "delegation proof store requires root caller",
            ));
        }

        DelegatedTokenOps::cache_public_keys_for_cert(&proof.cert)
            .await
            .map_err(Self::map_delegation_error)?;
        if let Err(err) = DelegatedTokenOps::verify_delegation_proof(&proof, root_pid) {
            let local = IcOps::canister_self();
            log!(
                Topic::Auth,
                Warn,
                "delegation proof rejected kind={:?} local={} shard={} issued_at={} expires_at={} error={}",
                kind,
                local,
                proof.cert.shard_pid,
                proof.cert.issued_at,
                proof.cert.expires_at,
                err
            );
            return Err(Self::map_delegation_error(err));
        }

        DelegationStateOps::set_proof_from_dto(proof);
        let local = IcOps::canister_self();
        let stored = DelegationStateOps::proof_dto()
            .ok_or_else(|| Error::invariant("delegation proof missing after store"))?;
        log!(
            Topic::Auth,
            Info,
            "delegation proof stored kind={:?} local={} shard={} issued_at={} expires_at={}",
            kind,
            local,
            stored.cert.shard_pid,
            stored.cert.issued_at,
            stored.cert.expires_at
        );

        Ok(())
    }

    /// Install delegation proof and key material directly, bypassing management-key lookups.
    ///
    /// This is intended for controlled root-driven test flows where deterministic
    /// key material is used instead of chain-key ECDSA.
    pub fn install_test_delegation_material(
        proof: DelegationProof,
        root_public_key: Vec<u8>,
        shard_public_key: Vec<u8>,
    ) -> Result<(), Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        let caller = IcOps::msg_caller();
        if caller != root_pid {
            return Err(Error::forbidden(
                "test delegation material install requires root caller",
            ));
        }

        if proof.cert.root_pid != root_pid {
            return Err(Error::invalid(format!(
                "delegation proof root mismatch: expected={} found={}",
                root_pid, proof.cert.root_pid
            )));
        }

        if root_public_key.is_empty() || shard_public_key.is_empty() {
            return Err(Error::invalid("delegation public keys must not be empty"));
        }

        DelegationStateOps::set_root_public_key(root_public_key);
        DelegationStateOps::set_shard_public_key(proof.cert.shard_pid, shard_public_key);
        DelegationStateOps::set_proof_from_dto(proof);
        Ok(())
    }

    fn require_proof() -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        DelegationStateOps::proof_dto().ok_or_else(|| {
            record_signer_issue_without_proof();
            Error::not_found("delegation proof not set")
        })
    }

    // Resolve a proof that is currently usable for token issuance.
    async fn ensure_signing_proof(claims: &DelegatedTokenClaims) -> Result<DelegationProof, Error> {
        let now_secs = IcOps::now_secs();

        match Self::require_proof() {
            Ok(proof) if !Self::proof_is_reusable_for_claims(&proof, claims, now_secs) => {
                Self::setup_delegation(claims).await
            }
            Ok(proof) => Ok(proof),
            Err(err) if err.code == ErrorCode::NotFound => Self::setup_delegation(claims).await,
            Err(err) => Err(err),
        }
    }

    // Provision a fresh delegation from root, then load locally stored proof.
    async fn setup_delegation(claims: &DelegatedTokenClaims) -> Result<DelegationProof, Error> {
        let request = Self::delegation_request_from_claims(claims)?;
        let _ = Self::request_delegation(request).await?;
        Self::require_proof()
    }

    // Build a canonical delegation request from token claims.
    fn delegation_request_from_claims(
        claims: &DelegatedTokenClaims,
    ) -> Result<DelegationRequest, Error> {
        let ttl_secs = claims.exp.saturating_sub(claims.iat);
        if ttl_secs == 0 {
            return Err(Error::invalid(
                "delegation ttl_secs must be greater than zero",
            ));
        }

        Ok(DelegationRequest {
            shard_pid: IcOps::canister_self(),
            scopes: claims.scopes.clone(),
            aud: claims.aud.clone(),
            ttl_secs,
            verifier_targets: Vec::new(),
            include_root_verifier: true,
            metadata: None,
        })
    }

    // Check whether a proof can be reused safely for the requested claims.
    fn proof_is_reusable_for_claims(
        proof: &DelegationProof,
        claims: &DelegatedTokenClaims,
        now_secs: u64,
    ) -> bool {
        if now_secs > proof.cert.expires_at {
            return false;
        }

        if claims.shard_pid != proof.cert.shard_pid {
            return false;
        }

        if claims.iat < proof.cert.issued_at || claims.exp > proof.cert.expires_at {
            return false;
        }

        Self::is_principal_subset(&claims.aud, &proof.cert.aud)
            && Self::is_string_subset(&claims.scopes, &proof.cert.scopes)
    }

    // Return true when every principal in `subset` is present in `superset`.
    fn is_principal_subset(
        subset: &[crate::cdk::types::Principal],
        superset: &[crate::cdk::types::Principal],
    ) -> bool {
        subset.iter().all(|item| superset.contains(item))
    }

    // Return true when every scope in `subset` is present in `superset`.
    fn is_string_subset(subset: &[String], superset: &[String]) -> bool {
        subset.iter().all(|item| superset.contains(item))
    }

    fn clamp_delegated_session_expires_at(
        now_secs: u64,
        token_expires_at: u64,
        configured_max_ttl_secs: u64,
        requested_ttl_secs: Option<u64>,
    ) -> Result<u64, Error> {
        if configured_max_ttl_secs == 0 {
            return Err(Error::invariant(
                "delegated session configured max ttl_secs must be greater than zero",
            ));
        }

        if let Some(ttl_secs) = requested_ttl_secs
            && ttl_secs == 0
        {
            return Err(Error::invalid(
                "delegated session requested ttl_secs must be greater than zero",
            ));
        }

        let mut expires_at = token_expires_at;
        expires_at = expires_at.min(now_secs.saturating_add(configured_max_ttl_secs));
        if let Some(ttl_secs) = requested_ttl_secs {
            expires_at = expires_at.min(now_secs.saturating_add(ttl_secs));
        }

        if expires_at <= now_secs {
            return Err(Error::forbidden(
                "delegated session bootstrap token is expired",
            ));
        }

        Ok(expires_at)
    }
}

#[cfg(test)]
mod tests;
