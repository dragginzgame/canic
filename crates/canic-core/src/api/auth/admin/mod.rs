use super::{DelegationApi, proof_store::AudienceBindingFailureStage};
use crate::{
    cdk::types::Principal,
    dto::{
        auth::{
            DelegationAdminCommand, DelegationAdminResponse, DelegationProof,
            DelegationVerifierProofPushRequest,
        },
        error::Error,
    },
    ops::{
        config::ConfigOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            DelegationInstallNormalizationRejectReason, DelegationInstallValidationFailureReason,
            record_delegation_install_fanout_bucket,
            record_delegation_install_normalization_rejected,
            record_delegation_install_normalized_target_count, record_delegation_install_total,
            record_delegation_install_validation_failed,
        },
        storage::{
            auth::DelegationStateOps, directory::subnet::SubnetDirectoryOps,
            registry::subnet::SubnetRegistryOps,
        },
    },
    workflow::auth::DelegationWorkflow,
};

///
/// PreparedDelegationVerifierPush
///

struct PreparedDelegationVerifierPush {
    proof: DelegationProof,
    verifier_targets: Vec<Principal>,
    intent: crate::dto::auth::DelegationProofInstallIntent,
}

impl PreparedDelegationVerifierPush {
    // Convert a validated admin push plan into the workflow command shape.
    fn into_command(self) -> DelegationAdminCommand {
        let request = DelegationVerifierProofPushRequest {
            proof: self.proof,
            verifier_targets: self.verifier_targets,
        };
        match self.intent {
            crate::dto::auth::DelegationProofInstallIntent::Prewarm => {
                DelegationAdminCommand::PrewarmVerifiers(request)
            }
            crate::dto::auth::DelegationProofInstallIntent::Repair => {
                DelegationAdminCommand::RepairVerifiers(request)
            }
            crate::dto::auth::DelegationProofInstallIntent::Provisioning => {
                unreachable!("provisioning does not use explicit admin push")
            }
        }
    }
}

impl DelegationApi {
    /// Execute explicit root-controlled delegation repair/prewarm operations.
    pub async fn admin(cmd: DelegationAdminCommand) -> Result<DelegationAdminResponse, Error> {
        let cfg = ConfigOps::delegated_tokens_config().map_err(Error::from)?;
        if !cfg.enabled {
            return Err(Error::forbidden(Self::DELEGATED_TOKENS_DISABLED));
        }
        if !EnvOps::is_root() {
            return Err(Error::forbidden("delegation admin requires root canister"));
        }

        let prepared = match cmd {
            DelegationAdminCommand::PrewarmVerifiers(request) => {
                record_delegation_install_total(
                    crate::dto::auth::DelegationProofInstallIntent::Prewarm,
                );
                Self::prepare_explicit_verifier_push(
                    request,
                    crate::dto::auth::DelegationProofInstallIntent::Prewarm,
                )
                .await?
            }
            DelegationAdminCommand::RepairVerifiers(request) => {
                record_delegation_install_total(
                    crate::dto::auth::DelegationProofInstallIntent::Repair,
                );
                Self::prepare_explicit_verifier_push(
                    request,
                    crate::dto::auth::DelegationProofInstallIntent::Repair,
                )
                .await?
            }
        };

        DelegationWorkflow::handle_admin(prepared.into_command())
            .await
            .map_err(Self::map_delegation_error)
    }

    // Normalize and verify an explicit verifier-push request before workflow fanout.
    async fn prepare_explicit_verifier_push(
        request: DelegationVerifierProofPushRequest,
        intent: crate::dto::auth::DelegationProofInstallIntent,
    ) -> Result<PreparedDelegationVerifierPush, Error> {
        let request = Self::normalize_explicit_verifier_push_request_with(
            request,
            intent,
            EnvOps::root_pid().map_err(Error::from)?,
            Self::is_registered_canister,
        )?;
        record_delegation_install_normalized_target_count(intent, request.verifier_targets.len());
        record_delegation_install_fanout_bucket(intent, request.verifier_targets.len());
        Self::prepare_explicit_verifier_push_proof(&request.proof, intent).await?;

        Ok(PreparedDelegationVerifierPush {
            proof: request.proof,
            verifier_targets: request.verifier_targets,
            intent,
        })
    }

