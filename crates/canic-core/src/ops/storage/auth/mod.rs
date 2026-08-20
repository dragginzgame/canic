//! Module: ops::storage::auth
//!
//! Responsibility: provide the authorized access path to persisted auth state.
//! Does not own: access policy, auth verification, or stable auth schemas.
//! Boundary: storage ops facade between auth/access logic and stable auth records.
//!
//! This is a security-sensitive boundary for local application sessions and
//! role-attestation keys. Callers should use this facade instead of depending
//! on stable storage implementation details.

pub mod application_sessions;
pub mod mapper;

use crate::{
    cdk::types::Principal,
    dto::auth::{
        ActiveDelegationProof, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessV1,
        ChainKeyDelegationCertV1, ChainKeyRootSignatureV1, DelegationCert,
    },
    model::auth::{RootIssuerPolicy, RootIssuerRenewalState, RootIssuerRenewalTemplate},
    ops::storage::auth::mapper::{
        ActiveDelegationProofRecordMapper, ChainKeyRootDelegationBatchRecordMapper,
        RootIssuerPolicyRecordMapper, RootIssuerRenewalStateRecordMapper,
        RootIssuerRenewalTemplateRecordMapper,
    },
    storage::stable::auth::AuthState,
};

///
/// ChainKeyRootDelegationBatchStatus
///
/// Persisted lifecycle state for one root-signed chain-key delegation batch.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainKeyRootDelegationBatchStatus {
    Prepared,
    Signing,
    Signed,
    Installing,
    Installed,
    FailedRetryable,
}

///
/// ChainKeyRootDelegationBatchIssuer
///
/// Per-issuer proof material carried by one chain-key root delegation batch.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainKeyRootDelegationBatchIssuer {
    pub issuer_pid: Principal,
    pub cert_hash: [u8; 32],
    pub delegation_cert: DelegationCert,
    pub chain_key_delegation_cert: ChainKeyDelegationCertV1,
    pub issuer_witness: ChainKeyBatchWitnessV1,
    pub refresh_after_ns: u64,
    pub installed_at_ns: Option<u64>,
    pub last_failure: Option<String>,
}

///
/// ChainKeyRootDelegationBatch
///
/// Root-owned persisted batch state for bridge-free chain-key renewal.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainKeyRootDelegationBatch {
    pub batch_id: [u8; 32],
    pub status: ChainKeyRootDelegationBatchStatus,
    pub header_hash: [u8; 32],
    pub header: ChainKeyBatchHeaderV1,
    pub signature: Option<ChainKeyRootSignatureV1>,
    pub issuers: Vec<ChainKeyRootDelegationBatchIssuer>,
    pub prepared_at_ns: u64,
    pub signed_at_ns: Option<u64>,
    pub install_started_at_ns: Option<u64>,
    pub installed_at_ns: Option<u64>,
    pub retry_after_ns: Option<u64>,
    pub failure: Option<String>,
}

///
/// AuthStateOps
///
/// Storage-ops facade for local application sessions and auth issuer state.
///

pub struct AuthStateOps;

