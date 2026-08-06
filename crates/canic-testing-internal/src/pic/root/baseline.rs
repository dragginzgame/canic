use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
    topology::{wait_for_bootstrap, wait_for_children_ready, wait_for_snapshot_pids_ready},
};
use ic_testkit::pic::{
    BaselinePoolContractError, BaselinePreparationStage, CachedPocketIcBaseline,
    CanisterRestoreReceipt, CanisterSnapshotTarget, ControllerSnapshotError, CycleResetPolicy,
    FailureDisposition, FixtureRecipeId, PocketIcBaselineRecipe, PreparedBaseline,
    ReadinessReceipt, RebuildReason, ResetAchievement, ResetReceipt, ResetRequirement,
    ResetRequirements, SnapshotRestoreFunding, TimeResetPolicy, ValidationReceipt,
    is_dead_pocket_ic_transport_error,
};
use std::{collections::BTreeMap, error::Error as StdError, fmt, path::PathBuf, time::Instant};

/// Typed lifecycle recipe for one reusable root topology profile.
pub struct RootBaselineRecipe {
    id: FixtureRecipeId,
    reset_requirements: ResetRequirements,
    spec: RootBaselineSpec<'static>,
}

/// Structured failure while preparing one pooled root topology.
#[derive(Debug)]
pub enum RootBaselineRecipeError {
    /// The root artifact set did not contain the expected root Wasm.
    MissingRootWasm(PathBuf),
    /// The testkit rejected recipe or reset evidence.
    Contract(BaselinePoolContractError),
    /// Capturing or restoring a controller snapshot failed.
    Snapshot(ControllerSnapshotError),
    /// A post-restore root topology invariant failed.
    Invariant(String),
}

impl RootBaselineRecipe {
    /// Construct one profile-specific root baseline recipe.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity or mandatory reset contract is invalid.
    pub fn try_new(
        identity: impl Into<String>,
        spec: RootBaselineSpec<'static>,
    ) -> Result<Self, BaselinePoolContractError> {
        Ok(Self {
            id: FixtureRecipeId::try_new(identity)?,
            reset_requirements: ResetRequirements::try_new([
                ResetRequirement::CanisterSnapshots,
                ResetRequirement::CanisterCycles(CycleResetPolicy::TopUpTo(
                    crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
                )),
                ResetRequirement::PocketIcTime(TimeResetPolicy::PreserveCurrent),
            ])?,
            spec,
        })
    }
}

impl PocketIcBaselineRecipe for RootBaselineRecipe {
    type Metadata = RootBaselineMetadata;
    type Error = RootBaselineRecipeError;

    fn id(&self) -> &FixtureRecipeId {
        &self.id
    }

    fn reset_requirements(&self) -> &ResetRequirements {
        &self.reset_requirements
    }

    fn build(&self) -> Result<CachedPocketIcBaseline<Self::Metadata>, Self::Error> {
        super::ensure_root_release_artifacts_built(&self.spec);
        let root_wasm = super::load_root_wasm(&self.spec).ok_or_else(|| {
            RootBaselineRecipeError::MissingRootWasm(self.spec.root_wasm_path.clone())
        })?;
        Ok(build_root_cached_baseline(&self.spec, root_wasm))
    }

    fn restore_canisters(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<CanisterRestoreReceipt, Self::Error> {
        baseline.restore_with_funding(
            baseline.metadata().root_id,
            SnapshotRestoreFunding::TopUpTo {
                minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
            },
        )?;
        CanisterRestoreReceipt::try_from_baseline(
            baseline,
            CycleResetPolicy::TopUpTo(crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES),
        )
        .map_err(Into::into)
    }

    fn reset_non_snapshot_state(
        &self,
        _baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<ResetReceipt, Self::Error> {
        ResetReceipt::try_new([ResetAchievement::PocketIcTime(
            TimeResetPolicy::PreserveCurrent,
        )])
        .map_err(Into::into)
    }

    fn drive_to_readiness(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<ReadinessReceipt, Self::Error> {
        wait_for_bootstrap(
            &self.spec,
            baseline.pocket_ic(),
            baseline.metadata().root_id,
        );
        wait_for_children_ready(
            &self.spec,
            baseline.pocket_ic(),
            &baseline.metadata().component_canisters,
        );
        wait_for_snapshot_pids_ready(
            &self.spec,
            baseline.pocket_ic(),
            &baseline.metadata().snapshot_pids,
        );
        ReadinessReceipt::try_new("root-and-managed-children-ready").map_err(Into::into)
    }

    fn validate(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
        _preparation: &PreparedBaseline,
    ) -> Result<ValidationReceipt, Self::Error> {
        for canister_id in baseline.snapshot_canister_ids() {
            if !baseline.pocket_ic().canister_exists(canister_id) {
                return Err(RootBaselineRecipeError::Invariant(format!(
                    "captured root topology canister {canister_id} no longer exists"
                )));
            }
        }
        ValidationReceipt::try_new(self.id.clone(), "captured-root-topology-exists")
            .map_err(Into::into)
    }

    fn classify_failure(
        &self,
        stage: BaselinePreparationStage,
        error: &Self::Error,
    ) -> FailureDisposition {
        if is_dead_pocket_ic_transport_error(error) {
            FailureDisposition::Rebuild(RebuildReason::DeadPocketIcTransport)
        } else {
            FailureDisposition::Rebuild(stage.default_rebuild_reason())
        }
    }
}

impl From<BaselinePoolContractError> for RootBaselineRecipeError {
    fn from(error: BaselinePoolContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<ControllerSnapshotError> for RootBaselineRecipeError {
    fn from(error: ControllerSnapshotError) -> Self {
        Self::Snapshot(error)
    }
}

impl fmt::Display for RootBaselineRecipeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRootWasm(path) => {
                write!(formatter, "missing root Wasm at {}", path.display())
            }
            Self::Contract(error) => error.fmt(formatter),
            Self::Snapshot(error) => error.fmt(formatter),
            Self::Invariant(message) => formatter.write_str(message),
        }
    }
}

impl StdError for RootBaselineRecipeError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::MissingRootWasm(_) | Self::Invariant(_) => None,
            Self::Contract(error) => Some(error),
            Self::Snapshot(error) => Some(error),
        }
    }
}

