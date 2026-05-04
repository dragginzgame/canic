//! Infra-scoped IC helpers.
//!
//! These wrappers provide low-level IC management canister calls and common
//! ICC call patterns without layering concerns.

use crate::{
    cdk::{
        self, api,
        candid::{CandidType, Nat, Principal, encode_args, utils::ArgumentEncoder},
        types::Cycles,
    },
    infra::{InfraError, ic::IcInfraError, ic::call::Call},
};
use serde::Deserialize;
use thiserror::Error as ThisError;

//
// MgmtInfraError
//

#[derive(Debug, ThisError)]
pub enum MgmtInfraError {
    #[error("raw_rand returned {len} bytes")]
    RawRandInvalidLength { len: usize },
}

//
// InfraCanisterSettings
//

#[derive(CandidType, Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraCanisterSettings {
    pub controllers: Option<Vec<Principal>>,
    pub compute_allocation: Option<Nat>,
    pub memory_allocation: Option<Nat>,
    pub freezing_threshold: Option<Nat>,
    pub reserved_cycles_limit: Option<Nat>,
    pub log_visibility: Option<InfraLogVisibility>,
    pub log_memory_limit: Option<Nat>,
    pub wasm_memory_limit: Option<Nat>,
    pub wasm_memory_threshold: Option<Nat>,
    pub environment_variables: Option<Vec<InfraEnvironmentVariable>>,
}

//
// InfraCreateCanisterArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraCreateCanisterArgs {
    settings: Option<InfraCanisterSettings>,
    sender_canister_version: Option<u64>,
}

//
// InfraCreateCanisterResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraCreateCanisterResult {
    canister_id: Principal,
}

//
// InfraCanisterIdRecord
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct InfraCanisterIdRecord {
    canister_id: Principal,
}

//
// InfraCanisterIdRecordExtended
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct InfraCanisterIdRecordExtended {
    canister_id: Principal,
    sender_canister_version: Option<u64>,
}

//
// InfraCanisterSnapshot
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraCanisterSnapshot {
    pub id: Vec<u8>,
    pub taken_at_timestamp: u64,
    pub total_size: u64,
}

//
// InfraTakeCanisterSnapshotArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraTakeCanisterSnapshotArgs {
    canister_id: Principal,
    replace_snapshot: Option<Vec<u8>>,
    uninstall_code: Option<bool>,
    sender_canister_version: Option<u64>,
}

//
// InfraLoadCanisterSnapshotArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraLoadCanisterSnapshotArgs {
    canister_id: Principal,
    snapshot_id: Vec<u8>,
    sender_canister_version: Option<u64>,
}

//
// InfraCanisterInstallMode
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraCanisterInstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade(Option<InfraUpgradeFlags>),
}

//
// InfraUpgradeFlags
//

#[derive(CandidType, Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct InfraUpgradeFlags {
    pub skip_pre_upgrade: Option<bool>,
    pub wasm_memory_persistence: Option<InfraWasmMemoryPersistence>,
}

//
// InfraWasmMemoryPersistence
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraWasmMemoryPersistence {
    #[serde(rename = "keep")]
    Keep,
    #[serde(rename = "replace")]
    Replace,
}

//
// InfraChunkHash
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraChunkHash {
    hash: Vec<u8>,
}

//
// InfraUploadChunkArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraUploadChunkArgs {
    canister_id: Principal,
    chunk: Vec<u8>,
}

//
// InfraClearChunkStoreArgs
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
struct InfraClearChunkStoreArgs {
    canister_id: Principal,
}

//
// InfraInstallChunkedCodeArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraInstallChunkedCodeArgs {
    mode: InfraCanisterInstallMode,
    target_canister: Principal,
    store_canister: Option<Principal>,
    chunk_hashes_list: Vec<InfraChunkHash>,
    wasm_module_hash: Vec<u8>,
    arg: Vec<u8>,
    sender_canister_version: Option<u64>,
}

//
// InfraInstallCodeArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
struct InfraInstallCodeArgs {
    mode: InfraCanisterInstallMode,
    canister_id: Principal,
    wasm_module: Vec<u8>,
    arg: Vec<u8>,
    sender_canister_version: Option<u64>,
}

//
// InfraUpdateSettingsArgs
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraUpdateSettingsArgs {
    pub canister_id: Principal,
    pub settings: InfraCanisterSettings,
    pub sender_canister_version: Option<u64>,
}

//
// InfraCanisterStatusType
//

#[derive(CandidType, Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraCanisterStatusType {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

//
// InfraLogVisibility
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub enum InfraLogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
    #[serde(rename = "allowed_viewers")]
    AllowedViewers(Vec<Principal>),
}

//
// InfraEnvironmentVariable
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraEnvironmentVariable {
    pub name: String,
    pub value: String,
}

//
// InfraDefiniteCanisterSettings
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraDefiniteCanisterSettings {
    pub controllers: Vec<Principal>,
    pub compute_allocation: Nat,
    pub memory_allocation: Nat,
    pub freezing_threshold: Nat,
    pub reserved_cycles_limit: Nat,
    pub log_visibility: InfraLogVisibility,
    pub log_memory_limit: Nat,
    pub wasm_memory_limit: Nat,
    pub wasm_memory_threshold: Nat,
    pub environment_variables: Vec<InfraEnvironmentVariable>,
}

