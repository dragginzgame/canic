//! Minimal root stub for delegation access tests.

#![allow(clippy::unused_async)]

use canic::{
    CANIC_WASM_CHUNK_BYTES, Error,
    api::auth::DelegationApi,
    api::canister::CanisterRole,
    cdk::candid::Principal,
    dto::auth::{
        AttestationKey, AttestationKeySet, AttestationKeyStatus, DelegatedToken,
        DelegatedTokenClaims, DelegationAudience, DelegationCert, DelegationProof, RoleAttestation,
        RoleAttestationRequest, SignedRoleAttestation,
    },
    prelude::*,
};
use canic_control_plane::{
    api::template::WasmStoreBootstrapApi,
    dto::template::{TemplateChunkInput, TemplateChunkSetPrepareInput, TemplateManifestInput},
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use ic_cdk::api::msg_caller;
use k256::ecdsa::{Signature, SigningKey, signature::hazmat::PrehashSigner};
use sha2::{Digest, Sha256};

const TEST_ATTESTATION_DOMAIN: &[u8] = b"CANIC_ROLE_ATTESTATION_V1";
const TEST_ATTESTATION_KEY_ID: u32 = 4_242;
const TEST_ATTESTATION_KEY_SEED: [u8; 32] = [7u8; 32];
const TEST_DELEGATION_CERT_DOMAIN: &[u8] = b"CANIC_DELEGATION_CERT_V1";
const TEST_DELEGATED_TOKEN_DOMAIN: &[u8] = b"CANIC_DELEGATED_TOKEN_V1";
const TEST_DELEGATION_ROOT_KEY_SEED: [u8; 32] = [11u8; 32];
const TEST_DELEGATION_SHARD_KEY_SEED: [u8; 32] = [13u8; 32];
type TestAttestationKeyEntry = (u32, u8, AttestationKeyStatus, Option<u64>, Option<u64>);

//
// TestTokenSigningPayload
//

#[derive(CandidType)]
struct TestTokenSigningPayload {
    cert_hash: Vec<u8>,
    claims: DelegatedTokenClaims,
}

canic::start_root!(
    init = {
        seed_chunked_bootstrap_release_set(CHUNKED_BOOTSTRAP_RELEASE_SET);
    }
);

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

#[canic_query]
async fn root_now_secs() -> Result<u64, Error> {
    Ok(ic_cdk::api::time() / 1_000_000_000)
}

#[canic_query]
async fn root_test_delegation_public_keys() -> Result<(Vec<u8>, Vec<u8>), Error> {
    Ok((
        test_public_key(TEST_DELEGATION_ROOT_KEY_SEED)?,
        test_public_key(TEST_DELEGATION_SHARD_KEY_SEED)?,
    ))
}

#[canic_update(requires(caller::is_root()))]
async fn root_issue_test_delegated_token(
    claims: DelegatedTokenClaims,
) -> Result<DelegatedToken, Error> {
    if claims.exp <= claims.iat {
        return Err(Error::invalid("token exp must be greater than iat"));
    }
    if claims.aud.as_ref().is_some_and(Vec::is_empty) {
        return Err(Error::invalid("token aud role list must not be empty"));
    }
    if claims.scopes.is_empty() {
        return Err(Error::invalid("token scopes must not be empty"));
    }

    let cert = DelegationCert {
        root_pid: canister_self(),
        shard_pid: claims.shard_pid,
        issued_at: claims.iat,
        expires_at: claims.exp,
        scopes: claims.scopes.clone(),
        aud: claims.aud.clone(),
    };
    let proof = DelegationProof {
        cert: cert.clone(),
        cert_sig: sign_delegation_cert(&cert, TEST_DELEGATION_ROOT_KEY_SEED)?,
    };
    let token_sig = sign_delegated_token(&claims, &cert, TEST_DELEGATION_SHARD_KEY_SEED)?;

    Ok(DelegatedToken {
        claims,
        proof,
        token_sig,
    })
}

// This endpoint is test-only and is compiled in when
// CANIC_TEST_DELEGATION_MATERIAL enables `canic_test_delegation_material`.
#[canic_update(requires(caller::is_root()))]
#[cfg(canic_test_delegation_material)]
async fn root_install_test_delegation_material(
    proof: DelegationProof,
    root_public_key: Vec<u8>,
    shard_public_key: Vec<u8>,
) -> Result<(), Error> {
    DelegationApi::install_test_delegation_material(proof, root_public_key, shard_public_key)
}

#[canic_update]
async fn root_bootstrap_delegated_session(
    token: DelegatedToken,
    delegated_subject: candid::Principal,
    requested_ttl_secs: Option<u64>,
) -> Result<(), Error> {
    DelegationApi::set_delegated_session_subject(delegated_subject, token, requested_ttl_secs)
}

#[canic_update]
async fn root_clear_delegated_session() -> Result<(), Error> {
    DelegationApi::clear_delegated_session();
    Ok(())
}

#[canic_query]
async fn root_delegated_session_subject() -> Result<Option<candid::Principal>, Error> {
    Ok(DelegationApi::delegated_session_subject())
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

fn sign_delegation_cert(cert: &DelegationCert, seed: [u8; 32]) -> Result<Vec<u8>, Error> {
    let digest = cert_hash(cert);
    sign_digest(digest, seed)
}

fn sign_delegated_token(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
    seed: [u8; 32],
) -> Result<Vec<u8>, Error> {
    let digest = token_signing_hash(claims, cert)?;
    sign_digest(digest, seed)
}

fn sign_digest(digest: [u8; 32], seed: [u8; 32]) -> Result<Vec<u8>, Error> {
    let signing_key = SigningKey::from_bytes((&seed).into())
        .map_err(|err| Error::internal(format!("test signing key invalid: {err}")))?;
    let signature: Signature = signing_key
        .sign_prehash(&digest)
        .map_err(|err| Error::internal(format!("sign failed: {err}")))?;
    Ok(signature.to_bytes().to_vec())
}

fn cert_hash(cert: &DelegationCert) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((TEST_DELEGATION_CERT_DOMAIN.len() as u64).to_be_bytes());
    hasher.update(TEST_DELEGATION_CERT_DOMAIN);
    update_principal(&mut hasher, cert.root_pid);
    update_principal(&mut hasher, cert.shard_pid);
    hasher.update(cert.issued_at.to_be_bytes());
    hasher.update(cert.expires_at.to_be_bytes());
    update_strings(&mut hasher, &cert.scopes);
    update_audience(&mut hasher, &cert.aud);
    hasher.finalize().into()
}

fn token_signing_hash(
    claims: &DelegatedTokenClaims,
    cert: &DelegationCert,
) -> Result<[u8; 32], Error> {
    let payload = TestTokenSigningPayload {
        cert_hash: cert_hash(cert).to_vec(),
        claims: claims.clone(),
    };
    let encoded = candid::encode_one(&payload)
        .map_err(|err| Error::internal(format!("encode failed: {err}")))?;
    Ok(hash_domain_separated(TEST_DELEGATED_TOKEN_DOMAIN, &encoded))
}

fn hash_domain_separated(domain: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update((domain.len() as u64).to_be_bytes());
    hasher.update(domain);
    hasher.update((payload.len() as u64).to_be_bytes());
    hasher.update(payload);
    hasher.finalize().into()
}

fn update_principal(hasher: &mut Sha256, principal: Principal) {
    update_bytes(hasher, principal.as_slice());
}

fn update_audience(hasher: &mut Sha256, audience: &DelegationAudience) {
    match audience {
        DelegationAudience::Any => {
            hasher.update(0u8.to_be_bytes());
        }
        DelegationAudience::Roles(roles) => {
            hasher.update(1u8.to_be_bytes());
            hasher.update((roles.len() as u64).to_be_bytes());
            for role in roles {
                update_bytes(hasher, role.as_str().as_bytes());
            }
        }
    }
}

fn update_strings(hasher: &mut Sha256, values: &[String]) {
    hasher.update((values.len() as u64).to_be_bytes());
    for value in values {
        update_bytes(hasher, value.as_bytes());
    }
}

fn update_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn stage_chunked_bootstrap_release(role: CanisterRole, bytes: &'static [u8]) {
    let version = TemplateVersion::new(env!("CARGO_PKG_VERSION"));
    let template_id = TemplateId::owned(format!("embedded:{role}"));
    let payload_hash = Sha256::digest(bytes).to_vec();
    let now_secs = ic_cdk::api::time() / 1_000_000_000;
    let chunks = bytes
        .chunks(CANIC_WASM_CHUNK_BYTES)
        .map(<[u8]>::to_vec)
        .collect::<Vec<_>>();
    let chunk_hashes = chunks
        .iter()
        .map(|chunk| Sha256::digest(chunk).to_vec())
        .collect::<Vec<_>>();

    WasmStoreBootstrapApi::stage_manifest(TemplateManifestInput {
        template_id: template_id.clone(),
        role,
        version: version.clone(),
        payload_hash: payload_hash.clone(),
        payload_size_bytes: bytes.len() as u64,
        store_binding: WasmStoreBinding::new("bootstrap"),
        chunking_mode: TemplateChunkingMode::Chunked,
        manifest_state: TemplateManifestState::Approved,
        approved_at: Some(now_secs),
        created_at: now_secs,
    });

    WasmStoreBootstrapApi::prepare_chunk_set(TemplateChunkSetPrepareInput {
        template_id: template_id.clone(),
        version: version.clone(),
        payload_hash,
        payload_size_bytes: bytes.len() as u64,
        chunk_hashes,
    })
    .expect("prepare chunked bootstrap release");

    for (chunk_index, bytes) in chunks.into_iter().enumerate() {
        WasmStoreBootstrapApi::publish_chunk(TemplateChunkInput {
            template_id: template_id.clone(),
            version: version.clone(),
            chunk_index: u32::try_from(chunk_index).expect("chunk index fits"),
            bytes,
        })
        .expect("publish chunked bootstrap release chunk");
    }
}

fn seed_chunked_bootstrap_release_set(releases: &'static [(CanisterRole, &[u8])]) {
    for (role, bytes) in releases {
        stage_chunked_bootstrap_release(role.clone(), bytes);
    }
}

// Staged non-root releases used by the root stub after the built-in bootstrap
// wasm_store comes up.
const SIGNER_ROLE: CanisterRole = CanisterRole::new("signer");
const PROJECT_HUB_ROLE: CanisterRole = CanisterRole::new("project_hub");
const PROJECT_INSTANCE_ROLE: CanisterRole = CanisterRole::new("project_instance");
const SIGNER_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/delegation_signer_stub.wasm"));
const PROJECT_HUB_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/project_hub_stub.wasm"));
const PROJECT_INSTANCE_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/project_instance_stub.wasm"));
const CHUNKED_BOOTSTRAP_RELEASE_SET: &[(CanisterRole, &[u8])] = &[
    (SIGNER_ROLE, SIGNER_WASM),
    (PROJECT_HUB_ROLE, PROJECT_HUB_WASM),
    (PROJECT_INSTANCE_ROLE, PROJECT_INSTANCE_WASM),
];

canic::cdk::export_candid_debug!();