/// Build one fresh root topology and capture immutable controller snapshots for cache reuse.
#[must_use]
pub fn build_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> CachedPocketIcBaseline<RootBaselineMetadata> {
    let initialized = super::topology::setup_root_topology(spec, root_wasm);
    capture_cached_root_baseline(spec, initialized)
}

/// Restore one cached root topology and wait until root plus children are ready again.
///
/// # Panics
///
/// Panics if PocketIC cannot restore the captured snapshots or the restored
/// root and children do not become ready within the configured tick limit.
pub fn restore_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    baseline: &CachedPocketIcBaseline<RootBaselineMetadata>,
) {
    progress(spec, "restoring cached root snapshots");
    let restore_started_at = Instant::now();
    baseline
        .restore_with_funding(
            baseline.metadata().root_id,
            SnapshotRestoreFunding::TopUpTo {
                minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
            },
        )
        .expect("restore cached root snapshots");
    progress_elapsed(spec, "restored cached root snapshots", restore_started_at);

    progress(spec, "waiting for restored root bootstrap");
    let root_wait_started_at = Instant::now();
    wait_for_bootstrap(spec, baseline.pocket_ic(), baseline.metadata().root_id);
    progress_elapsed(spec, "restored root bootstrap ready", root_wait_started_at);

    progress(spec, "waiting for restored child canisters ready");
    let child_wait_started_at = Instant::now();
    wait_for_children_ready(
        spec,
        baseline.pocket_ic(),
        &baseline.metadata().component_canisters,
    );
    wait_for_snapshot_pids_ready(
        spec,
        baseline.pocket_ic(),
        &baseline.metadata().snapshot_pids,
    );
    progress_elapsed(
        spec,
        "restored child canisters ready",
        child_wait_started_at,
    );
}

// Capture the immutable root + child controller snapshots for one initialized topology.
fn capture_cached_root_baseline(
    spec: &RootBaselineSpec<'_>,
    initialized: InitializedRootTopology,
) -> CachedPocketIcBaseline<RootBaselineMetadata> {
    let snapshot_targets = root_baseline_snapshot_targets(&initialized.metadata);

    progress(spec, "capturing cached root snapshots");
    let started_at = Instant::now();
    let baseline = CachedPocketIcBaseline::capture_with_senders(
        initialized.pic,
        snapshot_targets,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}

fn root_baseline_snapshot_targets(metadata: &RootBaselineMetadata) -> Vec<CanisterSnapshotTarget> {
    let mut senders = BTreeMap::from([(metadata.root_id, None)]);
    for canister_id in metadata
        .snapshot_pids
        .iter()
        .chain(&metadata.managed_store_pids)
    {
        senders
            .entry(*canister_id)
            .or_insert(Some(metadata.root_id));
    }
    senders
        .into_iter()
        .map(|(canister_id, sender)| CanisterSnapshotTarget::new(canister_id, sender))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn root_baseline_snapshot_targets_are_unique_and_controller_exact() {
        let root_id = candid::Principal::from_slice(&[0x41; 29]);
        let child_id = candid::Principal::from_slice(&[0x42; 29]);
        let store_id = candid::Principal::from_slice(&[0x43; 29]);
        let metadata = RootBaselineMetadata {
            root_id,
            component_canisters: HashMap::new(),
            snapshot_pids: vec![child_id, store_id],
            managed_store_pids: vec![store_id, root_id],
        };

        let targets = root_baseline_snapshot_targets(&metadata);

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].canister_id(), root_id);
        assert_eq!(targets[0].sender(), None);
        assert_eq!(targets[1].canister_id(), child_id);
        assert_eq!(targets[1].sender(), Some(root_id));
        assert_eq!(targets[2].canister_id(), store_id);
        assert_eq!(targets[2].sender(), Some(root_id));
    }
}
