//! Module: fleet_ensure::ops
//!
//! Responsibility: own current-generation durable files and approved single IC effects.
//! Does not own: plan decisions or multi-step orchestration.
//! Boundary: workflow persists an intent here before invoking one platform effect.

mod authority_seal;
mod bounded_observations;
mod canic_init;
pub(super) mod continuation;
mod current_inventory;
pub(super) mod current_protocol;
pub(super) mod funding;
mod install_history;
mod plan_content;
mod platform;
mod protocol;
pub(super) mod recovery;
pub(super) mod reinstall;

use crate::{
    durable_io::{
        RegularFileLockError, RegularFileReadError, lock_regular_file_with_parents,
        read_optional_regular_bytes, write_bytes,
    },
    fleet_ensure::model::{
        DesiredCanisterKind, DesiredFleet, DesiredFleetArtifacts, EffectRecord, EnsureAction,
        FLEET_ENSURE_SCHEMA_VERSION, FleetEnsureJournalRecord, FleetEnsurePlan,
        FleetEnsureStateRecord, FleetObservation, ProtocolArtifactDigests,
        RetainedRootStartAuthorityRecord, RootManagementObservation, RootOwnedCanisterLifecycle,
    },
};
use canic_core::{
    cdk::{types::Cycles, utils::hash::sha256_hex},
    dto::pool::CanisterPoolAssetStatus,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io,
    path::{Path, PathBuf},
};
use thiserror::Error as ThisError;

pub(crate) use platform::{EstateFundingObservation, estate_funding_applied};
pub use platform::{IcpEnsurePlatform, IcpEnsurePlatformError};
#[cfg(test)]
pub(crate) use platform::{
    NativeFundingObservation, install_effect_applied, native_funding_applied,
};

/// Decode the reviewed bounds for Create execution and its first live observation.
pub(crate) fn maximum_creation_observation_burn(desired: &DesiredFleet) -> Option<u128> {
    let observation = desired
        .maximum_observation_burn_cycles
        .parse::<Cycles>()
        .ok()?
        .to_u128();
    let update = desired
        .maximum_update_burn_cycles
        .parse::<Cycles>()
        .ok()?
        .to_u128();
    crate::fleet_ensure::model::creation_observation_burn(observation, update)
}

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
    /// Latest protected failure, excluded from work-progress identity.
    pub provisioning_failure:
        Option<canic_core::dto::component_provisioning::FleetComponentProvisioningRootFailure>,
    pub applied: bool,
    /// Exact Root-owned Cycles Ledger pause returned by current Component provisioning.
    pub estate_funding_required:
        Option<canic_core::dto::component_provisioning::RootEstateFundingRequired>,
    /// Exact live source balance observed while reconciling this effect.
    ///
    /// This is populated only when the terminal predicate itself owns a
    /// stronger observation than the ordinary inventory surface.
    pub post_cycles: Option<u128>,
    pub progress_identity: String,
    pub retry: EffectRetry,
}

/// Typed observation-owned authority to replay one exact retained issued command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectRetry {
    None,
    /// The protected Ready reserve is incomplete; continue the bounded Root-owned refill.
    ContinuePoolMaintenance,
    /// A created Root-owned canister is intentionally not observable by the
    /// operator until the exact Root installation later in this same plan.
    /// Continue the reviewed prerequisites, then revisit this issued effect.
    DeferUntilControllerObservation,
    /// The exact created Principal and Ledger receipt are retained, but the
    /// first live balance is outside the reviewed target and observation-burn
    /// bound. Close the immutable operation before any later action.
    ReplanRequiredAfterCreateBalanceDrift,
    ReplayExactIssuedCommand,
}

/// Complete verified terminal projection published by one current protocol owner.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TerminalFleetInventory {
    pub active_registry: Option<canic_core::dto::fleet_registry::FleetRegistry>,
    pub controlled_cycles_by_principal: BTreeMap<String, u128>,
    pub entries: Vec<crate::registry::RegistryEntry>,
}

/// Exact current infrastructure status and sealed physical reset closure.
#[derive(Clone, Debug)]
pub struct FleetReinstallObservation {
    pub authorities:
        BTreeMap<String, crate::fleet_ensure::model::RootManagementCanisterObservation>,
    pub assets: Vec<crate::fleet_ensure::model::FleetReinstallAssetRecord>,
    pub observation: FleetObservation,
}