//
// InfraMemoryMetrics
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraMemoryMetrics {
    pub wasm_memory_size: Nat,
    pub stable_memory_size: Nat,
    pub global_memory_size: Nat,
    pub wasm_binary_size: Nat,
    pub custom_sections_size: Nat,
    pub canister_history_size: Nat,
    pub wasm_chunk_store_size: Nat,
    pub snapshots_size: Nat,
}

//
// InfraQueryStats
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraQueryStats {
    pub num_calls_total: Nat,
    pub num_instructions_total: Nat,
    pub request_payload_bytes_total: Nat,
    pub response_payload_bytes_total: Nat,
}

//
// InfraCanisterStatusResult
//

#[derive(CandidType, Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct InfraCanisterStatusResult {
    pub status: InfraCanisterStatusType,
    pub settings: InfraDefiniteCanisterSettings,
    pub module_hash: Option<Vec<u8>>,
    pub memory_size: Nat,
    pub memory_metrics: InfraMemoryMetrics,
    pub cycles: Nat,
    pub reserved_cycles: Nat,
    pub idle_cycles_burned_per_day: Nat,
    pub query_stats: InfraQueryStats,
}

//
// MgmtInfra
//

pub struct MgmtInfra;

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

    // ────────────────────────────── CANISTER STATUS ──────────────────────────────

    // Query the management canister for a canister's status.
    pub async fn canister_status(
        canister_pid: Principal,
    ) -> Result<InfraCanisterStatusResult, InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        let response = Call::bounded_wait(Principal::management_canister(), "canister_status")
            .with_arg(args)?
            .execute()
            .await?;
        let (status,): (InfraCanisterStatusResult,) = response.candid_tuple()?;

        Ok(status)
    }

    // Creates one canister snapshot through the management canister.
    pub async fn take_canister_snapshot(
        canister_pid: Principal,
        replace_snapshot: Option<Vec<u8>>,
        uninstall_code: Option<bool>,
    ) -> Result<InfraCanisterSnapshot, InfraError> {
        let args = InfraTakeCanisterSnapshotArgs {
            canister_id: canister_pid,
            replace_snapshot,
            uninstall_code,
            sender_canister_version: Some(api::canister_version()),
        };
        let response =
            Call::bounded_wait(Principal::management_canister(), "take_canister_snapshot")
                .with_arg(args)?
                .execute()
                .await?;
        let (snapshot,): (InfraCanisterSnapshot,) = response.candid_tuple()?;

        Ok(snapshot)
    }

    // Loads one canister snapshot through the management canister.
    pub async fn load_canister_snapshot(
        canister_pid: Principal,
        snapshot_id: Vec<u8>,
    ) -> Result<(), InfraError> {
        let args = InfraLoadCanisterSnapshotArgs {
            canister_id: canister_pid,
            snapshot_id,
            sender_canister_version: Some(api::canister_version()),
        };
        Call::bounded_wait(Principal::management_canister(), "load_canister_snapshot")
            .with_arg(args)?
            .execute()
            .await?;

        Ok(())
    }

    // ──────────────────────────────── CYCLES API ─────────────────────────────────

    // Returns the local canister's cycle balance (cheap).
    #[must_use]
    pub fn canister_cycle_balance() -> Cycles {
        cdk::api::canister_cycle_balance().into()
    }

    // Deposits cycles into a canister.
    pub async fn deposit_cycles(canister_pid: Principal, cycles: u128) -> Result<(), InfraError> {
        let args = InfraCanisterIdRecord {
            canister_id: canister_pid,
        };
        Call::bounded_wait(Principal::management_canister(), "deposit_cycles")
            .with_arg(args)?
            .with_cycles(cycles)
            .execute()
            .await?;

        Ok(())
    }

    // Gets a canister's cycle balance (expensive: calls mgmt canister).
    pub async fn get_cycles(canister_pid: Principal) -> Result<Cycles, InfraError> {
        let status = Self::canister_status(canister_pid).await?;
        Ok(status.cycles.into())
    }

    // ──────────────────────────────── RANDOMNESS ────────────────────────────────

    // Query the management canister for raw randomness.
    pub async fn raw_rand() -> Result<[u8; 32], InfraError> {
        let response = Call::unbounded_wait(Principal::management_canister(), "raw_rand")
            .execute()
            .await?;

        let bytes: Vec<u8> = response.candid()?;
        let len = bytes.len();

        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| MgmtInfraError::RawRandInvalidLength { len })
            .map_err(IcInfraError::from)?;

        Ok(seed)
    }

    // ────────────────────────────── INSTALL / UNINSTALL ──────────────────────────

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

    // ─────────────────────────────── SETTINGS API ────────────────────────────────

    // Updates canister settings via the management canister.
    pub async fn update_settings(args: &InfraUpdateSettingsArgs) -> Result<(), InfraError> {
        Call::bounded_wait(Principal::management_canister(), "update_settings")
            .with_arg(args.clone())?
            .execute()
            .await?;

        Ok(())
    }
}
