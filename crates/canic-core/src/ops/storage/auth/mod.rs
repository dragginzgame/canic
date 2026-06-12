pub mod mapper;

use crate::{
    cdk::types::Principal,
    dto::auth::ActiveDelegationProof,
    storage::stable::auth::{
        AuthState, DelegatedSessionBootstrapBindingRecord, DelegatedSessionRecord,
    },
};
use mapper::ActiveDelegationProofRecordMapper;

pub use crate::storage::stable::auth::DelegatedSessionUpsertResult;

///
/// DelegatedSession
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedSession {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bootstrap_token_fingerprint: Option<[u8; 32]>,
}

///
/// DelegatedSessionBootstrapBinding
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DelegatedSessionBootstrapBinding {
    pub wallet_pid: Principal,
    pub delegated_pid: Principal,
    pub token_fingerprint: [u8; 32],
    pub bound_at: u64,
    pub expires_at: u64,
}

///
/// AuthStateOps
///
/// WHY THIS FILE EXISTS
/// --------------------
/// This module defines the **only authorized access path** to persisted
/// auth state stored in stable memory.
///
/// It intentionally sits between:
///   - access / auth logic
///   - stable storage implementation details
///
/// Responsibilities:
/// - Provide a narrow, explicit API for auth state access
/// - Prevent access-layer code from depending on storage internals
/// - Serve as the choke point for schema and lifecycle changes
///
/// This is a **security-sensitive boundary**:
/// auth state stores delegated sessions and role-attestation keys.
///

pub struct AuthStateOps;

impl AuthStateOps {
    /// Return an active delegated session for the provided wallet caller.
    #[must_use]
    pub fn delegated_session(wallet_pid: Principal, now_secs: u64) -> Option<DelegatedSession> {
        AuthState::get_active_delegated_session(wallet_pid, now_secs)
            .map(delegated_session_record_to_view)
    }

    /// Return the active delegated subject for the provided wallet caller.
    #[must_use]
    pub fn delegated_session_subject(wallet_pid: Principal, now_secs: u64) -> Option<Principal> {
        Self::delegated_session(wallet_pid, now_secs).map(|session| session.delegated_pid)
    }

    /// Upsert the delegated session for the provided wallet caller.
    #[cfg(test)]
    pub fn upsert_delegated_session(
        session: DelegatedSession,
        now_secs: u64,
    ) -> DelegatedSessionUpsertResult {
        AuthState::upsert_delegated_session(delegated_session_view_to_record(session), now_secs)
    }

    pub fn upsert_delegated_session_with_bootstrap_binding(
        session: DelegatedSession,
        binding: DelegatedSessionBootstrapBinding,
        now_secs: u64,
    ) -> DelegatedSessionUpsertResult {
        AuthState::upsert_delegated_session_with_bootstrap_binding(
            delegated_session_view_to_record(session),
            delegated_session_bootstrap_binding_view_to_record(binding),
            now_secs,
        )
    }

    /// Remove the delegated session for the provided wallet caller.
    pub fn clear_delegated_session(wallet_pid: Principal) {
        AuthState::clear_delegated_session(wallet_pid);
    }

    /// Remove all expired delegated sessions and return removed count.
    #[must_use]
    pub fn prune_expired_delegated_sessions(now_secs: u64) -> usize {
        AuthState::prune_expired_delegated_sessions(now_secs)
    }

    /// Resolve an active delegated-session bootstrap binding by token fingerprint.
    #[must_use]
    pub fn delegated_session_bootstrap_binding(
        token_fingerprint: [u8; 32],
        now_secs: u64,
    ) -> Option<DelegatedSessionBootstrapBinding> {
        AuthState::get_active_delegated_session_bootstrap_binding(token_fingerprint, now_secs)
            .map(delegated_session_bootstrap_binding_record_to_view)
    }

    /// Remove expired delegated-session bootstrap bindings and return removed count.
    #[must_use]
    pub fn prune_expired_delegated_session_bootstrap_bindings(now_secs: u64) -> usize {
        AuthState::prune_expired_delegated_session_bootstrap_bindings(now_secs)
    }

    #[must_use]
    pub fn active_delegation_proof(now_ns: u64) -> Option<ActiveDelegationProof> {
        let proof = AuthState::get_active_delegation_proof()
            .map(ActiveDelegationProofRecordMapper::record_to_dto)?;
        if now_ns < proof.not_before_ns || now_ns >= proof.expires_at_ns {
            return None;
        }
        Some(proof)
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
}

const fn delegated_session_record_to_view(record: DelegatedSessionRecord) -> DelegatedSession {
    DelegatedSession {
        wallet_pid: record.wallet_pid,
        delegated_pid: record.delegated_pid,
        issued_at: record.issued_at,
        expires_at: record.expires_at,
        bootstrap_token_fingerprint: record.bootstrap_token_fingerprint,
    }
}

const fn delegated_session_view_to_record(view: DelegatedSession) -> DelegatedSessionRecord {
    DelegatedSessionRecord {
        wallet_pid: view.wallet_pid,
        delegated_pid: view.delegated_pid,
        issued_at: view.issued_at,
        expires_at: view.expires_at,
        bootstrap_token_fingerprint: view.bootstrap_token_fingerprint,
    }
}

const fn delegated_session_bootstrap_binding_record_to_view(
    record: DelegatedSessionBootstrapBindingRecord,
) -> DelegatedSessionBootstrapBinding {
    DelegatedSessionBootstrapBinding {
        wallet_pid: record.wallet_pid,
        delegated_pid: record.delegated_pid,
        token_fingerprint: record.token_fingerprint,
        bound_at: record.bound_at,
        expires_at: record.expires_at,
    }
}

const fn delegated_session_bootstrap_binding_view_to_record(
    view: DelegatedSessionBootstrapBinding,
) -> DelegatedSessionBootstrapBindingRecord {
    DelegatedSessionBootstrapBindingRecord {
        wallet_pid: view.wallet_pid,
        delegated_pid: view.delegated_pid,
        token_fingerprint: view.token_fingerprint,
        bound_at: view.bound_at,
        expires_at: view.expires_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dto::auth::{
            DelegatedRoleGrant, DelegationAudience, DelegationCert, DelegationProof,
            IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding, RootProof,
        },
        ids::CanisterRole,
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
                    aud: DelegationAudience::CanicSubnet(p(7)),
                    grants: vec![DelegatedRoleGrant {
                        target: CanisterRole::owned("project_instance".to_string()),
                        scopes: vec!["read".to_string(), "write".to_string()],
                    }],
                },
                root_proof: RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                    signature_cbor: vec![8; 64],
                    public_key_der: vec![9; 32],
                }),
            },
            cert_hash: [10; 32],
            not_before_ns: 20,
            expires_at_ns: 100,
            refresh_after_ns: 80,
            installed_at_ns: 15,
            installed_by: p(11),
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
}
