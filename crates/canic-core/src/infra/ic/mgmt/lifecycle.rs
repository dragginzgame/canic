use crate::{
    cdk::{
        api,
        candid::{Principal, encode_args, utils::ArgumentEncoder},
        types::Cycles,
    },
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};

use super::{
    MgmtInfra,
    types::{
        InfraCanisterIdRecord, InfraCanisterIdRecordExtended, InfraCanisterInstallMode,
        InfraCanisterSettings, InfraChunkHash, InfraClearChunkStoreArgs, InfraCreateCanisterArgs,
        InfraCreateCanisterResult, InfraInstallChunkedCodeArgs, InfraInstallCodeArgs,
        InfraUploadChunkArgs,
    },
};

impl MgmtInfra {
    // Create a canister with explicit controllers and an initial cycle balance.
    pub async fn create_canister(
        controllers: Vec<Principal>,
        cycles: Cycles,
    ) -> Result<Principal, InfraError> {
        let settings = Some(InfraCanisterSettings {
            controllers: Some(controllers),
            ..Default::default()
        });

        let args = InfraCreateCanisterArgs {
            settings,
            sender_canister_version: Some(api::canister_version()),
        };
        let response = Call::bounded_wait(Principal::management_canister(), "create_canister")
            .with_arg(args)?
            .with_cycles(cycles.to_u128())
            .execute()
            .await?;
        let (created,): (InfraCreateCanisterResult,) = response.candid_tuple()?;

        Ok(created.canister_id)
    }

    // Upload one wasm chunk into a canister's chunk store.
    pub async fn upload_chunk(
        canister_pid: Principal,
        chunk: Vec<u8>,
    ) -> Result<Vec<u8>, InfraError> {
        let args = InfraUploadChunkArgs {
            canister_id: canister_pid,
            chunk,
        };

        let response = Call::bounded_wait(Principal::management_canister(), "upload_chunk")
            .with_arg(args)?
            .execute()
            .await?;
        let (hash,): (InfraChunkHash,) = response.candid_tuple()?;

        Ok(hash.hash)
    }

    // List the chunk hashes currently stored in one canister's chunk store.
    pub async fn stored_chunks(canister_pid: Principal) -> Result<Vec<Vec<u8>>, InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        let response = Call::bounded_wait(Principal::management_canister(), "stored_chunks")
            .with_arg(args)?
            .execute()
            .await?;
        let (hashes,): (Vec<InfraChunkHash>,) = response.candid_tuple()?;

        Ok(hashes.into_iter().map(|hash| hash.hash).collect())
    }

    // Clear the chunk store of one canister.
    pub async fn clear_chunk_store(canister_pid: Principal) -> Result<(), InfraError> {
        let args = InfraClearChunkStoreArgs {
            canister_id: canister_pid,
        };

        Call::unbounded_wait(Principal::management_canister(), "clear_chunk_store")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }

    // Install or upgrade a canister from chunks stored in a same-subnet store canister.
    pub async fn install_chunked_code<T: ArgumentEncoder>(
        mode: InfraCanisterInstallMode,
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        args: T,
    ) -> Result<(), InfraError> {
        let arg = encode_args(args).map_err(IcInfraError::from)?;
        let install_args = InfraInstallChunkedCodeArgs {
            mode,
            target_canister,
            store_canister: Some(store_canister),
            chunk_hashes_list: chunk_hashes_list
                .into_iter()
                .map(|hash| InfraChunkHash { hash })
                .collect(),
            wasm_module_hash,
            arg,
            sender_canister_version: Some(api::canister_version()),
        };

        Call::bounded_wait(Principal::management_canister(), "install_chunked_code")
            .with_arg(install_args)?
            .execute()
            .await?;

        Ok(())
    }

    // Install or upgrade a canister from an embedded wasm payload.
    pub async fn install_code<T: ArgumentEncoder>(
        mode: InfraCanisterInstallMode,
        canister_id: Principal,
        wasm_module: Vec<u8>,
        args: T,
    ) -> Result<(), InfraError> {
        let arg = encode_args(args).map_err(IcInfraError::from)?;
        let install_args = InfraInstallCodeArgs {
            mode,
            canister_id,
            wasm_module,
            arg,
            sender_canister_version: Some(api::canister_version()),
        };

        Call::bounded_wait(Principal::management_canister(), "install_code")
            .with_arg(install_args)?
            .execute()
            .await?;

        Ok(())
    }

    // Uninstalls code from a canister.
    pub async fn uninstall_code(canister_pid: Principal) -> Result<(), InfraError> {
        let args = InfraCanisterIdRecordExtended {
            canister_id: canister_pid,
            sender_canister_version: Some(api::canister_version()),
        };
        Call::bounded_wait(Principal::management_canister(), "uninstall_code")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }

    // Stops a canister.
    pub async fn stop_canister(canister_pid: Principal) -> Result<(), InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::bounded_wait(Principal::management_canister(), "stop_canister")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }

    // Deletes a canister (code + controllers) via the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::bounded_wait(Principal::management_canister(), "delete_canister")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }
}
