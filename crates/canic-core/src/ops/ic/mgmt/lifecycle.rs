//! Module: ops::ic::mgmt::lifecycle
//!
//! Responsibility: expose management-canister lifecycle and code-install calls.
//! Does not own: placement policy, upgrade planning, or lifecycle orchestration.
//! Boundary: `MgmtOps` extension for canister lifecycle management calls.

use super::*;
use crate::ops::cost_guard::CostGuardPermit;
use candid::utils::ArgumentEncoder;

impl MgmtOps {
    /// Install chunked code after a cost guard has reserved deployment quota and cycles.
    pub async fn install_chunked_code_with_permit<T: ArgumentEncoder>(
        _permit: &CostGuardPermit,
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        args: T,
    ) -> Result<(), InternalError> {
        let chunk_count = chunk_hashes_list.len();
        management_call(
            ManagementCallMetricOperation::InstallChunkedCode,
            MgmtInfra::install_chunked_code(
                target_canister,
                store_canister,
                chunk_hashes_list,
                wasm_module_hash,
                args,
            ),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::InstallCode);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_chunked_code: {target_canister} store={store_canister} chunks={chunk_count}"
        );

        Ok(())
    }

    /// Install embedded code after a cost guard has reserved deployment quota and cycles.
    pub async fn install_code_with_permit<T: ArgumentEncoder>(
        _permit: &CostGuardPermit,
        target_canister: Principal,
        wasm_module: Vec<u8>,
        args: T,
    ) -> Result<(), InternalError> {
        let payload_size_bytes = wasm_module.len();
        management_call(
            ManagementCallMetricOperation::InstallCode,
            MgmtInfra::install_code(target_canister, wasm_module, args),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::InstallCode);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_code: {target_canister} embedded_bytes={payload_size_bytes}"
        );

        Ok(())
    }

    /// Upload one wasm chunk into a canister's chunk store.
    pub async fn upload_chunk(
        canister_pid: Principal,
        chunk: Vec<u8>,
    ) -> Result<Vec<u8>, InternalError> {
        let chunk_len = chunk.len();
        let hash = management_call(
            ManagementCallMetricOperation::UploadChunk,
            MgmtInfra::upload_chunk(canister_pid, chunk),
        )
        .await?;

        #[expect(clippy::cast_precision_loss)]
        let bytes_kb = chunk_len as f64 / 1_000.0;
        log!(
            Topic::CanisterLifecycle,
            Ok,
            "upload_chunk: {canister_pid} ({bytes_kb} KB)"
        );

        Ok(hash)
    }

    /// List the chunk hashes currently stored in one canister's chunk store.
    pub async fn stored_chunks(canister_pid: Principal) -> Result<Vec<Vec<u8>>, InternalError> {
        management_call(
            ManagementCallMetricOperation::StoredChunks,
            MgmtInfra::stored_chunks(canister_pid),
        )
        .await
    }

    /// Clear the chunk store of one canister.
    pub async fn clear_chunk_store(canister_pid: Principal) -> Result<(), InternalError> {
        management_call(
            ManagementCallMetricOperation::ClearChunkStore,
            MgmtInfra::clear_chunk_store(canister_pid),
        )
        .await?;

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "clear_chunk_store: {canister_pid}"
        );

        Ok(())
    }

    /// Uninstalls code from a canister and records metrics.
    pub async fn uninstall_code(canister_pid: Principal) -> Result<(), InternalError> {
        management_call(
            ManagementCallMetricOperation::UninstallCode,
            MgmtInfra::uninstall_code(canister_pid),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::UninstallCode);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "🗑️ uninstall_code: {canister_pid}"
        );

        Ok(())
    }

    /// Stops a canister via the management canister.
    pub async fn stop_canister(canister_pid: Principal) -> Result<(), InternalError> {
        management_call(
            ManagementCallMetricOperation::StopCanister,
            MgmtInfra::stop_canister(canister_pid),
        )
        .await?;

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "stop_canister: {canister_pid}"
        );

        Ok(())
    }

    /// Deletes a canister (code + controllers) via the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), InternalError> {
        management_call(
            ManagementCallMetricOperation::DeleteCanister,
            MgmtInfra::delete_canister(canister_pid),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::DeleteCanister);

        Ok(())
    }
}
