use super::*;
use candid::utils::ArgumentEncoder;

impl MgmtOps {
    /// Create a canister with explicit controllers and an initial cycle balance.
    pub async fn create_canister(
        controllers: Vec<Principal>,
        cycles: Cycles,
    ) -> Result<Principal, InternalError> {
        let pid = management_call(
            ManagementCallMetricOperation::CreateCanister,
            MgmtInfra::create_canister(controllers, cycles),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::CreateCanister);

        Ok(pid)
    }

    /// Install or upgrade a canister from chunks stored in one same-subnet store canister.
    pub async fn install_chunked_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
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
                install_mode_to_infra(mode),
                target_canister,
                store_canister,
                chunk_hashes_list,
                wasm_module_hash,
                args,
            ),
        )
        .await?;

        let metric_kind = match mode {
            CanisterInstallMode::Install => SystemMetricKind::InstallCode,
            CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
            CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
        };
        SystemMetrics::increment(metric_kind);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_chunked_code: {target_canister} mode={mode:?} store={store_canister} chunks={chunk_count}"
        );

        Ok(())
    }

    /// Install or upgrade a canister from an embedded wasm payload.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        wasm_module: Vec<u8>,
        args: T,
    ) -> Result<(), InternalError> {
        let payload_size_bytes = wasm_module.len();
        management_call(
            ManagementCallMetricOperation::InstallCode,
            MgmtInfra::install_code(
                install_mode_to_infra(mode),
                target_canister,
                wasm_module,
                args,
            ),
        )
        .await?;

        let metric_kind = match mode {
            CanisterInstallMode::Install => SystemMetricKind::InstallCode,
            CanisterInstallMode::Reinstall => SystemMetricKind::ReinstallCode,
            CanisterInstallMode::Upgrade(_) => SystemMetricKind::UpgradeCode,
        };
        SystemMetrics::increment(metric_kind);

        log!(
            Topic::CanisterLifecycle,
            Ok,
            "install_code: {target_canister} mode={mode:?} embedded_bytes={payload_size_bytes}"
        );

        Ok(())
    }

    /// Install or reinstall a Canic-style canister from chunk-store-backed wasm.
    pub async fn install_chunked_canister_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        Self::install_chunked_code(
            mode,
            target_canister,
            store_canister,
            chunk_hashes_list,
            wasm_module_hash,
            (payload, extra_arg),
        )
        .await
    }

    /// Install or reinstall a Canic-style canister from an embedded wasm payload.
    pub async fn install_embedded_canister_with_payload<P: CandidType>(
        mode: CanisterInstallMode,
        target_canister: Principal,
        wasm_module: Vec<u8>,
        payload: P,
        extra_arg: Option<Vec<u8>>,
    ) -> Result<(), InternalError> {
        Self::install_code(mode, target_canister, wasm_module, (payload, extra_arg)).await
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

    /// Query the management canister for raw randomness and record metrics.
    pub async fn raw_rand() -> Result<[u8; 32], InternalError> {
        let seed = management_call(
            ManagementCallMetricOperation::RawRand,
            MgmtInfra::raw_rand(),
        )
        .await?;

        SystemMetrics::increment(SystemMetricKind::RawRand);

        Ok(seed)
    }
}
