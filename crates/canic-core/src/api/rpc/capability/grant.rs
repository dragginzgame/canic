use crate::{
    cdk::types::Principal,
    dto::{
        capability::{CapabilityService, DelegatedGrant, DelegatedGrantProof},
        error::Error,
        rpc::Request,
    },
    ops::{ic::ecdsa::EcdsaOps, storage::auth::DelegationStateOps},
};
use candid::encode_one;
use sha2::{Digest, Sha256};

pub(super) fn verify_delegated_grant_hash_binding(
    proof: &DelegatedGrantProof,
) -> Result<(), Error> {
    if proof.grant.capability_hash != proof.capability_hash {
        return Err(Error::invalid(
            "delegated grant capability_hash does not match proof capability_hash",
        ));
    }

    Ok(())
}

pub(super) fn verify_root_delegated_grant_proof(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    verify_root_delegated_grant_claims(capability, proof, caller, target_canister, now_secs)?;
    verify_root_delegated_grant_signature(&proof.grant, &proof.grant_sig)
}

pub(super) fn verify_root_delegated_grant_claims(
    capability: &Request,
    proof: &DelegatedGrantProof,
    caller: Principal,
    target_canister: Principal,
    now_secs: u64,
) -> Result<(), Error> {
    if proof.key_id != super::DELEGATED_GRANT_KEY_ID_V1 {
        return Err(Error::invalid(format!(
            "unsupported delegated grant key_id: {}",
            proof.key_id
        )));
    }

    let grant = &proof.grant;
    if grant.issuer != target_canister {
        return Err(Error::forbidden(
            "delegated grant issuer must match target canister",
        ));
    }
    if grant.subject != caller {
        return Err(Error::forbidden(
            "delegated grant subject must match caller",
        ));
    }
    if !grant.audience.contains(&target_canister) {
        return Err(Error::forbidden(
            "delegated grant audience must include target canister",
        ));
    }
    if grant.scope.service != CapabilityService::Root {
        return Err(Error::forbidden(
            "delegated grant scope service must be Root",
        ));
    }
    let expected_family = root_capability_family(capability);
    if grant.scope.capability_family != expected_family {
        return Err(Error::forbidden(format!(
            "delegated grant scope capability_family '{}' does not match '{}'",
            grant.scope.capability_family, expected_family
        )));
    }
    if grant.quota == 0 {
        return Err(Error::invalid(
            "delegated grant quota must be greater than zero",
        ));
    }
    if grant.expires_at <= grant.issued_at {
        return Err(Error::invalid(
            "delegated grant expires_at must be greater than issued_at",
        ));
    }
    if now_secs < grant.issued_at {
        return Err(Error::forbidden(
            "delegated grant is not valid yet for current time",
        ));
    }
    if now_secs > grant.expires_at {
        return Err(Error::forbidden("delegated grant has expired"));
    }

    Ok(())
}

pub(super) fn verify_root_delegated_grant_signature(
    grant: &DelegatedGrant,
    signature: &[u8],
) -> Result<(), Error> {
    if signature.is_empty() {
        return Err(Error::forbidden("delegated grant signature is required"));
    }

    let root_public_key = DelegationStateOps::root_public_key()
        .ok_or_else(|| Error::forbidden("delegated grant root public key unavailable"))?;
    let grant_hash = delegated_grant_hash(grant)?;
    EcdsaOps::verify_signature(&root_public_key, grant_hash, signature)
        .map_err(|err| Error::forbidden(format!("delegated grant signature invalid: {err}")))?;

    Ok(())
}

pub(super) const fn root_capability_family(capability: &Request) -> &'static str {
    capability.family().label()
}

pub(super) fn delegated_grant_hash(grant: &DelegatedGrant) -> Result<[u8; 32], Error> {
    let payload = encode_one(grant)
        .map_err(|err| Error::internal(format!("failed to encode delegated grant: {err}")))?;
    let mut hasher = Sha256::new();
    hasher.update(super::DELEGATED_GRANT_SIGNING_DOMAIN_V1);
    hasher.update(payload);
    Ok(hasher.finalize().into())
}
