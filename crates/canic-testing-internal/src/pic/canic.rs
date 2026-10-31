use super::{
    artifacts::{
        CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, build_internal_test_wasm_canisters,
    },
    startup::start_pocket_ic,
};
use canic::{
    Error,
    dto::{
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreAdoptionRequest,
            FleetSubnetWasmStoreInitArgs,
        },
        runtime::ReadinessStatus,
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, CyclesFundingBudget, FleetCoordinatorBinding,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreActivationAuthority,
        FleetSubnetWasmStoreAuthority, ReleaseSetDigest, SubnetId,
    },
};

use canic_contracts::protocol::initialization::encode;
use canic_contracts::{
    cycles::Cycles,
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId},
};
use canic_core::cdk::utils::hash::wasm_hash;
use canic_host::release_set::AppConfigSnapshot;
use ic_platform::principal::ExternalPrincipal as Principal;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};
use {
    ic_platform::pocket_ic::{PocketIc, PocketIcBuilder},
    ic_testkit::pic::{
        CanisterDiagnosticsRequest, CanisterInstallExt, InstallSpec,
        LabeledCanisterDiagnosticsRequest, PocketIcDiagnosticsExt, StandaloneCanisterFixture,
    },
};

const INSTALL_CYCLES: u128 = 500_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;

/// Drain outstanding callbacks before capturing or restoring a fixture snapshot set.
///
/// # Panics
/// Panics if a captured controller cannot stop one of its canisters.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn stop_snapshot_targets(
    pic: &PocketIc,
    targets: &[ic_testkit::pic::CanisterSnapshotTarget],
) {
    for target in targets {
        pic.stop_canister(target.canister_id, target.sender)
            .expect("stop fixture snapshot target and drain callbacks");
    }
}

/// Resume a fixture only after its complete captured set has been prepared.
///
/// # Panics
/// Panics if a captured controller cannot restart one of its canisters.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn start_snapshot_targets(
    pic: &PocketIc,
    targets: &[ic_testkit::pic::CanisterSnapshotTarget],
) {
    for target in targets {
        pic.start_canister(target.canister_id, target.sender)
            .expect("restart prepared fixture snapshot target");
    }
}

///
/// ManagedRootInstallInput
///
/// Complete test-only authority and artifact input for one managed root installation.
///

pub(super) struct ManagedRootInstallInput<'a> {
    pub root_id: Principal,
    pub wasm_store: Principal,
    pub installation_controller: Principal,
    pub coordinator: Principal,
    pub root_wasm: &'a [u8],
    pub wasm_store_wasm: &'a [u8],
    pub config_path: &'a Path,
    pub release_set_digest: ReleaseSetDigest,
}

pub(super) struct PreparedManagedRoot {
    pub root_id: Principal,
    pub root_args: FleetSubnetRootInitArgs,
}

pub(super) fn create_and_install_pre_adoption_root(
    pic: &PocketIc,
    root_wasm: Vec<u8>,
    wasm_store_wasm: Vec<u8>,
    config_path: &Path,
) -> Result<PreparedManagedRoot, Error> {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, INSTALL_CYCLES);
    let wasm_store = pic.create_canister();
    pic.add_cycles(wasm_store, INSTALL_CYCLES);
    let installation_controller = Principal::from_slice(&[0x46; 29]);
    let root_args = managed_test_root_init_args(ManagedRootInstallInput {
        root_id,
        wasm_store,
        installation_controller,
        coordinator: Principal::from_slice(&[0x41; 29]),
        root_wasm: &root_wasm,
        wasm_store_wasm: &wasm_store_wasm,
        config_path,
        release_set_digest: ReleaseSetDigest::from_bytes([0x44; 32]),
    })?;
    let store_args = FleetSubnetWasmStoreInitArgs {
        authority: root_args.authority.wasm_store_authority.clone(),
        install_id: root_args.wasm_store_activation.operation_id,
    };
    assert_ne!(root_args.install_id, store_args.install_id);
    prepare_sibling_wasm_store_controllers(pic, wasm_store, installation_controller, root_id);
    pic.install_canister(
        wasm_store,
        wasm_store_wasm,
        encode(store_args).map_err(|_| {
            Error::from_registered(canic_contracts::diagnostics::codes::STATE_FAILED)
        })?,
        Some(installation_controller),
    );
    pic.install_canister(
        root_id,
        root_wasm,
        encode(&root_args).map_err(|_| {
            Error::from_registered(canic_contracts::diagnostics::codes::STATE_FAILED)
        })?,
        None,
    );
    Ok(PreparedManagedRoot { root_id, root_args })
}

