use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            AttestationKeySet, DelegatedToken, DelegatedTokenClaims, DelegationCert,
            DelegationProof, DelegationProvisionResponse, DelegationProvisionTargetKind,
            DelegationRequest, RoleAttestationRequest, SignedRoleAttestation,
        },
        error::Error,
        rpc::{Request as RootCapabilityRequest, Response as RootCapabilityResponse},
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
            record_attestation_refresh_failed, record_signer_mint_without_proof,
        },
        storage::auth::DelegationStateOps,
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

    pub async fn sign_token(
        claims: DelegatedTokenClaims,
        proof: DelegationProof,
    ) -> Result<DelegatedToken, Error> {
        DelegatedTokenOps::sign_token(claims, proof)
            .await
            .map_err(Self::map_delegation_error)
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
        let response =
            RootResponseWorkflow::response(RootCapabilityRequest::IssueDelegation(request))
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
        let response =
            RootResponseWorkflow::response(RootCapabilityRequest::IssueRoleAttestation(request))
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

    pub fn require_proof() -> Result<DelegationProof, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }

        DelegationStateOps::proof_dto().ok_or_else(|| {
            record_signer_mint_without_proof();
            Error::not_found("delegation proof not set")
        })
    }
}

#[cfg(test)]
mod tests;
