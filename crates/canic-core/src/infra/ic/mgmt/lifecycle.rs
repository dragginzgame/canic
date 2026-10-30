//! Module: infra::ic::mgmt::lifecycle
//!
//! Responsibility: perform raw management canister lifecycle and code-install calls.
//! Does not own: deployment workflow, placement policy, or artifact validation.
//! Boundary: extends `MgmtInfra` with lifecycle management effects.

use super::MgmtInfra;
use crate::infra::ic::{IcInfraError, external_candid::decode_args};
use ic_call::{Call, WaitPolicy};
use ic_platform::candid::contract::encode_one;
use ic_platform::cdk::api;
use ic_platform::{
    management_canister_types::{
        CanisterIdRecord, CanisterInstallMode, ChunkHash, InstallChunkedCodeArgs,
        UninstallCodeArgs, UploadChunkArgs,
    },
    principal::ExternalPrincipal as Principal,
};

impl MgmtInfra {
    /// Upload one wasm chunk into a canister's chunk store.
    pub async fn upload_chunk(
        canister_pid: Principal,
        chunk: Vec<u8>,
    ) -> Result<Vec<u8>, IcInfraError> {
        let args = UploadChunkArgs {
            canister_id: canister_pid,
            chunk,
        };

        let response = Call::new(
            Principal::management_canister().into(),
            "upload_chunk",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;
        let (hash,): (ChunkHash,) = decode_args(&response.into_bytes())?;

        Ok(hash.hash)
    }

    /// List the chunk hashes currently stored in one canister's chunk store.
    pub async fn stored_chunks(canister_pid: Principal) -> Result<Vec<Vec<u8>>, IcInfraError> {
        let args = CanisterIdRecord {
            canister_id: canister_pid,
        };
        let response = Call::new(
            Principal::management_canister().into(),
            "stored_chunks",
            WaitPolicy::Bounded {
                timeout_seconds: 300,
            },
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;
        let (hashes,): (Vec<ChunkHash>,) = decode_args(&response.into_bytes())?;

        Ok(hashes.into_iter().map(|hash| hash.hash).collect())
    }

    /// Clear the chunk store of one canister.
    pub async fn clear_chunk_store(canister_pid: Principal) -> Result<(), IcInfraError> {
        let args = CanisterIdRecord {
            canister_id: canister_pid,
        };

        Call::new(
            Principal::management_canister().into(),
            "clear_chunk_store",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;

        Ok(())
    }

    /// Install a canister from chunks stored in a same-subnet store canister.
    pub async fn install_chunked_code(
        target_canister: Principal,
        store_canister: Principal,
        chunk_hashes_list: Vec<Vec<u8>>,
        wasm_module_hash: Vec<u8>,
        arg: Vec<u8>,
    ) -> Result<(), IcInfraError> {
        let install_args = InstallChunkedCodeArgs {
            mode: CanisterInstallMode::Install,
            target_canister,
            store_canister: Some(store_canister),
            chunk_hashes_list: chunk_hashes_list
                .into_iter()
                .map(|hash| ChunkHash { hash })
                .collect(),
            wasm_module_hash,
            arg,
            sender_canister_version: Some(api::canister_version()),
        };

        Call::new(
            Principal::management_canister().into(),
            "install_chunked_code",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(install_args)?)
        .execute()
        .await?;

        Ok(())
    }

    /// Quote the exact destructive request before recording its paid intent.
    pub fn uninstall_code_call_cost(canister_pid: Principal) -> Result<u128, IcInfraError> {
        Ok(Call::new(
            Principal::management_canister().into(),
            "uninstall_code",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(UninstallCodeArgs {
            canister_id: canister_pid,
            sender_canister_version: Some(api::canister_version()),
        })?)
        .get_cost())
    }

    /// Uninstall code from a canister.
    pub async fn uninstall_code(canister_pid: Principal) -> Result<(), IcInfraError> {
        let args = UninstallCodeArgs {
            canister_id: canister_pid,
            sender_canister_version: Some(api::canister_version()),
        };
        Call::new(
            Principal::management_canister().into(),
            "uninstall_code",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;

        Ok(())
    }

    /// Start an installed canister without changing its code or controllers.
    pub async fn start_canister(canister_pid: Principal) -> Result<(), IcInfraError> {
        let args = CanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::new(
            Principal::management_canister().into(),
            "start_canister",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;
        Ok(())
    }

    /// Quote a stop using the same request as the execution below.
    pub fn stop_canister_call_cost(canister_pid: Principal) -> Result<u128, IcInfraError> {
        Ok(Call::new(
            Principal::management_canister().into(),
            "stop_canister",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(CanisterIdRecord {
            canister_id: canister_pid,
        })?)
        .get_cost())
    }

    /// Stop a canister.
    pub async fn stop_canister(canister_pid: Principal) -> Result<(), IcInfraError> {
        let args = CanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::new(
            Principal::management_canister().into(),
            "stop_canister",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;

        Ok(())
    }

    /// Delete a canister through the management canister.
    pub async fn delete_canister(canister_pid: Principal) -> Result<(), IcInfraError> {
        let args = CanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::new(
            Principal::management_canister().into(),
            "delete_canister",
            WaitPolicy::Unbounded,
        )
        .take_raw_args(encode_one(args)?)
        .execute()
        .await?;

        Ok(())
    }
}