///
/// CanicPicExt
///

pub trait CanicPicExt {
    /// Install a root Canic canister with authority compiled from its exact build config.
    fn create_and_install_root_canister(
        &self,
        root_wasm: Vec<u8>,
        wasm_store_wasm: Vec<u8>,
        config_path: &Path,
    ) -> Result<Principal, Error>;

    /// Wait until one ordinary managed canister reports `canic_readiness`.
    fn wait_for_ready(
        &self,
        canister_id: Principal,
        diagnostic_sender: Principal,
        tick_limit: usize,
        context: &str,
    );

    /// Wait until one Root reports `canic_readiness`.
    fn wait_for_root_ready(
        &self,
        canister_id: Principal,
        diagnostic_sender: Principal,
        tick_limit: usize,
        context: &str,
    );

    /// Wait until all provided ordinary managed canisters report readiness.
    fn wait_for_all_ready<I>(&self, targets: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = (Principal, Principal)>;
}

impl CanicPicExt for PocketIc {
    fn create_and_install_root_canister(
        &self,
        root_wasm: Vec<u8>,
        wasm_store_wasm: Vec<u8>,
        config_path: &Path,
    ) -> Result<Principal, Error> {
        let prepared =
            create_and_install_pre_adoption_root(self, root_wasm, wasm_store_wasm, config_path)?;
        adopt_sibling_wasm_store(self, prepared.root_id, &prepared.root_args);
        Ok(prepared.root_id)
    }

    fn wait_for_ready(
        &self,
        canister_id: Principal,
        diagnostic_sender: Principal,
        tick_limit: usize,
        context: &str,
    ) {
        for _ in 0..tick_limit {
            self.tick();
            match fetch_ready(self, canister_id, diagnostic_sender) {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    report_canister_diagnostics(self, canister_id, diagnostic_sender, context);
                    panic!("{context}: canister {canister_id} readiness query failed: {error:?}");
                }
            }
        }

        report_canister_diagnostics(self, canister_id, diagnostic_sender, context);
        panic!("{context}: canister {canister_id} did not become ready after {tick_limit} ticks");
    }

    fn wait_for_root_ready(
        &self,
        canister_id: Principal,
        diagnostic_sender: Principal,
        tick_limit: usize,
        context: &str,
    ) {
        for _ in 0..tick_limit {
            self.tick();
            match fetch_ready(self, canister_id, diagnostic_sender) {
                Ok(true) => return,
                Ok(false) => {}
                Err(error) => {
                    report_canister_diagnostics(self, canister_id, diagnostic_sender, context);
                    panic!("{context}: Root {canister_id} readiness query failed: {error:?}");
                }
            }
        }

        report_canister_diagnostics(self, canister_id, diagnostic_sender, context);
        panic!("{context}: Root {canister_id} did not become ready after {tick_limit} ticks");
    }

