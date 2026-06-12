//! Repo-only PocketIC fixtures layered on top of `ic-testkit`.

mod artifacts;
mod attestation;
mod audit;
mod canic;
mod delegation;
mod lifecycle;
mod root;

pub use artifacts::{CanicWasmBuildProfile, build_internal_test_wasm_canisters};
pub use attestation::{
    BaselinePicGuard, CachedInstalledRoot, install_test_root_cached,
    install_test_root_with_verifier_cached, install_test_root_without_test_material_cached,
    issuer_pid, wasm_store_pid,
};
pub use audit::{
    RootAuditProbeFixture, install_audit_leaf_probe, install_audit_root_probe,
    install_audit_scaling_probe,
};
pub use canic::{CanicPicExt, install_standalone_canister, role_pid, wait_until_ready};
pub use delegation::{
    create_user_shard, issue_delegated_token, obtain_root_delegation_proof, role_grant,
    token_ttl_within_proof,
};
pub use lifecycle::{
    LifecycleBoundaryFixture, install_lifecycle_boundary_fixture, invalid_init_args, upgrade_args,
};
pub use root::{
    RootBaselineMetadata, RootBaselineSpec, build_root_cached_baseline,
    ensure_root_release_artifacts_built, load_root_wasm, restore_root_cached_baseline,
    setup_root_topology,
};