    // Normalize explicit verifier push targets with root/signer/registration guards.
    pub(super) fn normalize_explicit_verifier_push_request_with<F>(
        request: DelegationVerifierProofPushRequest,
        intent: crate::dto::auth::DelegationProofInstallIntent,
        root_pid: Principal,
        mut is_valid_target: F,
    ) -> Result<DelegationVerifierProofPushRequest, Error>
    where
        F: FnMut(Principal) -> bool,
    {
        let signer_pid = request.proof.cert.shard_pid;
        let mut verifier_targets = Vec::new();

        for principal in request.verifier_targets {
            if principal == signer_pid {
                record_delegation_install_normalization_rejected(
                    intent,
                    DelegationInstallNormalizationRejectReason::SignerTarget,
                );
                return Err(Error::invalid(
                    "delegation verifier target must not match signer shard",
                ));
            }
            if principal == root_pid {
                record_delegation_install_normalization_rejected(
                    intent,
                    DelegationInstallNormalizationRejectReason::RootTarget,
                );
                return Err(Error::invalid(
                    "delegation verifier target must not match root canister",
                ));
            }
            if !is_valid_target(principal) {
                record_delegation_install_normalization_rejected(
                    intent,
                    DelegationInstallNormalizationRejectReason::UnregisteredTarget,
                );
                return Err(Error::invalid(format!(
                    "delegation verifier target '{principal}' is not registered"
                )));
            }
            if !verifier_targets.contains(&principal) {
                verifier_targets.push(principal);
            }
        }

        for principal in &verifier_targets {
            Self::ensure_target_in_proof_audience(
                &request.proof,
                *principal,
                intent,
                AudienceBindingFailureStage::Normalization,
            )?;
        }

        Ok(DelegationVerifierProofPushRequest {
            proof: request.proof,
            verifier_targets,
        })
    }

    // Validate/caches proof dependencies once before explicit fanout.
    async fn prepare_explicit_verifier_push_proof(
        proof: &DelegationProof,
        intent: crate::dto::auth::DelegationProofInstallIntent,
    ) -> Result<(), Error> {
        let root_pid = EnvOps::root_pid().map_err(Error::from)?;
        crate::ops::auth::DelegatedTokenOps::cache_public_keys_for_cert(&proof.cert)
            .await
            .map_err(|err| {
                record_delegation_install_validation_failed(
                    intent,
                    DelegationInstallValidationFailureReason::CacheKeys,
                );
                Self::map_delegation_error(err)
            })?;
        Self::verify_delegation_proof(proof, root_pid).inspect_err(|_| {
            record_delegation_install_validation_failed(
                intent,
                DelegationInstallValidationFailureReason::VerifyProof,
            );
        })?;

        if intent == crate::dto::auth::DelegationProofInstallIntent::Repair {
            Self::ensure_repair_push_proof_is_locally_available(proof)?;
        }

        Ok(())
    }

    // Enforce repair as redistribution of already-installed proof state only.
    fn ensure_repair_push_proof_is_locally_available(proof: &DelegationProof) -> Result<(), Error> {
        Self::ensure_repair_push_proof_is_locally_available_with(proof, |candidate| {
            Ok(DelegationStateOps::matching_proof_dto(candidate))
        })
    }

    // Check repair preconditions using an injectable lookup for unit tests.
    pub(super) fn ensure_repair_push_proof_is_locally_available_with<F>(
        proof: &DelegationProof,
        lookup: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&DelegationProof) -> Result<Option<DelegationProof>, Error>,
    {
        let Some(stored) = lookup(proof)? else {
            record_delegation_install_validation_failed(
                crate::dto::auth::DelegationProofInstallIntent::Repair,
                DelegationInstallValidationFailureReason::RepairMissingLocal,
            );
            return Err(Error::not_found(
                "delegation repair requires an existing local proof",
            ));
        };

        if stored != *proof {
            record_delegation_install_validation_failed(
                crate::dto::auth::DelegationProofInstallIntent::Repair,
                DelegationInstallValidationFailureReason::RepairLocalMismatch,
            );
            return Err(Error::invalid(
                "delegation repair proof must match the existing local proof",
            ));
        }

        Ok(())
    }

    // Return true when a principal is a provisionable verifier canister target.
    pub(super) fn is_registered_canister(principal: Principal) -> bool {
        if SubnetRegistryOps::is_registered(principal) {
            return true;
        }

        SubnetDirectoryOps::data()
            .entries
            .iter()
            .any(|(_, pid)| *pid == principal)
    }
}
