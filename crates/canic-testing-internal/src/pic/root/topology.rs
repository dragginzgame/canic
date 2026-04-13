use super::{
    InitializedRootTopology, RootBaselineMetadata, RootBaselineSpec, progress, progress_elapsed,
};
use canic::{cdk::types::Principal, dto::topology::SubnetRegistryResponse, ids::CanisterRole};
use canic_testkit::pic::{Pic, PicBuilder, PicStartError};
use std::{collections::HashMap, time::Instant};

/// Install root, stage one ordinary release profile, resume bootstrap, and fetch the subnet map.
#[must_use]
pub fn setup_root_topology(
    spec: &RootBaselineSpec<'_>,
    root_wasm: Vec<u8>,
) -> InitializedRootTopology {
    for attempt in 1..=spec.root_setup_max_attempts {
        progress(
            spec,
            &format!(
                "initialize root setup attempt {attempt}/{}",
                spec.root_setup_max_attempts
            ),
        );
        let pic_started_at = Instant::now();
        let pic = match try_start_root_pic(spec) {
            Ok(pic) => {
                progress_elapsed(spec, "PocketIC instance ready", pic_started_at);
                pic
            }
            Err(err) if should_retry_root_pic_start(&err, spec, attempt) => {
                eprintln!(
                    "setup_root startup attempt {attempt}/{} failed; retrying: {err}",
                    spec.root_setup_max_attempts
                );
                continue;
            }
            Err(err) => {
                panic!(
                    "failed to start PocketIC instance for root baseline on attempt {attempt}/{}: {err}",
                    spec.root_setup_max_attempts
                );
            }
        };

        progress(spec, "installing root canister");
        let root_install_started_at = Instant::now();
        let root_id = pic
            .create_and_install_root_canister(root_wasm)
            .expect("install root canister");
        progress_elapsed(spec, "root canister installed", root_install_started_at);

        progress(spec, "staging managed release set");
        let stage_started_at = Instant::now();
        super::stage_managed_release_set(spec, &pic, root_id);
        progress_elapsed(spec, "staged managed release set", stage_started_at);

        progress(spec, "resuming root bootstrap");
        let resume_started_at = Instant::now();
        super::artifacts::resume_root_bootstrap(&pic, root_id);
        progress_elapsed(spec, "resumed root bootstrap", resume_started_at);

        progress(spec, "waiting for root bootstrap");
        let root_wait_started_at = Instant::now();
        wait_for_bootstrap(spec, &pic, root_id);
        progress_elapsed(spec, "root bootstrap ready", root_wait_started_at);

        progress(spec, "fetching subnet directory");
        let directory_started_at = Instant::now();
        let subnet_index = fetch_subnet_index(&pic, root_id);
        progress_elapsed(spec, "fetched subnet directory", directory_started_at);

        progress(spec, "waiting for child canisters ready");
        let child_wait_started_at = Instant::now();
        wait_for_children_ready(spec, &pic, &subnet_index);
        progress_elapsed(spec, "child canisters ready", child_wait_started_at);

        return InitializedRootTopology {
            pic,
            metadata: RootBaselineMetadata {
                root_id,
                subnet_index,
            },
        };
    }

    unreachable!("setup_root must return or panic")
}

// Wait until root reports `canic_ready`.
pub fn wait_for_bootstrap(spec: &RootBaselineSpec<'_>, pic: &Pic, root_id: Principal) {
    pic.wait_for_ready(root_id, spec.bootstrap_tick_limit, "root bootstrap");
}

// Wait until every child canister reports `canic_ready`.
pub fn wait_for_children_ready(
    spec: &RootBaselineSpec<'_>,
    pic: &Pic,
    subnet_index: &HashMap<CanisterRole, Principal>,
) {
    pic.wait_for_all_ready(
        subnet_index
            .iter()
            .filter(|(role, _)| !role.is_root())
            .map(|(_, pid)| *pid),
        spec.bootstrap_tick_limit,
        "root children bootstrap",
    );
}

// Start the PocketIC instance for one root baseline and retry only on the
// typed startup failures we explicitly treat as transient.
fn try_start_root_pic(spec: &RootBaselineSpec<'_>) -> Result<Pic, PicStartError> {
    progress(spec, "starting PocketIC instance");

    PicBuilder::new()
        .with_ii_subnet()
        .with_application_subnet()
        .try_build()
}

const fn should_retry_root_pic_start(
    err: &PicStartError,
    spec: &RootBaselineSpec<'_>,
    attempt: usize,
) -> bool {
    attempt < spec.root_setup_max_attempts
        && matches!(
            err,
            PicStartError::ServerStartFailed { .. } | PicStartError::StartupTimedOut { .. }
        )
}

// Fetch the authoritative subnet registry from root and project it into the
// role → principal map used by the root harness metadata.
fn fetch_subnet_index(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
    let registry: Result<SubnetRegistryResponse, canic::Error> = pic
        .query_call(root_id, canic::protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query subnet registry transport");

    let registry = registry.expect("query subnet registry application");

    registry
        .0
        .into_iter()
        .filter(|entry| !entry.role.is_root() && !entry.role.is_wasm_store())
        .map(|entry| (entry.role, entry.pid))
        .collect()
}
