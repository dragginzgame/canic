//! Module: ops::auth::delegation::root_issuer_renewal::identity
//!
//! Responsibility: derive deterministic renewal template, batch, and attempt identifiers.
//! Does not own: storage mutation, scheduling decisions, or DTO conversion.

use crate::{
    cdk::types::Principal,
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootIssuerRenewalTemplate,
    },
};
use sha2::{Digest, Sha256};

const ROOT_ISSUER_RENEWAL_TEMPLATE_FINGERPRINT_DOMAIN: &[u8] =
    b"canic-root-issuer-renewal-template:v1";
const ROOT_DELEGATION_RENEWAL_BATCH_ID_DOMAIN: &[u8] = b"canic-root-delegation-renewal-batch:v1";
const ROOT_ISSUER_RENEWAL_ATTEMPT_ID_DOMAIN: &[u8] = b"canic-root-issuer-renewal-attempt:v1";

pub(in crate::ops::auth::delegation) fn renewal_template_fingerprint(
    template: &RootIssuerRenewalTemplate,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_ISSUER_RENEWAL_TEMPLATE_FINGERPRINT_DOMAIN);
    hash_renewal_principal(&mut hasher, template.issuer_pid);
    hash_renewal_bool(&mut hasher, template.enabled);
    hash_renewal_policy_audience(&mut hasher, &template.audience);
    hash_renewal_policy_grants(&mut hasher, &template.grants);
    hash_renewal_u64(&mut hasher, template.cert_ttl_ns);
    hasher.finalize().into()
}

pub(super) fn renewal_batch_id(
    now_ns: u64,
    due_template_count: usize,
    due_template_fingerprints: impl Iterator<Item = (Principal, [u8; 32])>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_DELEGATION_RENEWAL_BATCH_ID_DOMAIN);
    hash_renewal_u64(&mut hasher, now_ns);
    hash_renewal_u64(&mut hasher, due_template_count as u64);
    for (issuer_pid, template_fingerprint) in due_template_fingerprints {
        hash_renewal_principal(&mut hasher, issuer_pid);
        hash_renewal_bytes(&mut hasher, &template_fingerprint);
    }
    hasher.finalize().into()
}

pub(super) fn renewal_attempt_id(
    batch_id: [u8; 32],
    issuer_pid: Principal,
    cert_hash: [u8; 32],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_renewal_bytes(&mut hasher, ROOT_ISSUER_RENEWAL_ATTEMPT_ID_DOMAIN);
    hash_renewal_bytes(&mut hasher, &batch_id);
    hash_renewal_principal(&mut hasher, issuer_pid);
    hash_renewal_bytes(&mut hasher, &cert_hash);
    hasher.finalize().into()
}

fn hash_renewal_policy_audience(hasher: &mut Sha256, audience: &RootDelegationAudiencePolicy) {
    match audience {
        RootDelegationAudiencePolicy::Canister(canister) => {
            hash_renewal_bytes(hasher, b"canister");
            hash_renewal_principal(hasher, *canister);
        }
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            hash_renewal_bytes(hasher, b"canic_subnet");
            hash_renewal_principal(hasher, *subnet);
        }
        RootDelegationAudiencePolicy::Project(project) => {
            hash_renewal_bytes(hasher, b"project");
            hash_renewal_bytes(hasher, project.as_bytes());
        }
    }
}

fn hash_renewal_policy_grants(hasher: &mut Sha256, grants: &[RootDelegatedRoleGrantPolicy]) {
    hash_renewal_u64(hasher, grants.len() as u64);
    for grant in grants {
        hash_renewal_bytes(hasher, grant.target.as_str().as_bytes());
        hash_renewal_u64(hasher, grant.scopes.len() as u64);
        for scope in &grant.scopes {
            hash_renewal_bytes(hasher, scope.as_bytes());
        }
    }
}

fn hash_renewal_bool(hasher: &mut Sha256, value: bool) {
    hasher.update([u8::from(value)]);
}

fn hash_renewal_principal(hasher: &mut Sha256, principal: Principal) {
    hash_renewal_bytes(hasher, principal.as_slice());
}

fn hash_renewal_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn hash_renewal_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hash_renewal_u64(hasher, bytes.len() as u64);
    hasher.update(bytes);
}
