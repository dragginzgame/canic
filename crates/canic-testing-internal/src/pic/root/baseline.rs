use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
    topology::{wait_for_bootstrap, wait_for_children_ready, wait_for_snapshot_pids_ready},
};
use ic_testkit::pic::{CachedPocketIcBaseline, SnapshotRestoreFunding};
use std::{collections::BTreeSet, time::Instant};

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
    let controller_ids = root_baseline_controller_ids(&initialized.metadata);

    progress(spec, "capturing cached root snapshots");
    let started_at = Instant::now();
    let baseline = CachedPocketIcBaseline::capture(
        initialized.pic,
        initialized.metadata.root_id,
        controller_ids,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}

fn root_baseline_controller_ids(metadata: &RootBaselineMetadata) -> Vec<candid::Principal> {
    std::iter::once(metadata.root_id)
        .chain(metadata.snapshot_pids.iter().copied())
        .chain(metadata.managed_store_pids.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn root_baseline_controller_ids_are_unique() {
        let root_id = candid::Principal::from_slice(&[0x41; 29]);
        let child_id = candid::Principal::from_slice(&[0x42; 29]);
        let store_id = candid::Principal::from_slice(&[0x43; 29]);
        let metadata = RootBaselineMetadata {
            root_id,
            component_canisters: HashMap::new(),
            snapshot_pids: vec![child_id, store_id],
            managed_store_pids: vec![store_id, root_id],
        };

        let controller_ids = root_baseline_controller_ids(&metadata);

        assert_eq!(controller_ids.len(), 3);
        assert!(controller_ids.contains(&root_id));
        assert!(controller_ids.contains(&child_id));
        assert!(controller_ids.contains(&store_id));
    }
}
