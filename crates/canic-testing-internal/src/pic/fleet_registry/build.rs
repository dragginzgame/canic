//! Test-Wasm and PocketIC builders for the prepared-root Registry journey.

use ic_testkit::artifacts::{
    ArtifactCacheOutcome, ArtifactCachePreparation, ArtifactCacheSpec, WasmBuildSpec,
    prepare_artifact_cache, resolve_cargo_build_inputs,
};
use ic_testkit::pic::{PocketIc, PocketIcBuilder};
#[cfg(test)]
use ic_testkit::pocket_ic::common::rest::{IcpFeatures, IcpFeaturesConfig};
#[cfg(test)]
use std::time::SystemTime;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use super::super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, InternalTestWasms,
    build_generated_fleet_wasm, build_internal_test_wasm_canisters_with_env,
    internal_test_artifact_build_target, internal_test_artifact_maintenance_interval,
    internal_test_artifact_prune_policy, report_artifact_cache_maintenance, retained_artifact_path,
};
use super::super::progress::{self as test_progress, ProgressStatus};
use super::super::startup::start_pocket_ic;
use super::fixture::{progress, progress_ready};

const ROOT_CANISTER_PACKAGE: &str = "delegation_root_stub";
#[cfg(test)]
const CYCLES_LEDGER_STUB_PACKAGE: &str = "cycles_ledger_stub";
#[cfg(test)]
const ICP_REFILL_STUB_PACKAGE: &str = "icp_refill_stub";
static BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static MAINNET_REFILL_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static MAINNET_FIVE_COMPONENT_REFILL_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static FIVE_COMPONENT_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static INITIAL_SHARD_WASM: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
#[cfg(test)]
static FIVE_TRILLION_COMPONENT_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static JOURNEY_LEDGER_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
#[cfg(test)]
static ICP_REFILL_STUB_BUILD_ONCE: OnceLock<InternalTestWasms> = OnceLock::new();
static CANISTER_BUILD_SERIAL: Mutex<()> = Mutex::new(());

// Build the test root wasm.
pub(super) fn build_test_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root).wasm("delegation_root_stub")
}

// Build a mainnet-qualified root and exact Cycles Ledger boundary stub.
#[cfg(test)]
pub(super) fn build_mainnet_refill_wasms() -> (Vec<u8>, Vec<u8>) {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("mainnet-refill");
    let wasms = MAINNET_REFILL_BUILD_ONCE.get_or_init(|| {
        let config_path = root_canister_config_path(&workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE, CYCLES_LEDGER_STUB_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env, ("ICP_ENVIRONMENT", "ic")],
        )
    });
    (
        wasms.wasm(ROOT_CANISTER_PACKAGE),
        wasms.wasm(CYCLES_LEDGER_STUB_PACKAGE),
    )
}

/// Build the exact five-Component fresh-pool Root and Cycles Ledger stub.
#[cfg(test)]
pub(super) fn build_mainnet_five_component_refill_wasms() -> (Vec<u8>, Vec<u8>) {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("mainnet-five-component-refill");
    let wasms = MAINNET_FIVE_COMPONENT_REFILL_BUILD_ONCE.get_or_init(|| {
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE, CYCLES_LEDGER_STUB_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env, ("ICP_ENVIRONMENT", "ic")],
        )
    });
    (
        wasms.wasm(ROOT_CANISTER_PACKAGE),
        wasms.wasm(CYCLES_LEDGER_STUB_PACKAGE),
    )
}

/// Build the exact local five-Component Root used for terminal activation.
#[cfg(test)]
pub(super) fn build_five_component_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("five-component");
    let wasms = FIVE_COMPONENT_BUILD_ONCE.get_or_init(|| {
        let config_path = five_component_root_canister_config_path(&workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env],
        )
    });
    wasms.wasm(ROOT_CANISTER_PACKAGE)
}

/// Build the existing audit Root with the managed-child fixture's exact config.
#[cfg(test)]
pub(super) fn build_child_reserve_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("child-reserve");
    let config_path = initial_shard_root_canister_config_path(&workspace_root);
    let wasms = build_internal_test_wasm_canisters_with_env(
        &workspace_root,
        &target_dir,
        &["root_probe"],
        CanicWasmBuildProfile::Fast,
        &[(
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        )],
    );
    wasms.wasm("root_probe")
}

/// Build the exact local Root whose only top-level Hub requires one initial Shard.
#[cfg(test)]
pub(super) fn build_initial_shard_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    INITIAL_SHARD_WASM
        .get_or_init(|| {
            crate::pic::artifacts::build_generated_fleet_wasm(
                &workspace_root,
                &initial_shard_root_canister_config_path(&workspace_root),
                "root",
                CanicWasmBuildProfile::Fast,
            )
        })
        .clone()
}