/// Installed inactive-activation receipts and the complete physical source estate.
#[derive(Clone, Debug)]
pub struct FleetActivationResetObservation {
    pub inventory: FleetReinstallObservation,
    pub roots: Vec<crate::fleet_ensure::model::RootActivationResetRecord>,
}

/// The authority evidence required at a physical-pool verification boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReinstallAssetCheck {
    /// Preserve the captured module and controllers before resetting Root records.
    BeforeReset,
    /// Preserve controllers after pool roles and current modules are reassigned.
    Terminal,
}

/// Platform boundary used by the workflow and deterministic test adapters.
pub trait EnsurePlatform {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Report informational progress without changing operation authority or effects.
    fn report_progress(&mut self, _progress: crate::fleet_ensure::dto::FleetEnsureProgress) {}

    /// Observe whether this exact operation owns the durable authority seal.
    fn authority_sealed(
        &mut self,
        _operation_id: &str,
        _action: &EnsureAction,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Reinspect the reviewed pool closure immediately before its Root loses old records.
    fn reinstall_assets_match(
        &mut self,
        _intent: &crate::fleet_ensure::model::FleetReinstallRecord,
        _root: &str,
        _check: ReinstallAssetCheck,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Observe exact management authority for all infrastructure reset targets.
    fn reinstall_authorities(
        &mut self,
        _state: &FleetEnsureStateRecord,
    ) -> Result<
        Option<BTreeMap<String, crate::fleet_ensure::model::RootManagementCanisterObservation>>,
        Self::Error,
    > {
        Ok(None)
    }

    /// Capture the complete controlled pool after the existing authority seal.
    fn reinstall_inventory(
        &mut self,
        _source_operation_id: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<FleetReinstallObservation>, Self::Error> {
        Ok(None)
    }

    /// Observe source activation receipts and every controlled physical asset before supersession.
    fn activation_reset_inventory(
        &mut self,
        _source: &crate::fleet_ensure::model::FleetActivationSourceRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<FleetActivationResetObservation>, Self::Error> {
        Ok(None)
    }

    /// Observe the exact retained physical closure through newly installed current Roots.
    fn activation_reset_inventory_after_reset(
        &mut self,
        _intent: &crate::fleet_ensure::model::FleetReinstallRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<FleetReinstallObservation>, Self::Error> {
        Ok(None)
    }

    /// Require exact retained Root activation authority before a dependent Store install.
    /// A reviewed Root install in the same closure supplies its own new authority.
    fn require_retained_root_activation(
        &mut self,
        _operation_id: &str,
        _root: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Bind every observation and effect to the desired input retained by the
    /// reviewed operation. Production adapters must replace any newer caller
    /// input before resuming an in-progress journal.
    fn bind_reviewed_desired(&mut self, desired: &DesiredFleet) -> Result<(), Self::Error>;

    /// Observe configured Roots through management authority only. Production
    /// returns this evidence before any protected Root-owned child query.
    fn observe_root_management(
        &mut self,
        _state: &FleetEnsureStateRecord,
        _reviewed_targets: &std::collections::BTreeSet<String>,
    ) -> Result<Option<RootManagementObservation>, Self::Error> {
        Ok(None)
    }

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

    /// Expand the complete fresh protocol from reviewed init inputs, without remote effects.
    fn fresh_protocol_actions(
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

    /// Pace the next passive observation of one typed asynchronous effect.
    ///
    /// The workflow calls this only after retaining a valid nonterminal
    /// observation. Production adapters wait without issuing an IC effect;
    /// deterministic adapters may advance their simulated clock instead.
    fn pace_effect_observation(
        &mut self,
        action: &EnsureAction,
        consecutive_unchanged_observations: u32,
    );

    /// Pace re-observation of one retained Root-owned canister whose protected
    /// lifecycle has not yet exposed its terminal live balance.
    ///
    /// This is observation-only: implementations must not submit a command,
    /// allocate funding, or mutate the remote estate.
    fn pace_root_owned_observation(
        &mut self,
        _target: &str,
        _consecutive_retained_observations: u32,
    ) {
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
    pub workspace: PathBuf,
    pub content: PathBuf,
    pub journal: PathBuf,
    pub lock: PathBuf,
    pub plan: PathBuf,
    pub root_start_authority: PathBuf,
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
            workspace: root.to_path_buf(),
            content: root
                .join(".canic")
                .join("fleet-ensure")
                .join("objects")
                .join("sha256"),
            journal: directory.join("journal.json"),
            lock: directory.join("operation.lock"),
            plan: directory.join("plan.json"),
            root_start_authority: directory.join("root-start-authority.json"),
            state: directory.join("state.json"),
        }
    }
}

/// Durable current-state I/O failure.

#[derive(Debug, ThisError)]
pub enum EnsureStateError {
    #[error(
        "retained activation source does not prove an Applied host prefix followed by Issued provisioning"
    )]
    InvalidActivationSource,

    #[error(
        "activation reset review or local adoption documents changed; preserve the retained evidence"
    )]
    ActivationResetAdoptionConflict,

    #[error("Fleet ensure continuation authority is invalid: {reason}")]
    ContinuationAuthority { reason: String },
    #[error("Fleet ensure document is invalid at {}: {source}", path.display())]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("Fleet ensure document is unsafe at {}", path.display())]
    Unsafe { path: PathBuf },

    #[error("Fleet ensure retained Root-start authority is invalid at {}", path.display())]
    InvalidRootStartAuthority { path: PathBuf },

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
    let lock = lock_regular_file_with_parents(&paths.lock).map_err(|error| match error {
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
    })?;
    reinstall::adoption::recover(paths)?;
    Ok(lock)
}

pub fn read_journal(
    paths: &EnsurePaths,
) -> Result<Option<FleetEnsureJournalRecord>, EnsureStateError> {
    let mut value: Option<FleetEnsureJournalRecord> = read_current(&paths.journal)?;
    if let Some(journal) = &mut value {
        continuation::hydrate_phases(paths, journal)?;
    }
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

/// Load and verify the exact generator-owned retained Root-start authority, when present.
pub(crate) fn read_root_start_authority(
    paths: &EnsurePaths,
) -> Result<Option<RetainedRootStartAuthorityRecord>, EnsureStateError> {
    let value: Option<RetainedRootStartAuthorityRecord> =
        read_current(&paths.root_start_authority)?;
    let value = validate_schema(value, &paths.root_start_authority, |record| {
        record.schema_version
    })?;
    value
        .map(|record| {
            valid_root_start_authority(&record)
                .then_some(record)
                .ok_or_else(|| EnsureStateError::InvalidRootStartAuthority {
                    path: paths.root_start_authority.clone(),
                })
        })
        .transpose()
}

pub(crate) fn compact_inline_plan(
    paths: &EnsurePaths,
    plan: &FleetEnsurePlan,
) -> Result<bool, EnsureStateError> {
    let Some(bytes) = read_document_bytes(&paths.plan)? else {
        return Ok(false);
    };
    let projection: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|source| EnsureStateError::Decode {
            path: paths.plan.clone(),
            source,
        })?;
    if !plan_content::contains_inline_bytes(&projection)? {
        return Ok(false);
    }
    write_plan(paths, plan)?;
    Ok(true)
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
        completed_reinstall_action_sha256: BTreeMap::default(),
        completed_reinstall_operation_id: None,
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
    let mut artifacts = DesiredFleetArtifacts {
        continuation: continuation::resolve_authority(root, desired)?,
        ..DesiredFleetArtifacts::default()
    };
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

/// Atomically retain one complete generator-owned Root-start authority.
pub(crate) fn write_root_start_authority(
    paths: &EnsurePaths,
    mut authority: RetainedRootStartAuthorityRecord,
) -> Result<RetainedRootStartAuthorityRecord, EnsureStateError> {
    authority.seal();
    write_current(&paths.root_start_authority, &authority)?;
    read_root_start_authority(paths)?.ok_or_else(|| EnsureStateError::InvalidRootStartAuthority {
        path: paths.root_start_authority.clone(),
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_root_start_authority(authority: &RetainedRootStartAuthorityRecord) -> bool {
    if authority.roots.is_empty()
        || authority.roots.len() > crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS
        || !authority.has_valid_digest()
    {
        return false;
    }
    let mut names = BTreeSet::new();
    let mut principals = BTreeSet::new();
    authority.roots.iter().all(|root| {
        let mut controllers = root.controllers.clone();
        controllers.sort();
        controllers.dedup();
        !root.name.is_empty()
            && !root.principal.is_empty()
            && !root.subnet.is_empty()
            && controllers == root.controllers
            && !controllers.is_empty()
            && names.insert(root.name.as_str())
            && principals.insert(root.principal.as_str())
            && is_sha256(&root.module_sha256)
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
