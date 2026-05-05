// Category C - Artifact / deployment test (embedded config).
// This test relies on embedded production config by design.

use crate::root_cached_support::{
    RootSetup, baseline_spec_for_roles_owned_env, setup_cached_root as setup_cached_root_common,
};
use canic_testing_internal::pic::RootBaselineMetadata;
use canic_testkit::{
    artifacts::{WasmBuildProfile, workspace_root_for},
    pic::CachedPicBaseline,
};
use std::sync::Mutex;

mod lifecycle_gap;

pub use lifecycle_gap::{reinstall_test_verifier, upgrade_user_shard_signer};

const ROOT_SHARDING_RELEASE_ROLES: &[&str] = &["test", "user_hub", "user_shard"];

static ROOT_SHARDING_BASELINE: Mutex<Option<CachedPicBaseline<RootBaselineMetadata>>> =
    Mutex::new(None);

/// Acquire the shared cached sharding root setup for delegation-flow tests.
pub fn setup_cached_root() -> RootSetup {
    setup_cached_root_common(
        "cached root sharding baseline",
        &ROOT_SHARDING_BASELINE,
        baseline_spec(),
    )
}

fn baseline_spec() -> canic_testing_internal::pic::RootBaselineSpec<'static> {
    let workspace_root = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
    let build_extra_env = vec![(
        "CANIC_CONFIG_PATH".to_string(),
        workspace_root
            .join("canisters/demo/test-configs/root-sharding.toml")
            .display()
            .to_string(),
    )];
    baseline_spec_for_roles_owned_env(
        workspace_root,
        ROOT_SHARDING_RELEASE_ROLES,
        WasmBuildProfile::Fast,
        build_extra_env,
    )
}
