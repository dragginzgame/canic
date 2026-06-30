//! Module: ops::auth::delegation::chain_key_registry
//!
//! Responsibility: derive the signed delegated-auth registry snapshot for chain-key batches.
//! Does not own: policy mutation, signing, issuer install, or verifier config parsing.
//! Boundary: deterministic root registry view consumed by chain-key batch preparation.

use super::{
    root_issuer_policy::{delegated_role_grant_views, delegation_audience_view},
    root_issuer_renewal::renewal_template_fingerprint,
};
use crate::{
    InternalError, InternalErrorOrigin,
    dto::auth::{
        DelegatedAuthIssuerPolicySnapshotV1, DelegatedAuthRegistrySnapshotV1, DelegatedRoleGrant,
        DelegationAudience, IssuerProofAlgorithm, IssuerProofBinding, RootKeyPolicyV1,
        RootProofMode,
    },
    ops::{
        auth::{
            delegated::canonical::{
                delegated_auth_registry_hash, issuer_proof_binding_hash, root_key_policy_hash,
            },
            issuer_canister_sig::{IssuerPayloadKind, issuer_canister_sig_seed_hash},
        },
        storage::auth::AuthStateOps,
    },
};

const DELEGATED_AUTH_REGISTRY_SCHEMA_VERSION_V1: u16 = 1;

///
/// ChainKeyDelegatedAuthRegistry
///
/// Current root registry epoch/hash pair for chain-key batch signing.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::ops::auth) struct ChainKeyDelegatedAuthRegistry {
    pub snapshot: DelegatedAuthRegistrySnapshotV1,
    pub hash: [u8; 32],
}

pub(in crate::ops::auth) fn current_chain_key_delegated_auth_registry(
    root_key_policy: &RootKeyPolicyV1,
) -> Result<ChainKeyDelegatedAuthRegistry, InternalError> {
    let snapshot = current_chain_key_delegated_auth_registry_snapshot(root_key_policy);
    let hash = delegated_auth_registry_hash(&snapshot).map_err(|err| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("delegated-auth registry snapshot is not canonical: {err}"),
        )
    })?;
    Ok(ChainKeyDelegatedAuthRegistry { snapshot, hash })
}

fn current_chain_key_delegated_auth_registry_snapshot(
    root_key_policy: &RootKeyPolicyV1,
) -> DelegatedAuthRegistrySnapshotV1 {
    let root_key_policy_hash = root_key_policy_hash(root_key_policy);
    let mut issuer_policies = AuthStateOps::root_issuer_policies()
        .into_iter()
        .map(|policy| {
            let renewal_template_hash =
                AuthStateOps::root_issuer_renewal_template(policy.issuer_pid)
                    .map_or([0; 32], |template| renewal_template_fingerprint(&template));
            let issuer_proof_algorithm = IssuerProofAlgorithm::IcCanisterSignatureV1;
            let issuer_proof_binding = IssuerProofBinding::IcCanisterSignatureV1 {
                seed_hash: issuer_canister_sig_seed_hash(IssuerPayloadKind::DelegatedTokenClaims),
            };

            let mut allowed_audiences = policy
                .allowed_audiences
                .iter()
                .map(delegation_audience_view)
                .collect::<Vec<_>>();
            normalize_audiences(&mut allowed_audiences);
            let mut allowed_grants = delegated_role_grant_views(&policy.allowed_grants);
            normalize_grants(&mut allowed_grants);

            DelegatedAuthIssuerPolicySnapshotV1 {
                issuer_canister_id: policy.issuer_pid,
                enabled: policy.enabled,
                preferred_proof_mode: RootProofMode::ChainKeyBatch,
                allowed_audiences,
                allowed_grants,
                max_root_proof_ttl_ns: policy.max_cert_ttl_ns,
                max_token_ttl_ns: policy.max_cert_ttl_ns,
                issuer_proof_algorithm,
                issuer_proof_binding_hash: issuer_proof_binding_hash(
                    policy.issuer_pid,
                    issuer_proof_algorithm,
                    issuer_proof_binding,
                ),
                renewal_template_hash,
            }
        })
        .collect::<Vec<_>>();
    issuer_policies.sort_by(|left, right| {
        left.issuer_canister_id
            .as_slice()
            .cmp(right.issuer_canister_id.as_slice())
    });

    DelegatedAuthRegistrySnapshotV1 {
        schema_version: DELEGATED_AUTH_REGISTRY_SCHEMA_VERSION_V1,
        root_canister_id: root_key_policy.root_canister_id,
        registry_epoch: AuthStateOps::delegated_auth_registry_epoch(),
        proof_mode: RootProofMode::ChainKeyBatch,
        root_key_policy_hash,
        issuer_policies,
    }
}

