use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
    snapshot::root_baseline_snapshot_targets,
    topology::{wait_for_bootstrap, wait_for_children_ready, wait_for_snapshot_pids_ready},
};
use ic_testkit::pic::{
    BaselinePoolContractError, BaselinePreparationStage, CachedPocketIcBaseline,
    ControllerSnapshotError, FailureDisposition, PocketIcBaselineRecipe, PreparedBaseline,
    RebuildReason, ResetPolicies, ResetReceipt, ResetRequirements, SnapshotRestoreFunding,
    TimeResetPolicy, is_dead_pocket_ic_transport_error,
};
use std::{error::Error as StdError, fmt, path::PathBuf, time::Instant};

/// Typed lifecycle recipe for one reusable root topology profile.
pub struct RootBaselineRecipe {
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
    #[must_use]
    pub fn new(spec: RootBaselineSpec<'static>) -> Self {
        Self {
            reset_requirements: ResetRequirements {
                funding: SnapshotRestoreFunding::TopUpTo {
                    minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
                },
                policies: ResetPolicies {
                    pocket_ic_time: Some(TimeResetPolicy::PreserveCurrent),
                    ..ResetPolicies::default()
                },
            },
            spec,
        }
    }
}

impl PocketIcBaselineRecipe for RootBaselineRecipe {
    type Metadata = RootBaselineMetadata;
    type Error = RootBaselineRecipeError;

    fn reset_requirements(&self) -> &ResetRequirements {
        &self.reset_requirements
    }

    fn build(&self) -> Result<CachedPocketIcBaseline<Self::Metadata>, Self::Error> {
        let artifacts = super::ensure_root_release_artifacts_built(&self.spec);
        let root_wasm = super::load_root_wasm(&self.spec, &artifacts).ok_or_else(|| {
            RootBaselineRecipeError::MissingRootWasm(
                crate::pic::artifacts::retained_artifact_path(&artifacts, "root").to_owned(),
            )
        })?;
        Ok(build_root_cached_baseline(
            &self.spec, &artifacts, root_wasm,
        ))
    }

    fn prepare_restore(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<(), Self::Error> {
        crate::pic::canic::stop_snapshot_targets(
            baseline.pocket_ic(),
            &root_baseline_snapshot_targets(baseline.metadata()),
        );
        Ok(())
    }

    fn reset_non_snapshot_state(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<ResetReceipt, Self::Error> {
        let metadata = baseline.metadata();
        crate::pic::fleet_registry::reset_pool_assets(
            baseline.pocket_ic(),
            metadata.root_id,
            &metadata.pool_assets,
            &metadata.snapshot_pids,
        )
        .map_err(RootBaselineRecipeError::Invariant)?;
        Ok(ResetReceipt {
            policies: ResetPolicies {
                pocket_ic_time: Some(TimeResetPolicy::PreserveCurrent),
                ..ResetPolicies::default()
            },
        })
    }

    fn drive_to_readiness(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
    ) -> Result<(), Self::Error> {
        crate::pic::canic::start_snapshot_targets(
            baseline.pocket_ic(),
            &root_baseline_snapshot_targets(baseline.metadata()),
        );
        wait_for_bootstrap(
            &self.spec,
            baseline.pocket_ic(),
            baseline.metadata().root_id,
        );
        wait_for_children_ready(
            &self.spec,
            baseline.pocket_ic(),
            baseline.metadata().root_id,
            &baseline.metadata().component_canisters,
        );
        wait_for_snapshot_pids_ready(
            &self.spec,
            baseline.pocket_ic(),
            baseline.metadata().root_id,
            &baseline.metadata().snapshot_pids,
        );
        Ok(())
    }

    fn validate(
        &self,
        baseline: &CachedPocketIcBaseline<Self::Metadata>,
        _preparation: &PreparedBaseline,
    ) -> Result<(), Self::Error> {
        for canister_id in baseline.snapshot_canister_ids() {
            if !baseline.pocket_ic().canister_exists(canister_id) {
                return Err(RootBaselineRecipeError::Invariant(format!(
                    "captured root topology canister {canister_id} no longer exists"
                )));
            }
        }
        Ok(())
    }

    fn classify_failure(
        &self,
        stage: BaselinePreparationStage,
        error: &(dyn StdError + 'static),
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
    artifacts: &ic_build::artifacts::ArtifactCacheRecord,
    root_wasm: Vec<u8>,
) -> CachedPocketIcBaseline<RootBaselineMetadata> {
    let initialized = super::topology::setup_root_topology(spec, artifacts, root_wasm);
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
    crate::pic::canic::stop_snapshot_targets(
        baseline.pocket_ic(),
        &root_baseline_snapshot_targets(baseline.metadata()),
    );
    baseline
        .restore(SnapshotRestoreFunding::TopUpTo {
            minimum_cycles: crate::pic::SNAPSHOT_RESTORE_MINIMUM_CYCLES,
        })
        .expect("restore cached root snapshots");
    let metadata = baseline.metadata();
    crate::pic::fleet_registry::reset_pool_assets(
        baseline.pocket_ic(),
        metadata.root_id,
        &metadata.pool_assets,
        &metadata.snapshot_pids,
    )
    .expect("restore Root pool custody");
    crate::pic::canic::start_snapshot_targets(
        baseline.pocket_ic(),
        &root_baseline_snapshot_targets(metadata),
    );
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
        baseline.metadata().root_id,
        &baseline.metadata().component_canisters,
    );
    wait_for_snapshot_pids_ready(
        spec,
        baseline.pocket_ic(),
        baseline.metadata().root_id,
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
    crate::pic::canic::stop_snapshot_targets(&initialized.pic, &snapshot_targets);
    let baseline = CachedPocketIcBaseline::capture(
        ic_testkit::pic::PocketIcInstance::from_borrowed_server(initialized.pic),
        snapshot_targets,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    crate::pic::canic::start_snapshot_targets(
        baseline.pocket_ic(),
        &root_baseline_snapshot_targets(baseline.metadata()),
    );
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}