    fn wait_for_all_ready<I>(&self, targets: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = (Principal, Principal)>,
    {
        let targets = targets.into_iter().collect::<Vec<_>>();

        for _ in 0..tick_limit {
            self.tick();
            let mut all_ready = true;
            let mut failures = Vec::new();
            for (canister_id, caller) in &targets {
                match fetch_ready(self, *canister_id, *caller) {
                    Ok(ready) => all_ready &= ready,
                    Err(error) => failures.push(format!("{canister_id}: {error:?}")),
                }
            }
            if !failures.is_empty() {
                report_canister_diagnostic_targets(self, &targets, context);
                panic!(
                    "{context}: readiness queries failed:\n{}",
                    failures.join("\n")
                );
            }
            if all_ready {
                return;
            }
        }

        report_canister_diagnostic_targets(self, &targets, context);
        panic!("{context}: canisters did not become ready after {tick_limit} ticks");
    }
}

/// Print bounded status and log diagnostics using the canister's exact controller.
pub fn report_canister_diagnostics(
    pic: &PocketIc,
    canister_id: Principal,
    diagnostic_sender: Principal,
    context: &str,
) {
    let report = pic.collect_canister_diagnostics(CanisterDiagnosticsRequest {
        canister_id,
        status_sender: diagnostic_sender,
        log_sender: diagnostic_sender,
        log_limits: ic_testkit::pic::CanisterLogRenderLimits::default(),
    });
    eprintln!("{context}: {report}");
}

fn report_canister_diagnostic_targets(
    pic: &PocketIc,
    targets: &[(Principal, Principal)],
    context: &str,
) {
    report_canister_diagnostics_batch(
        pic,
        targets.iter().map(|(canister_id, diagnostic_sender)| {
            (canister_id.to_string(), *canister_id, *diagnostic_sender)
        }),
        context,
    );
}

/// Print a labeled collect-all diagnostic batch with exact controllers.
pub fn report_canister_diagnostics_batch<I, L>(pic: &PocketIc, targets: I, context: &str)
where
    I: IntoIterator<Item = (L, Principal, Principal)>,
    L: Into<String>,
{
    let requests = targets
        .into_iter()
        .map(
            |(label, canister_id, diagnostic_sender)| LabeledCanisterDiagnosticsRequest {
                label: label.into(),
                request: CanisterDiagnosticsRequest {
                    canister_id,
                    status_sender: diagnostic_sender,
                    log_sender: diagnostic_sender,
                    log_limits: ic_testkit::pic::CanisterLogRenderLimits::default(),
                },
            },
        )
        .collect::<Vec<_>>();
    match pic.collect_canister_diagnostics_batch(&requests) {
        Ok(report) => eprintln!("{context}: {report}"),
        Err(error) => {
            eprintln!("{context}: diagnostic batch rejected before collection: {error}");
        }
    }
}

pub(super) fn prepare_sibling_wasm_store_controllers(
    pic: &PocketIc,
    wasm_store: Principal,
    installation_controller: Principal,
    root: Principal,
) {
    let mut controllers = vec![installation_controller, root];
    controllers.sort();
    pic.set_controllers(wasm_store, None, controllers.clone())
        .expect("prepare sibling Wasm Store controllers");

    let mut observed_controllers = pic
        .canister_status(wasm_store, Some(installation_controller))
        .expect("observe prepared sibling Wasm Store controllers")
        .settings
        .controllers;
    observed_controllers.sort();
    assert_eq!(observed_controllers, controllers);
}

pub(super) fn adopt_sibling_wasm_store(
    pic: &PocketIc,
    root: Principal,
    root_args: &FleetSubnetRootInitArgs,
) {
    let request = FleetSubnetWasmStoreAdoptionRequest {
        operation_id: root_store_adoption_operation_id(root_args.install_id),
        authority: root_args.authority.wasm_store_authority.clone(),
    };
    let receipt = crate::pic::canic::root_store_adopt(pic, root, request.clone())
        .expect("adopt sibling Wasm Store application");
    assert_eq!(receipt.operation_id, request.operation_id);
    let adopted = crate::pic::canic::root_store_adoption_status(pic, root, request.operation_id)
        .expect("query sibling Wasm Store adoption application");
    assert_eq!(adopted.authority, request.authority);
    let mut expected_controllers = vec![root, request.authority.installation_controller];
    expected_controllers.sort();
    assert_eq!(adopted.controllers, expected_controllers);
    let mut live_controllers = pic
        .canister_status(request.authority.wasm_store, Some(root))
        .expect("observe adopted sibling Wasm Store controllers");
    live_controllers.settings.controllers.sort();
    assert_eq!(live_controllers.settings.controllers, expected_controllers);
    let status = crate::pic::canic::root_store_adoption_status(pic, root, request.operation_id)
        .expect("query sibling Wasm Store adoption application");
    assert_eq!(status, adopted);
}

// Match the host install journal's distinct operation identity for Store adoption.
fn root_store_adoption_operation_id(install_operation_id: [u8; 32]) -> [u8; 32] {
    let mut input = b"canic.fleet-install.root-operation.v1\0store-adoption\0".to_vec();
    input.extend_from_slice(&install_operation_id);
    let mut operation_id: [u8; 32] = wasm_hash(&input)
        .try_into()
        .expect("SHA-256 Store-adoption operation identity");
    if operation_id == [0; 32] {
        operation_id[31] = 1;
    }
    operation_id
}

/// Wait until one Root reports `canic_readiness`.
///
/// # Panics
///
/// Panics if the canister does not report ready within `tick_limit` ticks, or
/// if querying readiness traps.
pub fn wait_until_ready(pic: &PocketIc, canister_id: Principal, tick_limit: usize) {
    for _ in 0..tick_limit {
        if fetch_ready(pic, canister_id, Principal::anonymous())
            .expect("controller readiness query")
        {
            return;
        }
        pic.tick();
    }
    panic!("canister did not report ready in time: {canister_id}");
}

/// Install one non-root Canic canister into a fresh PocketIC instance.
///
/// The installed canister uses the explicit `start_local!` lifecycle and the
/// internal test endpoint surface for that test build.
///
/// # Panics
///
/// Panics if `role` is root, the canister wasm cannot be built/read, the
/// canister install fails, or the canister does not report ready within the
/// configured tick limit.
#[must_use]
pub fn install_standalone_canister(
    crate_name: &str,
    role: CanisterRole,
    profile: CanicWasmBuildProfile,
) -> StandaloneCanisterFixture {
    assert!(
        !role.is_root(),
        "standalone helper is for non-root canisters"
    );

    let label = format!("standalone:{crate_name}:{role}");
    let wasm = standalone_canister_wasm(crate_name, profile);
    let pocket_ic = start_pocket_ic(PocketIcBuilder::new().with_application_subnet());
    let fixture = StandaloneCanisterFixture::try_install(
        ic_testkit::pic::PocketIcInstance::from_borrowed_server(pocket_ic),
        InstallSpec {
            wasm,
            init_bytes: local_init_args(),
            cycles: 0,
            install_sender: None,
            label: Some(label),
        },
    )
    .expect("install standalone canister");
    let canister_id = fixture.canister_id();
    let pic = fixture.pocket_ic();
    pic.wait_for_ready(
        canister_id,
        Principal::anonymous(),
        STANDALONE_READY_TICK_LIMIT,
        "standalone canister bootstrap",
    );

    fixture
}

/// Install one non-root Canic canister into an existing PocketIC instance.
///
/// # Panics
///
/// Panics if `role` is root, the canister wasm cannot be built/read, the
/// canister install fails, or the canister does not report ready within the
/// configured tick limit.
#[must_use]
pub fn install_standalone_canister_on_pic(
    pic: &PocketIc,
    crate_name: &str,
    role: CanisterRole,
    profile: CanicWasmBuildProfile,
    label: &str,
) -> Principal {
    assert!(
        !role.is_root(),
        "standalone helper is for non-root canisters"
    );

    let wasm = standalone_canister_wasm(crate_name, profile);
    let canister_id = pic
        .try_create_and_install(InstallSpec {
            wasm,
            init_bytes: local_init_args(),
            cycles: 0,
            install_sender: None,
            label: Some(label.to_string()),
        })
        .expect("install test canister");
    pic.wait_for_ready(
        canister_id,
        Principal::anonymous(),
        STANDALONE_READY_TICK_LIMIT,
        "standalone canister bootstrap",
    );

    canister_id
}

fn fetch_ready(
    pic: &PocketIc,
    canister_id: Principal,
    caller: Principal,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let response: Result<canic::dto::runtime_observation::CanicReadinessStatus, Error> =
        ic_testkit::pic::query_wire(
            pic,
            canister_id,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::protected_readiness::METHOD,
                method_id: canic::protocol::protected_readiness::METHOD_ID,
                request_limits: canic::protocol::protected_readiness::REQUEST_LIMITS,
                response_limits: canic::protocol::protected_readiness::RESPONSE_LIMITS,
            },
            &(),
        )?;
    response
        .map(|value| value.status == ReadinessStatus::Ready)
        .map_err(|error| std::io::Error::other(format!("{error:?}")).into())
}