impl AuthStateOps {
    #[must_use]
    pub fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
        let proof = AuthState::get_active_delegation_proof()
            .map(ActiveDelegationProofRecordMapper::record_to_dto)?;
        if now_ns < proof.not_before_ns || now_ns >= proof.expires_at_ns {
            return None;
        }
        Some(proof)
    }

    #[must_use]
    pub fn active_delegation_proof_snapshot() -> Option<ActiveDelegationProof> {
        AuthState::get_active_delegation_proof()
            .map(ActiveDelegationProofRecordMapper::record_to_dto)
    }

    pub fn set_active_delegation_proof(proof: ActiveDelegationProof) {
        AuthState::set_active_delegation_proof(ActiveDelegationProofRecordMapper::dto_to_record(
            proof,
        ));
    }

    #[cfg(test)]
    pub fn clear_active_delegation_proof() {
        AuthState::clear_active_delegation_proof();
    }

    #[must_use]
    pub fn root_issuer_policy(issuer_pid: Principal) -> Option<RootIssuerPolicy> {
        AuthState::get_root_issuer(issuer_pid).map(RootIssuerPolicyRecordMapper::record_to_policy)
    }

    #[must_use]
    pub fn root_issuer_policies() -> Vec<RootIssuerPolicy> {
        AuthState::list_root_issuers()
            .into_iter()
            .map(RootIssuerPolicyRecordMapper::record_to_policy)
            .collect()
    }

    pub fn upsert_root_issuer_policy(policy: RootIssuerPolicy) {
        AuthState::upsert_root_issuer(RootIssuerPolicyRecordMapper::policy_to_record(policy));
    }

    #[must_use]
    pub fn delegated_auth_registry_epoch() -> u64 {
        AuthState::delegated_auth_registry_epoch()
    }

    pub fn advance_delegated_auth_registry_epoch() -> u64 {
        AuthState::advance_delegated_auth_registry_epoch()
    }

    pub fn advance_delegated_auth_registry_epoch_at_least(min_epoch: u64) -> u64 {
        AuthState::advance_delegated_auth_registry_epoch_at_least(min_epoch)
    }

    #[must_use]
    #[cfg(test)]
    pub fn delegated_auth_proof_epoch() -> u64 {
        AuthState::delegated_auth_proof_epoch()
    }

    pub fn advance_delegated_auth_proof_epoch_at_least(min_epoch: u64) -> u64 {
        AuthState::advance_delegated_auth_proof_epoch_at_least(min_epoch)
    }

    #[must_use]
    pub fn root_issuer_renewal_template(
        issuer_pid: Principal,
    ) -> Option<RootIssuerRenewalTemplate> {
        AuthState::get_root_issuer_renewal_template(issuer_pid)
            .map(RootIssuerRenewalTemplateRecordMapper::record_to_template)
    }

    #[must_use]
    pub fn root_issuer_renewal_templates() -> Vec<RootIssuerRenewalTemplate> {
        AuthState::list_root_issuer_renewal_templates()
            .into_iter()
            .map(RootIssuerRenewalTemplateRecordMapper::record_to_template)
            .collect()
    }

    pub fn upsert_root_issuer_renewal_template(template: RootIssuerRenewalTemplate) {
        AuthState::upsert_root_issuer_renewal_template(
            RootIssuerRenewalTemplateRecordMapper::template_to_record(template),
        );
    }

    #[must_use]
    pub fn root_issuer_renewal_state(issuer_pid: Principal) -> Option<RootIssuerRenewalState> {
        AuthState::get_root_issuer_renewal_state(issuer_pid)
            .map(RootIssuerRenewalStateRecordMapper::record_to_state)
    }

    pub fn upsert_root_issuer_renewal_state(state: RootIssuerRenewalState) {
        AuthState::upsert_root_issuer_renewal_state(
            RootIssuerRenewalStateRecordMapper::state_to_record(state),
        );
    }

    #[must_use]
    pub fn chain_key_root_delegation_batch(
        batch_id: [u8; 32],
    ) -> Option<ChainKeyRootDelegationBatch> {
        AuthState::get_chain_key_root_delegation_batch(batch_id)
            .map(ChainKeyRootDelegationBatchRecordMapper::record_to_batch)
    }

    #[must_use]
    pub fn chain_key_root_delegation_batches() -> Vec<ChainKeyRootDelegationBatch> {
        AuthState::list_chain_key_root_delegation_batches()
            .into_iter()
            .map(ChainKeyRootDelegationBatchRecordMapper::record_to_batch)
            .collect()
    }

    pub fn upsert_chain_key_root_delegation_batch(batch: ChainKeyRootDelegationBatch) {
        AuthState::upsert_chain_key_root_delegation_batch(
            ChainKeyRootDelegationBatchRecordMapper::batch_to_record(batch),
        );
    }

    pub fn prune_chain_key_root_delegation_batches(now_ns: u64) -> usize {
        AuthState::prune_chain_key_root_delegation_batches(now_ns)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            ChainKeyAlgorithm, ChainKeyBatchHeaderV1, ChainKeyBatchWitnessStepV1,
            ChainKeyBatchWitnessV1, ChainKeyDelegationCertV1, ChainKeyKeyId,
            ChainKeyRootSignatureV1, DelegatedRoleGrant, DelegationAudience, DelegationCert,
            DelegationProof, IcChainKeyBatchSignatureProofV1, IssuerProofAlgorithm,
            IssuerProofBinding, RootProof,
        },
        ids::CanisterRole,
        model::auth::{
            RootDelegatedRoleGrantPolicy, RootIssuerPolicy, RootIssuerRenewalState,
            RootIssuerRenewalTemplate,
        },
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn active_proof() -> ActiveDelegationProof {
        let issuer_proof_alg = IssuerProofAlgorithm::IcCanisterSignatureV1;
        let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 { seed_hash: [5; 32] };

        ActiveDelegationProof {
            proof: DelegationProof {
                cert: DelegationCert {
                    root_pid: p(1),
                    issuer_pid: p(2),
                    issuer_proof_alg,
                    issuer_proof_binding_hash: [6; 32],
                    issuer_proof_binding,
                    issued_at_ns: 10,
                    not_before_ns: 20,
                    expires_at_ns: 100,
                    max_token_ttl_ns: 30,
                    aud: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
                    grants: vec![DelegatedRoleGrant {
                        target: CanisterRole::owned("project_instance".to_string()),
                        scopes: vec!["read".to_string(), "write".to_string()],
                    }],
                },
                root_proof: RootProof::IcChainKeyBatchSignatureV1(chain_key_root_proof(p(1), p(2))),
            },
            cert_hash: [10; 32],
            not_before_ns: 20,
            expires_at_ns: 100,
            refresh_after_ns: 80,
            installed_at_ns: 15,
            installed_by: p(11),
        }
    }

    fn chain_key_root_proof(
        root_canister_id: Principal,
        issuer_canister_id: Principal,
    ) -> IcChainKeyBatchSignatureProofV1 {
        let key_id = ChainKeyKeyId {
            name: "test_key_1".to_string(),
        };

        IcChainKeyBatchSignatureProofV1 {
            header: ChainKeyBatchHeaderV1 {
                schema_version: 1,
                root_canister_id,
                batch_id: [31; 32],
                proof_epoch: 2,
                registry_epoch: 3,
                registry_hash: [32; 32],
                tree_root: [33; 32],
                not_before_ns: 20,
                expires_at_ns: 100,
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id: key_id.clone(),
                derivation_path_hash: [34; 32],
                key_version: 4,
            },
            delegation_cert: ChainKeyDelegationCertV1 {
                root_canister_id,
                issuer_canister_id,
                proof_epoch: 2,
                issuer_proof_algorithm: IssuerProofAlgorithm::IcCanisterSignatureV1,
                issuer_proof_binding_hash: [35; 32],
                issuer_proof_binding: IssuerProofBinding::IcCanisterSignatureV1 {
                    seed_hash: [36; 32],
                },
                max_token_ttl_ns: 30,
                audience: DelegationAudience::Fleet(crate::test::support::fleet_key(1)),
                grants: vec![DelegatedRoleGrant {
                    target: CanisterRole::owned("project_instance".to_string()),
                    scopes: vec!["read".to_string(), "write".to_string()],
                }],
                not_before_ns: 20,
                expires_at_ns: 100,
                registry_epoch: 3,
                registry_hash: [32; 32],
            },
            issuer_witness: ChainKeyBatchWitnessV1 {
                steps: vec![
                    ChainKeyBatchWitnessStepV1::LeftSibling([37; 32]),
                    ChainKeyBatchWitnessStepV1::RightSibling([38; 32]),
                ],
            },
            signature: ChainKeyRootSignatureV1 {
                algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
                key_id,
                derivation_path: vec![b"canic".to_vec(), b"delegation".to_vec()],
                public_key: vec![39; 33],
                signature: vec![40; 64],
            },
        }
    }

    #[test]
    fn active_delegation_proof_round_trips_and_filters_by_time() {
        AuthStateOps::clear_active_delegation_proof();
        let proof = active_proof();

        AuthStateOps::set_active_delegation_proof(proof.clone());

        assert_eq!(AuthStateOps::active_delegation_proof(19), None);
        assert_eq!(AuthStateOps::active_delegation_proof(20), Some(proof));
        assert!(AuthStateOps::active_delegation_proof(99).is_some());
        assert_eq!(AuthStateOps::active_delegation_proof(100), None);

        AuthStateOps::clear_active_delegation_proof();
        assert_eq!(AuthStateOps::active_delegation_proof(20), None);
    }

    #[test]
    fn delegated_auth_proof_epoch_advances_monotonically_from_minimum() {
        let before = AuthStateOps::delegated_auth_proof_epoch();
        let minimum = before.saturating_add(5);

        let first = AuthStateOps::advance_delegated_auth_proof_epoch_at_least(minimum);
        let second = AuthStateOps::advance_delegated_auth_proof_epoch_at_least(1);

        assert_eq!(first, minimum);
        assert_eq!(second, first.saturating_add(1));
        assert_eq!(AuthStateOps::delegated_auth_proof_epoch(), second);
    }

    #[test]
    fn delegated_auth_registry_epoch_advances_monotonically_to_floor() {
        let before = AuthStateOps::delegated_auth_registry_epoch();
        let minimum = before.saturating_add(5);

        let first = AuthStateOps::advance_delegated_auth_registry_epoch_at_least(minimum);
        let second = AuthStateOps::advance_delegated_auth_registry_epoch_at_least(1);

        assert_eq!(first, minimum);
        assert_eq!(second, first);
        assert_eq!(AuthStateOps::delegated_auth_registry_epoch(), second);
    }

    #[test]
    fn root_issuer_policy_round_trips_through_auth_state() {
        let policy = RootIssuerPolicy {
            issuer_pid: p(31),
            enabled: true,
            allowed_audiences: vec![crate::test::support::fleet_key(1)],
            allowed_grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["canic.issue".to_string(), "canic.read".to_string()],
            }],
            max_cert_ttl_ns: 120_000_000_000,
            refresh_after_ratio_bps: 8_000,
        };

        AuthStateOps::upsert_root_issuer_policy(policy.clone());

        assert_eq!(AuthStateOps::root_issuer_policy(p(31)), Some(policy));
        assert_eq!(AuthStateOps::root_issuer_policy(p(34)), None);
    }

    #[test]
    fn root_issuer_renewal_template_round_trips_through_auth_state() {
        let template = RootIssuerRenewalTemplate {
            issuer_pid: p(41),
            enabled: true,
            audience: crate::test::support::fleet_key(1),
            grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["canic.read".to_string()],
            }],
            cert_ttl_ns: 120_000_000_000,
        };

        AuthStateOps::upsert_root_issuer_renewal_template(template.clone());

        assert_eq!(
            AuthStateOps::root_issuer_renewal_template(p(41)),
            Some(template)
        );
        assert_eq!(AuthStateOps::root_issuer_renewal_template(p(42)), None);
    }

    #[test]
    fn root_issuer_renewal_state_round_trips_through_auth_state() {
        let state = RootIssuerRenewalState {
            issuer_pid: p(51),
            template_fingerprint: [1; 32],
            last_installed_cert_hash: Some([2; 32]),
            last_installed_expires_at_ns: Some(200),
            last_installed_refresh_after_ns: Some(160),
            next_attempt_after_ns: 90,
            updated_at_ns: 80,
        };

        AuthStateOps::upsert_root_issuer_renewal_state(state.clone());

        assert_eq!(AuthStateOps::root_issuer_renewal_state(p(51)), Some(state));
        assert_eq!(AuthStateOps::root_issuer_renewal_state(p(52)), None);
    }
}
