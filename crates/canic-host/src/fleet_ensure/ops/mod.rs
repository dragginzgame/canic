//! Module: fleet_ensure::ops
//!
//! Responsibility: own current-generation durable files and approved single IC effects.
//! Does not own: plan decisions or multi-step orchestration.
//! Boundary: workflow persists an intent here before invoking one platform effect.

mod canic_init;
mod current_inventory;
pub(super) mod current_protocol;
mod plan_content;
mod platform;
mod protocol;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, lock_regular_file_with_parents,
        read_optional_regular_bytes, write_bytes,
    },
    fleet_ensure::model::{
        DesiredCanisterKind, DesiredFleet, DesiredFleetArtifacts, EffectRecord, EnsureAction,
        FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsureStateRecord, FleetObservation, ProtocolArtifactDigests,
        RootOwnedCanisterLifecycle,
    },
};
use canic_core::{cdk::utils::hash::sha256_hex, dto::pool::CanisterPoolAssetStatus};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::BTreeMap,
    fs::File,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub use platform::{IcpEnsurePlatform, IcpEnsurePlatformError};
#[cfg(test)]
pub(crate) use platform::{install_effect_applied, native_funding_applied};

pub(crate) const fn root_owned_lifecycle(
    kind: DesiredCanisterKind,
    status: &CanisterPoolAssetStatus,
) -> Option<RootOwnedCanisterLifecycle> {
    match kind {
        DesiredCanisterKind::Store if matches!(status, CanisterPoolAssetStatus::Store) => {
            Some(RootOwnedCanisterLifecycle::Store)
        }
        DesiredCanisterKind::Pool => match status {
            CanisterPoolAssetStatus::Ready
            | CanisterPoolAssetStatus::PendingReset
            | CanisterPoolAssetStatus::Failed { .. } => Some(RootOwnedCanisterLifecycle::Idle),
            CanisterPoolAssetStatus::Claimed { .. } => Some(RootOwnedCanisterLifecycle::Claimed),
            CanisterPoolAssetStatus::Workload { .. } => Some(RootOwnedCanisterLifecycle::Workload),
            _ => None,
        },
        _ => None,
    }
}

/// Successful evidence returned by one exact effect or exact replay.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectOutcome {
    pub created_principal: Option<String>,
    pub post_cycles: Option<u128>,
    pub receipt: Option<String>,
}

/// One exact live observation of whether an issued effect reached its terminal state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectObservation {
    pub applied: bool,
    pub progress_identity: String,
}

/// Complete verified terminal projection published by one current protocol owner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalFleetInventory {
    pub active_registry: Option<canic_core::dto::fleet_registry::FleetRegistry>,
    pub controlled_cycles_by_principal: BTreeMap<String, u128>,
    pub entries: Vec<crate::registry::RegistryEntry>,
}

/// Platform boundary used by the workflow and deterministic test adapters.
pub trait EnsurePlatform {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Bind every observation and effect to the desired input retained by the
    /// reviewed operation. Production adapters must replace any newer caller
    /// input before resuming an in-progress journal.
    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error>;

    fn observe(
        &mut self,
        operation_id: &str,
        state: &FleetEnsureStateRecord,
    ) -> Result<FleetObservation, Self::Error>;