#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn install_root_args_with_release_set_digest_and_coordinator(
    input: ManagedRootInstallInput<'_>,
) -> Result<Vec<u8>, Error> {
    encode(managed_test_root_init_args(input)?)
        .map_err(|_| Error::from_registered(canic_contracts::diagnostics::codes::STATE_FAILED))
}

/// Return the exact cached bytes used by standalone test installs and upgrades.
#[must_use]
pub fn standalone_canister_wasm(crate_name: &str, profile: CanicWasmBuildProfile) -> Vec<u8> {
    type CacheEntry = Arc<OnceLock<Vec<u8>>>;
    type CacheKey = (PathBuf, String, &'static str);

    static CACHE: OnceLock<Mutex<BTreeMap<CacheKey, CacheEntry>>> = OnceLock::new();

    let workspace_root = workspace_root();
    let key = (
        workspace_root.clone(),
        crate_name.to_owned(),
        profile.target_dir_name(),
    );
    let entry = {
        let mut cache = CACHE
            .get_or_init(|| Mutex::new(BTreeMap::new()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(
            cache
                .entry(key)
                .or_insert_with(|| Arc::new(OnceLock::new())),
        )
    };
    entry
        .get_or_init(|| {
            let target_name = format!("standalone-{crate_name}");
            let target_dir = workspace_root.join("target").join(&target_name);
            build_internal_test_wasm_canisters(&workspace_root, &target_dir, &[crate_name], profile)
                .wasm(crate_name)
        })
        .clone()
}

/// Extract the exact compiled declaration using the governed fixture build owner.
///
/// # Panics
/// Panics when declaration compilation, extraction or UTF-8 decoding fails.
#[must_use]
pub fn standalone_canister_candid(crate_name: &str, profile: CanicWasmBuildProfile) -> String {
    let workspace = workspace_root();
    let target = workspace
        .join("target")
        .join(format!("declaration-{crate_name}"));
    let artifacts = super::artifacts::build_internal_test_wasm_canisters_with_env(
        &workspace,
        &target,
        &[crate_name],
        profile,
        &[(canic_core::role_contract::CANONICAL_CANDID_BUILD_ENV, "1")],
    );
    String::from_utf8(
        canic_host::canister_build::extract_candid_bytes(artifacts.path(crate_name))
            .expect("extract compiled fixture declaration"),
    )
    .expect("Candid is UTF-8")
}

fn local_init_args() -> Vec<u8> {
    encode(None::<Vec<u8>>).expect("encode standalone-local init args")
}

///
/// ManagedTestIdentity
///
/// Deterministic non-root identity shared by internal lifecycle and authorization fixtures.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedTestIdentity {
    pub fleet: FleetBinding,
    pub install_id: [u8; 32],
    pub release_build_id: ReleaseBuildId,
}

/// Return the deterministic Fleet activation identity embedded in internal test Wasms.
///
/// # Panics
///
/// Panics if the repository-owned release-build fixture is not a valid identity.
#[must_use]
pub fn managed_test_init_identity() -> ManagedTestIdentity {
    let release_build_id = INTERNAL_TEST_RELEASE_BUILD_ID
        .1
        .parse::<ReleaseBuildId>()
        .expect("internal test release-build ID");
    ManagedTestIdentity {
        fleet: FleetBinding {
            fleet: FleetKey {
                canonical_network_id: CanonicalNetworkId::ic_mainnet(),
                fleet_id: FleetId::from_generated_bytes([0x42; 32]),
            },
            app: AppId::from("canic-internal-test"),
        },
        install_id: [0x43; 32],
        release_build_id,
    }
}

const fn managed_test_store_install_id() -> [u8; 32] {
    [0x47; 32]
}

fn managed_test_root_init_args(
    input: ManagedRootInstallInput<'_>,
) -> Result<FleetSubnetRootInitArgs, Error> {
    let ManagedRootInstallInput {
        root_id,
        wasm_store,
        installation_controller,
        coordinator,
        root_wasm,
        wasm_store_wasm,
        config_path,
        release_set_digest,
    } = input;
    let identity = managed_test_init_identity();
    let config = AppConfigSnapshot::load(config_path)
        .map_err(|_| Error::from_registered(canic_contracts::diagnostics::codes::STATE_FAILED))?;
    let topology = config.component_topology();
    let component_admissions = topology
        .component_specs
        .iter()
        .map(|spec| ComponentSpecAdmission {
            component_spec: spec.component_spec.clone(),
            spec_hash: spec.spec_hash,
            maximum_root_instances: spec.maximum_fleet_instances,
        })
        .collect::<Vec<_>>();
    let component_topology_digest = topology
        .project_for_admissions(&component_admissions)
        .and_then(|projection| projection.digest())
        .map_err(|_| Error::from_registered(canic_contracts::diagnostics::codes::STATE_FAILED))?;
    let expected_module_hash = <[u8; 32]>::try_from(wasm_hash(root_wasm))
        .expect("SHA-256 helper must return exactly 32 bytes");
    let expected_wasm_store_module_hash = <[u8; 32]>::try_from(wasm_hash(wasm_store_wasm))
        .expect("SHA-256 helper must return exactly 32 bytes");
    let authority = FleetRegistryAuthority {
        binding: FleetCoordinatorBinding {
            fleet: FleetBinding {
                fleet: identity.fleet.fleet,
                app: AppId::from(config.app_id()),
            },
            coordinator_subnet: test_subnet(0x40),
            coordinator,
            recovery_controllers: Vec::new(),
        },
        epoch: 1,
    };
    let mut store_controllers = vec![installation_controller, root_id];
    store_controllers.sort();

    Ok(FleetSubnetRootInitArgs {
        capacity_import_bootstrap: None,
        authority: FleetSubnetRootAuthority {
            binding: FleetSubnetRootBinding {
                authority: authority.clone(),
                placement_subnet: test_subnet(0x45),
                fleet_subnet_root: root_id,
                component_admissions,
                component_topology_digest,
                funding: super::root_funding_authority(),
                limits: FleetSubnetRootLimits {
                    maximum_component_instances: 4_096,
                    maximum_registry_bytes: 16_777_216,
                    maximum_wasm_store_bytes: 536_870_912,
                    maximum_group_placements: 16,
                    canister_pool: FleetSubnetCanisterPoolConfig {
                        minimum_size: 1,
                        maximum_size: 10,
                        canister_cycles: Cycles::new(5_000_000_000_000),
                        creation_execution_margin: Cycles::new(1_000_000_000_000),
                    },
                    cycles_funding: CyclesFundingBudget {
                        window_secs: 3_600,
                        maximum_cycles: Cycles::new(1_000_000_000_000_000),
                    },
                },
            },
            initial_release_set: FleetSubnetRootReleaseSet {
                release_build_id: identity.release_build_id,
                manifest_digest: release_set_digest,
            },
            expected_module_hash,
            wasm_store_authority: FleetSubnetWasmStoreAuthority {
                authority: authority.clone(),
                placement_subnet: test_subnet(0x45),
                fleet_subnet_root: root_id,
                wasm_store,
                installation_controller,
                release_build_id: identity.release_build_id,
                wasm_module_hash: expected_wasm_store_module_hash,
            },
        },
        install_id: identity.install_id,
        wasm_store_activation: FleetSubnetWasmStoreActivationAuthority {
            fleet: authority.binding.fleet,
            operation_id: managed_test_store_install_id(),
            fleet_subnet_root: root_id,
            wasm_store,
            release_build_id: identity.release_build_id,
            component_topology_digest,
            controllers: store_controllers,
            manifest_digest: release_set_digest,
        },
        canister_pool_imports: Vec::new(),
    })
}

const fn test_subnet(byte: u8) -> SubnetId {
    SubnetId::from_principal(Principal::from_slice(&[byte; 29]))
}

fn workspace_root() -> PathBuf {
    crate::workspace::discover().expect("discover Canic test workspace through Cargo")
}

/// Read the complete protected treasury response through the actual native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn coordinator_funding(
    pic: &ic_platform::pocket_ic::PocketIc,
    coordinator: Principal,
) -> Result<canic_contracts::dto::fleet_coordinator::CoordinatorFundingStatusResponse, Error> {
    let response: Result<canic_contracts::dto::funding_read::CoordinatorStatus, Error> =
        ic_testkit::pic::query_wire(
            pic,
            coordinator,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: canic::protocol::funding_read::coordinator_status::METHOD,
                method_id: canic::protocol::funding_read::coordinator_status::METHOD_ID,
                request_limits: canic::protocol::funding_read::REQUEST_LIMITS,
                response_limits: canic::protocol::funding_read::RESPONSE_LIMITS,
            },
            &(),
        )
        .expect("Coordinator funding transport");
    response.map(|native| {
        let expected = ic_wire::native::encode_response(
            canic::protocol::funding_read::coordinator_status::METHOD_ID,
            &native,
            canic::protocol::funding_read::RESPONSE_LIMITS,
        )
        .unwrap();
        let canonical =
            canic_contracts::dto::fleet_coordinator::CoordinatorFundingStatusResponse::from(native);
        let restored =
            canic_contracts::dto::funding_read::CoordinatorStatus::from(canonical.clone());
        assert_eq!(
            ic_wire::native::encode_response(
                canic::protocol::funding_read::coordinator_status::METHOD_ID,
                &restored,
                canic::protocol::funding_read::RESPONSE_LIMITS
            )
            .unwrap(),
            expected
        );
        canonical
    })
}
/// Observe exact Coordinator provisioning evidence through its native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn coordinator_provisioning(
    pic: &PocketIc,
    coordinator: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<
    canic_contracts::dto::component_provisioning::FleetComponentProvisioningStatusResponse,
    Error,