/// Build the one-Component Root whose retained recovery demand is exactly 5T.
#[cfg(test)]
pub(super) fn build_five_trillion_component_root_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("five-trillion-component");
    let wasms = FIVE_TRILLION_COMPONENT_BUILD_ONCE.get_or_init(|| {
        let config_path = five_trillion_component_root_canister_config_path(&workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env],
        )
    });
    wasms.wasm(ROOT_CANISTER_PACKAGE)
}

/// Build the deterministic Cycles Ledger boundary used by the literal-zero journey.
#[cfg(test)]
pub(super) fn build_journey_cycles_ledger_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("journey-ledger");
    let wasms = JOURNEY_LEDGER_BUILD_ONCE.get_or_init(|| {
        let config_path = root_canister_config_path(&workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[CYCLES_LEDGER_STUB_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env],
        )
    });
    wasms.wasm(CYCLES_LEDGER_STUB_PACKAGE)
}

/// Build the exact ICP Ledger/CMC boundary stub without rebuilding a production Root.
#[cfg(test)]
pub(super) fn build_icp_refill_stub_wasm() -> Vec<u8> {
    let workspace_root = workspace_root();
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let target_dir = test_target_dir(&workspace_root).join("icp-refill-stub");
    let wasms = ICP_REFILL_STUB_BUILD_ONCE.get_or_init(|| {
        build_internal_test_wasm_canisters_with_env(
            &workspace_root,
            &target_dir,
            &[ICP_REFILL_STUB_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[("ICP_ENVIRONMENT", "ic")],
        )
    });
    wasms.wasm(ICP_REFILL_STUB_PACKAGE)
}

// Build and read the exact release-qualified sibling wasm_store artifact.
pub(super) fn build_test_wasm_store_wasm() -> Vec<u8> {
    static WASM: OnceLock<Vec<u8>> = OnceLock::new();
    let workspace_root = workspace_root();
    WASM.get_or_init(|| {
        let _serial_guard = CANISTER_BUILD_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        build_bootstrap_wasm_store(&workspace_root, &root_canister_config_path(&workspace_root))
    })
    .clone()
}

// Build one independent PocketIC instance for a Fleet Registry fixture.
pub(super) fn build_pic() -> PocketIc {
    progress("starting PocketIC instance");
    let pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_ii_subnet()
            .with_application_subnet(),
    );
    progress_ready("PocketIC instance");
    pic
}

// Build one Fleet fixture whose HTTP gateway exposes an exact local root key
// for production-agent transport qualification.
#[cfg(test)]
pub(super) fn build_management_pic() -> PocketIc {
    progress("starting management-agent PocketIC instance");
    let pic = start_pocket_ic(
        // The HTTP gateway advances to wall time. Create funded assets on that
        // clock so starting the gateway cannot charge years of artificial idle burn.
        PocketIcBuilder::new()
            .with_initial_time(SystemTime::now().into())
            .with_nns_subnet()
            .with_ii_subnet()
            .with_application_subnet(),
    );
    progress_ready("management-agent PocketIC instance");
    pic
}

// Build one Fleet fixture with two distinct application Subnets.
#[cfg(test)]
pub(super) fn build_two_root_pic() -> PocketIc {
    progress("starting two-Root PocketIC instance");
    let pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_ii_subnet()
            .with_application_subnet()
            .with_application_subnet(),
    );
    progress_ready("two-Root PocketIC instance");
    pic
}

// Build one PocketIC instance with its production ICP Ledger and CMC system
// canisters. This is the value-transfer fixture for Root ICP refill journeys;
// repository stubs remain available only for deterministic adapter tests.
#[cfg(test)]
pub(super) fn build_icp_refill_pic() -> PocketIc {
    progress("starting PocketIC instance with ICP Ledger and CMC");
    let default_config = Some(IcpFeaturesConfig::DefaultConfig);
    let pic = start_pocket_ic(
        PocketIcBuilder::new()
            .with_ii_subnet()
            .with_application_subnet()
            .with_icp_features(IcpFeatures {
                cycles_minting: default_config.clone(),
                icp_token: default_config,
                ..IcpFeatures::default()
            }),
    );
    progress_ready("PocketIC instance with ICP Ledger and CMC");
    pic
}

// Build the test canisters once for the shared Fleet Registry fixtures.
fn build_canisters_once(workspace_root: &Path) -> &'static InternalTestWasms {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_ONCE.get_or_init(|| {
        let target_dir = test_target_dir(workspace_root);
        let config_path = root_canister_config_path(workspace_root);
        let canonical_config_env = (
            canic_core::role_contract::CANONICAL_BUILD_CONFIG_PATH_ENV,
            config_path.to_str().expect("config path UTF-8"),
        );
        progress("building PIC root wasm artifact");
        build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &[ROOT_CANISTER_PACKAGE],
            CanicWasmBuildProfile::Fast,
            &[canonical_config_env],
        )
    })
}

