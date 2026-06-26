//! Minimal root stub for delegation access tests.

#![expect(clippy::unused_async)]

use canic::{
    CANIC_WASM_CHUNK_BYTES, Error,
    api::auth::AuthApi,
    api::canister::CanisterRole,
    dto::auth::{DelegatedToken, SignedRoleAttestation},
    prelude::*,
};
use canic_control_plane::{
    api::template::WasmStoreBootstrapApi,
    dto::template::{TemplateChunkInput, TemplateChunkSetPrepareInput, TemplateManifestInput},
    ids::{
        TemplateChunkingMode, TemplateId, TemplateManifestState, TemplateVersion, WasmStoreBinding,
    },
};
use sha2::{Digest, Sha256};

canic::start!(
    init = {
        seed_chunked_bootstrap_release_set(CHUNKED_BOOTSTRAP_RELEASE_SET);
    }
);

async fn canic_setup() {}
async fn canic_install() {}
async fn canic_upgrade() {}

#[canic_update(public)]
async fn root_verify_role_attestation(
    attestation: SignedRoleAttestation,
    min_accepted_epoch: u64,
) -> Result<(), Error> {
    AuthApi::verify_role_attestation(&attestation, min_accepted_epoch).await
}

#[canic_query(public)]
async fn root_now_secs() -> Result<u64, Error> {
    Ok(ic_cdk::api::time() / 1_000_000_000)
}

#[canic_update(public)]
async fn root_bootstrap_delegated_session(
    token: DelegatedToken,
    delegated_subject: candid::Principal,
    requested_ttl_ns: Option<u64>,
) -> Result<(), Error> {
    AuthApi::set_delegated_session_subject(delegated_subject, token, requested_ttl_ns)
}

#[canic_update(public)]
async fn root_clear_delegated_session() -> Result<(), Error> {
    AuthApi::clear_delegated_session();
    Ok(())
}

#[canic_query(public)]
async fn root_delegated_session_subject() -> Result<Option<candid::Principal>, Error> {
    Ok(AuthApi::delegated_session_subject())
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
const ISSUER_ROLE: CanisterRole = CanisterRole::new("issuer");
const PROJECT_HUB_ROLE: CanisterRole = CanisterRole::new("project_hub");
const PROJECT_INSTANCE_ROLE: CanisterRole = CanisterRole::new("project_instance");
const ISSUER_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/delegation_issuer_stub.wasm"));
const PROJECT_HUB_WASM: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/project_hub_stub.wasm"));
const PROJECT_INSTANCE_WASM: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/project_instance_stub.wasm"));
const CHUNKED_BOOTSTRAP_RELEASE_SET: &[(CanisterRole, &[u8])] = &[
    (ISSUER_ROLE, ISSUER_WASM),
    (PROJECT_HUB_ROLE, PROJECT_HUB_WASM),
    (PROJECT_INSTANCE_ROLE, PROJECT_INSTANCE_WASM),
];

canic::finish!();
