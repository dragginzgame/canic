use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

use candid::{CandidType, Deserialize, Principal, encode_one};
use canic::{
    Error,
    dto::{
        fleet_activation::{
            FleetActivationPhase, FleetActivationResumeRequest, FleetActivationStatusResponse,
        },
        fleet_subnet_root::{
            FleetSubnetRootAuthority, FleetSubnetRootInitArgs, FleetSubnetWasmStoreAdoptionRequest,
            FleetSubnetWasmStoreInitArgs,
        },
        role::{OperationReceipt, OperationStatusRequest},
        runtime::{CanicReadinessStatus, ReadinessStatus},
    },
    ids::{
        CanisterRole, ComponentSpecAdmission, CyclesFundingBudget, FleetCoordinatorBinding,
        FleetRegistryAuthority, FleetSubnetCanisterPoolConfig, FleetSubnetRootBinding,
        FleetSubnetRootLimits, FleetSubnetRootReleaseSet, FleetSubnetWasmStoreActivationAuthority,
        FleetSubnetWasmStoreAuthority, ReleaseSetDigest, SubnetId,
    },
    protocol,
};
use canic_control_plane::dto::root::RootOperationStatusResponse;
use canic_core::{
    cdk::{types::Cycles, utils::hash::wasm_hash},
    ids::{AppId, CanonicalNetworkId, FleetBinding, FleetId, FleetKey, ReleaseBuildId},
};
use canic_host::release_set::AppConfigSnapshot;
use ic_testkit::{
    artifacts::{read_wasm, test_target_dir, workspace_root_for},
    pic::{
        CandidCallError, CandidCallExt, CanisterDiagnosticsRequest, CanisterInstallExt,
        InstallSpec, LabeledCanisterDiagnosticsRequest, PocketIc, PocketIcBuilder,
        PocketIcDiagnosticsExt, StandaloneCanisterFixture,
    },
};

use super::artifacts::{
    CanicWasmBuildProfile, INTERNAL_TEST_RELEASE_BUILD_ID, build_internal_test_wasm_canisters,
};
use super::startup::start_pocket_ic;

const INSTALL_CYCLES: u128 = 500_000_000_000_000;
const STANDALONE_READY_TICK_LIMIT: usize = 60;

#[derive(CandidType)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the direct Root command wire without a parallel DTO"
)]
enum RootCommandFragment {
    AdoptStore(FleetSubnetWasmStoreAdoptionRequest),
    PrepareFleetActivation,
    ResumeFleetActivation(FleetActivationResumeRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    OperationAccepted(OperationReceipt),
}

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Operation(OperationStatusRequest),
    Readiness,
}

#[derive(CandidType, Deserialize)]
#[expect(
    clippy::large_enum_variant,
    reason = "the test decoder mirrors the direct Root status wire without a parallel DTO"
)]
enum RootStatusResponseFragment {
    Operation(RootOperationStatusResponse),
    Readiness(CanicReadinessStatus),
}

