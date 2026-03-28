use super::{
    DEFAULT_WASM_STORE_PUBLISH_CHUNK_BYTES, resolved_store_chunk_set_for_manifest, split_chunks,
    verified_embedded_wasm_for_manifest,
};
use crate::{
    InternalError,
    cdk::candid::{CandidType, utils::ArgumentEncoder},
    cdk::types::Principal,
    dto::template::TemplateManifestResponse,
    ids::{CanisterRole, TemplateChunkingMode},
    ops::ic::mgmt::{CanisterInstallMode, MgmtOps},
};

///
/// TemplateInstallWorkflow
///

pub struct TemplateInstallWorkflow;

impl TemplateInstallWorkflow {
    // Inline installs are reserved for bootstrapping the wasm_store canister itself.
    fn ensure_inline_manifest_is_bootstrap_only(
        manifest: &TemplateManifestResponse,
    ) -> Result<(), InternalError> {
        if manifest.role == CanisterRole::WASM_STORE {
            Ok(())
        } else {
            Err(InternalError::workflow(
                crate::InternalErrorOrigin::Workflow,
                format!(
                    "inline template manifests are reserved for {} bootstrap; role '{}' must be published through {}",
                    CanisterRole::WASM_STORE,
                    manifest.role,
                    CanisterRole::WASM_STORE,
                ),
            ))
        }
    }

    // Upload one bootstrap-only inline wasm into the target chunk store.
    async fn prepare_bootstrap_inline_chunks(
        target_canister: Principal,
        manifest: &TemplateManifestResponse,
    ) -> Result<(Vec<Vec<u8>>, Vec<u8>), InternalError> {
        Self::ensure_inline_manifest_is_bootstrap_only(manifest)?;

        let wasm = verified_embedded_wasm_for_manifest(manifest)?;
        let chunks = split_chunks(wasm.bytes(), DEFAULT_WASM_STORE_PUBLISH_CHUNK_BYTES);

        // Rebuild the target chunk store from scratch so repeated bootstrap attempts
        // do not accumulate stale chunks across reinstall or upgrade flows.
        MgmtOps::clear_chunk_store(target_canister).await?;

        let mut chunk_hashes = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            chunk_hashes.push(MgmtOps::upload_chunk(target_canister, chunk).await?);
        }

        Ok((chunk_hashes, manifest.payload_hash.clone()))
    }

    /// Install or upgrade from the source bound by the approved manifest.
    pub async fn install_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        manifest: &TemplateManifestResponse,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        match manifest.chunking_mode {
            TemplateChunkingMode::Inline => {
                let (chunk_hashes, wasm_module_hash) =
                    Self::prepare_bootstrap_inline_chunks(target_canister, manifest).await?;
                MgmtOps::install_chunked_canister_with_payload(
                    mode,
                    target_canister,
                    target_canister,
                    chunk_hashes,
                    wasm_module_hash,
                    payload,
                    extra_arg,
                )
                .await
            }
            TemplateChunkingMode::Chunked => {
                let (store_pid, info) = resolved_store_chunk_set_for_manifest(manifest).await?;
                MgmtOps::install_chunked_canister_with_payload(
                    mode,
                    target_canister,
                    store_pid,
                    info.chunk_hashes,
                    manifest.payload_hash.clone(),
                    payload,
                    extra_arg,
                )
                .await
            }
        }
    }

    /// Install or upgrade from the source bound by the approved manifest.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        manifest: &TemplateManifestResponse,
        args: T,
    ) -> Result<(), InternalError> {
        match manifest.chunking_mode {
            TemplateChunkingMode::Inline => {
                let (chunk_hashes, wasm_module_hash) =
                    Self::prepare_bootstrap_inline_chunks(target_canister, manifest).await?;
                MgmtOps::install_chunked_code(
                    mode,
                    target_canister,
                    target_canister,
                    chunk_hashes,
                    wasm_module_hash,
                    args,
                )
                .await
            }
            TemplateChunkingMode::Chunked => {
                let (store_pid, info) = resolved_store_chunk_set_for_manifest(manifest).await?;
                MgmtOps::install_chunked_code(
                    mode,
                    target_canister,
                    store_pid,
                    info.chunk_hashes,
                    manifest.payload_hash.clone(),
                    args,
                )
                .await
            }
        }
    }
}
