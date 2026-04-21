use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
    topology::{wait_for_bootstrap, wait_for_children_ready},
};
use canic_testkit::pic::CachedPicBaseline;
use std::time::Instant;

/// Build one fresh root topology and capture immutable controller snapshots for cache reuse.
#[must_use]
pub fn build_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> CachedPicBaseline<RootBaselineMetadata> {
    let initialized = super::topology::setup_root_topology(spec, root_wasm);
    capture_cached_root_baseline(spec, initialized)
}

/// Restore one cached root topology and wait until root plus children are ready again.
pub fn restore_root_cached_baseline(
    spec: &RootBaselineSpec<'_>,
    baseline: &CachedPicBaseline<RootBaselineMetadata>,
) {
    progress(spec, "restoring cached root snapshots");
    let restore_started_at = Instant::now();
    baseline.restore(baseline.metadata().root_id);
    progress_elapsed(spec, "restored cached root snapshots", restore_started_at);

    progress(spec, "waiting for restored root bootstrap");
    let root_wait_started_at = Instant::now();
    wait_for_bootstrap(spec, baseline.pic(), baseline.metadata().root_id);
    progress_elapsed(spec, "restored root bootstrap ready", root_wait_started_at);

    progress(spec, "waiting for restored child canisters ready");
    let child_wait_started_at = Instant::now();
    wait_for_children_ready(spec, baseline.pic(), &baseline.metadata().subnet_index);
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
) -> CachedPicBaseline<RootBaselineMetadata> {
    let controller_ids = std::iter::once(initialized.metadata.root_id)
        .chain(initialized.metadata.subnet_index.values().copied())
        .chain(initialized.metadata.managed_store_pids.iter().copied())
        .collect::<Vec<_>>();

    progress(spec, "capturing cached root snapshots");
    let started_at = Instant::now();
    let baseline = CachedPicBaseline::capture(
        initialized.pic,
        initialized.metadata.root_id,
        controller_ids,
        initialized.metadata,
    )
    .expect("cached root snapshots must be available");
    progress_elapsed(spec, "captured cached root snapshots", started_at);
    baseline
}
