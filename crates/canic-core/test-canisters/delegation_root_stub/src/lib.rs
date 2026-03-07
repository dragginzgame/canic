//! Minimal root stub for delegation access tests.

#![allow(clippy::unused_async)]

use canic::{
    Error,
    api::auth::DelegationApi,
    api::canister::{CanisterRole, wasm::WasmApi},
    dto::auth::{
        AttestationKey, AttestationKeySet, AttestationKeyStatus, RoleAttestation,
        RoleAttestationRequest, SignedRoleAttestation,
    },
    prelude::*,
};
use ic_cdk::api::msg_caller;
use k256::ecdsa::{Signature, SigningKey, signature::hazmat::PrehashSigner};
use sha2::{Digest, Sha256};

const TEST_ATTESTATION_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
const TEST_ATTESTATION_KEY_ID: u32 = 4_242;
const TEST_ATTESTATION_KEY_SEED: [u8; 32] = [7u8; 32];
type TestAttestationKeyEntry = (u32, u8, AttestationKeyStatus, Option<u64>, Option<u64>);

canic::start_root!();

// Populate the in-memory WASM registry during eager initialization so root
// bootstrap can proceed under minimal test configs.
canic::eager_init!({
    WasmApi::import_static(WASMS);
});

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_update]
async fn root_issue_self_attestation(
    ttl_secs: u64,
    audience: Option<candid::Principal>,
    epoch: u64,
) -> Result<SignedRoleAttestation, Error> {
    let caller = msg_caller();
    let request = RoleAttestationRequest {
        subject: caller,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience,
        ttl_secs,
        epoch,
        metadata: None,
    };
    DelegationApi::request_role_attestation(request).await
}

#[canic_update]
async fn root_issue_self_attestation_test(
    ttl_secs: u64,
    audience: Option<candid::Principal>,
    epoch: u64,
) -> Result<SignedRoleAttestation, Error> {
    if ttl_secs == 0 {
        return Err(Error::invalid("ttl_secs must be greater than zero"));
    }

    let caller = msg_caller();
    let issued_at = ic_cdk::api::time() / 1_000_000_000;
    let expires_at = issued_at.saturating_add(ttl_secs);

    let payload = RoleAttestation {
        subject: caller,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience,
        issued_at,
        expires_at,
        epoch,
    };

    let signature = sign_attestation(&payload, TEST_ATTESTATION_KEY_SEED)?;
    let public_key = test_public_key(TEST_ATTESTATION_KEY_SEED)?;

    DelegationApi::replace_attestation_key_set(AttestationKeySet {
        root_pid: canister_self(),
        generated_at: issued_at,
        keys: vec![AttestationKey {
            key_id: TEST_ATTESTATION_KEY_ID,
            public_key,
            status: AttestationKeyStatus::Current,
            valid_from: Some(issued_at),
            valid_until: None,
        }],
    });

    Ok(SignedRoleAttestation {
        payload,
        signature,
        key_id: TEST_ATTESTATION_KEY_ID,
    })
}

#[canic_update]
async fn root_issue_self_attestation_test_with_key(
    ttl_secs: u64,
    audience: Option<candid::Principal>,
    epoch: u64,
    key_id: u32,
    key_seed: u8,
) -> Result<SignedRoleAttestation, Error> {
    if ttl_secs == 0 {
        return Err(Error::invalid("ttl_secs must be greater than zero"));
    }

    let caller = msg_caller();
    let issued_at = ic_cdk::api::time() / 1_000_000_000;
    let expires_at = issued_at.saturating_add(ttl_secs);
    let payload = RoleAttestation {
        subject: caller,
        role: CanisterRole::ROOT,
        subnet_id: None,
        audience,
        issued_at,
        expires_at,
        epoch,
    };

    Ok(SignedRoleAttestation {
        signature: sign_attestation(&payload, [key_seed; 32])?,
        payload,
        key_id,
    })
}

#[canic_update]
async fn root_set_test_attestation_key_set(
    entries: Vec<TestAttestationKeyEntry>,
) -> Result<(), Error> {
    let keys = entries
        .into_iter()
        .map(|(key_id, key_seed, status, valid_from, valid_until)| {
            Ok(AttestationKey {
                key_id,
                public_key: test_public_key([key_seed; 32])?,
                status,
                valid_from,
                valid_until,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;

    let generated_at = ic_cdk::api::time() / 1_000_000_000;
    DelegationApi::replace_attestation_key_set(AttestationKeySet {
        root_pid: canister_self(),
        generated_at,
        keys,
    });

    Ok(())
}

#[canic_update]
async fn root_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    DelegationApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

fn test_public_key(seed: [u8; 32]) -> Result<Vec<u8>, Error> {
    let signing_key = SigningKey::from_bytes((&seed).into())
        .map_err(|err| Error::internal(format!("test signing key invalid: {err}")))?;
    Ok(signing_key
        .verifying_key()
        .to_encoded_point(true)
        .as_bytes()
        .to_vec())
}

fn sign_attestation(payload: &RoleAttestation, seed: [u8; 32]) -> Result<Vec<u8>, Error> {
    let signing_key = SigningKey::from_bytes((&seed).into())
        .map_err(|err| Error::internal(format!("test signing key invalid: {err}")))?;
    let payload_bytes = candid::encode_one(payload)
        .map_err(|err| Error::internal(format!("encode failed: {err}")))?;

    let mut hasher = Sha256::new();
    hasher.update(TEST_ATTESTATION_DOMAIN);
    hasher.update(payload_bytes);
    let digest: [u8; 32] = hasher.finalize().into();

    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|err| Error::internal(format!("sign failed: {err}")))?;

    Ok(signature.to_bytes().to_vec())
}

// WASM registry entry to satisfy bootstrap invariants and allow
// auto-create of a non-root canister for delegation tests.
const SIGNER_ROLE: CanisterRole = CanisterRole::new("signer");
const SIGNER_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/delegation_signer_stub.wasm"));
const WASMS: &[(CanisterRole, &[u8])] = &[(SIGNER_ROLE, SIGNER_WASM)];

export_candid!();