#[derive(CandidType)]
enum StoreStatusRequestFragment {
    Operation(OperationStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum StoreStatusResponseFragment {
    Operation(StoreOperationStatusFragment),
}

#[derive(CandidType, Deserialize)]
enum StoreOperationStatusFragment {
    FleetActivation(FleetActivationStatusResponse),
}

fn root_command(
    pic: &PocketIc,
    root: Principal,
    command: RootCommandFragment,
) -> Result<RootCommandResponseFragment, Error> {
    pic.update_candid(root, protocol::CANIC_COMMAND, (command,))
        .expect("Root command transport")
}

fn root_status(
    pic: &PocketIc,
    root: Principal,
    request: RootStatusRequestFragment,
) -> Result<RootStatusResponseFragment, Error> {
    pic.query_candid(root, protocol::CANIC_STATUS, (request,))
        .expect("Root status transport")
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
    pub wasm_store: Principal,
    pub installation_controller: Principal,
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
        encode_one(store_args)
            .map_err(|_| Error::from_registered(canic_core::diagnostics::codes::STATE_FAILED))?,
        Some(installation_controller),
    );
    pic.install_canister(
        root_id,
        root_wasm,
        encode_one(&root_args)
            .map_err(|_| Error::from_registered(canic_core::diagnostics::codes::STATE_FAILED))?,
        None,
    );
    Ok(PreparedManagedRoot {
        root_id,
        wasm_store,
        installation_controller,
        root_args,
    })
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

    /// Wait until one Canic canister reports `canic_status(Readiness)`.
    fn wait_for_ready(
        &self,
        canister_id: Principal,
        diagnostic_sender: Principal,
        tick_limit: usize,
        context: &str,
    );

    /// Wait until all provided Canic canisters report `canic_status(Readiness)`.
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
            match fetch_ready(self, canister_id) {
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

    fn wait_for_all_ready<I>(&self, targets: I, tick_limit: usize, context: &str)
    where
        I: IntoIterator<Item = (Principal, Principal)>,
    {
        let targets = targets.into_iter().collect::<Vec<_>>();

        for _ in 0..tick_limit {
            self.tick();
            let mut all_ready = true;
            let mut failures = Vec::new();
            for (canister_id, _) in &targets {
                match fetch_ready(self, *canister_id) {
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
    let report = pic.collect_canister_diagnostics(CanisterDiagnosticsRequest::new(
        canister_id,
        diagnostic_sender,
        diagnostic_sender,
    ));
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
        .map(|(label, canister_id, diagnostic_sender)| {
            LabeledCanisterDiagnosticsRequest::new(
                label,
                CanisterDiagnosticsRequest::new(canister_id, diagnostic_sender, diagnostic_sender),
            )
        })
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
    let RootCommandResponseFragment::OperationAccepted(receipt) =
        root_command(pic, root, RootCommandFragment::AdoptStore(request.clone()))
            .expect("adopt sibling Wasm Store application");
    assert_eq!(receipt.operation_id, request.operation_id);
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::AdoptStore(adopted)) =
        root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: request.operation_id,
            }),
        )
        .expect("query sibling Wasm Store adoption application")
    else {
        panic!("Root returned a differently correlated operation status");
    };
    assert_eq!(adopted.authority, request.authority);
    let mut expected_controllers = vec![root, request.authority.installation_controller];
    expected_controllers.sort();
    assert_eq!(adopted.controllers, expected_controllers);
    let mut live_controllers = pic
        .canister_status(request.authority.wasm_store, Some(root))
        .expect("observe adopted sibling Wasm Store controllers");
    live_controllers.settings.controllers.sort();
    assert_eq!(live_controllers.settings.controllers, expected_controllers);
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::AdoptStore(status)) =
        root_status(
            pic,
            root,
            RootStatusRequestFragment::Operation(OperationStatusRequest {
                operation_id: request.operation_id,
            }),
        )
        .expect("query sibling Wasm Store adoption application")
    else {
        panic!("Root returned a differently correlated operation status");
    };
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

/// Drive one prepared managed Fleet through the exact controller activation protocol.
///
/// # Panics
///
/// Panics when preparation or activation transport fails, the root rejects
/// either operation, or the resulting phase is not the exact expected phase.
pub(super) fn activate_managed_fleet(
    pic: &PocketIc,
    root_id: Principal,
) -> FleetActivationStatusResponse {
    let RootCommandResponseFragment::OperationAccepted(receipt) =
        root_command(pic, root_id, RootCommandFragment::PrepareFleetActivation).unwrap_or_else(
            |error| {
                report_canister_diagnostics(
                    pic,
                    root_id,
                    Principal::anonymous(),
                    "Fleet activation preparation application",
                );
                panic!("Fleet activation preparation application: {error:?}");
            },
        );
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::FleetActivation(
        prepared,
    )) = root_status(
        pic,
        root_id,
        RootStatusRequestFragment::Operation(OperationStatusRequest {
            operation_id: receipt.operation_id,
        }),
    )
    .expect("Fleet activation preparation status")
    else {
        panic!("Root returned a differently correlated operation status");
    };
    assert_eq!(prepared.phase, FleetActivationPhase::Prepared);
    let credential = prepared
        .credential
        .expect("prepared root must expose its credential generation");

    let RootCommandResponseFragment::OperationAccepted(resumed) = root_command(
        pic,
        root_id,
        RootCommandFragment::ResumeFleetActivation(FleetActivationResumeRequest {
            operation_id: prepared.identity.operation_id,
            credential,
        }),
    )
    .expect("Fleet activation resume application");
    let RootStatusResponseFragment::Operation(RootOperationStatusResponse::FleetActivation(
        activated,
    )) = root_status(
        pic,
        root_id,
        RootStatusRequestFragment::Operation(OperationStatusRequest {
            operation_id: resumed.operation_id,
        }),
    )
    .expect("Fleet activation resume status")
    else {
        panic!("Root returned a differently correlated operation status");
    };
    assert_eq!(activated.phase, FleetActivationPhase::Active);
    let [store] = prepared
        .cascade_manifest
        .as_deref()
        .expect("prepared root must expose its Store cascade manifest")
    else {
        panic!("prepared root must retain exactly one Store child");
    };
    let store_response: Result<StoreStatusResponseFragment, Error> = pic
        .query_candid(
            store.principal,
            protocol::CANIC_STATUS,
            (StoreStatusRequestFragment::Operation(
                OperationStatusRequest {
                    operation_id: managed_test_store_install_id(),
                },
            ),),
        )
        .expect("Store activation status transport");
    let StoreStatusResponseFragment::Operation(StoreOperationStatusFragment::FleetActivation(
        store_status,
    )) = store_response.expect("Store activation status");
    assert_ne!(
        store_status.identity.operation_id,
        activated.identity.operation_id
    );
    assert_eq!(
        store_status.identity.operation_id,
        managed_test_store_install_id()
    );
    assert_eq!(store_status.phase, FleetActivationPhase::Active);
    activated
}

/// Wait until one Canic canister reports `canic_status(Readiness)`.
///
/// # Panics
///
/// Panics if the canister does not report ready within `tick_limit` ticks, or
/// if querying readiness traps.
pub fn wait_until_ready(pic: &PocketIc, canister_id: Principal, tick_limit: usize) {
    for _ in 0..tick_limit {
        if matches!(
            pic.query_candid_as::<Result<RootStatusResponseFragment, Error>, _>(
                canister_id,
                Principal::anonymous(),
                protocol::CANIC_STATUS,
                (RootStatusRequestFragment::Readiness,),
            ),
            Ok(Ok(RootStatusResponseFragment::Readiness(
                CanicReadinessStatus {
                    status: ReadinessStatus::Ready,
                    ..
                }
            )))
        ) {
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
    let fixture = StandaloneCanisterFixture::install(
        pocket_ic,
        InstallSpec::new(wasm, local_init_args(), 0).label(label),
    );
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
        .create_and_install(InstallSpec::new(wasm, local_init_args(), 0).label(label.to_string()));
    pic.wait_for_ready(
        canister_id,
        Principal::anonymous(),
        STANDALONE_READY_TICK_LIMIT,
        "standalone canister bootstrap",
    );

    canister_id
}

fn fetch_ready(pic: &PocketIc, canister_id: Principal) -> Result<bool, CandidCallError> {
    match pic.query_candid::<Result<RootStatusResponseFragment, Error>, _>(
        canister_id,
        protocol::CANIC_STATUS,
        (RootStatusRequestFragment::Readiness,),
    ) {
        Ok(Ok(RootStatusResponseFragment::Readiness(readiness))) => {
            Ok(readiness.status == ReadinessStatus::Ready)
        }
        Ok(Ok(_)) => panic!("role returned a differently correlated readiness status"),
        Ok(Err(_)) => Ok(false),
        Err(error) => Err(error),
    }
}

pub(super) fn install_root_args_with_release_set_digest_and_coordinator(
    input: ManagedRootInstallInput<'_>,
) -> Result<Vec<u8>, Error> {
    encode_one(managed_test_root_init_args(input)?)
        .map_err(|_| Error::from_registered(canic_core::diagnostics::codes::STATE_FAILED))
}

fn ensure_canister_wasm_ready(
    workspace_root: &Path,
    target_dir: &Path,
    crate_name: &str,
    profile: CanicWasmBuildProfile,
) {
    build_internal_test_wasm_canisters(workspace_root, target_dir, &[crate_name], profile);
}

fn standalone_canister_wasm(crate_name: &str, profile: CanicWasmBuildProfile) -> Vec<u8> {
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
            let target_dir = test_target_dir(&workspace_root, &target_name);
            ensure_canister_wasm_ready(&workspace_root, &target_dir, crate_name, profile);
            read_wasm(&target_dir, crate_name, profile.target_dir_name())
        })
        .clone()
}

fn local_init_args() -> Vec<u8> {
    encode_one(None::<Vec<u8>>).expect("encode standalone-local init args")
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
        .map_err(|_| Error::from_registered(canic_core::diagnostics::codes::STATE_FAILED))?;
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
        .map_err(|_| Error::from_registered(canic_core::diagnostics::codes::STATE_FAILED))?;
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
        },
        epoch: 1,
    };
    let mut store_controllers = vec![installation_controller, root_id];
    store_controllers.sort();

    Ok(FleetSubnetRootInitArgs {
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
    workspace_root_for(env!("CARGO_MANIFEST_DIR"))
}