// Build the sibling wasm_store independently before the fixture installs both Canisters.
fn build_bootstrap_wasm_store(workspace_root: &Path, config_path: &Path) -> Vec<u8> {
    let target_dir = canic_host::canister_build::canister_build_target_root(workspace_root);
    let artifact_path = workspace_root
        .join(".canic/release-builds")
        .join(INTERNAL_TEST_RELEASE_BUILD_ID.1)
        .join("artifacts/wasm_store/wasm_store.wasm.gz");
    let config_relative = config_path
        .strip_prefix(workspace_root)
        .expect("bootstrap Store config must be workspace-confined")
        .to_str()
        .expect("bootstrap Store config path UTF-8");
    let cargo_build = WasmBuildSpec::new(
        workspace_root,
        &target_dir,
        &["canic", "canic-host"],
        CanicWasmBuildProfile::Fast.target_dir_name(),
    )
    .with_cargo_profile_args(["--profile", "fast", "--locked"])
    .with_extra_env([
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ]);
    let cargo_inputs = resolve_cargo_build_inputs(&cargo_build)
        .expect("resolve bootstrap Store Cargo build inputs");
    let cache = ArtifactCacheSpec::new(
        &workspace_root.join("target/test-artifacts/external-artifact-cache"),
        "bootstrap-wasm-store",
        "canic/bootstrap-wasm-store/v1",
    )
    .with_coordination_scope("canic-external-artifact-builds")
    .with_arguments([
        "canic-host::build_workspace_canister_artifact",
        "wasm_store",
        "fast",
        config_relative,
    ])
    .with_environment([
        ("CARGO_INCREMENTAL", "0"),
        ("ICP_ENVIRONMENT", "local"),
        INTERNAL_TEST_RELEASE_BUILD_ID,
    ])
    .with_input("build-config", config_path)
    .with_input(
        "fixture-builder",
        &workspace_root.join("crates/canic-testing-internal/src/pic/artifacts.rs"),
    )
    .with_input(
        "store-fixture-builder",
        &workspace_root.join("crates/canic-testing-internal/src/pic/fleet_registry/build.rs"),
    )
    .with_input("icp-config", &workspace_root.join("icp.yaml"))
    .with_cargo_build_inputs("bootstrap-store-cargo", &cargo_build, &cargo_inputs)
    .with_output("wasm_store", &artifact_path)
    .with_prune_policy_at_most_every(
        internal_test_artifact_prune_policy(),
        internal_test_artifact_maintenance_interval(),
    );

    let outcome = match prepare_artifact_cache(&cache).expect("prepare bootstrap Store cache") {
        ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
        ArtifactCachePreparation::Build(transaction) => {
            build_generated_fleet_wasm(
                workspace_root,
                config_path,
                "wasm_store",
                CanicWasmBuildProfile::Fast,
            );
            transaction
                .import_output("wasm_store", &artifact_path)
                .expect("import bootstrap Store artifact");
            transaction
                .commit()
                .expect("commit bootstrap Store artifact cache")
        }
    };
    test_progress::timed(
        "WASM",
        if outcome.is_reused() {
            ProgressStatus::Cache
        } else {
            ProgressStatus::Done
        },
        "bootstrap Store artifact",
        outcome.record().timings().total(),
    );
    test_progress::detail("WASM", &format!("bootstrap Store cache: {outcome}"));
    report_artifact_cache_maintenance("bootstrap-wasm-store", outcome.record().maintenance());
    fs::read(retained_artifact_path(outcome.record(), "wasm_store"))
        .expect("read retained bootstrap Store")
}

// Resolve the one canonical Fleet config used by every managed fixture wasm.
pub(super) fn root_canister_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("canisters")
        .join("test")
        .join(ROOT_CANISTER_PACKAGE)
        .join("canic.toml")
}

/// Resolve the exact five-Component fresh-pool qualification config.
#[cfg(test)]
pub(super) fn five_component_root_canister_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("canisters")
        .join("test")
        .join(ROOT_CANISTER_PACKAGE)
        .join("canic.five-components.toml")
}

/// Resolve the exact one-Hub/one-initial-Shard qualification config.
#[cfg(test)]
pub(super) fn initial_shard_root_canister_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("apps")
        .join("test")
        .join("test-configs")
        .join("managed-component-group.toml")
}

/// Resolve the exact one-Component 5T retained-recovery qualification config.
#[cfg(test)]
pub(super) fn five_trillion_component_root_canister_config_path(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("canisters")
        .join("test")
        .join(ROOT_CANISTER_PACKAGE)
        .join("canic.five-trillion-component.toml")
}

// Resolve the shared PocketIC wasm target directory.
fn test_target_dir(workspace_root: &Path) -> PathBuf {
    internal_test_artifact_build_target(workspace_root)
}

// Resolve the canic workspace root from the internal test crate manifest dir.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}
