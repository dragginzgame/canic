//! Module: fleet_ensure::ops
//!
//! Responsibility: own current-generation durable files and approved single IC effects.
//! Does not own: plan decisions or multi-step orchestration.
//! Boundary: workflow persists an intent here before invoking one platform effect.

pub mod attempt_recovery;
mod bounded_observations;
mod canic_init;
pub mod capacity_import;
pub(in crate::fleet_ensure) mod certified_custody;
pub(super) mod clean_reinstall;
pub(super) mod continuation;
mod current_inventory;
pub(super) mod current_protocol;
pub(super) mod effect_preparation;
pub(super) mod funding;
pub(super) mod funding_observation;
pub mod independent_effects;
pub mod infrastructure_bootstrap;
mod install_history;
pub(super) mod operation_selection;
pub mod operator_mint;
mod plan_content;
mod platform;
pub(super) mod progress;
mod protocol;
pub(super) mod readiness;
pub(super) mod recovery;
pub(super) mod reinstall;
pub mod release;
pub mod retained_contract;
pub(super) mod startup_funding;
pub(super) mod terminal;

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

#[cfg(feature = "local-fleet")]
pub(crate) use canic_init::compile_root_authorities;
#[cfg(test)]
pub(in crate::fleet_ensure) use canic_init::tests::qualify_release_input_reuse;
#[cfg(feature = "local-fleet")]
pub use canic_init::{CanicInitError, CanicInitRequest, compile_arguments};
#[cfg(test)]
pub(crate) use platform::install_effect_applied;
pub(crate) use platform::{
    EstateFundingObservation, NativeFundingObservation, estate_funding_applied,
    native_funding_applied,
};
pub use platform::{IcpEnsurePlatform, IcpEnsurePlatformError};

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
            CanisterPoolAssetStatus::Ready => Some(RootOwnedCanisterLifecycle::Idle),
            CanisterPoolAssetStatus::PendingReset | CanisterPoolAssetStatus::Failed { .. } => {
                Some(RootOwnedCanisterLifecycle::Reconciling)
            }
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
    /// Informational progress only; never used for completion or retry authority.
    pub provisioning_progress: Option<crate::fleet_ensure::dto::FleetProvisioningProgress>,
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

    /// Inspect explicitly supplied infrastructure without querying uninitialized Root protocols.
    fn infrastructure_bootstrap_observation(
        &mut self,
        _record: &crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
        _state: &FleetEnsureStateRecord,
    ) -> Result<
        Option<
            crate::fleet_ensure::view::infrastructure_bootstrap::InfrastructureBootstrapObservation,
        >,
        Self::Error,
    > {
        Ok(None)
    }

    /// Reuse observations only within one read-only planning transaction. Adapters
    /// must expire evidence on exit, retries, changed inputs and before any effect.
    fn with_planning_observations<T, E>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        Self: Sized,
    {
        observe(self)
    }

    /// Share fresh pre-intent reads for one action; never retain them across submission.
    fn with_preparation_observations<T>(
        &mut self,
        _action: &EnsureAction,
        observe: impl FnOnce(&mut Self) -> Result<T, Self::Error>,
    ) -> Result<T, Self::Error>
    where
        Self: Sized,
    {
        observe(self)
    }

    /// Share Store catalog reads within one independent-effect reconciliation pass.
    /// Expire them on exit and before submission; balances remain fresh per effect.
    fn with_independent_observations<T, E>(
        &mut self,
        observe: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        Self: Sized,
    {
        observe(self)
    }

    /// Measure an existing activity without changing its result or owning its effects.
    fn with_activity<T, E>(
        &mut self,
        _stage: crate::fleet_ensure::dto::FleetObservationStage,
        activity: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        Self: Sized,
    {
        activity(self)
    }

    /// Report informational progress without changing operation authority or effects.
    fn report_progress(&mut self, _progress: crate::fleet_ensure::dto::FleetEnsureProgress) {}

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

    /// Bind original supplied custody for infrastructure initialization.
    fn bind_infrastructure_bootstrap(
        &mut self,
        _source: &crate::fleet_ensure::model::infrastructure_bootstrap::InfrastructureBootstrapRecord,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Bind fresh funding observations to the separately approved initialization targets.
    fn bind_bootstrap_registration_recovery(
        &mut self,
        _review: &crate::fleet_ensure::model::infrastructure_bootstrap::registration_recovery::BootstrapRegistrationReviewRecord,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Verify exact current Coordinator genesis before dependent infrastructure effects.
    fn verify_bootstrap_coordinator(
        &mut self,
        _state: &FleetEnsureStateRecord,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Verify completed infrastructure registration before publication permits pool import.
    fn verify_bootstrap_registry(
        &mut self,
        _expected: &canic_core::dto::fleet_registry::FleetRegistry,
    ) -> Result<bool, Self::Error> {
        Ok(false)
    }

    /// Observe configured Roots through management authority only. Production
    /// returns this evidence before any protected Root-owned child query.
    fn observe_root_management(
        &mut self,
        _state: &FleetEnsureStateRecord,
        _reviewed_targets: &std::collections::BTreeSet<String>,
    ) -> Result<Option<RootManagementObservation>, Self::Error> {
        Ok(None)
    }

    /// Read the operator's Cycles Ledger account and current fee without runtime queries.
    fn observe_operator_funding(
        &mut self,
    ) -> Result<Option<crate::fleet_ensure::view::OperatorFundingObservation>, Self::Error> {
        Ok(None)
    }

    /// Inspect one configured Root and the operator Ledger without runtime queries.
    fn observe_native_funding(
        &mut self,
        _root: &str,
        _state: &FleetEnsureStateRecord,
    ) -> Result<Option<crate::fleet_ensure::model::NativeFundingObservation>, Self::Error> {
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

    /// Expand only Store and Registry setup from reviewed initialization authority.
    fn infrastructure_protocol_actions(
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

    /// Observe completion and its exact live balance. Native funding must return
    /// its protected balance in `post_cycles`, including before the first intent;
    /// a record without a receipt cannot establish paid completion.
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

    /// Submit admitted independent effects with durable intents and drain every result.
    /// Results retain input order; a failed call must not discard sibling receipts.
    /// An outer error is permitted only before submitting any of the calls.
    fn apply_independent_effects(
        &mut self,
        operation_id: &str,
        uploads: &[independent_effects::IndependentEffect<'_>],
        state: &FleetEnsureStateRecord,
    ) -> Result<Vec<Result<EffectOutcome, Self::Error>>, Self::Error> {
        Ok(uploads
            .iter()
            .map(|upload| self.apply(operation_id, upload.action, upload.record, state))
            .collect())
    }
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
    #[error("reset requires reconciliation of the uncertain paid effect at {} ({effect}); preserve its request and receipt evidence before replacing this installation", path.display())]
    ResetUncertainEffect { path: PathBuf, effect: String },
    #[error("reset review digest or cancellation evidence differs; preserve retained authority")]
    ResetReviewConflict,
    #[error("reset publication path {} differs from retained {}; keep the selected policy and inventory paths", selected.display(), retained.display())]
    ResetInputConflict {
        retained: PathBuf,
        selected: PathBuf,
    },

    #[error(
        "reset review cancellation {plan_sha256} is incomplete; resume fleet ensure --cancel-reinstall with that exact digest"
    )]
    ResetReviewCancellationPending { plan_sha256: String },

    #[error("reset review has execution or side-operation evidence at {}; preserve it and resume its existing owner", path.display())]
    ResetReviewEffectEvidence { path: PathBuf },

    #[error("approved Fleet capacity import at {} must resume under its original authority before other Fleet operations", path.display())]
    CapacityImportInProgress { path: PathBuf },

    #[error("invalid Fleet capacity import journal at {}: {source}", path.display())]
    CapacityImportJournal {
        path: PathBuf,
        #[source]
        source: Box<capacity_import::journal::CapacityImportJournalError>,
    },
    #[error(
        "retained terminal evidence is incomplete or inconsistent; preserve all source documents and paid-effect receipts"
    )]
    InvalidTerminalSource,

    #[error("startup funding configuration differs from the reviewed deployment configuration")]
    StartupConfigurationMismatch,

    #[error("startup funding configuration is unavailable: {0}")]
    StartupConfiguration(#[source] Box<crate::release_set::AppConfigError>),

    #[error("startup funding requirement is invalid: {0}")]
    StartupFunding(#[source] Box<crate::fleet_ensure::policy::EnsurePolicyError>),
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
    #[error(
        "Fleet ensure document is invalid at {}: {source}; preserve the Fleet directory, artifacts and paid-effect receipts; use current-build fleet ensure --reinstall with explicit physical inventory for replacement; genuinely uncertain paid effects require reconciliation, but predecessor application completion is not required; do not insert missing fields or delete journals; see docs/features/operations/fleet-ensure.md#unreadable-retained-plan",
        path.display()
    )]
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
    let lock = lock_fleet_file(paths)?;
    capacity_import::journal::require_no_approved_import(paths)?;
    reinstall::adoption::recover(paths)?;
    Ok(lock)
}

/// The capacity owner resumes its retained journal while holding the ordinary Fleet lock.
fn lock_capacity_import_operation(paths: &EnsurePaths) -> Result<File, EnsureStateError> {
    let lock = lock_fleet_file(paths)?;
    Ok(lock)
}

fn lock_fleet_file(paths: &EnsurePaths) -> Result<File, EnsureStateError> {
    let lock = lock_fleet_file_without_recovery(paths)?;
    clean_reinstall::cancellation::require_no_pending(paths)?;
    operation_selection::retirement::recover(paths)?;
    Ok(lock)
}

/// Acquire only the lock inode so exact-digest cancellation can inspect its own intent first.
pub(in crate::fleet_ensure::ops) fn lock_fleet_file_without_recovery(
    paths: &EnsurePaths,
) -> Result<File, EnsureStateError> {
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
    Ok(lock)
}

pub fn read_journal(
    paths: &EnsurePaths,
) -> Result<Option<FleetEnsureJournalRecord>, EnsureStateError> {
    let Some(bytes) = read_document_bytes(&paths.journal)? else {
        return Ok(None);
    };
    let journal = decode_journal(paths, &bytes)?;
    validate_schema(Some(journal), &paths.journal, |record| {
        record.schema_version
    })
}

/// Resolve content references before any current or archived journal is interpreted.
pub(super) fn decode_journal(
    paths: &EnsurePaths,
    bytes: &[u8],
) -> Result<FleetEnsureJournalRecord, EnsureStateError> {
    let decode = |source| EnsureStateError::Decode {
        path: paths.journal.clone(),
        source,
    };
    let mut projection: serde_json::Value = serde_json::from_slice(bytes).map_err(decode)?;
    let mut journal: FleetEnsureJournalRecord = if let Some(review) = projection
        .pointer_mut("/bootstrap_registration_recovery/review")
        .filter(|review| !review.is_null())
    {
        plan_content::hydrate(paths, review)?;
        serde_json::from_value(projection).map_err(decode)?
    } else {
        serde_json::from_slice(bytes).map_err(decode)?
    };
    continuation::hydrate_phases(paths, &mut journal.successor_phases)?;
    Ok(journal)
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

/// Remove untyped inventory names superseded by a current configured binding.
/// Unique dynamic assets and conflicting configured owners remain for verification.
pub(crate) fn retain_configured_principal_bindings(state: &mut FleetEnsureStateRecord) {
    let configured = state
        .topology
        .keys()
        .filter_map(|name| state.principals.get(name).cloned())
        .collect::<BTreeSet<_>>();
    state.principals.retain(|name, principal| {
        state.topology.contains_key(name) || !configured.contains(principal)
    });
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
    let mut startup_funding_by_root = startup_funding::resolve(root, desired)?;
    let continuation =
        continuation::resolve_authority(root, desired, &mut startup_funding_by_root)?;
    let mut artifacts = DesiredFleetArtifacts {
        continuation,
        startup_funding_by_root,
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

/// Consume reviewed publication authority in memory before workflow persists and issues it.
/// A failed persistence prevents the effect; uncertain persistence conservatively consumes it.
pub(crate) const fn reserve_fixture_publication_attempt(
    record: &mut EffectRecord,
    maximum_attempts: u32,
) -> bool {
    if record.publication_attempts >= maximum_attempts {
        return false;
    }
    record.publication_attempts += 1;
    true
}

pub fn write_journal(
    paths: &EnsurePaths,
    journal: &FleetEnsureJournalRecord,
) -> Result<(), EnsureStateError> {
    if journal
        .bootstrap_registration_recovery
        .as_ref()
        .is_some_and(|record| record.review.is_some())
    {
        let projection = infrastructure_bootstrap::registration_recovery::journal_projection(
            journal,
        )
        .map_err(|source| EnsureStateError::Decode {
            path: paths.journal.clone(),
            source,
        })?;
        return write_current(&paths.journal, &projection);
    }
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