    /// Compile current Canic control-plane work from protected roles, topology,
    /// and live Registry evidence. Generic applications have no such work.
    fn protocol_actions(
        &mut self,
        _operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Vec<EnsureAction>, Self::Error> {
        Ok(Vec::new())
    }

    /// Return one complete verified live inventory after the typed protocol has
    /// no remaining transition. The workflow persists this derived projection
    /// only after every reviewed effect is terminal.
    fn terminal_inventory(
        &mut self,
        _operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<TerminalFleetInventory, Self::Error> {
        Ok(TerminalFleetInventory::default())
    }

    fn observe_effect(
        &mut self,
        operation_id: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectObservation, Self::Error>;

    fn action_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error>;

    fn action_destination_cycles(
        &mut self,
        action: &EnsureAction,
        state: &FleetEnsureStateRecord,
    ) -> Result<Option<u128>, Self::Error>;

    /// Return the exact management canister version before an install effect.
    /// Other effect classes have no version boundary.
    fn action_canister_version(
        &mut self,
        _action: &EnsureAction,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<u64>, Self::Error> {
        Ok(None)
    }

    fn apply(
        &mut self,
        operation_id: &str,
        action: &EnsureAction,
        record: &EffectRecord,
        state: &FleetEnsureStateRecord,
    ) -> Result<EffectOutcome, Self::Error>;
}

/// Current Fleet ensure state paths. Historical install directories are never read.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnsurePaths {
    pub content: PathBuf,
    pub journal: PathBuf,
    pub lock: PathBuf,
    pub plan: PathBuf,
    pub state: PathBuf,
}

impl EnsurePaths {
    #[must_use]
    pub fn under(root: &Path, environment: &str, fleet: &str) -> Self {
        let directory = root
            .join(".canic")
            .join("fleet-ensure")
            .join(environment)
            .join(fleet);
        Self {
            content: root
                .join(".canic")
                .join("fleet-ensure")
                .join("objects")
                .join("sha256"),
            journal: directory.join("journal.json"),
            lock: directory.join("operation.lock"),
            plan: directory.join("plan.json"),
            state: directory.join("state.json"),
        }
    }
}

/// Durable current-state I/O failure.

#[derive(Debug, ThisError)]
pub enum EnsureStateError {
    #[error("Fleet ensure document is invalid at {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("Fleet ensure document is unsafe at {}", path.display())]
    Unsafe { path: PathBuf },

    #[error("Fleet ensure document has unsupported schema {actual} at {}", path.display())]
    WrongSchema { path: PathBuf, actual: u16 },

    #[error("configured Fleet artifact is not a regular file: {}", path.display())]
    ArtifactUnavailable { path: PathBuf },

    #[error("failed to read configured Fleet artifact {}: {source}", path.display())]
    ArtifactRead {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to access Fleet ensure state {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("Fleet ensure Store chunk authority is invalid: {reason}")]
    StoreChunkAuthority { reason: String },

    #[error("Fleet ensure Store chunk content is unavailable at {}", path.display())]
    StoreChunkUnavailable { path: PathBuf },

    #[error("Fleet ensure Store chunk content differs from its retained hash at {}", path.display())]
    StoreChunkMismatch { path: PathBuf },

    #[error("failed to lock Fleet ensure state {}", path.display())]
    Lock { path: PathBuf },

    #[error("Fleet ensure state at {} belongs to Fleet {actual}, expected {expected}", path.display())]
    FleetMismatch {
        actual: String,
        expected: String,
        path: PathBuf,
    },
}

pub fn lock_operation(paths: &EnsurePaths) -> Result<File, EnsureStateError> {
    lock_regular_file_with_parents(&paths.lock).map_err(|error| match error {
        RegularFileLockError::Io(source) => EnsureStateError::Io {
            path: paths.lock.clone(),
            source,
        },
        RegularFileLockError::NotRegular => EnsureStateError::Lock {
            path: paths.lock.clone(),
        },
        #[cfg(windows)]
        RegularFileLockError::UnsupportedPlatform => EnsureStateError::Lock {
            path: paths.lock.clone(),
        },
    })
}

pub fn read_journal(
    paths: &EnsurePaths,
) -> Result<Option<FleetEnsureJournalRecord>, EnsureStateError> {
    let value: Option<FleetEnsureJournalRecord> = read_current(&paths.journal)?;
    validate_schema(value, &paths.journal, |record| record.schema_version)
}

pub fn read_plan(paths: &EnsurePaths) -> Result<Option<FleetEnsurePlan>, EnsureStateError> {
    let Some(bytes) = read_document_bytes(&paths.plan)? else {
        return Ok(None);
    };
    let mut projection: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|source| EnsureStateError::Decode {
            path: paths.plan.clone(),
            source,
        })?;
    plan_content::hydrate(paths, &mut projection)?;
    let value: FleetEnsurePlan =
        serde_json::from_value(projection).map_err(|source| EnsureStateError::Decode {
            path: paths.plan.clone(),
            source,
        })?;
    validate_schema(Some(value), &paths.plan, |record| record.schema_version)
}

pub fn read_state(
    paths: &EnsurePaths,
    fleet: &str,
) -> Result<FleetEnsureStateRecord, EnsureStateError> {
    let value: Option<FleetEnsureStateRecord> = read_current(&paths.state)?;
    let value = validate_schema(value, &paths.state, |record| record.schema_version)?;
    if let Some(record) = &value
        && record.fleet != fleet
    {
        return Err(EnsureStateError::FleetMismatch {
            actual: record.fleet.clone(),
            expected: fleet.to_string(),
            path: paths.state.clone(),
        });
    }
    Ok(value.unwrap_or_else(|| FleetEnsureStateRecord {
        active_registry: None,
        completed_reinstalls: BTreeMap::default(),
        fleet: fleet.to_string(),
        pending_principals: BTreeMap::default(),
        principals: BTreeMap::default(),
        retained_cycles_by_principal: BTreeMap::default(),
        schema_version: FLEET_ENSURE_SCHEMA_VERSION,
        topology: BTreeMap::default(),
    }))
}

/// Resolve current desired artifact identities outside pure policy code.
pub fn resolve_desired_artifacts(
    root: &Path,
    desired: &DesiredFleet,
) -> Result<DesiredFleetArtifacts, EnsureStateError> {
    let mut artifacts = DesiredFleetArtifacts::default();
    for canister in &desired.canisters {
        if let Some(wasm) = &canister.wasm {
            artifacts
                .wasm_sha256_by_canister
                .insert(canister.name.clone(), artifact_sha256(root, wasm)?);
        }
        if let Some(init_arg) = &canister.init_arg {
            artifacts
                .init_arg_sha256_by_canister
                .insert(canister.name.clone(), artifact_sha256(root, init_arg)?);
        }
        if let Some(init_candid) = &canister.init_candid {
            artifacts
                .init_candid_sha256_by_canister
                .insert(canister.name.clone(), artifact_sha256(root, init_candid)?);
        }
        if let Some(drain) = &canister.drain {
            artifacts
                .drain_candid_sha256_by_canister
                .insert(canister.name.clone(), artifact_sha256(root, &drain.candid)?);
        }
    }
    for step in &desired.protocol_steps {
        artifacts.protocol_by_step.insert(
            step.name.clone(),
            ProtocolArtifactDigests {
                candid_sha256: artifact_sha256(root, &step.candid)?,
                command_args_sha256: artifact_sha256(root, &step.command_args)?,
                expected_status_sha256: artifact_sha256(root, &step.expected_status)?,
                status_args_sha256: artifact_sha256(root, &step.status_args)?,
            },
        );
    }
    Ok(artifacts)
}

fn artifact_sha256(root: &Path, configured: &str) -> Result<String, EnsureStateError> {
    let configured = Path::new(configured);
    let path = if configured.is_absolute() {
        configured.to_path_buf()
    } else {
        root.join(configured)
    };
    if !path.is_file() {
        return Err(EnsureStateError::ArtifactUnavailable { path });
    }
    let bytes = std::fs::read(&path).map_err(|source| EnsureStateError::ArtifactRead {
        path: path.clone(),
        source,
    })?;
    Ok(sha256_hex(&bytes))
}

pub fn write_journal(
    paths: &EnsurePaths,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureStateError> {
    write_current(&paths.journal, journal)
}

pub fn write_plan(paths: &EnsurePaths, plan: &FleetEnsurePlan) -> Result<(), EnsureStateError> {
    plan_content::retain(paths, plan)?;
    let mut projection =
        super::json::to_value(plan).map_err(|source| EnsureStateError::Decode {
            path: paths.plan.clone(),
            source,
        })?;
    plan_content::remove_inline_bytes(&mut projection)?;
    let bytes =
        serde_json::to_vec_pretty(&projection).map_err(|source| EnsureStateError::Decode {
            path: paths.plan.clone(),
            source,
        })?;
    write_bytes(&paths.plan, &bytes).map_err(|source| EnsureStateError::Io {
        path: paths.plan.clone(),
        source,
    })
}

pub fn write_state(
    paths: &EnsurePaths,
    state: &FleetEnsureStateRecord,
) -> Result<(), EnsureStateError> {
    write_current(&paths.state, state)
}

#[must_use]
/// Hash one exact effect action.
///
/// # Panics
///
/// Panics only if the maintained action enum stops being JSON serializable.
pub fn action_sha256(action: &EnsureAction) -> String {
    sha256_hex(&super::json::to_vec(action).expect("ensure action is JSON serializable"))
}

fn read_current<T>(path: &Path) -> Result<Option<T>, EnsureStateError>
where
    T: DeserializeOwned,
{
    let Some(bytes) = read_document_bytes(path)? else {
        return Ok(None);
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|source| EnsureStateError::Decode {
            path: path.to_path_buf(),
            source,
        })
}

fn read_document_bytes(path: &Path) -> Result<Option<Vec<u8>>, EnsureStateError> {
    match read_optional_regular_bytes(path) {
        Ok(bytes) => Ok(bytes),
        Err(RegularFileReadError::NotRegular) => Err(EnsureStateError::Unsafe {
            path: path.to_path_buf(),
        }),
        Err(RegularFileReadError::Io(source)) => Err(EnsureStateError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(not(unix))]
        Err(RegularFileReadError::UnsupportedPlatform) => Err(EnsureStateError::Unsafe {
            path: path.to_path_buf(),
        }),
    }
}

fn write_current(path: &Path, value: &impl Serialize) -> Result<(), EnsureStateError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| EnsureStateError::Decode {
        path: path.to_path_buf(),
        source,
    })?;
    write_bytes(path, &bytes).map_err(|source| EnsureStateError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_schema<T>(
    value: Option<T>,
    path: &Path,
    schema: impl Fn(&T) -> u16,
) -> Result<Option<T>, EnsureStateError> {
    if let Some(record) = value.as_ref()
        && schema(record) != FLEET_ENSURE_SCHEMA_VERSION
    {
        return Err(EnsureStateError::WrongSchema {
            path: path.to_path_buf(),
            actual: schema(record),
        });
    }
    Ok(value)
}
