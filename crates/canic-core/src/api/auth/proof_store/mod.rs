use super::DelegationApi;
use crate::{
    cdk::types::Principal,
    dto::{
        auth::{DelegationProof, DelegationProofInstallRequest, DelegationProvisionTargetKind},
        error::Error,
    },
    log,
    log::Topic,
    ops::{
        auth::{DelegatedTokenOps, audience},
        config::ConfigOps,
        ic::IcOps,
        runtime::env::EnvOps,
        runtime::metrics::auth::{
            DelegationInstallNormalizationRejectReason, DelegationInstallValidationFailureReason,
            VerifierProofCacheEvictionClass, record_delegation_install_normalization_rejected,
            record_delegation_install_validation_failed, record_verifier_proof_cache_eviction,
            record_verifier_proof_cache_stats,
        },
        storage::auth::DelegationStateOps,
    },
};

impl DelegationApi {
    pub async fn store_proof(
        request: DelegationProofInstallRequest,
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

        let proof = request.proof;
        let intent = request.intent;

        if kind == DelegationProvisionTargetKind::Verifier {
            let local = IcOps::canister_self();
            Self::ensure_target_in_proof_audience(
                &proof,
                local,
                intent,
                AudienceBindingFailureStage::PostNormalization,
            )?;
        }

        DelegatedTokenOps::cache_public_keys_for_cert(&proof.cert)
            .await
            .map_err(Self::map_delegation_error)?;
        if let Err(err) = DelegatedTokenOps::verify_delegation_proof(&proof, root_pid) {
            let local = IcOps::canister_self();
            log!(
                Topic::Auth,
                Warn,
                "delegation proof rejected intent={:?} kind={:?} local={} shard={} issued_at={} expires_at={} error={}",
                intent,
                kind,
                local,
                proof.cert.shard_pid,
                proof.cert.issued_at,
                proof.cert.expires_at,
                err
            );
            return Err(Self::map_delegation_error(err));
        }

        let outcome = DelegationStateOps::upsert_proof_from_dto(proof.clone(), IcOps::now_secs())
            .map_err(Self::map_delegation_error)?;
        if kind == DelegationProvisionTargetKind::Verifier {
            Self::record_verifier_cache_install_outcome(outcome);
        }
        let local = IcOps::canister_self();
        log!(
            Topic::Auth,
            Info,
            "delegation proof stored intent={:?} kind={:?} local={} shard={} issued_at={} expires_at={}",
            intent,
            kind,
            local,
            proof.cert.shard_pid,
            proof.cert.issued_at,
            proof.cert.expires_at
        );

        Ok(())
    }

    /// Install delegation proof and key material directly, bypassing management-key lookups.
    ///
    /// This is intended for controlled root-driven test flows where deterministic
    /// key material is used instead of chain-key ECDSA.
    // Compiled only for controlled test canister builds.
    #[cfg(canic_test_delegation_material)]
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
        let outcome = DelegationStateOps::upsert_proof_from_dto(proof, IcOps::now_secs())
            .map_err(Self::map_delegation_error)?;
        Self::record_verifier_cache_install_outcome(outcome);
        Ok(())
    }

    // Enforce verifier-target audience binding for all explicit/admin install paths.
    pub(super) fn ensure_target_in_proof_audience(
        proof: &DelegationProof,
        target: Principal,
        intent: crate::dto::auth::DelegationProofInstallIntent,
        stage: AudienceBindingFailureStage,
    ) -> Result<(), Error> {
        if audience::principal_allowed(target, &proof.cert.aud) {
            return Ok(());
        }

        match stage {
            AudienceBindingFailureStage::Normalization => {
                record_delegation_install_normalization_rejected(
                    intent,
                    DelegationInstallNormalizationRejectReason::TargetNotInAudience,
                );
            }
            AudienceBindingFailureStage::PostNormalization => {
                record_delegation_install_validation_failed(
                    intent,
                    DelegationInstallValidationFailureReason::TargetNotInAudience,
                );
            }
        }

        Err(Error::invalid(format!(
            "delegation verifier target '{target}' is not in proof audience"
        )))
    }

    // Record verifier-cache occupancy/utilization and any eviction caused by install.
    fn record_verifier_cache_install_outcome(
        outcome: crate::ops::storage::auth::DelegationProofUpsertOutcome,
    ) {
        record_verifier_proof_cache_stats(
            outcome.stats.size,
            outcome.stats.active_count,
            outcome.stats.capacity,
            outcome.stats.profile,
            outcome.stats.active_window_secs,
        );

        if let Some(class) = outcome.evicted {
            let class = match class {
                crate::ops::storage::auth::DelegationProofEvictionClass::Cold => {
                    VerifierProofCacheEvictionClass::Cold
                }
                crate::ops::storage::auth::DelegationProofEvictionClass::Active => {
                    VerifierProofCacheEvictionClass::Active
                }
            };
            record_verifier_proof_cache_eviction(class);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AudienceBindingFailureStage {
    Normalization,
    PostNormalization,
}