> {
    use canic::protocol::component_read as wire;
    let response: Result<canic::dto::component_read::CoordinatorProvisioning, Error> =
        ic_testkit::pic::query_wire(
            pic,
            coordinator,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: wire::coordinator_provisioning::METHOD,
                method_id: wire::coordinator_provisioning::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Coordinator provisioning transport");
    response.map(|native| {
        let canonical = canic_contracts::dto::component_provisioning::FleetComponentProvisioningStatusResponse::from(native.clone());
        assert_eq!(canic::dto::component_read::CoordinatorProvisioning::from(canonical.clone()), native);
        canonical
    })
}

/// Observe exact Root provisioning evidence through its native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn root_provisioning(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<
    canic_contracts::dto::component_provisioning::RootComponentProvisioningStatusResponse,
    Error,
> {
    use canic::protocol::component_read as wire;
    let response: Result<canic::dto::component_read::Provisioning, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: wire::provisioning::METHOD,
                method_id: wire::provisioning::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root provisioning transport");
    response.map(|native| {
        let canonical = canic_contracts::dto::component_provisioning::RootComponentProvisioningStatusResponse::from(native.clone());
        assert_eq!(canic::dto::component_read::Provisioning::from(canonical.clone()), native);
        canonical
    })
}

/// Observe exact Root component allocation evidence through its native endpoint.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn root_component_allocation(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::root::RootComponentOperationStatus, Error> {
    use canic::protocol::component_read as wire;
    let response: Result<canic::dto::component_read::ComponentAllocation, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: wire::allocation::METHOD,
                method_id: wire::allocation::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root component allocation transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::root::RootComponentOperationStatus::from(native.clone());
        assert_eq!(
            canic::dto::component_read::ComponentAllocation::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Observe exact Root subtree removal evidence through its native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn root_subtree_removal(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::component_registry::RootComponentSubtreeRemovalResponse, Error> {
    use canic::protocol::subtree_read as wire;
    let response: Result<canic::dto::subtree_read::Status, Error> = ic_testkit::pic::query_wire(
        pic,
        root,
        caller,
        ic_testkit::pic::WireCallSpec {
            method: wire::METHOD,
            method_id: wire::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &operation_id,
    )
    .expect("native Root subtree removal transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::component_registry::RootComponentSubtreeRemovalResponse::from(
                native.clone(),
            );
        assert_eq!(
            canic::dto::subtree_read::Status::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Observe exact Root removal evidence through its native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn root_removal(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::root::RootRemovalOperationStatus, Error> {
    use canic::protocol::root_removal_read as wire;
    let response: Result<canic::dto::root_removal_read::Status, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: wire::METHOD,
                method_id: wire::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root removal transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::root::RootRemovalOperationStatus::from(native.clone());
        assert_eq!(
            canic::dto::root_removal_read::Status::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Observe exact Root child allocation evidence through its native endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn root_child_allocation(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::component_registry::RootComponentChildAllocationResponse, Error> {
    use canic::protocol::component_read as wire;
    let response: Result<canic::dto::component_read::ChildAllocation, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            caller,
            ic_testkit::pic::WireCallSpec {
                method: wire::child_allocation::METHOD,
                method_id: wire::child_allocation::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root child allocation transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::component_registry::RootComponentChildAllocationResponse::from(
                native.clone(),
            );
        assert_eq!(
            canic::dto::component_read::ChildAllocation::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Read retained acknowledgement evidence through its controller-bound Wire endpoint.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn coordinator_root_acknowledgements(
    pic: &ic_platform::pocket_ic::PocketIc,
    coordinator: Principal,
    caller: Principal,
) -> Result<
    Vec<canic_contracts::dto::fleet_registry::FleetSubnetRootSnapshotAcknowledgement>,
    canic::Error,
> {
    use canic::protocol::coordinator_acknowledgements as protocol;
    let response: Result<
        Vec<canic_contracts::dto::coordinator_acknowledgements::RootAcknowledgement>,
        canic::Error,
    > = ic_testkit::pic::query_wire(
        pic,
        coordinator,
        caller,
        ic_testkit::pic::WireCallSpec {
            method: protocol::METHOD,
            method_id: protocol::METHOD_ID,
            request_limits: protocol::REQUEST_LIMITS,
            response_limits: protocol::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("query Coordinator acknowledgement evidence");
    response.map(|values| values.into_iter().map(Into::into).collect())
}

pub(super) fn root_store_adopt(
    pic: &PocketIc,
    root: Principal,
    request: canic::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    use canic::protocol::root_store_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::adopt::METHOD,
            method_id: wire::adopt::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &canic::dto::root_store_control::AdoptRequest::from(request),
    )
    .expect("native Root Store control transport")
}
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn root_store_bootstrap(
    pic: &PocketIc,
    root: Principal,
    request: canic::dto::root_store::RootStoreBootstrapRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    use canic::protocol::root_store_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::bootstrap::METHOD,
            method_id: wire::bootstrap::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native Root Store control transport")
}
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn root_store_prepare(
    pic: &PocketIc,
    root: Principal,
    request: canic::dto::root_store::RootStoreFixturePrepareRequest,
) -> Result<
    Result<
        canic::dto::fixture_provisioning::FixtureSourceStatus,
        canic::dto::fixture_provisioning::FixtureStoreError,
    >,
    Error,
> {
    use canic::protocol::root_store_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::prepare::METHOD,
            method_id: wire::prepare::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native Root Store control transport")
}

/// Drive the production bounded pool step through its native controller endpoint.
#[cfg(all(test, feature = "governed-pocketic-tests"))]
pub(super) fn maintain_pool(
    pic: &PocketIc,
    root: Principal,
) -> Result<canic::dto::pool_control::MaintenanceResponse, Error> {
    use canic::protocol::pool_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        Principal::anonymous(),
        ic_testkit::pic::WireCallSpec {
            method: wire::maintain::METHOD,
            method_id: wire::maintain::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &(),
    )
    .expect("native pool maintenance transport")
}

/// Submit one exact Component reservation as the actual installed caller.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn provision_component(
    pic: &PocketIc,
    root: Principal,
    caller: Principal,
    request: canic::dto::component_registry::RootComponentAllocationRequest,
) -> Result<canic::dto::role::OperationReceipt, Error> {
    use canic::protocol::component_control as wire;
    ic_testkit::pic::update_wire(
        pic,
        root,
        caller,
        ic_testkit::pic::WireCallSpec {
            method: wire::provision::METHOD,
            method_id: wire::provision::METHOD_ID,
            request_limits: wire::REQUEST_LIMITS,
            response_limits: wire::RESPONSE_LIMITS,
        },
        &request,
    )
    .expect("native Component reservation transport")
}

/// Prepare the actual Root and retain canonical authority for installed assertions.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn prepare_component_registry(
    pic: &PocketIc,
    root: Principal,
    request: canic::dto::component_registry::RootComponentRegistryPreparationRequest,
) -> Result<canic::dto::component_registry::RootComponentRegistryStatusResponse, Error> {
    use canic::protocol::component_control as wire;
    let response: Result<canic::dto::component_control::RegistryStatus, Error> =
        ic_testkit::pic::update_wire(
            pic,
            root,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: wire::prepare::METHOD,
                method_id: wire::prepare::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &canic::dto::component_control::PreparationRequest::from(request),
        )
        .expect("native Component registry preparation transport");
    response.map(Into::into)
}

/// Read exact retained Store adoption evidence through the native production endpoint.
pub(super) fn root_store_adoption_status(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse, Error> {
    use canic::protocol::root_store_control as wire;
    let response: Result<canic::dto::root_store_control::Adoption, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: wire::adoption::METHOD,
                method_id: wire::adoption::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root Store observation transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::fleet_subnet_root::FleetSubnetWasmStoreAdoptionResponse::from(
                native.clone(),
            );
        assert_eq!(
            canic::dto::root_store_control::Adoption::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Read exact retained Store bootstrap evidence through the native production endpoint.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn root_store_bootstrap_status(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::root_store::RootStoreBootstrapResponse, Error> {
    use canic::protocol::root_store_control as wire;
    let response: Result<canic::dto::root_store_control::Bootstrap, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: wire::bootstrap_status::METHOD,
                method_id: wire::bootstrap_status::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root Store observation transport");
    response.map(|native| {
        let canonical =
            canic_contracts::dto::root_store::RootStoreBootstrapResponse::from(native.clone());
        assert_eq!(
            canic::dto::root_store_control::Bootstrap::from(canonical.clone()),
            native
        );
        canonical
    })
}

/// Observe the original activation identity and credentials without resuming it.
#[cfg(any(not(test), feature = "governed-pocketic-tests"))]
pub(super) fn root_activation_status(
    pic: &PocketIc,
    root: Principal,
    operation_id: [u8; 32],
) -> Result<canic_contracts::dto::fleet_activation::FleetActivationStatusResponse, Error> {
    use canic::protocol::root_activation as wire;
    let response: Result<canic::dto::runtime_reporting::FleetActivationStatusResponse, Error> =
        ic_testkit::pic::query_wire(
            pic,
            root,
            Principal::anonymous(),
            ic_testkit::pic::WireCallSpec {
                method: wire::status::METHOD,
                method_id: wire::status::METHOD_ID,
                request_limits: wire::REQUEST_LIMITS,
                response_limits: wire::RESPONSE_LIMITS,
            },
            &operation_id,
        )
        .expect("native Root activation observation transport");
    response.map(|native| {
        let canonical = canic_contracts::dto::fleet_activation::FleetActivationStatusResponse::from(
            native.clone(),
        );
        assert_eq!(
            canic::dto::runtime_reporting::FleetActivationStatusResponse::from(canonical.clone()),
            native
        );
        canonical
    })
}