fn normalize_audiences(audiences: &mut Vec<DelegationAudience>) {
    audiences.sort_by_key(audience_sort_key);
    audiences.dedup();
}

fn audience_sort_key(audience: &DelegationAudience) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    match audience {
        DelegationAudience::Canister(canister) => {
            out.push(1);
            encode_sort_bytes(&mut out, canister.as_slice());
        }
        DelegationAudience::CanicSubnet(subnet) => {
            out.push(2);
            encode_sort_bytes(&mut out, subnet.as_slice());
        }
        DelegationAudience::Project(project) => {
            out.push(3);
            encode_sort_bytes(&mut out, project.as_bytes());
        }
    }
    out
}

fn encode_sort_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("delegated auth registry sort key exceeds u32");
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
}

fn normalize_grants(grants: &mut Vec<DelegatedRoleGrant>) {
    for grant in grants.iter_mut() {
        grant.scopes.sort();
        grant.scopes.dedup();
    }
    grants.sort_by(|left, right| left.target.as_str().cmp(right.target.as_str()));

    let mut merged = Vec::<DelegatedRoleGrant>::with_capacity(grants.len());
    for grant in grants.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.target == grant.target
        {
            last.scopes.extend(grant.scopes);
            last.scopes.sort();
            last.scopes.dedup();
            continue;
        }
        merged.push(grant);
    }
    *grants = merged;
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cdk::types::Principal,
        domain::policy::auth::{
            RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerPolicy,
            RootIssuerRenewalTemplate,
        },
        dto::auth::{ChainKeyAlgorithm, ChainKeyKeyId},
        ids::{BuildNetwork, CanisterRole},
    };

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn root_key_policy() -> RootKeyPolicyV1 {
        RootKeyPolicyV1 {
            root_canister_id: p(1),
            proof_mode: RootProofMode::ChainKeyBatch,
            algorithm: ChainKeyAlgorithm::EcdsaSecp256k1,
            key_id: ChainKeyKeyId {
                name: "test_key_1".to_string(),
            },
            derivation_path_hash: [2; 32],
            public_key: vec![3; 33],
            key_version: 4,
            min_accepted_key_version: 4,
            min_accepted_proof_epoch: 10,
            min_accepted_registry_epoch: 1,
            max_revocation_latency_ns: 60_000_000_000,
            valid_from_ns: 1,
            accept_until_ns: 120_000_000_000,
            build_network: BuildNetwork::Local,
        }
    }

    fn policy(issuer_pid: Principal) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid,
            enabled: true,
            allowed_audiences: vec![
                RootDelegationAudiencePolicy::Project("zeta".to_string()),
                RootDelegationAudiencePolicy::Project("alpha".to_string()),
            ],
            allowed_grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["write".to_string(), "read".to_string()],
            }],
            max_cert_ttl_ns: 60_000_000_000,
            refresh_after_ratio_bps: 8_000,
        }
    }

    fn template(issuer_pid: Principal) -> RootIssuerRenewalTemplate {
        RootIssuerRenewalTemplate {
            issuer_pid,
            enabled: true,
            audience: RootDelegationAudiencePolicy::Project("alpha".to_string()),
            grants: vec![RootDelegatedRoleGrantPolicy {
                target: CanisterRole::owned("project_instance".to_string()),
                scopes: vec!["read".to_string()],
            }],
            cert_ttl_ns: 60_000_000_000,
        }
    }

    #[test]
    fn chain_key_registry_snapshot_hashes_current_policies_and_epoch() {
        let first = p(72);
        let second = p(71);
        AuthStateOps::upsert_root_issuer_policy(policy(first));
        AuthStateOps::upsert_root_issuer_policy(policy(second));
        AuthStateOps::upsert_root_issuer_renewal_template(template(first));
        AuthStateOps::advance_delegated_auth_registry_epoch();

        let registry = current_chain_key_delegated_auth_registry(&root_key_policy())
            .expect("registry should hash");

        assert_eq!(
            registry.snapshot.registry_epoch,
            AuthStateOps::delegated_auth_registry_epoch()
        );
        assert_eq!(registry.snapshot.issuer_policies.len(), 2);
        assert_eq!(
            registry.snapshot.issuer_policies[0].issuer_canister_id,
            second
        );
        assert_eq!(
            registry.snapshot.issuer_policies[1].issuer_canister_id,
            first
        );
        assert_eq!(
            registry.snapshot.issuer_policies[0].allowed_audiences,
            vec![
                DelegationAudience::Project("zeta".to_string()),
                DelegationAudience::Project("alpha".to_string()),
            ]
        );
        assert_eq!(
            registry.snapshot.issuer_policies[0].allowed_grants[0].scopes,
            vec!["read".to_string(), "write".to_string()]
        );
        assert_eq!(
            registry.hash,
            delegated_auth_registry_hash(&registry.snapshot).unwrap()
        );
    }
}
